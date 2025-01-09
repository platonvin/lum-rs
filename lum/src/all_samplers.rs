use lumal::{descriptors::{DescriptorInfo, RelativeDescriptorPos, ShortDescriptorInfo}, ring::Ring, ComputePipe, LumalRenderer, LumalSettings, RasterPipe};
use vulkanalia::vk::{self, DeviceV1_0, Extent2D, Handle};
use vk::Sampler;
use RelativeDescriptorPos::*;

use crate::*;

impl crate::LumRenderer {
    pub fn create_all_samplers(lumal: &LumalRenderer, lum_settings: &LumSettings, lumal_settings: &LumalSettings) -> LumSamplers {

        let mut create_sampler = |info: vk::SamplerCreateInfo| -> vk::Sampler {
            unsafe {
                lumal.create_sampler(&info).expect("Failed to create sampler")
            }
        };
        
        let base_sampler_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            next: std::ptr::null(),
            mag_filter: vk::Filter::NEAREST,
            min_filter: vk::Filter::NEAREST,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            mip_lod_bias: 0.0,
            anisotropy_enable: vk::FALSE,
            max_anisotropy: 1.0,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::LESS_OR_EQUAL,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: vk::BorderColor::FLOAT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            ..Default::default()
        };

        // Nearest Sampler
        let nearest_sampler = create_sampler(base_sampler_info);

        // Linear Sampler
        let linear_sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            ..base_sampler_info
        };
        let linear_sampler = create_sampler(linear_sampler_info);

        // Linear Tiled Sampler
        let linear_sampler_tiled_info = vk::SamplerCreateInfo {
            address_mode_u: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_v: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_w: vk::SamplerAddressMode::MIRRORED_REPEAT,
            ..linear_sampler_info
        };
        let linear_sampler_tiled = create_sampler(linear_sampler_tiled_info);
        let linear_sampler_tiled_mirrored = create_sampler(linear_sampler_tiled_info);

        // Overlay Sampler
        let overlay_sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::NEAREST,
            min_filter: vk::Filter::NEAREST,
            ..base_sampler_info
        };
        let overlay_sampler = create_sampler(overlay_sampler_info);

        // Unnormalized Linear Sampler
        let unnorm_linear_info = vk::SamplerCreateInfo {
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            unnormalized_coordinates: vk::FALSE,
            ..base_sampler_info
        };
        let unnorm_linear = create_sampler(unnorm_linear_info);

        // Unnormalized Nearest Sampler
        let unnorm_nearest_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::NEAREST,
            min_filter: vk::Filter::NEAREST,
            ..unnorm_linear_info
        };
        let unnorm_nearest = create_sampler(unnorm_nearest_info);

        // shadowmap Sampler with possible hw depth comparison filtering
        let shadow_sampler_info = vk::SamplerCreateInfo {
            address_mode_u: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_v: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_w: vk::SamplerAddressMode::MIRRORED_REPEAT,
            compare_enable: vk::TRUE,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            compare_op: vk::CompareOp::LESS,
            ..unnorm_nearest_info
        };
        let shadow_sampler = create_sampler(shadow_sampler_info);

        return LumSamplers {
            nearest_sampler,
            linear_sampler,
            linear_sampler_tiled,
            linear_sampler_tiled_mirrored,
            overlay_sampler,
            shadow_sampler,
            unnorm_linear,
            unnorm_nearest,
        }
    }  
}