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
    shift: vec4<f32>,
    time_size: vec4<i32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) orig: vec3<f32>,
}

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(0) @binding(1) var state: texture_2d<f32>; 
@group(0) @binding(2) var linear_samp_tiled: sampler;

@group(1) @binding(0) var<storage, read> pco_shared: array<PushConstants>;

fn rand(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn get_height(globalpos: vec2<f32>, time: f32) -> f32 {
    var total_height = 0.0;

    let uv1 = globalpos / 13.0;
    let uv2 = globalpos / 31.0;
    let uv3 = globalpos / 35.0;
    let uv4 = globalpos / 42.0;

    total_height += textureSampleLevel(state, linear_samp_tiled, uv1, 0.0).x * (13.0 / 55.0);
    total_height += textureSampleLevel(state, linear_samp_tiled, uv2, 0.0).y * (31.0 / 55.0);
    total_height += textureSampleLevel(state, linear_samp_tiled, uv3, 0.0).z * (35.0 / 55.0);
    total_height += textureSampleLevel(state, linear_samp_tiled, uv4, 0.0).w * (42.0 / 55.0);

    return total_height;
}

fn make_offset(globalpos: vec2<f32>, offset_pixels: vec2<i32>) -> f32 {
    var s = 0.0;
    let texture_dim_f = vec2<f32>(textureDimensions(state, 0u));
    let offset_uv = vec2<f32>(offset_pixels) / texture_dim_f;

    let base_uv1 = globalpos / 13.0;
    let base_uv2 = globalpos / 31.0;
    let base_uv3 = globalpos / 35.0;
    let base_uv4 = globalpos / 42.0;

    s += textureSampleLevel(state, linear_samp_tiled, base_uv1 + offset_uv, 0.0).x * (13.0 / 55.0);
    s += textureSampleLevel(state, linear_samp_tiled, base_uv2 + offset_uv, 0.0).y * (31.0 / 55.0);
    s += textureSampleLevel(state, linear_samp_tiled, base_uv3 + offset_uv, 0.0).z * (35.0 / 55.0);
    s += textureSampleLevel(state, linear_samp_tiled, base_uv4 + offset_uv, 0.0).w * (42.0 / 55.0);

    return s;
}

fn get_normal(globalpos: vec2<f32>, time: f32) -> vec3<f32> {
    let sample_step_dist = 2.0;

    let offset_xm = vec2<i32>(-1, 0);
    let offset_xp = vec2<i32>(1, 0);
    let offset_ym = vec2<i32>(0, -1);
    let offset_yp = vec2<i32>(0, 1);

    let height_xm = make_offset(globalpos, offset_xm); // (x-1, y)
    let height_xp = make_offset(globalpos, offset_xp); // (x+1, y)
    let height_ym = make_offset(globalpos, offset_ym); // (x, y-1)
    let height_yp = make_offset(globalpos, offset_yp); // (x, y+1)

    // tangent along X-axis
    let va = normalize(vec3<f32>(sample_step_dist, 0.0, height_xp - height_xm));
    // tangent along Y-axis
    let vb = normalize(vec3<f32>(0.0, sample_step_dist, height_yp - height_ym));

    let norm = cross(va, vb);

    return vec3f(1, 0, 0);
}

fn wave_water_vert(pos: vec2<f32>, shift: vec2<f32>, time: f32) -> vec3<f32> {
    let current_global_pos = pos + shift;
    let height = get_height(current_global_pos / 100.0, time);
    // let normal = get_normal(current_global_pos, time);
    return vec3<f32>(height, 0.0, 0.0);
}

fn get_water_vert(vert_index: i32, instance_index: i32, shift: vec2<f32>, pco: PushConstants) -> vec3<f32> {
    var vertex = vec3<f32>(0.0);

    let instance_y_offset = f32(instance_index);
    let y_in_strip = f32(vert_index % 2);
    let x_in_strip = f32((vert_index + 1) / 2);

    let grid_scale = 16.0;
    vertex.x = (x_in_strip / f32(pco.time_size.y)) * grid_scale;
    vertex.y = ((y_in_strip + instance_y_offset) / f32(pco.time_size.y)) * grid_scale;

    let time_for_waves = f32(pco.time_size.x) / 300.0;
    let wave_data = wave_water_vert(vertex.xy, shift, time_for_waves);
    vertex.z = wave_data.x;

    return vertex;
}

@vertex
fn main(@builtin(vertex_index) vert_id: u32, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let batch_index = instance_id / (32);
    let blade_index = instance_id % (32);
    
    let pco = pco_shared[batch_index];
    
    var out: VertexOutput;

    let shift = pco.shift;
    let local_pos = get_water_vert(i32(vert_id), i32(blade_index), shift.xy, pco);

    let world_pos_vec3 = local_pos + shift.xyz;
    let world_pos = vec4<f32>(world_pos_vec3, 1.0);

    var clip_pos = (ubo.trans_w2s * world_pos).xyz;
    clip_pos.z = 1.0 + clip_pos.z;
    out.position = vec4<f32>(clip_pos, 1.0);

    out.orig = world_pos_vec3;

    return out;
}
