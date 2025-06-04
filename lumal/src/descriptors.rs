use ash::vk::DescriptorType;
use ash::{vk, Device};

use crate::{
    ring::Ring, set_debug_names, Buffer, DescriptorCounter, Image, LumalSettings, RasterPipe,
    DEFAULT_FRAMES_IN_FLIGHT,
};
use crate::{set_debug_name, Renderer};
use std::ops::Index;
use std::{any::TypeId, cell::UnsafeCell};
use std::{option, ptr::null};

#[derive(PartialEq, Eq, Clone)]
pub enum BlendAttachment {
    NoBlend,
    BlendMix,
    BlendSub,
    BlendReplaceIfGreater, // Basically max
    BlendReplaceIfLess,    // Basically min
}

#[allow(non_camel_case_types)]
#[derive(PartialEq, Clone, Copy)]
pub enum DepthTesting {
    DT_None,
    DT_Read,
    DT_Write,
    DT_ReadWrite,
}

pub enum Discard {
    NoDiscard,
    DoDiscard,
}

pub enum LoadStoreOp {
    DontCare,
    Clear,
    Store,
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

pub enum MaybeRing<'a, T> {
    Ring(&'a Ring<T>),
    Single(&'a T),
}
impl<'a, T> MaybeRing<'a, T> {
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

pub struct AttachmentDescription<'a> {
    pub images: MaybeRing<'a, Image>,
    pub load: LoadStoreOp,
    pub store: LoadStoreOp,
    pub sload: LoadStoreOp,
    pub sstore: LoadStoreOp,
    pub clear: vk::ClearValue,
    pub final_layout: vk::ImageLayout, // Default value is GENERAL
}

// everything is a a pointer to be able to compare them later
pub struct SubpassDescription<'lt> {
    pub pipes: &'lt mut [&'lt mut RasterPipe],
    pub a_input: &'lt [MaybeRing<'lt, Image>], // Input images for the subpass
    pub a_color: &'lt [MaybeRing<'lt, Image>], // Color images for the subpass
    pub a_depth: Option<MaybeRing<'lt, Image>>, // Depth image for the subpass
}

#[derive(Clone, Default, Debug)]
pub struct SubpassAttachmentRefs {
    pub a_input: Vec<vk::AttachmentReference>,
    pub a_color: Vec<vk::AttachmentReference>,
    // using Option is unconvenient because we need to point'er it afterwards. But still
    pub a_depth: Option<vk::AttachmentReference>,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum RelativeDescriptorPos {
    #[default]
    NotPresented, // What?
    Previous, // Relative Descriptor position previous - for accumulators
    Current,  // Relative Descriptor position matching - common CPU-paired
    First,    // Relative Descriptor position first - for GPU-only
}

#[derive(Clone, Debug)]
pub struct ShaderStage<'a> {
    pub stage: vk::ShaderStageFlags,
    pub spirv_code: &'a [u8],
}

#[derive(Clone, Debug)]
pub struct AttrFormOffs {
    pub format: vk::Format,
    pub binding: u32,
    pub offset: usize,
}

