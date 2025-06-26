//! Module for managing Descriptors - sets, layouts, pools

use crate::Renderer;
use crate::{Buffer, Image, LumalSettings, RasterPipe, DEFAULT_FRAMES_IN_FLIGHT};
use ash::vk::DescriptorType;
use ash::{vk, Device};
use containers::Ring;
use std::ops::Index;

#[derive(PartialEq, Eq, Clone)]
/// Describes how new pixel color is combined with an existing (old) pixel color during rendering.
///
/// Each variant configures the blending operations and factors for color and alpha channels.
pub enum BlendAttachment {
    /// No blending is performed.
    /// The new pixel color entirely replaces the old pixel color.
    ///
    /// $FinalColor = NewColor$
    /// $FinalAlpha = NewAlpha$
    NoBlend,
    /// Common blending mode that mixes old and new colors
    /// based on the new alpha value. Typically used for transparency.
    ///
    /// $FinalColor = (NewColor \times NewAlpha) + (OldColor \times (1 - NewAlpha))$
    /// $FinalAlpha = NewAlpha + OldAlpha$
    BlendMix,
    /// Performs a subtraction blending operation.
    ///
    /// $FinalColor = NewColor - OldColor$
    /// $FinalAlpha = NewAlpha + OldAlpha$
    BlendSub,
    /// Replaces the old color with the new color if the new color
    /// component value is greater than the old. Basically takes the max() of each channel.
    ///
    /// $FinalColor = max(NewColor, OldColor)$
    /// $FinalAlpha = NewAlpha + OldAlpha$
    BlendReplaceIfGreater, // Basically max
    /// Replaces the old color with the new color if the new color
    /// component value is les than the old. Basically takes the min() of each channel.
    ///
    /// $FinalColor = min(NewColor, OldColor)$
    /// $FinalAlpha = NewAlpha + OldAlpha$
    BlendReplaceIfLess,
}

#[allow(non_camel_case_types)]
#[derive(PartialEq, Clone, Copy)]
/// Describes how is depth testing performed during rendering
///
/// Each variant configures the depth read & write operations;
pub enum DepthTesting {
    /// Do not perform depth testing (new values always written)
    None,
    /// Perform depth testing, but do not write depth value (when depth test passed)
    Read,
    /// Do nit perform depth testing, but write new depth values
    Write,
    /// "Normal" mode - perform depth testing (read), and write new depth values when test succeeded
    ReadWrite,
}

/// Describes what happens with Renderpass Attachaments when starting / ending the Renderpass
///
/// Each variant configures StoreOp / LoadOp
pub enum LoadStoreOp {
    /// Does not specify anything, allowing driver to pick fastest option
    DontCare,
    /// Clear attachment to the specified ClearColor when starting Renderpass
    Clear,
    /// Store the attachment when ending Renderpass
    Store,
    /// Store the attachment when starting Renderpass
    Load,
}
impl LoadStoreOp {
    pub(crate) fn to_vk_load(&self) -> vk::AttachmentLoadOp {
        match self {
            LoadStoreOp::DontCare => vk::AttachmentLoadOp::DONT_CARE,
            LoadStoreOp::Clear => vk::AttachmentLoadOp::CLEAR,
            LoadStoreOp::Load => vk::AttachmentLoadOp::LOAD,
            LoadStoreOp::Store => panic!(),
        }
    }
    pub(crate) fn to_vk_store(&self) -> vk::AttachmentStoreOp {
        match self {
            LoadStoreOp::DontCare => vk::AttachmentStoreOp::DONT_CARE,
            LoadStoreOp::Store => vk::AttachmentStoreOp::STORE,
            LoadStoreOp::Clear => panic!(),
            LoadStoreOp::Load => panic!(),
        }
    }
}

impl PartialEq for LoadStoreOp {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (LoadStoreOp::DontCare, LoadStoreOp::DontCare)
                | (LoadStoreOp::Clear, LoadStoreOp::Clear)
                | (LoadStoreOp::Load, LoadStoreOp::Load)
                | (LoadStoreOp::Store, LoadStoreOp::Store)
        )
    }
}

