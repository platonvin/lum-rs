#![no_std]
use common::*;

#[repr(C)]
pub struct LightmapModelsPushConstants {
    pub rot: Vec4,
    pub shift: Vec4,
}

#[spirv(vertex)]
#[unsafe(no_mangle)]
pub fn main(
    pos_in: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(push_constant)] pco: &LightmapModelsPushConstants,
    #[spirv(position)] out_clip_pos: &mut Vec4,
) {
    let fpos = pos_in.as_vec3();

    let local_pos = qtransform(pco.rot, fpos);

    let world_pos = (local_pos + pco.shift.xyz()).extend(1.0);

    let mut clip_coords = (ubo.trans_w2s * world_pos).xyz();
    clip_coords.z = 1.0 + clip_coords.z;

    *out_clip_pos = clip_coords.extend(1.0);
}
