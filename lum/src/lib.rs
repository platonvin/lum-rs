#![allow(dead_code, unused)]
#![allow(unused_parens)]

/*
 * This is a glue-file
 * files that start with "all_" are initializing / destroying resources (packed in structs)
 * internal_renderer is where all the gpu commands are submitted
 * renderer is a wrapper around internal_renderer that is more stable and easier to use
 */

pub mod consts;
pub mod all_pipes;
pub mod all_buffers;
pub mod all_images;
pub mod all_samplers;
pub mod types;
pub mod all_rpasses;

pub mod internal_renderer;

use std::{mem, ptr::null_mut};
use anyhow::Result;
use consts::*;
use internal_renderer::{Camera, SunLight};
use types::*; // to use GENERAL as general image layout
use lumal::{atrace, descriptors::{DescriptorInfo, RelativeDescriptorPos, ShortDescriptorInfo}, ring::Ring, ComputePipe, LumalRenderer, LumalSettings, RasterPipe};
use vulkanalia::vk::{self, DeviceV1_3, Extent2D, Handle};
// use winit::event_loop::EventLoop;
use winit::window::Window;
// use glam::*;
// use lumal::LumalDescriptorType::*;
// use lumal::LumalShaderStageFlags::*;

use vk::Sampler;
use RelativeDescriptorPos::Current;


#[derive(Clone, Copy)]
pub struct LumSettings {
    world_size: uvec3,
    static_block_palette_size: u32,
    max_particle_count: u32,
    lightmap_extent: vk::Extent2D,
}

impl LumSettings {
    pub fn default() -> LumSettings {
        LumSettings {
            world_size: uvec3::new(48, 48, 16),
            static_block_palette_size: 15,
            max_particle_count: 8128,
            lightmap_extent: Extent2D {width:1024, height:1024},
        }
    }
}

#[allow(non_camel_case_types)]
type BlockID_t = i16;
#[allow(non_camel_case_types)]
type MatID_t = u8;
type Voxel = u8;

pub struct Particle {
    pos: vec3,
    vel: vec3,
    life_time: f32,
    mat_id: MatID_t,
}

pub struct AoLut {
    world_shift: vec3,
    weight_normalized: f32, // ((1-r^2)/total_weight)*0.7
    screen_shift: vec2,
    padding: vec2,
}

const FRAME_FORMAT:vk::Format = vk::Format::R16G16B16A16_UNORM;
const LIGHTMAPS_FORMAT:vk::Format = vk::Format::D16_UNORM;
const MATNORM_FORMAT:vk::Format = vk::Format::R8G8B8A8_UINT;
const RADIANCE_FORMAT:vk::Format = vk::Format::A2B10G10R10_UNORM_PACK32;
const SECONDARY_DEPTH_FORMAT:vk::Format = vk::Format::R16_SFLOAT;
static mut CHOSEN_DEPTH_FORMAT:vk::Format = vk::Format::UNDEFINED;
const BLOCK_PALETTE_SIZE_X:u32 = 64;
const BLOCK_PALETTE_SIZE_Y:u32 = 64;
const FRAMES_IN_FLIGHT:usize = 2;

#[derive(Default)]
pub struct LumPipes {
    lightmap_blocks_pipe:  lumal::RasterPipe,
    lightmap_models_pipe:  lumal::RasterPipe,

    raygen_blocks_pipe:        lumal::RasterPipe,
    raygen_models_pipe:        lumal::RasterPipe,
    raygen_models_push_layout:  vk::DescriptorSetLayout,
    raygen_particles_pipe:     lumal::RasterPipe,
    raygen_water_pipe:         lumal::RasterPipe,
    raygen_grass_pipes:    Vec<lumal::RasterPipe>,
 
    diffuse_pipe:             lumal::RasterPipe,
    ao_pipe:                  lumal::RasterPipe,
    fill_stencil_glossy_pipe: lumal::RasterPipe,
    fill_stencil_smoke_pipe:  lumal::RasterPipe,
    glossy_pipe:              lumal::RasterPipe,
    smoke_pipe:               lumal::RasterPipe,
    tonemap_pipe:             lumal::RasterPipe,
    overlay_pipe:             lumal::RasterPipe,

    
    // raytrace_pipe: lumal::ComputePipe,
    radiance_pipe: lumal::ComputePipe,
    map_pipe: lumal::ComputePipe,
    map_push_layout: vk::DescriptorSetLayout,
    update_grass_pipe: lumal::ComputePipe,
    update_water_pipe: lumal::ComputePipe,
    gen_perlin2d_pipe: lumal::ComputePipe, //generate noise for grass
    gen_perlin3d_pipe: lumal::ComputePipe, //generate noise for grass
    // dfx_pipe: lumal::ComputePipe,
    // dfy_pipe: lumal::ComputePipe,
    // dfz_pipe: lumal::ComputePipe,
    // bitmask_pipe: lumal::ComputePipe,
}

