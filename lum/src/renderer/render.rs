
use winit::window::Window;

use crate::types::{quat, uvec3};

use super::{
    containers::Array3D,
    internal_renderer::InternalRenderer,
    types::{
        i16vec3, mat4, u8vec3, vec3, vec4, BlockID_t, InternalMeshFoliage,
        InternalMeshFoliageDesc, InternalMeshLiquid, InternalMeshModel, InternalMeshVolumetric,
        MatID_t, MeshTransform,
    },
};

pub struct ModelRenderRequest {
    pub cam_dist: f32,
    pub mesh: InternalMeshModel,
    pub trans: MeshTransform,
}
pub struct BlockRenderRequest {
    pub cam_dist: f32,
    pub block: BlockID_t,
    // snapped to voxel grid
    pub pos: i16vec3,
}
pub struct FoliageRenderRequest {
    pub cam_dist: f32,
    pub mesh: InternalMeshFoliage,
    // pub pos: i16vec3, // snapped to voxel grid
    pub pos: vec3,
}
pub struct LiquidRenderRequest {
    pub cam_dist: f32,
    pub mesh: InternalMeshLiquid,
    // pub pos: i16vec3, // snapped to voxel grid
    pub pos: vec3,
}
pub struct VolumetricRenderRequest {
    pub cam_dist: f32,
    pub mesh: InternalMeshVolumetric,
    // pub pos: i16vec3, // snapped to voxel grid
    pub pos: vec3,
}

// pre-init stage Renderer that should be converted to initialized before usage
#[pub_fields::pub_fields]
pub struct PreInitRenderer {
    // renderer: InternalRenderer,
    foliage_descriptions: Vec<InternalMeshFoliageDesc>,
}

// initialized fully working Renderer that can be used to draw voxels on screen
#[pub_fields::pub_fields]
pub struct Renderer {
    renderer: InternalRenderer,
    // foliages: Vec<InternalMeshFoliage>,
    block_que: Vec<BlockRenderRequest>,
    model_que: Vec<ModelRenderRequest>,
    foliage_que: Vec<FoliageRenderRequest>,
    liquid_que: Vec<LiquidRenderRequest>,
    volumetric_que: Vec<VolumetricRenderRequest>,
}

impl PreInitRenderer {
    // makes initialized Renderer from PreInitRenderer
    pub fn init(
        self,
        settings: &super::internal_renderer::Settings,
        window: &mut Window,
    ) -> anyhow::Result<Renderer> {
        Ok(Renderer {
            renderer: unsafe {
                InternalRenderer::create(settings, window, self.foliage_descriptions)
            }?,
            block_que: vec![],
            model_que: vec![],
            foliage_que: vec![],
            liquid_que: vec![],
            volumetric_que: vec![],
        })
    }

    // creates a CPU-side struct for foliage
    // this is not foliage mesh itself yet, but a blank used to register foliage for future creation*
    // Foliage in lum is not a controlled simulation with a mesh. Instead, it is a (vertex) shader
    // This is highest level of flexibility** and also enforces perfomance
    // You use foliage meshes to draw things like grass in worldspace
    // TODO: is there a way to make src extendable to such degree without sacrificing anything?
    // * done this way for simplicity (aka pre-counting size)
    // **: Lum is not trying to be general-purpose engine at all. Some very basic parts that are expected from game engine
    // are and will forever be missing. You cant make fast abstraction on top of everything.
    pub fn load_foliage(
        &mut self,
        path_to_shader: &str,
        vertices_per_blade: u32,
        density: u32,
    ) -> InternalMeshFoliage {
        // current vec size is the index of last (which is what we need)
        let index = self.foliage_descriptions.len() as u32;
        // and then we push the one so it is created afterwards (defer into queue)
        self.foliage_descriptions.push(InternalMeshFoliageDesc {
            vertex_shader_file: path_to_shader.to_string(),
            vertices: vertices_per_blade,
            density,
        });

        InternalMeshFoliage { stored_id: index }
    }
}

