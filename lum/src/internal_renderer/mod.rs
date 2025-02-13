pub mod aabb;
pub mod all_resources;
pub mod ao_lut;
pub mod gen_perlin_noise;
pub mod load;
// pub mod ogt_vox;
pub mod ogt_vox;
pub mod render;

use anyhow::Result;
use lumal::{ring::Ring, trace, RasterPipe};
use render::{Camera, SunLight};
use vulkanalia::vk::{self, DeviceV1_0, Extent2D};
use winit::window::Window;

use crate::{containers::Array3D, types::*};

#[derive(Clone, Copy)]
pub struct Settings {
    pub world_size: uvec3,
    pub static_block_palette_size: u32,
    pub max_particle_count: u32,
    pub lightmap_extent: vk::Extent2D,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            world_size: uvec3::new(48, 48, 16),
            static_block_palette_size: 15,
            max_particle_count: 8128,
            lightmap_extent: Extent2D {
                width: 1024,
                height: 1024,
            },
        }
    }
}

const FRAME_FORMAT: vk::Format = vk::Format::R16G16B16A16_UNORM;
const LIGHTMAPS_FORMAT: vk::Format = vk::Format::D16_UNORM;
const MATNORM_FORMAT: vk::Format = vk::Format::R8G8B8A8_UINT;
const RADIANCE_FORMAT: vk::Format = vk::Format::A2B10G10R10_UNORM_PACK32;
const SECONDARY_DEPTH_FORMAT: vk::Format = vk::Format::R16_SFLOAT;
static mut CHOSEN_DEPTH_FORMAT: vk::Format = vk::Format::UNDEFINED;
const BLOCK_PALETTE_SIZE_X: u32 = 64;
const BLOCK_PALETTE_SIZE_Y: u32 = 64;
const FRAMES_IN_FLIGHT: usize = 2;

// groups all Pipes (abstraction on top of Vulkan Pipelines) into one struct
// most of them are hardcoded, but foliage pipes are optional and partially managed by user
#[derive(Default)]
pub struct AllPipes {
    lightmap_blocks_pipe: RasterPipe,
    lightmap_models_pipe: RasterPipe,

    raygen_blocks_pipe: RasterPipe,
    raygen_models_pipe: RasterPipe,
    raygen_models_push_layout: vk::DescriptorSetLayout,
    raygen_particles_pipe: RasterPipe,
    raygen_water_pipe: RasterPipe,
    raygen_foliage_pipes: Vec<RasterPipe>,

    diffuse_pipe: RasterPipe,
    ao_pipe: RasterPipe,
    fill_stencil_glossy_pipe: RasterPipe,
    fill_stencil_smoke_pipe: RasterPipe,
    glossy_pipe: RasterPipe,
    smoke_pipe: RasterPipe,
    tonemap_pipe: RasterPipe,
    overlay_pipe: RasterPipe,

    radiance_pipe: lumal::ComputePipe,
    map_pipe: lumal::ComputePipe,
    map_push_layout: vk::DescriptorSetLayout,
    update_grass_pipe: lumal::ComputePipe,
    update_water_pipe: lumal::ComputePipe,
    gen_perlin2d_pipe: lumal::ComputePipe, // generate noise for grass
    gen_perlin3d_pipe: lumal::ComputePipe, /* generate noise for grass
                                            * Lum has a long history of optimizations, and these were used in the past, but unnecessary now
                                            * dfx_pipe: lumal::ComputePipe,
                                            * dfy_pipe: lumal::ComputePipe,
                                            * dfz_pipe: lumal::ComputePipe,
                                            * bitmask_pipe: lumal::ComputePipe, */
}

pub struct AllSamplers {
    nearest_sampler: vk::Sampler,
    linear_sampler: vk::Sampler,
    linear_sampler_tiled: vk::Sampler,
    linear_sampler_tiled_mirrored: vk::Sampler,
    overlay_sampler: vk::Sampler,
    shadow_sampler: vk::Sampler,
    unnorm_linear: vk::Sampler,
    unnorm_nearest: vk::Sampler,
}

