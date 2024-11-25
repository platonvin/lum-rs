use crate::{Buffer, DescriptorCounter, Image, RasterPipe};
use crate::LumalRenderer; 
use vulkanalia::vk::{self, DeviceV1_0};
use anyhow::*;

use vulkanalia::prelude::v1_0::*;

// Enum for BlendAttachment
pub enum BlendAttachment {
    NoBlend,
    BlendMix,
    BlendSub,
    BlendReplaceIfGreater, // Basically max
    BlendReplaceIfLess,    // Basically min
}

// Enum for DepthTesting
pub enum DepthTesting {
    DepthTestNoneBit = 0,
    DepthTestReadBit = 1 << 0,
    DepthTestWriteBit = 1 << 1,
}

// Enum for Discard
pub enum Discard {
    NoDiscard,
    DoDiscard,
}

// Enum for LoadStoreOp
pub enum LoadStoreOp {
    DontCare,
    Clear,
    Store,
    Load,
}
// Structure for AttachmentDescription
pub struct AttachmentDescription {
    pub images: Vec<Image>, // Assuming ring is a Vec-like structure
    pub load: LoadStoreOp,
    pub store: LoadStoreOp,
    pub sload: LoadStoreOp,
    pub sstore: LoadStoreOp,
    pub clear: vk::ClearValue,
    pub final_layout: vk::ImageLayout, // Default value is GENERAL
}

// Structure for SubpassAttachments
pub struct SubpassAttachments {
    pub pipes: Vec<RasterPipe>,
    pub a_input: Vec<Vec<Image>>, // Input images for the subpass
    pub a_color: Vec<Vec<Image>>, // Color images for the subpass
    pub a_depth: Vec<Image>,         // Depth image for the subpass
}

// Structure for SubpassAttachmentRefs
pub struct SubpassAttachmentRefs {
    pub a_input: Vec<vk::AttachmentReference>,
    pub a_color: Vec<vk::AttachmentReference>,
    pub a_depth: vk::AttachmentReference,
}
// Enum for RelativeDescriptorPos (relative descriptor positions)
pub enum RelativeDescriptorPos {
    RDNone,     // What?
    RDPrevious, // Relative Descriptor position previous - for accumulators
    RDCurrent,  // Relative Descriptor position matching - common CPU-paired
    RDFirst,    // Relative Descriptor position first - for GPU-only
}

// Structure for ShaderStage
pub struct ShaderStage {
    pub src: *const i8,              // Source code as a C string (raw pointer)
    pub stage: vk::ShaderStageFlags, // Shader stage flags
}

// Structure for DescriptorInfo
pub struct DescriptorInfo {
    pub descriptor_type: vk::DescriptorType,
    pub relative_pos: RelativeDescriptorPos,
    pub buffers: Option<Vec<Buffer>>, // Option ring of Buffers
    pub images: Option<Vec<Image>>,   // Option ring of Images
    pub image_sampler: vk::Sampler,
    pub image_layout: vk::ImageLayout, // Image layout for use (not current)
    pub stages: vk::ShaderStageFlags,  // Shader stages
}

// Structure for ShortDescriptorInfo
pub struct ShortDescriptorInfo {
    pub descriptor_type: vk::DescriptorType,
    pub stages: vk::ShaderStageFlags,
}

// Structure for DelayedDescriptorSetup
pub struct DelayedDescriptorSetup {
    pub set_layout: *mut vk::DescriptorSetLayout, // Pointer to descriptor set layout
    pub sets: Option<Vec<vk::DescriptorSet>>, // Option ring of descriptor sets
    pub descriptions: Vec<DescriptorInfo>,     // Descriptors
    pub stages: vk::ShaderStageFlags,
    pub create_flags: vk::DescriptorSetLayoutCreateFlags, // Create flags
}

impl LumalRenderer {
    /// immediately creates vulkan descriptor set layout
    pub unsafe fn create_descriptor_set_layout(
        &self,
        descriptor_infos: Vec<ShortDescriptorInfo>,
        layout: &mut vk::DescriptorSetLayout,
        flags: vk::DescriptorSetLayoutCreateFlags,
    ) {
        let bindings: Vec<vk::DescriptorSetLayoutBinding> = descriptor_infos
            .iter()
            .enumerate()
            .map(|(i, info)| vk::DescriptorSetLayoutBinding {
                binding: i as u32,
                descriptor_type: info.descriptor_type,
                descriptor_count: 1,
                stage_flags: info.stages,
                ..Default::default()
            })
            .collect();

        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            flags,
            binding_count: bindings.len() as u32,
            bindings: bindings.as_ptr(),
            ..Default::default()
        };
        
