// grass.vert (WGSL)

// --- Assumed constants (from common/consts.glsl & shader) ---
const BLOCK_PALETTE_SIZE_X: i32 = 64;
const STATIC_BLOCK_COUNT: i32 = 15; // Unused in this shader?
const PI: f32 = 3.1415926535;
const world_size: vec3<i32> = vec3<i32>(48, 48, 16);
const BLADES_PER_INSTANCE: i32 = 1; // From shader const
const VERTICES_PER_BLADE: u32 = 6u; // From shader const (use u32 for indices)
const MAX_HEIGHT: f32 = 3.0; // From shader const (use f32)

// --- UBO Structure ---
// Using previously corrected structure
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

// --- Texture/Sampler Bindings ---
@group(0) @binding(1) var state_tex: texture_2d<f32>; // sampler2D state
@group(0) @binding(2) var linear_samp: sampler;      // Sampler for state_tex

@group(1) @binding(0) var<uniform> pco: PushConstants;
// --- Push Constants ---
struct PushConstants {
    shift: vec4<f32>,
    size: i32, // total size*size blades
    time: i32, // seed (passed as f32 time?)
    x_flip: i32,
    y_flip: i32,
};
// Assuming push constants mapped to uniform buffer at group 1

// --- Vertex Output Structure ---
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    // Pass material ID and normal packed into uvec4, flat interpolated
    @location(0) @interpolate(flat) mat_norm: vec4<u32>,
};

// --- Helper Functions ---

// Note: voxel_in_palette seems unused in the final GLSL main logic for grass.vert
// fn voxel_in_palette(relative_voxel_pos: vec3<i32>, block_id: i32) -> vec3<i32> {
//     let block_x = block_id % BLOCK_PALETTE_SIZE_X;
//     let block_y = block_id / BLOCK_PALETTE_SIZE_X;
//     return relative_voxel_pos + vec3<i32>(16 * block_x, 16 * block_y, 0);
// }

// Hash function
fn hash21(p: vec2<u32>) -> u32 {
    var p_mut = p * vec2<u32>(73333u, 7777u);
    // WGSL requires explicit type casting for shift amounts if needed, >> works directly on u32
    p_mut = p_mut ^ (vec2<u32>(3333777777u) >> (p_mut >> vec2<u32>(28u))); // GLSL p>>28 applied element-wise
    let n = p_mut.x * p_mut.y;
    return n ^ (n >> 15u);
}

// Random number generator [0, 1)
fn rand(p: vec2<f32>) -> f32 {
    let h = hash21(bitcast<vec2<u32>>(p)); // Use bitcast as requested
    // Convert u32 max to f32 for normalization
    return f32(h) * (1.0 / 4294967295.0);
}

fn square(a: f32) -> f32 { return a * a; }

fn get_blade_width(height: f32) -> f32 {
    let max_h = MAX_HEIGHT - 1.0; // Compare against max index
    return clamp((max_h - height) / max_h, 0.0, 1.0); // Ensure width is [0, 1]
}

