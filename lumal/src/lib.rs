//! Lumal is Vulkan abstraction specifically for Lum renderer
//! It is not trying to be generic, super extendable and scalable
//! "Great idea" of Lumal is init-time resources - by forcing most resources to be created in init-time
//! abstraction can be much simpler

#![feature(optimize_attribute)]
#![feature(const_type_id)]
#![feature(default_field_values)]
#![feature(const_slice_make_iter)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]
#![warn(missing_docs)]
// lumal is divided into files (aka modules)
// this in needed for whole thing to compile
// Rust is so good that figuring it out only took 1 hour

pub mod barriers;
// pub mod blit_copy;
pub mod buffers;
pub mod descriptors;
pub mod images;
pub mod macros;
pub mod pipes;
pub mod renderer;
pub mod rpass;
pub mod samplers;

pub use ash::vk;
use ash::{
    ext::debug_utils,
    khr::{push_descriptor, surface, swapchain},
    prelude::VkResult,
    vk::{
        CommandPool, ConformanceVersion, DebugUtilsObjectNameInfoEXT, DescriptorPool, Extent2D,
        Fence, Format, ImageAspectFlags, PhysicalDevice, Queue, Semaphore, SurfaceKHR,
        SwapchainKHR, EXT_DEBUG_UTILS_NAME, KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME,
        KHR_PORTABILITY_ENUMERATION_NAME,
    },
    Device, Entry, Instance,
};
use containers::Ring;
use gpu_allocator::vulkan::{self as vma, Allocator, AllocatorCreateDesc};
use std::{
    any::{Any, TypeId},
    collections::HashSet,
    ffi::CStr,
    os::raw::c_void,
    process::exit,
};
use winit::{
    dpi::PhysicalSize,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

const VALIDATION_LAYERS: &CStr = c"VK_LAYER_KHRONOS_validation";
const LUNARG_MONITOR_LAYER: &CStr = c"VK_LAYER_LUNARG_monitor";
/// The required device extensions.
const REQUIRED_DEVICE_EXTENSIONS: &[VkExtName] = &[
    VkExtName::from_str("VK_KHR_swapchain\0"),
    VkExtName::from_str("VK_EXT_host_query_reset\0"),
    VkExtName::from_str("VK_KHR_push_descriptor\0"),
];
/// Vulkan SDK version that started requiring the portability subset extension for macOS.
const PORTABILITY_MACOS_VERSION: ConformanceVersion = ConformanceVersion {
    major: 1,
    minor: 3,
    patch: 216,
    subminor: 0,
};
/// number of frames that will be processed concurrently. 2 is perferct - CPU prepares frame N, GPU renders frame N-1
const DEFAULT_FRAMES_IN_FLIGHT: usize = 2;

/// Simple vk::Buffer wrapper
#[derive(Debug)]
pub struct Buffer {
    /// Vulkan buffer
    pub buffer: vk::Buffer,
    /// A for the buffer
    pub allocation: vma::Allocation,
}
impl Default for Buffer {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
            allocation: unsafe { std::mem::zeroed() },
        }
    }
}

/// Simple vk::Image + vk::ImageView + some metadata wrapper
#[derive(Debug)]
pub struct Image {
    /// Vulkan image
    pub image: vk::Image,
    /// Allocation for the image
    pub allocation: vma::Allocation,
    /// "default" (fullsize all aspects) view to the image
    pub view: vk::ImageView,
    // TODO: can we just keep everything in GENERAL?
    /// Current format of the image
    pub format: vk::Format,
    /// Aspects of the image
    pub aspect: vk::ImageAspectFlags,
    /// Size of the image
    pub extent: vk::Extent3D,
}
impl Default for Image {
    fn default() -> Self {
        Self {
            image: Default::default(),
            allocation: unsafe { std::mem::zeroed() },
            view: Default::default(),
            format: Default::default(),
            aspect: Default::default(),
            extent: Default::default(),
        }
    }
}

/// Wrapper around rasterization vk::Pipeline with some metadata, used for simplication of managing pipelines
#[derive(Clone, Debug, Default)]
pub struct RasterPipe {
    /// Pipeline of this pipe
    pub line: vk::Pipeline,
    /// Pipeline Layout for the pipeline of this pipe
    pub layout: vk::PipelineLayout,
    /// FIF amount of descriptor sets to cover CPU-GPU and GPU resources
    /// we move to next set every frame to ensure no concurrent use of the same resource
    // TODO: LCM ??
    pub sets: Ring<vk::DescriptorSet>,
    /// Descriptor Set Layout of each set in sets
    pub set_layout: vk::DescriptorSetLayout,
    /// Renderpass in which this Pipe is going to be used.
    ///
    /// We don't strictly have to store it here, but it simplifies things
    pub renderpass: vk::RenderPass,
    /// Index of the subpass, which this Pipe is going to be used in that renderpass
    pub subpass_id: i32,
}

/// Wrapper around Compute vk::Pipeline with some metadata, used for simplication of managing pipelines
#[derive(Clone, Default)]
pub struct ComputePipe {
    /// Pipeline of this pipe
    pub line: vk::Pipeline,
    /// Pipeline Layout for the pipeline of this pipe
    pub line_layout: vk::PipelineLayout,
    /// FIF amount of descriptor sets to cover CPU-GPU and GPU resources
    /// we move to next set every frame to ensure no concurrent use of the same resource
    // TODO: LCM ??
    pub sets: Ring<vk::DescriptorSet>,
    /// Descriptor Set Layout of each set in sets
    pub set_layout: vk::DescriptorSetLayout,
}

/// Wrapper around Vulkan renderpass with some metadata
#[derive(Default)]
pub struct RenderPass {
    /// The actual vk::RenderPass object
    pub render_pass: vk::RenderPass,
    /// ClearColor values for each attachment in this renderpass
    /// LoadStoreOp::Clear will use them
    pub clear_colors: Vec<vk::ClearValue>,
    /// Framebuffers containing views to all attachments
    ///
    /// there is lcm of them, to cover every combination of attachements
    pub framebuffers: Ring<vk::Framebuffer>,
    /// Size of every attachment in the renderpass
    pub extent: vk::Extent2D,
}

// // Structure for SwapChainSupportDetails
// pub(crate) struct SwapChainSupportDetails {
//     pub capabilities: vk::SurfaceCapabilitiesKHR,
//     pub formats: Vec<vk::SurfaceFormatKHR>,
//     pub present_modes: Vec<vk::PresentModeKHR>,
// }

// impl SwapChainSupportDetails {
//     pub fn is_suitable(&self) -> bool {
//         !self.formats.is_empty() && !self.present_modes.is_empty()
//     }
// }

