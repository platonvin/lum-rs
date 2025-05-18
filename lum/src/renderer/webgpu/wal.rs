use std::{borrow::Cow, collections::HashMap, num::NonZero};
use wgpu::{
    util::DeviceExt, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferBinding, BufferBindingType, ColorTargetState,
    ComputePipelineDescriptor, Features, FragmentState, FrontFace, MultisampleState,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource,
    VertexBufferLayout, VertexState,
};
use wgpu::{BindingResource, BindingType, DepthStencilState, Limits, ShaderStages};

use lumal::ring::Ring;

use winit::window::Window;

use super::SWAPCHAIN_FORMAT;

// Webgpu Abstraction Layer
pub struct Wal<'window> {
    // pub window: Window,
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'window>,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    // pub pc_buffers: Ring<wgpu::Buffer>,
    // pub pc_buffer_size: Option<wgpu::BufferSize>,
    pub frame_index: usize,
}

#[derive(Debug, Default)]
pub struct ComputePipe {
    pub pipeline: Option<wgpu::ComputePipeline>,
    pub pipeline_layout: Option<wgpu::PipelineLayout>,
    pub pc_buffers: Option<Ring<wgpu::Buffer>>,
    pub pc_size: u32,
    // current push constants offset
    pub current_pc_offset: u32,
    pub static_bind_groups: Option<Ring<wgpu::BindGroup>>,
    pub pc_bind_groups: Option<Ring<wgpu::BindGroup>>,
}
#[derive(Debug, Default)]
pub struct RasterPipe {
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub pipeline_layout: Option<wgpu::PipelineLayout>,
    // Bind groups that are the same accross frames (e.g. camera parameters)
    pub static_bind_groups: Option<Ring<wgpu::BindGroup>>,
    // layout for bind groups, where dynamic bindings need to be
    pub pc_buffers: Option<Ring<wgpu::Buffer>>,
    pub pc_size: u32,
    pub current_pc_offset: u32,
    pub dynamic_bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub pc_bind_groups: Option<Ring<wgpu::BindGroup>>,
}

pub struct PipePcWrite<'a> {
    write: Option<wgpu::QueueWriteBufferView<'a>>,
    buffers: Option<Ring<wgpu::Buffer>>,
}

#[derive(Clone, Debug)]
pub struct AttrFormOffs {
    pub format: wgpu::VertexFormat,
    pub binding: u32,
    pub offset: usize,
}

#[derive(Clone)]
pub enum ResourceType<'a> {
    PushConstant, // for marking where pc is
    Dynamic(BindingType),
    Static(BindingType, Ring<BindingResource<'a>>),
}

#[derive(Clone)]
pub struct BindGroupDescription<'a> {
    /// Binding index. Must match shader index and be unique inside a BindGroupLayout. A binding
    /// of index 1, would be described as `layout(set = 0, binding = 1) uniform` in shaders.
    pub binding: u32,
    /// Which shader stages can see this binding.
    pub visibility: ShaderStages,
    // / The type of the binding
    /// so, what function gets is array of rings, and they get converted to ring of arrays
    /// If bind group is dynamic, then Ring is empty
    pub resources: ResourceType<'a>,
}

#[derive(Clone, Debug)]
pub struct PushConstantDescription {
    pub size: u32,
    pub max_count: u32,
    pub stages: ShaderStages,
}
#[derive(Clone, Debug)]
pub struct ShaderStage {
    pub stage: ShaderStages,
    pub code: &'static str,
}

pub struct RenderPass {
    /// The clear color for the pass (for the single color attachment).
    pub clear_color: wgpu::Color,
    /// A ring (vector) of framebuffer attachments (in WGPU, these are TextureViews).
    /// Typically you create these from your offscreen textures.
    pub framebuffer_views: Option<Ring<wgpu::TextureView>>,
    /// Optional depth/stencil attachment view.
    pub depth_stencil_view: Option<Ring<wgpu::TextureView>>,
    /// The extent (width and height) of the render area.
    pub extent: winit::dpi::PhysicalSize<u32>,
}

