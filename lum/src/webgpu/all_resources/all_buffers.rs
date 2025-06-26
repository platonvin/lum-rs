use super::all_types::UboData;
use crate::Dim3;
use crate::{
    types::{i8vec4, mat4, AoLut, Particle},
    webgpu::{types::InternalBlockId, wal::Wal, AllBuffers, InternalRendererWebGPU},
    Settings,
};
use std::mem;
use wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

impl<'window, D: Dim3> InternalRendererWebGPU<'window, D> {
    pub fn create_all_buffers(wal: &mut Wal, lum_settings: &Settings<D>) -> AllBuffers {
        let gpu_particles = wal.create_buffers(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            (lum_settings.max_particle_count as usize) * mem::size_of::<Particle>(),
            Some("Particles"),
        );
        let uniform = wal.create_buffers(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            size_of::<UboData>(), // pre-calculated size of UBO. No way i write it with mem::size_of::<
            // if should be visible to CPU
            Some("Uniform"),
        );
        let light_uniform = wal.create_buffers(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mem::size_of::<mat4>(),
            Some("Light Uniform"),
        );
        let ao_lut_uniform = wal.create_buffers(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mem::size_of::<AoLut>() * 8,
            Some("AO LUT Uniform"),
        ); // TODO DYNAMIC AO SAMPLE COUNT
        let gpu_radiance_updates = wal.create_buffers(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mem::size_of::<i8vec4>()
                * (lum_settings.world_size.x() as usize)
                * (lum_settings.world_size.y() as usize)
                * (lum_settings.world_size.z() as usize),
            Some("Radiance Updates"),
        ); // TODO test extra mem

        let padded_x_size = lum_settings.world_size.x().next_multiple_of(
            COPY_BYTES_PER_ROW_ALIGNMENT as usize / std::mem::size_of::<InternalBlockId>(),
        );
        let padded_staging_world_size = padded_x_size
            * lum_settings.world_size.y() as usize
            * lum_settings.world_size.z() as usize
            * std::mem::size_of::<InternalBlockId>();

        let staging_world = wal.create_buffers(
            wal.config.desired_maximum_frame_latency as usize,
            wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            padded_staging_world_size,
            Some("Staging World"),
        );

        AllBuffers {
            staging_world,
            light_uniform,
            uniform,
            ao_lut_uniform,
            gpu_radiance_updates,
            gpu_particles,
        }
    }
}
