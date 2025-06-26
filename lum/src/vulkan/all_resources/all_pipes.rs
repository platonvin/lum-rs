use crate::types::*;
use crate::{
    vulkan::{
        self, types::*, AllBuffers, AllIndependentImages, AllPipes, AllSamplers,
        AllSwapchainDependentImages, InternalRendererVulkan,
    },
    Settings,
};
use containers::array3d::Dim3;
use containers::Ring;
use lumal::descriptors::{
    AttrFormOffs, BlendAttachment, DepthTesting, DescriptorInfo, DescriptorResource,
    ShortDescriptorInfo,
};
use lumal::{vk, LumalSettings, Renderer};
use std::mem::offset_of;

// This file could be just data?
// it is setting up all the descriptors/layouts for pipes and pipes themeselves

impl<'a, D: Dim3> InternalRendererVulkan<'a, D> {
    /// Creates bundle of all pipes
    /// Most pipes are hardcoded, but foliage pipes are defined by shaders
    pub fn create_all_pipes(
        lumal: &mut Renderer,
        lum_settings: &Settings<D>,
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
            &InternalRendererVulkan::<D>::anounce_descriptor_setup_wrapper,
            lumal,
            buffers,
            iimages,
            dimages,
            samplers,
            pipes,
        );

        // (actually) allocate space that is enough for all descriptors
        // this is one of the places where init-time resources simplify everything
        // otherwise we have to reallocate and rebuild entire engine once in a while (or lose some perfomance)
        lumal.flush_descriptor_setup();

        // allocate each descriptor set
        Self::do_smth_all_descriptors(
            &InternalRendererVulkan::<D>::acutally_setup_descriptor_wrapper,
            lumal,
            buffers,
            iimages,
            dimages,
            samplers,
            pipes,
        );

