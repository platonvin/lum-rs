use crate::{
    internal_renderer::{
        render_wgpu::{
            wal::{
                AttrFormOffs, BindGroupDescription, DescriptorInfo, Image, PushConstantDescription,
                ShaderStage, Wal,
            },
            AllBuffers, AllIndependentImages, AllPipes, AllSamplers, AllSwapchainDependentImages,
            InternalRendererWebGPU,
        },
        Settings,
    },
    types::*,
};
use lumal::ring::Ring;
use std::{mem::offset_of, num::NonZero};
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
    for ring in rings_of_buffers.iter().skip(1) {
        if ring.len() != first_length {
            eprintln!("Error: Input rings have inconsistent lengths.");
            return None;
        }
    }
    let num_rings = rings_of_buffers.len();
    let output_ring_length = first_length;
    let mut output_data: Vec<Vec<BindingResource<'a>>> = Vec::with_capacity(output_ring_length);
    for i in 0..output_ring_length {
        let mut inner_vec: Vec<BindingResource<'a>> = Vec::with_capacity(num_rings);
        for ring in rings_of_buffers.iter() {
            inner_vec.push(BindingResource::Buffer(BufferBinding {
                buffer: ring.get(i),
                offset: 0,
                size: None,
            }));
        }
        output_data.push(inner_vec);
    }
    Some(Ring::from_vec(output_data))
}
pub fn rings_of_texture_views_to_ring_of_texture_view_bindings<'a>(
    rings_of_texture_views: &[Ring<&'a wgpu::TextureView>],
) -> Option<Ring<Vec<BindingResource<'a>>>> {
    if rings_of_texture_views.is_empty() {
        return None;
    }
    let first_length = rings_of_texture_views[0].len();
    for ring in rings_of_texture_views.iter().skip(1) {
        if ring.len() != first_length {
            eprintln!("Error: Input rings have inconsistent lengths.");
            return None;
        }
    }
    let num_rings = rings_of_texture_views.len();
    let output_ring_length = first_length;
    let mut output_data: Vec<Vec<BindingResource<'a>>> = Vec::with_capacity(output_ring_length);
    for i in 0..output_ring_length {
        let mut inner_vec: Vec<BindingResource<'a>> = Vec::with_capacity(num_rings);
        for ring in rings_of_texture_views.iter() {
            inner_vec.push(BindingResource::TextureView(ring.get(i)));
        }
        output_data.push(inner_vec);
    }
    Some(Ring::from_vec(output_data))
}
pub struct PackedVoxelCircuit {
    pub pos: u8vec4,
}
impl<'window> InternalRendererWebGPU<'window> {
    pub unsafe fn create_all_pipes(
        wal: &Wal,
        lum_settings: &Settings,
        buffers: &AllBuffers,
        iimages: &AllIndependentImages,
        dimages: &AllSwapchainDependentImages,
        samplers: &AllSamplers,
        foliage_descriptions: &[InternalMeshFoliageDesc],
    ) -> AllPipes {
        let lightmap_blocks_pipe = Wal::create_raster_pipe(
            &wal,
            &[BindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
                resources: buffers_to_binding_resources(&buffers.light_uniform),
            }],
            &[ShaderStage {
                stage: ShaderStages::VERTEX,
                code: shaders::get_wgsl("lightmapBlocks.vert").unwrap(),
            }],
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
            Some(PushConstantDescription {
                size: todo!(),
                max_count: todo!(),
            }),
            Some("lightmap blocks pipe"),
        );
        let lightmap_models_pipe = Wal::create_raster_pipe(
            &wal,
            &[BindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
                resources: buffers_to_binding_resources(&buffers.light_uniform),
            }],
            &[ShaderStage {
                stage: ShaderStages::VERTEX,
                code: shaders::get_wgsl("lightmap_models.vert").unwrap(),
            }],
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
            Some("Lightmap Models Pipe"),
        );
        let raygen_blocks_pipe = Wal::create_raster_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.origin_block_palette),
                },
                BindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.unnorm_nearest.as_ref().unwrap(),
                    ),
                },
            ],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("raygen_blocks.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("raygen_blocks.frag").unwrap(),
                },
            ],
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
                format: TextureFormat::R8Unorm,
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
            Some("Raygen Blocks Pipe"),
        );
        let raygen_models_pipe = Wal::create_raster_pipe(
            &wal,
            &[BindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
                resources: buffers_to_binding_resources(&buffers.uniform),
            }],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("raygen_models.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("raygen_models.frag").unwrap(),
                },
            ],
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
                format: TextureFormat::R8Unorm,
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
            Some("Raygen Models Pipe"),
        );
        let raygen_particles_pipe = Wal::create_raster_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::R32Uint,
                        view_dimension: TextureViewDimension::D3,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.world),
                },
                BindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::R32Uint,
                        view_dimension: TextureViewDimension::D3,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.origin_block_palette),
                },
            ],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("raygen_particles.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("raygen_particles.frag").unwrap(),
                },
            ],
            &[VertexBufferLayout {
                array_stride: size_of::<Particle>() as u64,
                step_mode: VertexStepMode::Vertex,
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
            PrimitiveTopology::PointList,
            vec![Some(ColorTargetState {
                format: TextureFormat::R8Unorm,
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
            Some("Raygen Particles Pipe"),
        );
        let raygen_water_pipe = Wal::create_raster_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.water_state),
                },
                BindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.linear_sampler_tiled.as_ref().unwrap(),
                    ),
                },
            ],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("water.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("water.frag").unwrap(),
                },
            ],
            &[],
            PrimitiveTopology::TriangleStrip,
            vec![Some(ColorTargetState {
                format: TextureFormat::R8Unorm,
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
            Some("Raygen Water Pipe"),
        );
        let raygen_foliage_pipe_example = Wal::create_raster_pipe(
            &wal,
            &[ /* Bindings: The Vulkan code doesn't show a 'process' call specifically for foliage pipes inside the loop.
          Assuming it might reuse bindings from raygen_blocks/models or have its own setup elsewhere.
          Leaving empty for now, requires clarification on descriptor setup for foliage. */
    ],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("foliage").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("grass.frag").unwrap(),
                },
            ],
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
                format: TextureFormat::R8Unorm,
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
            Some("Raygen Foliage Pipe"),
        );
        let diffuse_pipe = Wal::create_raster_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&dimages.highres_mat_norm),
                },
                BindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&dimages.highres_depth_stencil),
                },
                BindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.material_palette),
                },
                BindGroupDescription {
                    binding: 4,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.nearest_sampler.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 5,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.radiance_cache),
                },
                BindGroupDescription {
                    binding: 6,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.unnorm_linear.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 7,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.lightmap),
                },
                BindGroupDescription {
                    binding: 8,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Comparison),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.shadow_sampler.as_ref().unwrap(),
                    ),
                },
            ],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("fullscreen_triag.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("diffuse.frag").unwrap(),
                },
            ],
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: TextureFormat::R8Unorm,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            None,
            None,
            Some("Diffuse Pipe"),
        );
        let ao_pipe = Wal::create_raster_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                    resources: buffers_to_binding_resources(&buffers.ao_lut_uniform),
                },
                BindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&dimages.highres_mat_norm),
                },
                BindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&dimages.highres_depth_stencil),
                },
                BindGroupDescription {
                    binding: 4,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.nearest_sampler.as_ref().unwrap(),
                    ),
                },
            ],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("fullscreen_triag.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("hbao.frag").unwrap(),
                },
            ],
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: TextureFormat::R8Unorm,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            None,
            None,
            Some("Ambient Occlusion Pipe"),
        );
        let fill_stencil_glossy_pipe = Wal::create_raster_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&dimages.highres_mat_norm),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.material_palette),
                },
                BindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.nearest_sampler.as_ref().unwrap(),
                    ),
                },
            ],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("fullscreen_triag.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("fill_stencil_glossy.frag").unwrap(),
                },
            ],
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: TextureFormat::R8Unorm,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            Some(DepthStencilState {
                format: TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState {
                        compare: CompareFunction::Always,
                        fail_op: StencilOperation::Replace,
                        depth_fail_op: StencilOperation::Replace,
                        pass_op: StencilOperation::Replace,
                    },
                    back: StencilFaceState::default(),
                    read_mask: 0x00,
                    write_mask: 0x01,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Fill Stencil+Glossy Pipe"),
        );
        let fill_stencil_smoke_pipe = Wal::create_raster_pipe(
            &wal,
            &[BindGroupDescription {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
                resources: buffers_to_binding_resources(&buffers.uniform),
            }],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("fill_stencil_smoke.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("fill_stencil_smoke.frag").unwrap(),
                },
            ],
            &[],
            PrimitiveTopology::TriangleList,
            vec![
                Some(ColorTargetState {
                    format: TextureFormat::Rg16Float,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::Zero,
                            operation: BlendOperation::Max,
                        },
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                }),
                Some(ColorTargetState {
                    format: TextureFormat::R16Float,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::Zero,
                            operation: BlendOperation::Min,
                        },
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                }),
            ],
            Some(DepthStencilState {
                format: TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Less,
                stencil: StencilState {
                    front: StencilFaceState {
                        compare: CompareFunction::Always,
                        fail_op: StencilOperation::Keep,
                        depth_fail_op: StencilOperation::Keep,
                        pass_op: StencilOperation::Replace,
                    },
                    back: StencilFaceState::default(),
                    read_mask: 0x00,
                    write_mask: 0x02,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Fill Stencil for Smoke Pipe"),
        );
        let glossy_pipe = Wal::create_raster_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&dimages.highres_mat_norm),
                },
                BindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.nearest_sampler.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&dimages.highres_depth_stencil),
                },
                BindGroupDescription {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.nearest_sampler.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.world),
                },
                BindGroupDescription {
                    binding: 6,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.unnorm_nearest.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 7,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.origin_block_palette),
                },
                BindGroupDescription {
                    binding: 8,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.unnorm_nearest.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 9,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.material_palette),
                },
                BindGroupDescription {
                    binding: 10,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.nearest_sampler.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 11,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.radiance_cache),
                },
                BindGroupDescription {
                    binding: 12,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.unnorm_linear.as_ref().unwrap(),
                    ),
                },
            ],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("fullscreen_triag.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("glossy.frag").unwrap(),
                },
            ],
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: TextureFormat::R8Unorm,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            Some(DepthStencilState {
                format: TextureFormat::Depth24PlusStencil8,
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
                    read_mask: 0x01,
                    write_mask: 0x00,
                },
                bias: wgpu::DepthBiasState::default(),
            }),
            None,
            Some("Glossy Pipe"),
        );
        let smoke_pipe = Wal::create_raster_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                    resources: buffers_to_binding_resources(&buffers.uniform),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&dimages.far_depth),
                },
                BindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&dimages.near_depth),
                },
                BindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::Rgba16Float,
                        view_dimension: TextureViewDimension::D3,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.radiance_cache),
                },
                BindGroupDescription {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.perlin_noise3d),
                },
                BindGroupDescription {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.linear_sampler_tiled.as_ref().unwrap(),
                    ),
                },
            ],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("fullscreen_triag.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("smoke.frag").unwrap(),
                },
            ],
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: TextureFormat::R8Unorm,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
            Some(DepthStencilState {
                format: TextureFormat::Depth24PlusStencil8,
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
            Some("Smoke Pipe"),
        );
        let tonemap_pipe = Wal::create_raster_pipe(
            &wal,
            &[BindGroupDescription {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
                resources: images_to_binding_resources(&dimages.highres_frame),
            }],
            &[
                ShaderStage {
                    stage: ShaderStages::VERTEX,
                    code: shaders::get_wgsl("fullscreen_triag.vert").unwrap(),
                },
                ShaderStage {
                    stage: ShaderStages::FRAGMENT,
                    code: shaders::get_wgsl("tonemap.frag").unwrap(),
                },
            ],
            &[],
            PrimitiveTopology::TriangleList,
            vec![Some(ColorTargetState {
                format: TextureFormat::R8Unorm,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            None,
            None,
            Some("Tonemap Pipe"),
        );
        let radiance_pipe = Wal::create_compute_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.world),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.unnorm_nearest.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.origin_block_palette),
                },
                BindGroupDescription {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.unnorm_nearest.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.material_palette),
                },
                BindGroupDescription {
                    binding: 5,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.nearest_sampler.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 6,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.radiance_cache),
                },
                BindGroupDescription {
                    binding: 7,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.unnorm_linear.as_ref().unwrap(),
                    ),
                },
                BindGroupDescription {
                    binding: 8,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::Rgba16Float,
                        view_dimension: TextureViewDimension::D3,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.radiance_cache),
                },
                BindGroupDescription {
                    binding: 9,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                    resources: buffers_to_binding_resources(&buffers.gpu_radiance_updates),
                },
            ],
            &ShaderStage {
                stage: ShaderStages::COMPUTE,
                code: shaders::get_wgsl("radiance.comp").unwrap(),
            },
            Some("Radiance Pipe"),
        );
        let update_grass_pipe = Wal::create_compute_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::Rg16Float,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.grass_state),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.perlin_noise2d),
                },
                BindGroupDescription {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                    resources: sampler_to_binding_resources(
                        samplers.linear_sampler_tiled.as_ref().unwrap(),
                    ),
                },
            ],
            &ShaderStage {
                stage: ShaderStages::COMPUTE,
                code: shaders::get_wgsl("update_grass.comp").unwrap(),
            },
            Some("Update Grass Pipe"),
        );
        let update_water_pipe = Wal::create_compute_pipe(
            &wal,
            &[BindGroupDescription {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::ReadWrite,
                    format: TextureFormat::Rg16Float,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
                resources: images_to_binding_resources(&iimages.water_state),
            }],
            &ShaderStage {
                stage: ShaderStages::COMPUTE,
                code: shaders::get_wgsl("update_water.comp").unwrap(),
            },
            Some("Water Updates Pipe"),
        );
        let gen_perlin2d_pipe = Wal::create_compute_pipe(
            &wal,
            &[BindGroupDescription {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rg8Unorm,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
                resources: images_to_binding_resources(&iimages.perlin_noise2d),
            }],
            &ShaderStage {
                stage: ShaderStages::COMPUTE,
                code: shaders::get_wgsl("perlin2.comp").unwrap(),
            },
            Some("Gen Perlin 2D Pipe"),
        );
        let gen_perlin3d_pipe = Wal::create_compute_pipe(
            &wal,
            &[BindGroupDescription {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::R8Unorm,
                    view_dimension: TextureViewDimension::D3,
                },
                count: None,
                resources: images_to_binding_resources(&iimages.perlin_noise3d),
            }],
            &ShaderStage {
                stage: ShaderStages::COMPUTE,
                code: shaders::get_wgsl("perlin3.comp").unwrap(),
            },
            Some("Gen Perlin 3D Pipe"),
        );
        let map_pipe = Wal::create_compute_pipe(
            &wal,
            &[
                BindGroupDescription {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::R32Uint,
                        view_dimension: TextureViewDimension::D3,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.world),
                },
                BindGroupDescription {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::R32Uint,
                        view_dimension: TextureViewDimension::D3,
                    },
                    count: None,
                    resources: images_to_binding_resources(&iimages.origin_block_palette),
                },
            ],
            &ShaderStage {
                stage: ShaderStages::COMPUTE,
                code: shaders::get_wgsl("map.comp").unwrap(),
            },
            Some("Mapping Models Voxels Pipe"),
        );
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
            raygen_foliage_pipes: todo!(),
            overlay_pipe: todo!(),
        }
    }
}
