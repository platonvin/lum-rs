use std::mem::offset_of;

use crate::{
    internal_renderer::{
        render_gl::{
            pipe::{create_compute_pipe, create_raster_pipe, AttrFormOffs, RasterPipe},
            AllBuffers, AllIndependentImages, AllPipes, AllSamplers, AllSwapchainDependentImages,
            InternalRendererGL,
        },
        Settings,
    },
    types::*,
    *,
};
use glow::{Context, HasContext};

impl InternalRendererGL {
    #[cold]
    #[optimize(size)]
    pub unsafe fn create_all_pipes(
        gl: &Context,
        lum_settings: &Settings,
        buffers: &AllBuffers,
        iimages: &AllIndependentImages,
        dimages: &AllSwapchainDependentImages,
        samplers: &AllSamplers,
        foliage_descriptions: &[InternalMeshFoliageDesc],
    ) -> AllPipes {
        let vs_source = shaders::get_glsl("lightmapBlocks.vert").unwrap();
        let lightmap_blocks_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            None, // No fragment shader, depth-only pass
            &[AttrFormOffs {
                binding: 0,
                offset: offset_of!(PackedVoxelCircuit, pos) as i32,
                data_type: glow::UNSIGNED_BYTE,
                size: 3,
                stride: size_of::<PackedVoxelCircuit>() as i32,
                normalized: false,
            }],
        );

