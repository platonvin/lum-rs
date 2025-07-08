#![no_std]
#![allow(unexpected_cfgs)]

mod grass;

use core::mem::transmute;

use fastnoise_lite::*;
use spirv_std::arch::Derivative;
use spirv_std::glam::*;
use spirv_std::glam::{bvec3, ivec3, vec3, Mat2};
use spirv_std::glam::{BVec3, FloatExt};
use spirv_std::image::SampledImage;
use spirv_std::num_traits::Float;
use spirv_std::{spirv, Image};

pub const WORLD_SIZE: IVec3 = IVec3::new(48, 48, 16);
pub const PI: f32 = 3.1415926535;
pub const BLOCK_PALETTE_SIZE_X: i32 = 64;
pub const RAYS_PER_PROBE: u32 = 64;
pub const REACTIVNESS: f32 = 0.01;
pub const GLOBAL_LIGHT_DIR: Vec3 = vec3(0.5, 0.5, -0.9);
pub const STATIC_BLOCK_COUNT: i32 = 15;
pub const BLADES_PER_INSTANCE: i32 = 1;
pub const VERTICES_PER_BLADE: i32 = 6;
pub const MAX_HEIGHT: i32 = 3;

#[spirv(vertex)]
pub fn fullscreen_triag_vert(
    #[spirv(vertex_index)] vertex_id: u32,
    #[spirv(position)] clip_pos: &mut Vec4,
) {
    let out_uv = Vec2::new(((vertex_id << 1) & 2) as f32, (vertex_id & 2) as f32);
    *clip_pos = (out_uv * 2.0 - Vec2::splat(1.0)).extend(0.0).extend(1.0);
}

#[repr(C)]
pub struct UniformBufferObject {
    pub trans_w2s: Mat4,
    pub campos: Vec4,
    pub camdir: Vec4,
    pub horizline_scaled: Vec4,
    pub vertiline_scaled: Vec4,
    pub global_light_dir: Vec4,
    pub lightmap_proj: Mat4,
    pub frame_size: Vec2,
    pub time_seed: i32,
}

#[derive(Copy, Clone, Default)]
pub struct Material {
    pub color: Vec3,
    pub emittance: f32,
    pub roughness: f32,
}

pub fn mix_vec3(a: Vec3, b: Vec3, t: Vec3) -> Vec3 {
    vec3(
        a.x * (1.0 - t.x) + b.x * t.x,
        a.y * (1.0 - t.y) + b.y * t.y,
        a.z * (1.0 - t.z) + b.z * t.z,
    )
}

const VERTICES: [Vec3; 36] = [
    vec3(0.0, 1.0, 1.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 0.0, 0.0),
    vec3(0.0, 0.0, 0.0),
    vec3(0.0, 0.0, 1.0),
    vec3(0.0, 1.0, 1.0),
    vec3(1.0, 0.0, 0.0),
    vec3(1.0, 1.0, 0.0),
    vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0),
    vec3(1.0, 0.0, 1.0),
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 0.0, 0.0),
    vec3(1.0, 0.0, 0.0),
    vec3(1.0, 0.0, 1.0),
    vec3(1.0, 0.0, 1.0),
    vec3(0.0, 0.0, 1.0),
    vec3(0.0, 0.0, 0.0),
    vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 0.0),
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 0.0, 0.0),
    vec3(0.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(1.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0),
    vec3(1.0, 0.0, 1.0),
    vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0),
    vec3(0.0, 1.0, 1.0),
    vec3(0.0, 0.0, 1.0),
];

fn read_probe(probe_ipos: IVec3, radiance_cache: &Image!(3D, type=f32, sampled=false)) -> Vec3 {
    let probe_ipos_clamped = probe_ipos.clamp(IVec3::ZERO, WORLD_SIZE);
    let light = radiance_cache.read(probe_ipos_clamped).xyz();
    light.clamp(Vec3::ZERO, Vec3::splat(2.0))
}

