use crate::{ring::Ring, Buffer, Renderer}; // Import the LumalRenderer struct
use ash::vk;
use std::ptr;

impl Renderer {
    #[cold]
    #[optimize(size)]
    pub fn create_sampler(&self, sampler_info: &vk::SamplerCreateInfo) -> vk::Sampler {
        unsafe { self.device.create_sampler(sampler_info, None) }.unwrap()
    }

    #[cold]
    #[optimize(size)]
    pub fn destroy_sampler(&self, sampler: vk::Sampler) {
        unsafe { self.device.destroy_sampler(sampler, None) };
    }
}
