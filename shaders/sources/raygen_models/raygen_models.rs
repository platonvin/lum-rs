#![no_std]
use common::*;

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn raygen_models_frag(
    #[spirv(descriptor_set = 1, binding = 0)] model_voxels_image: &Image!(3D, type=u32, sampled=false),
    sample_point: Vec3,
    #[spirv(flat)] normal_encoded_packed: u32,
    out_mat_norm: &mut UVec4,
) {
    let unpacked_vec4: U8Vec4 = unsafe { transmute(normal_encoded_packed) };
    let normal_encoded_unorm = unpacked_vec4.xyz();

    out_mat_norm.y = normal_encoded_unorm.x as u32;
    out_mat_norm.z = normal_encoded_unorm.y as u32;
    out_mat_norm.w = normal_encoded_unorm.z as u32;

    let ipos = sample_point.as_ivec3();

    out_mat_norm.x = get_model_voxel(model_voxels_image, ipos) as u32;
}

#[repr(C)]
pub struct RaygenModelsPushConstants {
    pub rot: Vec4,
    pub shift: Vec4,
    pub fnormal: Vec4,
}

#[spirv(vertex)]
#[unsafe(no_mangle)]
pub fn raygen_models_vert(
    pos_in: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(push_constant)] pco: &RaygenModelsPushConstants,
    sample_point: &mut Vec3,
    #[spirv(flat)] normal_encoded_packed: &mut u32,
    #[spirv(position)] out_clip_pos: &mut Vec4,
) {
    let fpos = pos_in.as_vec3();
    let fnorm_ms = pco.fnormal.xyz().normalize();

    let local_pos = qtransform(pco.rot, fpos);

    let world_pos = (local_pos + pco.shift.xyz()).extend(1.0);

    let mut clip_coords = (ubo.trans_w2s * world_pos).xyz();
    clip_coords.z = 1.0 + clip_coords.z;

    *out_clip_pos = clip_coords.extend(1.0);

    let fnorm_ws = qtransform(pco.rot, fnorm_ms);

    let normal_remapped = (fnorm_ws + Vec3::splat(1.0)) / 2.0;
    let packed_vec4 = normal_remapped.extend(0.0);

    *normal_encoded_packed = unsafe {
        transmute::<U8Vec4, u32>(u8vec4(
            packed_vec4.x as u8,
            packed_vec4.y as u8,
            packed_vec4.z as u8,
            packed_vec4.w as u8,
        ))
    };

    *sample_point = fpos - fnorm_ms * 0.5;
}
