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

use crate::types::*;

use super::{aabb, InternalRenderer};

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
    pub fn update_light_transform(&mut self, world_size: uvec3) {
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

impl InternalRenderer {
    pub fn update_camera(&mut self) {
        self.camera.update_camera();
    }

    pub fn update_light_transform(&mut self) {
        self.light.update_light_transform(self.settings.world_size);
        // let horizon =
    }

    // starts the stage where you can "request drawing" things
    // under the hood it prepares Vulkan for recording draw calls
    pub fn start_frame(&mut self) {
        self.update_camera();
        self.update_light_transform();

        self.lumal.start_frame(&[
            *self.cmdbufs.compute_command_buffers.current(),
            *self.cmdbufs.graphics_command_buffers.current(),
            *self.cmdbufs.copy_command_buffers.current(),
            *self.cmdbufs.lightmap_command_buffers.current(),
        ]);
    }

    pub fn start_blockify(&mut self) {
        self.block_copies_queue.clear();
        self.palette_counter = self.static_block_palette_size as usize;

        // reset the current world to the origin
        self.current_world.copy_data_from(&self.origin_world);
    }

    pub fn index_block_xy(&self, n: usize) -> uvec2 {
        let x = n % BLOCK_PALETTE_SIZE_X as usize;
        let y = n / BLOCK_PALETTE_SIZE_X as usize;
        debug_assert!(y <= BLOCK_PALETTE_SIZE_Y as usize);
        uvec2!(x, y)
    }

    // allocates temp block in palette for every block that intersects with every mesh blockified
    pub fn blockify_mesh(&mut self, mesh: &InternalMeshModel, trans: &MeshTransform) {
        let rotate = mat4::from(trans.rotation);
        let shift = mat4::identity().translated_3d(trans.translation);
        let border_in_voxel = get_shift(shift * rotate, mesh.total_size);

        let mut border = iAABB {
            min: ivec3!(border_in_voxel.min - 1.0) / 16,
            max: ivec3!(border_in_voxel.max + 1.0) / 16,
        };

        // clamp to world size so no out of bounds
        border.min = ivec3::clamped(
            border.min,
            ivec3::zero(),
            ivec3!(self.settings.world_size - 1),
        );
        border.max = ivec3::clamped(
            border.max,
            ivec3::zero(),
            ivec3!(self.settings.world_size - 1),
        );

        for zz in border.min.z..=border.max.z {
            for yy in border.min.y..=border.max.y {
                for xx in border.min.x..=border.max.x {
                    let current_block = self.current_world[(xx as usize, yy as usize, zz as usize)];
                    if (current_block as u32) < self.static_block_palette_size {
                        // static
                        //add to copy queue
                        let src_block = self.index_block_xy(current_block as usize);
                        let dst_block = self.index_block_xy(self.palette_counter);

                        // do image copy on for non-zero-src blocks. Other things still done for every allocated block
                        // because zeroing is fast
                        if current_block != 0 {
                            let static_block_copy = vk::ImageCopy {
                                src_subresource: vk::ImageSubresourceLayers {
                                    aspect_mask: vk::ImageAspectFlags::COLOR,
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                },
                                src_offset: vk::Offset3D {
                                    x: src_block.x as i32 * 16,
                                    y: src_block.y as i32 * 16,
                                    z: 0,
                                },
                                dst_subresource: vk::ImageSubresourceLayers {
                                    aspect_mask: vk::ImageAspectFlags::COLOR,
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                },
                                dst_offset: vk::Offset3D {
                                    x: dst_block.x as i32 * 16,
                                    y: dst_block.y as i32 * 16,
                                    z: 0,
                                },
                                extent: vk::Extent3D {
                                    width: 16,
                                    height: 16,
                                    depth: 16,
                                },
                            };
                            // TODO: more compact representation
                            self.block_copies_queue.push(static_block_copy);
                        }

                        self.current_world[(xx as usize, yy as usize, zz as usize)] =
                            self.palette_counter as BlockId;
                        self.palette_counter += 1;
                    } else {
                        //already new block, just leave it
                    }
                }
            }
        }
    }

    pub fn end_blockify(&mut self) {
        let count_to_copy = self.current_world.dimensions().0
            * self.current_world.dimensions().1
            * self.current_world.dimensions().2;
        let size_to_copy = count_to_copy * size_of::<BlockId>();
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.current_world.data.as_ptr(),
                self.buffers.staging_world.current().allocation.mapped_ptr().unwrap().as_ptr()
                    as *mut BlockId,
                count_to_copy, // converts to size automatically
            )
        };
        unsafe {
            self.lumal
                .device
                .flush_mapped_memory_ranges(&[vk::MappedMemoryRange {
                    memory: self.buffers.staging_world.current().allocation.memory(),
                    offset: 0,
                    size: size_to_copy as u64,
                    ..Default::default()
                }])
                .unwrap();
        };
    }

    #[inline(always)]
    fn _function_i_had_to_write_to_be_able_to_use_goto(
        world_size: &uvec3,
        current_world: &Array3D<BlockId>,
        visited: &mut BitArray3d<usize>,
        zz: isize,
        yy: isize,
        xx: isize,
    ) -> bool {
        type TheType = isize;

        macro_rules! check_neighbor {
            ($total:expr, $dx:expr, $dy:expr, $dz:expr) => {
                let dx = $dx;
                let dy = $dy;
                let dz = $dz;
                let x = (xx as TheType + dx).max(0).min(world_size.x as TheType - 1);
                let y = (yy as TheType + dy).max(0).min(world_size.y as TheType - 1);
                let z = (zz as TheType + dz).max(0).min(world_size.z as TheType - 1);
                let block = current_world.get(x as usize, y as usize, z as usize);

                assert_assume!((block > 0) == (block != 0));

                // so, the idea is to make less checks, and also .set() only once
                $total += block;
                // if block > 0 {
                //     // visited.set(xx as usize, yy as usize, zz as usize, true);
                //     return true;
                // }
            };
        }
        let mut total = 0;
        check_neighbor!(total, 0, 0, 0);
        if total > 0 {
            return true;
        }
        check_neighbor!(total, 1, 0, 0);
        check_neighbor!(total, -1, 0, 0);
        if total > 0 {
            return true;
        }
        check_neighbor!(total, 0, -1, 0);
        check_neighbor!(total, 1, -1, 0);
        check_neighbor!(total, -1, -1, 0);
        if total > 0 {
            return true;
        }
        check_neighbor!(total, 0, 1, 0);
        check_neighbor!(total, 1, 1, 0);
        check_neighbor!(total, -1, 1, 0);
        if total > 0 {
            return true;
        }
        check_neighbor!(total, 0, 0, 1);
        check_neighbor!(total, 1, 0, 1);
        check_neighbor!(total, -1, 0, 1);
        check_neighbor!(total, 0, 1, 1);
        if total > 0 {
            return true;
        }
        check_neighbor!(total, 1, 1, 1);
        check_neighbor!(total, -1, 1, 1);
        check_neighbor!(total, 0, -1, 1);
        check_neighbor!(total, 1, -1, 1);
        check_neighbor!(total, -1, -1, 1);
        if total > 0 {
            return true;
        }
        check_neighbor!(total, 0, 0, -1);
        check_neighbor!(total, 1, 0, -1);
        check_neighbor!(total, -1, 0, -1);
        check_neighbor!(total, 0, 1, -1);
        if total > 0 {
            return true;
        }
        check_neighbor!(total, 1, 1, -1);
        check_neighbor!(total, -1, 1, -1);
        check_neighbor!(total, 0, -1, -1);
        check_neighbor!(total, 1, -1, -1);
        check_neighbor!(total, -1, -1, -1);
        // Manual unrolling with spatial coherence (same z)
        // TODO: is there a way to script stuff like this?
        // self:
        // Layer 0
        // Layer +1

        // Layer -1
        if total > 0 {
            visited.set(xx as usize, yy as usize, zz as usize, true);
            // continue;
        }

        false
    }

    // i love the fact that none of these does anything
    #[optimize(speed)]
    // Note: this is the last function that can be called before Vulkan interraction
    // which means that you HAVE to wait at most after it
    pub fn find_radiance_to_update(&mut self) {
        // separation for multiverse
        // let self = &mut *__self;
        flame::start("prepare");
        // somehow caching allocated is slower...
        // let mut visited = &mut self.m_ru_visited;
        // visited.fill(false);

        // like a hash_set, but optimized (no hashing, no collisions)
        // its literally 3d array of bools, each corresponding to "if set"
        let mut visited = BitArray3d::<usize>::new_filled(
            self.settings.world_size.x as usize,
            self.settings.world_size.y as usize,
            self.settings.world_size.z as usize,
            false, // each value in set corresponds to "if the block is already updated"
        );

        flame::end("prepare");
        flame::start("algorithm");

        // well, native size turned to be the fastest
        type TheType = isize;
        // only radiance updates with this offset should be processed

        let magic_number = 2;
        self.counter += 1;
        let current_offset = (self.counter) % magic_number;

        let mut pushed_radiance_count = 0;
        // push block into queue of update requests if the block has neighbours
        for xx in (0 as TheType)..(self.settings.world_size.x as TheType) {
            for yy in (0 as TheType)..(self.settings.world_size.y as TheType) {
                // skip some blocks to reduce the number of requests
                for zz in ((current_offset as TheType)..(self.settings.world_size.z as TheType))
                    .step_by(magic_number as usize)
                {
                    // simple version that is also ~2/570 slower (so not much)
                    'free: for dz in -1..=1 {
                        for dy in -1..=1 {
                            for dx in -1..=1 {
                                // clamp has an assert inside LOL
                                let x = (xx as TheType + dx)
                                    .max(0)
                                    .min(self.settings.world_size.x as TheType - 1);
                                let y = (yy as TheType + dy)
                                    .max(0)
                                    .min(self.settings.world_size.y as TheType - 1);
                                let z = (zz as TheType + dz)
                                    .max(0)
                                    .min(self.settings.world_size.z as TheType - 1);
                                let block =
                                    self.current_world.get(x as usize, y as usize, z as usize);

                                assert_assume!((block > 0) == (block != 0));

                                if block > 0 {
                                    visited.set(xx as usize, yy as usize, zz as usize, true);
                                    pushed_radiance_count += 1;
                                    //i want to
                                    break 'free;
                                }
                            }
                        }
                    }

                    // so, the idea is to make less checks, and also .set() only once (in asm)
                    // let found_non_empty = Self::function_i_had_to_write_to_be_able_to_use_goto(
                    //     &self.settings.world_size,
                    //     &self.current_world,
                    //     &mut visited,
                    //     zz,
                    //     yy,
                    //     xx,
                    // );
                    // if found_non_empty {
                    //     let offset = (xx + yy + zz) as i32 % magic_number;
                    //     visited.set(xx as usize, yy as usize, zz as usize, true);
                    //     pushed_radiance_count += 1;
                    // }
                }
            }
        }

        // self.radiance_updates.clear();

        // for zz in 0..self.settings.world_size.z {
        //     for yy in 0..self.settings.world_size.y {
        //         for xx in 0..self.settings.world_size.x {
        //             if visited.get(xx as usize, yy as usize, zz as usize) {
        //                 self.radiance_updates.push(i8vec4::new(xx as i8, yy as i8, zz as i8, 0));
        //             }
        //         }
        //     }
        // }

        self.radiance_updates.resize(pushed_radiance_count as usize, i8vec4::zero());

        let mut i = 0;
        for zz in 0..self.settings.world_size.z {
            for yy in 0..self.settings.world_size.y {
                for xx in 0..self.settings.world_size.x {
                    if visited.get(xx as usize, yy as usize, zz as usize) {
                        assert_assume!(i < self.radiance_updates.len());
                        self.radiance_updates[i] = i8vec4!(xx, yy, zz, 0);
                        i += 1;
                    }
                }
            }
        }

        flame::end("algorithm");
        flame::start("special");

        // special updates are ones requested via API
        for u in &self.special_radiance_updates {
            // if not already updated in loop before, add it to the queue
            if !visited.get(u.x as usize, u.y as usize, u.z as usize) {
                self.radiance_updates.push(*u);
            }
        }
        flame::end("special");

        drop(visited);
    }

    #[optimize(speed)]
    pub fn update_radiance(&mut self) {
        // separation for multiverse
        Self::_update_radiance(self);
    }

    // #[multiversion(targets("x86_64+avx2"))]
    #[optimize(speed)]
    fn _update_radiance(self: &mut InternalRenderer) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();

        flame::start("copy radiance updates");

        // flame::start("copy radiance updates");
        let count_to_copy = self.radiance_updates.len();
        let size_to_copy = count_to_copy * size_of::<i8vec4>();
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.radiance_updates.as_ptr(),
                self.buffers
                    .staging_radiance_updates
                    .first()
                    .allocation
                    .mapped_ptr()
                    .unwrap()
                    .as_ptr() as *mut i8vec4,
                count_to_copy, // converts to size automatically
            )
        };
        flame::end("copy radiance updates");

        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.staging_radiance_updates.first(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        );
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.gpu_radiance_updates.first(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        );

        let copy = vk::BufferCopy {
            size: size_to_copy as u64,
            src_offset: 0,
            dst_offset: 0,
        };

        flame::start("cmd copy");
        if count_to_copy > 0 {
            unsafe {
                self.lumal.device.cmd_copy_buffer(
                    *command_buffer,
                    self.buffers.staging_radiance_updates.first().buffer,
                    self.buffers.gpu_radiance_updates.first().buffer,
                    &[copy],
                );
                // self.lumal.device.cmd_copy_buffer(
                //     *command_buffer,
                //     self.buffers.staging_radiance_updates.current().buffer,
                //     self.buffers.gpu_radiance_updates.next().buffer,
                //     &[copy],
                // );
            };
        }
        flame::end("cmd copy");

        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.gpu_radiance_updates.first(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        );

        // binds descriptor sets and pipeline itself
        self.lumal.bind_compute_pipe(command_buffer, &self.pipes.radiance_pipe);

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            time: i32,
            iters: i32,
        }

        let push_constant = PushConstant {
            time: self.lumal.frame,
            iters: 0,
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.radiance_pipe.line_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                push_constant.as_u8_slice(),
            );
        }

        let wg_count = self.radiance_updates.len();

        unsafe {
            self.lumal.device.cmd_dispatch(
                *command_buffer,
                // TOOD: current implementation just marches through the whole array skipping a lot of elements
                // Why didn't i just pack work tightly?
                wg_count as u32,
                // TODO: remove
                1,
                1,
            )
        };

        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.radiance_cache.first(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL, // Most of images are in GENERAL because:
                                      // 1. Highly optimized GPU code uses images in multiple ways, which restricts to GENERAL only
                                      // 2. When it does not, gained perfomance is negligible compared to the (my) work required to manage layouts
                                      // 3. Most popular GPU's dont give a fuck about layouts (NVIDIA)
                                      // 4. Even AMD did not gain any perfomance in my tests (at some point, i did whole thing with correct layouts and barriers and it was the same perfomance)
                                      // 5. anyways, there is still reason to do it, but only when all other optimizations are done
        );
    }

    // when shift is zero, no work is done (so dont cache this)
    pub fn shift_radiance(&mut self, radiance_shift: ivec3) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();

        let cam_shift = radiance_shift + 0;

        if cam_shift.x.abs() >= self.settings.world_size.x as i32
            || cam_shift.y.abs() >= self.settings.world_size.y as i32
            || cam_shift.z.abs() >= self.settings.world_size.z as i32
        {
            return; // then its pointless (zero-volume intersection). We can set it to zero os some pre-computed value in future, tho
        }

        let process_axis = |shift: i32, _world_size: i32| -> ivec2 {
            let self_src_offset;
            let self_dst_offset;
            if shift >= 0 {
                self_src_offset = shift;
            } else {
                self_src_offset = 0;
            }

            if shift >= 0 {
                self_dst_offset = 0;
            } else {
                self_dst_offset = shift.abs();
            }

            ivec2::new(self_src_offset, self_dst_offset)
        };

        let self_src_offset = ivec3!(
            process_axis(cam_shift.x, self.settings.world_size.x as i32).x,
            process_axis(cam_shift.y, self.settings.world_size.y as i32).x,
            process_axis(cam_shift.z, self.settings.world_size.z as i32).x
        );
        let self_dst_offset = ivec3!(
            process_axis(cam_shift.x, self.settings.world_size.x as i32).y,
            process_axis(cam_shift.y, self.settings.world_size.y as i32).y,
            process_axis(cam_shift.z, self.settings.world_size.z as i32).y
        );

        let intersection_size = self.settings.world_size
            - uvec3!(
                cam_shift.x.unsigned_abs(),
                cam_shift.y.unsigned_abs(),
                cam_shift.z.unsigned_abs()
            );

        let mut copy_region = vk::ImageCopy {
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: 1,
                mip_level: 0,
            },
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: 1,
                mip_level: 0,
            },
            extent: vk::Extent3D {
                width: intersection_size.x,
                height: intersection_size.y,
                depth: intersection_size.z,
            },
            src_offset: vk::Offset3D {
                x: self_src_offset.x,
                y: self_src_offset.y,
                z: self_src_offset.z,
            },
            dst_offset: vk::Offset3D {
                x: 0, // no reason to copy anywhere else - DST IS TEMP STORAGE
                y: 0,
                z: 0,
            },
        };

        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.radiance_cache.current(),
            vk::PipelineStageFlags::TRANSFER, // well sometimes i feel like i should pick better barriers
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.radiance_cache.previous(),
            vk::PipelineStageFlags::TRANSFER, // well sometimes i feel like i should pick better barriers
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );

        // copy to temp
        unsafe {
            self.lumal.device.cmd_copy_image(
                *command_buffer,
                self.independent_images.radiance_cache.current().image,
                vk::ImageLayout::GENERAL,
                self.independent_images.radiance_cache.previous().image,
                vk::ImageLayout::GENERAL,
                &[copy_region],
            );
        };

        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.radiance_cache.current(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.radiance_cache.previous(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );

        // copy back (setting up region)
        copy_region.extent = vk::Extent3D {
            width: intersection_size.x,
            height: intersection_size.y,
            depth: intersection_size.z,
        };
        copy_region.src_offset = vk::Offset3D {
            x: 0, // we want 0,0,0 to end up in shift
            y: 0,
            z: 0,
        };
        copy_region.dst_offset = vk::Offset3D {
            x: self_dst_offset.x, // well, this is how to tell it to end up in (shift)
            y: self_dst_offset.y,
            z: self_dst_offset.z,
        };
        // actually copy back
        unsafe {
            self.lumal.device.cmd_copy_image(
                *command_buffer,
                self.independent_images.radiance_cache.previous().image,
                vk::ImageLayout::GENERAL,
                self.independent_images.radiance_cache.current().image,
                vk::ImageLayout::GENERAL,
                &[copy_region],
            );
        };

        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.radiance_cache.current(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.radiance_cache.previous(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
    }

    pub fn exec_copies(&mut self) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();

        let clear_color = vk::ClearColorValue::default();
        let clear_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        // Transition images for copying (lol no transition atm)
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.origin_block_palette.current(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );

        unsafe {
            // zero out the current image (zeroing 90% and copying the rest is faster than copying all)
            self.lumal.device.cmd_clear_color_image(
                *command_buffer,
                self.independent_images.origin_block_palette.current().image,
                vk::ImageLayout::GENERAL,
                &clear_color,
                &[clear_range],
            )
        };

        // sync
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.origin_block_palette.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );

        // TODO: multi-raw copy
        debug_assert!(self.static_block_palette_size < BLOCK_PALETTE_SIZE_X);
        let static_block_palette_copy = vk::ImageCopy {
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: 1,
                mip_level: 0,
            },
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: 1,
                mip_level: 0,
            },
            extent: vk::Extent3D {
                width: 16 * self.static_block_palette_size,
                height: 16,
                depth: 16,
            },
            src_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            dst_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
        };

        // copy static blocks back (to zeroed). So clean version of palette now
        unsafe {
            self.lumal.device.cmd_copy_image(
                *command_buffer,
                self.independent_images.origin_block_palette.previous().image, // we zeroed current, but previous stayed the same, so we grap static palette from there
                vk::ImageLayout::GENERAL,
                self.independent_images.origin_block_palette.current().image,
                vk::ImageLayout::GENERAL,
                &[static_block_palette_copy],
            );
        };

        // sync
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.origin_block_palette.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );

        // Execute actual block copy for each allocated temporal block
        // TODO: maybe we should copy from current to current, cause these blocks are allocated, and we just copy (clone)
        // the static blocks to allocated ones. So they never intersect. Maybe its faster
        if !self.block_copies_queue.is_empty() {
            // idk if copying 0 is allowed
            unsafe {
                self.lumal.device.cmd_copy_image(
                    *command_buffer,
                    self.independent_images.origin_block_palette.previous().image,
                    vk::ImageLayout::GENERAL,
                    self.independent_images.origin_block_palette.current().image,
                    vk::ImageLayout::GENERAL,
                    self.block_copies_queue.as_slice(),
                )
            };
        }

        // sync
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.origin_block_palette.current(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );

        // copy the entire world buffer to the world image (there is no direct way so intermediate copy (buffer) is needed)
        let copy_region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D {
                width: self.settings.world_size.x,
                height: self.settings.world_size.y,
                depth: self.settings.world_size.z,
            },
        };

        // sync
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );

        unsafe {
            self.lumal.device.cmd_copy_buffer_to_image(
                *command_buffer,
                self.buffers.staging_world.current().buffer,
                self.independent_images.world.current().image,
                vk::ImageLayout::GENERAL,
                &[copy_region],
            );
        };

        // sync
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
    }

    pub fn start_map(&mut self) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();

        self.lumal.bind_compute_pipe(command_buffer, &self.pipes.map_pipe);
    }

    pub fn map_mesh(&mut self, mesh: &InternalMeshModel, trans: &MeshTransform) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();
        let model_voxels_info = vk::DescriptorImageInfo {
            image_view: mesh.voxels.view,
            image_layout: vk::ImageLayout::GENERAL,
            sampler: vk::Sampler::null(),
        };
        let binding = [model_voxels_info];
        let model_voxels_write = vk::WriteDescriptorSet {
            dst_set: vk::DescriptorSet::null(),
            dst_binding: 0,
            dst_array_element: 0,
            descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
            descriptor_count: 1,
            p_image_info: &binding as *const _,
            ..Default::default()
        };

        unsafe {
            self.lumal.push_descriptors_loader.cmd_push_descriptor_set(
                *command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipes.map_pipe.line_layout,
                1,
                &[model_voxels_write],
            )
        };

        let rotate = mat4::from(trans.rotation);
        let shift = mat4::identity().translated_3d(trans.translation);
        let transform = shift * rotate;

        let border_in_voxel = get_shift(transform, mesh.total_size);

        let border = iAABB {
            min: ivec3!(border_in_voxel.min.floor()),
            max: ivec3!(border_in_voxel.max.ceil()),
        };

        let map_area = border.max - border.min;
        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            trans: mat4,
            shift: ivec4,
        }
        let push_constant = PushConstant {
            trans: transform.inverted(),
            shift: ivec4!(border.min, 0),
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.map_pipe.line_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                push_constant.as_u8_slice(),
            )
        };

        // NOTE: it was *3+3 but i have no idea why and did i break anything
        unsafe {
            self.lumal.device.cmd_dispatch(
                *command_buffer,
                (map_area.x + 3) as u32 / 4,
                (map_area.y + 3) as u32 / 4,
                (map_area.z + 3) as u32 / 4,
            )
        };
    }

    pub fn end_map(&mut self) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.world.current(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::AccessFlags::SHADER_WRITE,
            vk::AccessFlags::SHADER_READ,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
    }

    pub fn end_compute(&mut self) {
        let _command_buffer = self.cmdbufs.compute_command_buffers.current();
        // do nothing
    }

    pub fn start_lightmap(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct BufferPatch {
            trans: mat4,
        }
        let buffer_patch = BufferPatch {
            trans: self.light.light_transform,
        };

        // sync
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.light_uniform.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        );

        unsafe {
            self.lumal.device.cmd_update_buffer(
                *command_buffer,
                self.buffers.light_uniform.current().buffer,
                0,
                buffer_patch.as_u8_slice(),
            )
        };

        // sync
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.light_uniform.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        );

        self.lumal.cmd_begin_renderpass(
            command_buffer,
            &self.rpasses.lightmap_rpass,
            vk::SubpassContents::INLINE,
        );
    }

    pub fn lightmap_start_blocks(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.lightmap_blocks_pipe);
    }

    pub fn lightmap_start_models(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.lightmap_models_pipe);
    }

    pub fn end_lightmap(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        self.lumal.cmd_end_renderpass(command_buffer, &mut self.rpasses.lightmap_rpass)
    }

    pub fn start_raygen(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct BufferPatch {
            trans_w2s: mat4,
            campos: vec4,
            camdir: vec4,
            horizline_scaled: vec4,
            vertiline_scaled: vec4,
            global_light_dir: vec4,
            lightmap_proj: mat4,
            size: vec2,
            timeseed: i32,
        }

        let horizline_scaled = self.camera.horizline * (self.camera.view_size.x / 2.0);
        let vertiline_scaled = self.camera.vertiline * (self.camera.view_size.y / 2.0);

        let buffer_patch = BufferPatch {
            trans_w2s: self.camera.camera_transform,
            campos: vec4!(self.camera.camera_pos, 0),
            camdir: vec4!(self.camera.camera_dir, 0),
            horizline_scaled: vec4!(horizline_scaled, 0),
            vertiline_scaled: vec4!(vertiline_scaled, 0),
            global_light_dir: vec4!(self.light.light_dir, 0),
            lightmap_proj: self.light.light_transform,
            size: qvek::vec2!(
                self.lumal.vulkan_data.swapchain_extent.width,
                self.lumal.vulkan_data.swapchain_extent.height
            ),
            timeseed: self.lumal.frame,
        };
        // dbg!(self.lumal.frame);

        // sync
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.uniform.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        );

        unsafe {
            self.lumal.device.cmd_update_buffer(
                *command_buffer,
                self.buffers.uniform.current().buffer,
                0,
                buffer_patch.as_u8_slice(),
            )
        };

        // sync
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.uniform.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        );

        self.lumal.cmd_begin_renderpass(
            command_buffer,
            // gbuffer is also somewhat referred to as raygen (cause generated gbuffer is used as source for raytrace)
            &self.rpasses.gbuffer_rpass,
            vk::SubpassContents::INLINE,
        );
    }

    pub fn raygen_start_blocks(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.raygen_blocks_pipe);
    }

    fn is_face_visible(&self, normal: vec3, camera_dir: vec3) -> bool {
        normal.dot(camera_dir) < 0.0
    }

    fn raygen_block_face(&self, normal: ivec3, buff: &IndexedVertices, block_id: BlockId) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        debug_assert!(block_id > 0);
        let sum = normal.x + normal.y + normal.z;
        // u8 sign = (sum > 0) ? 0 : 1;
        let neg_sign = match sum > 0 {
            true => 0,
            false => 1,
        };

        let absnorm = u8vec3::new(
            normal.x.unsigned_abs() as u8,
            normal.y.unsigned_abs() as u8,
            normal.z.unsigned_abs() as u8,
        );
        debug_assert!((absnorm.x + absnorm.y + absnorm.z) == 1);
        let pbn = { (neg_sign << 7) | absnorm.x | (absnorm.y << 1) | (absnorm.z << 2) };
        //signBit_4EmptyBits_xBit_yBit_zBit
        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            // block: BlockID_t, // passed before separately
            // shift: i16vec3, // passed before separately
            inorm: u8vec4,
        }
        let push_constant = PushConstant {
            inorm: u8vec4::new(pbn, 0, 0, 0), // TODO: what the hell was i smoking?
        };
        debug_assert!(push_constant.as_u8_slice().len() == 4);

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_blocks_pipe.line_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                8,
                push_constant.as_u8_slice(),
            )
        };

        unsafe {
            self.lumal
                .device
                .cmd_draw_indexed(*command_buffer, buff.icount, 1, buff.offset, 0, 0)
        };
    }

    pub fn raygen_block(&mut self, block_id: BlockId, shift: ivec3) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        let block_mesh = &self.block_palette_meshes[block_id as usize];
        unsafe {
            self.lumal.device.cmd_bind_vertex_buffers(
                *command_buffer,
                0,
                &[block_mesh.triangles.vertexes.buffer],
                &[0],
            );
            self.lumal.device.cmd_bind_index_buffer(
                *command_buffer,
                block_mesh.triangles.indices.buffer,
                0,
                vk::IndexType::UINT16, // yes, they are not 32 bit. And what?
            );
        };

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            block: BlockId,
            shift: i16vec3,
            // inorm: i8vec4, // passed separately
        }

        let push_constant = PushConstant {
            block: block_id,
            shift: i16vec3!(shift),
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_blocks_pipe.line_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_constant.as_u8_slice(),
            )
        };

        // loving macros. IDK if C is better, i am not nearly as good in Rust as i am in C, but still cool
        macro_rules! CHECK_AND_DRAW_BLOCK_FACE {
            ($__normal:expr, $__face:ident) => {
                let fnorm = vec3::new($__normal.x as f32, $__normal.y as f32, $__normal.z as f32);
                let inorm = ivec3!($__normal.x as i32, $__normal.y as i32, $__normal.z as i32);
                if self.is_face_visible(fnorm, self.camera.camera_dir) {
                    self.raygen_block_face(inorm, &block_mesh.triangles.$__face, block_id);
                }
            };
        }

        // draw every face (separately). This allows per-face culling
        // damn, my rasterization is really optimized
        // on 1660s it takes like 0.11 for all blocks (few thouthands) to raster
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(1, 0, 0), Pzz);
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(-1, 0, 0), Nzz);
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 1, 0), zPz);
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, -1, 0), zNz);
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, 1), zzP);
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, -1), zzN);
    }

    pub fn raygen_start_models(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        unsafe { self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE) };

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.raygen_models_pipe);
    }

    fn raygen_model_face(&mut self, normal: vec3, buff: &IndexedVertices) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            inorm: vec4,
        }
        let push_constant = PushConstant {
            inorm: vec4!(normal, 0),
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_models_pipe.line_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                32, // TODO sizeof of merged struct
                push_constant.as_u8_slice(),
            )
        };

        unsafe {
            self.lumal
                .device
                .cmd_draw_indexed(*command_buffer, buff.icount, 1, buff.offset, 0, 0)
        }
    }

    pub fn raygen_model(&mut self, model_mesh: &InternalMeshModel, model_trans: &MeshTransform) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        unsafe {
            self.lumal.device.cmd_bind_vertex_buffers(
                *command_buffer,
                0,
                &[model_mesh.triangles.vertexes.buffer],
                &[0],
            );
            self.lumal.device.cmd_bind_index_buffer(
                *command_buffer,
                model_mesh.triangles.indices.buffer,
                0,
                vk::IndexType::UINT16,
            );
        };
        /*
            vec4 rot;
            vec4 shift;
            vec4 fnormal; //not encoded
        */
        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            rot: quat,
            shift: vec4,
            // fnormal: vec4,
        }
        let push_constant = PushConstant {
            rot: model_trans.rotation,
            shift: vec4!(model_trans.translation, 0),
            // fnormal: vec4::new(normal.x, normal.y, normal.z, 0.0),
        };
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_models_pipe.line_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_constant.as_u8_slice(),
            )
        }

        let model_voxels_info = vk::DescriptorImageInfo {
            image_view: model_mesh.voxels.view,
            image_layout: vk::ImageLayout::GENERAL,
            sampler: self.samplers.unnorm_nearest,
        };
        let binding = [model_voxels_info];
        let model_voxels_write = vk::WriteDescriptorSet {
            dst_set: vk::DescriptorSet::null(),
            dst_binding: 0,
            dst_array_element: 0,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
            p_image_info: &binding as *const _,
            ..Default::default()
        };

        unsafe {
            self.lumal.push_descriptors_loader.cmd_push_descriptor_set(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipes.raygen_models_pipe.line_layout,
                1,
                &[model_voxels_write],
            )
        }

        macro_rules! CHECK_AND_DRAW_MODEL_FACE {
            ($__normal:expr, $__face:ident) => {
                let fnorm = vec3::new($__normal.x as f32, $__normal.y as f32, $__normal.z as f32);
                if self.is_face_visible(model_trans.rotation * fnorm, self.camera.camera_dir) {
                    self.raygen_model_face(fnorm, &model_mesh.triangles.$__face);
                }
            };
        }

        CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(1, 0, 0), Pzz);
        CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(-1, 0, 0), Nzz);
        CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(0, 1, 0), zPz);
        CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(0, -1, 0), zNz);
        CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(0, 0, 1), zzP);
        CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(0, 0, -1), zzN);
        // let _ :i64 = 0x0_c001_babe_face; // why did i port this?
    }

    fn lightmap_block_face(&self, _normal: ivec3, buff: &IndexedVertices, _block_id: BlockId) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();
        unsafe {
            self.lumal
                .device
                .cmd_draw_indexed(*command_buffer, buff.icount, 1, buff.offset, 0, 0)
        }
    }

    pub fn lightmap_block(&mut self, block_id: BlockId, shift: ivec3) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        let block_mesh = &self.block_palette_meshes[block_id as usize];
        unsafe {
            self.lumal.device.cmd_bind_vertex_buffers(
                *command_buffer,
                0,
                &[block_mesh.triangles.vertexes.buffer],
                &[0],
            );
            self.lumal.device.cmd_bind_index_buffer(
                *command_buffer,
                block_mesh.triangles.indices.buffer,
                0,
                vk::IndexType::UINT16, // yes, they are not 32 bit. And what?
            );
        };
        /*
            int16_t block;
            i16vec3 shift;
            i8vec4 inorm;
        */
        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            shift: i16vec4,
        }
        let push_constant = PushConstant {
            shift: i16vec4!(shift, 0),
        };
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.lightmap_blocks_pipe.line_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                push_constant.as_u8_slice(),
            )
        };

        macro_rules! CHECK_AND_DRAW_BLOCK_FACE {
            ($__normal:expr, $__face:ident) => {
                let fnorm = vec3!($__normal);
                let inorm = ivec3!($__normal);
                if self.is_face_visible(fnorm, self.camera.camera_dir) {
                    self.lightmap_block_face(inorm, &block_mesh.triangles.$__face, block_id);
                }
            };
        }

        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(1, 0, 0), Pzz);
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(-1, 0, 0), Nzz);
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 1, 0), zPz);
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, -1, 0), zNz);
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, 1), zzP);
        CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, -1), zzN);
    }

    fn lightmap_model_face(&mut self, _normal: vec3, buff: &IndexedVertices) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        unsafe {
            self.lumal
                .device
                .cmd_draw_indexed(*command_buffer, buff.icount, 1, buff.offset, 0, 0)
        }
    }

    pub fn lightmap_model(&mut self, model_mesh: &InternalMeshModel, model_trans: &MeshTransform) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();
        unsafe {
            self.lumal.device.cmd_bind_vertex_buffers(
                *command_buffer,
                0,
                &[model_mesh.triangles.vertexes.buffer],
                &[0],
            );
            self.lumal.device.cmd_bind_index_buffer(
                *command_buffer,
                model_mesh.triangles.indices.buffer,
                0,
                vk::IndexType::UINT16,
            );
        };
        /*
            vec4 rot;
            vec4 shift;
            vec4 fnormal; //not encoded
        */
        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            rot: quat,
            shift: vec4,
            // fnormal: vec4,
        }
        let push_constant = PushConstant {
            rot: model_trans.rotation,
            shift: vec4!(model_trans.translation, 0),
            // fnormal: vec4::new(normal.x, normal.y, normal.z, 0.0),
        };
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.lightmap_models_pipe.line_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                push_constant.as_u8_slice(),
            )
        }

        macro_rules! CHECK_AND_LIGHTMAP_MODEL_FACE {
            ($__normal:expr, $__face:ident) => {
                let fnorm = vec3!($__normal);
                if self.is_face_visible(model_trans.rotation * fnorm, self.camera.camera_dir) {
                    self.lightmap_model_face(fnorm, &model_mesh.triangles.$__face);
                }
            };
        }

        CHECK_AND_LIGHTMAP_MODEL_FACE!(i8vec3::new(1, 0, 0), Pzz);
        CHECK_AND_LIGHTMAP_MODEL_FACE!(i8vec3::new(-1, 0, 0), Nzz);
        CHECK_AND_LIGHTMAP_MODEL_FACE!(i8vec3::new(0, 1, 0), zPz);
        CHECK_AND_LIGHTMAP_MODEL_FACE!(i8vec3::new(0, -1, 0), zNz);
        CHECK_AND_LIGHTMAP_MODEL_FACE!(i8vec3::new(0, 0, 1), zzP);
        CHECK_AND_LIGHTMAP_MODEL_FACE!(i8vec3::new(0, 0, -1), zzN);
    }

    pub fn update_particles(&mut self) {
        let mut write_index = 0;

        for i in 0..self.particles.len() {
            let should_keep = self.particles[i].life_time > 0.0;
            if should_keep {
                self.particles[write_index] = self.particles[i];

                let velocity = self.particles[write_index].vel;
                self.particles[write_index].pos += velocity * self.delta_time;

                self.particles[write_index].life_time -= self.delta_time;
                write_index += 1;
            }
        }

        self.particles.shrink_to(write_index);
        let capped_particle_count = write_index.clamp(0, self.settings.max_particle_count as usize);

        unsafe {
            std::ptr::copy_nonoverlapping(
                self.particles.as_ptr(),
                self.buffers.gpu_particles.current().allocation.mapped_ptr().unwrap().as_ptr()
                    as *mut Particle,
                capped_particle_count, // converts to size automatically
            )
        }

        let size_to_flush = capped_particle_count * size_of::<Particle>();
        unsafe {
            self.lumal
                .device
                .flush_mapped_memory_ranges(&[vk::MappedMemoryRange {
                    memory: self.buffers.gpu_particles.current().allocation.memory(),
                    offset: 0,
                    size: size_to_flush as u64,
                    ..Default::default()
                }])
                .unwrap();
        }
    }

    pub fn raygen_map_particles(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }

        if !self.particles.is_empty() {
            // just for safity
            self.lumal.bind_raster_pipe(command_buffer, &self.pipes.raygen_particles_pipe);
            unsafe {
                self.lumal.device.cmd_bind_vertex_buffers(
                    *command_buffer,
                    0,
                    &[self.buffers.gpu_particles.current().buffer],
                    &[0],
                );
                self.lumal
                    .device
                    .cmd_draw(*command_buffer, self.particles.len() as u32, 1, 0, 0);
            }
        }
    }

    pub fn raygen_start_grass(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }
    }

    pub fn updade_grass(&mut self, wind_direction: vec2) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();
        self.lumal.bind_compute_pipe(command_buffer, &self.pipes.update_grass_pipe);

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            wind_direction: vec2,
            _wtf_is_this: vec2,
            time: f32,
        }
        let push_constant = PushConstant {
            wind_direction,
            _wtf_is_this: vec2::new(0.0, 0.0),
            time: self.lumal.frame as f32,
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.update_grass_pipe.line_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                push_constant.as_u8_slice(),
            )
        }

        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.grass_state.current(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::AccessFlags::SHADER_WRITE,
            vk::AccessFlags::SHADER_WRITE | vk::AccessFlags::SHADER_READ,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );

        unsafe {
            self.lumal.device.cmd_dispatch(
                //2x8 2x8 1x1
                *command_buffer,
                (self.settings.world_size.x * 2).div_ceil(8),
                (self.settings.world_size.y * 2).div_ceil(8),
                1,
            );
        }
    }

    pub fn updade_water(&mut self) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();
        self.lumal.bind_compute_pipe(command_buffer, &self.pipes.update_water_pipe);

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            wind_direction: vec2,
            time: f32,
        }
        let push_constant = PushConstant {
            wind_direction: vec2::new(0.0, 0.0),
            time: self.lumal.frame as f32,
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.update_water_pipe.line_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                push_constant.as_u8_slice(),
            )
        }

        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.water_state.current(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::AccessFlags::SHADER_WRITE,
            vk::AccessFlags::SHADER_WRITE | vk::AccessFlags::SHADER_READ,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );

        unsafe {
            self.lumal.device.cmd_dispatch(
                //2x8 2x8 1x1
                *command_buffer,
                (self.settings.world_size.x * 2).div_ceil(8),
                (self.settings.world_size.y * 2).div_ceil(8),
                1,
            );
        }
    }

    pub fn raygen_map_grass(&mut self, grass: &InternalMeshFoliage, pos: &vec3) {
        let command_buffers = self.cmdbufs.graphics_command_buffers.current();

        let size = 10;
        let x_flip = self.camera.camera_dir.x < 0.0;
        let y_flip = self.camera.camera_dir.y < 0.0;

        let pipe = &self.pipes.raygen_foliage_pipes[grass.stored_id as usize];
        let desc = &self.foliage_descriptions[grass.stored_id as usize];
        // it is somewhat cached
        self.lumal.bind_raster_pipe(command_buffers, pipe);

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            shift: vec4,
            _size: i32,
            _time: i32,
            xf: i32,
            yf: i32,
        }
        let push_constant = PushConstant {
            shift: vec4!(*pos, 0),
            _size: size as i32,
            _time: self.lumal.frame,
            xf: x_flip as i32, // TODO: compress
            yf: y_flip as i32,
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffers,
                pipe.line_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_constant.as_u8_slice(),
            )
        }

        let verts_per_blade = desc.vertices;
        let blade_per_instance = 1; //for triangle strip
        unsafe {
            #[allow(clippy::manual_div_ceil)]
            self.lumal.device.cmd_draw(
                *command_buffers,
                verts_per_blade * blade_per_instance,
                (size * size + (blade_per_instance - 1)) / blade_per_instance,
                0,
                0,
            )
        };
    }

    pub fn raygen_start_water(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe { self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE) };

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.raygen_water_pipe);
    }

    pub fn raygen_map_water(&mut self, _water: &InternalMeshLiquid, pos: &vec3) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        let quality_size = 32;
        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            shift: vec4,
            _size: i32,
            _time: i32,
        }

        let push_constant = PushConstant {
            shift: vec4!(*pos, 0),
            _size: quality_size as i32,
            _time: self.lumal.frame,
        };
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_water_pipe.line_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_constant.as_u8_slice(),
            )
        }

        let verts_per_water_tape = quality_size * 2 + 2;
        let tapes_per_block = quality_size;
        unsafe {
            self.lumal
                .device
                .cmd_draw(*command_buffer, verts_per_water_tape, tapes_per_block, 0, 0)
        };
    }

    pub fn end_raygen(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        self.lumal.cmd_end_renderpass(command_buffer, &mut self.rpasses.gbuffer_rpass);
    }

    pub fn start_2nd_spass(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        let ao_lut = InternalRenderer::generate_lut::<8>(
            16.0 / 1000.0,
            vec2::new(
                self.lumal.vulkan_data.swapchain_extent.width as f32,
                self.lumal.vulkan_data.swapchain_extent.height as f32,
            ),
            self.camera.horizline * self.camera.view_size.x / 2.0,
            self.camera.vertiline * self.camera.view_size.y / 2.0,
        );

        // sync
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.ao_lut_uniform.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        );

        unsafe {
            self.lumal.device.cmd_update_buffer(
                *command_buffer,
                self.buffers.ao_lut_uniform.current().buffer,
                0,
                // TODO: derive?
                std::slice::from_raw_parts(
                    (&ao_lut as *const AoLut) as *const u8,
                    std::mem::size_of::<AoLut>(),
                ),
            );
        }

        // sync
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.ao_lut_uniform.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        );

        self.lumal.cmd_begin_renderpass(
            command_buffer,
            &self.rpasses.shade_rpass,
            vk::SubpassContents::INLINE,
        );
    }

    pub fn diffuse(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.diffuse_pipe);

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            v1: vec4,
            v2: vec4,
            lp: mat4,
        }
        // kinda rnd source
        let transmuted_frame = unsafe { transmute::<i32, f32>(self.lumal.frame) };
        let push_constant = PushConstant {
            v1: vec4!(self.camera.camera_pos, transmuted_frame),
            v2: vec4!(self.camera.camera_dir, 0),
            lp: self.light.light_transform,
        };
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.diffuse_pipe.line_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_constant.as_u8_slice(),
            )
        };

        unsafe {
            // you may wonder - why no bound buffer? Answer: its fullscreen triangle
            self.lumal.device.cmd_draw(*command_buffer, 3, 1, 0, 0);
        } // btw, every such call is fullscreen triangle
    }

    pub fn ambient_occlusion(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.ao_pipe);

        unsafe {
            // fullscreen triangle
            self.lumal.device.cmd_draw(*command_buffer, 3, 1, 0, 0);
        }
    }

    pub fn glossy_raygen(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }

        self.lumal
            .bind_raster_pipe(command_buffer, &self.pipes.fill_stencil_glossy_pipe);

        unsafe {
            // fullscreen triangle
            self.lumal.device.cmd_draw(*command_buffer, 3, 1, 0, 0);
        }
    }

    pub fn raygen_start_smoke(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.fill_stencil_smoke_pipe);
    }

    pub fn raygen_map_smoke(&mut self, _smoke: &InternalMeshVolumetric, pos: &vec3) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            center_size: vec4,
        }
        let push_constant = PushConstant {
            center_size: vec4!(pos * 16.0, 32),
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.fill_stencil_smoke_pipe.line_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_constant.as_u8_slice(),
            )
        };

        unsafe {
            // least optimized cube in the world. If GPU programmers used twitter, i would be getting canceled for this
            self.lumal.device.cmd_draw(*command_buffer, 36, 1, 0, 0);
        }
    }

    pub fn smoke(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.smoke_pipe);

        unsafe {
            // fullscreen triangle
            self.lumal.device.cmd_draw(*command_buffer, 3, 1, 0, 0);
        }
    }

    pub fn glossy(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.glossy_pipe);

        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            v1: vec4,
            v2: vec4,
        }
        let push_constant = PushConstant {
            v1: vec4!(self.camera.camera_pos, 0),
            v2: vec4!(self.camera.camera_dir, 0),
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.glossy_pipe.line_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_constant.as_u8_slice(),
            )
        };

        unsafe {
            // fullscreen triangle
            self.lumal.device.cmd_draw(*command_buffer, 3, 1, 0, 0);
        }
    }

    pub fn tonemap(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.tonemap_pipe);

        unsafe {
            // fullscreen triangle
            self.lumal.device.cmd_draw(*command_buffer, 3, 1, 0, 0);
        }
    }

    pub fn end_2nd_spass(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        // Currently, there is no UI because it is getting abstracted away (l0l)
        self.lumal.cmd_end_renderpass(command_buffer, &mut self.rpasses.shade_rpass);
    }

    pub fn end_frame(&mut self, window: &Window) {
        self.lumal.end_frame(
            &[
                // "Special" cmb used by UI copies & layout transitions HAS to be first
                // Otherwise copied images are in LAYOUT_UNDEFINED because copies did not happen yet
                // so, copy before using the copy (makes sense, right?)
                *self.cmdbufs.copy_command_buffers.current(),
                // world-space things
                *self.cmdbufs.compute_command_buffers.current(),
                // lightmap! yes, a single one. But we can always add more!
                *self.cmdbufs.lightmap_command_buffers.current(),
                // per-pixel
                *self.cmdbufs.graphics_command_buffers.current(),
            ],
            window,
        );

        self.cmdbufs.copy_command_buffers.move_next(); //runtime copies for ui. Also does first frame resources
        self.cmdbufs.compute_command_buffers.move_next();
        self.cmdbufs.lightmap_command_buffers.move_next();
        self.cmdbufs.graphics_command_buffers.move_next();

        self.independent_images.lightmap.move_next();
        self.dependent_images.highres_frame.move_next();
        self.dependent_images.highres_depth_stencil.move_next();
        self.dependent_images.highres_mat_norm.move_next();
        self.dependent_images.stencil_view_for_ds.move_next();
        self.dependent_images.far_depth.move_next(); //represents how much should smoke traversal for
        self.dependent_images.near_depth.move_next(); //represents how much should smoke traversal for
                                                      // self.dependent_images.mask_frame.move_next(); //where lowres renders to. Blends with highres afterwards
        self.buffers.staging_world.move_next();
        self.independent_images.world.move_next(); //can i really use just one?
        self.independent_images.origin_block_palette.move_next();
        // self.independent_images.distance_palette.move_next();
        // self.independent_images.bit_palette.move_next(); //bitmask of originBlockPalette
        self.independent_images.material_palette.move_next();
        self.buffers.light_uniform.move_next();
        self.buffers.uniform.move_next();
        self.buffers.ao_lut_uniform.move_next();
        self.buffers.gpu_radiance_updates.move_next();
        self.buffers.staging_radiance_updates.move_next();
        self.buffers.gpu_particles.move_next(); //multiple because cpu-related work
        self.independent_images.perlin_noise2d.move_next(); //full-world grass shift (~direction) texture sampled in grass
        self.independent_images.perlin_noise3d.move_next(); //full-world grass shift (~direction) texture sampled in grass

        self.independent_images.grass_state.move_next(); //full-world grass shift (~direction) texture sampled in grass
        self.independent_images.water_state.move_next(); //~same but water

        let should_recreate = self.lumal.should_recreate;
        if should_recreate {
            self.recreate_window(window);
            self.lumal.should_recreate = false;
        }

        // self
    }
}
