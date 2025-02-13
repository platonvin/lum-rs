use internal_renderer::*;
use lumal::{descriptors::*, LumalSettings, Renderer};
use vulkanalia::vk::{self, Handle};

use crate::*;

impl InternalRenderer {
    #[cold]
    #[optimize(size)]
    pub fn create_all_rpasses(
        lumal: &mut Renderer,
        _lum_settings: &Settings,
        _lumal_settings: &LumalSettings,
        iimages: &AllIndependentImages,
        dimages: &mut AllSwapchainDependentImages,
        pipes: &mut AllPipes,
    ) -> AllRenderPasses {
        println!("creating rpass: lightmap");
        let lightmap_rpass = lumal.create_renderpass(
            &[AttachmentDescription {
                images: &iimages.lightmap,
                load: LoadStoreOp::Clear,
                store: LoadStoreOp::Store,
                sload: LoadStoreOp::DontCare,
                sstore: LoadStoreOp::DontCare,
                clear: vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
                final_layout: vk::ImageLayout::GENERAL,
            }],
            &mut [SubpassDescription {
                pipes: &mut [
                    &mut pipes.lightmap_blocks_pipe,
                    &mut pipes.lightmap_models_pipe,
                ],
                a_input: &[],
                a_color: &[],
                a_depth: Some(&iimages.lightmap),
            }],
        );

        // Second render pass
        println!("creating rpass: gbuffer");

        let mut foliage_pipes = vec![];
        for pipe in &mut pipes.raygen_foliage_pipes {
            foliage_pipes.push(pipe);
        }

        let gbuffer_rpass = lumal.create_renderpass(
            &[
                AttachmentDescription {
                    images: &dimages.highres_mat_norm,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    sload: LoadStoreOp::DontCare,
                    sstore: LoadStoreOp::DontCare,
                    clear: vk::ClearValue::default(),
                    final_layout: vk::ImageLayout::GENERAL,
                },
                AttachmentDescription {
                    images: &dimages.highres_depth_stencil,
                    load: LoadStoreOp::Clear,
                    store: LoadStoreOp::Store,
                    sload: LoadStoreOp::Clear,
                    sstore: LoadStoreOp::Store,
                    clear: vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    },
                    final_layout: vk::ImageLayout::GENERAL,
                },
            ],
            &mut [
                SubpassDescription {
                    pipes: &mut [&mut pipes.raygen_blocks_pipe],
                    a_input: &[],
                    a_color: &[(&dimages.highres_mat_norm)],
                    a_depth: Some(&dimages.highres_depth_stencil),
                },
                SubpassDescription {
                    pipes: &mut [&mut pipes.raygen_models_pipe],
                    a_input: &[],
                    a_color: &[(&dimages.highres_mat_norm)],
                    a_depth: Some(&dimages.highres_depth_stencil),
                },
                SubpassDescription {
                    pipes: &mut [&mut pipes.raygen_particles_pipe],
                    a_input: &[],
                    a_color: &[(&dimages.highres_mat_norm)],
                    a_depth: Some(&dimages.highres_depth_stencil),
                },
                SubpassDescription {
                    pipes: &mut foliage_pipes,
                    a_input: &[],
                    a_color: &[(&dimages.highres_mat_norm)],
                    a_depth: Some(&dimages.highres_depth_stencil),
                },
                SubpassDescription {
                    pipes: &mut [&mut pipes.raygen_water_pipe],
                    a_input: &[],
                    a_color: &[(&dimages.highres_mat_norm)],
                    a_depth: Some(&dimages.highres_depth_stencil),
                },
            ],
        );
        assert!(gbuffer_rpass.render_pass != vk::RenderPass::null());
        assert!(pipes.raygen_models_pipe.render_pass != vk::RenderPass::null());

