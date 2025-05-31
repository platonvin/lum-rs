// grass.vert (WGSL)

// --- Constants ---
const BLOCK_PALETTE_SIZE_X: i32 = 64;
const STATIC_BLOCK_COUNT: i32 = 15; // Currently unused in this shader.
const PI: f32 = 3.1415926535;
const world_size: vec3<i32> = vec3<i32>(48, 48, 16); // Dimensions of the world for texture mapping.
const VERTICES_PER_BLADE: u32 = 6u; // Number of vertices that define a single blade's geometry.
const MAX_HEIGHT: f32 = 3.0; // Maximum height a blade can reach, used for scaling effects.

// --- UBO Structure ---
struct UboData {
    trans_w2s: mat4x4<f32>, // World to screen transformation matrix.
    campos: vec4<f32>, // Camera position in world space.
    camdir: vec4<f32>, // Camera direction vector.
    horizline_scaled: vec4<f32>, // Horizon line data.
    vertiline_scaled: vec4<f32>, // Vertical line data.
    global_light_dir: vec4<f32>, // Global light direction.
    lightmap_proj: mat4x4<f32>, // Lightmap projection matrix.
    frame_size: vec2<f32>, // Size of the frame/viewport.
    wind_direction: vec2<f32>, // Global wind direction.
    timeseed: i32, // Time-based seed for procedural generation.
    delta_time: f32, // Time elapsed since the last frame.
}
;

@group(0) @binding(0) var<uniform> ubo: UboData;

// --- Texture/Sampler Bindings ---
@group(0) @binding(1) var state_tex: texture_2d<f32>; // Texture containing wind/state information.
@group(0) @binding(2) var linear_samp: sampler; // Linear sampler for state_tex.

// --- Push Constants ---
struct PushConstants {
    shift: vec4<f32>, // Positional offset for the current batch of grass.
    stxy: vec4<i32>, // Packed data: (grid_size, time_seed_related, x_flip_flag, y_flip_flag)
}
;
@group(1) @binding(0) var<storage, read> pco_shared: array<PushConstants>;


// --- Vertex Output Structure ---
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    // Pass material ID and normal packed into vec4<u32>, flat interpolated.
    @location(0) @interpolate(flat) mat_norm: vec4<u32>,
}
;

// --- Blade Geometry Structure (for get_blade_vert return) ---
struct BladeGeometry {
    position_in_tile: vec3<f32>, // Vertex position relative to its tile's origin.
    normal: vec3<f32>, // Calculated vertex normal.
}

// --- Helper Functions ---

// Generates a pseudo-random u32 from a 2D u32 vector.
fn hash21(p: vec2<u32>) -> u32 {
    var p_mut = p * vec2<u32>(73333u, 7777u);
    p_mut = p_mut ^ (vec2<u32>(3333777777u) >> (p_mut >> vec2<u32>(28u)));
    let n = p_mut.x * p_mut.y;
    return n ^ (n >> 15u);
}

// Generates a pseudo-random f32 in the range [0, 1) from a 2D f32 vector.
fn rand(p: vec2<f32>) -> f32 {
    let h = hash21(bitcast<vec2<u32>>(p));
    // Normalize u32 to f32 [0,1)
    return f32(h) * (1.0 / 4294967295.0);
}

fn square(a: f32) -> f32 { return a * a; }

// Calculates blade width based on its height, making blades thinner towards the top.
fn get_blade_width(height: f32) -> f32 {
    let max_h = MAX_HEIGHT - 1.0; // Compare against max height index for width calculation.
    return (max_h - height) / max_h; // Ensure width is normalized [0, 1].
}

// Rotates a vertex and its normal around the Z-axis.
fn rotate_blade_vert(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>) {
    let angle = rnd01 * PI * 2.0; // Random rotation angle.
    let cos_rot = cos(angle);
    let sin_rot = sin(angle);

    let vx = (*vertex).x;
    let vy = (*vertex).y;
    (*vertex).x = vx * cos_rot + vy * sin_rot;
    (*vertex).y = -vx * sin_rot + vy * cos_rot;

    let nx = (*normal).x;
    let ny = (*normal).y;
    (*normal).x = nx * cos_rot + ny * sin_rot;
    (*normal).y = -nx * sin_rot + ny * cos_rot;
}

// Displaces a blade vertex by a random offset.
fn displace_blade(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>) {
    // Using different phases/multipliers for sin/cos for varied displacement.
    let shift = vec2<f32>(sin(rnd01 * 42.1424) * 0.5, // Reduced displacement magnitude.
    cos(rnd01 * 58.1424) * 0.5);
    (*vertex).x = (*vertex).x + shift.x;
    (*vertex).y = (*vertex).y + shift.y;
}

