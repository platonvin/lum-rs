use crate::renderer::{
    types::uvec3,
    vulkan::{
        types::uvec3_to_extent3d, AllIndependentImages, AllSwapchainDependentImages,
        InternalRendererVulkan, BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y, CHOSEN_DEPTH_FORMAT,
        FRAME_FORMAT, LIGHTMAPS_FORMAT, MATNORM_FORMAT, RADIANCE_FORMAT, SECONDARY_DEPTH_FORMAT,
    },
    Settings,
};
// use internal_renderer::{InternalRendererVulkan, *};
use lumal::vk;
use lumal::{ring::Ring, set_debug_names, LumalSettings, Renderer};

impl InternalRendererVulkan {
    #[cold]
    #[optimize(size)]
    pub fn create_independent_images(
        lumal: &mut Renderer,
        lum_settings: &Settings,
        lumal_settings: &LumalSettings,
    ) -> AllIndependentImages {
        let world = lumal.create_image_ring(
            lumal_settings.fif,
            vk::ImageType::TYPE_3D,
            vk::Format::R16_SINT,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(lum_settings.world_size),
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("World"),
        ); // TODO: dynamic

        let lightmap = lumal.create_image(
            vk::ImageType::TYPE_2D,
            LIGHTMAPS_FORMAT,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::DEPTH,
            vk::Extent3D {
                width: lum_settings.lightmap_extent.width,
                height: lum_settings.lightmap_extent.height,
                depth: 1,
            },
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Lightmap"),
        );

        let radiance_cache = lumal.create_image_ring(
            lumal_settings.fif,
            vk::ImageType::TYPE_3D,
            RADIANCE_FORMAT,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(lum_settings.world_size),
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Radiance Cache"),
        );
        let origin_block_palette = lumal.create_image_ring(
            lumal_settings.fif,
            vk::ImageType::TYPE_3D,
            vk::Format::R8_UINT,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: 16 * BLOCK_PALETTE_SIZE_X,
                height: 16 * BLOCK_PALETTE_SIZE_Y,
                depth: 16,
            },
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Origin Block Palette"),
        );
        let material_palette = lumal.create_image_ring(
            lumal_settings.fif,
            vk::ImageType::TYPE_2D,
            vk::Format::R32_SFLOAT, // try R32G32
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: 6,
                height: 256,
                depth: 1,
            },
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Material Palette"),
        );
        let grass_state = lumal.create_image(
            vk::ImageType::TYPE_2D,
            vk::Format::R16G16_SFLOAT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            // vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            // vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: lum_settings.world_size.x * 2,
                height: lum_settings.world_size.y * 2,
                depth: 1,
            },
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Grass State"),
        );
        let water_state = lumal.create_image(
            vk::ImageType::TYPE_2D,
            vk::Format::R16G16B16A16_SFLOAT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: lum_settings.world_size.x * 2,
                height: lum_settings.world_size.y * 2,
                depth: 1,
            },
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Water State"),
        );
        let perlin_noise2d = lumal.create_image(
            vk::ImageType::TYPE_2D,
            vk::Format::R16G16_SNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            // vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            // vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: lum_settings.world_size.x,
                height: lum_settings.world_size.y,
                depth: 1,
            },
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Perlin Noise 2D"),
        ); // does not matter than much
        let perlin_noise3d = lumal.create_image(
            vk::ImageType::TYPE_3D,
            vk::Format::R16G16B16A16_UNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            // vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            // vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: 32,
                height: 32,
                depth: 32,
            },
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Perlin Noise 3D"),
        ); // does not matter than much

        AllIndependentImages {
            grass_state,
            water_state,
            perlin_noise2d,
            perlin_noise3d,
            world,
            radiance_cache,
            origin_block_palette,
            lightmap,
            // distance_palette: distance_palette,
            // bit_palette: bit_palette,
            material_palette,
        }
    }

    // dependent = swapchain dependent
    pub fn create_dependent_images(
        lumal: &mut Renderer,
        _lum_settings: &Settings,
        lumal_settings: &LumalSettings,
    ) -> AllSwapchainDependentImages {
        let sextent = uvec3::new(
            lumal.vulkan_data.swapchain_extent.width,
            lumal.vulkan_data.swapchain_extent.height,
            1,
        );

        let highres_mat_norm = lumal.create_image(
            vk::ImageType::TYPE_2D,
            MATNORM_FORMAT,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::INPUT_ATTACHMENT,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(sextent),
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Highres Frame"),
        );

        let highres_depth_stencil = lumal.create_image(
            vk::ImageType::TYPE_2D,
            unsafe { CHOSEN_DEPTH_FORMAT },
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                | vk::ImageUsageFlags::INPUT_ATTACHMENT,
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
            uvec3_to_extent3d(sextent),
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Highres Depth Stencil"),
        );

        let highres_frame = lumal.create_image(
            vk::ImageType::TYPE_2D,
            FRAME_FORMAT,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::INPUT_ATTACHMENT
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(sextent),
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Highres Material Norm"),
        );

        // Create stencil views for the depth-stencil images
        let mut stencil_view_for_ds = {
            let view_info = vk::ImageViewCreateInfo {
                flags: vk::ImageViewCreateFlags::empty(),
                image: highres_depth_stencil.image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: unsafe { CHOSEN_DEPTH_FORMAT },
                components: vk::ComponentMapping::default(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::STENCIL,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            let stencil_view_for_ds =
                unsafe { lumal.device.create_image_view(&view_info, None).unwrap() };
            set_debug_names!(
                lumal,
                Some("Stencil View for DS"),
                (&stencil_view_for_ds[i], "Image View")
            );

            stencil_view_for_ds
        };

        let far_depth = lumal.create_image(
            vk::ImageType::TYPE_2D,
            SECONDARY_DEPTH_FORMAT,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::INPUT_ATTACHMENT
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(sextent),
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Far Depth"),
        );

        let near_depth = lumal.create_image(
            vk::ImageType::TYPE_2D,
            SECONDARY_DEPTH_FORMAT,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::INPUT_ATTACHMENT
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(sextent),
            1,
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Near Depth"),
        );

        AllSwapchainDependentImages {
            highres_frame,
            highres_depth_stencil,
            highres_mat_norm,
            stencil_view_for_ds,
            far_depth,
            near_depth,
        }
    }

    #[cold]
    #[optimize(size)]
    pub fn destroy_independent_images(
        lumal: &mut Renderer,
        independent_images: AllIndependentImages,
    ) {
        println!("started destroying independent images");
        lumal.destroy_image(independent_images.grass_state);
        lumal.destroy_image(independent_images.water_state);
        lumal.destroy_image(independent_images.perlin_noise2d);
        lumal.destroy_image(independent_images.perlin_noise3d);
        lumal.destroy_image_ring(independent_images.world);
        lumal.destroy_image_ring(independent_images.radiance_cache);
        lumal.destroy_image_ring(independent_images.origin_block_palette);
        lumal.destroy_image_ring(independent_images.material_palette);
        lumal.destroy_image(independent_images.lightmap);
        println!("destroyed independent images");
    }

    #[cold]
    #[optimize(size)]
    pub fn destroy_dependent_images(
        lumal: &mut Renderer,
        dependent_images: AllSwapchainDependentImages,
    ) {
        println!("started destroying swapchain dependent images");

        // Not supposed to happen - swapchain images are destroyed by the driver
        // self.lumal.destroy_image_ring(&self.dependent_images.swapchain_images);

        lumal.destroy_image(dependent_images.highres_frame);
        lumal.destroy_image(dependent_images.highres_depth_stencil);
        lumal.destroy_image(dependent_images.highres_mat_norm);
        unsafe { lumal.device.destroy_image_view(dependent_images.stencil_view_for_ds, None) };
        lumal.destroy_image(dependent_images.far_depth);
        lumal.destroy_image(dependent_images.near_depth);
        println!("destroyed swapchain dependent images");
    }
}
