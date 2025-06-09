use std::collections::HashMap;

use vek::{Mat4, Vec3, Vec4};

// omg i should include binary instead i think
static K_DEFAULT_VOX_PALETTE: [u8; 256 * 4] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xcc, 0xff, 0xff, 0xff, 0x99, 0xff, 0xff, 0xff, 0x66, 0xff,
    0xff, 0xff, 0x33, 0xff, 0xff, 0xff, 0x00, 0xff, 0xff, 0xcc, 0xff, 0xff, 0xff, 0xcc, 0xcc, 0xff,
    0xff, 0xcc, 0x99, 0xff, 0xff, 0xcc, 0x66, 0xff, 0xff, 0xcc, 0x33, 0xff, 0xff, 0xcc, 0x00, 0xff,
    0xff, 0x99, 0xff, 0xff, 0xff, 0x99, 0xcc, 0xff, 0xff, 0x99, 0x99, 0xff, 0xff, 0x99, 0x66, 0xff,
    0xff, 0x99, 0x33, 0xff, 0xff, 0x99, 0x00, 0xff, 0xff, 0x66, 0xff, 0xff, 0xff, 0x66, 0xcc, 0xff,
    0xff, 0x66, 0x99, 0xff, 0xff, 0x66, 0x66, 0xff, 0xff, 0x66, 0x33, 0xff, 0xff, 0x66, 0x00, 0xff,
    0xff, 0x33, 0xff, 0xff, 0xff, 0x33, 0xcc, 0xff, 0xff, 0x33, 0x99, 0xff, 0xff, 0x33, 0x66, 0xff,
    0xff, 0x33, 0x33, 0xff, 0xff, 0x33, 0x00, 0xff, 0xff, 0x00, 0xff, 0xff, 0xff, 0x00, 0xcc, 0xff,
    0xff, 0x00, 0x99, 0xff, 0xff, 0x00, 0x66, 0xff, 0xff, 0x00, 0x33, 0xff, 0xff, 0x00, 0x00, 0xff,
    0xcc, 0xff, 0xff, 0xff, 0xcc, 0xff, 0xcc, 0xff, 0xcc, 0xff, 0x99, 0xff, 0xcc, 0xff, 0x66, 0xff,
    0xcc, 0xff, 0x33, 0xff, 0xcc, 0xff, 0x00, 0xff, 0xcc, 0xcc, 0xff, 0xff, 0xcc, 0xcc, 0xcc, 0xff,
    0xcc, 0xcc, 0x99, 0xff, 0xcc, 0xcc, 0x66, 0xff, 0xcc, 0xcc, 0x33, 0xff, 0xcc, 0xcc, 0x00, 0xff,
    0xcc, 0x99, 0xff, 0xff, 0xcc, 0x99, 0xcc, 0xff, 0xcc, 0x99, 0x99, 0xff, 0xcc, 0x99, 0x66, 0xff,
    0xcc, 0x99, 0x33, 0xff, 0xcc, 0x99, 0x00, 0xff, 0xcc, 0x66, 0xff, 0xff, 0xcc, 0x66, 0xcc, 0xff,
    0xcc, 0x66, 0x99, 0xff, 0xcc, 0x66, 0x66, 0xff, 0xcc, 0x66, 0x33, 0xff, 0xcc, 0x66, 0x00, 0xff,
    0xcc, 0x33, 0xff, 0xff, 0xcc, 0x33, 0xcc, 0xff, 0xcc, 0x33, 0x99, 0xff, 0xcc, 0x33, 0x66, 0xff,
    0xcc, 0x33, 0x33, 0xff, 0xcc, 0x33, 0x00, 0xff, 0xcc, 0x00, 0xff, 0xff, 0xcc, 0x00, 0xcc, 0xff,
    0xcc, 0x00, 0x99, 0xff, 0xcc, 0x00, 0x66, 0xff, 0xcc, 0x00, 0x33, 0xff, 0xcc, 0x00, 0x00, 0xff,
    0x99, 0xff, 0xff, 0xff, 0x99, 0xff, 0xcc, 0xff, 0x99, 0xff, 0x99, 0xff, 0x99, 0xff, 0x66, 0xff,
    0x99, 0xff, 0x33, 0xff, 0x99, 0xff, 0x00, 0xff, 0x99, 0xcc, 0xff, 0xff, 0x99, 0xcc, 0xcc, 0xff,
    0x99, 0xcc, 0x99, 0xff, 0x99, 0xcc, 0x66, 0xff, 0x99, 0xcc, 0x33, 0xff, 0x99, 0xcc, 0x00, 0xff,
    0x99, 0x99, 0xff, 0xff, 0x99, 0x99, 0xcc, 0xff, 0x99, 0x99, 0x99, 0xff, 0x99, 0x99, 0x66, 0xff,
    0x99, 0x99, 0x33, 0xff, 0x99, 0x99, 0x00, 0xff, 0x99, 0x66, 0xff, 0xff, 0x99, 0x66, 0xcc, 0xff,
    0x99, 0x66, 0x99, 0xff, 0x99, 0x66, 0x66, 0xff, 0x99, 0x66, 0x33, 0xff, 0x99, 0x66, 0x00, 0xff,
    0x99, 0x33, 0xff, 0xff, 0x99, 0x33, 0xcc, 0xff, 0x99, 0x33, 0x99, 0xff, 0x99, 0x33, 0x66, 0xff,
    0x99, 0x33, 0x33, 0xff, 0x99, 0x33, 0x00, 0xff, 0x99, 0x00, 0xff, 0xff, 0x99, 0x00, 0xcc, 0xff,
    0x99, 0x00, 0x99, 0xff, 0x99, 0x00, 0x66, 0xff, 0x99, 0x00, 0x33, 0xff, 0x99, 0x00, 0x00, 0xff,
    0x66, 0xff, 0xff, 0xff, 0x66, 0xff, 0xcc, 0xff, 0x66, 0xff, 0x99, 0xff, 0x66, 0xff, 0x66, 0xff,
    0x66, 0xff, 0x33, 0xff, 0x66, 0xff, 0x00, 0xff, 0x66, 0xcc, 0xff, 0xff, 0x66, 0xcc, 0xcc, 0xff,
    0x66, 0xcc, 0x99, 0xff, 0x66, 0xcc, 0x66, 0xff, 0x66, 0xcc, 0x33, 0xff, 0x66, 0xcc, 0x00, 0xff,
    0x66, 0x99, 0xff, 0xff, 0x66, 0x99, 0xcc, 0xff, 0x66, 0x99, 0x99, 0xff, 0x66, 0x99, 0x66, 0xff,
    0x66, 0x99, 0x33, 0xff, 0x66, 0x99, 0x00, 0xff, 0x66, 0x66, 0xff, 0xff, 0x66, 0x66, 0xcc, 0xff,
    0x66, 0x66, 0x99, 0xff, 0x66, 0x66, 0x66, 0xff, 0x66, 0x66, 0x33, 0xff, 0x66, 0x66, 0x00, 0xff,
    0x66, 0x33, 0xff, 0xff, 0x66, 0x33, 0xcc, 0xff, 0x66, 0x33, 0x99, 0xff, 0x66, 0x33, 0x66, 0xff,
    0x66, 0x33, 0x33, 0xff, 0x66, 0x33, 0x00, 0xff, 0x66, 0x00, 0xff, 0xff, 0x66, 0x00, 0xcc, 0xff,
    0x66, 0x00, 0x99, 0xff, 0x66, 0x00, 0x66, 0xff, 0x66, 0x00, 0x33, 0xff, 0x66, 0x00, 0x00, 0xff,
    0x33, 0xff, 0xff, 0xff, 0x33, 0xff, 0xcc, 0xff, 0x33, 0xff, 0x99, 0xff, 0x33, 0xff, 0x66, 0xff,
    0x33, 0xff, 0x33, 0xff, 0x33, 0xff, 0x00, 0xff, 0x33, 0xcc, 0xff, 0xff, 0x33, 0xcc, 0xcc, 0xff,
    0x33, 0xcc, 0x99, 0xff, 0x33, 0xcc, 0x66, 0xff, 0x33, 0xcc, 0x33, 0xff, 0x33, 0xcc, 0x00, 0xff,
    0x33, 0x99, 0xff, 0xff, 0x33, 0x99, 0xcc, 0xff, 0x33, 0x99, 0x99, 0xff, 0x33, 0x99, 0x66, 0xff,
    0x33, 0x99, 0x33, 0xff, 0x33, 0x99, 0x00, 0xff, 0x33, 0x66, 0xff, 0xff, 0x33, 0x66, 0xcc, 0xff,
    0x33, 0x66, 0x99, 0xff, 0x33, 0x66, 0x66, 0xff, 0x33, 0x66, 0x33, 0xff, 0x33, 0x66, 0x00, 0xff,
    0x33, 0x33, 0xff, 0xff, 0x33, 0x33, 0xcc, 0xff, 0x33, 0x33, 0x99, 0xff, 0x33, 0x33, 0x66, 0xff,
    0x33, 0x33, 0x33, 0xff, 0x33, 0x33, 0x00, 0xff, 0x33, 0x00, 0xff, 0xff, 0x33, 0x00, 0xcc, 0xff,
    0x33, 0x00, 0x99, 0xff, 0x33, 0x00, 0x66, 0xff, 0x33, 0x00, 0x33, 0xff, 0x33, 0x00, 0x00, 0xff,
    0x00, 0xff, 0xff, 0xff, 0x00, 0xff, 0xcc, 0xff, 0x00, 0xff, 0x99, 0xff, 0x00, 0xff, 0x66, 0xff,
    0x00, 0xff, 0x33, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00, 0xcc, 0xff, 0xff, 0x00, 0xcc, 0xcc, 0xff,
    0x00, 0xcc, 0x99, 0xff, 0x00, 0xcc, 0x66, 0xff, 0x00, 0xcc, 0x33, 0xff, 0x00, 0xcc, 0x00, 0xff,
    0x00, 0x99, 0xff, 0xff, 0x00, 0x99, 0xcc, 0xff, 0x00, 0x99, 0x99, 0xff, 0x00, 0x99, 0x66, 0xff,
    0x00, 0x99, 0x33, 0xff, 0x00, 0x99, 0x00, 0xff, 0x00, 0x66, 0xff, 0xff, 0x00, 0x66, 0xcc, 0xff,
    0x00, 0x66, 0x99, 0xff, 0x00, 0x66, 0x66, 0xff, 0x00, 0x66, 0x33, 0xff, 0x00, 0x66, 0x00, 0xff,
    0x00, 0x33, 0xff, 0xff, 0x00, 0x33, 0xcc, 0xff, 0x00, 0x33, 0x99, 0xff, 0x00, 0x33, 0x66, 0xff,
    0x00, 0x33, 0x33, 0xff, 0x00, 0x33, 0x00, 0xff, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00, 0xcc, 0xff,
    0x00, 0x00, 0x99, 0xff, 0x00, 0x00, 0x66, 0xff, 0x00, 0x00, 0x33, 0xff, 0xee, 0x00, 0x00, 0xff,
    0xdd, 0x00, 0x00, 0xff, 0xbb, 0x00, 0x00, 0xff, 0xaa, 0x00, 0x00, 0xff, 0x88, 0x00, 0x00, 0xff,
    0x77, 0x00, 0x00, 0xff, 0x55, 0x00, 0x00, 0xff, 0x44, 0x00, 0x00, 0xff, 0x22, 0x00, 0x00, 0xff,
    0x11, 0x00, 0x00, 0xff, 0x00, 0xee, 0x00, 0xff, 0x00, 0xdd, 0x00, 0xff, 0x00, 0xbb, 0x00, 0xff,
    0x00, 0xaa, 0x00, 0xff, 0x00, 0x88, 0x00, 0xff, 0x00, 0x77, 0x00, 0xff, 0x00, 0x55, 0x00, 0xff,
    0x00, 0x44, 0x00, 0xff, 0x00, 0x22, 0x00, 0xff, 0x00, 0x11, 0x00, 0xff, 0x00, 0x00, 0xee, 0xff,
    0x00, 0x00, 0xdd, 0xff, 0x00, 0x00, 0xbb, 0xff, 0x00, 0x00, 0xaa, 0xff, 0x00, 0x00, 0x88, 0xff,
    0x00, 0x00, 0x77, 0xff, 0x00, 0x00, 0x55, 0xff, 0x00, 0x00, 0x44, 0xff, 0x00, 0x00, 0x22, 0xff,
    0x00, 0x00, 0x11, 0xff, 0xee, 0xee, 0xee, 0xff, 0xdd, 0xdd, 0xdd, 0xff, 0xbb, 0xbb, 0xbb, 0xff,
    0xaa, 0xaa, 0xaa, 0xff, 0x88, 0x88, 0x88, 0xff, 0x77, 0x77, 0x77, 0xff, 0x55, 0x55, 0x55, 0xff,
    0x44, 0x44, 0x44, 0xff, 0x22, 0x22, 0x22, 0xff, 0x11, 0x11, 0x11, 0xff, 0x00, 0x00, 0x00, 0xff,
];

