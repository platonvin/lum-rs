#![no_std]
use common::*;

#[repr(C)]
pub struct RadianceConstants {
    pub time_seed: i32,
    pub iters: i32,
}

#[repr(C)]
pub struct ReqBuffer {
    pub update_requests: [IVec4; 1],
}

const K_HASH: u32 = 1103515245;

fn hash33(mut x: UVec3) -> Vec3 {
    x = ((x >> 8) ^ uvec3(x.y, x.z, x.x)) * K_HASH;
    x = ((x >> 8) ^ uvec3(x.y, x.z, x.x)) * K_HASH;
    x = ((x >> 8) ^ uvec3(x.y, x.z, x.x)) * K_HASH;

    vec3(x.x as f32, x.y as f32, x.z as f32) * (1.0 / 0xffffffff_u32 as f32)
}

fn random3d(random_storage: &mut UVec3) -> Vec3 {
    let res = hash33(*random_storage);
    random_storage.x += 1;
    res
}

fn random_sphere_point(rand: Vec3) -> Vec3 {
    let ang1 = (rand.x + 1.0) * PI;
    let u = rand.y;
    let u2 = u * u;
    let sqrt1_minus_u2 = (1.0 - u2).sqrt();
    let x = sqrt1_minus_u2 * ang1.cos();
    let y = sqrt1_minus_u2 * ang1.sin();
    let z = u;
    vec3(x, y, z)
}

fn normal_oriented_hemisphere_point(rand: Vec3, n: Vec3) -> Vec3 {
    let v = random_sphere_point(rand);
    v * v.dot(n).signum()
}

fn random_cosine_weighted_hemisphere_point(rand: Vec3, n: Vec3) -> Vec3 {
    let r = rand.x * 0.5 + 0.5;
    let angle = PI * rand.y + PI;
    let sr = r.sqrt();
    let p = vec2(sr * angle.cos(), sr * angle.sin());
    let ph_z = (1.0 - p.x * p.x - p.y * p.y).sqrt();
    let ph = vec3(p.x, p.y, ph_z);

    let tangent = rand.normalize();
    let bitangent = tangent.cross(n);
    let tangent = bitangent.cross(n);

    tangent * ph.x + bitangent * ph.y + n * ph.z
}

fn get_voxel3(
    pos: Vec3,
    blocks: &Image!(3D, type = i32, sampled),
    block_palette: &Image!(3D, type = u32, sampled),
) -> IVec2 {
    let ipos = pos.as_ivec3();
    let iblock_pos = ipos / 16;
    let relative_voxel_pos = ipos % 16;

    let block_id = blocks.fetch(iblock_pos).x;

    let voxel_pos = voxel_in_palette(relative_voxel_pos, block_id);
    let voxel = block_palette.fetch(voxel_pos).x as i32;

    ivec2(voxel, block_id)
}

