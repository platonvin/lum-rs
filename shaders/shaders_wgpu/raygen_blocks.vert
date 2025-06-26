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
    block: i32,
    shift_x: i32,
    shift_y: i32,
    shift_z: i32,
    FUCKWEB_unorm: u32, 
};

struct VertexInput {
    @location(0) @interpolate(flat) posIn: vec4<u32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(linear) sample_point: vec3<f32>,
    @location(1) @interpolate(flat) bunorm: u32,
};


@group(0) @binding(0) var<uniform> ubo: UboData;
// @group(0) @binding(2) var blockPalette: texture_3d<i32>;
@group(1) @binding(0) var<storage, read> pco_shared: array<PushConstants>;

fn qtransform(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    return v + 2.0 * cross(cross(v, -q.xyz) + q.w * v, -q.xyz);
}

@vertex
fn main(@builtin(instance_index) instance_id: u32, in: VertexInput) -> VertexOutput {
    let pco = pco_shared[instance_id];
    
    let upos = vec3<i32>(in.posIn.xyz);
    let normal_encoded = pco.FUCKWEB_unorm;
    let block = pco.block;
    let shift = vec3<i32>(pco.shift_x, pco.shift_y, pco.shift_z);

    let s = (normal_encoded & (1<<7))>>7; // 0 if position 1 if negative
    let axis = vec3<i32>(
        i32((normal_encoded & (1<<0))>>0),
        i32((normal_encoded & (1<<1))>>1),
        i32((normal_encoded & (1<<2))>>2)
    );
    let inorm = axis * (1 - i32(s) * 2);
    let fnorm = vec3<f32>(inorm);

    let uworld_pos = upos + shift;
    let fworld_pos = vec4<f32>(vec3<f32>(uworld_pos), 1.0);

    var clip_coords: vec3<f32> = (ubo.trans_w2s * fworld_pos).xyz;
    clip_coords.z = 1.0 + clip_coords.z;

    var out: VertexOutput;
    out.position = vec4<f32>(clip_coords, 1.0);
    out.sample_point = vec3<f32>(upos) - fnorm * 0.5; // for better rounding lol

    let sample_block = u32(block);

    out.bunorm = (sample_block & 0xFFFFu) | ((pco.FUCKWEB_unorm & 0xFFFFu) << 16u);
    return out;
}