// glossy.frag (WGSL)

// --- Assumed constants (from common/consts.glsl & shader) ---
// Define world_size, adjust as needed
const world_size: vec3<i32> = vec3<i32>(256, 256, 256); // Example value
const PI: f32 = 3.1415926535;
const BLOCK_PALETTE_SIZE_X: i32 = 64; // From constant_id
const COLOR_ENCODE_VALUE: f32 = 8.0;

// --- UBO Structure ---
struct UboData {
    trans_w2s: mat4x4<f32>, // Not used in this shader?
    campos: vec4<f32>,
    camdir: vec4<f32>,
    horizline_scaled: vec4<f32>,
    vertiline_scaled: vec4<f32>,
    global_light_dir: vec4<f32>,
    lightmap_proj: mat4x4<f32>, // Not used in this shader?
    frame_size: vec2<f32>,
    timeseed: i32,             // Not used in this shader?
};

@group(0) @binding(0) var<uniform> ubo: UboData;

// --- Bindings ---
// Note: Binding indices are assumed, adjust to match pipeline layout
@group(0) @binding(1) var mat_norm_tex: texture_2d<u32>;          // usampler2D matNorm (Assuming single uint channel, maybe RGBA8ui?)
@group(0) @binding(2) var depthBuffer_tex: texture_depth_2d;     // sampler2D depthBuffer (Depth texture)
@group(0) @binding(3) var depth_samp: sampler;                   // Sampler for depthBuffer_tex

@group(0) @binding(4) var blocks_tex: texture_3d<i32>;           // isampler3D blocks (Signed integer texture)
@group(0) @binding(5) var blockPalette_tex: texture_3d<u32>;     // usampler3D blockPalette (Unsigned integer texture)
@group(0) @binding(6) var voxelPalette_tex: texture_2d<f32>;     // sampler2D voxelPalette (Assuming R32Float format per channel)
// Binding 7: Sampler for voxelPalette (Nearest assumed, using textureLoad only)

@group(0) @binding(8) var radianceCache_tex: texture_3d<f32>;    // sampler3D radianceCache
@group(0) @binding(9) var linear_samp: sampler;                // Sampler for radianceCache

// --- Structs ---
struct Material {
    color: vec3<f32>,
    emmitance: f32,
    roughness: f32,
};

struct FragmentOutput {
    @location(0) frame_color: vec4<f32>,
};

// --- Helper Functions ---

fn get_origin_from_depth(depth: f32, clip_pos: vec2<f32>) -> vec3<f32> {
    let origin = ubo.campos.xyz +
        (ubo.horizline_scaled.xyz * clip_pos.x) +
        (ubo.vertiline_scaled.xyz * clip_pos.y) +
        (ubo.camdir.xyz * depth);
    return origin;
}

// Gets the block ID at a given world integer coordinate
fn GetBlock(block_pos: vec3<i32>) -> i32 {
    // Clamp coordinates to avoid out-of-bounds reads if necessary
    let clamped_pos = clamp(block_pos, vec3<i32>(0), world_size - vec3<i32>(1));
    // Load from integer texture
    let block = textureLoad(blocks_tex, clamped_pos, 0).r; // LOD 0, assuming ID is in 'r' channel
    return block;
}

// Calculates the coordinate within the block palette texture atlas
fn voxel_in_palette(relative_voxel_pos: vec3<i32>, block_id: i32) -> vec3<i32> {
    let block_x = block_id % BLOCK_PALETTE_SIZE_X;
    let block_y = block_id / BLOCK_PALETTE_SIZE_X;
    // Assuming palette is a 3D texture where Z=0 is used, and blocks are laid out in XY plane
    return relative_voxel_pos + vec3<i32>(16 * block_x, 16 * block_y, 0);
}

// Gets the voxel material ID and block ID at a given world position (f32)
fn GetVoxel(pos: vec3<f32>) -> vec2<i32> {
    let ipos = vec3<i32>(floor(pos)); // Convert f32 pos to integer voxel coord
    let iblock_pos = ipos / 16;
    let relative_voxel_pos = ipos % 16;

    let block_id = GetBlock(iblock_pos);
    if (block_id < 0) { // Assuming negative block_id might mean empty or invalid
        return vec2<i32>(0, block_id); // Return material 0 if block is invalid/empty
    }

    let voxel_pos_in_palette = voxel_in_palette(relative_voxel_pos, block_id);

    // Load from unsigned integer texture
    let voxel_mat_id = i32(textureLoad(blockPalette_tex, voxel_pos_in_palette, 0).r); // LOD 0

    return vec2<i32>(voxel_mat_id, block_id);
}

