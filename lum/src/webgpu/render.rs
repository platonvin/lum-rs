use super::InternalRendererWebGPU;
use crate::{
    aabb::{get_shift, iAABB},
    ao_lut, assert_assume, fBLOCK_SIZE,
    load_interface::{BlockData, LoadInterface, ModelData},
    render_interface::{FoliageDescriptionBuilder, RendererInterface},
    types::{
        i16vec3, i8vec3, ivec3, ivec4, mat4, quat, u8vec3, u8vec4, uvec3, vec2, vec3, vec4, AoLut,
        MeshTransform, *,
    },
    webgpu::{
        all_resources::all_types::UboData, types::*, wal::Wal, MeshFoliageDesc,
        BLOCK_PALETTE_SIZE_X, BLOCK_PALETTE_SIZE_Y,
    },
    Settings, BLOCK_SIZE,
};
use as_u8_slice_derive::AsU8Slice;
use containers::{
    array3d::{Array3DView, Array3DViewMut, Dim3},
    Arena, BitArray3d,
};
use qvek::{
    i16vec3, ivec3, ivec4, uvec2, uvec3, vec2, vec3, vec4,
    vek::{Clamp, Vec3},
};
use std::mem::{size_of, transmute};
use wgpu::{
    BufferSize, Color, ComputePassDescriptor, Extent3d, Origin3d, TexelCopyBufferInfo,
    TexelCopyTextureInfo, COPY_BYTES_PER_ROW_ALIGNMENT,
};
use winit::{dpi::PhysicalSize, window::Window};

// i am clearly trash with managing division into files
// if someone has a good idea on how to do it, message me (or just make a PR)
impl<'window, D: Dim3> InternalRendererWebGPU<'window, D> {
    pub fn update_camera(&mut self) {
        self.camera.update_camera(true); // wgpu has worst possible y orientation
                                         // like, why would we have Y flipped for UV coords relative to clip space?
                                         // Whats next - make (0,0) at (0.23, -0.55) and rotate each axis by 5.01∘ ?
    }

    pub fn update_light_transform(&mut self) {
        self.light.update_light_transform(self.settings.world_size, false);
    }

    pub fn start_blockify(&mut self) {
        self.block_copies_queue.clear();
        self.palette_counter = self.static_block_palette_size as usize;

        // reset the current world to the origin
        // basically clears allocated blocks (keeps static blocks)
        self.current_world.copy_data_from(&self.origin_world);
    }

    pub fn index_block_xy(&self, n: usize) -> uvec2 {
        let x = n % BLOCK_PALETTE_SIZE_X as usize;
        let y = n / BLOCK_PALETTE_SIZE_X as usize;
        debug_assert!(y <= BLOCK_PALETTE_SIZE_Y as usize);
        uvec2!(x, y)
    }

    /// CPU-only function that determines which blocks need light to be updated and adds them to queue (of which blocks to update)
    /// Note: this is the last function that can be called before Vulkan interraction
    /// which means that you HAVE to wait at most after it
    pub fn find_radiance_to_update(&mut self) {
        // somehow caching allocated is slower... TODO:
        // explanation: i used to avoid new allocation by reusing memory
        // but somehow thats slower than freeing and allocating new memory every frame

        // TODO: optimize with "spreading" / "blurring" apporach where we do 3 (X,Y,Z) passes with 1x1x3 kernel

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

        self.radiance_updates.resize(pushed_radiance_count as usize, ivec4::zero());

        let mut i = 0;
        for zz in 0..self.settings.world_size.z() {
            for yy in 0..self.settings.world_size.y() {
                for xx in 0..self.settings.world_size.x() {
                    if visited.get(xx as usize, yy as usize, zz as usize) {
                        assert_assume!(i < self.radiance_updates.len());
                        self.radiance_updates[i] = ivec4!(xx, yy, zz, 0);
                        i += 1;
                    }
                }
            }
        }

        // Special updates are ones requested via API
        // If not already picked for an update, add it to the queue
        self.radiance_updates.extend(
            self.special_radiance_updates
                .iter()
                .filter(|u| !visited.get(u.x as usize, u.y as usize, u.z as usize)),
        );

        drop(visited);
    }

    /// Starts the stage where you can "request drawing" things
    pub fn start_frame(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let now = std::time::Instant::now();
            let delta = now - self.last_time;
            self.delta_time = (delta.subsec_nanos() as f64 / 1e9_f64) as f32;
            self.last_time = now;
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.delta_time = 15.0 / 1000.0;
        }

        self.update_camera();
        self.update_light_transform();