        // actually create layout and write it to ptr 
        *layout = self.device
            .create_descriptor_set_layout(&layout_info, None)
            .expect("Failed to create descriptor set layout");
    }

    pub unsafe fn create_descriptor_pool(&self) -> Result<vk::DescriptorPool> {
        let mut pool_sizes = Vec::new();

        macro_rules! make_descriptor_type {
            ($name:ident) => {
                if self.descriptor_counter.$name != 0 {
                    pool_sizes.push(vk::DescriptorPoolSize {
                        type_: vk::DescriptorType::$name,
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
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: pool_sizes.len() as u32,
            pool_sizes: pool_sizes.as_ptr(),
            max_sets: self.descriptor_sets_count * self.settings.fif as u32,
            ..Default::default()
        };

        Ok(self.device
            .create_descriptor_pool(&pool_info, None)?)
    }

    pub unsafe fn allocate_descriptor(
        &self,
        sets: &mut Vec<vk::DescriptorSet>,
        layout: vk::DescriptorSetLayout,
        pool: vk::DescriptorPool,
        count: usize,
    ) {
        let layouts = vec![layout; count];
        let alloc_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            descriptor_pool: pool,
            descriptor_set_count: layouts.len() as u32,
            set_layouts: layouts.as_ptr(),
            ..Default::default()
        };

        *sets = self
            .device
            .allocate_descriptor_sets(&alloc_info)
            .expect("Failed to allocate descriptor sets");
    }

    pub fn reset_descriptor_setup(&mut self) {
        self.delayed_descriptor_setups.clear();
    }

    pub fn defer_descriptor_setup(
        &mut self,
        dset_layout: & mut vk::DescriptorSetLayout,
        descriptor_sets: & mut Vec<vk::DescriptorSet>,
        descriptions: Vec<DescriptorInfo>,
        base_stages: vk::ShaderStageFlags,
        create_flags: vk::DescriptorSetLayoutCreateFlags,
    ) {
        if *dset_layout == vk::DescriptorSetLayout::null() {
            let descriptor_infos: Vec<ShortDescriptorInfo> = descriptions
                .iter()
                .map(|desc| ShortDescriptorInfo {
                    descriptor_type: desc.descriptor_type,
                    stages: if desc.stages.is_empty() {
                        base_stages
                    } else {
                        desc.stages
                    },
                })
                .collect();
            unsafe {
                // actually create layout and write it to ptr 
                self.create_descriptor_set_layout(descriptor_infos, dset_layout, create_flags);
            }
        }

        self.descriptor_sets_count += self.settings.fif as u32; // cuase dset per fif
        self.delayed_descriptor_setups.push(
            DelayedDescriptorSetup {
                set_layout: dset_layout,
                sets: Some(descriptor_sets.to_vec()),
                descriptions: descriptions,
                stages: base_stages,
                create_flags,
            }
        );
    }
}


// impl LumalRenderer {
//     // defer is just a request, this is an actual logic
//     pub unsafe fn actually_setup_descriptor(
//         &self,
//         dset_layout: &vk::DescriptorSetLayout,
//         descriptor_sets: &mut Vec<vk::DescriptorSet>,
//         descriptions: &[DescriptorInfo],
//         stages: vk::ShaderStageFlags,
//     ) {
//         for frame_i in 0..descriptor_sets.len() {
//             let previous_frame_i = if frame_i == 0 {
//                 self.settings.fif as usize - 1
//             } else {
//                 frame_i - 1
//             };

//             let mut image_infos = vec![vk::DescriptorImageInfo::default(); descriptions.len()];
//             let mut buffer_infos = vec![vk::DescriptorBufferInfo::default(); descriptions.len()];
//             let mut writes = vec![vk::WriteDescriptorSet::default(); descriptions.len()];

//             for (i, desc) in descriptions.iter().enumerate() {
//                 writes[i] = vk::WriteDescriptorSet {
//                     s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
//                     dst_set: descriptor_sets[frame_i],
//                     dst_binding: i as u32,
//                     dst_array_element: 0,
//                     descriptor_count: 1,
//                     descriptor_type: desc.descriptor_type,
//                     ..Default::default()
//                 };

//                 let descriptor_frame_id = match desc.relative_pos {
//                     RelativeDescriptorPos::RDCurrent => frame_i,
//                     RelativeDescriptorPos::RDPrevious => previous_frame_i,
//                     RelativeDescriptorPos::RDFirst => 0,
//                     RelativeDescriptorPos::RDNone => {
//                         writes[i].descriptor_count = 0;
//                         continue;
//                     }
//                 };

//                 if let Some(images) = &desc.images {
//                     assert!(images[descriptor_frame_id].view != vk::ImageView::null());
//                     image_infos[i] = vk::DescriptorImageInfo {
//                         image_view: images[descriptor_frame_id].view,
//                         image_layout: desc.image_layout,
//                         sampler: desc.image_sampler.unwrap_or(vk::Sampler::null()),
//                     };
//                     writes[i].p_image_info = &image_infos[i];

//                     assert!(desc.buffers.is_none());
//                     if desc.image_sampler.is_some()
//                         && desc.descriptor_type != vk::DescriptorType::COMBINED_IMAGE_SAMPLER
//                     {
//                         panic!("Descriptor has sampler but type is not for sampler");
//                     }
//                 } else if let Some(buffers) = &desc.buffers {
//                     buffer_infos[i] = vk::DescriptorBufferInfo {
//                         buffer: buffers[descriptor_frame_id].buffer,
//                         offset: 0,
//                         range: vk::WHOLE_SIZE,
//                     };
//                     writes[i].p_buffer_info = &buffer_infos[i];
//                 } else {
//                     panic!("Unknown descriptor type");
//                 }
//             }

//             self.device.update_descriptor_sets(&writes, &[]);
//         }
//     }

//     pub unsafe fn flush_descriptor_setup(&mut self) {
//         // create vulkan descriptor pool
//         self.vulkan_data.descriptor_pool = self.create_descriptor_pool()?;
        
//         // create descriptor sets. Layouts are alredy created
//         for setup in &self.delayed_descriptor_setups {
//             if let Some(sets) = &setup.sets {
//                 if sets.is_empty() {
//                     self.allocate_descriptor(
//                         &mut sets,
//                         *setup.set_layout,
//                         self.v.unwrap(),
//                         self.settings.fif as usize,
//                     );
//                 }
//                 self.actually_setup_descriptor(
//                     setup.set_layout,
//                     sets,
//                     &setup.descriptions,
//                     setup.stages,
//                 );
//             }
//         }
//     }
// }
