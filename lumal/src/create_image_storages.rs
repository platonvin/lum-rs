use crate::{debug_callback, DescriptorCounter};
use crate::{create_command_buffers, create_command_pool, create_logical_device, create_swapchain, create_swapchain_image_views, create_sync_objects, pick_physical_device, ring::Ring, Buffer, LumalRenderer, LumalSettings, VulkanData, PORTABILITY_MACOS_VERSION, VALIDATION_LAYER}; // Import the LumalRenderer struct
use crate::Image;
use std::{collections::HashSet, ptr};
use anyhow::*; 
use vulkanalia::vk::{self, DeviceV1_0, KhrSurfaceExtension, KhrSwapchainExtension};

use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::window as vk_window;
use vulkanalia_vma::{self as vma};
use vulkanalia_vma::Alloc;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use Vec as vector;

impl LumalRenderer {
    pub fn create_image_storage(
        &self,
        image_type: vk::ImageType,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        vma_usage: vma::MemoryUsage,
        vma_flags: vma::AllocationCreateFlags,
        aspect: vk::ImageAspectFlags,
        extent: vk::Extent3D,
        mipmaps: u32,
        sample_count: vk::SampleCountFlags,
    ) -> Result<Image> {
        let image_aspect = aspect;
        let image_format = format;
        let image_extent = extent;
        let image_mip_levels = mipmaps;

        let image_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            flags: vk::ImageCreateFlags::empty(),
            image_type,
            format,
            extent,
            mip_levels: mipmaps,
            array_layers: 1,
            samples: sample_count,
            tiling: vk::ImageTiling::OPTIMAL,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            // p_queue_family_indices: ptr::null(),
            initial_layout: vk::ImageLayout::UNDEFINED,
            next: ptr::null(),
            queue_family_indices: ptr::null(),
        };

        let alloc_info = vma::AllocationOptions {
            usage: vma_usage,
            flags: vma_flags,
            ..Default::default()
        };

        let (vk_image, allocation) = unsafe { self
            .allocator
            .create_image(image_info, &alloc_info) }?;

        let image_image = vk_image;
        let image_allocation = allocation;

        let view_type = match image_type {
            vk::ImageType::_1D => vk::ImageViewType::_1D,
            vk::ImageType::_2D => vk::ImageViewType::_2D,
            vk::ImageType::_3D => vk::ImageViewType::_3D,
            _ => return Err(anyhow!("Unsupported image type")),
        };

        let mut view_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            // p_next: std::ptr::null(),
            flags: vk::ImageViewCreateFlags::empty(),
            image: vk_image,
            view_type,
            format,
            components: vk::ComponentMapping::default(),
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: if (aspect.contains(vk::ImageAspectFlags::DEPTH))
                    && (aspect.contains(vk::ImageAspectFlags::STENCIL))
                {
                    vk::ImageAspectFlags::DEPTH
                } else {
                    aspect
                },
                base_mip_level: 0,
                level_count: mipmaps,
                base_array_layer: 0,
                layer_count: 1,
            },
            next: ptr::null(),
        };

        let image_view = unsafe { self.device.create_image_view(&view_info, None)? };

        let mut image_mip_views = vec![];
        if mipmaps > 1 {
            image_mip_views = (0..mipmaps)
                .map(|mip| {
                    view_info.subresource_range.base_mip_level = mip;
                    view_info.subresource_range.level_count = 1;
                    unsafe { self.device.create_image_view(&view_info, None) }
                })
                .collect::<Result<Vec<_>, _>>()?;
        }

        // self.transition_image_layout_single_time(
        //     &image.image,
        //     vk::ImageLayout::GENERAL,
        //     mipmaps,
        // )?;

        Ok(Image {
            image: image_image,
            allocation: image_allocation,
            view: image_view,
            mip_views: image_mip_views,
            format: image_format,
            aspect: image_aspect,
            extent: image_extent,
            mip_levels: image_mip_levels,
        })
    }
    pub fn create_image_storages(
        &self,
        size: usize,
        image_type: vk::ImageType,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        vma_usage: vma::MemoryUsage,
        vma_flags: vma::AllocationCreateFlags,
        aspect: vk::ImageAspectFlags,
        extent: vk::Extent3D,
        mipmaps: u32,
        sample_count: vk::SampleCountFlags,
    ) -> Result<Ring<Image>> {
        // Create a vector to hold the images.
        let mut images = Vec::with_capacity(size);

        // Initialize each image and push to the vector.
        for _ in 0..size {
            let image = self.create_image_storage(
                image_type,
                format,
                usage,
                vma_usage,
                vma_flags,
                aspect,
                extent,
                mipmaps,
                sample_count,
            )?;
            images.push(image);
        }

        // Return the Ring initialized with the images.
        Ok(Ring {
            data: images,
            index: 0,
        })
    }
}
// }