// use block_mesh::{greedy_quads, GreedyQuadsBuffer, VoxelVisibility};
// use internal_renderer::*;
// use lumal::{atrace, vk::MappedMemoryRange, BufferDeletion, Image, ImageDeletion};
// use qvek::{vec3, vek::Vec3};
// // use rand::Rng;
// use crate::{
//     containers::Array3D,
//     internal_renderer::{
//         load_interface::LoadInterface,
//         render_interface::LumRendererAPI,
//         render_wgpu::{BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y, FRAMES_IN_FLIGHT},
//     },
//     types::*,
//     *,
// };
// use lumal::vk;

// use super::InternalRendererVulkan;

// // impl InternalRendererVulkan {}

// impl super::InternalRendererVulkan {
//     // TODO: runtime copies in single copy command buffer instead of per-model cmb
//     // creation
// }

// impl LoadInterface for InternalRendererVulkan {
//     type BufferType = lumal::Buffer;
//     type ImageType = lumal::Image;

//     // Palette on CPU side is (should) be represented as a POD array
//     // Palette on GPU side is stored differently (in 2d array of 3d blocks). This is
//     // due to perfomance win + hw limitations E.g. just doing 16*len x 16 x 16
//     // will not work cause 16xlen will be too big size for some gpus

//     fn update_block_palette_to_gpu(&mut self) {
//         assert!(self.block_palette_voxels.len() == self.static_block_palette_size as usize);
//         // create 3d array to be copied to gpu-side image after it is filled
//         let mut block_palette_prepared = Array3D::<Voxel>::new_filled(
//             (16 * BLOCK_PALETTE_SIZE_X) as usize,
//             (16 * BLOCK_PALETTE_SIZE_Y) as usize,
//             16,
//             0 as Voxel,
//         );

//         for (i, block) in self.block_palette_voxels.iter().enumerate() {
//             let block_xy = self.index_block_xy(i);
//             for_zyx!(16, 16, 16, |x, y, z| {
//                 #[allow(clippy::unnecessary_cast)]
//                 let vox = block[x as usize][y as usize][z as usize];
//                 block_palette_prepared[(
//                     x + ((block_xy.x as usize) * 16),
//                     y + ((block_xy.y as usize) * 16),
//                     z,
//                 )] = vox;
//             });
//         }

//         #[rustfmt::skip]
//         let buffer_count = block_palette_prepared.dimensions().0
//                          * block_palette_prepared.dimensions().1
//                          * block_palette_prepared.dimensions().2;
//         let buffer_size = buffer_count * std::mem::size_of::<Voxel>();

//         let staging_buffer =
//             self.lumal.create_buffer(vk::BufferUsageFlags::TRANSFER_SRC, buffer_size, true);

//         unsafe {
//             std::ptr::copy_nonoverlapping(
//                 block_palette_prepared.data.as_ptr(),
//                 staging_buffer.allocation.mapped_ptr().unwrap().as_ptr() as *mut Voxel,
//                 buffer_count,
//             );
//         };

//         unsafe {
//             debug_assert!(staging_buffer.allocation.mapped_ptr().is_some());
//             self.lumal
//                 .device
//                 .flush_mapped_memory_ranges(&[MappedMemoryRange {
//                     memory: staging_buffer.allocation.memory(),
//                     offset: 0,
//                     size: buffer_size as u64,
//                     ..Default::default()
//                 }])
//                 .unwrap();
//         };

//         for block_palette in self.independent_images.origin_block_palette.iter() {
//             assert!(block_palette_prepared.dimensions().0 == block_palette.extent.width as usize);
//             assert!(block_palette_prepared.dimensions().1 == block_palette.extent.height as usize);
//             assert!(block_palette_prepared.dimensions().2 == block_palette.extent.depth as usize);
//             self.lumal.copy_buffer_to_image_single_time(
//                 staging_buffer.buffer,
//                 block_palette,
//                 vk::Extent3D {
//                     width: block_palette_prepared.dimensions().0 as u32,
//                     height: block_palette_prepared.dimensions().1 as u32,
//                     depth: block_palette_prepared.dimensions().2 as u32,
//                 },
//             );
//         }

//         self.lumal.destroy_buffer(staging_buffer);
//     }

//     fn update_material_palette_to_gpu(&mut self) {
//         // we do not write it to intermediate buffer cuz its already in right layout - 6
//         // float rows one by one 256 total
//         assert!(!self.material_palette.is_empty());
//         // dbg!(&self.material_palette);
//         dbg!(&self.material_palette.len());
//         let buffer_count = self.material_palette.len();
//         let buffer_size = buffer_count * std::mem::size_of::<Material>();

//         // dbg!(&self.material_palette);

//         let staging_buffer =
//             self.lumal.create_buffer(vk::BufferUsageFlags::TRANSFER_SRC, buffer_size, true);

