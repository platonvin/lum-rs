use lumal::trace;
use qvek::{i16vec3, ivec3, vec3, vec4};
use winit::{event_loop, window::Window};

use crate::{
    containers::Arena,
    internal_renderer::{
        load_interface::LoadInterface, render_gl::InternalRendererGL,
        render_interface::LumRendererAPI, render_vk::InternalRendererVulkan,
    },
    types::{ivec3, quat, uvec3},
};

use super::{
    containers::Array3D,
    types::{
        i16vec3, mat4, u8vec3, vec3, BlockId, InternalMeshFoliage, InternalMeshFoliageDesc,
        InternalMeshLiquid, InternalMeshModel, InternalMeshVolumetric, MatId, MeshTransform,
    },
};

// opaque handlers. Done this way for cheap copying and simple lifetime management
#[derive(Clone, Copy)]
pub struct MeshModel(usize);
#[derive(Clone, Copy)]
pub struct MeshVolumetric(usize);
#[derive(Clone, Copy)]
pub struct MeshLiquid(usize);
#[derive(Clone)]
// internal foliage mesh is already opaque handle
pub struct MeshFoliage(InternalMeshFoliage);

pub struct ModelRenderRequest {
    pub cam_dist: f32,
    pub mesh: MeshModel,
    pub trans: MeshTransform,
}
pub struct BlockRenderRequest {
    pub cam_dist: f32,
    pub block: BlockId,
    // snapped to voxel grid
    pub pos: i16vec3,
}
pub struct FoliageRenderRequest {
    pub cam_dist: f32,
    pub mesh: MeshFoliage,
    //TODO: pub size: vec2
    pub pos: vec3,
}
pub struct LiquidRenderRequest {
    pub cam_dist: f32,
    pub mesh: MeshLiquid,
    //TODO: pub size: vec2/vec3?
    pub pos: vec3,
}
pub struct VolumetricRenderRequest {
    pub cam_dist: f32,
    pub mesh: MeshVolumetric,
    //TODO: pub size: vec3?
    pub pos: vec3,
}

// pre-init stage Renderer that should be converted to initialized before usage
#[pub_fields::pub_fields]
pub struct PreInitRenderer {
    // renderer: InternalRenderer,
    foliage_descriptions: Vec<InternalMeshFoliageDesc>,
}

#[derive(Default)]
struct RendererStorage<BufferType, ImageType> {
    // TODO: arena?
    models: Arena<InternalMeshModel<BufferType, ImageType>>,
    volumetrics: Arena<InternalMeshVolumetric>,
    liquids: Arena<InternalMeshLiquid>,
    // TODO: do smth about that this is stored inside internal renderer and everything else is stored here
    // foliages: Arena<InternalMeshFoliage>,
}

// initialized fully working Renderer that can be used to draw voxels on screen
#[pub_fields::pub_fields]
pub struct Renderer {
    #[cfg(feature = "vk_backend")]
    renderer: InternalRendererVulkan,
    #[cfg(feature = "gl_backend")]
    renderer: InternalRendererGL,
    // foliages: Vec<InternalMeshFoliage>,
    block_que: Vec<BlockRenderRequest>,
    model_que: Vec<ModelRenderRequest>,
    foliage_que: Vec<FoliageRenderRequest>,
    liquid_que: Vec<LiquidRenderRequest>,
    volumetric_que: Vec<VolumetricRenderRequest>,
    #[cfg(feature = "vk_backend")]
    storage: RendererStorage<lumal::Buffer, lumal::Image>,
    #[cfg(feature = "gl_backend")]
    storage: RendererStorage<Option<glow::Buffer>, Option<glow::Texture>>,
    radiance_shift: ivec3,
}

