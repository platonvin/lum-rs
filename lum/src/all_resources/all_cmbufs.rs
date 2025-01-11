use lumal::{descriptors::{DescriptorInfo, RelativeDescriptorPos, ShortDescriptorInfo}, ring::Ring, ComputePipe, LumalRenderer, LumalSettings, RasterPipe};
use vulkanalia::vk::{self, DeviceV1_0, Extent2D, Handle};
use vk::Sampler;
use RelativeDescriptorPos::*;

use crate::*;

impl crate::LumRenderer {
    pub fn create_all_command_buffers(lumal: &LumalRenderer, lum_settings: &LumSettings, lumal_settings: &LumalSettings) -> LumCommandBuffers {
        let compute_command_buffers = lumal.create_command_buffer();
        let lightmap_command_buffers = lumal.create_command_buffer();
        let graphics_command_buffers = lumal.create_command_buffer();
        let copy_command_buffers = lumal.create_command_buffer();

        return LumCommandBuffers {
            compute_command_buffers,
            lightmap_command_buffers,
            graphics_command_buffers,
            copy_command_buffers,
        };
    }

    pub fn destroy_all_command_buffers(lumal: &LumalRenderer, command_buffers: &LumCommandBuffers) {
        lumal.destroy_command_buffer(&command_buffers.compute_command_buffers);
        lumal.destroy_command_buffer(&command_buffers.lightmap_command_buffers);
        lumal.destroy_command_buffer(&command_buffers.graphics_command_buffers);
        lumal.destroy_command_buffer(&command_buffers.copy_command_buffers);
    }
}