const fn make_vox_chunk_id(c0: u8, c1: u8, c2: u8, c3: u8) -> u32 {
    (c0 as u32) | (c1 as u32) << 8 | (c2 as u32) << 16 | (c3 as u32) << 24
}

// Define the chunk IDs as constants
pub const CHUNK_ID_VOX_: u32 = make_vox_chunk_id(b'V', b'O', b'X', b' ');
pub const CHUNK_ID_MAIN: u32 = make_vox_chunk_id(b'M', b'A', b'I', b'N');
pub const CHUNK_ID_SIZE: u32 = make_vox_chunk_id(b'S', b'I', b'Z', b'E');
pub const CHUNK_ID_XYZI: u32 = make_vox_chunk_id(b'X', b'Y', b'Z', b'I');
pub const CHUNK_ID_RGBA: u32 = make_vox_chunk_id(b'R', b'G', b'B', b'A');
pub const CHUNK_ID_N_TRN: u32 = make_vox_chunk_id(b'n', b'T', b'R', b'N');
pub const CHUNK_ID_N_GRP: u32 = make_vox_chunk_id(b'n', b'G', b'R', b'P');
pub const CHUNK_ID_N_SHP: u32 = make_vox_chunk_id(b'n', b'S', b'H', b'P');
pub const CHUNK_ID_IMAP: u32 = make_vox_chunk_id(b'I', b'M', b'A', b'P');
pub const CHUNK_ID_LAYR: u32 = make_vox_chunk_id(b'L', b'A', b'Y', b'R');
pub const CHUNK_ID_MATL: u32 = make_vox_chunk_id(b'M', b'A', b'T', b'L');
pub const CHUNK_ID_MATT: u32 = make_vox_chunk_id(b'M', b'A', b'T', b'T');
pub const CHUNK_ID_R_OBJ: u32 = make_vox_chunk_id(b'r', b'O', b'B', b'J');
pub const CHUNK_ID_R_CAM: u32 = make_vox_chunk_id(b'r', b'C', b'A', b'M');

