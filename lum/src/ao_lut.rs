//! Small crate to generate per-frame constants for accelerating GPU HBAO computation
//! Currently used by both wgpu and Vulkan backends and thus moved here

use super::types::*;
use qvek::vec2;
use std::f32::consts::PI;

fn get_world_shift_from_clip_shift(
    clip_shift: vec2,
    horizline_scaled: vec3,
    vertiline_scaled: vec3,
) -> vec3 {
    horizline_scaled * clip_shift.x + vertiline_scaled * clip_shift.y
}

fn calculate_total_weight(sample_count: usize) -> f32 {
    (1..=sample_count)
        .map(|i| {
            let normalized_radius = i as f32 / sample_count as f32;
            let transformed_radius = normalized_radius * normalized_radius;
            1.0 - transformed_radius
        })
        .sum()
}

/// Generates array of precomputed per-frame constants for our HBAO shader
pub fn generate_lut<const SAMPLE_COUNT: usize>(
    max_radius: f32,
    _frame_size: vec2,
    horizline_scaled: vec3,
    vertiline_scaled: vec3,
) -> [AoLut; SAMPLE_COUNT] {
    let mut lut = [AoLut::default(); SAMPLE_COUNT];
    let total_weight = calculate_total_weight(SAMPLE_COUNT);
    let norm_radius_step = 1.0 / SAMPLE_COUNT as f32;

    lut.iter_mut().enumerate().for_each(|(i, entry)| {
        let angle = (i as f32 + 1.0) * (6.9 * PI) / SAMPLE_COUNT as f32;
        let normalized_radius = (i as f32 + 1.0) * norm_radius_step;
        let radius = normalized_radius.sqrt() * max_radius;
        let screen_shift = vec2!(angle.sin(), angle.cos()) * radius;
        let clip_shift = screen_shift * 2.0;
        let world_shift =
            get_world_shift_from_clip_shift(clip_shift, horizline_scaled, vertiline_scaled);

        let weight = 1.0 - normalized_radius * normalized_radius;
        let weight_normalized = (weight / total_weight) * 0.7;

        *entry = AoLut {
            world_shift,
            weight_normalized,
            screen_shift,
            padding: vec2::zero(),
        };
    });

    lut
}
