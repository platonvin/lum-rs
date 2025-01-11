use std::ptr;

use as_u8_slice_derive::AsU8Slice;
use internal_renderer::{fAABB, get_shift, iAABB};
use vek::{num_traits::Float, transform, vec, Clamp, FrustumPlanes};
use vk::{AccessFlags, DeviceV1_0, HasBuilder, Image, ImageLayout, KhrPushDescriptorExtension, PipelineBindPoint, PipelineStageFlags, ShaderStageFlags};

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
        let horizline = camera_ray_dir_plane
            .cross(vec3::new(0.0, 0.0, 1.0))
            .normalized();

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
        let light_pos = vec3::new(
            f32::from((world_size.x * (16 as u32)) as i16),
            f32::from((world_size.y * (16 as u32)) as i16),
            0.0,
        ) / 2.0 
            - (1.0 * 16.0 * self.light_dir);

        let view = mat4::look_at_lh(light_pos, light_pos + self.light_dir, up);
        let voxel_in_pixels = 5.0;
        let view_width_in_voxels = 3000.0 / voxel_in_pixels;
        let view_height_in_voxels = 3000.0 / voxel_in_pixels;
        let projection = mat4::orthographic_lh_no(FrustumPlanes {
            left: -view_width_in_voxels / 2.0,
            right: view_width_in_voxels / 2.0,
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

    // starts the stage where you can "request drawing" things
    // under the hood it prepares Vulkan for recording draw calls
    pub fn start_frame(&mut self) {
        self.update_camera();
        self.update_light_transform();

        self.lumal.start_frame(&[
            *self.cmdbufs.compute_command_buffers.current(),
            *self.cmdbufs.lightmap_command_buffers.current(),
            *self.cmdbufs.graphics_command_buffers.current(),
            *self.cmdbufs.copy_command_buffers.current(),
        ]);
    }

    pub fn start_blockify(&mut self){
        self.block_copies_queue.clear();
        self.palette_counter = 0;

        // reset the current world to the origin
        self.current_world.copy_data_from(&self.origin_world);
    }

    pub fn index_block_xy (&self, n: i32) -> ivec2 {
        let x = n % BLOCK_PALETTE_SIZE_X as i32;
        let y = n / BLOCK_PALETTE_SIZE_X as i32;
        assert!(y <= BLOCK_PALETTE_SIZE_Y as i32);
        ivec2::new(x, y)
    }
    
    // allocates temp block in palette for every block that intersects with every mesh blockified
    pub fn blockify_mesh(&mut self, mesh: &InternalMeshModel, trans: &MeshTransform) {
        let rotate = vek::Mat4::from(trans.rotation);
        let shift = vek::Mat4::<f32>::identity().translated_3d(trans.translation);  
        // let 
        let border_in_voxel = get_shift(shift * rotate, mesh.size);   

        let mut border = iAABB {
            min: ivec3::new(
                (border_in_voxel.min.x - 1.0) as i32 / 16 ,
                (border_in_voxel.min.y - 1.0) as i32 / 16 ,
                (border_in_voxel.min.z - 1.0) as i32 / 16 ,
            ),
            max: ivec3::new(
                (border_in_voxel.max.x + 1.0) as i32 / 16, 
                (border_in_voxel.max.y + 1.0) as i32 / 16,
                (border_in_voxel.max.z + 1.0) as i32 / 16,
            ),
        };

        // clamp to world size so no out of bounds
        border.min = ivec3::clamped(
            border.min,
            ivec3::zero(),
            ivec3::new(
                self.settings.world_size.x as i32  - 1,
                self.settings.world_size.y as i32  - 1,
                self.settings.world_size.z as i32  - 1,
            ),
        );
        border.max = ivec3::clamped(
            border.max,
            ivec3::zero(),
            ivec3::new(
                self.settings.world_size.x as i32 - 1,
                self.settings.world_size.y as i32 - 1,
                self.settings.world_size.z as i32 - 1,
            ),
        );

        for zz in border.min.z..=border.max.z {
        for yy in border.min.y..=border.max.y {
        for xx in border.min.x..=border.max.x {
            let current_block = self.current_world[(xx as usize, yy as usize, zz as usize)];
            if (current_block as u32) < self.static_block_palette_size { // static
                //add to copy queue
                let src_block = self.index_block_xy(current_block as i32);
                let dst_block = self.index_block_xy(self.palette_counter as i32);

                // do image copy on for non-zero-src blocks. Other things still done for every allocated block
                // because zeroing is fast
                if(current_block != 0){
                    let mut static_block_copy = vk::ImageCopy::default();
                        static_block_copy.src_subresource = vk::ImageSubresourceLayers::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .layer_count(1)
                            .mip_level(0)
                            .build();
                        static_block_copy.dst_subresource = vk::ImageSubresourceLayers::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .layer_count(1)
                            .mip_level(0)
                            .build();
                        static_block_copy.extent = vk::Extent3D::builder()
                            .width(16)
                            .height(16)
                            .depth(16)
                            .build();
                        static_block_copy.src_offset = vk::Offset3D::builder()
                            .x(src_block.x * 16)
                            .y(src_block.y * 16)
                            .z(0)
                            .build();
                        static_block_copy.dst_offset = vk::Offset3D::builder()
                            .x(dst_block.x * 16)
                            .y(dst_block.y * 16)
                            .z(0)
                            .build();
                    // TODO: more compact representation
                    self.block_copies_queue.push(static_block_copy);
                }

                self.current_world[(xx as usize, yy as usize, zz as usize)] = self.palette_counter as BlockID_t;
                self.palette_counter += 1;

                // if(current_block == 0) zero_blocks++;
                // else just_blocks++;
            } else {
                //already new block, just leave it
            }
        }}}
    }

    pub fn end_blockify(&mut self) {
        let dimensions_to_copy = self.current_world.dimensions();
        let count = dimensions_to_copy.0 * 
                    dimensions_to_copy.1 * 
                    dimensions_to_copy.2;
        let size_to_copy = count * size_of::<BlockID_t>();
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.current_world.data.as_ptr(),
                self.buffers.staging_world.current().mapped.unwrap() as *mut BlockID_t,
                count, // converts to size automatically
            )
        };
        unsafe {
            self.lumal.allocator.as_ref().unwrap().flush_allocation(
                self.buffers.staging_world.current().allocation,
                0,
                size_to_copy as u64,
            )
        };
    }

    /*
    void LumInternal::LumInternalRenderer::update_radiance() {
    CommandBuffer& commandBuffer = computeCommandBuffers.current();
    TRACE()
    table3d<bool> 
        set = {};
    TRACE()
        set.allocate(world_size);
    TRACE()
        set.set(false);
    TRACE()
    radianceUpdates.clear();
    
    TRACE()
    for (int zz = 0; zz < world_size.z; zz++) {
    for (int yy = 0; yy < world_size.y; yy++) {
    for (int xx = 0; xx < world_size.x; xx++) {
        // int block_id = who cares on less then a million blocks?
        // UPD: actually, smarter algorithms resulted in less perfomance
        int sum_of_neighbours = 0;
        for (int dz = -1; (dz <= +1); dz++) {
        for (int dy = -1; (dy <= +1); dy++) {
        for (int dx = -1; (dx <= +1); dx++) {
            ivec3 test_block = ivec3 (xx + dx, yy + dy, zz + dz);
            // kinda slow... but who cares on less then 1m blocks
            // safity
            test_block = glm::clamp(test_block, ivec3(0), ivec3(world_size)-1);
            sum_of_neighbours += current_world (test_block);
        }}}
        if(sum_of_neighbours > 0){
            radianceUpdates.push_back (i8vec4 (xx, yy, zz, 0));
            set(ivec3(xx, yy, zz)) = true;
        }
    }}}
    // special updates are ones requested via API
    TRACE()
    for (auto u : specialRadianceUpdates) {
        if (!set(ivec3(u.x, u.y, u.z))) {
            radianceUpdates.push_back (u);
        }
    } specialRadianceUpdates.clear();
    set.deallocate();
    VkDeviceSize bufferSize = sizeof (radianceUpdates[0]) * radianceUpdates.size();
    memcpy (stagingRadianceUpdates.current().mapped, radianceUpdates.data(), bufferSize);

    commandBuffer.cmdPipelineBarrier (        VK_PIPELINE_STAGE_ALL_COMMANDS_BIT, VK_PIPELINE_STAGE_ALL_COMMANDS_BIT,
        VK_ACCESS_MEMORY_READ_BIT | VK_ACCESS_MEMORY_WRITE_BIT, VK_ACCESS_MEMORY_READ_BIT | VK_ACCESS_MEMORY_WRITE_BIT,
        gpuRadianceUpdates.current());
    commandBuffer.cmdPipelineBarrier (        VK_PIPELINE_STAGE_ALL_COMMANDS_BIT, VK_PIPELINE_STAGE_ALL_COMMANDS_BIT,
        VK_ACCESS_MEMORY_READ_BIT | VK_ACCESS_MEMORY_WRITE_BIT, VK_ACCESS_MEMORY_READ_BIT | VK_ACCESS_MEMORY_WRITE_BIT,
        stagingRadianceUpdates.current());

    VkBufferCopy
        copyRegion = {};
        copyRegion.size = bufferSize;
    
    vkCmdCopyBuffer (commandBuffer.commandBuffer, stagingRadianceUpdates.current().buffer, gpuRadianceUpdates.current().buffer, 1, &copyRegion);
    commandBuffer.cmdPipelineBarrier (        VK_PIPELINE_STAGE_ALL_COMMANDS_BIT, VK_PIPELINE_STAGE_ALL_COMMANDS_BIT,
        VK_ACCESS_MEMORY_READ_BIT | VK_ACCESS_MEMORY_WRITE_BIT, VK_ACCESS_MEMORY_READ_BIT | VK_ACCESS_MEMORY_WRITE_BIT,
        gpuRadianceUpdates.current());
    commandBuffer.cmdBindPipe(&radiancePipe);
    /**/ vkCmdBindDescriptorSets (commandBuffer.commandBuffer, VK_PIPELINE_BIND_POINT_COMPUTE, radiancePipe.lineLayout, 0, 1, &radiancePipe.sets.current(), 0, 0);
    int magic_number = lumal.iFrame % 2;
    //if fast than increase work
    if (timeTakenByRadiance < 1.8) {
        magicSize --;
        //if slow than decrease work
    } else if (timeTakenByRadiance > 2.2) {
        magicSize ++;
    }
    // LOG(timeTakenByRadiance);
    // LOG(magicSize);
    magicSize = glm::max (magicSize, 1); //never remove
    magicSize = glm::min (magicSize, 10);
    struct rtpc {int time, iters, size, shift;} pushconstant = {lumal.iFrame, 0, magicSize, lumal.iFrame % magicSize};
    vkCmdPushConstants (commandBuffer.commandBuffer, radiancePipe.lineLayout, VK_SHADER_STAGE_COMPUTE_BIT, 0, sizeof (pushconstant), &pushconstant);
    int wg_count = radianceUpdates.size() / magicSize;
    PLACE_TIMESTAMP_OUTSIDE(commandBuffer.commandBuffer);
    vkCmdDispatch (commandBuffer.commandBuffer, wg_count, 1, 1);
    PLACE_TIMESTAMP_OUTSIDE(commandBuffer.commandBuffer);
    commandBuffer.cmdPipelineBarrier (        VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT, VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT,
        VK_ACCESS_SHADER_WRITE_BIT, VK_ACCESS_SHADER_WRITE_BIT,
        radianceCache.current());
    commandBuffer.cmdPipelineBarrier (        VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT, VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT,
        VK_ACCESS_SHADER_WRITE_BIT, VK_ACCESS_SHADER_READ_BIT,
        radianceCache.current());
}

    */

    pub fn update_radiance(&mut self) {
        let mut command_buffer = self.cmdbufs.compute_command_buffers.current();

        // set is like a hash_set, but optimized (no hashing, no collisions)
        // its literally 3d array of bools, each corresponding to "if set"
        let mut set = Array3D::<bool>::new_filled(
            self.settings.world_size.x as usize,
            self.settings.world_size.y as usize,
            self.settings.world_size.z as usize,
            false, // each value in set corresponds to "if the block is already updated"
        );

        self.radiance_updates.clear();
        
        // push block into queue of update requests if the block has neighbours
        for zz in 0_i32..self.settings.world_size.z as i32 {
        for yy in 0_i32..self.settings.world_size.y as i32 {
        for xx in 0_i32..self.settings.world_size.x as i32 {
            let mut sum_of_neighbours = 0;

            for dz in -1_i32..=1 {
            for dy in -1_i32..=1 {
            for dx in -1_i32..=1 {
                let x = xx + dx;
                let y = yy + dy;
                let z = zz + dz;
                if x < 0 || x >= self.settings.world_size.x as i32 {continue;}
                if y < 0 || y >= self.settings.world_size.y as i32 {continue;}
                if z < 0 || z >= self.settings.world_size.z as i32 {continue;}
                let neighbor_block = self.current_world[(x as usize, y as usize, z as usize)];
                // we could add one, but it does not matter - we only need presence of neighbours
                sum_of_neighbours += neighbor_block; 
            }}}

            if sum_of_neighbours > 0 {
                self.radiance_updates.push(i8vec4::new(
                    xx as i8,
                    yy as i8, 
                    zz as i8,
                    0
                ));
                set[(xx as usize, yy as usize, zz as usize)] = true;
            }
        }}}

        // special updates are ones requested via API
        for u in &self.special_radiance_updates {
            // if not already updated in loop before, add it to the queue
            if !set[(u.x as usize, u.y as usize, u.z as usize)] {
                self.radiance_updates.push(u.clone());
            }
        }

        drop(set);

        let count_to_copy = self.radiance_updates.len();
        let size_to_copy = count_to_copy * size_of::<i8vec4>();
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.radiance_updates.as_ptr(),
                self.buffers.staging_radiance_updates.current().mapped.unwrap() as *mut i8vec4,
                count_to_copy, // converts to size automatically
            )
        };

        self.lumal.buffer_memory_barrier(
            command_buffer,
            &self.buffers.staging_radiance_updates.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
        );
        self.lumal.buffer_memory_barrier(
            command_buffer,
            &self.buffers.gpu_radiance_updates.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
        );

        let copy = vk::BufferCopy {
            size: size_to_copy as u64,
            src_offset: 0,
            dst_offset: 0,
        };

        unsafe {
            self.lumal.device.cmd_copy_buffer(
                *command_buffer,
                self.buffers.staging_radiance_updates.current().buffer,
                self.buffers.gpu_radiance_updates.current().buffer,
                &[copy],
            );
        };

        self.lumal.buffer_memory_barrier(
            command_buffer,
            &self.buffers.gpu_radiance_updates.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
        );

        // binds descriptor sets and pipeline itself
        self.lumal.bind_compute_pipe(command_buffer, &self.pipes.radiance_pipe);

        let magic_number = self.lumal.frame % 2;

        #[repr(C)]// for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            time: i32,
            iters: i32,
            size: i32,
            shift: i32,
        }

        fn as_u8_slice(push_constant: &PushConstant) -> &[u8] {
            unsafe {
                std::slice::from_raw_parts(
                    (push_constant as *const PushConstant) as *const u8,
                    std::mem::size_of::<PushConstant>(),
                )
            }
        }

        let push_constant = PushConstant {
            time: self.lumal.frame as i32,
            iters: 0,
            size: magic_number as i32,
            shift: self.lumal.frame as i32 % magic_number as i32,
        };

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.radiance_pipe.line_layout,
                ShaderStageFlags::COMPUTE,
                0,
                push_constant.as_u8_slice(),
            );
        }

        let wg_count = self.radiance_updates.len() / magic_number as usize;

        unsafe {
            self.lumal.device.cmd_dispatch(
                *command_buffer,
                // TOOD: current implementation just marches through the whole array skipping a lot of elements
                // Why didn't i just pack work tightly?
                wg_count as u32, 
                1,
                1,
            )
        };

        self.lumal.image_memory_barrier(
            command_buffer,
            &self.independent_images.radiance_cache.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            GENERAL, GENERAL, // Most of images are in GENERAL because:
            // 1. Highly optimized GPU code uses images in multiple ways, which restricts to GENERAL only
            // 2. When it does not, gained perfomance is negligible compared to the work required to manage layouts
            // 3. Most popular GPU's dont give a fuck about layouts (NVIDIA)
            // 4. Even AMD did not gain any perfomance in my tests (at some point, i did whole thing with correct layouts and barriers and it was the same perfomance)
        );
    }

    pub fn shift_radiance(&mut self, radiance_shift: ivec3) {
        let mut command_buffer = self.cmdbufs.compute_command_buffers.current();

        let cam_shift = radiance_shift + 0;

        if cam_shift.x.abs() >= self.settings.world_size.x as i32
        || cam_shift.y.abs() >= self.settings.world_size.y as i32
        || cam_shift.z.abs() >= self.settings.world_size.z as i32 {
            return; // then its pointless (zero-volume intersection). We can set it to zero os some pre-computed value in future, tho
        }

        let process_axis = |shift : i32, world_size : i32| -> ivec2 {
            let self_src_offset;
            let self_dst_offset;
            let extent = shift.abs();
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

        let self_src_offset = ivec3::new(
            process_axis(cam_shift.x, self.settings.world_size.x as i32).x, 
            process_axis(cam_shift.y, self.settings.world_size.y as i32).x, 
            process_axis(cam_shift.z, self.settings.world_size.z as i32).x
        );
        let self_dst_offset = ivec3::new(
            process_axis(cam_shift.x, self.settings.world_size.x as i32).y, 
            process_axis(cam_shift.y, self.settings.world_size.y as i32).y, 
            process_axis(cam_shift.z, self.settings.world_size.z as i32).y
        );

        let intersection_size = uvec3::new(
            self.settings.world_size.x - cam_shift.x.abs() as u32,
            self.settings.world_size.y - cam_shift.y.abs() as u32,
            self.settings.world_size.z - cam_shift.z.abs() as u32,
        );

        let mut copy_region = vk::ImageCopy {
            src_subresource: vk::ImageSubresourceLayers::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .layer_count(1)
                .mip_level(0)
                .build(),
            dst_subresource: vk::ImageSubresourceLayers::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .layer_count(1)
                .mip_level(0)
                .build(),
            extent: vk::Extent3D::builder()
                .width(intersection_size.x)
                .height(intersection_size.y)
                .depth(intersection_size.z)
                .build(),
            src_offset: vk::Offset3D::builder()
                .x(self_src_offset.x) // we want 0,0,0 to end up in shift
                .y(self_src_offset.y)
                .z(self_src_offset.z)
                .build(),
            dst_offset: vk::Offset3D::builder()
                .x(0) // no reason to copy anywhere else - DST IS TEMP STORAGE
                .y(0)
                .z(0)
                .build(),
        };

        self.lumal.image_memory_barrier(
            command_buffer,
            &self.independent_images.radiance_cache.current(),
            vk::PipelineStageFlags::TRANSFER, // well sometimes i feel like i should pick better barriers
            vk::PipelineStageFlags::TRANSFER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            &self.independent_images.radiance_cache.previous(),
            vk::PipelineStageFlags::TRANSFER, // well sometimes i feel like i should pick better barriers
            vk::PipelineStageFlags::TRANSFER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
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
            &self.independent_images.radiance_cache.current(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            &self.independent_images.radiance_cache.previous(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );

        // copy back (setting up region)
            copy_region.extent = vk::Extent3D::builder()
                .width(intersection_size.x)
                .height(intersection_size.y)
                .depth(intersection_size.z)
                .build();
            copy_region.src_offset = vk::Offset3D::builder()
                .x(0) // we want 0,0,0 to end up in shift
                .y(0)
                .z(0)
                .build();
            copy_region.dst_offset = vk::Offset3D::builder()
                .x(self_dst_offset.x) // well, this is how to tell it to end up in (shift)
                .y(self_dst_offset.y)
                .z(self_dst_offset.z)
                .build();
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
            &self.independent_images.radiance_cache.current(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            command_buffer,
            &self.independent_images.radiance_cache.previous(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
    }

    pub fn exec_copies(&mut self) {
        let mut command_buffer = self.cmdbufs.compute_command_buffers.current();

        let clear_color = vk::ClearColorValue::default();
        let clear_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1)
            .build();

        // Transition images for copying (lol no transition atm)
        self.lumal.image_memory_barrier(
            &command_buffer,
            &self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            &command_buffer,
            &self.independent_images.origin_block_palette.current(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );

        unsafe {
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
            &command_buffer,
            &self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            &command_buffer,
            &self.independent_images.origin_block_palette.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
        
        let static_block_palette_copy = vk::ImageCopy {
            src_subresource: vk::ImageSubresourceLayers::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .layer_count(1)
                .mip_level(0)
                .build(),
            dst_subresource: vk::ImageSubresourceLayers::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .layer_count(1)
                .mip_level(0)
                .build(),
            extent: vk::Extent3D::builder()
                .width (16 * self.static_block_palette_size)
                .height(16)
                .depth (16)
                .build(),
            src_offset: vk::Offset3D::builder()
                .x(0)
                .y(0)
                .z(0)
                .build(),
            dst_offset: vk::Offset3D::builder()
                .x(0)
                .y(0)
                .z(0)
                .build(),
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
            &command_buffer,
            &self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            &command_buffer,
            &self.independent_images.origin_block_palette.current(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );

        // Execute actual block copy for each allocated temporal block
        // TODO: maybe we should copy from current to current, cause these blocks are allocated, and we just copy (clone)
        // the static blocks to allocated ones. So they never intersect. Maybe its faster
        if !self.block_copies_queue.is_empty() { // idk if copying 0 is allowed
            unsafe { self.lumal.device.cmd_copy_image(
                *command_buffer,
                self.independent_images.origin_block_palette.previous().image,
                vk::ImageLayout::GENERAL,
                self.independent_images.origin_block_palette.current().image,
                vk::ImageLayout::GENERAL,
                self.block_copies_queue.as_slice(),
            ) };
        }

        // sync
        self.lumal.image_memory_barrier(
            &command_buffer,
            &self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
        self.lumal.image_memory_barrier(
            &command_buffer,
            &self.independent_images.origin_block_palette.current(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );

        // copy the entire world buffer to the world image (there is no direct way so intermediate copy (buffer) is needed)
        let copy_region = vk::BufferImageCopy::builder()
            .image_extent(vk::Extent3D {
                width: self.settings.world_size.x,
                height: self.settings.world_size.y,
                depth: self.settings.world_size.z,
            })
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            });

        // sync
        self.lumal.image_memory_barrier(
            &command_buffer,
            &self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
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
            &command_buffer,
            &self.independent_images.origin_block_palette.previous(),
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
    }

    pub fn start_map(&mut self) {
        let mut command_buffer = self.cmdbufs.compute_command_buffers.current();

        self.lumal.bind_compute_pipe(
            &command_buffer,
            &self.pipes.map_pipe,
        );
    }

    pub fn map_mesh(&mut self, mesh: &InternalMeshModel, trans: &MeshTransform) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();
        let model_voxels_info = vk::DescriptorImageInfo::builder()
            .image_view(self.independent_images.world.current().view)
            .image_layout(vk::ImageLayout::GENERAL);
        let binding = [model_voxels_info];
        let model_voxels_write = vk::WriteDescriptorSet::builder()
            .dst_set(vk::DescriptorSet::null())
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&binding);

        unsafe {
            self.lumal.device.cmd_push_descriptor_set_khr(
                *command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipes.map_pipe.line_layout,
                1,
                &[model_voxels_write],
            ) 
        };

        let rotate = vek::Mat4::from(trans.rotation);
        let shift = vek::Mat4::<f32>::identity().translated_3d(trans.translation);  
        let transform = shift * rotate;
        let border_in_voxel = get_shift(shift * rotate, mesh.size);   

        let mut border_in_voxel = get_shift(transform, mesh.size);

        let mut border = iAABB {
            min: ivec3::new(
                border_in_voxel.min.x.floor() as i32,
                border_in_voxel.min.y.floor() as i32,
                border_in_voxel.min.z.floor() as i32,
            ),
            max: ivec3::new(
                border_in_voxel.max.x.ceil() as i32, 
                border_in_voxel.max.y.ceil() as i32,
                border_in_voxel.max.z.ceil() as i32,
            ),
        };

        let map_area = border.max - border.min;
        #[repr(C)]// for push constants
        #[derive(AsU8Slice)]// allow cast to &[u8]
        struct PushConstant {
            trans: mat4,
            shift: ivec4,
        }
        let push_constant = PushConstant {
            trans: transform,
            shift: ivec4::new(
                border.min.x,
                border.min.y,
                border.min.z,
                0,
            ),
        };
        
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.map_pipe.line_layout,
                ShaderStageFlags::COMPUTE,
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
            &command_buffer,
            &self.independent_images.world.current(),
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            AccessFlags::SHADER_WRITE,
            AccessFlags::SHADER_READ,
            vk::ImageLayout::GENERAL, vk::ImageLayout::GENERAL,
        );
    }

    pub fn end_compute(&mut self) {
        let command_buffer = self.cmdbufs.compute_command_buffers.current();
        // do nothing
    }

    pub fn start_lightmap(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();
        
        #[repr(C)]// for push constants
        #[derive(AsU8Slice)]// allow cast to &[u8]
        struct PushConstant {
            trans: mat4,
        }
        let push_constant = PushConstant {
            trans: self.light.light_transform,
        };

        // sync
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.light_uniform.current(),
            PipelineStageFlags::ALL_COMMANDS,
            PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
        );

        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.lightmap_blocks_pipe.line_layout,
                ShaderStageFlags::COMPUTE,
                0,
                push_constant.as_u8_slice(),
            )
        };

        // sync
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.light_uniform.current(),
            PipelineStageFlags::ALL_COMMANDS,
            PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
        );
    }

    pub fn lightmap_start_blocks(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();
        
        self.lumal.bind_raster_pipe(
            &command_buffer,
            &self.pipes.lightmap_blocks_pipe,
        );
    }

    pub fn lightmap_start_models(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();
        
        self.lumal.bind_raster_pipe(
            &command_buffer,
            &self.pipes.lightmap_models_pipe,
        );
    }

    pub fn end_lightmap(&mut self) {
        let command_buffer = self.cmdbufs.lightmap_command_buffers.current();

        unsafe { 
            self.lumal.device.cmd_end_render_pass(
                *command_buffer
            ) 
        };
    }

    pub fn start_raygen(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();
        
        #[repr(C)]// for push constants
        #[derive(AsU8Slice)]// allow cast to &[u8]
        struct PushConstant {
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

        let push_constant = PushConstant {
            trans_w2s: self.camera.camera_transform,
            campos: vec4::new(self.camera.camera_pos.x, self.camera.camera_pos.y, self.camera.camera_pos.z, 0.0),
            camdir: vec4::new(self.camera.camera_dir.x, self.camera.camera_dir.y, self.camera.camera_dir.z, 0.0),
            horizline_scaled: vec4::new(self.camera.horizline.x * self.camera.view_size.x / 2.0, 0.0, 0.0, 0.0),
            vertiline_scaled: vec4::new(self.camera.vertiline.y * self.camera.view_size.y / 2.0, 0.0, 0.0, 0.0),
            global_light_dir: vec4::new(self.light.light_dir.x, self.light.light_dir.y, self.light.light_dir.z, 0.0),
            lightmap_proj: self.light.light_transform,
            size: vec2::new(self.lumal.vulkan_data.swapchain_extent.width as f32, self.lumal.vulkan_data.swapchain_extent.height as f32),
            timeseed: self.lumal.frame as i32,
        };

        // sync
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.uniform.current(),
            PipelineStageFlags::ALL_COMMANDS,
            PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
        );
        
        unsafe {
            self.lumal.device.cmd_push_constants(
                *command_buffer,
                self.pipes.raygen_blocks_pipe.line_layout,
                ShaderStageFlags::COMPUTE,
                0,
                push_constant.as_u8_slice(),
            )
        };

        // sync
        self.lumal.buffer_memory_barrier(
            command_buffer,
            self.buffers.uniform.current(),
            PipelineStageFlags::ALL_COMMANDS,
            PipelineStageFlags::ALL_COMMANDS,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
        );
    }

    pub fn raygen_start_blocks(&mut self) {
        let command_buffer = self.cmdbufs.graphics_command_buffers.current();

        self.lumal.bind_raster_pipe(
            &command_buffer,
            &self.pipes.raygen_blocks_pipe,
        );
    }
/*
static bool is_face_visible (vec3 normal, vec3 camera_dir) {
    return (dot (normal, camera_dir) < 0.0f);
}

#define CHECK_N_DRAW_BLOCK(__norm, __dir) \
if(is_face_visible(i8vec3(__norm), camera.cameraDir)) {\
    draw_block_face(__norm, (*block_mesh).triangles.__dir, block_id);\
}
void LumInternal::LumInternalRenderer::draw_block_face (i8vec3 normal, IndexedVertices& buff, int block_id) {
    CommandBuffer& commandBuffer = graphicsCommandBuffers.current();
    // assert (buff.indexes.data());
    assert (block_id);
    i8 sum = normal.x + normal.y + normal.z;
    u8 sign = (sum > 0) ? 0 : 1;
    u8vec3 absnorm = abs (normal);
    // assert(sign != 0);
    assert ((absnorm.x + absnorm.y + absnorm.z) == 1);
    u8 pbn = (
                 sign << 7 |
                 absnorm.x << 0 |
                 absnorm.y << 1 |
                 absnorm.z << 2);
    //signBit_4EmptyBits_xBit_yBit_zBit
    struct {u8vec4 inorm;} norms = {u8vec4 (pbn, 0, 0, 0)};
    vkCmdPushConstants (commandBuffer.commandBuffer, raygenBlocksPipe.lineLayout, VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT,
        8, sizeof (norms), &norms);

    commandBuffer.cmdDrawIndexed (buff.icount, 1, buff.offset, 0, 0);
}

void LumInternal::LumInternalRenderer::raygen_block (InternalMeshModel* block_mesh, int block_id, ivec3 shift) {
    CommandBuffer& commandBuffer = graphicsCommandBuffers.current();
    VkBuffer vertexBuffers[] = {(*block_mesh).triangles.vertexes.buffer};
    VkDeviceSize offsets[] = {0};
    DEBUG_LOG((*block_mesh).triangles.vertexes.buffer)

    commandBuffer.cmdBindVertexBuffers (0, 1, vertexBuffers, offsets);
    commandBuffer.cmdBindIndexBuffer ((*block_mesh).triangles.indices.buffer, 0, VK_INDEX_TYPE_UINT16);
    ;

    /*
        int16_t block;
        i16vec3 shift;
        i8vec4 inorm;
    */
    struct {i16 block; i16vec3 shift;} blockshift = {i16 (block_id), i16vec3 (shift)};
    vkCmdPushConstants (commandBuffer.commandBuffer, raygenBlocksPipe.lineLayout, VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT,
        0, sizeof (blockshift), &blockshift);
    CHECK_N_DRAW_BLOCK (i8vec3 (+1, 0, 0), Pzz);
    CHECK_N_DRAW_BLOCK (i8vec3 (-1, 0, 0), Nzz);
    CHECK_N_DRAW_BLOCK (i8vec3 (0, +1, 0), zPz);
    CHECK_N_DRAW_BLOCK (i8vec3 (0, -1, 0), zNz);
    CHECK_N_DRAW_BLOCK (i8vec3 (0, 0, +1), zzP);
    CHECK_N_DRAW_BLOCK (i8vec3 (0, 0, -1), zzN);
}
*/
    // pub fn is_face_visible(&mut self, face: PackedVoxelQuad, pos: ivec3) -> bool {

    // }
}