/// Settings for some runtime properties of Lumal
#[derive(Clone, Copy, Debug, Default)]
pub struct LumalSettings {
    /// how many timestamps to allocate (currently unused)
    pub timestamp_count: i32 = 0,
    /// Same as desired frame latency in wgpu
    /// also means how many resources are going to be in Rings for CPU-GPU resources
    /// also means "how many frames might there be in flight at the same time"
    pub fif: usize = DEFAULT_FRAMES_IN_FLIGHT,
    /// FIFO / Mailbox
    pub vsync: bool = false,
    /// Wether to enable validation layers
    pub debug: bool = false,
    /// wether to profile (with timestamps (currently unused))
    pub profile: bool = false,
    // pub device_features: vk::PhysicalDeviceFeatures,
    // pub device_features11: vk::PhysicalDeviceVulkan11Features,
    // pub device_features12: vk::PhysicalDeviceVulkan12Features,
    // pub physical_features2: vk::PhysicalDeviceFeatures2,
    // pub instance_layers: Vec<*const i8>,
    // pub instance_extensions: Vec<*const i8>,
    // pub device_extensions: Vec<*const i8>,
}

/// Bundle of counters for how many corresponding descriptors we will need.
/// This way we will allocate descriptor pool with exact sizes we need
#[allow(non_snake_case)]
#[derive(Debug, Default)]
pub struct DescriptorCounter {
    COMBINED_IMAGE_SAMPLER: u32,
    INPUT_ATTACHMENT: u32,
    SAMPLED_IMAGE: u32,
    SAMPLER: u32,
    STORAGE_BUFFER: u32,
    STORAGE_BUFFER_DYNAMIC: u32,
    STORAGE_IMAGE: u32,
    STORAGE_TEXEL_BUFFER: u32,
    UNIFORM_BUFFER: u32,
    UNIFORM_BUFFER_DYNAMIC: u32,
    UNIFORM_TEXEL_BUFFER: u32,
}

impl DescriptorCounter {
    fn increment_counter(&mut self, desc_type: vk::DescriptorType) {
        match desc_type {
            vk::DescriptorType::SAMPLER => self.SAMPLER += 1,
            vk::DescriptorType::COMBINED_IMAGE_SAMPLER => self.COMBINED_IMAGE_SAMPLER += 1,
            vk::DescriptorType::SAMPLED_IMAGE => self.SAMPLED_IMAGE += 1,
            vk::DescriptorType::STORAGE_IMAGE => self.STORAGE_IMAGE += 1,
            vk::DescriptorType::UNIFORM_TEXEL_BUFFER => self.UNIFORM_TEXEL_BUFFER += 1,
            vk::DescriptorType::STORAGE_TEXEL_BUFFER => self.STORAGE_TEXEL_BUFFER += 1,
            vk::DescriptorType::UNIFORM_BUFFER => self.UNIFORM_BUFFER += 1,
            vk::DescriptorType::STORAGE_BUFFER => self.STORAGE_BUFFER += 1,
            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC => self.UNIFORM_BUFFER_DYNAMIC += 1,
            vk::DescriptorType::STORAGE_BUFFER_DYNAMIC => self.STORAGE_BUFFER_DYNAMIC += 1,
            vk::DescriptorType::INPUT_ATTACHMENT => self.INPUT_ATTACHMENT += 1,
            _ => {
                panic!("Unknown descriptor type");
            }
        }
    }

    fn get_pool_sizes(&self) -> Vec<vk::DescriptorPoolSize> {
        let mut pool_sizes = Vec::new();

        macro_rules! add_pool_size_if_not_zero {
            ($name:ident) => {
                if self.$name != 0 {
                    pool_sizes.push(vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::$name,
                        descriptor_count: self.$name,
                    });
                }
            };
        }

        add_pool_size_if_not_zero!(SAMPLER);
        add_pool_size_if_not_zero!(COMBINED_IMAGE_SAMPLER);
        add_pool_size_if_not_zero!(SAMPLED_IMAGE);
        add_pool_size_if_not_zero!(STORAGE_IMAGE);
        add_pool_size_if_not_zero!(UNIFORM_TEXEL_BUFFER);
        add_pool_size_if_not_zero!(STORAGE_TEXEL_BUFFER);
        add_pool_size_if_not_zero!(UNIFORM_BUFFER);
        add_pool_size_if_not_zero!(STORAGE_BUFFER);
        add_pool_size_if_not_zero!(UNIFORM_BUFFER_DYNAMIC);
        add_pool_size_if_not_zero!(STORAGE_BUFFER_DYNAMIC);
        add_pool_size_if_not_zero!(INPUT_ATTACHMENT);

        pool_sizes
    }
}

// TODO: not copy? or Copy image?
/// Request for destroying an Image (after some time)
#[derive(Default, Debug)]
pub struct ImageDeletion {
    /// vk::Image to destroy
    pub image: vk::Image,
    /// Allocation of that image
    pub allocation: vma::Allocation,
    /// View to that image
    pub view: vk::ImageView, // Main view
    /// How many frames (including this one) should the image be kept alive for.
    ///
    /// This is needed becuase immediately destroying image will make it invalid for already in flight frames
    pub lifetime: i32,
}

impl Clone for ImageDeletion {
    fn clone(&self) -> Self {
        Self {
            image: self.image,
            view: self.view,
            lifetime: self.lifetime,
            // TODO:
            allocation: unsafe { std::ptr::read(&self.allocation) },
        }
    }
}

/// Request for destroying a Image (after some time)
#[derive(Default, Debug)]
pub struct BufferDeletion {
    /// Lumal Buffer to destroy
    pub buffer: Buffer,
    /// How many frames (including this one) should the buffer be kept alive for.
    ///
    /// This is needed becuase immediately destroying buffer will make it invalid for already in flight frames
    pub lifetime: i32,
}

pub struct Renderer {
    /// Vulkan memory allocator (for buffer and images)
    /// we do not allocate a lot in runtime so small overhead is fine
    pub allocator: vma::Allocator,
    /// Settings for some runtime properties of Lumal
    pub settings: LumalSettings,