fn sample_radiance_with_normal(
    position: Vec3,
    normal: Vec3,
    radiance_cache: &Image!(3D, type=f32, sampled=false),
) -> Vec3 {
    let zero_probe_ipos = (position - Vec3::splat(8.0)).as_ivec3() / IVec3::splat(16);
    let zero_probe_ipos = zero_probe_ipos.clamp(IVec3::ZERO, WORLD_SIZE);
    let zero_probe_pos = zero_probe_ipos.as_vec3() * 16.0 + Vec3::splat(8.0);

    let alpha =
        ((position - zero_probe_pos) / Vec3::splat(16.0)).clamp(Vec3::ZERO, Vec3::splat(1.0));

    let mut total_weight = 0.0;
    let mut total_colour = Vec3::ZERO;

    for i in 0..8 {
        let offset = IVec3::new((i & 1), ((i >> 1) & 1), ((i >> 2) & 1));

        let probe_pos = zero_probe_pos + offset.as_vec3() * 16.0;
        let probe_to_point = probe_pos - position;
        let direction_to_probe = probe_to_point.normalize();

        let trilinear = vec3(
            1.0.lerp(alpha.x, offset.x as f32),
            1.0.lerp(alpha.y, offset.y as f32),
            1.0.lerp(alpha.z, offset.z as f32),
        );
        let mut probe_weight = trilinear.x * trilinear.y * trilinear.z;

        let direction_weight = (direction_to_probe.dot(normal)).clamp(0.1, 1.0);
        probe_weight *= direction_weight;

        let probe_colour = read_probe(zero_probe_ipos + offset, radiance_cache);

        probe_weight = probe_weight.max(1e-7);
        total_weight += probe_weight;
        total_colour += probe_weight * probe_colour;
    }

    total_colour / total_weight
}

// fn sample_radiance_no_normal(
//     position: Vec3,
//     radiance_cache: &Image!(3D, type=f32, sampled),
// ) -> Vec3 {
//     let block_pos = position / Vec3::splat(16.0);
//     let uv = block_pos / WORLD_SIZE.as_vec3();
//     radiance_cache.sample_by_lod(* uv, 0.0).xyz()
// }

fn load_norm(mat_norm: &Image!(subpass, type=u32, sampled=false)) -> Vec3 {
    let rgba = mat_norm.read_subpass(IVec2::ZERO);
    let gba = rgba.yzw().as_vec3();
    (gba / 255.0) * 2.0 - Vec3::splat(1.0)
}

fn load_mat(mat_norm: &Image!(subpass, type=u32, sampled=false)) -> i32 {
    mat_norm.read_subpass(IVec2::ZERO).x as i32
}

fn load_depth(depth_buffer: &Image!(subpass, type=f32, sampled=false)) -> f32 {
    depth_buffer.read_subpass(IVec2::ZERO).x * 1000.0
}

fn get_mat(voxel: i32, voxel_palette: &Image!(2D, type=f32, sampled=false)) -> Material {
    let mut mat = Material {
        color: Vec3::ZERO,
        emittance: 0.0,
        roughness: 0.0,
    };

    mat.color.x = voxel_palette.read(IVec2::new(0, voxel)).x;
    mat.color.y = voxel_palette.read(IVec2::new(1, voxel)).x;
    mat.color.z = voxel_palette.read(IVec2::new(2, voxel)).x;
    mat.emittance = voxel_palette.read(IVec2::new(4, voxel)).x;
    mat.roughness = voxel_palette.read(IVec2::new(5, voxel)).x;

    mat
}

fn get_origin_from_depth(depth: f32, clip_pos: Vec2, ubo: &UniformBufferObject) -> Vec3 {
    ubo.campos.xyz()
        + (ubo.horizline_scaled.xyz() * clip_pos.x)
        + (ubo.vertiline_scaled.xyz() * clip_pos.y)
        + (ubo.camdir.xyz() * depth)
}

fn sample_lightmap_with_shift(
    pos: Vec2,
    xx: i32,
    yy: i32,
    lightmap: &SampledImage<Image!(2D, type=f32, sampled)>,
    test_depth: f32,
) -> f32 {
    let pcf_shift = Vec2::splat(1.0 / 1024.0);
    let lightmap_shift = IVec2::new(xx, yy).as_vec2() * pcf_shift;
    let final_pos = pos + lightmap_shift;

    let depth = lightmap.sample_by_lod(final_pos, 0.0).x;

    (depth > test_depth) as i32 as f32
}