// Curves a blade vertex based on its height and a random factor.
fn curve_blade_vert(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>) {
    // Bend along Y based on Z height, normalized by MAX_HEIGHT.
    // Randomness added to curve amount.
    (*vertex).y = ((*vertex).z / MAX_HEIGHT) * (0.5 + rnd01);
}

// Loads a wind-induced offset from the state texture.
fn load_offset(local_pos: vec2<f32>, pco: PushConstants) -> vec2<f32> {
    let world_pos = local_pos * 16.0 + pco.shift.xy;
    // Normalize world position to UV coordinates for state texture sampling.
    let state_uv = world_pos / (vec2<f32>(world_size.xy) * 16.0);
    // let offset = textureSample(state_tex, linear_samp, state_uv).xy; // Offset stored in RG channels.
    let offset = vec2f(0.0); // Currently disabled, returning zero offset.
    return offset;
}

// Applies a wiggling effect to a blade vertex due to wind and procedural motion.
fn wiggle_blade_vert(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>, pos_in_tile: vec2<f32>, pco: PushConstants) {
    // Global offset from wind texture (currently disabled in load_offset).
    let global_offset = load_offset(pos_in_tile, pco);

    // Local per-blade procedural wiggle.
    var local_offset = vec2(0.0);
    let base_freq = 1.0;
    let freq_step = 1.2;
    let ampl = 0.05;
    let t = f32(pco.stxy.y) / 200.0; // Time factor from push constants.

    for (var freq = base_freq; freq < 4.0; freq = freq + freq_step) {
        // Use different phases for x and y for more natural movement.
        local_offset.x = local_offset.x + sin(t * freq + rnd01 * 400.0 * freq + 0.0) * ampl;
        local_offset.y = local_offset.y + sin(t * freq + rnd01 * 400.0 * freq + 1.5) * ampl; // Phase shift for Y.
    }

    let offset = local_offset + global_offset;
    // Apply offset scaled by vertex height (z-component).
    (*vertex).x = (*vertex).x + offset.x * (*vertex).z * 1.0;
    (*vertex).y = (*vertex).y + offset.y * (*vertex).z * 1.0;
// Note: Normal is not explicitly recalculated here after wiggling.
// It's assumed transformations in other functions (like rotate_blade_vert) keep it reasonably correct.
}


// Generates the blade vertex position relative to its tile and its normal.
// Generates the blade vertex position relative to its tile and its normal,
// aligned more closely with the provided GLSL logic.
fn get_blade_vert(blade_vertex_id: u32, rand01: f32, relative_pos_in_tile: vec2<f32>, pco: PushConstants) -> BladeGeometry {
    var vertex: vec3<f32>;
    var normal: vec3<f32>;

    // Determine the vertex's height level (e.g., 0, 1, or 2 for VERTICES_PER_BLADE = 6u)
    let z_height_level: f32 = floor(f32(blade_vertex_id) / 2.0);
    // Determine side (0.0 or 1.0)
    var x_pos: f32 = f32(blade_vertex_id % 2u);

    // GLSL: if(iindex == (VERTICES_PER_BLADE-1)) x_pos = 0.5;
    // For VERTICES_PER_BLADE = 6u, the last vertex index is 5.
    // This makes the very tip of the blade (the single highest indexed vertex) centered.
    if (blade_vertex_id == (VERTICES_PER_BLADE - 1u)) {
        x_pos = 0.5;
    }

    // Initial vertex position based on height level and side.
    vertex = vec3<f32>(x_pos, 0.0, z_height_level);

    // Apply width narrowing towards the top, as per GLSL logic.
    // width is [0..1], 0 at max_height, 1 at z_height_level 0.
    let width = get_blade_width(vertex.z); // vertex.z is the current z_height_level
    let width_diff = 1.0 - width;
    // Scale x_pos by width and shift to center it based on width_diff.
    // If width is 1, vertex.x remains x_pos.
    // If width is 0, vertex.x becomes 0.5 (center).
    vertex.x = width * vertex.x + width_diff / 2.0;

    // Normal calculation, as per GLSL logic.
    // Uses the original x_pos (0.0, 1.0, or 0.5 for the tip vertex) to determine normal direction.
    let n1 = vec3<f32>(-0.5, 1.0, 0.0); // Normal for x_pos = 0.0 side
    let n2 = vec3<f32>(0.5, 1.0, 0.0); // Normal for x_pos = 1.0 side
    normal = normalize(mix(n1, n2, x_pos));

    // Initial scaling of vertex components, as per GLSL.
    // Note: GLSL's vertex.x *= 3.7; is a very large scaling factor.
    // If blades appear too wide, this value (3.7) might need adjustment.
    vertex.x = vertex.x * 3.7;
    vertex.z = vertex.z * 2.0; // Equivalent to GLSL's vertex.z *= 6.0 / 3.0;

    // Apply sequence of transformations.
    curve_blade_vert(rand01, &vertex, &normal);
    rotate_blade_vert(rand01, &vertex, &normal);
    wiggle_blade_vert(rand01, &vertex, &normal, relative_pos_in_tile, pco);
    displace_blade(rand01, &vertex, &normal);

    // Final height scaling, as per GLSL.
    // This introduces random height variation.
    vertex.z = vertex.z * (1.5 + (rand01 * 1.5) * (rand01 * 1.5));

    // Position the vertex relative to its tile's origin.
    // This step was part of the WGSL function previously and is kept here
    // for consistency with its role in preparing data for the main vertex shader.
    let tile_render_size = 16.0; // Assumed size of the tile in world units.
    let shift_in_tile = relative_pos_in_tile * tile_render_size;
    let vertex_pos_in_tile = vertex + vec3<f32>(shift_in_tile.x, shift_in_tile.y, 0.0);

    return BladeGeometry(vertex_pos_in_tile, normal);
}

