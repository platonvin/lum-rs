pub mod casts;

use std::mem;
use anyhow::Result;
use casts::*;
use lumal::{ring::Ring, LumalRenderer, LumalSettings};
use vulkanalia::vk::{self, Extent2D};
// use winit::event_loop::EventLoop;
use winit::window::Window;
use glam::*;

#[derive(Clone, Copy)]
pub struct LumSettings {
    world_size: UVec3,
    static_block_palette_size: u32,
    max_particle_count: u32,
    lightmap_extent: vk::Extent2D,
}

impl LumSettings {
    pub fn default() -> LumSettings {
        LumSettings {
            world_size: UVec3::new(48, 48, 16),
            static_block_palette_size: 15,
            max_particle_count: 8128,
            lightmap_extent: Extent2D {width:0, height:0},
        }
    }
}

#[allow(non_camel_case_types)]
type BlockID_t = i16;
#[allow(non_camel_case_types)]
type MatID_t = u8;
type Voxel = u8;

pub struct Particle {
    pos: Vec3,
    vel: Vec3,
    life_time: f32,
    mat_id: MatID_t,
}

pub struct AoLut {
    world_shift: Vec3,
    weight_normalized: f32, // ((1-r^2)/total_weight)*0.7
    screen_shift: Vec2,
    padding: Vec2,
}

const FRAME_FORMAT:vk::Format = vk::Format::R16G16B16A16_UNORM;
const LIGHTMAPS_FORMAT:vk::Format = vk::Format::D16_UNORM;
const MATNORM_FORMAT:vk::Format = vk::Format::R8G8B8A8_UINT;
const RADIANCE_FORMAT:vk::Format = vk::Format::A2B10G10R10_UNORM_PACK32;
const SECONDARY_DEPTH_FORMAT:vk::Format = vk::Format::R16_SFLOAT;
static mut chosen_depth_format:vk::Format = vk::Format::UNDEFINED;
const BLOCK_PALETTE_SIZE_X:u32 = 64;
const BLOCK_PALETTE_SIZE_Y:u32 = 64;

pub struct LumPipes {
    lightmap_blocks_pipe:  lumal::RasterPipe,
    lightmap_models_pipe:  lumal::RasterPipe,

    raygen_blocks_pipe:    lumal::RasterPipe,
    raygen_models_pipe:    lumal::RasterPipe,
    raygen_models_push_layout: vk::DescriptorSetLayout,
    raygen_particles_pipe: lumal::RasterPipe,
    raygen_grass_pipe:     lumal::RasterPipe,
    raygen_water_pipe:     lumal::RasterPipe,

    diffuse_pipe:           lumal::RasterPipe,
    ao_pipe:                lumal::RasterPipe,
    fill_stencil_glossy_pipe: lumal::RasterPipe,
    fill_stencil_smoke_pipe:  lumal::RasterPipe,
    glossy_pipe:            lumal::RasterPipe,
    smoke_pipe:             lumal::RasterPipe,
    tonemap_pipe:           lumal::RasterPipe,
    overlay_pipe:           lumal::RasterPipe,

    
    raytrace_pipe: lumal::ComputePipe,
    radiance_pipe: lumal::ComputePipe,
    map_pipe: lumal::ComputePipe,
    map_push_layout: vk::DescriptorSetLayout,
    update_grass_pipe: lumal::ComputePipe,
    update_water_pipe: lumal::ComputePipe,
    gen_perlin2d_pipe: lumal::ComputePipe, //generate noise for grass
    gen_perlin3d_pipe: lumal::ComputePipe, //generate noise for grass
    dfx_pipe: lumal::ComputePipe,
    dfy_pipe: lumal::ComputePipe,
    dfz_pipe: lumal::ComputePipe,
    bitmask_pipe: lumal::ComputePipe,
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
    swapchain_images: Ring<lumal::Image>,
    highres_frame: Ring<lumal::Image>,
    highres_depth_stencil: Ring<lumal::Image>,
    highres_mat_norm: Ring<lumal::Image>,
    stencil_view_for_ds: Ring<vk::ImageView>,
    far_depth: Ring<lumal::Image>,  //represents how much should smoke traversal for
    near_depth: Ring<lumal::Image>, //represents how much should smoke traversal for
    mask_frame: Ring<lumal::Image>, //where lowres renders to. Blends with highres afterwards
}

pub struct LumIndependentImages {
    grass_state: Ring<lumal::Image>, //full-world grass shift (~direction) texture sampled in grass
    water_state: Ring<lumal::Image>, //~same but water
    
    perlin_noise2d: Ring<lumal::Image>, //full-world grass shift (~direction) texture sampled in grass
    perlin_noise3d: Ring<lumal::Image>, //4 channels of different tileable noise for volumetrics

    world: Ring<lumal::Image>, //can i really use just one?
    radiance_cache: Ring<lumal::Image>,

