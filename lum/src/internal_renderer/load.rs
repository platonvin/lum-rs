use block_mesh::{greedy_quads, GreedyQuadsBuffer, VoxelVisibility};
use internal_renderer::*;
use lumal::{atrace, BufferDeletion, Image, ImageDeletion};
use qvek::vek::Vec3;
// use rand::Rng;
use vulkanalia::vk::{self, Handle};

use crate::{types::*, *};

fn from_addr<'b, T>(address: *const T) -> &'b T {
    unsafe { &*address }
}

impl super::InternalRenderer {
    // Palette on CPU side is (should) be represented as a POD array
    // Palette on GPU side is stored differently (in 2d array of 3d blocks). This is
    // due to perfomance win + hw limitations E.g. just doing 16*len x 16 x 16
    // will not work cause 16xlen will be too big size for some gpus
    #[cold]
    #[optimize(speed)]
    pub fn update_block_palette_to_gpu(&mut self) {
        assert!(self.block_palette_voxels.len() == self.static_block_palette_size as usize);
        // create 3d array to be copied to gpu-side image after it is filled
        let mut block_palette_prepared = Array3D::<Voxel>::new_filled(
            (16 * BLOCK_PALETTE_SIZE_X) as usize,
            (16 * BLOCK_PALETTE_SIZE_Y) as usize,
            16,
            0 as Voxel,
        );

        for (i, block) in self.block_palette_voxels.iter().enumerate() {
            let block_xy = self.index_block_xy(i);
            for_zyx!(16, 16, 16, |x, y, z| {
                #[allow(clippy::unnecessary_cast)]
                let vox = block[x as usize][y as usize][z as usize];
                block_palette_prepared[(
                    x + ((block_xy.x as usize) * 16),
                    y + ((block_xy.y as usize) * 16),
                    z,
                )] = vox;
            });
        }

        #[rustfmt::skip]
        let buffer_count = block_palette_prepared.dimensions().0
                         * block_palette_prepared.dimensions().1
                         * block_palette_prepared.dimensions().2;
        let buffer_size = buffer_count * std::mem::size_of::<Voxel>();

        let staging_buffer =
            self.lumal.create_buffer(vk::BufferUsageFlags::TRANSFER_SRC, buffer_size, true);

        unsafe {
            std::ptr::copy_nonoverlapping(
                block_palette_prepared.data.as_ptr(),
                staging_buffer.mapped.unwrap() as *mut Voxel,
                buffer_count,
            );
        };

        unsafe {
            self.lumal
                .allocator
                .as_ref()
                .unwrap()
                .flush_allocation(staging_buffer.allocation, 0, buffer_size as u64)
                .unwrap()
        };

        for block_palette in self.independent_images.origin_block_palette.iter() {
            assert!(block_palette_prepared.dimensions().0 == block_palette.extent.width as usize);
            assert!(block_palette_prepared.dimensions().1 == block_palette.extent.height as usize);
            assert!(block_palette_prepared.dimensions().2 == block_palette.extent.depth as usize);
            self.lumal.copy_buffer_to_image_single_time(
                staging_buffer.buffer,
                block_palette,
                vk::Extent3D {
                    width: block_palette_prepared.dimensions().0 as u32,
                    height: block_palette_prepared.dimensions().1 as u32,
                    depth: block_palette_prepared.dimensions().2 as u32,
                },
            );
        }

        self.lumal.destroy_buffer(staging_buffer);
    }