fn cast_ray_fast2(
    blocks: &Image!(3D, type = i32, sampled),
    block_palette: &Image!(3D, type = u32, sampled),
    voxel_palette: &Image!(2D, type = f32, sampled = false),
    origin: Vec3,
    direction: Vec3,
    fraction: &mut f32,
    normal: &mut Vec3,
    material: &mut Material,
    left_bounds: &mut bool,
) -> bool {
    let mut block_hit = false;
    let max_dist = 16.0 * 8.0;

    let one_div_dir = Vec3::splat(1.0) / direction;
    let bprecomputed_corner = direction.cmpgt(Vec3::ZERO);
    let precomputed_corner = bvec3_as_vec3(bprecomputed_corner) * 16.0;

    *fraction = 0.0;
    let mut current_voxel = 0;
    let mut current_block = 0;

    loop {
        *fraction += 0.5;

        let pos = origin + direction * *fraction;

        let voxel_info = get_voxel3(pos, blocks, block_palette);
        current_voxel = voxel_info.x;
        current_block = voxel_info.y;

        if current_block == 0 {
            let box_precomp = (pos.as_ivec3() / 16 * 16).as_vec3() + precomputed_corner;
            let temp = -pos * one_div_dir;
            let block_corner = box_precomp * one_div_dir + temp;

            let f_val = block_corner.x.min(block_corner.y.min(block_corner.z));
            let f_safe = f_val.max(0.01);
            *fraction += f_safe;
        }

        if current_voxel != 0 {
            block_hit = true;
            break;
        }

        if pos.cmplt(Vec3::ZERO).any() || pos.cmpge(WORLD_SIZE.as_vec3() * 16.0).any() {
            block_hit = false;
            *left_bounds = true;
            break;
        }

        if *fraction >= max_dist {
            block_hit = false;
            *left_bounds = true;
            break;
        }
    }

    let before_hit = origin + direction * (*fraction - 1.5);

    if current_voxel != 0 {
        let steps = bvec3_as_ivec3(direction.cmpgt(Vec3::ZERO)) * 2 - IVec3::splat(1);

        let mut t_max_local;
        let mut t_delta_local;
        let mut voxel_pos_local;

        let block_corner1 = (before_hit.floor() - before_hit) / direction;
        let block_corner2 =
            (before_hit.floor() - before_hit) / direction + Vec3::splat(1.0) / direction;
        t_max_local = vec3(
            block_corner1.x.max(block_corner2.x),
            block_corner1.y.max(block_corner2.y),
            block_corner1.z.max(block_corner2.z),
        );

        t_delta_local = Vec3::splat(1.0) / direction.abs();
        voxel_pos_local = before_hit.as_ivec3();

        let mut fcurrent_step_direction = Vec3::ZERO;
        let mut current_voxel_id = get_voxel3(voxel_pos_local.as_vec3(), blocks, block_palette).x;
        let mut iterations = 0;

        while iterations <= 4 && current_voxel_id == 0 {
            iterations += 1;
            fcurrent_step_direction.x =
                (t_max_local.x <= t_max_local.y && t_max_local.x <= t_max_local.z) as u32 as f32;
            fcurrent_step_direction.y =
                (t_max_local.x >= t_max_local.y && t_max_local.y <= t_max_local.z) as u32 as f32;
            fcurrent_step_direction.z =
                (t_max_local.z <= t_max_local.x && t_max_local.z <= t_max_local.y) as u32 as f32;

            voxel_pos_local += steps * fcurrent_step_direction.as_ivec3();
            t_max_local += t_delta_local * fcurrent_step_direction;

            current_voxel_id = get_voxel3(voxel_pos_local.as_vec3(), blocks, block_palette).x;
        }

        *normal = -(steps.as_vec3() * fcurrent_step_direction);
        let t_final = t_max_local - t_delta_local;
        *fraction = t_final.dot(fcurrent_step_direction);

        *material = get_mat(current_voxel_id, voxel_palette);
    }

    current_voxel != 0
}

fn spherical_fibonacci(i: f32, n: f32) -> Vec3 {
    let phi_val = i * (f32::sqrt(5.0) * 0.5 + 0.5 - 1.0);
    let phi = 2.0 * PI * (phi_val - phi_val.floor());

    let cos_theta = 1.0 - (2.0 * i + 1.0) * (1.0 / n);
    let sin_theta = (1.0 - cos_theta * cos_theta).clamp(0.0, 1.0).sqrt();

    vec3(phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta)
}

fn probe_id_to_ray_dir(probe_id: i32) -> Vec3 {
    spherical_fibonacci(probe_id as f32, RAYS_PER_PROBE as f32)
}

fn trace_ray(
    blocks: &Image!(3D, type = i32, sampled),
    block_palette: &Image!(3D, type = u32, sampled),
    voxel_palette: &Image!(2D, type = f32, sampled=false),
    radiance_cache2read: &Image!(3D, type=f32, sampled = false),
    ray_pos: Vec3,
    ray_dir: Vec3,
) -> Vec3 {
    let mut fraction = 0.0;
    let mut normal = Vec3::ZERO;

    let origin = ray_pos;
    let direction = ray_dir;

    let mut light = Vec3::ZERO;
    let reflection = Vec3::splat(1.0);
    let mut material = Material {
        color: Vec3::ZERO,
        emittance: 0.0,
        roughness: 0.0,
    };
    let mut left_bounds = false;

    let hit = cast_ray_fast2(
        blocks,
        block_palette,
        voxel_palette,
        origin,
        direction,
        &mut fraction,
        &mut normal,
        &mut material,
        &mut left_bounds,
    );

    if hit {
        let propagated_light = sample_radiance_with_normal2(
            radiance_cache2read,
            origin + fraction * direction,
            normal,
        );
        light +=
            reflection * propagated_light * material.color + material.color * material.emittance;
    } else {
        let global_light_participance = -direction.dot(GLOBAL_LIGHT_DIR);
        if global_light_participance > 0.95 {
            light += reflection * global_light_participance * (vec3(0.9, 0.9, 0.6) * 3.0);
        } else {
            light += reflection * (vec3(0.53, 0.81, 0.92) * 0.1);
        }
    }

    light
}

