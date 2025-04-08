// use crate::{internal_renderer::render_gl::FRAMES_IN_FLIGHT, *};

use glow::HasContext;

impl super::InternalRendererGL {
    #[cold]
    #[optimize(size)]
    pub fn gen_perlin_2d(&mut self) {
        let gl = &self.gl;

        unsafe { gl.use_program(Some(self.pipes.gen_perlin2d_pipe.program)) };
    }

    #[cold]
    #[optimize(size)]
    pub fn gen_perlin_3d(&mut self) {}
}
