use super::types::*;
use crate::{
    fBLOCK_SIZE,
    load_interface::{BlockData, ModelData},
    BLOCK_SIZE,
};
use containers::{
    array3d::{Array3DView, Array3DViewMut},
    Array3D,
};
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
    light_transform: mat4,
    light_dir: vec3,
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

pub trait FoliageDescriptionCreate {
    fn new(name: &str, vertices: usize, dencity: usize) -> Self;
}

// not over Vulkan, but over Lum needs
// this is sync, async is automatically implemented by Rust
pub trait RendererInterface {
    type FoliageDescription: FoliageDescriptionCreate;
    type FoliageDescriptionBuilder: FoliageDescriptionBuilder<Self::FoliageDescription>;

    type InternalBlockId: From<MeshBlock>;

    fn new(
        settings: &super::Settings,
        window: Window,
        foliage: &[Self::FoliageDescription],
    ) -> Self;
    async fn new_async(
        settings: &super::Settings,
        window: Window,
        foliages: &[Self::FoliageDescription],
    ) -> Self;

    fn destroy(self);

    fn load_model(&mut self, model_data: ModelData) -> MeshModel;
    fn unload_model(&mut self, model: MeshModel);
    fn get_model_size(&self, model: MeshModel) -> uvec3;

    fn load_block(&mut self, block: MeshBlock, block_data: BlockData);
    fn unload_block(&mut self, block: MeshBlock);

    fn update_block_palette_to_gpu(&mut self);
    fn update_material_palette_to_gpu(&mut self);

    fn load_volumetric(
        &mut self,
        max_density: f32,
        dencity_variation: f32,
        color: u8vec3,
    ) -> MeshVolumetric;
    fn unload_volumetric(&mut self, volumetric: MeshVolumetric);

    fn load_liquid(&mut self, main_mat: MatId, foam_mat: MatId) -> MeshLiquid;
    fn unload_liquid(&mut self, liquid: MeshLiquid);

    // fn load foliage
    fn unload_foliage(&mut self, foliage: MeshFoliage);

    fn start_frame(&mut self);
    fn prepare_frame(&mut self);
    fn end_frame(&mut self);

    fn is_block_visible(&self, pos: vec3) -> bool;
    fn is_model_visible(&self, model_size: &uvec3, trans: &MeshTransform) -> bool;

    fn draw_world(&mut self);
    fn draw_block(&mut self, block: MeshBlock, block_pos: &i16vec3);
    fn draw_model(&mut self, model: &MeshModel, trans: &MeshTransform);
    fn draw_foliage(&mut self, foliage: &MeshFoliage, pos: &vec3);
    fn draw_liquid(&mut self, liquid: &MeshLiquid, pos: &vec3);
    fn draw_volumetric(&mut self, volumetric: &MeshVolumetric, pos: &vec3);
    fn spawn_particle(&mut self, particle: &Particle);

    fn get_world_blocks(&self) -> Array3DView<Self::InternalBlockId, MeshBlock>;
    fn get_world_blocks_mut(&mut self) -> Array3DViewMut<Self::InternalBlockId, MeshBlock>;

    //TODO: arrays vs images?

    fn get_block_palette(&self) -> &[BlockVoxels];
    fn get_block_palette_mut(&mut self) -> &mut [BlockVoxels];

    fn get_material_palette(&self) -> &[Material];
    fn get_material_palette_mut(&mut self) -> &mut [Material];
}
