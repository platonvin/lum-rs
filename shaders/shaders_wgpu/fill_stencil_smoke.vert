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
    originSize: vec4<f32>, 
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) end_depth: f32, 
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(1) @binding(0) var<storage, read> pco_shared: array<PushConstants>;

const vertices = array(
    vec3(0.0, 1.0, 1.0), vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 0.0),
    vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), vec3(0.0, 1.0, 1.0),
    vec3(1.0, 0.0, 0.0), vec3(1.0, 1.0, 0.0), vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0), vec3(1.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0),
    vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(1.0, 0.0, 1.0),
    vec3(1.0, 0.0, 1.0), vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, 0.0),
    vec3(1.0, 1.0, 1.0), vec3(1.0, 1.0, 0.0), vec3(0.0, 1.0, 0.0),
    vec3(0.0, 1.0, 0.0), vec3(0.0, 1.0, 1.0), vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 0.0),
    vec3(0.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(1.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 1.0), vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0), vec3(0.0, 1.0, 1.0), vec3(0.0, 0.0, 1.0)
);

@vertex
fn main(@builtin(vertex_index) vertex_idx: u32, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let pco = pco_shared[instance_id];
    
    var output: VertexOutput;

    let vertex = vertices[vertex_idx];

    let scaled_vertex = vertex * pco.originSize.w;

    let world_pos = vec4<f32>(scaled_vertex + pco.originSize.xyz, 1.0);

    var clip_pos = ubo.trans_w2s * world_pos;

    output.end_depth = 1.0 + clip_pos.z;
    clip_pos.z = 1.0 + clip_pos.z;

    output.clip_position = clip_pos;

    return output;
}