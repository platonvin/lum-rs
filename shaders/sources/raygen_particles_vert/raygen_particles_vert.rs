#![no_std]
use common::*;

#[spirv(vertex)]
#[unsafe(no_mangle)]
pub fn main(
    #[spirv(vertex_index)] vertex_id: i32,
    pos_in: Vec3,
    _vel_in: Vec3,
    life_time_in: f32,
    mat_id_in: u32,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(descriptor_set = 0, binding = 1)] blocks: &Image!(3D, format = r16i, sampled = false),
    #[spirv(descriptor_set = 0, binding = 2)] block_palette: &Image!(
        3D,
        format = r8ui,
        sampled = false
    ),
    #[spirv(flat)] mat_out: &mut u32,
    #[spirv(position)] out_clip_pos: &mut Vec4,
) {
    let mut vertex = VERTICES[vertex_id as usize];
    let delta_time = 1.0f32 / 75.0;

    let world_pos = pos_in.extend(0.0);

    let mut clip_coords = (ubo.trans_w2s * world_pos).xyz();
    clip_coords.z = 1.0 + clip_coords.z;

    *out_clip_pos = clip_coords.extend(1.0);

    *mat_out = mat_id_in;
    let size = life_time_in / 14.0;

    if size > 0.15 {
        let target_voxel_in_world = pos_in.as_ivec3();
        let target_block_in_world = target_voxel_in_world / 16;

        let target_block_id = blocks.read(target_block_in_world) as i32;

        let target_voxel_in_palette_coords =
            voxel_in_palette(target_voxel_in_world % 16, target_block_id);
        if target_block_id >= STATIC_BLOCK_COUNT {
            unsafe {
                block_palette.write(
                    target_voxel_in_palette_coords,
                    UVec4::new(mat_id_in, 0, 0, 0),
                );
            }
        }
    }
}