fn store_probe(
    radiance_cache2read: &Image!(3D, type = f32, sampled = false),
    radiance_cache2write: &Image!(3D, format = rgb10_a2, sampled = false),
    probe_pos: IVec3,
    accumulated_light: Vec3,
) {
    let old_light = radiance_cache2read.read(probe_pos).xyz();
    let old_light = old_light.clamp(Vec3::ZERO, Vec3::splat(1.0));
    let mut new_light = old_light * (1.0 - REACTIVNESS) + accumulated_light * REACTIVNESS;
    unsafe {
        radiance_cache2write.write(probe_pos, vec4(new_light.x, new_light.y, new_light.z, 1.0))
    };
}

#[spirv(compute(threads(64, 1, 1)))]
#[unsafe(no_mangle)]
pub fn main(
    #[spirv(global_invocation_id)] global_id: UVec3,
    #[spirv(subgroup_id)] subgroup_id: u32,
    #[spirv(subgroup_size)] subgroup_size: u32,
    #[spirv(descriptor_set = 0, binding = 0)] blocks: &Image!(3D, type = i32, sampled),
    #[spirv(descriptor_set = 0, binding = 1)] block_palette: &Image!(3D, type = u32, sampled),
    #[spirv(descriptor_set = 0, binding = 2)] voxel_palette: &Image!(2D, type = f32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 3)] radiance_cache2read: &Image!(
         3D,
         type=f32,
         sampled = false
     ),
    #[spirv(descriptor_set = 0, binding = 4)] radiance_cache2write: &Image!(
        3D,
        format = rgb10_a2,
        sampled = false
    ),
    #[spirv(storage_buffer, descriptor_set = 0, binding = 5)] req_buffer: &[I8Vec4],
    #[spirv(push_constant)] pco: &RadianceConstants,
) {
    let local_pos = global_id.as_ivec3();

    let probe_id = local_pos.x % RAYS_PER_PROBE as i32;

    let req_num = local_pos.x / RAYS_PER_PROBE as i32;
    let probe_pos = req_buffer[req_num as usize].xyz();

    let mut random_storage = uvec3(local_pos.x as u32, local_pos.y as u32, pco.time_seed as u32);

    let block_corner = probe_pos.as_vec3() * 16.0;
    let block_center = block_corner + Vec3::splat(8.0);
    let mut ray_pos = block_center;
    let mut ray_dir = probe_id_to_ray_dir(probe_id);

    ray_dir =
        (ray_dir + 0.1 * ((random3d(&mut random_storage) - Vec3::splat(0.5)) * 2.0)).normalize();
    ray_pos += ray_dir * 6.0;
    ray_pos += (random3d(&mut random_storage) - Vec3::splat(0.5)) * 2.0;
    ray_dir = if (probe_id % 2) == 0 {
        ray_dir
    } else {
        -ray_dir
    };

    let ray_light = trace_ray(
        blocks,
        block_palette,
        voxel_palette,
        radiance_cache2read,
        ray_pos,
        ray_dir,
    );

    let ray_light_sum = unsafe { spirv_std::arch::subgroup_f_add(ray_light) };
    let ray_light_avg = ray_light_sum / subgroup_size as f32;

    // let mut _light_shared: [Vec3; (RAYS_PER_PROBE / 32) as usize] =
    //     [Vec3::ZERO; (RAYS_PER_PROBE / 32) as usize];
    // TODO: bro why cant i fucking write to an array what the hell
    let mut _light_shared_0 = Vec3::ZERO;
    let mut _light_shared_1 = Vec3::ZERO;

    unsafe {
        if subgroup_id == 0 {
            _light_shared_0 = ray_light_avg;
        } else {
            _light_shared_1 = ray_light_avg;
        }

        // let final_light = ray_light_avg;
        let mut final_light = Vec3::ZERO;
        // let subgroups_per_workgroup = RAYS_PER_PROBE / subgroup_size;
        // for i in 0..subgroups_per_workgroup {
        //     final_light += _light_shared[i as usize];
        // }

        // r/lies: i love this
        final_light += _light_shared_0;
        final_light += _light_shared_1;

        final_light /= 2.0;

        if subgroup_id == 0 {
            if spirv_std::arch::subgroup_elect() {
                store_probe(
                    radiance_cache2read,
                    radiance_cache2write,
                    probe_pos.as_ivec3(),
                    final_light,
                );
            }
        }
    }
}