    origin_block_palette: Ring<lumal::Image>,
    // distance_palette: Ring<lumal::Image>,
    // bit_palette: Ring<lumal::Image>, //bitmask of originBlockPalette
    material_palette: Ring<lumal::Image>,
    lightmap: Ring<lumal::Image>,
}

pub struct LumBuffers {
    //is or might be in use when cpu is recording new one. Is pretty cheap, so just leave it
    staging_world: Ring<lumal::Buffer>,
    light_uniform: Ring<lumal::Buffer>,
    uniform: Ring<lumal::Buffer>,
    ao_lut_uniform: Ring<lumal::Buffer>,
    gpu_radiance_updates: Ring<lumal::Buffer>,
    // Ring<void*> stagingRadianceUpdatesMapped;
    staging_radiance_updates: Ring<lumal::Buffer>,

    // particles: Vec<Particle>,
    gpu_particles: Ring<lumal::Buffer>, //multiple because cpu-related work
    // Ring<void* > gpuParticlesMapped; //multiple because cpu-related work
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

    // pipes: LumPipes,
    // dependent_images: LumSwapchainDependentImages,
    independent_images: LumIndependentImages,
    // buffers: LumBuffers,
    // samplers: LumSamplers,
    // cmdbufs: LumCommandBuffers,
    // rpasses: LumRenderPasses,

    radianceUpdates: Vec<I8Vec4>,
    specialRadianceUpdates: Vec<I8Vec4>,

    // descriptorPool: vk::DescriptorPool,
}
const DEPTH_FORMAT_SPARE :vk::Format = vk::Format::D24_UNORM_S8_UINT; //TODO somehow faster than vk::Format::D24_UNORM_S8_UINT on low-end
const DEPTH_FORMAT_PREFERED :vk::Format = vk::Format::D32_SFLOAT_S8_UINT;

impl LumRenderer {
    /// Creates our Vulkan app.
    pub unsafe fn create(lum_settings: LumSettings) -> Result<LumRenderer> {
        // Ok(Self {})
        let mut
        // that's the codestyle i use for Vulkan. It allows small ident and esier typing errors check
        lumal_settings = lumal::LumalSettings::create_default();
        lumal_settings.debug = true;
        let lumal = lumal::LumalRenderer::create(lumal_settings.clone())?;

        let lightmap_extent = vk::Extent2D {
            width: 1024, 
            height: 1024
        };

        chosen_depth_format = lumal.find_supported_format(&[DEPTH_FORMAT_PREFERED, DEPTH_FORMAT_SPARE], vk::ImageType::_2D, vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_SRC | 
            vk::ImageUsageFlags::TRANSFER_DST | 
            vk::ImageUsageFlags::SAMPLED | 
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | 
            vk::ImageUsageFlags::INPUT_ATTACHMENT).unwrap();
        
        // section where most important (not init-related) vulkan resources are created. Some of them will be recreated on window resize
        let pipes: LumPipes;
        let dependent_images: LumSwapchainDependentImages;
        let independent_images: LumIndependentImages = Self::create_independent_images(&lumal, lum_settings, lumal_settings.clone());
        let buffers: LumBuffers;
        let samplers: LumSamplers;
        let command_buffers: LumCommandBuffers;
        let renderpasses: LumRenderPasses;

            
        let lum = LumRenderer {
            lumal: lumal,
            settings: LumSettings::default(),

            // rpasses: renderpasses,
            // cmdbufs: command_buffers,

            lightmap_extent: lightmap_extent,
            // pipes: pipes,
            // dependent_images: dependent_images,
            independent_images: independent_images,
            // buffers: buffers,
            // samplers: samplers,
            radianceUpdates: vec![],
            specialRadianceUpdates: vec![],
        };
        
        return Ok(lum);
    }
    /// Renders a frame for our Vulkan app.
    pub unsafe fn render(&mut self, window: &Window) -> Result<()> {
        Ok(())
    }

    /// Destroys our Vulkan app.
    pub unsafe fn destroy(&mut self) {
        self.lumal.destroy();
    }
    
