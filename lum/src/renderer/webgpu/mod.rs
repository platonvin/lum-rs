pub mod all_resources;
pub mod gen_perlin_noise;
pub mod load;
pub mod render;
pub mod types;
pub mod wal;

use std::future::IntoFuture;

use super::{Camera, SunLight};
use futures::executor;
use lumal::ring::Ring;
use wal::{ComputePipe, Image, RasterPipe, Wal};
use wgpu::{Extent3d, TextureFormat};
use winit::window::Window;

use super::Settings;
use crate::renderer::types::*;
use crate::{containers::Array3D, renderer::webgpu::types::*};

const FRAME_FORMAT: TextureFormat = TextureFormat::Rgb10a2Unorm;
const LIGHTMAPS_FORMAT: TextureFormat = TextureFormat::Depth32Float;
const MATNORM_FORMAT: TextureFormat = TextureFormat::Rgba8Uint;
const RADIANCE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
const SECONDARY_DEPTH_FORMAT: TextureFormat = TextureFormat::R16Float;
const BLOCK_PALETTE_SIZE_X: u32 = 64;
const BLOCK_PALETTE_SIZE_Y: u32 = 64;
const BLOCK_SIZE: u32 = 16;
const FRAMES_IN_FLIGHT: usize = 2;
const DEPTH_FORMAT_SPARE: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8; // TODO somehow D32 faster than wgpu::TextureFormat::D24_UNORM_S8_UINT on low-end
const DEPTH_FORMAT_PREFERED: wgpu::TextureFormat = wgpu::TextureFormat::Depth32FloatStencil8;
static mut CHOSEN_DEPTH_FORMAT: Option<wgpu::TextureFormat> =
    Some(TextureFormat::Depth32FloatStencil8); // TODO:
static mut SWAPCHAIN_FORMAT: Option<wgpu::TextureFormat> = None;

#[derive(Default)]
pub struct AllPipes {
    pub lightmap_blocks_pipe: RasterPipe,
    pub lightmap_models_pipe: RasterPipe,

    pub raygen_blocks_pipe: RasterPipe, // Or ComputePipeline if it's ray tracing
    pub raygen_models_pipe: RasterPipe, // Or ComputePipeline
    // pub raygen_models_push_layout: Option<wgpu::BindGroupLayout>, // Equivalent to DescriptorSetLayout
    pub raygen_particles_pipe: RasterPipe,
    pub raygen_water_pipe: RasterPipe,
    pub raygen_foliage_pipes: Vec<RasterPipe>, // Or ComputePipeline

    pub diffuse_pipe: RasterPipe,
    pub ao_pipe: RasterPipe,
    pub fill_stencil_glossy_pipe: RasterPipe,
    pub fill_stencil_smoke_pipe: RasterPipe,
    pub glossy_pipe: RasterPipe,
    pub smoke_pipe: RasterPipe,
    pub tonemap_pipe: RasterPipe,
    // pub overlay_pipe: RasterPipe,
    pub radiance_pipe: ComputePipe,
    pub map_pipe: ComputePipe,
    // pub map_push_layout: Option<wgpu::BindGroupLayout>,
    pub update_grass_pipe: ComputePipe,
    pub update_water_pipe: ComputePipe,
    pub gen_perlin2d_pipe: ComputePipe,
    pub gen_perlin3d_pipe: ComputePipe,
}

pub struct AllSamplers {
    pub nearest_sampler: Option<wgpu::Sampler>,
    pub linear_sampler: Option<wgpu::Sampler>,
    pub linear_sampler_tiled: Option<wgpu::Sampler>,
    pub linear_sampler_tiled_mirrored: Option<wgpu::Sampler>,
    pub overlay_sampler: Option<wgpu::Sampler>,
    pub shadow_sampler: Option<wgpu::Sampler>,
    pub unnorm_linear: Option<wgpu::Sampler>,
    pub unnorm_nearest: Option<wgpu::Sampler>,
}

// #[derive(Default)]
pub struct AllSwapchainDependentImages {
    pub highres_frame: Ring<Image>, // Ring equivalent will need careful management
    pub highres_depth_stencil: Ring<Image>,
    pub highres_mat_norm: Ring<Image>,
    pub full_view_for_ds: Ring<wgpu::TextureView>, // Consider if this is needed in WebGPU
    pub stencil_view_for_ds: Ring<wgpu::TextureView>, // Consider if this is needed in WebGPU
    pub far_depth: Ring<Image>,
    pub near_depth: Ring<Image>,
}

pub struct AllIndependentImages {
    pub grass_state: Ring<Image>, // Ring equivalent
    pub water_state: Ring<Image>,
    pub perlin_noise2d: Ring<Image>,
    pub perlin_noise3d: Ring<Image>,
    pub world: Ring<Image>,
    pub radiance_cache: Ring<Image>,
    pub origin_block_palette: Ring<Image>,
    pub material_palette: Ring<Image>,
    pub lightmap: Ring<Image>,
}