    /// Current surface to render
    pub surface: vk::SurfaceKHR,
    /// Picked physical device
    pub physical_device: vk::PhysicalDevice,
    /// Queue for rasterization and compute (maybe TODO: ?)
    pub graphics_queue: vk::Queue,
    /// Queue for presentation to swapchain
    pub present_queue: vk::Queue,
    /// picked swapchain format
    pub swapchain_format: vk::Format,
    /// Size of the swapchain images, specified / queried from windows
    pub swapchain_extent: vk::Extent2D,
    /// Swap chain of image used for presentation
    pub swapchain: vk::SwapchainKHR,
    /// Actual images of the swapchain
    pub swapchain_images: Ring<crate::Image>,
    /// Single (shared between all cmd buffers) CommandPool
    pub command_pool: vk::CommandPool,
    /// Semaphore, which is signaled when corresponding swapchain image is available to render to (and present)
    pub image_available_semaphores: Ring<vk::Semaphore>,
    /// Semaphore, which is signaled when corresponding frame is finished rendering
    pub render_finished_semaphores: Ring<vk::Semaphore>,
    /// Fences for syncronizing different frames in flight
    /// (example for FIF=2 - prevents CPU from starting frame 0 when CPU is done with frame 1, but GPU is not done with frame 0 yet (and GPU did not even start frame 1))
    pub in_flight_fences: Ring<vk::Fence>,
    /// Pool for all descriptor sets - allocated once. Its size is precomputed for simplicity (so no runtime descriptor sets (re)allocation)
    pub descriptor_pool: vk::DescriptorPool,
    /// Internal vk entry point
    pub entry: Entry,
    /// Internal Vulkan instance
    pub instance: Instance,
    /// Internal Vulkan (logical) device
    pub device: Device,
    pub surface_loader: surface::Instance,
    pub swapchain_loader: swapchain::Device,
    pub debug_utils_loader: debug_utils::Instance,
    pub debug_utils_device_loader: debug_utils::Device,
    pub push_descriptors_loader: push_descriptor::Device,

    /// Global counter of rendered frames, mostly for rng
    pub frame: i32,
    /// Current frame index in FIF. Equals to (frame % fif)
    pub image_index: u32,
    /// If swapchain should be recreated (as soon as possible)
    pub should_recreate: bool,
    /// Counts how many descriptors of each type are anounced for allocating descriptor pool
    pub descriptor_counter: DescriptorCounter,
    /// Counts how many descriptor sets total are anounced
    pub descriptor_sets_count: u32,

    /// Queue of deferred (delayed) GPU-side deletion of buffers
    /// this is not immediate because there are some frames in flight that might still be using resources
    pub buffer_deletion_queue: Vec<BufferDeletion>,
    /// Queue of deferred (delayed) GPU-side deletion of images
    /// this is not immediate because there are some frames in flight that might still be using resources
    pub image_deletion_queue: Vec<ImageDeletion>,
}