    #[cold]
    #[optimize(speed)]
    pub fn update_material_palette_to_gpu(&mut self) {
        // we do not write it to intermediate buffer cuz its already in right layout - 6
        // float rows one by one 256 total
        assert!(!self.material_palette.is_empty());
        // dbg!(&self.material_palette);
        dbg!(&self.material_palette.len());
        let buffer_count = self.material_palette.len();
        let buffer_size = buffer_count * std::mem::size_of::<Material>();

        // dbg!(&self.material_palette);

        let staging_buffer =
            self.lumal.create_buffer(vk::BufferUsageFlags::TRANSFER_SRC, buffer_size, true);

        unsafe {
            std::ptr::copy_nonoverlapping(
                self.material_palette.as_ptr(),
                staging_buffer.mapped.unwrap() as *mut Material,
                buffer_count,
            );
        }

        for palette in self.independent_images.material_palette.iter() {
            self.lumal.copy_buffer_to_image_single_time(
                staging_buffer.buffer,
                palette,
                vk::Extent3D {
                    width: 6, // yep this is how it works for now
                    height: self.material_palette.len() as u32,
                    depth: 1,
                },
            );
        }

        self.lumal.destroy_buffer(staging_buffer);
    }

    #[cold]
    #[optimize(size)]
    pub fn extract_palette_from_scene(&mut self, scene: &ogt_vox::VoxScene) {
        for i in 0..scene.materials.matl.len() {
            self.material_palette[i].albedo = vec3::new(
                scene.palette.color[i].x as f32 / 255.0,
                scene.palette.color[i].y as f32 / 255.0,
                scene.palette.color[i].z as f32 / 255.0,
            );
            self.material_palette[i].transparency = scene.palette.color[i].w as f32 / 255.0;
            self.material_palette[i].emmitness = 0.0;
            self.material_palette[i].roughness = 0.0;

            match scene.materials.matl[i].type_ {
                ogt_vox::MatlType::Diffuse => {
                    self.material_palette[i].emmitness = 0.0;
                    self.material_palette[i].roughness = 1.0;
                }
                ogt_vox::MatlType::Emit => {
                    self.material_palette[i].emmitness =
                        scene.materials.matl[i].emit * (2.0 + scene.materials.matl[i].flux * 4.0);
                    self.material_palette[i].roughness = 0.5;
                }
                ogt_vox::MatlType::Metal => {
                    self.material_palette[i].emmitness = 0.0;
                    self.material_palette[i].roughness =
                        scene.materials.matl[i].rough + (1.0 - scene.materials.matl[i].metal) / 2.0;
                }
                _ => {
                    dbg!("Unknown material type");
                }
            }
        }
    }

    #[cold]
    #[optimize(size)]
    pub fn load_mesh_from_file(
        &mut self,
        mesh_file: &str,
        make_vertices: bool,
        extrude_palette: bool,
    ) -> InternalMeshModel {
        atrace!();
        let scene = ogt_vox::read_scene_from_file(mesh_file).unwrap();
        assert!(scene.models.len() == 1); // only one model per file supported for now
        let model = &scene.models[0];
        assert!(model.size_x > 0 && model.size_y > 0 && model.size_z > 0);
        atrace!();

        if extrude_palette && !self.has_palette {
            println!("Extruding palette");
            self.extract_palette_from_scene(&scene);
            self.has_palette = true;
        }
        atrace!();

        self.load_mesh_from_memory(model, true)
    }

    #[cold]
    #[optimize(size)]
    pub fn load_meshes_from_file(
        &mut self,
        meshes_file: &str,
        make_vertices: bool,
        extrude_palette: bool,
    ) -> Vec<InternalMeshModel> {
        let scene = ogt_vox::read_scene_from_file(meshes_file).unwrap();

        if extrude_palette && !self.has_palette {
            println!("Extruding palette");
            self.extract_palette_from_scene(&scene);
            self.has_palette = true;
        }

        scene
            .models
            .iter()
            .enumerate()
            .map(|(i, model)| {
                assert!(model.size_x > 0 && model.size_y > 0 && model.size_z > 0);

                self.load_mesh_from_memory(model, true)
            })
            .collect()
    }

