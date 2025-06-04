#![allow(non_camel_case_types)]

use crate::renderer::types::*;
use lumal::vk;

#[allow(non_camel_case_types)]
pub type BlockId = i16;
#[allow(non_camel_case_types)]
// Material ID and Voxel are essentially the same thing
pub type MatId = u8;
// TODO: enum with empty / non-empty using NonZeroU8
pub type Voxel = u8;

// opaque handlers. Done this way for cheap copying and simple lifetime management
#[derive(Clone, Copy)]
pub struct MeshModel(pub usize);
#[derive(Clone, Copy)]
pub struct MeshVolumetric(pub usize);
#[derive(Clone, Copy)]
pub struct MeshLiquid(pub usize);
#[derive(Clone)]
// internal foliage mesh is already opaque handle
pub struct MeshFoliage(pub InternalMeshFoliage);

// CPU side structure with actual voxel data but only gpu mesh handler
pub struct BlockWithMesh<BufferType, ImageType> {
    pub voxels: [[[Voxel; 16]; 16]; 16],
    pub mesh: InternalMeshModel<BufferType, ImageType>,
}

#[repr(C)]
#[derive(as_u8_slice_derive::AsU8Slice, Default, Clone, Copy, Debug)]
pub struct Material {
    pub albedo: vec3,
    pub transparency: f32,
    pub emmitness: f32,
    pub roughness: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Particle {
    pub pos: vec3,
    pub vel: vec3,
    pub life_time: f32,
    pub mat_id: MatId,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VoxelVertex {
    pub pos: u8vec3,
    pub norm: i8vec3,
    pub mat_id: MatId,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PackedVoxelVertex {
    pub pos: u8vec3,
    pub mat_id: MatId,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PackedVoxelQuad {
    pub size: u8vec2,
    pub pos: u8vec3,
    pub mat_id: MatId,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PackedVoxelCircuit {
    pub pos: u8vec3,
}

// Conversion function from UVec3 to vk::Extent3D
pub fn uvec3_to_extent3d(vec: uvec3) -> vk::Extent3D {
    vk::Extent3D {
        width: vec.x,
        height: vec.y,
        depth: vec.z,
    }
}

// Conversion function from UVec2 to vk::Extent2D
pub fn uvec2_to_extent2d(vec: uvec2) -> vk::Extent2D {
    vk::Extent2D {
        width: vec.x,
        height: vec.y,
    }
}

// Conversion function from ivec3 to vk::Extent3D (signed to unsigned)
pub fn ivec3_to_extent3d(vec: ivec3) -> vk::Extent3D {
    vk::Extent3D {
        width: vec.x as u32,
        height: vec.y as u32,
        depth: vec.z as u32,
    }
}

// Conversion function from ivec2 to vk::Extent2D (signed to unsigned)
pub fn ivec2_to_extent2d(vec: ivec2) -> vk::Extent2D {
    vk::Extent2D {
        width: vec.x as u32,
        height: vec.y as u32,
    }
}