// Gets material properties based on material ID
fn GetMat(voxel_mat_id: i32) -> Material {
    var mat: Material;
    if (voxel_mat_id <= 0) { // Handle air/empty material explicitly
        mat.color = vec3(0.0);
        mat.emmitance = 0.0;
        mat.roughness = 1.0; // Treat air as fully rough maybe?
        return mat;
    }
    // Use textureLoad with integer coordinates
    mat.color.r = textureLoad(voxelPalette_tex, vec2<i32>(0, voxel_mat_id), 0).r;
    mat.color.g = textureLoad(voxelPalette_tex, vec2<i32>(1, voxel_mat_id), 0).r;
    mat.color.b = textureLoad(voxelPalette_tex, vec2<i32>(2, voxel_mat_id), 0).r;
    mat.emmitance = textureLoad(voxelPalette_tex, vec2<i32>(4, voxel_mat_id), 0).r;
    mat.roughness = textureLoad(voxelPalette_tex, vec2<i32>(5, voxel_mat_id), 0).r;
    return mat;
}

// Samples the radiance cache (needs sampler)
fn sample_radiance(position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    // Offset position slightly along normal before sampling
    let block_pos = (position + normal * 0.1) / 16.0; // Reduced offset from 16.0
    let uvw = block_pos / vec3<f32>(world_size); // Normalize coordinates
    let sampled_light = textureSampleLevel(radianceCache_tex, linear_samp, uvw, 0.0).rgb; // LOD 0
    return sampled_light;
}

// Initializes DDA parameters
fn initTvals(rayOrigin: vec3<f32>, rayDirection: vec3<f32>) -> vec3<f32> {
    // Calculate tMax: distance to the nearest voxel boundary in each dimension
    // division by zero potential if rayDirection component is 0
    let inv_dir = 1.0 / rayDirection;
    let next_voxel_boundary = floor(rayOrigin) + step(vec3(0.0), rayDirection); // floor + (1 if dir>0 else 0)
    let tMax = (next_voxel_boundary - rayOrigin) * inv_dir;
    return tMax;
}

// Fast raycast with block skipping
// Returns true if hit, false otherwise. Outputs updated fraction, normal, material, left_bounds.
fn CastRay_fast(origin: vec3<f32>, direction: vec3<f32>,
                fraction_inout: ptr<function, f32>, normal_out: ptr<function, vec3<f32>>,
                material_out: ptr<function, Material>, left_bounds_out: ptr<function, bool>) -> bool {
    var fraction: f32 = *fraction_inout; // Start from current fraction
    *left_bounds_out = false;

    let max_dist = 128.0; // Maximum trace distance
    let step_size = 0.5; // Initial step size

    let one_div_dir = 1.0 / direction;
    let step_sign = sign(direction); // Direction of stepping (+1 or -1)
    let b_pos_dir = direction > vec3(0.0); // bool vec for positive direction

    for (var iterations: i32 = 0; fraction < max_dist; iterations = iterations + 1 ) { // Limited loop
        fraction = fraction + step_size;
        let pos = origin + direction * fraction;

        // Check world bounds
        if (any(pos < vec3(0.0)) || any(pos >= vec3<f32>(world_size * 16))) {
            *left_bounds_out = true;
            return false; // Left bounds
        }

        let voxel_info = GetVoxel(pos);
        let current_voxel = voxel_info.x;
        let current_block = voxel_info.y;

        // If inside an empty block (block ID 0), skip to the block boundary
        if (current_block == 0) {
            // Calculate position of the corner towards which the ray travels
            let block_base_coord = vec3<f32>((vec3<i32>(pos) / 16) * 16);
            let far_corner = block_base_coord + vec3<f32>(b_pos_dir) * 16.0;
            // Calculate distance (t) to hit the planes forming that corner
            let t_to_corner = (far_corner - pos) * one_div_dir;
            // Minimum positive t is the distance to exit the current empty block
            let t_exit = max(0.01, min(t_to_corner.x, min(t_to_corner.y, t_to_corner.z)));
            fraction = fraction + t_exit; // Advance fraction to block boundary
            continue; // Check the next position after skipping
        }

        // If hit a non-empty voxel
        if (current_voxel != 0) {
            // Refine the hit position using DDA from just before the hit
            let precise_origin = origin + direction * (fraction - step_size * 1.5); // Step back slightly
            var tMax = initTvals(precise_origin, direction);
            let tDelta = abs(1.0 / direction);
            var voxel_pos = vec3<i32>(floor(precise_origin));
            var current_voxel_id = GetVoxel(precise_origin).x; // Voxel at precise_origin
            var hit_normal = vec3(0.0);

            // DDA loop - limited steps for refinement
            for (var dda_iter = 0; dda_iter < 5 && current_voxel_id == 0; dda_iter = dda_iter + 1) {
                 var step_dir_mask = vec3(0.0);
                 if (tMax.x <= tMax.y && tMax.x <= tMax.z) {
                    step_dir_mask.x = 1.0;
                 } else if (tMax.y <= tMax.z) { // Implicit tMax.y < tMax.x
                    step_dir_mask.y = 1.0;
                 } else { // Implicit tMax.z < tMax.x and tMax.z < tMax.y
                    step_dir_mask.z = 1.0;
                 }

                voxel_pos = voxel_pos + vec3<i32>(step_dir_mask * step_sign);
                tMax = tMax + tDelta * step_dir_mask;
                current_voxel_id = GetVoxel(vec3<f32>(voxel_pos)).x; // Check new voxel
                hit_normal = -step_dir_mask * step_sign; // Normal is opposite the step direction
            }

             // If refinement found a voxel
             if (current_voxel_id != 0) {
                 // Calculate final fraction based on DDA exit point
                 let tFinal = tMax - tDelta * abs(hit_normal); // Step back one delta along normal
                 fraction = dot(tFinal, abs(hit_normal)); // Project final t onto normal axis
                 *normal_out = hit_normal;
                 *material_out = GetMat(current_voxel_id);
                 *fraction_inout = fraction; // Update original fraction variable
                 return true; // Hit!
             } else {
                // DDA refinement failed to find the hit? Should be rare if initial check was solid.
                // Fallback or treat as miss. For simplicity, treat as miss.
                return false;
             }
        }
        // Continue loop if current voxel is air (0) but block is not empty
    }

    *left_bounds_out = true; // Exceeded max distance
    return false; // No hit within max distance
}


