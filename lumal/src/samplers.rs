use crate::Renderer;
use ash::vk;

impl Renderer {
    pub fn create_sampler(&self, sampler_info: &vk::SamplerCreateInfo) -> vk::Sampler {
        unsafe { self.device.create_sampler(sampler_info, None) }.unwrap()
    }

    pub fn destroy_sampler(&self, sampler: vk::Sampler) {
        unsafe { self.device.destroy_sampler(sampler, None) };
    }
}
