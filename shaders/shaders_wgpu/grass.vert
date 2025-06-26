const BLOCK_PALETTE_SIZE_X: i32 = 64;
const STATIC_BLOCK_COUNT: i32 = 15;
const PI: f32 = 3.1415926535;
const world_size: vec3<i32> = vec3<i32>(48, 48, 16);
const VERTICES_PER_BLADE: u32 = 6u;
const MAX_HEIGHT: f32 = 3.0;

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

struct PushConstants {
    // position of a block in the world
    shift: vec4<f32>, 
    // Packed: (grid_size, time_seed_related, x_flip_flag, y_flip_flag)
    stxy: vec4<i32>,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(0) @binding(1) var state_tex: texture_2d<f32>;
@group(0) @binding(2) var linear_samp: sampler; 
@group(1) @binding(0) var<storage, read> pco_shared: array<PushConstants>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) mat_norm: vec4<u32>,
};

struct BladeGeometry {
    position_in_tile: vec3<f32>,
    normal: vec3<f32>,
}

fn hash21(p: vec2<u32>) -> u32 {
    var p_mut = p * vec2<u32>(73333u, 7777u);
    p_mut = p_mut ^ (vec2<u32>(3333777777u) >> (p_mut >> vec2<u32>(28u)));
    let n = p_mut.x * p_mut.y;
    return n ^ (n >> 15u);
}

// random f32 in  [0, 1)
fn rand(p: vec2<f32>) -> f32 {
    let h = hash21(bitcast<vec2<u32>>(p));
    // Normalize u32 to f32 [0,1)
    return f32(h) * (1.0 / 4294967295.0);
}

fn square(a: f32) -> f32 { return a * a; }

fn get_blade_width(height: f32) -> f32 {
    let max_h = MAX_HEIGHT - 1.0;
    return (max_h - height) / max_h; 
}

fn rotate_blade_vert(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>) {
    let angle = rnd01 * PI * 2.0;
    let cos_rot = cos(angle);
    let sin_rot = sin(angle);

    let vx = (*vertex).x;
    let vy = (*vertex).y;
    (*vertex).x = vx * cos_rot + vy * sin_rot;
    (*vertex).y = -vx * sin_rot + vy * cos_rot;

    let nx = (*normal).x;
    let ny = (*normal).y;
    (*normal).x = nx * cos_rot + ny * sin_rot;
    (*normal).y = -nx * sin_rot + ny * cos_rot;
}

fn displace_blade(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>) {
    let shift = vec2<f32>(sin(rnd01 * 42.1424) * 0.5, 
    cos(rnd01 * 58.1424) * 0.5);
    (*vertex).x = (*vertex).x + shift.x;
    (*vertex).y = (*vertex).y + shift.y;
}

fn curve_blade_vert(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>) {
    (*vertex).y = ((*vertex).z / MAX_HEIGHT) * (0.5 + rnd01);
}

// wind-induced offset from the state texture
fn load_offset(local_pos: vec2<f32>, pco: PushConstants) -> vec2<f32> {
    let world_pos = local_pos * 16.0 + pco.shift.xy;
    let state_uv = world_pos / (vec2<f32>(world_size.xy) * 16.0);
    // let offset = textureSample(state_tex, linear_samp, state_uv).xy;
    let offset = vec2f(0.0);
    return offset;
}

fn wiggle_blade_vert(rnd01: f32, vertex: ptr<function, vec3<f32>>, normal: ptr<function, vec3<f32>>, pos_in_tile: vec2<f32>, pco: PushConstants) {
    let global_offset = load_offset(pos_in_tile, pco);

    var local_offset = vec2(0.0);
    let base_freq = 1.0;
    let freq_step = 1.2;
    let ampl = 0.05;
    let t = f32(pco.stxy.y) / 200.0; // Magic!

    for (var freq = base_freq; freq < 4.0; freq = freq + freq_step) {
        local_offset.x = local_offset.x + sin(t * freq + rnd01 * 400.0 * freq + 0.0) * ampl;
        local_offset.y = local_offset.y + sin(t * freq + rnd01 * 400.0 * freq + 1.5) * ampl; 
    }

    let offset = local_offset + global_offset;
    (*vertex).x = (*vertex).x + offset.x * (*vertex).z * 1.0;
    (*vertex).y = (*vertex).y + offset.y * (*vertex).z * 1.0;
    // Note: Normal is not explicitly recalculated here after wiggling
    // It's assumed reasonably correct
}


