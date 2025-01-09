use lumal::{descriptors::{DescriptorInfo, RelativeDescriptorPos, ShortDescriptorInfo}, ring::Ring, ComputePipe, LumalRenderer, LumalSettings, RasterPipe};
use vulkanalia::vk::{self, DeviceV1_0, Extent2D, Handle};
use vk::Sampler;
use RelativeDescriptorPos::*;

use crate::*;

impl crate::LumRenderer {
    pub fn create_independent_images(
        lumal: &LumalRenderer, 
        lum_settings: &LumSettings, 
        lumal_settings: &LumalSettings
    ) -> LumIndependentImages {
        let world = lumal.create_image_ring (
            lumal_settings.fif,
            vk::ImageType::_3D,
            vk::Format::R16_SINT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(lum_settings.world_size),
            1,
            vk::SampleCountFlags::_1,
            ); //TODO: dynamic
        let lightmap = lumal.create_image_ring (
            lumal_settings.fif,
            vk::ImageType::_2D,
            LIGHTMAPS_FORMAT,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::DEPTH,
            vk::Extent3D {
                width: lum_settings.lightmap_extent.width, 
                height: lum_settings.lightmap_extent.height, 
                depth: 1},
            1,
            vk::SampleCountFlags::_1,
            );
        let radiance_cache = lumal.create_image_ring (
            lumal_settings.fif,
            vk::ImageType::_3D,
            RADIANCE_FORMAT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(lum_settings.world_size),
            1,
            vk::SampleCountFlags::_1,
            );
        let origin_block_palette = lumal.create_image_ring (
            lumal_settings.fif,
            vk::ImageType::_3D,
            vk::Format::R8_UINT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: 16 * BLOCK_PALETTE_SIZE_X, 
                height: 16 * BLOCK_PALETTE_SIZE_Y, 
                depth: 16},
            1,
            vk::SampleCountFlags::_1,
            );
        let material_palette = lumal.create_image_ring (
            lumal_settings.fif,
            vk::ImageType::_2D,
            vk::Format::R32_SFLOAT, //try R32G32
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: 6, 
                height: 256, 
                depth: 1},
            1,
            vk::SampleCountFlags::_1,
            );
        let grass_state = lumal.create_image_ring (
            lumal_settings.fif,
            vk::ImageType::_2D,
            vk::Format::R16G16_SFLOAT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: lum_settings.world_size.x*2, 
                height: lum_settings.world_size.y*2, 
                depth: 1},
            1,
            vk::SampleCountFlags::_1);
        let water_state = lumal.create_image_ring (
            lumal_settings.fif,
            vk::ImageType::_2D,
            vk::Format::R16G16B16A16_SFLOAT,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: lum_settings.world_size.x*2, 
                height: lum_settings.world_size.y*2, 
                depth: 1},
            1,
            vk::SampleCountFlags::_1);
        let perlin_noise2d = lumal.create_image_ring (
            lumal_settings.fif,
            vk::ImageType::_2D,
            vk::Format::R16G16_SNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: lum_settings.world_size.x, 
                height: lum_settings.world_size.y, 
                depth: 1},
            1,
            vk::SampleCountFlags::_1); //does not matter than much
        let perlin_noise3d = lumal.create_image_ring (
            lumal_settings.fif,
            vk::ImageType::_3D,
            vk::Format::R16G16B16A16_UNORM,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::empty(),
            vk::ImageAspectFlags::COLOR,
            vk::Extent3D {
                width: 32, 
                height: 32, 
                depth: 32},
            1,
            vk::SampleCountFlags::_1); //does not matter than much
        
        return LumIndependentImages {
            grass_state: grass_state.unwrap(),
            water_state: water_state.unwrap(),
            perlin_noise2d: perlin_noise2d.unwrap(),
            perlin_noise3d: perlin_noise3d.unwrap(),
            world: world.unwrap(),
            radiance_cache: radiance_cache.unwrap(),
            origin_block_palette: origin_block_palette.unwrap(),
            lightmap: lightmap.unwrap(),
            // distance_palette: distance_palette.unwrap(),
            // bit_palette: bit_palette.unwrap(),
            material_palette: material_palette.unwrap(),
        };
    }
    
    // dependent = swapchain dependent
    pub fn create_dependent_images(
        lumal: &LumalRenderer, 
        lum_settings: &LumSettings, 
        lumal_settings: &LumalSettings
    ) -> LumSwapchainDependentImages {
        let sextent = uvec3::new(
            lumal.vulkan_data.swapchain_extent.width,
            lumal.vulkan_data.swapchain_extent.height,
            1
        );

        let highres_mat_norm = lumal.create_image_ring(
            lumal_settings.fif,
            vk::ImageType::_2D,
            MATNORM_FORMAT,
            vk::ImageUsageFlags::STORAGE 
                | vk::ImageUsageFlags::TRANSFER_SRC 
                | vk::ImageUsageFlags::TRANSFER_DST 
                | vk::ImageUsageFlags::SAMPLED 
                | vk::ImageUsageFlags::COLOR_ATTACHMENT 
                | vk::ImageUsageFlags::INPUT_ATTACHMENT,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(sextent),
            1,
            vk::SampleCountFlags::_1,
        ).unwrap();
        
        let mut highres_depth_stencil = lumal.create_image_ring(
            lumal_settings.fif,
            vk::ImageType::_2D,
            unsafe { CHOSEN_DEPTH_FORMAT },
            vk::ImageUsageFlags::TRANSFER_SRC 
                | vk::ImageUsageFlags::TRANSFER_DST 
                | vk::ImageUsageFlags::SAMPLED 
                | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT 
                | vk::ImageUsageFlags::INPUT_ATTACHMENT,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
            uvec3_to_extent3d(sextent),
            1,
            vk::SampleCountFlags::_1,
        ).unwrap();
        
        let highres_frame = lumal.create_image_ring(
            lumal_settings.fif,
            vk::ImageType::_2D,
            FRAME_FORMAT,
            vk::ImageUsageFlags::STORAGE 
                | vk::ImageUsageFlags::SAMPLED 
                | vk::ImageUsageFlags::TRANSFER_SRC 
                | vk::ImageUsageFlags::TRANSFER_DST 
                | vk::ImageUsageFlags::INPUT_ATTACHMENT 
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(sextent),
            1,
            vk::SampleCountFlags::_1,
        ).unwrap();
        
        // Create stencil views for the depth-stencil images
        let mut stencil_view_for_ds = Ring::new(
            lumal.settings.fif,
            vk::ImageView::default(), // Initial value for the Ring
        );
        for i in 0..lumal_settings.fif {
            let view_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                next: std::ptr::null(),

                flags: vk::ImageViewCreateFlags::empty(),
                image: highres_depth_stencil[i].image,
                view_type: vk::ImageViewType::_2D,
                format: unsafe { CHOSEN_DEPTH_FORMAT },
                components: vk::ComponentMapping::default(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::STENCIL,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            stencil_view_for_ds[i] = unsafe { lumal.device.create_image_view(&view_info, None).unwrap() };
        }
        
        let far_depth = lumal.create_image_ring(
            lumal_settings.fif,
            vk::ImageType::_2D,
            SECONDARY_DEPTH_FORMAT,
            vk::ImageUsageFlags::STORAGE 
                | vk::ImageUsageFlags::SAMPLED 
                | vk::ImageUsageFlags::TRANSFER_SRC 
                | vk::ImageUsageFlags::TRANSFER_DST 
                | vk::ImageUsageFlags::INPUT_ATTACHMENT 
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(sextent),
            1,
            vk::SampleCountFlags::_1,
        ).unwrap();
        
        let near_depth = lumal.create_image_ring(
            lumal_settings.fif,
            vk::ImageType::_2D,
            SECONDARY_DEPTH_FORMAT,
            vk::ImageUsageFlags::STORAGE 
                | vk::ImageUsageFlags::SAMPLED 
                | vk::ImageUsageFlags::TRANSFER_SRC 
                | vk::ImageUsageFlags::TRANSFER_DST 
                | vk::ImageUsageFlags::INPUT_ATTACHMENT 
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vulkanalia_vma::MemoryUsage::AutoPreferDevice,
            vulkanalia_vma::AllocationCreateFlags::DEDICATED_MEMORY,
            vk::ImageAspectFlags::COLOR,
            uvec3_to_extent3d(sextent),
            1,
            vk::SampleCountFlags::_1,
        ).unwrap();

        let mut swapchain_images_ring = Ring::new(
            lumal.vulkan_data.swapchain_images.len(),
            lumal::Image::default(), // Initial value for the Ring
        );
        
        for (i, swapchain_image) in lumal.vulkan_data.swapchain_images.iter().enumerate() {
            let image_view = lumal.vulkan_data.swapchain_image_views[i];
            let extent = vk::Extent3D {
                width: lumal.vulkan_data.swapchain_extent.width,
                height: lumal.vulkan_data.swapchain_extent.height,
                depth: 1,
            };

            let noalloc : vulkanalia_vma::Allocation = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
                // as vulkanalia_vma::vma::VmaAllocation;

            // Populate the Ring with actual data
            swapchain_images_ring.data[i] = lumal::Image {
                image: *swapchain_image,
                allocation: noalloc, // or some default/constructed allocation 
                view: image_view,
                mip_views: vec![image_view], // Add more mip views if necessary
                format: lumal.vulkan_data.swapchain_format,
                aspect: vk::ImageAspectFlags::COLOR,
                extent,
                mip_levels: 1, // Set this according to your mip levels
            };
        }

        return LumSwapchainDependentImages {
            swapchain_images: swapchain_images_ring, 
            highres_frame: highres_frame,
            highres_depth_stencil: highres_depth_stencil,
            highres_mat_norm: highres_mat_norm,
            stencil_view_for_ds: stencil_view_for_ds,
            far_depth: far_depth,
            near_depth: near_depth,
            // mask_frame: mask_frame,
        };
    }

    pub fn destroy_independent_images(&self) {
        println!("started destroying independent images");
        self.lumal.destroy_image_ring(&self.independent_images.grass_state);
        self.lumal.destroy_image_ring(&self.independent_images.water_state);
        self.lumal.destroy_image_ring(&self.independent_images.perlin_noise2d);
        self.lumal.destroy_image_ring(&self.independent_images.perlin_noise3d);
        self.lumal.destroy_image_ring(&self.independent_images.world);
        self.lumal.destroy_image_ring(&self.independent_images.radiance_cache);
        self.lumal.destroy_image_ring(&self.independent_images.origin_block_palette);
        self.lumal.destroy_image_ring(&self.independent_images.material_palette);
        self.lumal.destroy_image_ring(&self.independent_images.lightmap);
        println!("destroyed independent images");
    }

    pub fn destroy_dependent_images(&self) {
        println!("started destroying swapchain dependent images");
        
        // Not supposed to happen - swapchain images are destroyed by the driver
        // self.lumal.destroy_image_ring(&self.dependent_images.swapchain_images);

        self.lumal.destroy_image_ring(&self.dependent_images.highres_frame);
        self.lumal.destroy_image_ring(&self.dependent_images.highres_depth_stencil);
        self.lumal.destroy_image_ring(&self.dependent_images.highres_mat_norm);
        // self.lumal.destroy_image_ring(&self.dependent_images.stencil_view_for_ds);
        for stencil_view in self.dependent_images.stencil_view_for_ds.into_iter() {
            unsafe { self.lumal.device.destroy_image_view(*stencil_view, None) };
        }
        self.lumal.destroy_image_ring(&self.dependent_images.far_depth);
        self.lumal.destroy_image_ring(&self.dependent_images.near_depth);
        println!("destroyed swapchain dependent images");
    } 
}