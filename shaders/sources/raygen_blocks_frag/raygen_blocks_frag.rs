#![no_std]
use common::*;

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn main(
    #[spirv(descriptor_set = 0, binding = 1)] block_palette_image: &Image!(3D, type=u32, sampled=false),
    sample_point: Vec3,
    #[spirv(flat)] bunorm: u32,
    out_mat_norm: &mut UVec4,
) {
    let packed: U16Vec2 = unsafe { transmute(bunorm) };
    let sample_block = packed.x as u32;
    let normal_encoded = packed.y as u32;

    let axis = UVec3::new(
        normal_encoded & 0x1,
        (normal_encoded >> 1) & 0x1,
        (normal_encoded >> 2) & 0x1,
    );
    let _sign = 1 - 2 * ((normal_encoded >> 7) & 0x1) as i32;
    let inorm = axis.as_ivec3() * _sign;

    out_mat_norm.y = (((inorm.x + 1) * 255) / 2) as u32;
    out_mat_norm.z = (((inorm.y + 1) * 255) / 2) as u32;
    out_mat_norm.w = (((inorm.z + 1) * 255) / 2) as u32;

    let ipos = sample_point.as_ivec3();

    out_mat_norm.x = get_voxel2(block_palette_image, sample_block as i32, ipos) as u32;
}
