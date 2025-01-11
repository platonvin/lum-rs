use vek::Aabb;

use super::{ivec3, mat4, vec3, vec4};

// pub struct AABB {
//     pub min: vec3,
//     pub max: vec3,
// }
#[allow(non_camel_case_types)]
pub type fAABB = Aabb<f32>;
#[allow(non_camel_case_types)]
pub type iAABB = Aabb<i32>;

trait OverlapTrait {
    fn contains(&self, point: vec3) -> bool;

    fn intersects(&self, other: &fAABB) -> bool;
}

impl OverlapTrait for fAABB {
    fn contains(&self, point: vec3) -> bool {
        return (self.min.x) <= (point.x)
            && (self.min.y) <= (point.y)
            && (self.min.z) <= (point.z)
            && (self.max.x) >= (point.x)
            && (self.max.y) >= (point.y)
            && (self.max.z) >= (point.z);
    }

    fn intersects(&self, other: &fAABB) -> bool {
        return self.min.x <= other.max.x
            && self.min.y <= other.max.y
            && self.min.z <= other.max.z
            && self.max.x >= other.min.x
            && self.max.y >= other.min.y
            && self.max.z >= other.min.z;
    }
}

pub fn get_shift(trans: mat4, size: ivec3) -> fAABB {
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
    let mut tmin = vec3::new(std::f32::MAX, std::f32::MAX, std::f32::MAX);
    let mut tmax = vec3::new(std::f32::MIN, std::f32::MIN, std::f32::MIN);

    // Transform all corners and calculate AABB bounds
    for corner in corners {
        let transformed = trans * vec4::new(corner.x, corner.y, corner.z, 1.0);
        let point = vec3::try_from(transformed).unwrap();

        tmin = vec3::partial_min(tmin, point);
        tmax = vec3::partial_max(tmax, point);
    }

    return fAABB {
        min: tmin,
        max: tmax,
    };
}
