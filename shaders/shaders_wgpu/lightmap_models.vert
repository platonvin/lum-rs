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
    rot: vec4<f32>,   // Quaternion rotation
    shift: vec4<f32>, // Translation shift
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(1) @binding(0) var<uniform> pco: PushConstants;

struct VertexInput {
    @location(0) pos_in: vec3<u32>, // Matches layout(location = 0) in lowp uvec3 posIn;
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

// --- Helper Functions ---
// Quaternion transformation function
fn qtransform(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    // GLSL: v + 2.0*cross(cross(v, -q.xyz ) + q.w*v, -q.xyz)
    let q_xyz = q.xyz;
    let q_w = q.w;
    // Note: GLSL used -q.xyz in cross products.
    let u = -q_xyz; // Use u = -q.xyz for clarity
    let cross_v_u = cross(v, u);
    let term1 = cross(cross_v_u + q_w * v, u);
    return v + 2.0 * term1;

    // Alternative implementation (often seen):
    // return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v);
}

// --- Vertex Entry Point ---
@vertex
fn main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Convert u32 input position to f32
    let fpos = vec3<f32>(input.pos_in);

    // Apply quaternion rotation from push constants
    let rotated_pos = qtransform(pco.rot, fpos);

    // Apply shift from push constants
    let world_pos = vec4<f32>(rotated_pos + pco.shift.xyz, 1.0);

    // Transform to homogeneous clip space using the simplified UBO matrix
    let clip_pos_h = ubo.trans_w2s * world_pos;

    // Apply Z offset as in GLSL (clip_coords.z = 1 + clip_coords.z)
    // Equivalent to adding 'w' to 'z': z' = z + w
    var final_clip_pos = clip_pos_h;
    final_clip_pos.z = final_clip_pos.z + final_clip_pos.w;

    // Assign final position
    output.clip_position = final_clip_pos;

    return output;
}