impl Renderer {
    /// Constructs new Renderer for rendering into given Window
    /// You should probably construct it only once
    pub fn new(settings: &LumalSettings, window: &Window, size: PhysicalSize<u32>) -> Renderer {
        println!("Starting app.");

        unsafe {
            let entry = Entry::load().expect("Failed to load Vulkan entry point");
            let instance = Renderer::create_instance(window, &entry, settings.debug);
            let surface = ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                None,
            )
            .unwrap();
            let physical_device = pick_physical_device(&instance, &entry, &surface);
            let (device, graphics_queue, present_queue) =
                create_logical_device(&entry, &instance, &surface, &physical_device);

            let allocator = Allocator::new(&AllocatorCreateDesc {
                instance: instance.clone(),
                device: device.clone(),
                physical_device: physical_device,
                debug_settings: Default::default(),
                buffer_device_address: false,
                allocation_sizes: Default::default(),
            })
            .unwrap();

            let (swapchain, swapchain_images, swapchain_extent, swapchain_format) =
                create_swapchain(
                    settings,
                    size,
                    &instance,
                    &entry,
                    &device,
                    &surface,
                    &physical_device,
                );

            let command_pool =
                create_command_pool(&instance, &entry, &device, &surface, &physical_device);
            let (image_available_semaphores, render_finished_semaphores, in_flight_fences) =
                create_sync_objects(&device);

            // these are loading functions (i mean this code is loading function pointers) (separate because they might not be presented)
            let surface_loader = surface::Instance::new(&entry, &instance);
            let swapchain_loader = swapchain::Device::new(&instance, &device);
            let debug_utils_loader = debug_utils::Instance::new(&entry, &instance);
            let debug_utils_device_loader = debug_utils::Device::new(&instance, &device);
            let push_descriptors_loader = push_descriptor::Device::new(&instance, &device);

            let descriptor_pool = DescriptorPool::null();

            Renderer {
                allocator,
                entry,
                instance,
                device,
                surface,
                physical_device,
                graphics_queue,
                present_queue,
                swapchain_format,
                swapchain_extent,
                swapchain,
                swapchain_images,
                command_pool,
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
                descriptor_pool,

                frame: 0,
                should_recreate: false,
                settings: *settings,
                descriptor_counter: DescriptorCounter::default(),
                descriptor_sets_count: 0,
                image_index: 0, // cause just init'ed, no descriptor setup deferred yet
                // delayed_descriptor_setups: vec![],
                buffer_deletion_queue: vec![],
                image_deletion_queue: vec![],

                surface_loader,
                swapchain_loader,
                debug_utils_loader,
                debug_utils_device_loader,
                push_descriptors_loader,
            }
        }
    }

    /// Creates Vulkan instance
    pub fn create_instance(window: &Window, entry: &Entry, validation: bool) -> Instance {
        let application_info = vk::ApplicationInfo {
            p_application_name: c"renderer_vk".as_ptr(),
            application_version: vk::make_api_version(0, 1, 3, 0),
            p_engine_name: c"No Engine".as_ptr(),
            engine_version: vk::make_api_version(0, 1, 3, 0),
            api_version: vk::make_api_version(0, 1, 3, 0),
            ..Default::default()
        };

        let available_layers = unsafe {
            entry
                .enumerate_instance_layer_properties()
                .unwrap()
                .iter()
                .map(|l| l.layer_name)
                .collect::<HashSet<[i8; 256]>>()
        };

        let mut validation_layers = VALIDATION_LAYERS
            .to_bytes_with_nul()
            .iter()
            .map(|c| *c as i8)
            .collect::<Vec<_>>();
        validation_layers.resize(256, 0);
        let validation_layers: [i8; 256] = validation_layers.try_into().unwrap();

        if validation && !available_layers.contains(&validation_layers) {
            panic!("Validation layers requested but not supported");
        }

        let mut layers = if validation {
            vec![VALIDATION_LAYERS.as_ptr()]
        } else {
            Vec::new()
        };

        let mut lunarg_monitor_layer = LUNARG_MONITOR_LAYER
            .to_bytes_with_nul()
            .iter()
            .map(|c| *c as i8)
            .collect::<Vec<_>>();
        lunarg_monitor_layer.resize(256, 0);
        let lunarg_monitor_layer: [i8; 256] = lunarg_monitor_layer.try_into().unwrap();

        if available_layers.contains(&lunarg_monitor_layer) {
            layers.push(LUNARG_MONITOR_LAYER.as_ptr());
        }

        let mut extensions =
            ash_window::enumerate_required_extensions(window.display_handle().unwrap().as_raw())
                .unwrap()
                .to_vec();

        // Required by Vulkan SDK on macOS since 1.3.216 (i actually have no idea how this works, i dont own any apple devices).
        let flags = if cfg!(target_os = "macos")
            && unsafe { entry.try_enumerate_instance_version().unwrap().unwrap() }
                >= PORTABILITY_MACOS_VERSION.major as u32
        {
            println!("Enabling extensions for macOS portability.");

            extensions.push(KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME.as_ptr());
            extensions.push(KHR_PORTABILITY_ENUMERATION_NAME.as_ptr());
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::empty()
        };

        if validation {
            extensions.push(EXT_DEBUG_UTILS_NAME.as_ptr());
        }

        let mut info = vk::InstanceCreateInfo {
            p_application_info: &application_info,
            enabled_layer_count: layers.len() as u32,
            pp_enabled_layer_names: layers.as_ptr(),
            enabled_extension_count: extensions.len() as u32,
            pp_enabled_extension_names: extensions.as_ptr(),
            flags,
            ..Default::default()
        };

        let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            pfn_user_callback: Some(debug_callback),
            ..Default::default()
        };

        if validation {
            info.p_next = &mut debug_info as *mut _ as *mut c_void;
        }

        unsafe { entry.create_instance(&info, None).unwrap() }
    }

    /// Destroys the renderer.
    /// Buffers, images, pipelines - everything created manually should be destroyed manually before this call
    pub unsafe fn destroy(mut self) {
        self.process_deletion_queues_untill_all_done();
        {
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.descriptor_pool = vk::DescriptorPool::null();
        }
        self.device.destroy_command_pool(self.command_pool, None);
        self.destroy_swapchain();
        self.destroy_sync_primitives();

        // i FUCKING HATE that they implement it in a drop
        // how the fuck am i supposed to do it? Put it 7 lines below and it fucking segfaults
        std::mem::drop(self.allocator);

        self.device.destroy_device(None);
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
        self.instance.destroy_instance(None);
    }

    unsafe fn destroy_swapchain(&self) {
        self.swapchain_images
            .iter()
            .for_each(|v| self.device.destroy_image_view(v.view, None));
        unsafe {
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }

    unsafe fn destroy_sync_primitives(&self) {
        self.in_flight_fences.iter().for_each(|f| self.device.destroy_fence(*f, None));
        self.render_finished_semaphores
            .iter()
            .for_each(|s| self.device.destroy_semaphore(*s, None));
        self.image_available_semaphores
            .iter()
            .for_each(|s| self.device.destroy_semaphore(*s, None));
    }

    /// Creates command buffer for single-time use (like loading resource in runtime)
    /// You should use existing command buffers if possible (because likely you will stall pipeline waiting for this to execute)
    pub fn begin_single_time_command_buffer(&self) -> vk::CommandBuffer {
        let alloc_info = vk::CommandBufferAllocateInfo {
            level: vk::CommandBufferLevel::PRIMARY,
            command_pool: self.command_pool,
            command_buffer_count: 1,
            ..Default::default()
        };
        let command_buffers = unsafe { self.device.allocate_command_buffers(&alloc_info).unwrap() };
        let command_buffer = command_buffers[0];
        unsafe {
            self.device
                .begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo::default())
                .unwrap();
        }
        command_buffer
    }

    /// Executes single-time-use command buffer on graphics queue. Stalls the pipeline until done. You should avoid using it
    pub fn end_single_time_command_buffer(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device.end_command_buffer(command_buffer).unwrap();
        }
        let submit_info = vk::SubmitInfo {
            wait_semaphore_count: 0,
            signal_semaphore_count: 0,
            command_buffer_count: 1,
            p_command_buffers: &command_buffer,
            ..Default::default()
        };
        unsafe {
            // grapics is also capable of compute and transfer
            self.device
                .queue_submit(self.graphics_queue, &[submit_info], vk::Fence::null())
                .unwrap();
            // yep unoptimal but you are not supposed to use this at all
            self.device.queue_wait_idle(self.graphics_queue).unwrap();
        }
        unsafe {
            self.device.free_command_buffers(self.command_pool, &[command_buffer]);
        }
    }

    /// Binds given ComputePipe to the command buffer, along with current() descriptor set
    pub fn bind_compute_pipe(&self, cmb: &vk::CommandBuffer, pipe: &ComputePipe) {
        unsafe {
            self.device.cmd_bind_pipeline(*cmb, vk::PipelineBindPoint::COMPUTE, pipe.line);
            self.device.cmd_bind_descriptor_sets(
                *cmb,
                vk::PipelineBindPoint::COMPUTE,
                pipe.line_layout,
                0,
                &[*pipe.sets.current()],
                &[],
            );
        }
    }

    /// Binds given RasterPipe to the command buffer, along with current() descriptor set
    pub fn bind_raster_pipe(&self, cmb: &vk::CommandBuffer, pipe: &RasterPipe) {
        unsafe {
            self.device.cmd_bind_pipeline(*cmb, vk::PipelineBindPoint::GRAPHICS, pipe.line);
            self.device.cmd_bind_descriptor_sets(
                *cmb,
                vk::PipelineBindPoint::GRAPHICS,
                pipe.layout,
                0,
                &[*pipe.sets.current()],
                &[],
            );
        }
    }

    /// creates primary command buffers. Lumal does not interact with non-primary command buffers
    pub fn create_command_buffer(&self) -> Ring<vk::CommandBuffer> {
        let info = vk::CommandBufferAllocateInfo {
            command_pool: self.command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: self.settings.fif as u32,
            ..Default::default()
        };

        Ring::from_vec(unsafe { self.device.allocate_command_buffers(&info).unwrap() })
    }

    /// Destroys command buffers (thin wrapper)
    pub fn destroy_command_buffer(&self, compute_command_buffers: &Ring<vk::CommandBuffer>) {
        unsafe {
            self.device
                .free_command_buffers(self.command_pool, compute_command_buffers.as_slice())
        };
    }

    /// Create single-time-use cmd buffer and transition image layout with barriers. You should avoid using it
    pub fn transition_image_layout_single_time(
        &self,
        image: &Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        let command_buffer = self.begin_single_time_command_buffer();
        let barrier = vk::ImageMemoryBarrier {
            old_layout,
            new_layout,
            image: image.image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: image.aspect,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_access_mask: vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            dst_access_mask: vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            ..Default::default()
        };
        unsafe {
            self.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::DependencyFlags::empty(),
                &[] as &[vk::MemoryBarrier],
                &[] as &[vk::BufferMemoryBarrier],
                &[barrier],
            );
        };
        self.end_single_time_command_buffer(command_buffer);
    }

    /// Create single-time-use cmd buffer and copy buffer data to an image. You should avoid using it
    pub fn copy_buffer_to_image_single_time(
        &self,
        buffer: vk::Buffer,
        img: &Image,
        extent: vk::Extent3D,
    ) {
        let command_buffer = self.begin_single_time_command_buffer();
        let copy_region = vk::BufferImageCopy {
            image_extent: extent,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            buffer_offset: 0,
            ..Default::default()
        };
        unsafe {
            self.device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                img.image,
                vk::ImageLayout::GENERAL,
                &[copy_region],
            );
        };
        self.end_single_time_command_buffer(command_buffer);
    }

    /// Create single-time-use cmd buffer and copy buffer data to another buffer. You should avoid using it
    pub fn copy_buffer_to_buffer_single_time(
        &mut self,
        src_buffer: vk::Buffer,
        dst_buffer: vk::Buffer,
        size: vk::DeviceSize,
    ) {
        let command_buffer = self.begin_single_time_command_buffer();
        unsafe {
            self.device.cmd_copy_buffer(
                command_buffer,
                src_buffer,
                dst_buffer,
                &[vk::BufferCopy {
                    src_offset: 0,
                    dst_offset: 0,
                    size,
                }],
            );
        };
        self.end_single_time_command_buffer(command_buffer);
    }

    /// Iterate over all deletion, queues decrease lifetime of resources left, and destroy resources whose lifetime is over
    pub fn process_deletion_queues(&mut self) {
        let mut write_index = 0;
        let len = self.buffer_deletion_queue.len();
        let mut i = 0;

        while i < len {
            if self.buffer_deletion_queue[i].lifetime > 0 {
                // If keeping the buffer, swap it to write_index if needed
                if i != write_index {
                    self.buffer_deletion_queue.swap(i, write_index);
                }
                self.buffer_deletion_queue[write_index].lifetime -= 1;
                write_index += 1;
            } else {
                // Destroy the buffer before overwriting
                let buffer = std::mem::take(&mut self.buffer_deletion_queue[i].buffer);
                self.allocator.free(buffer.allocation).unwrap();
                unsafe { self.device.destroy_buffer(buffer.buffer, None) };
            }
            i += 1;
        }
        self.buffer_deletion_queue.truncate(write_index);

        let mut write_index = 0;
        let len = self.image_deletion_queue.len();
        let mut i = 0;

        while i < len {
            if self.image_deletion_queue[i].lifetime > 0 {
                // If keeping the image, swap it to write_index if needed
                if i != write_index {
                    self.image_deletion_queue.swap(i, write_index);
                }
                self.image_deletion_queue[write_index].lifetime -= 1;
                write_index += 1;
            } else {
                // Destroy the image and view before overwriting
                let image = self.image_deletion_queue[i].image;
                let view = self.image_deletion_queue[i].view;
                // let mip_views = std::mem::take(&mut self.image_deletion_queue[i].mip_views);
                let allocation = std::mem::take(&mut self.image_deletion_queue[i].allocation);
                unsafe {
                    self.allocator.free(allocation).unwrap();
                    self.device.destroy_image_view(view, None);
                    // for mip_view in mip_views {
                    //     self.device.destroy_image_view(mip_view, None);
                    // }
                    self.device.destroy_image(image, None);
                }
            }
            i += 1;
        }
        self.image_deletion_queue.truncate(write_index);
    }

    /// Process deletion queues untill all resources are destroyed
    pub fn process_deletion_queues_untill_all_done(&mut self) {
        while !self.buffer_deletion_queue.is_empty() || !self.image_deletion_queue.is_empty() {
            self.process_deletion_queues();
        }
    }

    /// Recreates swapchain, its images and all dependent resources
    pub fn recreate_swapchain(&mut self, size: PhysicalSize<u32>) {
        // like catching an exception
        self.should_recreate = false;

        if size.width == 0 || size.height == 0 {
            // like throwing an exception back
            // TODO: do not render until recreated but invalid?
            self.should_recreate = true;
            return;
        }

        unsafe {
            self.device.device_wait_idle().unwrap();

            // in past, instead of manually recreating window, i used to pass lambdas to renderer for everything else that needs to be recreated
            // match self.destroy_swapchain_dependent_resources {
            //     Some(ref mut fun) => fun(window),
            //     None => { /* not fun */ }
            // }
            // TODO: investigate storing settings/arenas/lambdas more for auto-recreation

            self.destroy_swapchain();

            self.device.device_wait_idle().unwrap();

            let (swapchain, images, extent, format) = create_swapchain(
                &self.settings,
                size,
                &self.instance,
                &self.entry,
                &self.device,
                &self.surface,
                &self.physical_device,
            );

            self.swapchain = swapchain;
            self.swapchain_images = images;
            self.swapchain_extent = extent;
            self.swapchain_format = format;

            self.image_index = 0;
            self.should_recreate = false;
        };
    }

    /// Finds first image format from given candidates that is supported by device (for given type, tiling and usage)
    pub fn find_supported_format(
        &self,
        candidates: &[vk::Format],
        ty: vk::ImageType,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
    ) -> Option<vk::Format> {
        for &format in candidates {
            let result = unsafe {
                self.instance.get_physical_device_image_format_properties(
                    self.physical_device,
                    format,
                    ty,
                    tiling,
                    usage,
                    vk::ImageCreateFlags::empty(),
                )
            };

            if result.is_ok() {
                return Some(format);
            }
        }
        None
    }

    /// Vulkan set viewport/scissors wrapper
    pub fn cmd_set_viewport(&self, cmdbuf: vk::CommandBuffer, width: u32, height: u32) {
        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width, height },
        };

        unsafe {
            self.device.cmd_set_viewport(cmdbuf, 0, &[viewport]);
            self.device.cmd_set_scissor(cmdbuf, 0, &[scissor]);
        }
    }

    /// Gives name(label) for Vulkan object, visible validation layers
    #[cfg(feature = "debug_validation_names")]
    pub fn name_var(&self, o_type: vk::ObjectType, o: u64, o_name: &str) {
        let name_info = DebugUtilsObjectNameInfoEXT {
            object_type: o_type,
            object_handle: o,
            p_object_name: o_name.as_bytes().as_ptr() as *const i8,
            ..Default::default()
        };
        unsafe {
            self.debug_utils_device_loader.set_debug_utils_object_name(&name_info).unwrap();
        }
    }
}

