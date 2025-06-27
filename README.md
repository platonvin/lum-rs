[![CI](https://github.com/platonvin/lum-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/platonvin/lum-rs/actions/workflows/ci.yml)

### Lum
Fast voxel renderer for web and native.

Lum is not an extendable engine*, but a specialized rendering library. You should only use it if you want to build a voxel game that looks very close to what Lum has to offer.

\* I don't believe in engines that are extendable, fast, **and** simple 

### Prerequisites
- nightly Rust: for certain #![features]
- Vulkan drivers
- Vulkan SDK: glslc and validation layers

### Usage
look at the [demo source code](example/demo/src/demo_lib.rs) and [documentation](https://platonvin.github.io/docs/lum.html)

### How to run example (demo)...

> Fun fact - Lum's demo fits on a floppy disk! (current Vulkan build - `cargo biv` - is 1.36 mb)

#### ...natively:

> You can also download pre-built binaries from the [releases](https://github.com/platonvin/lum-rs/releases)

`cargo xyz`, where
- **X** = `b` / `r` - build / build & run
- **Y** = `d` / `r` / `n` / `i` - dev / release(some optimizations) / native (all optimizations with SIMD) / distribution(all optimizations without SIMD) profile
- **Z** = `v` / `w` - Vulkan / WGPU backend

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


### Philosophy
Lum is just a library. It does not handle animations, UI, input, networking, or anything else you might expect from a full-fledged game engine.

It was built around the idea that most resources are loaded at initialization. Modern game engines do the opposite, creating (loading) most resources at runtime, but this complicates things immensely and is a common source of in-game freezes (if not done properly. None of the mayor game engines do it properlye).

Runtime loading might make sense for some large games, but Lum targets smaller games with fewer assets - they all are expected to fit in memory. No

### Architecture
The Vulkan backend came first. It makes heavy use of per-drawcall push constants and frequent state changes, which are cheap in Vulkan.

The WGPU backend had to be designed differently. Native push constants are not available on the web, and emulating them with dynamic-offset-buffers is a performance crime. This led to a divergence in rendering strategy:

- Vulkan: sorts by depth (since state changes are cheap).
- WGPU: sorts by state to batch draw calls (since state changes are expensive).

Lum was originally written in C++ and its structure still reflects that - the Rust code is not always "idiomatic".

Lum has a lot of hard-coded constants, such as max of 255 materials. Most of them were chosen because they map well to certain formats / hw limits. These will be moved to generics or runtime settings in the future.

Look at the (ash/wgpu)/winit examples to understand setup code.
If you plan to target the web, start compiling to WASM as early as possible to catch any specific issues.

#### Asset Pipeline
Lum operates on data in a specific memory format. The voxc crate is a tool to process MagicaVoxel (.vox) files into this format - mesh and repack voxels.

The demo embeds all assets directly into the binary. This simplifies things by removing I/O and makes the web possible. Since the philosophy is to load everything at init-time, there is no benefit in a file system, embedding is simply better.