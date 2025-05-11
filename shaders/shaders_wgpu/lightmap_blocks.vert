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
    shift: vec4<i32>,
};
@group(0) @binding(0) var<uniform> ubo: UboData;
@group(1) @binding(0) var<uniform> pco: PushConstants;

struct VertexInput {
    @location(0) pos_in: vec4<u32>,
};

// --- Vertex Output Structure ---
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

// --- Helper Functions (qtransform not used here) ---
// fn qtransform(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
//     return v + 2.0 * cross(cross(v, -q.xyz) + q.w * v, -q.xyz);
// }

// --- Vertex Entry Point ---
@vertex
fn main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Convert u32 input position to f32 for calculations
    let local_pos_f32 = vec3<f32>(input.pos_in.xyz);

    // Apply shift from push constants (cast i32 shift to f32)
    let world_pos = vec4<f32>(local_pos_f32 + vec3<f32>(pco.shift.xyz), 1.0);

    // Transform to homogeneous clip space using UBO matrix
    var clip_pos = ubo.lightmap_proj * world_pos;
    clip_pos.z = 1.0 + clip_pos.z;

    // Assign final position
    output.clip_position = clip_pos;

    return output;
}