use crate::sBLOCK_SIZE;
use crate::types::{ivec3, u8vec3, uvec3, BlockId, Voxel};
use qvek::vek::num_traits::{One, Zero};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SimpleVertex {
    pub pos: u8vec3,
}

/// IndexedVertices is just another way to store where the data is in (single) allocated buffer
/// this could have been 6 buffers, but insted it is 1 buffer and 6 (offset+index_count)s
#[derive(Clone, Copy, Debug, Default)]
pub struct SImpleIndexedSlice {
    // TODO: u16?
    pub offset: u32, // they are all stored in same buffer and accessed with offset
    pub icount: u32,
}

#[allow(non_snake_case)]
#[derive(Debug, Default, Clone, Copy)]
/// Bundle of "virtual" (indexed) buffers for faces of a mesh
/// IndexedVertices point to slices of indices which point to (not necessarily slices (almost never slices) of) vertices
/// zPz means zero-Positive-zero ((0,1,0)normal), zzN means zero-zero-Negative ((0,0,-1) normal) and so on
pub struct SimpleFaceIndices {
    /// Slice of indices, corresponding to +X face
    pub Pzz: SImpleIndexedSlice,
    /// Slice of indices, corresponding to -X face
    pub Nzz: SImpleIndexedSlice,
    /// Slice of indices, corresponding to +Y face
    pub zPz: SImpleIndexedSlice,
    /// Slice of indices, corresponding to -Y face
    pub zNz: SImpleIndexedSlice,
    /// Slice of indices, corresponding to +Z face
    pub zzP: SImpleIndexedSlice,
    /// Slice of indices, corresponding to -Z face
    pub zzN: SImpleIndexedSlice,
}

#[repr(C)]
#[derive(as_u8_slice_derive::AsU8Slice)]
pub struct Metadata {
    pub size: ivec3,
    pub face_indices: SimpleFaceIndices,
}

/// Model mesh memory in relatively compact form (matches Vulkan, needs repacking for wgpu)
#[repr(C)]
pub struct ModelData<'a> {
    pub size: ivec3,
    pub iv: SimpleFaceIndices,
    pub voxels: &'a [Voxel],
    pub vertices: &'a [SimpleVertex],
    pub indices: &'a [u16],
}

/// Block mesh memory in relatively compact form (matches Vulkan, needs repacking for wgpu)
#[repr(C)]
pub struct BlockData<'a> {
    pub iv: SimpleFaceIndices,
    pub voxels: &'a [Voxel; sBLOCK_SIZE * sBLOCK_SIZE * sBLOCK_SIZE],
    pub vertices: &'a [SimpleVertex],
    pub indices: &'a [u16],
}

#[repr(C)]
pub struct SceneData<'a> {
    pub size: ivec3,
    pub blocks: &'a [BlockId],
}

