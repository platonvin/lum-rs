struct UboData {
    trans_w2s: mat4x4<f32>,
    campos: vec4<f32>,
    camdir: vec4<f32>,
    horizline_scaled: vec4<f32>,
    vertiline_scaled: vec4<f32>,
    global_light_dir: vec4<f32>,
    lightmap_proj: mat4x4<f32>,
    frame_size: vec2<f32>,
    timeseed: i32,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(0) @binding(1) var blocks: texture_3d<i32>;
@group(0) @binding(2) var blockPalette: texture_storage_3d<r8uint, write>; // Assuming write access

struct VertexInput {
    @location(0) posIn: vec3<f32>,
    @location(1) velIn: vec3<f32>,
    @location(2) lifeTimeIn: f32,
    @location(3) matIDIn: u32,
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
};

struct VS_OUT {
    @location(0) size: f32,
    @location(1) mat: u32,
    @location(2) normal: vec3<f32>,
};

struct FragmentOutput {
    @location(0) outMatNorm: u32,
};

const BLOCK_PALETTE_SIZE_X: i32 = 64;
const STATIC_BLOCK_COUNT: i32 = 15;
const DELTA_TIME: f32 = 1.0 / 75.0;

const CUBE_STRIP_POS: array<vec3<f32>, 14> = array(
    vec3(-1.0, 1.0, 1.0),
    vec3(1.0, 1.0, 1.0),
    vec3(-1.0, -1.0, 1.0),
    vec3(1.0, -1.0, 1.0),
    vec3(1.0, -1.0, -1.0),
    vec3(1.0, 1.0, 1.0),
    vec3(1.0, 1.0, -1.0),
    vec3(-1.0, 1.0, 1.0),
    vec3(-1.0, 1.0, -1.0),
    vec3(-1.0, -1.0, 1.0),
    vec3(-1.0, -1.0, -1.0),
    vec3(1.0, -1.0, -1.0),
    vec3(-1.0, 1.0, -1.0),
    vec3(1.0, 1.0, -1.0),
);

const CUBE_STRIP_NORM: array<vec3<f32>, 14> = array(
    vec3(0.0, 0.0, 1.0),
    vec3(0.0, 0.0, 1.0),
    vec3(0.0, -1.0, 0.0),
    vec3(1.0, 0.0, 0.0),
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(-1.0, 0.0, 0.0),
    vec3(-1.0, 0.0, 0.0),
    vec3(0.0, -1.0, 0.0),
    vec3(0.0, 0.0, -1.0),
    vec3(0.0, 0.0, -1.0),
    vec3(1.0, 1.0, 1.0), // Unused in original geometry shader?
    vec3(1.0, 1.0, 1.0), // Unused in original geometry shader?
);

fn voxel_in_palette(relative_voxel_pos: vec3<i32>, block_id: i32) -> vec3<i32> {
    let block_x = block_id % BLOCK_PALETTE_SIZE_X;
    let block_y = block_id / BLOCK_PALETTE_SIZE_X;
    return relative_voxel_pos + vec3<i32>(16 * block_x, 16 * block_y, 0);
}

@vertex
fn vert_main(in: VertexInput) -> VS_OUT {
    let world_pos = vec4<f32>(in.posIn, 1.0);
    let clip_coords = (ubo.trans_w2s * world_pos).xyz;

    var vs_out: VS_OUT;
    vs_out.size = in.lifeTimeIn / 14.0;
    vs_out.mat = in.matIDIn;

    let geom_index = in.vertex_index % 14u;
    vs_out.normal = CUBE_STRIP_NORM[geom_index];

    return vs_out;
}

@fragment
fn frag_main(in: VS_OUT) -> FragmentOutput {
    let fmat = (f32(in.mat) - 127.0) / 127.0;
    let fmat_norm = vec4<f32>(fmat, in.normal);
    let mat_norm_encoded = u32(((fmat_norm + 1.0) / 2.0) * 255.0 + 0.5); // Adding 0.5 for proper rounding

    var out: FragmentOutput;
    out.outMatNorm = mat_norm_encoded;
    return out;
}

@compute @workgroup_size(1, 1, 1) // Example workgroup size
fn compute_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let instance_id = global_id.x; // Assuming each compute invocation processes one particle

    // Fetch particle data (you'll need a buffer for this)
    // Example:
    // struct ParticleBuffer { particles: array<ParticleData>; };
    // @group(0) @binding(3) var<storage, read> particleBuffer: ParticleBuffer;
    // let particle = particleBuffer.particles[instance_id];
    // let posIn = particle.pos;
    // let matIDIn = particle.matID;
    // let lifeTimeIn = particle.lifeTime;
    // let velIn = particle.vel;

    // For demonstration, let's use a fixed particle for mapping
    let posIn = vec3<f32>(global_id.x * 0.5, global_id.y * 0.5, global_id.z * 0.5);
    let matIDIn = global_id.x % 256;
    let lifeTimeIn = f32(global_id.x) / 10.0;

    if (lifeTimeIn > 0.15) {
        let target_voxel_in_world = vec3<i32>(floor(posIn));
        let target_block_in_world = target_voxel_in_world / 16;
        let target_block_id = textureLoad(blocks, target_block_in_world, 0).x;
        let target_voxel_in_palette = voxel_in_palette(target_voxel_in_world % 16, target_block_id);
        if (target_block_id >= STATIC_BLOCK_COUNT) {
            imageStore(blockPalette, target_voxel_in_palette, uvec4(matIDIn, 0, 0, 0));
        }
    }
}