//         unsafe {
//             std::ptr::copy_nonoverlapping(
//                 self.material_palette.as_ptr(),
//                 staging_buffer.allocation.mapped_ptr().unwrap().as_ptr() as *mut Material,
//                 buffer_count,
//             );
//         }

//         for palette in self.independent_images.material_palette.iter() {
//             self.lumal.copy_buffer_to_image_single_time(
//                 staging_buffer.buffer,
//                 palette,
//                 vk::Extent3D {
//                     width: 6, // yep this is how it works for now
//                     height: self.material_palette.len() as u32,
//                     depth: 1,
//                 },
//             );
//         }

//         self.lumal.destroy_buffer(staging_buffer);
//     }

//     #[cold]
//     #[optimize(size)]
//     fn load_mesh_from_memory(
//         &mut self,
//         model: &ogt_vox::VoxModel,
//         _make_vertices: bool,
//     ) -> InternalMeshModel<Self::BufferType, Self::ImageType> {
//         let size = uvec3 {
//             x: model.size_x,
//             y: model.size_y,
//             z: model.size_z,
//         };

//         let mut padded_voxel_data = Array3D::<VoxelForContour>::new(
//             // +2 cause padding of 1 from each side
//             (size.x + 2) as usize,
//             (size.y + 2) as usize,
//             (size.z + 2) as usize,
//         );
//         padded_voxel_data.data.fill(VoxelForContour(0));

//         for xx in 0..size.x {
//             for yy in 0..size.y {
//                 for zz in 0..size.z {
//                     let voxel =
//                         model.voxel_data[(xx + yy * size.x + zz * size.x * size.y) as usize];
//                     // some padding for generator
//                     padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)] =
//                         VoxelForContour(voxel);
//                 }
//             }
//         }

//         let pvd_data_slice = unsafe {
//             std::slice::from_raw_parts(
//                 model.voxel_data.as_ptr() as *const Voxel,
//                 (size.x * size.y * size.z) as usize,
//             )
//         };

//         let voxels = self.create_rayrace_voxel_image(
//             pvd_data_slice,
//             size,
//             #[cfg(feature = "debug_validation_names")]
//             Some("Mesh Voxels"),
//         );

//         let triangles = self.make_contour_vertices(size, padded_voxel_data);

//         InternalMeshModel {
//             triangles,
//             voxels,
//             total_size: size,
//             // sprites: vec![],
//         }
//     }

//     #[cold]
//     #[optimize(size)]
//     fn create_rayrace_voxel_image(
//         &mut self,
//         voxels: &[Voxel],
//         size: uvec3,
//         #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
//     ) -> Self::ImageType {
//         let buffer_count = size.x * size.y * size.z;
//         let buffer_size = buffer_count * std::mem::size_of::<Voxel>() as u32;
//         assert_eq!(voxels.len(), ((size.x) * (size.y) * (size.z)) as usize);

//         let voxel_image = self.lumal.create_image(
//             vk::ImageType::TYPE_3D,
//             vk::Format::R8_UINT,
//             vk::ImageUsageFlags::STORAGE
//                 | vk::ImageUsageFlags::TRANSFER_DST
//                 | vk::ImageUsageFlags::SAMPLED,
//             // vulkanalia_vma::MemoryUsage::AutoPreferDevice,
//             // vulkanalia_vma::AllocationCreateFlags::empty(),
//             vk::ImageAspectFlags::COLOR,
//             uvec3_to_extent3d(size),
//             1,
//             vk::SampleCountFlags::TYPE_1,
//             #[cfg(feature = "debug_validation_names")]
//             Some("Rayrace Voxels"),
//         );

//         self.lumal.transition_image_layout_single_time(
//             &voxel_image,
//             vk::ImageLayout::UNDEFINED,
//             vk::ImageLayout::GENERAL,
//         );

//         let staging_buffer = self.lumal.create_buffer(
//             vk::BufferUsageFlags::TRANSFER_SRC,
//             buffer_size.try_into().unwrap(),
//             true,
//         );

//         unsafe {
//             std::ptr::copy_nonoverlapping(
//                 voxels.as_ptr(),
//                 staging_buffer.allocation.mapped_ptr().unwrap().as_ptr() as *mut Voxel,
//                 buffer_count.try_into().unwrap(),
//             );
//         };

//         self.lumal.copy_buffer_to_image_single_time(
//             staging_buffer.buffer,
//             &voxel_image,
//             uvec3_to_extent3d(size),
//         );

//         self.lumal.destroy_buffer(staging_buffer);

//         voxel_image
//     }

