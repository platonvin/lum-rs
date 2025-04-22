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
    shift: vec4<f32>,
    time: i32,
    size: i32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) orig: vec3<f32>,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(0) @binding(1) var state: texture_2d<f32>;
@group(0) @binding(2) var linear_samp: sampler;

@group(1) @binding(0) var<uniform> pco: Constants;

const BLOCK_PALETTE_SIZE_X: i32 = 64;
const STATIC_BLOCK_COUNT: i32 = 15;
const PI: f32 = 3.1415926535;
const LODS: i32 = 6;
const BLADES_PER_INSTANCE: i32 = 1;
const VERTICES_PER_BLADE: i32 = 11;
const MAX_HEIGHT: f32 = 5.0;

fn rand(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn get_height(globalpos: vec2<f32>, time: f32) -> f32 {
    var total_height = 0.0;
    let uv1 = globalpos / 13.0;
    let uv2 = globalpos / 31.0;
    let uv3 = globalpos / 35.0;
    let uv4 = globalpos / 42.0;

    total_height += textureSampleLevel(state, linear_samp, uv1, 0.0).x * (13.0 / 55.0);
    total_height += textureSampleLevel(state, linear_samp, uv2, 0.0).y * (31.0 / 55.0);
    total_height += textureSampleLevel(state, linear_samp, uv3, 0.0).z * (35.0 / 55.0);
    total_height += textureSampleLevel(state, linear_samp, uv4, 0.0).w * (42.0 / 55.0);

    return total_height / 1.0;
}

fn make_offset(globalpos: vec2<f32>, offset: vec2<i32>) -> f32 {
    var s = 0.0;
    let off_f = vec2<f32>(offset);
    let texture_size = textureDimensions(state, 0);
    let uv1 = globalpos / 13.0 + off_f / vec2<f32>(texture_size);
    let uv2 = globalpos / 31.0 + off_f / vec2<f32>(texture_size);
    let uv3 = globalpos / 35.0 + off_f / vec2<f32>(texture_size);
    let uv4 = globalpos / 42.0 + off_f / vec2<f32>(texture_size);

    s += textureSampleLevel(state, linear_samp, uv1, 0.0).x * (13.0 / 55.0);
    s += textureSampleLevel(state, linear_samp, uv2, 0.0).y * (31.0 / 55.0);
    s += textureSampleLevel(state, linear_samp, uv3, 0.0).z * (35.0 / 55.0);
    s += textureSampleLevel(state, linear_samp, uv4, 0.0).w * (42.0 / 55.0);
    return s;
}

fn get_normal(globalpos: vec2<f32>, time: f32) -> vec3<f32> {
    let size = vec2<f32>(2.0, 0.0);
    let off = array<vec2<i32>, 3>(vec2<i32>(-1, 0), vec2<i32>(1, 0), vec2<i32>(0, -1));

    let s01 = make_offset(globalpos, off[0]);
    let s21 = make_offset(globalpos, off[1]);
    let s10 = make_offset(globalpos, off[2]);
    let s12 = make_offset(globalpos + vec2<f32>(0.0, 2.0), vec2<i32>(0, 0)); // Approximate s12

    let va = normalize(vec3<f32>(size.x, 0.0, s21 - s01));
    let vb = normalize(vec3<f32>(0.0, size.x, s12 - s10));
    let norm = cross(va, vb);

    return norm;
}

fn wave_water_vert(pos: vec2<f32>, shift: vec2<f32>, time: f32) -> vec3<f32> {
    let height = get_height(pos + shift, time);
    let normal = get_normal(pos + shift, time);
    return vec3<f32>(height, normal.xy); // Returning height and xy of normal
}

fn get_water_vert(vert_index: i32, instance_index: i32, shift: vec2<f32>) -> vec3<f32> {
    var vertex = vec3<f32>(0.0);

    let instance_y_shift = f32(instance_index);
    let y_shift = f32(vert_index % 2);
    let x_shift = f32((vert_index + 1) / 2);
    vertex.x = (x_shift / f32(pco.size)) * 16.0;
    vertex.y = (y_shift + instance_y_shift) / f32(pco.size) * 16.0;

    let wave_data = wave_water_vert(vertex.xy, shift, f32(pco.time) / 300.0);
    vertex.z = wave_data.x; // Height
    // Discarding normal here as fragment shader calculates it

    return vertex;
}

@vertex
fn main(@builtin(vertex_index) vert_id: u32, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let rel2world = get_water_vert(i32(vert_id), i32(instance_id), pco.shift.xy);

    let world_pos = vec4<f32>(rel2world, 1.0) + pco.shift;
    let clip_coords = (ubo.trans_w2s * world_pos).xyz;

    var out: VertexOutput;
    out.position = vec4<f32>(clip_coords, 1.0);
    out.orig = world_pos.xyz;
    return out;
}