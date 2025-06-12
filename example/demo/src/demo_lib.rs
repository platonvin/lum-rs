// #![allow(dead_code)]
// #![allow(unused_variables)]
// #![feature(inherent_associated_types)]

// use assets::{BlockAsset, ModelAsset};
// use lum::{
//     fBLOCK_SIZE, for_zyx,
//     render_interface::{FoliageDescriptionBuilder, FoliageDescriptionCreate, RendererInterface},
//     types::{
//         quat, u8vec3, uvec3, vec3, MeshBlock, MeshFoliage, MeshLiquid, MeshModel, MeshTransform,
//         MeshVolumetric,
//     },
//     Settings,
// };
// use winit::{
//     application::ApplicationHandler,
//     event::{DeviceEvent, DeviceId, WindowEvent},
//     event_loop::{ActiveEventLoop, EventLoop},
//     window::{Window, WindowId},
// };

// // i hardcode it but you probably should use some sort of "Asset library" - hashmap (array) of YourEntityTypeEnum -> LumMeshModel
// struct AllMeshes {
//     tank_body: MeshModel,
//     tank_head: MeshModel,
//     tank_rf_leg: MeshModel,
//     tank_lb_leg: MeshModel,
//     tank_lf_leg: MeshModel,
//     tank_rb_leg: MeshModel,
//     water: MeshLiquid,
//     grass: MeshFoliage,
//     smoke: MeshVolumetric,
// }
// #[derive(Default)]
// struct AllTransforms {
//     tank_body: MeshTransform,
//     tank_head: MeshTransform,
//     tank_rf_leg: MeshTransform,
//     tank_lb_leg: MeshTransform,
//     tank_lf_leg: MeshTransform,
//     tank_rb_leg: MeshTransform,
// }

// impl AllMeshes {
//     fn new<T: RendererInterface>(lum: &mut T, grass: MeshFoliage) -> Self {
//         Self {
//             tank_body: lum.load_model(assets::get_model(ModelAsset::TankBody)),
//             tank_head: lum.load_model(assets::get_model(ModelAsset::TankHead)),
//             tank_rf_leg: lum.load_model(assets::get_model(ModelAsset::TankRfLbLeg)),
//             tank_lb_leg: lum.load_model(assets::get_model(ModelAsset::TankRfLbLeg)),
//             tank_lf_leg: lum.load_model(assets::get_model(ModelAsset::TankLfRbLeg)),
//             tank_rb_leg: lum.load_model(assets::get_model(ModelAsset::TankLfRbLeg)),
//             water: lum.load_liquid(69, 42),
//             grass,
//             smoke: lum.load_volumetric(1.0, 0.5, u8vec3::zero()),
//         }
//     }

//     fn unload<T: RendererInterface>(self, lum: &mut T) {
//         lum.unload_model(self.tank_body);
//         lum.unload_model(self.tank_head);
//         lum.unload_model(self.tank_rf_leg);
//         lum.unload_model(self.tank_lb_leg);
//         lum.unload_model(self.tank_lf_leg);
//         lum.unload_model(self.tank_rb_leg);
//         lum.unload_liquid(self.water);
//         lum.unload_foliage(self.grass);
//         lum.unload_volumetric(self.smoke);
//     }
// }

// struct DemoState<Renderer: RendererInterface> {
//     // window: &'renderer Window,
//     lum: Renderer,
//     meshes: AllMeshes,
//     transforms: AllTransforms,
//     about_to_close: bool,
// }

// impl<'renderer, Renderer: RendererInterface> DemoState<Renderer> {
//     type FoliageDescription = Renderer::FoliageDescription;

//     fn new(window: Window, event_loop: &EventLoop<()>) -> Self {
//         let settings = Settings {
//             static_block_palette_size: 15,
//             ..Settings::default()
//         };

//         let mut foliage_desc_builder =
//             <Renderer as RendererInterface>::FoliageDescriptionBuilder::new();

