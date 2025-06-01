use crate::renderer::{
    webgpu::{
        wal::Wal, AllIndependentImages, AllSwapchainDependentImages, InternalRendererWebGPU,
        BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y, CHOSEN_DEPTH_FORMAT, CHOSEN_STENCIL_FORMAT,
        FRAME_FORMAT, LIGHTMAPS_FORMAT, MATNORM_FORMAT, RADIANCE_FORMAT,
    },
    Settings,
};

impl<'window> InternalRendererWebGPU<'window> {
    #[cold]
    #[optimize(size)]
    pub fn create_independent_images(wal: &Wal, lum_settings: &Settings) -> AllIndependentImages {
        let fif = wal.config.desired_maximum_frame_latency as usize;

        let world = wal.create_images(
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
            Some("World"),
        );

        let lightmap = wal.create_image(
            wgpu::TextureDimension::D2,
            LIGHTMAPS_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: lum_settings.lightmap_extent.width,
                height: lum_settings.lightmap_extent.height,
                depth_or_array_layers: 1,
            },
            Some("Lightmap"),
        );

        let radiance_cache = wal.create_images(
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
            Some("Radiance Cache"),
        );

        let origin_block_palette = wal.create_images(
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
            Some("Origin Block Palette"),
        );

        let material_palette = wal.create_image(
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: 6,
                height: 256,
                depth_or_array_layers: 1,
            },
            Some("Material Palette"),
        );

        let grass_state = wal.create_image(
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::Rg32Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: lum_settings.world_size.x * 2,
                height: lum_settings.world_size.y * 2,
                depth_or_array_layers: 1,
            },
            Some("Grass State"),
        );

        let water_state = wal.create_image(
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: lum_settings.world_size.x * 2,
                height: lum_settings.world_size.y * 2,
                depth_or_array_layers: 1,
            },
            Some("Water State"),
        );

        let perlin_noise2d = wal.create_image(
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::Rg32Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: lum_settings.world_size.x,
                height: lum_settings.world_size.y,
                depth_or_array_layers: 1,
            },
            Some("Perlin Noise 2D"),
        );

        let perlin_noise3d = wal.create_image(
            wgpu::TextureDimension::D3,
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 32,
            },
            Some("Perlin Noise 3D"),
        );

        AllIndependentImages {
            world,
            lightmap,
            radiance_cache,
            block_palette: origin_block_palette,
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
        _lum_settings: &Settings,
    ) -> AllSwapchainDependentImages {
        let sextent = wgpu::Extent3d {
            width: wal.config.width,
            height: wal.config.height,
            depth_or_array_layers: 1,
        };

        let mat_norm = wal.create_image(
            wgpu::TextureDimension::D2,
            MATNORM_FORMAT,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            Some("Highres Mat Norm"),
        );

        let depth = wal.create_image(
            wgpu::TextureDimension::D2,
            unsafe { CHOSEN_DEPTH_FORMAT.unwrap() },
            wgpu::TextureUsages::TEXTURE_BINDING
                // | wgpu::TextureUsages::COPY_SRC
                // | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            Some("Highres Depth"),
        );

        let stencil = wal.create_image(
            wgpu::TextureDimension::D2,
            unsafe { CHOSEN_STENCIL_FORMAT.unwrap() },
            wgpu::TextureUsages::TEXTURE_BINDING
                // | wgpu::TextureUsages::COPY_SRC
                // | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            Some("Highres Stencil"),
        );

        let frame = wal.create_image(
            wgpu::TextureDimension::D2,
            FRAME_FORMAT,
            // wgpu::TextureUsages::STORAGE_BINDING|
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            Some("Highres Frame"),
        );

        // these are not native depth textures, they achive depth functionality via blend
        let far_depth = wal.create_image(
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::R16Float,
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            Some(""),
        );
        let near_depth = wal.create_image(
            wgpu::TextureDimension::D2,
            wgpu::TextureFormat::R16Float,
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            sextent,
            Some(""),
        );

        AllSwapchainDependentImages {
            frame,
            depth,
            stencil,
            mat_norm,
            far_depth,
            near_depth,
        }
    }
}
