// Global constants. TODO: move to specialization constants
const WORLD_SIZE: vec3<i32> = vec3<i32>(48, 48, 16);
const COLOR_ENCODE_VALUE: f32 = 1.0;
const PI: f32 = 3.1415926535;

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

@group(0) @binding(0) var<uniform> ubo: UboData;

@group(0) @binding(1) var material_normal_texture: texture_2d<u32>;
@group(0) @binding(2) var depth_buffer_texture: texture_depth_2d;

@group(0) @binding(3) var voxel_palette_texture: texture_2d<f32>;
@group(0) @binding(4) var nearest_sampler: sampler;

@group(0) @binding(5) var radiance_cache_texture: texture_3d<f32>;
@group(0) @binding(6) var linear_sampler: sampler; 

@group(0) @binding(7) var lightmap_texture: texture_depth_2d;
@group(0) @binding(8) var lightmap_comparison_sampler: sampler_comparison;

struct Material {
    color_emission: vec4<f32>,
    roughness: f32,
    transparency: f32,
};

struct FragmentOutput {
    @location(0) frame_color: vec4<f32>,
};

// Samples a probe from radiance cache.
fn sample_probe(probe_ipos: vec3<i32>, direction: vec3<f32>) -> vec3<f32> {
    // TODO: hw clamp
    let probe_ipos_clamped = clamp(probe_ipos, vec3<i32>(0), WORLD_SIZE - vec3<i32>(1));
    let light = textureLoad(radiance_cache_texture, probe_ipos_clamped, 0).rgb;
    // TODO: reasonable range?
    return clamp(light, vec3<f32>(0.0), vec3<f32>(2.0));
}

fn square(value: f32) -> f32 {
    return value * value;
}

fn sample_radiance_directional(position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    var total_weight: f32 = 0.0;
    var total_color: vec3<f32> = vec3<f32>(0.0);

    let zero_probe_ipos = clamp(vec3<i32>(floor(position - 8.0) / 16.0), vec3<i32>(0), WORLD_SIZE - vec3<i32>(1));
    let zero_probe_position = vec3<f32>(zero_probe_ipos) * 16.0 + 8.0;

    let alpha = clamp((position - zero_probe_position) / 16.0, vec3<f32>(0.0), vec3<f32>(1.0));

    for (var i: i32 = 0; i < 8; i = i + 1) {
        let offset = vec3<i32>(i, i >> 1, i >> 2) & vec3<i32>(1);

        var probe_weight: f32 = 1.0;
        var probe_color: vec3<f32> = vec3<f32>(0.0);

        let probe_position = zero_probe_position + vec3<f32>(offset) * 16.0;

        let probe_to_point = probe_position - position;
        let direction_to_probe = normalize(probe_to_point);

        let trilinear_factor = mix(vec3<f32>(1.0) - alpha, alpha, vec3<f32>(offset));
        probe_weight = trilinear_factor.x * trilinear_factor.y * trilinear_factor.z;

        // clamped to [0.1, 1.0] to prevent zero weight (we want it to have some impact).
        let direction_weight = clamp(dot(direction_to_probe, normal), 0.1, 1.0);
        probe_weight = probe_weight * direction_weight;

        probe_color = sample_probe(zero_probe_ipos + offset, direction_to_probe);

        // prevent division by very small numbers
        probe_weight = max(1e-7, probe_weight);
        total_weight += probe_weight;
        total_color += probe_weight * probe_color;
    }

    if total_weight < 1e-6 {
        return vec3<f32>(0.0);
    }
    return total_color / total_weight;
}

fn sample_radiance_simple(position: vec3<f32>) -> vec3<f32> {
    let block_position = position / 16.0;
    let uv = block_position / vec3<f32>(WORLD_SIZE);
    let sampled_light = textureSampleLevel(radiance_cache_texture, linear_sampler, uv, 0.0).rgb;
    return sampled_light;
}

fn load_normal(frag_coord_xy: vec2<i32>) -> vec3<f32> {
    let loaded_value = textureLoad(material_normal_texture, frag_coord_xy, 0); 
    let normal = (vec3<f32>(loaded_value.gba) / 255.0 * 2.0) - 1.0;
    return normal;
}

fn load_material_id(frag_coord_xy: vec2<i32>) -> i32 {
    let loaded_value = textureLoad(material_normal_texture, frag_coord_xy, 0); 
    let material_id = i32(loaded_value.x);
    return material_id;
}

fn load_depth(frag_coord_xy: vec2<i32>) -> f32 {
    var encoded_depth = textureLoad(depth_buffer_texture, frag_coord_xy, 0); 
    return encoded_depth * 1000.0;
}