//         #[cfg(feature = "vk_backend")]
//         let grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
//             "grass.vert.spv",
//             13,
//             100,
//         ));

//         #[cfg(feature = "wgpu_backend")]
//         let grass =
//             foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new("grass.vert", 13, 100));

//         let mut lum = Renderer::new(&settings, window, &foliage_desc_builder.build());

//         let meshes = AllMeshes::new(&mut lum, grass);

//         lum.load_block(1, assets::get_block(BlockAsset::Dirt));
//         lum.load_block(2, assets::get_block(BlockAsset::Grass));
//         lum.load_block(3, assets::get_block(BlockAsset::GrassNdirt));
//         lum.load_block(4, assets::get_block(BlockAsset::StoneDirt));
//         lum.load_block(5, assets::get_block(BlockAsset::Bush));
//         lum.load_block(6, assets::get_block(BlockAsset::Leaves));
//         lum.load_block(7, assets::get_block(BlockAsset::Iron));
//         lum.load_block(8, assets::get_block(BlockAsset::Lamp));
//         lum.load_block(9, assets::get_block(BlockAsset::StoneBrick));
//         lum.load_block(10, assets::get_block(BlockAsset::StoneBrickCracked));
//         lum.load_block(11, assets::get_block(BlockAsset::StonePack));
//         lum.load_block(12, assets::get_block(BlockAsset::Bark));
//         lum.load_block(13, assets::get_block(BlockAsset::Wood));
//         lum.load_block(14, assets::get_block(BlockAsset::Planks));

//         lum.get_material_palette_mut().copy_from_slice(assets::get_palette());

//         // dbg!(lum.get_material_palette());
//         // dbg!(lum.get_world_blocks());

//         // TODO:
//         lum.update_block_palette_to_gpu();
//         lum.update_material_palette_to_gpu();

//         Self {
//             // window,
//             lum,
//             meshes,
//             transforms: AllTransforms {
//                 tank_body: MeshTransform {
//                     rotation: quat::identity(),
//                     translation: vec3::new(13.1, 14.1, 3.1) * fBLOCK_SIZE,
//                 },
//                 ..Default::default()
//             },
//             about_to_close: false,
//         }
//     }

//     async fn new_async(window: Window, event_loop: &EventLoop<()>) -> Self {
//         let settings = Settings {
//             static_block_palette_size: 15,
//             ..Settings::default()
//         };

//         let mut foliage_desc_builder =
//             <Renderer as RendererInterface>::FoliageDescriptionBuilder::new();

//         #[cfg(feature = "vk_backend")]
//         let grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
//             "grass.vert.spv",
//             13,
//             100,
//         ));

//         #[cfg(feature = "wgpu_backend")]
//         let grass =
//             foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new("grass.vert", 13, 100));

//         let mut lum = Renderer::new_async(&settings, window, &foliage_desc_builder.build()).await;

//         let meshes = AllMeshes::new(&mut lum, grass);

//         lum.load_block(1, assets::get_block(BlockAsset::Dirt));
//         lum.load_block(2, assets::get_block(BlockAsset::Grass));
//         lum.load_block(3, assets::get_block(BlockAsset::GrassNdirt));
//         lum.load_block(4, assets::get_block(BlockAsset::StoneDirt));
//         lum.load_block(5, assets::get_block(BlockAsset::Bush));
//         lum.load_block(6, assets::get_block(BlockAsset::Leaves));
//         lum.load_block(7, assets::get_block(BlockAsset::Iron));
//         lum.load_block(8, assets::get_block(BlockAsset::Lamp));
//         lum.load_block(9, assets::get_block(BlockAsset::StoneBrick));
//         lum.load_block(10, assets::get_block(BlockAsset::StoneBrickCracked));
//         lum.load_block(11, assets::get_block(BlockAsset::StonePack));
//         lum.load_block(12, assets::get_block(BlockAsset::Bark));
//         lum.load_block(13, assets::get_block(BlockAsset::Wood));
//         lum.load_block(14, assets::get_block(BlockAsset::Planks));

