vec3 decode_color(vec3 encoded_color) {
    return encoded_color * COLOR_ENCODE_VALUE;
}
vec3 encode_color(vec3 color) {
    return color / COLOR_ENCODE_VALUE;
}

//not really nextafter but somewhat close
float next_after(float x, int s) {
    int ix = floatBitsToInt(x);
    float fxp1 = intBitsToFloat(ix + s);
    return fxp1;
}
float next_after(float x) {
    return next_after(x, 1);
}
float prev_befor(float x, int s) {
    int ix = floatBitsToInt(x);
    float fxp1 = intBitsToFloat(ix - s);
    return fxp1;
}
float prev_befor(float x) {
    return prev_befor(x, 1);
}

struct Material {
    vec3 color;
    float emmitance;
    float roughness;
    // float transparancy;
};

Material get_mat(in int voxel, in sampler2D voxel_palette) {
    Material mat;

    mat.color.r = texelFetch(voxel_palette, ivec2(0, voxel), 0).r;
    mat.color.g = texelFetch(voxel_palette, ivec2(1, voxel), 0).r;
    mat.color.b = texelFetch(voxel_palette, ivec2(2, voxel), 0).r;
    // mat.transparancy = 1.0 - texelFetch(voxel_palette, ivec2(3,voxel), 0).r; // 
    mat.emmitance = texelFetch(voxel_palette, ivec2(4, voxel), 0).r;
    mat.roughness = texelFetch(voxel_palette, ivec2(5, voxel), 0).r;

    return mat;
}

float square(float a) {
    return a * a;
}

vec3 load_probe(ivec3 probe_ipos, vec3 direction, readonly image3D _radiance_cache) {
    ivec3 probe_ipos_clamped = clamp(probe_ipos, ivec3(0), world_size);
    ivec3 subprobe_pos;
    subprobe_pos.x = probe_ipos_clamped.x; //same as local_pos actually but its optimized away and worth it for modularity
    subprobe_pos.yz = probe_ipos_clamped.yz; //reuses but its optimized away
    // vec3 light = imageLoad(radianceCache, (subprobe_pos)).xyz;
    vec3 light = imageLoad(_radiance_cache, (subprobe_pos)).rgb;
    return clamp(light, 0, 2);
}

vec3 fetch_probe(ivec3 probe_ipos, vec3 direction, sampler3D _radiance_cache) {
    ivec3 probe_ipos_clamped = clamp(probe_ipos, ivec3(0), world_size);
    ivec3 subprobe_pos;
    subprobe_pos.x = probe_ipos_clamped.x; //same as local_pos actually but its optimized away and worth it for modularity
    subprobe_pos.yz = probe_ipos_clamped.yz; //reuses but its optimized away
    // vec3 light = imageLoad(radianceCache, (subprobe_pos)).xyz;
    vec3 light = texelFetch(_radiance_cache, (subprobe_pos), 0).rgb;
    return clamp(light, 0, 2);
}

vec3 texture_radiance(vec3 position, vec3 normal, sampler3D _radiance_cache){
    vec3 block_pos = (position+normal*16.0) / 16.0;
    vec3 sampled_light = textureLod(_radiance_cache, (block_pos+0.5) / vec3(world_size), 0).rgb;
    return sampled_light;
}

vec3 sample_radiance(vec3 position, vec3 normal, sampler3D _radiance_cache) {
    vec3 sampled_light;

    float total_weight = 0;
    vec3 total_colour = vec3(0);

    ivec3 zero_probe_ipos = clamp(ivec3(floor(position - 8.0)) / 16, ivec3(0), world_size);
    vec3 zero_probe_pos = vec3(zero_probe_ipos) * 16.0 + 8.0;

    vec3 alpha = clamp((position - zero_probe_pos) / 16.0, 0, 1);
    // alpha = vec3(1);

    for (int i = 0; i < 8; i++) {
        //to make it little more readable
        ivec3 offset = ivec3(i, i >> 1, i >> 2) & ivec3(1);

        float probe_weight = (1);
        vec3 probe_colour = vec3(0);

        vec3 probe_pos = zero_probe_pos + vec3(offset) * 16.0;

        vec3 probeToPoint = probe_pos - position;
        vec3 direction_to_probe = normalize(probeToPoint);

        vec3 trilinear = mix(1.0 - alpha, alpha, vec3(offset));
        probe_weight = trilinear.x * trilinear.y * trilinear.z;

        /*
                                actually, not using directional weight **might** increase quality
                                by adding extra shadows in corners made of solid blocks
                                but im still going to use it

                                0.1 clamp to prevent weird cases where occasionally every single one would be 0 - in such cases, it will lead to trilinear
                                */
        float direction_weight = clamp(dot(direction_to_probe, normal), 0.1, 1);
        // float direction_weight = square(max(0.0001, (dot(direction_to_probe, normal) + 1.0) * 0.5)) + 0.2;
        // float direction_weight = float(dot(direction_to_probe, normal) > 0);

        probe_weight *= direction_weight;

        // const float crushThreshold = 0.2;
        // if (probe_weight < crushThreshold) {
        //     probe_weight *= probe_weight * probe_weight * (1.0 / square(crushThreshold));
        // }

        probe_colour = fetch_probe(zero_probe_ipos + offset, direction_to_probe, _radiance_cache);
        // probe_colour = vec3(zero_probe_ipos + offset) / vec3(world_size);

        probe_weight = max(1e-7, probe_weight);
        total_weight += probe_weight;
        total_colour += probe_weight * probe_colour;
    }

    return total_colour / total_weight;
}

vec3 sample_radiance(vec3 position, sampler3D _radiance_cache) {
    vec3 block_pos = position / 16.0;

    vec3 uv = block_pos / vec3(world_size);
    vec3 sampled_light = textureLod(_radiance_cache, uv, 0).rgb;
    return sampled_light;
}