#![no_std]
use common::*;

pub fn ssr_intersects(
    test_depth: f32,
    pix: Vec2,
    depth_buffer: &Image!(2D, type=f32, sampled),
    size: IVec2,
) -> (bool, bool) {
    let depth = load_depth_glossy(depth_buffer, pix.as_ivec2());
    let diff = test_depth - depth;
    let ssr = diff > 0.1;
    let smooth_intersection = diff < 0.2;
    (ssr, smooth_intersection)
}

pub fn ssr_trace_ray(
    origin: Vec3,
    direction: Vec3,
    ubo: &UniformBufferObject,
    mat_norm: &Image!(2D, type=u32, sampled=false),
    depth_buffer: &Image!(2D, type=f32, sampled),
    voxel_palette: &Image!(2D, type=f32, sampled=false),
    size: IVec2,
) -> Option<(f32, Vec3, Material, Vec2, f32)> {
    let mut fraction = 0.15;
    let fraction_step = 0.15;

    let horizline = ubo.horizline_scaled.xyz().normalize();
    let vertiline = ubo.vertiline_scaled.xyz().normalize();

    let dir_proj_to_screen = Vec2::new(direction.dot(horizline), direction.dot(vertiline));
    let dir_proj_to_camera_dir = -direction.dot(ubo.camdir.xyz());

    let mut current_pixel = origin.xy();
    let mut current_depth = 0.0;

    let clip_pos_from_origin = (ubo.trans_w2s * origin.extend(1.0)).xyz();
    current_pixel = ((clip_pos_from_origin.xy() + Vec2::splat(1.0)) * 0.5) * size.as_vec2();
    current_depth = clip_pos_from_origin.z * 1000.0;

    loop {
        current_pixel += dir_proj_to_screen * fraction;
        current_depth += dir_proj_to_camera_dir * fraction;

        let (ssr, smooth_intersection) =
            ssr_intersects(current_depth, current_pixel, depth_buffer, size);

        if ssr {
            if smooth_intersection {
                let normal = load_norm_glossy(mat_norm, current_pixel.as_ivec2());
                let material = get_mat(
                    load_mat_glossy(mat_norm, current_pixel.as_ivec2()),
                    voxel_palette,
                );
                return Some((fraction, normal, material, current_pixel, current_depth));
            } else {
                return None;
            }
        }
        fraction += fraction_step;
        if fraction >= 1.0 {
            return None;
        }
    }
}

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn glossy_frag(
    #[spirv(frag_coord)] in_frag_coord: Vec4,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(descriptor_set = 0, binding = 1)] mat_norm: &Image!(2D, type=u32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 2)] depth_buffer: &Image!(2D, type=f32, sampled),
    #[spirv(descriptor_set = 0, binding = 3)] blocks: &Image!(3D, type=i32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 4)] block_palette: &Image!(3D, type=u32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 5)] voxel_palette: &Image!(2D, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 6)] radiance_cache: &SampledImage<
        Image!(3D, type=f32, sampled),
    >,
    out_frame_color: &mut Vec4,
) {
    let pix = in_frag_coord.xy().as_ivec2();

    let mat = get_mat(load_mat_glossy(mat_norm, pix), voxel_palette);
    let mut direction = ubo.camdir.xyz();

    let clip_pos = in_frag_coord.xy() / ubo.frame_size * 2.0 - Vec2::splat(1.0);
    let mut origin = get_origin_from_depth(load_depth_glossy(depth_buffer, pix), clip_pos, ubo);
    let normal = load_norm_glossy(mat_norm, pix);

    let mut accumulated_light = Vec3::ZERO;
    let mut accumulated_reflection = Vec3::splat(1.0);

    process_hit(
        &mut origin,
        &mut direction,
        0.0,
        normal,
        mat,
        &mut accumulated_light,
        &mut accumulated_reflection,
        radiance_cache,
    );

    let traced_color = trace_glossy_ray(
        origin,
        direction,
        accumulated_light,
        accumulated_reflection,
        blocks,
        block_palette,
        voxel_palette,
        radiance_cache,
        ubo,
        WORLD_SIZE,
    );

    *out_frame_color = encode_color(traced_color).extend(1.0 - mat.roughness);
}
