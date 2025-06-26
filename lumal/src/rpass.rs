//! Module for managing RenderPasses

use crate::{
    descriptors::{
        AttachmentDescription, LoadStoreOp, MaybeRing, SubpassAttachmentRefs, SubpassDescription,
    },
    Image, RenderPass, Renderer,
};
use ash::vk;
use containers::Ring;
use std::{collections::HashMap, ptr::null};

impl Renderer {
    /// Destroys RenderPass. Does not destroy resources, refered to by RenderPass (like attachment's images)
    pub fn destroy_renderpass(&mut self, rpass: RenderPass) {
        assert!(rpass.render_pass != vk::RenderPass::null());
        assert!(!rpass.framebuffers.is_empty());
        for framebuffer in rpass.framebuffers.into_iter() {
            assert!(*framebuffer != vk::Framebuffer::null());
            unsafe {
                self.device.destroy_framebuffer(*framebuffer, None);
            }
        }

        unsafe {
            self.device.destroy_render_pass(rpass.render_pass, None);
        }
    }

    /// Creates RenderPass object from given description.
    /// Attachments describe specific (maybe rings of) images and what happens with them in renderpass.
    /// Each subpass description specifies which pipes operate on which (maybe rings of) images.
    // Vulkan actually wants array of attachments and indices,
    // so we convert (maybe rings of) image reference(s) from subpass descriptions to indices in array of attachments (by hashmap or smth)
    pub fn create_renderpass(
        &self,
        attachments: &[AttachmentDescription],
        subpass_descriptions: &mut [SubpassDescription],
    ) -> RenderPass {
        let mut rpass = RenderPass::default();

        // no subpasses / attachments is invalid and i dont like returning errors
        assert!(!attachments.is_empty());
        assert!(!subpass_descriptions.is_empty());

        let mut adescs = vec![vk::AttachmentDescription::default(); attachments.len()];
        let mut arefs = vec![vk::AttachmentReference::default(); attachments.len()];
        // instead of forcing user into specifying indices of attachments, we determine them from comparing references
        let mut img2ref = HashMap::new();
        let mut clears = Vec::new();

        for (i, attachment) in attachments.iter().enumerate() {
            // reference to first image in the Ring of images given by pointer
            let first_image = attachment.images.get_first(); //

            adescs[i] = vk::AttachmentDescription {
                format: first_image.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: attachment.load.to_vk_load(),
                store_op: attachment.store.to_vk_store(),
                stencil_load_op: attachment.sload.to_vk_load(),
                stencil_store_op: attachment.sstore.to_vk_store(),
                initial_layout: if (attachment.load == LoadStoreOp::DontCare
                    || attachment.load == LoadStoreOp::Clear)
                    && (attachment.sload == LoadStoreOp::DontCare
                        || attachment.sload == LoadStoreOp::Clear)
                {
                    vk::ImageLayout::UNDEFINED
                } else {
                    vk::ImageLayout::GENERAL
                },
                final_layout: attachment.final_layout,
                flags: vk::AttachmentDescriptionFlags::empty(),
            };
            // so i'th attachment reference references to i'th attachment. Very convenient
            arefs[i] = vk::AttachmentReference {
                attachment: i as u32,
                layout: vk::ImageLayout::GENERAL,
            };

            // we cast it to pointer because otherwise its implicitly dereferenced
            img2ref.insert(first_image as *const _, i);

            clears.push(attachment.clear);
        }

        rpass.clear_colors = clears;

        // this vec's are used to figure out vulkan stuff from what user supplied
        // i just feel like passing references is more convenient than manually recomputing indices every time
        let mut subpasses = vec![vk::SubpassDescription::default(); subpass_descriptions.len()];
        let mut sas_refs = vec![SubpassAttachmentRefs::default(); subpass_descriptions.len()];

        for (i, subpass) in subpass_descriptions.iter().enumerate() {
            if let Some(depth) = &subpass.a_depth {
                let index = *img2ref.get(&(depth.get_first() as *const _)).unwrap();
                sas_refs[i].a_depth = Some(arefs[index])
            } else {
                sas_refs[i].a_depth = None;
            };
            for color in subpass.a_color {
                let index = *img2ref.get(&(color.get_first() as *const _)).unwrap();
                sas_refs[i].a_color.push(arefs[index]);
            }
            for input in subpass.a_input {
                let index = *img2ref.get(&(input.get_first() as *const _)).unwrap();
                sas_refs[i].a_input.push(arefs[index]);
            }
        }

        assert!(subpasses.len() == sas_refs.len());
        for (i, sas) in sas_refs.iter_mut().enumerate() {
            subpasses[i].color_attachment_count = sas.a_color.len() as u32;
            subpasses[i].p_color_attachments = sas.a_color.as_ptr();
            subpasses[i].input_attachment_count = sas.a_input.len() as u32;
            subpasses[i].p_input_attachments = sas.a_input.as_ptr();
            // we cant just reference attachment hidden in Option because its literally not what we want
            // aka we want *a_depth, not *Option<a_depth> cause there is (might be) more bits (from enum)
            subpasses[i].p_depth_stencil_attachment = match sas.a_depth {
                Some(_) => sas.a_depth.as_mut().unwrap(),
                None => null(),
            }
        }

        // for every subpass, set subpass_id of every pipe in that subpass to the subpass index
        for i in 0..subpass_descriptions.len() {
            for pipe in &mut *subpass_descriptions[i].pipes {
                pipe.subpass_id = i as i32;
            }
        }
        // alternative (how is that so complicated?)
        // spass_attachs
        //     .iter_mut()
        //     .enumerate()
        //     .map(|(i, spass)| spass.pipes.iter_mut().map(move |pipe| pipe.subpass_id = i as i32));

        // not real vulkan struct, just barriers inside a subpass (currently, dummy barriers)
        let dependencies = Self::create_subpass_dependencies(subpass_descriptions);

        // typical Vulkan createinfo struct

        let create_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            attachment_count: adescs.len() as u32,
            p_attachments: adescs.as_ptr(),
            subpass_count: subpasses.len() as u32,
            p_subpasses: subpasses.as_ptr(),
            dependency_count: dependencies.len() as u32,
            p_dependencies: dependencies.as_ptr(),
            ..Default::default()
        };

