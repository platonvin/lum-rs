use crate::*;

#[repr(C)]
pub struct UpdateGrassPushConstants {
    pub wind_direction: Vec2,
    pub collision_point: Vec2,
    pub time: f32,
}

#[spirv(compute(threads(8, 8, 1)))]
#[unsafe(no_mangle)]
pub fn update_grass_comp(
    #[spirv(descriptor_set = 0, binding = 0)] state_image: &Image!(
        2D,
        format = rg16f,
        sampled = false
    ),
    #[spirv(descriptor_set = 0, binding = 1)] perlin_noise_texture: &SampledImage<
        Image!(2D, type=f32, sampled),
    >,

    #[spirv(push_constant)] pco: &UpdateGrassPushConstants,

    #[spirv(global_invocation_id)] global_invocation_id: UVec3,
) {
    let pix = global_invocation_id.xy().as_ivec2();
    let pos = (pix.as_vec2() + 0.5) / 16.0;

    let mut new_direction = Vec2::ZERO;

    let wind_direction = Vec2::new(1.0, 1.0);

    let uv_shift = wind_direction * pco.time / 300.0;
    let noise_uv = pos + uv_shift;

    new_direction.x = perlin_noise_texture.sample_by_lod(noise_uv, 0.0).x;

    new_direction.y = perlin_noise_texture.sample_by_lod(noise_uv, 0.0).x;

    unsafe {
        state_image.write(pix, new_direction.extend(0.0).extend(0.0));
    }
}
