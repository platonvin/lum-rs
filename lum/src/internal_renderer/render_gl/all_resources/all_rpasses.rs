use glow::HasContext;
use lumal::LumalSettings;

use crate::internal_renderer::{
    render_gl::{
        rpass::{
            AttachmentDescriptionGL, AttachmentInfoGL, ClearValueGL, LoadStoreOp, RenderPassGL,
        },
        AllIndependentImages, AllPipes, AllSwapchainDependentImages, InternalRendererGL,
    },
    Settings,
};

impl InternalRendererGL {
    #[cold]
    #[optimize(size)]
    pub fn create_all_rpasses(
        gl: &glow::Context,
        lum_settings: &Settings,
        iimages: &AllIndependentImages,
        dimages: &mut AllSwapchainDependentImages,
        wh: (u32, u32),
    ) -> AllRenderPasses {
        let fullscreen_extent = wh;

        let lightmap_rpass = Self::create_renderpass(
            gl,
            &[AttachmentDescriptionGL {
                texture: iimages.lightmap,
                load: LoadStoreOp::Clear,
                store: LoadStoreOp::Store,
                clear: ClearValueGL::DepthStencil(1.0, 0),
                attachment_type: glow::DEPTH_ATTACHMENT, // Assuming lightmap is a depth texture
            }],
            (
                lum_settings.lightmap_extent.width,
                lum_settings.lightmap_extent.height,
            ),
        );

        // Second render pass - Gbuffer
        let gbuffer_rpass_0 = Self::create_renderpass(
            gl,
            &[
                AttachmentDescriptionGL {
                    texture: dimages.highres_mat_norm,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::COLOR_ATTACHMENT0,
                },
                AttachmentDescriptionGL {
                    texture: dimages.highres_depth_stencil,
                    load: LoadStoreOp::Clear,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::DepthStencil(1.0, 0),
                    attachment_type: glow::DEPTH_ATTACHMENT,
                },
            ],
            fullscreen_extent,
        );

        let gbuffer_rpass_1 = Self::create_renderpass(
            gl,
            &[
                AttachmentDescriptionGL {
                    texture: dimages.highres_mat_norm,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::COLOR_ATTACHMENT0,
                },
                AttachmentDescriptionGL {
                    texture: dimages.highres_depth_stencil,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::DEPTH_ATTACHMENT,
                },
            ],
            fullscreen_extent,
        );

        let gbuffer_rpass_2 = Self::create_renderpass(
            gl,
            &[AttachmentDescriptionGL {
                texture: dimages.highres_depth_stencil,
                load: LoadStoreOp::DontCare,
                store: LoadStoreOp::Store,
                clear: ClearValueGL::None,
                attachment_type: glow::DEPTH_ATTACHMENT,
            }],
            fullscreen_extent,
        );

        let gbuffer_rpass_3 = Self::create_renderpass(
            gl,
            &[
                AttachmentDescriptionGL {
                    texture: dimages.far_depth,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::COLOR_ATTACHMENT0,
                },
                AttachmentDescriptionGL {
                    texture: dimages.near_depth,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::COLOR_ATTACHMENT1,
                },
                AttachmentDescriptionGL {
                    texture: dimages.highres_depth_stencil,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::DEPTH_ATTACHMENT,
                },
            ],
            fullscreen_extent,
        );

        let gbuffer_rpass_4 = Self::create_renderpass(
            gl,
            &[
                AttachmentDescriptionGL {
                    texture: dimages.highres_mat_norm,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::COLOR_ATTACHMENT0,
                },
                AttachmentDescriptionGL {
                    texture: dimages.highres_depth_stencil,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::DEPTH_ATTACHMENT,
                },
            ],
            fullscreen_extent,
        );

        let gbuffer_rpass_5 = Self::create_renderpass(
            gl,
            &[
                AttachmentDescriptionGL {
                    texture: dimages.near_depth,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::COLOR_ATTACHMENT0,
                },
                AttachmentDescriptionGL {
                    texture: dimages.far_depth,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::COLOR_ATTACHMENT1,
                },
                AttachmentDescriptionGL {
                    texture: dimages.highres_depth_stencil,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::Store,
                    clear: ClearValueGL::None,
                    attachment_type: glow::DEPTH_ATTACHMENT,
                },
            ],
            fullscreen_extent,
        );

        // Third render pass - Shade
        let shade_rpass_0 = Self::create_renderpass(
            gl,
            &[
                AttachmentDescriptionGL {
                    texture: dimages.highres_frame,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::DontCare,
                    clear: ClearValueGL::None,
                    attachment_type: glow::COLOR_ATTACHMENT0,
                },
                AttachmentDescriptionGL {
                    texture: dimages.highres_depth_stencil,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::DontCare,
                    clear: ClearValueGL::None,
                    attachment_type: glow::DEPTH_ATTACHMENT,
                },
            ],
            fullscreen_extent,
        );

        let shade_rpass_1 = Self::create_renderpass(
            gl,
            &[
                AttachmentDescriptionGL {
                    texture: dimages.highres_frame,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::DontCare,
                    clear: ClearValueGL::None,
                    attachment_type: glow::COLOR_ATTACHMENT0,
                },
                AttachmentDescriptionGL {
                    texture: dimages.highres_depth_stencil,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::DontCare,
                    clear: ClearValueGL::None,
                    attachment_type: glow::DEPTH_ATTACHMENT,
                },
            ],
            fullscreen_extent,
        );

        let shade_rpass_2 = Self::create_renderpass(
            gl,
            &[AttachmentDescriptionGL {
                texture: dimages.highres_depth_stencil,
                load: LoadStoreOp::DontCare,
                store: LoadStoreOp::DontCare,
                clear: ClearValueGL::None,
                attachment_type: glow::DEPTH_ATTACHMENT,
            }],
            fullscreen_extent,
        );

        let shade_rpass_3 = Self::create_renderpass(
            gl,
            &[
                AttachmentDescriptionGL {
                    texture: dimages.far_depth,
                    load: LoadStoreOp::Clear,
                    store: LoadStoreOp::DontCare,
                    clear: ClearValueGL::Color([-1000.0, -1000.0, -1000.0, -1000.0]),
                    attachment_type: glow::COLOR_ATTACHMENT0,
                },
                AttachmentDescriptionGL {
                    texture: dimages.near_depth,
                    load: LoadStoreOp::Clear,
                    store: LoadStoreOp::DontCare,
                    clear: ClearValueGL::Color([1000.0, 1000.0, 1000.0, 1000.0]),
                    attachment_type: glow::COLOR_ATTACHMENT1,
                },
                AttachmentDescriptionGL {
                    texture: dimages.highres_depth_stencil,
                    load: LoadStoreOp::DontCare,
                    store: LoadStoreOp::DontCare,
                    clear: ClearValueGL::None,
                    attachment_type: glow::DEPTH_ATTACHMENT,
                },
            ],
            fullscreen_extent,
        );

        let shade_rpass_4 = Self::create_renderpass(
            gl,
            &[AttachmentDescriptionGL {
                texture: dimages.highres_frame,
                load: LoadStoreOp::DontCare,
                store: LoadStoreOp::DontCare,
                clear: ClearValueGL::None,
                attachment_type: glow::COLOR_ATTACHMENT0,
            }],
            fullscreen_extent,
        );

        let shade_rpass_5 = Self::create_renderpass(
            gl,
            &[AttachmentDescriptionGL {
                texture: dimages.highres_frame,
                load: LoadStoreOp::DontCare,
                store: LoadStoreOp::DontCare,
                clear: ClearValueGL::None,
                attachment_type: glow::COLOR_ATTACHMENT0,
            }],
            fullscreen_extent,
        );

        // this is not created because swapchain framebuffer is the "default" one
        // but we still memorize (for locality) clear / store values here
        // let shade_rpass_swapchain = Self::create_renderpass(
        //     gl,
        //     &[AttachmentDescriptionGL {
        //         texture: dimages.swapchain_image,
        //         load: LoadStoreOp::DontCare,
        //         store: LoadStoreOp::Store,
        //         clear: ClearValueGL::None,
        //         attachment_type: glow::COLOR_ATTACHMENT0,
        //     }],
        //     fullscreen_extent,
        // );

        let shade_rpass_swapchain = RenderPassGL {
            framebuffer: None,
            attachments: vec![AttachmentInfoGL {
                texture: None,
                attachment_type: glow::COLOR_ATTACHMENT0,
                load: LoadStoreOp::DontCare, // cause we overrwrite it
                store: LoadStoreOp::Store,   // otherwise nothing will be shown
                clear: ClearValueGL::None,
            }],
            extent: fullscreen_extent,
        };

        AllRenderPasses {
            lightmap_rpass,
            gbuffer_rpass_0,
            gbuffer_rpass_1,
            gbuffer_rpass_2,
            gbuffer_rpass_3,
            gbuffer_rpass_4,
            gbuffer_rpass_5,
            shade_rpass_0,
            shade_rpass_1,
            shade_rpass_2,
            shade_rpass_3,
            shade_rpass_4,
            shade_rpass_5,
            shade_rpass_swapchain,
        }
    }

