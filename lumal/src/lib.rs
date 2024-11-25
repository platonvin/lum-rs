#![allow(clippy::missing_safety_doc)]

// lumal is divided into files (aka modules)
// this in needed for whole thing to compile
pub mod create_buffer_storages;
pub mod create_image_storages;
pub mod descriptors;
pub mod ring; // circular vector
pub mod ops;

use anyhow::{anyhow, Ok, Result};
use cgmath::{vec2, vec4};
use descriptors::DelayedDescriptorSetup;
use ring::Ring;
use std::collections::HashSet;
use std::ffi::CStr;
use std::mem::{size_of, size_of_val};
use std::os::raw::c_void;
use std::ptr::copy_nonoverlapping as memcpy;
use std::{cell::RefCell, process::exit};
use vulkanalia::{bytecode::Bytecode, loader::{LibloadingLoader, LIBRARY}};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::Version;
use vulkanalia_vma::{self as vma};
use winit::{dpi::LogicalSize, event_loop::EventLoop, window::WindowBuilder};
use winit::window::Window;
use Option as optional;
use Vec as vector;

use vk::{KhrSurfaceExtension, KhrSwapchainExtension};

/// The required instance and device layer if validation is enabled.
const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

/// The required device extensions.
const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[
    vk::KHR_SWAPCHAIN_EXTENSION.name,
    vk::EXT_HOST_QUERY_RESET_EXTENSION.name,
];
/// The Vulkan SDK version that started requiring the portability subset extension for macOS.
const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

/// The number of frames that will be processed concurrently.
const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub allocation: vma::Allocation,
    pub mapped: Option<*mut u8>, // If allocation is mapped
}
pub struct Image {
    pub image: vk::Image,
    pub allocation: vma::Allocation,
    pub view: vk::ImageView,              // Main view
    pub mip_views: vector<vk::ImageView>, // Vector for mip views
    pub format: vk::Format,
    pub aspect: vk::ImageAspectFlags,
    pub extent: vk::Extent3D,
    pub mip_levels: u32,
}

// Structure for ImageDeletion
pub struct ImageDeletion {
    pub image: Image,
    pub lifetime: i32,
}

// Structure for BufferDeletion
pub struct BufferDeletion {
    pub buffer: Buffer,
    pub lifetime: i32,
}

// Structure for RasterPipe (Graphics pipeline)
pub struct RasterPipe {
    pub line: vk::Pipeline,
    pub line_layout: vk::PipelineLayout,
    pub sets: vector<vk::DescriptorSet>,
    pub set_layout: vk::DescriptorSetLayout,
    pub render_pass: vk::RenderPass, // We don't need to store it in here but why not
    pub subpass_id: i32,
}

// Structure for ComputePipe (Compute pipeline)
pub struct ComputePipe {
    pub line: vk::Pipeline,
    pub line_layout: vk::PipelineLayout,
    pub sets: vector<vk::DescriptorSet>,
    pub set_layout: vk::DescriptorSetLayout,
}

// Structure for RenderPass
pub struct RenderPass {
    pub clear_colors: vector<vk::ClearValue>,  // Colors to clear
    pub framebuffers: vector<vk::Framebuffer>, // Framebuffers for the pass
    pub extent: vk::Extent2D,                  // Extent of the render pass
    pub render_pass: vk::RenderPass,           // The actual RenderPass object
}


// Structure for Window
pub struct LumalWindow {
    // pub pointer: *mut glfw::ffi::GLFWwindow, // GLFW window pointer
    pub pointer: *mut winit::window::Window,
    pub width: i32,
    pub height: i32,
}

// Structure for QueueFamilyIndices
pub struct LumalQueueFamilyIndices {
    pub graphical_and_compute: optional<u32>,
    pub present: optional<u32>,
}

impl LumalQueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.graphical_and_compute.is_some() && self.present.is_some()
    }
}

// Structure for SwapChainSupportDetails
pub struct SwapChainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: vector<vk::SurfaceFormatKHR>,
    pub present_modes: vector<vk::PresentModeKHR>,
}