        // we use single encoder for the entire frame
        // this is essentially just a command buffer
        self.current_encoder = Some(self.wal.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Frame Command Encoder"),
            },
        ));
    }

    /// Updates the radiance field (per-block lighting) by copying staging data and dispatching compute work.
    fn update_radiance(&mut self) {
        // Copy radiance_updates from CPU memory to staging buffer.
        let count_to_copy = self.radiance_updates.len();
        let size_to_copy = count_to_copy * size_of::<ivec4>();
        let data: &[u8] = unsafe {
            std::slice::from_raw_parts(self.radiance_updates.as_ptr() as *const u8, size_to_copy)
        };

        // Record a buffer copy from the staging buffer to the GPU radiance updates buffer.
        if count_to_copy > 0 {
            let mut write = self.wal.queue.write_buffer_with(
                self.buffers.gpu_radiance_updates.current(),
                0,
                std::num::NonZeroU64::new(size_to_copy as u64).unwrap(),
            );

            write.as_mut().unwrap().copy_from_slice(data);

            drop(write);
        }

        // Begin a compute pass.
        let mut compute_pass = self
            .current_encoder
            .as_mut()
            .unwrap()
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Radiance Compute Pass"),
                timestamp_writes: None,
            })
            .forget_lifetime();

        // Dispatch the compute work.

        self.wal.bind_compute_pipeline(&mut compute_pass, &self.pipes.radiance_pipe);

        let workgroup_count = count_to_copy as u32;

        self.wal.dispatch_with_params(
            &mut compute_pass,
            &mut self.pipes.radiance_pipe,
            None,
            workgroup_count,
            1,
            1,
        );
    }

    /// Shifts the radiance cache texture content by copying a region from the "current" image
    /// to the "previous" image and then copying it back with an offset.
    /// This exists because Lum operates on small-size world
    /// and you are supposed to project your big world to Lum world (and also position camera relative to projection)
    /// and moving this "sliding window" makes light move in opposite direction, which we fix by shifting it back
    /// (this leaves gaps in light on borders, but we kinda just hope they will get updated)
    pub fn shift_radiance(&mut self, radiance_shift: ivec3) {
        let cam_shift = radiance_shift;

        // If the shift in any axis is greater than or equal to world size, nothing is done.
        if cam_shift.x.abs() >= self.settings.world_size.x() as i32
            || cam_shift.y.abs() >= self.settings.world_size.y() as i32
            || cam_shift.z.abs() >= self.settings.world_size.z() as i32
        {
            return;
        }

        // Compute source and destination offsets along each axis.
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

        // Compute the intersection size of new "sliding window" position and old one
        let intersection_size: uvec3 = uvec3!(self.settings.world_size.xyz())
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

        // We have just "move_next"ed, and updated (most recent) radiance_cache image is previous().
        // Not updated (so outdated) is current() and we can safely overwrite it (use as temp storage as we do here).
        // previous() will be used for generating new current(),
        // and then they will be swapped in the end of the frame (with move_next()).

        // First, copy the previous() (aka latest) radiance cache to the current() (aka temp storage) one
        let src_copy = TexelCopyTextureInfo {
            texture: &self.independent_images.radiance_cache.previous().texture,
            mip_level: 0,
            origin: Origin3d {
                x: self_src_offset.x as u32,
                y: self_src_offset.y as u32,
                z: self_src_offset.z as u32,
            },
            aspect: wgpu::TextureAspect::All,
        };
        let dst_copy = TexelCopyTextureInfo {
            texture: &self.independent_images.radiance_cache.current().texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        self.current_encoder.as_mut().unwrap().copy_texture_to_texture(
            src_copy,
            dst_copy,
            copy_extent,
        );

        // Then, copy back from the current() (aka temp storage) image to the previous() (aka latest) one with a destination offset.
        let src_back = TexelCopyTextureInfo {
            texture: &self.independent_images.radiance_cache.current().texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        let dst_back = TexelCopyTextureInfo {
            texture: &self.independent_images.radiance_cache.previous().texture,
            mip_level: 0,
            origin: Origin3d {
                x: self_dst_offset.x as u32,
                y: self_dst_offset.y as u32,
                z: self_dst_offset.z as u32,
            },
            aspect: wgpu::TextureAspect::All,
        };

        self.current_encoder.as_mut().unwrap().copy_texture_to_texture(
            src_back,
            dst_back,
            copy_extent,
        );
    }

    pub fn exec_copies(&mut self) {
        // TODO: we assume that static block palettes only take the first raw, which is a bad assumption
        assert!(self.static_block_palette_size < BLOCK_PALETTE_SIZE_X);

        // At this point, we have written to block_palette.current() in map_meshes
        // then we "move_next"ed the block_palette.
        // (cause its faster to zero entire image and copy static block_palette part back)
        // so we put data back by copying current() to previous() and now both are purely static palette

        // Copy the static block palette region from untouched image .
        let copy_extent = Extent3d {
            // TODO: we assume that static block palettes only take the first raw, which is a bad assumption
            width: BLOCK_SIZE * self.static_block_palette_size,
            height: BLOCK_SIZE,
            depth_or_array_layers: BLOCK_SIZE,
        };
        let src = TexelCopyTextureInfo {
            texture: &self.independent_images.block_palette.previous().texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        let dst = TexelCopyTextureInfo {
            texture: &self.independent_images.block_palette.current().texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        self.current_encoder
            .as_mut()
            .unwrap()
            .copy_texture_to_texture(src, dst, copy_extent);

        // Execute all the requested copies (for "duplicating" blocks data in block palette.
        // We need this to write model voxels to world)
        for (src, dst, region) in self.block_copies_queue.iter() {
            self.current_encoder
                .as_mut()
                .unwrap()
                .copy_texture_to_texture(*src, *dst, *region);
        }

        // Finally, copy the world buffer to the world texture.
        let bytes_per_row =
            (self.settings.world_size.x() * std::mem::size_of::<MeshBlock>()) as u32;

        //TODO: idk pad this
        let padded_bytes_per_row = bytes_per_row.next_multiple_of(COPY_BYTES_PER_ROW_ALIGNMENT);

        let layout = wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(padded_bytes_per_row),
            rows_per_image: Some(self.settings.world_size.y() as u32),
            // rows_per_image: None, // not required?
        };
        let buffer_copy = TexelCopyBufferInfo {
            buffer: self.buffers.staging_world.current(),
            layout,
        };
        let dst = TexelCopyTextureInfo {
            texture: &self.independent_images.world.current().texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        let extent = Extent3d {
            width: self.settings.world_size.x() as u32,
            height: self.settings.world_size.y() as u32,
            depth_or_array_layers: self.settings.world_size.z() as u32,
        };

        self.current_encoder
            .as_mut()
            .unwrap()
            .copy_buffer_to_texture(buffer_copy, dst, extent);
    }

    pub fn gen_perlin_noises(&mut self) {
        let mut encoder = self.wal.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("perlin noise encoder"),
        });

        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("perlin noise generation pass"),
            timestamp_writes: None,
        });

        self.wal.bind_compute_pipeline(&mut compute_pass, &self.pipes.gen_perlin2d_pipe);

        self.wal.dispatch_with_params(
            &mut compute_pass,
            &mut self.pipes.gen_perlin2d_pipe,
            None,
            self.origin_world.dims.x().div_ceil(8) as u32,
            self.origin_world.dims.y().div_ceil(8) as u32,
            1,
        );

        self.wal.bind_compute_pipeline(&mut compute_pass, &self.pipes.gen_perlin3d_pipe);

        self.wal.dispatch_with_params(
            &mut compute_pass,
            &mut self.pipes.gen_perlin3d_pipe,
            None,
            32 / 4,
            32 / 4,
            32 / 4,
        );

        drop(compute_pass);
        self.wal.queue.submit([encoder.finish()]);
    }
}

