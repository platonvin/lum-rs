pub fn get_shader(name: &str) -> Option<&'static [u8]> {
    // Note: `include_bytes!` happens at compile time. We need to know the exact paths
    // If anyone knows a good way to bundle files into a binary and put in target, let me know
    match name {
        "diffuse.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "diffuse.frag.spv"
        ))),
        "fillStencilGlossy.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "fillStencilGlossy.frag.spv"
        ))),
        "fillStencilSmoke.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "fillStencilSmoke.frag.spv"
        ))),
        "fillStencilSmoke.vert.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "fillStencilSmoke.vert.spv"
        ))),
        "fullscreenTriag.vert.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "fullscreenTriag.vert.spv"
        ))),
        "glossy.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "glossy.frag.spv"
        ))),
        "grass.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "grass.frag.spv"
        ))),
        "grass.vert.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "grass.vert.spv"
        ))),
        "hbao.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "hbao.frag.spv"
        ))),
        "lightmapBlocks.vert.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "lightmapBlocks.vert.spv"
        ))),
        "lightmapModels.vert.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "lightmapModels.vert.spv"
        ))),
        "map.comp.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "map.comp.spv"
        ))),
        "overlay.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "overlay.frag.spv"
        ))),
        "overlay.vert.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "overlay.vert.spv"
        ))),
        "perlin2.comp.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "perlin2.comp.spv"
        ))),
        "perlin3.comp.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "perlin3.comp.spv"
        ))),
        "radiance.comp.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "radiance.comp.spv"
        ))),
        "rayGenBlocks.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenBlocks.frag.spv"
        ))),
        "rayGenBlocks.vert.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenBlocks.vert.spv"
        ))),
        "rayGenModels.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenModels.frag.spv"
        ))),
        "rayGenModels.vert.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenModels.vert.spv"
        ))),
        "rayGenParticles.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenParticles.frag.spv"
        ))),
        "rayGenParticles.geom.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenParticles.geom.spv"
        ))),
        "rayGenParticles.vert.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenParticles.vert.spv"
        ))),
        "smoke.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "smoke.frag.spv"
        ))),
        "tonemap.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "tonemap.frag.spv"
        ))),
        "updateGrass.comp.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "updateGrass.comp.spv"
        ))),
        "updateWater.comp.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "updateWater.comp.spv"
        ))),
        "water.frag.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "water.frag.spv"
        ))),
        "water.vert.spv" => Some(include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "water.vert.spv"
        ))),
        _ => None,
    }
}

// pub fn get_glsl(name: &str) -> Option<&'static str> {
//     match name {
//         "diffuse.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "diffuse.frag.glsl"
//         ))),
//         "fillStencilGlossy.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "fillStencilGlossy.frag.glsl"
//         ))),
//         "fillStencilSmoke.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "fillStencilSmoke.frag.glsl"
//         ))),
//         "fillStencilSmoke.vert.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "fillStencilSmoke.vert.glsl"
//         ))),
//         "fullscreenTriag.vert.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "fullscreenTriag.vert.glsl"
//         ))),
//         "glossy.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "glossy.frag.glsl"
//         ))),
//         "grass.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "grass.frag.glsl"
//         ))),
//         "grass.vert.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "grass.vert.glsl"
//         ))),
//         "hbao.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "hbao.frag.glsl"
//         ))),
//         "lightmapBlocks.vert.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "lightmapBlocks.vert.glsl"
//         ))),
//         "lightmapModels.vert.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "lightmapModels.vert.glsl"
//         ))),
//         "map.comp.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "map.comp.glsl"
//         ))),
//         "overlay.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "overlay.frag.glsl"
//         ))),
//         "overlay.vert.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "overlay.vert.glsl"
//         ))),
//         "perlin2.comp.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "perlin2.comp.glsl"
//         ))),
//         "perlin3.comp.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "perlin3.comp.glsl"
//         ))),
//         "radiance.comp.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "radiance.comp.glsl"
//         ))),
//         "rayGenBlocks.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "rayGenBlocks.frag.glsl"
//         ))),
//         "rayGenBlocks.vert.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "rayGenBlocks.vert.glsl"
//         ))),
//         "rayGenModels.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "rayGenModels.frag.glsl"
//         ))),
//         "rayGenModels.vert.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "rayGenModels.vert.glsl"
//         ))),
//         "rayGenParticles.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "rayGenParticles.frag.glsl"
//         ))),
//         "rayGenParticles.geom.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "rayGenParticles.geom.glsl"
//         ))),
//         "rayGenParticles.vert.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "rayGenParticles.vert.glsl"
//         ))),
//         "smoke.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "smoke.frag.glsl"
//         ))),
//         "tonemap.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "tonemap.frag.glsl"
//         ))),
//         "updateGrass.comp.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "updateGrass.comp.glsl"
//         ))),
//         "updateWater.comp.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "updateWater.comp.glsl"
//         ))),
//         "water.frag.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "water.frag.glsl"
//         ))),
//         "water.vert.glsl" => Some(include_str!(concat!(
//             env!("COMPILED_SHADERS_PATH"),
//             "water.vert.glsl"
//         ))),
//         _ => None,
//     }
// }

