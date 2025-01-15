use std::{fs::File, io::Read, ptr::null};

use block_mesh::{greedy_quads, GreedyQuadsBuffer, VoxelVisibility};
use vulkanalia::vk;
use vulkanalia_vma::Alloc;

use crate::{
    containers::Array3D, ivec3, types::{self, Voxel}, uvec3, vec3, BlockID_t, BlockWithMesh, FaceBuffers, InternalMeshModel, LumRenderer, Material, BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y
};

impl crate::LumRenderer {
    // Palette on CPU side is (should) be represented as a POD array
    // Palette on GPU side is stored differently (in 2d array of 3d blocks). This is due to perfomance win + hw limitations
    // E.g. just doing 16*len x 16 x 16 will not work cause 16xlen will be too big size for some gpus
    pub fn update_block_palette_to_gpu(&mut self, block_palette: &[BlockWithMesh]) {
        assert!(block_palette.len() == self.static_block_palette_size as usize);
        // create 3d array to be copied to gpu-side image after it is filled
        let mut block_palette_prepared = Array3D::<Voxel>::new(
            (16 * BLOCK_PALETTE_SIZE_X).try_into().unwrap(),
            (16 * BLOCK_PALETTE_SIZE_Y).try_into().unwrap(),
            16,
        );
        block_palette_prepared.data.fill(Voxel(0));

        for (i, block) in block_palette.iter().enumerate() {
            let block_xy = self.index_block_xy(i as i32);
            for x in 0..16 {
                for y in 0..16 {
                    for z in 0..16 {
                        block_palette_prepared[(
                            x + block_xy.x as usize * 16,
                            y + block_xy.y as usize * 16,
                            z,
                        )] = block.voxels[x as usize][y as usize][z as usize];
                    }
                }
            }
        }

        let buffer_size = block_palette_prepared.dimensions().0
            * block_palette_prepared.dimensions().1
            * block_palette_prepared.dimensions().2
            * std::mem::size_of::<Voxel>();

        let stagin_buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            flags: vk::BufferCreateFlags::empty(),
            size: buffer_size as vk::DeviceSize,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            next: null(),
            queue_family_indices: null(),
        };

        let stagin_alloc_info = vulkanalia_vma::AllocationOptions {
            flags: vulkanalia_vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            usage: vulkanalia_vma::MemoryUsage::Auto,
            required_flags: vk::MemoryPropertyFlags::HOST_COHERENT,
            preferred_flags: Default::default(),
            memory_type_bits: Default::default(),
            priority: Default::default(),
        };

        let (staging_buffer, stagin_alloc) = unsafe {
            self.lumal
                .allocator
                .as_ref()
                .unwrap()
                .create_buffer(stagin_buffer_info, &stagin_alloc_info)
                .unwrap()
        };

        let mapped = unsafe {
            self.lumal
                .allocator
                .as_ref()
                .unwrap()
                .map_memory(stagin_alloc)
                .unwrap()
        };

        unsafe {
            std::ptr::copy_nonoverlapping(
                block_palette_prepared.data.as_ptr(),
                mapped as *mut Voxel,
                buffer_size.try_into().unwrap(),
            );
        };

