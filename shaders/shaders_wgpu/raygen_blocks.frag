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

@group(0) @binding(0) var<uniform> uniforms: UboData;
// @group(0) @binding(1) var<uniform> pco: Constants;
@group(0) @binding(2) var blockPalette: texture_3d<i32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) sample_point: vec3<f32>,
    @location(1) bunorm: u32,
};

struct FragmentOutput {
    @location(0) outMatNorm: vec4<u32>,
};

const BLOCK_PALETTE_SIZE_X: i32 = 64;

fn voxel_in_palette(relative_voxel_pos: vec3<i32>, block_id: i32) -> vec3<i32> {
    let block_x = block_id % BLOCK_PALETTE_SIZE_X;
    let block_y = block_id / BLOCK_PALETTE_SIZE_X;
    return relative_voxel_pos + vec3<i32>(16 * block_x, 16 * block_y, 0);
}

fn voxel_in_bit_palette(relative_voxel_pos: vec3<i32>, block_id: i32) -> vec3<i32> {
    let block_x = block_id % BLOCK_PALETTE_SIZE_X;
    let block_y = block_id / BLOCK_PALETTE_SIZE_X;
    return relative_voxel_pos + vec3<i32>(0 + 2 * block_x, 0 + 16 * block_y, 0);
}

fn GetVoxel(block_id: i32, relative_voxel_pos: vec3<i32>) -> u32 {
    let voxel_pos = voxel_in_palette(relative_voxel_pos, block_id);
    return u32(textureLoad(blockPalette, voxel_pos, 0).r);
}

@fragment
fn main(in: VertexOutput) -> FragmentOutput {
    let packed_bunorm = in.bunorm;
    let sample_block = (packed_bunorm & 0xFFFFu);
    let normal_encoded = (packed_bunorm >> 16u);

    let axis = vec3<i32>(
        i32(normal_encoded & 0x1u),
        i32((normal_encoded >> 1u) & 0x1u),
        i32((normal_encoded >> 2u) & 0x1u),
    );
    let _sign = 1 - 2 * i32((normal_encoded >> 7u) & 0x1u);
    let inorm = axis * _sign;
    let normal_encoded_out = vec3<u32>((inorm + 1) * 255 / 2);

    let ipos = vec3<i32>(floor(in.sample_point));

    var out: FragmentOutput;
    let voxel = GetVoxel(i32(sample_block), ipos);

    // all values are in range of u8, custom encoded
    out.outMatNorm = vec4<u32>(voxel, normal_encoded_out);

    return out;
}