// #[cfg(feature = "backend_opengl")]
// pub fn get_glsl(name: &str) -> Option<&'static str> {
//     match name {
//         "diffuse.frag" => Some(include_str!("../shaders_gl/diffuse.frag")),
//         "fillStencilGlossy.frag" => Some(include_str!("../shaders_gl/fillStencilGlossy.frag")),
//         "fillStencilSmoke.frag" => Some(include_str!("../shaders_gl/fillStencilSmoke.frag")),
//         "fillStencilSmoke.vert" => Some(include_str!("../shaders_gl/fillStencilSmoke.vert")),
//         "fullscreenTriag.vert" => Some(include_str!("../shaders_gl/fullscreenTriag.vert")),
//         "glossy.frag" => Some(include_str!("../shaders_gl/glossy.frag")),
//         "grass.frag" => Some(include_str!("../shaders_gl/grass.frag")),
//         "grass.vert" => Some(include_str!("../shaders_gl/grass.vert")),
//         "hbao.frag" => Some(include_str!("../shaders_gl/hbao.frag")),
//         "lightmapBlocks.vert" => Some(include_str!("../shaders_gl/lightmapBlocks.vert")),
//         "lightmapModels.vert" => Some(include_str!("../shaders_gl/lightmapModels.vert")),
//         "map.comp" => Some(include_str!("../shaders_gl/map.comp")),
//         "overlay.frag" => Some(include_str!("../shaders_gl/overlay.frag")),
//         "overlay.vert" => Some(include_str!("../shaders_gl/overlay.vert")),
//         "perlin2.comp" => Some(include_str!("../shaders_gl/perlin2.comp")),
//         "perlin3.comp" => Some(include_str!("../shaders_gl/perlin3.comp")),
//         "radiance.comp" => Some(include_str!("../shaders_gl/radiance.comp")),
//         "rayGenBlocks.frag" => Some(include_str!("../shaders_gl/rayGenBlocks.frag")),
//         "rayGenBlocks.vert" => Some(include_str!("../shaders_gl/rayGenBlocks.vert")),
//         "rayGenModels.frag" => Some(include_str!("../shaders_gl/rayGenModels.frag")),
//         "rayGenModels.vert" => Some(include_str!("../shaders_gl/rayGenModels.vert")),
//         "rayGenParticles.frag" => Some(include_str!("../shaders_gl/rayGenParticles.frag")),
//         "rayGenParticles.geom" => Some(include_str!("../shaders_gl/rayGenParticles.geom")),
//         "rayGenParticles.vert" => Some(include_str!("../shaders_gl/rayGenParticles.vert")),
//         "smoke.frag" => Some(include_str!("../shaders_gl/smoke.frag")),
//         "tonemap.frag" => Some(include_str!("../shaders_gl/tonemap.frag")),
//         "updateGrass.comp" => Some(include_str!("../shaders_gl/updateGrass.comp")),
//         "updateWater.comp" => Some(include_str!("../shaders_gl/updateWater.comp")),
//         "water.frag" => Some(include_str!("../shaders_gl/water.frag")),
//         "water.vert" => Some(include_str!("../shaders_gl/water.vert")),
//         _ => None,
//     }
// }