    pub fn destroy_all_rpasses(gl: &glow::Context, rpasses: AllRenderPasses) {
        Self::destroy_renderpass(gl, rpasses.lightmap_rpass);
        Self::destroy_renderpass(gl, rpasses.gbuffer_rpass_0);
        Self::destroy_renderpass(gl, rpasses.gbuffer_rpass_1);
        Self::destroy_renderpass(gl, rpasses.gbuffer_rpass_2);
        Self::destroy_renderpass(gl, rpasses.gbuffer_rpass_3);
        Self::destroy_renderpass(gl, rpasses.gbuffer_rpass_4);
        Self::destroy_renderpass(gl, rpasses.gbuffer_rpass_5);
        Self::destroy_renderpass(gl, rpasses.shade_rpass_0);
        Self::destroy_renderpass(gl, rpasses.shade_rpass_1);
        Self::destroy_renderpass(gl, rpasses.shade_rpass_2);
        Self::destroy_renderpass(gl, rpasses.shade_rpass_3);
        Self::destroy_renderpass(gl, rpasses.shade_rpass_4);
        Self::destroy_renderpass(gl, rpasses.shade_rpass_5);
        Self::destroy_renderpass(gl, rpasses.shade_rpass_swapchain);
    }
}

pub struct AllRenderPasses {
    pub lightmap_rpass: RenderPassGL,
    pub gbuffer_rpass_0: RenderPassGL,
    pub gbuffer_rpass_1: RenderPassGL,
    pub gbuffer_rpass_2: RenderPassGL,
    pub gbuffer_rpass_3: RenderPassGL,
    pub gbuffer_rpass_4: RenderPassGL,
    pub gbuffer_rpass_5: RenderPassGL,
    pub shade_rpass_0: RenderPassGL,
    pub shade_rpass_1: RenderPassGL,
    pub shade_rpass_2: RenderPassGL,
    pub shade_rpass_3: RenderPassGL,
    pub shade_rpass_4: RenderPassGL,
    pub shade_rpass_5: RenderPassGL,
    pub shade_rpass_swapchain: RenderPassGL,
}
