// subpass-loading related functions. Valid only in fragment (thus moved into separate file)

vec3 load_norm_spass(usubpassInput _mat_norm) {
    vec3 norm = (((subpassLoad(_mat_norm).gba / 255.0) * 2.0 - 1.0));
    return norm;
}
int load_mat_spass(usubpassInput _mat_norm) {
    int mat = int((subpassLoad(_mat_norm).x));
    return mat;
}
highp float load_depth_spass(subpassInput _depth_buffer) {
    highp float depth_encoded = (subpassLoad(_depth_buffer).x);
    return (depth_encoded) * 1000.0;
}