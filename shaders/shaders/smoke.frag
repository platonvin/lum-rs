#version 450

precision highp float;
precision highp int;

//dont swap
#extension GL_GOOGLE_include_directive : require
#include "common/ext.glsl"
#include "common/ubo.glsl"
#include "common/consts.glsl"
#include "common/lib.glsl"

layout(location = 0) out vec4 smoke_color;
layout(input_attachment_index = 0, set = 0, binding = 1) uniform subpassInput smoke_depth_far;
layout(input_attachment_index = 0, set = 0, binding = 2) uniform subpassInput smoke_depth_near;
layout(set = 0, binding = 3) uniform sampler3D radianceCache;
layout(set = 0, binding = 4) uniform sampler3D noise;

#include "common/radiance.glsl"

float decode_depth(float d){
    return (d)*1000.0;
}
float load_depth_far(){
    return decode_depth(subpassLoad(smoke_depth_far).x);
    // return (subpassLoad(smoke_depth_far).x);
    // return subpassLoad(smoke_depth_far).x;
}
float load_depth_near(){
    return decode_depth(subpassLoad(smoke_depth_near).x);
    // return (subpassLoad(smoke_depth_near).x);
    // return subpassLoad(smoke_depth_far).x;
}

vec3 cameraRayDirPlane;
vec3 horizline;
vec3 vertiline;

vec2 rotate(vec2 v, float a) {
	float s = sin(a);
	float c = cos(a);
	mat2 m = mat2(c, s, -s, c);
	return m * v;
}

mat2 rotatem(float a) {
	float s = sin(a);
	float c = cos(a);
	mat2 m = mat2(c, s, -s, c);
	return m;
}

void main() {
    vec3 direction = (ubo.camdir.xyz);

    const float near = (load_depth_near());
    const float  far = (load_depth_far());

    const float diff = (far-near);

    const int max_steps = 8; //does not really matter
    const float step_size = diff/float(max_steps);

    //https://en.wikipedia.org/wiki/Beer%E2%80%93Lambert_law
    //I = I0 * exp(-K * L)
    //dI = -K*dL * I0 * exp(-K * L)
    //In+1 = (1-denisty_n*ΔL) * In

    float I0 = 1.0;
    float I = 1.0;

    vec3 position;
    const float treshhold = 0.7;
    const float multiplier = 1.7;

    float total_dencity = 0;

    // never do this
    // for(float fraction = near; fraction <= far; fraction+=step_size){ 
    
    float fraction = near;
    // [[loop]] 
    float time = ubo.timeseed;
    for(int i=0; i<max_steps; i++){
        fraction += step_size;
            vec2 clip_pos = gl_FragCoord.xy / ubo.frame_size * 2.0 - 1.0;
            position = get_origin_from_depth(fraction, clip_pos);
            vec3 voxel_pos = vec3(position);
            vec3 noise_clip_pos = voxel_pos / 32.0;
        vec4 noises;
                vec3 wind_direction = vec3(1,0,0);
                const mat2 wind_rotate = rotatem(1.6);
            //TODO: derivatives? dFxy possibly solves mem access 
            noises.x = texture(noise, noise_clip_pos/1.0 + wind_direction*time/3500.0).x;
                wind_direction.xy *= wind_rotate;
            noises.y = texture(noise, noise_clip_pos/2.1 + wind_direction*time/3000.0).y;
                wind_direction.xy *= wind_rotate;
            noises.z = texture(noise, noise_clip_pos/3.2 + wind_direction*time/2500.0).z;
                wind_direction.xy *= wind_rotate;
            noises.w = texture(noise, noise_clip_pos/4.3 + wind_direction*time/2000.0).w;

        float close_to_border = clamp(diff,0.1,16.0)/16.0;

        float dencity = (noises.x + noises.y + noises.z - noises.w / close_to_border) / 2.0 - treshhold;

        dencity = clamp(dencity, 0,treshhold) * multiplier;
        // dencity *= exp(-clamp(diff, 0, 100)/100.0);
        // dencity += clamp(diff/10,0,1)/20.0;
        I = (1.0 - dencity * step_size) * I;
        total_dencity += dencity * step_size;
    }

    //1-I because its inverted
    float smoke_opacity = 1.0 - I;
    
    //at point of leaving smoke
    //does not look realistic but fits engine blocky style

    // vec3 final_light = sample_radiance(position, direction);
    // smoke_color = vec4(encode_color(vec3(final_light)), smoke_opacity);

    smoke_color = vec4(encode_color(vec3(0.15)), smoke_opacity);
} 