//         lum.get_material_palette_mut().copy_from_slice(assets::get_palette());

//         // dbg!(lum.get_material_palette());
//         // dbg!(lum.get_world_blocks());

//         // TODO:
//         lum.update_block_palette_to_gpu();
//         lum.update_material_palette_to_gpu();

//         Self {
//             // window,
//             lum,
//             meshes,
//             transforms: AllTransforms {
//                 tank_body: MeshTransform {
//                     rotation: quat::identity(),
//                     translation: vec3::new(13.1, 14.1, 3.1) * fBLOCK_SIZE,
//                 },
//                 ..Default::default()
//             },
//             about_to_close: false,
//         }
//     }

//     pub fn destroy(mut self) {
//         println!("Shutting down renderer");
//         self.meshes.unload(&mut self.lum);
//         self.lum.unload_block(1);
//         self.lum.unload_block(2);
//         self.lum.unload_block(3);
//         self.lum.unload_block(4);
//         self.lum.unload_block(5);
//         self.lum.unload_block(6);
//         self.lum.unload_block(7);
//         self.lum.unload_block(8);
//         self.lum.unload_block(9);
//         self.lum.unload_block(10);
//         self.lum.unload_block(11);
//         self.lum.unload_block(12);
//         self.lum.unload_block(13);
//         self.lum.unload_block(14);
//         self.lum.destroy();
//     }

//     pub fn load_scene(&mut self) {
//         let scene = assets::get_scene();
//         for_zyx!(scene.size, |x, y, z| {
//             let index =
//                 x + y * scene.size.x as usize + z * scene.size.x as usize * scene.size.y as usize;
//             let v = scene.blocks[index];
//             self.lum.get_world_blocks_mut().set((x, y, z), v);
//         })
//     }

//     pub fn render(&mut self) {
//         // self.transforms.tank_body.translation.x -= 10.0 * self.lum.renderer.delta_time;

//         self.lum.start_frame();

//         self.lum.draw_world();
//         self.lum.draw_model(&self.meshes.tank_body, &self.transforms.tank_body);

//         // literally procedural grass placement every frame. You probably want to store it as entities in your own structures
//         for xx in 4..20 {
//             for yy in 4..20 {
//                 if (5..12).contains(&xx) && (6..16).contains(&yy) {
//                     continue;
//                 };
//                 let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 16.0);
//                 self.lum.draw_foliage(&self.meshes.grass, &pos);
//             }
//         }

//         // literally procedural water placement every frame. You probably want to store it as entities in your own structures
//         for xx in 5..12 {
//             for yy in 6..16 {
//                 let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 14.0);
//                 self.lum.draw_liquid(&self.meshes.water, &pos);
//             }
//         }

//         // literally procedural smoke placement every frame. You probably want to store it as entities in your own structures
//         for xx in 8..10 {
//             for yy in 10..13 {
//                 let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 20.0);
//                 self.lum.draw_volumetric(&self.meshes.smoke, &pos);
//             }
//         }

//         self.lum.prepare_frame();

//         self.lum.end_frame();
//     }
// }

// impl<'renderer, Renderer: RendererInterface> ApplicationHandler for DemoState<Renderer> {
//     fn resumed(&mut self, event_loop: &ActiveEventLoop) {
//         println!("Resumed")
//     }

//     fn window_event(
//         &mut self,
//         _event_loop: &ActiveEventLoop,
//         _window_id: WindowId,
//         event: WindowEvent,
//     ) {
//         match event {
//             WindowEvent::CloseRequested => {
//                 self.about_to_close = true;
//             }
//             WindowEvent::KeyboardInput {
//                 device_id,
//                 event,
//                 is_synthetic,
//             } => {
//                 if event.logical_key
//                     == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
//                 {
//                     self.about_to_close = true;
//                 }
//             }
//             WindowEvent::RedrawRequested => {
//                 #[cfg(target_arch = "wasm32")]
//                 {
//                     self.render();
//                 }
//             }
//             _ => {}
//         }
//     }

