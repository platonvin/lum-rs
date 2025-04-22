use block_mesh::{greedy_quads, GreedyQuadsBuffer};
use lumal::atrace;

use crate::{
    containers::Array3D,
    types::{
        u8vec3, uvec3, vec3, BlockId, FaceBuffers, IndexedVertices, InternalMeshBlock,
        InternalMeshModel, PackedVoxelCircuit, Voxel, VoxelForContour,
    },
};

use super::ogt_vox;

pub trait LoadInterface {
    type BufferType;
    type ImageType;

    fn update_block_palette_to_gpu(&mut self);
    fn update_material_palette_to_gpu(&mut self);

    fn load_mesh_from_memory(
        &mut self,
        model: &ogt_vox::VoxModel,
        _make_vertices: bool,
    ) -> InternalMeshModel<Self::BufferType, Self::ImageType>;

    // i love that we can implement functions in traits
    #[cold]
    #[optimize(size)]
    fn load_mesh_from_file(
        &mut self,
        mesh_file: &str,
        _make_vertices: bool,
        extrude_palette: bool,
    ) -> InternalMeshModel<Self::BufferType, Self::ImageType> {
        let scene = ogt_vox::read_scene_from_file(mesh_file).unwrap();
        assert!(scene.models.len() == 1); // only one model per file supported for now
        let model = &scene.models[0];
        assert!(model.size_x > 0 && model.size_y > 0 && model.size_z > 0);

        if extrude_palette && !self.has_palette() {
            println!("Extruding palette");
            self.extract_palette_from_scene(&scene);
            self.set_has_palette(true);
        }

        self.load_mesh_from_memory(model, true)
    }

    #[cold]
    #[optimize(size)]
    fn load_meshes_from_file(
        &mut self,
        meshes_file: &str,
        _make_vertices: bool,
        extrude_palette: bool,
    ) -> Vec<InternalMeshModel<Self::BufferType, Self::ImageType>> {
        let scene = ogt_vox::read_scene_from_file(meshes_file).unwrap();

        if extrude_palette && !self.has_palette() {
            println!("Extruding palette");
            self.extract_palette_from_scene(&scene);
            self.set_has_palette(true);
        }

        scene
            .models
            .iter()
            .map(|model| {
                assert!(model.size_x > 0 && model.size_y > 0 && model.size_z > 0);

                self.load_mesh_from_memory(model, true)
            })
            .collect()
    }

    #[cold]
    #[optimize(size)]
    fn load_block_from_file(&mut self, block: BlockId, path: &str) {
        let scene = ogt_vox::read_scene_from_file(path).unwrap();
        assert!(scene.models.len() == 1); // only one model per file supported for now
                                          // blocks are always 16x16x16
        let model = &scene.models[0];
        assert!(model.size_x == 16 && model.size_y == 16 && model.size_z == 16);
        self.load_block_from_memory(block, model);
    }

