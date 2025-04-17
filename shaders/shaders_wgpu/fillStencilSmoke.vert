// fillStencilSmoke.vert (WGSL)

// --- Corrected UBO Structure ---
struct UboData {
    trans_w2s: mat4x4<f32>,
    campos: vec4<f32>,
    camdir: vec4<f32>,
    horizline_scaled: vec4<f32>,
    vertiline_scaled: vec4<f32>,
    global_light_dir: vec4<f32>, // Corrected name
    lightmap_proj: mat4x4<f32>,
    frame_size: vec2<f32>,
    timeseed: i32,
};

@group(0) @binding(0) var<uniform> ubo: UboData;

// --- Push Constants ---
// WGSL often requires push constants in a <uniform> buffer or specific address space
// This assumes a uniform buffer binding for them. Adjust if using actual push constants.
struct PushConstants {
    originSize: vec4<f32>, // Corresponds to pco in GLSL
};
@group(1) @binding(0) var<uniform> pco: PushConstants; // Assuming group 1, binding 0 for push constants

// --- Vertex Output Structure ---
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) end_depth: f32, // Output to fragment shader
};

// --- Embedded Vertex Data (Requires WGSL 2021+ or storage buffer alternative) ---
// Note: Embedding large arrays like this is less common/performant in WGSL
// than using vertex buffers. This is a direct translation for demonstration.
// If not using WGSL 2021+, this array needs to be passed via a storage buffer.
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


// --- Vertex Entry Point ---
@vertex
fn main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    var output: VertexOutput;

    // Access vertex data using the vertex index
    // Cast u32 index to i32 if needed for array access depending on WGSL version/spec details
    let vertex = vertices[vertex_idx];

    // Apply scaling from push constant
    let scaled_vertex = vertex * pco.originSize.w;

    // Calculate world position
    let world_pos = vec4<f32>(scaled_vertex + pco.originSize.xyz, 1.0);

    // Transform to homogeneous clip space
    let clip_pos_h = ubo.trans_w2s * world_pos;

    // Calculate depth value to pass to fragment shader
    // Perform perspective divide manually for depth calculation if needed before outputting z
    // Ensure clip_pos_h.w is not zero
    var calculated_depth: f32 = 0.0;
    if (clip_pos_h.w != 0.0) {
         // Divide by w to get normalized device coordinates [-1, 1] for z
        calculated_depth = clip_pos_h.z / clip_pos_h.w;
    }
    // Apply the same +1.0 offset as GLSL for consistency
    output.end_depth = calculated_depth + 1.0;


    // Assign final homogeneous clip space position for rasterizer
    output.clip_position = clip_pos_h;

    return output;
}