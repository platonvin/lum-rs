use as_u8_slice_derive::AsU8Slice;
use qvek::vek::{vec2::Vec2, vec4::Vec4, Mat4};

#[repr(C)]
#[derive(Clone, Copy, AsU8Slice)]
pub struct UboData {
    pub trans_w2s: Mat4<f32>,
    pub campos: Vec4<f32>,
    pub camdir: Vec4<f32>,
    pub horizline_scaled: Vec4<f32>,
    pub vertiline_scaled: Vec4<f32>,
    pub global_light_dir: Vec4<f32>,
    pub lightmap_proj: Mat4<f32>,
    pub frame_size: Vec2<f32>,
    pub wind_direction: Vec2<f32>,
    pub timeseed: i32,
    pub delta_time: f32,
    pub _pad_1: i32,
    pub _pad_2: i32,
}
