#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(inherent_associated_types)]

//! Actual code for the Lum example ([bin]s demo_vk and demo_wgpu are 3-line callers of this [lib])

use assets::{BlockAsset, ModelAsset};
use containers::array3d::ConstDims;
use lum::render_interface::ShaderSource;
use lum::{
    fBLOCK_SIZE, for_zyx,
    render_interface::{FoliageDescriptionBuilder, FoliageDescriptionCreate, RendererInterface},
    types::{u8vec3, vec3, MeshFoliage, MeshLiquid, MeshModel, MeshTransform, MeshVolumetric},
    Settings,
};
use std::marker::PhantomData;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
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

// we "harcode" world size but it could also be dynamic with `RuntimeDims`
pub type WorldSize = ConstDims<48, 48, 16>;
const WORLD_SIZE: WorldSize = WorldSize {};

struct DemoState<'renderer, Renderer: RendererInterface<'renderer, WorldSize>> {
    window: Arc<Window>,
    lum: Renderer,
    meshes: AllMeshes,
    transforms: AllTransforms,
    about_to_close: bool,
    _phantom: PhantomData<&'renderer Renderer>,
}

impl<'r, Renderer: RendererInterface<'r, WorldSize>> DemoState<'r, Renderer> {
    type FoliageDescription = Renderer::FoliageDescription;

    fn new(window: Arc<Window>, lum: Renderer) -> Self {
        Self {
            window,
            lum,
            meshes: AllMeshes::default(),
            transforms: Default::default(),
            about_to_close: false,
            _phantom: PhantomData::default(),
        }
    }

    pub fn load_scene(&mut self) {
        let lum = &mut self.lum;

        // TODO: move to template args?
        let settings = Settings::<WorldSize> {
            static_block_palette_size: 15,
            ..Settings::default()
        };

        let mut foliage_desc_builder =
            <Renderer as RendererInterface<WorldSize>>::FoliageDescriptionBuilder::new();

        // here im using my `shaders` crate, but you (most likely)
        // need to load it from somewhere else
        #[cfg(feature = "vk_backend")]
        let grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
            ShaderSource::SpirV(shaders::Shader::get_spirv(shaders::Shader::GrassVert)),
            13,
            100,
        ));

        #[cfg(feature = "wgpu_backend")]
        let grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
            ShaderSource::Wgsl(shaders::Shader::get_wgsl(shaders::Shader::GrassVert)),
            13,
            100,
        ));

        self.meshes = AllMeshes::new(lum, grass);

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
        self.meshes.unload(&mut self.lum);
        // Unload blocks
        self.lum.unload_block(1);
        self.lum.unload_block(2);
        self.lum.unload_block(3);
        self.lum.unload_block(4);
        self.lum.unload_block(5);
        self.lum.unload_block(6);
        self.lum.unload_block(7);
        self.lum.unload_block(8);
        self.lum.unload_block(9);
        self.lum.unload_block(10);
        self.lum.unload_block(11);
        self.lum.unload_block(12);
        self.lum.unload_block(13);
        self.lum.unload_block(14);
        self.lum.destroy();
    }

    pub fn render(&mut self) {
        let lum = &mut self.lum;

        lum.start_frame();
        lum.draw_world();
        lum.draw_model(&self.meshes.tank_body, &self.transforms.tank_body);

        for xx in 4..20 {
            for yy in 4..20 {
                if (5..12).contains(&xx) && (6..16).contains(&yy) {
                    continue;
                };
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 16.0);
                lum.draw_foliage(&self.meshes.grass, &pos);
            }
        }

        for xx in 5..12 {
            for yy in 6..16 {
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 14.0);
                lum.draw_liquid(&self.meshes.water, &pos);
            }
        }

        for xx in 8..10 {
            for yy in 10..13 {
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 20.0);
                lum.draw_volumetric(&self.meshes.smoke, &pos);
            }
        }

        lum.prepare_frame();
        lum.end_frame();
    }
}

