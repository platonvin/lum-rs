use crate::internal_renderer::render_interface::LumRendererAPI;

use super::InternalRendererGL;

impl LumRendererAPI for InternalRendererGL {
    type BufferType = Option<glow::Buffer>;
    type ImageType = Option<glow::Texture>;

    fn new(
        lum_settings: &crate::internal_renderer::Settings,
        window: &winit::window::Window,
        foliage_descriptions: Vec<crate::types::InternalMeshFoliageDesc>,
    ) -> Self {
        todo!()
    }

    fn update_camera(&mut self) {
        todo!()
    }

    fn update_light_transform(&mut self) {
        todo!()
    }

    fn start_blockify(&mut self) {
        todo!()
    }

    fn index_block_xy(&self, n: usize) -> crate::types::uvec2 {
        todo!()
    }

    fn blockify_mesh(
        &mut self,
        mesh: &crate::types::InternalMeshModel<Self::BufferType, Self::ImageType>,
        trans: &crate::types::MeshTransform,
    ) {
        todo!()
    }

    fn end_blockify(&mut self) {
        todo!()
    }

    fn find_radiance_to_update(&mut self) {
        todo!()
    }

    fn update_radiance(&mut self) {
        todo!()
    }

    fn start_frame(&mut self) {
        todo!()
    }

    fn _update_radiance(&mut self) {
        todo!()
    }

    fn shift_radiance(&mut self, radiance_shift: crate::types::ivec3) {
        todo!()
    }

    fn exec_copies(&mut self) {
        todo!()
    }

    fn start_map(&mut self) {
        todo!()
    }

    fn map_mesh(
        &mut self,
        mesh: &crate::types::InternalMeshModel<Self::BufferType, Self::ImageType>,
        trans: &crate::types::MeshTransform,
    ) {
        todo!()
    }

    fn end_map(&mut self) {
        todo!()
    }

    fn end_compute(&mut self) {
        todo!()
    }

    fn start_lightmap(&mut self) {
        todo!()
    }

    fn lightmap_start_blocks(&mut self) {
        todo!()
    }

    fn lightmap_start_models(&mut self) {
        todo!()
    }

    fn end_lightmap(&mut self) {
        todo!()
    }

    fn start_raygen(&mut self) {
        todo!()
    }

    fn raygen_start_blocks(&mut self) {
        todo!()
    }

    fn is_face_visible(&self, normal: crate::types::vec3, camera_dir: crate::types::vec3) -> bool {
        todo!()
    }

    fn raygen_block_face(
        &self,
        normal: crate::types::ivec3,
        buff: &crate::types::IndexedVertices,
        block_id: crate::types::BlockId,
    ) {
        todo!()
    }

    fn raygen_block(&mut self, block_id: crate::types::BlockId, shift: crate::types::ivec3) {
        todo!()
    }

    fn raygen_start_models(&mut self) {
        todo!()
    }

    fn raygen_model_face(
        &mut self,
        normal: crate::types::vec3,
        buff: &crate::types::IndexedVertices,
    ) {
        todo!()
    }

    fn raygen_model(
        &mut self,
        model_mesh: &crate::types::InternalMeshModel<Self::BufferType, Self::ImageType>,
        model_trans: &crate::types::MeshTransform,
    ) {
        todo!()
    }

    fn lightmap_block_face(
        &self,
        _normal: crate::types::ivec3,
        buff: &crate::types::IndexedVertices,
        _block_id: crate::types::BlockId,
    ) {
        todo!()
    }

    fn lightmap_block(&mut self, block_id: crate::types::BlockId, shift: crate::types::ivec3) {
        todo!()
    }

    fn lightmap_model_face(
        &mut self,
        _normal: crate::types::vec3,
        buff: &crate::types::IndexedVertices,
    ) {
        todo!()
    }

    fn lightmap_model(
        &mut self,
        model_mesh: &crate::types::InternalMeshModel<Self::BufferType, Self::ImageType>,
        model_trans: &crate::types::MeshTransform,
    ) {
        todo!()
    }

    fn update_particles(&mut self) {
        todo!()
    }

    fn raygen_map_particles(&mut self) {
        todo!()
    }

    fn raygen_start_grass(&mut self) {
        todo!()
    }

    fn updade_grass(&mut self, wind_direction: crate::types::vec2) {
        todo!()
    }

    fn updade_water(&mut self) {
        todo!()
    }

    fn raygen_map_grass(
        &mut self,
        grass: &crate::types::InternalMeshFoliage,
        pos: &crate::types::vec3,
    ) {
        todo!()
    }

    fn raygen_start_water(&mut self) {
        todo!()
    }

    fn raygen_map_water(
        &mut self,
        _water: &crate::types::InternalMeshLiquid,
        pos: &crate::types::vec3,
    ) {
        todo!()
    }

    fn end_raygen(&mut self) {
        todo!()
    }

    fn start_2nd_spass(&mut self) {
        todo!()
    }

    fn diffuse(&mut self) {
        todo!()
    }

    fn ambient_occlusion(&mut self) {
        todo!()
    }

    fn glossy_raygen(&mut self) {
        todo!()
    }

    fn raygen_start_smoke(&mut self) {
        todo!()
    }

    fn raygen_map_smoke(
        &mut self,
        _smoke: &crate::types::InternalMeshVolumetric,
        pos: &crate::types::vec3,
    ) {
        todo!()
    }

    fn smoke(&mut self) {
        todo!()
    }

    fn glossy(&mut self) {
        todo!()
    }

    fn tonemap(&mut self) {
        todo!()
    }

    fn end_2nd_spass(&mut self) {
        todo!()
    }

    fn end_frame(&mut self, window: &winit::window::Window) {
        todo!()
    }
}