pub struct AllBuffers {
    // we dont need staging buffers since thats not how WGPU works
    pub staging_world: Ring<wgpu::Buffer>,
    pub light_uniform: Ring<wgpu::Buffer>,
    pub uniform: Ring<wgpu::Buffer>,
    pub ao_lut_uniform: Ring<wgpu::Buffer>,
    pub gpu_radiance_updates: Ring<wgpu::Buffer>,
    // we dont need staging buffers since thats not how WGPU works
    pub staging_radiance_updates: Ring<wgpu::Buffer>,
    pub gpu_particles_staged: Ring<wgpu::Buffer>,
    pub gpu_particles: Ring<wgpu::Buffer>,
}

#[pub_fields::pub_fields]
pub struct InternalRendererWebGPU<'window> {
    wal: Wal<'window>,
    current_encoder: Option<wgpu::CommandEncoder>,
    counter: isize,
    settings: Settings,
    lightmap_extent: Extent3d,

    pipes: AllPipes,
    foliage_descriptions: Vec<MeshFoliageDesc>,
    dependent_images: Option<AllSwapchainDependentImages>,
    // rpasses: AllRenderPasses,
    independent_images: AllIndependentImages,
    buffers: AllBuffers,
    samplers: AllSamplers,
    // cmdbufs: AllCommandBuffers,
    radiance_updates: Vec<ivec4>,
    special_radiance_updates: Vec<ivec4>,

    camera: Camera,
    light: SunLight,

    block_copies_queue: Vec<(
        wgpu::TexelCopyTextureInfo<'window>,
        wgpu::TexelCopyTextureInfo<'window>,
        wgpu::Extent3d,
    )>,
    block_clear_queue: Vec<wgpu::ImageSubresourceRange>,

    palette_counter: usize,
    static_block_palette_size: u32,

    origin_world: Array3D<BlockId>,
    current_world: Array3D<BlockId>,

    particles: Vec<Particle>,

    delta_time: f32,

    has_palette: bool,
    material_palette: Vec<Material>,
    block_palette_voxels: Vec<BlockVoxels<Voxel>>,
    block_palette_meshes: Vec<InternalMeshBlock<Option<wgpu::Buffer>>>, // Adjust buffer type
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

        let origin_world = Array3D::<BlockId>::new(
            lum_settings.world_size.x as usize,
            lum_settings.world_size.y as usize,
            lum_settings.world_size.z as usize,
        );
        let current_world = Array3D {
            data: origin_world.data.clone(),
            x_size: origin_world.x_size,
            y_size: origin_world.y_size,
            z_size: origin_world.z_size,
        };

        // 8. Assemble our renderer structure.
        let mut renderer = InternalRendererWebGPU {
            counter: 69420,
            wal,
            settings: Settings::default(),
            delta_time: 0.0,

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
                [[[0; 16]; 16]; 16];
                lum_settings.static_block_palette_size as usize
            ],
            block_copies_queue: vec![],
            block_clear_queue: vec![],
            foliage_descriptions,
            block_palette_meshes: (0..lum_settings.static_block_palette_size)
                .map(|_| InternalMeshBlock::<Option<wgpu::Buffer>>::default())
                .collect(),
            current_encoder: None,
        };

        // 9. Generate perlin noise images (for grass, water, smoke, etc.)
        // renderer.gen_perlin_2d();
        // renderer.gen_perlin_3d();

        renderer
    }

    /// Called when the window is resized. This method recreates dependent resources.
    pub fn recreate_window(&mut self, window: &Window) {
        // poll the device to make sure work is finished:
        self.wal.device.poll(wgpu::Maintain::Wait);

        // 2. Reconfigure the surface (swapchain) using our Wal's resize method.
        self.wal.resize(window.inner_size());

        // 4. Recreate dependent resources.
        let settings_copy = self.settings.clone();
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
    foliage_descriptions: &Vec<MeshFoliageDesc>,
    lumal_settings: &Settings,
    independent_images: &AllIndependentImages,
    buffers: &AllBuffers,
    samplers: &AllSamplers,
) -> (AllSwapchainDependentImages, AllPipes) {
    let mut dependent_images = InternalRendererWebGPU::create_dependent_images(wal, settings);
    // let mut pipes: AllPipes = AllPipes::default();
    // pipes
    //     .raygen_foliage_pipes
    //     .resize_with(foliage_descriptions.len(), || RasterPipe::default());

    let pipes = unsafe {
        InternalRendererWebGPU::create_all_pipes(
            wal,
            settings,
            buffers,
            independent_images,
            &dependent_images,
            samplers,
            foliage_descriptions,
        )
    };
    (dependent_images, pipes)
}