//     fn device_event(
//         &mut self,
//         _event_loop: &ActiveEventLoop,
//         _device_id: DeviceId,
//         event: DeviceEvent,
//     ) {
//         // println!("Device event {:?}", event);
//     }

//     fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
//         if self.about_to_close {
//             _event_loop.exit();
//         } else {
//             #[cfg(not(target_arch = "wasm32"))]
//             self.render();
//         }
//     }

//     fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
//         let _ = (event_loop, cause);
//     }

//     fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
//         let _ = (event_loop, event);
//     }

//     fn suspended(&mut self, event_loop: &ActiveEventLoop) {
//         let _ = event_loop;
//     }

//     fn exiting(&mut self, event_loop: &ActiveEventLoop) {
//         let _ = event_loop;
//     }

//     fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
//         let _ = event_loop;
//     }
// }
// #[cfg(not(target_arch = "wasm32"))]
// pub fn run<Renderer: RendererInterface>() {
//     let event_loop = EventLoop::new().unwrap();
//     event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
//     let window_attributes = Window::default_attributes()
//         .with_title("Lumal")
//         .with_maximized(true)
//         // .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
//         ;
//     #[allow(deprecated)] // cause winit is going crazy
//     let window = event_loop.create_window(window_attributes).unwrap();

//     let mut state: DemoState<Renderer> = DemoState::new(window, &event_loop);

//     state.load_scene();

//     let result = event_loop.run_app(&mut state);
//     state.destroy();

//     result.unwrap();
// }

// #[cfg(target_arch = "wasm32")]
// #[wasm_bindgen::prelude::wasm_bindgen(start)]
// pub fn run() {
//     use lum::webgpu::render::RendererWgpu;
//     use wasm_bindgen::JsCast;
//     use winit::platform::web::WindowAttributesExtWebSys;

//     console_error_panic_hook::set_once();

//     panic!("no reason");

//     let window = web_sys::window().unwrap();
//     let document = window.document().unwrap();
//     let canvas = document
//         .get_element_by_id("my_canvas")
//         .expect("put <canvas id='my_canvas'> in html")
//         .dyn_into::<web_sys::HtmlCanvasElement>()
//         .unwrap();

//     let event_loop = EventLoop::new().unwrap();

//     let builder = Window::default_attributes()
//         .with_title("Lumal")
//         .with_maximized(true)
//         // .with_platform_attributes(Box::new(WindowAttributesWeb::default().with_append(true)))
//         .with_canvas(Some(canvas));

//     let window = event_loop.create_window(builder).unwrap();

//     let mut state = DemoState::<RendererWgpu>::new(window, &event_loop);

//     state.load_scene();
//     let result = event_loop.run_app(&mut state);
//     state.destroy();
//     result.unwrap();
// }

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use wgpu::InstanceDescriptor;

