pub mod all_resources;
pub mod gen_perlin_noise;
pub mod load;
pub mod render;
pub mod types;
pub mod wal;

use super::Settings;
use super::{Camera, SunLight};
use crate::webgpu::types::*;
use crate::{types::*, BLOCK_SIZE};
use containers::Array3D;
use containers::Ring;
use futures::executor;
use std::time::Instant;
use wal::{ComputePipe, Image, RasterPipe, Wal};
use wgpu::{Extent3d, TextureFormat};
use winit::window::Window;

const FRAME_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
const LIGHTMAPS_FORMAT: TextureFormat = TextureFormat::Depth32Float;
const MATNORM_FORMAT: TextureFormat = TextureFormat::Rgba8Uint;
const RADIANCE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
const BLOCK_PALETTE_SIZE_X: u32 = 64;
const BLOCK_PALETTE_SIZE_Y: u32 = 64;
const DEPTH_FORMAT_PREFERED: wgpu::TextureFormat = wgpu::TextureFormat::Depth32FloatStencil8;
static mut CHOSEN_DEPTH_FORMAT: Option<wgpu::TextureFormat> = Some(TextureFormat::Depth32Float);
static mut CHOSEN_STENCIL_FORMAT: Option<wgpu::TextureFormat> = Some(TextureFormat::Stencil8);

#[derive(Default)]
pub struct PipeWithPushConstants {
    pub pipe: RasterPipe,
    pub push_constants: Vec<u8>,
    pub pc_count: i32,
    pub pc_buffer: Option<wgpu::Buffer>,
    pub pc_bg: Option<wgpu::BindGroup>,
}

#[derive(Default)]
pub struct AllPipes {
    pub lightmap_blocks_pipe: RasterPipe,
    pub lightmap_models_pipe: RasterPipe,

    pub raygen_blocks_pipe: RasterPipe,
    pub raygen_models_pipe: RasterPipe,
    pub raygen_particles_pipe: RasterPipe,
    pub raygen_water_pipe: PipeWithPushConstants,
    pub raygen_foliage_pipes: Box<[PipeWithPushConstants]>,

    pub diffuse_pipe: RasterPipe,
    pub ao_pipe: RasterPipe,
    pub fill_stencil_glossy_pipe: RasterPipe,
    // smoke pipe goes hard
    // pc cause smoke is composed of a bunch of drawcalls rendering cubes
    pub fill_stencil_smoke_pipe: PipeWithPushConstants,
    pub glossy_pipe: RasterPipe,
    pub smoke_pipe: RasterPipe,
    pub tonemap_pipe: RasterPipe,

    pub radiance_pipe: ComputePipe,
    pub map_pipe: ComputePipe,

    pub update_grass_pipe: ComputePipe,
    pub update_water_pipe: ComputePipe,
    pub gen_perlin2d_pipe: ComputePipe,
    pub gen_perlin3d_pipe: ComputePipe,
}

pub struct AllSamplers {
    pub nearest_sampler: wgpu::Sampler,
    pub linear_sampler: wgpu::Sampler,
    pub linear_sampler_tiled: wgpu::Sampler,
    pub linear_sampler_tiled_mirrored: wgpu::Sampler,
    pub shadow_sampler: wgpu::Sampler,
    pub unnorm_linear: wgpu::Sampler,
}

// #[derive(Default)]
pub struct AllSwapchainDependentImages {
    pub frame: Image, // Ring equivalent will need careful management
    pub depth: Image,
    pub stencil: Image,
    pub mat_norm: Image,
    pub far_depth: Image,
    pub near_depth: Image,
}

pub struct AllIndependentImages {
    pub grass_state: Image, // Ring equivalent
    pub water_state: Image,
    pub perlin_noise2d: Image,
    pub perlin_noise3d: Image,
    pub world: Ring<Image>,
    pub radiance_cache: Ring<Image>,
    pub block_palette: Ring<Image>,
    pub material_palette: Image,
    pub lightmap: Image,
}

pub struct AllBuffers {
    // we dont need staging buffers since thats not how WGPU works
    pub staging_world: Ring<wgpu::Buffer>,
    pub light_uniform: Ring<wgpu::Buffer>,
    pub uniform: Ring<wgpu::Buffer>,
    pub ao_lut_uniform: Ring<wgpu::Buffer>,
    pub gpu_radiance_updates: Ring<wgpu::Buffer>,
    // we dont need staging buffers since thats not how WGPU works
    // pub staging_radiance_updates: Ring<wgpu::Buffer>,
    // pub gpu_particles_staged: Ring<wgpu::Buffer>,
    pub gpu_particles: Ring<wgpu::Buffer>,
}

