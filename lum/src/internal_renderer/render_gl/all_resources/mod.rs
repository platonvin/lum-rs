pub mod all_buffers;
pub mod all_images;
// we still have idea of pipes, however, they only hold VAO, description of input vertex and ubo
pub mod all_pipes;
// we still have idea of renderpasses, but instead of Vulkan stateful objects they hold the necessary state to be set for OpenGL
pub mod all_rpasses;
pub mod all_samplers;
