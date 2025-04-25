use block_mesh::{greedy_quads, GreedyQuadsBuffer, VoxelVisibility};
use qvek::{vec3, vek::Vec3};
// use rand::Rng;
use crate::{
    containers::Array3D,
    internal_renderer::{
        load_interface::LoadInterface,
        render_gl::{BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y, FRAMES_IN_FLIGHT},
        render_interface::LumRendererAPI,
    },
    types::*,
    *,
};

use super::InternalRendererGL;

impl LoadInterface for InternalRendererGL {
    type BufferType = Option<glow::Buffer>;
    type ImageType = Option<glow::Texture>;

    fn update_block_palette_to_gpu(&mut self) {
        todo!()
    }

    fn update_material_palette_to_gpu(&mut self) {
        todo!()
    }

    fn load_mesh_from_memory(
        &mut self,
        model: &internal_renderer::ogt_vox::VoxModel,
        _make_vertices: bool,
    ) -> InternalMeshModel<Self::BufferType, Self::ImageType> {
        todo!()
    }

    fn extract_palette_from_scene(&mut self, scene: &internal_renderer::ogt_vox::VoxScene) {
        todo!()
    }

    fn has_palette(&self) -> bool {
        todo!()
    }

    fn set_has_palette(&mut self, has_palette: bool) {
        todo!()
    }

    fn set_block_palette_voxels(&mut self, block_id: BlockId, pos: uvec3, voxel: Voxel) {
        todo!()
    }

    fn get_block_palette_voxels(&self, block_id: BlockId, pos: uvec3) -> Voxel {
        todo!()
    }

    fn set_block_palette_mesh(
        &mut self,
        block_id: BlockId,
        mesh: InternalMeshBlock<Self::BufferType>,
    ) {
        todo!()
    }

    fn get_block_palette_mesh(&self, block_id: BlockId) -> &InternalMeshBlock<Self::BufferType> {
        todo!()
    }

    fn create_and_upload_contour_buffers(
        &mut self,
        verts: &[PackedVoxelCircuit],
        indices: &[u16],
    ) -> (Self::BufferType, Self::BufferType) {
        todo!()
    }

    fn create_rayrace_voxel_image(
        &mut self,
        voxels: &[Voxel],
        size: uvec3,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> Self::ImageType {
        todo!()
    }

    fn free_mesh(&mut self, mesh: InternalMeshModel<Self::BufferType, Self::ImageType>) {
        todo!()
    }

    fn free_block(&mut self, block: crate::types::BlockId) {
        todo!()
    }
}
