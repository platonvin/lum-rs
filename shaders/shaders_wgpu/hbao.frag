// hbao.frag (WGSL translation)

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


// --- AO Look-Up Table (LUT) Structure and Binding ---
struct AoLutEntry {
    world_shift: vec3<f32>,
    weight_normalized: f32, // Normalized weight [0, ~0.7]
    screen_shift: vec2<f32>, // Precomputed screen space UV offset
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(0) @binding(1) var<uniform> lut_buffer: array<AoLutEntry, 8>;
@group(0) @binding(2) var matNorm_tex: texture_2d<u32>; // usubpassInput matNorm (Assuming RGBA format)
@group(0) @binding(3) var depthBuffer_tex: texture_depth_2d; // sampler2D depthBuffer
@group(0) @binding(4) var depth_samp: sampler; // Sampler for depthBuffer

const COLOR_ENCODE_VALUE: f32 = 1.0;
const SAMPLE_COUNT: i32 = 8;

struct FragmentOutput {
    @location(0) frame_color: vec4<f32>,
}
;

fn load_norm(frag_coord_xy: vec2<i32>) -> vec3<f32> {
    let loaded_val = textureLoad(matNorm_tex, frag_coord_xy, 0); // LOD 0
    let norm = (vec3f(loaded_val.gba) / 256.0 * 2.0) - 1.0;
    return norm;
}

// Loads material ID (unused in this shader, but kept for consistency if needed)
// fn load_mat(frag_coord_xy: vec2<i32>) -> i32 {
//     let loaded_val = textureLoad(matNorm_tex, frag_coord_xy, 0); // LOD 0
//     return i32(loaded_val.x); // Assuming mat ID is in X component
// }

fn load_depth(uv: vec2<f32>) -> f32 {
    let depth_encoded = textureSampleLevel(depthBuffer_tex, depth_samp, uv, 0);
    return f32(depth_encoded * 1000.0);
}

// Color encoding
fn encode_color(color: vec3<f32>) -> vec3<f32> {
    return color / COLOR_ENCODE_VALUE;
}

// Rotate 2D vector
fn rotate2d(angle: f32) -> mat2x2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return mat2x2<f32>(c, s, -s, c); // WGSL constructor column-major
}

fn square(x: f32) -> f32 { return x * x; }

// --- Fragment Entry Point ---
@fragment
fn main(@builtin(position) frag_coord: vec4<f32>) -> FragmentOutput {
    var output: FragmentOutput;

    // Calculate initial screen UVs [0, 1]
    let initial_uv = frag_coord.xy / ubo.frame_size;

    // Load normal and depth for the current fragment
    let normal = load_norm(vec2<i32>(frag_coord.xy));
    let initial_depth = load_depth(initial_uv);

    var total_ao: f32 = 0.0;

    // Loop through precomputed samples in the LUT
    for (var i: i32 = 0; i < SAMPLE_COUNT; i++) {
        let sample_data = lut_buffer[i];

        // Get precomputed screen UV offset
        let screen_shift = sample_data.screen_shift;

        // Sample depth at the offset UV coordinate
        let current_depth = load_depth(initial_uv + screen_shift);
        let depth_shift = current_depth - initial_depth; // Difference in depth values

        // Reconstruct relative world position using precomputed world shift and depth difference
        // world_shift accounts for (horizline*clip_x + vertiline*clip_y) part
        let relative_pos = sample_data.world_shift + (ubo.camdir.xyz * depth_shift);

        let direction = normalize(relative_pos);

        // Calculate AO contribution: how much the direction aligns with the normal
        let ao_contribution = max(dot(direction, normal), 0.0);

        // Get precomputed weight for this sample
        var weight = sample_data.weight_normalized;

        let depth_attenuation = sqrt(clamp(8.0 + depth_shift, 0.0, 8.0) / 8.0);
        weight = weight * depth_attenuation;

        // Accumulate weighted AO contribution
        total_ao += ao_contribution * weight;
    }

    // Final AO value (already weighted and summed)
    let obfuscation = total_ao; // Clamp to [0, 1] range

    // Output AO factor in the alpha channel, color is black (encoded)
    output.frame_color = (vec4(encode_color(vec3(0.0)), obfuscation));
    // output.frame_color = vec4<f32>(encode_color(vec3(0.0)), 0.0);

    return output;
}
