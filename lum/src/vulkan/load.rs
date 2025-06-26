use super::InternalRendererVulkan;
use crate::load_interface::{ModelData, SimpleVertex};
use crate::vulkan::types::InternalVoxel;
use crate::vulkan::BLOCK_SIZE;
use crate::{for_zyx, sBLOCK_SIZE, types::*};
use crate::{
    load_interface::LoadInterface,
    vulkan::{types::*, BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y},
};
use containers::array3d::{ConstDims, Dim3};
use containers::*;
use lumal::vk::{self};
use lumal::{vk::MappedMemoryRange, BufferDeletion, ImageDeletion};
use qvek::uvec3;

fn uvec3_to_extent3d(size: uvec3) -> vk::Extent3D {
    vk::Extent3D {
        width: size.x,
        height: size.y,
        depth: size.z,
    }
}

type BlockPaletteImageSize = ConstDims<
    { (BLOCK_SIZE * BLOCK_PALETTE_SIZE_X) as usize },
    { (BLOCK_SIZE * BLOCK_PALETTE_SIZE_Y) as usize },
    { BLOCK_SIZE as usize },
>;

impl<'a, D: Dim3> LoadInterface for InternalRendererVulkan<'a, D> {
    type Buffer = lumal::Buffer;
    type Image = lumal::Image;
    type BlockId = MeshBlock;
    type MatId = MatId;
    type Voxel = InternalVoxel;
    type IndexedVertices = IndexedVertices;
    type InternalMeshModel = InternalMeshModel;
    type InternalMeshBlock = InternalMeshBlock;
    type InternalMeshFoliage = InternalMeshFoliage;
    type InternalMeshLiquid = InternalMeshLiquid;
    type InternalMeshVolumetric = InternalMeshVolumetric;
    type FaceBuffers = FaceBuffers;
    // Palette on CPU side is (should) be represented as a POD array
    // Palette on GPU side is stored differently (in 2d array of 3d blocks). This is
    // due to perfomance win + hw limitations E.g. just doing BLOCK_SIZE*len x BLOCK_SIZE x BLOCK_SIZE
    // will not work cause BLOCK_SIZE*len will be too big size for some gpus