pub struct Image {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl<'window> Wal<'window> {
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();
        // 1) Create instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        // 2) Create surface
        let surface = instance.create_surface(window).unwrap();

        // 3) Request an adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapters found");

        // 4) Request device + queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: Features::DEPTH32FLOAT_STENCIL8 | Features::FLOAT32_FILTERABLE,
                required_limits: Limits {
                    min_uniform_buffer_offset_alignment: 256,
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
            .unwrap();
        unsafe {
            SWAPCHAIN_FORMAT =
                Some(surface.get_capabilities(&adapter).formats[0].remove_srgb_suffix());
            dbg!(SWAPCHAIN_FORMAT);
        };

        // 5) Configure the swapchain (surface)
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: unsafe { SWAPCHAIN_FORMAT.unwrap() },
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // // Create the shared push constant buffer
        // let push_constant_buffers = (0..config.desired_maximum_frame_latency)
        //     .map(|_| {
        //         device.create_buffer(&wgpu::BufferDescriptor {
        //             label: Some("Shared Push Constant Buffer"),
        //             size: push_constant_buffer_size.map_or(0, |size| size.get()),
        //             usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        //             mapped_at_creation: false,
        //         })
        //     })
        //     .collect();

        Self {
            // window,
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
            // pc_buffers: push_constant_buffers,
            // pc_buffer_size: push_constant_buffer_size,
            frame_index: 0,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    // The usage controls what the buffer is used for.
    // If host_visible is true the buffer is created with MAP_WRITE (and optionally COPY_SRC)
    // so you can map it immediately.
    pub fn create_buffer(
        &self,
        mut usage: wgpu::BufferUsages,
        size: usize,
        host_visible: bool,
        label: Option<&str>,
    ) -> wgpu::Buffer {
        if host_visible {
            usage |= wgpu::BufferUsages::MAP_WRITE;
        }

        self.device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size: size as u64,
            usage,
            // For host buffers, we initialize mapped_at_creation=true so that the memory is available.
            mapped_at_creation: host_visible,
        })
    }

    // Creates a ring of buffers.
    pub fn create_buffer_rings(
        &self,
        ring_size: usize,
        usage: wgpu::BufferUsages,
        buffer_size: usize,
        host_visible: bool,
        label: Option<&str>,
    ) -> Ring<wgpu::Buffer> {
        (0..ring_size)
            .map(|_| self.create_buffer(usage, buffer_size, host_visible, label))
            .collect()
    }

    pub fn destroy_buffer(&self, buffer: wgpu::Buffer) {
        buffer.destroy();
    }

    pub fn destroy_buffer_ring(&self, buffers: Ring<wgpu::Buffer>) {
        for buffer in buffers.data {
            self.destroy_buffer(buffer);
        }
    }

