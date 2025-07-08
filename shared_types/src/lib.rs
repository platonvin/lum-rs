//! Module with all visible in API types (the ones that do not change based on backend)

// vec4 > Vec<f32, 4>
#![allow(non_camel_case_types)]

pub const BLOCK_SIZE: u32 = 16;
#[allow(non_upper_case_globals)]
pub const sBLOCK_SIZE: usize = 16;
#[allow(non_upper_case_globals)]
pub const fBLOCK_SIZE: f32 = BLOCK_SIZE as f32;

use qvek::vek;

// my glsl brain dictaited me to do this
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

pub type dvec4 = vek::Vec4<f64>;
pub type dvec3 = vek::Vec3<f64>;
pub type dvec2 = vek::Vec2<f64>;

pub type mat4 = vek::Mat4<f32>;
pub type dmat4 = vek::Mat4<f64>;
pub type quat = vek::quaternion::Quaternion<f32>;
pub type dquat = vek::quaternion::Quaternion<f64>;

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct VoxelForContour<V: PartialEq>(pub V);

#[repr(C)]
#[derive(as_u8_slice_derive::AsU8Slice, Default, Clone, Copy)]
pub struct Material {
    pub albedo: vec3,
    pub transparency: f32,
    pub emmitness: f32,
    pub roughness: f32,
}
impl std::fmt::Debug for Material {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2})",
            self.albedo.x,
            self.albedo.y,
            self.albedo.z,
            self.transparency,
            self.emmitness,
            self.roughness,
        )
    }
}

/// API BlockId type - same across all backends. Used in API and CPU-side world representation (GPU-side does NOT use this exact type)
pub type BlockId = i16;
// Material ID and Voxel are essentially the same thing
pub type MatId = u8;
// TODO: enum with empty / non-empty using NonZeroU8
pub type Voxel = u8;

pub type MeshBlock = i16;
// opaque handlers. Done this way for cheap copying and simple lifetime management
pub type MeshModel = usize;
pub type MeshVolumetric = usize;
pub type MeshLiquid = usize;
pub type MeshFoliage = usize;

// I am unsure about if this should be shared between backends but it is at the moment
/// CPU-side particle (grid-aligned but not grid-snapped cube with material and size, dependent on lifetime)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Particle {
    pub pos: vec3,
    pub vel: vec3,
    pub life_time: f32,
    pub mat_id: MatId,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct MeshTransform {
    pub rotation: quat,
    pub translation: vec3,
}

pub type BlockVoxels = [[[Voxel; BLOCK_SIZE as usize]; BLOCK_SIZE as usize]; BLOCK_SIZE as usize];

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AoLut {
    pub world_shift: vec3,
    pub weight_normalized: f32, // ((1-r^2)/total_weight)*0.7
    pub screen_shift: vec2,
    pub padding: vec2,
}

#[cfg(feature = "vk_backend")]
pub mod vk_types {
    pub use crate::*;

    pub type InternalBlockId = i16;
    // Material ID and Voxel are essentially the same thing
    pub type InternalMatId = u8;
    // TODO: enum with empty / non-empty using NonZeroU8
    pub type InternalVoxel = u8;

    /// IndexedVertices is just another way to store where the data is in (single) allocated buffer
    /// this could have been 6 buffers, but insted it is 1 buffer and 6 (offset+index_count)s
    #[derive(Clone, Copy, Debug, Default)]
    pub struct IndexedVertices {
        // TODO: u16?
        pub offset: u32, // they are all stored in same buffer and accessed with offset
        pub icount: u32,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Default)]
    /// Bundle of "virtual" (indexed) buffers for faces of a mesh
    /// IndexedVertices point to slices of indices which point to (not necessarily slices (almost never slices) of) vertices
    /// zPz means zero-Positive-zero ((0,1,0)normal), zzN means zero-zero-Negative ((0,0,-1) normal) and so on
    pub struct FaceBuffers {
        /// Slice of indices, corresponding to (+1,0,0) normal
        pub Pzz: IndexedVertices,
        /// Slice of indices, corresponding to (-1,0,0) normal
        pub Nzz: IndexedVertices,
        /// Slice of indices, corresponding to (0,+1,0) normal
        pub zPz: IndexedVertices,
        /// Slice of indices, corresponding to (0,-1,0) normal
        pub zNz: IndexedVertices,
        /// Slice of indices, corresponding to (0,0,+1) normal
        pub zzP: IndexedVertices,
        /// Slice of indices, corresponding to (0,0,-1) normal
        pub zzN: IndexedVertices,
        /// actual GPU buffer for vertices
        pub vertexes: lumal::Buffer,
        /// actual GPU buffer for indices
        pub indices: lumal::Buffer,
    }

    #[derive(Debug, Default)]
    /// Does not own CPU-side memory
    pub struct InternalMeshModel {
        pub triangles: FaceBuffers,
        // when model has multiple sprites in a spritesheet, `voxels` contains all of them, stacked along `Y`
        pub voxels: lumal::Image,
        // size of voxels. So if only one sprite, equal to its size, but when multiple - equal to sum of sizes
        // integer because in voxels
        pub size: uvec3,
    }

    /// Holds CPU-side voxel data and GPU-side mesh handle
    #[derive(Debug, Default)]
    pub struct InternalMeshBlock {
        // pub voxels: [[[Voxel; BLOCK_SIZE]; BLOCK_SIZE]; BLOCK_SIZE],
        pub triangles: FaceBuffers,
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

    // #[repr(C)]
    // #[derive(as_u8_slice_derive::AsU8Slice, Default, Clone, Copy, Debug)]
    // pub struct Material {
    //     pub albedo: vec3,
    //     pub transparency: f32,
    //     pub emmitness: f32,
    //     pub roughness: f32,
    // }

    // #[repr(C)]
    // #[derive(Debug, Clone, Copy, Default)]
    // pub struct Particle {
    //     pub pos: vec3,
    //     pub vel: vec3,
    //     pub life_time: f32,
    //     pub mat_id: MatId,
    // }

    // I am unsure about if this should be shared between backends but it is at the moment
    /// CPU-side particle (grid-aigned but not grid-snapped cube with material and size dependent lifetime)
    // #[repr(C)]
    // #[derive(Debug, Clone, Copy, Default)]
    // pub struct InternalParticle {
    //     pub pos: vec3,
    //     pub vel: vec3,
    //     pub life_time: f32,
    //     pub mat_id: InternalMatId,
    // }

    #[derive(Clone, Copy, Debug, Default)]
    pub struct VoxelVertex {
        pub pos: u8vec3,
        pub norm: i8vec3,
        pub mat_id: InternalMatId,
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub struct PackedVoxelVertex {
        pub pos: u8vec3,
        pub mat_id: InternalMatId,
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub struct PackedVoxelQuad {
        pub size: u8vec2,
        pub pos: u8vec3,
        pub mat_id: InternalMatId,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default)]
    pub struct PackedVoxelCircuit {
        pub pos: u8vec3,
    }
}
