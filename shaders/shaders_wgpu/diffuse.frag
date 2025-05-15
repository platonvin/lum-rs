// WGSL translation of diffuse.frag

const world_size: vec3<i32> = vec3<i32>(48, 48, 16); // Example value
const COLOR_ENCODE_VALUE: f32 = 1.0;
const RAYS_PER_PROBE: i32 = 32; // Seems unused in fragment shader logic provided
const PI: f32 = 3.1415926535;

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
}
;

@group(0) @binding(0) var<uniform> ubo: UboData;

// --- Bindings ---
// Input Attachments treated as Textures
@group(0) @binding(1) var matNorm_tex: texture_2d<u32>;
@group(0) @binding(2) var depthBuffer_tex: texture_depth_2d; // subpassInput depthBuffer

@group(0) @binding(3) var voxelPalette_tex: texture_2d<f32>; // sampler2D voxelPalette (Assuming R32Float format)
@group(0) @binding(4) var nearest_samp: sampler; // Sampler for voxelPalette_tex

@group(0) @binding(5) var radianceCache_tex: texture_3d<f32>; // sampler3D radianceCache
@group(0) @binding(6) var linear_samp: sampler; // Sampler for radianceCache_tex

@group(0) @binding(7) var lightmap_tex: texture_depth_2d; // sampler2DShadow lightmap
@group(0) @binding(8) var lightmap_samp: sampler_comparison; // Comparison sampler for lightmap_tex

// --- Structs ---
struct Material {
    color_emmit: vec4<f32>,
    // emmitance: f32,
    roughness: f32,
    transparancy: f32, // Note: transparancy wasn't loaded in GLSL GetMat
}
;

struct FragmentOutput {
    @location(0) frame_color: vec4<f32>,
}
;

// --- Helper Functions ---
fn sample_probe(probe_ipos: vec3<i32>, direction: vec3<f32>) -> vec3<f32> {
    // probe_ipos_clamped uses world_size constant directly
    let probe_ipos_clamped = clamp(probe_ipos, vec3<i32>(0), world_size - vec3<i32>(1)); // WGSL textures use 0-based indexing up to size-1
    // subprobe_pos calculation seems identical to probe_ipos_clamped, simplifying:
    let light = textureLoad(radianceCache_tex, probe_ipos_clamped, 0).rgb; // textureLoad takes i32 coords
    return clamp(light, vec3<f32>(0.0), vec3<f32>(2.0));
}

fn square(a: f32) -> f32 { return a * a; }

fn sample_radiance_directional(position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    var total_weight: f32 = 0.0;
    var total_colour: vec3<f32> = vec3<f32>(0.0);

    // world_size constant used here
    let zero_probe_ipos = clamp(vec3<i32>(floor(position - 8.0) / 16.0), vec3<i32>(0), world_size - vec3<i32>(1));
    let zero_probe_pos = vec3<f32>(zero_probe_ipos) * 16.0 + 8.0;

    let alpha = clamp((position - zero_probe_pos) / 16.0, vec3<f32>(0.0), vec3<f32>(1.0));

    for (var i: i32 = 0; i < 8; i = i + 1) {
        let offset = vec3<i32>(i, i >> 1, i >> 2) & vec3<i32>(1);

        var probe_weight: f32 = 1.0;
        var probe_colour: vec3<f32> = vec3<f32>(0.0);

        let probe_pos = zero_probe_pos + vec3<f32>(offset) * 16.0;

        let probeToPoint = probe_pos - position;
        let direction_to_probe = normalize(probeToPoint);

        let trilinear = mix(vec3<f32>(1.0) - alpha, alpha, vec3<f32>(offset));
        probe_weight = trilinear.x * trilinear.y * trilinear.z;

        let direction_weight = clamp(dot(direction_to_probe, normal), 0.1, 1.0);
        probe_weight = probe_weight * direction_weight;

        probe_colour = sample_probe(zero_probe_ipos + offset, direction_to_probe);

        probe_weight = max(1e-7, probe_weight);
        total_weight += probe_weight;
        total_colour += probe_weight * probe_colour;
    }

    if total_weight < 1e-6 {
        // Prevent division by zero if total_weight is extremely small
        return vec3<f32>(0.0);
    }
    return total_colour / total_weight;
}