// Rotate vertex and normal around Z axis
fn rotate_blade_vert(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>) {
    // Angle calculation seems overly complex, simplifying to just rnd01 * 2PI
    let angle = rnd01 * PI * 2.0;
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

// Displace vertex based on random value
fn displace_blade(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>) {
    // Using different phases/multipliers for sin for variety
    let shift = vec2<f32>(
        sin(rnd01 * 42.1424) * 0.5, // Reduced displacement magnitude
        cos(rnd01 * 58.1424) * 0.5  // Used cos for variation
    );
    // wgsl does not support swizzles LOL
    (*vertex).x = (*vertex).x + shift.x; 
    (*vertex).y = (*vertex).y + shift.y;
}

// Scale vertex (normal is direction, unaffected by uniform scale)
// fn scale_blade_vert(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>) {
//     let scale = 0.5 + (rnd01) * 0.5;
//     *vertex = *vertex * scale;
// } // Not used in GLSL main flow

// Curve the blade based on height
fn curve_blade_vert(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>) {
    // Bend along Y based on Z height, normalized by MAX_HEIGHT
    // Reduced curve factor from 1.5
    (*vertex).y = ((*vertex).z / MAX_HEIGHT) * (0.5 + rnd01); // Add randomness to curve amount
}

// Load wind offset from state texture
fn load_offset(local_pos: vec2<f32>) -> vec2<f32> {
    let world_pos = local_pos * 16.0 + pco.shift.xy;
    // Normalize world pos to UVs for state texture
    let state_uv = world_pos / (vec2<f32>(world_size.xy) * 16.0);
    // Sample texture using sampler
    // let offset = textureSample(state_tex, linear_samp, state_uv).xy;
    let offset = vec2f(0);
    return offset; // Assuming offset stored in RG channels
}

// Apply wiggling effect to vertex
fn wiggle_blade_vert(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>, pos: vec2<f32>) {
    // Global offset from wind texture
    let global_offset = load_offset(pos);

    // Local per-blade procedural wiggle
    var local_offset = vec2(0.0);
    let base_freq = 1.0;
    let freq_step = 1.2;
    let ampl = 0.05;
    let t = f32(pco.time) / 200.0; // Use float time

    for (var freq = base_freq; freq < 4.0; freq = freq + freq_step) {
        // Use different phases for x and y
        local_offset.x = local_offset.x + sin(t * freq + rnd01 * 400.0 * freq + 0.0) * ampl;
        local_offset.y = local_offset.y + sin(t * freq + rnd01 * 400.0 * freq + 1.5) * ampl; // Phase shift Y
    }

    let offset = local_offset + global_offset;
    // Apply offset scaled by vertex height (z)
    // wgsl does not support swizzles LOL
    (*vertex).x = (*vertex).x + offset.x * (*vertex).z * 1.0; 
    (*vertex).y = (*vertex).y + offset.y * (*vertex).z * 1.0;

    // Recalculate normal after wiggling? Or assume it's roughly the same?
    // For simplicity, normal is not recalculated here. This might affect lighting.
}

// Generate vertex position and normal for a specific vertex index on the blade
fn get_blade_vert(vertex_index_on_blade: u32, rnd01: f32, pos_in_tile: vec2<f32>) -> vec3<f32> {
    var vertex: vec3<f32>;
    var normal: vec3<f32>; // Will be calculated alongside vertex

    // Calculate base position based on index (triangle strip for a quad: 0,1,2 / 2,1,3)
    // This assumes 6 vertices make up the blade geometry (e.g., 3 quads)
    // Need to map 0..5 index to 3D coordinates. Using GLSL structure:
    let z_height: f32 = floor(f32(vertex_index_on_blade) / 2.0); // Height level (0, 1, 2)
    let x_pos: f32 = f32(vertex_index_on_blade % 2u); // Side (0 or 1)

    vertex = vec3(x_pos, 0.0, z_height); // Base position on Z-up quad strip

    // Apply width based on height
    let width = get_blade_width(vertex.z);
    vertex.x = mix(0.5, vertex.x, width); // Lerp x towards center based on width

    // Base normal (pointing up/slightly out along Y)
    normal = normalize(vec3(mix(-0.2, 0.2, x_pos), 1.0, 0.1)); // Slight bend default

    // Apply transformations
    let base_scale = 1.0; // Adjust base width/thickness
    vertex.x = vertex.x * base_scale;

    curve_blade_vert(rnd01, &vertex, &normal);      // Apply curvature
    rotate_blade_vert(rnd01, &vertex, &normal);     // Rotate around Z
    wiggle_blade_vert(rnd01, &vertex, &normal, pos_in_tile); // Apply wind/local wiggle
    displace_blade(rnd01, &vertex, &normal);    // Apply base displacement

    // Apply height scaling (non-uniform)
    let height_scale_base = 1.0; // Base height multiplier
    let height_scale_rand = (rnd01 * 1.5) * (rnd01 * 1.5); // Random height variance
    vertex.z = vertex.z * (height_scale_base + height_scale_rand); // Adjust height based on index + randomness

    // Store normal for output (adjustments are done in main)
    // Note: Need to pass normal out somehow if needed after all transforms
    // For now, returning only vertex position as normal calc seems complex to track through all ops
    // Revisit normal calculation if lighting is inaccurate.
    return vertex;
}

@vertex
fn main(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32
) -> VertexOutput {
    var output: VertexOutput;

    // Calculate blade and vertex IDs
    let sub_blade_id = vertex_idx / VERTICES_PER_BLADE; // ID within the instance
    let blade_id = i32(instance_idx * u32(BLADES_PER_INSTANCE) + sub_blade_id);
    let blade_vertex_id = vertex_idx % VERTICES_PER_BLADE; // Index on the current blade (0..5)

    // Calculate blade's grid position (potentially flipped)
    var blade_x = blade_id % pco.size;
    var blade_y = blade_id / pco.size;
    if (pco.x_flip == 0) { blade_x = pco.size - 1 - blade_x; } // Flip X
    if (pco.y_flip != 0) { blade_y = pco.size - 1 - blade_y; } // Flip Y

    // Position within the tile [0, 1]
    let relative_pos_in_tile = (vec2<f32>(f32(blade_x), f32(blade_y)) + 0.5) / f32(pco.size);

    // Generate random value for this blade
    let rand01 = rand(relative_pos_in_tile + pco.shift.xy);

    // Get vertex position relative to blade origin (0,0,0)
    // Need to calculate normal properly alongside vertex generation/transformation
    // Simplifying: Calculate position first, normal separately or approximate.
    // Let's try calculating a base normal and transforming it similarly.
    var normal: vec3<f32>;
    let z_height_level: f32 = floor(f32(blade_vertex_id) / 2.0);
    let x_side: f32 = f32(blade_vertex_id % 2u);
    // Base normal points somewhat up/out along Y, slightly different per side
    normal = normalize(vec3(mix(-0.1, 0.1, x_side), 0.8, 0.2));

    // Get base vertex position and apply transforms (modifies vertex and normal)
    var vertex = vec3(mix(0.5 - get_blade_width(z_height_level)/2.0, 0.5 + get_blade_width(z_height_level)/2.0, x_side), 0.0, z_height_level);
    let base_scale = 0.2; // Adjust base width
    vertex.x = (vertex.x-0.5)*base_scale + 0.5; // Apply width scale relative to center

    curve_blade_vert(rand01, &vertex, &normal);
    rotate_blade_vert(rand01, &vertex, &normal);
    wiggle_blade_vert(rand01, &vertex, &normal, relative_pos_in_tile);
    displace_blade(rand01, &vertex, &normal);

    // Apply height scale
    let height_scale_base = 1.0;
    let height_scale_rand = (rand01 * 0.5) * (rand01 * 0.5); // Less extreme height variance
    vertex.z = vertex.z * (height_scale_base + height_scale_rand) * 1.5; // Base height factor

    // Final normal rotation (only rotation affects normal direction significantly)
    // Apply the same rotation as applied to the vertex
    var temp = vec3(0.0);
    rotate_blade_vert(rand01, &temp, &normal); // Only apply rotation part to normal

    // Position vertex relative to tile origin
    let rel_to_tile_shift = relative_pos_in_tile * 16.0;
    let vertex_in_tile = vertex + vec3<f32>(rel_to_tile_shift, 0.0);

    // Calculate world position
    let world_pos = vec4<f32>(vertex_in_tile, 1.0) + pco.shift;

    // Transform to clip space
    let clip_pos_h = ubo.trans_w2s * world_pos;

    // Perspective divide for depth calculation (if needed)
    // var clip_coords_z = 0.0;
    // if (clip_pos_h.w != 0.0) {
    //     clip_coords_z = clip_pos_h.z / clip_pos_h.w;
    // }
    // let depth_output = clip_coords_z + 1.0; // Match GLSL z modification if needed

    // Assign final position
    output.clip_position = clip_pos_h;

    // --- Calculate output mat_norm ---
    // Ensure normal faces camera
    var final_normal = normalize(normal); // Ensure normalization
    if (dot(ubo.camdir.xyz, final_normal) > 0.0) {
        final_normal = -final_normal;
    }

    // Assign material ID (randomly chosen between 9 and 10 based on rand)
    // Slightly modify threshold based on distance from center of tile maybe?
    // let dist_factor = length(relative_pos_in_tile - 0.5) / 0.707; // Normalize distance [0, 1]
    // let mat_id = select(10u, 9u, rand01 > (rand(pco.shift.yx) - dist_factor * 0.1)); // uint
    let mat_id = select(10u, 9u, rand01 > 0.5); // Simpler threshold for now

    // Pack material ID (as float [-1, 1]) and normal (as [-1, 1]) into uvec4 [0, 255]
    // Convert uint mat_id to float [-1, 1] range. Mapping 9->~ -0.89, 10->~ -0.88
    let fmat = (f32(mat_id) - 127.5) / 127.5; // Map [0..255] range approx
    // Pack fmat and normal into a vec4
    let packed_f32 = vec4<f32>(fmat, final_normal);
    // Normalize to [0, 1] range
    let packed_01 = (packed_f32 + 1.0) * 0.5;
    // Scale to [0, 255] and cast to u32
    output.mat_norm = vec4<u32>(packed_01 * 255.0);
    // Alternatively, use pack4x8unorm if available and outputting vec4<f32> is okay
    // output.mat_norm_packed = pack4x8unorm(packed_01); // Requires output type change

    return output;
}