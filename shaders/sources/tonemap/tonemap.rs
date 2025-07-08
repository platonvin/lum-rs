#![no_std]
use common::*;

pub(crate) fn luminance(v: Vec3) -> f32 {
    v.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

pub(crate) fn change_luminance(c_in: Vec3, l_out: f32) -> Vec3 {
    let l_in = luminance(c_in);
    if l_in.abs() < f32::EPSILON {
        return c_in;
    }
    c_in * (l_out / l_in)
}

pub(crate) fn reinhard_extended(v: Vec3, max_white: f32) -> Vec3 {
    let numerator = v * (1.0 + (v / Vec3::splat(max_white * max_white)));
    numerator / (1.0 + v)
}

pub(crate) fn reinhard_extended_luminance(v: Vec3, max_white_l: f32) -> Vec3 {
    let l_old = luminance(v);
    let numerator = l_old * (1.0 + (l_old / (max_white_l * max_white_l)));
    let l_new = numerator / (1.0 + l_old);
    change_luminance(v, l_new)
}

pub(crate) fn uncharted2_tonemap_partial(x: Vec3) -> Vec3 {
    const A: f32 = 0.15;
    const B: f32 = 0.50;
    const C: f32 = 0.10;
    const D: f32 = 0.20;
    const E: f32 = 0.02;
    const F: f32 = 0.30;
    ((x * (A * x + C * B) + D * E) / (x * (A * x + B) + D * F)) - E / F
}

pub(crate) fn uncharted2_filmic(v: Vec3) -> Vec3 {
    let exposure_bias = 2.0;
    let curr = uncharted2_tonemap_partial(v * exposure_bias);

    let w = Vec3::splat(11.2);
    let white_scale = Vec3::splat(1.0) / uncharted2_tonemap_partial(w);
    curr * white_scale
}

pub(crate) fn aces_approx(v: Vec3) -> Vec3 {
    let v_scaled = v * 0.6;
    const A: f32 = 2.51;
    const B: f32 = 0.03;
    const C: f32 = 2.43;
    const D: f32 = 0.59;
    const E: f32 = 0.14;
    ((v_scaled * (A * v_scaled + B)) / (v_scaled * (C * v_scaled + D) + E))
        .clamp(Vec3::ZERO, Vec3::splat(1.0))
}

pub(crate) fn tonemap(color: Vec3) -> Vec3 {
    reinhard_extended_luminance(color, 5.0)
}

pub(crate) fn adjust_brightness(color: Vec3, value: f32) -> Vec3 {
    color + value
}

pub(crate) fn adjust_contrast(color: Vec3, value: f32) -> Vec3 {
    0.5 + (1.0 + value) * (color - 0.5)
}

pub(crate) fn adjust_exposure(color: Vec3, value: f32) -> Vec3 {
    (1.0 + value) * color
}

pub(crate) fn adjust_saturation(color: Vec3, value: f32) -> Vec3 {
    let grayscale = luminance(color);
    Vec3::splat(grayscale).lerp(color, 1.0 + value)
}

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn tonemap_frag(
    #[spirv(input_attachment_index = 0, descriptor_set = 0, binding = 0)] rendered_frame: &Image!(subpass, type=f32, sampled=false),
    frame_color: &mut Vec4,
) {
    let mut color = decode_color(rendered_frame.read_subpass(IVec2::ZERO).xyz());
    // color = adjust_saturation(color, 0.1);
    // color = adjust_contrast(color, 0.1);
    color = adjust_exposure(color, 0.5);
    color = tonemap(color);

    *frame_color = color.extend(1.0);
}