    pub fn create_buffers(lumal: &LumalRenderer, lum_settings: LumSettings, lumal_settings: LumalSettings) -> LumBuffers {
        let gpu_particles = lumal.create_buffer_storages (
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            (lum_settings.max_particle_count as usize) * mem::size_of::<Particle>(), 
            true);
        let uniform = lumal.create_buffer_storages (
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            220, 
            false); //no way i write it with mem::size_of::<
        let light_uniform = lumal.create_buffer_storages (
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<Mat4>(), 
            false);
        let ao_lut_uniform = lumal.create_buffer_storages (
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<AoLut>() * 8, 
            false); //TODO DYNAMIC AO SAMPLE COUNT
        let gpu_radiance_updates = lumal.create_buffer_storages (
            lumal_settings.fif,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<I8Vec4>()*
                (lum_settings.world_size.x as usize) * 
                (lum_settings.world_size.y as usize) * 
                (lum_settings.world_size.z as usize), 
            false); //TODO test extra mem
        let staging_radiance_updates = lumal.create_buffer_storages (
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_SRC,
            mem::size_of::<IVec4>() as usize*
                (lum_settings.world_size.x as usize) * 
                (lum_settings.world_size.y as usize) * 
                (lum_settings.world_size.z as usize), 
            true); //TODO test extra mem

        let staging_world = lumal.create_buffer_storages (
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_SRC,
                    (lum_settings.world_size.x as usize) * 
                    (lum_settings.world_size.y as usize) * 
                    (lum_settings.world_size.z as usize) *  
            (mem::size_of::<BlockID_t>() as usize), true);
        return LumBuffers {
            staging_world: staging_world.unwrap(),
            light_uniform: light_uniform.unwrap(),
            uniform: uniform.unwrap(),
            ao_lut_uniform: ao_lut_uniform.unwrap(),
            gpu_radiance_updates: gpu_radiance_updates.unwrap(),
            staging_radiance_updates: staging_radiance_updates.unwrap(),
            gpu_particles: gpu_particles.unwrap(),
        };
    }

    pub fn create_independent_images(lumal: &LumalRenderer, lum_settings: LumSettings, lumal_settings: LumalSettings) -> LumIndependentImages {
        let world = lumal.create_image_storages (
            lumal_settings.fif,
            vk::ImageType::_3D,
            vk::Format::R16_SINT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(lum_settings.world_size),
            1,
            vk::SampleCountFlags::_1,
            ); //TODO: dynamic
        let lightmap = lumal.create_image_storages (
            lumal_settings.fif,
            vk::ImageType::_2D,
            LIGHTMAPS_FORMAT,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::DEPTH,
            vk::Extent3D {
                width: lum_settings.lightmap_extent.width, 
                height: lum_settings.lightmap_extent.height, 
                depth: 1},
            1,
            vk::SampleCountFlags::_1,
            );
        let radiance_cache = lumal.create_image_storages (
            lumal_settings.fif,
            vk::ImageType::_3D,
            RADIANCE_FORMAT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(lum_settings.world_size),
            1,
            vk::SampleCountFlags::_1,
            );
        let origin_block_palette = lumal.create_image_storages (
            lumal_settings.fif,
            vk::ImageType::_3D,
            vk::Format::R8_UINT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: 16 * BLOCK_PALETTE_SIZE_X, 
                height: 16 * BLOCK_PALETTE_SIZE_Y, 
                depth: 16},
            1,
            vk::SampleCountFlags::_1,
            );
        let material_palette = lumal.create_image_storages (
            lumal_settings.fif,
            vk::ImageType::_2D,
            vk::Format::R32_SFLOAT, //try R32G32
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: 6, 
                height: 256, 
                depth: 1},
            1,
            vk::SampleCountFlags::_1,
            );
        let grass_state = lumal.create_image_storages (
            lumal_settings.fif,
            vk::ImageType::_2D,
            vk::Format::R16G16_SFLOAT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: lum_settings.world_size.x*2, 
                height: lum_settings.world_size.y*2, 
                depth: 1},
            1,
            vk::SampleCountFlags::_1);
        let water_state = lumal.create_image_storages (
            lumal_settings.fif,
            vk::ImageType::_2D,
            vk::Format::R16G16B16A16_SFLOAT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: lum_settings.world_size.x*2, 
                height: lum_settings.world_size.y*2, 
                depth: 1},
            1,
            vk::SampleCountFlags::_1);
        let perlin_noise2d = lumal.create_image_storages (
            lumal_settings.fif,
            vk::ImageType::_2D,
            vk::Format::R16G16_SNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: lum_settings.world_size.x, 
                height: lum_settings.world_size.y, 
                depth: 1},
            1,
            vk::SampleCountFlags::_1); //does not matter than much
        let perlin_noise3d = lumal.create_image_storages (
            lumal_settings.fif,
            vk::ImageType::_3D,
            vk::Format::R16G16B16A16_UNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: 32, 
                height: 32, 
                depth: 32},
            1,
            vk::SampleCountFlags::_1); //does not matter than much
        
        return LumIndependentImages {
            grass_state: grass_state.unwrap(),
            water_state: water_state.unwrap(),
            perlin_noise2d: perlin_noise2d.unwrap(),
            perlin_noise3d: perlin_noise3d.unwrap(),
            world: world.unwrap(),
            radiance_cache: radiance_cache.unwrap(),
            origin_block_palette: origin_block_palette.unwrap(),
            lightmap: lightmap.unwrap(),
            // distance_palette: distance_palette.unwrap(),
            // bit_palette: bit_palette.unwrap(),
            material_palette: material_palette.unwrap(),
        };
    }
}