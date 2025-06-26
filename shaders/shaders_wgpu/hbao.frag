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

struct AoLutEntry {
    world_shift: vec3<f32>,
    weight_normalized: f32, // Normalized weight [0, ~0.7]
    screen_shift: vec2<f32>, // Precomputed screen space UV offset
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(0) @binding(1) var<uniform> lut_buffer: array<AoLutEntry, 8>;
@group(0) @binding(2) var material_normal_texture: texture_2d<u32>;
@group(0) @binding(3) var depth_buffer_texture: texture_depth_2d;
@group(0) @binding(4) var depth_sampler: sampler;

const COLOR_ENCODE_VALUE: f32 = 1.0;
const SAMPLE_COUNT: i32 = 8;

struct FragmentOutput {
    @location(0) frame_color: vec4<f32>,
};

fn load_normal(fragment_coordinate_xy: vec2<i32>) -> vec3<f32> {
    let loaded_value = textureLoad(material_normal_texture, fragment_coordinate_xy, 0);
    let normal = (vec3<f32>(loaded_value.gba) / 255.0 * 2.0) - 1.0;
    return normal;
}

// fn load_material_id(fragment_coordinate_xy: vec2<i32>) -> i32 {
//     let loaded_value = textureLoad(material_normal_texture, fragment_coordinate_xy, 0);
//     return i32(loaded_value.x);
// }

fn load_depth(uv: vec2<f32>) -> f32 {
    let encoded_depth = textureSampleLevel(depth_buffer_texture, depth_sampler, uv, 0);
    return f32(encoded_depth * 1000.0);
}

fn encode_color(color: vec3<f32>) -> vec3<f32> {
    return color / COLOR_ENCODE_VALUE;
}

fn rotate_2d(angle: f32) -> mat2x2<f32> {
    let sine = sin(angle);
    let cosine = cos(angle);
    return mat2x2<f32>(cosine, sine, -sine, cosine);
}

fn square(value: f32) -> f32 {
    return value * value;
}

@fragment
fn main(@builtin(position) fragment_coordinate: vec4<f32>) -> FragmentOutput {
    var output: FragmentOutput;

    let initial_uv = fragment_coordinate.xy / ubo.frame_size;

    let normal = load_normal(vec2<i32>(fragment_coordinate.xy));
    let initial_depth = load_depth(initial_uv);

    var total_ao: f32 = 0.0;

    for (var i: i32 = 0; i < SAMPLE_COUNT; i++) {
        let sample_data = lut_buffer[i];

        let screen_shift = sample_data.screen_shift;

        let current_depth = load_depth(initial_uv + screen_shift);
        let depth_difference = current_depth - initial_depth;

        let relative_position = sample_data.world_shift + (ubo.camera_direction.xyz * depth_difference);

        let direction = normalize(relative_position);

        let ao_contribution = max(dot(direction, normal), 0.0);

        var weight = sample_data.weight_normalized;

        // some magic
        let depth_attenuation = sqrt(clamp(8.0 + depth_difference, 0.0, 8.0) / 8.0);
        weight = weight * depth_attenuation;

        total_ao += ao_contribution * weight;
    }

    let obfuscation = total_ao; 

    // we occlude with writing black
    output.frame_color = (vec4<f32>(encode_color(vec3<f32>(0.0)), obfuscation));

    return output;
}
