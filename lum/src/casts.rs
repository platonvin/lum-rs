use glam::{UVec3, UVec2, IVec3, IVec2};
use vulkanalia::vk; // Vulkan structs

// Conversion function from UVec3 to vk::Extent3D
pub fn uvec3_to_extent3d(vec: UVec3) -> vk::Extent3D {
    vk::Extent3D {
        width: vec.x,
        height: vec.y,
        depth: vec.z,
    }
}

// Conversion function from UVec2 to vk::Extent2D
pub fn uvec2_to_extent2d(vec: UVec2) -> vk::Extent2D {
    vk::Extent2D {
        width: vec.x,
        height: vec.y,
    }
}

// Conversion function from IVec3 to vk::Extent3D (signed to unsigned)
pub fn ivec3_to_extent3d(vec: IVec3) -> vk::Extent3D {
    vk::Extent3D {
        width: vec.x as u32,
        height: vec.y as u32,
        depth: vec.z as u32,
    }
}

// Conversion function from IVec2 to vk::Extent2D (signed to unsigned)
pub fn ivec2_to_extent2d(vec: IVec2) -> vk::Extent2D {
    vk::Extent2D {
        width: vec.x as u32,
        height: vec.y as u32,
    }
}