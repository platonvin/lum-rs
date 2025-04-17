// fillStencilGlossy.frag (WGSL)

// --- Bindings ---
// Input Attachment treated as Texture
@group(0) @binding(0) var matNorm_tex: texture_2d<f32>; // usubpassInput matNorm

// Voxel Palette Texture & Sampler
@group(0) @binding(1) var voxelPalette_tex: texture_2d<f32>; // sampler2D voxelPalette (Assuming R32Float format)
// @group(0) @binding(2) var nearest_samp: sampler; // Sampler - Not needed if only using textureLoad

// --- Structs ---
struct Material {
    color: vec3<f32>,
    emmitance: f32,
    diffuse_light: vec3<f32>, // Matches GLSL struct, though unused
    roughness: f32,
};

// --- Helper Functions ---
fn load_mat(frag_coord_xy: vec2<i32>) -> i32 {
    // Load from texture bound at binding 0
    let loaded_val = textureLoad(matNorm_tex, frag_coord_xy, 0); // LOD 0
    // Round and cast to i32
    let mat = i32(round(loaded_val.x));
    return mat;
}

fn GetMat(voxel: i32) -> Material {
    var mat: Material;
    // Use textureLoad with integer coordinates
    mat.color.r = textureLoad(voxelPalette_tex, vec2<i32>(0, voxel), 0).r;
    mat.color.g = textureLoad(voxelPalette_tex, vec2<i32>(1, voxel), 0).r;
    mat.color.b = textureLoad(voxelPalette_tex, vec2<i32>(2, voxel), 0).r;
    // mat.transparancy was commented out
    mat.emmitance = textureLoad(voxelPalette_tex, vec2<i32>(4, voxel), 0).r;
    mat.roughness = textureLoad(voxelPalette_tex, vec2<i32>(5, voxel), 0).r;
    mat.diffuse_light = vec3<f32>(0.0); // Initialize

    return mat;
}

// --- Fragment Entry Point ---
@fragment
fn main(@builtin(position) frag_coord: vec4<f32>) {
    let frag_coord_xy = vec2<i32>(frag_coord.xy); // Integer coordinates for textureLoad

    let rough = GetMat(load_mat(frag_coord_xy)).roughness;

    if (rough > 0.5) {
        discard; // Discard the fragment if roughness is high
    }
    // If not discarded, the pipeline's stencil state handles writing the stencil value.
    // No color output is needed for a stencil-only pass.
}