impl SwapChainSupportDetails {
    pub fn is_suitable(&self) -> bool {
        !self.formats.is_empty() && !self.present_modes.is_empty()
    }
}

// Structure for Settings
#[derive(Clone)]
pub struct LumalSettings {
    pub timestamp_count: i32,
    pub fif: usize,
    pub vsync: bool,
    pub fullscreen: bool,
    pub debug: bool,
    pub profile: bool,
    pub device_features: vk::PhysicalDeviceFeatures,
    pub device_features11: vk::PhysicalDeviceVulkan11Features,
    pub device_features12: vk::PhysicalDeviceVulkan12Features,
    pub physical_features2: vk::PhysicalDeviceFeatures2,

    pub instance_layers: vector<*const i8>,
    pub instance_extensions: vector<*const i8>,
    pub device_extensions: vector<*const i8>,
}
impl LumalSettings {
    pub fn create_default() -> LumalSettings {
        return LumalSettings {
            timestamp_count: 0,
            fif: MAX_FRAMES_IN_FLIGHT,
            vsync: true,
            fullscreen: false,
            debug: false,
            profile: false,
            device_features: vk::PhysicalDeviceFeatures::default(),
            device_features11: vk::PhysicalDeviceVulkan11Features::default(),
            device_features12: vk::PhysicalDeviceVulkan12Features::default(),
            physical_features2: vk::PhysicalDeviceFeatures2::default(),
            instance_layers: vec![],
            instance_extensions: vec![],
            device_extensions: vec![],
        };
    }
}

#[allow(non_snake_case)]
pub struct DescriptorCounter {
    pub COMBINED_IMAGE_SAMPLER: u32,
    pub INPUT_ATTACHMENT: u32,
    pub SAMPLED_IMAGE: u32,
    pub SAMPLER: u32,
    pub STORAGE_BUFFER: u32,
    pub STORAGE_BUFFER_DYNAMIC: u32,
    pub STORAGE_IMAGE: u32,
    pub STORAGE_TEXEL_BUFFER: u32,
    pub UNIFORM_BUFFER: u32,
    pub UNIFORM_BUFFER_DYNAMIC: u32,
    pub UNIFORM_TEXEL_BUFFER: u32,
}

impl DescriptorCounter {
    pub fn default() -> DescriptorCounter{
        return DescriptorCounter {
            COMBINED_IMAGE_SAMPLER: 0,
            INPUT_ATTACHMENT: 0,
            SAMPLED_IMAGE: 0,
            SAMPLER: 0,
            STORAGE_BUFFER: 0,
            STORAGE_BUFFER_DYNAMIC: 0,
            STORAGE_IMAGE: 0,
            STORAGE_TEXEL_BUFFER: 0,
            UNIFORM_BUFFER: 0,
            UNIFORM_BUFFER_DYNAMIC: 0,
            UNIFORM_TEXEL_BUFFER: 0,
        }
    }
}

// Define the Renderer struct
pub struct LumalRenderer {
    // pub custom_data: Option<T>,
    pub allocator: vma::Allocator,
    pub settings: LumalSettings,
    pub vulkan_data: VulkanData,
    pub event_loop: Option<EventLoop<()>>,
    pub window: Window,
    pub entry: Entry,
    pub instance: Instance,
    pub device: Device,
    pub frame: usize,
    pub resized: bool,
    pub descriptor_counter: DescriptorCounter,
    pub descriptor_sets_count: u32,
    pub delayed_descriptor_setups: vector<DelayedDescriptorSetup>,
}

