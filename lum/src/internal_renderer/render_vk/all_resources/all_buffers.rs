use std::mem;

use crate::{
    internal_renderer::{
        render_vk::{AllBuffers, InternalRendererVulkan},
        Settings,
    },
    types::{i8vec4, ivec4, mat4, AoLut, BlockId, Particle},
};
// use internal_renderer::{InternalRendererVulkan, *};
use lumal::vk;
use lumal::{ring::Ring, set_debug_names, LumalSettings, Renderer};

impl InternalRendererVulkan {
    #[cold]
    #[optimize(size)]
    pub fn create_all_buffers(
        lumal: &mut Renderer,
        lum_settings: &Settings,
        lumal_settings: &LumalSettings,
    ) -> AllBuffers {
        let gpu_particles = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            (lum_settings.max_particle_count as usize) * mem::size_of::<Particle>(),
            true,
        );
        let uniform = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            220,   // pre-calculated size of UBO. No way i write it with mem::size_of::<
            false, // if should be visible to CPU
        );
        let light_uniform = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<mat4>(),
            false,
        );
        let ao_lut_uniform = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<AoLut>() * 8,
            false,
        ); // TODO DYNAMIC AO SAMPLE COUNT
        let gpu_radiance_updates = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<i8vec4>()
                * (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize),
            false,
        ); // TODO test extra mem
        let staging_radiance_updates = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_SRC,
            mem::size_of::<ivec4>()
                * (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize),
            true,
        ); // TODO test extra mem

        let staging_world = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_SRC,
            (lum_settings.world_size.x as usize)
                * (lum_settings.world_size.y as usize)
                * (lum_settings.world_size.z as usize)
                * mem::size_of::<BlockId>(),
            true,
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
    pub fn destroy_all_buffers(lumal: &mut Renderer, buffers: AllBuffers) {
        println!("started destroying buffers");
        lumal.destroy_buffer_ring(buffers.staging_world);
        lumal.destroy_buffer_ring(buffers.light_uniform);
        lumal.destroy_buffer_ring(buffers.uniform);
        lumal.destroy_buffer_ring(buffers.ao_lut_uniform);
        lumal.destroy_buffer_ring(buffers.gpu_radiance_updates);
        lumal.destroy_buffer_ring(buffers.staging_radiance_updates);
        lumal.destroy_buffer_ring(buffers.gpu_particles);
        println!("destroyed buffers");
    }
}