#[derive(Default)]
pub struct AllSwapchainDependentImages {
    // my brain is too small to handle lifetimes
    // swapchain_images: Ring<lumal::Image>,
    highres_frame: Ring<lumal::Image>,
    highres_depth_stencil: Ring<lumal::Image>,
    highres_mat_norm: Ring<lumal::Image>,
    stencil_view_for_ds: Ring<vk::ImageView>,
    far_depth: Ring<lumal::Image>, // represents how much should smoke traversal for
    near_depth: Ring<lumal::Image>, /* represents how much should smoke traversal for
                                    * mask_frame: Ring<lumal::Image>, //where lowres renders to. Blends with highres afterwards */
}

pub struct AllIndependentImages {
    grass_state: Ring<lumal::Image>, /* full-world grass shift (~direction) texture sampled in grass */
    water_state: Ring<lumal::Image>, //~same but water
    perlin_noise2d: Ring<lumal::Image>, /* full-world grass shift (~direction) texture sampled in grass */
    perlin_noise3d: Ring<lumal::Image>, /* 4 channels of different tileable noise for volumetrics */
    world: Ring<lumal::Image>,          // can i really use just one?
    radiance_cache: Ring<lumal::Image>,
    origin_block_palette: Ring<lumal::Image>,
    material_palette: Ring<lumal::Image>,
    lightmap: Ring<lumal::Image>,
    // distance_palette: Ring<lumal::Image>,
    // bit_palette: Ring<lumal::Image>, //bitmask of originBlockPalette
}

pub struct AllBuffers {
    // is or might be in use when cpu is recording new one. Is pretty cheap, so just leave it
    staging_world: Ring<lumal::Buffer>,
    light_uniform: Ring<lumal::Buffer>,
    uniform: Ring<lumal::Buffer>,
    ao_lut_uniform: Ring<lumal::Buffer>,
    gpu_radiance_updates: Ring<lumal::Buffer>,
    staging_radiance_updates: Ring<lumal::Buffer>,
    gpu_particles: Ring<lumal::Buffer>, // multiple because cpu-related work
}

pub struct AllCommandBuffers {
    compute_command_buffers: Ring<vk::CommandBuffer>,
    lightmap_command_buffers: Ring<vk::CommandBuffer>,
    graphics_command_buffers: Ring<vk::CommandBuffer>,
    copy_command_buffers: Ring<vk::CommandBuffer>, /* runtime copies for ui. Also does first frame resources */
}

#[derive(Default)]
pub struct AllRenderPasses {
    lightmap_rpass: lumal::RenderPass,
    gbuffer_rpass: lumal::RenderPass,
    shade_rpass: lumal::RenderPass, // for no downscaling
}

pub struct AllSwapchainDependent {}

#[pub_fields::pub_fields] // lol i use crate for this
pub struct InternalRenderer {
    counter: isize,
    lumal: lumal::Renderer,
    // renderer settings. Cannot be changed after creation
    settings: Settings,
    lightmap_extent: vk::Extent2D,

    // fields called LumThings are just grouped Vulkan objects needed by renderer
    pipes: AllPipes,
    foliage_descriptions: Vec<InternalMeshFoliageDesc>,
    dependent_images: AllSwapchainDependentImages,
    rpasses: AllRenderPasses,
    independent_images: AllIndependentImages,
    buffers: AllBuffers,
    samplers: AllSamplers,
    cmdbufs: AllCommandBuffers,

    // Queue of blocks whose radiance field needs to be updated. Filled automatically by the renderer
    radiance_updates: Vec<i8vec4>,
    // somehow caching allocated is slower...
    // m_ru_visited: BitArray3d<u64>,
    // same but requested by user (manually)
    special_radiance_updates: Vec<i8vec4>,

    // position / direction / sizes of the Camera. Yes, no generic super-high level abstraction, just pod vectors
    camera: Camera,
    light: SunLight,

    // Queue of all the 3d block data that needs to be duplicated when allocating new blocks.
    // Lum uses references to blocks when possible for perfomance reasons
    // but when a block needs to be modified (like when it intersects a model), we have to instantiate it
    // which means allocating a new block, copiying the old one to allocated, and then referencing it instead
    // TODO: ImageCopy is quite big, use more compact representation
    block_copies_queue: Vec<vk::ImageCopy>,
    // Queue of all blocks that need to be zeroed
    // Quite often you need to copy "air" block (empty, zero one) on allocation
    // modern GPUs are very fast at zeroing memory, so we can do it separately as optimization
    block_clear_queue: Vec<vk::ImageSubresourceRange>,

