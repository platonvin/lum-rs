//! This module contains core structures for managing the wgpu state and rendering
//! Essentially, it's `lumal` for wgpu.
//! Provides abstractions over raw wgpu types to simplify rendering operations of Lum

use containers::Ring;
use std::borrow::Cow;
use wgpu::DepthStencilState;
use wgpu::{
    util::DeviceExt, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, ColorTargetState, ComputePipelineDescriptor, Features, FragmentState,
    FrontFace, MultisampleState, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource,
    VertexBufferLayout, VertexState,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

/// Manages the core wgpu state and resources required for rendering.
/// Represents primary interface for Lum to interact with wgpu.
/// Read wgpu documentation for detailed information on the underlying types.
pub struct Wal<'window> {
    /// The window surface we render to.
    pub surface: wgpu::Surface<'window>,
    /// The chosen graphics adapter (GPU).
    pub adapter: wgpu::Adapter,
    /// The logical device for interacting with the adapter.
    pub device: wgpu::Device,
    /// The command queue for submitting commands to the GPU.
    /// We use a single one because wgpu sync makes multiple queues useless.
    pub queue: wgpu::Queue,
    /// The configuration of the surface.
    pub config: wgpu::SurfaceConfiguration,
    /// Counter for the total number of frames rendered.
    ///
    /// This index is not circular and increments with each completed frame.
    /// Used as RNG source
    pub frame_index: usize,
    /// Texture format we decided to use (supported formats are different for different devices)
    pub swapchain_format: wgpu::TextureFormat,
}

/// Bundled resources for Compute Pipeline. Exists primarily for convenience.
#[derive(Debug, Default)]
pub struct ComputePipe {
    /// Underlying wgpu compute pipeline object.
    pub line: Option<wgpu::ComputePipeline>,
    /// Layout of the compute pipeline.
    pub layout: Option<wgpu::PipelineLayout>,
    /// Bind groups that are used for every dispatch call with this pipeline (if any).
    /// These typically hold `global` resources like UBO that are constant across different
    /// dispatches using this pipeline.
    pub static_bind_groups: Option<Ring<wgpu::BindGroup>>,
    /// Layout (not bindings themselves) for all the bind groups that are not static
    /// Yes, they have to be bundled in a single bind group (not like its fundamental property of wgpu, but such limitation really fits Lum)
    /// Usage example:
    /// you would create Pipe with dynamic bind group layout,
    /// then use it for per-model textures bind groups, stored in model (struct Model {dyn_bg: wgpu::BindGroup})
    pub dynamic_bind_group_layout: Option<wgpu::BindGroupLayout>,
}

/// Bundled resources for Rasterization Pipeline. Exists primarily for convenience.
#[derive(Debug, Default)]
pub struct RasterPipe {
    /// Underlying wgpu render pipeline object.
    pub line: Option<wgpu::RenderPipeline>,
    /// Layout of the render pipeline.
    pub layout: Option<wgpu::PipelineLayout>,
    /// Bind group that is used for every dispatch call with this pipeline (if any).
    /// This typically holds `global` resources like UBO that are constant across different
    /// dispatches using this pipeline.
    /// There is multiple for FIF (and thus they are in Ring)
    pub static_bind_groups: Option<Ring<wgpu::BindGroup>>,
    /// Layout (not bindings themselves) for all the bind groups that are not static
    /// Yes, they have to be bundled in a single bind group (not like its fundamental property of wgpu, but such limitation really fits Lum)
    pub dynamic_bind_group_layout: Option<wgpu::BindGroupLayout>,
}