/// Converts Rust type into vk::ObjectType enum. Used for giving names to Vulkan objects
#[rustfmt::skip]
pub fn get_vulkan_object_type<T: Any>(_object: &T) -> Option<vk::ObjectType> {
    let type_id = TypeId::of::<T>();

    use vk::Buffer;
    use vk::*;

    #[rustfmt::skip]
    macro_rules! elif {
        ($type_id:ident, $type_1:ident, $type_2:expr) => {
            if $type_id == TypeId::of::<$type_1>() {
                return Some($type_2);
            }
        };
    }

    elif!(type_id, Instance, vk::ObjectType::INSTANCE);
    elif!(type_id, PhysicalDevice, vk::ObjectType::PHYSICAL_DEVICE);
    elif!(type_id, Device, vk::ObjectType::DEVICE);
    elif!(type_id, Queue, vk::ObjectType::QUEUE);
    elif!(type_id, Semaphore, vk::ObjectType::SEMAPHORE);
    elif!(type_id, CommandBuffer, vk::ObjectType::COMMAND_BUFFER);
    elif!(type_id, Fence, vk::ObjectType::FENCE);
    elif!(type_id, DeviceMemory, vk::ObjectType::DEVICE_MEMORY);
    elif!(type_id, Buffer, vk::ObjectType::BUFFER);
    elif!(type_id, Image, vk::ObjectType::IMAGE);
    elif!(type_id, Event, vk::ObjectType::EVENT);
    elif!(type_id, QueryPool, vk::ObjectType::QUERY_POOL);
    elif!(type_id, BufferView, vk::ObjectType::BUFFER_VIEW);
    elif!(type_id, ImageView, vk::ObjectType::IMAGE_VIEW);
    elif!(type_id, ShaderModule, vk::ObjectType::SHADER_MODULE);
    elif!(type_id, PipelineLayout, vk::ObjectType::PIPELINE_LAYOUT);
    elif!(type_id, RenderPass, vk::ObjectType::RENDER_PASS);
    elif!(type_id, Pipeline, vk::ObjectType::PIPELINE);
    elif!(type_id, DescriptorSetLayout, vk::ObjectType::DESCRIPTOR_SET_LAYOUT);
    elif!(type_id, Sampler, vk::ObjectType::SAMPLER);
    elif!(type_id, DescriptorPool, vk::ObjectType::DESCRIPTOR_POOL);
    elif!(type_id, DescriptorSet, vk::ObjectType::DESCRIPTOR_SET);
    elif!(type_id, Framebuffer, vk::ObjectType::FRAMEBUFFER);
    elif!(type_id, CommandPool, vk::ObjectType::COMMAND_POOL);
    elif!(type_id, SurfaceKHR, vk::ObjectType::SURFACE_KHR);
    elif!(type_id, SwapchainKHR, vk::ObjectType::SWAPCHAIN_KHR);
    elif!(type_id, DebugUtilsMessengerEXT, vk::ObjectType::DEBUG_UTILS_MESSENGER_EXT);

    Some(vk::ObjectType::UNKNOWN)
}

