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

struct PushConstants {
    rot: vec4<f32>,
    shift: vec4<f32>,
    fnormal: vec4<f32>,
};

struct VertexInput {
    @location(0) @interpolate(flat) posIn: vec3<u32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) sample_point: vec3<f32>,
    @location(1) @interpolate(flat) normal_encoded_packed: u32,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(1) @binding(0) var<storage, read> pco_shared: array<PushConstants>;
// @group(1) @binding(1) var modelVoxels: texture_3d<i32>; 

fn qtransform(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    return v + 2.0 * cross(cross(v, -q.xyz) + q.w * v, -q.xyz);
}

@vertex
fn main(@builtin(instance_index) instance_id: u32, in: VertexInput) -> VertexOutput {
    let pco = pco_shared[instance_id];
    
    let fpos = vec3<f32>(in.posIn);
    let fnorm_ms = normalize(pco.fnormal.xyz);

    let local_pos = qtransform(pco.rot, fpos);

    let world_pos = vec4<f32>(local_pos + pco.shift.xyz, 1.0);

    var clip_coords: vec3<f32> = (ubo.trans_w2s * world_pos).xyz; 
    clip_coords.z = 1.0 + clip_coords.z;

    var out: VertexOutput;
    out.position = vec4<f32>(clip_coords, 1.0);

    let fnorm_ws = qtransform(pco.rot, fnorm_ms); 
    out.normal_encoded_packed = pack4x8unorm(vec4<f32>((fnorm_ws + 1.0) / 2.0, 0.0));
    out.sample_point = fpos - fnorm_ms * 0.5; // for better rounding lol

    return out;
}