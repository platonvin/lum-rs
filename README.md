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
install glslc
install Vulkan

push constants in wgpu are the last binding always for wgpu
push dsets for vk is set 1 binding 0
dynamic binds for wgpu are set 1 binding 0