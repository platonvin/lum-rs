#![no_std]
use common::*;

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn main(end_depth_in: f32, far_depth_out: &mut f32, near_depth_out: &mut f32) {
    *far_depth_out = end_depth_in;
    *near_depth_out = end_depth_in;
}