impl<'renderer, Renderer: RendererInterface<'renderer, WorldSize> + 'static> ApplicationHandler
    for DemoState<'renderer, Renderer>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Lum Resumed");
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = &self.window.clone();

        match event {
            WindowEvent::CloseRequested => {
                #[cfg(target_arch = "wasm32")]
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
                #[cfg(target_arch = "wasm32")]
                log::info!("Resizing renderer surface to: ({width}, {height})");
                self.lum.resize(PhysicalSize { width, height });
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
            // #[cfg(not(target_arch = "wasm32"))]
            // {
            //     if let Some(window) = self.window.as_ref() {
            //         window.request_redraw(); // Request redraw for continuous rendering on desktop
            //     }
            // }
            self.window.request_redraw();
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {}

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {}

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {}

    fn memory_warning(&mut self, _event_loop: &ActiveEventLoop) {}
}

// entry point for WASM
// So what is so special about WASM? Primary difference is that there is no blocking execution on web
// But also the fact that window events are different
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn run() {
    // web is only supported for wgpu backend
    use lum::webgpu::render::RendererWgpu;

    console_error_panic_hook::set_once();
    console_log::init().expect("Failed to initialize logger for WASM!");
    log::info!("Lumal starting...");

    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let canvas = web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.get_element_by_id("lum_canvas"))
        .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        .expect("HTML document must contain a <canvas id='lum_canvas'> element");

    let canvas_size = PhysicalSize::new(canvas.width(), canvas.height());
    let mut attributes = Window::default_attributes();
    attributes = attributes.with_canvas(Some(canvas));

    // i typically avoid arcs and restructure code to be simple references & lifetimes, but Window is an exception
    let window = Arc::new(event_loop.create_window(attributes).unwrap());

    let settings = Settings {
        static_block_palette_size: 15,
        ..Settings::default()
    };

    let mut foliage_desc_builder =
        <RendererWgpu<WorldSize> as RendererInterface<WorldSize>>::FoliageDescriptionBuilder::new();
    let _grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
        ShaderSource::Wgsl(shaders::Shader::get_wgsl(shaders::Shader::GrassVert)),
        13,
        100,
    ));

    log::info!("Requesting WGPU renderer...");
    let renderer = RendererWgpu::<WorldSize>::new_async(
        &settings,
        window.clone(),
        canvas_size,
        &foliage_desc_builder.build(),
    )
    .await; // browsers do not allow blocking exec
    log::info!("WGPU renderer initialized successfully!");

    let mut app = DemoState::new(window, renderer);

    app.load_scene();

    event_loop.run_app(&mut app).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run<'renderer, Renderer: RendererInterface<'renderer, WorldSize> + 'static>() {
    use lum::render_interface::ShaderSource;

    let event_loop = winit::event_loop::EventLoop::builder().build().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let attributes = Window::default_attributes().with_title("Lum demo");

    // i typically avoid arcs and restructure code to be simple references & lifetimes, but Window is an exception
    let window = Arc::new(event_loop.create_window(attributes).unwrap());

    let settings = Settings {
        static_block_palette_size: 15,
        ..Settings::default()
    };

    let mut foliage_desc_builder =
        <Renderer as RendererInterface<WorldSize>>::FoliageDescriptionBuilder::new();
    #[cfg(feature = "vk_backend")]
    let _grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
        ShaderSource::SpirV(shaders::Shader::get_spirv(shaders::Shader::GrassVert)),
        13,
        100,
    ));

    #[cfg(feature = "wgpu_backend")]
    let _grass = foliage_desc_builder.load_foliage(FoliageDescriptionCreate::new(
        ShaderSource::Wgsl(shaders::Shader::get_wgsl(shaders::Shader::GrassVert)),
        13,
        100,
    ));

    let renderer = Renderer::new(
        &settings,
        window.clone(),
        window.inner_size(),
        &foliage_desc_builder.build(),
    ); // on native we can execute sequentially

    let mut app = DemoState::new(window, renderer);

    app.load_scene();

    event_loop.run_app(&mut app).unwrap();
}

// You probably should use some sort of "Asset library" - hashmap (array) of YourEntityTypeEnum -> LumMeshModel
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
    fn new<'a, T: RendererInterface<'a, WorldSize>>(lum: &mut T, grass: MeshFoliage) -> Self {
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

    fn unload<'a, T: RendererInterface<'a, WorldSize>>(self, lum: &mut T) {
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
