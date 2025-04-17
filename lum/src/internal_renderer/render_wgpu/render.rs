use crate::{
    assert_assume,
    containers::BitArray3d,
    internal_renderer::{
        render_wgpu::{BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y},
        *,
    },
};

// use multiversion::multiversion;
use qvek::{i8vec4, ivec3, uvec2, uvec3};
use wgpu::{Extent3d, ImageCopyBuffer, ImageCopyTexture, Origin3d};
use winit::window::Window;

use super::InternalRendererWebGPU;

pub struct FrameContext<'frame> {
    pub encoder: &'frame mut wgpu::CommandEncoder,
    pub render_pass: Option<&'frame mut wgpu::RenderPass<'frame>>,
    pub compute_pass: Option<&'frame mut wgpu::ComputePass<'frame>>,
}

impl<'frame> FrameContext<'frame> {
    pub fn new(encoder: &'frame mut wgpu::CommandEncoder) -> Self {
        Self {
            encoder,
            render_pass: None,
            compute_pass: None,
        }
    }
}

// i am clearly trash with managing division into files
// if someone has a good idea on how to do it, message me (or just make a PR)
impl<'window> InternalRendererWebGPU<'window> {
    async fn new(
        lum_settings: &Settings,
        window: Window,
        // event_loop: &winit::event_loop::EventLoop<()>,
        foliage_descriptions: Vec<InternalMeshFoliageDesc>,
    ) -> Self {
        InternalRendererWebGPU::create(lum_settings, window, foliage_descriptions).await
    }

    pub fn update_camera(&mut self) {
        self.camera.update_camera();
    }

    pub fn update_light_transform(&mut self) {
        self.light.update_light_transform(self.settings.world_size);
        // let horizon =
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

    // // allocates temp block in palette for every block that intersects with every mesh blockified
    // pub fn blockify_mesh(
    //     &mut self,
    //     mesh: &InternalMeshModel<Option<wgpu::Buffer>, Option<wal::Image>>,
    //     trans: &MeshTransform,
    // ) {
    //     let rotate = mat4::from(trans.rotation);
    //     let shift = mat4::identity().translated_3d(trans.translation);
    //     let border_in_voxel = get_shift(shift * rotate, mesh.total_size);

    //     let mut border = iAABB {
    //         min: ivec3!(border_in_voxel.min - 1.0) / 16,
    //         max: ivec3!(border_in_voxel.max + 1.0) / 16,
    //     };

    //     // clamp to world size so no out of bounds
    //     border.min = ivec3::clamped(
    //         border.min,
    //         ivec3::zero(),
    //         ivec3!(self.settings.world_size - 1),
    //     );
    //     border.max = ivec3::clamped(
    //         border.max,
    //         ivec3::zero(),
    //         ivec3!(self.settings.world_size - 1),
    //     );

    //     for zz in border.min.z..=border.max.z {
    //         for yy in border.min.y..=border.max.y {
    //             for xx in border.min.x..=border.max.x {
    //                 let current_block = self.current_world[(xx as usize, yy as usize, zz as usize)];
    //                 if (current_block as u32) < self.static_block_palette_size {
    //                     // static
    //                     //add to copy queue
    //                     let src_block = self.index_block_xy(current_block as usize);
    //                     let dst_block = self.index_block_xy(self.palette_counter);

    //                     // do image copy on for non-zero-src blocks. Other things still done for every allocated block
    //                     // because zeroing is fast
    //                     if current_block != 0 {
    //                         let static_block_copy = vk::ImageCopy {
    //                             src_subresource: vk::ImageSubresourceLayers {
    //                                 aspect_mask: vk::ImageAspectFlags::COLOR,
    //                                 mip_level: 0,
    //                                 base_array_layer: 0,
    //                                 layer_count: 1,
    //                             },
    //                             src_offset: vk::Offset3D {
    //                                 x: src_block.x as i32 * 16,
    //                                 y: src_block.y as i32 * 16,
    //                                 z: 0,
    //                             },
    //                             dst_subresource: vk::ImageSubresourceLayers {
    //                                 aspect_mask: vk::ImageAspectFlags::COLOR,
    //                                 mip_level: 0,
    //                                 base_array_layer: 0,
    //                                 layer_count: 1,
    //                             },
    //                             dst_offset: vk::Offset3D {
    //                                 x: dst_block.x as i32 * 16,
    //                                 y: dst_block.y as i32 * 16,
    //                                 z: 0,
    //                             },
    //                             extent: vk::Extent3D {
    //                                 width: 16,
    //                                 height: 16,
    //                                 depth: 16,
    //                             },
    //                         };
    //                         // TODO: more compact representation
    //                         self.block_copies_queue.push((
    //                             static_block_copy.src_subresource,
    //                             static_block_copy.dst_subresource,
    //                             static_block_copy.extent,
    //                         ));
    //                     }

    //                     self.current_world[(xx as usize, yy as usize, zz as usize)] =
    //                         self.palette_counter as BlockId;
    //                     self.palette_counter += 1;
    //                 } else {
    //                     //already new block, just leave it
    //                 }
    //             }
    //         }
    //     }
    // }

    /// Copies the entire world state to the staging buffer.
    pub fn end_blockify(&mut self) {
        let (dim_x, dim_y, dim_z) = self.current_world.dimensions();
        let count_to_copy = dim_x * dim_y * dim_z;
        // Cast the current world data to a byte slice.
        let data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                self.current_world.data.as_ptr() as *const u8,
                count_to_copy * size_of::<BlockId>(),
            )
        };
        // Write the data to the staging_world buffer.
        self.wal.queue.write_buffer(&self.buffers.staging_world.current(), 0, data);
        // No explicit flush is required in WGPU.
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

