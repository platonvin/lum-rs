#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(inherent_associated_types)]

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
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

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
    // window: &'renderer Window,
    lum: Renderer,
    meshes: AllMeshes,
    transforms: AllTransforms,
    about_to_close: bool,
}

impl<'renderer, Renderer: RendererInterface> DemoState<Renderer> {
    type FoliageDescription = Renderer::FoliageDescription;

    fn new(window: Window, event_loop: &EventLoop<()>) -> Self {
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

        let mut lum = Renderer::new(&settings, window, &foliage_desc_builder.build());

        let meshes = AllMeshes::new(&mut lum, grass);

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

        // dbg!(lum.get_material_palette());
        // dbg!(lum.get_world_blocks());

        // TODO:
        lum.update_block_palette_to_gpu();
        lum.update_material_palette_to_gpu();

        Self {
            // window,
            lum,
            meshes,
            transforms: AllTransforms {
                tank_body: MeshTransform {
                    rotation: quat::identity(),
                    translation: vec3::new(13.1, 14.1, 3.1) * fBLOCK_SIZE,
                },
                ..Default::default()
            },
            about_to_close: false,
        }
    }

    pub fn destroy(mut self) {
        println!("Shutting down renderer");
        self.meshes.unload(&mut self.lum);
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

    pub fn load_scene(&mut self) {
        let scene = assets::get_scene();
        for_zyx!(scene.size, |x, y, z| {
            let index =
                x + y * scene.size.x as usize + z * scene.size.x as usize * scene.size.y as usize;
            let v = scene.blocks[index];
            self.lum.get_world_blocks_mut().set((x, y, z), v);
        })
    }

    pub fn render(&mut self) {
        // self.transforms.tank_body.translation.x -= 10.0 * self.lum.renderer.delta_time;

        self.lum.start_frame();

        self.lum.draw_world();
        self.lum.draw_model(&self.meshes.tank_body, &self.transforms.tank_body);

        // literally procedural grass placement every frame. You probably want to store it as entities in your own structures
        for xx in 4..20 {
            for yy in 4..20 {
                if (5..12).contains(&xx) && (6..16).contains(&yy) {
                    continue;
                };
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 16.0);
                self.lum.draw_foliage(&self.meshes.grass, &pos);
            }
        }

        // literally procedural water placement every frame. You probably want to store it as entities in your own structures
        for xx in 5..12 {
            for yy in 6..16 {
                let pos = vec3::new(xx as f32 * fBLOCK_SIZE, yy as f32 * fBLOCK_SIZE, 14.0);
                self.lum.draw_liquid(&self.meshes.water, &pos);
            }
        }

        // literally procedural smoke placement every frame. You probably want to store it as entities in your own structures
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

impl<'renderer, Renderer: RendererInterface> ApplicationHandler for DemoState<Renderer> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Resumed")
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
                device_id,
                event,
                is_synthetic,
            } => {
                if event.logical_key
                    == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
                {
                    self.about_to_close = true;
                }
            }
            _ => {}
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
        if self.about_to_close {
            // self.render();
            _event_loop.exit();
        } else {
            self.render();
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
#[cfg(not(target_arch = "wasm32"))]
pub fn run<Renderer: RendererInterface>() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let window_attributes = Window::default_attributes()
        .with_title("Lumal")
        .with_maximized(true)
        // .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
        ;
    #[allow(deprecated)] // cause winit is going crazy
    let window = event_loop.create_window(window_attributes).unwrap();

    let mut state: DemoState<Renderer> = DemoState::new(window, &event_loop);

    state.load_scene();
    // state.lum.

    let result = event_loop.run_app(&mut state);
    state.destroy();

    result.unwrap();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run() {
    use lum::webgpu::render::RendererWgpu;
    use wasm_bindgen::prelude::*;

    console_error_panic_hook::set_once();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let window_attributes = Window::default_attributes().with_title("Lumal").with_maximized(true);

    #[allow(deprecated)] // cause winit is going crazy
    let window = event_loop.create_window(window_attributes).unwrap();

    let mut state = DemoState::<RendererWgpu>::new(window, &event_loop);

    state.load_scene();

    let result = event_loop.run_app(&mut state);
    state.destroy();

    result.unwrap();
}
