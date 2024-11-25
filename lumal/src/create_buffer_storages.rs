use crate::{ring::Ring, Buffer, LumalRenderer}; // Import the LumalRenderer struct
use std::ptr;
use anyhow::*; 
use vulkanalia::vk::{self};

use vulkanalia_vma::{self as vma};
use vulkanalia_vma::Alloc;

impl LumalRenderer {
    pub fn create_buffer_storage(
        &self,
        usage: vk::BufferUsageFlags,
        size: usize,
        host: bool,
    ) -> Result<Buffer> {
        // buffers.allocate(self.vulkan_data.settings.fif as usize);
        // buffers = Ring::new(self.vulkan_data.settings.fif as usize, Buffer::default());

        let buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            // p_next: std::ptr::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: size as vk::DeviceSize,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            // p_queue_family_indices: std::ptr::null(),
            next: ptr::null(),
            queue_family_indices: ptr::null(),
        };

        let alloc_info = vma::AllocationOptions {
            flags: if host {
                vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
            } else {
                vma::AllocationCreateFlags::empty()
            },
            required_flags: if host {
                vk::MemoryPropertyFlags::HOST_VISIBLE
            } else {
                vk::MemoryPropertyFlags::empty()
            },
            usage: vma::MemoryUsage::Auto,
            ..Default::default()
        };

        let (vk_buffer, allocation) = unsafe { self
            .allocator
            .create_buffer(buffer_info, &alloc_info) }?;

        Ok(Buffer {
            buffer: vk_buffer,
            allocation,
            mapped: None
        })
    }

    // creates ring of vulkan buffers. Optionally maps
    pub fn create_buffer_storages(
        &self,
        ring_size: usize,
        usage: vk::BufferUsageFlags,
        biffer_size: usize,
        host: bool,
    ) -> Result<Ring<Buffer>> {
        // Create a vector to hold the images.
        let mut buffers = Vec::with_capacity(ring_size);

        // Initialize each image and push to the vector.
        for _ in 0..ring_size {
            let buffer = self.create_buffer_storage(
                usage,
                biffer_size,
                host,
            )?;
            buffers.push(buffer);
        }

        // Return the Ring initialized with the images.
        Ok(Ring {
            data: buffers,
            index: 0,
        })
    }
}