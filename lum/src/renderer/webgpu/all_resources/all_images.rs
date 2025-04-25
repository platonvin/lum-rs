use crate::renderer::{
    webgpu::{
        wal::Wal, AllIndependentImages, AllSwapchainDependentImages, InternalRendererWebGPU,
        BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y, CHOSEN_DEPTH_FORMAT, FRAME_FORMAT,
        LIGHTMAPS_FORMAT, MATNORM_FORMAT, RADIANCE_FORMAT, SECONDARY_DEPTH_FORMAT,
    },
    Settings,
};

impl<'window> InternalRendererWebGPU<'window> {
    #[cold]
    #[optimize(size)]
    pub fn create_independent_images(wal: &Wal, lum_settings: &Settings) -> AllIndependentImages {
        let fif = wal.config.desired_maximum_frame_latency as usize;

        let world = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D3,
            wgpu::TextureFormat::R32Sint,
            // wgpu::TextureUsages::STORAGE_BINDING // we dont need storage binding since we dont really write to it on gpu
            wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: lum_settings.world_size.x,
                height: lum_settings.world_size.y,
                depth_or_array_layers: lum_settings.world_size.z,
            },
            1,
            Some("World"),
        );

        let lightmap = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D2,
            LIGHTMAPS_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: lum_settings.lightmap_extent.width,
                height: lum_settings.lightmap_extent.height,
                depth_or_array_layers: 1,
            },
            1,
            Some("Lightmap"),
        );

        let radiance_cache = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D3,
            RADIANCE_FORMAT,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            wgpu::Extent3d {
                width: lum_settings.world_size.x,
                height: lum_settings.world_size.y,
                depth_or_array_layers: lum_settings.world_size.z,
            },
            1,
            Some("Radiance Cache"),
        );

        let origin_block_palette = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D3,
            wgpu::TextureFormat::R32Sint,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            wgpu::Extent3d {
                width: 16 * BLOCK_PALETTE_SIZE_X,
                height: 16 * BLOCK_PALETTE_SIZE_Y,
                depth_or_array_layers: 16,
            },
            1,
            Some("Origin Block Palette"),
        );

        let material_palette = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: 6,
                height: 256,
                depth_or_array_layers: 1,
            },
            1,
            Some("Material Palette"),
        );

        let grass_state = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::Rg32Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: lum_settings.world_size.x * 2,
                height: lum_settings.world_size.y * 2,
                depth_or_array_layers: 1,
            },
            1,
            Some("Grass State"),
        );

        let water_state = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::Rg32Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: lum_settings.world_size.x * 2,
                height: lum_settings.world_size.y * 2,
                depth_or_array_layers: 1,
            },
            1,
            Some("Water State"),
        );

        let perlin_noise2d = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::Rg32Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: lum_settings.world_size.x,
                height: lum_settings.world_size.y,
                depth_or_array_layers: 1,
            },
            1,
            Some("Perlin Noise 2D"),
        );

        let perlin_noise3d = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D3,
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 32,
            },
            1,
            Some("Perlin Noise 3D"),
        );

        AllIndependentImages {
            world,
            lightmap,
            radiance_cache,
            origin_block_palette,
            material_palette,
            grass_state,
            water_state,
            perlin_noise2d,
            perlin_noise3d,
        }
    }

    // dependent = swapchain dependent
    pub fn create_dependent_images(
        wal: &Wal,
        lum_settings: &Settings,
    ) -> AllSwapchainDependentImages {
        let sextent = wgpu::Extent3d {
            width: wal.config.width,
            height: wal.config.height,
            depth_or_array_layers: 1,
        };

        let fif = wal.config.desired_maximum_frame_latency as usize;

        let highres_mat_norm = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D2,
            MATNORM_FORMAT,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            1,
            Some("Highres Mat Norm"),
        );

        let highres_depth_stencil = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D2,
            unsafe { CHOSEN_DEPTH_FORMAT.unwrap() },
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            1,
            Some("Highres Depth Stencil"),
        );

        let highres_frame = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D2,
            FRAME_FORMAT,
            // wgpu::TextureUsages::STORAGE_BINDING|
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            1,
            Some("Highres Frame"),
        );

        let mut stencil_view_for_ds = Vec::with_capacity(fif);
        let mut full_view_for_ds = Vec::with_capacity(fif);
        for texture in &highres_depth_stencil {
            stencil_view_for_ds.push(texture.texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Stencil View for DS"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::StencilOnly,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
                usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
            }));
            full_view_for_ds.push(texture.texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Stencil View for DS"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
                usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
            }));
        }
        let stencil_view_for_ds = lumal::ring::Ring::from_vec(stencil_view_for_ds);
        let full_view_for_ds = lumal::ring::Ring::from_vec(full_view_for_ds);

        // these are not native depth textures, they achive depth functionality via blend
        let far_depth = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::R16Float,
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            1,
            Some(""),
        );
        let near_depth = wal.create_image_ring(
            fif,
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::R16Float,
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            1,
            Some(""),
        );

        AllSwapchainDependentImages {
            highres_frame,
            highres_depth_stencil,
            highres_mat_norm,
            stencil_view_for_ds,
            full_view_for_ds,
            far_depth,
            near_depth,
        }
    }
}
