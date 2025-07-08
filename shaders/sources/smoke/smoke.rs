#![no_std]
use common::*;

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn smoke_frag(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(descriptor_set = 0, binding = 1, input_attachment_index = 0)]
    smoke_depth_far_subpass: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 2, input_attachment_index = 1)]
    smoke_depth_near_subpass: &Image!(subpass, type=f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 3)] radiance_cache: &Image!(
         3D,
         type=f32,
         sampled = false
     ),
    #[spirv(descriptor_set = 0, binding = 4)] noise_texture: &SampledImage<
        Image!(3D, type=f32, sampled),
    >,
    #[spirv(frag_coord)] gl_frag_coord: Vec4,

    smoke_color: &mut Vec4,
) {
    let direction = ubo.camdir.xyz();

    let near = load_depth_near(smoke_depth_near_subpass);
    let far = load_depth_far(smoke_depth_far_subpass);

    let diff = far - near;

    const MAX_STEPS: i32 = 8;
    let step_size = diff / MAX_STEPS as f32;

    let mut i_attenuation = 1.0;

    let mut position = Vec3::ZERO;
    const TRESHHOLD: f32 = 0.7;
    const MULTIPLIER: f32 = 1.7;

    let mut fraction = near;
    let time = ubo.time_seed;
    for _i in 0..MAX_STEPS {
        fraction += step_size;
        let clip_pos = gl_frag_coord.xy() / ubo.frame_size * 2.0 - 1.0;
        position = get_origin_from_depth2(ubo, fraction, clip_pos);
        let voxel_pos = position;
        let noise_clip_pos = voxel_pos / 32.0;
        let mut noises = Vec4::ZERO;

        let mut wind_direction = vec3(1.0, 0.0, 0.0);
        let wind_rotate: Mat2 = rotate2d(1.6);

        noises.x = noise_texture
            .sample(noise_clip_pos / 1.0 + wind_direction * (time as f32 / 3500.0))
            .x;
        let temp = rotate_vec2(wind_direction.xy(), 1.6);
        wind_direction.x = temp.x;
        wind_direction.y = temp.y;
        noises.y = noise_texture
            .sample(noise_clip_pos / 2.1 + wind_direction * (time as f32 / 3000.0))
            .y;
        let temp = rotate_vec2(wind_direction.xy(), 1.6);
        wind_direction.x = temp.x;
        wind_direction.y = temp.y;
        noises.z = noise_texture
            .sample(noise_clip_pos / 3.2 + wind_direction * (time as f32 / 2500.0))
            .z;
        let temp = rotate_vec2(wind_direction.xy(), 1.6);
        wind_direction.x = temp.x;
        wind_direction.y = temp.y;
        noises.w = noise_texture
            .sample(noise_clip_pos / 4.3 + wind_direction * (time as f32 / 2000.0))
            .w;

        let close_to_border = diff.clamp(0.1, 16.0) / 16.0;

        let mut dencity =
            (noises.x + noises.y + noises.z - noises.w / close_to_border) / 2.0 - TRESHHOLD;

        dencity = dencity.clamp(0.0, TRESHHOLD) * MULTIPLIER;

        i_attenuation = (1.0 - dencity * step_size) * i_attenuation;
    }
    let final_light = sample_radiance_with_normal2(radiance_cache, position, direction);

    let smoke_opacity = 1.0 - i_attenuation;
    *smoke_color = encode_color(final_light).extend(smoke_opacity);
}
