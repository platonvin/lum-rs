use containers::array3d::Dim3;
use lumal::vk;

impl<'a, D: Dim3> super::InternalRendererVulkan<'a, D> {
    pub fn gen_perlin_2d(&mut self) {
        let cmb = self.lumal.begin_single_time_command_buffer();

        let pipe = &self.pipes.gen_perlin2d_pipe;

        self.lumal.bind_compute_pipe(&cmb, pipe);

        // bind sets
        // place barriers
        // dispatch the perlin noise compute shader
        assert!(!pipe.sets.is_empty());
        unsafe {
            self.lumal.device.cmd_bind_descriptor_sets(
                cmb,
                vk::PipelineBindPoint::COMPUTE,
                pipe.line_layout,
                0,
                &[*pipe.sets.first()], // does not matter which one we bind, they all point to the same resource
                &[],
            );

            self.lumal.image_memory_barrier(
                &cmb,
                &self.independent_images.perlin_noise2d,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE, // im lazy so all memory read|write's wait for all read|write's
                vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE, // using proper barriers increases performance, but not much
                vk::ImageLayout::UNDEFINED, // => transfer it from UNDEFED to GENERAL
                vk::ImageLayout::GENERAL, // if this was SHADER_READ_ONLY it would mean that we transfer from UNDEFINED to SHADER_READ_ONLY
            );

            self.lumal.device.cmd_dispatch(
                cmb,
                self.settings.world_size.x() as u32 / 8, // Divide by 8 because we use 8x8 "local_size" - the kernel size - the local workgroup size
                self.settings.world_size.y() as u32 / 8, // typically people use 64 threads (for different reason)
                1,
            );

            self.lumal.image_memory_barrier(
                &cmb,
                &self.independent_images.perlin_noise2d,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
                vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
                vk::ImageLayout::GENERAL, // from GENERAL to GENERAL which means no layout transfer, just the {execution} barrier
                vk::ImageLayout::GENERAL,
            );
        }

        self.lumal.end_single_time_command_buffer(cmb);
    }

    pub fn gen_perlin_3d(&mut self) {
        let lumal = &mut self.lumal;

        let cmb = lumal.begin_single_time_command_buffer();

        let pipe = &self.pipes.gen_perlin3d_pipe;

        lumal.bind_compute_pipe(&cmb, pipe);

        // bind sets
        // place barriers
        // dispatch the perlin noise compute shader
        assert!(!pipe.sets.is_empty());
        unsafe {
            lumal.device.cmd_bind_descriptor_sets(
                cmb,
                vk::PipelineBindPoint::COMPUTE,
                pipe.line_layout,
                0,
                &[*pipe.sets.first()],
                &[],
            );

            lumal.image_memory_barrier(
                &cmb,
                &self.independent_images.perlin_noise3d,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE, // im lazy so all memory read|write's wait for all read|write's
                vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE, // using proper barriers increases performance, but not much
                vk::ImageLayout::UNDEFINED, // => transfer it from UNDEFED to GENERAL
                vk::ImageLayout::GENERAL, // if this was SHADER_READ_ONLY it would mean that we transfer from UNDEFINED to SHADER_READ_ONLY
            );

            lumal.device.cmd_dispatch(
                cmb,
                64 / 4, // 64 is just the chosen size
                64 / 4, // kernel is 4x4x4
                64 / 4,
            );

            lumal.image_memory_barrier(
                &cmb,
                &self.independent_images.perlin_noise3d,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
                vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::GENERAL,
            );
        }

        lumal.end_single_time_command_buffer(cmb);
    }
}
