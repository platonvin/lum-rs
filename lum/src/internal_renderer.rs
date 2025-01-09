use std::ptr;

use vek::FrustumPlanes;
use vk::{AccessFlags, DeviceV1_0, Image, ImageLayout, PipelineBindPoint, PipelineStageFlags};

use crate::*;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    camera_pos: vec3,
    camera_dir: vec3,
    camera_transform: mat4,
    pixels_in_voxel: f32,
    origin_view_size: vec2,
    view_size: vec2,
    camera_ray_dir_plane: vec3,
    horizline: vec3,
    vertiline: vec3,
}
impl Default for Camera {
    fn default() -> Self {
        let origin_view_size = vec2::new(1920.0, 1080.0);
        let pixels_in_voxel = 5.0;
        let camera_dir = vec3::new(0.61, 1.0, -0.8).normalized();
        let camera_ray_dir_plane = vec3::new(camera_dir.x, camera_dir.y, 0.0).normalized();
        let horizline = camera_ray_dir_plane.cross(vec3::new(0.0, 0.0, 1.0)).normalized();

        Self {
            camera_pos: vec3::new(60.0, 0.0, 194.0),
            camera_dir,
            camera_transform: Default::default(),
            pixels_in_voxel,
            origin_view_size,
            view_size: origin_view_size / pixels_in_voxel,
            camera_ray_dir_plane,
            horizline,   
            vertiline: horizline.cross(camera_dir).normalized(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SunLight {
    light_transform: mat4,
    light_dir: vec3,
}
impl Default for SunLight {
    fn default() -> Self {
        Self {
            light_transform: Default::default(),
            light_dir: vec3::new(0.5, 0.5, -0.9).normalized(),
        }
    }
}

impl Camera {
    fn update_camera(&mut self) {
        let up = vec3::new(0.0, 0.0, 1.0); // Up vector
        self.view_size = self.origin_view_size / self.pixels_in_voxel;
        let view = mat4::look_at_lh(self.camera_pos, self.camera_pos + self.camera_dir, up);
        let projection = mat4::orthographic_lh_no(FrustumPlanes {
            left: -self.view_size.x / 2.0,
            right: self.view_size.x / 2.0,
            bottom: self.view_size.y / 2.0,
            top: -self.view_size.y / 2.0,
            near: -0.0,
            far: 2000.0,
        }); // => *(2000.0/2) for decoding
        self.camera_transform = projection * view;
        self.camera_ray_dir_plane = vec3::new(self.camera_dir.x, self.camera_dir.y, 0.0);
        vec3::normalize(&mut self.camera_ray_dir_plane);

        self.horizline = self
            .camera_ray_dir_plane
            .cross(vec3::new(0.0, 0.0, 1.0))
            .normalized();
    }
}

impl SunLight {
    pub fn update_light_transform(&mut self, world_size: uvec3) {
        let horizon = vec3::new(1.0, 0.0, 0.0).normalized();
        let up = vec3::new(0.0, 0.0, 1.0).normalized();
        let light_pos = vec3::new(f32::from((world_size.x * (16 as u32)) as i16), f32::from((world_size.y * (16 as u32)) as i16), 0.0) / 2.0 - 1.0 * 16.0 * self.light_dir;  
        let view = mat4::look_at_lh(light_pos, light_pos + self.light_dir, up);
        let voxel_in_pixels = 5.0;
        let view_width_in_voxels = 3000.0 / voxel_in_pixels;
        let view_height_in_voxels = 3000.0 / voxel_in_pixels;
        let projection = mat4::orthographic_lh_no(FrustumPlanes {
            left: -view_width_in_voxels  / 2.0,
            right: view_width_in_voxels  / 2.0,
            bottom: view_height_in_voxels / 2.0,
            top: -view_height_in_voxels / 2.0,
            near: -0.0,
            far: 2000.0,
        });
    }
}

impl crate::LumRenderer {
    pub fn update_camera(&mut self) {
        self.camera.update_camera();
    }

    pub fn update_light_transform(&mut self) {
        self.light.update_light_transform(self.settings.world_size);
        // let horizon =
    }

    pub fn gen_perlin_2d(&mut self) {
        let lumal = &mut self.lumal;

        let mut cmb = lumal.begin_single_time_command_buffer();

        let pipe = &self.pipes.gen_perlin2d_pipe;

        lumal.bind_compute_pipe(&mut cmb, pipe);

        // bind sets
        // place barriers
        // dispatch the perlin noise compute shader
        assert!(pipe.sets.len() != 0);
        for frame_i in 0..FRAMES_IN_FLIGHT {
            unsafe {
                lumal.device.cmd_bind_descriptor_sets(
                    cmb,
                    PipelineBindPoint::COMPUTE,
                    pipe.line_layout,
                    0,
                    &[pipe.sets[frame_i]],
                    &[],
                );

                lumal.image_memory_barrier(
                    cmb,
                    self.independent_images.perlin_noise2d.current(),
                    PipelineStageFlags::ALL_COMMANDS,
                    PipelineStageFlags::ALL_COMMANDS,
                    AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE, // im lazy so all memory read|write's wait for all read|write's
                    AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE, // using proper barriers increases performance, but not much
                    ImageLayout::UNDEFINED, // => transfer it from UNDEFED to GENERAL
                    ImageLayout::GENERAL, // if this was SHADER_READ_ONLY it would mean that we transfer from UNDEFINED to SHADER_READ_ONLY
                );

                lumal.device.cmd_dispatch(
                    cmb,
                    self.settings.world_size.x / 8, // Divide by 8 because we use 8x8 "local_size" - the kernel size - the local workgroup size
                    self.settings.world_size.y / 8, // typically people use 64 threads (for different reason)
                    1,
                );

                lumal.image_memory_barrier(
                    cmb,
                    self.independent_images.perlin_noise2d.current(),
                    PipelineStageFlags::ALL_COMMANDS,
                    PipelineStageFlags::ALL_COMMANDS,
                    AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
                    AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
                    ImageLayout::GENERAL, // from GENERAL to GENERAL which means no layout transfer, just the {execution} barrier
                    ImageLayout::GENERAL,
                );

                self.independent_images.perlin_noise2d.move_next();
            }
        }

        lumal.end_single_time_command_buffer(cmb);
    }
    pub fn gen_perlin_3d(&mut self) {
        let lumal = &mut self.lumal;

        let mut cmb = lumal.begin_single_time_command_buffer();

        let pipe = &self.pipes.gen_perlin3d_pipe;

        lumal.bind_compute_pipe(&mut cmb, pipe);

        // bind sets
        // place barriers
        // dispatch the perlin noise compute shader
        assert!(pipe.sets.len() != 0);
        for frame_i in 0..FRAMES_IN_FLIGHT {
            unsafe {
                lumal.device.cmd_bind_descriptor_sets(
                    cmb,
                    PipelineBindPoint::COMPUTE,
                    pipe.line_layout,
                    0,
                    &[pipe.sets[frame_i]],
                    &[],
                );

                lumal.image_memory_barrier(
                    cmb,
                    self.independent_images.perlin_noise3d.current(),
                    PipelineStageFlags::ALL_COMMANDS,
                    PipelineStageFlags::ALL_COMMANDS,
                    AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE, // im lazy so all memory read|write's wait for all read|write's
                    AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE, // using proper barriers increases performance, but not much
                    ImageLayout::UNDEFINED, // => transfer it from UNDEFED to GENERAL
                    ImageLayout::GENERAL, // if this was SHADER_READ_ONLY it would mean that we transfer from UNDEFINED to SHADER_READ_ONLY
                );

                lumal.device.cmd_dispatch(
                    cmb,
                    64 / 4, // 64 is just the chosen size
                    64 / 4, // kernel is 4x4x4
                    64 / 4,
                );

                lumal.image_memory_barrier(
                    cmb,
                    self.independent_images.perlin_noise3d.current(),
                    PipelineStageFlags::ALL_COMMANDS,
                    PipelineStageFlags::ALL_COMMANDS,
                    AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
                    AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
                    ImageLayout::GENERAL,
                    ImageLayout::GENERAL,
                );

                self.independent_images.perlin_noise3d.move_next();
            }
        }

        lumal.end_single_time_command_buffer(cmb);
    }

    pub fn start_frame(&mut self) {
        // self.lumal.start_frame(&[&self.);
    }
}