    #[cold]
    #[optimize(size)]
    fn load_block_from_memory(&mut self, block_id: BlockId, model: &ogt_vox::VoxModel) {
        let size = uvec3::new(model.size_x, model.size_y, model.size_z);

        let mut padded_voxel_data = Array3D::<VoxelForContour>::new(
            // +2 cause padding of 1 from each side
            (size.x + 2) as usize,
            (size.y + 2) as usize,
            (size.z + 2) as usize,
        );
        padded_voxel_data.data.fill(VoxelForContour(0));

        for xx in 0..size.x {
            for yy in 0..size.y {
                for zz in 0..size.z {
                    let voxel = model.voxel_data[(xx + yy * size.x + zz * size.x * size.y) as usize]
                        as Voxel;
                    // some padding for generator
                    padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)] =
                        VoxelForContour(voxel);
                }
            }
        }

        // yep, there is padding. Its to reuse memory. TODO: find nicer approach
        assert!(size.x == 16 && size.y == 16 && size.z == 16);
        for zz in 0..size.z {
            for yy in 0..size.y {
                for xx in 0..size.x {
                    self.set_block_palette_voxels(
                        block_id,
                        uvec3::new(xx, yy, zz),
                        padded_voxel_data[(xx as usize + 1, yy as usize + 1, zz as usize + 1)].0,
                    );
                }
            }
        }

        let triangles = self.make_contour_vertices(size, padded_voxel_data);

        self.set_block_palette_mesh(block_id, InternalMeshBlock { triangles });
    }

    fn set_block_palette_voxels(&mut self, block_id: BlockId, pos: uvec3, voxel: Voxel);
    fn get_block_palette_voxels(&self, block_id: BlockId, pos: uvec3) -> Voxel;

    fn set_block_palette_mesh(
        &mut self,
        block_id: BlockId,
        mesh: InternalMeshBlock<Self::BufferType>,
    );
    fn get_block_palette_mesh(&self, block_id: BlockId) -> &InternalMeshBlock<Self::BufferType>;

    #[cold]
    #[optimize(size)]
    fn make_contour_vertices(
        &mut self,
        // real size. TODO: do i need this?
        size: uvec3,
        // 3d array with 1 padding
        padded_voxel_data: Array3D<VoxelForContour>,
    ) -> FaceBuffers<Self::BufferType> {
        let mut buffer = GreedyQuadsBuffer::new(padded_voxel_data.data.len());

        // TODO: issue on block_mesh bad readme example
        let chunk_shape =
            block_mesh::ndshape::RuntimeShape::<u32, 3>::new([size.x + 2, size.y + 2, size.z + 2]);

        let faces = block_mesh::RIGHT_HANDED_Y_UP_CONFIG.faces;
        greedy_quads(
            padded_voxel_data.data.as_slice(),
            &chunk_shape,
            [0; 3],
            [size.x + 1, size.y + 1, size.z + 1],
            &faces,
            &mut buffer,
        );

        assert!(buffer.quads.num_quads() > 0);

        let num_indices = buffer.quads.num_quads() * 6;
        let num_vertices = buffer.quads.num_quads() * 4;
        // [0,1,2] [1,2,3] - indices of vertices in vertex array
        // each sequential three indices form a (single)triangle
        // triangles are made by mesher (block_mesh) from voxels
        let mut indices = Vec::with_capacity(num_indices);
        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);

        // problem with block_mesh is that even tho it is voxel, values are still in
        // floats so for now we repack & convert them
        // TODO: fork, fix and optimize
        for (group, face) in buffer.quads.groups.into_iter().zip(faces.into_iter()) {
            for quad in group.into_iter() {
                indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                positions.extend_from_slice(&face.quad_mesh_positions(&quad, 1.0));
                normals.extend_from_slice(&face.quad_mesh_normals());
            }
        }

        assert!(positions.len() == normals.len());
        // positions only!
        // normals are passed as push constants and defined in high-level (look down
        // below)
        let mut circ_verts = vec![PackedVoxelCircuit::default(); positions.len()];
        for i in 0..positions.len() {
            let u8pos = u8vec3::new(
                // substract 1 cause contour 1 padding
                positions[i][0] as u8 - 1,
                positions[i][1] as u8 - 1,
                positions[i][2] as u8 - 1,
            );
            circ_verts[i].pos = u8pos;
        }

        #[allow(non_snake_case)]
        let mut verts_idxs_Pzz = Vec::with_capacity(positions.len());
        #[allow(non_snake_case)]
        let mut verts_idxs_Nzz = Vec::with_capacity(positions.len());
        #[allow(non_snake_case)]
        let mut verts_idxs_zPz = Vec::with_capacity(positions.len());
        #[allow(non_snake_case)]
        let mut verts_idxs_zNz = Vec::with_capacity(positions.len());
        #[allow(non_snake_case)]
        let mut verts_idxs_zzP = Vec::with_capacity(positions.len());
        #[allow(non_snake_case)]
        let mut verts_idxs_zzN = Vec::with_capacity(positions.len());

        // TODO: how to return a ref to local_but_higher_scope variable?
        #[rustfmt::skip]
        let mut push_index_to_corresponding_vec = |normal: vec3, index: u16| {
            match normal {
                vec3 {x:  1.0, y:  0.0, z:  0.0} => {verts_idxs_Pzz.push(index);},
                vec3 {x: -1.0, y:  0.0, z:  0.0} => {verts_idxs_Nzz.push(index);},
                vec3 {x:  0.0, y:  1.0, z:  0.0} => {verts_idxs_zPz.push(index);},
                vec3 {x:  0.0, y: -1.0, z:  0.0} => {verts_idxs_zNz.push(index);},
                vec3 {x:  0.0, y:  0.0, z:  1.0} => {verts_idxs_zzP.push(index);},
                vec3 {x:  0.0, y:  0.0, z: -1.0} => {verts_idxs_zzN.push(index);},
                _ => {
                    panic!("Unknown normal: {:?}", normal);
                },
            }
        };
        // dbg!(&indices);
        for i in 0..indices.len() {
            let index = indices[i];
            // the first one in triangle. This is the one that points to vertex that is the
            // Provoking Vertex (google it) which means that when all 3 pass
            // some some value to fragment shader with flat qualifier (no interpolation),
            // Provoking Vertex's one is used
            let provoking_index = indices[(i / 3) * 3];
            // TODO: should i checks that they all actualyl have same normal?
            let norm = normals[provoking_index as usize];
            push_index_to_corresponding_vec(norm.into(), index as u16);
        }

        assert!(!verts_idxs_Pzz.is_empty());
        assert!(!verts_idxs_Nzz.is_empty());
        assert!(!verts_idxs_zPz.is_empty());
        assert!(!verts_idxs_zNz.is_empty());
        assert!(!verts_idxs_zzP.is_empty());
        assert!(!verts_idxs_zzN.is_empty());

        let mut all_indices = vec![];
        let mut offset_and_insert = |vec: &mut Vec<u16>, section: &mut IndexedVertices| {
            // starts at current length
            section.offset = all_indices.len() as u32;
            // continues for length of verts_idxs vec
            section.icount = vec.len() as u32;
            all_indices.extend_from_slice(vec.as_slice());
        };

        #[allow(non_snake_case)]
        {
            let mut triangles_Pzz = IndexedVertices::default();
            let mut triangles_Nzz = IndexedVertices::default();
            let mut triangles_zPz = IndexedVertices::default();
            let mut triangles_zNz = IndexedVertices::default();
            let mut triangles_zzP = IndexedVertices::default();
            let mut triangles_zzN = IndexedVertices::default();

            offset_and_insert(&mut verts_idxs_Pzz, &mut triangles_Pzz);
            offset_and_insert(&mut verts_idxs_Nzz, &mut triangles_Nzz);
            offset_and_insert(&mut verts_idxs_zPz, &mut triangles_zPz);
            offset_and_insert(&mut verts_idxs_zNz, &mut triangles_zNz);
            offset_and_insert(&mut verts_idxs_zzP, &mut triangles_zzP);
            offset_and_insert(&mut verts_idxs_zzN, &mut triangles_zzN);

            let (vertexes, indices) =
                self.create_and_upload_contour_buffers(&circ_verts, &all_indices);
            FaceBuffers::<Self::BufferType> {
                Pzz: triangles_Pzz,
                Nzz: triangles_Nzz,
                zPz: triangles_zPz,
                zNz: triangles_zNz,
                zzP: triangles_zzP,
                zzN: triangles_zzN,
                vertexes,
                indices,
            }
        }
    }

    fn create_and_upload_contour_buffers(
        &mut self,
        verts: &[PackedVoxelCircuit],
        indices: &[u16],
    ) -> (Self::BufferType, Self::BufferType);

    fn create_rayrace_voxel_image(
        &mut self,
        voxels: &[Voxel],
        size: uvec3,
        #[cfg(feature = "debug_validation_names")] debug_name: Option<&str>,
    ) -> Self::ImageType;

    fn free_mesh(&mut self, mesh: InternalMeshModel<Self::BufferType, Self::ImageType>);

    fn free_block(&mut self, block: crate::types::BlockId);

    fn extract_palette_from_scene(&mut self, scene: &ogt_vox::VoxScene);
    fn has_palette(&self) -> bool;
    fn set_has_palette(&mut self, has_palette: bool);
}
