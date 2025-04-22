use std::mem::size_of;
use std::ops::RangeBounds;

use as_u8_slice_derive::AsU8Slice;
use lumal::atrace;
use qvek::{i16vec3, i16vec4, ivec3, ivec4, vec3, vec4, vek::Clamp};
use wgpu::{BindGroup, BufferAddress, Color, COPY_BYTES_PER_ROW_ALIGNMENT};
use winit::{event_loop, window::Window};

use crate::{
    containers::Arena,
    internal_renderer::{
        aabb::{get_shift, iAABB},
        ao_lut,
        load_interface::LoadInterface,
        render_wgpu::{
            all_resources::all_types::UboData,
            wal::{self, Image, RasterPipe, Wal},
            InternalRendererWebGPU,
        },
    },
    types::{
        i16vec4, i8vec3, ivec3, ivec4, quat, u8vec4, uvec3, vec2, vec4, AoLut, IndexedVertices,
        Particle,
    },
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
    // foliage_descriptions: Vec<InternalMeshFoliageDesc>,
    foliage_descriptions: Vec<crate::internal_renderer::render_wgpu::MeshFoliageDesc>,
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
pub struct Renderer<'window> {
    renderer: InternalRendererWebGPU<'window>,
    block_que: Vec<BlockRenderRequest>,
    model_que: Vec<ModelRenderRequest>,
    foliage_que: Vec<FoliageRenderRequest>,
    liquid_que: Vec<LiquidRenderRequest>,
    volumetric_que: Vec<VolumetricRenderRequest>,
    storage: RendererStorage<Option<wgpu::Buffer>, Option<Image>>,
    radiance_shift: ivec3,
}

impl<'window> PreInitRenderer {
    // makes initialized Renderer from PreInitRenderer
    pub async fn init(
        self,
        settings: &super::internal_renderer::Settings,
        window: Window,
        event_loop: &event_loop::EventLoop<()>,
    ) -> Renderer<'window> {
        Renderer {
            renderer: InternalRendererWebGPU::create(settings, window, self.foliage_descriptions)
                .await,
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
        code: &'static str,
        vertices_per_blade: u32,
        density: u32,
    ) -> MeshFoliage {
        // current vec size is the index of last (which is what we need)
        let index = self.foliage_descriptions.len() as u32;
        // and then we push the one so it is created afterwards (defer into queue)
        // self.foliage_descriptions.push(InternalMeshFoliageDesc {
        //     spirv_code: spirv_shader_code,
        //     vertices: vertices_per_blade,
        //     density,
        // });
        self.foliage_descriptions
            .push(crate::internal_renderer::render_wgpu::MeshFoliageDesc {
                code: code,
                vertices: vertices_per_blade,
                density,
            });

        MeshFoliage(InternalMeshFoliage { stored_id: index })
    }
}