pub struct InternalRendererWebGPU<'window> {
    pub wal: Wal<'window>,
    pub current_encoder: Option<wgpu::CommandEncoder>,
    pub counter: isize,
    pub settings: Settings,
    pub lightmap_extent: Extent3d,

    pub pipes: AllPipes,
    pub foliage_descriptions: Vec<MeshFoliageDesc>,
    pub dependent_images: Option<AllSwapchainDependentImages>,
    pub independent_images: AllIndependentImages,
    pub buffers: AllBuffers,
    pub samplers: AllSamplers,
    pub radiance_updates: Vec<ivec4>,
    pub special_radiance_updates: Vec<ivec4>,

    pub camera: Camera,
    pub light: SunLight,

    pub block_copies_queue: Vec<(
        wgpu::TexelCopyTextureInfo<'window>,
        wgpu::TexelCopyTextureInfo<'window>,
        wgpu::Extent3d,
    )>,
    pub block_clear_queue: Vec<wgpu::ImageSubresourceRange>,

    pub palette_counter: usize,
    pub static_block_palette_size: u32,

    pub origin_world: Array3D<InternalBlockId>,
    pub current_world: Array3D<InternalBlockId>,

    pub particles: Vec<Particle>,

    // we update it right before doing rendering to have most accurate timestamp
    pub delta_time: f32,
    pub last_time: Instant,

    pub has_palette: bool,
    pub material_palette: Vec<Material>,
    pub block_palette_voxels: Vec<BlockVoxels>,
    pub block_palette_meshes: Vec<InternalMeshBlock>,
}

impl<'window> InternalRendererWebGPU<'window> {
    /// Creates our InternalRendererWebGPU.
    ///
    /// The idea is similar to Vulkan version: we initialize a Wal instance,
    /// create our independent and dependent resources, and then fill our render‑state.
    pub fn new(
        lum_settings: &Settings,
        window: Window,
        foliage_descriptions: Vec<MeshFoliageDesc>,
    ) -> InternalRendererWebGPU<'window> {
        // 1. Create our Wal context (the WGPU abstraction layer)
        let mut wal = executor::block_on(wal::Wal::new(window));

        // 2. Define our lightmap extent. Here we create an Extent3d with 1024×1024 dimensions.
        let lightmap_extent = Extent3d {
            width: 1024,
            height: 1024,
            depth_or_array_layers: 1,
        };

        // 3. WGPU limits depth so its kinda 100% supported
        let _chosen_depth_format = DEPTH_FORMAT_PREFERED;

        // 4. Create independent resources (images/textures that persist across swapchain changes)
        let independent_images =
            InternalRendererWebGPU::create_independent_images(&wal, lum_settings);
        // 5. Create buffers, samplers, and command buffers.
        let buffers = InternalRendererWebGPU::create_all_buffers(&mut wal, lum_settings);
        let samplers = InternalRendererWebGPU::create_all_samplers(&wal);
        // let command_buffers = InternalRendererWebGPU::create_all_command_buffers(&wal);

        // 6. Create dependent resources (those that depend on the swapchain)
        let (dependent_images, pipes) = create_dependent(
            &wal,
            lum_settings,
            &foliage_descriptions,
            lum_settings, // In lieu of a separate lumal_settings object
            &independent_images,
            &buffers,
            &samplers,
        );

        // 7. Create scene-level objects: camera, light, and world data.
        let camera = Camera::default();
        let light = SunLight::default();

        let origin_world = Array3D::<InternalBlockId>::new(
            lum_settings.world_size.x as usize,
            lum_settings.world_size.y as usize,
            lum_settings.world_size.z as usize,
        );
        let current_world = Array3D::<InternalBlockId>::new(
            origin_world.x_size,
            origin_world.y_size,
            origin_world.z_size,
        );

        // 8. Assemble our renderer structure.
        let mut renderer = InternalRendererWebGPU {
            counter: 69420,
            wal,
            settings: Settings::default(),
            delta_time: 0.0,
            last_time: Instant::now(),

            // rpasses: renderpasses,
            // cmdbufs: command_buffers,
            lightmap_extent,
            pipes,
            independent_images,
            dependent_images: Some(dependent_images),
            buffers,
            samplers,
            camera,
            light,
            palette_counter: 0,
            static_block_palette_size: lum_settings.static_block_palette_size,
            origin_world,
            current_world,
            has_palette: false,
            radiance_updates: vec![],
            special_radiance_updates: vec![],
            particles: vec![],
            material_palette: vec![Material::default(); 256],
            block_palette_voxels: vec![
                [[[0; BLOCK_SIZE as usize]; BLOCK_SIZE as usize];
                    BLOCK_SIZE as usize];
                lum_settings.static_block_palette_size as usize
            ],
            block_copies_queue: vec![],
            block_clear_queue: vec![],
            foliage_descriptions,
            block_palette_meshes: (0..lum_settings.static_block_palette_size)
                .map(|_| Default::default())
                .collect(),
            current_encoder: None,
        };

        // 9. Generate perlin noise images (for grass, water, smoke, etc.)
        renderer.gen_perlin_noises();

