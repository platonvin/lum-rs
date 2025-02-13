use std::mem::offset_of;

use consts::*;
use internal_renderer::*;
use lumal::{descriptors::*, ring::Ring, LumalSettings, Renderer};
use vk::Sampler;
use vulkanalia::vk::{self, DeviceV1_0, Handle};
use RelativeDescriptorPos::*;

use crate::*;

// This file could be just a data
// it is setting up all the descriptors/layouts for pipes and pipes themeselves

impl InternalRenderer {
    #[cold]
    #[optimize(size)]
    pub unsafe fn create_all_pipes(
        lumal: &mut Renderer,
        lum_settings: &Settings,
        _lumal_settings: &LumalSettings,
        buffers: &AllBuffers,
        iimages: &AllIndependentImages,
        dimages: &AllSwapchainDependentImages,
        samplers: &AllSamplers,
        pipes: &mut AllPipes,
        foliage_descriptions: &[InternalMeshFoliageDesc],
    ) {
        // they are seperate because they are actually secondary layouts - used for descriptor_push
        // this is a big TODO: - get rid of descriptor_push
        setup_all_separate_descriptor_layouts(lumal, pipes);

        lumal::trace!();
        // anounce (count) all descriptors
        Self::do_smth_all_descriptors(
            &InternalRenderer::anounce_descriptor_setup_wrapper,
            lumal,
            buffers,
            iimages,
            dimages,
            samplers,
            pipes,
        );

        lumal::trace!();

        // do same for grass
        pipes.raygen_foliage_pipes.iter_mut().for_each(|foliage| {
            InternalRenderer::anounce_descriptor_setup_wrapper(
                lumal,
                &mut foliage.set_layout,
                &mut foliage.sets,
                &[
                    DescriptorInfo::make_new(
                        UNIFORM_BUFFER,
                        Current,
                        Some(buffers.uniform.clone()),
                        None,
                        vk::Sampler::null(),
                        UNDEFINED,
                        VERTEX | FRAGMENT,
                    ),
                    DescriptorInfo::make_new(
                        COMBINED_IMAGE_SAMPLER,
                        First, // TODO grass state alwyas first
                        None,
                        Some(iimages.grass_state.clone()),
                        samplers.linear_sampler,
                        GENERAL,
                        VERTEX | FRAGMENT,
                    ),
                ],
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                vk::DescriptorSetLayoutCreateFlags::empty(),
            );
        });

        lumal::trace!();
        // (actually) allocate space that is enough for all descriptors
        lumal.flush_descriptor_setup();

        lumal::trace!();
        // allocate each descriptor set
        Self::do_smth_all_descriptors(
            &InternalRenderer::acutally_setup_descriptor_wrapper,
            lumal,
            buffers,
            iimages,
            dimages,
            samplers,
            pipes,
        );
        lumal::trace!();

        // do same for grass
        pipes.raygen_foliage_pipes.iter_mut().for_each(|foliage| {
            InternalRenderer::acutally_setup_descriptor_wrapper(
                lumal,
                &mut foliage.set_layout,
                &mut foliage.sets,
                &[
                    DescriptorInfo::make_new(
                        UNIFORM_BUFFER,
                        Current,
                        Some(buffers.uniform.clone()),
                        None,
                        vk::Sampler::null(),
                        UNDEFINED,
                        VERTEX | FRAGMENT,
                    ),
                    DescriptorInfo::make_new(
                        COMBINED_IMAGE_SAMPLER,
                        First, // TODO grass state alwyas first
                        None,
                        Some(iimages.grass_state.clone()),
                        samplers.linear_sampler,
                        GENERAL,
                        VERTEX | FRAGMENT,
                    ),
                ],
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                vk::DescriptorSetLayoutCreateFlags::empty(),
            );
        });

        lumal::trace!();

        lumal.create_raster_pipe(
            &mut pipes.lightmap_blocks_pipe,
            None, // extra_dynamic_layout
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "lightmapBlocks.vert.spv".to_string(),
                }, // Fragment shader is not needed
            ],
            &[AttrFormOffs {
                binding: 0,
                format: vk::Format::R8G8B8_UINT,
                offset: offset_of!(PackedVoxelCircuit, pos),
            }],
            std::mem::size_of::<PackedVoxelCircuit>() as u32,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lum_settings.lightmap_extent,
            &[BlendAttachment::NoBlend],
            std::mem::size_of::<i16vec4>() as u32, // push size
            DepthTesting::DT_ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(), // no stencil
        );

        lumal::trace!();

        lumal.create_raster_pipe(
            &mut pipes.lightmap_models_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "lightmapModels.vert.spv".to_string(),
                }, // Fragment shader is not needed
            ],
            &[AttrFormOffs {
                binding: 0,
                format: vk::Format::R8G8B8_UINT,
                offset: offset_of!(PackedVoxelCircuit, pos),
            }],
            std::mem::size_of::<PackedVoxelCircuit>() as u32,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lum_settings.lightmap_extent,
            &[BlendAttachment::NoBlend],
            (std::mem::size_of::<quat>() + std::mem::size_of::<vec4>()) as u32, // push size
            DepthTesting::DT_ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(), // no stencil
        );

        lumal::trace!();

        lumal.create_descriptor_set_layout(
            &[ShortDescriptorInfo {
                descriptor_type: UNIFORM_BUFFER,
                stages: vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX,
            }],
            &mut pipes.temp_push_layout,
            vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
        );

        lumal.create_raster_pipe(
            &mut pipes.raygen_blocks_pipe,
            Some(pipes.temp_push_layout),
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "rayGenBlocks.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "rayGenBlocks.frag.spv".to_string(),
                },
            ],
            &[AttrFormOffs {
                binding: 0,
                format: vk::Format::R8G8B8_UINT, // TODO: automatic in macro
                offset: offset_of!(PackedVoxelCircuit, pos),
            }],
            std::mem::size_of::<PackedVoxelCircuit>() as u32,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.vulkan_data.swapchain_extent,
            &[BlendAttachment::NoBlend],
            12, // push size
            DepthTesting::DT_ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(), // no stencil
        );

        lumal::trace!();

        lumal.create_descriptor_set_layout(
            &[ShortDescriptorInfo {
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                stages: vk::ShaderStageFlags::FRAGMENT,
            }],
            &mut pipes.raygen_models_push_layout,
            vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
        );

        lumal.create_raster_pipe(
            &mut pipes.raygen_models_pipe,
            Some(pipes.raygen_models_push_layout),
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "rayGenModels.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "rayGenModels.frag.spv".to_string(),
                },
            ],
            &[AttrFormOffs {
                binding: 0,
                format: vk::Format::R8G8B8_UINT,
                offset: offset_of!(PackedVoxelCircuit, pos),
            }],
            std::mem::size_of::<PackedVoxelCircuit>() as u32,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.vulkan_data.swapchain_extent,
            &[BlendAttachment::NoBlend],
            (std::mem::size_of::<vec4>() * 3) as u32,
            DepthTesting::DT_ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(),
        );

        lumal::trace!();

        lumal.create_raster_pipe(
            &mut pipes.raygen_particles_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "rayGenParticles.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::GEOMETRY,
                    src: "rayGenParticles.geom.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "rayGenParticles.frag.spv".to_string(),
                },
            ],
            &[
                AttrFormOffs {
                    binding: 0,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: offset_of!(Particle, pos),
                },
                AttrFormOffs {
                    binding: 0,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: offset_of!(Particle, vel),
                },
                AttrFormOffs {
                    binding: 0,
                    format: vk::Format::R32_SFLOAT,
                    offset: offset_of!(Particle, life_time),
                },
                AttrFormOffs {
                    binding: 0,
                    format: vk::Format::R8_UINT,
                    offset: offset_of!(Particle, mat_id),
                },
            ],
            std::mem::size_of::<Particle>() as u32,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::POINT_LIST,
            lumal.vulkan_data.swapchain_extent,
            &[BlendAttachment::NoBlend],
            0,
            DepthTesting::DT_ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(),
        );

        lumal::trace!();

        lumal.create_raster_pipe(
            &mut pipes.raygen_water_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "water.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "water.frag.spv".to_string(),
                },
            ],
            &[],
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_STRIP,
            lumal.vulkan_data.swapchain_extent,
            &[BlendAttachment::NoBlend],
            (std::mem::size_of::<vec4>() + (std::mem::size_of::<i32>() * 2)) as u32,
            DepthTesting::DT_ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(),
        );

        lumal::trace!();

        for (i, foliage) in pipes.raygen_foliage_pipes.iter_mut().enumerate() {
            let desc = &foliage_descriptions[i];
            // let vs = desc.vertex_shader_file.as_str();
            lumal.create_raster_pipe(
                foliage,
                None,
                &[
                    ShaderStage {
                        stage: vk::ShaderStageFlags::VERTEX,
                        src: desc.vertex_shader_file.clone(),
                    },
                    ShaderStage {
                        stage: vk::ShaderStageFlags::FRAGMENT,
                        src: "grass.frag.spv".to_string(),
                    },
                ],
                &[AttrFormOffs {
                    binding: 0,
                    format: vk::Format::R8G8B8_UINT,
                    offset: offset_of!(PackedVoxelCircuit, pos),
                }],
                std::mem::size_of::<PackedVoxelCircuit>() as u32,
                vk::VertexInputRate::VERTEX,
                vk::PrimitiveTopology::TRIANGLE_LIST,
                lumal.vulkan_data.swapchain_extent,
                &[BlendAttachment::NoBlend],
                (std::mem::size_of::<vec4>() + std::mem::size_of::<vec4>()) as u32, // push size
                DepthTesting::DT_ReadWrite,
                // DepthTesting::DT_None,
                vk::CompareOp::LESS,
                vk::CullModeFlags::NONE,
                vk::StencilOpState::default(),
            );
        }

        lumal.create_raster_pipe(
            &mut pipes.diffuse_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "fullscreenTriag.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "diffuse.frag.spv".to_string(),
                },
            ],
            &[],
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.vulkan_data.swapchain_extent,
            &[BlendAttachment::NoBlend],
            (std::mem::size_of::<ivec4>()
                + (std::mem::size_of::<vec4>() * 4)
                + std::mem::size_of::<mat4>()) as u32,
            DepthTesting::DT_None,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(),
        );

        lumal::trace!();

        lumal.create_raster_pipe(
            &mut pipes.ao_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "fullscreenTriag.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "hbao.frag.spv".to_string(),
                },
            ],
            &[],
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.vulkan_data.swapchain_extent,
            &[BlendAttachment::BlendMix],
            0,
            DepthTesting::DT_None,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(),
        );

        lumal::trace!();

        lumal.create_raster_pipe(
            &mut pipes.fill_stencil_glossy_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "fullscreenTriag.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "fillStencilGlossy.frag.spv".to_string(),
                },
            ],
            &[], // Fullscreen pass, no attributes
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.vulkan_data.swapchain_extent,
            &[BlendAttachment::NoBlend],
            0, // No push constants
            DepthTesting::DT_None,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState {
                fail_op: vk::StencilOp::REPLACE,
                pass_op: vk::StencilOp::REPLACE,
                depth_fail_op: vk::StencilOp::REPLACE,
                compare_op: vk::CompareOp::ALWAYS,
                compare_mask: 0b00,
                write_mask: 0b01, // 01 for reflection
                reference: 0b01,
            },
        );

        lumal::trace!();

        lumal.create_raster_pipe(
            &mut pipes.fill_stencil_smoke_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "fillStencilSmoke.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "fillStencilSmoke.frag.spv".to_string(),
                },
            ],
            &[], // Push constants only
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.vulkan_data.swapchain_extent,
            &[
                BlendAttachment::BlendReplaceIfGreater,
                BlendAttachment::BlendReplaceIfLess,
            ],
            (std::mem::size_of::<vec3>() + std::mem::size_of::<i32>() + std::mem::size_of::<vec4>())
                as u32,
            DepthTesting::DT_Read,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState {
                fail_op: vk::StencilOp::KEEP,
                pass_op: vk::StencilOp::REPLACE,
                depth_fail_op: vk::StencilOp::KEEP,
                compare_op: vk::CompareOp::ALWAYS,
                compare_mask: 0b00,
                write_mask: 0b10, // 10 for smoke
                reference: 0b10,
            },
        );

        lumal::trace!();

        lumal.create_raster_pipe(
            &mut pipes.glossy_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "fullscreenTriag.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "glossy.frag.spv".to_string(),
                },
            ],
            &[], // Fullscreen pass, no attributes
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.vulkan_data.swapchain_extent,
            &[BlendAttachment::BlendMix],
            (std::mem::size_of::<vec4>() + std::mem::size_of::<vec4>()) as u32,
            DepthTesting::DT_None,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState {
                fail_op: vk::StencilOp::KEEP,
                pass_op: vk::StencilOp::KEEP,
                depth_fail_op: vk::StencilOp::KEEP,
                compare_op: vk::CompareOp::EQUAL,
                compare_mask: 0b01,
                write_mask: 0b00, // 01 for glossy
                reference: 0b01,
            },
        );

        lumal::trace!();

        lumal.create_raster_pipe(
            &mut pipes.smoke_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "fullscreenTriag.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "smoke.frag.spv".to_string(),
                },
            ],
            &[], // Fullscreen pass, no attributes
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.vulkan_data.swapchain_extent,
            &[BlendAttachment::BlendMix],
            0, // No push constants
            DepthTesting::DT_None,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState {
                fail_op: vk::StencilOp::KEEP,
                pass_op: vk::StencilOp::KEEP,
                depth_fail_op: vk::StencilOp::KEEP,
                compare_op: vk::CompareOp::EQUAL,
                compare_mask: 0b10,
                write_mask: 0b00, // 10 for smoke
                reference: 0b10,
            },
        );

        lumal::trace!();

        lumal.create_raster_pipe(
            &mut pipes.tonemap_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    src: "fullscreenTriag.vert.spv".to_string(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    src: "tonemap.frag.spv".to_string(),
                },
            ],
            &[], // Fullscreen pass, no attributes
            0,   // No vertex size
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.vulkan_data.swapchain_extent,
            &[BlendAttachment::NoBlend],
            0, // No push constants
            DepthTesting::DT_None,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(), // no stencil
        );

        // aint no way i port RmlUi to Rust

        lumal::trace!();

        // lumal.create_raster_pipe(
        //     &mut pipes.overlay_pipe,
        //     None,
        //     &[
        //         ShaderStage {
        //             stage: vk::ShaderStageFlags::VERTEX,
        //             src: "overlay.vert.spv".to_string(),
        //         },
        //         ShaderStage {
        //             stage: vk::ShaderStageFlags::FRAGMENT,
        //             src: "overlay.frag.spv".to_string(),
        //         },
        //     ],
        //     &[
        //         AttrFormOffs {
        //             binding: 0,
        //             format: vk::Format::R32G32_SFLOAT,
        //             offset: offset_of!(RmlVertex, position),
        //         },
        //         AttrFormOffs {
        //             binding: 0,
        //             format: vk::Format::R8G8B8A8_UNORM,
        //             offset: offset_of!(RmlVertex, colour),
        //         },
        //         AttrFormOffs {
        //             binding: 0,
        //             format: vk::Format::R32G32_SFLOAT,
        //             offset: offset_of!(RmlVertex, tex_coord),
        //         },
        //     ],
        //     std::mem::size_of::<RmlVertex>() as u32,
        //     vk::VertexInputRate::VERTEX,
        //     vk::PrimitiveTopology::TRIANGLE_LIST,
        //     lumal.vulkan_data.swapchain_extent,
        //     &[BlendAttachment::BlendMix],
        //     (std::mem::size_of::<vec4>() + std::mem::size_of::<mat4>()) as u32, // Push size
        //     DepthTesting::DT_None,
        //     vk::CompareOp::LESS,
        //     vk::CullModeFlags::NONE,
        //     vk::StencilOpState::default(), // no stencil
        // );

        lumal::trace!();
        // Compute pipelines
        lumal.create_compute_pipe(
            &mut pipes.radiance_pipe,
            None,
            "radiance.comp.spv".to_string(),
            (std::mem::size_of::<i32>() * 2) as u32,
            vk::PipelineCreateFlags::DISPATCH_BASE,
        );

        lumal::trace!();
        lumal.create_compute_pipe(
            &mut pipes.update_grass_pipe,
            None,
            "updateGrass.comp.spv".to_string(),
            (std::mem::size_of::<vec2>() * 2 + std::mem::size_of::<f32>()) as u32,
            vk::PipelineCreateFlags::empty(),
        );

        lumal::trace!();
        lumal.create_compute_pipe(
            &mut pipes.update_water_pipe,
            None,
            "updateWater.comp.spv".to_string(),
            (std::mem::size_of::<f32>() + std::mem::size_of::<vec2>() * 2) as u32,
            vk::PipelineCreateFlags::empty(),
        );

        lumal::trace!();
        lumal.create_compute_pipe(
            &mut pipes.gen_perlin2d_pipe,
            None,
            "perlin2.comp.spv".to_string(),
            0, // No push constants
            vk::PipelineCreateFlags::empty(),
        );

        lumal::trace!();
        lumal.create_compute_pipe(
            &mut pipes.gen_perlin3d_pipe,
            None,
            "perlin3.comp.spv".to_string(),
            0, // No push constants
            vk::PipelineCreateFlags::empty(),
        );

        lumal::trace!();
        lumal.create_compute_pipe(
            &mut pipes.map_pipe,
            Some(pipes.map_push_layout),
            "map.comp.spv".to_string(),
            (std::mem::size_of::<mat4>() + std::mem::size_of::<ivec4>()) as u32,
            vk::PipelineCreateFlags::empty(),
        );
    }

    #[cold]
    #[optimize(size)]
    pub unsafe fn destroy_all_pipes(lumal: &mut Renderer, mut pipes: AllPipes) {
        lumal.destroy_raster_pipe(pipes.lightmap_blocks_pipe);
        lumal.destroy_raster_pipe(pipes.lightmap_models_pipe);

        lumal.destroy_raster_pipe(pipes.raygen_blocks_pipe);
        lumal.destroy_raster_pipe(pipes.raygen_models_pipe);
        lumal
            .device
            .destroy_descriptor_set_layout(pipes.raygen_models_push_layout, None);
        lumal.destroy_raster_pipe(pipes.raygen_particles_pipe);
        lumal.destroy_raster_pipe(pipes.raygen_water_pipe);

        for foliage in pipes.raygen_foliage_pipes {
            lumal.destroy_raster_pipe(foliage);
        }

        lumal.destroy_raster_pipe(pipes.diffuse_pipe);
        lumal.destroy_raster_pipe(pipes.ao_pipe);
        lumal.destroy_raster_pipe(pipes.fill_stencil_glossy_pipe);
        lumal.destroy_raster_pipe(pipes.fill_stencil_smoke_pipe);
        lumal.destroy_raster_pipe(pipes.glossy_pipe);
        lumal.destroy_raster_pipe(pipes.smoke_pipe);
        lumal.destroy_raster_pipe(pipes.tonemap_pipe);
        // lumal.destroy_raster_pipe(pipes.overlay_pipe);
        lumal.device.destroy_descriptor_set_layout(pipes.overlay_pipe.set_layout, None);

        // lumal.destroy_compute_pipe(&mut pipes.raytrace_pipe);
        lumal.destroy_compute_pipe(&mut pipes.radiance_pipe);
        lumal.destroy_compute_pipe(&mut pipes.map_pipe);
        lumal.device.destroy_descriptor_set_layout(pipes.map_push_layout, None);
        lumal.destroy_compute_pipe(&mut pipes.update_grass_pipe);
        lumal.destroy_compute_pipe(&mut pipes.update_water_pipe);
        lumal.destroy_compute_pipe(&mut pipes.gen_perlin2d_pipe); // generate noise for grass
        lumal.destroy_compute_pipe(&mut pipes.gen_perlin3d_pipe); // generate noise for grass
    }

    #[cold]
    #[optimize(size)]
    fn do_smth_all_descriptors<Fun>(
        process: &Fun,
        lumal: &mut Renderer,
        buffers: &AllBuffers,
        iimages: &AllIndependentImages,
        dimages: &AllSwapchainDependentImages,
        samplers: &AllSamplers,
        pipes: &mut AllPipes,
    ) where
        Fun: for<'b> Fn(
            &'b mut Renderer,
            &'b mut vk::DescriptorSetLayout,
            &'b mut Ring<vk::DescriptorSet>,
            &'b [DescriptorInfo],
            vk::ShaderStageFlags,
            vk::DescriptorSetLayoutCreateFlags,
        ),
    {
        // We DO clone buffer, but its pointers anyways, so its fine
        // If anyone is smart enough to work with references in Rust, please improve it

        // Defer descriptor setup for lightmapBlocksPipe
        process(
            lumal,
            &mut pipes.lightmap_blocks_pipe.set_layout,
            &mut pipes.lightmap_blocks_pipe.sets,
            &[DescriptorInfo::make_new(
                UNIFORM_BUFFER,
                Current,
                Some(buffers.light_uniform.clone()),
                None,
                vk::Sampler::null(),
                GENERAL,
                VERTEX,
            )],
            VERTEX,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        // Defer descriptor setup for lightmapModelsPipe
        process(
            lumal,
            &mut pipes.lightmap_models_pipe.set_layout,
            &mut pipes.lightmap_models_pipe.sets,
            &[DescriptorInfo::make_new(
                UNIFORM_BUFFER,
                Current,
                Some(buffers.light_uniform.clone()),
                None,
                Sampler::null(),
                GENERAL,
                VERTEX,
            )],
            VERTEX,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        // Defer descriptor setup for radiancePipe
        process(
            lumal,
            &mut pipes.radiance_pipe.set_layout,
            &mut pipes.radiance_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(iimages.world.clone()),
                    samplers.unnorm_nearest,
                    GENERAL,
                    COMPUTE,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(iimages.origin_block_palette.clone()),
                    samplers.unnorm_nearest,
                    GENERAL,
                    COMPUTE,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(iimages.material_palette.clone()),
                    samplers.nearest_sampler,
                    GENERAL,
                    COMPUTE,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(iimages.radiance_cache.clone()),
                    samplers.unnorm_linear,
                    GENERAL,
                    COMPUTE,
                ),
                DescriptorInfo::make_new(
                    STORAGE_IMAGE,
                    First,
                    None,
                    Some(iimages.radiance_cache.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    COMPUTE,
                ),
                DescriptorInfo::make_new(
                    STORAGE_BUFFER,
                    First,
                    Some(buffers.gpu_radiance_updates.clone()),
                    None,
                    vk::Sampler::null(),
                    GENERAL,
                    COMPUTE,
                ),
            ],
            COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        // Defer descriptor setup for diffusePipe
        process(
            lumal,
            &mut pipes.diffuse_pipe.set_layout,
            &mut pipes.diffuse_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    UNIFORM_BUFFER,
                    Current,
                    Some(buffers.uniform.clone()),
                    None,
                    vk::Sampler::null(),
                    GENERAL,
                    VERTEX | FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(dimages.highres_mat_norm.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(dimages.highres_depth_stencil.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(iimages.material_palette.clone()),
                    samplers.nearest_sampler,
                    GENERAL,
                    VERTEX | FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(iimages.radiance_cache.clone()),
                    samplers.unnorm_linear,
                    GENERAL,
                    VERTEX | FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(iimages.lightmap.clone()),
                    samplers.shadow_sampler,
                    GENERAL,
                    VERTEX | FRAGMENT,
                ),
            ],
            FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.ao_pipe.set_layout,
            &mut pipes.ao_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    UNIFORM_BUFFER,
                    Current,
                    Some(buffers.uniform.clone()),
                    None,
                    vk::Sampler::null(),
                    UNDEFINED,
                    VERTEX | FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    UNIFORM_BUFFER,
                    Current,
                    Some(buffers.ao_lut_uniform.clone()),
                    None,
                    vk::Sampler::null(),
                    UNDEFINED,
                    VERTEX | FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(dimages.highres_mat_norm.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(dimages.highres_depth_stencil.clone()),
                    samplers.nearest_sampler,
                    GENERAL,
                    VERTEX | FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.tonemap_pipe.set_layout,
            &mut pipes.tonemap_pipe.sets,
            &[DescriptorInfo::make_new(
                INPUT_ATTACHMENT,
                First,
                None,
                Some(dimages.highres_frame.clone()),
                vk::Sampler::null(),
                GENERAL,
                FRAGMENT,
            )],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.fill_stencil_glossy_pipe.set_layout,
            &mut pipes.fill_stencil_glossy_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(dimages.highres_mat_norm.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(iimages.material_palette.clone()),
                    samplers.nearest_sampler,
                    GENERAL,
                    FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.fill_stencil_smoke_pipe.set_layout,
            &mut pipes.fill_stencil_smoke_pipe.sets,
            &[DescriptorInfo::make_new(
                UNIFORM_BUFFER,
                Current,
                Some(buffers.uniform.clone()),
                None,
                vk::Sampler::null(),
                UNDEFINED,
                VERTEX,
            )],
            vk::ShaderStageFlags::VERTEX,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.glossy_pipe.set_layout,
            &mut pipes.glossy_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    UNIFORM_BUFFER,
                    Current,
                    Some(buffers.uniform.clone()),
                    None,
                    vk::Sampler::null(),
                    UNDEFINED,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(dimages.highres_mat_norm.clone()),
                    samplers.nearest_sampler,
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(dimages.highres_depth_stencil.clone()),
                    samplers.nearest_sampler,
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(iimages.world.clone()),
                    samplers.unnorm_nearest,
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(iimages.origin_block_palette.clone()),
                    samplers.unnorm_nearest,
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(iimages.material_palette.clone()),
                    samplers.nearest_sampler,
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(iimages.radiance_cache.clone()),
                    samplers.unnorm_linear,
                    GENERAL,
                    FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.smoke_pipe.set_layout,
            &mut pipes.smoke_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    UNIFORM_BUFFER,
                    Current,
                    Some(buffers.uniform.clone()),
                    None,
                    vk::Sampler::null(),
                    UNDEFINED,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(dimages.far_depth.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(dimages.near_depth.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    STORAGE_IMAGE,
                    First,
                    None,
                    Some(iimages.radiance_cache.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(iimages.perlin_noise3d.clone()),
                    samplers.linear_sampler_tiled,
                    GENERAL,
                    FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.raygen_blocks_pipe.set_layout,
            &mut pipes.raygen_blocks_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    UNIFORM_BUFFER,
                    Current,
                    Some(buffers.uniform.clone()),
                    None,
                    vk::Sampler::null(),
                    UNDEFINED,
                    VERTEX | FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(iimages.origin_block_palette.clone()),
                    samplers.unnorm_nearest,
                    GENERAL,
                    VERTEX | FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.raygen_models_pipe.set_layout,
            &mut pipes.raygen_models_pipe.sets,
            &[DescriptorInfo::make_new(
                UNIFORM_BUFFER,
                Current,
                Some(buffers.uniform.clone()),
                None,
                vk::Sampler::null(),
                UNDEFINED,
                VERTEX | FRAGMENT,
            )],
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.raygen_particles_pipe.set_layout,
            &mut pipes.raygen_particles_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    UNIFORM_BUFFER,
                    Current,
                    Some(buffers.uniform.clone()),
                    None,
                    vk::Sampler::null(),
                    UNDEFINED,
                    VERTEX | FRAGMENT | GEOMETRY,
                ),
                DescriptorInfo::make_new(
                    STORAGE_IMAGE,
                    First,
                    None,
                    Some(iimages.world.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    VERTEX | FRAGMENT | GEOMETRY,
                ),
                DescriptorInfo::make_new(
                    STORAGE_IMAGE,
                    Current,
                    None,
                    Some(iimages.origin_block_palette.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    VERTEX | FRAGMENT | GEOMETRY,
                ),
            ],
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::GEOMETRY,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.update_grass_pipe.set_layout,
            &mut pipes.update_grass_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    STORAGE_IMAGE,
                    First,
                    None,
                    Some(iimages.grass_state.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    VERTEX | FRAGMENT | COMPUTE,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(iimages.perlin_noise2d.clone()),
                    samplers.linear_sampler_tiled,
                    GENERAL,
                    VERTEX | FRAGMENT | COMPUTE,
                ),
            ],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.update_water_pipe.set_layout,
            &mut pipes.update_water_pipe.sets,
            &[DescriptorInfo::make_new(
                STORAGE_IMAGE,
                First,
                None,
                Some(iimages.water_state.clone()),
                vk::Sampler::null(),
                GENERAL,
                VERTEX | FRAGMENT | COMPUTE,
            )],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.raygen_water_pipe.set_layout,
            &mut pipes.raygen_water_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    UNIFORM_BUFFER,
                    Current,
                    Some(buffers.uniform.clone()),
                    None,
                    vk::Sampler::null(),
                    UNDEFINED,
                    VERTEX | FRAGMENT | GEOMETRY,
                ),
                DescriptorInfo::make_new(
                    COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(iimages.water_state.clone()),
                    samplers.linear_sampler_tiled,
                    GENERAL,
                    VERTEX | FRAGMENT | GEOMETRY,
                ),
            ],
            vk::ShaderStageFlags::VERTEX,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.gen_perlin2d_pipe.set_layout,
            &mut pipes.gen_perlin2d_pipe.sets,
            &[DescriptorInfo::make_new(
                STORAGE_IMAGE,
                First,
                None,
                Some(iimages.perlin_noise2d.clone()),
                vk::Sampler::null(),
                GENERAL,
                VERTEX | FRAGMENT | COMPUTE,
            )],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.gen_perlin3d_pipe.set_layout,
            &mut pipes.gen_perlin3d_pipe.sets,
            &[DescriptorInfo::make_new(
                STORAGE_IMAGE,
                First,
                None,
                Some(iimages.perlin_noise3d.clone()),
                vk::Sampler::null(),
                GENERAL,
                VERTEX | FRAGMENT | COMPUTE, // TODO: remove all unnecessary stages
            )],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );

        process(
            lumal,
            &mut pipes.map_pipe.set_layout,
            &mut pipes.map_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    STORAGE_IMAGE,
                    First,
                    None,
                    Some(iimages.world.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    COMPUTE,
                ),
                DescriptorInfo::make_new(
                    STORAGE_IMAGE,
                    Current,
                    None,
                    Some(iimages.origin_block_palette.clone()),
                    vk::Sampler::null(),
                    GENERAL,
                    COMPUTE,
                ),
            ],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
        );
    }

    // Sorry, i dont have enough iq to understand lifetimes
    #[cold]
    #[optimize(size)]
    fn anounce_descriptor_setup_wrapper(
        lumal: &mut Renderer,
        dset_layout: &mut vk::DescriptorSetLayout,
        descriptor_sets: &mut Ring<vk::DescriptorSet>,
        descriptions: &[DescriptorInfo],
        default_stages: vk::ShaderStageFlags,
        create_flags: vk::DescriptorSetLayoutCreateFlags,
    ) {
        lumal.anounce_descriptor_setup(
            dset_layout,
            descriptor_sets,
            descriptions,
            default_stages,
            create_flags,
        );
    }

    #[cold]
    #[optimize(size)]
    fn acutally_setup_descriptor_wrapper(
        lumal: &mut Renderer,
        dset_layout: &mut vk::DescriptorSetLayout,
        descriptor_sets: &mut Ring<vk::DescriptorSet>,
        descriptions: &[DescriptorInfo],
        default_stages: vk::ShaderStageFlags,
        create_flags: vk::DescriptorSetLayoutCreateFlags,
    ) {
        lumal.acutally_setup_descriptor(
            dset_layout,
            descriptor_sets,
            descriptions,
            default_stages,
            create_flags,
        );
    }
}

#[cold]
#[optimize(size)]
fn setup_all_separate_descriptor_layouts(lumal: &mut Renderer, pipes: &mut AllPipes) {
    lumal.create_descriptor_set_layout(
        &[ShortDescriptorInfo {
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            stages: vk::ShaderStageFlags::FRAGMENT,
        }],
        &mut pipes.overlay_pipe.set_layout,
        vk::DescriptorSetLayoutCreateFlags::empty(),
    );
    lumal.create_descriptor_set_layout(
        &[ShortDescriptorInfo {
            descriptor_type: STORAGE_IMAGE,
            stages: vk::ShaderStageFlags::COMPUTE,
        }],
        &mut pipes.map_push_layout,
        vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
    );
    lumal.create_descriptor_set_layout(
        &[ShortDescriptorInfo {
            descriptor_type: COMBINED_IMAGE_SAMPLER,
            stages: vk::ShaderStageFlags::FRAGMENT,
        }],
        &mut pipes.raygen_models_push_layout,
        vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
    );
    lumal.create_descriptor_set_layout(
        &[ShortDescriptorInfo {
            descriptor_type: UNIFORM_BUFFER,
            stages: vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX,
        }],
        &mut pipes.temp_push_layout,
        vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
    );
}