impl PreInitRenderer {
    // makes initialized Renderer from PreInitRenderer
    pub fn init(
        self,
        settings: &super::internal_renderer::Settings,
        window: &mut Window,
        event_loop: &event_loop::EventLoop<()>,
    ) -> Renderer {
        Renderer {
            #[cfg(feature = "vk_backend")]
            renderer: unsafe {
                InternalRendererVulkan::create(
                    settings,
                    window,
                    // event_loop,
                    self.foliage_descriptions,
                )
            },
            #[cfg(feature = "gl_backend")]
            renderer: unsafe {
                InternalRendererGL::create(
                    settings,
                    window,
                    // event_loop,
                    self.foliage_descriptions,
                )
            },
            block_que: vec![],
            model_que: vec![],
            foliage_que: vec![],
            liquid_que: vec![],
            volumetric_que: vec![],
            storage: Default::default(),
            radiance_shift: ivec3::zero(),
        }
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
        spirv_shader_code: Vec<u8>,
        vertices_per_blade: u32,
        density: u32,
    ) -> MeshFoliage {
        // current vec size is the index of last (which is what we need)
        let index = self.foliage_descriptions.len() as u32;
        // and then we push the one so it is created afterwards (defer into queue)
        self.foliage_descriptions.push(InternalMeshFoliageDesc {
            spirv_code: spirv_shader_code,
            vertices: vertices_per_blade,
            density,
        });

        MeshFoliage(InternalMeshFoliage { stored_id: index })
    }
}

impl Renderer {
    // Creates partially-initialized Renderer (separate struct to utilize type system)
    pub fn create() -> PreInitRenderer {
        PreInitRenderer {
            // renderer: unsafe { InternalRenderer::create(settings, window) }?,
            foliage_descriptions: vec![],
            // block_que: vec![],
            // mesh_que: vec![],
            // foliage_que: vec![],
            // liquid_que: vec![],
            // volumetric_que: vec![],
        }
    }
    pub fn destroy(self) {
        unsafe { self.renderer.destroy() };
    }

    pub fn load_model(&mut self, path: &str) -> MeshModel {
        let model_mesh = self.renderer.load_mesh_from_file(path, true, true);
        let index = self.storage.models.allocate(model_mesh).unwrap();
        MeshModel(index)
    }
    pub fn unload_model(&mut self, model: MeshModel) {
        let model_mesh = self.storage.models.take(model.0).unwrap();
        self.renderer.free_mesh(model_mesh);
    }
    pub fn get_model_size(&self, model: MeshModel) -> uvec3 {
        self.storage.models.get(model.0).unwrap().total_size
    }

    // loads a block (from file) into GPU-side mesh and CPU-side voxel data
    pub fn load_block(&mut self, block: BlockId, path: &str) {
        self.renderer.load_block_from_file(block, path);
    }
    pub fn unload_block(&mut self, block: BlockId) {
        self.renderer.free_block(block);
    }

    // volumetrics can be loaded any time (no context on GPU). But please, load them in the same way as models / foliage
    // rendered using same shader, mesh is just "uniforms"
    pub fn load_volumetric(
        &mut self,
        max_density: f32,
        dencity_variation: f32,
        color: u8vec3,
    ) -> MeshVolumetric {
        let volumetric_mesh = InternalMeshVolumetric {
            max_density,
            variation: dencity_variation,
            color,
        };
        let index = self.storage.volumetrics.allocate(volumetric_mesh).unwrap();
        MeshVolumetric(index)
    }
    pub fn unload_volumetric(&mut self, volumetric: MeshVolumetric) {
        let volumetric_mesh = self.storage.volumetrics.take(volumetric.0).unwrap();
        drop(volumetric_mesh);
    }

    // liquids can be loaded any time (no context on GPU). But please, load them in the same way as models / foliage / volumetrics
    // rendered using same shader, mesh is just "uniforms"
    pub fn load_liquid(&mut self, main_mat: MatId, foam_mat: MatId) -> MeshLiquid {
        let liquid_mesh = InternalMeshLiquid {
            main: main_mat,
            foam: foam_mat,
        };
        let index = self.storage.liquids.allocate(liquid_mesh).unwrap();
        MeshLiquid(index)
    }
    pub fn unload_liquid(&mut self, liquid: MeshLiquid) {
        let liquid_mesh = self.storage.liquids.take(liquid.0).unwrap();
        drop(liquid_mesh);
    }