    // Creates a GPU-only buffer, uploads provided data into it, and returns the buffer.
    //
    // The provided `usage` is OR‑ed with COPY_DST.
    // This method requires that the data type T implements Pod from bytemuck.
    #[inline]
    pub fn create_and_upload_buffer<T>(
        &self,
        elements: &[T],
        mut usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        // Ensure the usage includes COPY_DST for the write operation.
        usage |= wgpu::BufferUsages::COPY_DST;

        let size = std::mem::size_of_val(elements);

        // Upload the data to the buffer using queue.write_buffer.
        // no bytemuck
        let data: &[u8] =
            unsafe { std::slice::from_raw_parts(elements.as_ptr() as *const u8, size) };

        // Create the destination (GPU-only) buffer
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GPU Buffer from create_and_upload_buffer"),
            usage,
            contents: data,
        });
        buffer
    }

    pub fn create_command_encoder_ring(&self) -> Ring<wgpu::CommandEncoder> {
        Ring::new_with(self.config.desired_maximum_frame_latency as usize, |_| {
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command_encoder_ring"),
            })
        })
    }

    pub fn create_image_ring(
        &self,
        ring_size: usize,
        dimension: wgpu::TextureDimension,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        extent: wgpu::Extent3d,
        mip_level_count: u32,
        label: Option<&str>,
    ) -> Ring<Image> {
        (0..ring_size)
            .map(|_| self.create_image(dimension, format, usage, extent, mip_level_count, label))
            .collect()
    }

    pub fn create_image(
        &self,
        dimension: wgpu::TextureDimension,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        extent: wgpu::Extent3d,
        mip_level_count: u32,
        label: Option<&str>,
    ) -> Image {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: extent,
            mip_level_count,
            sample_count: 1,
            dimension,
            format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            // we dont create stencil views
            aspect: if format.has_depth_aspect() {
                wgpu::TextureAspect::DepthOnly
                // wgpu::TextureAspect::All
            } else {
                wgpu::TextureAspect::All
            },
            ..Default::default()
        });
        Image { texture, view }
    }

    pub fn create_shader_module(&self, wgsl_code: &str, label: Option<&str>) -> wgpu::ShaderModule {
        self.device.create_shader_module(ShaderModuleDescriptor {
            label,
            source: ShaderSource::Wgsl(Cow::Borrowed(wgsl_code)),
        })
    }

    pub fn create_raster_pipe(
        &self,
        bind_descriptions: &[BindGroupDescription],
        shader_stages: &[ShaderStage],
        vertex_buffer_layouts: &[VertexBufferLayout],
        primitive_topology: PrimitiveTopology,
        targets: Vec<Option<ColorTargetState>>,
        depth_stencil: Option<DepthStencilState>,
        cull_mode: Option<wgpu::Face>,
        push_constant_description: Option<PushConstantDescription>,
        label: Option<&str>,
    ) -> RasterPipe {
        let frame_count = self.config.desired_maximum_frame_latency as usize;
        let mut pc_buffers = None;

        if let Some(pc_desc) = &push_constant_description {
            // cant be single one cause then we would not be able to write a lot to a buffer
            // in other words, this buffer is array of push constant structs
            let single_pc_size = pc_desc.size.next_multiple_of(256);
            let pc_buffer_size = single_pc_size * pc_desc.max_count;

            pc_buffers = Some(
                (0..frame_count)
                    .map(|_| {
                        self.device.create_buffer(&wgpu::BufferDescriptor {
                            label,
                            size: pc_buffer_size as u64,
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        })
                    })
                    .collect::<Ring<_>>(),
            );
        }

        // Separate dynamic and static bind group descriptions
        let (static_bind_descriptions, mut dynamic_bind_descriptions): (Vec<_>, Vec<_>) =
            bind_descriptions
                .iter()
                .partition(|b| matches!(b.resources, ResourceType::Static(_, _)));

        // remove push constants
        dynamic_bind_descriptions
            .retain(|bind_desc| !matches!(bind_desc.resources, ResourceType::PushConstant));

        let static_bind_group_layout_entries: Vec<_> = static_bind_descriptions
            .iter()
            .map(|b| BindGroupLayoutEntry {
                binding: b.binding,
                visibility: b.visibility,
                ty: match &b.resources {
                    ResourceType::Static(ty, _resources_ring) => *ty,
                    ResourceType::Dynamic(_) | ResourceType::PushConstant => {
                        panic!("static bind groups cannot have dynamic/pc resources")
                    }
                },
                count: None,
            })
            .collect();

        let static_bind_group_layout =
            self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label,
                entries: &static_bind_group_layout_entries,
            });

        // Create static bind groups for each frame
        let mut all_static_bind_groups = Vec::new();

        for frame in 0..frame_count {
            let mut bind_group_entries: Vec<_> = static_bind_descriptions
                .iter()
                .map(|b| BindGroupEntry {
                    binding: b.binding,
                    resource: {
                        match &b.resources {
                            ResourceType::Dynamic(_) | ResourceType::PushConstant => {
                                panic!(
                                    "Dynamic/pc resources should not end up in static descriptions"
                                )
                            }
                            ResourceType::Static(_ty, binding_resources) => {
                                binding_resources[frame].clone()
                            }
                        }
                    },
                })
                .collect();

            let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label,
                layout: &static_bind_group_layout,
                entries: &bind_group_entries,
            });
            all_static_bind_groups.push(bind_group);
        }

        let static_bind_groups = Some(Ring::from_vec(all_static_bind_groups));

        // Create layout for dynamic bind groups
        let dynamic_bind_group_layout_entries: Vec<_> = dynamic_bind_descriptions
            .iter()
            .map(|b| BindGroupLayoutEntry {
                binding: b.binding,
                visibility: b.visibility,
                ty: match &b.resources {
                    ResourceType::Dynamic(ty) => *ty,
                    ResourceType::Static(_, _) | ResourceType::PushConstant => {
                        panic!(
                            "{}: dynamic bind groups cannot have static/pc resources",
                            label.unwrap_or("")
                        )
                    }
                },
                count: None,
            })
            .collect();

        let pc_bind_group_layout = if !push_constant_description.is_none() {
            Some(
                self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("dynamic_bind_group_layout"),
                    entries: &[BindGroupLayoutEntry {
                        binding: 0, // we dont actually care whats specified, we now it in advance
                        visibility: push_constant_description.as_ref().unwrap().stages,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(
                                NonZero::new(
                                    push_constant_description.as_ref().unwrap().size as u64,
                                )
                                .unwrap(),
                            ),
                        },
                        count: None,
                    }],
                }),
            )
        } else {
            None
        };

        let pc_bind_group = if push_constant_description.is_some() {
            Some(
                (0..frame_count)
                    .map(|frame| {
                        self.device.create_bind_group(&BindGroupDescriptor {
                            label,
                            layout: pc_bind_group_layout.as_ref().unwrap(),
                            entries: &[BindGroupEntry {
                                binding: 0,
                                resource: {
                                    BindingResource::Buffer(BufferBinding {
                                        buffer: &pc_buffers.as_ref().unwrap()[frame],
                                        offset: 0, // its dynamic so 0 means we start at the beginning of buffer
                                        size: Some(
                                            NonZero::new(
                                                push_constant_description.as_ref().unwrap().size
                                                    as u64,
                                            )
                                            .unwrap(),
                                        ),
                                    })
                                },
                            }],
                        })
                    })
                    .collect(),
            )
        } else {
            None
        };

        let dynamic_bind_group_layout = if !dynamic_bind_group_layout_entries.is_empty() {
            Some(
                self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("dynamic_bind_group_layout"),
                    entries: &dynamic_bind_group_layout_entries,
                }),
            )
        } else {
            None
        };

        // Define pipeline layout with both static and dynamic bind group layouts
        let mut bind_group_layouts = Vec::new();
        bind_group_layouts.push(&static_bind_group_layout); // static_bind_group_layout is always exists

        if let Some(ref pc_layout) = pc_bind_group_layout {
            bind_group_layouts.push(pc_layout);
        }

        if let Some(ref dynamic_layout) = dynamic_bind_group_layout {
            bind_group_layouts.push(dynamic_layout);
        }

        let pipeline_layout = self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label,
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        let shader_modules: HashMap<ShaderStages, ShaderModule> = shader_stages
            .iter()
            .map(|stage| {
                let module = self.create_shader_module(stage.code, label);
                (stage.stage, module)
            })
            .collect();

        let vertex_shader = shader_modules.get(&ShaderStages::VERTEX);
        let fragment_shader = shader_modules.get(&ShaderStages::FRAGMENT);

        let targets_state: Vec<Option<ColorTargetState>> = targets;

        let primitive = PrimitiveState {
            topology: primitive_topology,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: cull_mode,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        };

        let fragment = fragment_shader.map(|fs| FragmentState {
            module: fs,
            entry_point: Some("main"),
            targets: &targets_state,
            compilation_options: Default::default(),
        });

        let render_pipeline = if let Some(vs) = vertex_shader {
            self.device.create_render_pipeline(&RenderPipelineDescriptor {
                label,
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: vs,
                    entry_point: Some("main"),
                    buffers: vertex_buffer_layouts,
                    compilation_options: Default::default(),
                },
                fragment,
                primitive,
                depth_stencil,
                multisample: MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        } else {
            panic!("Error: Vertex shader not found for pipeline: {:?}", label)
        };

        RasterPipe {
            pipeline: Some(render_pipeline),
            pipeline_layout: Some(pipeline_layout),
            static_bind_groups,
            dynamic_bind_group_layout,
            pc_buffers,
            current_pc_offset: 0,
            pc_size: push_constant_description.map_or(0, |pc| pc.size),
            pc_bind_groups: pc_bind_group,
        }
    }

    pub fn create_compute_pipe(
        &self,
        bind_descriptions: &[BindGroupDescription],
        shader_stage: &ShaderStage,
        push_constant_description: Option<PushConstantDescription>,
        label: Option<&str>,
    ) -> ComputePipe {
        let frame_count = self.config.desired_maximum_frame_latency as usize;
        let mut pc_buffers = None;
        // let mut push_constant_binding_index = None;

        // Separate dynamic and static bind group descriptions
        let (static_bind_descriptions, mut dynamic_bind_descriptions): (Vec<_>, Vec<_>) =
            bind_descriptions
                .iter()
                .partition(|b| matches!(b.resources, ResourceType::Static(_, _)));

        // remove push constants
        dynamic_bind_descriptions
            .retain(|bind_desc| !matches!(bind_desc.resources, ResourceType::PushConstant));

        // Create layout for static bind groups
        let static_bind_group_layout_entries: Vec<_> = static_bind_descriptions
            .iter()
            .map(|b| BindGroupLayoutEntry {
                binding: b.binding,
                visibility: b.visibility,
                ty: match &b.resources {
                    ResourceType::Static(ty, _resources_ring) => *ty,
                    ResourceType::Dynamic(_) | ResourceType::PushConstant => {
                        panic!("static bind groups cannot have dynamic/pc resources")
                    }
                },
                count: None,
            })
            .collect();

        let mut all_bind_group_layout_entries = static_bind_group_layout_entries.clone();
        if let Some(pc_desc) = &push_constant_description {
            let single_pc_size = pc_desc.size;
            let pc_buffer_size = single_pc_size * pc_desc.max_count;

            pc_buffers = Some(
                (0..frame_count)
                    .map(|_| {
                        self.device.create_buffer(&wgpu::BufferDescriptor {
                            label,
                            size: pc_buffer_size as u64,
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        })
                    })
                    .collect::<Ring<_>>(),
            );
        }

        let static_bind_group_layout =
            self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label,
                entries: &all_bind_group_layout_entries,
            });

        // Create static bind groups for each frame
        let mut all_static_bind_groups = Vec::new();

        for frame in 0..frame_count {
            let mut bind_group_entries: Vec<_> = static_bind_descriptions
                .iter()
                .map(|b| BindGroupEntry {
                    binding: b.binding,
                    resource: {
                        match &b.resources {
                            ResourceType::Dynamic(_) | ResourceType::PushConstant => {
                                panic!(
                                    "Dynamic/pc resources should not end up in static descriptions"
                                )
                            }
                            ResourceType::Static(_ty, binding_resources) => {
                                binding_resources[frame].clone()
                            }
                        }
                    },
                })
                .collect();

            let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label,
                layout: &static_bind_group_layout,
                entries: &bind_group_entries,
            });
            all_static_bind_groups.push(bind_group);
        }

        let static_bind_groups = Some(Ring::from_vec(all_static_bind_groups));

        // Create layout for dynamic bind groups
        let dynamic_bind_group_layout_entries: Vec<_> = dynamic_bind_descriptions
            .iter()
            .map(|b| BindGroupLayoutEntry {
                binding: b.binding,
                visibility: b.visibility,
                ty: match &b.resources {
                    ResourceType::Static(_, _) => {
                        panic!("static bind groups cannot have dynamic resources")
                    }
                    ResourceType::Dynamic(ty) => *ty,
                    ResourceType::PushConstant => BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: NonZero::new(
                            push_constant_description.as_ref().unwrap().size as u64,
                        ),
                    },
                },
                count: None,
            })
            .collect();

        let pc_bind_group_layout = if !push_constant_description.is_none() {
            Some(
                self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("dynamic_bind_group_layout"),
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: push_constant_description.as_ref().unwrap().stages,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(
                                NonZero::new(
                                    push_constant_description.as_ref().unwrap().size as u64,
                                )
                                .unwrap(),
                            ),
                        },
                        count: None,
                    }],
                }),
            )
        } else {
            None
        };

        let pc_bind_group = if push_constant_description.is_some() {
            Some(
                (0..frame_count)
                    .map(|frame| {
                        self.device.create_bind_group(&BindGroupDescriptor {
                            label,
                            layout: pc_bind_group_layout.as_ref().unwrap(),
                            entries: &[BindGroupEntry {
                                binding: 0,
                                resource: {
                                    BindingResource::Buffer(BufferBinding {
                                        buffer: &pc_buffers.as_ref().unwrap()[frame],
                                        offset: 0, // its dynamic so 0 means we start at the beginning of buffer
                                        size: Some(
                                            NonZero::new(
                                                push_constant_description.as_ref().unwrap().size
                                                    as u64,
                                            )
                                            .unwrap(),
                                        ),
                                    })
                                },
                            }],
                        })
                    })
                    .collect(),
            )
        } else {
            None
        };

        let dynamic_bind_group_layout = if !dynamic_bind_group_layout_entries.is_empty() {
            Some(
                self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("dynamic_bind_group_layout"),
                    entries: &dynamic_bind_group_layout_entries,
                }),
            )
        } else {
            None
        };

        // Define pipeline layout with both static and dynamic bind group layouts
        let mut bind_group_layouts = Vec::new();
        bind_group_layouts.push(&static_bind_group_layout);

        if let Some(ref pc_layout) = pc_bind_group_layout {
            bind_group_layouts.push(pc_layout);
        }

        if let Some(ref dynamic_layout) = dynamic_bind_group_layout {
            bind_group_layouts.push(dynamic_layout);
        }

        let pipeline_layout = self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label,
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        let compute_shader = self.create_shader_module(shader_stage.code, label);

        let compute_pipeline = self.device.create_compute_pipeline(&ComputePipelineDescriptor {
            label,
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        ComputePipe {
            pipeline: Some(compute_pipeline),
            pipeline_layout: Some(pipeline_layout),
            static_bind_groups,
            pc_buffers,
            pc_size: push_constant_description.map_or(0, |desc| desc.size as u32),
            current_pc_offset: 0,
            pc_bind_groups: pc_bind_group,
        }
    }

    // pub fn bind_raster_pipeline(&self, render_pass: &mut wgpu::RenderPass, pipe: &RasterPipe) {
    //     let current_frame = self.frame_index % self.config.desired_maximum_frame_latency as usize;

    //     render_pass.set_pipeline(pipe.pipeline.as_ref().unwrap());

    //     let mut bind_index = 0;
    //     if let Some(ref static_bind_groups) = pipe.static_bind_groups {
    //         render_pass.set_bind_group(bind_index, static_bind_groups.current(), &[]);
    //         bind_index += 1;
    //     }

    //     if let Some(ref pc_bind_groups) = pipe.pc_bind_groups {
    //         let padded_pc_size = pipe.pc_size.next_multiple_of(256);
    //         render_pass.set_bind_group(
    //             bind_index,
    //             pc_bind_groups.current(),
    //             &[pipe.current_pc_offset * padded_pc_size as u32],
    //         );
    //         bind_index += 1;
    //     }
    // }

    pub fn bind_raster_pipeline<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        pipeline: &'a wgpu::RenderPipeline,
        static_bind_groups: Option<&'a Ring<wgpu::BindGroup>>,
        pc_bind_groups: Option<&'a Ring<wgpu::BindGroup>>,
        pc_buffers: Option<&'a Ring<wgpu::Buffer>>,
        pc_size: u32,
        current_pc_offset: u32,
    ) -> Option<wgpu::QueueWriteBufferView<'a>> {
        render_pass.set_pipeline(pipeline);

        let mut bind_index = 0;
        if let Some(static_bind_groups) = static_bind_groups {
            render_pass.set_bind_group(bind_index, static_bind_groups.current(), &[]);
            bind_index += 1;
        }

        let mut buffer_write_view = None;
        if let Some(pc_bind_groups) = pc_bind_groups {
            let padded_pc_size = pc_size.next_multiple_of(256);
            render_pass.set_bind_group(
                bind_index,
                pc_bind_groups.current(),
                &[current_pc_offset * padded_pc_size],
            );
            bind_index += 1;

            // Move the write_buffer_with call here
            if let Some(pc_buffers) = pc_buffers {
                buffer_write_view = self.queue.write_buffer_with(
                    pc_buffers.current(),
                    0,
                    std::num::NonZero::new(pc_buffers.current().size()).unwrap(),
                );
            }
        }

        buffer_write_view
    }

    pub fn bind_compute_pipeline<'a>(
        &self,
        compute_pass: &mut wgpu::ComputePass<'a>,
        pipe: &ComputePipe,
    ) {
        compute_pass.set_pipeline(&pipe.pipeline.as_ref().unwrap());

        let mut bind_index = 0;
        if let Some(ref static_bind_groups) = pipe.static_bind_groups {
            compute_pass.set_bind_group(bind_index, static_bind_groups.current(), &[]);
            bind_index += 1;
        }

        if let Some(ref pc_bind_groups) = pipe.pc_bind_groups {
            let padded_pc_size = pipe.pc_size.next_multiple_of(256);
            compute_pass.set_bind_group(
                bind_index,
                pc_bind_groups.current(),
                &[pipe.current_pc_offset * padded_pc_size as u32],
            );
            bind_index += 1;
        }
    }

    // Modified to accept individual pipe components and the mutable push constant slice
    pub fn draw_with_params<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'a>,
        pipeline: &'a wgpu::RenderPipeline, // Now passed individually
        pc_bind_groups: Option<&'a Ring<wgpu::BindGroup>>, // Now passed individually
        // pc_size: u32,                       // Now passed individually
        current_pc_offset: &mut u32, // Now mutable reference
        static_bind_groups: Option<&BindGroup>,
        dynamic_bind_group: Option<&BindGroup>,
        push_constants: Option<&[u8]>,
        pc_write_slice: Option<&mut [u8]>, // The mutable slice for writing
        vertices: core::ops::Range<u32>,
        instances: core::ops::Range<u32>,
    ) {
        // The pipeline is already set in bind_raster_pipeline, so no need to set it here
        // render_pass.set_pipeline(pipeline); // Removed

        let mut bind_index = 0;
        if let Some(ref static_bind_groups) = static_bind_groups {
            bind_index += 1;
        }

        if let Some(pc_data) = push_constants {
            let pc_size = pc_data.len();
            let padded_pc_size = pc_size.next_multiple_of(256) as u32;
            render_pass.set_bind_group(
                bind_index,
                pc_bind_groups.as_ref().unwrap().current(), // Access directly from the passed Ring
                &[*current_pc_offset * padded_pc_size],
            );
            bind_index += 1;

            // Write the push constants data to the buffer slice
            if let Some(mut pc_write_slice) = pc_write_slice {
                let offset = (*current_pc_offset * padded_pc_size) as usize;
                let len = pc_data.len();
                pc_write_slice[offset..offset + len].copy_from_slice(pc_data);
            }

            *current_pc_offset += 1; // Increment the mutable reference
        }

        if let Some(bind_group) = dynamic_bind_group {
            render_pass.set_bind_group(bind_index, bind_group, &[]);
            bind_index += 1;
        }

        render_pass.draw(vertices, instances);
    }

    pub fn draw_indexed_with_params<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'a>,
        pipeline: &'a wgpu::RenderPipeline, // Now passed individually
        pc_bind_groups: Option<&'a Ring<wgpu::BindGroup>>, // Now passed individually
        pc_size: u32,                       // Now passed individually
        current_pc_offset: &mut u32,        // Now mutable reference
        static_bind_groups: Option<&BindGroup>,
        dynamic_bind_group: Option<&BindGroup>,
        push_constants: Option<&[u8]>,
        pc_write_slice: Option<&mut [u8]>, // The mutable slice for writing
        indices: core::ops::Range<u32>,
        base_vertex: i32,
        instances: core::ops::Range<u32>,
    ) {
        let mut bind_index = 0;
        if let Some(ref static_bind_groups) = static_bind_groups {
            bind_index += 1;
        }

        if let Some(pc_data) = push_constants {
            assert!(pc_size == pc_data.len() as u32);
            let padded_pc_size = pc_size.next_multiple_of(256);
            render_pass.set_bind_group(
                bind_index,
                pc_bind_groups.as_ref().unwrap().current(), // Access directly from the passed Ring
                &[*current_pc_offset * padded_pc_size],
            );
            bind_index += 1;

            // Write the push constants data to the buffer slice
            if let Some(mut pc_write_slice) = pc_write_slice {
                let offset = (*current_pc_offset * padded_pc_size) as usize;
                let len = pc_data.len();
                pc_write_slice[offset..offset + len].copy_from_slice(pc_data);
            }

            *current_pc_offset += 1; // Increment the mutable reference
        }

        if let Some(bind_group) = dynamic_bind_group {
            render_pass.set_bind_group(bind_index, bind_group, &[]);
            bind_index += 1;
        }

        render_pass.draw_indexed(indices, base_vertex, instances);
    }

    pub fn dispatch_with_params<'a>(
        &self,
        compute_pass: &mut wgpu::ComputePass<'a>,
        pipe: &mut ComputePipe,
        dynamic_bind_group: Option<&BindGroup>,
        push_constants: Option<&[u8]>,
        workgroup_count_x: u32,
        workgroup_count_y: u32,
        workgroup_count_z: u32,
    ) {
        let mut bind_index = 0;
        if let Some(ref static_bind_groups) = pipe.static_bind_groups {
            // compute_pass.set_bind_group(bind_index, static_bind_groups.current(), &[]);
            bind_index += 1;
        }

        if let Some(pc_data) = push_constants {
            let padded_pc_size = pc_data.len().next_multiple_of(256);
            compute_pass.set_bind_group(
                bind_index,
                pipe.pc_bind_groups.as_ref().unwrap().current(),
                &[pipe.current_pc_offset * padded_pc_size as u32],
            );
            bind_index += 1;
        }

        if let Some(bind_group) = dynamic_bind_group {
            compute_pass.set_bind_group(bind_index, bind_group, &[]);
            bind_index += 1;
        }

        // emulate push constants with dynamically offset'ed separate buffer
        if let Some(pc_data) = push_constants {
            let padded_pc_size = pc_data.len().next_multiple_of(256);
            let pc_buffers = pipe.pc_buffers.as_ref().unwrap();
            let buffer = pc_buffers.current();
            self.queue.write_buffer(
                buffer,
                (pipe.current_pc_offset * padded_pc_size as u32) as wgpu::BufferAddress,
                pc_data,
            );

            pipe.current_pc_offset += 1;
        }

        compute_pass.dispatch_workgroups(workgroup_count_x, workgroup_count_y, workgroup_count_z);
    }

    pub fn write_buffer_with() {}
}

/*
order goes like this:
static bind group
push_constants
dynamic bind group
*/
