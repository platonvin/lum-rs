layout(binding = 0, set = 0) uniform restrict readonly UniformBufferObject {
    mat4 trans_w2s;
    vec4 campos;
    vec4 camdir;
    vec4 horizline_scaled;
    vec4 vertiline_scaled;
    vec4 globalLightDir;
    mat4 lightmap_proj;
    vec2 frame_size;
    int timeseed;
} ubo;

vec3 get_origin_from_depth(float depth, vec2 clip_pos) {
    vec3 origin = ubo.campos.xyz +
            (ubo.horizline_scaled.xyz * clip_pos.x) +
            (ubo.vertiline_scaled.xyz * clip_pos.y) +
            (ubo.camdir.xyz * depth);
    return origin;
}
vec3 get_origin_from_depth_interpolated(float depth, vec3 pos_interpol) {
    vec3 origin = pos_interpol + (ubo.camdir.xyz * depth);
    return origin;
}