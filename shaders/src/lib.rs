pub fn get_shader(name: &str) -> Option<&'static [u8]> {
    // Note: `include_bytes!` happens at compile time. We need to know the exact paths
    // If anyone knows a good way to bundle files into a binary and put in target, let me know
    #[cfg(feature = "vk_backend")]
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
    #[cfg(not(feature = "vk_backend"))]
    unreachable!()
}

pub fn get_wgsl(name: &str) -> Option<&'static str> {
    #[cfg(feature = "wgpu_backend")]
    match name {
        "diffuse.frag" => Some(include_str!("../shaders_wgpu/diffuse.frag")),
        "fill_stencil_glossy.frag" => {
            Some(include_str!("../shaders_wgpu/fill_stencil_glossy.frag"))
        }
        "fill_stencil_smoke.frag" => Some(include_str!("../shaders_wgpu/fill_stencil_smoke.frag")),
        "fill_stencil_smoke.vert" => Some(include_str!("../shaders_wgpu/fill_stencil_smoke.vert")),
        "fullscreen_triag.vert" => Some(include_str!("../shaders_wgpu/fullscreen_triag.vert")),
        "glossy.frag" => Some(include_str!("../shaders_wgpu/glossy.frag")),
        "grass.frag" => Some(include_str!("../shaders_wgpu/grass.frag")),
        "grass.vert" => Some(include_str!("../shaders_wgpu/grass.vert")),
        "hbao.frag" => Some(include_str!("../shaders_wgpu/hbao.frag")),
        "lightmap_blocks.vert" => Some(include_str!("../shaders_wgpu/lightmap_blocks.vert")),
        "lightmap_models.vert" => Some(include_str!("../shaders_wgpu/lightmap_models.vert")),
        "map.comp" => Some(include_str!("../shaders_wgpu/map.comp")),
        "perlin2.comp" => Some(include_str!("../shaders_wgpu/perlin2.comp")),
        "perlin3.comp" => Some(include_str!("../shaders_wgpu/perlin3.comp")),
        "radiance.comp" => Some(include_str!("../shaders_wgpu/radiance.comp")),
        "raygen_blocks.frag" => Some(include_str!("../shaders_wgpu/raygen_blocks.frag")),
        "raygen_blocks.vert" => Some(include_str!("../shaders_wgpu/raygen_blocks.vert")),
        "raygen_models.frag" => Some(include_str!("../shaders_wgpu/raygen_models.frag")),
        "raygen_models.vert" => Some(include_str!("../shaders_wgpu/raygen_models.vert")),
        "raygen_particles.frag" => Some(include_str!("../shaders_wgpu/raygen_particles.frag")),
        "raygen_particles.vert" => Some(include_str!("../shaders_wgpu/raygen_particles.vert")),
        "smoke.frag" => Some(include_str!("../shaders_wgpu/smoke.frag")),
        "tonemap.frag" => Some(include_str!("../shaders_wgpu/tonemap.frag")),
        "update_grass.comp" => Some(include_str!("../shaders_wgpu/update_grass.comp")),
        "update_water.comp" => Some(include_str!("../shaders_wgpu/update_water.comp")),
        "water.frag" => Some(include_str!("../shaders_wgpu/water.frag")),
        "water.vert" => Some(include_str!("../shaders_wgpu/water.vert")),
        _ => None,
    }

    #[cfg(not(feature = "wgpu_backend"))]
    unreachable!();
}