    fn update_block_palette_to_gpu(&mut self) {
        assert!(self.block_palette_voxels.len() == self.static_block_palette_size as usize);
        // create CPU-side 3d array to be copied to GPU-side image after it is filled
        let mut block_palette_prepared =
            Array3D::<InternalVoxel, BlockPaletteImageSize>::new_filled(
                BlockPaletteImageSize {},
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

        // TODO: move to static staging Ring and single buffer
        let staging_buffer =
            self.lumal.create_buffer(vk::BufferUsageFlags::TRANSFER_SRC, buffer_size, true);

        unsafe {
            std::ptr::copy_nonoverlapping(
                block_palette_prepared.data.as_ptr(),
                staging_buffer.allocation.mapped_ptr().unwrap().as_ptr() as *mut InternalVoxel,
                buffer_count,
            );
        };

        unsafe {
            debug_assert!(staging_buffer.allocation.mapped_ptr().is_some());
            self.lumal
                .device
                .flush_mapped_memory_ranges(&[MappedMemoryRange {
                    memory: staging_buffer.allocation.memory(),
                    offset: 0,
                    size: buffer_size as u64,
                    ..Default::default()
                }])
                .unwrap();
        };

        for block_palette in self.independent_images.block_palette.iter() {
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

    fn update_material_palette_to_gpu(&mut self) {
        // we do not write it to intermediate buffer cuz its already in right layout -
        // 6 float rows, one by one, 256 total
        assert!(!self.material_palette.is_empty());
        dbg!(&self.material_palette.len());
        let buffer_count = self.material_palette.len();
        let buffer_size = buffer_count * std::mem::size_of::<Material>();

        let staging_buffer =
            self.lumal.create_buffer(vk::BufferUsageFlags::TRANSFER_SRC, buffer_size, true);

        unsafe {
            std::ptr::copy_nonoverlapping(
                self.material_palette.as_ptr(),
                staging_buffer.allocation.mapped_ptr().unwrap().as_ptr() as *mut Material,
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

    // fn load_mesh(
    //     &mut self,
    //     model: &ogt_vox::VoxModel,
    // 1    _make_vertices: bool,
    // ) -> Self::InternalMeshModel {
    //     let size = uvec3 {
    //         x: model.size_x,
    //         y: model.size_y,
    //         z: model.size_z,
    //     };

    //     let mut padded_voxel_data = Array3D::<VoxelForContour<InternalVoxel>>::new(
    //         // +2 cause padding of 1 from each side
    //         (size.x + 2) as usize,
    //         (size.y + 2) as usize,
    //         (size.z + 2) as usize,
    //     );
    //     padded_voxel_data.data.fill(VoxelForContour(0));

    //     for xx in 0..size.x {
    //         for yy in 0..size.y {
    //             for zz in 0..size.z {
    //                 let voxel = model.voxels[(xx + yy * size.x + zz * size.x * size.y) as usize]
    //                     as InternalVoxel;
    //                 // some padding for generator
    //                 padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)] =
    //                     VoxelForContour(voxel);
    //             }
    //         }
    //     }

    //     let pvd_data_slice = unsafe {
    //         // we dont cast pointers and compiler verifies type for us
    //         std::slice::from_raw_parts(model.voxels.as_ptr(), (size.x * size.y * size.z) as usize)
    //     };

    //     let voxels = self.create_rayrace_voxel_image(
    //         pvd_data_slice,
    //         uvec3!(size),
    //         #[cfg(feature = "debug_validation_names")]
    //         Some("Mesh Voxels"),
    //     );

    //     let triangles = self.make_contour_vertices(size, padded_voxel_data);

    //     InternalMeshModel {
    //         triangles,
    //         voxels,
    //         size,
    //         voxels_bind_group_compute: None,

    //         compute_push_constants: vec![],
    //         compute_pc_buffer: None,
    //         compute_pc_count: 0,
    //     }
    // }

    fn load_model(&mut self, model_data: ModelData) -> Self::InternalMeshModel {
        let size = uvec3!(model_data.size);
        let voxel_data_u8 = unsafe {
            // we dont cast pointers and compiler verifies type for us
            std::slice::from_raw_parts(
                model_data.voxels.as_ptr(),
                (size.x * size.y * size.z) as usize,
            )
        };

        let voxels = self.create_rayrace_voxel_image(
            voxel_data_u8,
            size,
            #[cfg(feature = "debug_validation_names")]
            Some("Mesh Voxels"),
        );

        // TODO:
        debug_assert!(size_of::<SimpleVertex>() == size_of::<PackedVoxelCircuit>());

        let triangles = FaceBuffers {
            Pzz: IndexedVertices {
                offset: model_data.iv.Pzz.offset,
                icount: model_data.iv.Pzz.icount,
            },
            Nzz: IndexedVertices {
                offset: model_data.iv.Nzz.offset,
                icount: model_data.iv.Nzz.icount,
            },
            zPz: IndexedVertices {
                offset: model_data.iv.zPz.offset,
                icount: model_data.iv.zPz.icount,
            },
            zNz: IndexedVertices {
                offset: model_data.iv.zNz.offset,
                icount: model_data.iv.zNz.icount,
            },
            zzP: IndexedVertices {
                offset: model_data.iv.zzP.offset,
                icount: model_data.iv.zzP.icount,
            },
            zzN: IndexedVertices {
                offset: model_data.iv.zzN.offset,
                icount: model_data.iv.zzN.icount,
            },
            vertexes: self
                .lumal
                .create_and_upload_buffer(model_data.vertices, vk::BufferUsageFlags::VERTEX_BUFFER),
            indices: self
                .lumal
                .create_and_upload_buffer(model_data.indices, vk::BufferUsageFlags::INDEX_BUFFER),
        };

        InternalMeshModel {
            triangles,
            voxels,
            size,
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

        let voxel_image = self.lumal.create_image(
            vk::ImageType::TYPE_3D,
            vk::Format::R8_UINT,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED,
            // vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            // vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(size),
            // 1,
            vk::SampleCountFlags::TYPE_1,
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
                staging_buffer.allocation.mapped_ptr().unwrap().as_ptr() as *mut InternalVoxel,
                buffer_count.try_into().unwrap(),
            );
        };

        self.lumal.copy_buffer_to_image_single_time(
            staging_buffer.buffer,
            &voxel_image,
            uvec3_to_extent3d(size),
        );

        self.lumal.destroy_buffer(staging_buffer);

        voxel_image
    }

    // fn extract_palette_from_scene(&mut self, scene: &ogt_vox::VoxScene) {
    //     for i in 0..scene.materials.matl.len() {
    //         self.material_palette[i].albedo = vec3!(scene.palette.color[i].xyz()) / 255.0;
    //         self.material_palette[i].transparency = scene.palette.color[i].w as f32 / 255.0;
    //         self.material_palette[i].emmitness = 0.0;
    //         self.material_palette[i].roughness = 0.0;

    //         match scene.materials.matl[i].type_ {
    //             ogt_vox::MatlType::Diffuse => {
    //                 self.material_palette[i].emmitness = 0.0;
    //                 self.material_palette[i].roughness = 1.0;
    //             }
    //             ogt_vox::MatlType::Emit => {
    //                 self.material_palette[i].emmitness =
    //                     scene.materials.matl[i].emit * (2.0 + scene.materials.matl[i].flux * 4.0);
    //                 self.material_palette[i].roughness = 0.5;
    //             }
    //             ogt_vox::MatlType::Metal => {
    //                 self.material_palette[i].emmitness = 0.0;
    //                 self.material_palette[i].roughness =
    //                     scene.materials.matl[i].rough + (1.0 - scene.materials.matl[i].metal) / 2.0;
    //             }
    //             _ => {
    //                 dbg!("Unknown material type");
    //             }
    //         }
    //     }
    // }

    fn free_block(&mut self, block: MeshBlock) {
        let block_mesh = std::mem::take(&mut self.block_palette_meshes[block as usize]);

        assert!(block_mesh.triangles.vertexes.buffer != vk::Buffer::null());
        assert!(block_mesh.triangles.indices.buffer != vk::Buffer::null());

        self.lumal.buffer_deletion_queue.push(BufferDeletion {
            buffer: block_mesh.triangles.vertexes,
            lifetime: self.lumal.settings.fif as i32,
        });
        self.lumal.buffer_deletion_queue.push(BufferDeletion {
            buffer: block_mesh.triangles.indices,
            lifetime: self.lumal.settings.fif as i32,
        });
    }

    fn free_model(&mut self, mesh: Self::InternalMeshModel) {
        assert!(mesh.triangles.vertexes.buffer != vk::Buffer::null());
        assert!(mesh.triangles.indices.buffer != vk::Buffer::null());
        assert!(mesh.voxels.image != vk::Image::null());

        self.lumal.buffer_deletion_queue.push(BufferDeletion {
            buffer: mesh.triangles.vertexes,
            lifetime: self.lumal.settings.fif as i32,
        });
        self.lumal.buffer_deletion_queue.push(BufferDeletion {
            buffer: mesh.triangles.indices,
            lifetime: self.lumal.settings.fif as i32,
        });

        self.lumal.image_deletion_queue.push(ImageDeletion {
            image: mesh.voxels.image,
            view: mesh.voxels.view,
            allocation: mesh.voxels.allocation,
            // mip_views: mesh.voxels.mip_views,
            lifetime: self.lumal.settings.fif as i32,
        });
    }

    // fn has_palette(&self) -> bool {
    //     self.has_palette
    // }

    // fn set_has_palette(&mut self, has_palette: bool) {
    //     self.has_palette = has_palette;
    // }

    // fn set_block_palette_voxel(&mut self, block_id: MeshBlock, pos: uvec3, voxel: InternalVoxel) {
    //     self.block_palette_voxels[block_id as usize][pos.x as usize][pos.y as usize]
    //         [pos.z as usize] = voxel;
    // }

    // fn get_block_palette_voxel(&self, block_id: MeshBlock, pos: uvec3) -> InternalVoxel {
    //     self.block_palette_voxels[block_id as usize][pos.x as usize][pos.y as usize][pos.z as usize]
    // }

    // fn set_block_palette_mesh(&mut self, block_id: MeshBlock, triangles: Self::FaceBuffers) {
    //     self.block_palette_meshes[block_id as usize].triangles = triangles;
    // }

    // fn get_block_palette_mesh(&self, block_id: MeshBlock) -> &Self::InternalMeshBlock {
    //     &self.block_palette_meshes[block_id as usize]
    // }

    fn load_block(
        &mut self,
        block_id: Self::BlockId,
        block_data: crate::load_interface::BlockData,
    ) {
        debug_assert!(size_of::<SimpleVertex>() == size_of::<PackedVoxelCircuit>());
        // we cant directly copy memory since its in different layout
        // self.block_palette_voxels[block_id as usize] = * /*some cast to BlockVoxels*/ block_data.voxels;
        let block = &mut self.block_palette_voxels[block_id as usize];
        //TODO: get rid of this repacking?
        for_zyx!(
            BLOCK_SIZE,
            BLOCK_SIZE,
            BLOCK_SIZE,
            |x: usize, y: usize, z: usize| {
                block[x][y][z] =
                    block_data.voxels[x + sBLOCK_SIZE * y + sBLOCK_SIZE * sBLOCK_SIZE * z];
            }
        );

        let triangles = FaceBuffers {
            Pzz: IndexedVertices {
                offset: block_data.iv.Pzz.offset,
                icount: block_data.iv.Pzz.icount,
            },
            Nzz: IndexedVertices {
                offset: block_data.iv.Nzz.offset,
                icount: block_data.iv.Nzz.icount,
            },
            zPz: IndexedVertices {
                offset: block_data.iv.zPz.offset,
                icount: block_data.iv.zPz.icount,
            },
            zNz: IndexedVertices {
                offset: block_data.iv.zNz.offset,
                icount: block_data.iv.zNz.icount,
            },
            zzP: IndexedVertices {
                offset: block_data.iv.zzP.offset,
                icount: block_data.iv.zzP.icount,
            },
            zzN: IndexedVertices {
                offset: block_data.iv.zzN.offset,
                icount: block_data.iv.zzN.icount,
            },
            vertexes: self
                .lumal
                .create_and_upload_buffer(block_data.vertices, vk::BufferUsageFlags::VERTEX_BUFFER),
            indices: self
                .lumal
                .create_and_upload_buffer(block_data.indices, vk::BufferUsageFlags::INDEX_BUFFER),
        };
        self.block_palette_meshes[block_id as usize] = InternalMeshBlock { triangles };
    }
}
