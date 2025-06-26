use super::{
    wal::{self, Image},
    InternalRendererWebGPU,
};
use crate::{
    for_zyx,
    load_interface::{BlockData, LoadInterface},
    sBLOCK_SIZE,
    webgpu::{BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y},
    BLOCK_SIZE,
};
use crate::{types::*, webgpu::types::*};
use containers::{
    array3d::{ConstDims, Dim3},
    Array3D,
};
use qvek::uvec3;
use wgpu::BufferUsages;
use wgpu::{
    util::DeviceExt, BindGroupDescriptor, BindGroupEntry, Extent3d, Origin3d,
    TexelCopyBufferLayout, TexelCopyTextureInfo,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PackedVoxelCircuit {
    /// 3-component u8 position of vertex in voxels (integer)
    /// 4th component is wasted (no u8vec3 in wgpu), maybe we should put something in it later
    pub pos: u8vec4,
}

type BlockPaletteImageSize = ConstDims<
    { (BLOCK_SIZE * BLOCK_PALETTE_SIZE_X) as usize },
    { (BLOCK_SIZE * BLOCK_PALETTE_SIZE_Y) as usize },
    { BLOCK_SIZE as usize },
>;

impl<'window, D: Dim3> LoadInterface for InternalRendererWebGPU<'window, D> {
    type Buffer = Option<wgpu::Buffer>;
    type Image = Option<wal::Image>;
    type BlockId = MeshBlock;
    type MatId = InternalMatId;
    type Voxel = InternalVoxel;
    type IndexedVertices = IndexedVerticesQueue;
    type InternalMeshModel = InternalMeshModel;
    type InternalMeshBlock = InternalMeshBlock;
    type InternalMeshFoliage = InternalMeshFoliage;
    type InternalMeshLiquid = InternalMeshLiquid;
    type InternalMeshVolumetric = InternalMeshVolumetric;
    type FaceBuffers = FaceBuffers;

    // Palette on CPU side is (should) be represented as a POD array
    // Palette on GPU side is stored differently (in 2d array of 3d blocks). This is
    // due to perfomance win + hw limitations E.g. just doing BLOCK_SIZE*len x BLOCK_SIZE x BLOCK_SIZE
    // will not work cause BLOCK_SIZE x len will be too big size for some gpus (different dimensions have different limits)

    fn update_block_palette_to_gpu(&mut self) {
        assert!(self.block_palette_voxels.len() == self.static_block_palette_size as usize);
        // create 3d array to be copied to gpu-side image after it is filled

        let mut block_palette_prepared =
            Array3D::<InternalVoxel, BlockPaletteImageSize>::new_filled(
                BlockPaletteImageSize {}, // reminds me of C++ constructors LOL
                0 as InternalVoxel,
            );

        for (i, block) in self.block_palette_voxels.iter().enumerate() {
            let block_xy = self.index_block_xy(i);
            for_zyx!(BLOCK_SIZE, BLOCK_SIZE, BLOCK_SIZE, |x, y, z| {
                #[allow(clippy::unnecessary_cast)]
                let vox = block[x as usize][y as usize][z as usize];
                block_palette_prepared[(
                    x + (block_xy.x * BLOCK_SIZE) as usize,
                    y + (block_xy.y * BLOCK_SIZE) as usize,
                    z,
                )] = vox as InternalVoxel;
            });
        }

        #[rustfmt::skip]
        let buffer_count = block_palette_prepared.dimensions().0
                         * block_palette_prepared.dimensions().1
                         * block_palette_prepared.dimensions().2;
        let buffer_size = buffer_count * std::mem::size_of::<InternalVoxel>();

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
                        BLOCK_PALETTE_SIZE_X
                            * BLOCK_SIZE
                            * std::mem::size_of::<InternalVoxel>() as u32,
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

    fn load_model(&mut self, model: crate::load_interface::ModelData) -> Self::InternalMeshModel {
        let size = uvec3!(model.size);

        // `voxels` is not quite ready to be copied to GPU yet. It is still not in correct type (for wgpu), cause GPU Voxel is i32, not u8
        // unlike Vulkan, wgpu has no i8 support... which means work in runtime
        // however, its primary for web, thus saving space is more important
        let repacked_voxels = model.voxels.iter().map(|v| *v as InternalVoxel).collect::<Vec<_>>();

        let voxels = self.create_rayrace_voxel_image(
            &repacked_voxels,
            size,
            #[cfg(feature = "debug_validation_names")]
            Some("Mesh Voxels"),
        );

        let circ_verts: Vec<_> = model
            .vertices
            .iter()
            .map(|&vert| PackedVoxelCircuit {
                pos: vert.pos.into(),
            })
            .collect();

        // let indices: Vec<_> = model
        //     .indices
        //     .iter()
        //     .map(|&index| )
        //     .collect();

        let (vertexes, indices) =
            self.create_and_upload_contour_buffers(&circ_verts, model.indices);

        let mut triangles = FaceBuffers {
            Pzz: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: model.iv.Pzz.offset,
                    icount: model.iv.Pzz.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            Nzz: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: model.iv.Nzz.offset,
                    icount: model.iv.Nzz.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            zPz: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: model.iv.zPz.offset,
                    icount: model.iv.zPz.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            zNz: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: model.iv.zNz.offset,
                    icount: model.iv.zNz.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            zzP: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: model.iv.zzP.offset,
                    icount: model.iv.zzP.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            zzN: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: model.iv.zzN.offset,
                    icount: model.iv.zzN.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            vertexes,
            indices,
        };

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

    fn create_rayrace_voxel_image(
        &mut self,
        voxels: &[InternalVoxel],
        size: uvec3,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> Self::Image {
        let buffer_count = size.x * size.y * size.z;
        let buffer_size = buffer_count * std::mem::size_of::<InternalVoxel>() as u32;
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
    fn free_block(&mut self, block: MeshBlock) {
        let block_mesh = std::mem::take(&mut self.block_palette_meshes[block as usize]);

        drop(block_mesh);
    }

    fn free_model(&mut self, mesh: InternalMeshModel) {
        drop(mesh);
    }

    fn load_block(&mut self, block_id: MeshBlock, block_data: BlockData) {
        let block = &mut self.block_palette_voxels[block_id as usize];
        for_zyx!(
            BLOCK_SIZE,
            BLOCK_SIZE,
            BLOCK_SIZE,
            |x: usize, y: usize, z: usize| {
                block[x][y][z] =
                    block_data.voxels[x + y * sBLOCK_SIZE + z * sBLOCK_SIZE * sBLOCK_SIZE];
            }
        );

        let circ_verts: Vec<_> = block_data
            .vertices
            .iter()
            .map(|&vert| PackedVoxelCircuit {
                pos: vert.pos.into(),
            })
            .collect();

        // let indices: Vec<_> = model
        //     .indices
        //     .iter()
        //     .map(|&index| )
        //     .collect();

        let (vertexes, indices) =
            self.create_and_upload_contour_buffers(&circ_verts, block_data.indices);

        let mut triangles = FaceBuffers {
            Pzz: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: block_data.iv.Pzz.offset,
                    icount: block_data.iv.Pzz.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            Nzz: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: block_data.iv.Nzz.offset,
                    icount: block_data.iv.Nzz.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            zPz: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: block_data.iv.zPz.offset,
                    icount: block_data.iv.zPz.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            zNz: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: block_data.iv.zNz.offset,
                    icount: block_data.iv.zNz.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            zzP: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: block_data.iv.zzP.offset,
                    icount: block_data.iv.zzP.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            zzN: IndexedVerticesQueue {
                iv: IndexedVertices {
                    offset: block_data.iv.zzN.offset,
                    icount: block_data.iv.zzN.icount,
                },
                push_constants: vec![],
                pc_count: 0,
                pc_buffer: Some(self.wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    None,
                )),
                pc_bg: None,
            },
            vertexes,
            indices,
        };

        let create_face_bind_group = |face: &mut IndexedVerticesQueue| {
            let dynamic_bind_group = self.wal.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Dynamic per-face Voxels Bind Group"),
                layout: self.pipes.raygen_blocks_pipe.dynamic_bind_group_layout.as_ref().unwrap(),
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

        self.block_palette_meshes[block_id as usize].triangles = triangles;
    }
}

impl<D: Dim3> InternalRendererWebGPU<'_, D> {
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