// flags for vox_read_scene_with_flags
pub const K_READ_SCENE_FLAGS_GROUPS: u32 = 1 << 0; // if not specified, all instance transforms will be flattened into world space. If specified, will read group information and keep all transforms as local transform relative to the group they are in.
pub const K_READ_SCENE_FLAGS_KEYFRAMES: u32 = 1 << 1; // if specified, all instances and groups will contain keyframe data.
pub const K_READ_SCENE_FLAGS_KEEP_EMPTY_MODELS_INSTANCES: u32 = 1 << 2; // if specified, all empty models and instances referencing those will be kept rather than culled.
pub const K_READ_SCENE_FLAGS_KEEP_DUPLICATE_MODELS: u32 = 1 << 3; // if specified, we do not de-duplicate models.

pub const K_VOX_MATL_HAVE_METAL: u32 = 1 << 0;
pub const K_VOX_MATL_HAVE_ROUGH: u32 = 1 << 1;
pub const K_VOX_MATL_HAVE_SPEC: u32 = 1 << 2;
pub const K_VOX_MATL_HAVE_IOR: u32 = 1 << 3;
pub const K_VOX_MATL_HAVE_ATT: u32 = 1 << 4;
pub const K_VOX_MATL_HAVE_FLUX: u32 = 1 << 5;
pub const K_VOX_MATL_HAVE_EMIT: u32 = 1 << 6;
pub const K_VOX_MATL_HAVE_LDR: u32 = 1 << 7;
pub const K_VOX_MATL_HAVE_TRANS: u32 = 1 << 8;
pub const K_VOX_MATL_HAVE_ALPHA: u32 = 1 << 9;
pub const K_VOX_MATL_HAVE_D: u32 = 1 << 10;
pub const K_VOX_MATL_HAVE_SP: u32 = 1 << 11;
pub const K_VOX_MATL_HAVE_G: u32 = 1 << 12;
pub const K_VOX_MATL_HAVE_MEDIA: u32 = 1 << 13;

pub const K_INVALID_GROUP_INDEX: u32 = u32::MAX;

const K_VECTORS: [Vec3<f32>; 3] = [
    Vec3::new(1.0, 0.0, 0.0),
    Vec3::new(0.0, 1.0, 0.0),
    Vec3::new(0.0, 0.0, 1.0),
];
const K_ROW2_INDEX: [usize; 8] = [2, 2, 1, usize::MAX, 0, 0, 1, usize::MAX];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VoxModel {
    pub size_x: u32,
    pub size_y: u32,
    pub size_z: u32,
    pub voxel_hash: u32,
    pub voxel_data: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct VoxInstance {
    pub name: String,
    pub transform: Mat4<f32>,
    pub model_index: u32,
    pub layer_index: u32,
    pub group_index: u32,
    pub hidden: bool,
    // pub transform_anim: ogt_vox_anim_transform,
    // pub model_anim: ogt_vox_anim_model,
}

#[derive(Debug, Clone, Default)]
pub struct VoxLayer {
    pub name: String,
    pub color: Vec4<u8>,
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum MatlType {
    #[default]
    Diffuse = 0,
    Metal = 1,
    Glass = 2,
    Emit = 3,
    Blend = 4,
    Media = 5,
    None,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VoxMatl {
    pub content_flags: u32,
    pub type_: MatlType,
    pub metal: f32,
    pub rough: f32,
    pub spec: f32,
    pub ior: f32,
    pub att: f32,
    pub flux: f32,
    pub emit: f32,
    pub ldr: f32,
    pub trans: f32,
    pub alpha: f32,
    pub d: f32,
    pub sp: f32,
    pub g: f32,
    pub media: f32,
}

#[derive(Debug, Clone, Default)]
pub struct VoxGroup {
    pub name: String,
    pub transform: Mat4<f32>,
    pub parent_group_index: u32,
    pub layer_index: u32,
    pub hidden: bool,
    // pub transform_anim: ogt_vox_anim_transform,
}
#[derive(Debug, Clone)]
pub struct VoxPalette {
    pub color: [Vec4<u8>; 256usize],
}
impl Default for VoxPalette {
    fn default() -> Self {
        VoxPalette {
            color: [Vec4::default(); 256],
        }
    }
}

#[derive(Debug, Clone)]
pub struct VoxMatlArray {
    pub matl: [VoxMatl; 256usize],
}
impl Default for VoxMatlArray {
    fn default() -> Self {
        VoxMatlArray {
            matl: [VoxMatl::default(); 256],
        }
    }
}

#[derive(Debug, Clone, Default)]
struct VoxKeyframeModel {
    pub model_index: u32,
    pub _frame_index: u32,
}
#[derive(Debug, Clone, Default)]
struct VoxKeyframeTransform {
    pub _frame_index: u32,
    pub transform: Mat4<f32>,
}

// #[derive(Debug, Clone, Default)]
// pub struct OgtVoxCam {
//     pub camera_id: u32,
//     pub mode: ogt_cam_mode,
//     pub focus: [f32; 3usize],
//     pub angle: [f32; 3usize],
//     pub radius: ::std::os::raw::c_int,
//     pub frustum: f32,
//     pub fov: ::std::os::raw::c_int,
// }

#[derive(Debug, Clone)]
struct VoxSceneNodeTransform {
    pub name: String,
    pub transform: Mat4<f32>,
    pub child_node_id: u32,
    pub layer_id: u32,
    pub hidden: bool,
    pub _keyframes: Vec<VoxKeyframeTransform>,
    pub _loop_animation: bool,
}
#[derive(Debug, Clone)]
struct VoxSceneNodeGroup {
    pub first_child_node_id_index: u32,
    pub num_child_nodes: u32,
}
#[derive(Debug, Clone)]
struct VoxSceneNodeShape {
    pub model_id: u32,
    pub _keyframes: Vec<VoxKeyframeModel>,
    pub _loop_animation: bool,
}

#[derive(Debug, Clone)]
enum VoxSceneNode {
    Invalid,
    Transform(VoxSceneNodeTransform),
    Group(VoxSceneNodeGroup),
    Shape(VoxSceneNodeShape),
}

struct VoxFile {
    buffer: Vec<u8>,
    buffer_size: u32, // size of the data in the buffer
    offset: u32,      // current offset in the buffer data.
}

trait Readable: Copy + Default {
    fn from_le_bytes(bytes: &[u8]) -> Self;
}

impl Readable for u32 {
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("Incorrect byte length"))
    }
}
impl Readable for i32 {
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("Incorrect byte length"))
    }
}
impl Readable for f32 {
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().expect("Incorrect byte length"))
    }
}

