use std::mem;

use crate::{
    internal_renderer::{
        render_wgpu::{wal::Wal, AllBuffers, InternalRendererWebGPU},
        Settings,
    },
    types::{i8vec4, ivec4, mat4, AoLut, BlockId, Particle},
};
// use internal_renderer::{InternalRendererVulkan, *};

impl<'window> InternalRendererWebGPU<'window> {
    #[cold]
    #[optimize(size)]
    pub fn create_all_buffers(wal: &mut Wal, lum_settings: &Settings) -> AllBuffers {
        let gpu_particles = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::VERTEX,
            (lum_settings.max_particle_count as usize) * mem::size_of::<Particle>(),
            true,
        );
        let uniform = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            220,   // pre-calculated size of UBO. No way i write it with mem::size_of::<
            false, // if should be visible to CPU
        );
        let light_uniform = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mem::size_of::<mat4>(),
            false,
        );
        let ao_lut_uniform = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mem::size_of::<AoLut>() * 8,
            false,
        ); // TODO DYNAMIC AO SAMPLE COUNT
        let gpu_radiance_updates = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mem::size_of::<i8vec4>()
                * (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize),
            false,
        ); // TODO test extra mem
        let staging_radiance_updates = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::COPY_SRC,
            mem::size_of::<ivec4>()
                * (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize),
            true,
        ); // TODO test extra mem

        let staging_world = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::COPY_SRC,
            (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize)
                * mem::size_of::<BlockId>(),
            true,
        );
        AllBuffers {
            staging_world: staging_world,
            light_uniform: light_uniform,
            uniform: uniform,
            ao_lut_uniform: ao_lut_uniform,
            gpu_radiance_updates: gpu_radiance_updates,
            staging_radiance_updates: staging_radiance_updates,
            gpu_particles: gpu_particles,
        }
    }

    #[cold]
    #[optimize(size)]
    pub fn destroy_all_buffers(wal: &mut Wal, buffers: AllBuffers) {
        println!("started destroying buffers");
        wal.destroy_buffer_ring(buffers.staging_world);
        wal.destroy_buffer_ring(buffers.light_uniform);
        wal.destroy_buffer_ring(buffers.uniform);
        wal.destroy_buffer_ring(buffers.ao_lut_uniform);
        wal.destroy_buffer_ring(buffers.gpu_radiance_updates);
        wal.destroy_buffer_ring(buffers.staging_radiance_updates);
        wal.destroy_buffer_ring(buffers.gpu_particles);
        println!("destroyed buffers");
    }
}