#[macro_export]
macro_rules! include_bytes_aligned {
    ($align_to:expr, $path:expr) => {{
        #[repr(C, align($align_to))]
        struct __Aligned<T: ?Sized>(T);

        const __DATA: &'static __Aligned<[u8]> = &__Aligned(*include_bytes!($path));

        &__DATA.0
    }};
}
pub fn cast_meta(b: &'static [u8]) -> &'static Metadata {
    unsafe { &*(b.as_ptr() as *const _) }
}
pub fn cast_slice<T>(b: &'static [u8]) -> &'static [T] {
    let len = b.len() / std::mem::size_of::<T>();
    unsafe { std::slice::from_raw_parts(b.as_ptr() as *const _, len) }
}

// Lum is not going to parse any file format and create triangles, so you need to prepare them separately
// This is done this way for binary size and perfomance

/// Trait for abstracting (internal) renderer resource loading (to/from GPU)
pub(crate) trait LoadInterface {
    type Buffer;
    type Image;
    type BlockId: Clone;
    type MatId;
    type IndexedVertices;
    type InternalMeshModel;
    type InternalMeshBlock;
    type InternalMeshFoliage;
    type InternalMeshLiquid;
    type InternalMeshVolumetric;
    type FaceBuffers;
    type Voxel: Zero + One + Eq + Default + Clone + From<u8>;

    fn update_block_palette_to_gpu(&mut self);
    fn update_material_palette_to_gpu(&mut self);

    /// Loads mesh in specified format from provided memory
    fn load_model(&mut self, model: ModelData) -> Self::InternalMeshModel;

    // i love that we can implement functions in traits

    // fn load_mesh_from_file(
    //     &mut self,
    //     mesh_file: &str,
    //     _make_vertices: bool,
    //     extrude_palette: bool,
    // ) -> Self::InternalMeshModel {
    //     let scene = ogt_vox::read_scene_from_file(mesh_file).unwrap();
    //     assert!(scene.models.len() == 1); // only one model per file supported for now
    //     let model = &scene.models[0];
    //     assert!(model.size_x > 0 && model.size_y > 0 && model.size_z > 0);

    //     if extrude_palette && !self.has_palette() {
    //         println!("Extruding palette");
    //         self.extract_palette_from_scene(&scene);
    //         self.set_has_palette(true);
    //     }

    //     self.load_mesh_from_memory(model, true)
    // }

    // fn load_meshes_from_file(
    //     &mut self,
    //     meshes_file: &str,
    //     _make_vertices: bool,
    //     extrude_palette: bool,
    // ) -> Vec<Self::InternalMeshModel> {
    //     let scene = ogt_vox::read_scene_from_file(meshes_file).unwrap();

    //     if extrude_palette && !self.has_palette() {
    //         println!("Extruding palette");
    //         self.extract_palette_from_scene(&scene);
    //         self.set_has_palette(true);
    //     }

    //     scene
    //         .models
    //         .iter()
    //         .map(|model| {
    //             assert!(model.size_x > 0 && model.size_y > 0 && model.size_z > 0);

    //             self.load_mesh_from_memory(model, true)
    //         })
    //         .collect()
    // }

    // fn load_block_from_file(&mut self, block: Self::BlockId, path: &str) {
    //     let scene = ogt_vox::read_scene_from_file(path).unwrap();
    //     assert!(scene.models.len() == 1); // only one model per file supported for now
    //                                       // blocks are always BLOCK_SIZE*BLOCK_SIZE*BLOCK_SIZE
    //     let model = &scene.models[0];
    //     assert!(
    //         model.size_x == BLOCK_SIZE && model.size_y == BLOCK_SIZE && model.size_z == BLOCK_SIZE
    //     );
    //     self.load_block_from_memory(block, model);
    // }

    // fn load_block(&mut self, block_id: Self::BlockId, model: &ogt_vox::VoxModel) {
    //     let size = uvec3::new(model.size_x, model.size_y, model.size_z);

    //     let mut padded_voxel_data = Array3D::<VoxelForContour<Self::Voxel>>::new(
    //         // +2 cause padding of 1 from each side
    //         (size.x + 2) as usize,
    //         (size.y + 2) as usize,
    //         (size.z + 2) as usize,
    //     );
    //     padded_voxel_data.data.fill(VoxelForContour(Zero::zero()));

    //     for xx in 0..size.x {
    //         for yy in 0..size.y {
    //             for zz in 0..size.z {
    //                 let voxel = <Self as LoadInterface>::Voxel::from(
    //                     model.voxel_data[(xx + yy * size.x + zz * size.x * size.y) as usize],
    //                 );
    //                 // some padding for generator
    //                 padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)] =
    //                     VoxelForContour(voxel);
    //             }
    //         }
    //     }

    //     // yep, there is padding. Its to reuse memory. TODO: find nicer approach
    //     assert!(size.x == BLOCK_SIZE && size.y == BLOCK_SIZE && size.z == BLOCK_SIZE);
    //     for zz in 0..size.z {
    //         for yy in 0..size.y {
    //             for xx in 0..size.x {
    //                 self.set_block_palette_voxels(
    //                     block_id.clone(),
    //                     uvec3::new(xx, yy, zz),
    //                     padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)]
    //                         .0
    //                         .clone(),
    //                 );
    //             }
    //         }
    //     }

    //     let triangles = self.make_contour_vertices(size, padded_voxel_data);

    //     self.set_block_palette_mesh(block_id, triangles);
    // }

    fn load_block(&mut self, block_id: Self::BlockId, mesh: BlockData);

    // fn set_block_palette_voxel(&mut self, block_id: Self::BlockId, pos: uvec3, voxel: Self::Voxel);
    // fn get_block_palette_voxel(&self, block_id: Self::BlockId, pos: uvec3) -> Self::Voxel;

    // fn set_block_palette_mesh(&mut self, block_id: Self::BlockId, mesh: Self::FaceBuffers);
    // fn get_block_palette_mesh(&self, block_id: Self::BlockId) -> &Self::InternalMeshBlock;

    fn create_rayrace_voxel_image(
        &mut self,
        voxels: &[Self::Voxel],
        size: uvec3,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> Self::Image;

    fn free_model(&mut self, mesh: Self::InternalMeshModel);
    fn free_block(&mut self, block: Self::BlockId);

    // fn extract_palette_from_scene(&mut self, scene: &ogt_vox::VoxScene);
    // fn has_palette(&self) -> bool;
    // fn set_has_palette(&mut self, has_palette: bool);
}
