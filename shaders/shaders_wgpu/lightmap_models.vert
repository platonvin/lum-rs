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
    rotation: vec4<f32>, // Quaternion rotation
    shift: vec4<f32>,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(1) @binding(0) var<storage, read> push_constants_shared: array<PushConstants>;

struct VertexInput {
    @location(0) @interpolate(flat) position_input: vec4<u32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

fn quaternion_transform(quaternion: vec4<f32>, vector: vec3<f32>) -> vec3<f32> {
    let q_xyz = quaternion.xyz;
    let q_w = quaternion.w;
    return vector + 2.0 * cross(q_xyz, cross(q_xyz, vector) + q_w * vector);
}

@vertex
fn main(@builtin(instance_index) instance_id: u32, input: VertexInput) -> VertexOutput {
    let push_constant_data = push_constants_shared[instance_id];

    var output: VertexOutput;

    let local_position_f32 = vec3<f32>(input.position_input.xyz);

    let rotated_local_position = quaternion_transform(push_constant_data.rotation, local_position_f32);

    let world_position = vec4<f32>(rotated_local_position + push_constant_data.shift.xyz, 1.0);

    var clip_position = ubo.lightmap_projection * world_position;
    clip_position.z = 1.0 + clip_position.z;

    output.clip_position = clip_position;

    return output;
}
