This is a complete* rewrite of Lum in Rust programming language

* but MagicaVoxel loader is still in C++ and accessed via Rust FFI (generated with bindgen)

### Prerequisites

- nightly Rust
- Vulkan SDK (at least, glslc)

### How to run demo


```bash
cargo run --release
```

note: release is not very optimized, and there are custom profiles available, with fastest one being `native`:
```bash
cargo build -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --profile native
```
adding `optimize_for_size` to `build-std-features` makes binary under a megabyte (on windows) 

I did it because i need to find out best language to build renderers in, if you have any suggestions, please let me know