// Overloaded function for non-directional sampling (seems unused later)
fn sample_radiance_simple(position: vec3<f32>) -> vec3<f32> {
    let block_pos = position / 16.0;
    // world_size constant used here
    let uv = block_pos / vec3<f32>(world_size);
    // Use textureSampleLevel with explicit sampler for LOD 0
    let sampled_light = textureSampleLevel(radianceCache_tex, linear_samp, uv, 0.0).rgb;
    return sampled_light;
}


fn load_norm(frag_coord_xy: vec2<i32>) -> vec3<f32> {
    // Load from texture bound at binding 1
    // Assuming mat ID is in .x and normal in .gba, encoded as u8
    let loaded_val = textureLoad(matNorm_tex, frag_coord_xy, 0); // LOD 0
    // Decode GBA components from [0,1] to [-1,1]
    let norm = (vec3f(loaded_val.gba) / 255.0 * 2.0) - 1.0;
    return norm;
}

fn load_mat(frag_coord_xy: vec2<i32>) -> i32 {
    // Load from texture bound at binding 1
    // Assuming mat ID is in .x component
    let loaded_val = textureLoad(matNorm_tex, frag_coord_xy, 0); // LOD 0
    // Assuming mat ID is stored directly, possibly needs scaling if stored as u8
    let mat = i32(loaded_val.x); // Convert f32 to i32
    return mat;
}

fn load_depth(frag_coord_xy: vec2<i32>) -> f32 {
    // Load from depth texture bound at binding 2
    var depth_encoded = textureLoad(depthBuffer_tex, frag_coord_xy, 0); // LOD 0

    return depth_encoded * 1000.0;
}

fn GetMat(voxel: i32) -> Material {
    var mat: Material;

    var v = voxel;

    // Use textureLoad instead of texelFetch, needs texture and integer coords
    // Assuming voxelPalette_tex format allows direct loading (e.g., R32Float)
    mat.color_emmit.r = textureLoad(voxelPalette_tex, vec2<i32>(0, v), 0).r;
    mat.color_emmit.g = textureLoad(voxelPalette_tex, vec2<i32>(1, v), 0).r;
    mat.color_emmit.b = textureLoad(voxelPalette_tex, vec2<i32>(2, v), 0).r;
    // mat.transparancy = 1.0 - textureLoad(voxelPalette_tex, vec2<i32>(3, voxel), 0).r; // Was commented out
    mat.color_emmit.w = textureLoad(voxelPalette_tex, vec2<i32>(4, v), 0).r;
    mat.roughness = textureLoad(voxelPalette_tex, vec2<i32>(5, v), 0).r;
    mat.transparancy = 0.0; // Initialize explicitly if needed

    return mat;
}

fn get_origin_from_depth(depth: f32, clip_pos: vec2<f32>) -> vec3<f32> {
    let origin = ubo.campos.xyz + (ubo.horizline_scaled.xyz * clip_pos.x) + (ubo.vertiline_scaled.xyz * clip_pos.y) + (ubo.camdir.xyz * depth);
    return origin;
}

// fn get_origin_from_depth_interpolated(depth: f32, pos_interpol: vec3<f32>) -> vec3<f32> {
//     let origin = pos_interpol + (ubo.camdir.xyz * depth);
//     return origin;
// }

// WGSL equivalents for bit manipulation
fn next_after(x: f32, s: i32) -> f32 {
    let ix = bitcast<u32>(x);
    let fxp1 = bitcast<f32>(ix + u32(s)); // Cast s to u32 for bitwise add
    return fxp1;
}
fn next_after_1(x: f32) -> f32 { return next_after(x, 1); }

