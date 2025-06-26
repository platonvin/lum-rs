use crate::{
    types::{i8vec4, ivec4, AoLut, Particle},
    vulkan::{
        pc_types::{self, LightmapUBO},
        types::InternalBlockId,
        AllBuffers, InternalRendererVulkan, AO_LUT_SIZE,
    },
    Settings,
};
use containers::array3d::Dim3;
use lumal::{vk, LumalSettings, Renderer};
use std::mem;

impl<'a, D: Dim3> InternalRendererVulkan<'a, D> {
    /// Creates a bundle of all static buffers.
    pub fn create_all_buffers(
        lumal: &mut Renderer,
        lum_settings: &Settings<D>,
        lumal_settings: &LumalSettings,
    ) -> AllBuffers {
        let gpu_particles = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            (lum_settings.max_particle_count as usize) * mem::size_of::<Particle>(),
            true, // if should be visible to CPU (not the best memory for rendering, but still. TODO:)
        );
        let uniform = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            size_of::<pc_types::UBO>(),
            false,
        );
        let light_uniform = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<LightmapUBO>(),
            false,
        );
        let ao_lut_uniform = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<AoLut>() * AO_LUT_SIZE,
            false,
        ); // TODO DYNAMIC AO SAMPLE COUNT
        let gpu_radiance_updates = lumal.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            // we allocate enough to update entire world to not deal with reallocations
            mem::size_of::<i8vec4>()
                * (lum_settings.world_size.x() as usize)
                * (lum_settings.world_size.y() as usize)
                * (lum_settings.world_size.z() as usize),
            false, // we want this memory to be fast
        );
        let staging_radiance_updates = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_SRC,
            // we allocate enough to update entire world to not deal with reallocations
            mem::size_of::<ivec4>()
                * (lum_settings.world_size.x() as usize)
                * (lum_settings.world_size.y() as usize)
                * (lum_settings.world_size.z() as usize),
            true, // cause staging memory, this is what we write on CPU
        );

        let staging_world = lumal.create_buffer_rings(
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_SRC,
            // just size of world image
            (lum_settings.world_size.x() as usize)
                * (lum_settings.world_size.y() as usize)
                * (lum_settings.world_size.z() as usize)
                * mem::size_of::<InternalBlockId>(),
            true, // cause staging memory, this is what we write on CPU
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

    /// Destroys the bundle of all buffers.
    pub fn destroy_all_buffers(lumal: &mut Renderer, buffers: AllBuffers) {
        lumal.destroy_buffer_ring(buffers.staging_world);
        lumal.destroy_buffer_ring(buffers.light_uniform);
        lumal.destroy_buffer_ring(buffers.uniform);
        lumal.destroy_buffer_ring(buffers.ao_lut_uniform);
        lumal.destroy_buffer(buffers.gpu_radiance_updates);
        lumal.destroy_buffer_ring(buffers.staging_radiance_updates);
        lumal.destroy_buffer_ring(buffers.gpu_particles);
    }
}
