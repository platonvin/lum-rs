#![no_std]
use common::*;

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn main(#[spirv(flat)] mat_norm: UVec4, out_mat_norm: &mut UVec4) {
    *out_mat_norm = mat_norm;
}
