# Lum
Voxel renderer

### Prerequisites

- nightly Rust
- Vulkan SDK (at least, glslc and validation layers for debug)

### How to run demo


```bash
cargo run --release
```

note: release is not very optimized, and there are custom profiles available, with fastest one being `native`:
```bash
cargo build -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --profile native
```
adding `optimize_for_size` to `build-std-features` makes binary under a megabyte 

I did it because i need to find out the best language for renderers. If you have any suggestions, please let me know

### Limitation
apart from hardware limitations (which can be bypassed, but at a cost of complexity), there are arbitrary limitations like 255 materials max. They can be manually edited and will be moved to template parameters in future (shaders can be aware of them via specialization constants)

install rust
install glslc TODO detect
install Vulkan

push constants in wgpu are the last binding always for wgpu
push dsets for vk is set 1 binding 0
dynamic binds for wgpu are set 1 binding 0

Vulkan (backend) sorts by depth cause state change is fast
wgpu (backend) sorts by state cause state change is slow (no push constants in web) 
    push constants are now emulated with batched drawcalls, and pc buffer binding must be the same (set 1 bind 0)


instructions on instalation

// RIGHT HANDED MATH EVERYWHERE

demo is library for web (no bin in web)

All types like Voxe, MatId, BlockId... are CPU-side and shared across backends. Types that start with Internal are per-backend (and do not match quite often)

Lum operates on memory, not files, and it expects your assets to have special format (in memory). However, you dont have to "compile" assets yourselves - Lum has built-in tools for creating assets from magicavoxel (good voxel editor with) format (.vox) - both meshing and repacking

frontend types are different from backend but not for vulkan (so wgpu needs conversions, but its good cause compression)

//! I highly recommend looking into winit/wgpu examples
//! Also, if you are going to compile your project into web, its better to start doing so as soon as possible