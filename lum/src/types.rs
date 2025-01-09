#![allow(non_camel_case_types)]

use std::string::String;

use lumal::ring::Ring;
use lumal::Buffer;
use crate::Voxel;
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

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VoxelVertex {
    pub pos: u8vec3,
    pub norm: i8vec3,
    pub mat_id: MatID,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PackedVoxelVertex {
    pub pos: u8vec3,
    pub mat_id: MatID,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PackedVoxelQuad {
    pub size: u8vec2,
    pub pos: u8vec3,
    pub mat_id: MatID,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PackedVoxelCircuit {
    pub pos: u8vec3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct IndexedVertices {
    pub offset: u32,
    pub icount: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct FaceBuffers {
    pub pzz: IndexedVertices,
    pub nzz: IndexedVertices,
    pub zpz: IndexedVertices,
    pub znz: IndexedVertices,
    pub zzp: IndexedVertices,
    pub zzn: IndexedVertices,
    pub vertexes: lumal::Buffer,
    pub indices: lumal::Buffer,
}

#[repr(C)]
// #[derive(Debug)]
pub struct InternalMeshModel {
    pub triangles: FaceBuffers,
    pub voxels: Ring<lumal::Image>,
    pub size: ivec3, // integer because in voxels
}

#[repr(C)]
// #[derive(Debug)]
pub struct InternalMeshFoliage {
    pub vertex_shader_file: String,
    pub pipe: lumal::RasterPipe,
    pub vertices: i32,
    pub density: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InternalMeshLiquid {
    pub main: MatID,
    pub foam: MatID,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct InternalMeshVolumetric {
    pub max_density: f32,
    pub variation: f32,
    pub color: u8vec3,
}

#[repr(C)]
#[derive(Debug)]
pub struct InternalUiMesh {
    pub vertexes: lumal::Buffer,
    pub indexes: lumal::Buffer,
    pub icount: u32,
    pub image: *mut lumal::Image,
}

pub type BlockVoxels = [Voxel; 16*16*16];

#[repr(C)]
// #[derive(Debug)]
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