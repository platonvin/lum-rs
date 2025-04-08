use glow::HasContext;

use crate::internal_renderer::render_gl::{AllSamplers, InternalRendererGL};

impl InternalRendererGL {
    #[cold]
    #[optimize(size)]
    pub fn create_all_samplers(gl: &glow::Context) -> AllSamplers {
        unsafe {
            let mut base_sampler = gl.create_sampler().unwrap();
            gl.sampler_parameter_i32(base_sampler, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
            gl.sampler_parameter_i32(base_sampler, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            gl.sampler_parameter_i32(
                base_sampler,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_i32(
                base_sampler,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_i32(
                base_sampler,
                glow::TEXTURE_WRAP_R,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_f32(base_sampler, glow::TEXTURE_LOD_BIAS, 0.0);
            gl.sampler_parameter_f32(base_sampler, glow::TEXTURE_MAX_ANISOTROPY, 1.0);
            gl.sampler_parameter_i32(base_sampler, glow::TEXTURE_COMPARE_MODE, glow::NONE as i32);
            gl.sampler_parameter_f32(base_sampler, glow::TEXTURE_MIN_LOD, 0.0);
            gl.sampler_parameter_f32(base_sampler, glow::TEXTURE_MAX_LOD, 0.0);
            gl.sampler_parameter_f32_slice(
                base_sampler,
                glow::TEXTURE_BORDER_COLOR,
                &[0.0, 0.0, 0.0, 1.0],
            ); // FLOAT_OPAQUE_BLACK

            // Nearest Sampler
            let nearest_sampler = base_sampler;
            base_sampler = gl.create_sampler().unwrap(); // Create a new base sampler for the next one
            gl.sampler_parameter_i32(base_sampler, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
            gl.sampler_parameter_i32(base_sampler, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            gl.sampler_parameter_i32(
                base_sampler,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_i32(
                base_sampler,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_i32(
                base_sampler,
                glow::TEXTURE_WRAP_R,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_f32(base_sampler, glow::TEXTURE_LOD_BIAS, 0.0);
            gl.sampler_parameter_f32(base_sampler, glow::TEXTURE_MAX_ANISOTROPY, 1.0);
            gl.sampler_parameter_i32(base_sampler, glow::TEXTURE_COMPARE_MODE, glow::NONE as i32);
            gl.sampler_parameter_f32(base_sampler, glow::TEXTURE_MIN_LOD, 0.0);
            gl.sampler_parameter_f32(base_sampler, glow::TEXTURE_MAX_LOD, 0.0);
            gl.sampler_parameter_f32_slice(
                base_sampler,
                glow::TEXTURE_BORDER_COLOR,
                &[0.0, 0.0, 0.0, 1.0],
            );

            // Linear Sampler
            let linear_sampler = gl.create_sampler().unwrap();
            gl.sampler_parameter_i32(
                linear_sampler,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.sampler_parameter_i32(
                linear_sampler,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.sampler_parameter_i32(
                linear_sampler,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_i32(
                linear_sampler,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_i32(
                linear_sampler,
                glow::TEXTURE_WRAP_R,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_f32(linear_sampler, glow::TEXTURE_LOD_BIAS, 0.0);
            gl.sampler_parameter_f32(linear_sampler, glow::TEXTURE_MAX_ANISOTROPY, 1.0);
            gl.sampler_parameter_i32(
                linear_sampler,
                glow::TEXTURE_COMPARE_MODE,
                glow::NONE as i32,
            );
            gl.sampler_parameter_f32(linear_sampler, glow::TEXTURE_MIN_LOD, 0.0);
            gl.sampler_parameter_f32(linear_sampler, glow::TEXTURE_MAX_LOD, 0.0);
            gl.sampler_parameter_f32_slice(
                linear_sampler,
                glow::TEXTURE_BORDER_COLOR,
                &[0.0, 0.0, 0.0, 1.0],
            );

            // Linear Tiled Sampler
            let linear_sampler_tiled = gl.create_sampler().unwrap();
            gl.sampler_parameter_i32(
                linear_sampler_tiled,
                glow::TEXTURE_WRAP_S,
                glow::MIRRORED_REPEAT as i32,
            );
            gl.sampler_parameter_i32(
                linear_sampler_tiled,
                glow::TEXTURE_WRAP_T,
                glow::MIRRORED_REPEAT as i32,
            );
            gl.sampler_parameter_i32(
                linear_sampler_tiled,
                glow::TEXTURE_WRAP_R,
                glow::MIRRORED_REPEAT as i32,
            );
            gl.sampler_parameter_i32(
                linear_sampler_tiled,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.sampler_parameter_i32(
                linear_sampler_tiled,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.sampler_parameter_f32(linear_sampler_tiled, glow::TEXTURE_LOD_BIAS, 0.0);
            gl.sampler_parameter_f32(linear_sampler_tiled, glow::TEXTURE_MAX_ANISOTROPY, 1.0);
            gl.sampler_parameter_i32(
                linear_sampler_tiled,
                glow::TEXTURE_COMPARE_MODE,
                glow::NONE as i32,
            );
            gl.sampler_parameter_f32(linear_sampler_tiled, glow::TEXTURE_MIN_LOD, 0.0);
            gl.sampler_parameter_f32(linear_sampler_tiled, glow::TEXTURE_MAX_LOD, 0.0);
            gl.sampler_parameter_f32_slice(
                linear_sampler_tiled,
                glow::TEXTURE_BORDER_COLOR,
                &[0.0, 0.0, 0.0, 1.0],
            );
            let linear_sampler_tiled_mirrored = linear_sampler_tiled; // They have the same settings

            // Overlay Sampler
            let overlay_sampler = gl.create_sampler().unwrap();
            gl.sampler_parameter_i32(
                overlay_sampler,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.sampler_parameter_i32(
                overlay_sampler,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.sampler_parameter_i32(
                overlay_sampler,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_i32(
                overlay_sampler,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_i32(
                overlay_sampler,
                glow::TEXTURE_WRAP_R,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.sampler_parameter_f32(overlay_sampler, glow::TEXTURE_LOD_BIAS, 0.0);
            gl.sampler_parameter_f32(overlay_sampler, glow::TEXTURE_MAX_ANISOTROPY, 1.0);
            gl.sampler_parameter_i32(
                overlay_sampler,
                glow::TEXTURE_COMPARE_MODE,
                glow::NONE as i32,
            );
            gl.sampler_parameter_f32(overlay_sampler, glow::TEXTURE_MIN_LOD, 0.0);
            gl.sampler_parameter_f32(overlay_sampler, glow::TEXTURE_MAX_LOD, 0.0);
            gl.sampler_parameter_f32_slice(
                overlay_sampler,
                glow::TEXTURE_BORDER_COLOR,
                &[0.0, 0.0, 0.0, 1.0],
            );

            // Unnormalized Linear Sampler
            let unnorm_linear = gl.create_sampler().unwrap();
            gl.sampler_parameter_i32(unnorm_linear, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.sampler_parameter_i32(unnorm_linear, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.sampler_parameter_i32(unnorm_linear, glow::TEXTURE_WRAP_R, glow::REPEAT as i32);
            gl.sampler_parameter_i32(unnorm_linear, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.sampler_parameter_i32(unnorm_linear, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.sampler_parameter_f32(unnorm_linear, glow::TEXTURE_LOD_BIAS, 0.0);
            gl.sampler_parameter_f32(unnorm_linear, glow::TEXTURE_MAX_ANISOTROPY, 1.0);
            gl.sampler_parameter_i32(unnorm_linear, glow::TEXTURE_COMPARE_MODE, glow::NONE as i32);
            gl.sampler_parameter_f32(unnorm_linear, glow::TEXTURE_MIN_LOD, 0.0);
            gl.sampler_parameter_f32(unnorm_linear, glow::TEXTURE_MAX_LOD, 0.0);
            gl.sampler_parameter_f32_slice(
                unnorm_linear,
                glow::TEXTURE_BORDER_COLOR,
                &[0.0, 0.0, 0.0, 1.0],
            );

            // Unnormalized Nearest Sampler
            let unnorm_nearest = gl.create_sampler().unwrap();
            gl.sampler_parameter_i32(unnorm_nearest, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.sampler_parameter_i32(unnorm_nearest, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.sampler_parameter_i32(unnorm_nearest, glow::TEXTURE_WRAP_R, glow::REPEAT as i32);
            gl.sampler_parameter_i32(
                unnorm_nearest,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.sampler_parameter_i32(
                unnorm_nearest,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.sampler_parameter_f32(unnorm_nearest, glow::TEXTURE_LOD_BIAS, 0.0);
            gl.sampler_parameter_f32(unnorm_nearest, glow::TEXTURE_MAX_ANISOTROPY, 1.0);
            gl.sampler_parameter_i32(
                unnorm_nearest,
                glow::TEXTURE_COMPARE_MODE,
                glow::NONE as i32,
            );
            gl.sampler_parameter_f32(unnorm_nearest, glow::TEXTURE_MIN_LOD, 0.0);
            gl.sampler_parameter_f32(unnorm_nearest, glow::TEXTURE_MAX_LOD, 0.0);
            gl.sampler_parameter_f32_slice(
                unnorm_nearest,
                glow::TEXTURE_BORDER_COLOR,
                &[0.0, 0.0, 0.0, 1.0],
            );

            // shadowmap Sampler with possible hw depth comparison filtering
            let shadow_sampler = gl.create_sampler().unwrap();
            gl.sampler_parameter_i32(
                shadow_sampler,
                glow::TEXTURE_WRAP_S,
                glow::MIRRORED_REPEAT as i32,
            );
            gl.sampler_parameter_i32(
                shadow_sampler,
                glow::TEXTURE_WRAP_T,
                glow::MIRRORED_REPEAT as i32,
            );
            gl.sampler_parameter_i32(
                shadow_sampler,
                glow::TEXTURE_WRAP_R,
                glow::MIRRORED_REPEAT as i32,
            );
            gl.sampler_parameter_i32(
                shadow_sampler,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.sampler_parameter_i32(
                shadow_sampler,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.sampler_parameter_i32(
                shadow_sampler,
                glow::TEXTURE_COMPARE_MODE,
                glow::COMPARE_REF_TO_TEXTURE as i32,
            );
            gl.sampler_parameter_f32_slice(
                shadow_sampler,
                glow::TEXTURE_BORDER_COLOR,
                &[1.0, 1.0, 1.0, 1.0],
            ); // FLOAT_OPAQUE_WHITE
            gl.sampler_parameter_i32(
                shadow_sampler,
                glow::TEXTURE_COMPARE_FUNC,
                glow::LESS as i32,
            );
            gl.sampler_parameter_f32(shadow_sampler, glow::TEXTURE_LOD_BIAS, 0.0);
            gl.sampler_parameter_f32(shadow_sampler, glow::TEXTURE_MAX_ANISOTROPY, 1.0);
            gl.sampler_parameter_f32(shadow_sampler, glow::TEXTURE_MIN_LOD, 0.0);
            gl.sampler_parameter_f32(shadow_sampler, glow::TEXTURE_MAX_LOD, 0.0);

            AllSamplers {
                nearest_sampler,
                linear_sampler,
                linear_sampler_tiled,
                linear_sampler_tiled_mirrored,
                overlay_sampler,
                shadow_sampler,
                unnorm_linear,
                unnorm_nearest,
            }
        }
    }

    pub fn destroy_all_samplers(gl: &glow::Context, samplers: AllSamplers) {
        unsafe {
            gl.delete_sampler(samplers.nearest_sampler);
            gl.delete_sampler(samplers.linear_sampler);
            gl.delete_sampler(samplers.linear_sampler_tiled);
            gl.delete_sampler(samplers.linear_sampler_tiled_mirrored);
            gl.delete_sampler(samplers.overlay_sampler);
            gl.delete_sampler(samplers.shadow_sampler);
            gl.delete_sampler(samplers.unnorm_linear);
            gl.delete_sampler(samplers.unnorm_nearest);
        }
    }
}
