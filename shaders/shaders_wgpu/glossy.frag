const world_size: vec3<i32> = vec3<i32>(48, 48, 16);
const PI: f32 = 3.1415926535;
const BLOCK_PALETTE_SIZE_X: i32 = 64;
const COLOR_ENCODE_VALUE: f32 = 1.0;

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

struct Material {
    color: vec3<f32>,
    emmitance: f32,
    roughness: f32,
};

struct FragmentOutput {
    @location(0) frame_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(0) @binding(1) var mat_norm_tex: texture_2d<u32>;
@group(0) @binding(2) var depthBuffer_tex: texture_depth_2d;
@group(0) @binding(3) var depth_samp: sampler;
@group(0) @binding(4) var world: texture_3d<i32>;
@group(0) @binding(5) var blockPalette_tex: texture_3d<i32>; // GLSL: usampler3D. Kept as i32 per user note.
@group(0) @binding(6) var voxelPalette_tex: texture_2d<f32>;
@group(0) @binding(7) var radianceCache_tex: texture_3d<f32>;
@group(0) @binding(8) var linear_samp: sampler;


fn get_origin_from_depth(depth: f32, clip_pos: vec2<f32>) -> vec3<f32> {
    let origin = ubo.campos.xyz + (ubo.horizline_scaled.xyz * clip_pos.x) + (ubo.vertiline_scaled.xyz * clip_pos.y) + (ubo.camdir.xyz * depth);
    return origin;
}

fn GetBlock(block_pos: vec3<i32>) -> i32 {
    // TODO:
    let clamped_pos = clamp(block_pos, vec3<i32>(0), world_size - vec3<i32>(1));
    let block = textureLoad(world, clamped_pos, 0).r;
    return block;
}

fn voxel_in_palette(relative_voxel_pos: vec3<i32>, block_id: i32) -> vec3<i32> {
    let block_x = block_id % BLOCK_PALETTE_SIZE_X;
    let block_y = block_id / BLOCK_PALETTE_SIZE_X;
    return relative_voxel_pos + vec3<i32>(16 * block_x, 16 * block_y, 0);
}

fn GetVoxel(pos: vec3<f32>) -> vec2<i32> {
    let ipos = vec3<i32>(floor(pos));
    let iblock_pos = ipos / 16;
    let relative_voxel_pos = ipos % 16;

    let block_id = GetBlock(iblock_pos);

    let voxel_pos_in_palette = voxel_in_palette(relative_voxel_pos, block_id);
    let voxel_mat_id = textureLoad(blockPalette_tex, voxel_pos_in_palette, 0).r;

    return vec2<i32>(voxel_mat_id, block_id);
}

fn GetMat(voxel_mat_id: i32) -> Material {
    var mat: Material;
        mat.color.r = textureLoad(voxelPalette_tex, vec2<i32>(0, voxel_mat_id), 0).r;
        mat.color.g = textureLoad(voxelPalette_tex, vec2<i32>(1, voxel_mat_id), 0).r;
        mat.color.b = textureLoad(voxelPalette_tex, vec2<i32>(2, voxel_mat_id), 0).r;
        mat.emmitance = textureLoad(voxelPalette_tex, vec2<i32>(4, voxel_mat_id), 0).r;
        mat.roughness = textureLoad(voxelPalette_tex, vec2<i32>(5, voxel_mat_id), 0).r;
    return mat;
}

fn sample_radiance(position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let offset_position = position + normal * 16.0;
    let radiance_block_coord = offset_position / 16.0;
    let uvw = (radiance_block_coord + 0.5) / vec3<f32>(world_size);

    let sampled_light = textureSampleLevel(radianceCache_tex, linear_samp, uvw, 0.0).rgb;
    return sampled_light;
}

fn initTvals_standard(rayOrigin: vec3<f32>, rayDirection: vec3<f32>) -> vec3<f32> {
    let inv_dir = 1.0 / rayDirection;
    let next_voxel_boundary = floor(rayOrigin) + step(vec3(0.0), rayDirection);
    let tMax = (next_voxel_boundary - rayOrigin) * inv_dir;
    return tMax;
}


fn CastRay_fast(origin: vec3<f32>, direction: vec3<f32>, fraction_inout: ptr<function, f32>, normal_out: ptr<function, vec3<f32>>, material_out: ptr<function, Material>, left_bounds_out: ptr<function, bool>) -> bool {
    var fraction: f32 = *fraction_inout;
    *left_bounds_out = false;

    let max_dist = 16.0 * 8.0; 
    let step_size = 0.5; // fast & precise enough. TODO: shader jit balance?

    let one_div_dir = 1.0 / direction;
    let step_sign = sign(direction);
    let b_pos_dir = direction > vec3(0.0);

    // "coarse" marching trough blocks until hit
    loop {
        if (fraction >= max_dist) {
            *left_bounds_out = true;
            return false;
        }

        fraction = fraction + step_size;
        let pos = origin + direction * fraction;

        // world bounds check
        if (any(pos < vec3(0.0)) || any(pos >= vec3<f32>(world_size * 16))) {
            *left_bounds_out = true;
            return false;
        }

        let voxel_info = GetVoxel(pos);
        let current_voxel = voxel_info.x;
        let current_block = voxel_info.y;

        // yeah this is faster...s
        if (current_block == 0) {
            let block_base_coord = floor(pos / 16.0) * 16.0;
            let precomputed_corner_offset = vec3<f32>(b_pos_dir) * 16.0;
            let far_corner_of_block = block_base_coord + precomputed_corner_offset;

            // t values to hit the planes of the far_corner_of_block
            let t_to_far_corner_planes = (far_corner_of_block - pos) * one_div_dir;
            // Smallest positive t is the exit distance
            var t_exit = min(t_to_far_corner_planes.x, min(t_to_far_corner_planes.y, t_to_far_corner_planes.z));

            // my sanity
            t_exit = max(t_exit, 0.01);

            fraction = fraction + t_exit;
            continue;
        }

        // If hit a non-empty voxel (current_voxel != 0 inside a non-empty block)
        if (current_voxel != 0) {
            // DDA Refinement part
            // Corrected: Backtrack amount from GLSL (fraction - 1.5)
            let precise_origin = origin + direction * (fraction - 1.5);

            // Corrected: tMax initialization specific to GLSL's DDA refinement
            var tMax: vec3<f32>;
            let inv_dir_dda = 1.0 / direction; // Safe if direction components are non-zero
            let floor_precise_origin = floor(precise_origin);

            let term1 = (floor_precise_origin - precise_origin) * inv_dir_dda;
            let term2 = (floor_precise_origin + 1.0 - precise_origin) * inv_dir_dda;

            tMax.x = max(term1.x, term2.x);
            tMax.y = max(term1.y, term2.y);
            tMax.z = max(term1.z, term2.z);

            let tDelta = abs(inv_dir_dda); // abs(1.0 / direction)
            var voxel_pos_dda = vec3<i32>(floor(precise_origin));
            var current_voxel_id_dda = GetVoxel(precise_origin).x; // Voxel at DDA start
            var hit_normal_dda = vec3(0.0);

            // DDA refinement loop 
            for (var dda_iter = 0; dda_iter < 5; dda_iter = dda_iter + 1) {
                if (current_voxel_id_dda != 0) { break; } // Found solid voxel

                var step_dir_mask = vec3(0.0); // Which axis to step along
                if (tMax.x <= tMax.y && tMax.x <= tMax.z) {
                    step_dir_mask.x = 1.0;
                } else if (tMax.y <= tMax.z) {
                    step_dir_mask.y = 1.0;
                } else {
                    step_dir_mask.z = 1.0;
                }

                voxel_pos_dda = voxel_pos_dda + vec3<i32>(step_dir_mask * step_sign);
                tMax = tMax + tDelta * step_dir_mask;
                // Check new voxel
                current_voxel_id_dda = GetVoxel(vec3<f32>(voxel_pos_dda) + vec3(0.5)).x; // Sample center of voxel
                hit_normal_dda = -step_dir_mask * step_sign;
            }

            if (current_voxel_id_dda != 0) {
                // Calculate final fraction
                let tFinal_components = tMax - tDelta; // tMax is exit, tMax-tDelta is entry
                var final_fraction_dda: f32 = 0.0;
                     if (hit_normal_dda.x != 0.0) { final_fraction_dda = tFinal_components.x; } 
                else if (hit_normal_dda.y != 0.0) { final_fraction_dda = tFinal_components.y; } 
                else                              { final_fraction_dda = tFinal_components.z; }

                // The fraction is relative to `precise_origin`. We need total fraction from 'origin'
                *fraction_inout = (fraction - 1.5) + final_fraction_dda;
                *normal_out = hit_normal_dda;
                *material_out = GetMat(current_voxel_id_dda);
                return true; // Hit!
            } else {
                // DDA refinement failed to find the hit after coarse step suggested one
                // TODO: do something smart?
                return false; 
            }
        }
    // Continue coarse step loop if current_voxel is air but block is not empty, or if block was skipped
    } 

    *left_bounds_out = true;
    return false;
}


fn ProcessHit(origin_inout: ptr<function, vec3<f32>>, direction_inout: ptr<function, vec3<f32>>, fraction: f32, normal: vec3<f32>, material: Material, accumulated_light_inout: ptr<function, vec3<f32>>, accumulated_reflection_inout: ptr<function, vec3<f32>>) {
    let hit_pos = *origin_inout + (fraction * *direction_inout);
    // small offset to avoid self-intersection
    *origin_inout = hit_pos + normal * 0.001;

    let diffuse_light = sample_radiance(*origin_inout, normal);

    *accumulated_reflection_inout = *accumulated_reflection_inout * material.color;
    *accumulated_light_inout = *accumulated_light_inout + *accumulated_reflection_inout * (material.emmitance + diffuse_light);

    *direction_inout = reflect(*direction_inout, normal);
}

fn trace_glossy_ray(rayOrigin: vec3<f32>, rayDirection: vec3<f32>, accumulated_light_in: vec3<f32>, accumulated_reflection_in: vec3<f32>) -> vec3<f32> {
    var fraction: f32 = 0.0;
    var normal: vec3<f32> = vec3<f32>(0.0);
    var material: Material;
    var left_bounds: bool = false;

    var origin = rayOrigin;
    var direction = rayDirection;
    var light = accumulated_light_in;
    var reflection = accumulated_reflection_in;

    let hit = CastRay_fast(origin, direction, &fraction, &normal, &material, &left_bounds);

    if (hit) {
        ProcessHit(&origin, &direction, fraction, normal, material, &light, &reflection);
    } else {
        // hardcoded sky logic
        let global_light_participance = max(0.0, -dot(direction, ubo.global_light_dir.xyz));
        if (global_light_participance > 0.9) {
            light = light + (vec3(0.9, 0.9, 0.6) * 0.5) * reflection * (global_light_participance - 0.9) * 10.0;
        }
        light = light + (vec3(0.53, 0.81, 0.92) * 0.1) * reflection;
    }
    return light;
}

fn load_norm(pixel_coord: vec2<i32>, texture_size: vec2<i32>) -> vec3<f32> {
    let clamped_coord = clamp(pixel_coord, vec2<i32>(0), texture_size - vec2<i32>(1));
    let encoded_norm_uvec = textureLoad(mat_norm_tex, clamped_coord, 0);
    let norm_f32_components = vec3<f32>(
        f32(encoded_norm_uvec.y),
        f32(encoded_norm_uvec.z),
        f32(encoded_norm_uvec.w)
    ) / 255.0;
    return norm_f32_components * 2.0 - 1.0;
}

fn load_mat(pixel_coord: vec2<i32>, texture_size: vec2<i32>) -> i32 {
    let clamped_coord = clamp(pixel_coord, vec2<i32>(0), texture_size - vec2<i32>(1));
    let encoded_mat_uvec = textureLoad(mat_norm_tex, clamped_coord, 0);
    return i32(encoded_mat_uvec.x);
}

fn load_depth(uv: vec2<f32>) -> f32 {
    let depth_encoded = textureSample(depthBuffer_tex, depth_samp, uv);
    return depth_encoded * 1000.0;
}

fn decode_color(encoded_color: vec3<f32>) -> vec3<f32> {
    return encoded_color * COLOR_ENCODE_VALUE;
}

fn encode_color(color: vec3<f32>) -> vec3<f32> {
    return color / COLOR_ENCODE_VALUE;
}

@fragment
fn main(@builtin(position) frag_coord_in: vec4<f32>) -> FragmentOutput {
    var output: FragmentOutput;

    let tex_dims = textureDimensions(mat_norm_tex, 0);
    let texture_size = vec2<i32>(i32(tex_dims.x), i32(tex_dims.y));
    let pix_coord = vec2<i32>(frag_coord_in.xy);

    let initial_mat_id = load_mat(pix_coord, texture_size);
    let initial_mat = GetMat(initial_mat_id);
    let initial_normal = load_norm(pix_coord, texture_size);

    let uv_for_depth = frag_coord_in.xy / ubo.frame_size;
    let initial_depth = load_depth(uv_for_depth);

    let clip_pos_ndc = frag_coord_in.xy / ubo.frame_size * 2.0 - 1.0;
    var origin = get_origin_from_depth(initial_depth, clip_pos_ndc);
    var direction = ubo.camdir.xyz;

    var accumulated_light = vec3<f32>(0.0);
    var accumulated_reflection = vec3<f32>(1.0);

    // Process the first hit (where we start from)
    // The reason why we dont raytrace from camera is beacause there is no fucking reason to do so
    ProcessHit(&origin, &direction, 0.0, initial_normal, initial_mat, &accumulated_light, &accumulated_reflection);

    let traced_color = trace_glossy_ray(origin, direction, accumulated_light, accumulated_reflection);

    // output.frame_color = vec4<f32>(encode_color(traced_color), 1.0 - initial_mat.roughness);
    output.frame_color = vec4<f32>(encode_color(traced_color), 1.0);

    return output;
}