    #[cold]
    #[optimize(size)]
    pub fn load_mesh_from_memory(
        &mut self,
        model: &ogt_vox::VoxModel,
        make_vertices: bool,
    ) -> InternalMeshModel {
        let size = uvec3 {
            x: model.size_x,
            y: model.size_y,
            z: model.size_z,
        };

        let mut padded_voxel_data = Array3D::<VoxelForContour>::new(
            // +2 cause padding of 1 from each side
            (size.x + 2) as usize,
            (size.y + 2) as usize,
            (size.z + 2) as usize,
        );
        padded_voxel_data.data.fill(VoxelForContour(0));

        for xx in 0..size.x {
            for yy in 0..size.y {
                for zz in 0..size.z {
                    let voxel =
                        model.voxel_data[(xx + yy * size.x + zz * size.x * size.y) as usize];
                    // some padding for generator
                    padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)] =
                        VoxelForContour(voxel);
                }
            }
        }

        let pvd_data_slice = unsafe {
            std::slice::from_raw_parts(
                model.voxel_data.as_ptr() as *const Voxel,
                (size.x * size.y * size.z) as usize,
            )
        };

        let voxels = self.create_rayrace_voxel_image(
            pvd_data_slice,
            size,
            #[cfg(feature = "debug_validation_names")]
            Some("Mesh Voxels"),
        );

        let triangles = self.make_contour_vertices(size, padded_voxel_data);

        InternalMeshModel {
            triangles,
            voxels,
            total_size: size,
            // sprites: vec![],
        }
    }

    #[cold]
    #[optimize(size)]
    pub fn free_block(&mut self, block: BlockId) {
        let block_mesh = std::mem::take(&mut self.block_palette_meshes[block as usize]);

        assert!(block_mesh.triangles.vertexes.buffer != vk::Buffer::null());
        assert!(block_mesh.triangles.indices.buffer != vk::Buffer::null());

        self.lumal.buffer_deletion_queue.push(BufferDeletion {
            buffer: block_mesh.triangles.vertexes,
            lifetime: FRAMES_IN_FLIGHT as i32,
        });
        self.lumal.buffer_deletion_queue.push(BufferDeletion {
            buffer: block_mesh.triangles.indices,
            lifetime: FRAMES_IN_FLIGHT as i32,
        });
    }

    #[cold]
    #[optimize(size)]
    pub fn load_block_from_file(&mut self, block: BlockId, path: &str) {
        // let scene_data = std::fs::read(path).unwrap();
        let scene = ogt_vox::read_scene_from_file(path).unwrap(); // TODO: handle error
                                                                  // let scene = from_addr(scene);
                                                                  // assert!(scene.num_models == 1); // only one model per file supported for now
                                                                  // blocks are always 16x16x16
        let model = &scene.models[0];
        assert!(model.size_x == 16 && model.size_y == 16 && model.size_z == 16);
        self.load_block_from_memory(block, model);
    }

    #[cold]
    #[optimize(size)]
    pub fn load_block_from_memory(&mut self, block_id: BlockId, model: &ogt_vox::VoxModel) {
        let size = uvec3::new(model.size_x, model.size_y, model.size_z);

        let mut padded_voxel_data = Array3D::<VoxelForContour>::new(
            // +2 cause padding of 1 from each side
            (size.x + 2) as usize,
            (size.y + 2) as usize,
            (size.z + 2) as usize,
        );
        padded_voxel_data.data.fill(VoxelForContour(0));

        for xx in 0..size.x {
            for yy in 0..size.y {
                for zz in 0..size.z {
                    let voxel =
                        model.voxel_data[(xx + yy * size.x + zz * size.x * size.y) as usize];
                    // some padding for generator
                    padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)] =
                        VoxelForContour(voxel);
                }
            }
        }

        // yep, there is padding. Its to reuse memory. TODO: find nicer approach
        assert!(size.x == 16 && size.y == 16 && size.z == 16);
        for_zyx!(size, |xx, yy, zz| {
            self.block_palette_voxels[block_id as usize][xx as usize][yy as usize][zz as usize] =
                padded_voxel_data[((xx + 1) as usize, (yy + 1) as usize, (zz + 1) as usize)].0;
        });

        let triangles = self.make_contour_vertices(size, padded_voxel_data);

        self.block_palette_meshes[block_id as usize] = InternalMeshBlock { triangles };
    }

    #[cold]
    #[optimize(size)]
    pub fn make_contour_vertices(
        &mut self,
        // real size. TODO: do i need this?
        size: Vec3<u32>,
        // 3d array with 1 padding
        padded_voxel_data: Array3D<VoxelForContour>,
    ) -> FaceBuffers {
        let mut buffer = GreedyQuadsBuffer::new(padded_voxel_data.data.len());

        lumal::trace!();
        // TODO: issue on block_mesh bad readme example
        let chunk_shape =
            block_mesh::ndshape::RuntimeShape::<u32, 3>::new([size.x + 2, size.y + 2, size.z + 2]);

        lumal::trace!();
        let faces = block_mesh::RIGHT_HANDED_Y_UP_CONFIG.faces;
        greedy_quads(
            padded_voxel_data.data.as_slice(),
            &chunk_shape,
            [0; 3],
            [size.x + 1, size.y + 1, size.z + 1],
            &faces,
            &mut buffer,
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
        // problem with block_mesh is that even tho it is voxel, values are still in
        // floats so for now we repack & convert them
        // TODO: fork, fix and optimize
        for (group, face) in buffer.quads.groups.into_iter().zip(faces.into_iter()) {
            for quad in group.into_iter() {
                indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                positions.extend_from_slice(&face.quad_mesh_positions(&quad, 1.0));
                normals.extend_from_slice(&face.quad_mesh_normals());
            }
        }
        lumal::trace!();

        assert!(positions.len() == normals.len());
        // positions only!
        // normals are passed as push constants and defined in high-level (look down
        // below)
        let mut circ_verts = vec![PackedVoxelCircuit::default(); positions.len()];
        for i in 0..positions.len() {
            let u8pos = u8vec3::new(
                // substract 1 cause contour 1 padding
                positions[i][0] as u8 - 1,
                positions[i][1] as u8 - 1,
                positions[i][2] as u8 - 1,
            );
            circ_verts[i].pos = u8pos;
        }

        #[rustfmt::skip] #[allow(non_snake_case)] let mut verts_idxs_Pzz = Vec::with_capacity(positions.len());
        #[rustfmt::skip] #[allow(non_snake_case)] let mut verts_idxs_Nzz = Vec::with_capacity(positions.len());
        #[rustfmt::skip] #[allow(non_snake_case)] let mut verts_idxs_zPz = Vec::with_capacity(positions.len());
        #[rustfmt::skip] #[allow(non_snake_case)] let mut verts_idxs_zNz = Vec::with_capacity(positions.len());
        #[rustfmt::skip] #[allow(non_snake_case)] let mut verts_idxs_zzP = Vec::with_capacity(positions.len());
        #[rustfmt::skip] #[allow(non_snake_case)] let mut verts_idxs_zzN = Vec::with_capacity(positions.len());

        // TODO: how to return a ref to local_but_higher_scope variable?
        #[rustfmt::skip]
        let mut push_index_to_corresponding_vec = |normal: vec3, index: u16| {
            match normal {
                vec3 {x:  1.0, y:  0.0, z:  0.0} => {verts_idxs_Pzz.push(index);},
                vec3 {x: -1.0, y:  0.0, z:  0.0} => {verts_idxs_Nzz.push(index);},
                vec3 {x:  0.0, y:  1.0, z:  0.0} => {verts_idxs_zPz.push(index);},
                vec3 {x:  0.0, y: -1.0, z:  0.0} => {verts_idxs_zNz.push(index);},
                vec3 {x:  0.0, y:  0.0, z:  1.0} => {verts_idxs_zzP.push(index);},
                vec3 {x:  0.0, y:  0.0, z: -1.0} => {verts_idxs_zzN.push(index);},
                _ => {
                    dbg!(normal);
                    panic!("Unknown normal");
                },
            }
        };
        // dbg!(&indices);
        for i in 0..indices.len() {
            let index = indices[i];
            // the first one in triangle. This is the one that points to vertex that is the
            // Provoking Vertex (google it) which means that when all 3 pass
            // some some value to fragment shader with flat qualifier (no interpolation),
            // Provoking Vertex's one is used
            let provoking_index = indices[(i / 3) * 3];
            // TODO: should i checks that they all actualyl have same normal?
            let norm = normals[provoking_index as usize];
            push_index_to_corresponding_vec(norm.into(), index as u16);
        }

        assert!(!verts_idxs_Pzz.is_empty());
        assert!(!verts_idxs_Nzz.is_empty());
        assert!(!verts_idxs_zPz.is_empty());
        assert!(!verts_idxs_zNz.is_empty());
        assert!(!verts_idxs_zzP.is_empty());
        assert!(!verts_idxs_zzN.is_empty());

        let mut triangles = FaceBuffers::default();

        let mut all_indices = vec![];
        let mut offset_and_insert = |vec: &mut Vec<u16>, section: &mut IndexedVertices| {
            // starts at current length
            section.offset = all_indices.len() as u32;
            // continues for length of verts_idxs vec
            section.icount = vec.len() as u32;
            all_indices.extend_from_slice(vec.as_slice());
        };

        offset_and_insert(&mut verts_idxs_Pzz, &mut triangles.Pzz);
        offset_and_insert(&mut verts_idxs_Nzz, &mut triangles.Nzz);
        offset_and_insert(&mut verts_idxs_zPz, &mut triangles.zPz);
        offset_and_insert(&mut verts_idxs_zNz, &mut triangles.zNz);
        offset_and_insert(&mut verts_idxs_zzP, &mut triangles.zzP);
        offset_and_insert(&mut verts_idxs_zzN, &mut triangles.zzN);

        triangles.vertexes = self.lumal.create_and_upload_buffer::<PackedVoxelCircuit>(
            &circ_verts,
            vk::BufferUsageFlags::TRANSFER_DST // trans_dst needed internally but specified explicitly 
            | vk::BufferUsageFlags::VERTEX_BUFFER,
        );
        triangles.indices = self.lumal.create_and_upload_buffer::<u16>(
            &all_indices,
            vk::BufferUsageFlags::TRANSFER_DST // trans_dst needed internally but specified explicitly 
            | vk::BufferUsageFlags::INDEX_BUFFER,
        );
        triangles
    }

    #[cold]
    #[optimize(size)]
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
    // TODO: runtime copies in single copy command buffer instead of per-model cmb
    // creation
    #[cold]
    #[optimize(size)]
    pub fn create_rayrace_voxel_image(
        &mut self,
        voxels: &[Voxel],
        size: uvec3,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> Image {
        let buffer_count = size.x * size.y * size.z;
        let buffer_size = buffer_count * std::mem::size_of::<Voxel>() as u32;
        assert_eq!(voxels.len(), ((size.x) * (size.y) * (size.z)) as usize);

        let mut voxel_image = self.lumal.create_image(
            vk::ImageType::_3D,
            vk::Format::R8_UINT,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(size),
            1,
            vk::SampleCountFlags::_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Rayrace Voxels"),
        );

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
                buffer_count.try_into().unwrap(),
            );
        };
        lumal::trace!();

        self.lumal.copy_buffer_to_image_single_time(
            staging_buffer.buffer,
            &voxel_image,
            uvec3_to_extent3d(size),
        );

        lumal::trace!();
        self.lumal.destroy_buffer(staging_buffer);

        voxel_image
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct VoxelForContour(pub Voxel);

impl block_mesh::Voxel for VoxelForContour {
    #[cold]
    #[optimize(size)]
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

    #[cold]
    #[optimize(size)]
    fn merge_value(&self) -> Self::MergeValue {
        // we only care about contour, thus if not emtpy, merging is allowed
        match self.0 {
            0 => VoxelForContour(0),
            _ => VoxelForContour(1),
        }
    }
}
