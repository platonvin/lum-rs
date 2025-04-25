pub mod all_resources;
pub mod gen_perlin_noise;
pub mod load;
pub mod pipe;
pub mod render;
pub mod rpass;

use super::{Camera, SunLight};
use all_resources::all_rpasses::AllRenderPasses;
use glow::HasContext as GlowContext;
use glutin::{
    config::Config,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, Version},
    display::GetGlDisplay,
    prelude::{GlDisplay, NotCurrentGlContext},
};
use pipe::{ComputePipe, RasterPipe};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use rpass::RenderPassGL;
use winit::window::Window;

use crate::{containers::Array3D, types::*};

use super::Settings;

const FRAME_FORMAT: u32 = glow::RGBA16F;
const LIGHTMAPS_FORMAT: u32 = glow::DEPTH_COMPONENT16;
const MATNORM_FORMAT: u32 = glow::RGBA8UI;
const RADIANCE_FORMAT: u32 = glow::RGB10_A2;
const SECONDARY_DEPTH_FORMAT: u32 = glow::DEPTH_COMPONENT16;
static mut CHOSEN_DEPTH_FORMAT: u32 = glow::DEPTH_COMPONENT32F;

const BLOCK_PALETTE_SIZE_X: u32 = 64;
const BLOCK_PALETTE_SIZE_Y: u32 = 64;
const FRAMES_IN_FLIGHT: usize = 2;

pub struct AllPipes {
    lightmap_blocks_pipe: RasterPipe,
    lightmap_models_pipe: RasterPipe,

    raygen_blocks_pipe: RasterPipe,
    raygen_models_pipe: RasterPipe,
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
    // overlay_pipe: RasterPipe,
    radiance_pipe: ComputePipe,
    map_pipe: ComputePipe,
    update_grass_pipe: ComputePipe,
    update_water_pipe: ComputePipe,
    gen_perlin2d_pipe: ComputePipe, // generate noise for grass
    gen_perlin3d_pipe: ComputePipe, // generate noise for grass
}

pub struct AllSamplers {
    nearest_sampler: glow::Sampler,
    linear_sampler: glow::Sampler,
    linear_sampler_tiled: glow::Sampler,
    linear_sampler_tiled_mirrored: glow::Sampler,
    overlay_sampler: glow::Sampler,
    shadow_sampler: glow::Sampler,
    unnorm_linear: glow::Sampler,
    unnorm_nearest: glow::Sampler,
}

pub struct AllSwapchainDependentImages {
    highres_frame: glow::Texture,
    highres_depth_stencil: glow::Texture,
    highres_mat_norm: glow::Texture,
    far_depth: glow::Texture, // represents how much should smoke traversal for
    near_depth: glow::Texture, // represents how much should smoke traversal for
}

pub struct AllIndependentImages {
    grass_state: glow::Texture, // full-world grass shift (~direction) texture sampled in grass
    water_state: glow::Texture, //~same but water
    perlin_noise2d: glow::Texture, // full-world grass shift (~direction) texture sampled in grass
    perlin_noise3d: glow::Texture, // 4 channels of different tileable noise for volumetrics
    world: glow::Texture,       // can i really use just one?
    radiance_cache: glow::Texture,
    origin_block_palette: glow::Texture,
    material_palette: glow::Texture,
    lightmap: glow::Texture,
}

pub struct AllBuffers {
    staging_world: glow::Buffer,
    light_uniform: glow::Buffer,
    uniform: glow::Buffer,
    ao_lut_uniform: glow::Buffer,
    gpu_radiance_updates: glow::Buffer,
    staging_radiance_updates: glow::Buffer,
    gpu_particles: glow::Buffer,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GlSettings {
    pub timestamp_count: i32,
    pub fif: usize,
    pub vsync: bool,
    pub fullscreen: bool,
    pub debug: bool,
    pub profile: bool,
}

#[pub_fields::pub_fields] // lol i use crate for this
pub struct InternalRendererGL {
    counter: isize,
    gl: glow::Context,
    width_height: (u32, u32),
    // renderer settings. Cannot be changed after creation
    settings: Settings,
    lightmap_extent: (u32, u32),