use std::sync::Arc;
use web_time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    window::{Theme, Window},
};

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    gui_state: Option<egui_winit::State>,
    last_render_time: Option<Instant>,
    #[cfg(target_arch = "wasm32")]
    renderer_receiver: Option<futures::channel::oneshot::Receiver<Renderer>>,
    last_size: (u32, u32),
    panels_visible: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut attributes = Window::default_attributes();

        #[cfg(not(target_arch = "wasm32"))]
        {
            attributes = attributes.with_title("Standalone Winit/Wgpu Example");
        }

        #[allow(unused_assignments)]
        #[cfg(target_arch = "wasm32")]
        let (mut canvas_width, mut canvas_height) = (0, 0);

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = wgpu::web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("canvas")
                .unwrap()
                .dyn_into::<wgpu::web_sys::HtmlCanvasElement>()
                .unwrap();
            canvas_width = canvas.width();
            canvas_height = canvas.height();
            self.last_size = (canvas_width, canvas_height);
            attributes = attributes.with_canvas(Some(canvas));
        }

        let Ok(window) = event_loop.create_window(attributes) else {
            return;
        };

        let first_window_handle = self.window.is_none();
        let window_handle = Arc::new(window);
        self.window = Some(window_handle.clone());
        if !first_window_handle {
            return;
        }
        let gui_context = egui::Context::default();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let inner_size = window_handle.inner_size();
            self.last_size = (inner_size.width, inner_size.height);
        }

        #[cfg(target_arch = "wasm32")]
        {
            gui_context.set_pixels_per_point(window_handle.scale_factor() as f32);
        }

        let viewport_id = gui_context.viewport_id();
        let gui_state = egui_winit::State::new(
            gui_context,
            viewport_id,
            &window_handle,
            Some(window_handle.scale_factor() as _),
            Some(Theme::Dark),
            None,
        );

        #[cfg(not(target_arch = "wasm32"))]
        let (width, height) = (
            window_handle.inner_size().width,
            window_handle.inner_size().height,
        );

        #[cfg(not(target_arch = "wasm32"))]
        {
            env_logger::init();
            let renderer = pollster::block_on(async move {
                Renderer::new(window_handle.clone(), width, height).await
            });
            self.renderer = Some(renderer);
        }

        #[cfg(target_arch = "wasm32")]
        {
            let (sender, receiver) = futures::channel::oneshot::channel();
            self.renderer_receiver = Some(receiver);
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init().expect("Failed to initialize logger!");
            log::info!("Canvas dimensions: ({canvas_width} x {canvas_height})");
            wasm_bindgen_futures::spawn_local(async move {
                let renderer =
                    Renderer::new(window_handle.clone(), canvas_width, canvas_height).await;
                if sender.send(renderer).is_err() {
                    log::error!("Failed to create and send renderer!");
                }
            });
        }

        self.gui_state = Some(gui_state);
        self.last_render_time = Some(Instant::now());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        #[cfg(target_arch = "wasm32")]
        {
            let mut renderer_received = false;
            if let Some(receiver) = self.renderer_receiver.as_mut() {
                if let Ok(Some(renderer)) = receiver.try_recv() {
                    self.renderer = Some(renderer);
                    renderer_received = true;
                }
            }
            if renderer_received {
                self.renderer_receiver = None;
            }
        }

        let (Some(gui_state), Some(renderer), Some(window), Some(last_render_time)) = (
            self.gui_state.as_mut(),
            self.renderer.as_mut(),
            self.window.as_ref(),
            self.last_render_time.as_mut(),
        ) else {
            return;
        };

        // Receive gui window event
        if gui_state.on_window_event(window, &event).consumed {
            return;
        }

        // If the gui didn't consume the event, handle it
        match event {
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        ..
                    },
                ..
            } => {
                // Exit by pressing the escape key
                if matches!(key_code, winit::keyboard::KeyCode::Escape) {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                log::info!("Resizing renderer surface to: ({width}, {height})");
                renderer.resize(width, height);
                self.last_size = (width, height);

                let scale_factor = window.scale_factor() as f32;
                gui_state.egui_ctx().set_pixels_per_point(scale_factor);
            }
            WindowEvent::CloseRequested => {
                log::info!("Close requested. Exiting...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta_time = now - *last_render_time;
                *last_render_time = now;

                let gui_input = gui_state.take_egui_input(window);
                gui_state.egui_ctx().begin_pass(gui_input);

                #[cfg(not(target_arch = "wasm32"))]
                let title = "Rust/Wgpu";

                #[cfg(feature = "webgpu")]
                let title = "Rust/Wgpu/Webgpu";

                #[cfg(feature = "webgl")]
                let title = "Rust/Wgpu/Webgl";

                if self.panels_visible {
                    egui::TopBottomPanel::top("top").show(gui_state.egui_ctx(), |ui| {
                        ui.horizontal(|ui| {
                            ui.label("File");
                            ui.label("Edit");
                        });
                    });

                    egui::SidePanel::left("left").show(gui_state.egui_ctx(), |ui| {
                        ui.heading("Scene Explorer");
                        if ui.button("Click me!").clicked() {
                            log::info!("Button clicked!");
                        }
                    });

                    egui::SidePanel::right("right").show(gui_state.egui_ctx(), |ui| {
                        ui.heading("Inspector");
                        if ui.button("Click me!").clicked() {
                            log::info!("Button clicked!");
                        }
                    });

                    egui::TopBottomPanel::bottom("bottom").show(gui_state.egui_ctx(), |ui| {
                        ui.heading("Assets");
                        if ui.button("Click me!").clicked() {
                            log::info!("Button clicked!");
                        }
                    });
                }

                egui::Window::new(title).show(gui_state.egui_ctx(), |ui| {
                    ui.checkbox(&mut self.panels_visible, "Show Panels");
                });

                let egui_winit::egui::FullOutput {
                    textures_delta,
                    shapes,
                    pixels_per_point,
                    platform_output,
                    ..
                } = gui_state.egui_ctx().end_pass();

                gui_state.handle_platform_output(window, platform_output);

                let paint_jobs = gui_state.egui_ctx().tessellate(shapes, pixels_per_point);

                let screen_descriptor = {
                    let (width, height) = self.last_size;
                    egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [width, height],
                        pixels_per_point: window.scale_factor() as f32,
                    }
                };

                renderer.render_frame(screen_descriptor, paint_jobs, textures_delta, delta_time);
            }
            _ => (),
        }

        window.request_redraw();
    }
}

