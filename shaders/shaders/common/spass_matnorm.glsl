// subpass-loading related functions. Valid only in fragment (thus moved into separate file)

int load_mat_spass() {
    int mat = int((subpassLoad(matNorm).x));
    return mat;
}
vec3 load_norm_spass() {
    vec3 norm = (((subpassLoad(matNorm).gba / 255.0) * 2.0 - 1.0));
    return norm;
}