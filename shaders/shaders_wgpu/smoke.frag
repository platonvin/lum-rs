const COLOR_ENCODE_VALUE: f32 = 1.0;
const MAX_STEPS: i32 = 8;
const TRESHOLD: f32 = 0.7;
const MULTIPLIER: f32 = 1.7;
const WORLD_SIZE: vec3<i32> = vec3<i32>(48, 48, 16);

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

struct Constants {
    shift: vec4<f32>,
    time: i32,
    size: i32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

struct FragmentOutput {
    @location(0) smoke_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> ubo: UboData;
@group(0) @binding(1) var smoke_depth_far: texture_2d<f32>;
@group(0) @binding(2) var smoke_depth_near: texture_2d<f32>;
@group(0) @binding(3) var radianceCache: texture_3d<f32>;
@group(0) @binding(4) var noise: texture_3d<f32>;
@group(0) @binding(5) var linear_sampler_tiled: sampler;
@group(1) @binding(0) var<uniform> pc: Constants;

fn sample_probe(probe_ipos: vec3<i32>, direction: vec3<f32>) -> vec3<f32> {
    let probe_ipos_clamped = clamp(probe_ipos, vec3<i32>(0), WORLD_SIZE);
    var subprobe_pos: vec3<i32>;
    subprobe_pos.x = probe_ipos_clamped.x;
    // i still cant get around the decision to force every browser to implement text parser for language thats does not even support FUCKING SWIZZLES
    subprobe_pos.y = probe_ipos_clamped.y;
    subprobe_pos.z = probe_ipos_clamped.z; 
    let light = textureLoad(radianceCache, subprobe_pos, 0).rgb;
    return clamp(light, vec3f(0.0), vec3f(2.0));
}

fn square(a: f32) -> f32 {
    return a * a;
}

fn sample_radiance(position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    var total_weight = 0.0;
    var total_colour = vec3<f32>(0.0);

    let zero_probe_ipos_float = floor(position - vec3<f32>(8.0)) / vec3<f32>(16.0);
    let zero_probe_ipos = clamp(vec3<i32>(zero_probe_ipos_float), vec3<i32>(0), WORLD_SIZE);
    let zero_probe_pos = vec3<f32>(zero_probe_ipos) * 16.0 + vec3<f32>(8.0);

    let alpha = clamp((position - zero_probe_pos) / 16.0, vec3<f32>(0.0), vec3<f32>(1.0));

    for (var i: i32 = 0; i < 8; i++) {
        let offset = (vec3<i32>(i, i >> 1, i >> 2) & vec3<i32>(1));

        var probe_weight = 1.0;
        var probe_colour = vec3<f32>(0.0);

        let probe_pos = zero_probe_pos + vec3<f32>(offset) * 16.0;
        let probeToPoint = probe_pos - position;
        let direction_to_probe = normalize(probeToPoint);

        let trilinear = mix(vec3<f32>(1.0) - alpha, alpha, vec3<f32>(offset));
        probe_weight = trilinear.x * trilinear.y * trilinear.z;

        let direction_weight = clamp(dot(direction_to_probe, normal), 0.1, 1.0);
        probe_weight *= direction_weight;

        probe_colour = sample_probe(zero_probe_ipos + offset, direction_to_probe);

        probe_weight = max(1e-7, probe_weight);
        total_weight += probe_weight;
        total_colour += probe_weight * probe_colour;
    }

    if (total_weight > 0.0) {
        return total_colour / total_weight;
    } else {
        return vec3<f32>(0.0);
    }
}

fn sample_radiance_no_normal(position: vec3<f32>) -> vec3<f32> {
    var total_weight = 0.0;
    var total_colour = vec3<f32>(0.0);

    let zero_probe_ipos_float = floor(position - vec3<f32>(8.0)) / vec3<f32>(16.0);
    let zero_probe_ipos = clamp(vec3<i32>(zero_probe_ipos_float), vec3<i32>(0), WORLD_SIZE);
    let zero_probe_pos = vec3<f32>(zero_probe_ipos) * 16.0 + vec3<f32>(8.0);

    let alpha = clamp((position - zero_probe_pos) / 16.0, vec3<f32>(0.0), vec3<f32>(1.0));

    for (var i: i32 = 0; i < 8; i++) {
        let offset = (vec3<i32>(i, i >> 1, i >> 2) & vec3<i32>(1));

        var probe_weight = 1.0;
        var probe_colour = vec3<f32>(0.0);

        let probe_pos = zero_probe_pos + vec3<f32>(offset) * 16.0;
        let probeToPoint = probe_pos - position;
        let direction_to_probe = normalize(probeToPoint);

        let trilinear = mix(vec3<f32>(1.0) - alpha, alpha, vec3<f32>(offset));
        probe_weight = trilinear.x * trilinear.y * trilinear.z;

        probe_colour = sample_probe(zero_probe_ipos + offset, direction_to_probe);

        total_weight += probe_weight;
        total_colour += probe_weight * probe_colour;
    }

    if (total_weight > 0.0) {
        return total_colour / total_weight;
    } else {
        return vec3<f32>(0.0);
    }
}

fn decode_depth(d: f32) -> f32 {
    return d * 1000.0;
}

fn load_depth_far(frag_coord: vec2<i32>) -> f32 {
    let d = textureLoad(smoke_depth_far, frag_coord, 0).x;
    return decode_depth(d);
}

fn load_depth_near(frag_coord: vec2<i32>) -> f32 {
    let d = textureLoad(smoke_depth_near, frag_coord, 0).x;
    return decode_depth(d);
}

fn get_origin_from_depth(depth: f32, clip_pos: vec2<f32>) -> vec3<f32> {
    return ubo.campos.xyz +
           (ubo.horizline_scaled.xyz * clip_pos.x) +
           (ubo.vertiline_scaled.xyz * clip_pos.y) +
           (ubo.camdir.xyz * depth);
}

fn rotate(v: vec2<f32>, a: f32) -> vec2<f32> {
    let s = sin(a);
    let c = cos(a);
    let m = mat2x2<f32>(c, s, -s, c);
    return m * v;
}

fn rotatem(a: f32) -> mat2x2<f32> {
    let s = sin(a);
    let c = cos(a);
    return mat2x2<f32>(c, s, -s, c);
}

fn decode_color(encoded_color: vec3<f32>) -> vec3<f32> {
    return encoded_color * COLOR_ENCODE_VALUE;
}

fn encode_color(color: vec3<f32>) -> vec3<f32> {
    return color / COLOR_ENCODE_VALUE;
}

@fragment
fn main(@builtin(position) pos: vec4<f32>) -> FragmentOutput {
    let gl_FragCoord = vec2<i32>(pos.xy);
    let direction = (ubo.camdir.xyz);;

    let near = load_depth_near(gl_FragCoord);
    let far = load_depth_far(gl_FragCoord);
    var diff = far - near;

    if far > near {
        diff = far - near;
    } else {
        diff = near - far;
    }

    let step_size = diff / f32(MAX_STEPS);

    // Lambert law
    // I = I0 * exp(-K * L)
    // dI = -K*dL * I0 * exp(-K * L)
    // In+1 = (1-denisty_n*ΔL) * In

    var I = 1.0;
    var position: vec3<f32>;
    var total_dencity = 0.0;

    var fraction = near;
    // let time = f32(u32(ubo.timeseed));
    let time = 1.0;
    for (var i: i32 = 0; i < MAX_STEPS; i++) {
        fraction += step_size;
        let pix: vec2<f32> = pos.xy;
        let clip_pos: vec2<f32> = (pix / ubo.frame_size) * 2.0 - vec2<f32>(1.0, 1.0);

        position = get_origin_from_depth(fraction, clip_pos);

        let voxel_pos = position;
        let noise_clip_pos = voxel_pos / 32.0;
        var noises: vec4<f32>;
        var wind_direction = vec3<f32>(1.0, 0.0, 0.0);
        let wind_rotate = rotatem(1.6);

        noises.x = textureSampleLevel(noise, linear_sampler_tiled, noise_clip_pos / 1.0 + wind_direction * time / 3500.0, 0.0).x;
        wind_direction.x = (wind_rotate * wind_direction.xy).x;
        wind_direction.y = (wind_rotate * wind_direction.xy).y;
        noises.y = textureSampleLevel(noise, linear_sampler_tiled, noise_clip_pos / 2.1 + wind_direction * time / 3000.0, 0.0).y;
        wind_direction.x = (wind_rotate * wind_direction.xy).x;
        wind_direction.y = (wind_rotate * wind_direction.xy).y;
        noises.z = textureSampleLevel(noise, linear_sampler_tiled, noise_clip_pos / 3.2 + wind_direction * time / 2500.0, 0.0).z;
        wind_direction.x = (wind_rotate * wind_direction.xy).x;
        wind_direction.y = (wind_rotate * wind_direction.xy).y;
        noises.w = textureSampleLevel(noise, linear_sampler_tiled, noise_clip_pos / 4.3 + wind_direction * time / 2000.0, 0.0).w;

        let close_to_border = clamp(diff, 0.1, 16.0) / 16.0;
        var dencity = (noises.x + noises.y + noises.z - noises.w / close_to_border) / 2.0 - TRESHOLD;
        dencity = clamp(dencity, 0.0, TRESHOLD) * MULTIPLIER;

        I = (1.0 - dencity * step_size) * I;
        total_dencity += dencity * step_size;
    }

    // let final_light = sample_radiance_no_normal(position);
    var smoke_opacity = 1.0 - I;
    // smoke_opacity = diff / 10.0;
    

    var out: FragmentOutput;
    out.smoke_color = vec4<f32>(encode_color(vec3f(0.15)), smoke_opacity);

    return out;
}