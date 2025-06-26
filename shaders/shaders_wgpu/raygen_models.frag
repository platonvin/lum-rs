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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) sample_point: vec3<f32>,
    @location(1) @interpolate(flat) normal_encoded_packed: u32,
};

struct FragmentOutput {
    @location(0) @interpolate(flat) outMatNorm: vec4<u32>,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
// @group(1) @binding(0) var<storage, read> pco_shared: array<Constants>;
@group(1) @binding(1) var modelVoxels: texture_3d<i32>; 

fn GetModelVoxel(relative_voxel_pos: vec3<i32>) -> u32 {
    return u32(textureLoad(modelVoxels, relative_voxel_pos, 0).r);
}

@fragment
fn main(in: VertexOutput) -> FragmentOutput {
    let normal_encoded_packed_in = in.normal_encoded_packed;
    let normal_encoded = vec3<u32>(
        (normal_encoded_packed_in >> 0u) & 255u,
        (normal_encoded_packed_in >> 8u) & 255u,
        (normal_encoded_packed_in >> 16u) & 255u,
    );

    var out: FragmentOutput;

    let ipos = vec3<i32>(floor(in.sample_point));
    let voxel = GetModelVoxel(ipos);
    out.outMatNorm = vec4<u32>(voxel, normal_encoded);

    return out;
}