/// Bundled descriptions and resource bindings for a static bind group.
///
/// Static bind groups are those whose resources are bundled with the Pipe (e.g. global UBO with per-frame data).
/// They are typically bound once with `Wal::bind_raster_pipe` / `Wal::bind_compute_pipe` functions.
///
/// They take the first set available (which is 0): `layout(set = 0, binding = ...) ... `
#[derive(Clone)]
pub struct StaticBindGroupDescription<'a> {
    /// Binding index of the resource within the bind group in the shader.
    ///
    /// For example, `binding = 1` would correspond to `layout(set = 0, binding = 1) ...`
    /// in shader.
    pub binding: u32,
    /// Shader stages that can access this binding.
    pub visibility: wgpu::ShaderStages,
    /// Type of the binding resource (e.g., buffer, sampler, texture).
    pub binding_type: wgpu::BindingType,
    /// Actual binding resources (there is no need to store CPU-side resource handles in here, BindingReource is a better fit)
    pub resources: Ring<wgpu::BindingResource<'a>>,
}

/// Bundled descriptions for a dynamic bind group layout.
///
/// Dynamic bind groups are those whose resources change frequently, often on a
/// per-draw/dispatch basis (e.g., per-mesh textures). This struct only
/// describes the *layout* of such a bind group; the actual resources are provided
/// when binding the group before a draw or dispatch call.
///
/// In shader code, dynamic bind groups are placed in a set *after* static bind groups (if any).
/// For example, if there's static bind group presented `set = 0`,
/// the dynamic bind group would be in `set = 1`.
#[derive(Clone)]
pub struct DynamicBindGroupDescription {
    /// Binding index of the resource within the bind group in the shader.
    ///
    /// For example, `binding = 2` would correspond to `layout(set = ..., binding = 2) ...`
    /// in the shader code. The specific set index depends on the pipeline's layout
    /// and the presence of static bind group.
    pub binding: u32,
    /// Shader stages that can access this binding.
    pub visibility: wgpu::ShaderStages,
    /// Type of the binding resource (e.g., buffer, sampler, texture).
    pub binding_type: wgpu::BindingType,
    // resources are not stored here as they are provided dynamically per draw/dispatch.
}

/// Describes a single shader stage with its `code`.
#[derive(Clone, Debug)]
pub struct ShaderStageSource {
    /// Single shader stage (Vertex/Fragment/Compute...) the source code belongs to
    pub stage: wgpu::ShaderStages,
    /// Source code of the shader.
    pub code: &'static str,
}

/// Bundled texture and its view.
pub struct Image {
    /// Underlying wgpu texture object.
    pub texture: wgpu::Texture,
    /// Default texture view, covering the entire texture with all aspects (except stencil).
    pub view: wgpu::TextureView,
}