fn get_material(voxel_id: i32) -> Material {
    var material: Material;

    material.color_emission.r = textureLoad(voxel_palette_texture, vec2<i32>(0, voxel_id), 0).r;
    material.color_emission.g = textureLoad(voxel_palette_texture, vec2<i32>(1, voxel_id), 0).r;
    material.color_emission.b = textureLoad(voxel_palette_texture, vec2<i32>(2, voxel_id), 0).r;
    material.color_emission.w = textureLoad(voxel_palette_texture, vec2<i32>(4, voxel_id), 0).r;
    material.roughness = textureLoad(voxel_palette_texture, vec2<i32>(5, voxel_id), 0).r;
    material.transparency = 0.0;

    return material;
}

fn get_origin_from_depth(depth: f32, clip_pos: vec2<f32>) -> vec3<f32> {
    let origin = ubo.camera_position.xyz + (ubo.horizontal_line_scaled.xyz * clip_pos.x) + (ubo.vertical_line_scaled.xyz * clip_pos.y) + (ubo.camera_direction.xyz * depth);
    return origin;
}

fn next_after_float(x: f32, s: i32) -> f32 {
    let ix = bitcast<u32>(x);
    let fxp1 = bitcast<f32>(ix + u32(s));
    return fxp1;
}

fn next_after_float_one(x: f32) -> f32 {
    return next_after_float(x, 1);
}

fn previous_before_float(x: f32, s: i32) -> f32 {
    let ix = bitcast<u32>(x);
    let fxp1 = bitcast<f32>(ix - u32(s));
    return fxp1;
}

fn previous_before_float_one(x: f32) -> f32 {
    return previous_before_float(x, 1);
}

fn sample_lightmap_with_shift(base_uv: vec2<f32>, test_depth: f32, offset: vec2<f32>) -> f32 {
    var shadow = textureSampleCompare(lightmap_texture, lightmap_comparison_sampler, base_uv + offset, test_depth);
    return shadow;
}

fn sample_lightmap(world_pos: vec3<f32>, normal: vec3<f32>) -> f32 {
    var biased_position = world_pos;

    // bias to prevent "shadow acne" by pushing the sample point
    if dot(normal, ubo.global_light_direction.xyz) > 0.0 {
        biased_position -= normal * 0.9;
    } else {
        biased_position += normal * 0.9;
    }

    var light_clip = (ubo.lightmap_projection * vec4<f32>(biased_position, 1.0));
    light_clip.z = 1.0 + light_clip.z;

    var light_uv = (light_clip.xy + 1.0) / 2.0;
    light_uv.y = 1.0 - light_uv.y;

    let world_depth_in_light_space = light_clip.z;

    let pcf_shift = vec2<f32>(1.0 / 1024.0);
    var total_light: f32 = 0.0;

    total_light += sample_lightmap_with_shift(light_uv, world_depth_in_light_space, vec2<f32>(-pcf_shift.x, 0.0));
    total_light += sample_lightmap_with_shift(light_uv, world_depth_in_light_space, vec2<f32>(0.0, 0.0));
    total_light += sample_lightmap_with_shift(light_uv, world_depth_in_light_space, vec2<f32>(pcf_shift.x, 0.0));
    total_light += sample_lightmap_with_shift(light_uv, world_depth_in_light_space, vec2<f32>(0.0, -pcf_shift.y));
    total_light += sample_lightmap_with_shift(light_uv, world_depth_in_light_space, vec2<f32>(0.0, pcf_shift.y));

    return ((total_light / 5.0) * 0.15);
}

fn decode_color(encoded_color: vec3<f32>) -> vec3<f32> {
    return encoded_color * COLOR_ENCODE_VALUE;
}

fn encode_color(color: vec3<f32>) -> vec3<f32> {
    return color / COLOR_ENCODE_VALUE;
}

@fragment
fn main(@builtin(position) fragment_coordinate: vec4<f32>) -> FragmentOutput {
    var output: FragmentOutput;
    let fragment_coordinate_xy = vec2<i32>(fragment_coordinate.xy);

    let material_id = load_material_id(fragment_coordinate_xy);

    let stored_material: Material = get_material(material_id);
    let stored_normal: vec3<f32> = load_normal(fragment_coordinate_xy);
    let current_depth: f32 = load_depth(fragment_coordinate_xy);

    var clip_position = (fragment_coordinate.xy / ubo.frame_size) * 2.0 - 1.0;

    var origin = get_origin_from_depth(current_depth, clip_position);

    // 6.0 shift in direction of normal is vibe calculated 
    let probe_light = sample_radiance_directional(origin + stored_normal * 6.0, stored_normal);

    // direct sunlight. Distinct from probes cause we are humans. And for humans sun is a special thing.
    let sunlight = sample_lightmap(origin, stored_normal);

    var final_color = (2.0 * probe_light + stored_material.color_emission.w + sunlight) * stored_material.color_emission.rgb;

    output.frame_color = vec4<f32>(encode_color(final_color), 1.0);

    return output;
}
