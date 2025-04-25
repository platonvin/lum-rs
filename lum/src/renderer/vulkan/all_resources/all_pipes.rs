use std::mem::offset_of;

use crate::renderer::types::*;
use crate::{
    renderer::vulkan,
    renderer::vulkan::types::*,
    renderer::{
        vulkan::{
            AllBuffers, AllIndependentImages, AllPipes, AllSamplers, AllSwapchainDependentImages,
            InternalRendererVulkan,
        },
        Settings,
    },
};

// use internal_renderer::{InternalRendererVulkan, *};
use lumal::{
    descriptors::RelativeDescriptorPos::{Current, First},
    vk::Sampler,
};
use lumal::{descriptors::*, vk};
use lumal::{ring::Ring, LumalSettings, Renderer};
// This file could be just a data
// it is setting up all the descriptors/layouts for pipes and pipes themeselves

impl InternalRendererVulkan {
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
        foliage_descriptions: &[vulkan::render::MeshFoliageDescription],
    ) {
        // they are seperate because they are actually secondary layouts - used for descriptor_push
        // this is a big TODO: - get rid of descriptor_push
        setup_all_separate_descriptor_layouts(lumal, pipes);

        // anounce (count) all descriptors
        Self::do_smth_all_descriptors(
            &InternalRendererVulkan::anounce_descriptor_setup_wrapper,
            lumal,
            buffers,
            iimages,
            dimages,
            samplers,
            pipes,
        );

        // do same for grass
        pipes.raygen_foliage_pipes.iter_mut().for_each(|foliage| {
            InternalRendererVulkan::anounce_descriptor_setup_wrapper(
                lumal,
                &mut foliage.set_layout,
                &mut foliage.sets,
                &[
                    DescriptorInfo::make_new(
                        vk::DescriptorType::UNIFORM_BUFFER,
                        Current,
                        Some(&buffers.uniform),
                        None,
                        vk::Sampler::null(),
                        vk::ImageLayout::UNDEFINED,
                        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    ),
                    DescriptorInfo::make_new(
                        vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        First, // TODO grass state alwyas first
                        None,
                        Some(&iimages.grass_state),
                        samplers.linear_sampler,
                        vk::ImageLayout::GENERAL,
                        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    ),
                ],
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                vk::DescriptorSetLayoutCreateFlags::empty(),
                // #[cfg(feature = "debug_validation_names")]
                Some("Foliage Descriptor Set Layout"),
            );
        });

        // (actually) allocate space that is enough for all descriptors
        lumal.flush_descriptor_setup();

        // allocate each descriptor set
        Self::do_smth_all_descriptors(
            &InternalRendererVulkan::acutally_setup_descriptor_wrapper,
            lumal,
            buffers,
            iimages,
            dimages,
            samplers,
            pipes,
        );

        // do same for grass
        pipes.raygen_foliage_pipes.iter_mut().for_each(|foliage| {
            InternalRendererVulkan::acutally_setup_descriptor_wrapper(
                lumal,
                &mut foliage.set_layout,
                &mut foliage.sets,
                &[
                    DescriptorInfo::make_new(
                        vk::DescriptorType::UNIFORM_BUFFER,
                        Current,
                        Some(&buffers.uniform),
                        None,
                        vk::Sampler::null(),
                        vk::ImageLayout::UNDEFINED,
                        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    ),
                    DescriptorInfo::make_new(
                        vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        First, // TODO grass state alwyas first
                        None,
                        Some(&iimages.grass_state),
                        samplers.linear_sampler,
                        vk::ImageLayout::GENERAL,
                        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    ),
                ],
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                vk::DescriptorSetLayoutCreateFlags::empty(),
                // #[cfg(feature = "debug_validation_names")]
                Some("Fill Stencil for Smoke Descriptor Set Layout"),
            );
        });

        lumal.create_raster_pipe(
            &mut pipes.lightmap_blocks_pipe,
            None, // extra_dynamic_layout
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("lightmapBlocks.vert.spv").unwrap(),
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
            Some("Lightmap Blocks"),
        );

        lumal.create_raster_pipe(
            &mut pipes.lightmap_models_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("lightmapModels.vert.spv").unwrap(),
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
            Some("Lightmap Models"),
        );

        lumal.create_raster_pipe(
            &mut pipes.raygen_blocks_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("rayGenBlocks.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("rayGenBlocks.frag.spv").unwrap(),
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
            Some("Raygen Blocks"),
        );

        lumal.create_raster_pipe(
            &mut pipes.raygen_models_pipe,
            Some(pipes.raygen_models_push_layout),
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("rayGenModels.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("rayGenModels.frag.spv").unwrap(),
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
            Some("Raygen Models"),
        );

        lumal.create_raster_pipe(
            &mut pipes.raygen_particles_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("rayGenParticles.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::GEOMETRY,
                    spirv_code: shaders::get_shader("rayGenParticles.geom.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("rayGenParticles.frag.spv").unwrap(),
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
            Some("Raygen Particles"),
        );

        lumal.create_raster_pipe(
            &mut pipes.raygen_water_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("water.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("water.frag.spv").unwrap(),
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
            Some("Raygen Water"),
        );

        for (i, foliage) in pipes.raygen_foliage_pipes.iter_mut().enumerate() {
            let desc = &foliage_descriptions[i];
            // let vs = desc.vk::ShaderStageFlags::vertex_shader_file.as_str();
            lumal.create_raster_pipe(
                foliage,
                None,
                &[
                    ShaderStage {
                        stage: vk::ShaderStageFlags::VERTEX,
                        spirv_code: &desc.spirv_code,
                    },
                    ShaderStage {
                        stage: vk::ShaderStageFlags::FRAGMENT,
                        spirv_code: shaders::get_shader("grass.frag.spv").unwrap(),
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
                Some("Raygen Foliage"),
            );
        }

        lumal.create_raster_pipe(
            &mut pipes.diffuse_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("fullscreenTriag.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("diffuse.frag.spv").unwrap(),
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
            Some("Diffuse"),
        );

        lumal.create_raster_pipe(
            &mut pipes.ao_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("fullscreenTriag.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("hbao.frag.spv").unwrap(),
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
            Some("Ambient Occlusion"),
        );

        lumal.create_raster_pipe(
            &mut pipes.fill_stencil_glossy_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("fullscreenTriag.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("fillStencilGlossy.frag.spv").unwrap(),
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
            Some("Fill Stencil+Glossy"),
        );

        lumal.create_raster_pipe(
            &mut pipes.fill_stencil_smoke_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("fillStencilSmoke.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("fillStencilSmoke.frag.spv").unwrap(),
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
            Some("Fill Stencil for Smoke"),
        );

        lumal.create_raster_pipe(
            &mut pipes.glossy_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("fullscreenTriag.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("glossy.frag.spv").unwrap(),
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
            Some("Glossy"),
        );

        lumal.create_raster_pipe(
            &mut pipes.smoke_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("fullscreenTriag.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("smoke.frag.spv").unwrap(),
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
            Some("Smoke"),
        );

        lumal.create_raster_pipe(
            &mut pipes.tonemap_pipe,
            None,
            &[
                ShaderStage {
                    stage: vk::ShaderStageFlags::VERTEX,
                    spirv_code: shaders::get_shader("fullscreenTriag.vert.spv").unwrap(),
                },
                ShaderStage {
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    spirv_code: shaders::get_shader("tonemap.frag.spv").unwrap(),
                },
            ],
            &[], // Fullscreen pass, no attributes
            0,   // No vk::ShaderStageFlags::vertex size
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
            Some("Tonemap"),
        );

        // aint no way i port RmlUi to Rust

        // lumal.create_raster_pipe(
        //     &mut pipes.overlay_pipe,
        //     None,
        //     &[
        //         ShaderStage {
        //             stage: vk::ShaderStageFlags::VERTEX,
        //             src: "overlay.vert.spv",
        //         },
        //         ShaderStage {
        //             stage: vk::ShaderStageFlags::FRAGMENT,
        //             src: "overlay.frag.spv",
        //         },
        //     ],
        //     &[
        //         AttrFormOffs {
        //             binding: 0,
        //             format: vk::Format::R32G32_SFLOAT,
        //             offset: offset_of!(Rmlvk::ShaderStageFlags::Vertex, position),
        //         },
        //         AttrFormOffs {
        //             binding: 0,
        //             format: vk::Format::R8G8B8A8_UNORM,
        //             offset: offset_of!(Rmlvk::ShaderStageFlags::Vertex, colour),
        //         },
        //         AttrFormOffs {
        //             binding: 0,
        //             format: vk::Format::R32G32_SFLOAT,
        //             offset: offset_of!(Rmlvk::ShaderStageFlags::Vertex, tex_coord),
        //         },
        //     ],
        //     std::mem::size_of::<Rmlvk::ShaderStageFlags::Vertex>() as u32,
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

        // vk::ShaderStageFlags::Compute pipelines
        lumal.create_compute_pipe(
            &mut pipes.radiance_pipe,
            None,
            shaders::get_shader("radiance.comp.spv").unwrap(),
            (std::mem::size_of::<i32>() * 2) as u32,
            vk::PipelineCreateFlags::DISPATCH_BASE,
            #[cfg(feature = "debug_validation_names")]
            Some("Radiance"),
        );

        lumal.create_compute_pipe(
            &mut pipes.update_grass_pipe,
            None,
            shaders::get_shader("updateGrass.comp.spv").unwrap(),
            (std::mem::size_of::<vec2>() * 2 + std::mem::size_of::<f32>()) as u32,
            vk::PipelineCreateFlags::empty(),
            #[cfg(feature = "debug_validation_names")]
            Some("Grass Updates"),
        );

        lumal.create_compute_pipe(
            &mut pipes.update_water_pipe,
            None,
            shaders::get_shader("updateWater.comp.spv").unwrap(),
            (std::mem::size_of::<f32>() + std::mem::size_of::<vec2>() * 2) as u32,
            vk::PipelineCreateFlags::empty(),
            #[cfg(feature = "debug_validation_names")]
            Some("Water Updates"),
        );

        lumal.create_compute_pipe(
            &mut pipes.gen_perlin2d_pipe,
            None,
            shaders::get_shader("perlin2.comp.spv").unwrap(),
            0, // No push constants
            vk::PipelineCreateFlags::empty(),
            #[cfg(feature = "debug_validation_names")]
            Some("Perlin 2D Noise"),
        );

        lumal.create_compute_pipe(
            &mut pipes.gen_perlin3d_pipe,
            None,
            shaders::get_shader("perlin3.comp.spv").unwrap(),
            0, // No push constants
            vk::PipelineCreateFlags::empty(),
            #[cfg(feature = "debug_validation_names")]
            Some("Perlin 3D Noise"),
        );

        lumal.create_compute_pipe(
            &mut pipes.map_pipe,
            Some(pipes.map_push_layout),
            shaders::get_shader("map.comp.spv").unwrap(),
            (std::mem::size_of::<mat4>() + std::mem::size_of::<ivec4>()) as u32,
            vk::PipelineCreateFlags::empty(),
            #[cfg(feature = "debug_validation_names")]
            Some("Mapping Models Voxels"),
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
    fn do_smth_all_descriptors<FunWithoutDebugNames>(
        process: &FunWithoutDebugNames,

        lumal: &mut Renderer,
        buffers: &AllBuffers,
        iimages: &AllIndependentImages,
        dimages: &AllSwapchainDependentImages,
        samplers: &AllSamplers,
        pipes: &mut AllPipes,
    ) where
        FunWithoutDebugNames: for<'b> Fn(
            &'b mut Renderer,
            &'b mut vk::DescriptorSetLayout,
            &'b mut Ring<vk::DescriptorSet>,
            &'b [DescriptorInfo],
            vk::ShaderStageFlags,
            vk::DescriptorSetLayoutCreateFlags,
            Option<&str>,
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
                vk::DescriptorType::UNIFORM_BUFFER,
                Current,
                Some(&buffers.light_uniform),
                None,
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
                vk::ShaderStageFlags::VERTEX,
            )],
            vk::ShaderStageFlags::VERTEX,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Lightmap Blocks Descriptor Set Layout"),
        );

        // Defer descriptor setup for lightmapModelsPipe
        process(
            lumal,
            &mut pipes.lightmap_models_pipe.set_layout,
            &mut pipes.lightmap_models_pipe.sets,
            &[DescriptorInfo::make_new(
                vk::DescriptorType::UNIFORM_BUFFER,
                Current,
                Some(&buffers.light_uniform),
                None,
                Sampler::null(),
                vk::ImageLayout::GENERAL,
                vk::ShaderStageFlags::VERTEX,
            )],
            vk::ShaderStageFlags::VERTEX,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Lightmap Models Descriptor Set Layout"),
        );

        // Defer descriptor setup for radiancePipe
        process(
            lumal,
            &mut pipes.radiance_pipe.set_layout,
            &mut pipes.radiance_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&iimages.world),
                    samplers.unnorm_nearest,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::COMPUTE,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(&iimages.origin_block_palette),
                    samplers.unnorm_nearest,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::COMPUTE,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(&iimages.material_palette),
                    samplers.nearest_sampler,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::COMPUTE,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&iimages.radiance_cache),
                    samplers.unnorm_linear,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::COMPUTE,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::STORAGE_IMAGE,
                    First,
                    None,
                    Some(&iimages.radiance_cache),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::COMPUTE,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::STORAGE_BUFFER,
                    First,
                    Some(&buffers.gpu_radiance_updates),
                    None,
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::COMPUTE,
                ),
            ],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Radiance Descriptor Set Layout"),
        );

        // Defer descriptor setup for diffusePipe
        process(
            lumal,
            &mut pipes.diffuse_pipe.set_layout,
            &mut pipes.diffuse_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::UNIFORM_BUFFER,
                    Current,
                    Some(&buffers.uniform),
                    None,
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(&dimages.highres_mat_norm),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(&dimages.highres_depth_stencil),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&iimages.material_palette),
                    samplers.nearest_sampler,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(&iimages.radiance_cache),
                    samplers.unnorm_linear,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&iimages.lightmap),
                    samplers.shadow_sampler,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Fill Stencil Glossy Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.ao_pipe.set_layout,
            &mut pipes.ao_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::UNIFORM_BUFFER,
                    Current,
                    Some(&buffers.uniform),
                    None,
                    vk::Sampler::null(),
                    vk::ImageLayout::UNDEFINED,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::UNIFORM_BUFFER,
                    Current,
                    Some(&buffers.ao_lut_uniform),
                    None,
                    vk::Sampler::null(),
                    vk::ImageLayout::UNDEFINED,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(&dimages.highres_mat_norm),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&dimages.highres_depth_stencil),
                    samplers.nearest_sampler,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Fill Stencil Smoke Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.tonemap_pipe.set_layout,
            &mut pipes.tonemap_pipe.sets,
            &[DescriptorInfo::make_new(
                vk::DescriptorType::INPUT_ATTACHMENT,
                First,
                None,
                Some(&dimages.highres_frame),
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
                vk::ShaderStageFlags::FRAGMENT,
            )],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Tonemap Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.fill_stencil_glossy_pipe.set_layout,
            &mut pipes.fill_stencil_glossy_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(&dimages.highres_mat_norm),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(&iimages.material_palette),
                    samplers.nearest_sampler,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Fill Stencil Glossy Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.fill_stencil_smoke_pipe.set_layout,
            &mut pipes.fill_stencil_smoke_pipe.sets,
            &[DescriptorInfo::make_new(
                vk::DescriptorType::UNIFORM_BUFFER,
                Current,
                Some(&buffers.uniform),
                None,
                vk::Sampler::null(),
                vk::ImageLayout::UNDEFINED,
                vk::ShaderStageFlags::VERTEX,
            )],
            vk::ShaderStageFlags::VERTEX,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Fill Stencil Smoke Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.glossy_pipe.set_layout,
            &mut pipes.glossy_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::UNIFORM_BUFFER,
                    Current,
                    Some(&buffers.uniform),
                    None,
                    vk::Sampler::null(),
                    vk::ImageLayout::UNDEFINED,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&dimages.highres_mat_norm),
                    samplers.nearest_sampler,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&dimages.highres_depth_stencil),
                    samplers.nearest_sampler,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&iimages.world),
                    samplers.unnorm_nearest,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(&iimages.origin_block_palette),
                    samplers.unnorm_nearest,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(&iimages.material_palette),
                    samplers.nearest_sampler,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&iimages.radiance_cache),
                    samplers.unnorm_linear,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Glossy Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.smoke_pipe.set_layout,
            &mut pipes.smoke_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::UNIFORM_BUFFER,
                    Current,
                    Some(&buffers.uniform),
                    None,
                    vk::Sampler::null(),
                    vk::ImageLayout::UNDEFINED,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(&dimages.far_depth),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::INPUT_ATTACHMENT,
                    First,
                    None,
                    Some(&dimages.near_depth),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::STORAGE_IMAGE,
                    First,
                    None,
                    Some(&iimages.radiance_cache),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&iimages.perlin_noise3d),
                    samplers.linear_sampler_tiled,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Smoke Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.raygen_blocks_pipe.set_layout,
            &mut pipes.raygen_blocks_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::UNIFORM_BUFFER,
                    Current,
                    Some(&buffers.uniform),
                    None,
                    vk::Sampler::null(),
                    vk::ImageLayout::UNDEFINED,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    Current,
                    None,
                    Some(&iimages.origin_block_palette),
                    samplers.unnorm_nearest,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ),
            ],
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Raygen Blocks Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.raygen_models_pipe.set_layout,
            &mut pipes.raygen_models_pipe.sets,
            &[DescriptorInfo::make_new(
                vk::DescriptorType::UNIFORM_BUFFER,
                Current,
                Some(&buffers.uniform),
                None,
                vk::Sampler::null(),
                vk::ImageLayout::UNDEFINED,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            )],
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Raygen Models Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.raygen_particles_pipe.set_layout,
            &mut pipes.raygen_particles_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::UNIFORM_BUFFER,
                    Current,
                    Some(&buffers.uniform),
                    None,
                    vk::Sampler::null(),
                    vk::ImageLayout::UNDEFINED,
                    vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::GEOMETRY,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::STORAGE_IMAGE,
                    First,
                    None,
                    Some(&iimages.world),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::GEOMETRY,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::STORAGE_IMAGE,
                    Current,
                    None,
                    Some(&iimages.origin_block_palette),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::GEOMETRY,
                ),
            ],
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::GEOMETRY,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Raygen Particles Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.update_grass_pipe.set_layout,
            &mut pipes.update_grass_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::STORAGE_IMAGE,
                    First,
                    None,
                    Some(&iimages.grass_state),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::COMPUTE,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&iimages.perlin_noise2d),
                    samplers.linear_sampler_tiled,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::COMPUTE,
                ),
            ],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Update Grass Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.update_water_pipe.set_layout,
            &mut pipes.update_water_pipe.sets,
            &[DescriptorInfo::make_new(
                vk::DescriptorType::STORAGE_IMAGE,
                First,
                None,
                Some(&iimages.water_state),
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
                vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT
                    | vk::ShaderStageFlags::COMPUTE,
            )],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Update Water Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.raygen_water_pipe.set_layout,
            &mut pipes.raygen_water_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::UNIFORM_BUFFER,
                    Current,
                    Some(&buffers.uniform),
                    None,
                    vk::Sampler::null(),
                    vk::ImageLayout::UNDEFINED,
                    vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::GEOMETRY,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    First,
                    None,
                    Some(&iimages.water_state),
                    samplers.linear_sampler_tiled,
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::GEOMETRY,
                ),
            ],
            vk::ShaderStageFlags::VERTEX,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Raygen Water Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.gen_perlin2d_pipe.set_layout,
            &mut pipes.gen_perlin2d_pipe.sets,
            &[DescriptorInfo::make_new(
                vk::DescriptorType::STORAGE_IMAGE,
                First,
                None,
                Some(&iimages.perlin_noise2d),
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
                vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT
                    | vk::ShaderStageFlags::COMPUTE,
            )],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Gen Perlin 2D Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.gen_perlin3d_pipe.set_layout,
            &mut pipes.gen_perlin3d_pipe.sets,
            &[DescriptorInfo::make_new(
                vk::DescriptorType::STORAGE_IMAGE,
                First,
                None,
                Some(&iimages.perlin_noise3d),
                vk::Sampler::null(),
                vk::ImageLayout::GENERAL,
                vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT
                    | vk::ShaderStageFlags::COMPUTE, // TODO: remove all unnecessary stages
            )],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Gen Perlin 3D Descriptor Set Layout"),
        );

        process(
            lumal,
            &mut pipes.map_pipe.set_layout,
            &mut pipes.map_pipe.sets,
            &[
                DescriptorInfo::make_new(
                    vk::DescriptorType::STORAGE_IMAGE,
                    First,
                    None,
                    Some(&iimages.world),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::COMPUTE,
                ),
                DescriptorInfo::make_new(
                    vk::DescriptorType::STORAGE_IMAGE,
                    Current,
                    None,
                    Some(&iimages.origin_block_palette),
                    vk::Sampler::null(),
                    vk::ImageLayout::GENERAL,
                    vk::ShaderStageFlags::COMPUTE,
                ),
            ],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Map Descriptor Set Layout"),
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
        _debug_name: Option<&str>,
    ) {
        lumal.anounce_descriptor_setup(
            dset_layout,
            descriptor_sets,
            descriptions,
            default_stages,
            create_flags,
            #[cfg(feature = "debug_validation_names")]
            _debug_name,
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
        _debug_name: Option<&str>,
    ) {
        lumal.acutally_setup_descriptor(
            dset_layout,
            descriptor_sets,
            descriptions,
            default_stages,
            create_flags,
            #[cfg(feature = "debug_validation_names")]
            _debug_name,
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
        #[cfg(feature = "debug_validation_names")]
        Some(&"Overlay Pipeline Set Layout"),
    );
    lumal.create_descriptor_set_layout(
        &[ShortDescriptorInfo {
            descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
            stages: vk::ShaderStageFlags::COMPUTE,
        }],
        &mut pipes.map_push_layout,
        vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
        #[cfg(feature = "debug_validation_names")]
        Some(&"Map"),
    );
    lumal.create_descriptor_set_layout(
        &[ShortDescriptorInfo {
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            stages: vk::ShaderStageFlags::FRAGMENT,
        }],
        &mut pipes.raygen_models_push_layout,
        vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
        #[cfg(feature = "debug_validation_names")]
        Some(&"Raygen Models"),
    );
}