        renderer
    }

    pub async fn new_async(
        lum_settings: &Settings,
        window: Window,
        foliage_descriptions: Vec<MeshFoliageDesc>,
    ) -> InternalRendererWebGPU<'window> {
        // 1. Create our Wal context (the WGPU abstraction layer)
        let mut wal = wal::Wal::new(window).await;

        // 2. Define our lightmap extent. Here we create an Extent3d with 1024×1024 dimensions.
        let lightmap_extent = Extent3d {
            width: 1024,
            height: 1024,
            depth_or_array_layers: 1,
        };

        // 3. WGPU limits depth so its kinda 100% supported
        let _chosen_depth_format = DEPTH_FORMAT_PREFERED;

        // 4. Create independent resources (images/textures that persist across swapchain changes)
        let independent_images =
            InternalRendererWebGPU::create_independent_images(&wal, lum_settings);
        // 5. Create buffers, samplers, and command buffers.
        let buffers = InternalRendererWebGPU::create_all_buffers(&mut wal, lum_settings);
        let samplers = InternalRendererWebGPU::create_all_samplers(&wal);
        // let command_buffers = InternalRendererWebGPU::create_all_command_buffers(&wal);

        // 6. Create dependent resources (those that depend on the swapchain)
        let (dependent_images, pipes) = create_dependent(
            &wal,
            lum_settings,
            &foliage_descriptions,
            lum_settings, // In lieu of a separate lumal_settings object
            &independent_images,
            &buffers,
            &samplers,
        );

        // 7. Create scene-level objects: camera, light, and world data.
        let camera = Camera::default();
        let light = SunLight::default();

        let origin_world = Array3D::<InternalBlockId>::new(
            lum_settings.world_size.x as usize,
            lum_settings.world_size.y as usize,
            lum_settings.world_size.z as usize,
        );
        let current_world = Array3D::<InternalBlockId>::new(
            origin_world.x_size,
            origin_world.y_size,
            origin_world.z_size,
        );

        // 8. Assemble our renderer structure.
        let mut renderer = InternalRendererWebGPU {
            counter: 69420,
            wal,
            settings: Settings::default(),
            delta_time: 0.0,
            last_time: Instant::now(),

            // rpasses: renderpasses,
            // cmdbufs: command_buffers,
            lightmap_extent,
            pipes,
            independent_images,
            dependent_images: Some(dependent_images),
            buffers,
            samplers,
            camera,
            light,
            palette_counter: 0,
            static_block_palette_size: lum_settings.static_block_palette_size,
            origin_world,
            current_world,
            has_palette: false,
            radiance_updates: vec![],
            special_radiance_updates: vec![],
            particles: vec![],
            material_palette: vec![Material::default(); 256],
            block_palette_voxels: vec![
                [[[0; BLOCK_SIZE as usize]; BLOCK_SIZE as usize];
                    BLOCK_SIZE as usize];
                lum_settings.static_block_palette_size as usize
            ],
            block_copies_queue: vec![],
            block_clear_queue: vec![],
            foliage_descriptions,
            block_palette_meshes: (0..lum_settings.static_block_palette_size)
                .map(|_| Default::default())
                .collect(),
            current_encoder: None,
        };

        // 9. Generate perlin noise images (for grass, water, smoke, etc.)
        renderer.gen_perlin_noises();

        renderer
    }

    /// Called when the window is resized. This method recreates dependent resources.
    pub fn recreate_window(&mut self, window: &Window) {
        // poll the device to make sure work is finished:
        self.wal.device.poll(wgpu::MaintainBase::Wait).unwrap();

        // 2. Reconfigure the surface (swapchain) using our Wal's resize method.
        self.wal.resize(window.inner_size());

        // 4. Recreate dependent resources.
        let settings_copy = self.settings;
        let (dimages, pipes) = create_dependent(
            &self.wal,
            &self.settings,
            &self.foliage_descriptions,
            &settings_copy,
            &self.independent_images,
            &self.buffers,
            &self.samplers,
        );
        self.dependent_images = Some(dimages);
        self.pipes = pipes;
        // self.rpasses = rpasses;
    }

    /// Destroys our renderer. In wgpu, resources are mostly cleaned up automatically
    pub fn destroy(self) {}
}

/// Create dependent resources (swapchain‐dependent images, pipelines, render passes)
fn create_dependent(
    wal: &wal::Wal,
    settings: &Settings,
    foliage_descriptions: &[MeshFoliageDesc],
    _lumal_settings: &Settings,
    independent_images: &AllIndependentImages,
    buffers: &AllBuffers,
    samplers: &AllSamplers,
) -> (AllSwapchainDependentImages, AllPipes) {
    let dependent_images = InternalRendererWebGPU::create_dependent_images(wal, settings);

    let pipes = InternalRendererWebGPU::create_all_pipes(
        wal,
        settings,
        buffers,
        independent_images,
        &dependent_images,
        samplers,
        foliage_descriptions,
    );

    (dependent_images, pipes)
}
