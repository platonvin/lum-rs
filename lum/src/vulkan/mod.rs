pub mod all_resources;
pub mod gen_perlin_noise;
pub mod load;
pub mod pc_types;
pub mod render;
pub mod types;

use super::{Camera, Settings, SunLight};
use crate::{types::*, vulkan::types::*, BLOCK_SIZE};
use containers::array3d::Dim3;
use containers::Array3D;
use containers::Ring;
use lumal::{vk, RasterPipe};
use render::MeshFoliageDescription;
use winit::dpi::PhysicalSize;
use winit::window::Window;

// TODO: choose dynamically?
const FRAME_FORMAT: vk::Format = vk::Format::R16G16B16A16_UNORM;
const LIGHTMAPS_FORMAT: vk::Format = vk::Format::D16_UNORM;
const MATNORM_FORMAT: vk::Format = vk::Format::R8G8B8A8_UINT;
const RADIANCE_FORMAT: vk::Format = vk::Format::A2B10G10R10_UNORM_PACK32;
const SECONDARY_DEPTH_FORMAT: vk::Format = vk::Format::R16_SFLOAT;
static mut CHOSEN_DEPTH_FORMAT: vk::Format = vk::Format::UNDEFINED;

const BLOCK_PALETTE_SIZE_X: u32 = 64;
const BLOCK_PALETTE_SIZE_Y: u32 = 64;
const AO_LUT_SIZE: usize = 8;

/// Bundle of all Pipes (abstraction on top of Vulkan Pipelines)
/// most of them are hardcoded, but foliage pipes are optional and (partially?) managed by user
#[derive(Default)]
pub struct AllPipes {
    /// Rasterizes blocks into depth for lightmapping
    lightmap_blocks_pipe: RasterPipe,
    /// Rasterizes models into depth for lightmapping
    lightmap_models_pipe: RasterPipe,

    /// Rasterizes blocks into gbuffer
    raygen_blocks_pipe: RasterPipe,
    /// Rasterizes models into gbuffer
    raygen_models_pipe: RasterPipe,
    /// Descriptor set layout for per-model (not static) data in models pipe
    raygen_models_push_layout: vk::DescriptorSetLayout,
    /// Rasterizes particles into gbuffer. Particles are rendered in a single drawcall
    raygen_particles_pipe: RasterPipe,
    /// Rasterizes water into gbuffer. Water vertices are generated from combining water heightmaps
    /// submitted as drawcall per "block"
    raygen_water_pipe: RasterPipe,
    /// Rasterizes grass into gbuffer. Grass vertices are generated from wind direction and shader (yes, code not data) for flexibility
    /// submitted as drawcall per "block"
    raygen_foliage_pipes: Vec<RasterPipe>,

    /// Pipe for actual shading (diffuse light)
    diffuse_pipe: RasterPipe,
    /// Ambient Occlusion pipe (nvidia calls this HBAO lol)
    ao_pipe: RasterPipe,
    /// Shader for culling expensive glossy shader by generating stencil mask for pixels that need to be processed in cheap shader
    fill_stencil_glossy_pipe: RasterPipe,
    /// Shader for culling expensive smoke shader by generating stencil mask for pixels that need to be processed in cheap shader
    fill_stencil_smoke_pipe: RasterPipe,
    /// Renders reflections on top of diffuse light
    glossy_pipe: RasterPipe,
    /// Renders volumetrics
    smoke_pipe: RasterPipe,
    /// Transforms color and writes it to swapchain image
    tonemap_pipe: RasterPipe,
    /// currently unused
    overlay_pipe: RasterPipe,

    /// Updades per-block lighting
    radiance_pipe: lumal::ComputePipe,
    /// Maps voxels of a model into the worldspace
    map_pipe: lumal::ComputePipe,
    map_push_layout: vk::DescriptorSetLayout,
    /// Updates wiggle (wind) direction of grass state image
    update_grass_pipe: lumal::ComputePipe,
    /// Updates heightmaps of water state(s)
    update_water_pipe: lumal::ComputePipe,
    /// Single-use pipe for generating tileable perline noise (2d)
    gen_perlin2d_pipe: lumal::ComputePipe,
    /// Single-use pipe for generating tileable perline noise (3d)
    gen_perlin3d_pipe: lumal::ComputePipe,
}

