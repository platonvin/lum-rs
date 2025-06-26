use crate::webgpu::{wal::Wal, AllSamplers, InternalRendererWebGPU};
use containers::array3d::Dim3;
use wgpu::{
    AddressMode, CompareFunction, FilterMode, Sampler, SamplerBorderColor, SamplerDescriptor,
};

impl<'window, D: Dim3> InternalRendererWebGPU<'window, D> {
    pub fn create_all_samplers(wal: &Wal) -> AllSamplers {
        // Helper closure to create a sampler from a descriptor.
        let create_sampler =
            |desc: SamplerDescriptor<'_>| -> Sampler { wal.device.create_sampler(&desc) };

        let nearest_sampler = create_sampler(SamplerDescriptor {
            label: Some("nearest_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = create_sampler(SamplerDescriptor {
            label: Some("linear_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let linear_sampler_tiled = create_sampler(SamplerDescriptor {
            label: Some("linear_sampler_tiled"),
            address_mode_u: AddressMode::MirrorRepeat,
            address_mode_v: AddressMode::MirrorRepeat,
            address_mode_w: AddressMode::MirrorRepeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let linear_sampler_tiled_mirrored = create_sampler(SamplerDescriptor {
            label: Some("linear_sampler_tiled_mirrored"),
            address_mode_u: AddressMode::MirrorRepeat,
            address_mode_v: AddressMode::MirrorRepeat,
            address_mode_w: AddressMode::MirrorRepeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let unnorm_linear = create_sampler(SamplerDescriptor {
            label: Some("unnorm_linear"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let shadow_sampler = create_sampler(SamplerDescriptor {
            label: Some("shadow_sampler"),
            address_mode_u: AddressMode::MirrorRepeat,
            address_mode_v: AddressMode::MirrorRepeat,
            address_mode_w: AddressMode::MirrorRepeat,
            compare: Some(CompareFunction::Less),
            border_color: Some(SamplerBorderColor::OpaqueWhite),
            ..Default::default()
        });

        AllSamplers {
            nearest_sampler,
            linear_sampler,
            linear_sampler_tiled,
            linear_sampler_tiled_mirrored,
            shadow_sampler,
            unnorm_linear,
        }
    }
}
