//! Module for managing buffers

use crate::{Buffer, Renderer};
use ash::vk::{self, BufferUsageFlags};
use containers::Ring;
use gpu_allocator::vulkan::{self as vma};
use std::ptr::copy_nonoverlapping;

impl Renderer {
    /// Creates Buffer. Optionally maps it.
    pub fn create_buffer(
        &mut self,
        usage: vk::BufferUsageFlags,
        size: usize,
        host: bool,
    ) -> Buffer {
        let buffer_info = vk::BufferCreateInfo {
            flags: vk::BufferCreateFlags::empty(),
            size: size as vk::DeviceSize,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            ..Default::default()
        };

        let location = if host {
            gpu_allocator::MemoryLocation::CpuToGpu
        } else {
            gpu_allocator::MemoryLocation::GpuOnly
        };

        let vk_buffer = unsafe { self.device.create_buffer(&buffer_info, None) }.unwrap();
        let requirements = unsafe { self.device.get_buffer_memory_requirements(vk_buffer) };

        let alloc_info = vma::AllocationCreateDesc {
            requirements,
            location,
            allocation_scheme: vma::AllocationScheme::GpuAllocatorManaged,
            linear: true, // buffers are always linear
            name: "",
        };

        let allocation = self.allocator.allocate(&alloc_info).unwrap();

        // bind memory to the buffer
        unsafe {
            self.device
                .bind_buffer_memory(vk_buffer, allocation.memory(), allocation.offset())
                .unwrap()
        };

        // TODO: Integrated CPU memory utilization
        // TODO: what if it fails? Different set of flags?
        Buffer {
            buffer: vk_buffer,
            allocation,
        }
    }

    /// Creates Ring of Buffers. Optionally maps them.
    pub fn create_buffer_rings(
        &mut self,
        ring_size: usize,
        usage: vk::BufferUsageFlags,
        biffer_size: usize,
        host: bool,
    ) -> Ring<Buffer> {
        (0..ring_size).map(|_| self.create_buffer(usage, biffer_size, host)).collect()
    }

    /// Destroys Buffer.
    pub fn destroy_buffer(&mut self, buf: Buffer) {
        unsafe {
            // unmap if mapped
            // match buf.mapped {
            //     Some(_) => self.device.unmap_memory(buf.allocation.memory()),
            //     None => {} // do nothing
            // }
            self.allocator.free(buf.allocation).unwrap();
            self.device.destroy_buffer(buf.buffer, None);
        };
    }

    /// Destroys Ring of Buffers.
    pub fn destroy_buffer_ring(&mut self, buffers: Ring<Buffer>) {
        for buf in buffers.data {
            self.destroy_buffer(buf);
        }
    }

    /// creates a Buffer and copies given elements into it
    /// does buffer_usage |= TRANSFER_DST automatically
    // TODO: only u8, why the fuck did i make a template?
    pub fn create_and_upload_buffer<T>(
        &mut self,
        elements: &[T],
        mut buffer_usage: vk::BufferUsageFlags,
    ) -> Buffer {
        buffer_usage |= vk::BufferUsageFlags::TRANSFER_DST;

        let count = elements.len();
        let size = std::mem::size_of_val(elements);
        let buffer = self.create_buffer(
            buffer_usage,
            size,
            false, // TODO: bool -> Enum
        );

        let staging_buffer = self.create_buffer(BufferUsageFlags::TRANSFER_SRC, size, true);

        unsafe {
            copy_nonoverlapping(
                elements.as_ptr(),
                staging_buffer.allocation.mapped_ptr().unwrap().as_ptr() as *mut T,
                count,
            );
        }

        self.copy_buffer_to_buffer_single_time(
            staging_buffer.buffer,
            buffer.buffer,
            size as vk::DeviceSize,
        );

        self.destroy_buffer(staging_buffer);

        buffer
    }
    // create elem ring not implemented.
}
