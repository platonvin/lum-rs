#![no_std]
use common::*;

#[repr(C)]
pub struct WaterPushConstants {
    pub shift: Vec4,
    pub size: i32,
    pub time: i32,
}

pub const LODS: i32 = 6;

pub(crate) fn rand_float(co: Vec2) -> f32 {
    (co.dot(Vec2::new(12.9898, 78.233))).sin().fract() * 43758.5453
}

pub(crate) fn get_height(
    state_texture: &SampledImage<Image!(2D, type=f32, sampled)>,
    globalpos: Vec2,
    pco_time: i32,
    _pco_size: i32,
) -> f32 {
    let _time_float = pco_time as f32;
    let _direction = Vec2::new((_time_float / 100.0).sin(), (_time_float / 100.0).cos());

    let mut total_height = 0.0;

    total_height += state_texture.sample_by_lod(globalpos / 13.0, 0.0).x * (13.0 / 55.0);
    total_height += state_texture.sample_by_lod(globalpos / 31.0, 0.0).y * (31.0 / 55.0);
    total_height += state_texture.sample_by_lod(globalpos / 35.0, 0.0).z * (35.0 / 55.0);
    total_height += state_texture.sample_by_lod(globalpos / 42.0, 0.0).w * (42.0 / 55.0);

    total_height / 1.0
}

pub(crate) fn get_height_offset(
    state_texture: &SampledImage<Image!(2D, type=f32, sampled)>,
    globalpos: Vec2,
    offset: IVec2,
    texture_size: IVec2,
) -> f32 {
    let uv_offset = offset.as_vec2() / texture_size.as_vec2();

    let mut s = 0.0;
    s += state_texture.sample_by_lod(globalpos / 13.0 + uv_offset, 0.0).x * (13.0 / 55.0);
    s += state_texture.sample_by_lod(globalpos / 31.0 + uv_offset, 0.0).y * (31.0 / 55.0);
    s += state_texture.sample_by_lod(globalpos / 35.0 + uv_offset, 0.0).z * (35.0 / 55.0);
    s += state_texture.sample_by_lod(globalpos / 42.0 + uv_offset, 0.0).w * (42.0 / 55.0);
    s
}

pub(crate) fn get_normal(
    state_texture: &SampledImage<Image!(2D, type=f32, sampled)>,
    globalpos: Vec2,
    pco_time: i32,
    pco_size: i32,
) -> Vec3 {
    let _time_float = pco_time as f32;
    let _direction = Vec2::new((_time_float / 100.0).sin(), (_time_float / 100.0).cos());

    const SIZE_VEC2: Vec2 = Vec2::new(2.0, 0.0);
    const OFF_IVEC3: IVec3 = IVec3::new(-1, 0, 1);

    let texture_dims = IVec2 { x: 48, y: 48 };

    let s01 = get_height_offset(
        state_texture,
        globalpos,
        IVec2::new(OFF_IVEC3.x, OFF_IVEC3.y),
        texture_dims,
    );
    let s21 = get_height_offset(
        state_texture,
        globalpos,
        IVec2::new(OFF_IVEC3.z, OFF_IVEC3.y),
        texture_dims,
    );
    let s10 = get_height_offset(
        state_texture,
        globalpos,
        IVec2::new(OFF_IVEC3.y, OFF_IVEC3.x),
        texture_dims,
    );
    let s12 = get_height_offset(
        state_texture,
        globalpos,
        IVec2::new(OFF_IVEC3.y, OFF_IVEC3.z),
        texture_dims,
    );

    let va = vec3(SIZE_VEC2.x, SIZE_VEC2.y, s21 - s01).normalize();
    let vb = vec3(SIZE_VEC2.y, SIZE_VEC2.x, s12 - s10).normalize();
    let norm = va.cross(vb);

    norm
}

pub(crate) fn wave_water_vert(
    state_texture: &SampledImage<Image!(2D, type=f32, sampled)>,
    pos: Vec2,
    shift: Vec2,
    pco_time: i32,
    pco_size: i32,
    height: &mut f32,
    normal: &mut Vec3,
) {
    let t = pco_time as f32 / 300.0;
    *height = get_height(state_texture, pos + shift, pco_time, pco_size);

    *normal = get_normal(state_texture, pos + shift, pco_time, pco_size);
}

pub(crate) fn get_water_vert(
    state_texture: &SampledImage<Image!(2D, type=f32, sampled)>,
    vert_index: i32,
    instance_index: i32,
    pco_shift: Vec4,
    pco_size: i32,
    normal: &mut Vec3,
) -> Vec3 {
    let mut vertex = Vec3::ZERO;

    let instance_y_shift = instance_index;
    let y_shift = vert_index % 2;
    let x_shift = (vert_index + 1) / 2;
    vertex.x = (x_shift as f32 / pco_size as f32) * 16.0;
    vertex.y = ((y_shift + instance_y_shift) as f32 / pco_size as f32) * 16.0;

    wave_water_vert(
        state_texture,
        vertex.xy(),
        pco_shift.xy(),
        pco_shift.w as i32,
        pco_size,
        &mut vertex.z,
        normal,
    );

    vertex
}

#[spirv(vertex)]
#[unsafe(no_mangle)]
pub fn water_vert(
    #[spirv(vertex_index)] gl_vertex_index: i32,
    #[spirv(instance_index)] gl_instance_index: i32,
    #[spirv(push_constant)] pco: &WaterPushConstants,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(descriptor_set = 0, binding = 1)] state_texture: &SampledImage<
        Image!(2D, type=f32, sampled),
    >,
    orig: &mut Vec3,
    #[spirv(position)] out_clip_pos: &mut Vec4,
) {
    let vert_id = gl_vertex_index;
    let instance_id = gl_instance_index;

    let mut normal = Vec3::ZERO;
    let rel2world = get_water_vert(
        state_texture,
        vert_id,
        instance_id,
        pco.shift,
        pco.size,
        &mut normal,
    );

    let world_pos = (rel2world + pco.shift.xyz()).extend(1.0);
    let mut clip_coords = (ubo.trans_w2s * world_pos).xyz();
    clip_coords.z = 1.0 + clip_coords.z;

    *out_clip_pos = clip_coords.extend(1.0);

    *orig = world_pos.xyz();
}
