This is a complete rewrite of Lum Renderer in Rust programming language

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