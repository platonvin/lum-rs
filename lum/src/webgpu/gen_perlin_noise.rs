// use lumal::vk;

// use crate::{internal_/*renderer::*/render_wgpu::FRAMES_IN_FLIGHT, *};

// impl super::InternalRendererVulkan {
//
//
//     pub fn gen_perlin_2d(&mut self) {
//         let lumal = &mut self.lumal;

//         let cmb = lumal.begin_single_time_command_buffer();

//         let pipe = &self.pipes.gen_perlin2d_pipe;

//         lumal.bind_compute_pipe(&cmb, pipe);

//         // bind sets
//         // place barriers
//         // dispatch the perlin noise compute shader
//         assert!(!pipe.sets.is_empty());
//         for frame_i in 0..FRAMES_IN_FLIGHT {
//             unsafe {
//                 lumal.device.cmd_bind_descriptor_sets(
//                     cmb,
//                     vk::PipelineBindPoint::COMPUTE,
//                     pipe.line_layout,
//                     0,
//                     &[pipe.sets[frame_i]],
//                     &[],
//                 );

//                 lumal.image_memory_barrier(
//                     &cmb,
//                     self.independent_images.perlin_noise2d.current(),
//                     vk::PipelineStageFlags::ALL_COMMANDS,
//                     vk::PipelineStageFlags::ALL_COMMANDS,
//                     vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE, // im lazy so all memory read|write's wait for all read|write's
//                     vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE, // using proper barriers increases performance, but not much
//                     vk::ImageLayout::UNDEFINED, // => transfer it from UNDEFED to GENERAL
//                     vk::ImageLayout::GENERAL, // if this was SHADER_READ_ONLY it would mean that we transfer from UNDEFINED to SHADER_READ_ONLY
//                 );

//                 lumal.device.cmd_dispatch(
//                     cmb,
//                     self.settings.world_size.x / 8, // Divide by 8 because we use 8x8 "local_size" - the kernel size - the local workgroup size
//                     self.settings.world_size.y / 8, // typically people use 64 threads (for different reason)
//                     1,
//                 );

//                 lumal.image_memory_barrier(
//                     &cmb,
//                     self.independent_images.perlin_noise2d.current(),
//                     vk::PipelineStageFlags::ALL_COMMANDS,
//                     vk::PipelineStageFlags::ALL_COMMANDS,
//                     vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
//                     vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
//                     vk::ImageLayout::GENERAL, // from GENERAL to GENERAL which means no layout transfer, just the {execution} barrier
//                     vk::ImageLayout::GENERAL,
//                 );

//                 self.independent_images.perlin_noise2d.move_next();
//             }
//         }

//         lumal.end_single_time_command_buffer(cmb);
//     }

//
//
//     pub fn gen_perlin_3d(&mut self) {
//         let lumal = &mut self.lumal;

//         let mut cmb = lumal.begin_single_time_command_buffer();

//         let pipe = &self.pipes.gen_perlin3d_pipe;

//         lumal.bind_compute_pipe(&mut cmb, pipe);

//         // bind sets
//         // place barriers
//         // dispatch the perlin noise compute shader
//         assert!(!pipe.sets.is_empty());
//         for frame_i in 0..FRAMES_IN_FLIGHT {
//             unsafe {
//                 lumal.device.cmd_bind_descriptor_sets(
//                     cmb,
//                     vk::PipelineBindPoint::COMPUTE,
//                     pipe.line_layout,
//                     0,
//                     &[pipe.sets[frame_i]],
//                     &[],
//                 );

//                 lumal.image_memory_barrier(
//                     &cmb,
//                     self.independent_images.perlin_noise3d.current(),
//                     vk::PipelineStageFlags::ALL_COMMANDS,
//                     vk::PipelineStageFlags::ALL_COMMANDS,
//                     vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE, // im lazy so all memory read|write's wait for all read|write's
//                     vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE, // using proper barriers increases performance, but not much
//                     vk::ImageLayout::UNDEFINED, // => transfer it from UNDEFED to GENERAL
//                     vk::ImageLayout::GENERAL, // if this was SHADER_READ_ONLY it would mean that we transfer from UNDEFINED to SHADER_READ_ONLY
//                 );

//                 lumal.device.cmd_dispatch(
//                     cmb,
//                     64 / 4, // 64 is just the chosen size
//                     64 / 4, // kernel is 4x4x4
//                     64 / 4,
//                 );

//                 lumal.image_memory_barrier(
//                     &cmb,
//                     self.independent_images.perlin_noise3d.current(),
//                     vk::PipelineStageFlags::ALL_COMMANDS,
//                     vk::PipelineStageFlags::ALL_COMMANDS,
//                     vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
//                     vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
//                     vk::ImageLayout::GENERAL,
//                     vk::ImageLayout::GENERAL,
//                 );

//                 self.independent_images.perlin_noise3d.move_next();
//             }
//         }

//         lumal.end_single_time_command_buffer(cmb);
//     }
// }
