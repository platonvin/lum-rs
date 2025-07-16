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
use containers::array3d::Dim3;
use containers::Array3D;
use containers::Ring;
use futures::executor;
use wal::{ComputePipe, Image, RasterPipe, Wal};
use wgpu::{Extent3d, TextureFormat};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use web_time::Instant;

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

pub struct InternalRendererWebGPU<'window, D: Dim3> {
    pub wal: Wal<'window>,
    pub current_encoder: Option<wgpu::CommandEncoder>,
    pub counter: isize,
    pub settings: Settings<D>,
    pub lightmap_extent: Extent3d,

    pub pipes: AllPipes,
    // foliage descriptions live as long as renderer ('window)
    pub foliage_descriptions: Vec<MeshFoliageDesc<'window>>,
    pub dependent_images: Option<AllSwapchainDependentImages>,
    pub independent_images: AllIndependentImages,
    pub buffers: AllBuffers,
    pub samplers: AllSamplers,
    pub radiance_updates: Vec<ivec4>,
    pub special_radiance_updates: Vec<ivec4>,

    pub camera: Camera,
    pub light: SunLight,

    pub block_copies_queue: Vec<(wgpu::Origin3d, wgpu::Origin3d)>,
    pub block_clear_queue: Vec<wgpu::Origin3d>,

    pub palette_counter: usize,
    pub static_block_palette_size: u32,

    pub origin_world: Array3D<InternalBlockId, D>,
    pub current_world: Array3D<InternalBlockId, D>,

    pub particles: Vec<Particle>,

    // we update it right before doing rendering to have most accurate timestamp
    pub delta_time: f32,
    pub last_time: Instant,

    pub has_palette: bool,
    pub material_palette: Vec<Material>,
    pub block_palette_voxels: Vec<BlockVoxels>,
    pub block_palette_meshes: Vec<InternalMeshBlock>,
}

#[derive(Debug, Clone)]
pub struct IndicesSnapshot {
    pub staging_world: usize,
    pub light_uniform: usize,
    pub uniform: usize,
    pub ao_lut_uniform: usize,
    pub gpu_radiance_updates: usize,
    pub gpu_particles: usize,

    pub world_image: usize,
    pub radiance_cache_image: usize,
    pub block_palette_image: usize,

    pub lightmap_blocks_bg: Option<usize>,
    pub lightmap_models_bg: Option<usize>,
    pub raygen_blocks_bg: Option<usize>,
    pub raygen_models_bg: Option<usize>,
    pub raygen_particles_bg: Option<usize>,
    pub raygen_water_bg: Option<usize>,
    pub diffuse_bg: Option<usize>,
    pub ao_bg: Option<usize>,
    pub fill_stencil_glossy_bg: Option<usize>,
    pub fill_stencil_smoke_bg: Option<usize>,
    pub glossy_bg: Option<usize>,
    pub smoke_bg: Option<usize>,
    pub tonemap_bg: Option<usize>,
    pub radiance_bg: Option<usize>,
    pub map_bg: Option<usize>,
    pub update_grass_bg: Option<usize>,
    pub update_water_bg: Option<usize>,
    pub gen_perlin2d_bg: Option<usize>,
    pub gen_perlin3d_bg: Option<usize>,

    pub raygen_foliage_bg: Vec<usize>,
}

impl<'window, D: Dim3> InternalRendererWebGPU<'window, D> {
    /// Creates our InternalRendererWebGPU.
    ///
    /// The idea is similar to Vulkan version: we initialize a Wal instance,
    /// create our independent and dependent resources, and then fill our render‑state.
    pub fn new(
        lum_settings: &Settings<D>,
        window: std::sync::Arc<Window>,
        size: PhysicalSize<u32>,
        foliage_descriptions: Vec<MeshFoliageDesc<'window>>,
    ) -> InternalRendererWebGPU<'window, D> {
        // i prefer non-async code when possible for not-web we just block_on
        let mut wal = executor::block_on(wal::Wal::new(window, size));

        let lightmap_extent = Extent3d {
            width: 1024,
            height: 1024,
            depth_or_array_layers: 1,
        };

        let _chosen_depth_format = DEPTH_FORMAT_PREFERED;

        let independent_images =
            InternalRendererWebGPU::<D>::create_independent_images(&wal, lum_settings);
        let buffers = InternalRendererWebGPU::<D>::create_all_buffers(&mut wal, lum_settings);
        let samplers = InternalRendererWebGPU::<D>::create_all_samplers(&wal);

        let (dependent_images, pipes) = create_dependent(
            &wal,
            lum_settings,
            &foliage_descriptions,
            lum_settings,
            &independent_images,
            &buffers,
            &samplers,
        );

