use qvek::vek::Aabb;

use super::types::*;

// use super::{mat4, uvec3, vec3, vec4};

#[allow(non_camel_case_types)]
pub type fAABB = Aabb<f32>;
#[allow(non_camel_case_types)]
pub type iAABB = Aabb<i32>;

pub fn get_shift(trans: mat4, size: uvec3) -> fAABB {
    let box_vec = vec3::new(
        (size.x - 1) as f32,
        (size.y - 1) as f32,
        (size.z - 1) as f32,
    );
    let corners = [
        vec3::new(0.0, 0.0, 0.0),
        vec3::new(0.0, box_vec.y, 0.0),
        vec3::new(0.0, box_vec.y, box_vec.z),
        vec3::new(0.0, 0.0, box_vec.z),
        vec3::new(box_vec.x, 0.0, 0.0),
        vec3::new(box_vec.x, box_vec.y, 0.0),
        box_vec,
        vec3::new(box_vec.x, 0.0, box_vec.z),
    ];

    // transform the first corner
    let mut tmin = vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut tmax = vec3::new(f32::MIN, f32::MIN, f32::MIN);

    // Transform all corners and calculate AABB bounds
    for corner in corners {
        let point = trans * vec4::new(corner.x, corner.y, corner.z, 1.0);

        tmin = vec3::partial_min(tmin, point);
        tmax = vec3::partial_max(tmax, point);
    }

    fAABB {
        min: tmin,
        max: tmax,
    }
}
