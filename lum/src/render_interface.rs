use super::types::*;
use crate::load_interface::{BlockData, ModelData};
use containers::array3d::{Array3DView, Array3DViewMut, Dim3};
use winit::window::Window;

// i am clearly trash with managing division into files
// if someone has a good idea on how to do it, message me (or just make a PR)

pub trait FoliageDescriptionBuilder<FoliageDescType> {
    fn new() -> Self;
    fn load_foliage(&mut self, foliage_desc: FoliageDescType) -> MeshFoliage;
    fn build(self) -> Vec<FoliageDescType>;
}

/// Represents a (compiled from GLSL for Vulkan) shader source.
pub enum ShaderSource<'a> {
    /// SPIR-V binary data for Vulkan (compiled from GLSL).
    SpirV(&'a [u8]),
    /// WGSL string for WebGPU.
    Wgsl(&'a str),
}

pub trait FoliageDescriptionCreate<'a> {
    fn new(code: ShaderSource<'a>, vertices: usize, dencity: usize) -> Self;
}

// not over Vulkan, but over Lum needs
pub trait RendererInterface<'a, D: Dim3> {
    type FoliageDescription: FoliageDescriptionCreate<'a>;
    type FoliageDescriptionBuilder: FoliageDescriptionBuilder<Self::FoliageDescription>;

    type InternalBlockId: From<MeshBlock>;

    /// Constructs new Renderer.
    fn new(
        settings: &super::Settings<D>,
        window: std::sync::Arc<Window>,
        size: winit::dpi::PhysicalSize<u32>,
        foliage: &[Self::FoliageDescription],
    ) -> Self;

    /// Constructs new Renderer (async).
    fn new_async(
        settings: &super::Settings<D>,
        window: std::sync::Arc<Window>,
        size: winit::dpi::PhysicalSize<u32>,
        foliages: &[Self::FoliageDescription],
    ) -> impl std::future::Future<Output = Self>;

    /// Destroys the Renderer.
    fn destroy(self);

    /// Makes MeshModel from given ModelData. Allocates & copies to GPU resources.
    fn load_model(&mut self, model_data: ModelData) -> MeshModel;
    /// Destroys MeshModel and its GPU resources.
    fn unload_model(&mut self, model: MeshModel);
    fn get_model_size(&self, model: MeshModel) -> uvec3;

    /// Sets specified block mesh data to provided one (creates GPU resources for it).
    fn load_block(&mut self, block: MeshBlock, block_data: BlockData);
    /// Destroys GPU resources for block mesh data.
    fn unload_block(&mut self, block: MeshBlock);

    /// Copies CPU block palette data to GPU.
    fn update_block_palette_to_gpu(&mut self);
    /// Copies CPU material palette data to GPU.
    fn update_material_palette_to_gpu(&mut self);

    /// Makes a MeshVolumetric from given properties.
    fn load_volumetric(
        &mut self,
        max_density: f32,
        dencity_variation: f32,
        color: u8vec3,
    ) -> MeshVolumetric;
    /// Destroys a MeshVolumetric.
    fn unload_volumetric(&mut self, volumetric: MeshVolumetric);

    /// Makes a MeshLiquid from given properties.
    fn load_liquid(&mut self, main_mat: MatId, foam_mat: MatId) -> MeshLiquid;
    /// Destroys a MeshLiquid.
    fn unload_liquid(&mut self, liquid: MeshLiquid);

    /// Destroys a MeshFoliage.
    fn unload_foliage(&mut self, foliage: MeshFoliage);

    /// Enters the phase when draw_thing() calls are valid.
    fn start_frame(&mut self);
    /// (potentially) CPU-heavy work that should be done before end_frame.
    /// Currently it sorts draw requests by depth for both backends.
    fn prepare_frame(&mut self);
    /// Actually submits work to GPU (along with some CPU computations). Will wait until second-to-last* frame finishes GPU work
    /// * depends on your FIF count. It basically waits for `current()` fence in Ring (with FIF len) of fences.
    fn end_frame(&mut self);

    /// Waits until idle and recreates swapchain and all swapchain dependent resources (with new size).
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>);

    // TODO: lightmaps, more precise culiling
    /// Returns if any of corners of a block appear on camera.
    fn is_block_visible(&self, pos: vec3) -> bool;
    /// Returns if any of corners of a model appear on camera.
    fn is_model_visible(&self, model_size: &uvec3, trans: &MeshTransform) -> bool;

    /// Draws all the static blocks in the (origin) world.
    fn draw_world(&mut self);
    /// Draws given block at given block position (snapped to a block grid).
    fn draw_block(&mut self, block: MeshBlock, block_pos: &i16vec3);
    /// Draws given model with given transformation (position, rotation).
    fn draw_model(&mut self, model: &MeshModel, trans: &MeshTransform);
    fn draw_foliage(&mut self, foliage: &MeshFoliage, pos: &vec3);
    fn draw_liquid(&mut self, liquid: &MeshLiquid, pos: &vec3);
    fn draw_volumetric(&mut self, volumetric: &MeshVolumetric, pos: &vec3);
    /// Creates Particle (location, lifetime and other properties are _part_ of Particle)
    fn spawn_particle(&mut self, particle: &Particle);

    /// Returns reference to 3d array of "origin" world blocks - static blocks in the world, not allocated ones.
    fn get_world_blocks(&'_ self) -> Array3DView<'_, Self::InternalBlockId, MeshBlock, D>;
    /// Returns mutable reference to 3d array of "origin" world blocks - static blocks in the world, not allocated ones.
    fn get_world_blocks_mut(
        &'_ mut self,
    ) -> Array3DViewMut<'_, Self::InternalBlockId, MeshBlock, D>;

    //TODO: arrays vs images?

    fn get_block_palette(&self) -> &[BlockVoxels];
    fn get_block_palette_mut(&mut self) -> &mut [BlockVoxels];

    fn get_material_palette(&self) -> &[Material];
    fn get_material_palette_mut(&mut self) -> &mut [Material];

    fn get_counter(&self) -> isize;
    fn get_time(&self) -> std::time::Instant;
    fn get_dt(&self) -> f32;
}
