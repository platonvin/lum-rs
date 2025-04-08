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

pub fn get_glsl(arg: &str) -> Option<&'static str> {
    todo!()
}