/// Hold either Ring of resources, or a single resource.
pub enum MaybeRing<'a, T> {
    /// Ring (multiple) of resources.
    /// This is used for CPU-GPU resources and temporal computations.
    Ring(&'a Ring<T>),
    /// Single resource
    /// This is used for purely GPU resources.
    Single(&'a T),
}
impl<'a, T> MaybeRing<'a, T> {
    /// Returns either first resource in a Ring, or element if it is Single element
    pub fn get_first(&self) -> &T {
        match self {
            MaybeRing::Ring(ring) => &ring[0],
            MaybeRing::Single(elem) => elem,
        }
    }

    /// Returns number of elements hold (len for Ring, 1 for Single)
    pub(crate) fn len(&self) -> usize {
        match self {
            MaybeRing::Ring(ring) => ring.len(),
            MaybeRing::Single(_) => 1,
        }
    }
}
impl<'a, T> Index<usize> for MaybeRing<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            MaybeRing::Ring(ring) => &ring[index],
            MaybeRing::Single(elem) => elem,
        }
    }
}

/// Description of a single attachment for the renderpass. Close to Vulkan
pub struct AttachmentDescription<'a> {
    /// (maybe ring of) image(s), which are resource of this attachment
    pub images: MaybeRing<'a, Image>,
    /// executed operation in the beginning of the renderpass for Color aspects
    pub load: LoadStoreOp,
    /// executed operation in the end of the renderpass for Color aspects
    pub store: LoadStoreOp,
    /// executed operation in the beginning of the renderpass for Stencil aspects
    pub sload: LoadStoreOp,
    /// executed operation in the end of the renderpass for Stencil aspects
    pub sstore: LoadStoreOp,
    /// Value used for clearing when LoadOp is Clear
    pub clear: vk::ClearValue,
    /// Wanted layout of Image after Renderpass ends
    pub final_layout: vk::ImageLayout,
}

/// Description of a subpass for the renderpass,
pub struct SubpassDescription<'lt> {
    /// all Pipes, that might be used with this subpass
    pub pipes: &'lt mut [&'lt mut RasterPipe],
    /// All input attachments that pipes of this subpass will have
    /// Input attachments are 1:1 read-only images
    pub a_input: &'lt [MaybeRing<'lt, Image>],
    /// All color attachments that pipes of this subpass will have
    /// Color attachments are 1:1 write-only, and you must write to them
    /// this is how you draw pixels on screen
    pub a_color: &'lt [MaybeRing<'lt, Image>],
    /// All depth/stencil attachments that pipes of this subpass will have
    /// Depth attachments are 1:1 read-write images for depth/stencil tests
    /// this is how you do depth testing
    pub a_depth: Option<MaybeRing<'lt, Image>>, // Depth image for the subpass
}

/// Intermediate struct which we accumulate corresponding subpass attachment data into (for later referencing it for Vulkan calls)
#[derive(Clone, Default, Debug)]
pub struct SubpassAttachmentRefs {
    /// All the Input attachments, used by corresponding subpass
    pub a_input: Vec<vk::AttachmentReference>,
    /// All the Color attachments, used by corresponding subpass
    pub a_color: Vec<vk::AttachmentReference>,
    // using Option is unconvenient because we need to point'er it afterwards. But still
    /// All the Depth attachments, used by corresponding subpass
    pub a_depth: Option<vk::AttachmentReference>,
}

/// Description of a Vertex Attributes for given Pipe
/// Note: location is defined by position of this AttrFormOffs in array of AttrFormOffs for Pipe creation
#[derive(Clone, Debug)]
pub struct AttrFormOffs {
    /// Size and type of the vertex attribute data (they are hardware thing, so somewhat limited)
    pub format: vk::Format,
    /// Binding number which this attribute takes its data from (you have to bind vertex buffers accordingly!)
    pub binding: u32,
    /// Byte offset of this attribute relative to the start of an element in the vertex input binding
    pub offset: usize,
}

