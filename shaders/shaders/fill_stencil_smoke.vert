#version 450

precision highp float;

// shader to generate cube for smoke bounding box

#extension GL_GOOGLE_include_directive : require
#include "common/ext.glsl"
#include "common/ubo.glsl"

layout (location = 0) out float end_depth;

layout(push_constant) uniform restrict constants {
    vec4 originSize;
} pco;

const vec3 CUBE_VERTICES[36] = vec3[](
    vec3(0.0, 1.0, 1.0), vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 0.0),
    vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), vec3(0.0, 1.0, 1.0),

    vec3(1.0, 0.0, 0.0), vec3(1.0, 1.0, 0.0), vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0), vec3(1.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0),

    vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(1.0, 0.0, 1.0),
    vec3(1.0, 0.0, 1.0), vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, 0.0),

    vec3(1.0, 1.0, 1.0), vec3(1.0, 1.0, 0.0), vec3(0.0, 1.0, 0.0),
    vec3(0.0, 1.0, 0.0), vec3(0.0, 1.0, 1.0), vec3(1.0, 1.0, 1.0),

    vec3(1.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 0.0),
    vec3(0.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(1.0, 1.0, 0.0),

    vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 1.0), vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0), vec3(0.0, 1.0, 1.0), vec3(0.0, 0.0, 1.0)
);


void main() {
    vec3 vertex = CUBE_VERTICES[gl_VertexIndex];

    vec3 scaled_vertex = vertex * pco.originSize.w;

    vec4 world_pos = vec4(scaled_vertex + pco.originSize.xyz, 1.0);
    vec4 clip_pos = ubo.trans_w2s * world_pos;
         clip_pos.z = 1.0 + clip_pos.z;

    end_depth = clip_pos.z;
    
    gl_Position = clip_pos;
}
