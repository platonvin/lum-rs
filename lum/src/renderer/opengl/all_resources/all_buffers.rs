use std::mem;

use glow::HasContext;

use crate::{
    internal_renderer::{
        render_gl::{AllBuffers, InternalRendererGL},
        Settings,
    },
    types::{i8vec4, ivec4, mat4, AoLut, BlockId, Particle},
};

impl InternalRendererGL {
    fn create_buffer(
        gl: &glow::Context,
        target: u32,
        usage_hint: u32, // e.g., glow::STATIC_DRAW, glow::DYNAMIC_DRAW
        size: usize,     // in bytes
    ) -> glow::Buffer {
        unsafe {
            let buffer = gl.create_buffer().expect("Failed to create buffer");
            gl.bind_buffer(target, Some(buffer));
            gl.buffer_data_size(target, size as i32, usage_hint);
            buffer
        }
    }

    #[cold]
    #[optimize(size)]
    pub fn create_all_buffers(gl: &glow::Context, lum_settings: &Settings) -> AllBuffers {
        let gpu_particles = Self::create_buffer(
            gl,
            glow::ARRAY_BUFFER,
            glow::DYNAMIC_DRAW,
            (lum_settings.max_particle_count as usize) * std::mem::size_of::<Particle>(),
        );
        let uniform = Self::create_buffer(gl, glow::UNIFORM_BUFFER, glow::DYNAMIC_DRAW, 220);
        let light_uniform = Self::create_buffer(
            gl,
            glow::UNIFORM_BUFFER,
            glow::DYNAMIC_DRAW,
            std::mem::size_of::<mat4>(),
        );
        let ao_lut_uniform = Self::create_buffer(
            gl,
            glow::UNIFORM_BUFFER,
            glow::STATIC_DRAW,
            std::mem::size_of::<AoLut>() * 8,
        );
        let gpu_radiance_updates = Self::create_buffer(
            gl,
            glow::SHADER_STORAGE_BUFFER,
            glow::DYNAMIC_DRAW,
            std::mem::size_of::<i8vec4>()
                * (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize),
        );
        let staging_radiance_updates = Self::create_buffer(
            gl,
            glow::COPY_WRITE_BUFFER,
            glow::DYNAMIC_DRAW,
            std::mem::size_of::<ivec4>()
                * (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize),
        );
        let staging_world = Self::create_buffer(
            gl,
            glow::COPY_WRITE_BUFFER,
            glow::DYNAMIC_DRAW,
            (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize)
                * std::mem::size_of::<BlockId>(),
        );
        AllBuffers {
            staging_world,
            light_uniform,
            uniform,
            ao_lut_uniform,
            gpu_radiance_updates,
            staging_radiance_updates,
            gpu_particles,
        }
    }

    #[cold]
    #[optimize(size)]
    pub fn destroy_all_buffers(gl: &glow::Context, buffers: AllBuffers) {
        println!("started destroying buffers");
        unsafe {
            gl.delete_buffer(buffers.staging_world);
            gl.delete_buffer(buffers.light_uniform);
            gl.delete_buffer(buffers.uniform);
            gl.delete_buffer(buffers.ao_lut_uniform);
            gl.delete_buffer(buffers.gpu_radiance_updates);
            gl.delete_buffer(buffers.staging_radiance_updates);
            gl.delete_buffer(buffers.gpu_particles);
        }
        println!("destroyed buffers");
    }
}
