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

struct InstanceInput {
    @location(0) posIn: vec3<f32>,
    @location(1) velIn: vec3<f32>,
    @location(2) lifeTimeIn: f32,
    @location(3) @interpolate(flat) matIDIn: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) mat_norm: vec4<u32>,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
// @group(0) @binding(1) var blocks: texture_3d<i32>;
// @group(0) @binding(2) var blockPalette: texture_storage_3d<r32sint, write>;

const CUBE_VERTICES: array<vec3<f32>, 36> = array<vec3<f32>, 36>(
    // +X face
    vec3<f32>(1, -1,  1), vec3<f32>(1, -1, -1), vec3<f32>(1,  1, -1),
    vec3<f32>(1, -1,  1), vec3<f32>(1,  1, -1), vec3<f32>(1,  1,  1),
    // -X face
    vec3<f32>(-1, -1, -1), vec3<f32>(-1, -1,  1), vec3<f32>(-1,  1,  1),
    vec3<f32>(-1, -1, -1), vec3<f32>(-1,  1,  1), vec3<f32>(-1,  1, -1),
    // +Y face
    vec3<f32>(-1, 1,  1), vec3<f32>(1, 1,  1), vec3<f32>(1, 1, -1),
    vec3<f32>(-1, 1,  1), vec3<f32>(1, 1, -1), vec3<f32>(-1, 1, -1),
    // -Y face
    vec3<f32>(-1, -1, -1), vec3<f32>(1, -1, -1), vec3<f32>(1, -1,  1),
    vec3<f32>(-1, -1, -1), vec3<f32>(1, -1,  1), vec3<f32>(-1, -1,  1),
    // +Z face
    vec3<f32>(-1, -1, 1), vec3<f32>(-1,  1, 1), vec3<f32>(1,  1, 1),
    vec3<f32>(-1, -1, 1), vec3<f32>(1,  1, 1), vec3<f32>(1, -1, 1),
    // -Z face
    vec3<f32>(1, -1, -1), vec3<f32>(1,  1, -1), vec3<f32>(-1,  1, -1),
    vec3<f32>(1, -1, -1), vec3<f32>(-1,  1, -1), vec3<f32>(-1, -1, -1)
);

const CUBE_NORMALS: array<vec3<f32>, 36> = array<vec3<f32>, 36>(
    // +X normals
    vec3<f32>(1, 0, 0), vec3<f32>(1, 0, 0), vec3<f32>(1, 0, 0),
    vec3<f32>(1, 0, 0), vec3<f32>(1, 0, 0), vec3<f32>(1, 0, 0),
    // -X normals
    vec3<f32>(-1, 0, 0), vec3<f32>(-1, 0, 0), vec3<f32>(-1, 0, 0),
    vec3<f32>(-1, 0, 0), vec3<f32>(-1, 0, 0), vec3<f32>(-1, 0, 0),
    // +Y normals
    vec3<f32>(0, 1, 0), vec3<f32>(0, 1, 0), vec3<f32>(0, 1, 0),
    vec3<f32>(0, 1, 0), vec3<f32>(0, 1, 0), vec3<f32>(0, 1, 0),
    // -Y normals
    vec3<f32>(0, -1, 0), vec3<f32>(0, -1, 0), vec3<f32>(0, -1, 0),
    vec3<f32>(0, -1, 0), vec3<f32>(0, -1, 0), vec3<f32>(0, -1, 0),
    // +Z normals
    vec3<f32>(0, 0, 1), vec3<f32>(0, 0, 1), vec3<f32>(0, 0, 1),
    vec3<f32>(0, 0, 1), vec3<f32>(0, 0, 1), vec3<f32>(0, 0, 1),
    // -Z normals
    vec3<f32>(0, 0, -1), vec3<f32>(0, 0, -1), vec3<f32>(0, 0, -1),
    vec3<f32>(0, 0, -1), vec3<f32>(0, 0, -1), vec3<f32>(0, 0, -1)
);

@vertex
fn main(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32, in: InstanceInput) -> VertexOutput {
    // fetch per-particle attributes
    let base_pos    = in.posIn;
    var life_size   = in.lifeTimeIn / 2.0;
    let material_id = in.matIDIn;

    // expand to cube corner + scale
    let corner = CUBE_VERTICES[vertex_index] * life_size;
    let world_pos = vec4<f32>(base_pos + corner, 1.0);

    var clip = ubo.trans_w2s * world_pos;
    clip.z = 1.0 + clip.z;

    var out: VertexOutput;

    out.position = clip;

    let norm = CUBE_NORMALS[vertex_index];
    out.mat_norm = vec4<u32>(material_id, vec3<u32>(norm));

    return out;
}