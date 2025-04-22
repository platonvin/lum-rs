struct UboData {
    trans_w2s: mat4x4<f32>,
    campos: vec4<f32>,
    camdir: vec4<f32>,
    horizline_scaled: vec4<f32>,
    vertiline_scaled: vec4<f32>,
    global_light_dir: vec4<f32>,
    lightmap_proj: mat4x4<f32>,
    frame_size: vec2<f32>,
    wind_direction: vec2<f32>,
    timeseed: i32,
    delta_time: f32,
};


struct Constants {
    rot: vec4<f32>,
    shift: vec4<f32>,
    fnormal: vec4<f32>, // not encoded
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(1) @binding(0) var<uniform> pco: Constants;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) mat_norm: vec4<u32>,
};

struct FragmentOutput {
    @location(0) outMatNorm: vec4<u32>,
};

@fragment
fn main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.outMatNorm = in.mat_norm;
    return out;
}