pub struct LumSamplers {
    nearest_sampler: vk::Sampler,
    linear_sampler: vk::Sampler,
    linear_sampler_tiled: vk::Sampler,
    linear_sampler_tiled_mirrored: vk::Sampler,
    overlay_sampler: vk::Sampler,
    shadow_sampler: vk::Sampler,
    unnorm_linear: vk::Sampler,
    unnorm_nearest: vk::Sampler,
}

pub struct LumSwapchainDependentImages {
    // my brain is too small to handle lifetimes
    swapchain_images: Ring<lumal::Image>,
    highres_frame: Ring<lumal::Image>,
    highres_depth_stencil: Ring<lumal::Image>,
    highres_mat_norm: Ring<lumal::Image>,
    stencil_view_for_ds: Ring<vk::ImageView>,
    far_depth: Ring<lumal::Image>,  //represents how much should smoke traversal for
    near_depth: Ring<lumal::Image>, //represents how much should smoke traversal for
    // mask_frame: Ring<lumal::Image>, //where lowres renders to. Blends with highres afterwards
}

pub struct LumIndependentImages {
    grass_state: Ring<lumal::Image>, //full-world grass shift (~direction) texture sampled in grass
    water_state: Ring<lumal::Image>, //~same but water
    perlin_noise2d: Ring<lumal::Image>, //full-world grass shift (~direction) texture sampled in grass
    perlin_noise3d: Ring<lumal::Image>, //4 channels of different tileable noise for volumetrics
    world: Ring<lumal::Image>, //can i really use just one?
    radiance_cache: Ring<lumal::Image>,
    origin_block_palette: Ring<lumal::Image>,
    material_palette: Ring<lumal::Image>,
    lightmap: Ring<lumal::Image>,
    // distance_palette: Ring<lumal::Image>,
    // bit_palette: Ring<lumal::Image>, //bitmask of originBlockPalette
}

pub struct LumBuffers {
    //is or might be in use when cpu is recording new one. Is pretty cheap, so just leave it
    staging_world: Ring<lumal::Buffer>,
    light_uniform: Ring<lumal::Buffer>,
    uniform: Ring<lumal::Buffer>,
    ao_lut_uniform: Ring<lumal::Buffer>,
    gpu_radiance_updates: Ring<lumal::Buffer>,
    staging_radiance_updates: Ring<lumal::Buffer>,
    gpu_particles: Ring<lumal::Buffer>, //multiple because cpu-related work
}

pub struct LumCommandBuffers {
    compute_command_buffers: Ring<vk::CommandBuffer>,
    lightmap_command_buffers: Ring<vk::CommandBuffer>,
    graphics_command_buffers: Ring<vk::CommandBuffer>,
    copy_command_buffers: Ring<vk::CommandBuffer>, //runtime copies for ui. Also does first frame resources
}

pub struct LumRenderPasses {
    lightmap_rpass: lumal::RenderPass,
    gbuffer_rpass: lumal::RenderPass,
    shade_rpass: lumal::RenderPass, //for no downscaling
}

#[allow(non_snake_case)]
pub struct LumRenderer {
    lumal: lumal::LumalRenderer,
    settings: LumSettings,
    lightmap_extent: vk::Extent2D,

    pipes: LumPipes,
    dependent_images: LumSwapchainDependentImages,
    independent_images: LumIndependentImages,
    buffers: LumBuffers,
    samplers: LumSamplers,
    // cmdbufs: LumCommandBuffers,
    rpasses: LumRenderPasses,

    radianceUpdates: Vec<i8vec4>,
    specialRadianceUpdates: Vec<i8vec4>,

    camera: internal_renderer::Camera,
    light: SunLight,
    // descriptorPool: vk::DescriptorPool,
}
const DEPTH_FORMAT_SPARE :vk::Format = vk::Format::D24_UNORM_S8_UINT; //TODO somehow faster than vk::Format::D24_UNORM_S8_UINT on low-end
const DEPTH_FORMAT_PREFERED :vk::Format = vk::Format::D32_SFLOAT_S8_UINT;

extern "C" {
    fn my_cpp_function();
}

