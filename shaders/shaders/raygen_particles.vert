#version 450

precision highp float;
#extension GL_GOOGLE_include_directive : require
#include "common/ext.glsl"
#include "common/ubo.glsl"
#include "common/consts.glsl"

layout(location = 0) in  vec3 posIn;
layout(location = 1) in  vec3 velIn;
layout(location = 2) in float lifeTimeIn;
layout(location = 3) in  uint matIDIn;

layout(location = 0) lowp flat out uvec4 mat_norm;

layout(set = 0, binding = 1, r16i) uniform restrict  readonly iimage3D blocks;
layout(set = 0, binding = 2, r8ui) uniform restrict writeonly uimage3D blockPalette;

ivec3 voxel_in_palette(ivec3 relative_voxel_pos, int block_id) {
    int block_x = block_id % BLOCK_PALETTE_SIZE_X;
    int block_y = block_id / BLOCK_PALETTE_SIZE_X;

    return relative_voxel_pos + ivec3(16*block_x, 16*block_y, 0);
}

const vec3 CUBE_VERTICES[36] = vec3[](
    vec3(1, -1,  1), vec3(1, -1, -1), vec3(1,  1, -1),
    vec3(1, -1,  1), vec3(1,  1, -1), vec3(1,  1,  1),

    vec3(-1, -1, -1), vec3(-1, -1,  1), vec3(-1,  1,  1),
    vec3(-1, -1, -1), vec3(-1,  1,  1), vec3(-1,  1, -1),

    vec3(-1, 1,  1), vec3(1, 1,  1), vec3(1, 1, -1),
    vec3(-1, 1,  1), vec3(1, 1, -1), vec3(-1, 1, -1),

    vec3(-1, -1, -1), vec3(1, -1, -1), vec3(1, -1,  1),
    vec3(-1, -1, -1), vec3(1, -1,  1), vec3(-1, -1,  1),

    vec3(-1, -1, 1), vec3(-1,  1, 1), vec3(1,  1, 1),
    vec3(-1, -1, 1), vec3(1,  1, 1), vec3(1, -1, 1),

    vec3(1, -1, -1), vec3(1,  1, -1), vec3(-1,  1, -1),
    vec3(1, -1, -1), vec3(-1,  1, -1), vec3(-1, -1, -1)
);

const vec3 CUBE_NORMALS[36] = vec3[](
    vec3(1, 0, 0), vec3(1, 0, 0), vec3(1, 0, 0),
    vec3(1, 0, 0), vec3(1, 0, 0), vec3(1, 0, 0),

    vec3(-1, 0, 0), vec3(-1, 0, 0), vec3(-1, 0, 0),
    vec3(-1, 0, 0), vec3(-1, 0, 0), vec3(-1, 0, 0),

    vec3(0, 1, 0), vec3(0, 1, 0), vec3(0, 1, 0),
    vec3(0, 1, 0), vec3(0, 1, 0), vec3(0, 1, 0),

    vec3(0, -1, 0), vec3(0, -1, 0), vec3(0, -1, 0),
    vec3(0, -1, 0), vec3(0, -1, 0), vec3(0, -1, 0),

    vec3(0, 0, 1), vec3(0, 0, 1), vec3(0, 0, 1),
    vec3(0, 0, 1), vec3(0, 0, 1), vec3(0, 0, 1),

    vec3(0, 0, -1), vec3(0, 0, -1), vec3(0, 0, -1),
    vec3(0, 0, -1), vec3(0, 0, -1), vec3(0, 0, -1)
);

void main() {
    int vertex_in_cube_index = gl_VertexIndex;

    float deltaTime = 1.0/75.0;

    vec4 particle_world_pos = vec4(posIn, 1.0);

    float life_size = lifeTimeIn / 2.0;
    uint material_id = matIDIn;

    if (vertex_in_cube_index == 0) {
        if(life_size * 2.0 > .15){
            ivec3 target_voxel_in_world = ivec3(posIn);
            ivec3 target_block_in_world = target_voxel_in_world / 16;

            int target_block_id = imageLoad(blocks, target_block_in_world).x;
            ivec3 target_voxel_in_palette = voxel_in_palette(target_voxel_in_world % 16, target_block_id);
            if(target_block_id>=STATIC_BLOCK_COUNT){
                imageStore(blockPalette, target_voxel_in_palette, uvec4(matIDIn));
            }
        }
    }

    vec3 corner = CUBE_VERTICES[vertex_in_cube_index] * life_size;
    vec3 norm = CUBE_NORMALS[vertex_in_cube_index];

    vec4 world_pos = vec4(posIn + corner, 1.0);

    vec4 clip = ubo.trans_w2s * world_pos;

    clip.z = 1.0 + clip.z;

    gl_Position = clip;

    mat_norm = uvec4(
        material_id,
        uint((norm.x * 0.5 + 0.5) * 255.0),
        uint((norm.y * 0.5 + 0.5) * 255.0),
        uint((norm.z * 0.5 + 0.5) * 255.0)
    );
}
