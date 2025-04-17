use std::{borrow::Cow, collections::HashMap, num::NonZeroU64};
use wgpu::{
    util::{DeviceExt, StagingBelt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BufferBindingType, BufferUsages,
    ColorTargetState, ComputePipelineDescriptor, Features, FragmentState, FrontFace,
    MultisampleState, PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, PushConstantRange, RenderPipelineDescriptor, ShaderModule, ShaderSource,
    VertexBufferLayout, VertexState,
};
use wgpu::{BindGroupLayout, ShaderModuleDescriptor};
use wgpu::{BindGroupLayoutDescriptor, StencilState};
use wgpu::{BindGroupLayoutEntry, TextureFormat};

use lumal::ring::Ring;

use wgpu::{
    BindingResource, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
    CompareFunction, DepthStencilState, ShaderStages,
};
use winit::window::Window;

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
    pub bind_groups: Option<Ring<wgpu::BindGroup>>,
    pub pc_buffers: Option<Ring<wgpu::Buffer>>,
    pub pc_belts: Option<wgpu::util::StagingBelt>,
    pub pc_size: u32,
    // pub push_constant_binding_index: Option<u32>,
    // current push constants offset
    pub current_pc_offset: u32,
}
#[derive(Debug, Default)]
pub struct RasterPipe {
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub pipeline_layout: Option<wgpu::PipelineLayout>,
    /// Instead of a ring of descriptor sets, we maintain one or more bind groups.
    pub bind_groups: Option<Ring<wgpu::BindGroup>>,
    // we need to store them. Otherwise, they are dropped
    pub pc_buffers: Option<Ring<wgpu::Buffer>>,
    pub pc_belts: Option<wgpu::util::StagingBelt>,
    // pub pc_desc: Option<PushConstantDescription>,
    pub pc_size: Option<NonZeroU64>,
    // why would we need this?
    // pub push_constant_binding_index: Option<u32>,
    pub current_pc_offset: u32,
}

pub struct Image {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

#[derive(Clone, Debug)]
pub struct AttrFormOffs {
    pub format: wgpu::VertexFormat,
    pub binding: u32,
    pub offset: usize,
}

#[derive(Debug)]
pub struct DescriptorInfo {
    // Define the structure of DescriptorInfo based on your needs
    pub size: wgpu::BufferSize,
    pub usage: wgpu::BufferUsages,
    // Add other relevant information
}

#[derive(Clone)]
pub struct BindGroupDescription<'a> {
    /// Binding index. Must match shader index and be unique inside a BindGroupLayout. A binding
    /// of index 1, would be described as `layout(set = 0, binding = 1) uniform` in shaders.
    pub binding: u32,
    /// Which shader stages can see this binding.
    pub visibility: ShaderStages,
    /// The type of the binding
    pub ty: BindingType,
    /// If this value is Some, indicates this entry is an array. Array size must be 1 or greater.
    ///
    /// If this value is Some and `ty` is `BindingType::Texture`, [`Features::TEXTURE_BINDING_ARRAY`] must be supported.
    ///
    /// If this value is Some and `ty` is any other variant, bind group creation will fail.
    pub count: Option<std::num::NonZeroU32>,
    /// so, what function gets is array of rings, and they get converted to ring of arrays
    pub resources: Ring<BindingResource<'a>>,
}

#[derive(Clone, Debug)]
pub struct PushConstantDescription {
    pub size: NonZeroU64,
    pub max_count: u64,
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

impl<'window> Wal<'window> {
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();
        // 1) Create instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
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
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: Features::TEXTURE_FORMAT_16BIT_NORM
                        | Features::DEPTH32FLOAT_STENCIL8,
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        // 5) Configure the swapchain (surface)
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_capabilities(&adapter).formats[0], // TODO: pick proper format
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
        usage: wgpu::BufferUsages,
        size: usize,
        host_visible: bool,
    ) -> wgpu::Buffer {
        // let usage_flags = if host_visible {
        //     usage | wgpu::BufferUsages::MAP_WRITE
        // } else {
        //     usage
        // };

        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Buffer"),
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
    ) -> Ring<wgpu::Buffer> {
        (0..ring_size)
            .map(|_| self.create_buffer(usage, buffer_size, host_visible))
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
    ) -> Ring<Image> {
        (0..ring_size)
            .map(|_| {
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Image Ring Texture"),
                    size: extent,
                    mip_level_count,
                    sample_count: 1,
                    dimension,
                    format,
                    usage,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                Image { texture, view }
            })
            .collect()
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
        let mut push_constant_binding_index = None;

        if let Some(pc_desc) = &push_constant_description {
            // cant be single one cause then we would not be able to write a lot to a buffer
            // in other words, this buffer is array of push constant structs
            let single_pc_size = pc_desc.size.get();
            let pc_buffer_size = single_pc_size * pc_desc.max_count;

            pc_buffers = Some(
                (0..frame_count)
                    .map(|_| {
                        self.device.create_buffer(&wgpu::BufferDescriptor {
                            label,
                            size: pc_buffer_size,
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        })
                    })
                    .collect::<Ring<_>>(),
            );

            push_constant_binding_index =
                Some(bind_descriptions.iter().map(|b| b.binding).max().map_or(0, |max| max + 1));
        }

        let mut bind_group_layout_entries: Vec<_> = bind_descriptions
            .iter()
            .map(|b| BindGroupLayoutEntry {
                binding: b.binding,
                visibility: b.visibility,
                ty: b.ty.clone(),
                count: b.count,
            })
            .collect();

        if let Some(pc_desc) = &push_constant_description {
            bind_group_layout_entries.push(BindGroupLayoutEntry {
                binding: push_constant_binding_index.unwrap(),
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true, // cause we use offset to emulate single push constant struct buffer bound while its actually an array
                    min_binding_size: Some(pc_desc.size), // always the same, we dont bind more then one push constant struct at some offset
                },
                count: None,
            });
        }

