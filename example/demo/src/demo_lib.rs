#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(inherent_associated_types)]

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

// WASM specific imports for DOM access and asynchronous tasks
#[cfg(target_arch = "wasm32")]
use lum::webgpu::render::RendererWgpu; // Explicitly using your RendererWgpu for WASM
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowAttributesExtWebSys; // For .with_canvas() // For .dyn_into()

// For logging and error handling on both platforms
#[cfg(target_arch = "wasm32")]
use console_error_panic_hook;
#[cfg(target_arch = "wasm32")]
use console_log;
use futures::channel::oneshot;
#[cfg(not(target_arch = "wasm32"))]
use pollster; // For blocking on async in non-WASM environments // Provides the oneshot channel for async results

// i hardcode it but you probably should use some sort of "Asset library" - hashmap (array) of YourEntityTypeEnum -> LumMeshModel
#[derive(Default)]
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
    // window: Option<Arc<Window>>, // Now an Option<Arc<Window>>
    lum: Renderer,     // Renderer is now an Option, initialized asynchronously
    meshes: AllMeshes, // Meshes will be loaded after renderer is ready
    transforms: AllTransforms,
    about_to_close: bool,
    #[cfg(target_arch = "wasm32")]
    // renderer_receiver: Option<oneshot::Receiver<Renderer>>, // For async WASM init result
    // last_render_time: Option<Instant>, // To calculate delta time between frames
    last_size: (u32, u32), // Stores the last known window/canvas size
}

// impl<Renderer: RendererInterface> Default for DemoState<Renderer> {
//     fn default() -> Self {
//         Self {
//             // window: Default::default(),
//             lum: Default::default(),
//             meshes: Default::default(),
//             transforms: Default::default(),
//             about_to_close: Default::default(),
//             // #[cfg(target_arch = "wasm32")]
//             // renderer_receiver: None,
//             // last_render_time: Default::default(),
//             last_size: Default::default(),
//             init: false,
//         }
//     }
// }

impl<'renderer, Renderer: RendererInterface> DemoState<Renderer> {
    type FoliageDescription = Renderer::FoliageDescription;

    fn new(event_loop: &EventLoop<()>) -> Self {
        let mut attributes = Window::default_attributes();

        {
            let (mut canvas_width, mut canvas_height) = (0, 0); // Initialize for WASM
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
            // self.last_size = (canvas_width, canvas_height); // Store initial canvas size
            attributes = attributes.with_canvas(Some(canvas)); // Attach winit window to this canvas

            log::info!("resumed-init canvas dimensions: ({canvas_width} * {canvas_height})");
        }

        let Ok(window) = event_loop.create_window(attributes) else {
            log::error!("Failed to create winit window!");
            panic!();
        };

        let settings = Settings {
            static_block_palette_size: 15,
            ..Settings::default()
        };

        let settings_clone = settings.clone();

        let mut foliage_desc_builder =
            <Renderer as RendererInterface>::FoliageDescriptionBuilder::new();

        #[cfg(feature = "wgpu_backend")]
        let grass =
            foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new("grass.vert", 13, 100));

        let mut lum = Renderer::new(&settings_clone, window, &foliage_desc_builder.build());

        let meshes = AllMeshes::new(&mut lum, grass);

        // Your block and palette loading logic
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

        // Your scene data loading (world blocks)
        let scene = assets::get_scene();
        for_zyx!(scene.size, |x, y, z| {
            let index =
                x + y * scene.size.x as usize + z * scene.size.x as usize * scene.size.y as usize;
            let v = scene.blocks[index];
            lum.get_world_blocks_mut().set((x, y, z), v);
        });

        Self {
            lum,
            meshes,
            transforms: AllTransforms::default(),
            about_to_close: false,
            last_size: (0, 0),
        }
    }

    // `load_scene` method is called when the renderer is ready
    pub fn load_scene(&mut self) {
        // let Some(lum) = self.lum.as_mut() else {
        //     log::error!("Cannot load scene: Renderer not initialized yet.");
        //     return;
        // };

        // These settings and foliage descriptions are part of your scene loading,
        // and should be performed *after* the renderer is ready.
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

        // #[cfg(feature = "wgpu_backend")]
        let grass =
            foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new("grass.vert", 13, 100));

        println!("Lumal: Scene loaded!");
    }

    pub fn destroy(mut self) {
        println!("Shutting down renderer");
        // Take ownership of `lum` and `meshes` to ensure proper cleanup
        let meshes = self.meshes;
        let mut lum = self.lum;
        meshes.unload(&mut lum);
        // Unload blocks
        lum.unload_block(1);
        lum.unload_block(2);
        lum.unload_block(3);
        lum.unload_block(4);
        lum.unload_block(5);
        lum.unload_block(6);
        lum.unload_block(7);
        lum.unload_block(8);
        lum.unload_block(9);
        lum.unload_block(10);
        lum.unload_block(11);
        lum.unload_block(12);
        lum.unload_block(13);
        lum.unload_block(14);
        lum.destroy(); // Call the renderer's destroy method
    }

    pub fn render(&mut self) {
        // let Some(lum) = self.lum.as_mut() else {
        //     // Only log this warning for WASM, as desktop initializes synchronously
        //     #[cfg(target_arch = "wasm32")]
        //     log::warn!("Renderer not yet initialized for rendering. Waiting for WGPU context.");
        //     return;
        // };
        // let Some(meshes) = self.meshes.as_ref() else {
        //     log::warn!("Meshes not loaded yet. Skipping render frame.");
        //     return;
        // };

        // Calculate delta time
        // let now = Instant::now();
        // let _delta_time = now - self.last_render_time.unwrap_or(now);
        // self.last_render_time = Some(now);

        // Your existing rendering logic using `lum` and `meshes`
        self.lum.start_frame();
        self.lum.draw_world();
        self.lum.draw_model(&self.meshes.tank_body, &self.transforms.tank_body);

        for xx in 4..20 {
            for yy in 4..20 {
                if (5..12).contains(&xx) && (6..16).contains(&yy) {
                    continue;
                };
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 16.0);
                self.lum.draw_foliage(&self.meshes.grass, &pos);
            }
        }

        for xx in 5..12 {
            for yy in 6..16 {
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 14.0);
                self.lum.draw_liquid(&self.meshes.water, &pos);
            }
        }

        for xx in 8..10 {
            for yy in 10..13 {
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 20.0);
                self.lum.draw_volumetric(&self.meshes.smoke, &pos);
            }
        }

        self.lum.prepare_frame();
        self.lum.end_frame();
    }
}