    // tracks amount of allocated (including static) blocks in palette. Used internally for allocation.
    // Resets to static_block_count every frame
    palette_counter: usize,

    // how many blocks are static blocks (not allocated). Static blocks have voxel data (loaded from file)
    static_block_palette_size: u32,

    // ground truth for block references data, without any block allocations (no models)
    origin_world: Array3D<BlockId>,
    // modified origin world, with some blocks allocated for models
    // for internal use only
    current_world: Array3D<BlockId>,

    // just particles. Hardocded.
    particles: Vec<Particle>,

    // time, taken by the last frame - you know
    delta_time: f32,

    // used to track if loaded magicavoxel file should write its palette to Lum (implicitly)
    has_palette: bool,
    // CPU side material palette in vector (not in image like on GPU)
    material_palette: Vec<Material>, // its fixed size but its fine
    block_palette_voxels: Vec<BlockVoxels>, // its fixed size but its fine
    block_palette_meshes: Vec<InternalMeshBlock>, // its fixed size but its fine
}
const DEPTH_FORMAT_SPARE: vk::Format = vk::Format::D24_UNORM_S8_UINT; // TODO somehow D32 faster than vk::Format::D24_UNORM_S8_UINT on low-end
const DEPTH_FORMAT_PREFERED: vk::Format = vk::Format::D32_SFLOAT_S8_UINT;

// TODO: separate file
// extern "C" {
//     fn my_cpp_function();
// }

impl InternalRenderer {
    // Creates Lum::InternalRenderer. You should use Renderer::create() and then .init() instead
    pub unsafe fn create(
        lum_settings: &Settings,
        window: &Window,
        mut foliage_descriptions: Vec<InternalMeshFoliageDesc>,
    ) -> Result<InternalRenderer> {
        let mut lumal_settings = lumal::LumalSettings::create_default();
        if cfg!(debug_assertions) {
            lumal_settings.debug = true;
        }
        let mut lumal = lumal::Renderer::create(&lumal_settings, window)?;

        let lightmap_extent = vk::Extent2D {
            width: 1024,
            height: 1024,
        };

        CHOSEN_DEPTH_FORMAT = lumal
            .find_supported_format(
                &[DEPTH_FORMAT_PREFERED, DEPTH_FORMAT_SPARE],
                vk::ImageType::_2D,
                vk::ImageTiling::OPTIMAL,
                vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::SAMPLED
                    | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                    | vk::ImageUsageFlags::INPUT_ATTACHMENT,
            )
            .unwrap();

        // section where most important (not init-related) vulkan resources are created. Some of them will be recreated on window resize
        let mut independent_images =
            InternalRenderer::create_independent_images(&lumal, lum_settings, &lumal_settings);
        let buffers = InternalRenderer::create_all_buffers(&lumal, lum_settings, &lumal_settings);
        let samplers = InternalRenderer::create_all_samplers(&lumal, lum_settings, &lumal_settings);
        let command_buffers =
            InternalRenderer::create_all_command_buffers(&lumal, lum_settings, &lumal_settings);

        let (dependent_images, pipes, renderpasses) = create_dependent(
            &mut lumal,
            lum_settings,
            &foliage_descriptions,
            &lumal_settings,
            &independent_images,
            &buffers,
            &samplers,
        );

        let camera = Camera::default();
        let light = SunLight::default();
        trace!();

        let origin_world = Array3D::<BlockId>::new(
            lum_settings.world_size.x as usize,
            lum_settings.world_size.y as usize,
            lum_settings.world_size.z as usize,
        );
        // same as initalization but cleaner imho
        let current_world = Array3D {
            data: origin_world.data.clone(),
            x_size: origin_world.x_size,
            y_size: origin_world.y_size,
            z_size: origin_world.z_size,
        };

        let mut lum = InternalRenderer {
            counter: 69420,
            lumal,
            settings: Settings::default(),
            delta_time: 0.0,

            rpasses: renderpasses,
            cmdbufs: command_buffers,

            lightmap_extent,
            pipes,
            independent_images,
            dependent_images,
            buffers,
            samplers,
            camera,
            light,
            palette_counter: 0,
            static_block_palette_size: lum_settings.static_block_palette_size, /* TODO: remove settings */
            origin_world,
            current_world,
            has_palette: false,
            // somehow caching allocated is slower...
            // m_ru_visited: BitArray3d::new_filled(
            //     lum_settings.world_size.x as usize,
            //     lum_settings.world_size.y as usize,
            //     lum_settings.world_size.z as usize,
            //     false,
            // ),
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
            block_palette_meshes: vec![
                Default::default();
                lum_settings.static_block_palette_size as usize
            ],
        };

        trace!();

        // fills noise images with values. I use them for grass / water / smoke
        lum.gen_perlin_2d();
        lum.gen_perlin_3d();

        Ok(lum)
    }

