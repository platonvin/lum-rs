#version 450 

// shader to find "far / near" bounds of volumetrics on the scene. 
// This means that any empty space between two volumetrics will be lost. 
// Also sets stencil values to 10 on rasterization (as "culling" optimization to not run expensive shader as much)

layout (location = 0) in float end_depth_in;

layout(location = 0) out float  far_depth_out;
layout(location = 1) out float near_depth_out;

#extension GL_GOOGLE_include_directive : require
#include "common/ext.glsl"

void main() {
    if(!gl_FrontFacing){
        gl_FragDepth = end_depth_in - 0.01;
    } else {
        gl_FragDepth = end_depth_in;
    }

    far_depth_out = end_depth_in;
    near_depth_out = end_depth_in;
}