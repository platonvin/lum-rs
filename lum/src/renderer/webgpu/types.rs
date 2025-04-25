#![allow(non_camel_case_types)]

use block_mesh::VoxelVisibility;
use qvek::vek::{
    self,
    num_traits::{One, Zero},
};

use crate::renderer::types::*;

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

// IndexedVertices is just another way to store where the data is in (single) allocated buffer
// this could have been 6 buffers, but insted it is 1 buffer and 6 (offset+index_count)s
#[derive(Clone, Copy, Debug, Default)]
pub struct IndexedVertices {
    // TODO: u16
    pub offset: u32, // yes, they are all stored in same buffer and accessed with offset
    pub icount: u32,
}
