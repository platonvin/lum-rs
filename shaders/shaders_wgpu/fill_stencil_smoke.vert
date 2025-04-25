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

@group(0) @binding(0) var<uniform> ubo: UboData;

struct PushConstants {
    originSize: vec4<f32>, 
};
@group(1) @binding(0) var<uniform> pco: PushConstants; 

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) end_depth: f32, 
};

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
fn main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    var output: VertexOutput;

    let vertex = vertices[vertex_idx];

    let scaled_vertex = vertex * pco.originSize.w;

    let world_pos = vec4<f32>(scaled_vertex + pco.originSize.xyz, 1.0);

    let clip_pos_h = ubo.trans_w2s * world_pos;

    var calculated_depth: f32 = 0.0;
    if (clip_pos_h.w != 0.0) {
        calculated_depth = clip_pos_h.z / clip_pos_h.w;
    }
    output.end_depth = calculated_depth + 1.0;


    // Assign final homogeneous clip space position for rasterizer
    output.clip_position = clip_pos_h;

    return output;
}