#![allow(dead_code)]
#![allow(unused_variables)]

use lum::{internal_renderer::Settings, renderer::Renderer, types::{u8vec3, BlockID_t, InternalMeshFoliage, InternalMeshLiquid, InternalMeshModel, InternalMeshVolumetric}};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

// i hardcode it but you probably should use some sort of "Asset library" - hashmap of YourEntityEnum -> LumMeshModel
// #[derive(Default)]
struct AllMeshes {
    tank_body: InternalMeshModel,
    tank_head: InternalMeshModel,
    tank_rf_leg: InternalMeshModel,
    tank_lb_leg: InternalMeshModel,
    tank_lf_leg: InternalMeshModel,
    tank_rb_leg: InternalMeshModel,
    water: InternalMeshLiquid,
    grass: InternalMeshFoliage,
    smoke: InternalMeshVolumetric,
}

impl AllMeshes {
    fn new(lum: &mut Renderer, grass: InternalMeshFoliage) -> Self {
        let tank = lum.load_model("assets/tank_body.vox");
        Self {
            tank_body: tank,
            tank_head: lum.load_model("assets/tank_head.vox"),
            tank_rf_leg: lum.load_model("assets/tank_rf_lb_leg.vox"),
            tank_lb_leg: lum.load_model("assets/tank_rf_lb_leg.vox"),
            tank_lf_leg: lum.load_model("assets/tank_lf_rb_leg.vox"),
            tank_rb_leg: lum.load_model("assets/tank_lf_rb_leg.vox"),
            water: lum.load_liquid(17, 16),
            grass : grass,
            smoke: lum.load_volumetric(0.5, 0.1, u8vec3::new(0, 0, 0)),
        }
    }

    fn unload(self, lum: &mut Renderer) {
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

// #[derive(Default)]
struct AppState {
    // event_loop: EventLoop<()>,
    window: Window,
    lum: lum::renderer::Renderer,
    meshes: AllMeshes,
}
impl AppState {
    fn new(mut window: Window) -> Self {
        let settings = Settings {
            static_block_palette_size: 15,
            ..Settings::default()
        };

        let mut pre_init_lum = Renderer::create().unwrap();
        let grass = pre_init_lum.load_foliage(
            // this is compiled by lum. But you should compile such shaders yourself
            "shaders/compiled/grass.vert.spv",
            13,
            100,
        );

        let mut lum = pre_init_lum.init(&settings, &mut window).unwrap(); 
        let meshes = AllMeshes::new(&mut lum, grass);

        lum.load_block(1, "assets/dirt.vox");
        lum.load_block(2, "assets/grass.vox");
        lum.load_block(3, "assets/grassNdirt.vox");
        lum.load_block(4, "assets/stone_dirt.vox");
        lum.load_block(5, "assets/bush.vox");
        lum.load_block(6, "assets/leaves.vox");
        lum.load_block(7, "assets/iron.vox");
        lum.load_block(8, "assets/lamp.vox");
        lum.load_block(9, "assets/stone_brick.vox");
        lum.load_block(10,"assets/stone_brick_cracked.vox");
        lum.load_block(11,"assets/stone_pack.vox");
        lum.load_block(12,"assets/bark.vox");
        lum.load_block(13,"assets/wood.vox");
        lum.load_block(14,"assets/planks.vox");

        lum.renderer.update_block_palette_to_gpu();
        lum.renderer.update_material_palette_to_gpu();
        
        Self {
            window,
            lum,
            meshes,
        }
    }

    pub fn destroy(mut self) {
        println!("Shutting down renderer");
        self.meshes.unload(&mut self.lum);
        self.lum.destroy();
    }

    pub fn load_scene(&mut self, scene_file: &str) {
        for zz in 0..self.lum.renderer.settings.world_size.z { 
        for yy in 0..self.lum.renderer.settings.world_size.y {
        for xx in 0..self.lum.renderer.settings.world_size.x {
            let is_floor = zz <= 1;
            if is_floor {
                let block = &mut self.lum.renderer.origin_world[(xx as usize, yy as usize, zz as usize)];
                // why the hell when i cast it to 16 breaks
                // reminder math was a mistake
                let mut bid = rand::random::<u16>() as u16 % 15;
                // bid = 5;
                *block = bid as i16;
            }
        }}}
    }
    
    pub fn render(&mut self) {
        self.lum.start_frame();
        self.lum.draw_world();
        self.lum.prepare_frame();
        self.lum.end_frame();
    }
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Resumed")
    }
    
    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        if matches!(event, WindowEvent::CloseRequested) {
            _event_loop.exit();
        }
    }
    
    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        // println!("Device event {:?}", event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.render();
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let window_attributes = Window::default_attributes()
        .with_title("Lumal")
        // .with_maximized(true)
        ;
    #[allow(deprecated)] // cause winit is going crazy
    let window = event_loop.create_window(window_attributes).unwrap();
    let mut state = AppState::new(window);
    state.load_scene("assets/scene");
    let result = event_loop.run_app(&mut state);
    state.destroy();
    result.unwrap();
}