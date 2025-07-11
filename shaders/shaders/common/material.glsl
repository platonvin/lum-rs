struct Material {
    vec3 color;
    float emmitance;
    float roughness;
};

Material get_mat(in int voxel) {
    Material mat;

    mat.color.r = texelFetch(voxelPalette, ivec2(0, voxel), 0).r;
    mat.color.g = texelFetch(voxelPalette, ivec2(1, voxel), 0).r;
    mat.color.b = texelFetch(voxelPalette, ivec2(2, voxel), 0).r;
    // mat.transparancy = 1.0 - texelFetch(voxelPalette, ivec2(3,voxel), 0).r; // 
    mat.emmitance = texelFetch(voxelPalette, ivec2(4, voxel), 0).r;
    mat.roughness = texelFetch(voxelPalette, ivec2(5, voxel), 0).r;

    return mat;
}