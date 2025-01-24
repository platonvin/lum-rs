use std::ptr::null;

use block_mesh::{greedy_quads, GreedyQuadsBuffer, VoxelVisibility};
use internal_renderer::*;
use lumal::{BufferDeletion, ImageDeletion};
use vulkanalia::vk::{self, Handle};
use vulkanalia_vma::Alloc;

use crate::*;
use crate::types::*;

impl super::InternalRenderer {
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
        block_palette_prepared.data.fill(0);

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
            }}}
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
        // dbg!(scene.materials.len());
        // dbg!(self.material_palette.len());

        // fill Albedo and transparency only, cause thats how it works
        for (i, material) in self.material_palette.iter_mut().enumerate() {
            let mv_col = &scene.palette[i];
            *material = super::Material {
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
        for mv_mat in scene.materials.iter() {
            let actual_mat_idx = mv_mat.id as usize - 1;

            // dbg!(actual_mat_idx);
            let material = &mut self.material_palette[actual_mat_idx];

            material.emmitness = mv_mat.emission().unwrap_or(0.0);
            material.roughness = mv_mat.roughness().unwrap_or(0.0);

            // mappings from MagicaVoxel materials to my materials are weird
            match mv_mat.properties.get("_type").map(String::as_str) {
                Some("_diffuse") => {
                    material.roughness = 1.0;
                }
                Some("_metal") => {
                    let old_rough = material.roughness;
                    let metalicness = mv_mat.metalness().unwrap_or(0.0);
                    material.roughness = (old_rough + (1.0 - metalicness)) / 2.0;
                }
                Some("_emit") => {
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
    */

    pub fn extract_palette_from_file(&mut self, scene_file: &str) {
        let scene = dot_vox::load(scene_file).unwrap();
        self.extract_palette_from_scene(&scene);
    }

    pub fn load_mesh_from_file(&mut self, mesh_file: &str, make_vertices: bool, extrude_palette: bool) -> InternalMeshModel{
        let scene = dot_vox::load(mesh_file).unwrap();
        assert!(scene.models.len() == 1); // only one model per file supported for now
        let model = &scene.models[0];
        assert!(model.size.x > 0 && model.size.y > 0 && model.size.z > 0);

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
        assert!(model.size.x > 0 && model.size.y > 0 && model.size.z > 0);
        // They do not necessarily have to be equal, e.g. for big grid single-voxel model is len()==1
        // assert!(model.voxels.len() == (model.size.x * model.size.y * model.size.z) as usize);
        
        let size = uvec3 {
            x: model.size.x,
            y: model.size.y,
            z: model.size.z,
        };
        
        // plain 3d array representation
        let mut plain_voxel_data = Array3D::<VoxelForContour>::new(
            // +2 cause padding of 1 from each side
            (size.x + 2) as usize,
            (size.y + 2) as usize,
            (size.z + 2) as usize,
        ); plain_voxel_data.data.fill(VoxelForContour(0));

        // contour means that 2 different voxels (different color, roughness, etc) will be meshed into same triangle(s)
        // - But where does material come from then?
        // - from GPU-side 3d voxel arrays. That is faster (cause triangles without such approach are too small and overhead of raster is too heavy)
        // NOTE: data is still represented with u8
        // but type is VoxelForContour to implement a trait for mesher lib
        // In original C++ implementation this was separate array
        model.voxels.iter().for_each(|voxel| {
            // +1 cause padding of 1 from each side
            // TODO: check if it is actually needed
            plain_voxel_data[(
                (voxel.x + 1) as usize,
                (voxel.y + 1) as usize,
                (voxel.z + 1) as usize
            )] = VoxelForContour(voxel.i);
        });

        // we need 3d array as slice of u8s but we also need a separate type for trait. Here we are
        let pvd_data_u8_slice = unsafe {
            std::slice::from_raw_parts(
                (plain_voxel_data.data.as_ptr() as *const Voxel) as *const u8,
                std::mem::size_of::<Voxel>() * plain_voxel_data.data.len(),
            )
            };

        let voxels = self.create_rayrace_voxel_image(pvd_data_u8_slice, size);

        let mut buffer = GreedyQuadsBuffer::new(plain_voxel_data.data.len());

        lumal::trace!();
        // TODO: issue on block_mesh bad readme example
        let chunk_shape = block_mesh::ndshape::RuntimeShape::<u32, 3>::new([
            size.x + 2,
            size.y + 2,
            size.z + 2,
        ]);
            
        lumal::trace!();
        let faces = block_mesh::RIGHT_HANDED_Y_UP_CONFIG.faces;
        greedy_quads(
            &plain_voxel_data.data.as_slice(),
            &chunk_shape,
            [0; 3],
            [size.x+1, size.y+1, size.z+1],
            &faces,
            &mut buffer
        );
        lumal::trace!();

        assert!(buffer.quads.num_quads() > 0);

        let num_indices = buffer.quads.num_quads() * 6;
        let num_vertices = buffer.quads.num_quads() * 4;
        // [0,1,2] [1,2,3] - indices of vertices in vertex array
        // each sequential three indices form a (single)triangle
        // triangles are made by mesher (block_mesh) from voxels
        let mut indices = Vec::with_capacity(num_indices);
        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        
        lumal::trace!();
        // problem with block_mesh is that even tho it is voxel, values are still in floats
        // so for now we repack & convert them
        // TODO: fork, fix and optimize
        for (group, face) in buffer.quads.groups.into_iter().zip(faces.into_iter()) {
            for quad in group.into_iter() {
                indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                positions.extend_from_slice(&face.quad_mesh_positions(&quad.into(), 1.0));
                normals.extend_from_slice(&face.quad_mesh_normals());
            }
        }
        lumal::trace!();

        assert!(positions.len() == normals.len());
        // positions only!
        // normals are passed as push constants and defined in high-level (look down below)
        let mut circ_verts = vec![PackedVoxelCircuit::default(); positions.len()];
        for i in 0..positions.len() {
            let u8pos = u8vec3::new (
                positions[i][0] as u8,
                positions[i][1] as u8,
                positions[i][2] as u8,
            );
            circ_verts[i].pos = u8pos;
        }

        #[allow(non_snake_case)] let mut verts_Pzz = vec![u16::default(); positions.len()];
        #[allow(non_snake_case)] let mut verts_Nzz = vec![u16::default(); positions.len()];
        #[allow(non_snake_case)] let mut verts_zPz = vec![u16::default(); positions.len()];
        #[allow(non_snake_case)] let mut verts_zNz = vec![u16::default(); positions.len()];
        #[allow(non_snake_case)] let mut verts_zzP = vec![u16::default(); positions.len()];
        #[allow(non_snake_case)] let mut verts_zzN = vec![u16::default(); positions.len()];

        // TODO: how to return a ref to local_but_higher_scope variable?
        let mut push_index_to_corresponding_vec = |normal: vec3, index: u16| {
            match normal {
                vec3 {x: 1.0, y: 0.0, z: 0.0} => {verts_Pzz.push(index);},
                vec3 {x: -1.0, y: 0.0, z: 0.0} => {verts_Nzz.push(index);},
                vec3 {x: 0.0, y: 1.0, z: 0.0} => {verts_zPz.push(index);},
                vec3 {x: 0.0, y: -1.0, z: 0.0} => {verts_zNz.push(index);},
                vec3 {x: 0.0, y: 0.0, z: 1.0} => {verts_zzP.push(index);},
                vec3 {x: 0.0, y: 0.0, z: -1.0} => {verts_zzN.push(index);},
                _ => {
                    dbg!(normal);
                    panic!("Unknown normal");
                },
            }
        };

        for i in 0..indices.len() {
            let index = indices[i];
            // the first one. This is the one that points to vertex that is Provoking Vertex (google it)
            let provoking_index = indices[(i / 3) * 3];
            // TODO: should i checks that they all actualyl have same normal?
            let norm = normals[provoking_index as usize];
            push_index_to_corresponding_vec(norm.into(), index as u16);
        }

        assert!(verts_Pzz.len() != 0);
        assert!(verts_Nzz.len() != 0);
        assert!(verts_zPz.len() != 0);
        assert!(verts_zNz.len() != 0);
        assert!(verts_zzP.len() != 0);
        assert!(verts_zzN.len() != 0);

        let mut triangles = FaceBuffers::default();

        let mut all_indices = vec![];
        let mut offset_and_insert = |vec: &mut Vec<u16>, section: &mut IndexedVertices| {
            section.offset = vec.len() as u32;
            section.icount = vec.len() as u32;
            all_indices.extend_from_slice(vec.as_slice());
        };

        offset_and_insert(&mut verts_Pzz, &mut triangles.Pzz);
        offset_and_insert(&mut verts_Nzz, &mut triangles.Nzz);
        offset_and_insert(&mut verts_zPz, &mut triangles.zPz);
        offset_and_insert(&mut verts_zNz, &mut triangles.zNz);
        offset_and_insert(&mut verts_zzP, &mut triangles.zzP);
        offset_and_insert(&mut verts_zzN, &mut triangles.zzN);

        triangles.vertexes = self.lumal.create_elem_buffer::<PackedVoxelCircuit>(
            &circ_verts,
            vk::BufferUsageFlags::TRANSFER_DST // trans_dst needed internally but specified explicitly 
            | vk::BufferUsageFlags::VERTEX_BUFFER,
        );
        triangles.indices = self.lumal.create_elem_buffer::<u16>(
            &all_indices,
            vk::BufferUsageFlags::TRANSFER_DST // trans_dst needed internally but specified explicitly 
            | vk::BufferUsageFlags::VERTEX_BUFFER,
        );

        return InternalMeshModel {
            triangles,
            voxels,
            size,
        };
    }

    pub fn free_mesh(&mut self, mesh: InternalMeshModel) {
        assert!(mesh.triangles.vertexes.buffer != vk::Buffer::null());
        assert!(mesh.triangles.indices.buffer != vk::Buffer::null());
        assert!(mesh.voxels.image != vk::Image::null());

        self.lumal.buffer_deletion_queue.push(BufferDeletion {
            buffer: mesh.triangles.vertexes,
            lifetime: FRAMES_IN_FLIGHT as i32,
        });
        self.lumal.buffer_deletion_queue.push(BufferDeletion {
            buffer: mesh.triangles.indices,
            lifetime: FRAMES_IN_FLIGHT as i32,
        });

        self.lumal.image_deletion_queue.push(ImageDeletion {
            image: mesh.voxels,
            lifetime: FRAMES_IN_FLIGHT as i32,
        });
    }
}

impl super::InternalRenderer {
    //TODO: runtime copies in single copy command buffer instead of per-model cmb creation
    pub fn create_rayrace_voxel_image(
        &mut self,
        voxels: &[Voxel],
        size: uvec3,
    ) -> lumal::Image {
        let buffer_size = size.x * size.y * size.z;
        assert_eq!(voxels.len(), ((size.x+2) * (size.y+2) * (size.z+2)) as usize);

        let mut voxel_image = self
            .lumal
            .create_image(
                vk::ImageType::_3D,
                vk::Format::R8_UINT,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
                vulkanalia_vma::MemoryUsage::AutoPreferDevice,
                vulkanalia_vma::AllocationCreateFlags::empty(),
                vk::ImageAspectFlags::COLOR,
                uvec3_to_extent3d(size),
                1,
                vk::SampleCountFlags::_1,
            )
            .unwrap();

        self.lumal.transition_image_layout_single_time(
            &voxel_image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
        );

        let staging_buffer = self.lumal.create_buffer(
            vk::BufferUsageFlags::TRANSFER_SRC,
            buffer_size.try_into().unwrap(),
            true,
        );

        unsafe {
            std::ptr::copy_nonoverlapping(
                voxels.as_ptr(),
                staging_buffer.mapped.unwrap() as *mut Voxel,
                buffer_size.try_into().unwrap(),
            );
        };
        lumal::trace!();

        self.lumal.copy_buffer_to_image_single_time(staging_buffer.buffer, &voxel_image, uvec3_to_extent3d(size));

        lumal::trace!();
        self.lumal.destroy_buffer(&staging_buffer);

        voxel_image
    }
    
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct VoxelForContour(pub Voxel);

impl block_mesh::Voxel for VoxelForContour {
    fn get_visibility(&self) -> VoxelVisibility {
        if self.0 == 0 {
            VoxelVisibility::Empty
        } else {
            VoxelVisibility::Opaque
        } // never transluent
    }
}

impl block_mesh::MergeVoxel for VoxelForContour {
    type MergeValue = Self;

    fn merge_value(&self) -> Self::MergeValue {
        // we only care about contour, thus if not emtpy, merging is allowed
        return match self.0 {
            0 => VoxelForContour(0),
            _ => VoxelForContour(1),
        }
    }
}
