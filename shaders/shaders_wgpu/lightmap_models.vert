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
@group(1) @binding(0) var<storage, read> pco_shared: array<PushConstants>;

struct VertexInput {
    @location(0) pos_in: vec4<u32>, // Matches layout(location = 0) in lowp uvec3 posIn;
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
    // return v + 2.0 * term1;

    // Alternative implementation (often seen):
    return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v);
}

// --- Vertex Entry Point ---
@vertex
fn main(@builtin(instance_index) instance_id: u32, input: VertexInput) -> VertexOutput {
    let pco = pco_shared[instance_id];

    var output: VertexOutput;

    // Convert u32 input position to f32
    let local_pos_f32 = vec3<f32>(input.pos_in.xyz);

    // Apply quaternion rotation from push constants
    let rotated_local_pos = qtransform(pco.rot, local_pos_f32);

    // Apply shift from push constants
    let world_pos = vec4<f32>(rotated_local_pos + pco.shift.xyz, 1.0);

    // Transform to homogeneous clip space using the simplified UBO matrix
    var clip_pos = ubo.lightmap_proj * world_pos;
    clip_pos.z = 1.0 + clip_pos.z;

    // Assign final position
    output.clip_position = clip_pos;

    return output;
}