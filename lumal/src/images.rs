use crate::Renderer; // Import the LumalRenderer struct
use crate::{set_debug_names, Image};
use ash::vk;
use containers::Ring;
use gpu_allocator::vulkan as vma;

impl Renderer {
    pub fn create_image(
        &mut self,
        image_type: vk::ImageType,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        aspect: vk::ImageAspectFlags,
        extent: vk::Extent3D,
        sample_count: vk::SampleCountFlags,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> Image {
        let image_aspect = aspect;
        let image_format = format;
        let image_extent = extent;
        // let image_mip_levels = mipmaps;

        let image_info = vk::ImageCreateInfo {
            image_type,
            format,
            extent,
            mip_levels: 1,
            array_layers: 1,
            samples: sample_count,
            tiling: vk::ImageTiling::OPTIMAL,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            queue_family_index_count: 0,
            ..Default::default()
        };

        let vk_image = unsafe { self.device.create_image(&image_info, None).unwrap() };
        let requirements = unsafe { self.device.get_image_memory_requirements(vk_image) };

        let alloc_info = vma::AllocationCreateDesc {
            name: "",
            requirements,
            location: gpu_allocator::MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: vma::AllocationScheme::GpuAllocatorManaged,
        };

        let allocation = self.allocator.allocate(&alloc_info).unwrap();

        unsafe {
            self.device
                .bind_image_memory(vk_image, allocation.memory(), allocation.offset())
                .unwrap()
        };

        let view_type = match image_type {
            vk::ImageType::TYPE_1D => vk::ImageViewType::TYPE_1D,
            vk::ImageType::TYPE_2D => vk::ImageViewType::TYPE_2D,
            vk::ImageType::TYPE_3D => vk::ImageViewType::TYPE_3D,
            _ => panic!("Unsupported image type"),
        };

        let view_info = vk::ImageViewCreateInfo {
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
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        let image_view = unsafe { self.device.create_image_view(&view_info, None).unwrap() };

        // let mut image_mip_views = vec![];
        // if mipmaps > 1 {
        //     image_mip_views = (0..mipmaps)
        //         .map(|mip| {
        //             view_info.subresource_range.base_mip_level = mip;
        //             view_info.subresource_range.level_count = 1;
        //             let view = unsafe { self.device.create_image_view(&view_info, None).unwrap() };
        //             set_debug_names!(self, Some("Stencil View for DS"), (&view, "Image View"));
        //             view
        //         })
        //         .collect::<Vec<_>>();
        // }

        let image = Image {
            image: vk_image,
            allocation,
            view: image_view,
            // mip_views: image_mip_views,
            format: image_format,
            aspect: image_aspect,
            extent: image_extent,
            // mip_levels: image_mip_levels,
        };

        self.transition_image_layout_single_time(
            &image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
        );

        set_debug_names!(
            self,
            debug_name,
            (&image.image, "Image"),
            (&image.view, "Image View"),
            (&image.allocation.memory(), "Image Allocation Device Memory")
        );

        image
    }

    pub fn create_image_ring(
        &mut self,
        size: usize,
        image_type: vk::ImageType,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        // vma_usage: vma::MemoryUsage,
        // vma_flags: vma::AllocationCreateFlags,
        aspect: vk::ImageAspectFlags,
        extent: vk::Extent3D,
        // mipmaps: u32,
        sample_count: vk::SampleCountFlags,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> Ring<Image> {
        // Create a vector to hold the images.
        let mut images = Vec::with_capacity(size);

        // Initialize each image and push to the vector.
        for _ in 0..size {
            let image = self.create_image(
                image_type,
                format,
                usage,
                // vma_usage,
                // vma_flags,
                aspect,
                extent,
                // mipmaps,
                sample_count,
                #[cfg(feature = "debug_validation_names")]
                debug_name,
            );
            images.push(image);
        }

        // Return the Ring initialized with the images.
        Ring {
            data: images.into_boxed_slice(),
            index: 0,
        }
    }

    pub fn destroy_image(&mut self, img: Image) {
        unsafe {
            self.device.destroy_image_view(img.view, None);
            self.allocator.free(img.allocation).unwrap();
            self.device.destroy_image(img.image, None);
        };
    }

    pub fn destroy_image_ring(&mut self, images: Ring<Image>) {
        for img in images.data {
            self.destroy_image(img);
        }
    }
}