impl<Renderer: RendererInterface + 'static> ApplicationHandler for DemoState<Renderer> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Application Resumed");
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.about_to_close = true;
            }
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code), // Use physical_key for consistent key handling
                        ..
                    },
                ..
            } => {
                if matches!(key_code, winit::keyboard::KeyCode::Escape) {
                    self.about_to_close = true;
                }
            }
            WindowEvent::RedrawRequested => {
                // This is the primary trigger for rendering a frame.
                // Call render only if `lum` (renderer) is initialized
                self.render();
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                log::info!("Resizing renderer surface to: ({width}, {height})");
                self.last_size = (width, height);
                self.lum.resize(PhysicalSize { width, height });
            }
            _ => { /* Ignore other window events for this example */ }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        // println!("Device event {:?}", event);
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
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        let _ = (event_loop, cause);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        let _ = (event_loop, event);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }
}

// // Entry point for desktop build (`cargo run`)
// #[cfg(not(target_arch = "wasm32"))]
// // The Renderer type must be 'static because it's stored in `DemoState` for the app's lifetime.
// pub fn run<Renderer: RendererInterface + 'static>() {
//     let event_loop = EventLoop::new().unwrap();
//     // Use `ControlFlow::Poll` for a game loop that redraws continuously.
//     event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

//     // Initialize DemoState with its default state.
//     // The actual window creation and renderer initialization will happen in `resumed`.
//     let mut app = DemoState::<Renderer>::new();

//     // Start the event loop.
//     event_loop.run_app(&mut app).unwrap();
//     // `state.destroy()` is now called in the `exiting` hook of ApplicationHandler.
// }

// Entry point for WASM build (`wasm-pack build --target web` and serve with a web server)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)] // This macro makes `run` the entry point for WASM
pub fn run() {
    // These use statements are important for the specific RendererWgpu implementation
    // and wasm-bindgen features.
    use lum::webgpu::render::RendererWgpu;

    // Initialize WASM-specific debugging tools for browser console output.
    console_error_panic_hook::set_once(); // Catch Rust panics and print to console
    console_log::init().expect("Failed to initialize logger for WASM!");
    log::info!("Lumal WASM application starting...");

    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
    // Use `ControlFlow::Poll` for continuous rendering in the browser.
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    // Initialize DemoState with its default state.
    // The actual canvas acquisition and asynchronous WGPU setup happens in `resumed`.
    let mut app = DemoState::<RendererWgpu>::new(&event_loop);

    // Start the event loop. `resumed` will be called.
    event_loop.run_app(&mut app).unwrap();
    // `state.destroy()` is now called in the `exiting` hook of ApplicationHandler.
}
