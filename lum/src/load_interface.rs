use crate::types::{ivec3, uvec3};
use crate::{sBLOCK_SIZE, BLOCK_SIZE};
use containers::Array3D;
use qvek::vek::num_traits::{One, Zero};

pub struct ModelData<'a, FaceIndicesTy, VoxelTy, VertexTy, IndexTy> {
    size: ivec3,
    iv: FaceIndicesTy,
    voxels: &'a [VoxelTy],
    vertices: &'a [VertexTy],
    indices: &'a [IndexTy],
}

pub struct BlockData<'a, FaceIndicesTy, VoxelTy, VertexTy, IndexTy> {
    iv: FaceIndicesTy,
    voxels: &'a [[[VoxelTy; sBLOCK_SIZE]; sBLOCK_SIZE]; sBLOCK_SIZE],
    vertices: &'a [VertexTy],
    indices: &'a [IndexTy],
}

// Lum is not going to parse any file format and create triangles, so you need to prepare them separately
// This is done this way for binary size and perfomance

pub trait LoadInterface {
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
    type Vertex: Clone + Copy;
    type Index: Clone + Copy;

    fn update_block_palette_to_gpu(&mut self);
    fn update_material_palette_to_gpu(&mut self);

    /// Loads mesh in specified format from provided memory
    fn load_model(
        &mut self,
        model: ModelData<Self::IndexedVertices, Self::Voxel, Self::Vertex, Self::Index>,
    ) -> Self::InternalMeshModel;

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

    fn load_block(&mut self, block_id: Self::BlockId, model: &[u8]) {}

    fn set_block_palette_voxels(&mut self, block_id: Self::BlockId, pos: uvec3, voxel: Self::Voxel);
    fn get_block_palette_voxels(&self, block_id: Self::BlockId, pos: uvec3) -> Self::Voxel;

    fn set_block_palette_mesh(&mut self, block_id: Self::BlockId, mesh: Self::FaceBuffers);
    fn get_block_palette_mesh(&self, block_id: Self::BlockId) -> &Self::InternalMeshBlock;

    fn make_contour_vertices(
        &mut self,
        // real size. TODO: do i need this?
        size: uvec3,
        // 3d array with 1 padding
        padded_voxel_data: Array3D<VoxelForContour<Self::Voxel>>,
    ) -> Self::FaceBuffers;

    fn create_rayrace_voxel_image(
        &mut self,
        voxels: &[Self::Voxel],
        size: uvec3,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> Self::Image;

    fn free_mesh(&mut self, mesh: Self::InternalMeshModel);

    fn free_block(&mut self, block: Self::BlockId);

    fn extract_palette_from_scene(&mut self, scene: &ogt_vox::VoxScene);
    fn has_palette(&self) -> bool;
    fn set_has_palette(&mut self, has_palette: bool);
}
