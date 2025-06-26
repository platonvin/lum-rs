use crate::{types::*, webgpu::PipeWithPushConstants};
use crate::{
    // renderer::webgpu::types::*,
    webgpu::{
        wal::{
            DynamicBindGroupDescription, Image, ShaderStageSource, StaticBindGroupDescription, Wal,
        },
        AllBuffers, AllIndependentImages, AllPipes, AllSamplers, AllSwapchainDependentImages,
        InternalRendererWebGPU, MeshFoliageDesc, FRAME_FORMAT, MATNORM_FORMAT,
    },
    Settings,
};
use containers::array3d::Dim3;
use containers::Ring;
use shaders::Shader;
use std::mem::offset_of;
use wgpu::*;

pub fn buffers_to_binding_resources<'a>(
    buffers: &'a Ring<wgpu::Buffer>,
) -> Ring<BindingResource<'a>> {
    let resources = buffers
        .iter()
        .map(|buffer| {
            BindingResource::Buffer(BufferBinding {
                buffer,
                offset: 0,
                size: None,
            })
        })
        .collect();
    Ring::from_vec(resources)
}
pub fn images_to_binding_resources<'a>(images: &'a Ring<Image>) -> Ring<BindingResource<'a>> {
    let resources = images.iter().map(|img| BindingResource::TextureView(&img.view)).collect();
    Ring::from_vec(resources)
}
pub fn images_to_binding_resources_previous<'a>(
    images: &'a Ring<Image>,
) -> Ring<BindingResource<'a>> {
    Ring::from_vec(
        (0..images.len())
            .map(|i| {
                let src_i = if i == 0 { images.len() - 1 } else { i - 1 };
                BindingResource::TextureView(&images[src_i].view)
            })
            .collect(),
    )
}
pub fn sampler_to_binding_resources<'a>(sampler: &'a wgpu::Sampler) -> Ring<BindingResource<'a>> {
    let resources = vec![BindingResource::Sampler(sampler)];
    Ring::from_vec(resources)
}

pub fn rings_of_buffers_to_ring_of_buffer_bindings<'a>(
    rings_of_buffers: &[Ring<&'a wgpu::Buffer>],
) -> Option<Ring<Vec<BindingResource<'a>>>> {
    if rings_of_buffers.is_empty() {
        return None;
    }

    let first_length = rings_of_buffers[0].len();

    // Check for consistent lengths using `all`
    if !rings_of_buffers.iter().all(|ring| ring.len() == first_length) {
        panic!("Error: Input rings have inconsistent lengths.");
    }

    Some(Ring::from_vec(
        (0..first_length)
            .map(|i| {
                rings_of_buffers
                    .iter()
                    .map(|ring| {
                        BindingResource::Buffer(BufferBinding {
                            buffer: ring.get(i),
                            offset: 0,
                            size: None,
                        })
                    })
                    .collect()
            })
            .collect(),
    ))
}

pub fn rings_of_texture_views_to_ring_of_texture_view_bindings<'a>(
    rings_of_texture_views: &[Ring<&'a wgpu::TextureView>],
) -> Option<Ring<Vec<BindingResource<'a>>>> {
    if rings_of_texture_views.is_empty() {
        return None;
    }

    let first_length = rings_of_texture_views[0].len();

    if !rings_of_texture_views.iter().all(|ring| ring.len() == first_length) {
        panic!("Input rings have inconsistent lengths.");
    }

    Some(Ring::from_vec(
        (0..first_length)
            .map(|i| {
                rings_of_texture_views
                    .iter()
                    .map(|ring| BindingResource::TextureView(ring.get(i)))
                    .collect()
            })
            .collect(),
    ))
}

pub struct PackedVoxelCircuit {
    pub pos: u8vec4,
}