        // Third render pass
        println!("creating rpass: shade");
        let shade_rpass = lumal.create_renderpass(
            &[
                AttachmentDescription {
                    images: &dimages.highres_mat_norm,
                    load: LoadStoreOp::Load,
                    store: LoadStoreOp::DontCare,
                    sload: LoadStoreOp::DontCare,
                    sstore: LoadStoreOp::DontCare,
                    clear: vk::ClearValue::default(),
                    final_layout: vk::ImageLayout::GENERAL,
                },
                AttachmentDescription {
                    images: &dimages.highres_frame,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::DontCare,
                    sload: LoadStoreOp::DontCare,
                    sstore: LoadStoreOp::DontCare,
                    clear: vk::ClearValue::default(),
                    final_layout: vk::ImageLayout::GENERAL,
                },
                AttachmentDescription {
                    images: &lumal.vulkan_data.swapchain_images,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    sload: LoadStoreOp::DontCare,
                    sstore: LoadStoreOp::DontCare,
                    clear: vk::ClearValue::default(),
                    final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                },
                AttachmentDescription {
                    images: &dimages.highres_depth_stencil,
                    load: LoadStoreOp::Load,
                    store: LoadStoreOp::DontCare,
                    sload: LoadStoreOp::Load,
                    sstore: LoadStoreOp::DontCare,
                    clear: vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    },
                    final_layout: vk::ImageLayout::GENERAL,
                },
                AttachmentDescription {
                    images: &dimages.far_depth,
                    load: LoadStoreOp::Clear,
                    store: LoadStoreOp::DontCare,
                    sload: LoadStoreOp::DontCare,
                    sstore: LoadStoreOp::DontCare,
                    clear: vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [-1000.0, -1000.0, -1000.0, -1000.0],
                        },
                    },
                    final_layout: vk::ImageLayout::GENERAL,
                },
                AttachmentDescription {
                    images: &dimages.near_depth,
                    load: LoadStoreOp::Clear,
                    store: LoadStoreOp::DontCare,
                    sload: LoadStoreOp::DontCare,
                    sstore: LoadStoreOp::DontCare,
                    clear: vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [1000.0, 1000.0, 1000.0, 1000.0],
                        },
                    },
                    final_layout: vk::ImageLayout::GENERAL,
                },
            ],
            &mut [
                SubpassDescription {
                    pipes: &mut [&mut pipes.diffuse_pipe],
                    a_input: &[&dimages.highres_mat_norm, &dimages.highres_depth_stencil],
                    a_color: &[(&dimages.highres_frame)],
                    a_depth: None,
                },
                SubpassDescription {
                    pipes: &mut [&mut pipes.ao_pipe],
                    a_input: &[&dimages.highres_mat_norm, &dimages.highres_depth_stencil],
                    a_color: &[(&dimages.highres_frame)],
                    a_depth: None,
                },
                SubpassDescription {
                    pipes: &mut [&mut pipes.fill_stencil_glossy_pipe],
                    a_input: &[&dimages.highres_mat_norm],
                    a_color: &[],
                    a_depth: Some(&dimages.highres_depth_stencil),
                },
                SubpassDescription {
                    pipes: &mut [&mut pipes.fill_stencil_smoke_pipe],
                    a_input: &[],
                    a_color: &[&dimages.far_depth, &dimages.near_depth],
                    a_depth: Some(&dimages.highres_depth_stencil),
                },
                SubpassDescription {
                    pipes: &mut [&mut pipes.glossy_pipe],
                    a_input: &[],
                    a_color: &[(&dimages.highres_frame)],
                    a_depth: Some(&dimages.highres_depth_stencil),
                },
                SubpassDescription {
                    pipes: &mut [&mut pipes.smoke_pipe],
                    a_input: &[&dimages.near_depth, &dimages.far_depth],
                    a_color: &[(&dimages.highres_frame)],
                    a_depth: Some(&dimages.highres_depth_stencil),
                },
                SubpassDescription {
                    pipes: &mut [&mut pipes.tonemap_pipe],
                    a_input: &[&dimages.highres_frame],
                    a_color: &[(&lumal.vulkan_data.swapchain_images)],
                    a_depth: None,
                },
                // SubpassDescription {
                //     pipes: &mut [&mut pipes.overlay_pipe],
                //     a_input: &[],
                //     a_color: &[(&dimages.swapchain_images)],
                //     a_depth: None,
                // },
            ],
        );

        println!("created all passes");

        AllRenderPasses {
            lightmap_rpass,
            gbuffer_rpass,
            shade_rpass,
        }
    }

    pub fn destroy_all_rpasses(lumal: &mut Renderer, rpasses: AllRenderPasses) {
        lumal.destroy_renderpass(rpasses.lightmap_rpass);
        lumal.destroy_renderpass(rpasses.gbuffer_rpass);
        lumal.destroy_renderpass(rpasses.shade_rpass);
    }
}