impl LumalRenderer {
    pub fn create(settings: LumalSettings) -> Result<LumalRenderer> {
        println!("Starting app.");

        let event_loop = EventLoop::new()?;
        let window = WindowBuilder::new()
            .with_title("renderer_vk_rs")
            .with_inner_size(LogicalSize::new(800, 600))
            .build(&event_loop)?;

        let mut vulkan_data = VulkanData::default();

        vulkan_data.validation = settings.debug; 
        if vulkan_data.validation {
            println!("Validation layers requested.");
        }
        unsafe {
            let loader = LibloadingLoader::new(LIBRARY)?;
            let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
            let instance = LumalRenderer::create_instance(&window, &entry, &mut vulkan_data)?;
            vulkan_data.surface = vulkanalia::window::create_surface(&instance, &window, &window)?;
            pick_physical_device(&instance, &mut vulkan_data)?;
            let device = create_logical_device(&entry, &instance, &mut vulkan_data)?;
            
            let allocator_options = vma::AllocatorOptions::new(&instance, &device, vulkan_data.physical_device);
            let allocator = vma::Allocator::new(&allocator_options)?;

            create_swapchain(&window, &instance, &device, &mut vulkan_data)?;
            create_swapchain_image_views(&device, &mut vulkan_data)?;
            // these are handled by downstream user
            // example.create_render_pass(&device, &mut data)?;
            // example.create_pipeline(&device, &mut data)?;
            // create_framebuffers(&device, &mut data)?;
            create_command_pool(&instance, &device, &mut vulkan_data)?;
            create_command_buffers(&device, &mut vulkan_data)?;
            create_sync_objects(&device, &mut vulkan_data)?;

            Ok(LumalRenderer {
                allocator,
                vulkan_data,
                event_loop: Some(event_loop),
                window,
                entry,
                instance,
                device,
                frame: 0,
                resized: false,
                settings,
                descriptor_counter: DescriptorCounter::default(),
                descriptor_sets_count: 0, // cause just init'ed, no descriptor setup deferred yet
                delayed_descriptor_setups: vec![],
            })
        }
    }
    pub fn destroy_image_ring(&self, images: &Ring<Image>){
        for img in images {
            unsafe { self.allocator.destroy_image(img.image, img.allocation); };
        }
    }
    pub fn destroy_buffer_ring(&self, buffers: &Ring<Buffer>){
        for buf in buffers {
            unsafe { self.allocator.destroy_buffer(buf.buffer, buf.allocation); };
        }
    }
    pub unsafe fn create_instance(
        window: &Window,
        entry: &Entry,
        data: &mut VulkanData,
    ) -> Result<Instance> {
        // Application Info
    
        let application_info = vk::ApplicationInfo::builder()
            .application_name(b"renderer_vk\0")
            .application_version(vk::make_version(1, 3, 0))
            .engine_name(b"No Engine\0")
            .engine_version(vk::make_version(1, 3, 0))
            .api_version(vk::make_version(1, 2, 0));
    
        // Layers
    
        let available_layers = entry
            .enumerate_instance_layer_properties()?
            .iter()
            .map(|l| l.layer_name)
            .collect::<HashSet<_>>();
    
        if data.validation && !available_layers.contains(&VALIDATION_LAYER) {
            return Err(anyhow!("Validation layers requested but not supported."));
        }
    
        let layers = if data.validation {
            vec![VALIDATION_LAYER.as_ptr()]
        } else {
            Vec::new()
        };
    
        // Extensions
    
        let mut extensions = vulkanalia::window::get_required_instance_extensions(window)
            .iter()
            .map(|e| e.as_ptr())
            .collect::<vector<_>>();
    
        // Required by Vulkan SDK on macOS since 1.3.216.
        let flags = if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
            println!("Enabling extensions for macOS portability.");
            extensions.push(
                vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION
                    .name
                    .as_ptr(),
            );
            extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::empty()
        };
    
        if data.validation {
            extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
        }
    
        // Create
    
        let mut info = vk::InstanceCreateInfo::builder()
            .application_info(&application_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .flags(flags);
    
        let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .user_callback(Some(debug_callback));
    
        if data.validation {
            info = info.push_next(&mut debug_info);
        }
    
        Ok(entry.create_instance(&info, None)?)
    }
    /// buffers, images, pipelines - everything created manually should be destroyed manually before this funcall
    pub unsafe fn destroy(&self){
        self.destroy_swapchain();
        self.destroy_sync_primitives();
        self.device.destroy_device(None);
        self.instance.destroy_surface_khr(self.vulkan_data.surface, None);
        self.instance.destroy_instance(None);
    }
    
