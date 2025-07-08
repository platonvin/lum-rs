use crate::*;

#[repr(C)]
pub struct FillStencilSmokePushConstants {
    pub origin_size: Vec4,
}

#[spirv(vertex)]
#[unsafe(no_mangle)]
pub fn fill_stencil_smoke_vert(
    #[spirv(vertex_index)] vertex_id: u32,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(push_constant)] pco: &FillStencilSmokePushConstants,
    end_depth: &mut f32,
    #[spirv(position)] clip_pos: &mut Vec4,
) {
    let mut vertex = VERTICES[vertex_id as usize];

    vertex *= pco.origin_size.w * 1.0;

    let world_pos = (vertex + pco.origin_size.xyz()).extend(1.0);

    let mut clip_coords = (ubo.trans_w2s * world_pos).xyz();
    clip_coords.z = 1.0 + clip_coords.z;
    *end_depth = clip_coords.z;

    *clip_pos = clip_coords.extend(1.0);
}

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn fill_stencil_smoke_frag(
    end_depth_in: f32,
    far_depth_out: &mut f32,
    near_depth_out: &mut f32,
) {
    *far_depth_out = end_depth_in;
    *near_depth_out = end_depth_in;
}