    pub fn unload_foliage(&mut self, foliage: MeshFoliage) {
        let _ = foliage;
    }

    pub fn calculate_and_sort_by_cam_dist<Type>(rqueue: &mut [Type], camera_transform: mat4)
    where
        Type: GetPos,
    {
        for rrequest in rqueue.iter_mut() {
            let clip_coords = camera_transform * vec4!(rrequest.get_pos(), 1.0);
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

                    // let clip = new_pos / new_pos.w;
                    let new_pos = quat::identity() * pos;
                    let corner = vec4!(new_pos + vec3!(x, y, z), 1.0);
                    let clip = self.renderer.camera.camera_transform * corner;

                    // Note: orth assumes w == 1.0
                    // Check if within NDC range
                    if (clip.x >= -1.0)
                        && (clip.y >= -1.0)
                        && (clip.z >= -1.0)
                        && (clip.x <= 1.0)
                        && (clip.y <= 1.0)
                        && (clip.z <= 1.0)
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

    pub fn is_model_visible(&self, model_size: &uvec3, trans: &MeshTransform) -> bool {
        let min_corner = vec3::zero();
        let max_corner = vec3!(*model_size);

        // Transform the corners
        let mut transformed_corners = [vec3::default(); 8];
        for x in 0..=1 {
            for y in 0..=1 {
                for z in 0..=1 {
                    let corner = vec3!(x, y, z) * max_corner + min_corner;
                    transformed_corners[x + y * 2 + z * 4] =
                        trans.rotation * corner + trans.translation;
                }
            }
        }

        for corner in transformed_corners {
            let mut clip = self.renderer.camera.camera_transform * vec4!(corner, 1.0);

            // Perspective divide (to convert from clip space to NDC)
            // NOTE: i have no idea if it actually helps. TODO:
            if clip.w != 0.0 {
                clip /= clip.w;
            }

            // Check if the point lies within the NDC range
            // i guess i can use GLM for simd but its not bottleneck for now
            // TODO: asm view to imrpove every fun
            if (clip.x >= -1.0)
                && (clip.y >= -1.0)
                && (clip.z >= -1.0)
                && (clip.x <= 1.0)
                && (clip.y <= 1.0)
                && (clip.z <= 1.0)
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
                    let block = self.renderer.origin_world[(xx as usize, yy as usize, zz as usize)];
                    if block == 0 {
                        continue;
                    }

                    let block_pos = i16vec3!(xx, yy, zz) * 16;

                    self.draw_block(block, &block_pos);
                }
            }
        }
    }

    pub fn draw_block(&mut self, block: i16, block_pos: &i16vec3) {
        let fpos = vec3!(*block_pos);

        if self.is_block_visible(fpos) {
            self.block_que.push(BlockRenderRequest {
                cam_dist: 0.0,
                block,
                pos: *block_pos,
            });
        }
    }

    pub fn draw_model(&mut self, model: &MeshModel, trans: &MeshTransform) {
        let model_mesh = self.storage.models.get(model.0).unwrap();
        // model size also happens to be >= its bounding box (dont leave voxel padding)
        if self.is_model_visible(&model_mesh.total_size, trans) {
            self.model_que.push(ModelRenderRequest {
                cam_dist: 0.0,
                mesh: *model,
                trans: *trans,
            });
        }
    }

    pub fn draw_foliage(&mut self, foliage: &MeshFoliage, pos: &vec3) {
        // foliage is assumed to be somewhat block constrained
        if self.is_block_visible(*pos) {
            self.foliage_que.push(FoliageRenderRequest {
                cam_dist: 0.0,
                mesh: foliage.clone(),
                pos: *pos,
            });
        }
    }