/// Gives name (label) to Vulkan object, visible to validation lyaers.
#[macro_export]
macro_rules! set_debug_name {
    ($lumal:expr, $variable:expr, $debug_name:expr) => {{
        #[cfg(feature = "debug_validation_names")]
        if let Some(debug_name) = $debug_name {
            let object_handle = $variable.as_raw();
            let object_type_option = $crate::get_vulkan_object_type($variable);

            if let Some(object_type_vk) = object_type_option {
                let object_type_debug_report = object_type_vk;
                $lumal.name_var(object_type_debug_report, object_handle, debug_name);
            }
        }
    }};
}

/// Gives names (labels) to Vulkan objects, visible to validation lyaers.
#[macro_export]
macro_rules! set_debug_names {
    ($renderer:expr, $base_name:expr, $( ($object:expr, $suffix:expr) ),*) => {
        #[cfg(feature = "debug_validation_names")]
        {
            if let Some(name) = $base_name {
                $(
                    let debug_name = format!("{}{}\0", name, $suffix);
                    let object_handle = $object.as_raw();

                    if let Some(object_type_vk) = $crate::get_vulkan_object_type($object) {
                        $renderer.name_var(object_type_vk, object_handle, &debug_name);
                    }
                )*
            }
        }
    };
}

/// Callback function called by validation layers to print messages
// TODO: optional for binary size?
extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.p_message) }.to_string_lossy();

    if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR)
        || severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING)
    {
        println!("({type_:?}) {message}");
    }

    vk::FALSE
}

/// Picks a suitable physical device.
unsafe fn pick_physical_device(
    instance: &Instance,
    entry: &Entry,
    surface: &SurfaceKHR,
) -> PhysicalDevice {
    for physical_device in instance.enumerate_physical_devices().unwrap() {
        let properties = instance.get_physical_device_properties(physical_device);
        if let Err(err) = check_physical_device(instance, entry, surface, &physical_device) {
            //TODO:
            println!(
                "Skipping physical device: `{}`, error: {}",
                properties.device_name_as_c_str().unwrap().to_string_lossy(),
                err
            );
        } else {
            println!(
                "Selected physical device (`{}`).",
                properties.device_name_as_c_str().unwrap().to_string_lossy()
            );
            return physical_device;
        }
    }

    panic!("Failed to find suitable physical device")
}

/// Checks that a physical device is suitable.
unsafe fn check_physical_device(
    instance: &Instance,
    entry: &Entry,
    surface: &SurfaceKHR,
    physical_device: &vk::PhysicalDevice,
) -> VkResult<()> {
    QueueFamilyIndices::get(instance, entry, surface, physical_device)?;
    check_physical_device_extensions(instance, physical_device)?;
    let support = SwapchainSupport::get(instance, entry, physical_device, surface)?;
    if support.formats.is_empty() || support.present_modes.is_empty() {
        return Err(vk::Result::ERROR_UNKNOWN);
    }
    Ok(())
}

/// Wrapper around the raw Vulkan extension name
#[derive(Copy, Clone)]
struct VkExtName([i8; 256]);
impl VkExtName {
    /// Build from raw array returned by Vulkan
    pub fn from_raw(raw: [i8; 256]) -> Self {
        VkExtName(raw)
    }

