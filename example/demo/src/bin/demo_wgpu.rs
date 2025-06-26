#[cfg(not(target_arch = "wasm32"))]
use demo_lib::WorldSize;

fn main() {
    // moved into lib cause shared code & needs to be compilable into web

    // I have no fucking idea why is [bin] compiled by trunk
    #[cfg(not(target_arch = "wasm32"))]
    demo_lib::run::<lum::webgpu::render::RendererWgpu<WorldSize>>();
}