    unsafe fn destroy_swapchain(&self) {
        self.device.free_command_buffers(self.vulkan_data.command_pool, &self.vulkan_data.command_buffers);
        self.vulkan_data.framebuffers.iter().for_each(|f| self.device.destroy_framebuffer(*f, None));
        self.device.destroy_command_pool(self.vulkan_data.command_pool, None);
        self.device.destroy_pipeline(self.vulkan_data.pipeline, None);
        self.device.destroy_pipeline_layout(self.vulkan_data.pipeline_layout, None);
        self.device.destroy_render_pass(self.vulkan_data.render_pass, None);
        self.vulkan_data.swapchain_image_views.iter().for_each(|v| self.device.destroy_image_view(*v, None));
        self.device.destroy_swapchain_khr(self.vulkan_data.swapchain, None);
    }

    unsafe fn destroy_sync_primitives(&self) {
        self.vulkan_data.in_flight_fences.iter().for_each(|f| self.device.destroy_fence(*f, None));
        self.vulkan_data.render_finished_semaphores.iter().for_each(|s| self.device.destroy_semaphore(*s, None));
        self.vulkan_data.image_available_semaphores.iter().for_each(|s| self.device.destroy_semaphore(*s, None));
    }
}


/// The Vulkan handles and associated properties used by an example Vulkan app.
#[derive(Clone, Debug, Default)]
pub struct VulkanData {
    pub validation: bool,
    // Surface
    pub surface: vk::SurfaceKHR,
    // Physical Device / Logical Device
    pub physical_device: vk::PhysicalDevice,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    // Swapchain
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: vector<vk::Image>,
    pub swapchain_image_views: vector<vk::ImageView>,
    // Pipeline
    pub render_pass: vk::RenderPass,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    // Command Pool
    pub command_pool: vk::CommandPool,
    // Framebuffers
    pub framebuffers: vector<vk::Framebuffer>,
    // Command Buffers
    pub command_buffers: vector<vk::CommandBuffer>,
    // Sync Objects
    pub image_available_semaphores: vector<vk::Semaphore>,
    pub render_finished_semaphores: vector<vk::Semaphore>,
    pub in_flight_fences: vector<vk::Fence>,
    pub images_in_flight: vector<vk::Fence>,
    // Descriptor pool
    pub descriptor_pool: vk::DescriptorPool,
}

//================================================
// Instance
//================================================

/// Creates an instance.

/// Logs debug messages.
extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

    if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        println!("({:?}) {}", type_, message);
    } else if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
        println!("({:?}) {}", type_, message);
    }
    // else if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::INFO) {
    //     println!("({:?}) {}", type_, message);
    // } else {
    //     println!("({:?}) {}", type_, message);
    // }

    return vk::FALSE;
}

//================================================
// Physical Device
//================================================

