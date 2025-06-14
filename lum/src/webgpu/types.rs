#![allow(non_camel_case_types)]
//! module with types for wgpu backend, including Push Constant structs (PcName)

use crate::{
    render_interface::{FoliageDescriptionCreate, ShaderSource},
    types::*,
    webgpu::wal,
};
use as_u8_slice_derive::AsU8Slice; // cast struct to u8 slice

pub type InternalBlockId = i32;
// Material ID and Voxel are essentially the same thing
pub type InternalMatId = i32;
// TODO: enum with empty / non-empty using NonZeroU8
pub type InternalVoxel = i32;

/// GPU-side particle (grid-aigned but not grid-snapped cube with material and size dependent lifetime)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InternalParticle {
    pub pos: vec3,
    pub vel: vec3,
    pub life_time: f32,
    pub mat_id: InternalMatId,
}

impl<'a> FoliageDescriptionCreate<'a> for MeshFoliageDesc<'a> {
    fn new(code: ShaderSource<'a>, vertices: usize, dencity: usize) -> Self {
        let ShaderSource::Wgsl(code) = code else {
            panic!()
        };
        Self {
            code: code,
            vertices: vertices as u32,
            density: dencity as u32,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MeshFoliageDesc<'a> {
    pub code: &'a str,

    // Stored separately cause im fell in love with ecs
    // pub pipe: lumal::RasterPipe,

    // how many vertices will be in per-blade drawcall
    pub vertices: u32,
    // how many blades is there in a block (linear)
    pub density: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IndexedVertices {
    // TODO: u16?
    pub offset: u32, // they are all stored in same buffer and accessed with offset
    pub icount: u32,
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

#[allow(non_snake_case)]
#[derive(Debug, Default)]
/// Bundle of "virtual" (indexed)buffers for faces of a mesh
/// zPz means zero-Positive-zero, zzN means zero-zero-Negative and so on
pub struct FaceBuffers {
    // we emulate push constants with per-face push constant queues
    pub Pzz: IndexedVerticesQueue,
    pub Nzz: IndexedVerticesQueue,
    pub zPz: IndexedVerticesQueue,
    pub zNz: IndexedVerticesQueue,
    pub zzP: IndexedVerticesQueue,
    pub zzN: IndexedVerticesQueue,
    /// actual GPU buffer for vertices
    pub vertexes: Option<wgpu::Buffer>,
    /// actual GPU buffer for indices
    pub indices: Option<wgpu::Buffer>,
}

#[derive(Default)]
pub struct InternalMeshModel {
    pub triangles: FaceBuffers,
    // when model has multiple sprites in a spritesheet, `voxels` contains all of them, stacked along `Y`
    pub voxels: Option<wal::Image>,
    // size of voxels. So if only one sprite, equal to its size, but when multiple - equal to sum of sizes
    // integer because in voxels
    pub size: uvec3,

    // this is not needed since bind groups are now per-face and include what this used to bind
    // pub voxels_bind_group_fragment: Option<wgpu::BindGroup>,
    pub compute_push_constants: Vec<u8>,
    // pub compute_metadata: Vec<MetadataMapModel>,
    // separate from faces cause thats what i came up with.
    pub compute_pc_buffer: Option<wgpu::Buffer>,
    // this is still needed cause our compute workload is per-mehsh, not per-face
    pub voxels_bind_group_compute: Option<wgpu::BindGroup>,
    pub compute_pc_count: i32,
}

#[derive(Debug, Clone, Default)]
pub struct InternalMeshFoliage {
    pub stored_id: u32,
}

#[derive(Clone, Debug, Default)]
pub struct InternalMeshLiquid {
    pub main: InternalMatId,
    pub foam: InternalMatId,
    // pub pc_buffer: Option<wgpu::Buffer>,
    // pub pc_bg: Option<wgpu::BindGroup>,
    // pub push_constants: Vec<u8>,
    // pub pc_count: i32,
}

#[derive(Clone, Debug, Default)]
pub struct InternalMeshVolumetric {
    pub max_density: f32,
    pub variation: f32,
    pub color: u8vec3,
}

/// CPU-side voxel data and GPU-side mesh handler
#[derive(Default)]
pub struct InternalMeshBlock {
    // pub voxels: [[[Voxel; 16]; 16]; 16],
    pub triangles: FaceBuffers,
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