    // pub fn create_swapchain_dependent(&mut lumal) {
    //     self.dependent_images = InternalRenderer::create_dependent_images(
    //         &self.lumal,
    //         &self.settings,
    //         &self.lumal.settings,
    //     );
    // }

    // disassemble the renderer, recreate resources, and reassemble it back
    #[cold]
    #[optimize(size)]
    pub fn recreate_window(&mut self, window: &Window) {
        unsafe { self.lumal.device.device_wait_idle().unwrap() };
        lumal::atrace!();
        unsafe {
            Self::destroy_dependent(
                &mut self.lumal,
                std::mem::take(&mut self.dependent_images),
                std::mem::take(&mut self.pipes),
                std::mem::take(&mut self.rpasses),
            )
        };
        lumal::atrace!();
        self.lumal.recreate_swapchain(window);
        lumal::atrace!();

        let settings_copy = self.lumal.settings.clone();
        let (dimages, pipes, rpasses) = create_dependent(
            &mut self.lumal,
            &self.settings,
            &self.foliage_descriptions,
            &settings_copy, // damn it
            &self.independent_images,
            &self.buffers,
            &self.samplers,
        );
        lumal::atrace!();

        // return back the values
        self.dependent_images = dimages;
        self.pipes = pipes;
        self.rpasses = rpasses;
    }

    /// Destroys our Vulkan app.
    pub unsafe fn destroy(mut self) {
        let mut lumal = self.lumal;

        // TODO: there is something im missing in winit that should make this unnecessary. How did i do it in C++?
        lumal.device.device_wait_idle().unwrap();
        Self::destroy_independent_images(&lumal, &mut self.independent_images);
        Self::destroy_all_buffers(&lumal, self.buffers);

        lumal.process_deletion_queues_untill_all_done();

        Self::destroy_dependent(&mut lumal, self.dependent_images, self.pipes, self.rpasses);

        Self::destroy_all_samplers(&mut lumal, &mut self.samplers);
        Self::destroy_all_command_buffers(&mut lumal, &self.cmdbufs);

        lumal.destroy();
    }

    unsafe fn destroy_dependent(
        lumal: &mut lumal::Renderer,
        dependent_images: AllSwapchainDependentImages,
        pipes: AllPipes,
        rpasses: AllRenderPasses,
    ) {
        Self::destroy_dependent_images(&*lumal, dependent_images);
        Self::destroy_all_pipes(lumal, pipes);
        Self::destroy_all_rpasses(lumal, rpasses);
    }
}

fn create_dependent(
    lumal: &mut lumal::Renderer,
    lum_settings: &Settings,
    foliage_descriptions: &Vec<InternalMeshFoliageDesc>,
    lumal_settings: &lumal::LumalSettings,
    independent_images: &AllIndependentImages,
    buffers: &AllBuffers,
    samplers: &AllSamplers,
) -> (AllSwapchainDependentImages, AllPipes, AllRenderPasses) {
    let mut dependent_images =
        InternalRenderer::create_dependent_images(&*lumal, lum_settings, &lumal_settings);
    let mut pipes: AllPipes = AllPipes::default();
    pipes
        .raygen_foliage_pipes
        .resize(foliage_descriptions.len(), RasterPipe::default());

    let renderpasses: AllRenderPasses = InternalRenderer::create_all_rpasses(
        lumal,
        lum_settings,
        &lumal_settings,
        independent_images,
        &mut dependent_images,
        &mut pipes,
    );
    trace!();

    unsafe {
        InternalRenderer::create_all_pipes(
            lumal,
            lum_settings,
            &lumal_settings,
            buffers,
            &*independent_images,
            &dependent_images,
            samplers,
            &mut pipes,
            foliage_descriptions,
        )
    };
    (dependent_images, pipes, renderpasses)
}
