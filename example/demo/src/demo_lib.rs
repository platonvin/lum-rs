#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(inherent_associated_types)]

//! This is actual code for the provided example ([bin]s demo_vk and demo_wgpu are 3-line callers of this [lib])
//! I highly recommend looking into winit/wgpu examples, too
//! Also, if you are going to compile your project into web, its better to start doing so as soon as possible

use std::{sync::Arc, time::Instant};

use assets::{BlockAsset, ModelAsset};
use lum::{
    fBLOCK_SIZE, for_zyx,
    render_interface::{FoliageDescriptionBuilder, FoliageDescriptionCreate, RendererInterface},
    types::{
        quat, u8vec3, uvec3, vec3, MeshBlock, MeshFoliage, MeshLiquid, MeshModel, MeshTransform,
        MeshVolumetric,
    },
    Settings,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys;

#[cfg(target_arch = "wasm32")]
use console_error_panic_hook;
#[cfg(target_arch = "wasm32")]
use console_log;
use futures::channel::oneshot;

// i hardcode it but you probably should use some sort of "Asset library" - hashmap (array) of YourEntityTypeEnum -> LumMeshModel
struct AllMeshes {
    tank_body: MeshModel,
    tank_head: MeshModel,
    tank_rf_leg: MeshModel,
    tank_lb_leg: MeshModel,
    tank_lf_leg: MeshModel,
    tank_rb_leg: MeshModel,
    water: MeshLiquid,
    grass: MeshFoliage,
    smoke: MeshVolumetric,
}
#[derive(Default)]
struct AllTransforms {
    tank_body: MeshTransform,
    tank_head: MeshTransform,
    tank_rf_leg: MeshTransform,
    tank_lb_leg: MeshTransform,
    tank_lf_leg: MeshTransform,
    tank_rb_leg: MeshTransform,
}

impl AllMeshes {
    fn new<T: RendererInterface>(lum: &mut T, grass: MeshFoliage) -> Self {
        Self {
            tank_body: lum.load_model(assets::get_model(ModelAsset::TankBody)),
            tank_head: lum.load_model(assets::get_model(ModelAsset::TankHead)),
            tank_rf_leg: lum.load_model(assets::get_model(ModelAsset::TankRfLbLeg)),
            tank_lb_leg: lum.load_model(assets::get_model(ModelAsset::TankRfLbLeg)),
            tank_lf_leg: lum.load_model(assets::get_model(ModelAsset::TankLfRbLeg)),
            tank_rb_leg: lum.load_model(assets::get_model(ModelAsset::TankLfRbLeg)),
            water: lum.load_liquid(69, 42),
            grass,
            smoke: lum.load_volumetric(1.0, 0.5, u8vec3::zero()),
        }
    }

    fn unload<T: RendererInterface>(self, lum: &mut T) {
        lum.unload_model(self.tank_body);
        lum.unload_model(self.tank_head);
        lum.unload_model(self.tank_rf_leg);
        lum.unload_model(self.tank_lb_leg);
        lum.unload_model(self.tank_lf_leg);
        lum.unload_model(self.tank_rb_leg);
        lum.unload_liquid(self.water);
        lum.unload_foliage(self.grass);
        lum.unload_volumetric(self.smoke);
    }
}

struct DemoState<Renderer: RendererInterface> {
    window: Option<Arc<Window>>,
    lum: Option<Renderer>,
    meshes: Option<AllMeshes>,
    transforms: AllTransforms,
    about_to_close: bool,
    #[cfg(target_arch = "wasm32")]
    renderer_receiver: Option<oneshot::Receiver<Renderer>>, // For async WASM init result
}

impl<Renderer: RendererInterface> Default for DemoState<Renderer> {
    fn default() -> Self {
        Self {
            window: Default::default(),
            lum: Default::default(),
            meshes: Default::default(),
            transforms: Default::default(),
            about_to_close: Default::default(),
            #[cfg(target_arch = "wasm32")]
            renderer_receiver: None,
        }
    }
}

impl<'renderer, Renderer: RendererInterface> DemoState<Renderer> {
    type FoliageDescription = Renderer::FoliageDescription;

    fn new() -> Self {
        Self::default()
    }

    // called when the renderer is ready
    pub fn load_scene(&mut self) {
        let Some(lum) = self.lum.as_mut() else {
            log::error!("Cannot load scene: Renderer not initialized yet.");
            return;
        };

        // These settings and foliage descriptions are part of your scene loading,
        // and should be performed *after* the renderer is ready.
        // TODO: move to template args?
        let settings = Settings {
            static_block_palette_size: 15,
            ..Settings::default()
        };

        let mut foliage_desc_builder =
            <Renderer as RendererInterface>::FoliageDescriptionBuilder::new();

        #[cfg(feature = "vk_backend")]
        let grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
            "grass.vert.spv",
            13,
            100,
        ));

        #[cfg(feature = "wgpu_backend")]
        let grass =
            foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new("grass.vert", 13, 100));

        let meshes = AllMeshes::new(lum, grass);
        self.meshes = Some(meshes);

        lum.load_block(1, assets::get_block(BlockAsset::Dirt));
        lum.load_block(2, assets::get_block(BlockAsset::Grass));
        lum.load_block(3, assets::get_block(BlockAsset::GrassNdirt));
        lum.load_block(4, assets::get_block(BlockAsset::StoneDirt));
        lum.load_block(5, assets::get_block(BlockAsset::Bush));
        lum.load_block(6, assets::get_block(BlockAsset::Leaves));
        lum.load_block(7, assets::get_block(BlockAsset::Iron));
        lum.load_block(8, assets::get_block(BlockAsset::Lamp));
        lum.load_block(9, assets::get_block(BlockAsset::StoneBrick));
        lum.load_block(10, assets::get_block(BlockAsset::StoneBrickCracked));
        lum.load_block(11, assets::get_block(BlockAsset::StonePack));
        lum.load_block(12, assets::get_block(BlockAsset::Bark));
        lum.load_block(13, assets::get_block(BlockAsset::Wood));
        lum.load_block(14, assets::get_block(BlockAsset::Planks));

        lum.get_material_palette_mut().copy_from_slice(assets::get_palette());

        lum.update_block_palette_to_gpu();
        lum.update_material_palette_to_gpu();

        let scene = assets::get_scene();
        for_zyx!(scene.size, |x, y, z| {
            let index =
                x + y * scene.size.x as usize + z * scene.size.x as usize * scene.size.y as usize;
            let v = scene.blocks[index];
            lum.get_world_blocks_mut().set((x, y, z), v);
        });

        println!("Lumal: Scene loaded!");
    }

    pub fn destroy(mut self) {
        println!("Shutting down renderer");
        if let Some(mut lum_instance) = self.lum.take() {
            if let Some(meshes_instance) = self.meshes.take() {
                meshes_instance.unload(&mut lum_instance);
            }
            // Unload blocks
            lum_instance.unload_block(1);
            lum_instance.unload_block(2);
            lum_instance.unload_block(3);
            lum_instance.unload_block(4);
            lum_instance.unload_block(5);
            lum_instance.unload_block(6);
            lum_instance.unload_block(7);
            lum_instance.unload_block(8);
            lum_instance.unload_block(9);
            lum_instance.unload_block(10);
            lum_instance.unload_block(11);
            lum_instance.unload_block(12);
            lum_instance.unload_block(13);
            lum_instance.unload_block(14);
            lum_instance.destroy();
        }
    }

    pub fn render(&mut self) {
        let Some(lum) = self.lum.as_mut() else {
            // Only log this warning for WASM, as desktop initializes synchronously
            #[cfg(target_arch = "wasm32")]
            log::warn!("Renderer not yet initialized for rendering. Waiting for WGPU context.");
            return;
        };
        let Some(meshes) = self.meshes.as_ref() else {
            log::warn!("Meshes not loaded yet. Skipping render frame.");
            return;
        };

        lum.start_frame();
        lum.draw_world();
        lum.draw_model(&meshes.tank_body, &self.transforms.tank_body);

        for xx in 4..20 {
            for yy in 4..20 {
                if (5..12).contains(&xx) && (6..16).contains(&yy) {
                    continue;
                };
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 16.0);
                lum.draw_foliage(&meshes.grass, &pos);
            }
        }

        for xx in 5..12 {
            for yy in 6..16 {
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 14.0);
                lum.draw_liquid(&meshes.water, &pos);
            }
        }

        for xx in 8..10 {
            for yy in 10..13 {
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 20.0);
                lum.draw_volumetric(&meshes.smoke, &pos);
            }
        }

        lum.prepare_frame();
        lum.end_frame();
    }
}

