use crate::{
    types::uvec3,
    vulkan::{
        AllIndependentImages, AllSwapchainDependentImages, InternalRendererVulkan,
        BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y, BLOCK_SIZE, CHOSEN_DEPTH_FORMAT, FRAME_FORMAT,
        LIGHTMAPS_FORMAT, MATNORM_FORMAT, RADIANCE_FORMAT, SECONDARY_DEPTH_FORMAT,
    },
    Settings,
};
use containers::array3d::Dim3;
#[cfg(feature = "debug_validation_names")]
use lumal::set_debug_names;
use lumal::{vk, LumalSettings, Renderer};

fn uvec3_to_extent3d(size: uvec3) -> vk::Extent3D {
    vk::Extent3D {
        width: size.x,
        height: size.y,
        depth: size.z,
    }
}
fn usvec3_to_extent3d(size: qvek::vek::Vec3<usize>) -> vk::Extent3D {
    vk::Extent3D {
        width: size.x as u32,
        height: size.y as u32,
        depth: size.z as u32,
    }
}

impl<'a, D: Dim3> InternalRendererVulkan<'a, D> {
    /// Creates bundle of all swapchain size INdependent images
    /// You DONT need to recreate them when swapchain resizes (so they are created only once)
    pub fn create_independent_images(
        lumal: &mut Renderer,
        lum_settings: &Settings<D>,
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
            usvec3_to_extent3d(lum_settings.world_size.xyz()),
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
                width: lum_settings.lightmap_extent.x,
                height: lum_settings.lightmap_extent.y,
                depth: 1,
            },
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
            usvec3_to_extent3d(lum_settings.world_size.xyz()),
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
                width: BLOCK_SIZE * BLOCK_PALETTE_SIZE_X,
                height: BLOCK_SIZE * BLOCK_PALETTE_SIZE_Y,
                depth: BLOCK_SIZE,
            },
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
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Material Palette"),
        );

        let grass_state = lumal.create_image(
            vk::ImageType::TYPE_2D,
            vk::Format::R16G16_SFLOAT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: (lum_settings.world_size.x() * 2) as u32,
                height: (lum_settings.world_size.y() * 2) as u32,
                depth: 1,
            },
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
                width: (lum_settings.world_size.x() * 2) as u32,
                height: (lum_settings.world_size.y() * 2) as u32,
                depth: 1,
            },
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Water State"),
        );

        let perlin_noise2d = lumal.create_image(
            vk::ImageType::TYPE_2D,
            vk::Format::R16G16_SNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: lum_settings.world_size.x() as u32,
                height: lum_settings.world_size.y() as u32,
                depth: 1,
            },
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Perlin Noise 2D"),
        );

        let perlin_noise3d = lumal.create_image(
            vk::ImageType::TYPE_3D,
            vk::Format::R16G16B16A16_UNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: 32,
                height: 32,
                depth: 32,
            },
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Perlin Noise 3D"),
        );

        AllIndependentImages {
            grass_state,
            water_state,
            perlin_noise2d,
            perlin_noise3d,
            world,
            radiance_cache,
            block_palette: origin_block_palette,
            lightmap,
            material_palette,
        }
    }

    // Destroys bundle of all swapchain size dependent images
    /// You DO need to recreate them when swapchain resizes
    /// Does not creates swapchain images themselves
    pub fn create_dependent_images(
        lumal: &mut Renderer,
        _lum_settings: &Settings<D>,
        _lumal_settings: &LumalSettings,
    ) -> AllSwapchainDependentImages {
        let sextent = uvec3::new(
            lumal.swapchain_extent.width,
            lumal.swapchain_extent.height,
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
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Highres Material Norm"),
        );

        // Create stencil views for the depth-stencil images. Sometimes we need stencil-only
        let stencil_view_for_ds = {
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

            #[cfg(feature = "debug_validation_names")]
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
            vk::SampleCountFlags::TYPE_1,
            #[cfg(feature = "debug_validation_names")]
            Some("Near Depth"),
        );

        AllSwapchainDependentImages {
            frame: highres_frame,
            depth_stencil: highres_depth_stencil,
            mat_norm: highres_mat_norm,
            stencil_view_for_ds,
            far_depth,
            near_depth,
        }
    }

    /// Destroys bundle of all swapchain size INdependent images
    pub fn destroy_independent_images(
        lumal: &mut Renderer,
        independent_images: AllIndependentImages,
    ) {
        lumal.destroy_image(independent_images.grass_state);
        lumal.destroy_image(independent_images.water_state);
        lumal.destroy_image(independent_images.perlin_noise2d);
        lumal.destroy_image(independent_images.perlin_noise3d);
        lumal.destroy_image_ring(independent_images.world);
        lumal.destroy_image_ring(independent_images.radiance_cache);
        lumal.destroy_image_ring(independent_images.block_palette);
        lumal.destroy_image_ring(independent_images.material_palette);
        lumal.destroy_image(independent_images.lightmap);
    }

    /// Destroys bundle of all swapchain size dependent images
    /// Does not destroy swapchain images themselves
    pub fn destroy_dependent_images(
        lumal: &mut Renderer,
        dependent_images: AllSwapchainDependentImages,
    ) {
        // Not supposed to happen - swapchain images are destroyed by the driver
        // self.lumal.destroy_image_ring(&self.dependent_images.swapchain_images);

        lumal.destroy_image(dependent_images.frame);
        lumal.destroy_image(dependent_images.depth_stencil);
        unsafe { lumal.device.destroy_image_view(dependent_images.stencil_view_for_ds, None) };
        lumal.destroy_image(dependent_images.mat_norm);
        lumal.destroy_image(dependent_images.far_depth);
        lumal.destroy_image(dependent_images.near_depth);
    }
}