        let bind_group_layout = self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label,
            entries: &bind_group_layout_entries,
        });

        let mut all_bind_groups = Vec::new();

        for frame in 0..frame_count {
            let mut bind_group_entries: Vec<_> = bind_descriptions
                .iter()
                .map(|b| BindGroupEntry {
                    binding: b.binding,
                    resource: b.resources[frame].clone(),
                })
                .collect();

            if let Some(ref buffers_ring) = pc_buffers {
                bind_group_entries.push(BindGroupEntry {
                    binding: push_constant_binding_index.unwrap(),
                    resource: BindingResource::Buffer(
                        buffers_ring[frame].as_entire_buffer_binding(),
                    ),
                });
            }

            let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label,
                layout: &bind_group_layout,
                entries: &bind_group_entries,
            });
            all_bind_groups.push(bind_group);
        }

        let bind_groups = Some(Ring::from_vec(all_bind_groups));

        let pipeline_layout = self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label,
            bind_group_layouts: &[&bind_group_layout],
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
            bind_groups,
            pc_buffers,
            current_pc_offset: 0,
            pc_size: push_constant_description.map(|pc_desc| pc_desc.size),
        }
    }

    pub fn create_compute_pipe(
        &self,
        bind_descriptions: &[BindGroupDescription],
        shader_stage: &ShaderStage,
        label: Option<&str>,
        push_constant_description: Option<PushConstantDescription>,
    ) -> ComputePipe {
        let frame_count = self.config.desired_maximum_frame_latency as usize;
        let mut pc_buffers = None;
        let mut push_constant_binding_index = None;

        let mut bind_group_layout_entries: Vec<_> = bind_descriptions
            .iter()
            .map(|b| BindGroupLayoutEntry {
                binding: b.binding,
                visibility: b.visibility,
                ty: b.ty.clone(),
                count: b.count,
            })
            .collect();

        if let Some(pc_desc) = &push_constant_description {
            let single_pc_size = pc_desc.size.get();
            let pc_buffer_size = single_pc_size * pc_desc.max_count;

            pc_buffers = Some(
                (0..frame_count)
                    .map(|_| {
                        self.device.create_buffer(&wgpu::BufferDescriptor {
                            label,
                            size: pc_buffer_size,
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        })
                    })
                    .collect::<Ring<_>>(),
            );

            push_constant_binding_index =
                Some(bind_descriptions.iter().map(|b| b.binding).max().map_or(0, |max| max + 1));

            bind_group_layout_entries.push(BindGroupLayoutEntry {
                binding: push_constant_binding_index.unwrap(),
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(pc_desc.size),
                },
                count: None,
            });
        }

        let bind_group_layout = self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label,
            entries: &bind_group_layout_entries,
        });

        let mut all_bind_groups = Vec::new();

        for frame in 0..frame_count {
            let mut bind_group_entries: Vec<_> = bind_descriptions
                .iter()
                .map(|b| BindGroupEntry {
                    binding: b.binding,
                    resource: b.resources[frame].clone(),
                })
                .collect();

            if let Some(ref buffers_ring) = pc_buffers {
                bind_group_entries.push(BindGroupEntry {
                    binding: push_constant_binding_index.unwrap(),
                    resource: BindingResource::Buffer(
                        buffers_ring[frame].as_entire_buffer_binding(),
                    ),
                });
            }

            let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label,
                layout: &bind_group_layout,
                entries: &bind_group_entries,
            });
            all_bind_groups.push(bind_group);
        }

        let bind_groups = Some(Ring::from_vec(all_bind_groups));

        let pipeline_layout = self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label,
            bind_group_layouts: &[&bind_group_layout],
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
            bind_groups,
            pc_buffers,
            pc_size: push_constant_description.map_or(0, |desc| desc.size.get() as u32),
            current_pc_offset: 0,
        }
    }
}
