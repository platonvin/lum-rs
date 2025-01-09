use lumal::{descriptors::{DescriptorInfo, RelativeDescriptorPos, ShortDescriptorInfo}, ring::Ring, ComputePipe, LumalRenderer, LumalSettings, RasterPipe};
use vulkanalia::vk::{self, Extent2D, Handle};
use vk::Sampler;
use RelativeDescriptorPos::*;

use crate::*;


impl crate::LumRenderer {
    pub fn create_all_buffers(lumal: &LumalRenderer, lum_settings: &LumSettings, lumal_settings: &LumalSettings) -> LumBuffers {
        let gpu_particles = lumal.create_buffer_rings (
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            (lum_settings.max_particle_count as usize) * mem::size_of::<Particle>(), 
            true);
        let uniform = lumal.create_buffer_rings (
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            220, 
            false); //no way i write it with mem::size_of::<
        let light_uniform = lumal.create_buffer_rings (
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<mat4>(), 
            false);
        let ao_lut_uniform = lumal.create_buffer_rings (
            lumal_settings.fif,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<AoLut>() * 8, 
            false); //TODO DYNAMIC AO SAMPLE COUNT
        let gpu_radiance_updates = lumal.create_buffer_rings (
            lumal_settings.fif,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            mem::size_of::<i8vec4>()*
                (lum_settings.world_size.x as usize) * 
                (lum_settings.world_size.y as usize) * 
                (lum_settings.world_size.z as usize), 
            false); //TODO test extra mem
        let staging_radiance_updates = lumal.create_buffer_rings (
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_SRC,
            mem::size_of::<ivec4>() as usize*
                (lum_settings.world_size.x as usize) * 
                (lum_settings.world_size.y as usize) * 
                (lum_settings.world_size.z as usize), 
            true); //TODO test extra mem

        let staging_world = lumal.create_buffer_rings (
            lumal_settings.fif,
            vk::BufferUsageFlags::TRANSFER_SRC,
                    (lum_settings.world_size.x as usize) * 
                    (lum_settings.world_size.y as usize) * 
                    (lum_settings.world_size.z as usize) *  
            (mem::size_of::<BlockID_t>() as usize), true);
        return LumBuffers {
            staging_world: staging_world.unwrap(),
            light_uniform: light_uniform.unwrap(),
            uniform: uniform.unwrap(),
            ao_lut_uniform: ao_lut_uniform.unwrap(),
            gpu_radiance_updates: gpu_radiance_updates.unwrap(),
            staging_radiance_updates: staging_radiance_updates.unwrap(),
            gpu_particles: gpu_particles.unwrap(),
        };
    }

    pub fn destroy_all_buffers(&self) {
        println!("started destroying buffers");
        self.lumal.destroy_buffer_ring(&self.buffers.staging_world);
        self.lumal.destroy_buffer_ring(&self.buffers.light_uniform);
        self.lumal.destroy_buffer_ring(&self.buffers.uniform);
        self.lumal.destroy_buffer_ring(&self.buffers.ao_lut_uniform);
        self.lumal.destroy_buffer_ring(&self.buffers.gpu_radiance_updates);
        self.lumal.destroy_buffer_ring(&self.buffers.staging_radiance_updates);
        self.lumal.destroy_buffer_ring(&self.buffers.gpu_particles);
        println!("destroyed buffers");
    }
}