// Processes a ray hit, updating ray origin, direction, and accumulated light/reflection
fn ProcessHit(origin_inout: ptr<function, vec3<f32>>, direction_inout: ptr<function, vec3<f32>>,
              fraction: f32, normal: vec3<f32>, material: Material,
              accumulated_light_inout: ptr<function, vec3<f32>>, accumulated_reflection_inout: ptr<function, vec3<f32>>) {

    let hit_pos = *origin_inout + (fraction * *direction_inout);
    // Apply small offset along normal to avoid self-intersection
    *origin_inout = hit_pos + normal * 0.01;

    // Sample diffuse lighting at the hit point
    let diffuse_light = sample_radiance(*origin_inout, normal);

    // Update accumulated reflection (albedo)
    *accumulated_reflection_inout = *accumulated_reflection_inout * material.color;
    // Add emitted and diffuse light, modulated by accumulated reflection
    *accumulated_light_inout = *accumulated_light_inout + *accumulated_reflection_inout * (material.emmitance + diffuse_light);

    // Reflect the ray direction
    *direction_inout = reflect(*direction_inout, normal);

    // TODO: Add roughness handling (e.g., sample lobe, mix reflection/diffuse based on roughness)
}

// Traces a single glossy ray bounce
fn trace_glossy_ray(rayOrigin: vec3<f32>, rayDirection: vec3<f32>,
                    accumulated_light_in: vec3<f32>, accumulated_reflection_in: vec3<f32>) -> vec3<f32> {
    var fraction: f32 = 0.0;
    var normal: vec3<f32> = vec3<f32>(0.0);
    var material: Material;
    var left_bounds: bool = false;

    // Use local mutable copies for inout parameters passed to CastRay/ProcessHit
    var origin = rayOrigin;
    var direction = rayDirection;
    var light = accumulated_light_in;
    var reflection = accumulated_reflection_in;

    let hit = CastRay_fast(origin, direction, &fraction, &normal, &material, &left_bounds);

    if (hit) {
        ProcessHit(&origin, &direction, fraction, normal, material, &light, &reflection);
        // For multi-bounce, could add another CastRay call here with updated origin/direction
    } else {
        // If ray missed or left bounds, sample skybox/environment light
        // Simplified sky light based on direction vs global light
        let global_light_participance = max(0.0, -dot(direction, ubo.global_light_dir.xyz)); // Use corrected UBO name
        if (global_light_participance > 0.9) {
            // Sun highlight
            light = light + (vec3(0.9, 0.9, 0.6) * 0.5) * reflection * (global_light_participance - 0.9) * 10.0;
        }
        // Base sky blue
        light = light + (vec3(0.53, 0.81, 0.92) * 0.1) * reflection;
    }
    return light;
}