        // call Vulkan function to actually create the render pass
        let render_pass = unsafe {
            self.device
                .create_render_pass(&create_info, None)
                .expect("Failed to create render pass")
        };

        // Pipes (which are abstractions of Vulkan pipelines) need to know the render pass
        for subpass in subpass_descriptions {
            for pipe in &mut *subpass.pipes {
                pipe.renderpass = render_pass;
            }
        }

        rpass.render_pass = render_pass;
        // This is the metadata i store in my render pass abstraction. It helps (me).
        // TODO: atm we hope that they all match extent
        rpass.extent = vk::Extent2D {
            width: attachments[0].images.get_first().extent.width,
            height: attachments[0].images.get_first().extent.height,
        };

        let binding: Vec<&MaybeRing<Image>> = attachments.iter().map(|desc| &desc.images).collect();
        let fb_images = binding.as_slice();

        rpass.framebuffers = self.create_framebuffers(
            render_pass,
            fb_images,
            rpass.extent.width,
            rpass.extent.height,
        );

        rpass
    }

    // Function to create subpass dependencies

    fn create_subpass_dependencies(
        spass_attachs: &[SubpassDescription],
    ) -> Vec<vk::SubpassDependency> {
        let mut dependencies = Vec::new();

        // Initial external to first subpass dependency
        dependencies.push(vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::ALL_GRAPHICS
                | vk::PipelineStageFlags::ALL_COMMANDS,
            dst_stage_mask: vk::PipelineStageFlags::ALL_GRAPHICS,
            src_access_mask: vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            dst_access_mask: vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            dependency_flags: vk::DependencyFlags::empty(),
        });

        // Full wait dependencies between all subpasses
        for i in 0..spass_attachs.len() {
            for j in (i + 1)..spass_attachs.len() {
                dependencies.push(vk::SubpassDependency {
                    src_subpass: i as u32,
                    dst_subpass: j as u32,
                    src_stage_mask: vk::PipelineStageFlags::ALL_GRAPHICS,
                    dst_stage_mask: vk::PipelineStageFlags::ALL_GRAPHICS,
                    src_access_mask: vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
                    dst_access_mask: vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
                    dependency_flags: vk::DependencyFlags::BY_REGION,
                });
            }
        }

        // Final dependency from last subpass to external
        dependencies.push(vk::SubpassDependency {
            src_subpass: (spass_attachs.len() - 1) as u32,
            dst_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::ALL_GRAPHICS,
            dst_stage_mask: vk::PipelineStageFlags::ALL_GRAPHICS
                | vk::PipelineStageFlags::ALL_COMMANDS,
            src_access_mask: vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            dst_access_mask: vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
            dependency_flags: vk::DependencyFlags::empty(),
        });

        dependencies
    }

    // Function to create framebuffers

    fn create_framebuffers(
        &self,
        // device: &vulkanalia::Device,
        render_pass: vk::RenderPass,
        imgs4views: &[&MaybeRing<Image>],
        width: u32,
        height: u32,
    ) -> Ring<vk::Framebuffer> {
        // Calculate Least Common Multiple (LCM) of the sizes of the image view rings
        let lcm = imgs4views.iter().map(|imgs| imgs.len()).fold(1, lcm_custom);
        assert!(lcm != 0);

        let mut framebuffers = Ring::new(lcm);

        for i in 0..lcm {
            let mut attachment_views = Vec::new();

            for imgs in imgs4views {
                let internal_iter = i % imgs.len();
                attachment_views.push(imgs[internal_iter].view);
            }

            let framebuffer_info = vk::FramebufferCreateInfo {
                render_pass,
                attachment_count: attachment_views.len() as u32,
                p_attachments: attachment_views.as_ptr(),
                width,
                height,
                layers: 1,
                ..Default::default()
            };

            let framebuffer = unsafe {
                self.device
                    .create_framebuffer(&framebuffer_info, None)
                    .expect("Failed to create framebuffer")
            };

            framebuffers[i] = framebuffer;
        }

        framebuffers
    }

    /// Submits command(s) to begin RenderPass.
    /// Wrapper around cmd_begin_render_pass + cmd_set_viewport
    pub fn cmd_begin_renderpass(
        &self,
        command_buffer: &vk::CommandBuffer,
        render_pass: &RenderPass,
        inline: vk::SubpassContents,
    ) {
        let begin_info = vk::RenderPassBeginInfo {
            render_pass: render_pass.render_pass,
            framebuffer: *render_pass.framebuffers.current(),
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: render_pass.extent,
            },
            clear_value_count: render_pass.clear_colors.len() as u32,
            p_clear_values: render_pass.clear_colors.as_slice().as_ptr(),
            ..Default::default()
        };

        unsafe {
            self.device.cmd_begin_render_pass(*command_buffer, &begin_info, inline);
            self.cmd_set_viewport(
                *command_buffer,
                render_pass.extent.width,
                render_pass.extent.height,
            );
        }
    }

    /// Ends renderpass and moves its next framebuffer
    pub fn cmd_end_renderpass(
        &self,
        command_buffer: &vk::CommandBuffer,
        render_pass: &mut RenderPass,
    ) {
        unsafe {
            self.device.cmd_end_render_pass(*command_buffer);
        }
        render_pass.framebuffers.move_next();
    }
}

fn gcd(a: usize, b: usize) -> usize {
    let mut a_copy = a;
    let mut b_copy = b;
    while b_copy != 0 {
        let temp = b_copy;
        b_copy = a_copy % b_copy;
        a_copy = temp;
    }
    a_copy
}

fn lcm_custom(a: usize, b: usize) -> usize {
    if a == 0 || b == 0 {
        return 0;
    }
    (a * b) / gcd(a, b)
}
