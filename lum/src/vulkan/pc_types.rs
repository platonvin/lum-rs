// all the push constants need C repr and ability to be casted into bytes

use as_u8_slice_derive::AsU8Slice;

use crate::types::{i16vec3, i16vec4, ivec4, mat4, quat, u8vec4, vec2, vec4, MeshBlock};

#[repr(C)]
#[derive(AsU8Slice)]
pub struct Radiance {
    pub time: i32,
    pub iters: i32,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct Map {
    pub trans: mat4,
    pub shift: ivec4,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct LightmapUBO {
    pub trans: mat4,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct UBO {
    pub trans_w2s: mat4,
    pub campos: vec4,
    pub camdir: vec4,
    pub horizline_scaled: vec4,
    pub vertiline_scaled: vec4,
    pub global_light_dir: vec4,
    pub lightmap_proj: mat4,
    pub size: vec2,
    pub timeseed: i32,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct RaygenBlockPerBlock {
    // block: BlockID_t, // passed before separately
    // shift: i16vec3, // passed before separately
    pub inorm: u8vec4,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct RaygenBlockPerFace {
    pub block: MeshBlock,
    pub shift: i16vec3,
    // inorm: i8vec4, // passed separately
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct RaygenModelPerFace {
    // rot: quat - passed per model
    // shift: vec4 - passed per model
    pub inorm: vec4,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct RaygenModelPerModel {
    pub rot: quat,
    pub shift: vec4,
    // inorm: vec4 - passed per face
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct LightmapBlock {
    pub shift: i16vec4,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct LightmapModel {
    pub rot: quat,
    pub shift: vec4,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct Grass {
    pub wind_direction: vec2,
    pub _wtf_is_this: vec2,
    pub time: f32,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct WaterUpdate {
    pub wind_direction: vec2,
    pub time: f32,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct RaygenMapGrass {
    pub shift: vec4,
    pub _size: i32,
    pub _time: i32,
    pub xf: i32,
    pub yf: i32,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct RaygenMapWater {
    pub shift: vec4,
    pub _size: i32,
    pub _time: i32,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct Diffuse {
    pub v1: vec4,
    pub v2: vec4,
    pub lp: mat4,
}

#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct RaygenMapSmoke {
    pub center_size: vec4,
}
#[repr(C)] // for push constants
#[derive(AsU8Slice)] // allow cast to &[u8]
pub struct Glossy {
    pub v1: vec4,
    pub v2: vec4,
}
