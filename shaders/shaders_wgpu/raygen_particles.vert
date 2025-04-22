struct UboData {
    trans_w2s: mat4x4<f32>,
    campos: vec4<f32>,
    camdir: vec4<f32>,
    horizline_scaled: vec4<f32>,
    vertiline_scaled: vec4<f32>,
    global_light_dir: vec4<f32>,
    lightmap_proj: mat4x4<f32>,
    frame_size: vec2<f32>,
    wind_direction: vec2<f32>,
    timeseed: i32,
    delta_time: f32,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
// @group(0) @binding(1) var blocks: texture_3d<i32>;
// @group(0) @binding(2) var blockPalette: texture_storage_3d<r32sint, write>; // Assuming write access

struct VertexInput {
    @location(0) posIn: vec3<f32>,
    @location(1) velIn: vec3<f32>,
    @location(2) lifeTimeIn: f32,
    @location(3) matIDIn: u32,
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) mat_norm: vec4<u32>,
};

struct FragmentOutput {
    @location(0) outMatNorm: vec4<u32>,
};

const BLOCK_PALETTE_SIZE_X: i32 = 64;
const STATIC_BLOCK_COUNT: i32 = 15;
const DELTA_TIME: f32 = 1.0 / 75.0;

const CUBE_STRIP_POS: array<vec3<f32>, 14> = array(
    vec3<f32>(-1.0, 1.0, 1.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(-1.0, -1.0, 1.0),
    vec3<f32>(1.0, -1.0, 1.0),
    vec3<f32>(1.0, -1.0, -1.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(1.0, 1.0, -1.0),
    vec3<f32>(-1.0, 1.0, 1.0),
    vec3<f32>(-1.0, 1.0, -1.0),
    vec3<f32>(-1.0, -1.0, 1.0),
    vec3<f32>(-1.0, -1.0, -1.0),
    vec3<f32>(1.0, -1.0, -1.0),
    vec3<f32>(-1.0, 1.0, -1.0),
    vec3<f32>(1.0, 1.0, -1.0),
);

const CUBE_STRIP_NORM: array<vec3<f32>, 14> = array(
    vec3<f32>(0.0, 0.0, 1.0),
    vec3<f32>(0.0, 0.0, 1.0),
    vec3<f32>(0.0, -1.0, 0.0),
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(-1.0, 0.0, 0.0),
    vec3<f32>(-1.0, 0.0, 0.0),
    vec3<f32>(0.0, -1.0, 0.0),
    vec3<f32>(0.0, 0.0, -1.0),
    vec3<f32>(0.0, 0.0, -1.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(1.0, 1.0, 1.0),
);

fn voxel_in_palette(relative_voxel_pos: vec3<i32>, block_id: i32) -> vec3<i32> {
    let block_x = block_id % BLOCK_PALETTE_SIZE_X;
    let block_y = block_id / BLOCK_PALETTE_SIZE_X;
    return relative_voxel_pos + vec3<i32>(16 * block_x, 16 * block_y, 0);
}

@vertex
fn main(in: VertexInput) -> VertexOutput {
    let world_pos = vec4<f32>(in.posIn, 1.0);
    let clip_coords = (ubo.trans_w2s * world_pos).xyz;

    var out: VertexOutput;
    out.position = vec4<f32>(clip_coords, 1.0);

    let size = in.lifeTimeIn / 14.0;
    let mat = in.matIDIn;
    let geom_index = in.vertex_index % 14u;
    let normal = CUBE_STRIP_NORM[geom_index];

    // pack mat and normal to u32 (4xu8)
    out.mat_norm = vec4<u32>(mat, vec3<u32>(normal));
    
    return out;
}