CPU:
 - RUST:
    - C-like Rust
    - less traits
    - SIMD friendly
    - unsafe is fine
    - macros are fine
 - C/C++ is fine, but should be build by build.rs
    - verify LLVM lto

RIGHT HANDED MATH EVERYWHERE

GPU:
 - use glsl
 - compile in build time
 - measure performance

RUST-SPECIAL:
 - cmd_smth is vulkan command buffer command
 - BufferPatch is used for VkCmdUpdateBuffer
 - PushConstant is used for VkCmdPushConstants