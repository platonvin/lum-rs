#![no_std]
use common::*;

#[repr(C)]
pub struct UpdateWaterPushConstants {
    pub wind_direction: Vec2,
    pub time: f32,
}

pub(crate) fn calculate_height(local_pos: Vec2, time: f32) -> f32 {
    let mut height = 0.0;

    let mut direction = Vec2::new(1.0, 1.0);
    let mut ampl = 1.0;
    let mut freq = 1.0;

    for _ in 0..22 {
        if freq >= 20.0 {
            break;
        }

        ampl *= 0.8;
        direction.x += 0.1;
        direction = direction.normalize();
        let t_val = time;

        height += ampl * (t_val * freq + direction.dot(local_pos) * 2.0 * PI).sin();
        freq *= 1.15;
    }
    height
}

#[spirv(compute(threads(8, 8, 1)))]
#[unsafe(no_mangle)]
pub fn update_water_comp(
    #[spirv(descriptor_set = 0, binding = 0)] heighmap: &Image!(
        2D,
        format = rgba16f,
        sampled = false
    ),

    #[spirv(push_constant)] pco: &UpdateWaterPushConstants,

    #[spirv(global_invocation_id)] global_invocation_id: UVec3,
) {
    let pix = global_invocation_id.xy().as_ivec2();
    let pos = (pix.as_vec2() + 0.5) / (48.0 * 2.0);

    let mut height_channels = Vec4::ZERO;
    let time_scaled = pco.time / 300.0;

    height_channels.x = calculate_height(pos / 1.01, time_scaled / 1.25);
    height_channels.y = calculate_height(pos / 1.02, time_scaled / 1.5);
    height_channels.z = calculate_height(pos / 1.03, time_scaled / 2.5);
    height_channels.w = calculate_height(pos / 1.04, time_scaled / 3.5);

    unsafe {
        heighmap.write(pix, height_channels);
    }
}
