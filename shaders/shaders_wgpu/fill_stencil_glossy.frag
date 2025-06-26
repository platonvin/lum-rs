@group(0) @binding(0) var material_normal_texture: texture_2d<u32>;
@group(0) @binding(1) var voxel_palette_texture: texture_2d<f32>;

struct Material {
    color: vec3<f32>,
    emittance: f32,
    roughness: f32,
    diffuse_light: vec3<f32>,
};

fn load_material_id(fragment_coordinate_xy: vec2<i32>) -> i32 {
    let loaded_value = textureLoad(material_normal_texture, fragment_coordinate_xy, 0);
    let material_id = i32(loaded_value.x);
    return material_id;
}

fn get_material(voxel_id: i32) -> Material {
    var material: Material;

    material.color.r = textureLoad(voxel_palette_texture, vec2<i32>(0, voxel_id), 0).r;
    material.color.g = textureLoad(voxel_palette_texture, vec2<i32>(1, voxel_id), 0).r;
    material.color.b = textureLoad(voxel_palette_texture, vec2<i32>(2, voxel_id), 0).r;
    material.emittance = textureLoad(voxel_palette_texture, vec2<i32>(4, voxel_id), 0).r;
    material.roughness = textureLoad(voxel_palette_texture, vec2<i32>(5, voxel_id), 0).r;
    material.diffuse_light = vec3<f32>(0.0);

    return material;
}

@fragment
fn main(@builtin(position) fragment_coordinate: vec4<f32>) {
    let fragment_coordinate_xy = vec2<i32>(fragment_coordinate.xy);

    let roughness = get_material(load_material_id(fragment_coordinate_xy)).roughness;

    if (roughness > 0.5) {
        discard;
    }

    // If not discarded, stencil value is written.
    // We use it as cheap "culling" of expensive shader (GPUs have really fast hw stencil tests)
}   