impl VoxFile {
    fn bytes_remaining(&self) -> usize {
        if self.offset < self.buffer_size {
            self.buffer_size as usize - self.offset as usize
        } else {
            0
        }
    }
    fn read<T: Copy + Readable>(&mut self) -> T {
        let data_size_to_read = size_of::<T>();
        let start = self.offset as usize;
        let end = start + data_size_to_read;
        let data = &self.buffer[start..end];

        self.offset += data_size_to_read as u32;
        T::from_le_bytes(data)
    }

    fn read_bytes(&mut self, count: usize) -> &[u8] {
        let start = self.offset as usize;
        let end = start + count;
        self.offset += count as u32;
        &self.buffer[start..end]
    }

    /// Reads a dictionary from the .vox file and returns it as a HashMap<String, String>.
    fn read_dict(&mut self) -> HashMap<String, String> {
        let mut dict = HashMap::new();
        let num_pairs_to_read = self.read::<u32>();

        for _ in 0..num_pairs_to_read {
            // Read key size and key string
            let key_size = self.read::<u32>() as usize;
            let key_bytes = self.read_bytes(key_size);
            let key = String::from_utf8_lossy(key_bytes).into_owned();

            // Read value size and value string
            let value_size = self.read::<u32>() as usize;
            let value_bytes = self.read_bytes(value_size);
            let value = String::from_utf8_lossy(value_bytes).into_owned();

            dict.insert(key, value);
        }

        dict
    }

    fn skip_bytes(&mut self, remaining: usize) {
        self.offset += remaining as u32;
    }

    fn current_data_ref(&mut self) -> &[u8] {
        &self.buffer[self.offset as usize..]
    }
}

/// Retrieves a string value from the dictionary, or returns the default.
fn dict_get_str<'a>(dict: &'a HashMap<String, String>, key: &str, default: &'a str) -> &'a str {
    dict.get(key).map(String::as_str).unwrap_or(default)
}

/// Retrieves a boolean value from the dictionary, or returns the default.
fn dict_get_bool(dict: &HashMap<String, String>, key: &str, default: bool) -> bool {
    dict.get(key).map_or(default, |v| v == "1")
}

