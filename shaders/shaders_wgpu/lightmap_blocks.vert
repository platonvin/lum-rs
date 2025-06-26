struct UboData {
    transform_world_to_screen: mat4x4<f32>,
    camera_position: vec4<f32>,
    camera_direction: vec4<f32>,
    horizontal_line_scaled: vec4<f32>,
    vertical_line_scaled: vec4<f32>,
    global_light_direction: vec4<f32>,
    lightmap_projection: mat4x4<f32>,
    frame_size: vec2<f32>,
    wind_direction: vec2<f32>,
    time_seed: i32,
    delta_time: f32,
};

struct PushConstants {
    block: i32,
    shift_x: i32,
    shift_y: i32,
    shift_z: i32,
    web_compatibility_unorm: u32,
};

struct VertexInput {
    @location(0) @interpolate(flat) position_input: vec4<u32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(1) @binding(0) var<storage, read> push_constants_shared: array<PushConstants>;


@vertex
fn main(@builtin(instance_index) instance_id: u32, input: VertexInput) -> VertexOutput {
    let push_constant_data = push_constants_shared[instance_id];
    let shift = vec3<i32>(push_constant_data.shift_x, push_constant_data.shift_y, push_constant_data.shift_z);
    
    var output: VertexOutput;

    let local_position_f32 = vec3<f32>(input.position_input.xyz);

    let world_position = vec4<f32>(local_position_f32 + vec3<f32>(shift), 1.0);

    var clip_position = ubo.lightmap_projection * world_position;
    clip_position.z = 1.0 + clip_position.z;

    output.clip_position = clip_position;

    return output;
}
