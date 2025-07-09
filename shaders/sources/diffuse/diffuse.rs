#![no_std]
use common::*;

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn main(
    #[spirv(frag_coord)] in_frag_coord: Vec4,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(descriptor_set = 0, binding = 1, input_attachment_index = 0)] mat_norm: &Image!(subpass, type=u32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 2, input_attachment_index = 1)] depth_buffer: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 3)] voxel_palette: &Image!(2D, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 4)] radiance_cache: &SampledImage<
        Image!(3D, type=f32, sampled),
    >,
    #[spirv(descriptor_set = 0, binding = 5)] lightmap: &SampledImage<
        Image!(2D, type=f32, sampled),
    >,
    out_frame_color: &mut Vec4,
) {
    let stored_mat = get_mat(load_mat(mat_norm), voxel_palette);

    let clip_pos = in_frag_coord.xy() / ubo.frame_size * 2.0 - Vec2::splat(1.0);
    let origin = get_origin_from_depth(load_depth(depth_buffer), clip_pos, ubo);
    let stored_normal = load_norm(mat_norm);

    // let incoming_light = sample_radiance_with_normal(origin + stored_normal * 6.0, stored_normal, radiance_cache);
    let incoming_light = sample_radiance_simple(origin + stored_normal * 6.0, radiance_cache);
    let sunlight = sample_lightmap(origin, stored_normal, ubo, lightmap);

    let final_color = (2.0 * incoming_light + stored_mat.emittance + sunlight) * stored_mat.color;

    *out_frame_color = encode_color(final_color).extend(1.0);
}