/// An error that indicates a missing requirement for a physical device.
// #[derive(Debug, Error)]
// #[error("{0}")]
pub struct SuitabilityError(pub &'static str);

/// Picks a suitable physical device.
unsafe fn pick_physical_device(instance: &Instance, data: &mut VulkanData) -> Result<()> {
    for physical_device in instance.enumerate_physical_devices()? {
        let properties = instance.get_physical_device_properties(physical_device);

        if let Err(error) = check_physical_device(instance, data, physical_device) {
            println!(
                "Skipping physical device (`{}`): {}",
                properties.device_name, error
            );
        } else {
            println!("Selected physical device (`{}`).", properties.device_name);
            data.physical_device = physical_device;
            return Ok(());
        }
    }

    Err(anyhow!("Failed to find suitable physical device."))
}

/// Checks that a physical device is suitable.
unsafe fn check_physical_device(
    instance: &Instance,
    data: &VulkanData,
    physical_device: vk::PhysicalDevice,
) -> Result<()> {
    QueueFamilyIndices::get(instance, data, physical_device)?;
    check_physical_device_extensions(instance, physical_device)?;

    let support = SwapchainSupport::get(instance, data, physical_device)?;
    if support.formats.is_empty() || support.present_modes.is_empty() {
        // return Err(anyhow!(SuitabilityError("Insufficient swapchain support.")));
        println!("Insufficient swapchain support");
        exit(1);
    }

    Ok(())
}

/// Checks that a physical device supports the required device extensions.
unsafe fn check_physical_device_extensions(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<()> {
    let extensions = instance
        .enumerate_device_extension_properties(physical_device, None)?
        .iter()
        .map(|e| e.extension_name)
        .collect::<HashSet<_>>();
    if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
        Ok(())
    } else {
        // Err(anyhow!(SuitabilityError("Missing required device extensions.")))
        println!("Missing required device extensions");
        exit(1);
    }
}

//================================================
// Logical Device
//================================================

/// Creates a logical device for the picked physical device.
#[allow(unused_variables)]
unsafe fn create_logical_device(
    entry: &Entry,
    instance: &Instance,
    data: &mut VulkanData,
) -> Result<Device> {
    // Queue Create Infos

    let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;

    let mut unique_indices = HashSet::new();
    unique_indices.insert(indices.graphics);
    unique_indices.insert(indices.present);

    let queue_priorities = &[1.0];
    let queue_infos = unique_indices
        .iter()
        .map(|i| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(*i)
                .queue_priorities(queue_priorities)
        })
        .collect::<vector<_>>();

    // Layers

    let layers = if data.validation {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        vec![]
    };

    // Extensions

    let mut extensions = DEVICE_EXTENSIONS
        .iter()
        .map(|n| n.as_ptr())
        .collect::<vector<_>>();

    // Required by Vulkan SDK on macOS since 1.3.216.
    if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
    }

    // Features

    let features = vk::PhysicalDeviceFeatures::builder().sampler_anisotropy(true);

    // Create

    let info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features);

    let device = instance.create_device(data.physical_device, &info, None)?;

    // Queues

    data.graphics_queue = device.get_device_queue(indices.graphics, 0);
    data.present_queue = device.get_device_queue(indices.present, 0);

    Ok(device)
}

//================================================
// Swapchain
//================================================

/// Creates a swapchain and swapchain images.
unsafe fn create_swapchain(
    window: &Window,
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    // Image

    let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;
    let support = SwapchainSupport::get(instance, data, data.physical_device)?;

    let surface_format = get_swapchain_surface_format(&support.formats);
    let present_mode = get_swapchain_present_mode(&support.present_modes);
    let extent = get_swapchain_extent(window, support.capabilities);

    data.swapchain_format = surface_format.format;
    data.swapchain_extent = extent;

    // A max image count of 0 indicates that the surface has no upper limit on number of images.
    let max_image_count = if support.capabilities.max_image_count != 0 {
        support.capabilities.max_image_count
    } else {
        u32::MAX
    };

    let image_count = (support.capabilities.min_image_count + 1).min(max_image_count);

    let mut queue_family_indices = vec![];
    let image_sharing_mode = if indices.graphics != indices.present {
        queue_family_indices.push(indices.graphics);
        queue_family_indices.push(indices.present);
        vk::SharingMode::CONCURRENT
    } else {
        vk::SharingMode::EXCLUSIVE
    };

    // Create

    let info = vk::SwapchainCreateInfoKHR::builder()
        .surface(data.surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(image_sharing_mode)
        .queue_family_indices(&queue_family_indices)
        .pre_transform(support.capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true);

    data.swapchain = device.create_swapchain_khr(&info, None)?;

    // Images

    data.swapchain_images = device.get_swapchain_images_khr(data.swapchain)?;

    Ok(())
}

/// Gets a suitable swapchain surface format.
fn get_swapchain_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    formats
        .iter()
        .cloned()
        .find(|f| {
            f.format == vk::Format::R8G8B8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or_else(|| formats[0])
}

/// Gets a suitable swapchain present mode.
fn get_swapchain_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    present_modes
        .iter()
        .cloned()
        .find(|m| *m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

/// Gets a suitable swapchain extent.
#[rustfmt::skip]
fn get_swapchain_extent(window: &Window, capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D::builder()
            .width(window.inner_size().width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ))
            .height(window.inner_size().height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ))
            .build()
    }
}