    pub fn draw_liquid(&mut self, liquid: &MeshLiquid, pos: &vec3) {
        // liquids are assumed to be somewhat block constrained
        if self.is_block_visible(*pos) {
            self.liquid_que.push(LiquidRenderRequest {
                cam_dist: 0.0,
                mesh: *liquid,
                pos: *pos,
            });
        }
    }

    pub fn draw_volumetric(&mut self, volumetric: &MeshVolumetric, pos: &vec3) {
        // volumetrics are assumed to be somewhat block constrained
        if self.is_block_visible(*pos) {
            self.volumetric_que.push(VolumetricRenderRequest {
                cam_dist: 0.0,
                mesh: *volumetric,
                pos: *pos,
            });
        }
    }

    pub fn shift_radiance(&mut self, shift: ivec3) {
        self.radiance_shift = shift;
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

    pub fn end_frame(&mut self, window: &Window) {
        // yes, started here cause no reason not to group
        // flame::start("GLOBAL_FRAME");

        // flame::start("start_blockify");
        self.renderer.start_blockify();
        // flame::end("start_blockify");
        // flame::start("blockify_mesh");
        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh.0).unwrap();
            self.renderer.blockify_mesh(model_mesh, &mrr.trans);
        }
        // flame::end("blockify_mesh");
        // flame::start("end_blockify");
        self.renderer.end_blockify();
        // flame::end("end_blockify");
        // flame::start("find_radiance_to_update");
        self.renderer.find_radiance_to_update();
        // flame::end("find_radiance_to_update");
        // you may wonder why is start_frame here, and not in the beginning
        // this is because it contains syncronization, which im trying to delay as much as possible
        // sadly, it does not help when you are CPU-bound (which is the case here). But still useful
        // flame::start("start_frame");
        self.renderer.start_frame();
        // flame::end("start_frame");
        // flame::start("shift_radiance");
        self.renderer.shift_radiance(self.radiance_shift);
        self.radiance_shift = ivec3::zero();
        // flame::end("shift_radiance");
        // flame::start("update_radiance");
        self.renderer.update_radiance();
        // flame::end("update_radiance");
        // flame::start("updade_grass");
        self.renderer.updade_grass(Default::default());
        // flame::end("updade_grass");
        // flame::start("updade_water");
        self.renderer.updade_water();
        // flame::end("updade_water");
        // flame::start("exec_copies");
        self.renderer.exec_copies();
        // flame::end("exec_copies");
        // flame::start("start_map");
        self.renderer.start_map();
        // flame::end("start_map");
        // flame::start("map_mesh");
        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh.0).unwrap();
            self.renderer.map_mesh(model_mesh, &mrr.trans);
        }
        // flame::end("map_mesh");
        // flame::start("end_map");
        self.renderer.end_map();
        // flame::end("end_map");
        // flame::start("end_compute");
        self.renderer.end_compute();
        // flame::end("end_compute");
        // flame::start("start_lightmap");
        self.renderer.start_lightmap();
        // flame::end("start_lightmap");
        // flame::start("lightmap_start_blocks");
        self.renderer.lightmap_start_blocks();
        // flame::end("lightmap_start_blocks");
        // flame::start("lightmap_blocks");
        for brr in &self.block_que {
            let ipos = ivec3!(brr.pos);
            self.renderer.lightmap_block(brr.block, ipos);
        }
        // flame::end("lightmap_blocks");
        // flame::start("lightmap_start_models");
        self.renderer.lightmap_start_models();
        // flame::end("lightmap_start_models");
        // flame::start("lightmap_models");
        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh.0).unwrap();
            self.renderer.lightmap_model(model_mesh, &mrr.trans);
        }
        // flame::end("lightmap_models");
        // flame::start("end_lightmap");
        self.renderer.end_lightmap();
        // flame::end("end_lightmap");
        // flame::start("start_raygen");
        self.renderer.start_raygen();
        // flame::end("start_raygen");
        // flame::start("raygen_start_blocks");
        self.renderer.raygen_start_blocks();
        // flame::end("raygen_start_blocks");
        // flame::start("raygen_blocks");
        for brr in &self.block_que {
            let ipos = ivec3!(brr.pos);
            self.renderer.raygen_block(brr.block, ipos);
        }
        // flame::end("raygen_blocks");
        // flame::start("raygen_start_models");
        self.renderer.raygen_start_models();
        // flame::end("raygen_start_models");
        // flame::start("raygen_models");
        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh.0).unwrap();
            self.renderer.raygen_model(model_mesh, &mrr.trans);
        }
        // flame::end("raygen_models");
        // flame::start("update_particles");
        self.renderer.update_particles();
        // flame::end("update_particles");
        // flame::start("raygen_map_particles");
        self.renderer.raygen_map_particles();
        // flame::end("raygen_map_particles");
        // flame::start("raygen_start_grass");
        self.renderer.raygen_start_grass();
        // flame::end("raygen_start_grass");
        // flame::start("raygen_grass");
        for frr in &self.foliage_que {
            self.renderer.raygen_map_grass(&frr.mesh.0, &frr.pos);
        }
        // flame::end("raygen_grass");
        // flame::start("raygen_start_water");
        self.renderer.raygen_start_water();
        // flame::end("raygen_start_water");
        // flame::start("raygen_water");
        for lrr in &self.liquid_que {
            let liquid_mesh = self.storage.liquids.get(lrr.mesh.0).unwrap();
            self.renderer.raygen_map_water(liquid_mesh, &lrr.pos);
        }
        // flame::end("raygen_water");
        // flame::start("end_raygen");
        self.renderer.end_raygen();
        // flame::end("end_raygen");
        // flame::start("start_2nd_spass");
        self.renderer.start_2nd_spass();
        // flame::end("start_2nd_spass");
        // flame::start("diffuse");
        self.renderer.diffuse();
        // flame::end("diffuse");
        // flame::start("ambient_occlusion");
        self.renderer.ambient_occlusion();
        // flame::end("ambient_occlusion");
        // flame::start("glossy_raygen");
        self.renderer.glossy_raygen();
        // flame::end("glossy_raygen");
        // flame::start("raygen_start_smoke");
        self.renderer.raygen_start_smoke();
        // flame::end("raygen_start_smoke");
        // flame::start("raygen_smoke");
        for vrr in &self.volumetric_que {
            let volumetric_mesh = self.storage.volumetrics.get(vrr.mesh.0).unwrap();
            self.renderer.raygen_map_smoke(volumetric_mesh, &vrr.pos);
        }
        // flame::end("raygen_smoke");
        // flame::start("glossy");
        self.renderer.glossy();
        // flame::end("glossy");
        // flame::start("smoke");
        self.renderer.smoke();
        // flame::end("smoke");
        // flame::start("tonemap");
        self.renderer.tonemap();
        // flame::end("tonemap");
        // flame::start("end_2nd_spass");
        self.renderer.end_2nd_spass();
        // flame::end("end_2nd_spass");
        // flame::start("end_frame");
        self.renderer.end_frame(window);
        // flame::end("end_frame");

        // flame::end("GLOBAL_FRAME");
    }

    pub fn get_world_blocks(&self) -> &Array3D<BlockId> {
        &self.renderer.current_world
    }
    pub fn get_world_blocks_mut(&mut self) -> &mut Array3D<BlockId> {
        &mut self.renderer.current_world
    }

    // pub fn
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
        self.trans.translation
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
        vec3!(self.pos)
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
        vec3!(self.pos)
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
        vec3!(self.pos)
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
        vec3!(self.pos)
    }

    fn set_cam_dist(&mut self, cam_dist: f32) {
        self.cam_dist = cam_dist;
    }

    fn get_cam_dist(&self) -> f32 {
        self.cam_dist
    }
}
