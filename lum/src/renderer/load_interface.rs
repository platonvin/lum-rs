use block_mesh::{greedy_quads, GreedyQuadsBuffer};
use lumal::atrace;
use qvek::vek::num_traits::{One, Zero};

use crate::{
    containers::Array3D,
    renderer::types::{
        u8vec3, u8vec4, uvec3, vec3, FaceBuffers, IndexedVertices, InternalMeshBlock,
        InternalMeshModel, VoxelForContour,
    },
};

use super::ogt_vox;

pub trait LoadInterface {
    type BufferType;
    type ImageType;
    type BlockId: Clone;
    type MatId;
    type Voxel: Zero + One + Eq + Default + Clone + From<u8>;

    fn update_block_palette_to_gpu(&mut self);
    fn update_material_palette_to_gpu(&mut self);

    fn load_mesh_from_memory(
        &mut self,
        model: &ogt_vox::VoxModel,
        _make_vertices: bool,
    ) -> InternalMeshModel<Self::BufferType, Self::ImageType>;

    // i love that we can implement functions in traits
    #[cold]
    #[optimize(size)]
    fn load_mesh_from_file(
        &mut self,
        mesh_file: &str,
        _make_vertices: bool,
        extrude_palette: bool,
    ) -> InternalMeshModel<Self::BufferType, Self::ImageType> {
        let scene = ogt_vox::read_scene_from_file(mesh_file).unwrap();
        assert!(scene.models.len() == 1); // only one model per file supported for now
        let model = &scene.models[0];
        assert!(model.size_x > 0 && model.size_y > 0 && model.size_z > 0);

        if extrude_palette && !self.has_palette() {
            println!("Extruding palette");
            self.extract_palette_from_scene(&scene);
            self.set_has_palette(true);
        }

        self.load_mesh_from_memory(model, true)
    }

    #[cold]
    #[optimize(size)]
    fn load_meshes_from_file(
        &mut self,
        meshes_file: &str,
        _make_vertices: bool,
        extrude_palette: bool,
    ) -> Vec<InternalMeshModel<Self::BufferType, Self::ImageType>> {
        let scene = ogt_vox::read_scene_from_file(meshes_file).unwrap();

        if extrude_palette && !self.has_palette() {
            println!("Extruding palette");
            self.extract_palette_from_scene(&scene);
            self.set_has_palette(true);
        }

        scene
            .models
            .iter()
            .map(|model| {
                assert!(model.size_x > 0 && model.size_y > 0 && model.size_z > 0);

                self.load_mesh_from_memory(model, true)
            })
            .collect()
    }

    #[cold]
    #[optimize(size)]
    fn load_block_from_file(&mut self, block: Self::BlockId, path: &str) {
        let scene = ogt_vox::read_scene_from_file(path).unwrap();
        assert!(scene.models.len() == 1); // only one model per file supported for now
                                          // blocks are always 16x16x16
        let model = &scene.models[0];
        assert!(model.size_x == 16 && model.size_y == 16 && model.size_z == 16);
        self.load_block_from_memory(block, model);
    }

    #[cold]
    #[optimize(size)]
    fn load_block_from_memory(&mut self, block_id: Self::BlockId, model: &ogt_vox::VoxModel) {
        let size = uvec3::new(model.size_x, model.size_y, model.size_z);

        let mut padded_voxel_data = Array3D::<VoxelForContour<Self::Voxel>>::new(
            // +2 cause padding of 1 from each side
            (size.x + 2) as usize,
            (size.y + 2) as usize,
            (size.z + 2) as usize,
        );
        padded_voxel_data.data.fill(VoxelForContour(Zero::zero()));

        for xx in 0..size.x {
            for yy in 0..size.y {
                for zz in 0..size.z {
                    let voxel = <Self as LoadInterface>::Voxel::from(
                        model.voxel_data[(xx + yy * size.x + zz * size.x * size.y) as usize],
                    );
                    // some padding for generator
                    padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)] =
                        VoxelForContour(voxel);
                }
            }
        }

        // yep, there is padding. Its to reuse memory. TODO: find nicer approach
        assert!(size.x == 16 && size.y == 16 && size.z == 16);
        for zz in 0..size.z {
            for yy in 0..size.y {
                for xx in 0..size.x {
                    self.set_block_palette_voxels(
                        block_id.clone(),
                        uvec3::new(xx, yy, zz),
                        padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)]
                            .0
                            .clone(),
                    );
                }
            }
        }

        let triangles = self.make_contour_vertices(size, padded_voxel_data);

        self.set_block_palette_mesh(block_id, InternalMeshBlock { triangles });
    }

    fn set_block_palette_voxels(&mut self, block_id: Self::BlockId, pos: uvec3, voxel: Self::Voxel);
    fn get_block_palette_voxels(&self, block_id: Self::BlockId, pos: uvec3) -> Self::Voxel;

    fn set_block_palette_mesh(
        &mut self,
        block_id: Self::BlockId,
        mesh: InternalMeshBlock<Self::BufferType>,
    );
    fn get_block_palette_mesh(
        &self,
        block_id: Self::BlockId,
    ) -> &InternalMeshBlock<Self::BufferType>;

    #[cold]
    fn make_contour_vertices(
        &mut self,
        // real size. TODO: do i need this?
        size: uvec3,
        // 3d array with 1 padding
        padded_voxel_data: Array3D<VoxelForContour<Self::Voxel>>,
    ) -> FaceBuffers<Self::BufferType>;

    fn create_rayrace_voxel_image(
        &mut self,
        voxels: &[Self::Voxel],
        size: uvec3,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> Self::ImageType;

    fn free_mesh(&mut self, mesh: InternalMeshModel<Self::BufferType, Self::ImageType>);

    fn free_block(&mut self, block: Self::BlockId);

    fn extract_palette_from_scene(&mut self, scene: &ogt_vox::VoxScene);
    fn has_palette(&self) -> bool;
    fn set_has_palette(&mut self, has_palette: bool);
}