fn sample_lightmap(
    world_pos: Vec3,
    normal: Vec3,
    ubo: &UniformBufferObject,
    lightmap: &SampledImage<Image!(2D, type=f32, sampled)>,
) -> f32 {
    let mut biased_pos = world_pos;

    if normal.dot(ubo.global_light_dir.xyz()) > 0.0 {
        biased_pos -= normal * 0.9;
    } else {
        biased_pos += normal * 0.9;
    }

    let mut light_clip = (ubo.lightmap_proj * biased_pos.extend(1.0)).xyz();
    light_clip.z = 1.0 + light_clip.z;
    let world_depth = light_clip.z;

    let light_uv = (light_clip.xy() + Vec2::splat(1.0)) / 2.0;

    let mut total_light = 0.0;

    total_light += sample_lightmap_with_shift(light_uv, -1, 0, lightmap, world_depth);
    total_light += sample_lightmap_with_shift(light_uv, 0, 0, lightmap, world_depth);
    total_light += sample_lightmap_with_shift(light_uv, 1, 0, lightmap, world_depth);
    total_light += sample_lightmap_with_shift(light_uv, 0, -1, lightmap, world_depth);
    total_light += sample_lightmap_with_shift(light_uv, 0, 1, lightmap, world_depth);

    (total_light / 5.0) * 0.15
}

const COLOR_ENCODE_VALUE: f32 = 8.0;

fn decode_color(encoded_color: Vec3) -> Vec3 {
    encoded_color * COLOR_ENCODE_VALUE
}

fn encode_color(color: Vec3) -> Vec3 {
    color / COLOR_ENCODE_VALUE
}

mod diffuse;
mod fill_stencil_glossy;
mod fill_stencil_smoke;

fn get_block(blocks: &Image!(3D, type=i32, sampled=false), block_pos: IVec3) -> i32 {
    blocks.read(block_pos).x
}

fn voxel_in_palette(relative_voxel_pos: IVec3, block_id: i32) -> IVec3 {
    let block_x = block_id % BLOCK_PALETTE_SIZE_X;
    let block_y = block_id / BLOCK_PALETTE_SIZE_X;
    relative_voxel_pos + IVec3::new(16 * block_x, 16 * block_y, 0)
}

fn voxel_in_bit_palette(relative_voxel_pos: IVec3, block_id: i32) -> IVec3 {
    let block_x = block_id % BLOCK_PALETTE_SIZE_X;
    let block_y = block_id / BLOCK_PALETTE_SIZE_X;
    relative_voxel_pos + IVec3::new(0 + 2 * block_x, 0 + 16 * block_y, 0)
}

fn get_voxel(
    pos: Vec3,
    blocks: &Image!(3D, type=i32, sampled=false),
    block_palette: &Image!(3D, type=u32, sampled=false),
) -> IVec2 {
    let ipos = pos.as_ivec3();
    let iblock_pos = ipos / 16;
    let relative_voxel_pos = ipos % 16;
    let block_id = get_block(blocks, iblock_pos);
    let voxel_pos = voxel_in_palette(relative_voxel_pos, block_id);
    let voxel = block_palette.read(voxel_pos).x as i32;
    IVec2::new(voxel, block_id)
}

fn sample_radiance_simple(
    position: Vec3,
    radiance_cache: &SampledImage<Image!(3D, type=f32, sampled)>,
) -> Vec3 {
    let block_pos = position / 16.0;
    radiance_cache
        .sample_by_lod((block_pos + Vec3::splat(0.5)) / WORLD_SIZE.as_vec3(), 0.0)
        .xyz()
}

fn init_t_vals(ray_origin: Vec3, ray_direction: Vec3) -> (Vec3, Vec3, IVec3) {
    let effective_origin = ray_origin;

    let block_corner1 = (effective_origin.floor() - effective_origin) / ray_direction;
    let block_corner2 =
        (effective_origin.floor() - effective_origin) / ray_direction + 1.0 / ray_direction;

    let t_max_x = block_corner1.x.max(block_corner2.x);
    let t_max_y = block_corner1.y.max(block_corner2.y);
    let t_max_z = block_corner1.z.max(block_corner2.z);
    let t_max = vec3(t_max_x, t_max_y, t_max_z);

    let t_delta = 1.0 / ray_direction.abs();
    let block_pos = effective_origin.as_ivec3();

    (t_max, t_delta, block_pos)
}