    // fields called LumThings are just grouped Vulkan objects needed by renderer
    pipes: AllPipes,
    foliage_descriptions: Vec<InternalMeshFoliageDesc>,
    dependent_images: AllSwapchainDependentImages,
    rpasses: AllRenderPasses,
    independent_images: AllIndependentImages,
    buffers: AllBuffers,
    samplers: AllSamplers,

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
    // block_copies_queue: Vec<vk::ImageCopy>, // No direct equivalent
    // Queue of all blocks that need to be zeroed
    // Quite often you need to copy "air" block (empty, zero one) on allocation
    // modern GPUs are very fast at zeroing memory, so we can do it separately as optimization
    // block_clear_queue: Vec<vk::ImageSubresourceRange>,

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

    // just particles. Hardocoded.
    particles: Vec<Particle>,

    // time, taken by the last frame - you know
    delta_time: f32,

    // used to track if loaded magicavoxel file should write its palette to Lum (implicitly)
    has_palette: bool,
    // CPU side material palette in vector (not in image like on GPU)
    material_palette: Vec<Material>, // its fixed size but its fine
    block_palette_voxels: Vec<BlockVoxels>, // its fixed size but its fine
    block_palette_meshes: Vec<InternalMeshBlock<Option<glow::Buffer>>>, // its fixed size but its fine
}
const DEPTH_FORMAT_SPARE: u32 = glow::DEPTH24_STENCIL8; // Example
const DEPTH_FORMAT_PREFERED: u32 = glow::DEPTH32F_STENCIL8; // Example

impl InternalRendererGL {
    pub unsafe fn create(
        lum_settings: &Settings,
        window: &winit::window::Window,
        // event_loop: &winit::event_loop::EventLoop<()>,
        foliage_descriptions: Vec<InternalMeshFoliageDesc>,
    ) -> InternalRendererGL {
        // Just fucking loading gl context took more time due to shitty docs then loading Vulkan. BASED
        // Seriously, WHY THE FUCK IT IS SO COMPLICATED?
        // I whish there was a crate to do it.... WAIT, 4th ONE?

        // 1) Create a glutin Display from winit raw handles
        let raw_window_handle = window.window_handle().ok().unwrap().as_raw();
        let raw_disaply_handle = window.display_handle().ok().unwrap().as_raw();
        let preference = glutin::display::DisplayApiPreference::Wgl(Some(raw_window_handle));
        let gl_display = glutin::display::Display::new(raw_disaply_handle, preference).unwrap();

        // 2) Pick a framebuffer config
        let template = glutin::config::ConfigTemplateBuilder::new().with_alpha_size(8).build();
        let mut configs = gl_display.find_configs(template).unwrap();
        let config = configs.next().unwrap();

        // 3) Create a GL context (not current yet)
        let context_attrs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 1))))
            .with_debug(true) // TODO:
            .build(Some(raw_window_handle));
        let not_current_ctx: NotCurrentContext =
            gl_display.create_context(&config, &context_attrs).unwrap();

        // 4) Create a WindowSurface to draw into
        let size = window.inner_size();
        let surface_attrs =
            glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
                .build(
                    raw_window_handle,
                    std::num::NonZeroU32::new(size.width).unwrap(),
                    std::num::NonZeroU32::new(size.height).unwrap(),
                );
        let surface = gl_display.create_window_surface(&config, &surface_attrs).unwrap();

        // 5) Make the context current (TODO: what does this thing return?)
        let _gl_context = not_current_ctx.make_current(&surface).unwrap();

        // 6) Load all GL function pointers
        let mut gl = glow::Context::from_loader_function(|symbol| {
            let c_str = std::ffi::CString::new(symbol).unwrap();
            gl_display.get_proc_address(&c_str)
        });

        unsafe {
            gl.enable(glow::DEBUG_OUTPUT);
            gl.enable(glow::DEBUG_OUTPUT_SYNCHRONOUS); // Optional: makes callbacks synchronous
            gl.debug_message_callback(|source: u32, kind: u32, id: u32, severity: u32, message: &str|
                    println!("OpenGL Debug Message\nSource: {}\nType: {}\nID: {}\nSeverity: {}\nMessage: {}",
                        source, kind, id, severity, message)
            );
        }

        let lightmap_extent = (1024, 1024);

        CHOSEN_DEPTH_FORMAT = DEPTH_FORMAT_PREFERED;

        let independent_images = Self::create_independent_images(&gl, lum_settings);
        let buffers = Self::create_all_buffers(&gl, lum_settings);
        let samplers = Self::create_all_samplers(&gl);

        let (width, height) = (window.inner_size().width, window.inner_size().height);

        let (dependent_images, pipes, renderpasses) = Self::create_dependent(
            &gl,
            lum_settings,
            &foliage_descriptions,
            &independent_images,
            &buffers,
            &samplers,
            (width, height),
        );

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

        let mut lum = InternalRendererGL {
            counter: 69420,
            gl,
            width_height: (width, height),
            settings: Settings::default(),
            delta_time: 0.0,

            rpasses: renderpasses,

            lightmap_extent,
            pipes,
            independent_images,
            dependent_images,
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
            foliage_descriptions,
            block_palette_meshes: (0..lum_settings.static_block_palette_size)
                .map(|_| InternalMeshBlock::default())
                .collect(),
        };

        lum.gen_perlin_2d();
        lum.gen_perlin_3d();

        lum
    }

    // disassemble the renderer, recreate resources, and reassemble it back
    #[cold]
    #[optimize(size)]
    pub fn recreate_window(&mut self, window: &Window) {
        unsafe { self.gl.finish() }; // Wait for GPU to finish

        unsafe {
            Self::destroy_dependent(
                &self.gl,
                std::mem::replace(
                    &mut self.dependent_images,
                    std::mem::MaybeUninit::zeroed().assume_init(),
                ),
                std::mem::replace(
                    &mut self.pipes,
                    std::mem::MaybeUninit::zeroed().assume_init(),
                ),
                std::mem::replace(
                    &mut self.rpasses,
                    std::mem::MaybeUninit::zeroed().assume_init(),
                ),
            )
        };

        let (width, height) = unsafe {
            let drawable = window.inner_size();
            (drawable.width, drawable.height)
        };
        unsafe { self.gl.viewport(0, 0, width as i32, height as i32) }; // Reset viewport

        let (dimages, pipes, rpasses) = Self::create_dependent(
            &self.gl,
            &self.settings,
            &self.foliage_descriptions,
            &self.independent_images,
            &self.buffers,
            &self.samplers,
            (0, 0),
        );

        // return back the values
        self.dependent_images = dimages;
        self.pipes = pipes;
        self.rpasses = rpasses;
    }

    /// Destroys our renderer
    pub unsafe fn destroy(mut self) {
        let gl = &self.gl;

        gl.finish(); // Wait for GPU to finish
        Self::destroy_independent_images(gl, self.independent_images);
        Self::destroy_all_buffers(gl, self.buffers);

        Self::destroy_dependent(gl, self.dependent_images, self.pipes, self.rpasses);

        Self::destroy_all_samplers(gl, self.samplers);
    }

    unsafe fn destroy_dependent(
        gl: &glow::Context,
        dependent_images: AllSwapchainDependentImages,
        pipes: AllPipes,
        rpasses: AllRenderPasses,
    ) {
        Self::destroy_dependent_images(gl, dependent_images);
        // Self::destroy_all_pipes(gl, pipes);
        Self::destroy_all_rpasses(gl, rpasses);
    }

    fn create_dependent(
        gl: &glow::Context,
        lum_settings: &Settings,
        foliage_descriptions: &Vec<InternalMeshFoliageDesc>,
        independent_images: &AllIndependentImages,
        buffers: &AllBuffers,
        samplers: &AllSamplers,
        wh: (u32, u32),
    ) -> (AllSwapchainDependentImages, AllPipes, AllRenderPasses) {
        let mut dependent_images = Self::create_dependent_images(gl, wh);

        // let mut pipes: AllPipes = AllPipes::default();
        // pipes
        //     .raygen_foliage_programs
        //     .resize(foliage_descriptions.len(), RasterPipe::default()); // Initialize with default

        let renderpasses: AllRenderPasses = Self::create_all_rpasses(
            gl,
            &lum_settings,
            independent_images,
            &mut dependent_images,
            wh,
        );

        let pipes = unsafe {
            Self::create_all_pipes(
                gl,
                lum_settings,
                buffers,
                independent_images,
                &dependent_images,
                samplers,
                foliage_descriptions,
            )
        };

        (dependent_images, pipes, renderpasses)
    }
}
