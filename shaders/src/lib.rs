pub fn get_shader(name: &str) -> &'static [u8] {
    // Note: `include_bytes!` happens at compile time. We need to know the exact paths
    // If anyone knows a good way to bundle files into a binary and put in target, let me know
    #[cfg(feature = "vk_backend")]
    match name {
        "diffuse.frag.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "diffuse.frag.spv"))
        }
        "fillStencilGlossy.frag.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "fillStencilGlossy.frag.spv"
        )),
        "fillStencilSmoke.frag.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "fillStencilSmoke.frag.spv"
        )),
        "fillStencilSmoke.vert.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "fillStencilSmoke.vert.spv"
        )),
        "fullscreenTriag.vert.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "fullscreenTriag.vert.spv"
        )),
        "glossy.frag.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "glossy.frag.spv"))
        }
        "grass.frag.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "grass.frag.spv"))
        }
        "grass.vert.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "grass.vert.spv"))
        }
        "hbao.frag.spv" => include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "hbao.frag.spv")),
        "lightmapBlocks.vert.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "lightmapBlocks.vert.spv"
        )),
        "lightmapModels.vert.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "lightmapModels.vert.spv"
        )),
        "map.comp.spv" => include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "map.comp.spv")),
        "overlay.frag.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "overlay.frag.spv"))
        }
        "overlay.vert.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "overlay.vert.spv"))
        }
        "perlin2.comp.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "perlin2.comp.spv"))
        }
        "perlin3.comp.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "perlin3.comp.spv"))
        }
        "radiance.comp.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "radiance.comp.spv"))
        }
        "rayGenBlocks.frag.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenBlocks.frag.spv"
        )),
        "rayGenBlocks.vert.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenBlocks.vert.spv"
        )),
        "rayGenModels.frag.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenModels.frag.spv"
        )),
        "rayGenModels.vert.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenModels.vert.spv"
        )),
        "rayGenParticles.frag.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenParticles.frag.spv"
        )),
        "rayGenParticles.geom.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenParticles.geom.spv"
        )),
        "rayGenParticles.vert.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "rayGenParticles.vert.spv"
        )),
        "smoke.frag.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "smoke.frag.spv"))
        }
        "tonemap.frag.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "tonemap.frag.spv"))
        }
        "updateGrass.comp.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "updateGrass.comp.spv"
        )),
        "updateWater.comp.spv" => include_bytes!(concat!(
            env!("COMPILED_SHADERS_PATH"),
            "updateWater.comp.spv"
        )),
        "water.frag.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "water.frag.spv"))
        }
        "water.vert.spv" => {
            include_bytes!(concat!(env!("COMPILED_SHADERS_PATH"), "water.vert.spv"))
        }
        _ => panic!(),
    }
    #[cfg(not(feature = "vk_backend"))]
    unreachable!()
}

pub fn get_wgsl(name: &str) -> &'static str {
    #[cfg(feature = "wgpu_backend")]
    match name {
        "diffuse.frag" => include_str!("../shaders_wgpu/diffuse.frag"),
        "fill_stencil_glossy.frag" => {
            include_str!("../shaders_wgpu/fill_stencil_glossy.frag")
        }
        "fill_stencil_smoke.frag" => include_str!("../shaders_wgpu/fill_stencil_smoke.frag"),
        "fill_stencil_smoke.vert" => include_str!("../shaders_wgpu/fill_stencil_smoke.vert"),
        "fullscreen_triag.vert" => include_str!("../shaders_wgpu/fullscreen_triag.vert"),
        "glossy.frag" => include_str!("../shaders_wgpu/glossy.frag"),
        "grass.frag" => include_str!("../shaders_wgpu/grass.frag"),
        "grass.vert" => include_str!("../shaders_wgpu/grass.vert"),
        "hbao.frag" => include_str!("../shaders_wgpu/hbao.frag"),
        "lightmap_blocks.vert" => include_str!("../shaders_wgpu/lightmap_blocks.vert"),
        "lightmap_models.vert" => include_str!("../shaders_wgpu/lightmap_models.vert"),
        "map.comp" => include_str!("../shaders_wgpu/map.comp"),
        "perlin2.comp" => include_str!("../shaders_wgpu/perlin2.comp"),
        "perlin3.comp" => include_str!("../shaders_wgpu/perlin3.comp"),
        "radiance.comp" => include_str!("../shaders_wgpu/radiance.comp"),
        "raygen_blocks.frag" => include_str!("../shaders_wgpu/raygen_blocks.frag"),
        "raygen_blocks.vert" => include_str!("../shaders_wgpu/raygen_blocks.vert"),
        "raygen_models.frag" => include_str!("../shaders_wgpu/raygen_models.frag"),
        "raygen_models.vert" => include_str!("../shaders_wgpu/raygen_models.vert"),
        "raygen_particles.frag" => include_str!("../shaders_wgpu/raygen_particles.frag"),
        "raygen_particles.vert" => include_str!("../shaders_wgpu/raygen_particles.vert"),
        "smoke.frag" => include_str!("../shaders_wgpu/smoke.frag"),
        "tonemap.frag" => include_str!("../shaders_wgpu/tonemap.frag"),
        "update_grass.comp" => include_str!("../shaders_wgpu/update_grass.comp"),
        "update_water.comp" => include_str!("../shaders_wgpu/update_water.comp"),
        "water.frag" => include_str!("../shaders_wgpu/water.frag"),
        "water.vert" => include_str!("../shaders_wgpu/water.vert"),
        _ => unreachable!(),
    }

    #[cfg(not(feature = "wgpu_backend"))]
    unreachable!();
}
