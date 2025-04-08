use glow::HasContext;
use std::collections::HashMap;

use super::InternalRendererGL;

#[derive(Clone, Debug)]
pub struct AttrFormOffs {
    pub binding: u32,
    pub data_type: u32, // Corresponding OpenGL data type (e.g., glow::FLOAT, glow::UNSIGNED_BYTE)
    pub size: i32,      // Number of components per attribute (e.g., 3 for Vec3)
    pub stride: i32,    // Stride in bytes between consecutive vertex attributes
    pub offset: i32,    // Offset in bytes of the first component of the attribute
    pub normalized: bool, // Whether integer data should be normalized
}
#[derive(Debug)]
pub enum BoundResource {
    Texture(glow::Texture, u32), // Texture and its texture unit
    Buffer(glow::Buffer, u32),   // Buffer and its binding point (e.g., for UBO/SSBO)
}

pub struct ComputePipe {
    pub program: glow::Program,
}

impl ComputePipe {
    pub fn bind(&self, gl: &glow::Context) {
        unsafe {
            gl.use_program(Some(self.program));
        }
    }
}

pub struct RasterPipe {
    pub program: glow::Program,
    pub vao: glow::VertexArray,
}

impl RasterPipe {
    pub fn bind(&self, gl: &glow::Context) {
        unsafe {
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
        }
    }
}

pub fn create_compute_pipe(gl: &glow::Context, shader_source: &str) -> ComputePipe {
    unsafe {
        let program = gl.create_program().unwrap();

        let shader = gl.create_shader(glow::COMPUTE_SHADER).unwrap();
        gl.shader_source(shader, shader_source);
        gl.compile_shader(shader);

        if !gl.get_shader_compile_status(shader) {
            panic!("{}", gl.get_shader_info_log(shader));
        }

        gl.attach_shader(program, shader);
        gl.link_program(program);

        if !gl.get_program_link_status(program) {
            panic!("{}", gl.get_program_info_log(program));
        }

        gl.detach_shader(program, shader);
        gl.delete_shader(shader);

        ComputePipe { program }
    }
}

pub fn create_raster_pipe(
    gl: &glow::Context,
    vertex_shader_source: Option<&str>,
    fragment_shader_source: Option<&str>,
    vertex_attributes: &[AttrFormOffs],
) -> RasterPipe {
    unsafe {
        let program = gl.create_program().unwrap();
        let mut vs = None;
        let mut fs = None;

        if let Some(vs_source) = vertex_shader_source {
            let shader = gl.create_shader(glow::VERTEX_SHADER).unwrap();
            gl.shader_source(shader, vs_source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                panic!(
                    "Vertex shader compile error: {}",
                    gl.get_shader_info_log(shader)
                );
            }
            gl.attach_shader(program, shader);
            vs = Some(shader);
        }

        if let Some(fs_source) = fragment_shader_source {
            let shader = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
            gl.shader_source(shader, fs_source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                panic!(
                    "Fragment shader compile error: {}",
                    gl.get_shader_info_log(shader)
                );
            }
            gl.attach_shader(program, shader);
            fs = Some(shader);
        }

        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("Program link error: {}", gl.get_program_info_log(program));
        }

        if let Some(shader) = vs {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        if let Some(shader) = fs {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        for attribute in vertex_attributes.iter() {
            let location = gl
                .get_attrib_location(program, &format!("in_binding{}", attribute.binding))
                .unwrap();
            gl.enable_vertex_attrib_array(location as u32);

            match attribute.data_type {
                glow::FLOAT | glow::HALF_FLOAT => {
                    gl.vertex_attrib_pointer_f32(
                        location as u32,
                        attribute.size,
                        attribute.data_type,
                        attribute.normalized,
                        attribute.stride,
                        attribute.offset,
                    );
                }
                glow::BYTE
                | glow::UNSIGNED_BYTE
                | glow::SHORT
                | glow::UNSIGNED_SHORT
                | glow::INT
                | glow::UNSIGNED_INT => {
                    gl.vertex_attrib_pointer_i32(
                        location as u32,
                        attribute.size,
                        attribute.data_type,
                        attribute.stride,
                        attribute.offset,
                    );
                }
                _ => panic!(),
            }
        }

        gl.bind_vertex_array(None);

        RasterPipe { program, vao }
    }
}