        lumal.create_raster_pipe(
            &mut pipes.lightmap_blocks_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::LightmapBlocksVert),
            None,
            &[AttrFormOffs {
                binding: 0,
                format: vk::Format::R8G8B8_UINT,
                offset: offset_of!(PackedVoxelCircuit, pos),
            }],
            std::mem::size_of::<PackedVoxelCircuit>() as u32,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            vk::Extent2D {
                width: lum_settings.lightmap_extent.x,
                height: lum_settings.lightmap_extent.y,
            },
            &[BlendAttachment::NoBlend],
            std::mem::size_of::<i16vec4>() as u32,
            DepthTesting::ReadWrite,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(), // no stencil
            #[cfg(feature = "debug_validation_names")]
            Some("Lightmap Blocks"),
        );

        lumal.create_raster_pipe(
            &mut pipes.lightmap_models_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::LightmapModelsVert),
            None,
            &[AttrFormOffs {
                binding: 0,
                format: vk::Format::R8G8B8_UINT,
                offset: offset_of!(PackedVoxelCircuit, pos),
            }],
            std::mem::size_of::<PackedVoxelCircuit>() as u32,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            vk::Extent2D {
                width: lum_settings.lightmap_extent.x,
                height: lum_settings.lightmap_extent.y,
            },
            &[BlendAttachment::NoBlend],
            (std::mem::size_of::<quat>() + std::mem::size_of::<vec4>()) as u32, // push size
            DepthTesting::ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(), // no stencil
            #[cfg(feature = "debug_validation_names")]
            Some("Lightmap Models"),
        );

        lumal.create_raster_pipe(
            &mut pipes.raygen_blocks_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::RaygenBlocksVert),
            Some(shaders::Shader::get_spirv(
                shaders::Shader::RaygenBlocksFrag,
            )),
            &[AttrFormOffs {
                binding: 0,
                format: vk::Format::R8G8B8_UINT, // TODO: automatic in macro
                offset: offset_of!(PackedVoxelCircuit, pos),
            }],
            std::mem::size_of::<PackedVoxelCircuit>() as u32,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.swapchain_extent,
            &[BlendAttachment::NoBlend],
            12, // push size
            DepthTesting::ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(), // no stencil
            #[cfg(feature = "debug_validation_names")]
            Some("Raygen Blocks"),
        );

        lumal.create_raster_pipe(
            &mut pipes.raygen_models_pipe,
            Some(pipes.raygen_models_push_layout),
            shaders::Shader::get_spirv(shaders::Shader::RaygenModelsVert),
            Some(shaders::Shader::get_spirv(
                shaders::Shader::RaygenModelsFrag,
            )),
            &[AttrFormOffs {
                binding: 0,
                format: vk::Format::R8G8B8_UINT,
                offset: offset_of!(PackedVoxelCircuit, pos),
            }],
            std::mem::size_of::<PackedVoxelCircuit>() as u32,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.swapchain_extent,
            &[BlendAttachment::NoBlend],
            (std::mem::size_of::<vec4>() * 3) as u32,
            DepthTesting::ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(),
            #[cfg(feature = "debug_validation_names")]
            Some("Raygen Models"),
        );

        lumal.create_raster_pipe(
            &mut pipes.raygen_particles_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::RaygenParticlesVert),
            Some(shaders::Shader::get_spirv(
                shaders::Shader::RaygenParticlesFrag,
            )),
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
            lumal.swapchain_extent,
            &[BlendAttachment::NoBlend],
            0,
            DepthTesting::ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(),
            #[cfg(feature = "debug_validation_names")]
            Some("Raygen Particles"),
        );

        lumal.create_raster_pipe(
            &mut pipes.raygen_water_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::WaterVert),
            Some(shaders::Shader::get_spirv(shaders::Shader::WaterFrag)),
            &[],
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_STRIP,
            lumal.swapchain_extent,
            &[BlendAttachment::NoBlend],
            (std::mem::size_of::<vec4>() + (std::mem::size_of::<i32>() * 2)) as u32,
            DepthTesting::ReadWrite,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(),
            #[cfg(feature = "debug_validation_names")]
            Some("Raygen Water"),
        );

        for (i, foliage) in pipes.raygen_foliage_pipes.iter_mut().enumerate() {
            let desc = &foliage_descriptions[i];
            // let vs = desc.vk::ShaderStageFlags::vertex_shader_file.as_str();
            lumal.create_raster_pipe(
                foliage,
                None,
                &desc.spirv_code,
                Some(shaders::Shader::get_spirv(shaders::Shader::GrassFrag)),
                &[AttrFormOffs {
                    binding: 0,
                    format: vk::Format::R8G8B8_UINT,
                    offset: offset_of!(PackedVoxelCircuit, pos),
                }],
                std::mem::size_of::<PackedVoxelCircuit>() as u32,
                vk::VertexInputRate::VERTEX,
                vk::PrimitiveTopology::TRIANGLE_LIST,
                lumal.swapchain_extent,
                &[BlendAttachment::NoBlend],
                (std::mem::size_of::<vec4>() + std::mem::size_of::<vec4>()) as u32, // push size
                DepthTesting::ReadWrite,
                // DepthTesting::DT_None,
                vk::CompareOp::LESS,
                vk::CullModeFlags::NONE,
                vk::StencilOpState::default(),
                #[cfg(feature = "debug_validation_names")]
                Some("Raygen Foliage"),
            );
        }

        lumal.create_raster_pipe(
            &mut pipes.diffuse_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_spirv(shaders::Shader::DiffuseFrag)),
            &[],
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.swapchain_extent,
            &[BlendAttachment::NoBlend],
            (std::mem::size_of::<ivec4>()
                + (std::mem::size_of::<vec4>() * 4)
                + std::mem::size_of::<mat4>()) as u32,
            DepthTesting::None,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(),
            #[cfg(feature = "debug_validation_names")]
            Some("Diffuse"),
        );

        lumal.create_raster_pipe(
            &mut pipes.ao_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_spirv(shaders::Shader::HbaoFrag)),
            &[],
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.swapchain_extent,
            &[BlendAttachment::BlendMix],
            0,
            DepthTesting::None,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(),
            #[cfg(feature = "debug_validation_names")]
            Some("Ambient Occlusion"),
        );

        lumal.create_raster_pipe(
            &mut pipes.fill_stencil_glossy_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_spirv(
                shaders::Shader::FillStencilGlossyFrag,
            )),
            &[], // Fullscreen pass, no attributes
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.swapchain_extent,
            &[BlendAttachment::NoBlend],
            0, // No push constants
            DepthTesting::None,
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
            #[cfg(feature = "debug_validation_names")]
            Some("Fill Stencil+Glossy"),
        );

        lumal.create_raster_pipe(
            &mut pipes.fill_stencil_smoke_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::FillStencilSmokeVert),
            Some(shaders::Shader::get_spirv(
                shaders::Shader::FillStencilSmokeFrag,
            )),
            &[], // Push constants only
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.swapchain_extent,
            &[
                BlendAttachment::BlendReplaceIfGreater,
                BlendAttachment::BlendReplaceIfLess,
            ],
            (std::mem::size_of::<vec3>() + std::mem::size_of::<i32>() + std::mem::size_of::<vec4>())
                as u32,
            DepthTesting::Read,
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
            #[cfg(feature = "debug_validation_names")]
            Some("Fill Stencil for Smoke"),
        );

        lumal.create_raster_pipe(
            &mut pipes.glossy_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_spirv(shaders::Shader::GlossyFrag)),
            &[], // Fullscreen pass, no attributes
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.swapchain_extent,
            &[BlendAttachment::BlendMix],
            (std::mem::size_of::<vec4>() + std::mem::size_of::<vec4>()) as u32,
            DepthTesting::None,
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
            #[cfg(feature = "debug_validation_names")]
            Some("Glossy"),
        );

        lumal.create_raster_pipe(
            &mut pipes.smoke_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_spirv(shaders::Shader::SmokeFrag)),
            &[], // Fullscreen pass, no attributes
            0,
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.swapchain_extent,
            &[BlendAttachment::BlendMix],
            0, // No push constants
            DepthTesting::None,
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
            #[cfg(feature = "debug_validation_names")]
            Some("Smoke"),
        );

        lumal.create_raster_pipe(
            &mut pipes.tonemap_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::FullscreenTriagVert),
            Some(shaders::Shader::get_spirv(shaders::Shader::TonemapFrag)),
            &[], // Fullscreen pass, no attributes
            0,   // No vk::ShaderStageFlags::vertex size
            vk::VertexInputRate::VERTEX,
            vk::PrimitiveTopology::TRIANGLE_LIST,
            lumal.swapchain_extent,
            &[BlendAttachment::NoBlend],
            0, // No push constants
            DepthTesting::None,
            // DepthTesting::DT_None,
            vk::CompareOp::LESS,
            vk::CullModeFlags::NONE,
            vk::StencilOpState::default(), // no stencil
            #[cfg(feature = "debug_validation_names")]
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
        //     lumal.swapchain_extent,
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
            shaders::Shader::get_spirv(shaders::Shader::RadianceComp),
            (std::mem::size_of::<i32>() * 2) as u32,
            vk::PipelineCreateFlags::DISPATCH_BASE,
            #[cfg(feature = "debug_validation_names")]
            Some("Radiance"),
        );

        lumal.create_compute_pipe(
            &mut pipes.update_grass_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::UpdateGrassComp),
            (std::mem::size_of::<vec2>() * 2 + std::mem::size_of::<f32>()) as u32,
            vk::PipelineCreateFlags::empty(),
            #[cfg(feature = "debug_validation_names")]
            Some("Grass Updates"),
        );

        lumal.create_compute_pipe(
            &mut pipes.update_water_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::UpdateWaterComp),
            (std::mem::size_of::<f32>() + std::mem::size_of::<vec2>() * 2) as u32,
            vk::PipelineCreateFlags::empty(),
            #[cfg(feature = "debug_validation_names")]
            Some("Water Updates"),
        );

        lumal.create_compute_pipe(
            &mut pipes.gen_perlin2d_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::Perlin2Comp),
            0, // No push constants
            vk::PipelineCreateFlags::empty(),
            #[cfg(feature = "debug_validation_names")]
            Some("Perlin 2D Noise"),
        );

        lumal.create_compute_pipe(
            &mut pipes.gen_perlin3d_pipe,
            None,
            shaders::Shader::get_spirv(shaders::Shader::Perlin3Comp),
            0, // No push constants
            vk::PipelineCreateFlags::empty(),
            #[cfg(feature = "debug_validation_names")]
            Some("Perlin 3D Noise"),
        );

        lumal.create_compute_pipe(
            &mut pipes.map_pipe,
            Some(pipes.map_push_layout),
            shaders::Shader::get_spirv(shaders::Shader::MapComp),
            (std::mem::size_of::<mat4>() + std::mem::size_of::<ivec4>()) as u32,
            vk::PipelineCreateFlags::empty(),
            #[cfg(feature = "debug_validation_names")]
            Some("Mapping Models Voxels"),
        );
    }

    pub fn destroy_all_pipes(lumal: &mut Renderer, mut pipes: AllPipes) {
        lumal.destroy_raster_pipe(pipes.lightmap_blocks_pipe);
        lumal.destroy_raster_pipe(pipes.lightmap_models_pipe);

        lumal.destroy_raster_pipe(pipes.raygen_blocks_pipe);
        lumal.destroy_raster_pipe(pipes.raygen_models_pipe);
        unsafe {
            lumal
                .device
                .destroy_descriptor_set_layout(pipes.raygen_models_push_layout, None)
        };
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
        unsafe { lumal.device.destroy_descriptor_set_layout(pipes.overlay_pipe.set_layout, None) };

        lumal.destroy_compute_pipe(pipes.radiance_pipe);
        lumal.destroy_compute_pipe(pipes.map_pipe);
        unsafe { lumal.device.destroy_descriptor_set_layout(pipes.map_push_layout, None) };
        lumal.destroy_compute_pipe(pipes.update_grass_pipe);
        lumal.destroy_compute_pipe(pipes.update_water_pipe);
        lumal.destroy_compute_pipe(pipes.gen_perlin2d_pipe); // generate noise for grass
        lumal.destroy_compute_pipe(pipes.gen_perlin3d_pipe); // generate noise for grass
    }

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
            &[DescriptorInfo {
                resources: DescriptorResource::UniformBuffer(
                    lumal::descriptors::RelativeResource::Current(&buffers.light_uniform),
                ),
                specified_stages: vk::ShaderStageFlags::VERTEX,
            }],
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
            &[DescriptorInfo {
                resources: DescriptorResource::UniformBuffer(
                    lumal::descriptors::RelativeResource::Current(&buffers.light_uniform),
                ),
                specified_stages: vk::ShaderStageFlags::VERTEX,
            }],
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
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.world),
                        vk::ImageLayout::GENERAL,
                        samplers.unnorm_nearest,
                    ),
                    specified_stages: vk::ShaderStageFlags::COMPUTE,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.block_palette),
                        vk::ImageLayout::GENERAL,
                        samplers.unnorm_nearest,
                    ),
                    specified_stages: vk::ShaderStageFlags::COMPUTE,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.material_palette),
                        vk::ImageLayout::GENERAL,
                        samplers.nearest_sampler,
                    ),
                    specified_stages: vk::ShaderStageFlags::COMPUTE,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Previous(&iimages.radiance_cache),
                        vk::ImageLayout::GENERAL,
                        samplers.unnorm_linear,
                    ),
                    specified_stages: vk::ShaderStageFlags::COMPUTE,
                },
                DescriptorInfo {
                    resources: DescriptorResource::StorageImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.radiance_cache),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::COMPUTE,
                },
                DescriptorInfo {
                    resources: DescriptorResource::StorageBuffer(
                        lumal::descriptors::RelativeResource::Single(&buffers.gpu_radiance_updates),
                    ),
                    specified_stages: vk::ShaderStageFlags::COMPUTE,
                },
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
                DescriptorInfo {
                    resources: DescriptorResource::UniformBuffer(
                        lumal::descriptors::RelativeResource::Current(&buffers.uniform),
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::InputAttachment(
                        lumal::descriptors::RelativeResource::Single(&dimages.mat_norm),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::InputAttachment(
                        lumal::descriptors::RelativeResource::Single(&dimages.depth_stencil),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.material_palette),
                        vk::ImageLayout::GENERAL,
                        samplers.nearest_sampler,
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.radiance_cache),
                        vk::ImageLayout::GENERAL,
                        samplers.linear_sampler,
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Single(&iimages.lightmap),
                        vk::ImageLayout::GENERAL,
                        samplers.shadow_sampler,
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                },
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
                DescriptorInfo {
                    resources: DescriptorResource::UniformBuffer(
                        lumal::descriptors::RelativeResource::Current(&buffers.uniform),
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::UniformBuffer(
                        lumal::descriptors::RelativeResource::Current(&buffers.ao_lut_uniform),
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::InputAttachment(
                        lumal::descriptors::RelativeResource::Single(&dimages.mat_norm),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Single(&dimages.depth_stencil),
                        vk::ImageLayout::GENERAL,
                        samplers.nearest_sampler,
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                },
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
            &[DescriptorInfo {
                resources: DescriptorResource::InputAttachment(
                    lumal::descriptors::RelativeResource::Single(&dimages.frame),
                    vk::ImageLayout::GENERAL,
                ),
                specified_stages: vk::ShaderStageFlags::FRAGMENT,
            }],
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
                DescriptorInfo {
                    resources: DescriptorResource::InputAttachment(
                        lumal::descriptors::RelativeResource::Single(&dimages.mat_norm),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.material_palette),
                        vk::ImageLayout::GENERAL,
                        samplers.nearest_sampler,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
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
            &[DescriptorInfo {
                resources: DescriptorResource::UniformBuffer(
                    lumal::descriptors::RelativeResource::Current(&buffers.uniform),
                ),
                specified_stages: vk::ShaderStageFlags::VERTEX,
            }],
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
                DescriptorInfo {
                    resources: DescriptorResource::UniformBuffer(
                        lumal::descriptors::RelativeResource::Current(&buffers.uniform),
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Single(&dimages.mat_norm),
                        vk::ImageLayout::GENERAL,
                        samplers.nearest_sampler,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Single(&dimages.depth_stencil),
                        vk::ImageLayout::GENERAL,
                        samplers.nearest_sampler,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.world),
                        vk::ImageLayout::GENERAL,
                        samplers.unnorm_nearest,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.block_palette),
                        vk::ImageLayout::GENERAL,
                        samplers.unnorm_nearest,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.material_palette),
                        vk::ImageLayout::GENERAL,
                        samplers.nearest_sampler,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.radiance_cache),
                        vk::ImageLayout::GENERAL,
                        samplers.unnorm_linear,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
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
                DescriptorInfo {
                    resources: DescriptorResource::UniformBuffer(
                        lumal::descriptors::RelativeResource::Current(&buffers.uniform),
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::InputAttachment(
                        lumal::descriptors::RelativeResource::Single(&dimages.far_depth),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::InputAttachment(
                        lumal::descriptors::RelativeResource::Single(&dimages.near_depth),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::StorageImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.radiance_cache),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Single(&iimages.perlin_noise3d),
                        vk::ImageLayout::GENERAL,
                        samplers.linear_sampler_tiled,
                    ),
                    specified_stages: vk::ShaderStageFlags::FRAGMENT,
                },
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
                DescriptorInfo {
                    resources: DescriptorResource::UniformBuffer(
                        lumal::descriptors::RelativeResource::Current(&buffers.uniform),
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.block_palette),
                        vk::ImageLayout::GENERAL,
                        samplers.unnorm_nearest,
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                },
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
            &[DescriptorInfo {
                resources: DescriptorResource::UniformBuffer(
                    lumal::descriptors::RelativeResource::Current(&buffers.uniform),
                ),
                specified_stages: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            }],
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
                DescriptorInfo {
                    resources: DescriptorResource::UniformBuffer(
                        lumal::descriptors::RelativeResource::Current(&buffers.uniform),
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::GEOMETRY,
                },
                DescriptorInfo {
                    resources: DescriptorResource::StorageImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.world),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::GEOMETRY,
                },
                DescriptorInfo {
                    resources: DescriptorResource::StorageImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.block_palette),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::GEOMETRY,
                },
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
                DescriptorInfo {
                    resources: DescriptorResource::StorageImage(
                        lumal::descriptors::RelativeResource::Single(&iimages.grass_state),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::COMPUTE,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Single(&iimages.perlin_noise2d),
                        vk::ImageLayout::GENERAL,
                        samplers.linear_sampler_tiled,
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::COMPUTE,
                },
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
            &[DescriptorInfo {
                resources: DescriptorResource::StorageImage(
                    lumal::descriptors::RelativeResource::Single(&iimages.water_state),
                    vk::ImageLayout::GENERAL,
                ),
                specified_stages: vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT
                    | vk::ShaderStageFlags::COMPUTE,
            }],
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
                DescriptorInfo {
                    resources: DescriptorResource::UniformBuffer(
                        lumal::descriptors::RelativeResource::Current(&buffers.uniform),
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::GEOMETRY,
                },
                DescriptorInfo {
                    resources: DescriptorResource::SampledImage(
                        lumal::descriptors::RelativeResource::Single(&iimages.water_state),
                        vk::ImageLayout::GENERAL,
                        samplers.linear_sampler_tiled,
                    ),
                    specified_stages: vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::GEOMETRY,
                },
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
            &[DescriptorInfo {
                resources: DescriptorResource::StorageImage(
                    lumal::descriptors::RelativeResource::Single(&iimages.perlin_noise2d),
                    vk::ImageLayout::GENERAL,
                ),
                specified_stages: vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT
                    | vk::ShaderStageFlags::COMPUTE,
            }],
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
            &[DescriptorInfo {
                resources: DescriptorResource::StorageImage(
                    lumal::descriptors::RelativeResource::Single(&iimages.perlin_noise3d),
                    vk::ImageLayout::GENERAL,
                ),
                specified_stages: vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT
                    | vk::ShaderStageFlags::COMPUTE,
            }],
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
                DescriptorInfo {
                    resources: DescriptorResource::StorageImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.world),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::COMPUTE,
                },
                DescriptorInfo {
                    resources: DescriptorResource::StorageImage(
                        lumal::descriptors::RelativeResource::Current(&iimages.block_palette),
                        vk::ImageLayout::GENERAL,
                    ),
                    specified_stages: vk::ShaderStageFlags::COMPUTE,
                },
            ],
            vk::ShaderStageFlags::COMPUTE,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            #[cfg(not(feature = "debug_validation_names"))]
            None,
            #[cfg(feature = "debug_validation_names")]
            Some("Map Descriptor Set Layout"),
        );

        pipes.raygen_foliage_pipes.iter_mut().for_each(|foliage| {
            process(
                lumal,
                &mut foliage.set_layout,
                &mut foliage.sets,
                &[
                    DescriptorInfo {
                        resources: DescriptorResource::UniformBuffer(
                            lumal::descriptors::RelativeResource::Current(&buffers.uniform),
                        ),
                        specified_stages: vk::ShaderStageFlags::VERTEX
                            | vk::ShaderStageFlags::FRAGMENT,
                    },
                    DescriptorInfo {
                        resources: DescriptorResource::SampledImage(
                            lumal::descriptors::RelativeResource::Single(&iimages.grass_state),
                            vk::ImageLayout::GENERAL,
                            samplers.linear_sampler,
                        ),
                        specified_stages: vk::ShaderStageFlags::VERTEX
                            | vk::ShaderStageFlags::FRAGMENT,
                    },
                ],
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                vk::DescriptorSetLayoutCreateFlags::empty(),
                #[cfg(not(feature = "debug_validation_names"))]
                None,
                #[cfg(feature = "debug_validation_names")]
                Some("Foliage Descriptor Set Layout"),
            );
        });
    }

    // Sorry, i dont have enough iq to understand lifetimes

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