    // starts the stage where you can "request drawing" things
    // under the hood it prepares Vulkan for recording draw calls
    /// Begins the frame—updates camera/light transforms and creates a new command encoder.
    pub fn start_frame(&mut self) {
        self.update_camera();
        self.update_light_transform();

        // In WGPU you typically create one command encoder per frame.
        // Save the encoder to a field (assumed to be Option<wgpu::CommandEncoder>).
        self.current_encoder = Some(self.wal.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Frame Command Encoder"),
            },
        ));
    }

    // #[multiversion(targets("x86_64+avx2"))]
    #[optimize(speed)]
    /// Updates the radiance field by copying staging data, dispatching compute work, and setting push constants.
    pub fn _update_radiance(&mut self) {
        // Get the current command encoder.
        let encoder = self
            .current_encoder
            .as_mut()
            .expect("Command encoder should be created in start_frame");

        // Copy radiance_updates from CPU memory to staging buffer.
        let count_to_copy = self.radiance_updates.len();
        let size_to_copy = (count_to_copy * size_of::<i8vec4>()) as u64;
        let data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                self.radiance_updates.as_ptr() as *const u8,
                count_to_copy * size_of::<i8vec4>(),
            )
        };
        self.wal
            .queue
            .write_buffer(&self.buffers.staging_radiance_updates.first(), 0, data);

        // Record a buffer copy from the staging buffer to the GPU radiance updates buffer.
        if count_to_copy > 0 {
            encoder.copy_buffer_to_buffer(
                &self.buffers.staging_radiance_updates.first(),
                0,
                &self.buffers.gpu_radiance_updates.first(),
                0,
                size_to_copy,
            );
        }
        // No explicit barriers are needed in WGPU.

        // Begin a compute pass.
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Radiance Compute Pass"),
            timestamp_writes: None,
        });
        // Bind the compute pipeline.
        let bind_group = self.pipes.radiance_pipe.bind_groups.as_ref().unwrap().current();
        compute_pass.set_pipeline(self.pipes.radiance_pipe.pipeline.as_ref().unwrap());
        compute_pass.set_bind_group(0, Some(bind_group), &[]);

        #[repr(C)]
        #[derive(Clone, Copy, as_u8_slice_derive::AsU8Slice)]
        struct PushConstant {
            time: i32,
            iters: i32,
        }
        let push_constant = PushConstant {
            // time: self.frame_counter,
            time: 1,
            iters: 0,
        };
        compute_pass.set_push_constants(0, push_constant.as_u8_slice());

        // Dispatch the compute work.
        let workgroup_count = count_to_copy as u32;
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    /// Shifts the radiance cache texture content by copying a region from the "current" image
    /// to the "previous" image and then copying it back with an offset.
    pub fn shift_radiance(&mut self, radiance_shift: ivec3) {
        // Retrieve the command encoder (assumed to be stored in self.current_encoder).
        let encoder = self
            .current_encoder
            .as_mut()
            .expect("Must have a command encoder active during shift_radiance");

        // Compute the effective shift.
        let cam_shift = radiance_shift;

        // If the shift in any axis is greater than or equal to world size, nothing is done.
        if cam_shift.x.abs() >= self.settings.world_size.x as i32
            || cam_shift.y.abs() >= self.settings.world_size.y as i32
            || cam_shift.z.abs() >= self.settings.world_size.z as i32
        {
            return;
        }

        // Compute source and destination offsets along each axis.
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

        // Compute the intersection size.
        let intersection_size = self.settings.world_size
            - uvec3!(
                cam_shift.x.unsigned_abs(),
                cam_shift.y.unsigned_abs(),
                cam_shift.z.unsigned_abs()
            );

        let copy_extent = Extent3d {
            width: intersection_size.x,
            height: intersection_size.y,
            depth_or_array_layers: intersection_size.z,
        };

        // First, copy from the current radiance cache to the previous one.
        // In WGPU, we use copy_texture_to_texture. (No explicit barriers are needed.)
        let src_copy = ImageCopyTexture {
            texture: &self.independent_images.radiance_cache.current().texture,
            mip_level: 0,
            origin: Origin3d {
                x: self_src_offset.x as u32,
                y: self_src_offset.y as u32,
                z: self_src_offset.z as u32,
            },
            aspect: wgpu::TextureAspect::All,
        };
        let dst_copy = ImageCopyTexture {
            texture: &self.independent_images.radiance_cache.previous().texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        encoder.copy_texture_to_texture(src_copy, dst_copy, copy_extent);

        // Then, copy back from the previous image to the current one with a destination offset.
        let src_back = ImageCopyTexture {
            texture: &self.independent_images.radiance_cache.previous().texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        let dst_back = ImageCopyTexture {
            texture: &self.independent_images.radiance_cache.current().texture,
            mip_level: 0,
            origin: Origin3d {
                x: self_dst_offset.x as u32,
                y: self_dst_offset.y as u32,
                z: self_dst_offset.z as u32,
            },
            aspect: wgpu::TextureAspect::All,
        };
        encoder.copy_texture_to_texture(src_back, dst_back, copy_extent);
    }

    /// Executes various copies:
    /// 1. Clears the current origin block palette texture.
    /// 2. Copies a static block palette region.
    /// 3. Executes a queue of additional block copies.
    /// 4. Copies the world buffer to the world texture.
    pub fn exec_copies(&mut self) {
        let encoder = self
            .current_encoder
            .as_mut()
            .expect("Command encoder should be active during exec_copies");

        // Clear the current origin block palette.
        // In WGPU, a texture can be cleared via a render pass.
        {
            let clear_color = wgpu::Color::default();
            // Assume self.independent_images.origin_block_palette.current_view() returns a &wgpu::TextureView.
            let view = &self.independent_images.origin_block_palette.current().view;
            let rp_desc = wgpu::RenderPassDescriptor {
                label: Some("Clear OriginBlockPalette"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            };
            {
                // Begin and immediately end a render pass to clear.
                encoder.begin_render_pass(&rp_desc);
            }
        }

        // Copy the static block palette region.
        {
            let copy_extent = Extent3d {
                width: 16 * self.static_block_palette_size,
                height: 16,
                depth_or_array_layers: 16,
            };
            let src = ImageCopyTexture {
                texture: &self.independent_images.origin_block_palette.previous().texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            };
            let dst = ImageCopyTexture {
                texture: &self.independent_images.origin_block_palette.current().texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            };
            encoder.copy_texture_to_texture(src, dst, copy_extent);
        }

        // Execute additional block copy commands if any.
        if !self.block_copies_queue.is_empty() {
            for (src, dst, region) in self.block_copies_queue.iter() {
                encoder.copy_texture_to_texture(*src, *dst, *region);
            }
        }

        // Finally, copy the world buffer to the world texture.
        {
            // We assume that self.buffers.world_buffer is a wgpu::Buffer.
            let bytes_per_row = self.settings.world_size.x * std::mem::size_of::<BlockId>() as u32;
            let layout = wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(self.settings.world_size.y),
            };
            let buffer_copy = ImageCopyBuffer {
                buffer: self.buffers.staging_world.current(),
                layout,
            };
            let dst = ImageCopyTexture {
                texture: &self.independent_images.world.current().texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            };
            let extent = Extent3d {
                width: self.settings.world_size.x,
                height: self.settings.world_size.y,
                depth_or_array_layers: self.settings.world_size.z,
            };
            encoder.copy_buffer_to_texture(buffer_copy, dst, extent);
        }
    }
}

fn process_axis(shift: i32, _world_size: i32) -> ivec2 {
    if shift >= 0 {
        ivec2::new(shift, 0)
    } else {
        ivec2::new(0, shift.abs())
    }
}