        self.lumal.copy_buffer_to_image_single_time(
            staging_buffer,
            &self.independent_images.origin_block_palette.current(),
            vk::Extent3D {
                width: block_palette_prepared.dimensions().0 as u32,
                height: block_palette_prepared.dimensions().1 as u32,
                depth: block_palette_prepared.dimensions().2 as u32,
            },
        );
    }

    pub fn update_material_palette_to_gpu(&mut self) {
        let buffer_size = self.material_palette.len() * std::mem::size_of::<Material>();

        let staging_buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            flags: vk::BufferCreateFlags::empty(),
            size: buffer_size as vk::DeviceSize,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            next: null(),
            queue_family_indices: null(),
        };

        let stagin_alloc_info = vulkanalia_vma::AllocationOptions {
            flags: vulkanalia_vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            usage: vulkanalia_vma::MemoryUsage::Auto,
            required_flags: vk::MemoryPropertyFlags::HOST_COHERENT,
            preferred_flags: Default::default(),
            memory_type_bits: Default::default(),
            priority: Default::default(),
        };

        let (staging_buffer, staging_alloc) = unsafe {
            self.lumal
                .allocator
                .as_ref()
                .unwrap()
                .create_buffer(staging_buffer_info, &stagin_alloc_info)
                .unwrap()
        };

        let mapped = unsafe {
            self.lumal
                .allocator
                .as_ref()
                .unwrap()
                .map_memory(staging_alloc)
                .unwrap()
        };

        unsafe {
            std::ptr::copy_nonoverlapping(
                self.material_palette.as_ptr(),
                mapped as *mut Material,
                buffer_size.try_into().unwrap(),
            );
        };

        self.lumal.copy_buffer_to_image_single_time(
            staging_buffer,
            &self.independent_images.material_palette.current(),
            vk::Extent3D {
                width: 6, // yep this is how it works for now
                height: self.material_palette.len() as u32,
                depth: 1,
            },
        );
    }

    pub fn extract_palette_from_scene(&mut self, scene: &dot_vox::DotVoxData) {
        assert!(self.material_palette.len() == scene.materials.len());
        dbg!(scene.materials.len());
        dbg!(self.material_palette.len());

        // fill Albedo and transparency only, cause thats how it works
        for (i, material) in self.material_palette.iter_mut().enumerate() {
            let mv_col = &scene.palette[i];
            *material = crate::Material {
                albedo: vec3::new(
                    mv_col.r as f32 / 255.0,
                    mv_col.g as f32 / 255.0,
                    mv_col.b as f32 / 255.0,
                ),
                transparency: mv_col.a as f32 / 255.0,
                emmitness: 0.0,
                roughness: 0.0,
            }
        }

        // now fill emission and roughness
        for (i, mv_mat) in scene.materials.iter().enumerate() {
            let actual_mat_idx = mv_mat.id as usize - 1;

            dbg!(actual_mat_idx);
            let material = &mut self.material_palette[actual_mat_idx];

            // TODO: Enum/match?
            let is_metal = mv_mat.properties.get("_type") == Some(&"_metal".to_string());
            let is_diffuse = mv_mat.properties.get("_type") == Some(&"_diffuse".to_string());
            let is_emmit = mv_mat.properties.get("_type") == Some(&"_emit".to_string());

            material.emmitness = mv_mat.emission().unwrap_or(0.0);
            material.roughness = mv_mat.roughness().unwrap_or(0.0);

            // mappings from MagicaVoxel materials to my materials are weird
            match mv_mat.properties.get("_type").map(String::as_str) {
                Some("_diffuse") => {
                    material.roughness = 1.0;
                }
                Some("_metal") => {
                    assert!(!is_diffuse);
                    let old_rough = material.roughness;
                    let metalicness = mv_mat.metalness().unwrap_or(0.0);
                    material.roughness = (old_rough + (1.0 - metalicness)) / 2.0;
                }
                Some("_emit") => {
                    assert!(!is_diffuse);
                    assert!(!is_metal);
                    let old_emit = material.emmitness;
                    let fluxness = mv_mat.radiant_flux().unwrap_or(0.0);
                    material.emmitness = (old_emit * (2.0 + fluxness * 4.0)); // yep i store it in IEEE 754 float
                    material.roughness = 0.5; // when just shiny it looks worse
                }
                Some(wtf) => {
                    println!("Unknown material type");
                    println!("{:?}", wtf);
                    panic!();
                }
                None => {
                    // panic!("Unknown material type");
                }
            }
        }
    }
    /*
    //size limited by 256^3
    void LumInternal::LumInternalRenderer::load_mesh(InternalMeshModel* mesh, const char* vox_file, bool _make_vertices, bool extrude_palette, Material* mat_palette) {
        auto buffer = readFileBuffer(vox_file);
        assert(not buffer.empty());
        const ogt::vox_scene* scene = ogt::vox_read_scene((u8*)buffer.data(), buffer.size());

        assert(scene != NULL);
        assert(scene->num_models == 1);

        load_mesh(mesh, (Voxel*)scene->models[0]->voxel_data, scene->models[0]->size_x, scene->models[0]->size_y, scene->models[0]->size_z, _make_vertices);

        if (extrude_palette && !hasPalette) {
            assert(mat_palette != NULL);
            hasPalette = true;
            extract_palette_mem(scene, mat_palette);
        }

        ogt::vox_destroy_scene(scene);
    }

    // pointer to block_ptr, so block_ptr can be modified
    void LumInternal::LumInternalRenderer::load_block(BlockWithMesh* block, const char* vox_file) {
        auto buffer = readFileBuffer(vox_file);
        assert(not buffer.empty());
        const ogt::vox_scene* scene = ogt::vox_read_scene((u8*)buffer.data(), buffer.size());

        assert(scene != NULL);
        assert(scene->num_models == 1);
        // if ((block) == NULL) {
        //     *block = new Block;
        TRACE()
        // }
        (block)->mesh.size = ivec3(scene->models[0]->size_x, scene->models[0]->size_y, scene->models[0]->size_z);
        TRACE()
        assert(scene->models[0]->size_x > 0 && scene->models[0]->size_y > 0 && scene->models[0]->size_z > 0);
        TRACE()
        make_vertices(&((block)->mesh), (Voxel*)scene->models[0]->voxel_data, scene->models[0]->size_x, scene->models[0]->size_y, scene->models[0]->size_z);
        TRACE()
        for (int x = 0; x < scene->models[0]->size_x; x++) {
            for (int y = 0; y < scene->models[0]->size_y; y++) {
                for (int z = 0; z < scene->models[0]->size_z; z++) {
                    (block)->voxels[x][y][z] = (u16)scene->models[0]->voxel_data[x + y * scene->models[0]->size_x + z * scene->models[0]->size_x * scene->models[0]->size_y];
                }
            }
        }
        TRACE()
        ogt::vox_destroy_scene(scene);
    }

    // #define free_helper(dir) \
    // if(not (block)->mesh.triangles.dir.indexes.empty()) vmaDestroyBuffer(render.VMAllocator, (block)->mesh.triangles.dir.indexes[i].buffer, (block)->mesh.triangles.dir.indexes[i].alloc);
    #define free_helper(__dir) \
    if(block->mesh.triangles.__dir.indexes.buffer) {\
        BufferDeletion bd = {};\
            bd.buffer = block->mesh.triangles.__dir.indexes;\
            bd.life_counter = render.settings.fif; /* delete after fif+1 frames*/\
        render.bufferDeletionQueue.push_back(bd);\
        block->mesh.triangles.__dir.indexes.buffer = VK_NULL_HANDLE;\
    }

    void LumInternal::LumInternalRenderer::free_block(BlockWithMesh* block) {
        assert(block != NULL);
        // assert(*block != NULL);

        // free_helper(Pzz);
        // free_helper(Nzz);
        // free_helper(zPz);
        // free_helper(zNz);
        // free_helper(zzP);
        // free_helper(zzN);
        if(block->mesh.triangles.indices.buffer) {\
            Lumal::BufferDeletion bd = {};\
                bd.buffer = block->mesh.triangles.indices;\
                bd.life_counter = lumal.settings.fif; /* delete after fif+1 frames*/\
            lumal.bufferDeletionQueue.push_back(bd);\
            block->mesh.triangles.indices.buffer = VK_NULL_HANDLE;\
        }
        if((block)->mesh.triangles.vertexes.buffer){
            vmaDestroyBuffer(lumal.VMAllocator, (block)->mesh.triangles.vertexes.buffer, (block)->mesh.triangles.vertexes.alloc);
            (block)->mesh.triangles.vertexes.buffer = VK_NULL_HANDLE;
        }
        // for (int i = 0; i < lumal.settings.fif; i++) {
            // if (!(block)->mesh.triangles.vertexes.empty()) {
            // }
        // }
        // delete *block;
        // *block = NULL;
    }

    void LumInternal::LumInternalRenderer::load_mesh(InternalMeshModel* mesh, Voxel* Voxels, int x_size, int y_size, int z_size, bool _make_vertices) {
        assert(mesh != NULL);
        assert(Voxels != NULL);
        assert(x_size > 0 && y_size > 0 && z_size > 0);
        mesh->size = ivec3(x_size, y_size, z_size);
        if (_make_vertices) {
            make_vertices(mesh, Voxels, x_size, y_size, z_size);
        }
        mesh->voxels = create_RayTrace_VoxelImages(Voxels, mesh->size);
    }

    //frees only gpu side stuff, not mesh ptr
    #undef  free_helper
    #define free_helper(__dir) \
    if(mesh->triangles.__dir.indexes.buffer) {\
        BufferDeletion bd = {};\
            bd.buffer = mesh->triangles.__dir.indexes;\
            bd.life_counter = render.settings.fif; /* delete after fif+1 frames*/\
        render.bufferDeletionQueue.push_back(bd);\
    }

    void LumInternal::LumInternalRenderer::free_mesh(InternalMeshModel* mesh) {
        assert(mesh != NULL);
        // free_helper(Pzz);
        // free_helper(Nzz);
        // free_helper(zPz);
        // free_helper(zNz);
        // free_helper(zzP);
        // free_helper(zzN);
        if(mesh->triangles.indices.buffer) {\
            Lumal::BufferDeletion bd = {};\
                bd.buffer = mesh->triangles.indices;\
                bd.life_counter = lumal.settings.fif; /* delete after fif+1 frames*/\
            lumal.bufferDeletionQueue.push_back(bd);\
        }
        Lumal::BufferDeletion
            bd = {};
            bd.buffer = mesh->triangles.vertexes;
            bd.life_counter = lumal.settings.fif; // delete after fif+1 frames
        lumal.bufferDeletionQueue.push_back(bd);
        for (int i = 0; i < lumal.settings.fif; i++) {
            // if (!mesh->triangles.vertexes.empty()) {
                // vmaDestroyBuffer(render.VMAllocator, mesh->triangles.vertexes[i].buffer, mesh->triangles.vertexes[i].alloc);
            // }
            if (!mesh->voxels.empty()) {
                // vmaDestroyImage(render.VMAllocator, mesh->voxels[i].image, mesh->voxels[i].alloc);
                // vkDestroyImageView(render.device, mesh->voxels[i].view, NULL);
                Lumal::ImageDeletion
                    id = {};
                    id.image = mesh->voxels[i];
                    id.life_counter = lumal.settings.fif; // delete after fif+1 frames
                lumal.imageDeletionQueue.push_back(id);
            }
        }
    }
    #undef free_helper

    bool operator== (const LumInternal::PackedVoxelVertex& one, const LumInternal::PackedVoxelVertex& other) {
        return
            (one.pos.x == other.pos.x) &&
            (one.pos.y == other.pos.y) &&
            (one.pos.z == other.pos.z)
            ;
    }

    bool operator!= (const LumInternal::PackedVoxelVertex& one, const LumInternal::PackedVoxelVertex& other) {
        return ! (one == other);
    }

    bool operator< (const LumInternal::PackedVoxelVertex& one, const LumInternal::PackedVoxelVertex& other) {
        return one != other;
    }

    LumInternal::PackedVoxelQuad pack_quad (const array<LumInternal::PackedVoxelVertex, 6> vertices, uvec3 norm) {
        LumInternal::PackedVoxelQuad quad = {};
        vector<LumInternal::PackedVoxelVertex> uniq;
        unsigned char mat = vertices[0].matID;
        for (auto v : vertices) {
            cout << glm::to_string (v.pos) << " \n";
            // uniq.insert(v);
            for (auto _v : uniq) {
                if (v == _v) {
                    goto label;
                }
            }
            uniq.push_back (v);
    label:
            ;
            // assert(mat == v.matID);
        }
        assert (uniq.size() == 4);
        cout << " \n";
        uvec3 low = uvec3 (+10000); //_corner
        uvec3 high = uvec3 (0); //_corner
        uvec3 diff = {};
        for (auto v : uniq) {
            cout << glm::to_string (v.pos) << " \n";
            low = glm::min (low, uvec3 (v.pos));
            high = glm::max (high, uvec3 (v.pos));
        }
        //general direction is from negative to positive
        diff = high - low;
        assert (any (greaterThan (diff, {0, 0, 0})));
        uvec3 plane = uvec3 (1) - abs (norm);
        cout << glm::to_string (plane) << " \n";
        cout << glm::to_string (diff) << " \n";
        assert (((diff.x == 0) == (plane.x == 0)));
        assert (((diff.y == 0) == (plane.y == 0)));
        assert (((diff.z == 0) == (plane.z == 0)));
        if (abs (norm).x == 1) {
            quad.size.x = diff.y;
            quad.size.y = diff.z;
        } else if (abs (norm).y == 1) {
            quad.size.x = diff.x;
            quad.size.y = diff.z;
        } else if (abs (norm).z == 1) {
            quad.size.x = diff.x;
            quad.size.y = diff.y;
        }
        quad.pos = low;
        return quad;
    }

    void repack_helper (vector<LumInternal::PackedVoxelVertex>& vs, vector<LumInternal::PackedVoxelQuad>& qs, uvec3 norm) {
        assert ((vs.size() % 6) == 0);
        for (int i = 0; i < vs.size() / 6; i++) {
            qs.push_back (pack_quad ({
                vs[i + 0], vs[i + 1], vs[i + 2],
                vs[i + 3], vs[i + 4], vs[i + 5],
            }, norm));
        }
    }

    //defenetly not an example of highly optimized code
    void LumInternal::LumInternalRenderer::make_vertices (InternalMeshModel* mesh, Voxel* Voxels, int x_size, int y_size, int z_size) {
        ogt::ogt_voxel_meshify_context ctx = {};

        const int totalVoxels = x_size * y_size * z_size;
        std::vector<Voxel> contour(totalVoxels);
        for (int i = 0; i < totalVoxels; ++i) {
            contour[i] = (Voxels[i] == 0) ? 0 : 1;
        }

        ogt::ogt_int_mesh* ogt_mesh = ogt::my_int_mesh_from_paletted_voxels (&ctx, (const u8*)contour.data(), x_size, y_size, z_size);
        ogt::my_int_mesh_optimize (&ctx, ogt_mesh);
    // TRACE();
        vector<VoxelVertex > verts (ogt_mesh->vertex_count);
        vector<PackedVoxelVertex> packed_verts (ogt_mesh->vertex_count);
        vector<PackedVoxelCircuit> circ_verts (ogt_mesh->vertex_count);
        for (u32 i = 0; i < ogt_mesh->vertex_count; i++) {
            // vec<3, unsigned char, defaultp> packed_posNorm = {};
            // packed_posNorm = uvec3(ogt_mesh->vertices[i].pos);
            // packed_posNorm.x |= (ogt_mesh->vertices[i].normal.x != 0)? (POS_NORM_BIT_MASK) : 0;
            // packed_posNorm.y |= (ogt_mesh->vertices[i].normal.y != 0)? (POS_NORM_BIT_MASK) : 0;
            // packed_posNorm.z |= (ogt_mesh->vertices[i].normal.z != 0)? (POS_NORM_BIT_MASK) : 0;
            verts[i].norm = ivec3 (ogt_mesh->vertices[i].normal);
            verts[i].pos = uvec3 (ogt_mesh->vertices[i].pos);
            // printl((int)verts[i].pos.x);
            verts[i].matID = (MatID_t)ogt_mesh->vertices[i].palette_index;
            assert (verts[i].norm != (vec<3, signed char, defaultp>) (0));
            assert (length (vec3 (verts[i].norm)) == 1.0f);
            assert (!any (greaterThanEqual (ogt_mesh->vertices[i].pos, uvec3 (255))));
            packed_verts[i].pos = uvec3 (ogt_mesh->vertices[i].pos);
            packed_verts[i].matID = (MatID_t)ogt_mesh->vertices[i].palette_index;
            circ_verts[i].pos = uvec3 (ogt_mesh->vertices[i].pos);
        }
        vector<u16> verts_Pzz = {};
        vector<u16> verts_Nzz = {};
        vector<u16> verts_zPz = {};
        vector<u16> verts_zNz = {};
        vector<u16> verts_zzP = {};
        vector<u16> verts_zzN = {};

        auto classify_normal = [&](const vec3& norm) {
            if (norm == vec3(1, 0, 0)) return &verts_Pzz;
            if (norm == vec3(-1, 0, 0)) return &verts_Nzz;
            if (norm == vec3(0, 1, 0)) return &verts_zPz;
            if (norm == vec3(0, -1, 0)) return &verts_zNz;
            if (norm == vec3(0, 0, 1)) return &verts_zzP;
            if (norm == vec3(0, 0, -1)) return &verts_zzN;
            return (std::vector<u16>*)(nullptr); // error
        };

        for (u32 i = 0; i < ogt_mesh->index_count; ++i) {
            u32 index = ogt_mesh->indices[i];
            u32 provoking_index = ogt_mesh->indices[(i / 3) * 3];
            const auto& norm = ogt_mesh->vertices[provoking_index].normal;

            auto* target_vector = classify_normal(norm);
            if (target_vector) target_vector->push_back(index);
            else crash("Unrecognized normal");
        }
    // TRACE();
        assert (circ_verts.size() == ogt_mesh->vertex_count);
        // mesh->triangles.vertexes = render.createElemBuffer<PackedVoxelCircuit> (circ_verts.data(), circ_verts.size(), VK_BUFFER_USAGE_VERTEX_BUFFER_BIT | VK_BUFFER_USAGE_TRANSFER_DST_BIT);
    // TRACE();
        assert (verts_Pzz.size() != 0);
        assert (verts_Nzz.size() != 0);
        assert (verts_zPz.size() != 0);
        assert (verts_zNz.size() != 0);
        assert (verts_zzP.size() != 0);
        assert (verts_zzN.size() != 0);

        std::vector<u16> all_indices;
        auto offset_and_insert = [&all_indices](auto& vector, IndexedVertices& section) {
            section.offset = all_indices.size();
            all_indices.insert(all_indices.end(), vector.begin(), vector.end());
            section.icount = vector.size();
        };

        offset_and_insert(verts_Pzz, mesh->triangles.Pzz);
        offset_and_insert(verts_Nzz, mesh->triangles.Nzz);
        offset_and_insert(verts_zPz, mesh->triangles.zPz);
        offset_and_insert(verts_zNz, mesh->triangles.zNz);
        offset_and_insert(verts_zzP, mesh->triangles.zzP);
        offset_and_insert(verts_zzN, mesh->triangles.zzN);

        mesh->triangles.vertexes = lumal.createElemBuffer<PackedVoxelCircuit>(
            circ_verts.data(),
            circ_verts.size(),
            VK_BUFFER_USAGE_VERTEX_BUFFER_BIT | VK_BUFFER_USAGE_TRANSFER_DST_BIT);

        mesh->triangles.indices = lumal.createElemBuffer<u16>(
            all_indices.data(),
            all_indices.size(),
            VK_BUFFER_USAGE_TRANSFER_DST_BIT | VK_BUFFER_USAGE_INDEX_BUFFER_BIT);

    // TRACE();
        ogt::ogt_mesh_destroy (&ctx, (ogt::ogt_mesh*)ogt_mesh);
    }

    */
    pub fn extract_palette_from_file(&mut self, scene_file: &str) {
        let scene = dot_vox::load(scene_file).unwrap();
        self.extract_palette_from_scene(&scene);
    }

    pub fn load_mesh_from_file(&mut self, mesh_file: &str, make_vertices: bool, extrude_palette: bool) -> InternalMeshModel{
        let scene = dot_vox::load(mesh_file).unwrap();
        assert!(scene.models.len() == 1); // only one model per file supported for now
        let model = &scene.models[0];

        if extrude_palette && !self.has_palette {
            self.extract_palette_from_scene(&scene);
        }
        
        return self.load_mesh_from_memory(model, true);
    }

    pub fn load_mesh_from_memory(
        &mut self,
        model: &dot_vox::Model,
        make_vertices: bool,
    ) -> InternalMeshModel {
        // dbg!(model.voxels.len());
        // dbg!(model.size);
        // dbg!((model.size.x * model.size.y * model.size.z) as usize);
        // They do not necessarily have to be equal, e.g. for big grid single-voxel model is len()==1
        // assert!(model.voxels.len() == (model.size.x * model.size.y * model.size.z) as usize);
        
        let size = uvec3 {
            x: model.size.x,
            y: model.size.y,
            z: model.size.z,
        };

        let mut plain_voxel_data = Array3D::<Voxel>::new(
            size.x as usize,
            size.y as usize,
            size.z as usize,
        );
        // fill with empty voxels
        plain_voxel_data.data.fill(Voxel(0));
        model.voxels.iter().for_each(|voxel| {
            plain_voxel_data[(
                voxel.x as usize, 
                voxel.y as usize, 
                voxel.z as usize
            )] = Voxel(voxel.i);
        });

        let voxels = self.create_rayrace_voxel_images(&plain_voxel_data.data, size);

        let mut buffer = GreedyQuadsBuffer::new(plain_voxel_data.data.len());

        // TODO: issue on block_mesh bad docs & examples
        let chunk_shape = block_mesh::ndshape::RuntimeShape::<u32, 3>::new([
            size.x + 2,
            size.y + 2,
            size.z + 2,
        ]);
            
        let faces = block_mesh::RIGHT_HANDED_Y_UP_CONFIG.faces;
        greedy_quads(
            &plain_voxel_data.data.as_slice(),
            &chunk_shape,
            [0; 3],
            [size.x+1, size.y+1, size.z+1],
            &faces,
            &mut buffer
        );

        assert!(buffer.quads.num_quads() > 0);

        let num_indices = buffer.quads.num_quads() * 6;
        let num_vertices = buffer.quads.num_quads() * 4;
        let mut indices = Vec::with_capacity(num_indices);
        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        
        for (group, face) in buffer.quads.groups.into_iter().zip(faces.into_iter()) {
            for quad in group.into_iter() {
                indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                positions.extend_from_slice(&face.quad_mesh_positions(&quad.into(), 1.0));
                normals.extend_from_slice(&face.quad_mesh_normals());
            }
        }

        dbg!(indices);
        dbg!(positions);
        dbg!(normals);

        return InternalMeshModel {
            triangles: todo!(),
            voxels,
            size,
        };
    }
}

impl block_mesh::Voxel for Voxel {
    fn get_visibility(&self) -> VoxelVisibility {
        if self.0 == 0 {
            VoxelVisibility::Empty
        } else {
            VoxelVisibility::Opaque
        }
    }
}

impl block_mesh::MergeVoxel for Voxel {
    type MergeValue = Self;

    fn merge_value(&self) -> Self::MergeValue {
        *self
    }
}