fn get_blade_vert(blade_vertex_id: u32, rand01: f32, relative_pos_in_tile: vec2<f32>, pco: PushConstants) -> BladeGeometry {
    var vertex: vec3<f32>;
    var normal: vec3<f32>;

    let z_height_level: f32 = floor(f32(blade_vertex_id) / 2.0);
    var x_pos: f32 = f32(blade_vertex_id % 2u);

    if (blade_vertex_id == (VERTICES_PER_BLADE - 1u)) {
        x_pos = 0.5;
    }

    vertex = vec3<f32>(x_pos, 0.0, z_height_level);

    let width = get_blade_width(vertex.z);
    let width_diff = 1.0 - width;

    vertex.x = width * vertex.x + width_diff / 2.0;

    let n1 = vec3<f32>(-0.5, 1.0, 0.0); // Normal for x_pos = 0.0 side
    let n2 = vec3<f32>(0.5, 1.0, 0.0); // Normal for x_pos = 1.0 side
    normal = normalize(mix(n1, n2, x_pos));

    // wizardry
    vertex.x = vertex.x * 3.7;
    vertex.z = vertex.z * 2.0;

    curve_blade_vert(rand01, &vertex, &normal);
    rotate_blade_vert(rand01, &vertex, &normal);
    wiggle_blade_vert(rand01, &vertex, &normal, relative_pos_in_tile, pco);
    displace_blade(rand01, &vertex, &normal);

    vertex.z = vertex.z * (1.5 + (rand01 * 1.5) * (rand01 * 1.5));

    let tile_render_size = 16.0;
    let shift_in_tile = relative_pos_in_tile * tile_render_size;
    let vertex_pos_in_tile = vertex + vec3<f32>(shift_in_tile.x, shift_in_tile.y, 0.0);

    return BladeGeometry(vertex_pos_in_tile, normal);
}

@vertex
fn main(@builtin(vertex_index) vertex_idx: u32, @builtin(instance_index) instance_idx: u32) -> VertexOutput {
    // instance_idx is not quite the batch id yet
    // total index count is batch_count * blades_per_batch
    let blades_per_batch = u32(10*10);
    let batch_index = instance_idx / blades_per_batch;
    let blade_index = instance_idx % blades_per_batch;
    let pco = pco_shared[batch_index];

    var output: VertexOutput;

    let sub_blade_id = vertex_idx / VERTICES_PER_BLADE; 
    let blade_id = i32(blade_index + sub_blade_id); 
    let blade_vertex_id = vertex_idx % VERTICES_PER_BLADE;

    let size = pco.stxy.x; 
    let x_flip = pco.stxy.z;
    let y_flip = pco.stxy.w;

    var blade_x = blade_id % size;
    var blade_y = blade_id / size;
    if (x_flip == 0) { blade_x = size - 1 - blade_x; }
    if (y_flip != 0) { blade_y = size - 1 - blade_y; }

    let relative_pos_in_tile = (vec2<f32>(f32(blade_x), f32(blade_y)) + 0.5) / f32(size);

    let rand01 = rand(relative_pos_in_tile + pco.shift.xy);

    let blade_geom = get_blade_vert(blade_vertex_id, rand01, relative_pos_in_tile, pco);
    var final_normal = blade_geom.normal;

    let world_pos = vec4<f32>(blade_geom.position_in_tile, 1.0) + pco.shift;

    var clip_pos = ubo.trans_w2s * world_pos;
    clip_pos.z = 1.0 + clip_pos.z;
    output.clip_position = clip_pos;

    final_normal = normalize(final_normal);
    if (dot(ubo.camdir.xyz, final_normal) > 0.0) {
        final_normal = -final_normal;
    }

    let mat_id = select(10u, 9u, rand01 > 0.5);

    let packed_f32_normal = (final_normal + 1.0) * 0.5; 
    let norm_uint_rgb = vec3<u32>(packed_f32_normal * 255.0);

    output.mat_norm = vec4<u32>(mat_id, norm_uint_rgb);
    return output;
}
