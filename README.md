[![CI](https://github.com/platonvin/lum-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/platonvin/lum-rs/actions/workflows/ci.yml)

### Lum
Fast voxel renderer for web and native.

Lum is not an extendable engine*, but a specialized rendering library. You should only use it if you want to build a voxel game that looks very close to what Lum has to offer.

\* I don't believe in engines that are extendable, fast, **and** simple 

### Prerequisites
- nightly Rust: for certain #![features]
- Vulkan drivers
- Vulkan SDK: glslc and validation layers (to build demo without validation layers enabled, glslc is sufficient)

### Usage
look at the [demo source code](example/demo/src/demo_lib.rs) and [documentation](https://platonvin.github.io/docs/lum.html)

### How to run example (demo)...

> Fun fact - Lum's demo fits on a floppy disk! (current Vulkan build - `cargo biv` - is 1.15 mb)

#### ...natively:

> You can also download pre-built binaries from the [releases](https://github.com/platonvin/lum-rs/releases)

`cargo 123`, where\
* **1** = `b` / `r` - build / build & run\
* **2** = `d` / `r` / `n` / `i` - dev / release(some optimizations) / native (all optimizations with SIMD) / distribution(all optimizations without SIMD) profile\
* **3** = `v` / `w` - Vulkan / WGPU backend\

example: `cargo brv` will `build` `release` `vulkan` demo 

#### ...in web:

> You can see it in action here: [Live Web Demo](https://platonvin.github.io/projects/lum.html)

You'll need to compile to WASM, generate JS bindings, and then serve the (demo) webpage

1) Build the WASM lib:

    ```bash
    cargo build -p demo --lib --target "wasm32-unknown-unknown" --features wgpu_backend --profile distribution
    ```

2) Generate JS bindings wasm-bindgen:

    ```bash
    wasm-bindgen ./target/wasm32-unknown-unknown/distribution/demo_lib.wasm --out-dir pkg --target web
    ```
3) (optional) Optimize the WASM:

    ```bash
    wasm-opt ./pkg/demo_lib_bg.wasm -O4 -o ./pkg/demo_lib_bg.wasm
    ```

4) Serve the demo webpage:\
Use any local HTTP server. For example, microserver (cargo install microserver):

    ```bash
    cd example
    microserver . -i ./index.html -p 8080
    ```


### Architecture
Lum is just a library. It does not handle animations, UI, input, networking, or anything else you might expect from a full-fledged game engine.

It was built around the idea that most resources are loaded at initialization. Modern game engines do the opposite, creating (loading) most resources at runtime, but this complicates things immensely and is a common source of in-game freezes (if not done properly. None of the mayor game engines do it properlye).

Runtime loading might make sense for some large games, but Lum targets smaller games with fewer assets - they all are expected to fit in memory (in GPU VRAM. And for RAM - even this page in your browser allocates more than lum demo).

Vulkan backend came first. It makes heavy use of per-drawcall push constants and frequent state changes, which are cheap in Vulkan.

WGPU backend had to be designed differently. Native push constants are not available on the web, and emulating them with dynamic-offset-buffers is a performance crime. This led to a divergence in rendering strategy:

* Vulkan: sorts by depth (since state changes are cheap).
* WGPU: sorts by state to batch draw calls (state changes are expensive).

Lum was originally [written in C++](https://github.com/platonvin/lum) and its structure still reflects that - the Rust code is not always idiomatic.

Look at the (ash/wgpu)/winit examples to understand setup code.
If you plan to target the web, start compiling to WASM as early as possible to catch any specific issues.

#### Asset pipeline
Lum operates on data in a specific memory format. The voxc crate is a tool to process MagicaVoxel (.vox) files into this format - mesh and repack voxels.

The demo embeds all assets directly into the binary. This simplifies things by removing I/O and makes the web possible. Since the philosophy is to load everything at init-time, there is no benefit in a file system, embedding is simply better.

#### Future plans
* transparency
* moving some hard-coded rendering constants into template arguments (this envolves really different code paths for some cases)
* built-in "sprite sheet animations (for 3D)" (maybe, there is no problem with creating a bunch of images and swapping)
* bindings for other languages (maybe)
* perfomance profiles and runtime quaility settings (including but not limited to [all optional:] actually directional lightmaps, variable sampling HBAO, extra accumulations and custom (material/normal aware) multisampling, screen-space reflections (they are implemented but lack quality in edge cases))
* ... and, ofcourse, bug fixes and perfomance improvements