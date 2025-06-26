use super::InternalRendererVulkan;
use crate::{
    assert_assume,
    load_interface::{BlockData, ModelData},
    render_interface::{FoliageDescriptionCreate, ShaderSource},
    vulkan::{pc_types, BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y},
    Settings, *,
};
use crate::{
    load_interface::LoadInterface, render_interface::RendererInterface, types::*, vulkan::types::*,
};
use aabb::{get_shift, iAABB};
use containers::Arena;
use containers::{
    array3d::{Array3DView, Array3DViewMut},
    BitArray3d,
};
use lumal::vk;
use qvek::{i16vec3, i16vec4, i8vec4, ivec3, ivec4, uvec2, uvec3, vec3, vec4, vek::Clamp};
use std::time::Instant;
use winit::window::Window;

// i am clearly trash with managing division into files
// if someone has a good idea on how to do it, message me (or just make a PR)
impl<'a, D: Dim3> InternalRendererVulkan<'a, D> {
    fn update_camera(&mut self) {
        self.camera.update_camera(false);
    }

    fn update_light_transform(&mut self) {
        self.light.update_light_transform(self.settings.world_size, false);
        // let horizon =
    }

    fn start_blockify(&mut self) {
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
    fn blockify_mesh(&mut self, mesh: &InternalMeshModel, trans: &MeshTransform) {
        let rotate = mat4::from(trans.rotation);
        let shift = mat4::identity().translated_3d(trans.translation);
        let border_in_voxel = get_shift(shift * rotate, mesh.size);

        let mut border = iAABB {
            min: ivec3!(border_in_voxel.min - 1.0) / (BLOCK_SIZE as i32),
            max: ivec3!(border_in_voxel.max + 1.0) / (BLOCK_SIZE as i32),
        };

        // clamp to world size so no out of bounds
        border.min = ivec3::clamped(
            border.min,
            ivec3::zero(),
            ivec3!(self.settings.world_size.xyz() - 1),
        );
        border.max = ivec3::clamped(
            border.max,
            ivec3::zero(),
            ivec3!(self.settings.world_size.xyz() - 1),
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
                                    x: src_block.x as i32 * BLOCK_SIZE as i32,
                                    y: src_block.y as i32 * BLOCK_SIZE as i32,
                                    z: 0,
                                },
                                dst_subresource: vk::ImageSubresourceLayers {
                                    aspect_mask: vk::ImageAspectFlags::COLOR,
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                },
                                dst_offset: vk::Offset3D {
                                    x: dst_block.x as i32 * BLOCK_SIZE as i32,
                                    y: dst_block.y as i32 * BLOCK_SIZE as i32,
                                    z: 0,
                                },
                                extent: vk::Extent3D {
                                    width: BLOCK_SIZE,
                                    height: BLOCK_SIZE,
                                    depth: BLOCK_SIZE,
                                },
                            };
                            // TODO: more compact representation
                            self.block_copies_queue.push(static_block_copy);
                        }