impl Renderer {
    // Creates partially-initialized Renderer (separate struct to utilize type system)
    pub fn create() -> anyhow::Result<PreInitRenderer> {
        Ok(PreInitRenderer {
            // renderer: unsafe { InternalRenderer::create(settings, window) }?,
            foliage_descriptions: vec![],
            // block_que: vec![],
            // mesh_que: vec![],
            // foliage_que: vec![],
            // liquid_que: vec![],
            // volumetric_que: vec![],
        })
    }
    pub fn destroy(self) {
        unsafe { self.renderer.destroy() };
    }

    pub fn load_model(&mut self, path: &str) -> InternalMeshModel {
        self.renderer.load_mesh_from_file_ogt(path, true, true)
    }
    pub fn unload_model(&mut self, model: InternalMeshModel) {
        self.renderer.free_mesh(model);
    }

    // loads a block (from file) into GPU-side mesh and CPU-side voxel data
    pub fn load_block(&mut self, block: BlockID_t, path: &str) {
        self.renderer.load_block_from_file_ogt(block, path);
    }

    // volumetrics can be loaded any time (no context on GPU). But please, load them in the same way as models / foliage
    // rendered using same shader, mesh is just "uniforms"
    pub fn load_volumetric(
        &mut self,
        max_density: f32,
        dencity_variation: f32,
        color: u8vec3,
    ) -> InternalMeshVolumetric {
        InternalMeshVolumetric {
            max_density,
            variation: dencity_variation,
            color,
        }
    }
    pub fn unload_volumetric(&mut self, volumetric: InternalMeshVolumetric) {
        drop(volumetric);
    }

    // liquids can be loaded any time (no context on GPU). But please, load them in the same way as models / foliage / volumetrics
    // rendered using same shader, mesh is just "uniforms"
    pub fn load_liquid(&mut self, main_mat: MatID_t, foam_mat: MatID_t) -> InternalMeshLiquid {
        InternalMeshLiquid {
            main: main_mat,
            foam: foam_mat,
        }
    }
    pub fn unload_liquid(&mut self, liquid: InternalMeshLiquid) {
        drop(liquid);
    }

    pub fn unload_foliage(&mut self, foliage: InternalMeshFoliage) {
        drop(foliage);
    }

