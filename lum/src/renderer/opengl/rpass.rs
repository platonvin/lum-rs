use glow::HasContext;

use super::InternalRendererGL;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ClearValueGL {
    Color([f32; 4]),
    DepthStencil(f32, i32),
    None,
}
impl Default for ClearValueGL {
    fn default() -> Self {
        ClearValueGL::None
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum LoadStoreOp {
    #[default]
    None,
    Load,
    Clear,
    Store,
    DontCare,
}

pub struct AttachmentDescriptionGL {
    pub texture: glow::Texture,
    pub load: LoadStoreOp,
    pub store: LoadStoreOp,
    pub clear: ClearValueGL,
    pub attachment_type: u32, // glow::COLOR_ATTACHMENT0, glow::DEPTH_ATTACHMENT, etc.
}

pub struct AttachmentInfoGL {
    pub texture: Option<glow::Texture>,
    pub attachment_type: u32,
    pub load: LoadStoreOp,
    pub store: LoadStoreOp,
    pub clear: ClearValueGL,
}

pub struct RenderPassGL {
    pub attachments: Vec<AttachmentInfoGL>,
    pub framebuffer: Option<glow::Framebuffer>,
    pub extent: (u32, u32),
}

impl InternalRendererGL {
    pub fn destroy_renderpass(gl: &glow::Context, rpass: RenderPassGL) {
        if let Some(framebuffer) = rpass.framebuffer {
            unsafe {
                gl.delete_framebuffer(framebuffer);
            }
        }
    }

    pub fn create_renderpass(
        gl: &glow::Context,
        attachments: &[AttachmentDescriptionGL],
        wh: (u32, u32),
    ) -> RenderPassGL {
        let (width, height) = wh;
        assert!(width > 0);
        assert!(height > 0);

        unsafe {
            assert!(!attachments.is_empty());

            let framebuffer = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

            let mut attachment_infos = Vec::new();

            for attachment in attachments.iter() {
                let texture = attachment.texture;
                let target = glow::FRAMEBUFFER;

                gl.framebuffer_texture_2d(
                    target,
                    attachment.attachment_type,
                    glow::TEXTURE_2D,
                    Some(texture),
                    0, // mipmap level
                );

                attachment_infos.push(AttachmentInfoGL {
                    texture: Some(texture),
                    attachment_type: attachment.attachment_type,
                    load: attachment.load,
                    store: attachment.store,
                    clear: attachment.clear,
                });
            }

            // Check if the framebuffer is complete
            if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                panic!(
                    "Framebuffer is not complete: {:?}",
                    gl.check_framebuffer_status(glow::FRAMEBUFFER)
                );
            }

            gl.bind_framebuffer(glow::FRAMEBUFFER, None); // Unbind framebuffer

            RenderPassGL {
                attachments: attachment_infos,
                framebuffer: Some(framebuffer),
                extent: (width, height),
            }
        }
    }
}
