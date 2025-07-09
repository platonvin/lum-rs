#![no_std]
use common::*;

#[repr(C)]
pub struct GrassPushConstants {
    pub shift: Vec4,
    pub size: i32,
    pub time: i32,
    pub x_flip: i32,
    pub y_flip: i32,
}

pub(crate) fn hash21(mut p: UVec2) -> u32 {
    p *= uvec2(73333, 7777);
    p ^= (uvec2(3333777777, 3333777777) >> (p >> 28));

    let n = p.x * p.y;
    n ^ (n >> 15)
}

pub(crate) fn rand(p: Vec2) -> f32 {
    let u_p = unsafe { transmute(p) };
    let h = hash21(u_p);
    h as f32 * (1.0 / 0xffffffffu32 as f32)
}

pub(crate) fn get_blade_width(height: f32) -> f32 {
    let max_height = (MAX_HEIGHT - 1) as f32;
    (max_height - height) / max_height
}

pub(crate) fn rotate_blade_vert(rnd01: f32, vertex: &mut Vec3, normal: &mut Vec3) {
    let angle = rnd01 * PI * 2.0 * 42.0;
    let cos_rot = angle.cos();
    let sin_rot = angle.sin();

    let vx_new = vertex.x * cos_rot + vertex.y * sin_rot;
    let vy_new = -vertex.x * sin_rot + vertex.y * cos_rot;
    vertex.x = vx_new;
    vertex.y = vy_new;

    let nx_new = normal.x * cos_rot + normal.y * sin_rot;
    let ny_new = -normal.x * sin_rot + normal.y * cos_rot;
    normal.x = nx_new;
    normal.y = ny_new;
}

pub(crate) fn displace_blade(rnd01: f32, vertex: &mut Vec3, _normal: &mut Vec3) {
    let shift = Vec2::new((rnd01 * 42.1424).sin(), (rnd01 * 58.1424).sin());
    vertex.x += shift.x;
    vertex.y += shift.y;
}

pub(crate) fn scale_blade_vert(rnd01: f32, vertex: &mut Vec3, _normal: &mut Vec3) {
    let scale = 0.5 + rnd01 * 0.5;
    *vertex *= scale;
}

pub(crate) fn curve_blade_vert(rnd01: f32, vertex: &mut Vec3, _normal: &mut Vec3) {
    vertex.y = (vertex.z / MAX_HEIGHT as f32) * 1.5;
}

pub(crate) fn load_offset(
    local_pos: Vec2,
    state_texture: &SampledImage<Image!(2D, type=f32, sampled)>,
    world_size: IVec3,
    pco_shift: Vec4,
) -> Vec2 {
    let world_pos = local_pos * 16.0 + pco_shift.xy();
    let state_pos = world_pos / (world_size.xy().as_vec2() * 16.0);
    state_texture.sample_by_lod(state_pos, 0.0).xy()
}

pub(crate) fn wiggle_blade_vert(
    rnd01: f32,
    vertex: &mut Vec3,
    _normal: &mut Vec3,
    pos: Vec2,
    time: i32,
    state_texture: &SampledImage<Image!(2D, type=f32, sampled)>,
    world_size: IVec3,
    pco_shift: Vec4,
) {
    let global_offset = load_offset(pos, state_texture, world_size, pco_shift);

    let mut local_offset = Vec2::ZERO;
    let mut freq = 1.0;
    while freq < 4.0 {
        let ampl = 0.05;
        let t = time as f32 / 200.0;
        local_offset += (t * freq + rnd01 * 400.0 * freq).sin() * ampl;
        freq += 1.2;
    }

    let offset = local_offset + global_offset;
    vertex.x += offset.x * vertex.z * 1.0;
    vertex.y += offset.y * vertex.z * 1.0;
}

