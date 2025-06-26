#![feature(stmt_expr_attributes)]
#![feature(custom_inner_attributes)]
#![feature(optimize_attribute)]
#![feature(where_clause_attrs)]
#![feature(associated_type_defaults)] // what the fuck how is this language so incomplete in some cases? Whats next - all const?
#![feature(slice_as_array)]
#![feature(default_field_values)]
#![feature(const_trait_impl)]
// clippy settings
#![allow(clippy::too_many_arguments)] // LOL
#![allow(clippy::option_map_unit_fn)]

use containers::array3d::{ConstDims, Dim3, RuntimeDims};
use qvek::vek::FrustumPlanes;
use qvek::{uvec2, vec3};
use types::{mat4, uvec2, uvec3, vec2, vec3};

pub const BLOCK_SIZE: u32 = 16;
#[allow(non_upper_case_globals)]
pub const sBLOCK_SIZE: usize = 16;
#[allow(non_upper_case_globals)]
pub const fBLOCK_SIZE: f32 = BLOCK_SIZE as f32;

pub mod aabb;
pub mod ao_lut;
pub mod load_interface;
pub mod render_interface;
pub mod types;
#[cfg(feature = "vk_backend")]
pub mod vulkan;
#[cfg(feature = "wgpu_backend")]
pub mod webgpu;

#[derive(Clone, Copy)]
pub struct Settings<D: Dim3 = ConstDims<48, 48, 16>> {
    pub world_size: D,
    pub static_block_palette_size: u32,
    pub max_particle_count: u32,
    pub lightmap_extent: uvec2,
}
impl<D: Dim3> Default for Settings<D> {
    fn default() -> Self {
        Self {
            world_size: Default::default(),
            static_block_palette_size: 15,
            max_particle_count: 8128,
            lightmap_extent: uvec2 { x: 1024, y: 1024 },
        }
    }
}

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
    pub light_transform: mat4,
    pub light_dir: vec3,
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
    fn update_camera(&mut self, y_flip: bool) {
        // hey, wats up?
        // up is 0,0,1
        let up = vec3!(0, 0, 1);
        self.view_size = self.origin_view_size / self.pixels_in_voxel;
        // RIGHT HANDED MATH EVERYWHERE
        let view = mat4::look_at_rh(self.camera_pos, self.camera_pos + self.camera_dir, up);

        let flipper = if y_flip { -1.0 } else { 1.0 };
        let projection = mat4::orthographic_rh_no(FrustumPlanes {
            left: -self.view_size.x / 2.0,
            right: self.view_size.x / 2.0,
            bottom: self.view_size.y / 2.0 * flipper,
            top: -self.view_size.y / 2.0 * flipper,
            near: -0.0,
            far: 2000.0,
        }); // => *(2000.0/2) for decoding

        self.camera_transform = projection * view;
        self.camera_ray_dir_plane = vec3!(self.camera_dir.xy(), 0).normalized();

        self.horizline = self.camera_ray_dir_plane.cross(vec3!(0, 0, 1)).normalized();
        self.vertiline = self.camera_dir.cross(self.horizline).normalized();
    }
}

impl SunLight {
    fn update_light_transform<D: Dim3>(&mut self, world_size: D, y_flip: bool) {
        // TODO: remove magic
        // :wizard_sad:
        let up = vec3!(0, 0, 1).normalized();
        let light_pos = vec3!(uvec2!(world_size.x(), world_size.y()) * BLOCK_SIZE, 0) / 2.0
            - (1.0 * fBLOCK_SIZE * self.light_dir);

        let view = mat4::look_at_rh(light_pos, light_pos + self.light_dir, up);
        let voxel_in_pixels = 5.0;
        let view_width_in_voxels = 3000.0 / voxel_in_pixels;
        let view_height_in_voxels = 3000.0 / voxel_in_pixels;

        let flipper = if y_flip { -1.0 } else { 1.0 };

        let projection = mat4::orthographic_rh_no(FrustumPlanes {
            left: -view_width_in_voxels / 2.0,
            right: view_width_in_voxels / 2.0,
            bottom: view_height_in_voxels / 2.0 * flipper,
            top: -view_height_in_voxels / 2.0 * flipper,
            near: -512.0,
            far: 1024.0,
        });
        self.light_transform = projection * view;
    }
}

// this is basically safier version of assert! that is checked in debug mode
// in release mode opens into just assume!
// std::intrinsics::assume
#[macro_export]
macro_rules! assert_assume {
    ($cond:expr) => {{
        // Do runtime checks in debug mode
        debug_assert!($cond);
        // but also provide assumption to the compiler
        unsafe {
            std::hint::assert_unchecked($cond);
        }
    }};
}

#[macro_export]
macro_rules! assert_unreachable {
    () => {
        if cfg!(debug_assertions) {
            // In debug mode, verify that the code never executes
            panic!();
        } else {
            unreachable_unchecked!();
        }
    };
}

#[macro_export]
macro_rules! for_zyx {
    // Handle ivec3 argument with a closure
    ($dims:expr, $body:expr) => {
        for zz in 0..$dims.z as usize {
        for yy in 0..$dims.y as usize {
        for xx in 0..$dims.x as usize {
            $body(xx, yy, zz)
        }}}
    };

    // Handle 3 separate integers with a closure
    ($x_dim:expr, $y_dim:expr, $z_dim:expr, $body:expr) => {
        for zz in 0..$z_dim as usize {
        for yy in 0..$y_dim as usize {
        for xx in 0..$x_dim as usize {
            $body(xx, yy, zz)
        }}}
    };
}
