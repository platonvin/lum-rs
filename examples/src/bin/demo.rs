#![allow(clippy::missing_safety_doc)]

use anyhow::Result;
use lum::{LumRenderer, LumSettings};
use winit::event::{self, Event};

/*
void Lum::Renderer::submitFrame() noexcept {
TRACE()
    opaque_members->render.start_frame();
TRACE()

        // opaque_members->render.start_compute();
            opaque_members->render.start_blockify();    
            for (let m : mesh_que){
                opaque_members->render.blockify_mesh((LumInternal::InternalMeshModel*)m.mesh.ptr, &m.trans);
            }
TRACE()

            opaque_members->render.end_blockify();
TRACE()
            opaque_members->render.shift_radiance(stored_radiance_shift);
TRACE()
            opaque_members->render.update_radiance();
TRACE()
            // opaque_members->render.recalculate_df(); // currently unused. per-voxel Distance Field 
            // opaque_members->render.recalculate_bit(); // currently unused. Bitpacking, like (block==Air) ? 0 : 1 
TRACE()
            opaque_members->render.updade_grass({});
TRACE()
            opaque_members->render.updade_water();
TRACE()
            opaque_members->render.exec_copies();
TRACE()
                opaque_members->render.start_map();
TRACE()
                for (let m : mesh_que){
                    opaque_members->render.map_mesh((LumInternal::InternalMeshModel*)m.mesh.ptr, &m.trans);
                }
TRACE()
                opaque_members->render.end_map();
TRACE()
            opaque_members->render.end_compute();
TRACE()
                // opaque_members->render.raytrace();
                opaque_members->render.start_lightmap();
TRACE()
                //yeah its wrong
                opaque_members->render.lightmap_start_blocks();
TRACE()
                    for(let b : block_que){
                        opaque_members->render.lightmap_block(&opaque_members->block_palette[b.block].mesh, b.block, b.pos);
                    }
TRACE()
                opaque_members->render.lightmap_start_models();
TRACE()
                    for (let m : mesh_que){
                        opaque_members->render.lightmap_model((LumInternal::InternalMeshModel*)m.mesh.ptr, &m.trans);
                    }
TRACE()
                opaque_members->render.end_lightmap();
TRACE()

                opaque_members->render.start_raygen();  
TRACE()
                // printl(block_que.size());
                opaque_members->render.raygen_start_blocks();
TRACE()
                    for(let b : block_que){
                        // DEBUG_LOG(b.block)
                        // DEBUG_LOG(&block_palette[b.block].mesh)
                        opaque_members->render.raygen_block(&opaque_members->block_palette[b.block].mesh, b.block, b.pos);
                    }  
TRACE()
                    
                opaque_members->render.raygen_start_models();
TRACE()
                    for (let m : mesh_que){
                        opaque_members->render.raygen_model((LumInternal::InternalMeshModel*)m.mesh.ptr, &m.trans);
                    }
TRACE()
                opaque_members->render.update_particles();
TRACE()
                opaque_members->render.raygen_map_particles();      
TRACE()
                opaque_members->render.raygen_start_grass();
TRACE()
                    for(let f : foliage_que){
                        opaque_members->render.raygen_map_grass((LumInternal::InternalMeshFoliage*)f.foliage, f.pos);
                    }
TRACE()

                opaque_members->render.raygen_start_water();
TRACE()
                    for(let l : liquid_que){
                        opaque_members->render.raygen_map_water(*((LumInternal::InternalMeshLiquid*)(l.liquid)), l.pos);
                    }
TRACE()
                opaque_members->render.end_raygen();
TRACE()
                opaque_members->render.start_2nd_spass();
TRACE()
                opaque_members->render.diffuse();
TRACE()
                opaque_members->render.ambient_occlusion(); 
TRACE()
                opaque_members->render.glossy_raygen();
TRACE()
                opaque_members->render.raygen_start_smoke();
TRACE()
                    for(let v : volumetric_que){
                        opaque_members->render.raygen_map_smoke(*((LumInternal::InternalMeshVolumetric*)(v.volumetric)), v.pos);
                    }
TRACE()
                opaque_members->render.glossy();
TRACE()
                opaque_members->render.smoke();
TRACE()
                opaque_members->render.tonemap();
TRACE()
            opaque_members->render.start_ui(); 
//                 ui.update();
TRACE()
//                 ui.draw();
TRACE()
        opaque_members->render.end_ui(); 
        opaque_members->render.end_2nd_spass();
TRACE()
    // this should happen BEFORE end_frame
    // otherwise (if after) there is more (visible!) inconsistency which might seem like its perfomance problems
    updateTime(); 

    opaque_members->render.end_frame();
TRACE()
}

*/
fn main() -> Result<()> {
    print!("started");
    let settings = LumSettings {
        static_block_palette_size: 0,
        ..LumSettings::default()
    };


    let mut lum = unsafe { LumRenderer::create(&settings) }?;
    let tank_body = lum.load_mesh_from_file("assets/tank_body.vox", true, true);

    loop {
        let mut should_break = false;
        lumal::atrace!();
        lum.lumal.event_loop.take().unwrap().run(
            |event, event_loop| {
                match event {
                    Event::LoopExiting => {
                        // break; // oh syntax problem creted for no reason by winit. Or is there a reason?
                        should_break = true;
                        println!("should_break");
                        return;
                    }
                    _ => {
                        println!("event");
                        lumal::atrace!();
                        lum.start_frame();
                        lumal::atrace!();
                        lum.start_blockify();
                        lumal::atrace!();
                        lum.end_blockify();
            
            lumal::atrace!();
                        lum.shift_radiance(Default::default());
            
            lumal::atrace!();
                        lum.update_radiance();
                        lumal::atrace!();
                        lum.updade_grass(Default::default());
                        lumal::atrace!();
                        lum.updade_water();
                        lumal::atrace!();
                        lum.exec_copies();
                        
                        lumal::atrace!();
                        lum.start_map();
                        lumal::atrace!();
                            // lum.map_mesh(&mesh, Default::default());
                            lumal::atrace!();
                        lum.end_map();
            
            lumal::atrace!();
                        lum.end_compute();
            
            lumal::atrace!();
                        lum.start_lightmap();
                        lumal::atrace!();
                            lum.lightmap_start_blocks();
                            lumal::atrace!();
                            // lum.lightmap_block(&mesh, 0, Default::default());
                            lumal::atrace!();
                            lum.lightmap_start_models();
                            lumal::atrace!();
                            // lum.lightmap_model(&mesh, Default::default());
                            lumal::atrace!();
                            lum.end_lightmap();
            
            lumal::atrace!();
                        lum.start_raygen();
                        lumal::atrace!();
                            lum.raygen_start_blocks();
                            lumal::atrace!();
                            // lum.raygen_block(&mesh, 0, Default::default());
                            lumal::atrace!();
                            lum.raygen_start_models();
                            lumal::atrace!();
                            // lum.raygen_model(&mesh, Default::default());
                            lumal::atrace!();
                            lum.update_particles();
                            lumal::atrace!();
                            lum.raygen_map_particles();
                            lumal::atrace!();
                            lum.raygen_start_grass();
                            lumal::atrace!();
                            // lum.raygen_map_grass(&mesh, Default::default());
                            lumal::atrace!();
                            lum.raygen_start_water();
                            lumal::atrace!();
                            // lum.raygen_map_water(&mesh, Default::default());
                            lumal::atrace!();
                            lum.end_raygen();
            
            lumal::atrace!();
                        lum.start_2nd_spass();
                        lumal::atrace!();
                            lum.diffuse();
                            lumal::atrace!();
                            lum.ambient_occlusion();
                            lumal::atrace!();
                            lum.glossy_raygen();
                            lumal::atrace!();
                            lum.raygen_start_smoke();
                            lumal::atrace!();
                            // lum.raygen_map_smoke(&mesh, Default::default());
                            lumal::atrace!();
                            lum.glossy();
                            lumal::atrace!();
                            lum.smoke();
                            lumal::atrace!();
                            lum.tonemap();
                        
                        lumal::atrace!();
                        // lum.start_ui();
                        //     ui.update();
                        //     ui.draw();
                        lumal::atrace!();
                        // lum.end_ui();
            
            lumal::atrace!();
                        lum.end_2nd_spass();
                        lumal::atrace!();
                    lum.end_frame();
                    }
                    // am i wrong or is winit defeating purpose of Vulkan async?
                }
            }
        )?;
        if should_break {
            break;
        }
    }
    lumal::atrace!();
    // lum.start_frame();
    
    // LumRenderer::
    lumal::atrace!();
    unsafe { lum.destroy() };

    print!("finished");
    Ok(())
}