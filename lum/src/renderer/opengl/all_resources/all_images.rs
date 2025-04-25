use glow::HasContext;

use crate::{
    internal_renderer::{
        render_gl::{
            AllIndependentImages, AllSwapchainDependentImages, GlSettings, InternalRendererGL,
            BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y, CHOSEN_DEPTH_FORMAT, FRAME_FORMAT,
            LIGHTMAPS_FORMAT, MATNORM_FORMAT, RADIANCE_FORMAT, SECONDARY_DEPTH_FORMAT,
        },
        Settings,
    },
    types::{uvec2, uvec3, uvec3_to_extent3d},
    *,
};

impl InternalRendererGL {
    #[cold]
    #[optimize(size)]
    pub fn create_image(
        gl: &glow::Context,
        image_type: u32,      // TEXTURE_1D, TEXTURE_2D, or TEXTURE_3D
        internal_format: u32, // GBA8, DEPTH24_STENCIL8, etc.
        extent: (u32, u32, u32),
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> glow::Texture {
        unsafe {
            let tex = gl.create_texture().expect("Failed to create texture");

            // 1) bind to the correct target
            gl.bind_texture(image_type, Some(tex));

            // 2) allocate immutable storage on that target
            let (w, h, d) = (extent.0 as i32, extent.1 as i32, extent.2 as i32);
            match image_type {
                glow::TEXTURE_1D => {
                    gl.tex_storage_1d(image_type, 1, internal_format, w);
                }
                glow::TEXTURE_2D => {
                    gl.tex_storage_2d(image_type, 1, internal_format, w, h);
                }
                glow::TEXTURE_3D => {
                    gl.tex_storage_3d(image_type, 1, internal_format, w, h, d);
                }
                _ => unreachable!("Unsupported texture dimension"),
            }

            // 3) TODO: this vs sampler?..
            gl.tex_parameter_i32(image_type, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(image_type, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);

            // 4) nbind
            gl.bind_texture(image_type, None);

            #[cfg(feature = "debug_validation_names")]
            if let Some(name) = debug_name {
                gl.object_label(glow::TEXTURE, tex, name);
            }

            tex
        }
    }

    #[cold]
    #[optimize(size)]
    pub fn create_independent_images(
        gl: &glow::Context,
        lum_settings: &Settings,
    ) -> AllIndependentImages {
        let world = Self::create_image(
            gl,
            glow::TEXTURE_3D,
            glow::R16I,
            (
                lum_settings.world_size.x as u32,
                lum_settings.world_size.y as u32,
                lum_settings.world_size.z as u32,
            ),
        );

        // Assuming lumal.create_image_ring creates a single image for now
        let lightmap = Self::create_image(
            gl,
            glow::TEXTURE_2D,
            glow::DEPTH_COMPONENT32F, // Assuming LIGHTMAPS_FORMAT is a depth format
            (
                lum_settings.lightmap_extent.width,
                lum_settings.lightmap_extent.height,
                1,
            ),
        );

        let radiance_cache = Self::create_image(
            gl,
            glow::TEXTURE_3D,
            glow::RGBA16F, // Assuming RADIANCE_FORMAT is a color format
            (
                lum_settings.world_size.x as u32,
                lum_settings.world_size.y as u32,
                lum_settings.world_size.z as u32,
            ),
        );

        let origin_block_palette = Self::create_image(
            gl,
            glow::TEXTURE_3D,
            glow::R8UI,
            (16 * BLOCK_PALETTE_SIZE_X, 16 * BLOCK_PALETTE_SIZE_Y, 16),
        );

        let material_palette = Self::create_image(gl, glow::TEXTURE_2D, glow::R32F, (6, 256, 1));

        let grass_state = Self::create_image(
            gl,
            glow::TEXTURE_2D,
            glow::RG16F,
            (
                lum_settings.world_size.x * 2,
                lum_settings.world_size.y * 2,
                1,
            ),
        );

        let water_state = Self::create_image(
            gl,
            glow::TEXTURE_2D,
            glow::RGBA16F,
            (
                lum_settings.world_size.x * 2,
                lum_settings.world_size.y * 2,
                1,
            ),
        );

        let perlin_noise2d = Self::create_image(
            gl,
            glow::TEXTURE_2D,
            glow::RG16_SNORM,
            (lum_settings.world_size.x, lum_settings.world_size.y, 1),
        );

        let perlin_noise3d = Self::create_image(gl, glow::TEXTURE_3D, glow::RGBA16, (32, 32, 32));

        AllIndependentImages {
            grass_state,
            water_state,
            perlin_noise2d,
            perlin_noise3d,
            world,
            radiance_cache,
            origin_block_palette,
            lightmap,
            material_palette,
        }
    }

    // dependent = swapchain dependent
    pub fn create_dependent_images(
        gl: &glow::Context,
        wh: (u32, u32),
    ) -> AllSwapchainDependentImages {
        let (width, height) = wh;

        let highres_mat_norm =
            Self::create_image(gl, glow::TEXTURE_2D, glow::RGBA8, (width, height, 1));

        let highres_depth_stencil = Self::create_image(
            gl,
            glow::TEXTURE_2D,
            glow::DEPTH24_STENCIL8,
            (width, height, 1),
        );

        let highres_frame =
            Self::create_image(gl, glow::TEXTURE_2D, glow::RGBA8, (width, height, 1));

        // software depth, not hw
        let far_depth = Self::create_image(gl, glow::TEXTURE_2D, glow::R32F, (width, height, 1));

        // software depth, not hw
        let near_depth = Self::create_image(gl, glow::TEXTURE_2D, glow::R32F, (width, height, 1));

        AllSwapchainDependentImages {
            highres_frame,
            highres_depth_stencil,
            highres_mat_norm,
            far_depth,
            near_depth,
        }
    }

    #[cold]
    #[optimize(size)]
    pub fn destroy_independent_images(
        gl: &glow::Context,
        independent_images: AllIndependentImages,
    ) {
        println!("started destroying independent images");
        unsafe {
            gl.delete_texture(independent_images.grass_state);
            gl.delete_texture(independent_images.water_state);
            gl.delete_texture(independent_images.perlin_noise2d);
            gl.delete_texture(independent_images.perlin_noise3d);
            gl.delete_texture(independent_images.world);
            gl.delete_texture(independent_images.radiance_cache);
            gl.delete_texture(independent_images.origin_block_palette);
            gl.delete_texture(independent_images.material_palette);
            gl.delete_texture(independent_images.lightmap);
        }
        println!("destroyed independent images");
    }

    #[cold]
    #[optimize(size)]
    pub fn destroy_dependent_images(
        gl: &glow::Context,
        dependent_images: AllSwapchainDependentImages,
    ) {
        println!("started destroying swapchain dependent images");

        // swapchain images are destroyed by the driver
        unsafe {
            gl.delete_texture(dependent_images.highres_frame);
            gl.delete_texture(dependent_images.highres_depth_stencil);
            gl.delete_texture(dependent_images.highres_mat_norm);
            gl.delete_texture(dependent_images.far_depth);
            gl.delete_texture(dependent_images.near_depth);
        }
        println!("destroyed swapchain dependent images");
    }
}
