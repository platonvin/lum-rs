use crate::{assert_assume, containers::BitArray3d, internal_renderer::*};
use std::mem::transmute;

use aabb::{get_shift, iAABB};
use as_u8_slice_derive::AsU8Slice;
// use multiversion::multiversion;
use lumal::vk;
use qvek::{
    i16vec3, i16vec4, i8vec4, ivec3, ivec4, uvec2, uvec3, vec3, vec4,
    vek::{Clamp, FrustumPlanes},
};
use winit::window::Window;

use crate::types::*;

// use super::{aabb, InternalRenderer};

// i am clearly trash with managing division into files
// if someone has a good idea on how to do it, message me (or just make a PR)

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub camera_pos: vec3,
    pub camera_dir: vec3,
    pub camera_transform: mat4,
    pub pixels_in_voxel: f32,
    pub origin_view_size: vec2,
    pub view_size: vec2, // in voxels
    pub camera_ray_dir_plane: vec3,
    pub horizline: vec3,
    pub vertiline: vec3,
}
impl Default for Camera {
    fn default() -> Self {
        let origin_view_size = qvek::vec2!(1920, 1080);
        let pixels_in_voxel = 5.0;
        let camera_dir = vec3!(0.61, 1.0, -0.8).normalized();
        let camera_ray_dir_plane = vec3!(camera_dir.xy(), 0).normalized();
        let horizline = camera_ray_dir_plane.cross(vec3!(0, 0, 1)).normalized();

        Self {
            camera_pos: vec3!(60, 0, 194),
            camera_dir,
            camera_transform: mat4::identity(),
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
            light_dir: vec3!(0.5, 0.5, -0.9).normalized(),
        }
    }
}

impl Camera {
    fn update_camera(&mut self) {
        let up = vec3!(0, 0, 1); // Up vector
        self.view_size = self.origin_view_size / self.pixels_in_voxel;
        // RIGHT HANDED MATH EVERYWHERE
        let view = mat4::look_at_rh(self.camera_pos, self.camera_pos + self.camera_dir, up);
        let projection = mat4::orthographic_rh_no(FrustumPlanes {
            left: -self.view_size.x / 2.0,
            right: self.view_size.x / 2.0,
            bottom: self.view_size.y / 2.0,
            top: -self.view_size.y / 2.0,
            near: -0.0,
            far: 2000.0,
        }); // => *(2000.0/2) for decoding
            // dbg!(&projection);
        self.camera_transform = projection * view;
        self.camera_ray_dir_plane = vec3!(self.camera_dir.xy(), 0).normalized();

        self.horizline = self.camera_ray_dir_plane.cross(vec3!(0, 0, 1)).normalized();

        self.vertiline = self.camera_dir.cross(self.horizline).normalized();
    }
}

impl SunLight {
    fn update_light_transform(&mut self, world_size: uvec3) {
        let _horizon = vec3!(1, 0, 0).normalized();
        let up = vec3!(0, 0, 1).normalized();
        let light_pos = vec3!(world_size.xy() * 16, 0) / 2.0 - (1.0 * 16.0 * self.light_dir);

        let view = mat4::look_at_rh(light_pos, light_pos + self.light_dir, up);
        let voxel_in_pixels = 5.0;
        let view_width_in_voxels = 3000.0 / voxel_in_pixels;
        let view_height_in_voxels = 3000.0 / voxel_in_pixels;
        let projection = mat4::orthographic_rh_no(FrustumPlanes {
            left: -view_width_in_voxels / 2.0,
            right: view_width_in_voxels / 2.0,
            bottom: view_height_in_voxels / 2.0,
            top: -view_height_in_voxels / 2.0,
            near: -512.0,
            far: 1024.0,
        });
        self.light_transform = projection * view;
    }
}

// not over Vulkan, but over Lum needs
// pub trait LumRendererAPI<'window> {
//     type BufferType;
//     type ImageType;

