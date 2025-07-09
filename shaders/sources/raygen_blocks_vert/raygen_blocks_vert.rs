#![no_std]
use common::*;

#[repr(C)]
pub struct RaygenBlocksPushConstants {
    // pub block: i16,
    // TODO: ISSUE: separating block into i16 breaks layout.
    pub block_shift: I16Vec4,
    pub unorm: U8Vec4,
}

#[spirv(vertex)]
#[unsafe(no_mangle)]
pub fn main(
    pos_in: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(push_constant)] pco: &RaygenBlocksPushConstants,
    sample_point: &mut Vec3,
    #[spirv(flat)] bunorm: &mut u32,
    #[spirv(position)] out_clip_pos: &mut Vec4,
) {
    let ipos = pos_in.as_ivec3();
    let normal_encoded = pco.unorm.x;

    let s = ((normal_encoded & (1 << 7)) >> 7) as i32;
    let axis = IVec3::new(
        ((normal_encoded & (1 << 0)) >> 0) as i32,
        ((normal_encoded & (1 << 1)) >> 1) as i32,
        ((normal_encoded & (1 << 2)) >> 2) as i32,
    );
    let inorm = axis * (1 - s * 2);
    let fnorm = inorm.as_vec3();

    let uworld_pos = ipos + pco.block_shift.yzw().as_ivec3();
    let fworld_pos = uworld_pos.as_vec3().extend(1.0);

    let mut clip_coords = (ubo.trans_w2s * fworld_pos).xyz();
    clip_coords.z = 1.0 + clip_coords.z;

    *out_clip_pos = clip_coords.extend(1.0);

    *sample_point = ipos.as_vec3() - fnorm * 0.5;

    let sample_block_u32 = pco.block_shift.x as u32;

    *bunorm = unsafe { transmute(U16Vec2::new(sample_block_u32 as u16, normal_encoded as u16)) };
}