pub struct Renderer {
    gpu: Gpu,
    depth_texture_view: wgpu::TextureView,
    egui_renderer: egui_wgpu::Renderer,
    scene: Scene,
}

impl Renderer {
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let gpu = Gpu::new_async(window, width, height).await;
        let depth_texture_view = gpu.create_depth_texture(width, height);

        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.surface_config.format,
            Some(Self::DEPTH_FORMAT),
            1,
            false,
        );

        let scene = Scene::new(&gpu.device, gpu.surface_format);

        Self {
            gpu,
            depth_texture_view,
            egui_renderer,
            scene,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.depth_texture_view = self.gpu.create_depth_texture(width, height);
    }

    pub fn render_frame(
        &mut self,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        paint_jobs: Vec<egui::epaint::ClippedPrimitive>,
        textures_delta: egui::TexturesDelta,
        delta_time: crate::Duration,
    ) {
        let delta_time = delta_time.as_secs_f32();

        self.scene.update(&self.gpu.queue, self.gpu.aspect_ratio(), delta_time);

        for (id, image_delta) in &textures_delta.set {
            self.egui_renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
        }

        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        self.egui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let surface_texture =
            self.gpu.surface.get_current_texture().expect("Failed to get surface texture!");

        let surface_texture_view =
            surface_texture.texture.create_view(&wgpu::TextureViewDescriptor {
                label: wgpu::Label::default(),
                aspect: wgpu::TextureAspect::default(),
                format: Some(self.gpu.surface_format),
                dimension: None,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
                usage: None,
            });

        encoder.insert_debug_marker("Render scene");

        // This scope around the crate::render_pass prevents the
        // crate::render_pass from holding a borrow to the encoder,
        // which would prevent calling `.finish()` in
        // preparation for queue submission.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.19,
                            g: 0.24,
                            b: 0.42,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.scene.render(&mut render_pass);

            self.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &paint_jobs,
                &screen_descriptor,
            );
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }
}

pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,
}

impl Gpu {
    pub fn aspect_ratio(&self) -> f32 {
        self.surface_config.width as f32 / self.surface_config.height.max(1) as f32
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn create_depth_texture(&self, width: u32, height: u32) -> wgpu::TextureView {
        let texture = self.device.create_texture(
            &(wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
        );
        texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            base_array_layer: 0,
            array_layer_count: None,
            mip_level_count: None,
            usage: None,
        })
    }

