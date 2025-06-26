//! Just a simplification-wrapper for barriers that suits Lum.
//! It is missing a lot but it does not matter anyways - drivers dont give a fuck about precise barriers.

use crate::{Buffer, Image, Renderer};
use ash::vk;

impl Renderer {
    /// Places Vulkan pipeline barrier for an image.
    pub fn image_memory_barrier(
        &self,
        cmdbuf: &vk::CommandBuffer,
        image: &Image,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        src_access_mask: vk::AccessFlags,
        dst_access_mask: vk::AccessFlags,
        src_layout: vk::ImageLayout,
        dst_layout: vk::ImageLayout,
    ) {
        let barrier: vk::ImageMemoryBarrier = vk::ImageMemoryBarrier {
            old_layout: src_layout,
            new_layout: dst_layout,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: image.image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: image.aspect,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_access_mask,
            dst_access_mask,
            ..Default::default()
        };

        unsafe {
            self.device.cmd_pipeline_barrier(
                *cmdbuf,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            )
        };
    }

    /// Places Vulkan pipeline barrier for a buffer.
    pub fn buffer_memory_barrier(
        &self,
        cmdbuf: &vk::CommandBuffer,
        buffer: &Buffer,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        src_access_mask: vk::AccessFlags,
        dst_access_mask: vk::AccessFlags,
    ) {
        let barrier: vk::BufferMemoryBarrier = vk::BufferMemoryBarrier {
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            buffer: buffer.buffer,
            offset: 0,
            size: vk::WHOLE_SIZE,
            src_access_mask,
            dst_access_mask,
            ..Default::default()
        };

        unsafe {
            self.device.cmd_pipeline_barrier(
                *cmdbuf,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[barrier],
                &[],
            )
        };
    }
}