/// Creates image views for the swapchain images.
unsafe fn create_swapchain_image_views(device: &Device, data: &mut VulkanData) -> Result<()> {
    data.swapchain_image_views = data
        .swapchain_images
        .iter()
        .map(|i| {
            let components = vk::ComponentMapping::builder()
                .r(vk::ComponentSwizzle::IDENTITY)
                .g(vk::ComponentSwizzle::IDENTITY)
                .b(vk::ComponentSwizzle::IDENTITY)
                .a(vk::ComponentSwizzle::IDENTITY);

            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let info = vk::ImageViewCreateInfo::builder()
                .image(*i)
                .view_type(vk::ImageViewType::_2D)
                .format(data.swapchain_format)
                .components(components)
                .subresource_range(subresource_range);

            device.create_image_view(&info, None)
        })
        .collect::<Result<_, _>>()?;
    Ok(())
}

//================================================
// Command Pool
//================================================

/// Creates a command pool.
unsafe fn create_command_pool(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;

    let info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(indices.graphics);

    data.command_pool = device.create_command_pool(&info, None)?;

    Ok(())
}

//================================================
// Command Buffers
//================================================

/// Creates the primary command buffers for recording frame commands.
unsafe fn create_command_buffers(device: &Device, data: &mut VulkanData) -> Result<()> {
    let info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(data.command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(data.swapchain_images.len() as u32);

    data.command_buffers = device.allocate_command_buffers(&info)?;

    Ok(())
}

//================================================
// Framebuffers
//================================================

/// Creates framebuffers for the swapchain image views.
// unsafe fn create_framebuffers(device: &Device, data: &mut VulkanData) -> Result<()> {
//     data.framebuffers = data
//         .swapchain_image_views
//         .iter()
//         .map(|iv| {
//             let attachments = &[*iv];
//             let info = vk::FramebufferCreateInfo::builder()
//                 .render_pass(data.render_pass)
//                 .attachments(attachments)
//                 .width(data.swapchain_extent.width)
//                 .height(data.swapchain_extent.height)
//                 .layers(1);
//             device.create_framebuffer(&info, None)
//         })
//         .collect::<Result<_, _>>()?;
//     Ok(())
// }

//================================================
// Sync Objects
//================================================

/// Creates synchronization objects to manage command buffer reuse and rendering.
unsafe fn create_sync_objects(device: &Device, data: &mut VulkanData) -> Result<()> {
    let semaphore_info = vk::SemaphoreCreateInfo::builder();
    let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        data.image_available_semaphores
            .push(device.create_semaphore(&semaphore_info, None)?);
        data.render_finished_semaphores
            .push(device.create_semaphore(&semaphore_info, None)?);
        data.in_flight_fences
            .push(device.create_fence(&fence_info, None)?);
    }

    data.images_in_flight = data
        .swapchain_images
        .iter()
        .map(|_| vk::Fence::null())
        .collect();

    Ok(())
}

//================================================
// Structs
//================================================

/// The indices of the required queue families for a physical device.
#[derive(Copy, Clone, Debug)]
struct QueueFamilyIndices {
    graphics: u32,
    present: u32,
}