        let camera = Camera::default();
        let light = SunLight::default();

        let origin_world = Array3D::<InternalBlockId, D>::new_default(lum_settings.world_size);
        let current_world = Array3D::<InternalBlockId, D>::new_default(lum_settings.world_size);

        let mut renderer = InternalRendererWebGPU::<'_, D> {
            counter: 69420,
            wal,
            settings: Settings::<D>::default(),
            delta_time: 0.0,
            last_time: Instant::now(),

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
        lum_settings: &Settings<D>,
        window: std::sync::Arc<Window>,
        size: PhysicalSize<u32>,
        foliage_descriptions: Vec<MeshFoliageDesc<'window>>,
    ) -> InternalRendererWebGPU<'window, D> {
        let mut wal = wal::Wal::new(window, size).await;

        let lightmap_extent = Extent3d {
            width: 1024,
            height: 1024,
            depth_or_array_layers: 1,
        };

        let _chosen_depth_format = DEPTH_FORMAT_PREFERED;

        let independent_images =
            InternalRendererWebGPU::<D>::create_independent_images(&wal, lum_settings);
        let buffers = InternalRendererWebGPU::<D>::create_all_buffers(&mut wal, lum_settings);
        let samplers = InternalRendererWebGPU::<D>::create_all_samplers(&wal);

        let (dependent_images, pipes) = create_dependent(
            &wal,
            lum_settings,
            &foliage_descriptions,
            lum_settings,
            &independent_images,
            &buffers,
            &samplers,
        );

        let camera = Camera::default();
        let light = SunLight::default();

        let origin_world = Array3D::<InternalBlockId, D>::new_default(lum_settings.world_size);
        let current_world = Array3D::<InternalBlockId, D>::new_default(lum_settings.world_size);

        let mut renderer = InternalRendererWebGPU {
            counter: 69420,
            wal,
            settings: Settings::default(),
            delta_time: 0.0,
            last_time: Instant::now(),

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
    pub fn recreate_window(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // wait idle??

        // reconfigure the surface (swapchain)
        self.wal.resize(new_size);

        // recreate dependent resources
        let settings_copy = self.settings;
        // self.reset_index();

        // there is a bug (on my side) on wgpu resize
        // i did not figure it out yet
        // it is related to ring buffers
        // for now, just memorize all indices and restore them

        // TODO: is this a bad thing to do?
        // looks NOT good to me
        // but it literally solves all the possible problem with syncronizing them
        // TODO: try out global indexing like in lum++? It would also be faster...
        let ring_indices = self.store_indices();

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

        self.restore_indices(&ring_indices);
        // self.reset_index();
    }

    /// Destroys our renderer. In wgpu, resources are mostly cleaned up automatically
    pub fn destroy(self) {}

    fn move_next(&mut self) {
        self.buffers.staging_world.move_next();
        self.buffers.light_uniform.move_next();
        self.buffers.uniform.move_next();
        self.buffers.ao_lut_uniform.move_next();
        self.buffers.gpu_radiance_updates.move_next();
        // self.buffers.staging_radiance_updates.move_next();
        // self.buffers.gpu_particles_staged.move_next();
        self.buffers.gpu_particles.move_next();

        // self.independent_images.grass_state.move_next();
        // self.independent_images.water_state.move_next();
        // self.independent_images.perlin_noise2d.move_next();
        // self.independent_images.perlin_noise3d.move_next();
        self.independent_images.world.move_next();
        self.independent_images.radiance_cache.move_next();
        self.independent_images.block_palette.move_next();
        // self.independent_images.material_palette.move_next();
        // self.independent_images.lightmap.move_next();

        self.pipes
            .lightmap_blocks_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes
            .lightmap_models_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes
            .raygen_blocks_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes
            .raygen_models_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes
            .raygen_particles_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes
            .raygen_water_pipe
            .pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes.diffuse_pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());

        self.pipes.ao_pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());

        self.pipes
            .fill_stencil_glossy_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes
            .fill_stencil_smoke_pipe
            .pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes.glossy_pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());

        self.pipes.smoke_pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());

        self.pipes.tonemap_pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());

        self.pipes.radiance_pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());

        self.pipes.map_pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());

        self.pipes
            .update_grass_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes
            .update_water_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes
            .gen_perlin2d_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.pipes
            .gen_perlin3d_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        for foliage_pipe in self.pipes.raygen_foliage_pipes.iter_mut() {
            foliage_pipe.pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());
        }
    }

    // TODO: try out single global indexing like in lum++?
    fn _reset_index(&mut self) {
        self.buffers.staging_world.reset_index();
        self.buffers.light_uniform.reset_index();
        self.buffers.uniform.reset_index();
        self.buffers.ao_lut_uniform.reset_index();
        self.buffers.gpu_radiance_updates.reset_index();
        // self.buffers.staging_radiance_updates.move_next();
        // self.buffers.gpu_particles_staged.move_next();
        self.buffers.gpu_particles.reset_index();

        // self.independent_images.grass_state.move_next();
        // self.independent_images.water_state.move_next();
        // self.independent_images.perlin_noise2d.move_next();
        // self.independent_images.perlin_noise3d.move_next();
        self.independent_images.world.reset_index();
        self.independent_images.radiance_cache.reset_index();
        self.independent_images.block_palette.reset_index();
        // self.independent_images.material_palette.move_next();
        // self.independent_images.lightmap.move_next();

        self.pipes
            .lightmap_blocks_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes
            .lightmap_models_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes
            .raygen_blocks_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes
            .raygen_models_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes
            .raygen_particles_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes
            .raygen_water_pipe
            .pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes.diffuse_pipe.static_bind_groups.as_mut().map(|bg| bg.reset_index());

        self.pipes.ao_pipe.static_bind_groups.as_mut().map(|bg| bg.reset_index());

        self.pipes
            .fill_stencil_glossy_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes
            .fill_stencil_smoke_pipe
            .pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes.glossy_pipe.static_bind_groups.as_mut().map(|bg| bg.reset_index());

        self.pipes.smoke_pipe.static_bind_groups.as_mut().map(|bg| bg.reset_index());

        self.pipes.tonemap_pipe.static_bind_groups.as_mut().map(|bg| bg.reset_index());

        self.pipes.radiance_pipe.static_bind_groups.as_mut().map(|bg| bg.reset_index());

        self.pipes.map_pipe.static_bind_groups.as_mut().map(|bg| bg.reset_index());

        self.pipes
            .update_grass_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes
            .update_water_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes
            .gen_perlin2d_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        self.pipes
            .gen_perlin3d_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.reset_index());

        for foliage_pipe in self.pipes.raygen_foliage_pipes.iter_mut() {
            foliage_pipe.pipe.static_bind_groups.as_mut().map(|bg| bg.reset_index());
        }
    }

    /// Capture all the current ring-buffer indices into a snapshot
    pub fn store_indices(&self) -> IndicesSnapshot {
        IndicesSnapshot {
            staging_world: self.buffers.staging_world.index,
            light_uniform: self.buffers.light_uniform.index,
            uniform: self.buffers.uniform.index,
            ao_lut_uniform: self.buffers.ao_lut_uniform.index,
            gpu_radiance_updates: self.buffers.gpu_radiance_updates.index,
            gpu_particles: self.buffers.gpu_particles.index,

            world_image: self.independent_images.world.index,
            radiance_cache_image: self.independent_images.radiance_cache.index,
            block_palette_image: self.independent_images.block_palette.index,

            lightmap_blocks_bg: self
                .pipes
                .lightmap_blocks_pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            lightmap_models_bg: self
                .pipes
                .lightmap_models_pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            raygen_blocks_bg: self
                .pipes
                .raygen_blocks_pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            raygen_models_bg: self
                .pipes
                .raygen_models_pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            raygen_particles_bg: self
                .pipes
                .raygen_particles_pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            raygen_water_bg: self
                .pipes
                .raygen_water_pipe
                .pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            diffuse_bg: self.pipes.diffuse_pipe.static_bind_groups.as_ref().map(|bg| bg.index),
            ao_bg: self.pipes.ao_pipe.static_bind_groups.as_ref().map(|bg| bg.index),
            fill_stencil_glossy_bg: self
                .pipes
                .fill_stencil_glossy_pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            fill_stencil_smoke_bg: self
                .pipes
                .fill_stencil_smoke_pipe
                .pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            glossy_bg: self.pipes.glossy_pipe.static_bind_groups.as_ref().map(|bg| bg.index),
            smoke_bg: self.pipes.smoke_pipe.static_bind_groups.as_ref().map(|bg| bg.index),
            tonemap_bg: self.pipes.tonemap_pipe.static_bind_groups.as_ref().map(|bg| bg.index),
            radiance_bg: self.pipes.radiance_pipe.static_bind_groups.as_ref().map(|bg| bg.index),
            map_bg: self.pipes.map_pipe.static_bind_groups.as_ref().map(|bg| bg.index),
            update_grass_bg: self
                .pipes
                .update_grass_pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            update_water_bg: self
                .pipes
                .update_water_pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            gen_perlin2d_bg: self
                .pipes
                .gen_perlin2d_pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),
            gen_perlin3d_bg: self
                .pipes
                .gen_perlin3d_pipe
                .static_bind_groups
                .as_ref()
                .map(|bg| bg.index),

            raygen_foliage_bg: self
                .pipes
                .raygen_foliage_pipes
                .iter()
                .filter_map(|fp| fp.pipe.static_bind_groups.as_ref().map(|bg| bg.index))
                .collect(),
        }
    }

    /// Restore all ring-buffer indices from a previously taken snapshot.
    pub fn restore_indices(&mut self, snap: &IndicesSnapshot) {
        self.buffers.staging_world.index = snap.staging_world;
        self.buffers.light_uniform.index = snap.light_uniform;
        self.buffers.uniform.index = snap.uniform;
        self.buffers.ao_lut_uniform.index = snap.ao_lut_uniform;
        self.buffers.gpu_radiance_updates.index = snap.gpu_radiance_updates;
        self.buffers.gpu_particles.index = snap.gpu_particles;

        self.independent_images.world.index = snap.world_image;
        self.independent_images.radiance_cache.index = snap.radiance_cache_image;
        self.independent_images.block_palette.index = snap.block_palette_image;

        // helper to set Option<Ring>.index
        fn set_idx<T>(opt: &mut Option<Ring<T>>, new: Option<usize>) {
            if let (Some(r), Some(i)) = (opt.as_mut(), new) {
                r.index = i;
            }
        }

        set_idx(
            &mut self.pipes.lightmap_blocks_pipe.static_bind_groups,
            snap.lightmap_blocks_bg,
        );
        set_idx(
            &mut self.pipes.lightmap_models_pipe.static_bind_groups,
            snap.lightmap_models_bg,
        );
        set_idx(
            &mut self.pipes.raygen_blocks_pipe.static_bind_groups,
            snap.raygen_blocks_bg,
        );
        set_idx(
            &mut self.pipes.raygen_models_pipe.static_bind_groups,
            snap.raygen_models_bg,
        );
        set_idx(
            &mut self.pipes.raygen_particles_pipe.static_bind_groups,
            snap.raygen_particles_bg,
        );
        set_idx(
            &mut self.pipes.raygen_water_pipe.pipe.static_bind_groups,
            snap.raygen_water_bg,
        );
        set_idx(
            &mut self.pipes.diffuse_pipe.static_bind_groups,
            snap.diffuse_bg,
        );
        set_idx(&mut self.pipes.ao_pipe.static_bind_groups, snap.ao_bg);
        set_idx(
            &mut self.pipes.fill_stencil_glossy_pipe.static_bind_groups,
            snap.fill_stencil_glossy_bg,
        );
        set_idx(
            &mut self.pipes.fill_stencil_smoke_pipe.pipe.static_bind_groups,
            snap.fill_stencil_smoke_bg,
        );
        set_idx(
            &mut self.pipes.glossy_pipe.static_bind_groups,
            snap.glossy_bg,
        );
        set_idx(&mut self.pipes.smoke_pipe.static_bind_groups, snap.smoke_bg);
        set_idx(
            &mut self.pipes.tonemap_pipe.static_bind_groups,
            snap.tonemap_bg,
        );
        set_idx(
            &mut self.pipes.radiance_pipe.static_bind_groups,
            snap.radiance_bg,
        );
        set_idx(&mut self.pipes.map_pipe.static_bind_groups, snap.map_bg);
        set_idx(
            &mut self.pipes.update_grass_pipe.static_bind_groups,
            snap.update_grass_bg,
        );
        set_idx(
            &mut self.pipes.update_water_pipe.static_bind_groups,
            snap.update_water_bg,
        );
        set_idx(
            &mut self.pipes.gen_perlin2d_pipe.static_bind_groups,
            snap.gen_perlin2d_bg,
        );
        set_idx(
            &mut self.pipes.gen_perlin3d_pipe.static_bind_groups,
            snap.gen_perlin3d_bg,
        );

        for (fp, &i) in self.pipes.raygen_foliage_pipes.iter_mut().zip(&snap.raygen_foliage_bg) {
            if let Some(bg) = fp.pipe.static_bind_groups.as_mut() {
                bg.index = i;
            }
        }
    }
}

/// Create dependent resources (swapchain‐dependent images, pipelines, render passes)
fn create_dependent<D: Dim3>(
    wal: &wal::Wal,
    settings: &Settings<D>,
    foliage_descriptions: &[MeshFoliageDesc],
    _lumal_settings: &Settings<D>,
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