/// Bundle of all samplers
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
/// Bundle of all images that are bound to swapchain (size) (and need to be recreated on e.g. resize)
pub struct AllSwapchainDependentImages {
    // my brain is too small to handle lifetimes
    /// image where frame is composed in multiple shaders (aka "virtual swapchain image")
    frame: lumal::Image,
    /// Gbuffer with material id and normal
    mat_norm: lumal::Image,
    /// Depth and stencil image
    depth_stencil: lumal::Image,
    /// stencil and depth are special, and sometimes we need stencil only view to depth-stencil image
    stencil_view_for_ds: vk::ImageView,
    /// represents how much should smoke traversal for
    far_depth: lumal::Image,
    /// represents how much should smoke traversal for
    near_depth: lumal::Image,
}

/// Bundle of all image that do not relate to swapchain
pub struct AllIndependentImages {
    /// Image whose pixels define grass wiggle direction in corresponding world position (to pixel)
    grass_state: lumal::Image,
    /// (4) Heightmaps for water, used in generating waves. We use 4 for better tiling
    /// each next level represents twice bigger tile with half impact (final height is sum of intersecting tiles)
    water_state: lumal::Image,
    /// Tiled 2d perlin noise
    perlin_noise2d: lumal::Image, // full-world grass shift (~direction) texture sampled in grass
    /// Tiled 3d perlin noise
    perlin_noise3d: lumal::Image, // 4 channels of different tileable noise for volumetrics
    /// 3d image of block indices (pointing to block palette). Written by CPU every frame to "allocate" blocks in palette for mapping models
    world: Ring<lumal::Image>, // can i really use just one?
    /// 3d image of radiance probes (aligned to world grid)
    radiance_cache: Ring<lumal::Image>,
    /// "arena" of actual voxel data for blocks. Part of it stays static, and part of it is written to when mapping models into world (it holds model data)
    block_palette: Ring<lumal::Image>,
    /// Voxel is just index of a material in material palette. Specifies color, roughness, transparency and emmitance of a material
    material_palette: Ring<lumal::Image>,
    /// Humans treat sunlight specially so its estimated with lightmapping
    lightmap: lumal::Image,
}

/// Bundle of all GPU buffers
/// we need Ring of CPU-GPU resources because when CPU is writing current one, GPU might (and will!) use previous one
pub struct AllBuffers {
    /// Buffer used to deliver world data to world_image
    /// Updated with just writing to it (its mapped), so CPU-GPU and in Ring
    staging_world: Ring<lumal::Buffer>,

    /// UBO for lightmapping
    /// Updated with vkCmdUpdateBuffer (cause small), so CPU-GPU and in Ring
    light_uniform: Ring<lumal::Buffer>,

    /// UBO for most shaders in frame (kinda like global readonly per-frame data)
    /// Updated with vkCmdUpdateBuffer (cause small), so CPU-GPU and in Ring
    uniform: Ring<lumal::Buffer>,

    /// Some precomputed per-frame data for AO shader (this is worth it believe me)
    ao_lut_uniform: Ring<lumal::Buffer>,

    /// Buffer ("queue") of requests for updating light (which compute shader picks up, processes and stores results)
    /// Not a ring, because not CPU-GPU (instead, copied from staging CPU-GPU buffer)
    gpu_radiance_updates: lumal::Buffer,

    /// CPU-visible memory for gpu_radiance_updates
    staging_radiance_updates: Ring<lumal::Buffer>,

    /// Buffer with all particles
    /// Updated with direct write from CPU (mapped), so CPU-GPU and in Ring
    gpu_particles: Ring<lumal::Buffer>, // multiple because cpu-related work
}