impl LumRenderer {
    /// Creates our Vulkan app.
    pub unsafe fn create(lum_settings: &LumSettings) -> Result<LumRenderer> {
        // Ok(Self {})
        my_cpp_function();
        
        let mut
            // that's the codestyle i use for Vulkan. It allows small ident and esier typing errors check
            lumal_settings = lumal::LumalSettings::create_default();
            lumal_settings.debug = true;
        let mut lumal = lumal::LumalRenderer::create(lumal_settings.clone())?;

        let lightmap_extent = vk::Extent2D {
            width: 1024, 
            height: 1024
        };

        CHOSEN_DEPTH_FORMAT = lumal.find_supported_format(&[DEPTH_FORMAT_PREFERED, DEPTH_FORMAT_SPARE], vk::ImageType::_2D, vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_SRC | 
            vk::ImageUsageFlags::TRANSFER_DST | 
            vk::ImageUsageFlags::SAMPLED | 
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | 
            vk::ImageUsageFlags::INPUT_ATTACHMENT).unwrap();
        
        // section where most important (not init-related) vulkan resources are created. Some of them will be recreated on window resize
        let mut dependent_images: LumSwapchainDependentImages = LumRenderer::create_dependent_images(&lumal, &lum_settings, &lumal_settings);
        let mut independent_images: LumIndependentImages = LumRenderer::create_independent_images(&lumal, &lum_settings, &lumal_settings);
        let buffers: LumBuffers = LumRenderer::create_all_buffers(&lumal, &lum_settings, &lumal_settings);
        let samplers: LumSamplers = LumRenderer::create_all_samplers(&lumal, &lum_settings, &lumal_settings);
        let command_buffers: LumCommandBuffers;
        
        let mut pipes: LumPipes = LumPipes::default();

        let renderpasses: LumRenderPasses = LumRenderer::create_all_rpasses(
            &mut lumal, 
            &lum_settings, 
            &lumal_settings, 
            &mut independent_images, 
            &mut dependent_images, 
            &mut pipes
        );
        atrace!();

        LumRenderer::create_all_pipes(
            &mut lumal, 
            &lum_settings, 
            &lumal_settings, 
            &buffers,
            &independent_images,
            &dependent_images,
            &samplers,
            &mut pipes,
        );
        
        let camera = Camera::default();
        let light = SunLight::default();
        atrace!();
            
        let mut lum = LumRenderer {
            lumal: lumal,
            settings: LumSettings::default(),

            rpasses: renderpasses,
            // cmdbufs: command_buffers,

            lightmap_extent: lightmap_extent,
            pipes: pipes,
            // dependent_images: dependent_images,
            independent_images: independent_images,
            dependent_images: dependent_images,
            buffers: buffers,
            samplers: samplers,
            camera: camera,
            light: light,
            radianceUpdates: vec![],
            specialRadianceUpdates: vec![],
        };
        
        atrace!();

        // fills noise images with values. I use them for grass / water / smoke
        lum.gen_perlin_2d();
        lum.gen_perlin_3d();

        return Ok(lum);
    }
    /// Renders a frame for our Vulkan app.
    pub unsafe fn render(&mut self, window: &Window) -> Result<()> {
        Ok(())
    }

    /// Destroys our Vulkan app.
    pub unsafe fn destroy(&mut self) {
        self.destroy_independent_images();
        self.destroy_dependent_images();
        self.destroy_all_buffers();

        LumRenderer::destroy_all_pipes(&mut self.lumal, &mut self.pipes);
        LumRenderer::destroy_all_rpasses(&mut self.lumal, &mut self.rpasses);
        LumRenderer::destroy_all_samplers(&mut self.lumal, &mut self.samplers); 
        
        self.lumal.destroy();
    }
    
    fn destroy_all_rpasses(lumal: &mut LumalRenderer, rpasses: &mut LumRenderPasses) {
        lumal.destroy_render_pass(&mut rpasses.lightmap_rpass);
        lumal.destroy_render_pass(&mut rpasses.gbuffer_rpass);
        lumal.destroy_render_pass(&mut rpasses.shade_rpass);
    }
    
    fn destroy_all_samplers(lumal: &mut LumalRenderer, samplers: &mut LumSamplers) {
        lumal.destroy_sampler(samplers.nearest_sampler);
        lumal.destroy_sampler(samplers.linear_sampler);
        lumal.destroy_sampler(samplers.linear_sampler_tiled);
        lumal.destroy_sampler(samplers.linear_sampler_tiled_mirrored);
        lumal.destroy_sampler(samplers.overlay_sampler);
        lumal.destroy_sampler(samplers.shadow_sampler);
        lumal.destroy_sampler(samplers.unnorm_linear);
        lumal.destroy_sampler(samplers.unnorm_nearest);
    }
}