pub fn get_wgsl(name: &str) -> Option<&'static str> {
    // unimplemented!();

    match name {
        "diffuse.frag" => Some(include_str!("../shaders_wgpu/diffuse.frag")),
        "fillStencilGlossy.frag" => Some(include_str!("../shaders_wgpu/fillStencilGlossy.frag")),
        "fillStencilSmoke.frag" => Some(include_str!("../shaders_wgpu/fillStencilSmoke.frag")),
        "fillStencilSmoke.vert" => Some(include_str!("../shaders_wgpu/fillStencilSmoke.vert")),
        "fullscreenTriag.vert" => Some(include_str!("../shaders_wgpu/fullscreenTriag.vert")),
        "glossy.frag" => Some(include_str!("../shaders_wgpu/glossy.frag")),
        "grass.frag" => Some(include_str!("../shaders_wgpu/grass.frag")),
        "grass.vert" => Some(include_str!("../shaders_wgpu/grass.vert")),
        "hbao.frag" => Some(include_str!("../shaders_wgpu/hbao.frag")),
        "lightmapBlocks.vert" => Some(include_str!("../shaders_wgpu/lightmapBlocks.vert")),
        "lightmapModels.vert" => Some(include_str!("../shaders_wgpu/lightmapModels.vert")),
        "map.comp" => Some(include_str!("../shaders_wgpu/map.comp")),
        // "overlay.frag" => Some(include_str!("../shaders_wgpu/overlay.frag")),
        // "overlay.vert" => Some(include_str!("../shaders_wgpu/overlay.vert")),
        "perlin2.comp" => Some(include_str!("../shaders_wgpu/perlin2.comp")),
        "perlin3.comp" => Some(include_str!("../shaders_wgpu/perlin3.comp")),
        "radiance.comp" => Some(include_str!("../shaders_wgpu/radiance.comp")),
        "rayGenBlocks.frag" => Some(include_str!("../shaders_wgpu/rayGenBlocks.frag")),
        "rayGenBlocks.vert" => Some(include_str!("../shaders_wgpu/rayGenBlocks.vert")),
        "rayGenModels.frag" => Some(include_str!("../shaders_wgpu/rayGenModels.frag")),
        "rayGenModels.vert" => Some(include_str!("../shaders_wgpu/rayGenModels.vert")),
        "rayGenParticles.frag" => Some(include_str!("../shaders_wgpu/rayGenParticles.frag")),
        "rayGenParticles.geom" => Some(include_str!("../shaders_wgpu/rayGenParticles.geom")),
        "rayGenParticles.vert" => Some(include_str!("../shaders_wgpu/rayGenParticles.vert")),
        "smoke.frag" => Some(include_str!("../shaders_wgpu/smoke.frag")),
        "tonemap.frag" => Some(include_str!("../shaders_wgpu/tonemap.frag")),
        "updateGrass.comp" => Some(include_str!("../shaders_wgpu/updateGrass.comp")),
        "updateWater.comp" => Some(include_str!("../shaders_wgpu/updateWater.comp")),
        "water.frag" => Some(include_str!("../shaders_wgpu/water.frag")),
        "water.vert" => Some(include_str!("../shaders_wgpu/water.vert")),
        _ => None,
    }
}