/// Bundle of CommandBuffer's. There is no real reason for them to be separate, but they are
pub struct AllCommandBuffers {
    /// cmd buffers used for compute work
    compute_command_buffers: Ring<vk::CommandBuffer>,
    /// cmd buffers used for generating lightmaps work
    lightmap_command_buffers: Ring<vk::CommandBuffer>,
    /// cmd buffers for rasterization work
    graphics_command_buffers: Ring<vk::CommandBuffer>,
    /// cmd buffers for runtime copies (they happen before everything else). Also does first frame resources (TODO: it doesnt, but it should)
    copy_command_buffers: Ring<vk::CommandBuffer>, /*  */
}

/// Bundle of all RenderPass'es
#[derive(Default)]
pub struct AllRenderPasses {
    /// Lightmap blocks and models. Separate cause not 1:1 dependency
    lightmap_rpass: lumal::RenderPass,
    /// Rasterizes block, model, foliage and water pixel data into Gbuffer (and depth).
    /// Separate because shading also needs near pixels (and depth)
    /// (thats just how subpasses work, read about them)
    gbuffer_rpass: lumal::RenderPass,
    /// Draws actual pixels on screen after multiple computational stages
    shade_rpass: lumal::RenderPass,
}

/// Main rendering struct
/// Unlike wgpu, Vulkan backend is split into 2 parts
/// This allows you to have less CPU-side overhead if you want (by submitting commands directly without queues)
/// *wgpu backend does not have this due to skill issues*
pub struct InternalRendererVulkan<'a, D: Dim3> {
    /// Internal frame counter. Used as rng seed
    pub counter: isize,
    /// Vulkan abstraction that Lum uses
    pub lumal: lumal::Renderer,
    /// renderer settings. Cannot be changed after creation
    pub settings: Settings<D>,

    /// All foliage descriptions (which have lifetime of renderer because they dont change after creation)
    pub foliage_descriptions: Vec<MeshFoliageDescription<'a>>,

    // fields called LumThings are just grouped Vulkan objects needed by renderer
    pub pipes: AllPipes,
    pub dependent_images: AllSwapchainDependentImages,
    pub rpasses: AllRenderPasses,
    pub independent_images: AllIndependentImages,
    pub buffers: AllBuffers,
    pub samplers: AllSamplers,
    pub cmdbufs: AllCommandBuffers,

    /// Queue of blocks whose radiance field needs to be updated.
    /// Filled automatically by the renderer and can be extended with special_radiance_updates
    pub radiance_updates: Vec<i8vec4>,
    /// Appended to renderer-generated radinace updates
    pub special_radiance_updates: Vec<i8vec4>,

    /// position / direction / sizes of the Camera. Right, no generic super-high level abstraction, just pod vectors
    pub camera: Camera,
    // this feels wrong
    pub light: SunLight,

    /// Queue of all the 3d block data (in images) that needs to be duplicated when allocating new blocks.
    /// Lum uses references to blocks (via index that indexes into block palette) for perfomance reasons
    /// but when a block needs to be modified (like when it intersects a model), we have to instantiate it
    /// which means allocating a new block, copiying the old one to allocated, and then referencing it instead
    // TODO: ImageCopy is quite big, use more compact representation
    pub block_copies_queue: Vec<vk::ImageCopy>,

    /// tracks amount of allocated (including static) blocks in palette. Used internally for block allocation.
    /// Resets to static_block_palette_size every frame
    pub palette_counter: usize,

    /// how many blocks are static blocks (frame-persistent)
    pub static_block_palette_size: u32,

    /// ground truth for block references data, without any block allocations (no models)
    pub origin_world: Array3D<MeshBlock, D>,
    /// modified origin world, with some blocks allocated for models
    /// for internal use only
    pub current_world: Array3D<MeshBlock, D>,

    /// just particles. Very simple system to process them
    pub particles: Vec<Particle>,

    /// Time, taken by the last frame, in seconds
    pub delta_time: f32,
    /// Last time we measured time (delta = now - last)
    pub last_time: std::time::Instant,

    /// Used to track if loaded magicavoxel file should write its palette to Lum (implicitly)
    /// When Lum has no palette, it inherits first palette from loaded blocks
    pub has_palette: bool,
    /// CPU-side material palette in vector (not in image like on GPU)
    pub material_palette: Vec<Material>, // its fixed size but its fine
    /// CPU-side Voxel data for static blocks
    pub block_palette_voxels: Vec<BlockVoxels>, // its fixed size but its fine

    /// GPU objects for mesh data for blocks.
    /// Unlike models, meshes for blocks are stored internally and indexed instead
    pub block_palette_meshes: Vec<InternalMeshBlock>, // its fixed size but its fine

                                                      // somehow caching allocated is slower...
                                                      // i used to not recreate this thing every frame but it was noticably slower (LOL)
                                                      // m_ru_visited: BitArray3d<u64>,
}

