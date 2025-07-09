#![no_std]
use common::*;

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn main(
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