fn prev_befor(x: f32, s: i32) -> f32 {
    let ix = bitcast<u32>(x);
    let fxp1 = bitcast<f32>(ix - u32(s)); // Cast s to u32 for bitwise sub
    return fxp1;
}
fn prev_befor_1(x: f32) -> f32 { return prev_befor(x, 1); }


fn sample_lightmap_with_shift(base_uv: vec2<f32>, test_depth: f32, offset: vec2<f32>) -> f32 {
    var shadow = textureSampleCompare(lightmap_tex, lightmap_samp, base_uv + offset, test_depth);
    return shadow; // Returns 1.0 if not shadowed, 0.0 if shadowed (or potentially interpolated value)
}

fn sample_lightmap(world_pos: vec3<f32>, normal: vec3<f32>) -> f32 {
    var biased_pos = world_pos;

    if dot(normal, ubo.global_light_dir.xyz) > 0.0 {
        biased_pos -= normal * 0.9;
    } else {
        biased_pos += normal * 0.9;
    }

    var light_clip = (ubo.lightmap_proj * vec4<f32>(biased_pos, 1.0));
    light_clip.z = 1.0 + light_clip.z;

    var light_uv = (light_clip.xy + 1.0) / 2.0; // Convert clip space [-1, 1] to UV [0, 1]
    light_uv.y = 1.0 - light_uv.y; // Flip V

    let world_depth_in_light_space = light_clip.z; // Depth in light's view [0, 1] or similar range

    let pcfshift = vec2<f32>(1.0 / 1024.0);
    var total_light: f32 = 0.0;

    total_light += sample_lightmap_with_shift(light_uv, world_depth_in_light_space, vec2<f32>(-pcfshift.x, 0.0));
    total_light += sample_lightmap_with_shift(light_uv, world_depth_in_light_space, vec2<f32>(0.0, 0.0));
    total_light += sample_lightmap_with_shift(light_uv, world_depth_in_light_space, vec2<f32>(pcfshift.x, 0.0));
    total_light += sample_lightmap_with_shift(light_uv, world_depth_in_light_space, vec2<f32>(0.0, -pcfshift.y));
    total_light += sample_lightmap_with_shift(light_uv, world_depth_in_light_space, vec2<f32>(0.0, pcfshift.y));

    return ((total_light / 5.0) * 0.15);
    // return 0.5;
}

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
    let frag_coord_xy = vec2<i32>(frag_coord.xy); // Integer coordinates for textureLoad

    let mat_id = load_mat(frag_coord_xy);

    // Skip processing if material ID indicates empty space (e.g., ID 0)
    // This can be an optimization if empty space doesn't need lighting.
    // if (mat_id == 0) {
    //     discard; // Optional: discard fragment entirely
    //     // Or return background color:
    //     // output.frame_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    //     // return output;
    // }

    let stored_mat: Material = GetMat(mat_id);
    let stored_normal: vec3<f32> = load_norm(frag_coord_xy);
    let current_depth: f32 = load_depth(frag_coord_xy);

    var clip_pos = (frag_coord.xy / ubo.frame_size) * 2.0 - 1.0;
    // clip_pos.y = 1.0 - clip_pos.y;

    // Reconstruct world position
    var origin = get_origin_from_depth(current_depth, clip_pos);
    // origin = abs(origin);
    // let w = vec3f(world_size) * 16.0;
    // origin = w - origin;

    // Sample lighting
    // Applying normal offset before sampling radiance, as in GLSL
    let probe_light = sample_radiance_directional(origin + stored_normal * 6.0, stored_normal);
    // cause we are humans. And for humans sun is a special thing
    let sunlight = sample_lightmap(origin, stored_normal);

    // Combine lighting and material properties
    // Factor 2.0 applied to incoming_light as in GLSL
    var final_color = (2.0 * probe_light + stored_mat.color_emmit.w + sunlight) * stored_mat.color_emmit.rgb;
    // final_color = vec3f(sunlight);

    // Encode and set output color
    output.frame_color = vec4<f32>(encode_color(final_color), 1.0);

    return output;
}