fn bool_as_float(b: bool) -> f32 {
    if b {
        1.0
    } else {
        0.0
    }
}

fn bool_as_int(b: bool) -> i32 {
    if b {
        1
    } else {
        0
    }
}

fn bvec3_as_vec3(v: BVec3) -> Vec3 {
    vec3(bool_as_float(v.x), bool_as_float(v.y), bool_as_float(v.z))
}

fn bvec3_as_ivec3(v: BVec3) -> IVec3 {
    ivec3(bool_as_int(v.x), bool_as_int(v.y), bool_as_int(v.z))
}

fn cast_ray_fast_full(
    origin: Vec3,
    direction: Vec3,
    blocks: &Image!(3D, type=i32, sampled=false),
    block_palette: &Image!(3D, type=u32, sampled=false),
    voxel_palette: &Image!(2D, type=f32, sampled=false),
    world_size: IVec3,
) -> (bool, f32, Vec3, Material) {
    let mut fraction = 0.0;
    let max_dist = 16.0 * 8.0;

    let one_div_dir = 1.0 / direction;
    let b_precomputed_corner = direction.cmpgt(Vec3::ZERO);
    let precomputed_corner = bvec3_as_vec3(b_precomputed_corner) * 16.0;

    let mut current_voxel = 0;
    let mut pos = Vec3::ZERO;

    loop {
        fraction += 0.5;
        pos = origin + direction * fraction;

        let current_voxel_info = get_voxel(pos, blocks, block_palette);
        current_voxel = current_voxel_info.x;
        let current_block = current_voxel_info.y;

        if current_block == 0 {
            let box_precomp = (pos.as_ivec3() / 16).as_vec3() * 16.0 + precomputed_corner;
            let temp = -pos * one_div_dir;
            let block_corner = box_precomp * one_div_dir + temp;
            let f = block_corner.x.min(block_corner.y).min(block_corner.z).max(0.01);
            fraction += f;
            pos = origin + direction * fraction;
            current_voxel = get_voxel(pos, blocks, block_palette).x;
        }

        if current_voxel != 0 {
            break;
        }

        if pos.cmplt(Vec3::ZERO).any() || pos.cmpge(world_size.as_vec3() * 16.0).any() {
            return (false, fraction, Vec3::ZERO, Material::default());
        }
        if fraction >= max_dist {
            return (false, fraction, Vec3::ZERO, Material::default());
        }
    }

    let before_hit = origin + direction * (fraction - 1.5);

    let steps = bvec3_as_ivec3(direction.cmpgt(Vec3::ZERO)) * 2 - IVec3::splat(1);

    let (mut t_max, mut t_delta, mut voxel_pos) = init_t_vals(before_hit, direction);

    voxel_pos = before_hit.as_ivec3();

    let mut current_voxel_id = get_voxel(voxel_pos.as_vec3(), blocks, block_palette).x;
    let mut iterations = 0;
    let mut f_current_step_direction = Vec3::ZERO;

    while iterations <= 4 && current_voxel_id == 0 {
        f_current_step_direction = Vec3::ZERO;
        if t_max.x <= t_max.y && t_max.x <= t_max.z {
            f_current_step_direction.x = 1.0;
        } else if t_max.y <= t_max.z {
            f_current_step_direction.y = 1.0;
        } else {
            f_current_step_direction.z = 1.0;
        }

        voxel_pos += steps * f_current_step_direction.as_ivec3();
        t_max += t_delta * f_current_step_direction;

        current_voxel_id = get_voxel(voxel_pos.as_vec3(), blocks, block_palette).x;
        iterations += 1;
    }

    let normal = -(steps.as_vec3() * f_current_step_direction);
    let t_final = t_max - t_delta;
    fraction = t_final.dot(f_current_step_direction);

    let material = get_mat(current_voxel_id, voxel_palette);

    return (true, fraction, normal, material);
}