impl<'window, D: Dim3> InternalRendererWebGPU<'window, D> {
    pub fn create_all_pipes(
        wal: &Wal,
        _lum_settings: &Settings<D>,
        buffers: &AllBuffers,
        iimages: &AllIndependentImages,
        dimages: &AllSwapchainDependentImages,
        samplers: &AllSamplers,
        foliage_descriptions: &[MeshFoliageDesc],
    ) -> AllPipes {
        let lightmap_blocks_pipe = Wal::create_raster_pipe(
            wal,
            &[StaticBindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                binding_type: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resources: buffers_to_binding_resources(&buffers.uniform),
            }],
            &[DynamicBindGroupDescription {
                // push constant but not through regular emulation
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                binding_type: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            }],
            shaders::Shader::get_wgsl(Shader::LightmapBlocksVert),
            None,
            &[VertexBufferLayout {
                array_stride: size_of::<PackedVoxelCircuit>() as u64,
                step_mode: VertexStepMode::Vertex,
                attributes: &[VertexAttribute {
                    format: VertexFormat::Uint8x4,
                    offset: offset_of!(PackedVoxelCircuit, pos) as u64,
                    shader_location: 0,
                }],
            }],
            PrimitiveTopology::TriangleList,
            vec![],
            Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("lightmap blocks"),
        );
        let lightmap_models_pipe = Wal::create_raster_pipe(
            wal,
            &[StaticBindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                binding_type: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resources: buffers_to_binding_resources(&buffers.uniform),
            }],
            &[
                DynamicBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                DynamicBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Sint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                },
            ],
            shaders::Shader::get_wgsl(Shader::LightmapModelsVert),
            None,
            &[VertexBufferLayout {
                array_stride: size_of::<PackedVoxelCircuit>() as u64,
                step_mode: VertexStepMode::Vertex,
                attributes: &[VertexAttribute {
                    format: VertexFormat::Uint8x4,
                    offset: offset_of!(PackedVoxelCircuit, pos) as u64,
                    shader_location: 0,
                }],
            }],
            PrimitiveTopology::TriangleList,
            vec![],
            Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Lightmap Models"),
        );

        let raygen_blocks_pipe = Wal::create_raster_pipe(
            wal,
            &[
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                StaticBindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Sint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    resources: images_to_binding_resources(&iimages.block_palette),
                },
            ],
            &[DynamicBindGroupDescription {
                // push constant but not through regular emulation
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                binding_type: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            }],
            shaders::Shader::get_wgsl(Shader::RaygenBlocksVert),
            Some(shaders::Shader::get_wgsl(Shader::RaygenBlocksFrag)),
            &[VertexBufferLayout {
                array_stride: size_of::<PackedVoxelCircuit>() as u64,
                step_mode: VertexStepMode::Vertex,
                attributes: &[VertexAttribute {
                    format: VertexFormat::Uint8x4,
                    offset: offset_of!(PackedVoxelCircuit, pos) as u64,
                    shader_location: 0,
                }],
            }],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: MATNORM_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Raygen Blocks"),
        );

        let raygen_models_pipe = Wal::create_raster_pipe(
            wal,
            &[StaticBindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                binding_type: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resources: buffers_to_binding_resources(&buffers.uniform),
            }],
            &[
                DynamicBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                DynamicBindGroupDescription {
                    binding: 1, // group 1 binding 1. So 1 after the pco
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Sint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    }, // aka push descriptor sets in vk
                },
            ],
            shaders::Shader::get_wgsl(Shader::RaygenModelsVert),
            Some(shaders::Shader::get_wgsl(Shader::RaygenModelsFrag)),
            &[VertexBufferLayout {
                array_stride: size_of::<PackedVoxelCircuit>() as u64,
                step_mode: VertexStepMode::Vertex,
                attributes: &[VertexAttribute {
                    format: VertexFormat::Uint8x4,
                    offset: offset_of!(PackedVoxelCircuit, pos) as u64,
                    shader_location: 0,
                }],
            }],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: MATNORM_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Raygen Models"),
        );

        let raygen_particles_pipe = Wal::create_raster_pipe(
            wal,
            &[StaticBindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                binding_type: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resources: buffers_to_binding_resources(&buffers.uniform),
            }],
            &[],
            shaders::Shader::get_wgsl(Shader::RaygenParticlesVert),
            Some(shaders::Shader::get_wgsl(Shader::RaygenParticlesFrag)),
            &[VertexBufferLayout {
                array_stride: size_of::<Particle>() as u64,
                step_mode: VertexStepMode::Instance,
                attributes: &[
                    VertexAttribute {
                        format: VertexFormat::Float32x3,
                        offset: offset_of!(Particle, pos) as u64,
                        shader_location: 0,
                    },
                    VertexAttribute {
                        format: VertexFormat::Float32x3,
                        offset: offset_of!(Particle, vel) as u64,
                        shader_location: 1,
                    },
                    VertexAttribute {
                        format: VertexFormat::Float32,
                        offset: offset_of!(Particle, life_time) as u64,
                        shader_location: 2,
                    },
                    VertexAttribute {
                        format: VertexFormat::Uint8,
                        offset: offset_of!(Particle, mat_id) as u64,
                        shader_location: 3,
                    },
                ],
            }],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: MATNORM_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Raygen Particles"),
        );

        let raygen_water_pipe = Wal::create_raster_pipe(
            wal,
            &[
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                StaticBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &iimages.water_state.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    binding_type: BindingType::Sampler(SamplerBindingType::Filtering),
                    resources: sampler_to_binding_resources(
                        &samplers.linear_sampler_tiled_mirrored,
                    ),
                },
            ],
            &[DynamicBindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                binding_type: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            }],
            shaders::Shader::get_wgsl(Shader::WaterVert),
            Some(shaders::Shader::get_wgsl(Shader::WaterFrag)),
            &[],
            PrimitiveTopology::TriangleStrip,
            vec![Some(ColorTargetState {
                format: MATNORM_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Raygen Water"),
        );

        let raygen_water_pipe = {
            let pc_buffer = wal.create_buffer(
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                16 * 1024 * 20,
                Some("PC buffer for water"),
            );
            let pc_bind_group = wal.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("PC buffer for water"),
                layout: raygen_water_pipe.dynamic_bind_group_layout.as_ref().unwrap(),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(pc_buffer.as_entire_buffer_binding()),
                }],
            });

            PipeWithPushConstants {
                pipe: raygen_water_pipe,
                pc_buffer: Some(pc_buffer),
                pc_bg: Some(pc_bind_group),
                push_constants: vec![],
                pc_count: 0,
            }
        };

        let diffuse_pipe = Wal::create_raster_pipe(
            wal,
            &[
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                StaticBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &dimages.mat_norm.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &dimages.depth.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &iimages.material_palette.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 4,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    binding_type: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    resources: sampler_to_binding_resources(&samplers.nearest_sampler),
                },
                StaticBindGroupDescription {
                    binding: 5,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    resources: images_to_binding_resources(&iimages.radiance_cache),
                },
                StaticBindGroupDescription {
                    binding: 6,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    binding_type: BindingType::Sampler(SamplerBindingType::Filtering),
                    resources: sampler_to_binding_resources(&samplers.unnorm_linear),
                },
                StaticBindGroupDescription {
                    binding: 7,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &iimages.lightmap.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 8,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    binding_type: BindingType::Sampler(SamplerBindingType::Comparison),
                    resources: sampler_to_binding_resources(&samplers.shadow_sampler),
                },
            ],
            &[],
            shaders::Shader::get_wgsl(Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_wgsl(Shader::DiffuseFrag)),
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: FRAME_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            None,
            None,
            Some("Diffuse"),
        );

        // darkens certain areas on the frame image depending on screen-space normal variation of pixels
        // this is achieved by mixing black with current frame
        let ao_pipe = Wal::create_raster_pipe(
            wal,
            &[
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                StaticBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    resources: buffers_to_binding_resources(&buffers.ao_lut_uniform),
                },
                StaticBindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &dimages.mat_norm.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &dimages.depth.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    resources: sampler_to_binding_resources(&samplers.nearest_sampler),
                },
            ],
            &[],
            shaders::Shader::get_wgsl(Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_wgsl(Shader::HbaoFrag)),
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: FRAME_FORMAT,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            None,
            None,
            Some("Ambient Occlusion"),
        );
        let fill_stencil_glossy_pipe = Wal::create_raster_pipe(
            wal,
            &[
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &dimages.mat_norm.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &iimages.material_palette.view,
                    )]),
                },
            ],
            &[],
            shaders::Shader::get_wgsl(Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_wgsl(Shader::FillStencilGlossyFrag)),
            &[],
            PrimitiveTopology::TriangleList,
            vec![],
            Some(DepthStencilState {
                format: TextureFormat::Stencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState {
                        compare: CompareFunction::Always,
                        pass_op: StencilOperation::Replace,
                        fail_op: StencilOperation::Replace,
                        depth_fail_op: StencilOperation::Replace,
                    },
                    back: StencilFaceState {
                        compare: CompareFunction::Always,
                        pass_op: StencilOperation::Replace,
                        fail_op: StencilOperation::Replace,
                        depth_fail_op: StencilOperation::Replace,
                    },
                    read_mask: 0x00,
                    write_mask: 0x01, // ITS NOT THE SAME AS VULKAN????
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Fill Stencil for Glossy"),
        );

        let fill_stencil_smoke_pipe = Wal::create_raster_pipe(
            wal,
            &[StaticBindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                binding_type: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                resources: buffers_to_binding_resources(&buffers.uniform),
            }],
            &[DynamicBindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                binding_type: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            }],
            shaders::Shader::get_wgsl(Shader::FillStencilSmokeVert),
            Some(shaders::Shader::get_wgsl(Shader::FillStencilSmokeFrag)),
            &[],
            PrimitiveTopology::TriangleList,
            // these are emulating depth in a single pass
            vec![
                Some(ColorTargetState {
                    format: TextureFormat::R16Float,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Min,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::Zero,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                }),
                Some(ColorTargetState {
                    format: TextureFormat::R16Float,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Max,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::Zero,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                }),
            ],
            Some(DepthStencilState {
                format: TextureFormat::Stencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState {
                        compare: CompareFunction::Always,
                        pass_op: StencilOperation::Replace,
                        fail_op: StencilOperation::Replace,
                        depth_fail_op: StencilOperation::Replace,
                    },
                    back: StencilFaceState {
                        compare: CompareFunction::Always,
                        pass_op: StencilOperation::Replace,
                        fail_op: StencilOperation::Replace,
                        depth_fail_op: StencilOperation::Replace,
                    },
                    // dont care
                    read_mask: 0x00,
                    // mark all pixels that have a chance to see smoke to cull expensive smoke shader
                    write_mask: 0x02,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Fill Stencil for Smoke"),
        );

        let fill_stencil_smoke_pipe = {
            let pc_buffer = wal.create_buffer(
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                16 * 1024 * 20,
                Some("PC buffer for fill stencil smoke"),
            );
            let pc_bind_group = wal.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("PC buffer for fill stencil smoke"),
                layout: fill_stencil_smoke_pipe.dynamic_bind_group_layout.as_ref().unwrap(),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(pc_buffer.as_entire_buffer_binding()),
                }],
            });

            PipeWithPushConstants {
                pipe: fill_stencil_smoke_pipe,
                pc_buffer: Some(pc_buffer),
                pc_bg: Some(pc_bind_group),
                push_constants: vec![],
                pc_count: 0,
            }
        };

        let glossy_pipe = Wal::create_raster_pipe(
            wal,
            &[
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                StaticBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &dimages.mat_norm.view,
                    )]),
                },
                // StaticBindGroupDescription {
                //     binding: 2,
                //     visibility: ShaderStages::FRAGMENT,
                // binding_type: BindingType::
                //     resources: ResourceType::Static(
                //         BindingType::Sampler(SamplerBindingType::NonFiltering),
                //         sampler_to_binding_resources(&samplers.nearest_sampler),
                //     ),
                // },
                StaticBindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &dimages.depth.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    resources: sampler_to_binding_resources(&samplers.nearest_sampler),
                },
                StaticBindGroupDescription {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Sint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    resources: images_to_binding_resources(&iimages.world),
                },
                // StaticBindGroupDescription {
                //     binding: 5,
                //     visibility: ShaderStages::FRAGMENT,
                // binding_type: BindingType::
                //     resources: ResourceType::Static(
                //         BindingType::Sampler(SamplerBindingType::NonFiltering),
                //         sampler_to_binding_resources(&samplers.unnorm_nearest),
                //     ),
                // },
                StaticBindGroupDescription {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Sint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    resources: images_to_binding_resources(&iimages.block_palette),
                },
                // StaticBindGroupDescription {
                //     binding: 7,
                //     visibility: ShaderStages::FRAGMENT,
                // binding_type: BindingType::
                //     resources: ResourceType::Static(
                //         BindingType::Sampler(SamplerBindingType::NonFiltering),
                //         sampler_to_binding_resources(&samplers.unnorm_nearest),
                //     ),
                // },
                StaticBindGroupDescription {
                    binding: 6,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &iimages.material_palette.view,
                    )]),
                },
                // StaticBindGroupDescription {
                //     binding: 9,
                //     visibility: ShaderStages::FRAGMENT,
                // binding_type: BindingType::
                //     resources: ResourceType::Static(
                //         BindingType::Sampler(SamplerBindingType::NonFiltering),
                //         sampler_to_binding_resources(&samplers.nearest_sampler),
                //     ),
                // },
                StaticBindGroupDescription {
                    binding: 7,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    resources: images_to_binding_resources(&iimages.radiance_cache),
                },
                StaticBindGroupDescription {
                    binding: 8,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Sampler(SamplerBindingType::Filtering),
                    resources: sampler_to_binding_resources(&samplers.unnorm_linear),
                },
            ],
            &[],
            shaders::Shader::get_wgsl(Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_wgsl(Shader::GlossyFrag)),
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: FRAME_FORMAT,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            Some(DepthStencilState {
                format: TextureFormat::Stencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState {
                        compare: CompareFunction::Equal,
                        pass_op: StencilOperation::Keep,
                        fail_op: StencilOperation::Keep,
                        depth_fail_op: StencilOperation::Keep,
                    },
                    back: StencilFaceState::default(),
                    read_mask: 0x01,
                    write_mask: 0x00,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Glossy"),
        );

        // Like AO, Volumetrics are just blending into frame with their color.
        let smoke_pipe = Wal::create_raster_pipe(
            wal,
            &[
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                StaticBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &dimages.far_depth.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &dimages.near_depth.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    resources: images_to_binding_resources(&iimages.radiance_cache),
                },
                StaticBindGroupDescription {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &iimages.perlin_noise3d.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    binding_type: BindingType::Sampler(SamplerBindingType::Filtering),
                    resources: sampler_to_binding_resources(&samplers.linear_sampler_tiled),
                },
            ],
            &[],
            shaders::Shader::get_wgsl(Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_wgsl(Shader::SmokeFrag)),
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: FRAME_FORMAT,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            Some(DepthStencilState {
                format: TextureFormat::Stencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState {
                        compare: CompareFunction::Equal,
                        fail_op: StencilOperation::Keep,
                        depth_fail_op: StencilOperation::Keep,
                        pass_op: StencilOperation::Keep,
                    },
                    back: StencilFaceState::default(),
                    read_mask: 0x02,
                    write_mask: 0x00,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Smoke"),
        );

        // Tonemap Pipe is also responsible for putting frame image into swapchain. It does some... well, tonemapping as well as any simple other color filters. TODO: LUT?
        let tonemap_pipe = Wal::create_raster_pipe(
            wal,
            &[StaticBindGroupDescription {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                binding_type: BindingType::Texture {
                    // sampling unorm returns float
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                resources: Ring::from_vec(vec![BindingResource::TextureView(&dimages.frame.view)]),
            }],
            &[],
            shaders::Shader::get_wgsl(Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_wgsl(Shader::TonemapFrag)),
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: wal.swapchain_format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            None,
            None,
            Some("Tonemap"),
        );

        let radiance_pipe = Wal::create_compute_pipe(
            wal,
            &[
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                StaticBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Sint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    resources: images_to_binding_resources(&iimages.world),
                },
                StaticBindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Sint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    resources: images_to_binding_resources(&iimages.block_palette),
                },
                StaticBindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &iimages.material_palette.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    // reading old one to make a new one
                    resources: images_to_binding_resources_previous(&iimages.radiance_cache),
                },
                StaticBindGroupDescription {
                    binding: 5,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba16Float,
                        view_dimension: TextureViewDimension::D3,
                    },
                    // reading old one to make a new one
                    resources: images_to_binding_resources(&iimages.radiance_cache),
                },
                StaticBindGroupDescription {
                    binding: 6,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },

                    resources: buffers_to_binding_resources(&buffers.gpu_radiance_updates),
                },
                StaticBindGroupDescription {
                    binding: 7,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Sampler(SamplerBindingType::Filtering),
                    resources: sampler_to_binding_resources(&samplers.linear_sampler),
                },
            ],
            &[],
            &ShaderStageSource {
                stage: ShaderStages::COMPUTE,
                code: shaders::Shader::get_wgsl(Shader::RadianceComp),
            },
            Some("Radiance"),
        );

        let update_grass_pipe = Wal::create_compute_pipe(
            wal,
            &[
                // stuff that was previously in pc is now in ubo
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                StaticBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rg32Float,
                        view_dimension: TextureViewDimension::D2,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &iimages.grass_state.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &iimages.perlin_noise2d.view,
                    )]),
                },
                StaticBindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Sampler(SamplerBindingType::Filtering),
                    resources: sampler_to_binding_resources(&samplers.linear_sampler_tiled),
                },
            ],
            &[],
            &ShaderStageSource {
                stage: ShaderStages::COMPUTE,
                code: shaders::Shader::get_wgsl(Shader::UpdateGrassComp),
            },
            Some("Update Grass"),
        );

        let update_water_pipe = Wal::create_compute_pipe(
            wal,
            &[
                // stuff that was previously in pc is now in ubo
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                StaticBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba32Float,
                        view_dimension: TextureViewDimension::D2,
                    },
                    resources: Ring::from_vec(vec![BindingResource::TextureView(
                        &iimages.water_state.view,
                    )]),
                },
            ],
            &[],
            &ShaderStageSource {
                stage: ShaderStages::COMPUTE,
                code: shaders::Shader::get_wgsl(Shader::UpdateWaterComp),
            },
            Some("Water Updates"),
        );
        let gen_perlin2d_pipe = Wal::create_compute_pipe(
            wal,
            &[StaticBindGroupDescription {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                binding_type: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rg32Float,
                    view_dimension: TextureViewDimension::D2,
                },
                resources: Ring::from_vec(vec![BindingResource::TextureView(
                    &iimages.perlin_noise2d.view,
                )]),
            }],
            &[],
            &ShaderStageSource {
                stage: ShaderStages::COMPUTE,
                code: shaders::Shader::get_wgsl(Shader::Perlin2Comp),
            },
            Some("Gen Perlin 2D"),
        );
        let gen_perlin3d_pipe = Wal::create_compute_pipe(
            wal,
            &[StaticBindGroupDescription {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                binding_type: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rgba32Float,
                    view_dimension: TextureViewDimension::D3,
                },
                resources: Ring::from_vec(vec![BindingResource::TextureView(
                    &iimages.perlin_noise3d.view,
                )]),
            }],
            &[],
            &ShaderStageSource {
                stage: ShaderStages::COMPUTE,
                code: shaders::Shader::get_wgsl(Shader::Perlin3Comp),
            },
            Some("Gen Perlin 3D"),
        );
        let map_pipe = Wal::create_compute_pipe(
            wal,
            &[
                StaticBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Sint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    resources: images_to_binding_resources(&iimages.world),
                },
                StaticBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::StorageTexture {
                        // copy model voxels to block data in block palette
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::R32Sint,
                        view_dimension: TextureViewDimension::D3,
                    },
                    resources: images_to_binding_resources(&iimages.block_palette),
                },
            ],
            &[
                DynamicBindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                // aka push descriptor (group 1 bind 0) for model voxels
                // however, its really just another bind group
                DynamicBindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    binding_type: BindingType::Texture {
                        sample_type: TextureSampleType::Sint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                },
            ],
            &ShaderStageSource {
                stage: ShaderStages::COMPUTE,
                code: shaders::Shader::get_wgsl(Shader::MapComp),
            },
            Some("Mapping Models Voxels"),
        );

        // let pc_buffer_bind_group_layout =
        //     wal.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        //         label: Some("Layout of fake PC for foliages (they all share the same one)"),
        //         entries: &[BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: ShaderStages::VERTEX,
        //             ty: BindingType::Buffer {
        //                 ty: wgpu::BufferBindingType::Storage { read_only: true },
        //                 has_dynamic_offset: false,
        //                 min_binding_size: None,
        //             },
        //             count: None,
        //         }],
        //     });

        let raygen_foliage_pipes = foliage_descriptions
            .iter()
            .map(|foliage_description| {
                // we can actually create only one dynamic bind groups layout
                // but for more consistancy we will not
                let pipe = Wal::create_raster_pipe(
                    wal,
                    &[
                        StaticBindGroupDescription {
                            binding: 0,
                            visibility: ShaderStages::VERTEX,
                            binding_type: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            resources: buffers_to_binding_resources(&buffers.uniform),
                        },
                        StaticBindGroupDescription {
                            binding: 1,
                            visibility: ShaderStages::VERTEX,
                            binding_type: BindingType::Texture {
                                sample_type: TextureSampleType::Float { filterable: true },
                                view_dimension: TextureViewDimension::D2,
                                multisampled: false,
                            },
                            resources: Ring::from_vec(vec![BindingResource::TextureView(
                                &iimages.grass_state.view,
                            )]),
                        },
                        StaticBindGroupDescription {
                            binding: 2,
                            visibility: ShaderStages::VERTEX,
                            binding_type: BindingType::Sampler(SamplerBindingType::Filtering),

                            resources: sampler_to_binding_resources(&samplers.linear_sampler),
                        },
                    ],
                    &[DynamicBindGroupDescription {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        binding_type: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                    }],
                    foliage_description.code,
                    Some(shaders::Shader::get_wgsl(Shader::GrassFrag)),
                    &[], // no vertex buffers
                    PrimitiveTopology::TriangleList,
                    vec![Some(ColorTargetState {
                        format: MATNORM_FORMAT,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                    Some(DepthStencilState {
                        format: TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::Less,
                        stencil: StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    None,
                    Some("foliage"),
                );

                let pc_buffer = wal.create_buffer(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                    16 * 1024 * 20,
                    Some("PC buffer for foliage"),
                );
                let pc_bind_group = wal.device.create_bind_group(&BindGroupDescriptor {
                    label: Some("PC buffer for foliage"),
                    layout: pipe.dynamic_bind_group_layout.as_ref().unwrap(),
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(
                            pc_buffer.as_entire_buffer_binding(),
                        ),
                    }],
                });

                PipeWithPushConstants {
                    pipe,
                    push_constants: vec![],
                    pc_count: 0,
                    pc_buffer: Some(pc_buffer),
                    pc_bg: Some(pc_bind_group),
                }
            })
            .collect();

        AllPipes {
            lightmap_blocks_pipe,
            lightmap_models_pipe,
            raygen_blocks_pipe,
            raygen_models_pipe,
            raygen_particles_pipe,
            raygen_water_pipe,
            diffuse_pipe,
            ao_pipe,
            fill_stencil_glossy_pipe,
            fill_stencil_smoke_pipe,
            glossy_pipe,
            smoke_pipe,
            tonemap_pipe,
            radiance_pipe,
            map_pipe,
            update_grass_pipe,
            update_water_pipe,
            gen_perlin2d_pipe,
            gen_perlin3d_pipe,
            raygen_foliage_pipes,
            // overlay_pipe: todo!(),
        }
    }
}
