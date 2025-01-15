#![allow(non_camel_case_types)]

use std::string::String;

use lumal::ring::Ring;
use lumal::Buffer;
// use crate::Voxel;
pub type MatID = u32;

use vek::{Vec2, Vec3};
use vulkanalia::vk; // Vulkan structs

pub type uvec4 = vek::Vec4<u32>;
pub type u16vec4 = vek::Vec4<u16>;
pub type u8vec4 = vek::Vec4<u8>;
pub type uvec3 = vek::Vec3<u32>;
pub type u16vec3 = vek::Vec3<u16>;
pub type u8vec3 = vek::Vec3<u8>;
pub type uvec2 = vek::Vec2<u32>;
pub type u16vec2 = vek::Vec2<u16>;
pub type u8vec2 = vek::Vec2<u8>;

pub type ivec4 = vek::Vec4<i32>;
pub type i16vec4 = vek::Vec4<i16>;
pub type i8vec4 = vek::Vec4<i8>;
pub type ivec3 = vek::Vec3<i32>;
pub type i16vec3 = vek::Vec3<i16>;
pub type i8vec3 = vek::Vec3<i8>;
pub type ivec2 = vek::Vec2<i32>;
pub type i16vec2 = vek::Vec2<i16>;
pub type i8vec2 = vek::Vec2<i8>;

pub type vec4 = vek::Vec4<f32>;
pub type vec3 = vek::Vec3<f32>;
pub type vec2 = vek::Vec2<f32>;

pub type mat4 = vek::Mat4<f32>;
// pub type dmat4 = vek::Mat4<f32>;
pub type quat = vek::quaternion::Quaternion<f32>;


#[allow(non_camel_case_types)]
pub type BlockID_t = i16; 
#[allow(non_camel_case_types)]
pub type MatID_t = u8;
// TODO: enum with empty / non-empty using NonZeroU8
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct Voxel (pub u8);

// CPU side structure with actual voxel data but only gpu mesh handler
pub struct BlockWithMesh {
    pub voxels: [[[Voxel; 16]; 16]; 16],
    pub mesh: InternalMeshModel,
}

#[repr(C)]
#[derive(as_u8_slice_derive::AsU8Slice, Default, Clone, Copy)]
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
    pub mat_id: MatID_t,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AoLut {
    pub world_shift: vec3,
    pub weight_normalized: f32, // ((1-r^2)/total_weight)*0.7
    pub screen_shift: vec2,
    pub padding: vec2,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VoxelVertex {
    pub pos: u8vec3,
    pub norm: i8vec3,
    pub mat_id: MatID,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PackedVoxelVertex {
    pub pos: u8vec3,
    pub mat_id: MatID,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PackedVoxelQuad {
    pub size: u8vec2,
    pub pos: u8vec3,
    pub mat_id: MatID,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PackedVoxelCircuit {
    pub pos: u8vec3,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IndexedVertices {
    pub offset: u32, // yes, they are all stored in same buffer and accessed with offset
    pub icount: u32, 
}

#[allow(non_snake_case)]
#[derive(Debug, Default)]
pub struct FaceBuffers {
    // zPz means zero-Positive-zero
    // zzN means zero-zero-Negative
    pub Pzz: IndexedVertices,
    pub Nzz: IndexedVertices,
    pub zPz: IndexedVertices,
    pub zNz: IndexedVertices,
    pub zzP: IndexedVertices,
    pub zzN: IndexedVertices,
    pub vertexes: lumal::Buffer,
    pub indices: lumal::Buffer,
}

#[derive(Debug, Default)]
pub struct InternalMeshModel {
    pub triangles: FaceBuffers,
    pub voxels: Ring<lumal::Image>,
    pub size: uvec3, // integer because in voxels
}

#[derive(Debug, Default)]
pub struct InternalMeshFoliage {
    pub vertex_shader_file: String,
    pub pipe: lumal::RasterPipe,
    pub vertices: i32,
    pub density: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct InternalMeshLiquid {
    pub main: MatID,
    pub foam: MatID,
}

#[derive(Clone, Debug)]
pub struct InternalMeshVolumetric {
    pub max_density: f32,
    pub variation: f32,
    pub color: u8vec3,
}

#[derive(Debug)]
pub struct InternalUiMesh {
    pub vertexes: lumal::Buffer,
    pub indexes: lumal::Buffer,
    pub icount: u32,
    pub image: *mut lumal::Image,
}

#[derive(Debug)]
pub struct MeshTransform {
    pub rotation: quat,
    pub translation: vec3,
}

pub type BlockVoxels = [Voxel; 16*16*16];

#[derive(Debug)]
pub struct Block {
    pub voxels: BlockVoxels,
    pub mesh: InternalMeshModel,
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