fn process_axis(shift: i32, _world_size: i32) -> ivec2 {
    if shift >= 0 {
        ivec2::new(shift, 0)
    } else {
        ivec2::new(0, shift.abs())
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

#[derive(Clone)]
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
pub struct RendererWgpu<'window, D: Dim3> {
    pub renderer: InternalRendererWebGPU<'window, D>,
    pub block_que: Vec<BlockRenderRequest>,
    pub model_que: Vec<ModelRenderRequest>,
    pub liquid_que: Vec<LiquidRenderRequest>,
    pub volumetric_que: Vec<VolumetricRenderRequest>,
    // not a queue - collection of queues
    pub foliage_ques: Vec<Vec<FoliageRenderRequest>>,

    pub storage: RendererStorage,
    // how much shoudl we move radiance data in radiance image. Used for moving camera (otherwise light is late)
    pub radiance_shift: ivec3,
}

impl<'window, D: Dim3> RendererWgpu<'window, D> {
    pub fn destroy(self) {
        // unsafe { self.renderer.destroy() };
    }

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

    fn diffuse(&mut self) {
        // Begin the diffuse render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Diffuse Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        let pipe = &mut self.renderer.pipes.diffuse_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            pipe.line.as_ref().unwrap(),
            pipe.static_bind_groups.as_ref(),
        );

        // Draw fullscreen triangle
        rpass.draw(0..3, 0..1);
    }

    fn ambient_occlusion(&mut self) {
        // Begin the ambient occlusion render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Ambient Occlusion Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );
        let pipe = &mut self.renderer.pipes.ao_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            pipe.line.as_ref().unwrap(),
            pipe.static_bind_groups.as_ref(),
        );

        // Draw fullscreen triangle
        rpass.draw(0..3, 0..1);
    }

    fn raygen_glossy(&mut self) {
        // Begin the glossy raygen render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Glossy Raygen Render Pass"),
                // not really writing any color
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().stencil.view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        rpass.set_stencil_reference(0x01);

        let pipe = &mut self.renderer.pipes.fill_stencil_glossy_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            pipe.line.as_ref().unwrap(),
            pipe.static_bind_groups.as_ref(),
        );

        rpass.set_stencil_reference(0x01);

        // Draw fullscreen triangle
        rpass.draw(0..3, 0..1);
    }

    fn raygen_smoke(&mut self) {
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Smoke Raygen Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.renderer.dependent_images.as_ref().unwrap().near_depth.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color {
                                // just high enough numbers
                                r: 10000.0,
                                g: 10000.0,
                                b: 10000.0,
                                a: 10000.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.renderer.dependent_images.as_ref().unwrap().far_depth.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color {
                                // just negative-high enough numbers
                                r: -10000.0,
                                g: -10000.0,
                                b: -10000.0,
                                a: -10000.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().stencil.view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        // not clear because THAT WOULD FUCKING OVERRIDE THE GLOSSY RAYGEN
                        // TODO: build the system where i dont have to track these things
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        let pipe = &mut self.renderer.pipes.fill_stencil_smoke_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            pipe.pipe.line.as_ref().unwrap(),
            pipe.pipe.static_bind_groups.as_ref(),
        );

        rpass.set_stencil_reference(0x02);

        for vrr in &self.volumetric_que {
            #[repr(C)]
            #[derive(Clone, Copy, AsU8Slice)]
            struct PushConstant {
                center_size: vec4,
            }
            let push_constant = PushConstant {
                center_size: vec4!(vrr.pos, fBLOCK_SIZE),
            };

            pipe.push_constants.extend_from_slice(push_constant.as_u8_slice());
            pipe.pc_count += 1;
        }

        if pipe.pc_count > 0 {
            let count = pipe.push_constants.len();
            let write = self.renderer.wal.queue.write_buffer_with(
                pipe.pc_buffer.as_ref().unwrap(),
                0,
                std::num::NonZero::new(count as u64).unwrap(),
            );
            let src_pc_slice_u8 =
                unsafe { std::slice::from_raw_parts(pipe.push_constants.as_ptr(), count) };
            write.unwrap().copy_from_slice(src_pc_slice_u8);

            self.renderer.wal.draw_with_params(
                &mut rpass,
                Some(pipe.pipe.static_bind_groups.as_ref().unwrap().current()),
                pipe.pc_bg.as_ref(),
                0..36,
                0..pipe.pc_count as u32,
            );

            pipe.push_constants.clear();
            pipe.pc_count = 0;
        }
    }

    fn glossy(&mut self) {
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Glossy Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().stencil.view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        let pipe = &mut self.renderer.pipes.glossy_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            pipe.line.as_ref().unwrap(),
            pipe.static_bind_groups.as_ref(),
        );

        rpass.set_stencil_reference(0x01);

        rpass.draw(0..3, 0..1);
    }

    fn smoke(&mut self) {
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Glossy Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().stencil.view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        let pipe = &mut self.renderer.pipes.smoke_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            pipe.line.as_ref().unwrap(),
            pipe.static_bind_groups.as_ref(),
        );

        rpass.set_stencil_reference(0x02);

        rpass.draw(0..3, 0..1);
    }

    fn tonemap(&mut self) {
        let cmb = self.renderer.current_encoder.take().unwrap().finish();
        // sub all prev commands
        self.renderer.wal.queue.submit([cmb]);

        let mut current_encoder =
            self.renderer
                .wal
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Frame Command Encoder"),
                });

        // Create texture view
        let swapchain_texture = self
            .renderer
            .wal
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let swapchain_view = swapchain_texture.texture.create_view(&wgpu::TextureViewDescriptor {
            // Without add_srgb_suffix() the image we will be working with
            // might not be "gamma correct".
            format: Some(self.renderer.wal.config.format),
            // format: Some(self.renderer.wal.config.format.add_srgb_suffix()),
            ..Default::default()
        });
        {
            let mut rpass = current_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tonemap Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &swapchain_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let pipe = &mut self.renderer.pipes.tonemap_pipe;

            self.renderer.wal.bind_raster_pipeline(
                &mut rpass,
                pipe.line.as_ref().unwrap(),
                pipe.static_bind_groups.as_ref(),
            );

            // Draw fullscreen triangle
            rpass.draw(0..3, 0..1);
        }
        self.renderer.wal.queue.submit(Some(current_encoder.finish()));
        // turns out it does not present on drop. I thought it makes sense
        swapchain_texture.present();
    }

    fn move_next(&mut self) {
        self.renderer.buffers.staging_world.move_next();
        self.renderer.buffers.light_uniform.move_next();
        self.renderer.buffers.uniform.move_next();
        self.renderer.buffers.ao_lut_uniform.move_next();
        self.renderer.buffers.gpu_radiance_updates.move_next();
        // self.renderer.buffers.staging_radiance_updates.move_next();
        // self.renderer.buffers.gpu_particles_staged.move_next();
        self.renderer.buffers.gpu_particles.move_next();

        // self.renderer.independent_images.grass_state.move_next();
        // self.renderer.independent_images.water_state.move_next();
        // self.renderer.independent_images.perlin_noise2d.move_next();
        // self.renderer.independent_images.perlin_noise3d.move_next();
        self.renderer.independent_images.world.move_next();
        self.renderer.independent_images.radiance_cache.move_next();
        self.renderer.independent_images.block_palette.move_next();
        // self.renderer.independent_images.material_palette.move_next();
        // self.renderer.independent_images.lightmap.move_next();

        self.renderer
            .pipes
            .lightmap_blocks_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .lightmap_models_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .raygen_blocks_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .raygen_models_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .raygen_particles_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .raygen_water_pipe
            .pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .diffuse_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer.pipes.ao_pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());

        self.renderer
            .pipes
            .fill_stencil_glossy_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .fill_stencil_smoke_pipe
            .pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .glossy_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .smoke_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .tonemap_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .radiance_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .map_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .update_grass_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .update_water_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .gen_perlin2d_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .gen_perlin3d_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        for foliage_pipe in self.renderer.pipes.raygen_foliage_pipes.iter_mut() {
            foliage_pipe.pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());
        }
    }

    fn blockify_models(&mut self) {
        self.renderer.start_blockify();
        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh).unwrap();
            {
                let this = &mut self.renderer;
                let trans: &MeshTransform = &mrr.trans;
                let rotate = mat4::from(trans.rotation);
                let shift = mat4::identity().translated_3d(trans.translation);
                let border_in_voxel = get_shift(shift * rotate, model_mesh.size);

                let mut border = iAABB {
                    min: ivec3!(border_in_voxel.min - 1.0) / BLOCK_SIZE as i32,
                    max: ivec3!(border_in_voxel.max + 1.0) / BLOCK_SIZE as i32,
                };

                // clamp to world size so no out of bounds
                border.min = ivec3::clamped(
                    border.min,
                    ivec3::zero(),
                    ivec3!(this.settings.world_size.xyz() - 1),
                );
                border.max = ivec3::clamped(
                    border.max,
                    ivec3::zero(),
                    ivec3!(this.settings.world_size.xyz() - 1),
                );

                for zz in border.min.z..=border.max.z {
                    for yy in border.min.y..=border.max.y {
                        for xx in border.min.x..=border.max.x {
                            let current_block =
                                this.current_world[(xx as usize, yy as usize, zz as usize)];
                            if (current_block as u32) < this.static_block_palette_size {
                                // static
                                //add to copy queue
                                let _src_block = this.index_block_xy(current_block as usize);
                                let _dst_block = this.index_block_xy(this.palette_counter);

                                // do image copy on for non-zero-src blocks. Other things still done for every allocated block
                                // because zeroing is fast
                                if current_block != 0 {
                                    // Create a command encoder for copying
                                    // let mut encoder = this.wal.device.create_command_encoder(
                                    //     &wgpu::CommandEncoderDescriptor {
                                    //         label: Some("Block Copy Command Encoder"),
                                    //     },
                                    // );

                                    // Copy the block data
                                    // encoder.copy_texture_to_texture(
                                    //     wgpu::TexelCopyTextureInfo {
                                    //         texture: &this
                                    //             .dependent_images
                                    //             .as_ref()
                                    //             .unwrap()
                                    //             .highres_mat_norm
                                    //             .current()
                                    //             .texture,
                                    //         mip_level: 0,
                                    //         origin: wgpu::Origin3d {
                                    //             x: src_block.x as u32 * 16,
                                    //             y: src_block.y as u32 * 16,
                                    //             z: 0,
                                    //         },
                                    //         aspect: wgpu::TextureAspect::All,
                                    //     },
                                    //     wgpu::TexelCopyTextureInfo {
                                    //         texture: &this
                                    //             .dependent_images
                                    //             .as_ref()
                                    //             .unwrap()
                                    //             .highres_mat_norm
                                    //             .current()
                                    //             .texture,
                                    //         mip_level: 0,
                                    //         origin: wgpu::Origin3d {
                                    //             x: dst_block.x as u32 * 16,
                                    //             y: dst_block.y as u32 * 16,
                                    //             z: 0,
                                    //         },
                                    //         aspect: wgpu::TextureAspect::All,
                                    //     },
                                    //     wgpu::Extent3d {
                                    //         width: 16,
                                    //         height: 16,
                                    //         depth_or_array_layers: 1,
                                    //     },
                                    // );
                                    // this.current_encoder.unwrap().copy_texture_to_texture();

                                    // Submit the copy command
                                    // this.current_encoder.unwrap().texte;
                                }

                                this.current_world[(xx as usize, yy as usize, zz as usize)] =
                                    this.palette_counter as InternalBlockId;
                                this.palette_counter += 1;
                            } else {
                                //already new block, just leave it
                            }
                        }
                    }
                }
            };
        }
        {
            let (dim_x, dim_y, dim_z) = self.renderer.current_world.dimensions();
            let padded_dim_x = (dim_x).next_multiple_of(
                COPY_BYTES_PER_ROW_ALIGNMENT as usize / size_of::<InternalBlockId>(),
            );
            let padded_count_to_copy = padded_dim_x * dim_y * dim_z;

            let mut padded_data: Vec<InternalBlockId> = vec![0; padded_count_to_copy];
            for zz in 0..dim_z {
                for yy in 0..dim_y {
                    for xx in 0..dim_x {
                        let index = xx + yy * padded_dim_x + zz * padded_dim_x * dim_y;
                        padded_data[index] =
                            self.renderer.current_world[(xx, yy, zz)] as InternalBlockId;
                    }
                }
            }

            let size_to_copy = padded_count_to_copy * size_of::<InternalBlockId>();
            let data: &[u8] = unsafe {
                std::slice::from_raw_parts(padded_data.as_ptr() as *const u8, size_to_copy)
            };

            let mut write = self.renderer.wal.queue.write_buffer_with(
                self.renderer.buffers.staging_world.current(),
                0,
                std::num::NonZeroU64::new(size_to_copy as u64).unwrap(),
            );

            write.as_mut().unwrap().copy_from_slice(data);

            drop(write);
        };
    }

    fn updade_grass(&mut self, _wind_direction: vec2) {
        let mut encoder =
            self.renderer
                .wal
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Grass Update Command Encoder"),
                });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Grass Update Compute Pass"),
            timestamp_writes: None,
        });

        // bind pipe and its static bindgroups
        self.renderer
            .wal
            .bind_compute_pipeline(&mut compute_pass, &self.renderer.pipes.update_grass_pipe);

        // Dispatch workgroups
        compute_pass.dispatch_workgroups(
            (self.renderer.settings.world_size.x() * 2).div_ceil(8) as u32,
            (self.renderer.settings.world_size.y() * 2).div_ceil(8) as u32,
            1,
        );

        drop(compute_pass);
        self.renderer.wal.queue.submit(Some(encoder.finish()));
    }

    fn updade_water(&mut self) {
        let mut compute_pass = self.renderer.current_encoder.as_mut().unwrap().begin_compute_pass(
            &wgpu::ComputePassDescriptor {
                label: Some("Water Update Compute Pass"),
                timestamp_writes: None,
            },
        );

        self.renderer
            .wal
            .bind_compute_pipeline(&mut compute_pass, &self.renderer.pipes.update_water_pipe);

        self.renderer.wal.dispatch_with_params(
            &mut compute_pass,
            &mut self.renderer.pipes.update_water_pipe,
            None,
            (self.renderer.settings.world_size.x() * 2).div_ceil(8) as u32,
            (self.renderer.settings.world_size.y() * 2).div_ceil(8) as u32,
            1,
        );
    }

    fn update_ao_ubo(&mut self) {
        let ao_lut = ao_lut::generate_lut::<8>(
            fBLOCK_SIZE / 1000.0,
            vec2::new(
                self.renderer.wal.config.width as f32,
                self.renderer.wal.config.height as f32,
            ),
            self.renderer.camera.horizline * self.renderer.camera.view_size.x / 2.0,
            self.renderer.camera.vertiline * self.renderer.camera.view_size.y / 2.0,
        );

        // Update the buffer via the queue
        self.renderer.wal.queue.write_buffer(
            self.renderer.buffers.ao_lut_uniform.current(),
            0,
            unsafe {
                std::slice::from_raw_parts(
                    (&ao_lut as *const AoLut) as *const u8,
                    std::mem::size_of::<AoLut>(),
                )
            },
        );
    }

    // fn raygen_smoke(&mut self) {

    //     //
    //     //
    // }

    fn raygen_water(&mut self) {
        // Begin the raygen water render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Raygen Water Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().mat_norm.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        let water_pipe = &mut self.renderer.pipes.raygen_water_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            water_pipe.pipe.line.as_ref().unwrap(),
            water_pipe.pipe.static_bind_groups.as_ref(),
        );

        const QUALITY_SIZE: u32 = 32;

        for lrr in &self.liquid_que {
            let pos: &vec3 = &lrr.pos;

            #[repr(C)] // for push constants
            #[derive(AsU8Slice)] // allow cast to &[u8]
            struct PushConstant {
                shift: vec4,
                // size: i32,
                // time: i32,
                // pad: ivec2,
                size_time: ivec4,
            }

            let push_constant = PushConstant {
                shift: vec4!(*pos, 0),
                size_time: ivec4!(self.renderer.counter, QUALITY_SIZE, 0, 0),
            };

            water_pipe.push_constants.extend_from_slice(push_constant.as_u8_slice());
            water_pipe.pc_count += 1;
        }

        let verts_per_water_tape = QUALITY_SIZE * 2 + 2;
        let tapes_per_block = QUALITY_SIZE;
        let batches = water_pipe.pc_count as u32;

        // same as count > 0 ... i guess
        if batches > 0 {
            // only render batch if it has anything to render
            let count = water_pipe.push_constants.len();
            let write = self.renderer.wal.queue.write_buffer_with(
                water_pipe.pc_buffer.as_ref().unwrap(),
                0,
                std::num::NonZero::new(count as u64).unwrap(),
            );
            let src_pc_slice_u8 =
                unsafe { std::slice::from_raw_parts(water_pipe.push_constants.as_ptr(), count) };
            write.unwrap().copy_from_slice(src_pc_slice_u8);

            self.renderer.wal.draw_with_params(
                &mut rpass,
                Some(water_pipe.pipe.static_bind_groups.as_ref().unwrap().current()),
                water_pipe.pc_bg.as_ref(),
                0..verts_per_water_tape,
                0..tapes_per_block * batches,
            );

            water_pipe.push_constants.clear();
            water_pipe.pc_count = 0;
        }
    }

    fn raygen_grass(&mut self) {
        // Begin the raygen grass render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Raygen Grass Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().mat_norm.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        for (foliage_index, foliage_pipe) in
            self.renderer.pipes.raygen_foliage_pipes.iter_mut().enumerate()
        {
            self.renderer.wal.bind_raster_pipeline(
                &mut rpass,
                foliage_pipe.pipe.line.as_ref().unwrap(),
                foliage_pipe.pipe.static_bind_groups.as_ref(),
            );

            // let pipe = &mut self.renderer.pipes.raygen_foliage_pipes[foliage_index as usize];
            let foliage_queue = &self.foliage_ques[foliage_index];

            // TODO:
            let size = 10;

            for frr in foliage_queue {
                let x_flip = self.renderer.camera.camera_dir.x < 0.0;
                let y_flip = self.renderer.camera.camera_dir.y < 0.0;

                #[repr(C)] // for push constants
                #[derive(AsU8Slice)] // allow cast to &[u8]
                struct PushConstant {
                    shift: vec4,
                    _size: i32,
                    _time: i32,
                    x_flip: i32,
                    y_flip: i32,
                }
                let push_constant = PushConstant {
                    shift: vec4!(frr.pos, 0),
                    _size: size as i32,
                    _time: self.renderer.counter as i32,
                    x_flip: x_flip as i32,
                    y_flip: y_flip as i32,
                };

                // (as everywhere else, do repeat yourself if it helps understanding)
                // we do not submit a bunch of drawcalls (vertices generated btw)
                // instead, we record (fake, i.e. emulated) push constant buffers and then submit single "batched" drawcall
                // we were already using instancing - instance was a blade
                // now its the same, but grass_batch_id is instance_id / blades_per_batch
                // and fake_pco = pco_array[grass_batch_id]

                foliage_pipe.push_constants.extend_from_slice(push_constant.as_u8_slice());
                foliage_pipe.pc_count += 1;
            }

            let desc = &self.renderer.foliage_descriptions[foliage_index];
            let verts_per_blade = desc.vertices;
            let blade_per_instance = 1; //for triangle strip
            let instances_per_batch = ((size * size) as u32).div_ceil(blade_per_instance);
            let batch_count = foliage_pipe.pc_count as u32;

            // only render batch if it has anything to render
            if foliage_pipe.pc_count > 0 {
                let count = foliage_pipe.push_constants.len();
                let write = self.renderer.wal.queue.write_buffer_with(
                    foliage_pipe.pc_buffer.as_ref().unwrap(),
                    0,
                    std::num::NonZero::new(count as u64).unwrap(),
                );
                let src_pc_slice_u8 = unsafe {
                    std::slice::from_raw_parts(foliage_pipe.push_constants.as_ptr(), count)
                };
                write.unwrap().copy_from_slice(src_pc_slice_u8);

                self.renderer.wal.draw_with_params(
                    &mut rpass,
                    Some(foliage_pipe.pipe.static_bind_groups.as_ref().unwrap().current()),
                    foliage_pipe.pc_bg.as_ref(),
                    0..verts_per_blade * blade_per_instance,
                    0..instances_per_batch * batch_count,
                );

                foliage_pipe.push_constants.clear();
                foliage_pipe.pc_count = 0;
            }
        }

        drop(rpass);
    }

    fn update_raygen_particles(&mut self) {
        let mut write_index = 0;

        for i in 0..self.renderer.particles.len() {
            let should_keep = self.renderer.particles[i].life_time > 0.0;
            if should_keep {
                self.renderer.particles[write_index] = self.renderer.particles[i];

                let velocity = self.renderer.particles[write_index].vel;
                self.renderer.particles[write_index].pos += velocity * self.renderer.delta_time;

                self.renderer.particles[write_index].life_time -= self.renderer.delta_time;
                write_index += 1;
            }
        }

        self.renderer.particles.resize(write_index, Default::default());
        let capped_particle_count =
            write_index.clamp(0, self.renderer.settings.max_particle_count as usize);

        // Update the GPU particle buffer with the current particle data
        if capped_particle_count > 0 {
            // Convert particle data to bytes
            let size = capped_particle_count * std::mem::size_of::<Particle>();
            let particle_bytes = unsafe {
                std::slice::from_raw_parts(self.renderer.particles.as_ptr() as *const u8, size)
            };

            let write = self.renderer.wal.queue.write_buffer_with(
                &self.renderer.buffers.gpu_particles.current(),
                0,
                BufferSize::new(size as u64).unwrap(),
            );
            write.unwrap().copy_from_slice(particle_bytes);
        };

        // Render the particles
        if !self.renderer.particles.is_empty() {
            let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Particle Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.renderer.dependent_images.as_ref().unwrap().mat_norm.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.renderer.dependent_images.as_ref().unwrap().depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                },
            );

            let pipe = &mut self.renderer.pipes.raygen_particles_pipe;

            self.renderer.wal.bind_raster_pipeline(
                &mut rpass,
                pipe.line.as_ref().unwrap(),
                pipe.static_bind_groups.as_ref(),
            );

            rpass.set_vertex_buffer(0, self.renderer.buffers.gpu_particles.current().slice(..));

            rpass.draw(0..36, 0..self.renderer.particles.len() as u32);
        }
    }

    fn map_meshes(&mut self) {
        // thing about wgpu is that they very much want to be pain in the ass, and after few hours i still did not figure out how to store passes
        // so here we fucking are, doing everything in a single scope and refactoring entire renderer to fit a non-gapi gapi
        let mut compute_pass = self.renderer.current_encoder.as_mut().unwrap().begin_compute_pass(
            &wgpu::ComputePassDescriptor {
                label: Some("Map Compute Pass"),
                timestamp_writes: None,
            },
        );

        self.renderer
            .wal
            .bind_compute_pipeline(&mut compute_pass, &self.renderer.pipes.map_pipe);

        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get_mut(mrr.mesh).unwrap();
            {
                let trans: &MeshTransform = &mrr.trans;

                let rotate = mat4::from(trans.rotation);
                let shift = mat4::identity().translated_3d(trans.translation);
                let transform = shift * rotate;

                // grid-aligned bounding box for our mesh in our voxel world
                let border_in_voxel = get_shift(transform, model_mesh.size);
                let border = iAABB {
                    min: ivec3!(border_in_voxel.min.floor()),
                    max: ivec3!(border_in_voxel.max.ceil()),
                };
                // unused, since we just use upper bound and dont extra voxels cull on cpu for higher parallelism
                let map_area = border.max - border.min;

                // here we encounter a problem:
                // just amount of dispatch calls is not enough, we (also) need bounding box
                // more generic approach would be to store this as metadata in separate Vec
                // however, due to how we divide compute work, this is already in push constants
                let push_constant = PcMapModel {
                    trans: transform.inverted(),
                    shift: ivec4!(border.min, 0),
                    map_area: ivec4!(map_area, 0),
                };

                assert!(self.renderer.pipes.map_pipe.static_bind_groups.is_some());

                // as everywhere, instead of directly submitting command, "sort" by state and delay actual work
                model_mesh.compute_push_constants.extend_from_slice(push_constant.as_u8_slice());
                model_mesh.compute_pc_count += 1;
            }
        }

        for model_mesh in &mut self.storage.models {
            let model_mesh = model_mesh.1;

            let count = model_mesh.compute_push_constants.len();
            if count == 0 {
                continue;
            }
            let write = self.renderer.wal.queue.write_buffer_with(
                model_mesh.compute_pc_buffer.as_ref().unwrap(),
                0,
                std::num::NonZero::new(count as u64).unwrap(),
            );
            let src_pc_slice_u8 = unsafe {
                std::slice::from_raw_parts(
                    model_mesh.compute_push_constants.as_ptr() as *const u8,
                    count,
                )
            };
            write.unwrap().copy_from_slice(src_pc_slice_u8);

            // there is no instancing for compute work.
            // however, all the models in our batch are the same size, and thus dispatch size has same upper bound
            // so what we do, is we submit upper bound of voxels no matter how many actually needed and discard extra
            // this loses like 50% in bad cases, but doing anything CPU-side with wgpu is even more expensive

            let model_size = model_mesh.size;
            let max_extent = (vec3!(model_size.x, model_size.y, model_size.z))
                .distance(Vec3::new(0.0, 0.0, 0.0));
            let worst_case_aabb = uvec3!(max_extent.ceil(), max_extent.ceil(), max_extent.ceil());
            let worst_case_voxels = worst_case_aabb.x * worst_case_aabb.y * worst_case_aabb.z;

            self.renderer.wal.dispatch_with_params(
                &mut compute_pass,
                &mut self.renderer.pipes.map_pipe,
                Some(model_mesh.voxels_bind_group_compute.as_ref().unwrap()),
                worst_case_voxels, // this gets converted to xyz using size in push constants
                1,                 // we dont do anything with this one
                model_mesh.compute_pc_count as u32, // and this is our instancing giving us push constants
            );

            model_mesh.compute_pc_count = 0;
            model_mesh.compute_push_constants.clear();
        }
    }

    fn lightmap_models(&mut self) {
        let render_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Lightmap Models Render Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.renderer.independent_images.lightmap.view,
                depth_ops: Some(wgpu::Operations {
                    // not clear cause on top of blocks - just continuation
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut rpass = self
            .renderer
            .current_encoder
            .as_mut()
            .unwrap()
            .begin_render_pass(&render_pass_desc);

        let pipe = &mut self.renderer.pipes.lightmap_models_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            pipe.line.as_ref().unwrap(),
            pipe.static_bind_groups.as_ref(),
        );

        for mrr in &self.model_que {
            let model_id = mrr.mesh;
            let model_mesh = &self.storage.models.get(model_id).unwrap();

            rpass.set_vertex_buffer(0, model_mesh.triangles.vertexes.as_ref().unwrap().slice(..));
            rpass.set_index_buffer(
                model_mesh.triangles.indices.as_ref().unwrap().slice(..),
                wgpu::IndexFormat::Uint16,
            );

            #[repr(C)] // for push constants
            #[derive(AsU8Slice)] // allow cast to &[u8]
            struct PushConstant {
                rot: quat,
                shift: vec4,
            }
            let push_constant = PushConstant {
                rot: mrr.trans.rotation,
                // 1 cause we will add 1 anyways, right?
                shift: vec4!(mrr.trans.translation, 1),
            };

            macro_rules! CHECK_AND_DRAW_BLOCK_FACE {
                ($__normal:expr, $__face:ident) => {
                    let fnorm = vec3!($__normal);
                    if is_face_visible(fnorm, self.renderer.camera.camera_dir) {
                        {
                            let buff = &model_mesh.triangles.$__face;

                            self.renderer.wal.draw_indexed_with_params(
                                &mut rpass,
                                Some(pipe.static_bind_groups.as_ref().unwrap().current()),
                                buff.pc_bg.as_ref(),
                                buff.iv.offset..buff.iv.offset + buff.iv.icount,
                                0,
                                0..1,
                            );
                        };
                    };
                };
            }

            CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(1, 0, 0), Pzz);
            CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(-1, 0, 0), Nzz);
            CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 1, 0), zPz);
            CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, -1, 0), zNz);
            CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, 1), zzP);
            CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, -1), zzN);
        }
    }

    fn lightmap_blocks(&mut self) {
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Lightmap Blocks Render Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.independent_images.lightmap.view,
                    depth_ops: Some(wgpu::Operations {
                        // clear cause first
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        let pipe = &mut self.renderer.pipes.lightmap_blocks_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            pipe.line.as_ref().unwrap(),
            pipe.static_bind_groups.as_ref(),
        );

        // lightmap and raygen share same pc buffer for blocks
        // so no pc buffer update

        // for brr in &self.block_que {
        //     let ipos = ivec3!(brr.pos);
        //     let block_id = brr.block;

        //     let block_mesh = &mut self.renderer.block_palette_meshes[block_id as usize];

        //     let check_and_lightmap_block_face =
        //         |normal: i8vec3, face: &mut IndexedVerticesQueue| {
        //             let fnorm = vec3::new(normal.x as f32, normal.y as f32, normal.z as f32);
        //             if is_face_visible(fnorm, self.renderer.camera.camera_dir) {
        //                 // Self::lightmap_block_face(
        //                 //     &mut face.push_constants,
        //                 //     &mut face.pc_count,
        //                 //     ipos,
        //                 //     &face.iv,
        //                 //     block_id,
        //                 // );
        //             };
        //         };

        //     check_and_lightmap_block_face(i8vec3::new(1, 0, 0), &mut block_mesh.triangles.Pzz);
        //     check_and_lightmap_block_face(i8vec3::new(-1, 0, 0), &mut block_mesh.triangles.Nzz);
        //     check_and_lightmap_block_face(i8vec3::new(0, 1, 0), &mut block_mesh.triangles.zPz);
        //     check_and_lightmap_block_face(i8vec3::new(0, -1, 0), &mut block_mesh.triangles.zNz);
        //     check_and_lightmap_block_face(i8vec3::new(0, 0, 1), &mut block_mesh.triangles.zzP);
        //     check_and_lightmap_block_face(i8vec3::new(0, 0, -1), &mut block_mesh.triangles.zzN);
        // }

        for block_mesh in &mut self.renderer.block_palette_meshes {
            // for every block, for every of its sides
            // copy its push constants from stored queue for a side and submit a drawcall

            if block_mesh.triangles.vertexes.is_some() && block_mesh.triangles.indices.is_some() {
                // bind vertex & index buffers for that side
                rpass.set_vertex_buffer(
                    0,
                    block_mesh.triangles.vertexes.as_ref().unwrap().slice(..),
                );
                rpass.set_index_buffer(
                    block_mesh.triangles.indices.as_ref().unwrap().slice(..),
                    wgpu::IndexFormat::Uint16,
                );
            }

            let mut draw_indexed_block_face = |face: &mut IndexedVerticesQueue| {
                if face.pc_bg.is_some() {
                    // we dont need to update push constants memory since we reuse old one

                    self.renderer.wal.draw_indexed_with_params(
                        &mut rpass,
                        Some(pipe.static_bind_groups.as_ref().unwrap().current()),
                        face.pc_bg.as_ref(),
                        face.iv.offset..face.iv.offset + face.iv.icount,
                        0,
                        0..face.pc_count as u32,
                    );

                    face.pc_count = 0;
                }
            };

            draw_indexed_block_face(&mut block_mesh.triangles.Pzz);
            draw_indexed_block_face(&mut block_mesh.triangles.Nzz);
            draw_indexed_block_face(&mut block_mesh.triangles.zPz);
            draw_indexed_block_face(&mut block_mesh.triangles.zNz);
            draw_indexed_block_face(&mut block_mesh.triangles.zzP);
            draw_indexed_block_face(&mut block_mesh.triangles.zzN);
        }

        self.renderer.wal.queue.submit([]);
    }

    fn update_light_ubo(&mut self) {
        // Use a dedicated encoder for lightmap work (or the command encoder from your lightmap command-buffer ring).
        // Update the light uniform buffer with the light transform.
        #[repr(C)]
        #[derive(Clone, Copy, AsU8Slice)]
        struct BufferPatch {
            trans: mat4,
        }
        let buffer_patch = BufferPatch {
            trans: self.renderer.light.light_transform,
        };
        // Update the buffer via the queue.
        self.renderer.wal.queue.write_buffer(
            &self.renderer.buffers.light_uniform.current(),
            0,
            buffer_patch.as_u8_slice(),
        );
    }

    fn raygen_block_face<'a>(
        wal: &Wal,
        pc_write_slice: &mut Vec<u8>, // The mutable slice for writing
        pc_counter: &mut u32,
        normal: ivec3,
        shift: ivec3,
        buff: &IndexedVertices,
        block_id: MeshBlock,
    ) {
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

        let push_constant = PcRyagenBlockFace {
            block: block_id as InternalBlockId,
            shift,
            unorm: unsafe { transmute(u8vec4::new(pbn, 0, 0, 0)) },
        };

        // yeah we just delay rendering it cause sorting by state is faster for wgpu
        pc_write_slice.extend_from_slice(push_constant.as_u8_slice());
        *pc_counter += 1;
    }

    fn raygen_blocks(&mut self) {
        // Begin the raygen blocks render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Raygen Blocks Render Pass"),
                // raster mat_norm gbuffers
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().mat_norm.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // first use clears, other just load
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // depth is normal gbuffer depth
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().depth.view,
                    depth_ops: Some(wgpu::Operations {
                        // clear cause first
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        let pipe = &mut self.renderer.pipes.raygen_blocks_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            pipe.line.as_ref().unwrap(),
            pipe.static_bind_groups.as_ref(),
        );

        for brr in &self.block_que {
            let ipos = ivec3!(brr.pos);
            {
                let block_id = brr.block;
                let shift = ipos;

                let block_mesh = &mut self.renderer.block_palette_meshes[block_id as usize];

                let check_and_draw_block_face =
                    |normal: i8vec3, face: &mut IndexedVerticesQueue| {
                        let fnorm = vec3::new(normal.x as f32, normal.y as f32, normal.z as f32);
                        let inorm = ivec3!(normal.x as i32, normal.y as i32, normal.z as i32);

                        if is_face_visible(fnorm, self.renderer.camera.camera_dir) {
                            Self::raygen_block_face(
                                &self.renderer.wal,
                                &mut face.push_constants,
                                &mut face.pc_count,
                                inorm,
                                shift,
                                &face.iv,
                                block_id,
                            );
                        }
                    };

                // draw every face (separately). This allows per-face culling
                check_and_draw_block_face(i8vec3::new(1, 0, 0), &mut block_mesh.triangles.Pzz);
                check_and_draw_block_face(i8vec3::new(-1, 0, 0), &mut block_mesh.triangles.Nzz);
                check_and_draw_block_face(i8vec3::new(0, 1, 0), &mut block_mesh.triangles.zPz);
                check_and_draw_block_face(i8vec3::new(0, -1, 0), &mut block_mesh.triangles.zNz);
                check_and_draw_block_face(i8vec3::new(0, 0, 1), &mut block_mesh.triangles.zzP);
                check_and_draw_block_face(i8vec3::new(0, 0, -1), &mut block_mesh.triangles.zzN);
            }
        }

        // now we processed all block render requests and sorted them by state and now we will actually render them in very few (3 x block_count) drawcalls

        for block_mesh in &mut self.renderer.block_palette_meshes {
            // for every block, for every of its sides
            // copy its push constants from stored queue for a side and submit a drawcall

            if block_mesh.triangles.vertexes.is_some() && block_mesh.triangles.indices.is_some() {
                // bind vertex & index buffers for all sides
                rpass.set_vertex_buffer(
                    0,
                    block_mesh.triangles.vertexes.as_ref().unwrap().slice(..),
                );
                rpass.set_index_buffer(
                    block_mesh.triangles.indices.as_ref().unwrap().slice(..),
                    wgpu::IndexFormat::Uint16,
                );
            }

            let mut draw_block_face = |face: &mut IndexedVerticesQueue| {
                if face.pc_bg.is_some() && !face.push_constants.is_empty() {
                    let count = face.push_constants.len();
                    let size = count;

                    let write = self.renderer.wal.queue.write_buffer_with(
                        face.pc_buffer.as_ref().unwrap(),
                        0,
                        std::num::NonZero::new(size as u64).unwrap(),
                    );
                    let src_pc_slice_u8 = unsafe {
                        std::slice::from_raw_parts(face.push_constants.as_ptr() as *const u8, size)
                    };
                    write.unwrap().copy_from_slice(src_pc_slice_u8);

                    self.renderer.wal.draw_indexed_with_params(
                        &mut rpass,
                        Some(pipe.static_bind_groups.as_ref().unwrap().current()),
                        face.pc_bg.as_ref(),
                        face.iv.offset..face.iv.offset + face.iv.icount,
                        0,
                        0..face.pc_count as u32,
                    );

                    // we dont set pc_count to 0 here cause its used in lightmap_blocks cause they share pc buffers
                    // face.pc_count = 0;
                    face.push_constants.clear(); // cpu memory however can be cleaned
                }
            };

            draw_block_face(&mut block_mesh.triangles.Pzz);
            draw_block_face(&mut block_mesh.triangles.Nzz);
            draw_block_face(&mut block_mesh.triangles.zPz);
            draw_block_face(&mut block_mesh.triangles.zNz);
            draw_block_face(&mut block_mesh.triangles.zzP);
            draw_block_face(&mut block_mesh.triangles.zzN);
        }

        self.renderer.wal.queue.submit([]);
    }

    fn update_ubo(&mut self) {
        // let this = &mut self.renderer;
        // Update the uniform buffer with camera and light properties.
        let horizline_scaled =
            self.renderer.camera.horizline * (self.renderer.camera.view_size.x / 2.0);
        let vertiline_scaled =
            self.renderer.camera.vertiline * (self.renderer.camera.view_size.y / 2.0);

        let buffer_patch = UboData {
            trans_w2s: self.renderer.camera.camera_transform,
            campos: vec4!(self.renderer.camera.camera_pos, 0),
            camdir: vec4!(self.renderer.camera.camera_dir, 0),
            horizline_scaled: vec4!(horizline_scaled, 0),
            vertiline_scaled: vec4!(vertiline_scaled, 0),
            global_light_dir: vec4!(self.renderer.light.light_dir, 0),
            lightmap_proj: self.renderer.light.light_transform,
            timeseed: self.renderer.counter as i32,
            frame_size: vec2!(
                self.renderer.wal.config.width,
                self.renderer.wal.config.height
            ),
            wind_direction: vec2!(0.8, 0.2),
            delta_time: 0.15,
            _pad_1: Default::default(),
            _pad_2: Default::default(),
        };
        self.renderer.wal.queue.write_buffer(
            &self.renderer.buffers.uniform.current(),
            0,
            buffer_patch.as_u8_slice(),
        );
        // self.renderer.wal.queue.submit([]);
    }

    fn shift_radiance(&mut self, shift: ivec3) {
        self.radiance_shift = shift;
    }

    fn raygen_models(&mut self) {
        // Begin the raygen models render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Raygen Models Render Pass"),
                // raster mat_norm gbuffers
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().mat_norm.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // because why the fuck would we erase raygen'ed blocks?
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // depth is normal gbuffer depth
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.dependent_images.as_ref().unwrap().depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        let pipe = &mut self.renderer.pipes.raygen_models_pipe;

        self.renderer.wal.bind_raster_pipeline(
            &mut rpass,
            pipe.line.as_ref().unwrap(),
            pipe.static_bind_groups.as_ref(),
        );

        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get_mut(mrr.mesh).unwrap();
            let model_trans: &MeshTransform = &mrr.trans;

            rpass.set_vertex_buffer(0, model_mesh.triangles.vertexes.as_ref().unwrap().slice(..));
            rpass.set_index_buffer(
                model_mesh.triangles.indices.as_ref().unwrap().slice(..),
                wgpu::IndexFormat::Uint16,
            );

            // Update the model voxels bind group if needed
            // This would replace the Vulkan descriptor set update
            // For now, we'll assume the bind group is already set up correctly
            let check_and_raygen_model_face = |normal: i8vec3, face: &mut IndexedVerticesQueue| {
                let fnorm = vec3::new(normal.x as f32, normal.y as f32, normal.z as f32);
                if is_face_visible(
                    model_trans.rotation * fnorm,
                    self.renderer.camera.camera_dir,
                ) {
                    Self::raygen_model_face(
                        &mut face.push_constants,
                        &model_trans.rotation,
                        &model_trans.translation,
                        fnorm,
                        &face.iv,
                    );
                    face.pc_count += 1;
                }
            };

            check_and_raygen_model_face(i8vec3::new(1, 0, 0), &mut model_mesh.triangles.Pzz);
            check_and_raygen_model_face(i8vec3::new(-1, 0, 0), &mut model_mesh.triangles.Nzz);
            check_and_raygen_model_face(i8vec3::new(0, 1, 0), &mut model_mesh.triangles.zPz);
            check_and_raygen_model_face(i8vec3::new(0, -1, 0), &mut model_mesh.triangles.zNz);
            check_and_raygen_model_face(i8vec3::new(0, 0, 1), &mut model_mesh.triangles.zzP);
            check_and_raygen_model_face(i8vec3::new(0, 0, -1), &mut model_mesh.triangles.zzN);
        }

        // no iter by state (not by depth)
        for model_mesh in &mut self.storage.models {
            let model_mesh = model_mesh.1;

            // bind if valid (TODO: have single Option)
            if model_mesh.triangles.vertexes.is_some() && model_mesh.triangles.indices.is_some() {
                rpass.set_vertex_buffer(
                    0,
                    model_mesh.triangles.vertexes.as_ref().unwrap().slice(..),
                );
                rpass.set_index_buffer(
                    model_mesh.triangles.indices.as_ref().unwrap().slice(..),
                    wgpu::IndexFormat::Uint16,
                );
            }

            // TODO: no macro | reuse it?
            let mut draw_indexed_model_face = |face: &mut IndexedVerticesQueue| {
                if face.pc_bg.is_some() && !face.push_constants.is_empty() {
                    let count = face.push_constants.len();
                    let size = count;
                    let write = self.renderer.wal.queue.write_buffer_with(
                        face.pc_buffer.as_ref().unwrap(),
                        0,
                        std::num::NonZero::new(size as u64).unwrap(),
                    );
                    let src_pc_slice_u8 = unsafe {
                        std::slice::from_raw_parts(face.push_constants.as_ptr() as *const u8, size)
                    };
                    write.unwrap().copy_from_slice(src_pc_slice_u8);

                    self.renderer.wal.draw_indexed_with_params(
                        &mut rpass,
                        Some(pipe.static_bind_groups.as_ref().unwrap().current()),
                        face.pc_bg.as_ref(),
                        face.iv.offset..face.iv.offset + face.iv.icount,
                        0,
                        0..face.pc_count as u32,
                    );

                    face.pc_count = 0;
                    face.push_constants.clear();
                }
            };

            draw_indexed_model_face(&mut model_mesh.triangles.Pzz);
            draw_indexed_model_face(&mut model_mesh.triangles.Nzz);
            draw_indexed_model_face(&mut model_mesh.triangles.zPz);
            draw_indexed_model_face(&mut model_mesh.triangles.zNz);
            draw_indexed_model_face(&mut model_mesh.triangles.zzP);
            draw_indexed_model_face(&mut model_mesh.triangles.zzN);
        }
    }

    fn raygen_model_face<'a>(
        // wal: &Wal,
        // rpass: &mut wgpu::RenderPass<'a>,
        // pipeline: &'a wgpu::RenderPipeline, // Now passed individually
        // static_bind_group: Option<&BindGroup>,
        pc_write_slice: &mut Vec<u8>,
        rot: &quat,
        shift: &vec3,
        // model_voxels_bg: &BindGroup,
        normal: vec3,
        buff: &IndexedVertices,
    ) {
        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            rot: quat,
            shift: vec4,
            fnormal: vec4,
        }
        let push_constant = PushConstant {
            rot: *rot,
            shift: vec4!(*shift, 0),
            fnormal: vec4!(normal, 0.0),
        };

        pc_write_slice.extend_from_slice(push_constant.as_u8_slice());
        // rpass.draw_indexed(buff.offset..buff.offset + buff.icount, 0, 0..1);
    }
}