/// Abstraction which simplifies managing CPU-GPU resources
/// When the resource is operated by GPU only, we can have single copy of it, and always use this copy
/// When CPU writes to some resource, we need to (at least) double-buffer it,
/// because otherwise we might end up with situation, where CPU is writing data for current frame which GPU is reading for previous frame
#[derive(Debug)]
pub enum RelativeResource<'a, T> {
    /// Resource is Ring of things and we bind current() (most resource)
    Current(&'a Ring<T>),
    /// Resource is Ring of things and we bind previous()
    /// (e.g. reading old for reading and current for writing in case of radiance cache)
    Previous(&'a Ring<T>),
    /// Resource is a single thing (and is the same for all binds)
    Single(&'a T),
}

impl<'a, T> RelativeResource<'a, T> {
    fn get_matching_resource(&'a self, current_frame_i: usize, previous_frame_i: usize) -> &'a T {
        match self {
            RelativeResource::Current(ring) => &ring[current_frame_i],
            RelativeResource::Previous(ring) => &ring[previous_frame_i],
            RelativeResource::Single(resource) => resource,
        }
    }
}

/// Typed subset of bundles of Vulkan types and my objects
/// this moves work from runtime to compile time by enforcing relative presentance of descriptor information with type system
#[derive(Debug)]
pub enum DescriptorResource<'a> {
    /// Corresponds to VK_DESCRIPTOR_TYPE_STORAGE_IMAGE
    StorageImage(RelativeResource<'a, Image>, vk::ImageLayout),
    /// Corresponds to VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER
    SampledImage(RelativeResource<'a, Image>, vk::ImageLayout, vk::Sampler),
    /// Corresponds to VK_DESCRIPTOR_TYPE_INPUT_ATTACHMENT
    InputAttachment(RelativeResource<'a, Image>, vk::ImageLayout),
    /// Corresponds to VK_DESCRIPTOR_TYPE_UNIFORM_BUFFER
    UniformBuffer(RelativeResource<'a, Buffer>),
    /// Corresponds to VK_DESCRIPTOR_TYPE_STORAGE_BUFFER
    StorageBuffer(RelativeResource<'a, Buffer>),
}

/// DescriptorResource + which stages see this resource
#[derive(Debug)]
pub struct DescriptorInfo<'a> {
    /// The resource itself
    pub resources: DescriptorResource<'a>,
    /// Which shader stages see this resource
    pub specified_stages: vk::ShaderStageFlags,
}

impl<'a> DescriptorInfo<'a> {
    fn get_type(&self) -> DescriptorType {
        match self.resources {
            DescriptorResource::StorageImage(_, _) => DescriptorType::STORAGE_IMAGE,
            DescriptorResource::SampledImage(_, _, _) => DescriptorType::COMBINED_IMAGE_SAMPLER,
            DescriptorResource::InputAttachment(_, _) => DescriptorType::INPUT_ATTACHMENT,
            DescriptorResource::UniformBuffer(_) => DescriptorType::UNIFORM_BUFFER,
            DescriptorResource::StorageBuffer(_) => DescriptorType::STORAGE_BUFFER,
        }
    }
}

/// Subset of description info used for creating dset layouts and allocating pool
pub struct ShortDescriptorInfo {
    /// Vulkan Descriptor Type
    pub descriptor_type: vk::DescriptorType,
    /// Shader stages, which this descriptor is visible to
    pub stages: vk::ShaderStageFlags,
}

impl Renderer {
    /// immediately creates vulkan descriptor set layout
    pub fn create_descriptor_set_layout(
        &mut self,
        descriptor_infos: &[ShortDescriptorInfo],
        layout: &mut vk::DescriptorSetLayout,
        flags: vk::DescriptorSetLayoutCreateFlags,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) {
        let bindings: Vec<vk::DescriptorSetLayoutBinding> = descriptor_infos
            .iter()
            .enumerate()
            .map(|(i, info)| {
                // add 1 to corresponding counter
                self.descriptor_counter.increment_counter(info.descriptor_type);

                vk::DescriptorSetLayoutBinding {
                    binding: i as u32,
                    descriptor_type: info.descriptor_type,
                    descriptor_count: 1,
                    stage_flags: info.stages,
                    ..Default::default()
                }
            })
            .collect();

        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            flags,
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            ..Default::default()
        };

        // actually create layout and write it to ref
        *layout = unsafe {
            self.device
                .create_descriptor_set_layout(&layout_info, None)
                .expect("Failed to create descriptor set layout")
        };

        #[cfg(feature = "debug_validation_names")]
        crate::set_debug_names!(self, debug_name, (layout, " Layout"));
    }

    /// Creates Vulkan descriptor pool with exact sizes, specified in self.descriptor_counter
    pub fn create_descriptor_pool(&self) -> vk::DescriptorPool {
        let pool_sizes = self.descriptor_counter.get_pool_sizes();

        let pool_info = vk::DescriptorPoolCreateInfo {
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: self.descriptor_sets_count * self.settings.fif as u32,
            flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
            ..Default::default()
        };

        unsafe { self.device.create_descriptor_pool(&pool_info, None).unwrap() }
    }

    /// Tells Lumal that such descriptor will be setup.
    /// Used to count needed resources to then allocate them (to avoid runtime re-allocations).
    pub fn anounce_descriptor_setup(
        &mut self,
        dset_layout: &mut vk::DescriptorSetLayout,
        _descriptor_sets: &mut Ring<vk::DescriptorSet>, // Ring to setup into (some setup happens immediately on anounce)
        descriptions: &[DescriptorInfo],
        default_stages: vk::ShaderStageFlags,
        create_flags: vk::DescriptorSetLayoutCreateFlags,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) {
        if *dset_layout == vk::DescriptorSetLayout::null() {
            let descriptor_infos: Vec<ShortDescriptorInfo> = descriptions
                .iter()
                .map(|desc| ShortDescriptorInfo {
                    descriptor_type: desc.get_type(),
                    // default to generic stages if not specified
                    stages: if desc.specified_stages.is_empty() {
                        default_stages
                    } else {
                        desc.specified_stages
                    },
                })
                .collect();

            // actually create layout and write it to ptr
            self.create_descriptor_set_layout(
                &descriptor_infos,
                dset_layout,
                create_flags,
                #[cfg(feature = "debug_validation_names")]
                debug_name,
            );
        }

        self.descriptor_sets_count += self.settings.fif as u32; // cuase dset per fif
    }

    // anounce is just a request, this is an actual logic
    pub(crate) unsafe fn actually_setup_descriptor_impl(
        descriptor_pool: &vk::DescriptorPool,
        settings: &LumalSettings,
        device: &Device,
        dset_layout: &vk::DescriptorSetLayout,
        descriptor_sets: &mut Ring<vk::DescriptorSet>,
        descriptions: &[DescriptorInfo],
        _stages: vk::ShaderStageFlags,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) {
        *descriptor_sets = Ring::new(DEFAULT_FRAMES_IN_FLIGHT);
        let dset_layouts = [*dset_layout; DEFAULT_FRAMES_IN_FLIGHT];
        for frame_i in 0..DEFAULT_FRAMES_IN_FLIGHT {
            descriptor_sets[frame_i] = device
                .allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                    descriptor_pool: *descriptor_pool,
                    descriptor_set_count: DEFAULT_FRAMES_IN_FLIGHT as u32,
                    p_set_layouts: dset_layouts.as_ptr(),
                    ..Default::default()
                })
                .unwrap()[0];
        }
        assert!(descriptor_sets.len() == DEFAULT_FRAMES_IN_FLIGHT);

        // why FIF descriptors?
        // tats because some resources are FIF count in Ring
        // well, some are not, and there are pipelines that only need single reource to be bound
        // we might only use single descriptor for them, but its not done right now for simplicity
        for frame_i in 0..descriptor_sets.len() {
            let previous_frame_i = if frame_i == 0 {
                settings.fif - 1
            } else {
                frame_i - 1
            };

            // we have to keep theese around untill end of the scope because Vulkan wants descriptions to be pointers
            // and thus we need some sort of temporary memory
            // We could wrap them in Options, but there is no reason for it. Essentially its like very fast/unsafe slot allocator
            let mut image_infos = vec![vk::DescriptorImageInfo::default(); descriptions.len()];
            let mut buffer_infos = vec![vk::DescriptorBufferInfo::default(); descriptions.len()];

            let writes: Vec<_> = descriptions
                .iter()
                .enumerate()
                .map(|(i, desc)| {
                    let mut write = vk::WriteDescriptorSet {
                        s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                        dst_set: descriptor_sets[frame_i],
                        dst_binding: i as u32,
                        dst_array_element: 0,
                        descriptor_count: 1,
                        descriptor_type: desc.get_type(),
                        ..Default::default()
                    };

                    // now we need to extract resource from a description and find corresponding element in Ring (or just use it if its Single (nor Ring))

                    match &desc.resources {
                        // if descriptor is some type of image, we fill corresponding image slot in image_infos and point to it
                        DescriptorResource::StorageImage(images, image_layout) => {
                            let image = images.get_matching_resource(frame_i, previous_frame_i);
                            // we do not explicitly allocate slot in infos, but all [i] are unique so its fine (and fast)
                            image_infos[i] = vk::DescriptorImageInfo {
                                image_view: image.view,
                                image_layout: *image_layout,
                                sampler: vk::Sampler::null(), // cause storage image
                            };
                            write.p_image_info = &image_infos[i];
                        }
                        DescriptorResource::SampledImage(images, image_layout, sampler) => {
                            let image = images.get_matching_resource(frame_i, previous_frame_i);
                            image_infos[i] = vk::DescriptorImageInfo {
                                image_view: image.view,
                                image_layout: *image_layout,
                                sampler: *sampler,
                            };
                            write.p_image_info = &image_infos[i];
                        }
                        DescriptorResource::InputAttachment(images, image_layout) => {
                            let image = images.get_matching_resource(frame_i, previous_frame_i);
                            image_infos[i] = vk::DescriptorImageInfo {
                                image_view: image.view,
                                image_layout: *image_layout,
                                sampler: vk::Sampler::null(), // imput attachments are not sampled
                            };
                            write.p_image_info = &image_infos[i];
                        }
                        // if descriptor is some type of buffer, we fill corresponding buffer slot in buffer_infos and point to it
                        DescriptorResource::UniformBuffer(buffers) => {
                            let buffer = buffers.get_matching_resource(frame_i, previous_frame_i);
                            buffer_infos[i] = vk::DescriptorBufferInfo {
                                buffer: buffer.buffer,
                                offset: 0,
                                range: vk::WHOLE_SIZE, // we bind entire buffer in most cases for simplicity
                            };
                            write.p_buffer_info = &buffer_infos[i];
                        }
                        DescriptorResource::StorageBuffer(buffers) => {
                            let buffer = buffers.get_matching_resource(frame_i, previous_frame_i);
                            buffer_infos[i] = vk::DescriptorBufferInfo {
                                buffer: buffer.buffer,
                                offset: 0,
                                range: vk::WHOLE_SIZE, // we bind entire buffer in most cases for simplicity
                            };
                            write.p_buffer_info = &buffer_infos[i];
                        }
                    }

                    write
                })
                .collect();

            device.update_descriptor_sets(&writes, &[]);
        }
    }

    /// Allocates enough space previously counted (anounced) resources.
    /// (actually) Creates Vulkan descriptor pool.
    pub fn flush_descriptor_setup(&mut self) {
        if self.descriptor_pool == vk::DescriptorPool::null() {
            self.descriptor_pool = self.create_descriptor_pool();
        }
    }

    /// Allocated descriptor itself from a pool
    /// This must only be called after flush() happened and pool allocated
    pub fn acutally_setup_descriptor(
        &mut self,
        dset_layout: &mut vk::DescriptorSetLayout,
        descriptor_sets: &mut Ring<vk::DescriptorSet>, // Ring to setup into
        descriptions: &[DescriptorInfo],
        default_stages: vk::ShaderStageFlags,
        _create_flags: vk::DescriptorSetLayoutCreateFlags,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) {
        // actually setup descriptor
        unsafe {
            Self::actually_setup_descriptor_impl(
                &self.descriptor_pool,
                &self.settings,
                &self.device,
                dset_layout,
                descriptor_sets,
                descriptions,
                default_stages,
                #[cfg(feature = "debug_validation_names")]
                debug_name,
            );
        }
    }
}