//     #[cold]
//     #[optimize(size)]
//     fn extract_palette_from_scene(&mut self, scene: &ogt_vox::VoxScene) {
//         for i in 0..scene.materials.matl.len() {
//             self.material_palette[i].albedo = vec3!(scene.palette.color[i].xyz()) / 255.0;
//             self.material_palette[i].transparency = scene.palette.color[i].w as f32 / 255.0;
//             self.material_palette[i].emmitness = 0.0;
//             self.material_palette[i].roughness = 0.0;

//             match scene.materials.matl[i].type_ {
//                 ogt_vox::MatlType::Diffuse => {
//                     self.material_palette[i].emmitness = 0.0;
//                     self.material_palette[i].roughness = 1.0;
//                 }
//                 ogt_vox::MatlType::Emit => {
//                     self.material_palette[i].emmitness =
//                         scene.materials.matl[i].emit * (2.0 + scene.materials.matl[i].flux * 4.0);
//                     self.material_palette[i].roughness = 0.5;
//                 }
//                 ogt_vox::MatlType::Metal => {
//                     self.material_palette[i].emmitness = 0.0;
//                     self.material_palette[i].roughness =
//                         scene.materials.matl[i].rough + (1.0 - scene.materials.matl[i].metal) / 2.0;
//                 }
//                 _ => {
//                     dbg!("Unknown material type");
//                 }
//             }
//         }
//     }

//     #[cold]
//     #[optimize(size)]
//     fn free_block(&mut self, block: BlockId) {
//         let block_mesh = std::mem::take(&mut self.block_palette_meshes[block as usize]);

//         assert!(block_mesh.triangles.vertexes.buffer != vk::Buffer::null());
//         assert!(block_mesh.triangles.indices.buffer != vk::Buffer::null());

//         self.lumal.buffer_deletion_queue.push(BufferDeletion {
//             buffer: block_mesh.triangles.vertexes,
//             lifetime: FRAMES_IN_FLIGHT as i32,
//         });
//         self.lumal.buffer_deletion_queue.push(BufferDeletion {
//             buffer: block_mesh.triangles.indices,
//             lifetime: FRAMES_IN_FLIGHT as i32,
//         });
//     }

//     #[cold]
//     #[optimize(size)]
//     fn create_and_upload_contour_buffers(
//         &mut self,
//         verts: &[PackedVoxelCircuit],
//         indices: &[u16],
//     ) -> (lumal::Buffer, lumal::Buffer) {
//         let vertexes = self.lumal.create_and_upload_buffer::<PackedVoxelCircuit>(
//             verts,
//             vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
//         );
//         let indices = self.lumal.create_and_upload_buffer::<u16>(
//             indices,
//             vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
//         );
//         (vertexes, indices)
//     }

//     #[cold]
//     #[optimize(size)]
//     fn free_mesh(&mut self, mesh: InternalMeshModel<Self::BufferType, Self::ImageType>) {
//         assert!(mesh.triangles.vertexes.buffer != vk::Buffer::null());
//         assert!(mesh.triangles.indices.buffer != vk::Buffer::null());
//         assert!(mesh.voxels.image != vk::Image::null());

//         self.lumal.buffer_deletion_queue.push(BufferDeletion {
//             buffer: mesh.triangles.vertexes,
//             lifetime: FRAMES_IN_FLIGHT as i32,
//         });
//         self.lumal.buffer_deletion_queue.push(BufferDeletion {
//             buffer: mesh.triangles.indices,
//             lifetime: FRAMES_IN_FLIGHT as i32,
//         });

//         self.lumal.image_deletion_queue.push(ImageDeletion {
//             image: mesh.voxels.image,
//             view: mesh.voxels.view,
//             allocation: mesh.voxels.allocation,
//             mip_views: mesh.voxels.mip_views,
//             lifetime: FRAMES_IN_FLIGHT as i32,
//         });
//     }

//     fn has_palette(&self) -> bool {
//         self.has_palette
//     }

//     fn set_has_palette(&mut self, has_palette: bool) {
//         self.has_palette = has_palette;
//     }

//     fn set_block_palette_voxels(&mut self, block_id: BlockId, pos: uvec3, voxel: Voxel) {
//         self.block_palette_voxels[block_id as usize][pos.x as usize][pos.y as usize]
//             [pos.z as usize] = voxel;
//     }

//     fn get_block_palette_voxels(&self, block_id: BlockId, pos: uvec3) -> Voxel {
//         self.block_palette_voxels[block_id as usize][pos.x as usize][pos.y as usize][pos.z as usize]
//     }

//     fn set_block_palette_mesh(
//         &mut self,
//         block_id: BlockId,
//         mesh: InternalMeshBlock<Self::BufferType>,
//     ) {
//         self.block_palette_meshes[block_id as usize] = mesh;
//     }

//     fn get_block_palette_mesh(&self, block_id: BlockId) -> &InternalMeshBlock<Self::BufferType> {
//         &self.block_palette_meshes[block_id as usize]
//     }
// }
