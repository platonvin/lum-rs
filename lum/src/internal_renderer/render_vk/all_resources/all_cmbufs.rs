use crate::{
    internal_renderer::{
        render_vk::{
            AllCommandBuffers, AllIndependentImages, AllSwapchainDependentImages,
            InternalRendererVulkan, BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y,
            CHOSEN_DEPTH_FORMAT, FRAME_FORMAT, LIGHTMAPS_FORMAT, MATNORM_FORMAT, RADIANCE_FORMAT,
            SECONDARY_DEPTH_FORMAT,
        },
        Settings,
    },
    types::{uvec3, uvec3_to_extent3d},
    *,
};
// use internal_renderer::{InternalRendererVulkan, *};
use lumal::vk;
use lumal::{ring::Ring, set_debug_names, LumalSettings, Renderer};

impl InternalRendererVulkan {
    #[cold]
    #[optimize(size)]
    pub fn create_all_command_buffers(
        lumal: &Renderer,
        _lum_settings: &Settings,
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

    #[cold]
    #[optimize(size)]
    pub fn destroy_all_command_buffers(lumal: &Renderer, command_buffers: &AllCommandBuffers) {
        lumal.destroy_command_buffer(&command_buffers.compute_command_buffers);
        lumal.destroy_command_buffer(&command_buffers.lightmap_command_buffers);
        lumal.destroy_command_buffer(&command_buffers.graphics_command_buffers);
        lumal.destroy_command_buffer(&command_buffers.copy_command_buffers);
    }
}
