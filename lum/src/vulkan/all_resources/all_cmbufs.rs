use crate::{
    vulkan::{AllCommandBuffers, InternalRendererVulkan},
    Settings,
};
use containers::array3d::Dim3;
use lumal::{LumalSettings, Renderer};

impl<'a, D: Dim3> InternalRendererVulkan<'a, D> {
    /// Creates all command buffer.
    pub fn create_all_command_buffers(
        lumal: &Renderer,
        _lum_settings: &Settings<D>,
        _lumal_settings: &LumalSettings,
    ) -> AllCommandBuffers {
        let compute_command_buffers = lumal.create_command_buffer();
        let lightmap_command_buffers = lumal.create_command_buffer();
        let graphics_command_buffers = lumal.create_command_buffer();
        let copy_command_buffers = lumal.create_command_buffer();

        AllCommandBuffers {
            compute_command_buffers,
            lightmap_command_buffers,
            graphics_command_buffers,
            copy_command_buffers,
        }
    }

    /// Destroys all command buffer.
    pub fn destroy_all_command_buffers(lumal: &Renderer, command_buffers: &AllCommandBuffers) {
        lumal.destroy_command_buffer(&command_buffers.compute_command_buffers);
        lumal.destroy_command_buffer(&command_buffers.lightmap_command_buffers);
        lumal.destroy_command_buffer(&command_buffers.graphics_command_buffers);
        lumal.destroy_command_buffer(&command_buffers.copy_command_buffers);
    }
}
