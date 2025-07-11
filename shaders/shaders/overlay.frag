#version 450

precision highp float;
precision highp int;

#extension GL_GOOGLE_include_directive : require
#include "common/ext.glsl"
#include "common/ubo.glsl"
#include "common/consts.glsl"

layout(location = 0) in mediump vec2 fragUV;
layout(location = 1) in mediump vec4 fragColor;

layout(set = 0, binding = 0) uniform sampler2D ui_elem_texture;

layout(location = 0) out vec4 outColor;

void main() {
    vec2 final_uv = vec2(fragUV.x, fragUV.y);
    vec4 sampledColor = texture(ui_elem_texture, final_uv); 

    //as stated in rmlui docs
    vec4 final_color  = sampledColor;
         final_color *= fragColor;
    
    outColor = final_color;
} 