                        self.current_world[(xx as usize, yy as usize, zz as usize)] =
                            self.palette_counter as MeshBlock;
                        self.palette_counter += 1;
                    } else {
                        //already new block, just leave it
                    }
                }
            }
        }
    }

    fn end_blockify(&mut self) {
        let count_to_copy = self.current_world.dimensions().0
            * self.current_world.dimensions().1
            * self.current_world.dimensions().2;
        let size_to_copy = count_to_copy * size_of::<MeshBlock>();
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.current_world.data.as_ptr(),
                self.buffers.staging_world.current().allocation.mapped_ptr().unwrap().as_ptr()
                    as *mut MeshBlock,
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

    // i love the fact that none of these does anything

    // Note: this is the last function that can be called before Vulkan interraction
    // which means that you HAVE to wait at most after it
    fn find_radiance_to_update(&mut self) {
        // separation for multiverse
        // let self = &mut *__self;
        // somehow caching allocated is slower...
        // let mut visited = &mut self.m_ru_visited;
        // visited.fill(false);

        // like a hash_set, but optimized (no hashing, no collisions)
        // its literally 3d array of bools, each corresponding to "if set"
        let mut visited = BitArray3d::<usize, D>::new_filled(
            self.settings.world_size,
            false, // each value in set corresponds to "if the block is already updated"
        );

        // well, native size turned to be the fastest
        type TheType = isize;
        // only radiance updates with this offset should be processed

        let magic_number = 2;
        self.counter += 1;
        let current_offset = (self.counter) % magic_number;

        let mut pushed_radiance_count = 0;
        // push block into queue of update requests if the block has neighbours
        for xx in (0 as TheType)..(self.settings.world_size.x() as TheType) {
            for yy in (0 as TheType)..(self.settings.world_size.y() as TheType) {
                // skip some blocks to reduce the number of requests
                for zz in ((current_offset as TheType)..(self.settings.world_size.z() as TheType))
                    .step_by(magic_number as usize)
                {
                    // simple version that is also ~2/570 slower (so not much)
                    'free: for dz in -1..=1 {
                        for dy in -1..=1 {
                            for dx in -1..=1 {
                                // clamp has an assert inside LOL
                                let x = (xx as TheType + dx)
                                    .max(0)
                                    .min(self.settings.world_size.x() as TheType - 1);
                                let y = (yy as TheType + dy)
                                    .max(0)
                                    .min(self.settings.world_size.y() as TheType - 1);
                                let z = (zz as TheType + dz)
                                    .max(0)
                                    .min(self.settings.world_size.z() as TheType - 1);
                                let block =
                                    self.current_world.get(x as usize, y as usize, z as usize);

                                assert_assume!((*block > 0) == (*block != 0));

                                if *block > 0 {
                                    visited.set(xx as usize, yy as usize, zz as usize, true);
                                    pushed_radiance_count += 1;
                                    //i want to
                                    break 'free;
                                }
                            }
                        }
                    }
                }
            }
        }

        self.radiance_updates.resize(pushed_radiance_count as usize, i8vec4::zero());

        let mut i = 0;
        for zz in 0..self.settings.world_size.z() {
            for yy in 0..self.settings.world_size.y() {
                for xx in 0..self.settings.world_size.x() {
                    if visited.get(xx as usize, yy as usize, zz as usize) {
                        assert_assume!(i < self.radiance_updates.len());
                        self.radiance_updates[i] = i8vec4!(xx, yy, zz, 0);
                        i += 1;
                    }
                }
            }
        }

        // special updates are ones requested via API
        for u in &self.special_radiance_updates {
            // if not already updated in loop before, add it to the queue
            if !visited.get(u.x as usize, u.y as usize, u.z as usize) {
                self.radiance_updates.push(*u);
            }
        }

        drop(visited);
    }

    /// Starts rendering stage where you can "request drawing" things.
    fn start_frame(&mut self) {
        let now = Instant::now();
        self.delta_time = ((now - self.last_time).as_nanos() as f64 / 1e9) as f32;
        self.last_time = now;

        self.update_camera();
        self.update_light_transform();

        self.lumal.start_frame(&[
            *self.cmdbufs.compute_command_buffers.current(),
            *self.cmdbufs.graphics_command_buffers.current(),
            *self.cmdbufs.copy_command_buffers.current(),
            *self.cmdbufs.lightmap_command_buffers.current(),
        ]);
    }

    fn update_radiance(&mut self) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();

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
            &self.buffers.gpu_radiance_updates,
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

        if count_to_copy > 0 {
            unsafe {
                self.lumal.device.cmd_copy_buffer(
                    *command_buffer,
                    self.buffers.staging_radiance_updates.first().buffer,
                    self.buffers.gpu_radiance_updates.buffer,
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

        self.lumal.buffer_memory_barrier(
            command_buffer,
            &self.buffers.gpu_radiance_updates,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
        );

        // binds descriptor sets and pipeline itself
        self.lumal.bind_compute_pipe(command_buffer, &self.pipes.radiance_pipe);

        let push_constant = pc_types::Radiance {
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

        unsafe { self.lumal.device.cmd_dispatch(*command_buffer, wg_count as u32, 1, 1) };

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
                                      // 2. When it does not, gained perfomance is negligible compared to (my) work required to manage layouts,
                                      // especially in rapid development stage
                                      // 3. Most popular GPU's dont give a fuck about layouts (NVIDIA)
                                      // 4. Even AMD did not gain any perfomance in my tests (at some point, i did whole thing with correct layouts and barriers and it was the same perfomance)
                                      // Anyways, there is still reason to do it, but only when all other optimizations are done
        );
    }

    /// Moves radiance field by specified offset
    /// When shift is zero, no work is done (so dont cache this)
    fn shift_radiance(&mut self, radiance_shift: ivec3) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();

        let cam_shift = radiance_shift + 0;

        if cam_shift.x.abs() >= self.settings.world_size.x() as i32
            || cam_shift.y.abs() >= self.settings.world_size.y() as i32
            || cam_shift.z.abs() >= self.settings.world_size.z() as i32
        {
            return; // then its pointless (zero-volume intersection). We can set it to zero os some pre-computed value in future, tho
        }

        let process_axis = |shift: i32, _world_size: i32| -> ivec2 {
            let self_src_offset = shift.clamp(0, shift);
            let self_dst_offset = shift.clamp(shift, 0).abs();

            ivec2::new(self_src_offset, self_dst_offset)
        };

        let self_src_offset = ivec3!(
            process_axis(cam_shift.x, self.settings.world_size.x() as i32).x,
            process_axis(cam_shift.y, self.settings.world_size.y() as i32).x,
            process_axis(cam_shift.z, self.settings.world_size.z() as i32).x
        );
        let self_dst_offset = ivec3!(
            process_axis(cam_shift.x, self.settings.world_size.x() as i32).y,
            process_axis(cam_shift.y, self.settings.world_size.y() as i32).y,
            process_axis(cam_shift.z, self.settings.world_size.z() as i32).y
        );

        let intersection_size = uvec3!(self.settings.world_size.xyz())
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

    fn exec_copies(&mut self) {
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
            self.independent_images.block_palette.previous(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.block_palette.current(),
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
                self.independent_images.block_palette.current().image,
                vk::ImageLayout::GENERAL,
                &clear_color,
                &[clear_range],
            )
        };

        // sync
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.block_palette.previous(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.block_palette.current(),
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
                width: BLOCK_SIZE * self.static_block_palette_size,
                height: BLOCK_SIZE,
                depth: BLOCK_SIZE,
            },
            src_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            dst_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
        };

        // copy static blocks back (to zeroed). So clean version of palette now
        unsafe {
            self.lumal.device.cmd_copy_image(
                *command_buffer,
                self.independent_images.block_palette.previous().image, // we zeroed current, but previous stayed the same, so we grap static palette from there
                vk::ImageLayout::GENERAL,
                self.independent_images.block_palette.current().image,
                vk::ImageLayout::GENERAL,
                &[static_block_palette_copy],
            );
        };

        // sync
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.block_palette.previous(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.block_palette.current(),
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
                    self.independent_images.block_palette.previous().image,
                    vk::ImageLayout::GENERAL,
                    self.independent_images.block_palette.current().image,
                    vk::ImageLayout::GENERAL,
                    self.block_copies_queue.as_slice(),
                )
            };
        }

        // sync
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.block_palette.previous(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.block_palette.current(),
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
                width: self.settings.world_size.x() as u32,
                height: self.settings.world_size.y() as u32,
                depth: self.settings.world_size.z() as u32,
            },
        };

        // sync
        self.lumal.image_memory_barrier(
            command_buffer,
            self.independent_images.block_palette.previous(),
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
            self.independent_images.block_palette.previous(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::GENERAL,
        );
    }

    fn start_map(&mut self) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();

        self.lumal.bind_compute_pipe(command_buffer, &self.pipes.map_pipe);
    }

    fn map_mesh(&mut self, mesh: &InternalMeshModel, trans: &MeshTransform) {
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

        let border_in_voxel = get_shift(transform, mesh.size);

        let border = iAABB {
            min: ivec3!(border_in_voxel.min.floor()),
            max: ivec3!(border_in_voxel.max.ceil()),
        };

        let map_area = border.max - border.min;

        let push_constant = pc_types::Map {
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

    fn end_map(&mut self) {
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

    fn end_compute(&mut self) {
        let _command_buffer = self.cmdbufs.compute_command_buffers.current();
        // do nothing
    }

    fn start_lightmap(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        let buffer_patch = pc_types::LightmapUBO {
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

    fn lightmap_start_blocks(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.lightmap_blocks_pipe);
    }

    fn lightmap_start_models(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.lightmap_models_pipe);
    }

    fn end_lightmap(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        self.lumal.cmd_end_renderpass(command_buffer, &mut self.rpasses.lightmap_rpass)
    }

    fn start_raygen(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        let horizline_scaled = self.camera.horizline * (self.camera.view_size.x / 2.0);
        let vertiline_scaled = self.camera.vertiline * (self.camera.view_size.y / 2.0);

        let buffer_patch = pc_types::UBO {
            trans_w2s: self.camera.camera_transform,
            campos: vec4!(self.camera.camera_pos, 0),
            camdir: vec4!(self.camera.camera_dir, 0),
            horizline_scaled: vec4!(horizline_scaled, 0),
            vertiline_scaled: vec4!(vertiline_scaled, 0),
            global_light_dir: vec4!(self.light.light_dir, 0),
            lightmap_proj: self.light.light_transform,
            size: qvek::vec2!(
                self.lumal.swapchain_extent.width,
                self.lumal.swapchain_extent.height
            ),
            timeseed: self.lumal.frame,
        };

        // sync UBO write
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

    fn raygen_start_blocks(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.raygen_blocks_pipe);
    }

    fn is_face_visible(&self, normal: vec3, camera_dir: vec3) -> bool {
        normal.dot(camera_dir) < 0.0
    }

    fn raygen_block_face(&self, normal: ivec3, buff: &IndexedVertices, block_id: MeshBlock) {
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
        //signBit_4EmptyBits_xBit_yBit_zBit
        let pbn = { (neg_sign << 7) | absnorm.x | (absnorm.y << 1) | (absnorm.z << 2) };

        let push_constant = pc_types::RaygenBlockPerBlock {
            // block: BlockID_t, // passed before separately
            // shift: i16vec3, // passed before separately
            inorm: u8vec4::new(pbn, 0, 0, 0), // TODO: what the hell was i smoking?
        };
        debug_assert!(push_constant.as_u8_slice().len() == 4);

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_blocks_pipe.layout,
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

    fn raygen_block(&mut self, block_id: MeshBlock, shift: ivec3) {
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

        let push_constant = pc_types::RaygenBlockPerFace {
            block: block_id,
            shift: i16vec3!(shift),
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_blocks_pipe.layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_constant.as_u8_slice(),
            )
        };

        let check_and_raygen_block_face = |normal: i8vec3, face: &IndexedVertices| {
            let fnorm = vec3::new(normal.x as f32, normal.y as f32, normal.z as f32);
            let inorm = ivec3!(normal.x as i32, normal.y as i32, normal.z as i32);
            if self.is_face_visible(fnorm, self.camera.camera_dir) {
                self.raygen_block_face(inorm, face, block_id);
            }
        };

        // draw every face (separately). This allows per-face culling
        // damn, my rasterization is really optimized
        // on 1660s it takes like 0.11 for all blocks (few thouthands) to raster
        check_and_raygen_block_face(i8vec3::new(1, 0, 0), &block_mesh.triangles.Pzz);
        check_and_raygen_block_face(i8vec3::new(-1, 0, 0), &block_mesh.triangles.Nzz);
        check_and_raygen_block_face(i8vec3::new(0, 1, 0), &block_mesh.triangles.zPz);
        check_and_raygen_block_face(i8vec3::new(0, -1, 0), &block_mesh.triangles.zNz);
        check_and_raygen_block_face(i8vec3::new(0, 0, 1), &block_mesh.triangles.zzP);
        check_and_raygen_block_face(i8vec3::new(0, 0, -1), &block_mesh.triangles.zzN);
    }

    fn raygen_start_models(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        unsafe { self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE) };

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.raygen_models_pipe);
    }

    fn raygen_model_face(&mut self, normal: vec3, buff: &IndexedVertices) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        let push_constant = pc_types::RaygenModelPerFace {
            inorm: vec4!(normal, 0),
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_models_pipe.layout,
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

    fn raygen_model(&mut self, model_mesh: &InternalMeshModel, model_trans: &MeshTransform) {
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

        let push_constant = pc_types::RaygenModelPerModel {
            rot: model_trans.rotation,
            shift: vec4!(model_trans.translation, 0),
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_models_pipe.layout,
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

        // This is how Lum deals with dynamic descriptors
        unsafe {
            self.lumal.push_descriptors_loader.cmd_push_descriptor_set(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipes.raygen_models_pipe.layout,
                1,
                &[model_voxels_write],
            )
        }

        let mut check_and_raygen_model_face = |normal: i8vec3, face: &IndexedVertices| {
            let fnorm = vec3::new(normal.x as f32, normal.y as f32, normal.z as f32);
            if self.is_face_visible(model_trans.rotation * fnorm, self.camera.camera_dir) {
                self.raygen_model_face(fnorm, face);
            }
        };

        check_and_raygen_model_face(i8vec3::new(1, 0, 0), &model_mesh.triangles.Pzz);
        check_and_raygen_model_face(i8vec3::new(-1, 0, 0), &model_mesh.triangles.Nzz);
        check_and_raygen_model_face(i8vec3::new(0, 1, 0), &model_mesh.triangles.zPz);
        check_and_raygen_model_face(i8vec3::new(0, -1, 0), &model_mesh.triangles.zNz);
        check_and_raygen_model_face(i8vec3::new(0, 0, 1), &model_mesh.triangles.zzP);
        check_and_raygen_model_face(i8vec3::new(0, 0, -1), &model_mesh.triangles.zzN);

        // let _ :i64 = 0x0_c001_babe_face;
    }

    fn lightmap_block_face(&self, _normal: ivec3, buff: &IndexedVertices, _block_id: MeshBlock) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();
        unsafe {
            self.lumal
                .device
                .cmd_draw_indexed(*command_buffer, buff.icount, 1, buff.offset, 0, 0)
        }
    }

    fn lightmap_block(&mut self, block_id: MeshBlock, shift: ivec3) {
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

        let push_constant = pc_types::LightmapBlock {
            shift: i16vec4!(shift, 0),
        };
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.lightmap_blocks_pipe.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                push_constant.as_u8_slice(),
            )
        };

        let check_and_lightmap_block_face = |normal: i8vec3, face: &IndexedVertices| {
            let fnorm = vec3!(normal);
            let inorm = ivec3!(normal);
            if self.is_face_visible(fnorm, self.camera.camera_dir) {
                self.lightmap_block_face(inorm, face, block_id);
            }
        };

        check_and_lightmap_block_face(i8vec3::new(1, 0, 0), &block_mesh.triangles.Pzz);
        check_and_lightmap_block_face(i8vec3::new(-1, 0, 0), &block_mesh.triangles.Nzz);
        check_and_lightmap_block_face(i8vec3::new(0, 1, 0), &block_mesh.triangles.zPz);
        check_and_lightmap_block_face(i8vec3::new(0, -1, 0), &block_mesh.triangles.zNz);
        check_and_lightmap_block_face(i8vec3::new(0, 0, 1), &block_mesh.triangles.zzP);
        check_and_lightmap_block_face(i8vec3::new(0, 0, -1), &block_mesh.triangles.zzN);
    }

    fn lightmap_model_face(&mut self, _normal: vec3, buff: &IndexedVertices) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        unsafe {
            self.lumal
                .device
                .cmd_draw_indexed(*command_buffer, buff.icount, 1, buff.offset, 0, 0)
        }
    }

    fn lightmap_model(&mut self, model_mesh: &InternalMeshModel, model_trans: &MeshTransform) {
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

        let push_constant = pc_types::LightmapModel {
            rot: model_trans.rotation,
            shift: vec4!(model_trans.translation, 0),
        };
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.lightmap_models_pipe.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                push_constant.as_u8_slice(),
            )
        }

        let mut check_and_lightmap_model_face = |normal: i8vec3, face: &IndexedVertices| {
            let fnorm = vec3!(normal);
            if self.is_face_visible(model_trans.rotation * fnorm, self.camera.camera_dir) {
                self.lightmap_model_face(fnorm, face);
            }
        };

        check_and_lightmap_model_face(i8vec3::new(1, 0, 0), &model_mesh.triangles.Pzz);
        check_and_lightmap_model_face(i8vec3::new(-1, 0, 0), &model_mesh.triangles.Nzz);
        check_and_lightmap_model_face(i8vec3::new(0, 1, 0), &model_mesh.triangles.zPz);
        check_and_lightmap_model_face(i8vec3::new(0, -1, 0), &model_mesh.triangles.zNz);
        check_and_lightmap_model_face(i8vec3::new(0, 0, 1), &model_mesh.triangles.zzP);
        check_and_lightmap_model_face(i8vec3::new(0, 0, -1), &model_mesh.triangles.zzN);
    }

    fn update_particles(&mut self) {
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

    fn raygen_map_particles(&mut self) {
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

    fn raygen_start_grass(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }
    }

    fn updade_grass(&mut self, wind_direction: vec2) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();
        self.lumal.bind_compute_pipe(command_buffer, &self.pipes.update_grass_pipe);

        let push_constant = pc_types::Grass {
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
            &self.independent_images.grass_state,
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
                (self.settings.world_size.x() * 2).div_ceil(8) as u32,
                (self.settings.world_size.y() * 2).div_ceil(8) as u32,
                1,
            );
        }
    }

    fn updade_water(&mut self) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();
        self.lumal.bind_compute_pipe(command_buffer, &self.pipes.update_water_pipe);

        let push_constant = pc_types::WaterUpdate {
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
            &self.independent_images.water_state,
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
                (self.settings.world_size.x() * 2).div_ceil(8) as u32,
                (self.settings.world_size.y() * 2).div_ceil(8) as u32,
                1,
            );
        }
    }

    fn raygen_map_grass(&mut self, grass: &InternalMeshFoliage, pos: &vec3) {
        let command_buffers = self.cmdbufs.graphics_command_buffers.current();

        let size = 10;
        let x_flip = self.camera.camera_dir.x < 0.0;
        let y_flip = self.camera.camera_dir.y < 0.0;

        let pipe = &self.pipes.raygen_foliage_pipes[grass.stored_id as usize];
        let desc = &self.foliage_descriptions[grass.stored_id as usize];
        // it is somewhat cached
        self.lumal.bind_raster_pipe(command_buffers, pipe);

        let push_constant = pc_types::RaygenMapGrass {
            shift: vec4!(*pos, 0),
            _size: size as i32,
            _time: self.lumal.frame,
            xf: x_flip as i32, // TODO: compress
            yf: y_flip as i32,
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffers,
                pipe.layout,
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

    fn raygen_start_water(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe { self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE) };

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.raygen_water_pipe);
    }

    fn raygen_map_water(&mut self, _water: &InternalMeshLiquid, pos: &vec3) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        let quality_size = 32;

        let push_constant = pc_types::RaygenMapWater {
            shift: vec4!(*pos, 0),
            _size: quality_size as i32,
            _time: self.lumal.frame,
        };
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_water_pipe.layout,
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

    fn end_raygen(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        self.lumal.cmd_end_renderpass(command_buffer, &mut self.rpasses.gbuffer_rpass);
    }

    fn start_2nd_spass(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        let ao_lut = ao_lut::generate_lut::<{ vulkan::AO_LUT_SIZE }>(
            fBLOCK_SIZE / 1000.0,
            vec2::new(
                self.lumal.swapchain_extent.width as f32,
                self.lumal.swapchain_extent.height as f32,
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

    fn diffuse(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.diffuse_pipe);

        // kinda rnd RaygenMapGrass
        let transmuted_frame = f32::from_bits(i32::cast_unsigned(self.lumal.frame));
        let push_constant = pc_types::Diffuse {
            v1: vec4!(self.camera.camera_pos, transmuted_frame),
            v2: vec4!(self.camera.camera_dir, 0),
            lp: self.light.light_transform,
        };
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.diffuse_pipe.layout,
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

    fn ambient_occlusion(&mut self) {
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

    fn glossy_raygen(&mut self) {
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

    fn raygen_start_smoke(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.fill_stencil_smoke_pipe);
    }

    fn raygen_map_smoke(&mut self, _smoke: &InternalMeshVolumetric, pos: &vec3) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        let push_constant = pc_types::RaygenMapSmoke {
            center_size: vec4!(pos * fBLOCK_SIZE, 32),
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.fill_stencil_smoke_pipe.layout,
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

    fn smoke(&mut self) {
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

    fn glossy(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        unsafe {
            self.lumal.device.cmd_next_subpass(*command_buffer, vk::SubpassContents::INLINE);
        }

        self.lumal.bind_raster_pipe(command_buffer, &self.pipes.glossy_pipe);

        let push_constant = pc_types::Glossy {
            v1: vec4!(self.camera.camera_pos, 0),
            v2: vec4!(self.camera.camera_dir, 0),
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.glossy_pipe.layout,
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

    fn tonemap(&mut self) {
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

    fn end_2nd_spass(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        // Currently, there is no UI because it is getting abstracted away (l0l)
        self.lumal.cmd_end_renderpass(command_buffer, &mut self.rpasses.shade_rpass);
    }

    fn end_frame(&mut self, window: &Window) {
        self.lumal.end_frame(&[
            // "Special" cmb used by UI copies & layout transitions HAS to be first
            // Otherwise copied images are in LAYOUT_UNDEFINED because copies did not happen yet
            // so, copy before using the copy (makes sense, right?)
            *self.cmdbufs.copy_command_buffers.current(),
            *self.cmdbufs.compute_command_buffers.current(),
            *self.cmdbufs.lightmap_command_buffers.current(),
            *self.cmdbufs.graphics_command_buffers.current(),
        ]);

        self.cmdbufs.copy_command_buffers.move_next();
        self.cmdbufs.compute_command_buffers.move_next();
        self.cmdbufs.lightmap_command_buffers.move_next();
        self.cmdbufs.graphics_command_buffers.move_next();

        self.buffers.staging_world.move_next();
        self.independent_images.world.move_next();
        self.independent_images.block_palette.move_next();
        self.independent_images.material_palette.move_next();
        self.buffers.light_uniform.move_next();
        self.buffers.uniform.move_next();
        self.buffers.ao_lut_uniform.move_next();
        self.buffers.staging_radiance_updates.move_next();
        self.buffers.gpu_particles.move_next();

        let should_recreate = self.lumal.should_recreate;
        if should_recreate {
            self.recreate_window(window.inner_size());
            self.lumal.should_recreate = false;
        }
    }
}

pub struct ModelRenderRequest {
    pub cam_dist: f32,
    pub mesh: MeshModel,
    pub trans: MeshTransform,
}
pub struct BlockRenderRequest {
    pub cam_dist: f32,
    pub block: MeshBlock,
    // snapped to voxel grid
    pub pos: i16vec3,
}

pub struct FoliageRenderRequest {
    pub cam_dist: f32,
    pub mesh: MeshFoliage,

    //TODO: pub size: vec2
    pub pos: vec3,
}
pub struct LiquidRenderRequest {
    pub cam_dist: f32,
    pub mesh: MeshLiquid,
    //TODO: pub size: vec2/vec3?
    pub pos: vec3,
}
pub struct VolumetricRenderRequest {
    pub cam_dist: f32,
    pub mesh: MeshVolumetric,
    //TODO: pub size: vec3?
    pub pos: vec3,
}

#[derive(Default)]
pub struct RendererStorage {
    // TODO: arena?
    models: Arena<InternalMeshModel>,
    volumetrics: Arena<InternalMeshVolumetric>,
    liquids: Arena<InternalMeshLiquid>,
    // TODO: do smth about that this is stored inside internal renderer and everything else is stored here
    // foliages: Arena<InternalMeshFoliage>,
}

// initialized fully working Renderer that can be used to draw voxels on screen
pub struct RendererVulkan<'a, D: Dim3> {
    pub renderer: InternalRendererVulkan<'a, D>,
    pub window: std::sync::Arc<Window>,
    pub block_que: Vec<BlockRenderRequest>,
    pub model_que: Vec<ModelRenderRequest>,
    pub foliage_que: Vec<FoliageRenderRequest>,
    pub liquid_que: Vec<LiquidRenderRequest>,
    pub volumetric_que: Vec<VolumetricRenderRequest>,
    pub storage: RendererStorage,
    pub radiance_shift: ivec3,
    pub phantom: std::marker::PhantomData<&'a ()>,
}

// these lifetimes mean that lifetime of ref in MeshFoliageDescription is same as ref in ShaderSource
impl<'a> FoliageDescriptionCreate<'a> for MeshFoliageDescription<'a> {
    fn new(code: ShaderSource<'a>, vertices: usize, dencity: usize) -> Self {
        let ShaderSource::SpirV(spirv) = code else {
            panic!()
        };
        Self {
            spirv_code: spirv,
            vertices: vertices as u32,
            density: dencity as u32,
        }
    }
}

/// Description of foliage mesh to be created
#[derive(Debug, Clone, Default)]
pub struct MeshFoliageDescription<'a> {
    /// Shader, compiled into spirv.
    /// Owned by description for siplicity.
    pub spirv_code: &'a [u8],

    /// How many vertices will be in per-blade drawcall.
    /// This is dependent on how many vertices does your corresponding foliage shader need.
    pub vertices: u32,
    /// How many blades is there going to be in a "raw" (linear)
    /// and how many raws there will be in a block.
    /// Total density*density blades rendered.
    pub density: u32,
}

/// Simplistic implementation of FoliageDescriptionBuilder
pub struct SimpleFoliageDescriptionBuilder<'a> {
    foliage_descriptions: Vec<MeshFoliageDescription<'a>>,
}

impl<'a> render_interface::FoliageDescriptionBuilder<MeshFoliageDescription<'a>>
    for SimpleFoliageDescriptionBuilder<'a>
{
    fn new() -> Self {
        Self {
            foliage_descriptions: vec![],
        }
    }
    fn load_foliage(&mut self, foliage: MeshFoliageDescription<'a>) -> MeshFoliage {
        let index = self.foliage_descriptions.len() as u32;
        self.foliage_descriptions.push(foliage);
        index as MeshFoliage
    }
    fn build(self) -> Vec<MeshFoliageDescription<'a>> {
        self.foliage_descriptions
    }
}

impl<'a, D: Dim3> RendererVulkan<'a, D> {
    /// Function to sort our requests by depth (unlike wgpu backend, where we sort by state. State change in Vulkan is fast)
    pub fn calculate_and_sort_by_cam_dist<Type>(rqueue: &mut [Type], camera_transform: mat4)
    where
        Type: GetPos,
    {
        for rrequest in rqueue.iter_mut() {
            let clip_coords = camera_transform * vec4!(rrequest.get_pos(), 1.0);
            rrequest.set_cam_dist(-clip_coords.z);
        }

        rqueue.sort_unstable_by(|a, b| {
            if a.get_cam_dist() > b.get_cam_dist() {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        });
    }
}

impl<'a, D: Dim3> RendererInterface<'a, D> for RendererVulkan<'a, D> {
    type FoliageDescription = MeshFoliageDescription<'a>;
    type FoliageDescriptionBuilder = SimpleFoliageDescriptionBuilder<'a>;
    type InternalBlockId = InternalBlockId;

    fn new(
        settings: &Settings<D>,
        window: std::sync::Arc<Window>,
        size: winit::dpi::PhysicalSize<u32>,
        foliage: &[Self::FoliageDescription],
    ) -> Self {
        Self {
            renderer: InternalRendererVulkan::new(settings, &window, size, foliage.to_vec()),
            window: window.clone(),
            block_que: vec![],
            foliage_que: vec![],
            liquid_que: vec![],
            volumetric_que: vec![],
            model_que: vec![],
            storage: RendererStorage::default(),
            radiance_shift: ivec3::zero(),
            phantom: std::marker::PhantomData,
        }
    }

    async fn new_async(
        _settings: &Settings<D>,
        _window: std::sync::Arc<winit::window::Window>,
        _size: winit::dpi::PhysicalSize<u32>,
        _foliages: &[Self::FoliageDescription],
    ) -> Self {
        unreachable!("Vulkan backend does not need async")
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.renderer.recreate_window(new_size);
    }

    fn destroy(self) {
        self.renderer.destroy();
    }

    fn load_model(&mut self, model: ModelData) -> MeshModel {
        let model_mesh = self.renderer.load_model(model);
        let index = self.storage.models.allocate(model_mesh).unwrap();
        index as MeshModel
    }
    fn unload_model(&mut self, model: MeshModel) {
        let model_mesh = self.storage.models.take(model).unwrap();
        self.renderer.free_model(model_mesh);
    }
    fn get_model_size(&self, model: MeshModel) -> uvec3 {
        self.storage.models.get(model).unwrap().size
    }

    // loads a block (from file) into GPU-side mesh and CPU-side voxel data

    fn load_block(&mut self, block: MeshBlock, block_data: BlockData) {
        self.renderer.load_block(block, block_data);
    }
    fn unload_block(&mut self, block: MeshBlock) {
        self.renderer.free_block(block);
    }

    // volumetrics can be loaded any time (no context on GPU). But please, load them in the same way as models / foliage
    // rendered using same shader, mesh is just "uniforms"
    fn load_volumetric(
        &mut self,
        max_density: f32,
        dencity_variation: f32,
        color: u8vec3,
    ) -> MeshVolumetric {
        let volumetric_mesh = InternalMeshVolumetric {
            max_density,
            variation: dencity_variation,
            color,
        };
        let index = self.storage.volumetrics.allocate(volumetric_mesh).unwrap();
        index as MeshVolumetric
    }
    fn unload_volumetric(&mut self, volumetric: MeshVolumetric) {
        let volumetric_mesh = self.storage.volumetrics.take(volumetric).unwrap();
        drop(volumetric_mesh);
    }

    // liquids can be loaded any time (no context on GPU). But please, load them in the same way as models / foliage / volumetrics
    // rendered using same shader, mesh is just "uniforms"
    fn load_liquid(&mut self, main_mat: MatId, foam_mat: MatId) -> MeshLiquid {
        let liquid_mesh = InternalMeshLiquid {
            main: main_mat,
            foam: foam_mat,
            // pc_buffer: todo!(),
            // pc_bg: todo!(),
            // push_constants: todo!(),
            // pc_count: todo!(),
        };
        let index = self.storage.liquids.allocate(liquid_mesh).unwrap();
        index as MeshLiquid
    }
    fn unload_liquid(&mut self, liquid: MeshLiquid) {
        let liquid_mesh = self.storage.liquids.take(liquid).unwrap();
        drop(liquid_mesh);
    }

    fn unload_foliage(&mut self, foliage: MeshFoliage) {
        let _ = foliage;
    }

    // TODO: calculate distance here vs separate
    // TODO: check visibility here vs separate
    fn draw_world(&mut self) {
        for zz in 0..self.renderer.settings.world_size.z() {
            for yy in 0..self.renderer.settings.world_size.y() {
                for xx in 0..self.renderer.settings.world_size.x() {
                    let block = self.renderer.origin_world[(xx as usize, yy as usize, zz as usize)];
                    if block == 0 {
                        continue;
                    }

                    let block_pos = i16vec3!(xx, yy, zz) * BLOCK_SIZE as i16;
                    self.draw_block(block, &block_pos);
                }
            }
        }
    }

    fn draw_block(&mut self, block: i16, block_pos: &i16vec3) {
        let fpos = vec3!(*block_pos);

        if self.is_block_visible(fpos) {
            self.block_que.push(BlockRenderRequest {
                cam_dist: 0.0,
                block,
                pos: *block_pos,
            });
        }
    }

    fn draw_model(&mut self, model: &MeshModel, trans: &MeshTransform) {
        let model_mesh = self.storage.models.get(*model).unwrap();
        // model size also happens to be >= its bounding box (dont leave voxel padding)
        if self.is_model_visible(&model_mesh.size, trans) {
            self.model_que.push(ModelRenderRequest {
                cam_dist: 0.0,
                mesh: *model,
                trans: *trans,
            });
        }
    }

    fn draw_foliage(&mut self, foliage: &MeshFoliage, pos: &vec3) {
        // foliage is assumed to be somewhat block constrained
        if self.is_block_visible(*pos) {
            self.foliage_que.push(FoliageRenderRequest {
                cam_dist: 0.0,
                mesh: *foliage,
                pos: *pos,
            });
        }
    }

    fn draw_liquid(&mut self, liquid: &MeshLiquid, pos: &vec3) {
        // liquids are assumed to be somewhat block constrained
        if self.is_block_visible(*pos) {
            self.liquid_que.push(LiquidRenderRequest {
                cam_dist: 0.0,
                mesh: *liquid,
                pos: *pos,
            });
        }
    }

    fn draw_volumetric(&mut self, volumetric: &MeshVolumetric, pos: &vec3) {
        // volumetrics are assumed to be somewhat block constrained
        if self.is_block_visible(*pos) {
            self.volumetric_que.push(VolumetricRenderRequest {
                cam_dist: 0.0,
                mesh: *volumetric,
                pos: *pos,
            });
        }
    }

    fn spawn_particle(&mut self, particle: &Particle) {
        self.renderer.particles.push(*particle);
    }

    // fn shift_radiance(&mut self, shift: ivec3) {
    //     self.radiance_shift = shift;
    // }

    fn start_frame(&mut self) {
        // queues are like high-level draw calls, and we are clearing command buffers
        self.block_que.clear();
        self.model_que.clear();
        self.foliage_que.clear();
        self.liquid_que.clear();
        self.volumetric_que.clear();
    }

    // function that "optimizes" the frame
    // it could be implicit, but explicitnesss allows you to maybe do this work in parallel
    // such approach does not really play well with what i do (no multithreading in rendering), but anyways
    fn prepare_frame(&mut self) {
        // self.renderer.update_camera();
        // self.renderer.update_light_transform();
        let cam = self.renderer.camera.camera_transform;
        Self::calculate_and_sort_by_cam_dist(&mut self.model_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.block_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.foliage_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.liquid_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.volumetric_que, cam);
    }

    fn end_frame(&mut self) {
        // yes, started here cause no reason not to group

        self.renderer.start_blockify();
        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh).unwrap();
            self.renderer.blockify_mesh(model_mesh, &mrr.trans);
        }
        self.renderer.end_blockify();
        self.renderer.find_radiance_to_update();
        // you may wonder why is start_frame here, and not in the beginning
        // this is because it contains syncronization, which im trying to delay as much as possible
        // sadly, it does not help when you are CPU-bound (which is the case here). But still useful
        self.renderer.start_frame();
        self.renderer.shift_radiance(self.radiance_shift);
        self.radiance_shift = ivec3::zero();
        self.renderer.update_radiance();
        self.renderer.updade_grass(Default::default());
        self.renderer.updade_water();
        self.renderer.exec_copies();
        self.renderer.start_map();
        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh).unwrap();
            self.renderer.map_mesh(model_mesh, &mrr.trans);
        }
        self.renderer.end_map();
        self.renderer.end_compute();
        self.renderer.start_lightmap();
        self.renderer.lightmap_start_blocks();
        for brr in &self.block_que {
            let ipos = ivec3!(brr.pos);
            self.renderer.lightmap_block(brr.block, ipos);
        }
        self.renderer.lightmap_start_models();
        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh).unwrap();
            self.renderer.lightmap_model(model_mesh, &mrr.trans);
        }
        self.renderer.end_lightmap();
        self.renderer.start_raygen();
        self.renderer.raygen_start_blocks();
        for brr in &self.block_que {
            let ipos = ivec3!(brr.pos);
            self.renderer.raygen_block(brr.block, ipos);
        }
        self.renderer.raygen_start_models();
        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh).unwrap();
            self.renderer.raygen_model(model_mesh, &mrr.trans);
        }
        self.renderer.update_particles();
        self.renderer.raygen_map_particles();
        self.renderer.raygen_start_grass();
        for frr in &self.foliage_que {
            self.renderer.raygen_map_grass(
                &InternalMeshFoliage {
                    stored_id: frr.mesh as u32,
                },
                &frr.pos,
            );
        }
        self.renderer.raygen_start_water();
        for lrr in &self.liquid_que {
            let liquid_mesh = self.storage.liquids.get(lrr.mesh).unwrap();
            self.renderer.raygen_map_water(liquid_mesh, &lrr.pos);
        }
        self.renderer.end_raygen();
        self.renderer.start_2nd_spass();
        self.renderer.diffuse();
        self.renderer.ambient_occlusion();
        self.renderer.glossy_raygen();
        self.renderer.raygen_start_smoke();
        for vrr in &self.volumetric_que {
            let volumetric_mesh = self.storage.volumetrics.get(vrr.mesh).unwrap();
            self.renderer.raygen_map_smoke(volumetric_mesh, &vrr.pos);
        }
        self.renderer.glossy();

        self.renderer.smoke();
        self.renderer.tonemap();
        self.renderer.end_2nd_spass();
        self.renderer.end_frame(self.window.as_ref());
    }

    fn get_world_blocks(&'_ self) -> Array3DView<'_, InternalBlockId, MeshBlock, D> {
        self.renderer.origin_world.as_view()
    }

    fn get_world_blocks_mut(&'_ mut self) -> Array3DViewMut<'_, InternalBlockId, MeshBlock, D> {
        self.renderer.origin_world.as_view_mut()
    }

    fn is_block_visible(&self, pos: vec3) -> bool {
        for xx in 0..2 {
            for yy in 0..2 {
                for zz in 0..2 {
                    let x = xx as f32 * fBLOCK_SIZE;
                    let y = yy as f32 * fBLOCK_SIZE;
                    let z = zz as f32 * fBLOCK_SIZE;

                    // let clip = new_pos / new_pos.w;
                    let new_pos = quat::identity() * pos;
                    let corner = vec4!(new_pos + vec3!(x, y, z), 1.0);
                    let clip = self.renderer.camera.camera_transform * corner;

                    // Note: orth assumes w == 1.0
                    // Check if within NDC range
                    if (clip.x >= -1.0)
                        && (clip.y >= -1.0)
                        && (clip.z >= -1.0)
                        && (clip.x <= 1.0)
                        && (clip.y <= 1.0)
                        && (clip.z <= 1.0)
                    {
                        // if any corner is in NDC range, block is at least partially visible
                        return true;
                    }
                }
            }
        }

        // none corners are in NDC range

        false
    }

    fn is_model_visible(&self, model_size: &uvec3, trans: &MeshTransform) -> bool {
        let min_corner = vec3::zero();

        let max_corner = vec3!(*model_size);

        // Transform the corners

        let mut transformed_corners = [vec3::default(); 8];

        for x in 0..=1 {
            for y in 0..=1 {
                for z in 0..=1 {
                    let corner = vec3!(x, y, z) * max_corner + min_corner;
                    transformed_corners[x + y * 2 + z * 4] =
                        trans.rotation * corner + trans.translation;
                }
            }
        }

        for corner in transformed_corners {
            let mut clip = self.renderer.camera.camera_transform * vec4!(corner, 1.0);

            // Perspective divide (to convert from clip space to NDC)
            // NOTE: i have no idea if it actually helps. TODO:
            if clip.w != 0.0 {
                clip /= clip.w;
            }

            // Check if the point lies within the NDC range
            // i guess i can use GLM for simd but its not bottleneck for now
            // TODO: asm view to imrpove every fun

            if (clip.x >= -1.0)
                && (clip.y >= -1.0)
                && (clip.z >= -1.0)
                && (clip.x <= 1.0)
                && (clip.y <= 1.0)
                && (clip.z <= 1.0)
            {
                // if any corner is in NDC range, block is at least partially visible
                return true;
            }
        }

        // none corners are in NDC range
        false
    }

    fn update_block_palette_to_gpu(&mut self) {
        self.renderer.update_block_palette_to_gpu();
    }

    fn update_material_palette_to_gpu(&mut self) {
        self.renderer.update_material_palette_to_gpu();
    }

    fn get_block_palette(&self) -> &[BlockVoxels] {
        self.renderer.block_palette_voxels.as_slice()
    }

    fn get_block_palette_mut(&mut self) -> &mut [BlockVoxels] {
        self.renderer.block_palette_voxels.as_mut_slice()
    }

    fn get_material_palette(&self) -> &[Material] {
        &self.renderer.material_palette
    }

    fn get_material_palette_mut(&mut self) -> &mut [Material] {
        &mut self.renderer.material_palette
    }
}

// TODO: is there a simpler shorter)way to do this?
pub trait GetPos {
    // returns world-space pos
    fn get_pos(&self) -> vec3;
    fn set_cam_dist(&mut self, cam_dist: f32);
    fn get_cam_dist(&self) -> f32;
}

impl GetPos for ModelRenderRequest {
    fn get_pos(&self) -> vec3 {
        self.trans.translation
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}

impl GetPos for BlockRenderRequest {
    fn get_pos(&self) -> vec3 {
        vec3!(self.pos)
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}
impl GetPos for FoliageRenderRequest {
    fn get_pos(&self) -> vec3 {
        vec3!(self.pos)
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}
impl GetPos for LiquidRenderRequest {
    fn get_pos(&self) -> vec3 {
        vec3!(self.pos)
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}
impl GetPos for VolumetricRenderRequest {
    fn get_pos(&self) -> vec3 {
        vec3!(self.pos)
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}
