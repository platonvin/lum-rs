use crate::*;

#[repr(C)]
pub struct MapPushConstants {
    pub inverse_trans: Mat4,
    pub shift: I16Vec4,
}

#[spirv(compute(threads(4, 4, 4)))]
#[unsafe(no_mangle)]
pub fn map_comp(
    #[spirv(push_constant)] pco: &MapPushConstants,

    #[spirv(descriptor_set = 0, binding = 0)] blocks: &Image!(3D, format = r16i, sampled = false),
    #[spirv(descriptor_set = 0, binding = 1)] block_palette: &Image!(
        3D,
        format = r8ui,
        sampled = false
    ),
    #[spirv(descriptor_set = 1, binding = 0)] model_voxels: &Image!(
        3D,
        format = r8ui,
        sampled = false
    ),

    #[spirv(global_invocation_id)] global_invocation_id: UVec3,
) {
    let model_size: UVec3 = model_voxels.query_size();
    let shift = pco.shift.xyz();

    let iabsolute_but_relative_to_shift_voxel = global_invocation_id.as_ivec3();
    let absolute_but_relative_to_shift_voxel = iabsolute_but_relative_to_shift_voxel.as_vec3();

    let world_voxel = absolute_but_relative_to_shift_voxel + Vec3::splat(0.5) + shift.as_vec3();
    let iworld_voxel = world_voxel.as_ivec3();

    let model_voxel_transformed = (pco.inverse_trans * world_voxel.extend(1.0)).xyz();
    let imodel_voxel = model_voxel_transformed.as_ivec3();

    let itarget_block = iworld_voxel / 16;

    let target_block_in_palette = blocks.read(itarget_block) as i32;

    let target_palette_voxel = voxel_in_palette(iworld_voxel.xyz() % 16, target_block_in_palette);

    let voxel = model_voxels.read(imodel_voxel);

    if target_block_in_palette >= STATIC_BLOCK_COUNT {
        if voxel != 0 {
            unsafe {
                block_palette.write(target_palette_voxel, uvec4(voxel, 0, 0, 0));
            }
        }
    }
}