pub struct SimpleFoliageDescriptionBuilder<'a> {
    foliage_descriptions: Vec<MeshFoliageDesc<'a>>,
}

// impl very exact thing
impl<'a> FoliageDescriptionBuilder<MeshFoliageDesc<'a>> for SimpleFoliageDescriptionBuilder<'a> {
    fn new() -> Self {
        Self {
            foliage_descriptions: vec![],
        }
    }
    fn load_foliage(&mut self, foliage: MeshFoliageDesc<'a>) -> MeshFoliage {
        let index = self.foliage_descriptions.len() as u32;
        self.foliage_descriptions.push(foliage);
        index as MeshFoliage
    }
    fn build(self) -> Vec<MeshFoliageDesc<'a>> {
        self.foliage_descriptions
    }
}

// creates a CPU-side struct for foliage
// this is not foliage mesh itself yet, but a blank used to register foliage for future creation*
// Foliage in lum is not a controlled simulation with a mesh. Instead, it is a (vertex) shader
// This is highest level of flexibility** and also enforces perfomance
// You use foliage meshes to draw things like grass in worldspace
// TODO: is there a way to make src extendable to such degree without sacrificing anything?
// * done this way for simplicity (aka pre-counting size)
// **: Lum is not trying to be general-purpose engine at all. Some very basic parts that are expected from game engine
// are and will forever be missing. You cant make fast abstraction on top of everything.
impl<'window, D: Dim3> RendererInterface<'window, D> for RendererWgpu<'window, D> {
    type FoliageDescription = MeshFoliageDesc<'window>;
    type FoliageDescriptionBuilder = SimpleFoliageDescriptionBuilder<'window>;
    type InternalBlockId = InternalBlockId;

    fn new(
        settings: &Settings<D>,
        window: std::sync::Arc<Window>,
        size: PhysicalSize<u32>,
        foliages: &[MeshFoliageDesc<'window>],
    ) -> Self {
        Self {
            renderer: InternalRendererWebGPU::new(settings, window, size, foliages.to_vec()),
            block_que: vec![],
            // mesh_que: vec![],
            foliage_ques: vec![vec![]; foliages.len()],
            liquid_que: vec![],
            volumetric_que: vec![],
            model_que: vec![],
            storage: RendererStorage::default(),
            radiance_shift: ivec3::zero(),
        }
    }

    async fn new_async(
        settings: &Settings<D>,
        window: std::sync::Arc<Window>,
        size: PhysicalSize<u32>,
        foliages: &[MeshFoliageDesc<'window>],
    ) -> Self {
        Self {
            renderer: InternalRendererWebGPU::new_async(settings, window, size, foliages.to_vec())
                .await,
            block_que: vec![],
            // mesh_que: vec![],
            foliage_ques: vec![vec![]; foliages.len()],
            liquid_que: vec![],
            volumetric_que: vec![],
            model_que: vec![],
            storage: RendererStorage::default(),
            radiance_shift: ivec3::zero(),
        }
    }

    fn load_model(&mut self, model_data: ModelData) -> MeshModel {
        let model_mesh = self.renderer.load_model(model_data);
        let index = self.storage.models.allocate(model_mesh).unwrap();
        index as MeshModel
    }
    fn unload_model(&mut self, model: MeshModel) {
        let model_mesh = self.storage.models.take(model).unwrap();
        self.renderer.free_model(model_mesh);
    }
    // TODO: move to impl MeshModel
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
            main: main_mat as InternalMatId,
            foam: foam_mat as InternalMatId,
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

    fn start_frame(&mut self) {
        // queues are like high-level draw calls, and we are clearing command buffers
        self.block_que.clear();
        self.model_que.clear();
        for queue in &mut self.foliage_ques {
            queue.clear();
        }
        self.liquid_que.clear();
        self.volumetric_que.clear();
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

                    self.draw_block(block as MeshBlock, &block_pos);
                }
            }
        }
    }

    fn draw_block(&mut self, block: MeshBlock, block_pos: &i16vec3) {
        let fpos = vec3!(*block_pos);

        if self.is_block_visible(fpos) {
            self.block_que.push(BlockRenderRequest {
                cam_dist: 0.0,
                block: block as MeshBlock,
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
            let corresponding_foliage_queue = &mut self.foliage_ques[*foliage as usize];
            corresponding_foliage_queue.push(FoliageRenderRequest {
                cam_dist: 0.0,
                mesh: foliage.clone(),
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

    // function that "optimizes" the frame
    // it could be implicit, but explicitnesss allows you to maybe do this work in parallel
    // such approach does not really play well with what i do (no multithreading in rendering), but anyways
    fn prepare_frame(&mut self) {
        // self.renderer.update_camera();
        // self.renderer.update_light_transform();
        let cam = self.renderer.camera.camera_transform;
        Self::calculate_and_sort_by_cam_dist(&mut self.model_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.block_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.liquid_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.volumetric_que, cam);

        for queue in &mut self.foliage_ques {
            Self::calculate_and_sort_by_cam_dist(queue, cam);
        }
    }

    fn end_frame(&mut self) {
        // yes, started here cause no reason not to group

        // why these two are here is explained below
        self.blockify_models();
        self.renderer.find_radiance_to_update();

        // you may wonder why is start_frame here, and not in the beginning
        // this is because it contains GPU-sync, which im trying to delay as much as possible
        // it does not help much when you are CPU-bound (which is the case). But still a bit useful
        self.renderer.start_frame();
        self.update_ubo();

        self.radiance_shift = ivec3::zero();
        self.renderer.shift_radiance(self.radiance_shift);
        self.renderer.update_radiance();

        self.updade_grass(Default::default());
        self.updade_water();
        self.renderer.exec_copies();

        //
        //
        // here we can se divergence between wgpu and vulkan. Wgpu is too complicated for my small brain so i do everything in a single scope
        self.map_meshes();

        // self.update_light_ubo();
        self.lightmap_blocks();
        self.lightmap_models();

        self.update_ao_ubo();
        self.raygen_blocks();
        self.raygen_models();

        self.update_raygen_particles();

        self.raygen_grass();
        self.raygen_water();

        // wgpu is so good that most important Vulkan feature is missing (for convinience)

        self.diffuse();
        self.ambient_occlusion();
        self.raygen_glossy();
        self.raygen_smoke();

        self.glossy();
        self.smoke();
        self.tonemap();
        self.move_next();

        self.renderer.counter += 1;

        // atrace!();
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.renderer.recreate_window(new_size);
    }

    fn get_world_blocks(&self) -> Array3DView<InternalBlockId, MeshBlock, D> {
        self.renderer.origin_world.as_view()
    }

    fn get_world_blocks_mut(&mut self) -> Array3DViewMut<InternalBlockId, MeshBlock, D> {
        self.renderer.origin_world.as_view_mut()
    }

    fn destroy(self) {
        self.renderer.destroy();
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

// TODO: is there a simpler shorter way to do this? I hate setters & getters
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

fn is_face_visible(normal: vec3, camera_dir: vec3) -> bool {
    normal.dot(camera_dir) < 0.0
    // true
}