impl<'window> Renderer<'window> {
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
        // todo!()
    }
    pub fn unload_model(&mut self, model: MeshModel) {
        let model_mesh = self.storage.models.take(model.0).unwrap();
        self.renderer.free_mesh(model_mesh);
        // todo!()
    }
    pub fn get_model_size(&self, model: MeshModel) -> uvec3 {
        self.storage.models.get(model.0).unwrap().total_size
    }

    // loads a block (from file) into GPU-side mesh and CPU-side voxel data
    pub fn load_block(&mut self, block: BlockId, path: &str) {
        self.renderer.load_block_from_file(block, path);
        // todo!()
    }
    pub fn unload_block(&mut self, block: BlockId) {
        self.renderer.free_block(block);
        // todo!()
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

    pub fn draw_block(&mut self, block: BlockId, block_pos: &i16vec3) {
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

    pub fn end_frame(&mut self) {
        // yes, started here cause no reason not to group

        self.blockify_models();
        // self.renderer.find_radiance_to_update();
        // you may wonder why is start_frame here, and not in the beginning
        // this is because it contains syncronization, which im trying to delay as much as possible
        // sadly, it does not help when you are CPU-bound (which is the case here). But still useful
        self.renderer.start_frame();
        self.update_ubo();

        // self.renderer.shift_radiance(self.radiance_shift);
        // self.radiance_shift = ivec3::zero();
        // self.renderer.update_radiance();
        // self.updade_grass(Default::default());
        // self.updade_water();
        // self.renderer.exec_copies();

        //
        //
        // here we can se divergence between wgpu and vulkan. Wgpu is too complicated for my small brain so i do everything in a single scope
        // self.map_meshes();

        // self.update_light_ubo();
        // self.lightmap_blocks();
        // self.lightmap_models();

        // self.update_ao_ubo();
        // self.raygen_blocks();
        // self.raygen_models();

        // self.update_raygen_particles();

        // self.raygen_grass();
        // self.raygen_water();

        // webgpu is so good that most important Vulkan feature is missing (for convinience)
        // self.renderer.end_raygen();

        // self.diffuse();
        // self.ambient_occlusion();
        // self.glossy_raygen();
        // self.raygen_smoke();
        self.tonemap();

        self.renderer.dependent_images.as_mut().unwrap().highres_frame.move_next();
        self.renderer
            .dependent_images
            .as_mut()
            .unwrap()
            .highres_depth_stencil
            .move_next();
        self.renderer.dependent_images.as_mut().unwrap().highres_mat_norm.move_next();
        self.renderer.dependent_images.as_mut().unwrap().full_view_for_ds.move_next();
        self.renderer.dependent_images.as_mut().unwrap().stencil_view_for_ds.move_next();
        self.renderer.dependent_images.as_mut().unwrap().far_depth.move_next();
        self.renderer.dependent_images.as_mut().unwrap().near_depth.move_next();

        self.renderer.buffers.staging_world.move_next();
        self.renderer.buffers.light_uniform.move_next();
        self.renderer.buffers.uniform.move_next();
        self.renderer.buffers.ao_lut_uniform.move_next();
        self.renderer.buffers.gpu_radiance_updates.move_next();
        self.renderer.buffers.staging_radiance_updates.move_next();
        self.renderer.buffers.gpu_particles_staged.move_next();
        self.renderer.buffers.gpu_particles.move_next();

        self.renderer.independent_images.grass_state.move_next();
        self.renderer.independent_images.water_state.move_next();
        self.renderer.independent_images.perlin_noise2d.move_next();
        self.renderer.independent_images.perlin_noise3d.move_next();
        self.renderer.independent_images.world.move_next();
        self.renderer.independent_images.radiance_cache.move_next();
        self.renderer.independent_images.origin_block_palette.move_next();
        self.renderer.independent_images.material_palette.move_next();
        self.renderer.independent_images.lightmap.move_next();

        self.renderer
            .pipes
            .lightmap_blocks_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .lightmap_blocks_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .lightmap_blocks_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .lightmap_models_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .lightmap_models_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .lightmap_models_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .raygen_blocks_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next()); // Or ComputePipeline if it's ray tracing
        self.renderer
            .pipes
            .raygen_blocks_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next()); // Or ComputePipeline if it's ray tracing
        self.renderer
            .pipes
            .raygen_blocks_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next()); // Or ComputePipeline if it's ray tracing

        self.renderer
            .pipes
            .raygen_models_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next()); // Or ComputePipeline
        self.renderer
            .pipes
            .raygen_models_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next()); // Or ComputePipeline
        self.renderer
            .pipes
            .raygen_models_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next()); // Or ComputePipeline

        self.renderer
            .pipes
            .raygen_particles_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .raygen_particles_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .raygen_particles_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .raygen_water_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .raygen_water_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .raygen_water_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .diffuse_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer.pipes.diffuse_pipe.pc_buffers.as_mut().map(|bg| bg.move_next());
        self.renderer
            .pipes
            .diffuse_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer.pipes.ao_pipe.pc_bind_groups.as_mut().map(|bg| bg.move_next());
        self.renderer.pipes.ao_pipe.pc_buffers.as_mut().map(|bg| bg.move_next());
        self.renderer.pipes.ao_pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());

        self.renderer
            .pipes
            .fill_stencil_glossy_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .fill_stencil_glossy_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .fill_stencil_glossy_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .fill_stencil_smoke_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .fill_stencil_smoke_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .fill_stencil_smoke_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer.pipes.glossy_pipe.pc_bind_groups.as_mut().map(|bg| bg.move_next());
        self.renderer.pipes.glossy_pipe.pc_buffers.as_mut().map(|bg| bg.move_next());
        self.renderer
            .pipes
            .glossy_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer.pipes.smoke_pipe.pc_bind_groups.as_mut().map(|bg| bg.move_next());
        self.renderer.pipes.smoke_pipe.pc_buffers.as_mut().map(|bg| bg.move_next());
        self.renderer
            .pipes
            .smoke_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .tonemap_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer.pipes.tonemap_pipe.pc_buffers.as_mut().map(|bg| bg.move_next());
        self.renderer
            .pipes
            .tonemap_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .radiance_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer.pipes.radiance_pipe.pc_buffers.as_mut().map(|bg| bg.move_next());
        self.renderer
            .pipes
            .radiance_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer.pipes.map_pipe.pc_bind_groups.as_mut().map(|bg| bg.move_next());
        self.renderer.pipes.map_pipe.pc_buffers.as_mut().map(|bg| bg.move_next());
        self.renderer
            .pipes
            .map_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .update_grass_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .update_grass_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .update_grass_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .update_water_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .update_water_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .update_water_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .gen_perlin2d_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .gen_perlin2d_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .gen_perlin2d_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        self.renderer
            .pipes
            .gen_perlin3d_pipe
            .pc_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .gen_perlin3d_pipe
            .pc_buffers
            .as_mut()
            .map(|bg| bg.move_next());
        self.renderer
            .pipes
            .gen_perlin3d_pipe
            .static_bind_groups
            .as_mut()
            .map(|bg| bg.move_next());

        for foliage_pipe in self.renderer.pipes.raygen_foliage_pipes.iter_mut() {
            foliage_pipe.pc_bind_groups.as_mut().map(|bg| bg.move_next());
            foliage_pipe.pc_buffers.as_mut().map(|bg| bg.move_next());
            foliage_pipe.static_bind_groups.as_mut().map(|bg| bg.move_next());
        }

        atrace!();
    }

    fn blockify_models(&mut self) {
        {
            let this = &mut self.renderer;
            this.block_copies_queue.clear();
            this.palette_counter = this.static_block_palette_size as usize;

            // reset the current world to the origin
            this.current_world.copy_data_from(&this.origin_world);
        };
        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh.0).unwrap();
            {
                let this = &mut self.renderer;
                let trans: &MeshTransform = &mrr.trans;
                let rotate = mat4::from(trans.rotation);
                let shift = mat4::identity().translated_3d(trans.translation);
                let border_in_voxel = get_shift(shift * rotate, model_mesh.total_size);

                let mut border = iAABB {
                    min: ivec3!(border_in_voxel.min - 1.0) / 16,
                    max: ivec3!(border_in_voxel.max + 1.0) / 16,
                };

                // clamp to world size so no out of bounds
                border.min = ivec3::clamped(
                    border.min,
                    ivec3::zero(),
                    ivec3!(this.settings.world_size - 1),
                );
                border.max = ivec3::clamped(
                    border.max,
                    ivec3::zero(),
                    ivec3!(this.settings.world_size - 1),
                );

                for zz in border.min.z..=border.max.z {
                    for yy in border.min.y..=border.max.y {
                        for xx in border.min.x..=border.max.x {
                            let current_block =
                                this.current_world[(xx as usize, yy as usize, zz as usize)];
                            if (current_block as u32) < this.static_block_palette_size {
                                // static
                                //add to copy queue
                                let src_block = this.index_block_xy(current_block as usize);
                                let dst_block = this.index_block_xy(this.palette_counter);

                                // do image copy on for non-zero-src blocks. Other things still done for every allocated block
                                // because zeroing is fast
                                if current_block != 0 {
                                    // Create a command encoder for copying
                                    // let mut encoder = this.wal.device.create_command_encoder(
                                    //     &wgpu::CommandEncoderDescriptor {
                                    //         label: Some("Block Copy Command Encoder"),
                                    //     },
                                    // );

                                    // Copy the block data
                                    // encoder.copy_texture_to_texture(
                                    //     wgpu::TexelCopyTextureInfo {
                                    //         texture: &this
                                    //             .dependent_images
                                    //             .as_ref()
                                    //             .unwrap()
                                    //             .highres_mat_norm
                                    //             .current()
                                    //             .texture,
                                    //         mip_level: 0,
                                    //         origin: wgpu::Origin3d {
                                    //             x: src_block.x as u32 * 16,
                                    //             y: src_block.y as u32 * 16,
                                    //             z: 0,
                                    //         },
                                    //         aspect: wgpu::TextureAspect::All,
                                    //     },
                                    //     wgpu::TexelCopyTextureInfo {
                                    //         texture: &this
                                    //             .dependent_images
                                    //             .as_ref()
                                    //             .unwrap()
                                    //             .highres_mat_norm
                                    //             .current()
                                    //             .texture,
                                    //         mip_level: 0,
                                    //         origin: wgpu::Origin3d {
                                    //             x: dst_block.x as u32 * 16,
                                    //             y: dst_block.y as u32 * 16,
                                    //             z: 0,
                                    //         },
                                    //         aspect: wgpu::TextureAspect::All,
                                    //     },
                                    //     wgpu::Extent3d {
                                    //         width: 16,
                                    //         height: 16,
                                    //         depth_or_array_layers: 1,
                                    //     },
                                    // );

                                    // Submit the copy command
                                    // this.wal.queue.submit(Some(encoder.finish()));
                                }

                                this.current_world[(xx as usize, yy as usize, zz as usize)] =
                                    this.palette_counter as BlockId;
                                this.palette_counter += 1;
                            } else {
                                //already new block, just leave it
                            }
                        }
                    }
                }
            };
        }
        {
            let (dim_x, dim_y, dim_z) = (&mut self.renderer).current_world.dimensions();
            let padded_dim_x = (dim_x)
                .next_multiple_of(COPY_BYTES_PER_ROW_ALIGNMENT as usize / size_of::<BlockId>());
            let padded_count_to_copy = padded_dim_x * dim_y * dim_z;

            let mut padded_data: Vec<BlockId> = vec![0; padded_count_to_copy];
            for zz in 0..dim_z {
                for yy in 0..dim_y {
                    for xx in 0..dim_x {
                        let index = (xx + yy * padded_dim_x + zz * padded_dim_x * dim_y) as usize;
                        padded_data[index] =
                            self.renderer.current_world[(xx as usize, yy as usize, zz as usize)];
                    }
                }
            }

            let data: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    padded_data.as_ptr() as *const u8,
                    padded_count_to_copy * size_of::<BlockId>(),
                )
            };

            // Write the data to the staging_world buffer.
            let mut staging_world_slice =
                self.renderer.buffers.staging_world.current().slice(..).get_mapped_range_mut();
            staging_world_slice[..].copy_from_slice(data);
        };
    }

    fn updade_grass(&mut self, wind_direction: vec2) {
        let mut encoder =
            self.renderer
                .wal
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Grass Update Command Encoder"),
                });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Grass Update Compute Pass"),
            timestamp_writes: None,
        });

        // bind pipe and its static bindgroups
        self.renderer
            .wal
            .bind_compute_pipeline(&mut compute_pass, &self.renderer.pipes.update_grass_pipe);

        // Dispatch workgroups
        compute_pass.dispatch_workgroups(
            (self.renderer.settings.world_size.x * 2).div_ceil(8),
            (self.renderer.settings.world_size.y * 2).div_ceil(8),
            1,
        );

        drop(compute_pass);
        self.renderer.wal.queue.submit(Some(encoder.finish()));
    }

    fn updade_water(&mut self) {
        let mut encoder =
            self.renderer
                .wal
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Water Update Command Encoder"),
                });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Water Update Compute Pass"),
            timestamp_writes: None,
        });

        self.renderer
            .wal
            .bind_compute_pipeline(&mut compute_pass, &self.renderer.pipes.update_water_pipe);

        // Dispatch workgroups
        compute_pass.dispatch_workgroups(
            (self.renderer.settings.world_size.x * 2).div_ceil(8),
            (self.renderer.settings.world_size.y * 2).div_ceil(8),
            1,
        );

        drop(compute_pass);
        self.renderer.wal.queue.submit(Some(encoder.finish()));
    }

    fn update_ao_ubo(&mut self) {
        let ao_lut = ao_lut::generate_lut::<8>(
            16.0 / 1000.0,
            vec2::new(
                self.renderer.wal.config.width as f32,
                self.renderer.wal.config.height as f32,
            ),
            self.renderer.camera.horizline * self.renderer.camera.view_size.x / 2.0,
            self.renderer.camera.vertiline * self.renderer.camera.view_size.y / 2.0,
        );

        // Update the buffer via the queue
        self.renderer.wal.queue.write_buffer(
            &self.renderer.buffers.ao_lut_uniform.current(),
            0,
            unsafe {
                std::slice::from_raw_parts(
                    (&ao_lut as *const AoLut) as *const u8,
                    std::mem::size_of::<AoLut>(),
                )
            },
        );
    }

    fn raygen_water(&mut self) {
        // Begin the raygen water render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Raygen Water Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self
                        .renderer
                        .dependent_images
                        .as_ref()
                        .unwrap()
                        .highres_mat_norm
                        .current()
                        .view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self
                        .renderer
                        .dependent_images
                        .as_ref()
                        .unwrap()
                        .full_view_for_ds
                        .current(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        self.renderer
            .wal
            .bind_raster_pipeline(&mut rpass, &self.renderer.pipes.raygen_water_pipe);

        for lrr in &self.liquid_que {
            let liquid_mesh = self.storage.liquids.get(lrr.mesh.0).unwrap();
            let pos: &vec3 = &lrr.pos;
            let quality_size = 32;

            #[repr(C)] // for push constants
            #[derive(AsU8Slice)] // allow cast to &[u8]
            struct PushConstant {
                shift: vec4,
                _size: i32,
                _time: i32,
            }

            let push_constant = PushConstant {
                shift: vec4!(*pos, 0),
                _size: quality_size as i32,
                _time: self.renderer.counter as i32,
            };

            // rpass.set_push_constants(
            //     wgpu::ShaderStages::VERTEX_FRAGMENT,
            //     0,
            //     push_constant.as_u8_slice(),
            // );

            let verts_per_water_tape = quality_size * 2 + 2;
            let tapes_per_block = quality_size;

            rpass.draw(0..verts_per_water_tape, 0..tapes_per_block);
        }
    }

    fn raygen_grass(&mut self) {
        // Begin the raygen grass render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Raygen Grass Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self
                        .renderer
                        .dependent_images
                        .as_ref()
                        .unwrap()
                        .highres_mat_norm
                        .current()
                        .view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self
                        .renderer
                        .dependent_images
                        .as_ref()
                        .unwrap()
                        .full_view_for_ds
                        .current(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        for frr in &self.foliage_que {
            let grass: &InternalMeshFoliage = &frr.mesh.0;
            let pos: &vec3 = &frr.pos;

            let size = 10;
            let x_flip = self.renderer.camera.camera_dir.x < 0.0;
            let y_flip = self.renderer.camera.camera_dir.y < 0.0;

            let pipe = &self.renderer.pipes.raygen_foliage_pipes[grass.stored_id as usize];
            let desc = &self.renderer.foliage_descriptions[grass.stored_id as usize];

            self.renderer.wal.bind_raster_pipeline(&mut rpass, &pipe);

            #[repr(C)] // for push constants
            #[derive(AsU8Slice)] // allow cast to &[u8]
            struct PushConstant {
                shift: vec4,
                _size: i32,
                _time: i32,
                xf: i32,
                yf: i32,
            }
            let push_constant = PushConstant {
                shift: vec4!(*pos, 0),
                _size: size as i32,
                _time: self.renderer.counter as i32,
                xf: x_flip as i32,
                yf: y_flip as i32,
            };

            // rpass.set_push_constants(
            //     wgpu::ShaderStages::VERTEX_FRAGMENT,
            //     0,
            //     push_constant.as_u8_slice(),
            // );

            let verts_per_blade = desc.vertices;
            let blade_per_instance = 1; //for triangle strip
            let instance_count = (size * size + (blade_per_instance - 1)) / blade_per_instance;

            rpass.draw(0..verts_per_blade * blade_per_instance, 0..instance_count);
        }
    }

    fn update_raygen_particles(&mut self) {
        {
            let this = &mut self.renderer;
            let mut write_index = 0;

            for i in 0..this.particles.len() {
                let should_keep = this.particles[i].life_time > 0.0;
                if should_keep {
                    this.particles[write_index] = this.particles[i];

                    let velocity = this.particles[write_index].vel;
                    this.particles[write_index].pos += velocity * this.delta_time;

                    this.particles[write_index].life_time -= this.delta_time;
                    write_index += 1;
                }
            }

            this.particles.shrink_to(write_index);
            let capped_particle_count =
                write_index.clamp(0, this.settings.max_particle_count as usize);

            // Update the GPU particle buffer with the current particle data
            if capped_particle_count > 0 {
                // Convert particle data to bytes
                let particle_bytes = unsafe {
                    std::slice::from_raw_parts(
                        this.particles.as_ptr() as *const u8,
                        capped_particle_count * std::mem::size_of::<Particle>(),
                    )
                };

                this.wal.queue.write_buffer(
                    &this.buffers.gpu_particles.current(),
                    0,
                    particle_bytes,
                );
            }
        };

        // Render the particles
        if !self.renderer.particles.is_empty() {
            let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Particle Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self
                            .renderer
                            .dependent_images
                            .as_ref()
                            .unwrap()
                            .highres_mat_norm
                            .current()
                            .view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self
                            .renderer
                            .dependent_images
                            .as_ref()
                            .unwrap()
                            .highres_depth_stencil
                            .current()
                            .view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                },
            );

            self.renderer
                .wal
                .bind_raster_pipeline(&mut rpass, &self.renderer.pipes.raygen_particles_pipe);

            rpass.set_vertex_buffer(0, self.renderer.buffers.gpu_particles.current().slice(..));

            rpass.draw(0..self.renderer.particles.len() as u32, 0..1);
        }
    }

    fn map_meshes(&mut self) {
        {
            // thing about wgpu is that they very much want to be pain in the ass, and after few hours i still did not figure out how to store passes
            // so here we fucking are, doing everything in a single scope and refactoring entire renderer to fit a non-gapi gapi
            let mut compute_pass =
                self.renderer.current_encoder.as_mut().unwrap().begin_compute_pass(
                    &wgpu::ComputePassDescriptor {
                        label: Some("Map Compute Pass"),
                        timestamp_writes: None,
                    },
                );

            self.renderer
                .wal
                .bind_compute_pipeline(&mut compute_pass, &self.renderer.pipes.map_pipe);

            for mrr in &self.model_que {
                let model_mesh = self.storage.models.get(mrr.mesh.0).unwrap();
                {
                    let trans: &MeshTransform = &mrr.trans;

                    // In Vulkan a push descriptor was used to push a descriptor referencing mesh.voxels.
                    // In WGPU you must update the bind group in advance. For example:
                    // (&mut self.renderer).update_map_bind_group(model_mesh);
                    // todo!("push model voxels to map");

                    // Compute the transformation matrices.
                    let rotate = mat4::from(trans.rotation);
                    let shift = mat4::identity().translated_3d(trans.translation);
                    let transform = shift * rotate;

                    // Compute border (in voxel space) using your helper get_shift.
                    let border_in_voxel = get_shift(transform, model_mesh.total_size);
                    let border = iAABB {
                        min: ivec3!(border_in_voxel.min.floor()),
                        max: ivec3!(border_in_voxel.max.ceil()),
                    };
                    let map_area = border.max - border.min;

                    #[repr(C)]
                    #[derive(Clone, Copy, AsU8Slice)]
                    struct PushConstant {
                        trans: mat4,
                        shift: ivec4,
                    }
                    let push_constant = PushConstant {
                        trans: transform.inverted(),
                        shift: ivec4!(border.min, 0),
                    };

                    assert!(self.renderer.pipes.map_pipe.static_bind_groups.is_some());

                    self.renderer.wal.dispatch_with_params(
                        &mut compute_pass,
                        &mut self.renderer.pipes.map_pipe,
                        Some(model_mesh.voxels_bind_group_compute.as_ref().unwrap()),
                        Some(push_constant.as_u8_slice()),
                        ((map_area.x + 3) as u32) / 4,
                        ((map_area.y + 3) as u32) / 4,
                        ((map_area.z + 3) as u32) / 4,
                    );
                }
            }
        };
    }

    fn lightmap_models(&mut self) {
        {
            // Begin the lightmap models render pass.

            let render_pass_desc = wgpu::RenderPassDescriptor {
                label: Some("Lightmap Models Render Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.independent_images.lightmap.current().view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            };

            let mut rpass = self
                .renderer
                .current_encoder
                .as_mut()
                .unwrap()
                .begin_render_pass(&render_pass_desc);

            for mrr in &self.model_que {
                let ipos = ivec3!(mrr.get_pos());
                {
                    let model_id = mrr.mesh;
                    let shift = ipos;

                    let model_mesh = &self.storage.models.get(model_id.0).unwrap();

                    rpass.set_vertex_buffer(
                        0,
                        model_mesh.triangles.vertexes.as_ref().unwrap().slice(..),
                    );
                    rpass.set_index_buffer(
                        model_mesh.triangles.indices.as_ref().unwrap().slice(..),
                        wgpu::IndexFormat::Uint16,
                    );

                    /*
                        int16_t block;
                        i16vec3 shift;
                        i8vec4 inorm;
                    */
                    #[repr(C)] // for push constants
                    #[derive(AsU8Slice)] // allow cast to &[u8]

                    struct PushConstant {
                        shift: i16vec4,
                    }
                    let push_constant = PushConstant {
                        shift: i16vec4!(shift, 0),
                    };
                    // rpass.set_push_constants(
                    //     wgpu::ShaderStages::VERTEX,
                    //     0,
                    //     push_constant.as_u8_slice(),
                    // );

                    macro_rules! CHECK_AND_DRAW_BLOCK_FACE {
                        ($__normal:expr, $__face:ident) => {
                            let fnorm = vec3!((i8vec3::new(1, 0, 0)));
                            let inorm = ivec3!((i8vec3::new(1, 0, 0)));
                            let is_visible = {
                                let camera_dir = self.renderer.camera.camera_dir;
                                fnorm.dot(camera_dir) < 0.0
                            };
                            if is_visible {
                                {
                                    let _normal = inorm;
                                    let buff: &IndexedVertices = &model_mesh.triangles.Pzz;
                                    rpass.draw_indexed(
                                        buff.offset..buff.offset + buff.icount,
                                        0 as i32,
                                        0..1,
                                    );
                                };
                            };
                        };
                    }

                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(1, 0, 0), Pzz);
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(-1, 0, 0), Nzz);
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 1, 0), zPz);
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, -1, 0), zNz);
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, 1), zzP);
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, -1), zzN);
                };
            }
        };
    }

    fn lightmap_blocks(&mut self) {
        let render_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Lightmap Blocks Render Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.renderer.independent_images.lightmap.current().view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        };
        let mut rpass = self
            .renderer
            .current_encoder
            .as_mut()
            .unwrap()
            .begin_render_pass(&render_pass_desc);
        for brr in &self.block_que {
            let ipos = ivec3!(brr.pos);
            {
                let block_id = brr.block;
                let shift = ipos;

                let block_mesh = &self.renderer.block_palette_meshes[block_id as usize];

                rpass.set_vertex_buffer(
                    0,
                    block_mesh.triangles.vertexes.as_ref().unwrap().slice(..),
                );
                rpass.set_index_buffer(
                    block_mesh.triangles.indices.as_ref().unwrap().slice(..),
                    wgpu::IndexFormat::Uint16,
                );

                /*
                    int16_t block;
                    i16vec3 shift;
                    i8vec4 inorm;
                */
                #[repr(C)] // for push constants
                #[derive(AsU8Slice)] // allow cast to &[u8]

                struct PushConstant {
                    shift: i16vec4,
                }
                let push_constant = PushConstant {
                    shift: i16vec4!(shift, 0),
                };
                // rpass.set_push_constants(
                //     wgpu::ShaderStages::VERTEX,
                //     0,
                //     push_constant.as_u8_slice(),
                // );

                macro_rules! CHECK_AND_DRAW_BLOCK_FACE {
                    ($__normal:expr, $__face:ident) => {
                        let fnorm = vec3!((i8vec3::new(1, 0, 0)));
                        let inorm = ivec3!((i8vec3::new(1, 0, 0)));
                        let is_visible = {
                            let camera_dir = self.renderer.camera.camera_dir;
                            fnorm.dot(camera_dir) < 0.0
                        };
                        if is_visible {
                            {
                                let _normal = inorm;
                                let buff: &IndexedVertices = &block_mesh.triangles.Pzz;
                                let _block_id = block_id;
                                rpass.draw_indexed(
                                    buff.offset..buff.offset + buff.icount,
                                    0 as i32,
                                    0..1,
                                );
                            };
                        };
                    };
                }

                CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(1, 0, 0), Pzz);
                CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(-1, 0, 0), Nzz);
                CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 1, 0), zPz);
                CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, -1, 0), zNz);
                CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, 1), zzP);
                CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, -1), zzN);
            };
        }
    }

    fn update_light_ubo(&mut self) {
        // Use a dedicated encoder for lightmap work (or the command encoder from your lightmap command-buffer ring).
        // Update the light uniform buffer with the light transform.
        #[repr(C)]
        #[derive(Clone, Copy, AsU8Slice)]
        struct BufferPatch {
            trans: mat4,
        }
        let buffer_patch = BufferPatch {
            trans: self.renderer.light.light_transform,
        };
        // Update the buffer via the queue.
        self.renderer.wal.queue.write_buffer(
            &self.renderer.buffers.light_uniform.current(),
            0,
            buffer_patch.as_u8_slice(),
        );
    }

    fn raygen_blocks(&mut self) {
        {
            // Begin the raygen blocks render pass
            let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Raygen Blocks Render Pass"),
                    // raster mat_norm gbuffers
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self
                            .renderer
                            .dependent_images
                            .as_ref()
                            .unwrap()
                            .highres_mat_norm
                            .current()
                            .view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            // first use clears, other just load
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    // depth is normal gbuffer depth
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self
                            .renderer
                            .dependent_images
                            .as_ref()
                            .unwrap()
                            .full_view_for_ds
                            .current(),
                        depth_ops: Some(wgpu::Operations {
                            // clear cause first
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                },
            );

            self.renderer
                .wal
                .bind_raster_pipeline(&mut rpass, &self.renderer.pipes.raygen_blocks_pipe);

            for brr in &self.block_que {
                let ipos = ivec3!(brr.pos);
                {
                    let block_id = brr.block;
                    let shift = ipos;

                    let block_mesh = &self.renderer.block_palette_meshes[block_id as usize];

                    rpass.set_vertex_buffer(
                        0,
                        block_mesh.triangles.vertexes.as_ref().unwrap().slice(..),
                    );
                    rpass.set_index_buffer(
                        block_mesh.triangles.indices.as_ref().unwrap().slice(..),
                        wgpu::IndexFormat::Uint16,
                    );

                    #[repr(C)] // for push constants
                    #[derive(AsU8Slice)] // allow cast to &[u8]
                    struct PushConstant {
                        block: BlockId,
                        shift: i16vec3,
                        // inorm: i8vec4, // passed separately
                    }

                    let push_constant = PushConstant {
                        block: block_id,
                        shift: i16vec3!(shift),
                    };

                    // rpass.set_push_constants(
                    //     wgpu::ShaderStages::VERTEX_FRAGMENT,
                    //     0,
                    //     push_constant.as_u8_slice(),
                    // );

                    // loving macros. IDK if C is better, i am not nearly as good in Rust as i am in C, but still cool
                    macro_rules! CHECK_AND_DRAW_BLOCK_FACE {
                        ($__normal:expr, $__face:ident) => {
                            let fnorm = vec3::new(
                                $__normal.x as f32,
                                $__normal.y as f32,
                                $__normal.z as f32,
                            );
                            let inorm =
                                ivec3!($__normal.x as i32, $__normal.y as i32, $__normal.z as i32);
                            if is_face_visible(fnorm, self.renderer.camera.camera_dir) {
                                Self::raygen_block_face(
                                    &mut rpass,
                                    inorm,
                                    &block_mesh.triangles.$__face,
                                    block_id,
                                );
                            }
                        };
                    }

                    // draw every face (separately). This allows per-face culling
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(1, 0, 0), Pzz);
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(-1, 0, 0), Nzz);
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 1, 0), zPz);
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, -1, 0), zNz);
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, 1), zzP);
                    CHECK_AND_DRAW_BLOCK_FACE!(i8vec3::new(0, 0, -1), zzN);
                }
            }
        };
    }

    fn update_ubo(&mut self) {
        // let this = &mut self.renderer;
        // Update the uniform buffer with camera and light properties.
        let horizline_scaled =
            self.renderer.camera.horizline * (self.renderer.camera.view_size.x / 2.0);
        let vertiline_scaled =
            self.renderer.camera.vertiline * (self.renderer.camera.view_size.y / 2.0);

        let buffer_patch = UboData {
            trans_w2s: self.renderer.camera.camera_transform,
            campos: vec4!(self.renderer.camera.camera_pos, 0.0),
            camdir: vec4!(self.renderer.camera.camera_dir, 0.0),
            horizline_scaled: vec4!(horizline_scaled, 0.0),
            vertiline_scaled: vec4!(vertiline_scaled, 0.0),
            global_light_dir: vec4!(self.renderer.light.light_dir, 0.0),
            lightmap_proj: self.renderer.light.light_transform,
            timeseed: 666,
            frame_size: Default::default(),
            wind_direction: Default::default(),
            delta_time: Default::default(),
            _pad_1: Default::default(),
            _pad_2: Default::default(),
        };
        self.renderer.wal.queue.write_buffer(
            &self.renderer.buffers.uniform.current(),
            0,
            buffer_patch.as_u8_slice(),
        );
    }

    pub fn raygen_block_face(
        rpass: &mut wgpu::RenderPass<'_>,
        normal: ivec3,
        buff: &IndexedVertices,
        block_id: BlockId,
    ) {
        debug_assert!(block_id > 0);
        let sum = normal.x + normal.y + normal.z;
        // u8 sign = (sum > 0) ? 0 : 1;
        let neg_sign = match sum > 0 {
            true => 0,
            false => 1,
        };

        let absnorm = u8vec3::new(
            normal.x.unsigned_abs() as u8,
            normal.y.unsigned_abs() as u8,
            normal.z.unsigned_abs() as u8,
        );
        debug_assert!((absnorm.x + absnorm.y + absnorm.z) == 1);
        let pbn = { (neg_sign << 7) | absnorm.x | (absnorm.y << 1) | (absnorm.z << 2) };
        //signBit_4EmptyBits_xBit_yBit_zBit
        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            // block: BlockID_t, // passed before separately
            // shift: i16vec3, // passed before separately
            inorm: u8vec4,
        }
        let push_constant = PushConstant {
            inorm: u8vec4::new(pbn, 0, 0, 0), // TODO: what the hell was i smoking?
        };
        debug_assert!(push_constant.as_u8_slice().len() == 4);

        // rpass.set_push_constants(
        //     wgpu::ShaderStages::VERTEX_FRAGMENT,
        //     8,
        //     push_constant.as_u8_slice(),
        // );
    }

    fn raygen_models(&mut self) {
        // Begin the raygen models render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Raygen Models Render Pass"),
                // raster mat_norm gbuffers
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self
                        .renderer
                        .dependent_images
                        .as_ref()
                        .unwrap()
                        .highres_mat_norm
                        .current()
                        .view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // because why the fuck would we erase raygen'ed blocks?
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // depth is normal gbuffer depth
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self
                        .renderer
                        .dependent_images
                        .as_ref()
                        .unwrap()
                        .full_view_for_ds
                        .current(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        self.renderer
            .wal
            .bind_raster_pipeline(&mut rpass, &self.renderer.pipes.raygen_models_pipe);

        for mrr in &self.model_que {
            let model_mesh = self.storage.models.get(mrr.mesh.0).unwrap();
            let model_trans: &MeshTransform = &mrr.trans;

            rpass.set_vertex_buffer(0, model_mesh.triangles.vertexes.as_ref().unwrap().slice(..));
            rpass.set_index_buffer(
                model_mesh.triangles.indices.as_ref().unwrap().slice(..),
                wgpu::IndexFormat::Uint16,
            );

            #[repr(C)] // for push constants
            #[derive(AsU8Slice)] // allow cast to &[u8]
            struct PushConstant {
                rot: quat,
                shift: vec4,
                // fnormal: vec4,
            }
            let push_constant = PushConstant {
                rot: model_trans.rotation,
                shift: vec4!(model_trans.translation, 0),
                // fnormal: vec4::new(normal.x, normal.y, normal.z, 0.0),
            };

            // rpass.set_push_constants(
            //     wgpu::ShaderStages::VERTEX_FRAGMENT,
            //     0,
            //     push_constant.as_u8_slice(),
            // );

            // Update the model voxels bind group if needed
            // This would replace the Vulkan descriptor set update
            // For now, we'll assume the bind group is already set up correctly

            macro_rules! CHECK_AND_DRAW_MODEL_FACE {
                ($__normal:expr, $__face:ident) => {
                    let fnorm =
                        vec3::new($__normal.x as f32, $__normal.y as f32, $__normal.z as f32);
                    if is_face_visible(
                        model_trans.rotation * fnorm,
                        self.renderer.camera.camera_dir,
                    ) {
                        Self::raygen_model_face(
                            &mut self.renderer.wal,
                            &mut self.renderer.pipes.raygen_models_pipe,
                            model_mesh.voxels_bind_group_fragment.as_ref().unwrap(),
                            &mut rpass,
                            fnorm,
                            &model_mesh.triangles.$__face,
                        );
                    }
                };
            }

            // let wal = &mut self.renderer.wal;
            CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(1, 0, 0), Pzz);
            CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(-1, 0, 0), Nzz);
            CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(0, 1, 0), zPz);
            CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(0, -1, 0), zNz);
            CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(0, 0, 1), zzP);
            CHECK_AND_DRAW_MODEL_FACE!(i8vec3::new(0, 0, -1), zzN);
        }
    }

    pub fn raygen_model_face(
        wal: &mut Wal,
        pipe: &mut RasterPipe,
        model_voxels_bg: &BindGroup,
        rpass: &mut wgpu::RenderPass<'_>,
        normal: vec3,
        buff: &IndexedVertices,
    ) {
        #[repr(C)] // for push constants
        #[derive(AsU8Slice)] // allow cast to &[u8]
        struct PushConstant {
            fnormal: vec4,
        }
        let push_constant = PushConstant {
            fnormal: vec4!(normal, 0.0),
        };

        // rpass.draw_indexed(buff.offset..buff.offset + buff.icount, 0, 0..1);
        wal.draw_indexed_with_params(
            rpass,
            pipe,
            Some(model_voxels_bg),
            Some(push_constant.as_u8_slice()),
            buff.offset..buff.offset + buff.icount,
            0,
            0..1,
        );
    }

    pub fn get_world_blocks(&self) -> &Array3D<BlockId> {
        &self.renderer.current_world
    }
    pub fn get_world_blocks_mut(&mut self) -> &mut Array3D<BlockId> {
        &mut self.renderer.current_world
    }

    fn diffuse(&mut self) {
        // Begin the diffuse render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Diffuse Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self
                        .renderer
                        .dependent_images
                        .as_ref()
                        .unwrap()
                        .highres_frame
                        .current()
                        .view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                // Some(wgpu::RenderPassDepthStencilAttachment {
                //     view: &self
                //         .renderer
                //         .dependent_images
                //         .as_ref()
                //         .unwrap()
                //         .full_view_for_ds
                //         .current(),
                //     depth_ops: Some(wgpu::Operations {
                //         load: wgpu::LoadOp::Load,
                //         store: wgpu::StoreOp::Store,
                //     }),
                //     stencil_ops: None,
                // }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        self.renderer
            .wal
            .bind_raster_pipeline(&mut rpass, &self.renderer.pipes.diffuse_pipe);

        #[repr(C)]
        #[derive(Clone, Copy, AsU8Slice)]
        struct PushConstant {
            v1: vec4,
            v2: vec4,
            lp: mat4,
        }

        let transmuted_frame = unsafe { std::mem::transmute::<i32, f32>(666) };
        let push_constant = PushConstant {
            v1: vec4!(self.renderer.camera.camera_pos, transmuted_frame),
            v2: vec4!(self.renderer.camera.camera_dir, 0),
            lp: self.renderer.light.light_transform,
        };

        // rpass.set_push_constants(
        //     wgpu::ShaderStages::VERTEX_FRAGMENT,
        //     0,
        //     push_constant.as_u8_slice(),
        // );

        // Draw fullscreen triangle
        rpass.draw(0..3, 0..1);
    }

    fn ambient_occlusion(&mut self) {
        // Begin the ambient occlusion render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Ambient Occlusion Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self
                        .renderer
                        .dependent_images
                        .as_ref()
                        .unwrap()
                        .highres_frame
                        .current()
                        .view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        self.renderer.wal.bind_raster_pipeline(&mut rpass, &self.renderer.pipes.ao_pipe);

        // Draw fullscreen triangle
        rpass.draw(0..3, 0..1);
    }

    fn glossy_raygen(&mut self) {
        // Begin the glossy raygen render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Glossy Raygen Render Pass"),
                // not really writing any color
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self
                        .renderer
                        .dependent_images
                        .as_ref()
                        .unwrap()
                        .full_view_for_ds
                        .current(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        self.renderer
            .wal
            .bind_raster_pipeline(&mut rpass, &self.renderer.pipes.fill_stencil_glossy_pipe);

        // Draw fullscreen triangle
        rpass.draw(0..3, 0..1);
    }

    fn raygen_smoke(&mut self) {
        // Begin the smoke raygen render pass
        let mut rpass = self.renderer.current_encoder.as_mut().unwrap().begin_render_pass(
            &wgpu::RenderPassDescriptor {
                label: Some("Smoke Raygen Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self
                            .renderer
                            .dependent_images
                            .as_ref()
                            .unwrap()
                            .near_depth
                            .current()
                            .view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self
                            .renderer
                            .dependent_images
                            .as_ref()
                            .unwrap()
                            .far_depth
                            .current()
                            .view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self
                        .renderer
                        .dependent_images
                        .as_ref()
                        .unwrap()
                        .full_view_for_ds
                        .current(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            },
        );

        self.renderer
            .wal
            .bind_raster_pipeline(&mut rpass, &self.renderer.pipes.fill_stencil_smoke_pipe);

        // Draw fullscreen triangle for initial setup
        rpass.draw(0..3, 0..1);

        // Now map each volumetric mesh
        for vrr in &self.volumetric_que {
            let volumetric_mesh = self.storage.volumetrics.get(vrr.mesh.0).unwrap();

            #[repr(C)]
            #[derive(Clone, Copy, AsU8Slice)]
            struct PushConstant {
                center_size: vec4,
            }

            let push_constant = PushConstant {
                center_size: vec4!(vrr.pos, 1.0),
            };

            // rpass.set_push_constants(
            //     wgpu::ShaderStages::VERTEX_FRAGMENT,
            //     0,
            //     push_constant.as_u8_slice(),
            // );

            // Draw cube (36 vertices)
            self.renderer.wal.draw_with_params(
                &mut rpass,
                &mut self.renderer.pipes.fill_stencil_smoke_pipe,
                None,
                Some(push_constant.as_u8_slice()),
                0..36,
                0..1,
            );
        }
    }

    fn tonemap(&mut self) {
        let cmb = self.renderer.current_encoder.take().unwrap().finish();
        // sub all prev commands
        self.renderer.wal.queue.submit([cmb]);

        let mut current_encoder =
            self.renderer
                .wal
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Frame Command Encoder"),
                });

        // Create texture view
        let swapchain_texture = self
            .renderer
            .wal
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let swapchain_view = swapchain_texture.texture.create_view(&wgpu::TextureViewDescriptor {
            // Without add_srgb_suffix() the image we will be working with
            // might not be "gamma correct".
            format: Some(self.renderer.wal.config.format.add_srgb_suffix()),
            ..Default::default()
        });
        {
            let mut rpass = current_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tonemap Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &swapchain_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.renderer
                .wal
                .bind_raster_pipeline(&mut rpass, &self.renderer.pipes.tonemap_pipe);

            // Draw fullscreen triangle
            rpass.draw(0..3, 0..1);
        }
        self.renderer.wal.queue.submit(Some(current_encoder.finish()));
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

fn is_face_visible(normal: vec3, camera_dir: vec3) -> bool {
    normal.dot(camera_dir) < 0.0
}