impl QueueFamilyIndices {
    /// Gets the indices of the required queue families for a physical device.
    unsafe fn get(
        instance: &Instance,
        data: &VulkanData,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self> {
        let properties = instance.get_physical_device_queue_family_properties(physical_device);

        let graphics = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        let mut present = None;
        for (index, _) in properties.iter().enumerate() {
            if instance.get_physical_device_surface_support_khr(
                physical_device,
                index as u32,
                data.surface,
            )? {
                present = Some(index as u32);
                break;
            }
        }

        if let (Some(graphics), Some(present)) = (graphics, present) {
            Ok(Self { graphics, present })
        } else {
            // Err(anyhow!(SuitabilityError("Missing required queue families.")))
            println!("Missing required queue families");
            exit(1);
        }
    }
}

/// The swapchain support for a physical device.
#[derive(Clone, Debug)]
struct SwapchainSupport {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: vector<vk::SurfaceFormatKHR>,
    present_modes: vector<vk::PresentModeKHR>,
}

impl SwapchainSupport {
    /// Gets the swapchain support for a physical device.
    unsafe fn get(
        instance: &Instance,
        data: &VulkanData,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self> {
        Ok(Self {
            capabilities: instance
                .get_physical_device_surface_capabilities_khr(physical_device, data.surface)?,
            formats: instance
                .get_physical_device_surface_formats_khr(physical_device, data.surface)?,
            present_modes: instance
                .get_physical_device_surface_present_modes_khr(physical_device, data.surface)?,
        })
    }
}

//================================================
// Shared (buffers)
//================================================

/// Creates a device buffer.
pub unsafe fn create_buffer(
    instance: &Instance,
    device: &Device,
    data: &VulkanData,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    // Buffer

    let buffer_info = vk::BufferCreateInfo::builder()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = device.create_buffer(&buffer_info, None)?;

    // Memory

    let requirements = device.get_buffer_memory_requirements(buffer);
    let memory_type_index = get_memory_type_index(instance, data, properties, requirements)?;

    let memory_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type_index);

    let memory = device.allocate_memory(&memory_info, None)?;

    Ok((buffer, memory))
}

/// Fills a device buffer with data.
#[rustfmt::skip]
pub unsafe fn fill_buffer(device: &Device, buffer: vk::Buffer, memory: vk::DeviceMemory, data: &[impl Copy]) -> Result<()> {
    device.bind_buffer_memory(buffer, memory, 0)?;

    let dst = device.map_memory(memory, 0, size_of_val(data) as u64, vk::MemoryMapFlags::empty())?;
    memcpy(data.as_ptr(), dst.cast(), data.len());
    device.unmap_memory(memory);

    Ok(())
}

//================================================
// Shared (shaders)
//================================================

/// Creates a shader module from a compiled shader.
pub unsafe fn create_shader_module(device: &Device, bytecode: &[u8]) -> Result<vk::ShaderModule> {
    let bytecode = Bytecode::new(bytecode).unwrap();

    let info = vk::ShaderModuleCreateInfo::builder()
        .code_size(bytecode.code_size())
        .code(bytecode.code());

    Ok(device.create_shader_module(&info, None)?)
}

//================================================
// Shared (other)
//================================================

/// Gets a suitable memory type index for a device buffer.
pub unsafe fn get_memory_type_index(
    instance: &Instance,
    data: &VulkanData,
    properties: vk::MemoryPropertyFlags,
    requirements: vk::MemoryRequirements,
) -> Result<u32> {
    let memory = instance.get_physical_device_memory_properties(data.physical_device);
    (0..memory.memory_type_count)
        .find(|i| {
            let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
            let memory_type = memory.memory_types[*i as usize];
            suitable && memory_type.property_flags.contains(properties)
        })
        .ok_or_else(|| anyhow!("Failed to find suitable memory type."))
}

//================================================
// Vertex
//================================================

pub type Vec2 = cgmath::Vector2<f32>;
pub type Vec4 = cgmath::Vector4<f32>;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub pos: Vec2,
    pub color: Vec4,
}

impl Vertex {
    /// Gets the binding description for a vertex of this type.
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    /// Gets the attribute descriptions for a vertex of this type.
    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0)
            .build();
        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(size_of::<Vec2>() as u32)
            .build();
        [pos, color]
    }
}

/// The triangle vertices.
#[rustfmt::skip]
pub static VERTICES: [Vertex; 3] = [
    Vertex { pos: vec2(0.0, -0.5), color: vec4(1.0, 0.0, 0.0, 1.0) },
    Vertex { pos: vec2(0.5, 0.5), color: vec4(0.0, 1.0, 0.0, 1.0) },
    Vertex { pos: vec2(-0.5, 0.5), color: vec4(0.0, 0.0, 1.0, 0.0) },
];