//     async fn new(
//         lum_settings: &Settings,
//         window: &'window Window,
//         foliage_descriptions: Vec<InternalMeshFoliageDesc>,
//     ) -> Self;

//     fn update_camera(&mut self);
//     fn update_light_transform(&mut self);
//     fn start_blockify(&mut self);
//     fn index_block_xy(&self, n: usize) -> uvec2;
//     fn blockify_mesh(
//         &mut self,
//         mesh: &InternalMeshModel<Self::BufferType, Self::ImageType>,
//         trans: &MeshTransform,
//     );
//     fn end_blockify(&mut self);
//     fn find_radiance_to_update(&mut self);
//     fn update_radiance(&mut self);
//     // starts the stage where you can "request drawing" things
//     // under the hood it prepares Vulkan for recording draw calls
//     fn start_frame(&mut self);

//     // fn flush_buffer_memory(&mut self, buffer: &mut Self::Buffer);

//     fn _update_radiance(&mut self);

//     // when shift is zero, no work is done (so dont cache this)
//     fn shift_radiance(&mut self, radiance_shift: ivec3);
//     fn exec_copies(&mut self);
//     fn start_map(&mut self);

//     fn map_mesh(
//         &mut self,
//         mesh: &InternalMeshModel<Self::BufferType, Self::ImageType>,
//         trans: &MeshTransform,
//     );
//     fn end_map(&mut self);

//     fn end_compute(&mut self);

//     fn start_lightmap(&mut self);
//     fn lightmap_start_blocks(&mut self);

//     fn lightmap_start_models(&mut self);

//     fn end_lightmap(&mut self);

//     fn start_raygen(&mut self);
//     fn raygen_start_blocks(&mut self);
//     fn is_face_visible(&self, normal: vec3, camera_dir: vec3) -> bool;
//     fn raygen_block_face(&self, normal: ivec3, buff: &IndexedVertices, block_id: BlockId);
//     fn raygen_block(&mut self, block_id: BlockId, shift: ivec3);
//     fn raygen_start_models(&mut self);
//     fn raygen_model_face(&mut self, normal: vec3, buff: &IndexedVertices);
//     fn raygen_model(
//         &mut self,
//         model_mesh: &InternalMeshModel<Self::BufferType, Self::ImageType>,
//         model_trans: &MeshTransform,
//     );
//     fn lightmap_block_face(&self, _normal: ivec3, buff: &IndexedVertices, _block_id: BlockId);
//     fn lightmap_block(&mut self, block_id: BlockId, shift: ivec3);
//     fn lightmap_model_face(&mut self, _normal: vec3, buff: &IndexedVertices);
//     fn lightmap_model(
//         &mut self,
//         model_mesh: &InternalMeshModel<Self::BufferType, Self::ImageType>,
//         model_trans: &MeshTransform,
//     );
//     fn update_particles(&mut self);
//     fn raygen_map_particles(&mut self);
//     fn raygen_start_grass(&mut self);
//     fn updade_grass(&mut self, wind_direction: vec2);
//     fn updade_water(&mut self);
//     fn raygen_map_grass(&mut self, grass: &InternalMeshFoliage, pos: &vec3);
//     fn raygen_start_water(&mut self);
//     fn raygen_map_water(&mut self, _water: &InternalMeshLiquid, pos: &vec3);
//     fn end_raygen(&mut self);
//     fn start_2nd_spass(&mut self);
//     fn diffuse(&mut self);
//     fn ambient_occlusion(&mut self);
//     fn glossy_raygen(&mut self);
//     fn raygen_start_smoke(&mut self);
//     fn raygen_map_smoke(&mut self, _smoke: &InternalMeshVolumetric, pos: &vec3);
//     fn smoke(&mut self);
//     fn glossy(&mut self);
//     fn tonemap(&mut self);
//     fn end_2nd_spass(&mut self);
//     fn end_frame(&mut self, window: &Window);
// }
