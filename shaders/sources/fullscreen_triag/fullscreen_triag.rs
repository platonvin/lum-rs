#![no_std]
use common::*;

#[spirv(vertex)]
pub fn fullscreen_triag_vert(
    #[spirv(vertex_index)] vertex_id: u32,
    #[spirv(position)] clip_pos: &mut Vec4,
) {
    let out_uv = Vec2::new(((vertex_id << 1) & 2) as f32, (vertex_id & 2) as f32);
    *clip_pos = (out_uv * 2.0 - Vec2::splat(1.0)).extend(0.0).extend(1.0);
}
