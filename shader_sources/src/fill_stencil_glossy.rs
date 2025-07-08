use crate::*;

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn fill_stencil_glossy_frag(
    #[spirv(descriptor_set = 0, binding = 0, input_attachment_index = 0)] mat_norm: &Image!(
         subpass,
         type = u32,
         sampled = false
     ),
    #[spirv(descriptor_set = 0, binding = 1)] voxel_palette: &Image!(2D, type=f32, sampled=false),
) {
    let rough = get_mat(load_mat(mat_norm), voxel_palette).roughness;

    if rough > 0.5 {
        spirv_std::arch::kill();
    }
}