impl<Renderer: RendererInterface + 'static> ApplicationHandler for DemoState<Renderer> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Application Resumed");

        // Prevent re-initialization if `resumed` is called multiple times.
        // This is crucial for platforms like Android where `resumed` can be called often.

        #[cfg(target_arch = "wasm32")]
        if self.renderer_receiver.is_some() || self.lum.is_some() {
            return;
        }

        let mut attributes = Window::default_attributes();

        #[cfg(not(target_arch = "wasm32"))]
        {
            attributes = attributes.with_title("Lumal WGPU Renderer (Desktop)");
            // Keep your Lumal title
            // Initialize logger for desktop builds; `env_logger` is common.
            // env_logger::init();
        }

        #[cfg(target_arch = "wasm32")]
        let (mut canvas_width, mut canvas_height) = (0, 0); // Initialize for WASM

        #[cfg(target_arch = "wasm32")]
        {
            // Acquire the HTML canvas element. Your `index.html` must have:
            // `<canvas id="canvas" style="width: 100vw; height: 100vh;"></canvas>`
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("canvas") // Changed from "my_canvas" to "canvas" for common practice
                .expect("HTML document must contain a <canvas id='canvas'> element.")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();
            canvas_width = canvas.width();
            canvas_height = canvas.height();
            attributes = attributes.with_canvas(Some(canvas)); // Attach winit window to this canvas

            // Initialize WASM-specific debugging tools for browser console output.
            // std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            // console_log::init().expect("Failed to initialize logger for WASM!");
            log::info!("Canvas dimensions: ({canvas_width} x {canvas_height})");
        }

        let Ok(window) = event_loop.create_window(attributes) else {
            log::error!("Failed to create winit window!");
            return;
        };

        let window_handle = Arc::new(window);

        self.window = Some(window_handle.clone()); // Store the window handle in DemoState

        let settings = Settings {
            static_block_palette_size: 15,
            ..Settings::default()
        };

        // --- Asynchronous WGPU initialization for WASM, synchronous for desktop ---
        #[cfg(not(target_arch = "wasm32"))]
        {
            let inner_size = window_handle.inner_size();

            let mut foliage_desc_builder =
                <Renderer as RendererInterface>::FoliageDescriptionBuilder::new();

            // TODO: i probably should move it into demo asset library
            #[cfg(feature = "vk_backend")]
            let grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
                "grass.vert.spv",
                13,
                100,
            ));
            #[cfg(feature = "wgpu_backend")]
            let grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
                "grass.vert",
                13,
                100,
            ));
            let renderer = Renderer::new(&settings, window_handle, &foliage_desc_builder.build());

            self.lum = renderer; // Store the initialized renderer
            self.load_scene(); // Load scene after renderer is ready
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Set up a channel to receive the renderer once it's initialized asynchronously.
            let (sender, receiver) = oneshot::channel();
            self.renderer_receiver = Some(receiver);

            let settings_clone = settings.clone();

            // Spawn the asynchronous renderer creation task locally onto the browser's event loop
            wasm_bindgen_futures::spawn_local(async move {
                let mut foliage_desc_builder =
                    <Renderer as RendererInterface>::FoliageDescriptionBuilder::new();
                #[cfg(feature = "wgpu_backend")]
                let _grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
                    "grass.vert",
                    13,
                    100,
                ));

                let wh = window_handle.inner_size();
                log::info!("on creation window size: {} {}", wh.width, wh.height);
                log::info!(
                    "on creation canvas size: {} {}",
                    canvas_width,
                    canvas_height
                );

                let renderer = Renderer::new_async(
                    &settings_clone,
                    window_handle.clone(),
                    PhysicalSize {
                        width: canvas_width,
                        height: canvas_height,
                    },
                    &foliage_desc_builder.build(),
                )
                .await;

                // Send the initialized renderer back to the main thread
                if sender.send(renderer).is_err() {
                    log::error!("Failed to send initialized renderer over channel!");
                }
            });
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        #[cfg(target_arch = "wasm32")]
        {
            // attempt to receive the renderer from the oneshot channel
            let mut renderer_received = false;
            if let Some(receiver) = self.renderer_receiver.as_mut() {
                if let Ok(Some(renderer)) = receiver.try_recv() {
                    self.lum = Some(renderer); // Store the received renderer
                    renderer_received = true;
                }
            }
            if renderer_received {
                self.renderer_receiver = None; // Clear the receiver once lum received
                log::info!("Renderer initialized on WASM!");
                self.lum.as_mut().unwrap().resize(self.window.clone().unwrap().inner_size());
                log::info!("First resize over");
                // self.load_scene(); // Load the scene AFTER the renderer is fully ready
                log::info!("Loaded the scene");
            }
        }

        let Some(window) = self.window.as_ref().cloned() else {
            log::info!("Invalid window");
            return; // No window to process events for
        };

        match event {
            WindowEvent::CloseRequested => {
                log::info!("CloseRequested");
                self.about_to_close = true;
            }
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        ..
                    },
                ..
            } => {
                if matches!(key_code, winit::keyboard::KeyCode::Escape) {
                    self.about_to_close = true;
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                log::info!("Resizing renderer surface to: ({width}, {height})");
                if let Some(lum) = self.lum.as_mut() {
                    lum.resize(PhysicalSize { width, height });
                }
            }
            _ => { /* Ignore other window events for this example */ }
        }

        window.request_redraw();
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        _event: DeviceEvent,
    ) {
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // This is called when the event loop is about to go idle.
        if self.about_to_close {
            _event_loop.exit(); // Exit the event loop if requested
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw(); // Request redraw for continuous rendering on desktop
                }
            }
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {}

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {}

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {}

    fn memory_warning(&mut self, _event_loop: &ActiveEventLoop) {}
}

// entry point for WASM
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)] // This macro makes `run` the entry point for WASM
pub fn run() {
    // WASM-specific console output
    console_error_panic_hook::set_once(); // Catch Rust panics and print to console
    console_log::init().expect("Failed to initialize logger for WASM!");
    log::info!("Lumal starting...");

    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
    // for continuous rendering in the browser
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    // Initialize DemoState with its default state
    // The actual canvas acquisition and asynchronous WGPU setup happens in `resumed`
    let mut app = DemoState::<lum::webgpu::render::RendererWgpu>::new();

    // Start the event loop. `resumed` will be called
    event_loop.run_app(&mut app).unwrap();
}