const DEPTH_FORMAT_SPARE: vk::Format = vk::Format::D24_UNORM_S8_UINT; // TODO somehow D32 faster than vk::Format::D24_UNORM_S8_UINT on low-end
const DEPTH_FORMAT_PREFERED: vk::Format = vk::Format::D32_SFLOAT_S8_UINT;

impl<'a, D: Dim3> InternalRendererVulkan<'a, D> {
    pub fn new(
        lum_settings: &Settings<D>,
        window: &Window,
        size: PhysicalSize<u32>,
        foliage_descriptions: Vec<MeshFoliageDescription<'a>>,
    ) -> InternalRendererVulkan<'a, D> {
        let mut lumal_settings = lumal::LumalSettings::default();
        if cfg!(debug_assertions) {
            lumal_settings.debug = true;
        }
        let mut lumal = lumal::Renderer::new(&lumal_settings, window, size);

        unsafe {
            CHOSEN_DEPTH_FORMAT = lumal
                .find_supported_format(
                    &[DEPTH_FORMAT_PREFERED, DEPTH_FORMAT_SPARE],
                    vk::ImageType::TYPE_2D,
                    vk::ImageTiling::OPTIMAL,
                    vk::ImageUsageFlags::TRANSFER_SRC
                        | vk::ImageUsageFlags::TRANSFER_DST
                        | vk::ImageUsageFlags::SAMPLED
                        | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                        | vk::ImageUsageFlags::INPUT_ATTACHMENT,
                )
                .unwrap()
        };

        // section where most important (not init-related) vulkan resources are created. Some of them will be recreated on window resize
        let independent_images = InternalRendererVulkan::create_independent_images(
            &mut lumal,
            lum_settings,
            &lumal_settings,
        );
        let buffers =
            InternalRendererVulkan::create_all_buffers(&mut lumal, lum_settings, &lumal_settings);
        let samplers =
            InternalRendererVulkan::create_all_samplers(&lumal, lum_settings, &lumal_settings);
        let command_buffers = InternalRendererVulkan::create_all_command_buffers(
            &lumal,
            lum_settings,
            &lumal_settings,
        );

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

        let origin_world = Array3D::<MeshBlock, D>::new_default(lum_settings.world_size);
        // same as initalization but cleaner imho
        let current_world = Array3D::<_, D> {
            data: origin_world.data.clone(),
            dims: origin_world.dims,
        };

