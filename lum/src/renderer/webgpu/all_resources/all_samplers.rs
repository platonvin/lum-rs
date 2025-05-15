use wgpu::{AddressMode, FilterMode, Sampler, SamplerBorderColor, SamplerDescriptor};

use crate::renderer::{
    webgpu::{wal::Wal, AllSamplers, InternalRendererWebGPU},
    Settings,
};

impl<'window> InternalRendererWebGPU<'window> {
    #[cold]
    #[optimize(size)]
    pub fn create_all_samplers(wal: &Wal) -> AllSamplers {
        // A helper closure to create a sampler from a descriptor.
        let create_sampler =
            |desc: SamplerDescriptor<'_>| -> Sampler { wal.device.create_sampler(&desc) };

        // Base sampler descriptor (closest matching to your Vulkan base_sampler_info)
        // Note: WGPU samplers do not support a border color. We use ClampToEdge for the base.
        let base_sampler_desc = &SamplerDescriptor {
            label: Some("base_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            // WGPU does not offer separate anisotropy_enable and max_anisotropy: use anisotropy_clamp (None = disabled)
            anisotropy_clamp: 1,
            // For non-shadow samplers, compare mode is off.
            compare: None,
            ..Default::default()
        };

        // Nearest Sampler: exactly the base descriptor.
        let nearest_sampler = create_sampler(base_sampler_desc.clone());

        // Linear Sampler: change filters to linear.
        let linear_sampler_desc = SamplerDescriptor {
            label: Some("linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            // keep other fields from base_sampler_desc:
            address_mode_u: base_sampler_desc.address_mode_u,
            address_mode_v: base_sampler_desc.address_mode_v,
            address_mode_w: base_sampler_desc.address_mode_w,
            mipmap_filter: base_sampler_desc.mipmap_filter,
            lod_min_clamp: base_sampler_desc.lod_min_clamp,
            lod_max_clamp: base_sampler_desc.lod_max_clamp,
            anisotropy_clamp: base_sampler_desc.anisotropy_clamp,
            compare: None,
            ..Default::default()
        };
        let linear_sampler = create_sampler(linear_sampler_desc);

        // Linear Tiled Sampler: change address mode to MirrorRepeat.
        let linear_sampler_tiled_desc = SamplerDescriptor {
            label: Some("linear_sampler_tiled"),
            address_mode_u: AddressMode::MirrorRepeat,
            address_mode_v: AddressMode::MirrorRepeat,
            address_mode_w: AddressMode::MirrorRepeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: base_sampler_desc.mipmap_filter,
            lod_min_clamp: base_sampler_desc.lod_min_clamp,
            lod_max_clamp: base_sampler_desc.lod_max_clamp,
            anisotropy_clamp: base_sampler_desc.anisotropy_clamp,
            compare: None,
            ..Default::default()
        };
        let linear_sampler_tiled = create_sampler(linear_sampler_tiled_desc.clone());
        // In your original code, linear_sampler_tiled_mirrored used the same settings.
        let linear_sampler_tiled_mirrored = create_sampler(linear_sampler_tiled_desc);

        // Overlay Sampler: reuse base but with nearest filtering.
        let overlay_sampler_desc = SamplerDescriptor {
            label: Some("overlay_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..base_sampler_desc.clone()
        };
        let overlay_sampler = create_sampler(overlay_sampler_desc);

        // Unnormalized Linear Sampler: use REPEAT and linear filters.
        let unnorm_linear_desc = SamplerDescriptor {
            label: Some("unnorm_linear"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            // In Vulkan, unnormalized_coordinates is set but WGPU does not support that option,
            // so we ignore it.
            ..base_sampler_desc.clone()
        };
        let unnorm_linear = create_sampler(unnorm_linear_desc);

        // Unnormalized Nearest Sampler: use REPEAT and nearest filters.
        let unnorm_nearest_desc = SamplerDescriptor {
            label: Some("unnorm_nearest"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..base_sampler_desc.clone()
        };
        let unnorm_nearest = create_sampler(unnorm_nearest_desc);

        // Shadow Sampler: use mirrored repeat, nearest filtering,
        // and enable depth comparison (using CompareFunction::Less).
        let shadow_sampler_desc = SamplerDescriptor {
            label: Some("shadow_sampler"),
            address_mode_u: AddressMode::MirrorRepeat,
            address_mode_v: AddressMode::MirrorRepeat,
            address_mode_w: AddressMode::MirrorRepeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::Less),
            border_color: Some(SamplerBorderColor::OpaqueWhite),
            ..base_sampler_desc.clone()
        };
        let shadow_sampler = create_sampler(shadow_sampler_desc);

        AllSamplers {
            nearest_sampler: Some(nearest_sampler),
            linear_sampler: Some(linear_sampler),
            linear_sampler_tiled: Some(linear_sampler_tiled),
            linear_sampler_tiled_mirrored: Some(linear_sampler_tiled_mirrored),
            overlay_sampler: Some(overlay_sampler),
            shadow_sampler: Some(shadow_sampler),
            unnorm_linear: Some(unnorm_linear),
            unnorm_nearest: Some(unnorm_nearest),
        }
    }

    pub fn destroy_all_samplers(_wal: &mut Wal, samplers: &mut AllSamplers) {}
}