#[derive(Debug, Default)]
pub enum RelativeResource<'a, T> {
    #[default]
    None,
    /// Resource is Ring of things and we bind current() (most resource)
    Current(&'a Ring<T>),
    /// Resource is Ring of things and we bind previous()
    /// (e.g. reading old for reading and current for writing in case of radiance cache)
    Previous(&'a Ring<T>),
    // Resource is a single thing (and is the same for all binds)
    Single(&'a T),
}

impl<'a, T> RelativeResource<'a, T> {
    fn get_matching_resource(&'a self, current_frame_i: usize, previous_frame_i: usize) -> &'a T {
        match self {
            RelativeResource::None => panic!(),
            RelativeResource::Current(ring) => &ring[current_frame_i],
            RelativeResource::Previous(ring) => &ring[previous_frame_i],
            RelativeResource::Single(resource) => resource,
        }
    }
}

/// Typed subset of bundles of Vulkan types and my objects
/// this moves work from runtime to compile time by enforcing relative presentance of descriptor information with type system
#[derive(Debug, Default)]
pub enum DescriptorResource<'a> {
    #[default]
    None,
    StorageImage(RelativeResource<'a, Image>, vk::ImageLayout),
    SampledImage(RelativeResource<'a, Image>, vk::ImageLayout, vk::Sampler),
    InputAttachment(RelativeResource<'a, Image>, vk::ImageLayout),
    UniformBuffer(RelativeResource<'a, Buffer>),
    StorageBuffer(RelativeResource<'a, Buffer>),
}

#[derive(Debug, Default)]
pub struct DescriptorInfo<'a> {
    pub resources: DescriptorResource<'a>,
    pub specified_stages: vk::ShaderStageFlags,
}

impl<'a> DescriptorInfo<'a> {
    pub fn get_type(&self) -> DescriptorType {
        match self.resources {
            DescriptorResource::StorageImage(_, _) => DescriptorType::STORAGE_IMAGE,
            DescriptorResource::SampledImage(_, _, _) => DescriptorType::COMBINED_IMAGE_SAMPLER,
            DescriptorResource::InputAttachment(_, _) => DescriptorType::INPUT_ATTACHMENT,
            DescriptorResource::UniformBuffer(_) => DescriptorType::UNIFORM_BUFFER,
            DescriptorResource::StorageBuffer(_) => DescriptorType::STORAGE_BUFFER,
            DescriptorResource::None => todo!(),
        }
    }
}

pub struct ShortDescriptorInfo {
    pub descriptor_type: vk::DescriptorType,
    pub stages: vk::ShaderStageFlags,
}

impl Renderer {
    /// immediately creates vulkan descriptor set layout
    #[cold]
    #[optimize(size)]
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
                macro_rules! make_descriptor_type {
                    ($name:ident) => {
                        self.descriptor_counter.$name += 1
                    };
                }
                match info.descriptor_type {
                    vk::DescriptorType::SAMPLER => make_descriptor_type!(SAMPLER),
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER => {
                        make_descriptor_type!(COMBINED_IMAGE_SAMPLER)
                    }
                    vk::DescriptorType::SAMPLED_IMAGE => make_descriptor_type!(SAMPLED_IMAGE),
                    vk::DescriptorType::STORAGE_IMAGE => make_descriptor_type!(STORAGE_IMAGE),
                    vk::DescriptorType::UNIFORM_TEXEL_BUFFER => {
                        make_descriptor_type!(UNIFORM_TEXEL_BUFFER)
                    }
                    vk::DescriptorType::STORAGE_TEXEL_BUFFER => {
                        make_descriptor_type!(STORAGE_TEXEL_BUFFER)
                    }
                    vk::DescriptorType::UNIFORM_BUFFER => make_descriptor_type!(UNIFORM_BUFFER),
                    vk::DescriptorType::STORAGE_BUFFER => make_descriptor_type!(STORAGE_BUFFER),
                    vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC => {
                        make_descriptor_type!(UNIFORM_BUFFER_DYNAMIC)
                    }
                    vk::DescriptorType::STORAGE_BUFFER_DYNAMIC => {
                        make_descriptor_type!(STORAGE_BUFFER_DYNAMIC)
                    }
                    vk::DescriptorType::INPUT_ATTACHMENT => make_descriptor_type!(INPUT_ATTACHMENT),
                    _ => {
                        panic!("Unknown descriptor type");
                    }
                }

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
        set_debug_names!(self, debug_name, (layout, " Layout"));
    }

    #[cold]
    #[optimize(size)]
    pub unsafe fn create_descriptor_pool(&self) -> vk::DescriptorPool {
        let mut pool_sizes = Vec::new();

        macro_rules! make_descriptor_type {
            ($name:ident) => {
                if self.descriptor_counter.$name != 0 {
                    pool_sizes.push(vk::DescriptorPoolSize {
                        ty: vk::DescriptorType::$name,
                        descriptor_count: self.descriptor_counter.$name,
                    });
                }
            };
        }
        make_descriptor_type!(SAMPLER);
        make_descriptor_type!(COMBINED_IMAGE_SAMPLER);
        make_descriptor_type!(SAMPLED_IMAGE);
        make_descriptor_type!(STORAGE_IMAGE);
        make_descriptor_type!(UNIFORM_TEXEL_BUFFER);
        make_descriptor_type!(STORAGE_TEXEL_BUFFER);
        make_descriptor_type!(UNIFORM_BUFFER);
        make_descriptor_type!(STORAGE_BUFFER);
        make_descriptor_type!(UNIFORM_BUFFER_DYNAMIC);
        make_descriptor_type!(STORAGE_BUFFER_DYNAMIC);
        make_descriptor_type!(INPUT_ATTACHMENT);

        let pool_info = vk::DescriptorPoolCreateInfo {
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: self.descriptor_sets_count * self.settings.fif as u32,
            flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
            ..Default::default()
        };

        self.device.create_descriptor_pool(&pool_info, None).unwrap()
    }

    #[cold]
    #[optimize(size)]
    pub unsafe fn allocate_descriptor(
        device: Device,
        layout: vk::DescriptorSetLayout,
        pool: vk::DescriptorPool,
        count: usize,
    ) -> Ring<vk::DescriptorSet> {
        let layouts = vec![layout; count];
        let alloc_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: pool,
            descriptor_set_count: layouts.len() as u32,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };

        let mut ring = Ring::new(count);
        // return
        let vec = device
            .allocate_descriptor_sets(&alloc_info)
            .expect("Failed to allocate descriptor sets");
        for (i, v) in vec.iter().enumerate() {
            ring[i] = *v;
        }
        ring
    }

    // Tell the LumalRenderer that such descriptor will be setup
    // basically counts needed resources to then allocate them
    #[cold]
    #[optimize(size)]
    pub fn anounce_descriptor_setup(
        &mut self,
        dset_layout: &mut vk::DescriptorSetLayout,
        descriptor_sets: &mut Ring<vk::DescriptorSet>, // Ring to setup into (some setup happens immediately on anounce)
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
            unsafe {
                // actually create layout and write it to ptr
                self.create_descriptor_set_layout(
                    &descriptor_infos,
                    dset_layout,
                    create_flags,
                    #[cfg(feature = "debug_validation_names")]
                    debug_name,
                );
            }
        }

        self.descriptor_sets_count += (self.settings.fif as u32); // cuase dset per fif
    }

    // anounce is just a request, this is an actual logic
    #[cold]
    #[optimize(size)]
    pub unsafe fn actually_setup_descriptor_impl(
        descriptor_pool: &vk::DescriptorPool,
        settings: &LumalSettings,
        device: &Device,
        dset_layout: &vk::DescriptorSetLayout,
        descriptor_sets: &mut Ring<vk::DescriptorSet>,
        descriptions: &[DescriptorInfo],
        stages: vk::ShaderStageFlags,
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
                        DescriptorResource::None => panic!(),
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

    #[cold]
    #[optimize(size)]
    pub fn flush_descriptor_setup(&mut self) {
        // (actually) create Vulkan descriptor pool
        if self.descriptor_pool == vk::DescriptorPool::null() {
            self.descriptor_pool = unsafe { self.create_descriptor_pool() };
        }
    }

    #[cold]
    #[optimize(size)]
    pub fn acutally_setup_descriptor(
        &mut self,
        dset_layout: &mut vk::DescriptorSetLayout,
        descriptor_sets: &mut Ring<vk::DescriptorSet>, // Ring to setup into
        descriptions: &[DescriptorInfo],
        default_stages: vk::ShaderStageFlags,
        create_flags: vk::DescriptorSetLayoutCreateFlags,
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