pub(crate) fn get_blade_vert_data(
    iindex: i32,
    rnd01: f32,
    pos: Vec2,
    time: i32,
    state_texture: &SampledImage<Image!(2D, type=f32, sampled)>,
    world_size: IVec3,
    pco_shift: Vec4,
    out_normal: &mut Vec3,
) -> Vec3 {
    let mut vertex;

    let z_height = (iindex / 2) as f32;
    let mut x_pos = (iindex % 2) as f32;
    if iindex == (VERTICES_PER_BLADE - 1) {
        x_pos = 0.5;
    }

    vertex = vec3(x_pos, 0.0, z_height);

    let width = get_blade_width(vertex.z);
    let width_diff = 1.0 - width;
    vertex.x = width * vertex.x + width_diff / 2.0;

    let n1 = vec3(-0.5, 1.0, 0.0);
    let n2 = vec3(0.5, 1.0, 0.0);
    *out_normal = (n1.lerp(n2, x_pos)).normalize();

    vertex.x *= 3.7;
    vertex.z *= 6.0 / 3.0;

    curve_blade_vert(rnd01, &mut vertex, out_normal);
    rotate_blade_vert(rnd01, &mut vertex, out_normal);
    wiggle_blade_vert(
        rnd01,
        &mut vertex,
        out_normal,
        pos,
        time,
        state_texture,
        world_size,
        pco_shift,
    );
    displace_blade(rnd01, &mut vertex, out_normal);

    vertex.z *= 1.5 + (rnd01 * 1.5) * (rnd01 * 1.5);

    vertex
}

#[spirv(vertex)]
#[unsafe(no_mangle)]
pub fn main(
    #[spirv(vertex_index)] vertex_id: u32,
    #[spirv(instance_index)] instance_id: u32,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(descriptor_set = 0, binding = 1)] state_texture: &SampledImage<
        Image!(2D, type=f32, sampled),
    >,
    #[spirv(push_constant)] pco: &GrassPushConstants,
    mat_norm: &mut UVec4,
    #[spirv(position)] out_clip_pos: &mut Vec4,
) {
    let sub_blade_id = vertex_id as i32 / VERTICES_PER_BLADE;
    let mut blade_id = instance_id as i32 * BLADES_PER_INSTANCE + sub_blade_id;
    let blade_vertex_id = vertex_id as i32 % VERTICES_PER_BLADE;
    let mut blade_x = blade_id % pco.size;
    let mut blade_y = blade_id / pco.size;

    //for faster depth testing
    if pco.x_flip == 0 {
        blade_x = pco.size - blade_x;
    }
    if pco.y_flip != 0 {
        blade_y = pco.size - blade_y;
    }

    let relative_pos = (Vec2::new(blade_x as f32, blade_y as f32) + 0.5) / pco.size as f32;

    let mut normal = Vec3::ZERO;
    let rand01 = rand(relative_pos + pco.shift.xy());
    let rel2world = get_blade_vert_data(
        blade_vertex_id,
        rand01,
        relative_pos,
        pco.time,
        state_texture,
        WORLD_SIZE,
        pco.shift,
        &mut normal,
    );

    let rel2tile_shift = relative_pos * 16.0;
    let rel2tile = rel2world + vec3(rel2tile_shift.x, rel2tile_shift.y, 0.0);

    let world_pos = (rel2tile.extend(1.0)) + pco.shift;

    let mut clip_coords = (ubo.trans_w2s * world_pos).xyz();
    clip_coords.z = 1.0 + clip_coords.z;

    *out_clip_pos = clip_coords.extend(1.0);

    if ubo.camdir.xyz().dot(normal) > 0.0 {
        normal = -normal;
    }

    let mat_id =
        if rand01 > (rand(pco.shift.yx()) - (relative_pos - Vec2::splat(8.0)).length() / 32.0) {
            9u32
        } else {
            10u32
        };

    let fmat = (mat_id as f32 - 127.0) / 127.0;
    let fmat_norm_vec4 = vec4(fmat, normal.x, normal.y, normal.z);
    *mat_norm = (((fmat_norm_vec4 + 1.0) / 2.0) * 255.0).as_uvec4();
}