impl<'window> Wal<'window> {
    pub async fn new(window: std::sync::Arc<Window>, size: PhysicalSize<u32>) -> Self {
        // if you dont understand what is happening here i recommend looking into WGPU guide
        // essntially we are just setting up a few of GPU/Driver state objects in a way we need to proceed

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await // yes its async cause web
            .expect("No suitable GPU adapters found");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: Features::DEPTH32FLOAT_STENCIL8 | Features::FLOAT32_FILTERABLE,
                ..Default::default()
            })
            .await
            .unwrap();

        let capable_formats = surface.get_capabilities(&adapter).formats;
        let swapchain_format = capable_formats
            .iter()
            .find(|format| !(*format).is_srgb())
            .expect("no Non-SRGB swapchain format found"); // Lum is build around no srgb thing
        dbg!(swapchain_format);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width: size.width,
            height: size.height,
            #[cfg(not(target_arch = "wasm32"))]
            present_mode: wgpu::PresentMode::Mailbox,
            #[cfg(target_arch = "wasm32")]
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2, // two is enough for FIF but not too many for delays
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        Self {
            // instance, // we dont really have to store it
            surface,
            adapter,
            device,
            queue,
            config,
            frame_index: 0,
            swapchain_format: *swapchain_format,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Creates a single wgpu::Buffer
    pub fn create_buffer(
        &self,
        usage: wgpu::BufferUsages,
        size: usize,
        label: Option<&str>,
    ) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size: size as u64,
            usage,
            mapped_at_creation: false,
        })
    }

    // Creates a Ring of wgpu::Buffers
    pub fn create_buffers(
        &self,
        ring_size: usize,
        usage: wgpu::BufferUsages,
        buffer_size: usize,
        label: Option<&str>,
    ) -> Ring<wgpu::Buffer> {
        (0..ring_size).map(|_| self.create_buffer(usage, buffer_size, label)).collect()
    }

    /// Creates a GPU buffer and uploads provided data into it
    #[inline]
    pub fn create_and_upload_buffer<T>(
        &self,
        elements: &[T],
        mut usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        // Ensure the usage includes COPY_DST for the write operation.
        usage |= wgpu::BufferUsages::COPY_DST;

        let size = std::mem::size_of_val(elements);

        // no bytemuck
        let data: &[u8] =
            unsafe { std::slice::from_raw_parts(elements.as_ptr() as *const u8, size) };

        // This is wgpu convenience function which handles copy to GPU memory automatically
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GPU Buffer from create_and_upload_buffer"),
            usage,
            contents: data,
        })
    }

    /// creates Ring of Images (textures & views)
    pub fn create_images(
        &self,
        ring_size: usize,
        dimension: wgpu::TextureDimension,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        extent: wgpu::Extent3d,
        label: Option<&str>,
    ) -> Ring<Image> {
        (0..ring_size)
            .map(|_| self.create_image(dimension, format, usage, extent, label))
            .collect()
    }

    /// creates Single of Image (texture & view)
    pub fn create_image(
        &self,
        dimension: wgpu::TextureDimension,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        extent: wgpu::Extent3d,
        label: Option<&str>,
    ) -> Image {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: extent,
            mip_level_count: 1, // we dont need mipmaps in Lum
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

    fn create_shader_module(&self, wgsl_code: &str, label: Option<&str>) -> wgpu::ShaderModule {
        self.device.create_shader_module(ShaderModuleDescriptor {
            label,
            source: ShaderSource::Wgsl(Cow::Borrowed(wgsl_code)),
        })
    }

    pub fn create_raster_pipe(
        &self,
        // this could be single array, separated at runtime. But it never will be
        static_bind_descriptions: &[StaticBindGroupDescription],
        dynamic_bind_descriptions: &[DynamicBindGroupDescription],
        // Lum does not need geometry/ mesh /rtx shaders
        vertex_code: &str, // we could potentially make this Option too and give it meaning of `None = fullscreen triangle`
        fragment_code: Option<&str>, // None means.. No fragment shader. Its perfectly legal
        vertex_buffer_layouts: &[VertexBufferLayout],
        primitive_topology: PrimitiveTopology,
        targets: Vec<Option<ColorTargetState>>,
        depth_stencil: Option<DepthStencilState>,
        cull_mode: Option<wgpu::Face>,
        label: Option<&str>,
    ) -> RasterPipe {
        let frame_count = self.config.desired_maximum_frame_latency as usize;

        // repacking memory for wgpu
        let static_bind_group_layout_entries: Vec<_> = static_bind_descriptions
            .iter()
            .map(|bind_desc| BindGroupLayoutEntry {
                binding: bind_desc.binding,
                visibility: bind_desc.visibility,
                ty: bind_desc.binding_type,
                count: None,
            })
            .collect();

        let static_bind_group_layout =
            self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label,
                entries: &static_bind_group_layout_entries,
            });

        // Create static bind group for each frame in flight
        let static_bind_groups = if static_bind_descriptions.is_empty() {
            None
        } else {
            Some(Ring::from_vec(
                (0..frame_count)
                    .map(|frame| {
                        let bind_group_entries: Vec<_> = static_bind_descriptions
                            .iter()
                            .map(|b| BindGroupEntry {
                                binding: b.binding,
                                resource: b.resources[frame].clone(),
                            })
                            .collect();

                        self.device.create_bind_group(&BindGroupDescriptor {
                            label,
                            layout: &static_bind_group_layout,
                            entries: &bind_group_entries,
                        })
                    })
                    .collect(),
            ))
        };

        // Create layout for dynamic bind groups
        let dynamic_bind_group_layout = (!dynamic_bind_descriptions.is_empty()).then(|| {
            let entries: Vec<_> = dynamic_bind_descriptions
                .iter()
                .map(|b| BindGroupLayoutEntry {
                    binding: b.binding,
                    visibility: b.visibility,
                    ty: b.binding_type,
                    count: None,
                })
                .collect();

            self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Dynamic Bind Group Layout"),
                entries: &entries,
            })
        });

        // Define pipeline layout with both static and dynamic bind group layouts
        // Very smart choice - no explicit group indices, they are implicit and equal to position of layout in passed array
        // actually, this IS the way, but i dont see it fitting wgpu design philosophy
        let bind_group_layouts: Vec<&wgpu::BindGroupLayout> =
            std::iter::once(&static_bind_group_layout)
                .chain(dynamic_bind_group_layout.as_ref())
                .collect(); // TODO: check asm of this functional style

        let pipeline_layout = self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label,
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        let vertex_shader = self.create_shader_module(vertex_code, Some("Vertex shader module"));
        let fragment_shader = fragment_code
            .map(|fragment| self.create_shader_module(fragment, Some("Fragment shader module")));

        let primitive = PrimitiveState {
            topology: primitive_topology,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        };

        let fragment = fragment_shader.as_ref().map(|fs| FragmentState {
            module: fs,
            entry_point: Some("main"),
            targets: &targets,
            compilation_options: Default::default(),
        });

        let render_pipeline = {
            self.device.create_render_pipeline(&RenderPipelineDescriptor {
                label,
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &vertex_shader,
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
        };

        RasterPipe {
            line: Some(render_pipeline),
            layout: Some(pipeline_layout),
            static_bind_groups,
            dynamic_bind_group_layout,
        }
    }

    pub fn create_compute_pipe(
        &self,
        static_bind_descriptions: &[StaticBindGroupDescription],
        dynamic_bind_descriptions: &[DynamicBindGroupDescription],
        shader: &ShaderStageSource,
        label: Option<&str>,
    ) -> ComputePipe {
        let frame_count = self.config.desired_maximum_frame_latency as usize;

        // Create layout for static bind groups
        let static_bind_group_layout_entries: Vec<_> = static_bind_descriptions
            .iter()
            .map(|b| BindGroupLayoutEntry {
                binding: b.binding,
                visibility: b.visibility,
                ty: b.binding_type,
                count: None,
            })
            .collect();

        let all_bind_group_layout_entries = static_bind_group_layout_entries.clone();

        let static_bind_group_layout =
            self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label,
                entries: &all_bind_group_layout_entries,
            });

        let static_bind_groups: Vec<_> = (0..frame_count)
            .map(|frame| {
                let bind_group_entries: Vec<_> = static_bind_descriptions
                    .iter()
                    .map(|b| BindGroupEntry {
                        binding: b.binding,
                        resource: b.resources[frame].clone(),
                    })
                    .collect();

                self.device.create_bind_group(&BindGroupDescriptor {
                    label,
                    layout: &static_bind_group_layout,
                    entries: &bind_group_entries,
                })
            })
            .collect();

        // Create layout for dynamic bind groups
        let dynamic_bind_group_layout_entries: Vec<_> = dynamic_bind_descriptions
            .iter()
            .map(|b| BindGroupLayoutEntry {
                binding: b.binding,
                visibility: b.visibility,
                ty: b.binding_type,
                count: None,
            })
            .collect();

        let dynamic_bind_group_layout =
            (!dynamic_bind_group_layout_entries.is_empty()).then(|| {
                self.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("dynamic_bind_group_layout"),
                    entries: &dynamic_bind_group_layout_entries,
                })
            });

        // Define pipeline layout with both static and dynamic bind group layouts
        let bind_group_layouts: Vec<&wgpu::BindGroupLayout> =
            std::iter::once(&static_bind_group_layout)
                .chain(dynamic_bind_group_layout.as_ref())
                .collect(); // TODO: check asm of this functional style

        let pipeline_layout = self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label,
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        let compute_shader = self.create_shader_module(shader.code, label);

        let compute_pipeline = self.device.create_compute_pipeline(&ComputePipelineDescriptor {
            label,
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        ComputePipe {
            line: Some(compute_pipeline),
            layout: Some(pipeline_layout),
            static_bind_groups: Some(Ring::from_vec(static_bind_groups)),
            dynamic_bind_group_layout,
        }
    }

    pub fn bind_raster_pipeline<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        pipeline: &'a wgpu::RenderPipeline,
        static_bind_groups: Option<&'a Ring<wgpu::BindGroup>>,
    ) {
        render_pass.set_pipeline(pipeline);

        let mut bind_index = 0;
        if let Some(static_bind_groups) = static_bind_groups {
            render_pass.set_bind_group(bind_index, static_bind_groups.current(), &[]);
            #[allow(unused)]
            bind_index += 1;
        }
    }

    pub fn bind_compute_pipeline<'a>(
        &self,
        compute_pass: &mut wgpu::ComputePass<'a>,
        pipe: &ComputePipe,
    ) {
        compute_pass.set_pipeline(pipe.line.as_ref().unwrap());

        let mut bind_index = 0;
        if let Some(ref static_bind_groups) = pipe.static_bind_groups {
            compute_pass.set_bind_group(bind_index, static_bind_groups.current(), &[]);
            #[allow(unused)]
            bind_index += 1;
        }
    }

    // Modified to accept individual pipe components and the mutable push constant slice
    pub fn draw_with_params<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'a>,
        static_bind_groups: Option<&BindGroup>,
        dynamic_bind_group: Option<&BindGroup>,
        vertices: core::ops::Range<u32>,
        instances: core::ops::Range<u32>,
    ) {
        let mut bind_index = 0;
        if let Some(_static_bind_groups) = static_bind_groups {
            // already bound
            bind_index += 1;
        }

        if let Some(bind_group) = dynamic_bind_group {
            render_pass.set_bind_group(bind_index, bind_group, &[]);
            #[allow(unused)]
            bind_index += 1;
        }

        render_pass.draw(vertices, instances);
    }

    pub fn draw_indexed_with_params<'a>(
        &self,
        render_pass: &mut wgpu::RenderPass<'a>,
        static_bind_groups: Option<&BindGroup>,
        dynamic_bind_group: Option<&BindGroup>,
        indices: core::ops::Range<u32>,
        base_vertex: i32,
        instances: core::ops::Range<u32>,
    ) {
        let mut bind_index = 0;
        if let Some(_static_bind_groups) = static_bind_groups {
            // already bound
            bind_index += 1;
        }

        if let Some(bind_group) = dynamic_bind_group {
            render_pass.set_bind_group(bind_index, bind_group, &[]);
            #[allow(unused)]
            bind_index += 1;
        }

        render_pass.draw_indexed(indices, base_vertex, instances);
    }

    pub fn dispatch_with_params<'a>(
        &self,
        compute_pass: &mut wgpu::ComputePass<'a>,
        pipe: &mut ComputePipe,
        dynamic_bind_group: Option<&BindGroup>,
        workgroup_count_x: u32,
        workgroup_count_y: u32,
        workgroup_count_z: u32,
    ) {
        let mut bind_index = 0;
        if let Some(ref _static_bind_groups) = pipe.static_bind_groups {
            // already bound
            bind_index += 1;
        }

        if let Some(bind_group) = dynamic_bind_group {
            compute_pass.set_bind_group(bind_index, bind_group, &[]);
            #[allow(unused)]
            bind_index += 1;
        }

        compute_pass.dispatch_workgroups(workgroup_count_x, workgroup_count_y, workgroup_count_z);
    }
}
