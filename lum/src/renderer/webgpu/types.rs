#![allow(non_camel_case_types)]
//! module with types for wgpu backend, including Push Constant structs (PcName)

use crate::renderer::types::*;
use as_u8_slice_derive::AsU8Slice; // cast struct to u8 slice

#[allow(non_camel_case_types)]
pub type BlockId = i32;
#[allow(non_camel_case_types)]
// Material ID and Voxel are essentially the same thing
pub type MatId = i32;
// TODO: enum with empty / non-empty using NonZeroU8
// pub type Voxel = u8;
pub type Voxel = i32;

// opaque handlers. Done this way for cheap copying and simple lifetime management
#[derive(Clone, Copy)]
pub struct MeshModel(pub usize);
#[derive(Clone, Copy)]
pub struct MeshVolumetric(pub usize);
#[derive(Clone, Copy)]
pub struct MeshLiquid(pub usize);
// internal foliage mesh is already opaque handle
#[derive(Clone)]
pub struct MeshFoliage(pub InternalMeshFoliage);

#[derive(Debug, Clone, Default)]
pub struct MeshFoliageDesc {
    // shader, compiled into spirv
    // owned by description for siplicity
    pub code: &'static str,

    // Stored separately cause im fell in love with ecs
    // pub pipe: lumal::RasterPipe,

    // how many vertices will be in per-blade drawcall
    pub vertices: u32,
    // how many blades is there in a block (linear)
    pub density: u32,
}

/// The primary way of emulating push constants in wgpu
/// this struct (which is per-mesh-side) contains
#[derive(Default, Debug)]
pub struct IndexedVerticesQueue {
    pub iv: IndexedVertices,
    // TODO: there is probably some way to avoid double caching but wgpu has hidden it too good
    // guess i need to get a degree in digging trivial stuff
    pub push_constants: Vec<u8>, // size of it is equal to number of elements in draw queue
    pub pc_count: u32,
    // per-face push constant emulation buffer
    pub pc_buffer: Option<wgpu::Buffer>,
    // apart from per-face pc buffer, binds per-mesh mesh (lol) to save some bind groups
    pub pc_bg: Option<wgpu::BindGroup>,
}

/// CPU-side voxel data and GPU-side mesh handler
pub struct BlockWithMesh<BufferType, ImageType> {
    pub voxels: [[[Voxel; 16]; 16]; 16],
    pub mesh: InternalMeshModel<BufferType, ImageType>,
}

#[repr(C)]
#[derive(as_u8_slice_derive::AsU8Slice, Default, Clone, Copy, Debug)]
pub struct Material {
    pub albedo: vec3,
    // transparency is currently unused but presented because i hope to implement it soon (untouched for a year already :skull:)
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

#[repr(C)]
#[derive(AsU8Slice)]
pub struct PcRyagenBlockFace {
    pub block: i32,
    pub shift: ivec3,
    pub unorm: u32,
}

#[repr(C)]
#[derive(AsU8Slice)]
pub struct PcLightmapBlockFace {
    pub block: i32,
    pub shift: ivec3,
    pub unorm: u32,
}

#[repr(C)]
#[derive(AsU8Slice)]
pub struct PcMapModel {
    // transforms world-space coordinates into model-space (no mistake, its inverse for more precise and temporally stable mapping)
    pub trans: mat4,
    // offset of corner of the area we operate on in world space
    pub shift: ivec4,
    // area in worldspace to operate on. We submit upper bound and cull extra voxels using this (unlike Vulkan, where we submit exact size)
    pub map_area: ivec4,
}