fn cast_ray_precise(
    ray_origin: Vec3,
    ray_direction: Vec3,
    blocks: &Image!(3D, type=i32, sampled=false),
    block_palette: &Image!(3D, type=u32, sampled=false),
    voxel_palette: &Image!(2D, type=f32, sampled=false),
    world_size: IVec3,
) -> Option<(f32, Vec3, Material)> {
    let steps = bvec3_as_ivec3(ray_direction.cmpgt(Vec3::ZERO)) * 2 - IVec3::splat(1);
    let f_steps = steps.as_vec3();

    let (mut t_max, mut t_delta, mut voxel_pos) = init_t_vals(ray_origin, ray_direction);

    let max_steps = 128;
    let mut current_voxel = get_voxel(voxel_pos.as_vec3(), blocks, block_palette).x;

    let mut iterations = 0;
    let mut current_step_direction_b = BVec3::new(false, false, false);

    loop {
        let x_ly = t_max.x <= t_max.y;
        let z_lx = t_max.z <= t_max.x;
        let y_lz = t_max.y <= t_max.z;

        current_step_direction_b.x = x_ly && !z_lx;
        current_step_direction_b.y = y_lz && !x_ly;
        current_step_direction_b.z = z_lx && !y_lz;

        let current_step_direction_f = bvec3_as_vec3(current_step_direction_b);

        t_max += t_delta * current_step_direction_f;
        voxel_pos += steps * current_step_direction_f.as_ivec3();

        current_voxel = get_voxel(voxel_pos.as_vec3(), blocks, block_palette).x;

        if current_voxel != 0 {
            let normal = -(f_steps * current_step_direction_f);
            let t_final = t_max - t_delta;
            let fraction = t_final.dot(current_step_direction_f);
            let material = get_mat(current_voxel, voxel_palette);
            return Some((fraction, normal, material));
        }
        if voxel_pos.cmplt(IVec3::ZERO).any() || voxel_pos.cmpge(world_size * 16).any() {
            return None;
        }
        if iterations >= max_steps {
            return None;
        }
        iterations += 1;
    }
}

fn process_hit_simple(
    origin: &mut Vec3,
    direction: &mut Vec3,
    fraction: f32,
    normal: Vec3,
    material: Material,
    accumulated_light: &mut Vec3,
    accumulated_reflection: &mut Vec3,
) {
    *origin += fraction * *direction;

    let diffuse_light = Vec3::ZERO;

    *accumulated_reflection *= material.color;
    *accumulated_light += *accumulated_reflection * (material.emittance + diffuse_light);

    *direction = direction.reflect(normal);
}

fn process_hit(
    origin: &mut Vec3,
    direction: &mut Vec3,
    fraction: f32,
    normal: Vec3,
    material: Material,
    accumulated_light: &mut Vec3,
    accumulated_reflection: &mut Vec3,
    radiance_cache: &SampledImage<Image!(3D, type=f32, sampled)>,
) {
    *origin += fraction * *direction;

    let diffuse_light = sample_radiance_simple(*origin, radiance_cache);

    *accumulated_reflection *= material.color;
    *accumulated_light += *accumulated_reflection * (material.emittance + diffuse_light);

    *direction = direction.reflect(normal);
}

fn trace_glossy_ray(
    ray_origin: Vec3,
    ray_direction: Vec3,
    accumulated_light_in: Vec3,
    accumulated_reflection_in: Vec3,
    blocks: &Image!(3D, type=i32, sampled=false),
    block_palette: &Image!(3D, type=u32, sampled=false),
    voxel_palette: &Image!(2D, type=f32, sampled=false),
    radiance_cache: &SampledImage<Image!(3D, type=f32, sampled)>,
    ubo: &UniformBufferObject,
    world_size: IVec3,
) -> Vec3 {
    let mut origin = ray_origin;
    let mut direction = ray_direction;
    let mut light = accumulated_light_in;
    let mut reflection = accumulated_reflection_in;

    let (hit, fraction, normal, material) = cast_ray_fast_full(
        origin,
        direction,
        blocks,
        block_palette,
        voxel_palette,
        world_size,
    );
    if hit {
        process_hit(
            &mut origin,
            &mut direction,
            fraction,
            normal,
            material,
            &mut light,
            &mut reflection,
            radiance_cache,
        );
    } else {
        let global_light_participance = -direction.dot(ubo.global_light_dir.xyz());
        if global_light_participance > 0.9 {
            light += (Vec3::new(0.9, 0.9, 0.6) * 0.5)
                * reflection
                * (global_light_participance - 0.9)
                * 10.0;
        } else {
            light += (Vec3::new(0.53, 0.81, 0.92) * 0.1) * reflection;
        }
    }
    light
}

