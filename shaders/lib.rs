// pub fn get_shader() -> &'static [u8] {
//     let bytes = include_bytes!(env!("SHADER_PATH"));
//     bytes
// }

include!(concat!(env!("OUT_DIR"), "/shaders.rs"));
