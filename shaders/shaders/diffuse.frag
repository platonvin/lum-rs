#version 450 core

/*
shader to diffuse-color main frame
ambient + "radiant" diffuse + lightmaps
*/

precision highp int;
precision highp float;

#extension GL_GOOGLE_include_directive : require
#include "common/consts.glsl"
#include "common/ubo.glsl"
#include "common/lib.glsl"

layout(input_attachment_index = 0, set = 0, binding = 1) uniform usubpassInput matNorm;
layout(input_attachment_index = 1, set = 0, binding = 2) uniform subpassInput depthBuffer;
layout(set = 0, binding = 3) uniform sampler2D voxelPalette;
layout(set = 0, binding = 4) uniform sampler3D radianceCache;
layout(set = 0, binding = 5) uniform sampler2DShadow lightmap;

#include "common/radiance.glsl"
#include "common/material.glsl"
#include "common/spass_matnorm.glsl"
#include "common/spass_depth.glsl"

layout(location = 0) out vec4 frame_color;

float sample_lightmap_with_shift(int xx, int yy, vec2 base_uv, float test_depth) {
    vec2 pcfshift = vec2(1.0 / 1024.0);
    vec2 lighmap_shift = vec2(xx, yy) * pcfshift;

    float shadow = texture(lightmap, vec3(base_uv + lighmap_shift, test_depth)).r; //TODO PCF
    return shadow;
}

float sample_lightmap(vec3 world_pos, vec3 normal) {
    // float b = (float((dot(normal, ubo.globalLightDir.xyz) < 0.0))*2.0 - 1.0);
    vec3 biased_pos = world_pos;

    if (dot(normal, ubo.globalLightDir.xyz) > 0.0) {
        biased_pos -= normal * .9;
    } else {
        biased_pos += normal * .9;
    }

    vec3 light_clip = (ubo.lightmap_proj * vec4(biased_pos, 1)).xyz; //move up
    light_clip.z = 1 + light_clip.z;
    float world_depth = light_clip.z;

    // float bias = 0.0 * (float((dot(normal, ubo.globalLightDir.xyz) < 0.0))*2.0 - 1.0);

    vec2 light_uv = (light_clip.xy + 1.0) / 2.0;

    float total_light = 00;
    float total_weight = 00;

    vec2 pcfshift = vec2(1.0 / 1024.0);

    // const float PI = 3.15;
    // const int sample_count = 1; //in theory i should do smth with temporal accumulation
    // const float max_radius = 2.0 / 1000.0;
    // float angle = 00;
    // float normalized_radius = 00;
    // float norm_radius_step = 1.0 / float(sample_count);
    // [[unroll]]
    //this loop was not unrolled for whatever reason, so i did it manually
    // for(int xx=-1; xx<=+1; xx++){
    // for(int yy=-1; yy<=+1; yy++){
    //     // if((xx==00) || (yy==00)) continue;
    //     if(!((xx!=00) && (yy!=00))){
    //         vec2 lighmap_shift = vec2(xx, yy) * pcfshift;
    //         // float light_depth = texture(lightmap, vec3(light_uv + lighmap_shift, 0.0)).x; //TODO PCF

    //         float test = texture(lightmap, vec3(light_uv + lighmap_shift, world_depth)).r; //TODO PCF
    //         // float diff = abs(world_depth - light_depth);
    //         float weight = 1;
    //         total_light += test;
    //         total_weight += weight;
    //     }
    // }}

    total_light += sample_lightmap_with_shift(-1, 0, light_uv, world_depth);
    total_light += sample_lightmap_with_shift(0, 0, light_uv, world_depth);
    total_light += sample_lightmap_with_shift(1, 0, light_uv, world_depth);
    total_light += sample_lightmap_with_shift(0, -1, light_uv, world_depth);
    total_light += sample_lightmap_with_shift(0, 1, light_uv, world_depth);

    return ((total_light / 5.0)) * 0.15;
    // return 0.5;
}

void main(void) {
    vec3 final_color = vec3(0);

    const Material stored_mat = get_mat(load_mat_spass());
    const vec3 stored_accumulated_reflection = vec3(1);
    const vec3 stored_accumulated_light = vec3(0);
    const vec3 direction = ubo.camdir.xyz;

    vec2 clip_pos = gl_FragCoord.xy / ubo.frame_size * 2.0 - 1.0;
    const vec3 origin = get_origin_from_depth(load_depth_spass(), clip_pos);
    const vec3 stored_normal = load_norm_spass();

    vec3 incoming_light = sample_radiance(origin + stored_normal * 6.0);
    float sunlight = sample_lightmap(origin, stored_normal);

    final_color = (2.0 * incoming_light + stored_mat.emmitance + sunlight) * stored_mat.color;
    // final_color = vec3(sunlight);
    // final_color = origin / 1000.0;
    // final_color = vec3(clip_pos, 0.0);
    // final_color = vec3(load_depth_spass() / 1000.0);
    // final_color = vec3(1);

    frame_color = vec4(encode_color(final_color), 1);
}