/// Retrieves a u32 value from the dictionary, or returns the default.
fn dict_get_u32(dict: &HashMap<String, String>, key: &str, default: u32) -> u32 {
    dict.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Converts packed rotation and translation strings into a 4x4 transformation matrix.
fn parse_transform(rotation_string: &str, translation_string: &str) -> Mat4<f32> {
    let mut transform = Mat4::identity();

    let packed_rotation_bits: u32 = rotation_string.parse().unwrap_or(0);
    let row0_vec_index = packed_rotation_bits & 3;
    let row1_vec_index = (packed_rotation_bits >> 2) & 3;
    let row2_vec_index = K_ROW2_INDEX[(1 << row0_vec_index) | (1 << row1_vec_index)];

    assert!(row2_vec_index != usize::MAX, "Invalid packed rotation bits");

    let mut row0 = K_VECTORS[row0_vec_index as usize];
    let mut row1 = K_VECTORS[row1_vec_index as usize];
    let mut row2 = K_VECTORS[row2_vec_index];

    if packed_rotation_bits & (1 << 4) != 0 {
        row0 = -row0;
    }
    if packed_rotation_bits & (1 << 5) != 0 {
        row1 = -row1;
    }
    if packed_rotation_bits & (1 << 6) != 0 {
        row2 = -row2;
    }

    transform.cols[0].x = row0.x;
    transform.cols[0].y = row0.y;
    transform.cols[0].z = row0.z;

    transform.cols[1].x = row1.x;
    transform.cols[1].y = row1.y;
    transform.cols[1].z = row1.z;

    transform.cols[2].x = row2.x;
    transform.cols[2].y = row2.y;
    transform.cols[2].z = row2.z;

    let translation_parts: Vec<i32> =
        translation_string.split_whitespace().filter_map(|s| s.parse().ok()).collect();

    if translation_parts.len() == 3 {
        transform.cols[3].x = translation_parts[0] as f32;
        transform.cols[3].y = translation_parts[1] as f32;
        transform.cols[3].z = translation_parts[2] as f32;
    }

    transform
}

#[derive(Debug, Clone)]
pub struct VoxScene {
    pub models: Vec<VoxModel>,
    pub instances: Vec<VoxInstance>,
    pub layers: Vec<VoxLayer>,
    pub groups: Vec<VoxGroup>,
    pub palette: VoxPalette,
    pub materials: VoxMatlArray,
    // pub num_cameras: u32,
    // pub cameras: *const OgtVoxCam,
}

pub fn read_scene_from_file(path: &str) -> Result<VoxScene, std::io::Error> {
    let buffer = std::fs::read(path)?;
    read_scene_from_memory(&buffer)
}

fn read_scene_from_memory(buffer: &[u8]) -> Result<VoxScene, std::io::Error> {
    read_scene_from_memory_with_flags(buffer, 0)
}

fn read_scene_from_memory_with_flags(
    buffer: &[u8],
    read_flags: u32,
) -> Result<VoxScene, std::io::Error> {
    let mut fp = VoxFile {
        buffer: buffer.to_vec(),
        buffer_size: buffer.len() as u32,
        offset: 0,
    };

    // parsing state/context
    let mut models = vec![];
    let mut nodes = vec![];
    let mut instances = vec![];
    // let mut cameras = vec![];
    // let mut misc_data =
    let mut layers = vec![];
    let mut groups = vec![];
    let mut child_ids = vec![];
    let mut palette = VoxPalette::default();
    let mut materials = VoxMatlArray::default();
    // let mut dict = std::collections::HashMap::new();
    let mut size_x = 0;
    let mut size_y = 0;
    let mut size_z = 0;
    let mut index_map = [0u8; 256];
    let mut found_index_map_chunk = false;

    // push a sentinel character into these datastructures. This allows us to keep indexes
    // rather than pointers into data-structures that grow, and still allow an index of 0
    // to means invalid
    child_ids.push(u32::MAX);

    // copy the default palette into the scene. It may get overwritten by a palette chunk later
    for (i, color) in K_DEFAULT_VOX_PALETTE.chunks(4).enumerate() {
        palette.color[i].x = color[0];
        palette.color[i].y = color[1];
        palette.color[i].z = color[2];
        palette.color[i].w = color[3];
    }

    // zero initialize materials (this sets valid defaults)
    materials.matl.fill(VoxMatl::default());
    let file_header = fp.read::<u32>();
    let file_version = fp.read::<u32>();

    if (file_header != CHUNK_ID_VOX_) || (file_version != 150 && file_version != 200) {
        panic!("Invalid .vox file or i cant parse it yet");
    }

    // Parse chunks until the end of the file/buffer
    while fp.bytes_remaining() >= size_of::<u32>() * 3 {
        // Read the fields common to all chunks
        let chunk_id = fp.read::<u32>();
        let chunk_size = fp.read::<u32>();
        let chunk_child_size = fp.read::<u32>();

        // Process the chunk
        match chunk_id {
            CHUNK_ID_MAIN => {
                // No action needed for MAIN chunk, just skip it
            }
            CHUNK_ID_SIZE => {
                // read 3d size chunk
                assert_eq!(chunk_size, 12, "unexpected chunk size for SIZE chunk");
                assert_eq!(chunk_child_size, 0, "unexpected child size for SIZE chunk");
                size_x = fp.read::<u32>();
                size_y = fp.read::<u32>();
                size_z = fp.read::<u32>();
                assert!(
                    size_x > 0 && size_y > 0 && size_z > 0,
                    "SIZE chunk has zero size"
                );
            }
            CHUNK_ID_XYZI => {
                assert!(
                    size_x > 0 && size_y > 0 && size_z > 0,
                    "Expected SIZE chunk before XYZI chunk"
                );
                let num_voxels_in_chunk = fp.read::<u32>();

                if num_voxels_in_chunk != 0
                    || (read_flags & K_READ_SCENE_FLAGS_KEEP_EMPTY_MODELS_INSTANCES) != 0
                {
                    let voxel_count = (size_x * size_y * size_z) as usize;
                    let mut model = VoxModel {
                        size_x,
                        size_y,
                        size_z,
                        voxel_data: vec![0; voxel_count],
                        voxel_hash: 0,
                    };

                    let voxel_data = &mut model.voxel_data;

                    let k_stride_x = 1;
                    let k_stride_y = size_x;
                    let k_stride_z = size_x * size_y;

                    let voxels_to_read =
                        (fp.bytes_remaining() / 4).min(num_voxels_in_chunk as usize);
                    let packed_voxel_data = fp.current_data_ref();
                    for i in 0..voxels_to_read {
                        let x = packed_voxel_data[i * 4] as usize;
                        let y = packed_voxel_data[i * 4 + 1] as usize;
                        let z = packed_voxel_data[i * 4 + 2] as usize;
                        let color_index = packed_voxel_data[i * 4 + 3];

                        assert!(
                            x < size_x as usize && y < size_y as usize && z < size_z as usize,
                            "Invalid voxel data in XYZI chunk"
                        );

                        let index = (x * k_stride_x)
                            + (y * k_stride_y as usize)
                            + (z * k_stride_z as usize);
                        voxel_data[index] = color_index;
                    }

                    fp.skip_bytes(num_voxels_in_chunk as usize * 4);
                    // TODO: hash here
                    models.push(Some(model));
                } else {
                    models.push(None);
                }
            }
            CHUNK_ID_RGBA => {
                assert_eq!(chunk_size, 1024, "unexpected RGBA chunk size");
                let palette_bytes = fp.read_bytes(1024);

                for i in 0..256 {
                    palette.color[i] = Vec4::new(
                        palette_bytes[i * 4],
                        palette_bytes[i * 4 + 1],
                        palette_bytes[i * 4 + 2],
                        palette_bytes[i * 4 + 3],
                    );
                }
            }
            CHUNK_ID_N_TRN => {
                let node_id = fp.read::<u32>();

                // Read dictionary from file
                let dict = fp.read_dict();

                let node_name = dict_get_str(&dict, "_name", "").to_string();
                let hidden = dict_get_bool(&dict, "_hidden", false);
                let loop_animation = dict_get_bool(&dict, "_loop", false);

                let child_node_id = fp.read::<u32>();
                let _reserved_id = fp.read::<u32>();
                let layer_id = fp.read::<u32>();
                let num_frames = fp.read::<u32>();

                assert!(
                    _reserved_id == u32::MAX,
                    "unexpected values for reserved_id in nTRN chunk"
                );
                assert!(num_frames > 0, "nTRN chunk must have at least one frame");

                let mut keyframes = vec![VoxKeyframeTransform::default(); num_frames as usize];
                for i in 0..num_frames {
                    let frame_dict = fp.read_dict();
                    let rotation_value = dict_get_str(&frame_dict, "_r", "");
                    let translation_value = dict_get_str(&frame_dict, "_t", "");
                    let frame_index = dict_get_u32(&frame_dict, "_f", 0);

                    let transform = parse_transform(rotation_value, translation_value);
                    keyframes[i as usize] = VoxKeyframeTransform {
                        transform,
                        _frame_index: frame_index,
                    };
                }
                // setup the transform node.
                {
                    // grow to fit node_id
                    if node_id >= nodes.len().try_into().unwrap() {
                        nodes.resize((node_id + 1).try_into().unwrap(), VoxSceneNode::Invalid);
                    }
                    nodes[node_id as usize] = VoxSceneNode::Transform(VoxSceneNodeTransform {
                        child_node_id,
                        layer_id,
                        transform: keyframes[0].transform,
                        hidden,
                        _keyframes: keyframes,
                        _loop_animation: loop_animation,
                        name: node_name,
                    });
                }
            }
            CHUNK_ID_N_GRP => {
                let node_id = fp.read::<u32>();

                // Read dictionary (unused)
                let _dict = fp.read_dict();

                // Ensure `nodes` is large enough to fit `node_id`
                if nodes.len() <= node_id as usize {
                    nodes.resize(node_id as usize + 1, VoxSceneNode::Invalid);
                }

                nodes[node_id as usize] = VoxSceneNode::Group(VoxSceneNodeGroup {
                    first_child_node_id_index: 0,
                    num_child_nodes: 0,
                });

                // Read number of child nodes
                let num_child_nodes = fp.read::<u32>();

                if num_child_nodes > 0 {
                    let prior_size = child_ids.len();
                    assert!(prior_size > 0, "prior_size sanity test failed"); // Ensured by sentinel

                    child_ids.resize(prior_size + num_child_nodes as usize, 0);
                    for i in 0..num_child_nodes as usize {
                        child_ids[prior_size + i] = fp.read::<u32>();
                    }

                    match &mut nodes[node_id as usize] {
                        VoxSceneNode::Group(group) => {
                            group.first_child_node_id_index = prior_size as u32;
                            group.num_child_nodes = num_child_nodes;
                        }
                        _ => unreachable!(),
                    }
                }
            }
            CHUNK_ID_N_SHP => {
                let node_id = fp.read::<u32>();

                // Read dictionary from file
                let dict = fp.read_dict();
                let loop_animation = dict_get_bool(&dict, "_loop", false);

                let num_models = fp.read::<u32>();
                assert!(num_models > 0, "must have at least 1 frame in nSHP chunk");

                let mut keyframes = vec![VoxKeyframeModel::default(); num_models as usize];

                for i in 0..num_models {
                    let model_index = fp.read::<u32>();
                    assert!(
                        model_index < models.len() as u32,
                        "nSHP chunk references invalid model_id"
                    );

                    let frame_dict = fp.read_dict();
                    let frame_index = dict_get_u32(&frame_dict, "_f", 0);

                    keyframes[i as usize] = VoxKeyframeModel {
                        model_index,
                        _frame_index: frame_index,
                    };
                }

                if node_id >= nodes.len().try_into().unwrap() {
                    nodes.resize((node_id + 1).try_into().unwrap(), VoxSceneNode::Invalid);
                }
                nodes[node_id as usize] = VoxSceneNode::Shape(VoxSceneNodeShape {
                    model_id: keyframes[0].model_index,
                    _keyframes: keyframes,
                    _loop_animation: loop_animation,
                });
            }
            CHUNK_ID_IMAP => {
                assert_eq!(chunk_size, 256, "unexpected chunk size for IMAP chunk");
                index_map.copy_from_slice(fp.read_bytes(256));
                found_index_map_chunk = true;
            }
            CHUNK_ID_LAYR => {
                let layer_id = fp.read::<i32>();
                let dict = fp.read_dict();
                let _reserved_id = fp.read::<i32>();

                assert!(
                    _reserved_id == -1,
                    "unexpected value for reserved_id in LAYR chunk"
                );

                if layer_id >= layers.len().try_into().unwrap() {
                    layers.resize((layer_id + 1).try_into().unwrap(), VoxLayer::default());
                }
                let mut layer = VoxLayer {
                    name: dict.get("_name").cloned().unwrap_or_default(),
                    color: Vec4::new(255, 255, 255, 255),
                    hidden: dict_get_bool(&dict, "_hidden", false),
                };

                if let Some(color_string) = dict.get("_color") {
                    let parts: Vec<&str> = color_string.split_whitespace().collect();
                    if parts.len() == 3 {
                        layer.color[0] = parts[0].parse().unwrap_or(255);
                        layer.color[1] = parts[1].parse().unwrap_or(255);
                        layer.color[2] = parts[2].parse().unwrap_or(255);
                    }
                }

                layers[layer_id as usize] = layer;
            }
            CHUNK_ID_MATL => {
                let mut material_id = fp.read::<i32>() as usize;
                material_id &= 0xFF; // incoming material 256 is material 0

                let dict = fp.read_dict();
                let material = &mut materials.matl[material_id];

                if let Some(type_string) = dict.get("_type") {
                    material.type_ = match type_string.as_str() {
                        "_diffuse" => MatlType::Diffuse,
                        "_metal" => MatlType::Metal,
                        "_glass" => MatlType::Glass,
                        "_emit" => MatlType::Emit,
                        "_blend" => MatlType::Blend,
                        "_media" => MatlType::Media,
                        _ => MatlType::Diffuse, // TODO: htf does work in ogt?
                    };
                }

                if let Some(metal_string) = dict.get("_metal") {
                    material.content_flags |= K_VOX_MATL_HAVE_METAL;
                    material.metal = metal_string.parse::<f32>().unwrap();
                }
                if let Some(rough_string) = dict.get("_rough") {
                    material.content_flags |= K_VOX_MATL_HAVE_ROUGH;
                    material.rough = rough_string.parse::<f32>().unwrap();
                }
                if let Some(spec_string) = dict.get("_spec") {
                    material.content_flags |= K_VOX_MATL_HAVE_SPEC;
                    material.spec = spec_string.parse::<f32>().unwrap();
                }
                if let Some(ior_string) = dict.get("_ior") {
                    material.content_flags |= K_VOX_MATL_HAVE_IOR;
                    material.ior = ior_string.parse::<f32>().unwrap();
                }
                if let Some(att_string) = dict.get("_att") {
                    material.content_flags |= K_VOX_MATL_HAVE_ATT;
                    material.att = att_string.parse::<f32>().unwrap();
                }
                if let Some(flux_string) = dict.get("_flux") {
                    material.content_flags |= K_VOX_MATL_HAVE_FLUX;
                    material.flux = flux_string.parse::<f32>().unwrap();
                }
                if let Some(emit_string) = dict.get("_emit") {
                    material.content_flags |= K_VOX_MATL_HAVE_EMIT;
                    material.emit = emit_string.parse::<f32>().unwrap();
                }
                if let Some(ldr_string) = dict.get("_ldr") {
                    material.content_flags |= K_VOX_MATL_HAVE_LDR;
                    material.ldr = ldr_string.parse::<f32>().unwrap();
                }
                if let Some(trans_string) = dict.get("_trans") {
                    material.content_flags |= K_VOX_MATL_HAVE_TRANS;
                    material.trans = trans_string.parse::<f32>().unwrap();
                }
                if let Some(alpha_string) = dict.get("_alpha") {
                    material.content_flags |= K_VOX_MATL_HAVE_ALPHA;
                    material.alpha = alpha_string.parse::<f32>().unwrap();
                }
                if let Some(d_string) = dict.get("_d") {
                    material.content_flags |= K_VOX_MATL_HAVE_D;
                    material.d = d_string.parse::<f32>().unwrap();
                }
                if let Some(sp_string) = dict.get("_sp") {
                    material.content_flags |= K_VOX_MATL_HAVE_SP;
                    material.sp = sp_string.parse::<f32>().unwrap();
                }
                if let Some(g_string) = dict.get("_g") {
                    material.content_flags |= K_VOX_MATL_HAVE_G;
                    material.g = g_string.parse::<f32>().unwrap();
                }
                if let Some(media_string) = dict.get("_media") {
                    material.content_flags |= K_VOX_MATL_HAVE_MEDIA;
                    material.media = media_string.parse::<f32>().unwrap();
                }
            }
            CHUNK_ID_MATT => {
                let mut material_id = fp.read::<i32>() as usize;
                material_id &= 0xFF; // incoming material 256 is material 0

                // 0 : diffuse
                // 1 : metal
                // 2 : glass
                // 3 : emissive
                let material_type = fp.read::<i32>();

                // diffuse  : 1.0
                // metal    : (0.0 - 1.0] : blend between metal and diffuse material
                // glass    : (0.0 - 1.0] : blend between glass and diffuse material
                // emissive : (0.0 - 1.0] : self-illuminated material
                let material_weight = fp.read::<f32>();

                // bit(0) : Plastic
                // bit(1) : Roughness
                // bit(2) : Specular
                // bit(3) : IOR
                // bit(4) : Attenuation
                // bit(5) : Power
                // bit(6) : Glow
                // bit(7) : isTotalPower (*no value)
                let _property_bits = fp.read::<u32>();

                let material = &mut materials.matl[material_id];
                let material_type = match material_type {
                    0 => MatlType::Diffuse,
                    1 => MatlType::Metal,
                    2 => MatlType::Glass,
                    3 => MatlType::Emit,
                    _ => MatlType::Diffuse, // TODO: htf does work in ogt?
                };
                material.type_ = material_type;

                match material_type {
                    MatlType::Diffuse => {}
                    MatlType::Metal => {
                        material.content_flags |= K_VOX_MATL_HAVE_METAL;
                        material.metal = material_weight;
                    }
                    MatlType::Glass => {
                        material.content_flags |= K_VOX_MATL_HAVE_TRANS;
                        material.trans = material_weight;
                    }
                    MatlType::Emit => {
                        material.content_flags |= K_VOX_MATL_HAVE_EMIT;
                        material.emit = material_weight;
                    }
                    _ => {}
                }

                assert!(chunk_size >= 16, "unexpected chunk size for MATT chunk");
                let remaining = chunk_size - 16;
                fp.skip_bytes(remaining as usize);
            }
            _ => {
                // panic!("Unhandled chunk type");
                fp.skip_bytes(chunk_size as usize);
            }
        }
    }
    // **Post-processing**
    if !nodes.is_empty() {
        let generate_groups = (read_flags & K_READ_SCENE_FLAGS_GROUPS) != 0;
        let generate_keyframes = (read_flags & K_READ_SCENE_FLAGS_KEYFRAMES) != 0;

        let mut stack: Vec<u32> = Vec::with_capacity(64);
        generate_instances_for_node(
            &mut stack,
            &nodes,
            0,
            &child_ids,
            &models,
            &mut instances,
            &mut groups,
            K_INVALID_GROUP_INDEX,
            generate_keyframes,
        );

        if !generate_groups {
            // Flatten keyframes on instances
            if generate_keyframes {
                todo!();
                // for instance in &mut instances {
                //     let mut frame_indices: Vec<u32> = Vec::with_capacity(256);

                //     let mut group_index = instance.group_index;
                //     while group_index != k_invalid_group_index {
                //         let group = &groups[group_index as usize];
                //         for keyframe in &group.transform_anim.keyframes {
                //             if !frame_indices.contains(&keyframe.frame_index) {
                //                 frame_indices.push(keyframe.frame_index);
                //             }
                //         }
                //         group_index = group.parent_group_index;
                //     }

                //     frame_indices.sort_unstable();
                //     let mut new_keyframes = Vec::with_capacity(frame_indices.len());

                //     for &frame_index in &frame_indices {
                //         let mut flattened_transform = sample_keyframe_transform(
                //             &instance.transform_anim.keyframes,
                //             instance.transform_anim.loop_animation,
                //             frame_index,
                //         );

                //         let mut group_index = instance.group_index;
                //         while group_index != k_invalid_group_index {
                //             let group = &groups[group_index as usize];
                //             let group_transform = sample_keyframe_transform(
                //                 &group.transform_anim.keyframes,
                //                 group.transform_anim.loop_animation,
                //                 frame_index,
                //             );
                //             flattened_transform = flattened_transform * group_transform;
                //             group_index = group.parent_group_index;
                //         }

                //         new_keyframes.push(VoxKeyframeTransform {
                //             frame_index,
                //             transform: flattened_transform,
                //         });
                //     }

                //     instance.transform_anim.keyframes = new_keyframes;
                // }
            }

            // Flatten instance transforms if groups are disabled
            for instance in &mut instances {
                let mut flattened_transform = instance.transform;
                let mut group_index = instance.group_index;
                while group_index != K_INVALID_GROUP_INDEX {
                    flattened_transform *= groups[group_index as usize].transform;
                    group_index = groups[group_index as usize].parent_group_index;
                }
                instance.transform = flattened_transform;
                instance.group_index = 0;
            }

            // Create a root group
            groups.clear();
            groups.push(VoxGroup {
                name: String::new(),
                transform: Mat4::identity(),
                parent_group_index: K_INVALID_GROUP_INDEX,
                layer_index: 0,
                hidden: false,
                // transform_anim: Default::default(),
            });
        }
    } else if models.len() == 1 {
        // Add a single instance if only one model exists
        instances.push(VoxInstance {
            name: String::new(),
            transform: Mat4::identity(),
            model_index: 0,
            layer_index: 0,
            group_index: 0,
            hidden: false,
            // transform_anim: Default::default(),
        });

        groups.push(VoxGroup {
            name: String::new(),
            transform: Mat4::identity(),
            parent_group_index: K_INVALID_GROUP_INDEX,
            layer_index: 0,
            hidden: false,
            // transform_anim: Default::default(),
        });
    }

    // Ensure at least one layer exists
    if layers.is_empty() {
        for instance in &mut instances {
            instance.layer_index = 0;
        }
        layers.push(VoxLayer {
            name: String::new(),
            color: Vec4::new(255, 255, 255, 255),
            hidden: false,
        });
    }

    // **Apply IMAP chunk remapping if found**
    if found_index_map_chunk {
        let mut index_map_inverse = [0u8; 256];
        for i in 0..256 {
            index_map_inverse[index_map[i] as usize] = i as u8;
        }

        let old_palette = palette.clone();
        for i in 0..256 {
            palette.color[i] = old_palette.color[(index_map[i] as usize + 255) & 0xFF];
        }

        let old_materials = materials.clone();
        for i in 0..256 {
            let remapped_i = (i + 255) & 0xFF;
            let remapped_index = index_map[remapped_i] as usize;
            materials.matl[i] = old_materials.matl[remapped_index];
        }

        for model in &mut models {
            let model = model.as_mut().unwrap();
            let _num_voxels = (model.size_x * model.size_y * model.size_z) as usize;
            for voxel in &mut model.voxel_data {
                *voxel = index_map_inverse[*voxel as usize].wrapping_add(1);
            }
        }
    }

    // **Rotate the palette to align voxel indices with the palette**
    {
        let last_color = palette.color[255];
        for i in (1..=255).rev() {
            palette.color[i] = palette.color[i - 1];
        }
        palette.color[0] = last_color;
        palette.color[0][3] = 0; // Alpha is zero for transparency
    }

    // **Remove duplicate models**
    if (read_flags & K_READ_SCENE_FLAGS_KEEP_DUPLICATE_MODELS) == 0 {
        for i in 0..models.len() {
            for j in (i + 1)..models.len() {
                if models[j].clone().unwrap() == models[i].clone().unwrap() {
                    models.remove(j);
                    for instance in &mut instances {
                        if instance.model_index == j as u32 {
                            instance.model_index = i as u32;
                        }
                    }
                }
            }
        }
    }

    // **Remove empty models**
    if (read_flags & K_READ_SCENE_FLAGS_KEEP_EMPTY_MODELS_INSTANCES) == 0 {
        models.retain(|model| model.clone().unwrap().voxel_data.iter().any(|&v| v != 0));
    }

    // **Construct the final scene**
    // VoxScene {
    //     models,
    //     instances,
    //     layers,
    //     groups,
    //     palette,
    //     materials,
    //     num_models: todo!(),
    //     num_instances: todo!(),
    //     num_layers: todo!(),
    //     num_groups: todo!(),
    // }

    Ok(VoxScene {
        models: models.iter().map(|m| m.clone().unwrap()).collect(),
        instances,
        layers,
        groups,
        palette,
        materials,
    })
}

fn generate_instances_for_node(
    stack: &mut Vec<u32>,
    nodes: &[VoxSceneNode],
    node_index: u32,
    child_id_array: &[u32],
    models: &[Option<VoxModel>],
    instances: &mut Vec<VoxInstance>,
    groups: &mut Vec<VoxGroup>,
    group_index: u32,
    generate_keyframes: bool,
) {
    let node = &nodes[node_index as usize];

    match node {
        VoxSceneNode::Transform(node) => {
            stack.push(node_index);
            generate_instances_for_node(
                stack,
                nodes,
                node.child_node_id,
                child_id_array,
                models,
                instances,
                groups,
                group_index,
                generate_keyframes,
            );
            stack.pop();
        }
        VoxSceneNode::Group(_node) => {
            let next_group_index = 0_u32;
            {
                let last_transform_idx = *stack.last().unwrap();
                let last_transform = &nodes[last_transform_idx as usize];

                let last_transform = if let VoxSceneNode::Transform(trans) = last_transform {
                    trans
                } else {
                    panic!("Expected a transform node before a group node")
                };

                let _next_group_index = groups.len() as u32;
                let group = VoxGroup {
                    parent_group_index: group_index,
                    transform: last_transform.transform,
                    hidden: last_transform.hidden,
                    layer_index: last_transform.layer_id,
                    name: last_transform.name.clone(),
                    // transform_anim: Default::default(),
                };

                if generate_keyframes {
                    todo!()
                    /*                        group.transform_anim.num_keyframes = last_transform->u.transform.num_keyframes;
                    group.transform_anim.keyframes     = (const vox_keyframe_transform*)(last_transform->u.transform.keyframe_offset);
                    group.transform_anim.loop          = last_transform->u.transform.loop;
                     */
                    // group.transform_anim = last_transform.get_transform_anim();
                }

                groups.push(group);
            }

            stack.push(node_index);

            let group_node = if let VoxSceneNode::Group(group) = &nodes[node_index as usize] {
                group
            } else {
                panic!("Expected a group node")
            };
            let child_node_ids = &child_id_array[group_node.first_child_node_id_index as usize..];
            for &child_id in child_node_ids.iter() {
                generate_instances_for_node(
                    stack,
                    nodes,
                    child_id,
                    child_id_array,
                    models,
                    instances,
                    groups,
                    next_group_index,
                    generate_keyframes,
                );
            }
            stack.pop();
        }
        VoxSceneNode::Shape(node) => {
            assert!(
                (node.model_id as usize) < models.len(),
                "Unexpected model id for shape node"
            );

            if (node.model_id as usize) < models.len() && models[node.model_id as usize].is_some() {
                let last_transform =
                    *stack.last().expect("Expected transform node before shape node");
                let last_transform =
                    if let VoxSceneNode::Transform(trans) = &nodes[last_transform as usize] {
                        trans
                    } else {
                        panic!("Expected a transform node before shape node")
                    };
                let last_group = *stack
                    .get(stack.len().saturating_sub(2))
                    .expect("Expected group node before shape node");
                let _last_group = if let VoxSceneNode::Group(group) = &nodes[last_group as usize] {
                    group
                } else {
                    panic!("Expected a group node before shape node")
                };

                let new_instance = VoxInstance {
                    model_index: node.model_id,
                    transform: last_transform.transform,
                    layer_index: last_transform.layer_id,
                    group_index,
                    hidden: last_transform.hidden,
                    name: last_transform.name.clone(),
                    // model_anim: Default::default(),
                    // transform_anim: Default::default(),
                };

                if generate_keyframes {
                    todo!()
                    // new_instance.model_anim = VoxModelAnim {
                    //     num_keyframes: *num_keyframes,
                    //     keyframes: keyframe_offset.clone(),
                    //     loop_animation: *loop_animation,
                    // };
                    // new_instance.transform_anim = last_transform.get_transform_anim();
                }

                instances.push(new_instance);
            }
        }
        _ => panic!("Unhandled node type"),
    }
}

fn _sample_keyframe_transform(
    keyframes: &[VoxKeyframeTransform],
    loop_animation: bool,
    frame_index: u32,
) -> Mat4<f32> {
    assert!(!keyframes.is_empty(), "At least one keyframe is required");

    let frame_index = if loop_animation {
        _compute_looped_frame_index(
            keyframes.first().unwrap()._frame_index,
            keyframes.last().unwrap()._frame_index,
            frame_index,
        )
    } else {
        frame_index
    };

    if frame_index <= keyframes.first().unwrap()._frame_index {
        return keyframes.first().unwrap().transform;
    }
    if frame_index >= keyframes.last().unwrap()._frame_index {
        return keyframes.last().unwrap().transform;
    }

    for i in (0..keyframes.len() - 1).rev() {
        if frame_index >= keyframes[i]._frame_index {
            let next_frame = keyframes[i + 1]._frame_index;
            let curr_frame = keyframes[i]._frame_index;
            let t = (frame_index - curr_frame) as f32 / (next_frame - curr_frame) as f32;
            let t_inv = 1.0 - t;

            let mut curr_transform = keyframes[i].transform;
            let next_transform = keyframes[i + 1].transform;

            // Interpolate position while snapping rotation
            curr_transform.cols[3].x =
                ((next_transform.cols[3].x * t) + (curr_transform.cols[3].x * t_inv)).round();
            curr_transform.cols[3].y =
                ((next_transform.cols[3].y * t) + (curr_transform.cols[3].y * t_inv)).round();
            curr_transform.cols[3].z =
                ((next_transform.cols[3].z * t) + (curr_transform.cols[3].z * t_inv)).round();

            return curr_transform;
        }
    }

    panic!("This code path should not be reached");
}

fn _compute_looped_frame_index(
    first_loop_frame: u32,
    last_loop_frame: u32,
    frame_index: u32,
) -> u32 {
    let loop_len = 1 + last_loop_frame - first_loop_frame;
    let looped_frame_index = if frame_index >= first_loop_frame {
        let frames_since_first_frame = frame_index - first_loop_frame;
        first_loop_frame + (frames_since_first_frame % loop_len)
    } else {
        let frames_since_first_frame = first_loop_frame - frame_index - 1;
        last_loop_frame - (frames_since_first_frame % loop_len)
    };

    assert!(
        looped_frame_index >= first_loop_frame && looped_frame_index <= last_loop_frame,
        "Bug in looping logic!"
    );

    looped_frame_index
}
