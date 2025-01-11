use vulkanalia::vk; 
use vulkanalia::vk::ImageLayout; 
use vulkanalia::vk::ShaderStageFlags; 
use vulkanalia::vk::DescriptorType;

// Rust provokes you to write code in a way that does not play well with how i like to write code
// so sometimes i have to use crutches to get around it
// also, now i regret chosing vulkanalia as a rust wrapper

pub const UNDEFINED: vk::ImageLayout = ImageLayout::from_raw(0);
pub const GENERAL: vk::ImageLayout = ImageLayout::from_raw(1);
pub const COLOR_ATTACHMENT_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(2);
pub const DEPTH_STENCIL_ATTACHMENT_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(3);
pub const DEPTH_STENCIL_READ_ONLY_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(4);
pub const SHADER_READ_ONLY_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(5);
pub const TRANSFER_SRC_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(6);
pub const TRANSFER_DST_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(7);
pub const PREINITIALIZED: vk::ImageLayout = ImageLayout::from_raw(8);
pub const DEPTH_READ_ONLY_STENCIL_ATTACHMENT_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(1000117000);
pub const DEPTH_ATTACHMENT_STENCIL_READ_ONLY_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(1000117001);
pub const DEPTH_ATTACHMENT_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(1000241000);
pub const DEPTH_READ_ONLY_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(1000241001);
pub const STENCIL_ATTACHMENT_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(1000241002);
pub const STENCIL_READ_ONLY_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(1000241003);
pub const READ_ONLY_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(1000314000);
pub const ATTACHMENT_OPTIMAL: vk::ImageLayout = ImageLayout::from_raw(1000314001);
pub const PRESENT_SRC_KHR: vk::ImageLayout = ImageLayout::from_raw(1000001002);

pub const VERTEX : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1)};
pub const TESSELLATION_CONTROL : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 1)};
pub const TESSELLATION_EVALUATION : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 2)};
pub const GEOMETRY : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 3)};
pub const FRAGMENT : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 4)};
pub const ALL_GRAPHICS : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(VERTEX.bits() | TESSELLATION_CONTROL.bits() | TESSELLATION_EVALUATION.bits() | GEOMETRY.bits() | FRAGMENT.bits())};
pub const COMPUTE : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 5)};
pub const TASK_EXT : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 6)};
pub const MESH_EXT : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 7)};
pub const RAYGEN_KHR : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 8)};
pub const ANY_HIT_KHR : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 9)};
pub const CLOSEST_HIT_KHR : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 10)};
pub const MISS_KHR : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 11)};
pub const INTERSECTION_KHR : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 12)};
pub const CALLABLE_KHR : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 13)};
pub const SUBPASS_SHADING_HUAWEI : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 14)};
pub const CLUSTER_CULLING_HUAWEI : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(1 << 19)};
pub const ALL : vk::ShaderStageFlags = unsafe { ShaderStageFlags::from_bits_unchecked(i32::MAX as u32)};


pub const SAMPLER: vk::DescriptorType = vk::DescriptorType::from_raw(0);
pub const COMBINED_IMAGE_SAMPLER: vk::DescriptorType = vk::DescriptorType::from_raw(1);
pub const SAMPLED_IMAGE: vk::DescriptorType = vk::DescriptorType::from_raw(2);
pub const STORAGE_IMAGE: vk::DescriptorType = vk::DescriptorType::from_raw(3);
pub const UNIFORM_TEXEL_BUFFER: vk::DescriptorType = vk::DescriptorType::from_raw(4);
pub const STORAGE_TEXEL_BUFFER: vk::DescriptorType = vk::DescriptorType::from_raw(5);
pub const UNIFORM_BUFFER: vk::DescriptorType = vk::DescriptorType::from_raw(6);
pub const STORAGE_BUFFER: vk::DescriptorType = vk::DescriptorType::from_raw(7);
pub const UNIFORM_BUFFER_DYNAMIC: vk::DescriptorType = vk::DescriptorType::from_raw(8);
pub const STORAGE_BUFFER_DYNAMIC: vk::DescriptorType = vk::DescriptorType::from_raw(9);
pub const INPUT_ATTACHMENT: vk::DescriptorType = vk::DescriptorType::from_raw(10);
pub const INLINE_UNIFORM_BLOCK: vk::DescriptorType = vk::DescriptorType::from_raw(1000138000);
pub const ACCELERATION_STRUCTURE_KHR: vk::DescriptorType = vk::DescriptorType::from_raw(1000150000);
pub const ACCELERATION_STRUCTURE_NV: vk::DescriptorType = vk::DescriptorType::from_raw(1000165000);
pub const SAMPLE_WEIGHT_IMAGE_QCOM: vk::DescriptorType = vk::DescriptorType::from_raw(1000440000);
pub const BLOCK_MATCH_IMAGE_QCOM: vk::DescriptorType = vk::DescriptorType::from_raw(1000440001);
pub const MUTABLE_EXT: vk::DescriptorType = vk::DescriptorType::from_raw(1000351000);