    /// Build from &str
    pub const fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let mut arr = [0i8; 256];
        let mut i = 0;
        // copy up to the length of the string
        while i < bytes.len() {
            arr[i] = bytes[i] as i8;
            i += 1;
        }
        // the byte at arr[bytes.len()] is still 0
        VkExtName(arr)
    }

    /// Slice to "real" bytes - up to the first 0
    fn bytes(&self) -> &[i8] {
        let raw = &self.0;
        let len = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
        &raw[0..len]
    }
}

// TODO: is this safe?
impl std::fmt::Display for VkExtName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Get the slice up to the first null byte
        let raw = self.bytes();

        let bytes = raw.iter().map(|&b| b as u8).collect::<Vec<u8>>();
        let name = std::str::from_utf8(&bytes).unwrap_or("<invalid utf8>");

        write!(f, "{name:?}")
    }
}

impl PartialEq for VkExtName {
    fn eq(&self, other: &Self) -> bool {
        self.bytes() == other.bytes()
    }
}
impl Eq for VkExtName {}
impl std::hash::Hash for VkExtName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // only hash the bytes up to the null terminator
        for &b in self.bytes() {
            state.write_i8(b);
        }
    }
}

/// Checks that a physical device supports the required device extensions.
unsafe fn check_physical_device_extensions(
    instance: &Instance,
    physical_device: &vk::PhysicalDevice,
) -> VkResult<()> {
    let available_exts: HashSet<VkExtName> = instance
        .enumerate_device_extension_properties(*physical_device)?
        .iter()
        .map(|prop| VkExtName::from_raw(prop.extension_name))
        .collect::<HashSet<_>>();

    for &req in REQUIRED_DEVICE_EXTENSIONS.iter() {
        if !available_exts.contains(&req) {
            eprintln!("Missing required extension: {}", &req);
            eprintln!("Available extensions:");
            for ext in &available_exts {
                let bytes: Vec<u8> = ext.bytes().iter().map(|&c| c as u8).collect();
                eprintln!("  - {}", String::from_utf8_lossy(&bytes));
            }
            exit(34);
        }
    }

    Ok(())
}

/// Creates a logical device for the picked physical device.
unsafe fn create_logical_device(
    entry: &Entry,
    instance: &Instance,
    surface: &SurfaceKHR,
    physical_device: &PhysicalDevice,
) -> (Device, Queue, Queue) {
    let indices = QueueFamilyIndices::get(instance, entry, surface, physical_device).unwrap();

    let mut unique_indices = HashSet::new();
    unique_indices.insert(indices.graphics);
    unique_indices.insert(indices.present);

    let queue_priorities = &[1.0];
    let queue_infos = unique_indices
        .iter()
        .map(|i| vk::DeviceQueueCreateInfo {
            queue_family_index: *i,
            queue_count: 1,
            p_queue_priorities: queue_priorities.as_ptr(),
            ..Default::default()
        })
        .collect::<Vec<_>>();

    let mut extensions =
        REQUIRED_DEVICE_EXTENSIONS.iter().map(|n| n.0.as_ptr()).collect::<Vec<_>>();

    // Required by Vulkan SDK on macOS since 1.3.216.
    if cfg!(target_os = "macos")
        && entry.try_enumerate_instance_version().unwrap().unwrap()
            >= PORTABILITY_MACOS_VERSION.major as u32
    {
        extensions.push(vk::KHR_PORTABILITY_SUBSET_NAME.as_ptr());
    }

    // TODO: unhardcode
    let features = vk::PhysicalDeviceFeatures {
        sampler_anisotropy: vk::TRUE,
        shader_int16: vk::TRUE,
        geometry_shader: vk::TRUE,
        vertex_pipeline_stores_and_atomics: vk::TRUE,
        independent_blend: vk::TRUE,
        ..Default::default()
    };

    let mut features11 = vk::PhysicalDeviceVulkan11Features {
        storage_push_constant16: vk::TRUE,
        ..Default::default()
    };

    let mut features12 = vk::PhysicalDeviceVulkan12Features {
        storage_push_constant8: vk::TRUE,
        storage_buffer8_bit_access: vk::TRUE,
        shader_int8: vk::TRUE,
        ..Default::default()
    };

    features12.p_next = &mut features11 as *mut vk::PhysicalDeviceVulkan11Features as *mut c_void;

    let mut features2 = vk::PhysicalDeviceFeatures2 {
        features,
        p_next: &mut features12 as *mut vk::PhysicalDeviceVulkan12Features as *mut c_void,
        ..Default::default()
    };

    let info = vk::DeviceCreateInfo {
        queue_create_info_count: queue_infos.len() as u32,
        p_queue_create_infos: queue_infos.as_ptr(),
        enabled_extension_count: extensions.len() as u32,
        pp_enabled_extension_names: extensions.as_ptr(),
        p_next: &mut features2 as *mut vk::PhysicalDeviceFeatures2 as *mut c_void,
        ..Default::default()
    };

    let device = instance.create_device(*physical_device, &info, None).unwrap();

    let graphics_queue = device.get_device_queue(indices.graphics, 0);
    let present_queue = device.get_device_queue(indices.present, 0);

    (device, graphics_queue, present_queue)
}