        let vs_source = shaders::get_glsl("lightmapModels.vert").unwrap();
        let lightmap_models_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            None,
            &[AttrFormOffs {
                binding: 0,
                offset: offset_of!(PackedVoxelCircuit, pos) as i32,
                data_type: glow::UNSIGNED_BYTE,
                size: 3,
                stride: size_of::<PackedVoxelCircuit>() as i32,
                normalized: false,
            }],
        );

        let vs_source = shaders::get_glsl("rayGenBlocks.vert").unwrap();
        let fs_source = shaders::get_glsl("rayGenBlocks.frag").unwrap();
        let raygen_blocks_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[AttrFormOffs {
                binding: 0,
                offset: offset_of!(PackedVoxelCircuit, pos) as i32,
                data_type: glow::UNSIGNED_BYTE,
                size: 3,
                stride: size_of::<PackedVoxelCircuit>() as i32,
                normalized: false,
            }],
        );

        let vs_source = shaders::get_glsl("rayGenModels.vert").unwrap();
        let fs_source = shaders::get_glsl("rayGenModels.frag").unwrap();
        let raygen_models_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[AttrFormOffs {
                binding: 0,
                offset: offset_of!(PackedVoxelCircuit, pos) as i32,
                data_type: glow::UNSIGNED_BYTE,
                size: 3,
                stride: size_of::<PackedVoxelCircuit>() as i32,
                normalized: false,
            }],
        );

        // fuck geometry shaders
        let vs_source = shaders::get_glsl("rayGenParticles.vert").unwrap();
        // let gs_source = shaders::get_glsl("rayGenParticles.geom").unwrap();
        let fs_source = shaders::get_glsl("rayGenParticles.frag").unwrap();
        let raygen_particles_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[
                AttrFormOffs {
                    binding: 0,
                    offset: offset_of!(Particle, pos) as i32,
                    data_type: glow::FLOAT,
                    size: 3,
                    stride: size_of::<Particle>() as i32,
                    normalized: false,
                },
                AttrFormOffs {
                    binding: 1,
                    offset: offset_of!(Particle, vel) as i32,
                    data_type: glow::FLOAT,
                    size: 3,
                    stride: size_of::<Particle>() as i32,
                    normalized: false,
                },
                AttrFormOffs {
                    binding: 2,
                    offset: offset_of!(Particle, life_time) as i32,
                    data_type: glow::FLOAT,
                    size: 1,
                    stride: size_of::<Particle>() as i32,
                    normalized: false,
                },
                AttrFormOffs {
                    binding: 3,
                    offset: offset_of!(Particle, mat_id) as i32,
                    data_type: glow::UNSIGNED_BYTE,
                    size: 1,
                    stride: size_of::<Particle>() as i32,
                    normalized: false,
                },
            ],
        );

        // Raygen Water Pipeline (Vertex and Fragment Shaders)
        let vs_source = shaders::get_glsl("water.vert").unwrap();
        let fs_source = shaders::get_glsl("water.frag").unwrap();
        let raygen_water_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[], // Generated vertices
        );

        let raygen_foliage_pipes = foliage_descriptions
            .iter()
            .map(|desc| {
                let vs_source = shaders::get_glsl("foliage.vert").unwrap();
                let fs_source = shaders::get_glsl("grass.frag").unwrap();
                let foliage_pipe = create_raster_pipe(
                    gl,
                    Some(&vs_source),
                    Some(&fs_source),
                    // &[AttrFormOffs {
                    //     binding: 0,
                    //     offset: offset_of!(PackedVoxelCircuit, pos) as i32,
                    //     data_type: glow::UNSIGNED_BYTE,
                    //     size: 3,
                    //     stride: size_of::<PackedVoxelCircuit>() as i32,
                    //     normalized: false,
                    // }],
                    &[],
                );
                foliage_pipe
            })
            .collect();

        let vs_source = shaders::get_glsl("fullscreenTriag.vert").unwrap();
        let fs_source = shaders::get_glsl("diffuse.frag").unwrap();
        let diffuse_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[], // Fullscreen pass, no attributes
        );

        let vs_source = shaders::get_glsl("fullscreenTriag.vert").unwrap();
        let fs_source = shaders::get_glsl("hbao.frag").unwrap();
        let ao_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[], // Fullscreen pass, no attributes
        );

        let vs_source = shaders::get_glsl("fullscreenTriag.vert").unwrap();
        let fs_source = shaders::get_glsl("fillStencilGlossy.frag").unwrap();
        let fill_stencil_glossy_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[], // Fullscreen pass, no attributes
        );

        let vs_source = shaders::get_glsl("fillStencilSmoke.vert").unwrap();
        let fs_source = shaders::get_glsl("fillStencilSmoke.frag").unwrap();
        let fill_stencil_smoke_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[], // Push constants only
        );

        let vs_source = shaders::get_glsl("fullscreenTriag.vert").unwrap();
        let fs_source = shaders::get_glsl("glossy.frag").unwrap();
        let glossy_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[], // Fullscreen pass, no attributes
        );

        let vs_source = shaders::get_glsl("fullscreenTriag.vert").unwrap();
        let fs_source = shaders::get_glsl("smoke.frag").unwrap();
        let smoke_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[], // Fullscreen pass, no attributes
        );

        let vs_source = shaders::get_glsl("fullscreenTriag.vert").unwrap();
        let fs_source = shaders::get_glsl("tonemap.frag").unwrap();
        let tonemap_pipe = create_raster_pipe(
            gl,
            Some(&vs_source),
            Some(&fs_source),
            &[], // Fullscreen pass, no attributes
        );

        let source = shaders::get_glsl("radiance.comp.spv").unwrap();
        let radiance_pipe = create_compute_pipe(gl, source);

        let source = shaders::get_glsl("updateGrass.comp.spv").unwrap();
        let update_grass_pipe = create_compute_pipe(gl, source);

        let source = shaders::get_glsl("updateWater.comp.spv").unwrap();
        let update_water_pipe = create_compute_pipe(gl, source);

        let source = shaders::get_glsl("perlin2.comp.spv").unwrap();
        let gen_perlin2d_pipe = create_compute_pipe(gl, source);

        let source = shaders::get_glsl("perlin3.comp.spv").unwrap();
        let gen_perlin3d_pipe = create_compute_pipe(gl, source);

        let source = shaders::get_glsl("map.comp.spv").unwrap();
        let map_pipe = create_compute_pipe(gl, source);

        AllPipes {
            lightmap_blocks_pipe,
            lightmap_models_pipe,
            raygen_blocks_pipe,
            raygen_models_pipe,
            raygen_particles_pipe,
            raygen_water_pipe,
            raygen_foliage_pipes,
            diffuse_pipe,
            ao_pipe,
            fill_stencil_glossy_pipe,
            fill_stencil_smoke_pipe,
            glossy_pipe,
            smoke_pipe,
            tonemap_pipe,
            // overlay_pipe,
            radiance_pipe,
            map_pipe,
            update_grass_pipe,
            update_water_pipe,
            gen_perlin2d_pipe,
            gen_perlin3d_pipe,
        }
    }

    #[cold]
    #[optimize(size)]
    pub unsafe fn destroy_all_pipes(gl: &Context, pipes: AllPipes) {
        gl.delete_program(pipes.lightmap_blocks_pipe.program);
        gl.delete_program(pipes.lightmap_models_pipe.program);
        gl.delete_program(pipes.raygen_blocks_pipe.program);
        gl.delete_program(pipes.raygen_models_pipe.program);
        gl.delete_program(pipes.raygen_particles_pipe.program);
        gl.delete_program(pipes.raygen_water_pipe.program);
        for foliage in pipes.raygen_foliage_pipes {
            gl.delete_program(foliage.program);
        }
        gl.delete_program(pipes.diffuse_pipe.program);
        gl.delete_program(pipes.ao_pipe.program);
        gl.delete_program(pipes.fill_stencil_glossy_pipe.program);
        gl.delete_program(pipes.fill_stencil_smoke_pipe.program);
        gl.delete_program(pipes.glossy_pipe.program);
        gl.delete_program(pipes.smoke_pipe.program);
        gl.delete_program(pipes.tonemap_pipe.program);
        gl.delete_program(pipes.radiance_pipe.program);
        gl.delete_program(pipes.map_pipe.program);
        gl.delete_program(pipes.update_grass_pipe.program);
        gl.delete_program(pipes.update_water_pipe.program);
        gl.delete_program(pipes.gen_perlin2d_pipe.program);
        gl.delete_program(pipes.gen_perlin3d_pipe.program);
    }
}
