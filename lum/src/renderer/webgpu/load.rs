use crate::renderer::types::*;
use crate::{
    containers::Array3D,
    renderer::webgpu::types::*,
    renderer::{
        load_interface::LoadInterface,
        webgpu::{BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y, BLOCK_SIZE},
    },
    *,
};
use block_mesh::{greedy_quads, GreedyQuadsBuffer};
use qvek::vec3;
use renderer::*;
use wgpu::BufferUsages;
use wgpu::{
    util::DeviceExt, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Extent3d, Origin3d, ShaderStages, TexelCopyBufferLayout,
    TexelCopyTextureInfo,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PackedVoxelCircuit {
    pub pos: u8vec4,
}

use super::{
    wal::{self, Image},
    InternalRendererWebGPU,
};

// impl InternalRendererVulkan {}

impl<'window> LoadInterface for InternalRendererWebGPU<'window> {
    type BufferType = Option<wgpu::Buffer>;
    type ImageType = Option<wal::Image>;
    type BlockId = BlockId;
    type MatId = MatId;
    type Voxel = Voxel;
    type IndexedVertices = IndexedVerticesQueue;

    // Palette on CPU side is (should) be represented as a POD array
    // Palette on GPU side is stored differently (in 2d array of 3d blocks). This is
    // due to perfomance win + hw limitations E.g. just doing 16*len x 16 x 16
    // will not work cause 16xlen will be too big size for some gpus

    fn update_block_palette_to_gpu(&mut self) {
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

        let data_u8 = unsafe {
            std::slice::from_raw_parts(
                block_palette_prepared.data.as_ptr() as *const u8,
                buffer_size,
            )
        };
        for bp in self.independent_images.block_palette.iter() {
            self.wal.queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &bp.texture,
                    mip_level: 0,
                    origin: Origin3d { x: 0, y: 0, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                data_u8,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        BLOCK_PALETTE_SIZE_X * BLOCK_SIZE * std::mem::size_of::<Voxel>() as u32,
                    ),
                    rows_per_image: Some(BLOCK_PALETTE_SIZE_Y * BLOCK_SIZE),
                },
                Extent3d {
                    width: BLOCK_PALETTE_SIZE_X * BLOCK_SIZE,
                    height: BLOCK_PALETTE_SIZE_Y * BLOCK_SIZE,
                    depth_or_array_layers: BLOCK_SIZE,
                },
            );
            self.wal.queue.submit([]);
        }
    }

    fn update_material_palette_to_gpu(&mut self) {
        // we do not write it to intermediate buffer cuz its already in right layout - 6
        // float rows one by one 256 total
        assert!(!self.material_palette.is_empty());
        assert_eq!(self.material_palette.len(), 256);

        const _: () = assert!(size_of::<Material>() == size_of::<f32>() * 6);

        dbg!(&self.material_palette.len());
        let buffer_count = self.material_palette.len();
        // let buffer_count = 256;
        let buffer_size = buffer_count * std::mem::size_of::<Material>();

        let data_u8 = unsafe {
            std::slice::from_raw_parts(self.material_palette.as_ptr() as *const u8, buffer_size)
        };

        self.wal.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &self.independent_images.material_palette.texture,
                mip_level: 0,
                origin: Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            data_u8,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size_of::<Material>() as u32),
                rows_per_image: None,
            },
            Extent3d {
                width: 6,
                height: 256,
                depth_or_array_layers: 1,
            },
        );
        self.wal.queue.submit([]);
    }

    #[cold]
    #[optimize(size)]
    fn load_mesh_from_memory(
        &mut self,
        model: &ogt_vox::VoxModel,
        _make_vertices: bool,
    ) -> InternalMeshModel<Self::BufferType, Self::ImageType, IndexedVerticesQueue> {
        let size = uvec3 {
            x: model.size_x,
            y: model.size_y,
            z: model.size_z,
        };

        let mut padded_voxel_data = Array3D::<VoxelForContour<Voxel>>::new(
            // +2 cause padding of 1 from each side
            (size.x + 2) as usize,
            (size.y + 2) as usize,
            (size.z + 2) as usize,
        );
        padded_voxel_data.data.fill(VoxelForContour(0));

        for xx in 0..size.x {
            for yy in 0..size.y {
                for zz in 0..size.z {
                    let voxel = model.voxel_data[(xx + yy * size.x + zz * size.x * size.y) as usize]
                        as Voxel;
                    // some padding for generator
                    padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)] =
                        VoxelForContour(voxel);
                }
            }
        }

        let repacked_voxels = model.voxel_data.iter().map(|v| *v as Voxel).collect::<Vec<_>>();

        let voxels = self.create_rayrace_voxel_image(
            repacked_voxels.as_slice(),
            size,
            #[cfg(feature = "debug_validation_names")]
            Some("Mesh Voxels"),
        );

        let mut triangles = self.make_contour_vertices(size, padded_voxel_data);

        let create_face_bind_group = |face: &mut IndexedVerticesQueue| {
            let dynamic_bind_group = self.wal.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Dynamic per-face Voxels Bind Group"),
                layout: self.pipes.raygen_models_pipe.dynamic_bind_group_layout.as_ref().unwrap(),
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(
                            face.pc_buffer.as_ref().unwrap().as_entire_buffer_binding(),
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &voxels.as_ref().unwrap().view,
                        ),
                    },
                ],
            });
            face.pc_bg = Some(dynamic_bind_group);
        };

        create_face_bind_group(&mut triangles.Pzz);
        create_face_bind_group(&mut triangles.Nzz);
        create_face_bind_group(&mut triangles.zPz);
        create_face_bind_group(&mut triangles.zNz);
        create_face_bind_group(&mut triangles.zzP);
        create_face_bind_group(&mut triangles.zzN);

        let compute_pc_buffer = self.wal.create_buffer(
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            16 * 1024 * 20,
            Some("(per-mesh) pc buffer for compute"),
        );

        // Per-model Bind Group of dynamic resources.
        // So we can (and should) use dynamic bind group layout we declared earlier, stored in Pipe.
        let compute_dynamic_bind_group = self.wal.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Dynamic per-Mesh Voxels Bind Group"),
            layout: self.pipes.map_pipe.dynamic_bind_group_layout.as_ref().unwrap(),
            // we bind same voxel image and 6 different pc buffers to 6 different bind groups
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        compute_pc_buffer.as_entire_buffer_binding(),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&voxels.as_ref().unwrap().view),
                },
            ],
        });

        InternalMeshModel {
            triangles,
            voxels,
            size,
            compute_pc_buffer: Some(compute_pc_buffer),
            voxels_bind_group_compute: Some(compute_dynamic_bind_group),
            compute_push_constants: vec![],
            compute_pc_count: 0,
        }
    }

    #[cold]
    #[optimize(size)]
    fn create_rayrace_voxel_image(
        &mut self,
        voxels: &[Voxel],
        size: uvec3,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> Self::ImageType {
        let buffer_count = size.x * size.y * size.z;
        let buffer_size = buffer_count * std::mem::size_of::<Voxel>() as u32;
        assert_eq!(voxels.len(), ((size.x) * (size.y) * (size.z)) as usize);

        let data_u8 = unsafe {
            std::slice::from_raw_parts(voxels.as_ptr() as *const u8, buffer_size as usize)
        };

        let texture = self.wal.device.create_texture_with_data(
            &self.wal.queue,
            &wgpu::TextureDescriptor {
                label: Some("Image Ring Texture"),
                size: wgpu::Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: size.z,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::R32Sint,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data_u8,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            // we dont create stencil views
            aspect: if wgpu::TextureFormat::R32Sint.has_depth_aspect() {
                wgpu::TextureAspect::DepthOnly
            } else {
                wgpu::TextureAspect::All
            },
            ..Default::default()
        });
        Some(Image { texture, view })
    }

    #[cold]
    #[optimize(size)]
    fn extract_palette_from_scene(&mut self, scene: &ogt_vox::VoxScene) {
        for i in 0..scene.materials.matl.len() {
            self.material_palette[i].albedo = vec3!(scene.palette.color[i].xyz()) / 255.0;
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
    fn free_block(&mut self, block: BlockId) {
        // leaves None in place and drops the block mesh
        let block_mesh = std::mem::take(&mut self.block_palette_meshes[block as usize]);

        drop(block_mesh);
    }

    #[cold]
    #[optimize(size)]
    fn make_contour_vertices(
        &mut self,
        // real size. TODO: do i need this?
        size: uvec3,
        // 3d array with 1 padding
        padded_voxel_data: Array3D<VoxelForContour<Voxel>>,
    ) -> FaceBuffers<Self::BufferType, IndexedVerticesQueue> {
        let mut buffer = GreedyQuadsBuffer::new(padded_voxel_data.data.len());

        // TODO: issue on block_mesh bad readme example
        let chunk_shape =
            block_mesh::ndshape::RuntimeShape::<u32, 3>::new([size.x + 2, size.y + 2, size.z + 2]);

        let faces = block_mesh::RIGHT_HANDED_Y_UP_CONFIG.faces;
        greedy_quads(
            padded_voxel_data.data.as_slice(),
            &chunk_shape,
            [0; 3],
            [size.x + 1, size.y + 1, size.z + 1],
            &faces,
            &mut buffer,
        );

        assert!(buffer.quads.num_quads() > 0);

        let num_indices = buffer.quads.num_quads() * 6;
        let num_vertices = buffer.quads.num_quads() * 4;
        // [0,1,2] [1,2,3] - indices of vertices in vertex array
        // each sequential three indices form a (single)triangle
        // triangles are made by mesher (block_mesh) from voxels
        let mut indices = Vec::with_capacity(num_indices);
        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);

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

        assert!(positions.len() == normals.len());
        // positions only!
        // normals are passed as push constants and defined in high-level (look down
        // below)
        let mut circ_verts = vec![PackedVoxelCircuit::default(); positions.len()];
        for i in 0..positions.len() {
            let u8pos = u8vec4::new(
                // substract 1 cause contour 1 padding
                positions[i][0] as u8 - 1,
                positions[i][1] as u8 - 1,
                positions[i][2] as u8 - 1,
                0,
            );
            circ_verts[i].pos = u8pos;
        }

        #[allow(non_snake_case)]
        let mut verts_idxs_Pzz = Vec::with_capacity(positions.len());
        #[allow(non_snake_case)]
        let mut verts_idxs_Nzz = Vec::with_capacity(positions.len());
        #[allow(non_snake_case)]
        let mut verts_idxs_zPz = Vec::with_capacity(positions.len());
        #[allow(non_snake_case)]
        let mut verts_idxs_zNz = Vec::with_capacity(positions.len());
        #[allow(non_snake_case)]
        let mut verts_idxs_zzP = Vec::with_capacity(positions.len());
        #[allow(non_snake_case)]
        let mut verts_idxs_zzN = Vec::with_capacity(positions.len());

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
                    panic!("Unknown normal: {normal:?}");
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

        let mut all_indices = vec![];
        let mut offset_and_insert = |vec: &mut Vec<u16>, section: &mut IndexedVertices| {
            // starts at current length
            section.offset = all_indices.len() as u32;
            // continues for length of verts_idxs vec
            section.icount = vec.len() as u32;
            all_indices.extend_from_slice(vec.as_slice());
        };

        #[allow(non_snake_case)]
        {
            let mut triangles_Pzz = IndexedVertices::default();
            let mut triangles_Nzz = IndexedVertices::default();
            let mut triangles_zPz = IndexedVertices::default();
            let mut triangles_zNz = IndexedVertices::default();
            let mut triangles_zzP = IndexedVertices::default();
            let mut triangles_zzN = IndexedVertices::default();

            offset_and_insert(&mut verts_idxs_Pzz, &mut triangles_Pzz);
            offset_and_insert(&mut verts_idxs_Nzz, &mut triangles_Nzz);
            offset_and_insert(&mut verts_idxs_zPz, &mut triangles_zPz);
            offset_and_insert(&mut verts_idxs_zNz, &mut triangles_zNz);
            offset_and_insert(&mut verts_idxs_zzP, &mut triangles_zzP);
            offset_and_insert(&mut verts_idxs_zzN, &mut triangles_zzN);

            let (vertexes, indices) =
                self.create_and_upload_contour_buffers(&circ_verts, &all_indices);

            macro_rules! create_indexed_vertices_queue {
                ($wal:expr, $pc_layout:expr, $triangles:expr, $label_suffix:expr, $buffer_usage:expr, $buffer_size:expr) => {{
                    let buffer = $wal.create_buffer(
                        $buffer_usage,
                        $buffer_size,
                        Some(&format!("PC buffer for {}", $label_suffix)),
                    );
                    IndexedVerticesQueue {
                        iv: $triangles,
                        push_constants: vec![],
                        pc_count: 0,
                        pc_buffer: Some(buffer),
                        // we dont create bind group here cause we want it to also contain Voxel data, which comes into play later. So None for now
                        pc_bg: None,
                    }
                }};
            }

            FaceBuffers {
                Pzz: create_indexed_vertices_queue!(
                    self.wal,
                    &pc_buffer_bind_group_layout,
                    triangles_Pzz,
                    "Pzz",
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20
                ),
                Nzz: create_indexed_vertices_queue!(
                    self.wal,
                    &pc_buffer_bind_group_layout,
                    triangles_Nzz,
                    "Nzz",
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20
                ),
                zPz: create_indexed_vertices_queue!(
                    self.wal,
                    &pc_buffer_bind_group_layout,
                    triangles_zPz,
                    "zPz",
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20
                ),
                zNz: create_indexed_vertices_queue!(
                    self.wal,
                    &pc_buffer_bind_group_layout,
                    triangles_zNz,
                    "zNz",
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20
                ),
                zzP: create_indexed_vertices_queue!(
                    self.wal,
                    &pc_buffer_bind_group_layout,
                    triangles_zzP,
                    "zzP",
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20
                ),
                zzN: create_indexed_vertices_queue!(
                    self.wal,
                    &pc_buffer_bind_group_layout,
                    triangles_zzN,
                    "zzN",
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20
                ),
                vertexes,
                indices,
            }
        }
    }

    #[cold]
    #[optimize(size)]
    fn free_mesh(
        &mut self,
        mesh: InternalMeshModel<Self::BufferType, Self::ImageType, Self::IndexedVertices>,
    ) {
        drop(mesh);
    }

    fn has_palette(&self) -> bool {
        self.has_palette
    }

    fn set_has_palette(&mut self, has_palette: bool) {
        self.has_palette = has_palette;
    }

    fn set_block_palette_voxels(&mut self, block_id: BlockId, pos: uvec3, voxel: Voxel) {
        self.block_palette_voxels[block_id as usize][pos.x as usize][pos.y as usize]
            [pos.z as usize] = voxel;
    }

    fn get_block_palette_voxels(&self, block_id: BlockId, pos: uvec3) -> Voxel {
        self.block_palette_voxels[block_id as usize][pos.x as usize][pos.y as usize][pos.z as usize]
    }

    fn set_block_palette_mesh(
        &mut self,
        block_id: BlockId,
        mesh: InternalMeshBlock<Self::BufferType, Self::IndexedVertices>,
    ) {
        // let mesh: InternalMeshBlock<Option<wgpu::Buffer>, IndexedVerticesQueue<{ 44 }>> =
        //     InternalMeshBlock {
        //         triangles: FaceBuffers {
        //             Pzz: IndexedVerticesQueue {
        //                 iv: mesh.triangles.Pzz,
        //                 push_constants: vec![],
        //             },
        //             Nzz: IndexedVerticesQueue {
        //                 iv: mesh.triangles.Nzz,
        //                 push_constants: vec![],
        //             },
        //             zPz: IndexedVerticesQueue {
        //                 iv: mesh.triangles.zPz,
        //                 push_constants: vec![],
        //             },
        //             zNz: IndexedVerticesQueue {
        //                 iv: mesh.triangles.zNz,
        //                 push_constants: vec![],
        //             },
        //             zzP: IndexedVerticesQueue {
        //                 iv: mesh.triangles.zzP,
        //                 push_constants: vec![],
        //             },
        //             zzN: IndexedVerticesQueue {
        //                 iv: mesh.triangles.zzN,
        //                 push_constants: vec![],
        //             },
        //             vertexes: mesh.triangles.vertexes,
        //             indices: mesh.triangles.indices,
        //         },
        //     };
        self.block_palette_meshes[block_id as usize] = mesh;
    }

    fn get_block_palette_mesh(
        &self,
        block_id: BlockId,
    ) -> &InternalMeshBlock<Self::BufferType, Self::IndexedVertices> {
        &self.block_palette_meshes[block_id as usize]
    }

    // fn load_meshes_from_file(
    //     &mut self,
    //     meshes_file: &str,
    //     _make_vertices: bool,
    //     extrude_palette: bool,
    // ) -> Vec<InternalMeshModel<Self::BufferType, Self::ImageType>> {
    //     let scene = ogt_vox::read_scene_from_file(meshes_file).unwrap();

    //     if extrude_palette && !self.has_palette() {
    //         std::println!("Extruding palette_");
    //         self.extract_palette_from_scene(&scene);
    //         std::println!("Extruded palette");
    //         self.set_has_palette(true);
    //     }

    //     scene
    //         .models
    //         .iter()
    //         .map(|model| {
    //             assert!(model.size_x > 0 && model.size_y > 0 && model.size_z > 0);

    //             self.load_mesh_from_memory(model, true)
    //         })
    //         .collect()
    // }

    fn load_block_from_file(&mut self, block: BlockId, path: &str) {
        let scene = ogt_vox::read_scene_from_file(path).unwrap();
        assert!(scene.models.len() == 1); // only one model per file supported for now
                                          // blocks are always 16x16x16
        let model = &scene.models[0];
        assert!(model.size_x == 16 && model.size_y == 16 && model.size_z == 16);
        self.load_block_from_memory(block, model);
    }

    fn load_block_from_memory(&mut self, block_id: BlockId, model: &ogt_vox::VoxModel) {
        let size = uvec3::new(model.size_x, model.size_y, model.size_z);

        let mut padded_voxel_data = Array3D::<VoxelForContour<Voxel>>::new(
            // +2 cause padding of 1 from each side for trianglezation
            (size.x + 2) as usize,
            (size.y + 2) as usize,
            (size.z + 2) as usize,
        );
        padded_voxel_data.data.fill(VoxelForContour(0));

        for xx in 0..size.x {
            for yy in 0..size.y {
                for zz in 0..size.z {
                    let voxel = model.voxel_data[(xx + yy * size.x + zz * size.x * size.y) as usize]
                        as Voxel;
                    // some padding for generator
                    padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)] =
                        VoxelForContour(voxel);
                }
            }
        }

        // yep, there is padding. Its to reuse memory. TODO: find nicer approach
        assert!(size.x == 16 && size.y == 16 && size.z == 16);
        for zz in 0..size.z {
            for yy in 0..size.y {
                for xx in 0..size.x {
                    self.set_block_palette_voxels(
                        block_id,
                        uvec3::new(xx, yy, zz),
                        padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)].0,
                    );
                }
            }
        }

        let mut triangles = self.make_contour_vertices(size, padded_voxel_data);

        // TODO: reuse this
        let raster_dynamic_bind_group_layout =
            self.wal.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Dynamic per-Mesh Voxels Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let create_face_bind_group = |face: &mut IndexedVerticesQueue| {
            let dynamic_bind_group = self.wal.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Dynamic per-face Voxels Bind Group"),
                layout: &raster_dynamic_bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        face.pc_buffer.as_ref().unwrap().as_entire_buffer_binding(),
                    ),
                }],
            });
            face.pc_bg = Some(dynamic_bind_group);
        };

        create_face_bind_group(&mut triangles.Pzz);
        create_face_bind_group(&mut triangles.Nzz);
        create_face_bind_group(&mut triangles.zPz);
        create_face_bind_group(&mut triangles.zNz);
        create_face_bind_group(&mut triangles.zzP);
        create_face_bind_group(&mut triangles.zzN);

        self.set_block_palette_mesh(block_id, InternalMeshBlock { triangles });
    }
}

impl InternalRendererWebGPU<'_> {
    #[cold]
    #[optimize(size)]
    fn create_and_upload_contour_buffers(
        &mut self,
        verts: &[PackedVoxelCircuit],
        indices: &[u16],
    ) -> (Option<wgpu::Buffer>, Option<wgpu::Buffer>) {
        let vertexes = self
            .wal
            .create_and_upload_buffer::<PackedVoxelCircuit>(verts, wgpu::BufferUsages::VERTEX);
        let indices = self.wal.create_and_upload_buffer::<u16>(indices, wgpu::BufferUsages::INDEX);
        (Some(vertexes), Some(indices))
    }
}
