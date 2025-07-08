#![no_std]
use common::*;

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn water_frag(#[spirv(flat)] orig: Vec3, out_mat_norm: &mut UVec4) {
    let normal_encoded;

    let mut normal = (orig.dfdx_fine()).cross(orig.dfdx_fine()).normalize();
    normal = vec3(0.0, 0.0, 1.0);

    normal_encoded = ((normal + Vec3::splat(1.0)) / 2.0 * 255.0).as_uvec3();

    *out_mat_norm = UVec4::new(30, normal_encoded.x, normal_encoded.y, normal_encoded.z);
}
