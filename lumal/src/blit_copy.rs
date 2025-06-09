use crate::{Image, Renderer};
use ash::vk;

impl Renderer {
    pub fn copy_whole_image(&self, cmdbuf: vk::CommandBuffer, src: &Image, dst: &Image) {
        let copy_op = vk::ImageCopy {
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            dst_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            extent: src.extent,
        };

        unsafe {
            self.device.cmd_copy_image(
                cmdbuf,
                src.image,
                vk::ImageLayout::GENERAL, // TODO
                dst.image,
                vk::ImageLayout::GENERAL, // TODO
                &[copy_op],
            );

            let barrier: vk::ImageMemoryBarrier = vk::ImageMemoryBarrier {
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                old_layout: vk::ImageLayout::GENERAL,
                new_layout: vk::ImageLayout::GENERAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: dst.image, // assume you have the image handle
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
                ..Default::default() // initialize other fields if necessary
            };

            self.device.cmd_pipeline_barrier(
                cmdbuf,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }

    // basically copy image into another image (with possible dimension mismatch and thus scaling)

    pub fn blit_whole_image(
        &self,
        cmdbuf: vk::CommandBuffer,
        src: &Image,
        dst: &Image,
        filter: vk::Filter,
    ) {
        let src_offsets = [
            vk::Offset3D { x: 0, y: 0, z: 0 },
            vk::Offset3D {
                x: src.extent.width as i32,
                y: src.extent.height as i32,
                z: src.extent.depth as i32,
            },
        ];

        let dst_offsets = [
            vk::Offset3D { x: 0, y: 0, z: 0 },
            vk::Offset3D {
                x: dst.extent.width as i32,
                y: dst.extent.height as i32,
                z: dst.extent.depth as i32,
            },
        ];

        let blit_op = vk::ImageBlit {
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_offsets,
            dst_offsets,
        };

        unsafe {
            self.device.cmd_blit_image(
                cmdbuf,
                src.image,
                vk::ImageLayout::GENERAL, // TODO
                dst.image,
                vk::ImageLayout::GENERAL, // TODO
                &[blit_op],
                filter,
            );

            let barrier = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::GENERAL,
                new_layout: vk::ImageLayout::GENERAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: dst.image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
                ..Default::default()
            };

            self.device.cmd_pipeline_barrier(
                cmdbuf,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }

    /// Finds first image format from given candidates that is supported by device (for given type, tiling and usage)
    pub fn find_supported_format(
        &self,
        candidates: &[vk::Format],
        ty: vk::ImageType,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
    ) -> Option<vk::Format> {
        for &format in candidates {
            let result = unsafe {
                self.instance.get_physical_device_image_format_properties(
                    self.physical_device,
                    format,
                    ty,
                    tiling,
                    usage,
                    vk::ImageCreateFlags::empty(),
                )
            };

            if result.is_ok() {
                return Some(format);
            }
        }
        None
    }

    /// Vulkan set viewport/scissors wrapper
    pub fn cmd_set_viewport(&self, cmdbuf: vk::CommandBuffer, width: u32, height: u32) {
        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width, height },
        };

        unsafe {
            self.device.cmd_set_viewport(cmdbuf, 0, &[viewport]);
            self.device.cmd_set_scissor(cmdbuf, 0, &[scissor]);
        }
    }
}