@vertex
fn main(@builtin(vertex_index) vertex_idx: u32, @builtin(instance_index) instance_idx: u32) -> VertexOutput {
    // instance_idx is not quite the batch id yet
    // total index count is batch_count * blades_per_batch
    let blades_per_batch = u32(10*10);
    let batch_index = instance_idx / blades_per_batch;
    let blade_index = instance_idx % blades_per_batch;
    let pco = pco_shared[batch_index];

    var output: VertexOutput;

    // Calculate blade and vertex IDs.
    let sub_blade_id = vertex_idx / VERTICES_PER_BLADE; // ID of the blade within this instance.
    let blade_id = i32(blade_index + sub_blade_id); // Global blade ID.
    let blade_vertex_id = vertex_idx % VERTICES_PER_BLADE; // Index of the vertex on the current blade (0 to VERTICES_PER_BLADE-1).

    // Calculate blade's grid position, applying flips if specified in push constants.
    let size = pco.stxy.x; // Grid size.
    let x_flip = pco.stxy.z;
    let y_flip = pco.stxy.w;

    var blade_x = blade_id % size;
    var blade_y = blade_id / size;
    if (x_flip == 0) { blade_x = size - 1 - blade_x; } // Conditionally flip X.
    if (y_flip != 0) { blade_y = size - 1 - blade_y; } // Conditionally flip Y.

    // Calculate normalized position of the blade within its tile [0, 1].
    let relative_pos_in_tile = (vec2<f32>(f32(blade_x), f32(blade_y)) + 0.5) / f32(size);

    // Generate a random value for this blade based on its position and a shift.
    let rand01 = rand(relative_pos_in_tile + pco.shift.xy);

    // Generate blade vertex position (relative to tile) and normal.
    let blade_geom = get_blade_vert(blade_vertex_id, rand01, relative_pos_in_tile, pco);
    var final_normal = blade_geom.normal;

    // Position vertex in world space by adding the global shift from push constants.
    let world_pos = vec4<f32>(blade_geom.position_in_tile, 1.0) + pco.shift;

    // Transform vertex to clip space.
    var clip_pos = ubo.trans_w2s * world_pos;
    // Adjust Z for depth (specific rendering technique).
    clip_pos.z = 1.0 + clip_pos.z;
    output.clip_position = clip_pos;

    // Ensure normal faces towards the camera.
    final_normal = normalize(final_normal); // Ensure normalization.
    if (dot(ubo.camdir.xyz, final_normal) > 0.0) {
        final_normal = -final_normal;
    }

    // Assign material ID (randomly chosen between 9 and 10).
    let mat_id = select(10u, 9u, rand01 > 0.5);

    // Pack normal into u32 components (0-255 range).
    let packed_f32_normal = (final_normal + 1.0) * 0.5; // Remap from [-1,1] to [0,1].
    let norm_uint_rgb = vec3<u32>(packed_f32_normal * 255.0);

    output.mat_norm = vec4<u32>(mat_id, norm_uint_rgb);
    return output;
}
