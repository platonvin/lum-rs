pub mod aabb;
pub mod ao_lut;
pub mod gen_perlin_noise;
pub mod render;

use crate::*;
pub use aabb::*;
pub use ao_lut::*;
pub use render::*;

use std::ptr::null;

use lumal::ring::Ring;
use vulkanalia::vk::{self, DeviceV1_0};
use vulkanalia_vma::{
    vma, Alloc, AllocationCreateFlags, AllocationOptions, AllocatorCreateFlags, MemoryUsage,
};


impl crate::LumRenderer {
    //TODO: runtime copies in single copy command buffer instead of per-model cmb creation
    pub fn create_rayrace_voxel_images(
        &mut self,
        voxels: &[Voxel],
        size: uvec3,
    ) -> Ring<lumal::Image> {
        let buffer_size = size.x * size.y * size.z;
        assert!(voxels.len() == (size.x * size.y * size.z) as usize);

        let mut voxel_images = self
            .lumal
            .create_image_ring(
                self.lumal.settings.fif,
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

        for img in voxel_images.iter() {
            self.lumal.transition_image_layout_single_time(
                &img,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            );
        }

        // i hate vulkanalia. TODO: try vulkano
        let staging_buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            flags: vk::BufferCreateFlags::empty(),
            size: (buffer_size * std::mem::size_of::<Voxel>() as u32) as vk::DeviceSize,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            next: null(),
            queue_family_indices: null(),
        };

        let staging_alloc_info = AllocationOptions {
            flags: AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            usage: MemoryUsage::Auto,
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
                .create_buffer(staging_buffer_info, &staging_alloc_info)
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
            std::ptr::copy_nonoverlapping(voxels.as_ptr(), mapped as *mut Voxel, buffer_size.try_into().unwrap());
        };

        for img in voxel_images.iter() {
            self.lumal
                .copy_buffer_to_image_single_time(staging_buffer, &img, uvec3_to_extent3d(size));
        }

        unsafe {
            self.lumal
                .allocator
                .as_ref()
                .unwrap()
                .destroy_buffer(staging_buffer, stagin_alloc);
        };

        voxel_images
    }
}
