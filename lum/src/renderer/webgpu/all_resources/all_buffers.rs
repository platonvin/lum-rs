use std::mem;

use wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

use crate::renderer::{
    types::{i8vec4, ivec4, mat4, AoLut},
    webgpu::{
        types::{BlockId, Particle},
        wal::Wal,
        AllBuffers, InternalRendererWebGPU,
    },
    Settings,
};

use super::all_types::UboData;
// use internal_renderer::{InternalRendererVulkan, *};

impl<'window> InternalRendererWebGPU<'window> {
    #[cold]
    #[optimize(size)]
    pub fn create_all_buffers(wal: &mut Wal, lum_settings: &Settings) -> AllBuffers {
        let gpu_particles = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::VERTEX,
            (lum_settings.max_particle_count as usize) * mem::size_of::<Particle>(),
            false,
            Some("Particles"),
        );
        let uniform = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            size_of::<UboData>(), // pre-calculated size of UBO. No way i write it with mem::size_of::<
            false,                // if should be visible to CPU
            Some("Uniform"),
        );
        let light_uniform = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mem::size_of::<mat4>(),
            false,
            Some("Light Uniform"),
        );
        let ao_lut_uniform = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mem::size_of::<AoLut>() * 8,
            false,
            Some("AO LUT Uniform"),
        ); // TODO DYNAMIC AO SAMPLE COUNT
        let gpu_radiance_updates = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mem::size_of::<i8vec4>()
                * (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize),
            false,
            Some("Radiance Updates"),
        ); // TODO test extra mem
        let staging_radiance_updates = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            mem::size_of::<ivec4>()
                * (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize),
            false,
            Some("Staging Radiance Updates"),
        ); // TODO test extra mem

        let padded_x_size =
            lum_settings.world_size.x.next_multiple_of(
                COPY_BYTES_PER_ROW_ALIGNMENT / std::mem::size_of::<BlockId>() as u32,
            ) as usize;
        let padded_staging_world_size = padded_x_size
            * lum_settings.world_size.y as usize
            * lum_settings.world_size.z as usize
            * std::mem::size_of::<BlockId>();

        let staging_world = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            padded_staging_world_size,
            false,
            Some("Staging World"),
        );

        let gpu_particles_staged = wal.create_buffer_rings(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            (lum_settings.max_particle_count as usize) * mem::size_of::<Particle>(),
            false,
            Some("Particles Staged"),
        );

        AllBuffers {
            staging_world,
            light_uniform,
            uniform: uniform,
            ao_lut_uniform,
            gpu_radiance_updates,
            // staging_radiance_updates,
            gpu_particles,
            gpu_particles_staged,
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
        // wal.destroy_buffer_ring(buffers.staging_radiance_updates);
        wal.destroy_buffer_ring(buffers.gpu_particles);
        println!("destroyed buffers");
    }
}