/// Creates a swapchain and swapchain images.
unsafe fn create_swapchain(
    settings: &LumalSettings,
    size: PhysicalSize<u32>,
    instance: &Instance,
    entry: &Entry,
    device: &Device,
    surface: &SurfaceKHR,
    physical_device: &vk::PhysicalDevice,
) -> (SwapchainKHR, Ring<Image>, Extent2D, Format) {
    let indices = QueueFamilyIndices::get(instance, entry, surface, physical_device).unwrap();
    let support = SwapchainSupport::get(instance, entry, physical_device, surface).unwrap();
    let surface_format = get_swapchain_surface_format(&support.formats);
    let present_mode = get_swapchain_present_mode(&support.present_modes);
    let extent = get_swapchain_extent(size, support.capabilities);

    let max_image_count = if support.capabilities.max_image_count != 0 {
        support.capabilities.max_image_count
    } else {
        u32::MAX
    };
    let image_count =
        ((support.capabilities.min_image_count).max(settings.fif as u32)).min(max_image_count);

    let mut queue_family_indices = vec![];
    let image_sharing_mode = if indices.graphics != indices.present {
        queue_family_indices.push(indices.graphics);
        queue_family_indices.push(indices.present);
        vk::SharingMode::CONCURRENT
    } else {
        vk::SharingMode::EXCLUSIVE
    };

    let info = vk::SwapchainCreateInfoKHR {
        surface: *surface,
        min_image_count: image_count,
        image_format: surface_format.format,
        image_color_space: surface_format.color_space,
        image_extent: extent,
        image_array_layers: 1,
        image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
        image_sharing_mode,
        queue_family_index_count: queue_family_indices.len() as u32,
        p_queue_family_indices: queue_family_indices.as_ptr(),
        pre_transform: support.capabilities.current_transform,
        composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
        present_mode,
        clipped: vk::TRUE,
        ..Default::default()
    };

    let swapchain_loader = swapchain::Device::new(instance, device);
    let swapchain = swapchain_loader.create_swapchain(&info, None).unwrap();

    let swapchain_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();

    let swapchain_images = Ring::from_vec(
        swapchain_images
            .iter()
            .enumerate()
            .map(|(i, vk_img)| {
                // TODO: enumerate only when feature flag?
                let components = vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                };

                let subresource_range = vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                };

                let info = vk::ImageViewCreateInfo {
                    image: *vk_img,
                    view_type: vk::ImageViewType::TYPE_2D,
                    format: surface_format.format,
                    components,
                    subresource_range,
                    ..Default::default()
                };

                let view = device.create_image_view(&info, None).unwrap();

                // manually give swapchain image views debug names
                #[cfg(feature = "debug_validation_names")]
                {
                    let debug_name = format!("Swapchain Image View {}\0", i);
                    let object_handle = (&view).as_raw();
                    let object_type_option = crate::get_vulkan_object_type((&view));
                    if let Some(object_type_vk) = object_type_option {
                        let object_type_debug_report = object_type_vk;
                        let name_info = DebugUtilsObjectNameInfoEXT {
                            // TODO: get rid of vk, trait get_s_type & get_type_name
                            object_type: object_type_vk,
                            object_handle: object_handle,
                            object_name: debug_name.as_bytes().as_ptr() as *const i8,
                            ..Default::default()
                        };
                        unsafe {
                            vk::vk::ExtDebugUtilsExtension::set_debug_utils_object_name_ext(
                                instance,
                                device.handle(),
                                &name_info,
                            );
                        }
                    }
                };

                Image {
                    image: *vk_img,
                    allocation: vma::Allocation::default(),
                    view,
                    format: surface_format.format,
                    aspect: ImageAspectFlags::COLOR,
                    extent: vk::Extent3D {
                        width: extent.width,
                        height: extent.height,
                        depth: 1,
                    },
                }
            })
            .collect(),
    );

    (swapchain, swapchain_images, extent, surface_format.format)
}

/// Gets a suitable swapchain surface format
fn get_swapchain_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    for f in formats {
        if f.format == vk::Format::B8G8R8A8_UNORM
            && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return *f;
        }
    }
    for f in formats {
        if f.format == vk::Format::R8G8B8A8_UNORM
            && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return *f;
        }
    }
    formats[0]
}

/// Gets a suitable swapchain present mode
fn get_swapchain_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    present_modes
        .iter()
        .cloned()
        .find(|m| *m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

/// Gets a suitable swapchain extent
fn get_swapchain_extent(
    size: PhysicalSize<u32>,
    capabilities: vk::SurfaceCapabilitiesKHR,
) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D {
            width: size.width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: size.height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }
}

/// Creates a command pool
unsafe fn create_command_pool(
    instance: &Instance,
    entry: &Entry,
    device: &Device,
    surface: &SurfaceKHR,
    physical_device: &PhysicalDevice,
) -> CommandPool {
    let indices = QueueFamilyIndices::get(instance, entry, surface, physical_device).unwrap();
    let info = vk::CommandPoolCreateInfo {
        flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        queue_family_index: indices.graphics,
        ..Default::default()
    };
    device.create_command_pool(&info, None).unwrap()
}

unsafe fn create_sync_objects(device: &Device) -> (Ring<Semaphore>, Ring<Semaphore>, Ring<Fence>) {
    let semaphore_info = vk::SemaphoreCreateInfo::default();
    let fence_info = vk::FenceCreateInfo {
        flags: vk::FenceCreateFlags::SIGNALED,
        ..Default::default()
    };
    // TODO: we need swapchain_count of image_available_semaphores
    let mut image_available_semaphores = Ring::new(DEFAULT_FRAMES_IN_FLIGHT);
    // TODO: we might need only one???
    let mut render_finished_semaphores = Ring::new(DEFAULT_FRAMES_IN_FLIGHT);
    let mut in_flight_fences = Ring::new(DEFAULT_FRAMES_IN_FLIGHT);
    for i in 0..DEFAULT_FRAMES_IN_FLIGHT {
        image_available_semaphores[i] = device.create_semaphore(&semaphore_info, None).unwrap();
        render_finished_semaphores[i] = device.create_semaphore(&semaphore_info, None).unwrap();
        in_flight_fences[i] = device.create_fence(&fence_info, None).unwrap();
    }

    (
        image_available_semaphores,
        render_finished_semaphores,
        in_flight_fences,
    )
}

#[derive(Clone, Debug)]
struct QueueFamilyIndices {
    graphics: u32,
    present: u32,
}

impl QueueFamilyIndices {
    unsafe fn get(
        instance: &Instance,
        entry: &Entry,
        surface: &SurfaceKHR,
        physical_device: &vk::PhysicalDevice,
    ) -> VkResult<Self> {
        let properties = instance.get_physical_device_queue_family_properties(*physical_device);
        let surface_loader = surface::Instance::new(entry, instance);

        let graphics = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        let mut present = None;
        for (index, _) in properties.iter().enumerate() {
            if surface_loader.get_physical_device_surface_support(
                *physical_device,
                index as u32,
                *surface,
            )? {
                present = Some(index as u32);
                break;
            }
        }

        if let (Some(graphics), Some(present)) = (graphics, present) {
            Ok(Self { graphics, present })
        } else {
            println!("Missing required queue families");
            exit(1);
        }
    }
}

#[derive(Clone, Debug)]
struct SwapchainSupport {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupport {
    unsafe fn get(
        instance: &Instance,
        entry: &Entry,
        physical_device: &vk::PhysicalDevice,
        surface: &SurfaceKHR,
    ) -> VkResult<SwapchainSupport> {
        let surface_loader = surface::Instance::new(entry, instance);

        Ok(SwapchainSupport {
            capabilities: surface_loader
                .get_physical_device_surface_capabilities(*physical_device, *surface)?,
            formats: surface_loader
                .get_physical_device_surface_formats(*physical_device, *surface)?,
            present_modes: surface_loader
                .get_physical_device_surface_present_modes(*physical_device, *surface)?,
        })
    }
}
