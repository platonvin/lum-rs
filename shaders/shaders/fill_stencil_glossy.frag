#version 450 

#extension GL_GOOGLE_include_directive : require

layout(input_attachment_index = 0, set = 0, binding = 0) uniform usubpassInput matNorm;
layout(set = 0, binding = 1 ) uniform sampler2D voxelPalette;

// "culls" expensive glossy shader by marking pixels-to-process with stencil mask via small trick

#include "common/material.glsl"
#include "common/spass_matnorm.glsl"

void main() 
{
    float rough = get_mat(load_mat_spass()).roughness;

    if(rough > 0.5){
        discard;
    }
}