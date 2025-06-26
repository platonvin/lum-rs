use super::types::*;
use crate::{
    fBLOCK_SIZE,
    load_interface::{BlockData, ModelData},
    BLOCK_SIZE,
};
use containers::array3d::{Array3DView, Array3DViewMut, Dim3};
use qvek::{vec3, vek::FrustumPlanes};
use winit::window::Window;

// i am clearly trash with managing division into files
// if someone has a good idea on how to do it, message me (or just make a PR)

/// Single isometric camera with some precomputed values for shaders
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub camera_pos: vec3,
    pub camera_dir: vec3,
    pub camera_transform: mat4,
    pub pixels_in_voxel: f32,
    pub origin_view_size: vec2,
    pub view_size: vec2, // in voxels
    pub camera_ray_dir_plane: vec3,
    pub horizline: vec3,
    pub vertiline: vec3,
}
impl Default for Camera {
    fn default() -> Self {
        // magic values for what works in the demo
        // TODO:?
        let origin_view_size = qvek::vec2!(1920, 1080);
        let pixels_in_voxel = 5.0;
        let camera_dir = vec3!(0.61, 1.0, -0.8).normalized();
        let camera_ray_dir_plane = vec3!(camera_dir.xy(), 0).normalized();
        let horizline = camera_ray_dir_plane.cross(vec3!(0, 0, 1)).normalized();

        Self {
            camera_pos: vec3!(60, 0, 194),
            camera_dir,
            camera_transform: mat4::identity(),
            pixels_in_voxel,
            origin_view_size,
            view_size: origin_view_size / pixels_in_voxel,
            camera_ray_dir_plane,
            horizline,
            vertiline: horizline.cross(camera_dir).normalized(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SunLight {
    pub light_transform: mat4,
    pub light_dir: vec3,
}
impl Default for SunLight {
    fn default() -> Self {
        Self {
            light_transform: Default::default(),
            light_dir: vec3!(0.5, 0.5, -0.9).normalized(),
        }
    }
}

impl Camera {
    fn update_camera(&mut self) {
        let up = vec3!(0, 0, 1); // Up vector
        self.view_size = self.origin_view_size / self.pixels_in_voxel;
        // RIGHT HANDED MATH EVERYWHERE
        let view = mat4::look_at_rh(self.camera_pos, self.camera_pos + self.camera_dir, up);
        let projection = mat4::orthographic_rh_no(FrustumPlanes {
            left: -self.view_size.x / 2.0,
            right: self.view_size.x / 2.0,
            bottom: self.view_size.y / 2.0,
            top: -self.view_size.y / 2.0,
            near: -0.0,
            far: 2000.0,
        }); // => *(2000.0/2) for decoding
            // dbg!(&projection);
        self.camera_transform = projection * view;
        self.camera_ray_dir_plane = vec3!(self.camera_dir.xy(), 0).normalized();

        self.horizline = self.camera_ray_dir_plane.cross(vec3!(0, 0, 1)).normalized();

        self.vertiline = self.camera_dir.cross(self.horizline).normalized();
    }
}

impl SunLight {
    fn update_light_transform(&mut self, world_size: uvec3) {
        let _horizon = vec3!(1, 0, 0).normalized();
        let up = vec3!(0, 0, 1).normalized();
        let light_pos =
            vec3!(world_size.xy() * BLOCK_SIZE, 0) / 2.0 - (1.0 * fBLOCK_SIZE * self.light_dir);

        let view = mat4::look_at_rh(light_pos, light_pos + self.light_dir, up);
        let voxel_in_pixels = 5.0;
        let view_width_in_voxels = 3000.0 / voxel_in_pixels;
        let view_height_in_voxels = 3000.0 / voxel_in_pixels;
        let projection = mat4::orthographic_rh_no(FrustumPlanes {
            left: -view_width_in_voxels / 2.0,
            right: view_width_in_voxels / 2.0,
            bottom: view_height_in_voxels / 2.0,
            top: -view_height_in_voxels / 2.0,
            near: -512.0,
            far: 1024.0,
        });
        self.light_transform = projection * view;
    }
}

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
    fn get_world_blocks(&self) -> Array3DView<Self::InternalBlockId, MeshBlock, D>;
    /// Returns mutable reference to 3d array of "origin" world blocks - static blocks in the world, not allocated ones.
    fn get_world_blocks_mut(&mut self) -> Array3DViewMut<Self::InternalBlockId, MeshBlock, D>;

    //TODO: arrays vs images?

    fn get_block_palette(&self) -> &[BlockVoxels];
    fn get_block_palette_mut(&mut self) -> &mut [BlockVoxels];

    fn get_material_palette(&self) -> &[Material];
    fn get_material_palette_mut(&mut self) -> &mut [Material];
}