    pub fn calculate_and_sort_by_cam_dist<Type>(rqueue: &mut [Type], camera_transform: mat4)
    where
        Type: GetPos,
    {
        for rrequest in rqueue.iter_mut() {
            let clip_coords = camera_transform
                * vec4::new(
                    rrequest.get_pos().x,
                    rrequest.get_pos().y,
                    rrequest.get_pos().z,
                    1.0,
                );
            rrequest.set_cam_dist(-clip_coords.z);
        }

        rqueue.sort_unstable_by(|a, b| {
            if a.get_cam_dist() > b.get_cam_dist() {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        });
    }

    pub fn start_frame(&mut self) {
        // queues are like high-level draw calls, and we are clearing command buffers
        self.block_que.clear();
        self.model_que.clear();
        self.foliage_que.clear();
        self.liquid_que.clear();
        self.volumetric_que.clear();
    }

    pub fn is_block_visible(&self, pos: vec3) -> bool {
        for xx in 0..2 {
            for yy in 0..2 {
                for zz in 0..2 {
                    let x = xx as f32 * 16.0;
                    let y = yy as f32 * 16.0;
                    let z = zz as f32 * 16.0;

                    // let new_pos = trans * vec4::new(pos.x + x, pos.y + y, pos.z + z, 1.0);
                    // let clip = new_pos / new_pos.w;
                    let new_pos = quat::identity() * pos;
                    let corner = vec4::new(new_pos.x + x, new_pos.y + y, new_pos.z + z, 1.0);
                    let clip = self.renderer.camera.camera_transform * corner;

                    // Note: orth assumes w == 1.0
                    // Check if within NDC range
                    if ((clip.x >= -1.0)
                        && (clip.y >= -1.0)
                        && (clip.z >= -1.0)
                        && (clip.x <= 1.0)
                        && (clip.y <= 1.0)
                        && (clip.z <= 1.0))
                    {
                        // if any corner is in NDC range, block is at least partially visible
                        return true;
                    }
                }
            }
        }

        // none corners are in NDC range
        false
    }

    pub fn is_model_visible(&self, size: &uvec3, trans: &MeshTransform) -> bool {
        let model_size = size;
        let min_corner = vec3::new(0.0, 0.0, 0.0);
        let max_corner = vec3::new(
            model_size.x as f32,
            model_size.y as f32,
            model_size.z as f32,
        );

        // Transform the corners
        let mut transformed_corners = [vec3::default(); 8];
        for x in 0..=1 {
            for y in 0..=1 {
                for z in 0..=1 {
                    let corner = vec3::new(x as f32, y as f32, z as f32) * max_corner + min_corner;
                    transformed_corners[x + y * 2 + z * 4] =
                        trans.rotation * corner + trans.translation;
                }
            }
        }

        for corner in transformed_corners {
            let mut clip = self.renderer.camera.camera_transform
                * vec4::new(corner.x, corner.y, corner.z, 1.0);

            // Perspective divide (to convert from clip space to NDC)
            // NOTE: i have no idea if it actually helps. TODO:
            if clip.w != 0.0 {
                clip /= clip.w;
            }

            // Check if the point lies within the NDC range
            // i guess i can use GLM for simd but its not bottleneck for now
            // TODO: asm view to imrpove every fun
            if ((clip.x >= -1.0)
                && (clip.y >= -1.0)
                && (clip.z >= -1.0)
                && (clip.x <= 1.0)
                && (clip.y <= 1.0)
                && (clip.z <= 1.0))
            {
                // if any corner is in NDC range, block is at least partially visible
                return true;
            }
        }

        // none corners are in NDC range
        false
    }

    // TODO: calculate distance here vs separate
    // TODO: check visibility here vs separate
    pub fn draw_world(&mut self) {
        for zz in 0..self.renderer.settings.world_size.z {
            for yy in 0..self.renderer.settings.world_size.y {
                for xx in 0..self.renderer.settings.world_size.x {
                    let block =
                        self.renderer.current_world[(xx as usize, yy as usize, zz as usize)];
                    if block == 0 {
                        continue;
                    }

                    let block_pos = i16vec3::new(xx as i16, yy as i16, zz as i16) * 16;

                    self.draw_block(block, &block_pos);
                }
            }
        }
    }

    pub fn draw_block(&mut self, block: i16, block_pos: &vek::Vec3<i16>) {
        let fpos = vec3::new(block_pos.x as f32, block_pos.y as f32, block_pos.z as f32);

        if self.is_block_visible(fpos) {
            self.block_que.push(BlockRenderRequest {
                cam_dist: 0.0,
                block,
                pos: *block_pos,
            });
        }
    }

    pub fn draw_model(&mut self, model: &InternalMeshModel, trans: &MeshTransform) {
        if self.is_model_visible(&model.size, trans) {
            self.model_que.push(ModelRenderRequest {
                cam_dist: 0.0,
                mesh: model.clone(),
                trans: *trans,
            });
        }
    }

    // function that "optimizes" the frame
    // it could be implicit, but explicitnesss allows you to maybe do this work in parallel
    // such approach does not really play well with what i do (no multithreading in rendering), but anyways
    pub fn prepare_frame(&mut self) {
        // self.renderer.update_camera();
        // self.renderer.update_light_transform();
        let cam = self.renderer.camera.camera_transform;
        Self::calculate_and_sort_by_cam_dist(&mut self.model_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.block_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.foliage_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.liquid_que, cam);
        Self::calculate_and_sort_by_cam_dist(&mut self.volumetric_que, cam);
    }

    pub fn end_frame(&mut self) {
        // yes, started here cause no reason not to group
        self.renderer.start_frame();
        self.renderer.start_blockify();
        for mrr in &self.model_que {
            self.renderer.blockify_mesh(&mrr.mesh, &mrr.trans);
        }
        self.renderer.end_blockify();
        // self.renderer.shift_radiance(Default::default());
        self.renderer.update_radiance();
        self.renderer.updade_grass(Default::default());
        self.renderer.updade_water();
        self.renderer.exec_copies();
        self.renderer.start_map();
        for mrr in &self.model_que {
            self.renderer.map_mesh(&mrr.mesh, &mrr.trans);
        }
        self.renderer.end_map();
        self.renderer.end_compute();
        self.renderer.start_lightmap();
        self.renderer.lightmap_start_blocks();
        for brr in &self.block_que {
            let ipos =
                super::types::ivec3::new(brr.pos.x as i32, brr.pos.y as i32, brr.pos.z as i32);
            self.renderer.lightmap_block(brr.block, ipos);
        }
        self.renderer.lightmap_start_models();
        for mrr in &self.model_que {
            self.renderer.lightmap_model(&mrr.mesh, &mrr.trans);
        }
        self.renderer.end_lightmap();
        self.renderer.start_raygen();
        self.renderer.raygen_start_blocks();
        for brr in &self.block_que {
            let ipos =
                super::types::ivec3::new(brr.pos.x as i32, brr.pos.y as i32, brr.pos.z as i32);
            self.renderer.raygen_block(brr.block, ipos);
        }
        self.renderer.raygen_start_models();
        for mrr in &self.model_que {
            self.renderer.raygen_model(&mrr.mesh, &mrr.trans);
        }
        self.renderer.update_particles();
        self.renderer.raygen_map_particles();
        self.renderer.raygen_start_grass();
        for frr in &self.foliage_que {
            self.renderer.raygen_map_grass(&frr.mesh, &frr.pos);
        }
        self.renderer.raygen_start_water();
        for lrr in &self.liquid_que {
            self.renderer.raygen_map_water(&lrr.mesh, &lrr.pos);
        }
        self.renderer.end_raygen();
        self.renderer.start_2nd_spass();
        self.renderer.diffuse();
        self.renderer.ambient_occlusion();
        self.renderer.glossy_raygen();
        self.renderer.raygen_start_smoke();
        for vrr in &self.volumetric_que {
            self.renderer.raygen_map_smoke(&vrr.mesh, &vrr.pos);
        }
        self.renderer.glossy();
        self.renderer.smoke();
        self.renderer.tonemap();
        self.renderer.end_2nd_spass();
        self.renderer.end_frame();
    }

    pub fn get_world_blocks(&self) -> &Array3D<BlockID_t> {
        &self.renderer.current_world
    }
    pub fn get_world_blocks_mut(&mut self) -> &mut Array3D<BlockID_t> {
        &mut self.renderer.current_world
    }
}

// TODO: is there a simpler shorter)way to do this?
pub trait GetPos {
    // returns world-space pos
    fn get_pos(&self) -> vec3;
    fn set_cam_dist(&mut self, cam_dist: f32);
    fn get_cam_dist(&self) -> f32;
}

impl GetPos for ModelRenderRequest {
    fn get_pos(&self) -> vec3 {
        vec3::new(
            self.trans.translation.x,
            self.trans.translation.y,
            self.trans.translation.z,
        )
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}

impl GetPos for BlockRenderRequest {
    fn get_pos(&self) -> vec3 {
        vec3::new(self.pos.x as f32, self.pos.y as f32, self.pos.z as f32)
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}
impl GetPos for FoliageRenderRequest {
    fn get_pos(&self) -> vec3 {
        vec3::new(self.pos.x, self.pos.y, self.pos.z)
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}
impl GetPos for LiquidRenderRequest {
    fn get_pos(&self) -> vec3 {
        vec3::new(self.pos.x, self.pos.y, self.pos.z)
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}
impl GetPos for VolumetricRenderRequest {
    fn get_pos(&self) -> vec3 {
        vec3::new(self.pos.x, self.pos.y, self.pos.z)
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}
