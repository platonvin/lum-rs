//! # Lum shaders
//!
//! This crate embeds (compiled GLSL into) SPIR-V and WGSL shaders at build time
//! and provides a semi-type-safe enum `Shader` to access them.
//!
//! The appropriate `get_spirv()` or `get_wgsl()` method will be available
//! based on the active feature flags (`vk_backend` or `wgpu_backend`).

// Include the `shader_enums.rs` file that `build.rs` generates in `$OUT_DIR`.
// This module will contain the `Shader` enum and its implementations.
include!(concat!(env!("OUT_DIR"), "/shader_enums.rs"));
