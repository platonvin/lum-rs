#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(inherent_associated_types)]
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

// we import types directly so we can use them like BlockId
// it is more "correct" to do <Renderer as RendererInterface>::BlockId
// however, its basically unreadable
// use lum::internal_renderer::render_wgpu::render::RendererWebGPU as Renderer;
// use lum::internal_renderer::render_wgpu::types::*;
use lum::renderer::vulkan::render::RendererVulkan;
use lum::renderer::vulkan::types::*;
use lum::renderer::{
    load_interface::LoadInterface,
    render_interface::{FoliageDescriptionBuilder, RendererInterface},
    types::{u8vec3, uvec3, vec3, MeshTransform},
    Settings,
};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

// i hardcode it but you probably should use some sort of "Asset library" - hashmap of YourEntityTypeEnum -> LumMeshModel
// #[derive(Default)]
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
    fn new(lum: &mut RendererVulkan, grass: MeshFoliage) -> Self {
        let tank = lum.load_model("assets/tank_body.vox");
        Self {
            tank_body: tank,
            tank_head: lum.load_model("assets/tank_head.vox"),
            tank_rf_leg: lum.load_model("assets/tank_rf_lb_leg.vox"),
            tank_lb_leg: lum.load_model("assets/tank_rf_lb_leg.vox"),
            tank_lf_leg: lum.load_model("assets/tank_lf_rb_leg.vox"),
            tank_rb_leg: lum.load_model("assets/tank_lf_rb_leg.vox"),
            water: lum.load_liquid(69, 42),
            grass,
            smoke: lum.load_volumetric(1.0, 0.5, u8vec3::zero()),
        }
    }

    fn unload(self, lum: &mut RendererVulkan) {
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

struct AppState {
    // window: &'renderer Window,
    lum: RendererVulkan,
    meshes: AllMeshes,
    transforms: AllTransforms,
    about_to_close: bool,
}
impl AppState {
    type FoliageDescription = <RendererVulkan as RendererInterface>::FoliageDescription;

    fn new(window: Window, event_loop: &EventLoop<()>) -> Self {
        let settings = Settings {
            static_block_palette_size: 15,
            ..Settings::default()
        };

        let mut foliage_desc_builder =
            <RendererVulkan as RendererInterface>::FoliageDescriptionBuilder::new();
        let grass = foliage_desc_builder.load_foliage(Self::FoliageDescription {
            // code: shaders::get_wgsl("grass.vert").unwrap(),
            spirv_code: shaders::get_shader("grass.vert.spv").unwrap().to_vec(),
            vertices: 13,
            density: 100,
        });

        let mut lum = RendererVulkan::new(&settings, window, &foliage_desc_builder.build());

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
        lum.load_block(10, "assets/stone_brick_cracked.vox");
        lum.load_block(11, "assets/stone_pack.vox");
        lum.load_block(12, "assets/bark.vox");
        lum.load_block(13, "assets/wood.vox");
        lum.load_block(14, "assets/planks.vox");

        lum.renderer.update_block_palette_to_gpu();
        lum.renderer.update_material_palette_to_gpu();

        Self {
            // window,
            lum,
            meshes,
            transforms: Default::default(),
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

    pub fn load_scene(&mut self, vox_file: &str) -> io::Result<()> {
        let buffer = read_file_buffer(vox_file)?; // Read file into Vec<u8>

        if buffer.len() < std::mem::size_of::<uvec3>() {
            self.lum.renderer.origin_world.fill(0);
            return Ok(());
        }

        // Extract world size from header
        let stored_world_size = uvec3::from_slice(
            &buffer[..12]
                .chunks_exact(4)
                .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
                .collect::<Vec<_>>()[..],
        );

        let stored_world = &buffer[12..]; // Skip the header

        // Ensure we don't read past buffer bounds
        assert!(
            stored_world.len()
                >= stored_world_size.x as usize
                    * stored_world_size.y as usize
                    * stored_world_size.z as usize
                    * std::mem::size_of::<i16>()
        );

        let size2read =
            stored_world_size.map(|v| v.min(self.lum.renderer.origin_world.x_size as u32));

        for zz in 0..size2read.z {
            for yy in 0..size2read.y {
                for xx in 0..size2read.x {
                    let index = (xx
                        + stored_world_size.x * yy
                        + stored_world_size.x * stored_world_size.y * zz)
                        as usize;
                    let loaded_block = i16::from_le_bytes(
                        stored_world[index * 2..index * 2 + 2].try_into().unwrap(),
                    );

                    // Clamp and set block
                    self.lum.renderer.origin_world[(xx as usize, yy as usize, zz as usize)] =
                        loaded_block
                            .clamp(0 as i16, self.lum.renderer.static_block_palette_size as i16)
                            as BlockId;
                }
            }
        }

        Ok(())
    }

    pub fn render(&mut self) {
        self.transforms.tank_body.translation = vec3::new(13.1, 14.1, 3.1) * 16.0;

        self.lum.start_frame();

        self.lum.draw_world();
        self.lum.draw_model(&self.meshes.tank_body, &self.transforms.tank_body);

        // literally procedural grass placement every frame. You probably want to store it as entities in your own structures
        for xx in 4..20 {
            for yy in 4..20 {
                if (5..12).contains(&xx) && (6..16).contains(&yy) {
                    continue;
                };
                let pos = vec3::new(xx as f32 * 16.0, yy as f32 * 16.0, 16.0);
                self.lum.draw_foliage(&self.meshes.grass, &pos);
            }
        }

        // literally procedural water placement every frame. You probably want to store it as entities in your own structures
        for xx in 5..12 {
            for yy in 6..16 {
                let pos = vec3::new(xx as f32 * 16.0, yy as f32 * 16.0, 14.0);
                self.lum.draw_liquid(&self.meshes.water, &pos);
            }
        }

        // literally procedural smoke placement every frame. You probably want to store it as entities in your own structures
        for xx in 8..10 {
            for yy in 10..13 {
                let pos = vec3::new(xx as f32 * 16.0, yy as f32 * 16.0, 20.0);
                self.lum.draw_volumetric(&self.meshes.smoke, &pos);
            }
        }

        self.lum.prepare_frame();
        self.lum.end_frame();
    }
}

impl ApplicationHandler for AppState {
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
            flame::dump_html(&mut File::create("flame-graph.html").unwrap()).unwrap();
            _event_loop.exit();
        } else {
            flame::clear();
            self.render();
            // _event_loop.
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
fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let window_attributes = Window::default_attributes()
        .with_title("Lumal")
        .with_maximized(true)
        // .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
        ;
    #[allow(deprecated)] // cause winit is going crazy
    let window = event_loop.create_window(window_attributes).unwrap();

    let mut state = AppState::new(window, &event_loop);

    state.load_scene("assets/scene").unwrap();

    let result = event_loop.run_app(&mut state);
    state.destroy();

    result.unwrap();
    flame::dump_html(&mut File::create("flame-graph.html").unwrap()).unwrap();
}

// wtf did i put it into bottom?
fn read_file_buffer<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
