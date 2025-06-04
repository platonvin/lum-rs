#![allow(non_camel_case_types)]

use block_mesh::VoxelVisibility;
use qvek::vek::{
    self,
    num_traits::{One, Zero},
};

// use super::webgpu::types::IndexedVerticesQueue;

// // my glsl brain dictaited me to do this
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

impl<V: PartialEq + Zero> block_mesh::Voxel for VoxelForContour<V> {
    #[cold]
    #[optimize(size)]
    fn get_visibility(&self) -> VoxelVisibility {
        if self.0 == Zero::zero() {
            // TODO: ENUM
            VoxelVisibility::Empty
        } else {
            VoxelVisibility::Opaque
        } // never transluent
    }
}

impl<V: PartialEq + Zero + One + Eq> block_mesh::MergeVoxel for VoxelForContour<V> {
    type MergeValue = VoxelForContour<V>;

    #[cold]
    #[optimize(size)]
    fn merge_value(&self) -> Self::MergeValue {
        // we only care about contour, thus if not emtpy, merging is allowed
        let zero = V::zero();
        match &self.0 {
            zero => VoxelForContour(Zero::zero()),
            _ => VoxelForContour(One::one()),
        }
    }
}

// // CPU side structure with actual voxel data but only gpu mesh handler
// pub struct BlockWithMesh<BufferType, ImageType> {
//     pub voxels: [[[Voxel; 16]; 16]; 16],
//     pub mesh: InternalMeshModel<BufferType, ImageType>,
// }

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

// #[repr(C)]
// #[derive(Debug, Clone, Copy, Default)]
// pub struct AoLut {
//     pub world_shift: vec3,
//     pub weight_normalized: f32, // ((1-r^2)/total_weight)*0.7
//     pub screen_shift: vec2,
//     pub padding: vec2,
// }

// #[derive(Clone, Copy, Debug, Default)]
// pub struct VoxelVertex {
//     pub pos: u8vec3,
//     pub norm: i8vec3,
//     pub mat_id: MatId,
// }

// #[derive(Clone, Copy, Debug, Default)]
// pub struct PackedVoxelVertex {
//     pub pos: u8vec3,
//     pub mat_id: MatId,
// }

// #[derive(Clone, Copy, Debug, Default)]
// pub struct PackedVoxelQuad {
//     pub size: u8vec2,
//     pub pos: u8vec3,
//     pub mat_id: MatId,
// }

// #[repr(C)]
// #[derive(Clone, Copy, Debug, Default)]
// pub struct PackedVoxelCircuit {
//     pub pos: u8vec3,
// }

// IndexedVertices is just another way to store where the data is in (single) allocated buffer
// this could have been 6 buffers, but insted it is 1 buffer and 6 (offset+index_count)s
#[derive(Clone, Copy, Debug, Default)]
pub struct IndexedVertices {
    // TODO: u16
    pub offset: u32, // yes, they are all stored in same buffer and accessed with offset
    pub icount: u32,
}

#[allow(non_snake_case)]
#[derive(Debug, Default)]
pub struct FaceBuffers<BufferType, IndexedVerticesType = IndexedVertices> {
    // zPz means zero-Positive-zero
    // zzN means zero-zero-Negative
    pub Pzz: IndexedVerticesType,
    pub Nzz: IndexedVerticesType,
    pub zPz: IndexedVerticesType,
    pub zNz: IndexedVerticesType,
    pub zzP: IndexedVerticesType,
    pub zzN: IndexedVerticesType,
    pub vertexes: BufferType,
    pub indices: BufferType,
}

#[allow(non_snake_case)]
#[derive(Debug, Default, Clone, Copy)]
pub struct FaceBuffersShared {
    // zPz means zero-Positive-zero
    // zzN means zero-zero-Negative
    pub Pzz: IndexedVertices,
    pub Nzz: IndexedVertices,
    pub zPz: IndexedVertices,
    pub zNz: IndexedVertices,
    pub zzP: IndexedVertices,
    pub zzN: IndexedVertices,
}

// #[allow(non_snake_case)]
// #[derive(Debug, Default, Clone)]
// pub struct FaceBuffersData {
//     pub vertexes: lumal::Buffer,
//     pub indices: lumal::Buffer,
// }

// #[derive(Clone, Copy, Debug, Default)]
// pub struct SpriteDescription {
//     // offset & size of voxel data
//     pub offset: u8vec3,
//     pub size: u8vec3,
//     pub faces: FaceBuffersShared,
// }

#[derive(Debug)]
pub struct MetadataMapModel {
    pub workgroup_size: ivec3,
}

// handle (reference) to a mesh.
// You can clone it but still need to unload one time
#[derive(Debug, Default)]
pub struct InternalMeshModel<BufferType, ImageType, IndexedVerticesType = IndexedVertices> {
    pub triangles: FaceBuffers<BufferType, IndexedVerticesType>,
    // when model has multiple sprites in a spritesheet, `voxels` contains all of them, stacked along `Y`
    pub voxels: ImageType,
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

//5.76k bytes for compact 100 sprites (models)
//11.1k bytes for non-compact 100 sprites (models)

// handle (reference) to a block triangles.
// You can clone it but still need to unload one time
#[derive(Debug, Default)]
pub struct InternalMeshBlock<BufferType, IndexedVerticesType = IndexedVertices> {
    pub triangles: FaceBuffers<BufferType, IndexedVerticesType>,
}

#[derive(Debug, Clone, Default)]
pub struct InternalMeshFoliage {
    pub stored_id: u32,
}

#[derive(Clone, Debug, Default)]
pub struct InternalMeshLiquid<MatIdType> {
    pub main: MatIdType,
    pub foam: MatIdType,
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

#[derive(Debug, Default, Copy, Clone)]
pub struct MeshTransform {
    pub rotation: quat,
    pub translation: vec3,
}

pub type BlockVoxels<VoxelType> = [[[VoxelType; 16]; 16]; 16];

#[derive(Debug)]
pub struct Block<BufferType, ImageType, VoxelType> {
    pub voxels: BlockVoxels<VoxelType>,
    pub mesh: InternalMeshModel<BufferType, ImageType>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AoLut {
    pub world_shift: vec3,
    pub weight_normalized: f32, // ((1-r^2)/total_weight)*0.7
    pub screen_shift: vec2,
    pub padding: vec2,
}