fn load_norm_glossy(mat_norm: &Image!(2D, type=u32, sampled=false), pixel: IVec2) -> Vec3 {
    let rgba = mat_norm.read(pixel);
    let gba = vec3(rgba.y as f32, rgba.z as f32, rgba.w as f32);
    (gba / 255.0) * 2.0 - Vec3::splat(1.0)
}

fn load_mat_glossy(mat_norm: &Image!(2D, type=u32, sampled=false), pixel: IVec2) -> i32 {
    mat_norm.read(pixel).x as i32
}

fn load_depth_glossy(depth_buffer: &Image!(2D, type=f32, sampled), pixel: IVec2) -> f32 {
    depth_buffer.fetch(pixel).x * 1000.0
}

mod glossy;

fn rotate2d(a: f32) -> Mat2 {
    let s = a.sin();
    let c = a.cos();
    Mat2::from_cols(Vec2::new(c, s), Vec2::new(-s, c))
}

fn load_depth_sampled(depth_buffer: &SampledImage<Image!(2D, type=f32, sampled)>, uv: Vec2) -> f32 {
    depth_buffer.sample_by_lod(uv, 0.0).x * 1000.0
}

mod hbao;

fn qtransform(q: Vec4, v: Vec3) -> Vec3 {
    // let q = spirv_std::glam::Quat::from_vec4(q_vec4);
    // q * v
    return v + 2.0 * (v.cross(-q.xyz()) + q.w * v).cross(-q.xyz());
}

mod lightmap;

mod map;

fn get_voxel2(
    block_palette: &Image!(3D, type=u32, sampled=false),
    block_id: i32,
    relative_voxel_pos: IVec3,
) -> i32 {
    let voxel_pos = voxel_in_palette(relative_voxel_pos, block_id);
    block_palette.read(voxel_pos).x as i32
}

mod raygen_blocks;

fn get_model_voxel(
    model_voxels: &Image!(3D, type=u32, sampled=false),
    relative_voxel_pos: IVec3,
) -> i32 {
    let voxel_f32 = model_voxels.read(relative_voxel_pos).x;
    voxel_f32 as i32
}

mod raygen_models;

mod raygen_particles;

fn sample_probe2(
    radiance_cache: &Image!(3D, type=f32, sampled = false),
    probe_ipos: IVec3,
    _direction: Vec3,
) -> Vec3 {
    let probe_ipos_clamped = probe_ipos.clamp(IVec3::ZERO, WORLD_SIZE);
    let subprobe_pos = IVec3::new(
        probe_ipos_clamped.x,
        probe_ipos_clamped.y,
        probe_ipos_clamped.z,
    );
    let light_color_alpha = radiance_cache.read(subprobe_pos);
    let r = light_color_alpha.x as f32 / 1023.0;
    let g = light_color_alpha.y as f32 / 1023.0;
    let b = light_color_alpha.z as f32 / 1023.0;
    vec3(r, g, b).clamp(Vec3::splat(0.0), Vec3::splat(2.0))
}

