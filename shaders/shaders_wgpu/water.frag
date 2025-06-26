const BLOCK_PALETTE_SIZE_X: i32 = 64;
const STATIC_BLOCK_COUNT: i32 = 15;
const PI: f32 = 3.1415926535;
const LODS: i32 = 6;
const MAX_HEIGHT: f32 = 5.0;

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

// struct Constants {
//     shift: vec4<f32>,
//     time: i32,
//     size: i32,
// };

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) orig: vec3<f32>,
};

struct FragmentOutput {
    @location(0) @interpolate(flat) outMatNorm: vec4<u32>,
};

// @group(0) @binding(0) var<uniform> ubo: UboData;
// @group(0) @binding(1) var state: texture_2d<f32>;
// @group(0) @binding(2) var<push_constant> pco: Constants;

@fragment
fn main(in: VertexOutput) -> FragmentOutput {
    // let normal = normalize(cross(dpdxFine(in.orig), dpdyFine(in.orig)));
    let normal = vec3f(0, 0, 1);
    let normal_encoded = vec3<u32>(((normal + 1.0) / 2.0) * 255.0);

    let water_mat_id = 30u;

    var out: FragmentOutput;
    out.outMatNorm = vec4<u32>(water_mat_id, normal_encoded.x, normal_encoded.y, normal_encoded.z);
    return out;
}