    pub async fn new_async(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let instance = wgpu::Instance::new(&InstanceDescriptor::default());
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to request adapter!");
        let (device, queue) = {
            log::info!("WGPU Adapter Features: {:#?}", adapter.features());
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("WGPU Device"),
                        memory_hints: wgpu::MemoryHints::default(),
                        required_features: wgpu::Features::default(),
                        #[cfg(not(target_arch = "wasm32"))]
                        required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                        #[cfg(all(target_arch = "wasm32", feature = "webgpu"))]
                        required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                        #[cfg(all(target_arch = "wasm32", feature = "webgl"))]
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits()),
                    },
                    None,
                )
                .await
                .expect("Failed to request a device!")
        };

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb()) // egui wants a non-srgb surface texture
            .unwrap_or(surface_capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        Self {
            surface,
            device,
            queue,
            surface_config,
            surface_format,
        }
    }
}

struct Scene {
    pub model: nalgebra_glm::Mat4,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub uniform: UniformBinding,
    pub pipeline: wgpu::RenderPipeline,
}

impl Scene {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );
        let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("index Buffer"),
                contents: bytemuck::cast_slice(&INDICES),
                usage: wgpu::BufferUsages::INDEX,
            },
        );
        let uniform = UniformBinding::new(device);
        let pipeline = Self::create_pipeline(device, surface_format, &uniform);
        Self {
            model: nalgebra_glm::Mat4::identity(),
            uniform,
            pipeline,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn render<'rpass>(&'rpass self, renderpass: &mut wgpu::RenderPass<'rpass>) {
        renderpass.set_pipeline(&self.pipeline);
        renderpass.set_bind_group(0, &self.uniform.bind_group, &[]);

        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        renderpass.draw_indexed(0..(INDICES.len() as _), 0, 0..1);
    }

    pub fn update(&mut self, queue: &wgpu::Queue, aspect_ratio: f32, delta_time: f32) {
        let projection =
            nalgebra_glm::perspective_lh_zo(aspect_ratio, 80_f32.to_radians(), 0.1, 1000.0);
        let view = nalgebra_glm::look_at_lh(
            &nalgebra_glm::vec3(0.0, 0.0, 3.0),
            &nalgebra_glm::vec3(0.0, 0.0, 0.0),
            &nalgebra_glm::Vec3::y(),
        );
        self.model = nalgebra_glm::rotate(
            &self.model,
            30_f32.to_radians() * delta_time,
            &nalgebra_glm::Vec3::y(),
        );
        self.uniform.update_buffer(
            queue,
            0,
            UniformBuffer {
                mvp: projection * view * self.model,
            },
        );
    }

    fn create_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        uniform: &UniformBinding,
    ) -> wgpu::RenderPipeline {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER_SOURCE)),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniform.bind_group_layout],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vertex_main"),
                buffers: &[Vertex::description(&Vertex::vertex_attributes())],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint32),
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Renderer::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        })
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

impl Vertex {
    pub fn vertex_attributes() -> Vec<wgpu::VertexAttribute> {
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4].to_vec()
    }

    pub fn description(attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        }
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct UniformBuffer {
    mvp: nalgebra_glm::Mat4,
}

struct UniformBinding {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl UniformBinding {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[UniformBuffer::default()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        Self {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update_buffer(
        &mut self,
        queue: &wgpu::Queue,
        offset: wgpu::BufferAddress,
        uniform_buffer: UniformBuffer,
    ) {
        queue.write_buffer(
            &self.buffer,
            offset,
            bytemuck::cast_slice(&[uniform_buffer]),
        )
    }
}

const VERTICES: [Vertex; 3] = [
    Vertex {
        position: [1.0, -1.0, 0.0, 1.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-1.0, -1.0, 0.0, 1.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0, 1.0],
        color: [0.0, 0.0, 1.0, 1.0],
    },
];

#[wasm_bindgen::prelude::wasm_bindgen(start)]
fn run() {
    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}

const INDICES: [u32; 3] = [0, 1, 2]; // Clockwise winding order

const SHADER_SOURCE: &str = "
struct Uniform {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.color = vert.color;
    out.position = ubo.mvp * vert.position;
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color);
}
";