// Loads normal from matNorm texture (assuming RGBA8 format)
fn load_norm(pixel_coord: vec2<i32>, texture_size: vec2<i32>) -> vec3<f32> {
    // Clamp coordinates to avoid out-of-bounds reads
    let clamped_coord = clamp(pixel_coord, vec2<i32>(0), texture_size - vec2<i32>(1));
    // Load raw uint value (assuming RGBA8ui format for mat_norm_tex)
    let encoded_norm = textureLoad(mat_norm_tex, clamped_coord, 0); // LOD 0
    // Decode: Assuming normal stored in GBA, material ID in R. Need format clarification.
    // If RGBA8Unorm -> vec4<f32>, decode like this:
    // let norm_f32 = vec4<f32>(encoded_norm) / 255.0;
    // return norm_f32.gba * 2.0 - 1.0;
    // If RGBA8Uint -> vec4<u32>, decode like this:
    let norm_f32 = vec4<f32>(f32(encoded_norm.y), f32(encoded_norm.z), f32(encoded_norm.w), f32(encoded_norm.x)) / 255.0; // Assuming GBA order in YZW, matID in X
    return norm_f32.xyz * 2.0 - 1.0; // Assuming GBA needs decoding
}

// Loads material ID from matNorm texture
fn load_mat(pixel_coord: vec2<i32>, texture_size: vec2<i32>) -> i32 {
    let clamped_coord = clamp(pixel_coord, vec2<i32>(0), texture_size - vec2<i32>(1));
    let encoded_mat = textureLoad(mat_norm_tex, clamped_coord, 0); // LOD 0
    // Assuming material ID is stored in the first component (R or X)
    return i32(encoded_mat.x);
}

// Loads depth from depth texture using normalized UVs
fn load_depth(uv: vec2<f32>) -> f32 {
     // Use textureSample with depth texture and sampler
    let depth_encoded = textureSample(depthBuffer_tex, depth_samp, uv);
    // GLSL code multiplied by 1000, apply same scaling
    return depth_encoded * 1000.0;
}

// Color encoding/decoding
fn decode_color(encoded_color: vec3<f32>) -> vec3<f32> {
    return encoded_color * COLOR_ENCODE_VALUE;
}

fn encode_color(color: vec3<f32>) -> vec3<f32> {
    return color / COLOR_ENCODE_VALUE;
}

// --- Fragment Entry Point ---
@fragment
fn main(@builtin(position) frag_coord: vec4<f32>) -> FragmentOutput {
    var output: FragmentOutput;

    let texture_size = vec2<i32>(textureDimensions(mat_norm_tex, 0));
    let pix_coord = vec2<i32>(frag_coord.xy); // Integer pixel coordinates

    // Load initial material, normal, depth from G-Buffer textures
    let initial_mat = GetMat(load_mat(pix_coord, texture_size));
    let initial_normal = load_norm(pix_coord, texture_size);
    // Calculate UVs for depth sampling
    let uv = frag_coord.xy / vec2<f32>(texture_size);
    let initial_depth = load_depth(uv);

    // Reconstruct world position
    let clip_pos_ndc = frag_coord.xy / ubo.frame_size * 2.0 - 1.0;
    var origin = get_origin_from_depth(initial_depth, clip_pos_ndc);
    var direction = ubo.camdir.xyz; // Initial ray direction is camera view direction

    var accumulated_light = vec3<f32>(0.0);
    var accumulated_reflection = vec3<f32>(1.0);

    // Process the first hit (surface properties from G-Buffer)
    // This applies emittance/radiance at the first hit surface
    ProcessHit(&origin, &direction, 0.0, initial_normal, initial_mat,
               &accumulated_light, &accumulated_reflection);

    // Trace the first reflection bounce
    let traced_color = trace_glossy_ray(origin, direction, accumulated_light, accumulated_reflection);

    // Encode final color and set alpha based on roughness (matches GLSL)
    output.frame_color = vec4<f32>(encode_color(traced_color), 1.0 - initial_mat.roughness);

    return output;
}