fn sample_radiance_with_normal2(
    radiance_cache: &Image!(3D, type=f32, sampled = false),
    position: Vec3,
    normal: Vec3,
) -> Vec3 {
    let mut total_weight = 0.0;
    let mut total_colour = Vec3::ZERO;

    let zero_probe_ipos = (position - 8.0).floor().as_ivec3() / 16;
    let zero_probe_ipos = zero_probe_ipos.clamp(IVec3::ZERO, WORLD_SIZE);
    let zero_probe_pos = zero_probe_ipos.as_vec3() * 16.0 + 8.0;

    let alpha = ((position - zero_probe_pos) / 16.0).clamp(Vec3::ZERO, Vec3::splat(1.0));

    for i in 0..8 {
        let offset = IVec3::new(i, i >> 1, i >> 2) & IVec3::splat(1);

        let mut probe_weight = 1.0;
        let mut probe_colour;

        let probe_pos = zero_probe_pos + offset.as_vec3() * 16.0;

        let probe_to_point = probe_pos - position;
        let direction_to_probe = probe_to_point.normalize();

        let trilinear = vec3(
            1.0.lerp(alpha.x, offset.x as f32),
            1.0.lerp(alpha.y, offset.y as f32),
            1.0.lerp(alpha.z, offset.z as f32),
        );
        probe_weight *= trilinear.x * trilinear.y * trilinear.z;

        let direction_weight = direction_to_probe.dot(normal).clamp(0.1, 1.0);

        probe_weight *= direction_weight;

        probe_colour = sample_probe2(radiance_cache, zero_probe_ipos + offset, direction_to_probe);

        probe_weight = probe_weight.max(1e-7);
        total_weight += probe_weight;
        total_colour += probe_weight * probe_colour;
    }

    total_colour / total_weight
}

fn sample_radiance_without_normal(
    radiance_cache: &Image!(3D, type=f32, sampled = false),
    position: Vec3,
) -> Vec3 {
    let mut total_weight = 0.0;
    let mut total_colour = Vec3::ZERO;

    let zero_probe_ipos = (position - 8.0).floor().as_ivec3() / 16;
    let zero_probe_ipos = zero_probe_ipos.clamp(IVec3::ZERO, WORLD_SIZE);
    let zero_probe_pos = zero_probe_ipos.as_vec3() * 16.0 + 8.0;

    let alpha = ((position - zero_probe_pos) / 16.0).clamp(Vec3::ZERO, Vec3::splat(1.0));

    for i in 0..8 {
        let offset = IVec3::new(i, i >> 1, i >> 2) & IVec3::splat(1);

        let mut probe_weight = 1.0;
        let probe_colour;

        let probe_pos = zero_probe_pos + offset.as_vec3() * 16.0;

        let probe_to_point = probe_pos - position;
        let _direction_to_probe = probe_to_point.normalize();

        let trilinear = vec3(
            1.0.lerp(alpha.x, offset.x as f32),
            1.0.lerp(alpha.y, offset.y as f32),
            1.0.lerp(alpha.z, offset.z as f32),
        );
        probe_weight *= trilinear.x * trilinear.y * trilinear.z;

        probe_colour = sample_probe2(
            radiance_cache,
            zero_probe_ipos + offset,
            _direction_to_probe,
        );

        total_weight += probe_weight;
        total_colour += probe_weight * probe_colour;
    }

    total_colour / total_weight
}

fn decode_depth(d: f32) -> f32 {
    d * 1000.0
}

fn load_depth_far(smoke_depth_far_subpass: &Image!(subpass, type=f32, sampled=false)) -> f32 {
    decode_depth(smoke_depth_far_subpass.read_subpass(IVec2::ZERO).x)
}

fn load_depth_near(smoke_depth_near_subpass: &Image!(subpass, type=f32, sampled=false)) -> f32 {
    decode_depth(smoke_depth_near_subpass.read_subpass(IVec2::ZERO).x)
}

fn rotate_vec2(v: Vec2, a: f32) -> Vec2 {
    let s = a.sin();
    let c = a.cos();
    let m = Mat2::from_cols(Vec2::new(c, s), Vec2::new(-s, c));
    m * v
}

fn rotatem(a: f32) -> Mat2 {
    let s = a.sin();
    let c = a.cos();
    Mat2::from_cols(Vec2::new(c, s), Vec2::new(-s, c))
}

fn get_origin_from_depth2(ubo: &UniformBufferObject, depth: f32, clip_pos: Vec2) -> Vec3 {
    ubo.campos.xyz()
        + (ubo.horizline_scaled.xyz() * clip_pos.x)
        + (ubo.vertiline_scaled.xyz() * clip_pos.y)
        + (ubo.camdir.xyz() * depth)
}

mod radiance;
mod smoke;
mod tonemap;
mod update_grass;
mod update_water;
mod water;

mod perlin;