        let mut lum = InternalRendererVulkan {
            counter: 69420,
            lumal,
            settings: Settings::default(),
            delta_time: 0.0,
            last_time: std::time::Instant::now(),

            rpasses: renderpasses,
            cmdbufs: command_buffers,

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
                [[[0; BLOCK_SIZE as usize]; BLOCK_SIZE as usize];
                    BLOCK_SIZE as usize];
                lum_settings.static_block_palette_size as usize
            ],
            block_copies_queue: vec![],
            foliage_descriptions,
            block_palette_meshes: (0..lum_settings.static_block_palette_size)
                .map(|_| InternalMeshBlock::default())
                .collect(),
        };

        // fills noise images with values. I use them for grass / water / smoke
        lum.gen_perlin_2d();
        lum.gen_perlin_3d();

        lum
    }

    /// Disassemblse the renderer, recreates swapchain dependent resources, and reassembles it back
    pub fn recreate_window(&mut self, size: PhysicalSize<u32>) {
        // wait for GPU to complete all work so deleting resources wont break anything
        unsafe { self.lumal.device.device_wait_idle().unwrap() };

        // we CANT actually disassemble Renderer to recreate parts of it
        // this is due to mix of styles: functional (move self return Self) vs &mut self modifying fields
        // to solve this i implement unsafe default() with uninit (i really dont want Option<>)
        // TODO: converge on specific style
        Self::destroy_dependent(
            &mut self.lumal,
            std::mem::take(&mut self.dependent_images),
            std::mem::take(&mut self.pipes),
            std::mem::take(&mut self.rpasses),
        );
        self.lumal.recreate_swapchain(size);

        // in Vulkan, you can drop the entire pool or descriptors individually (you need special settings tho)
        // most of them are invalid after resizing anyways, so dropping pool is faster and easier
        unsafe {
            self.lumal.device.destroy_descriptor_pool(self.lumal.descriptor_pool, None);
            self.lumal.descriptor_pool = self.lumal.create_descriptor_pool();
        };

        let settings_copy = self.lumal.settings;
        let (dimages, pipes, rpasses) = create_dependent(
            &mut self.lumal,
            &self.settings,
            &self.foliage_descriptions,
            &settings_copy, // damn it
            &self.independent_images,
            &self.buffers,
            &self.samplers,
        );

        // return back the values
        self.dependent_images = dimages;
        self.pipes = pipes;
        self.rpasses = rpasses;
    }

    /// Destroys the renderer
    pub fn destroy(self) {
        let mut lumal = self.lumal;

        // TODO: there is something im missing in winit that should make this unnecessary. How did i do it in C++?
        unsafe { lumal.device.device_wait_idle().unwrap() };
        Self::destroy_independent_images(&mut lumal, self.independent_images);
        Self::destroy_all_buffers(&mut lumal, self.buffers);

        lumal.process_deletion_queues_untill_all_done();

        Self::destroy_dependent(&mut lumal, self.dependent_images, self.pipes, self.rpasses);

        Self::destroy_all_samplers(&mut lumal, self.samplers);
        Self::destroy_all_command_buffers(&lumal, &self.cmdbufs);

        unsafe { lumal.destroy() };
    }

    fn destroy_dependent(
        lumal: &mut lumal::Renderer,
        dependent_images: AllSwapchainDependentImages,
        pipes: AllPipes,
        rpasses: AllRenderPasses,
    ) {
        Self::destroy_dependent_images(lumal, dependent_images);
        Self::destroy_all_pipes(lumal, pipes);
        Self::destroy_all_rpasses(lumal, rpasses);
    }
}

fn create_dependent<D: Dim3>(
    lumal: &mut lumal::Renderer,
    lum_settings: &Settings<D>,
    foliage_descriptions: &[MeshFoliageDescription],
    lumal_settings: &lumal::LumalSettings,
    independent_images: &AllIndependentImages,
    buffers: &AllBuffers,
    samplers: &AllSamplers,
) -> (AllSwapchainDependentImages, AllPipes, AllRenderPasses) {
    let mut dependent_images =
        InternalRendererVulkan::create_dependent_images(lumal, lum_settings, lumal_settings);
    let mut pipes: AllPipes = AllPipes::default();
    pipes
        .raygen_foliage_pipes
        .resize(foliage_descriptions.len(), RasterPipe::default());

    let renderpasses: AllRenderPasses = InternalRendererVulkan::create_all_rpasses(
        lumal,
        lum_settings,
        lumal_settings,
        independent_images,
        &mut dependent_images,
        &mut pipes,
    );

    InternalRendererVulkan::create_all_pipes(
        lumal,
        lum_settings,
        lumal_settings,
        buffers,
        independent_images,
        &dependent_images,
        samplers,
        &mut pipes,
        foliage_descriptions,
    );
    (dependent_images, pipes, renderpasses)
}
