const COLOR_ENCODE_VALUE: f32 = 1.0;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

struct FragmentOutput {
    @location(0) frame_color: vec4<f32>,
};

@group(0) @binding(0) var rendered_frame: texture_2d<f32>;

fn decode_color(encoded_color: vec3<f32>) -> vec3<f32> {
    return encoded_color * COLOR_ENCODE_VALUE;
}

fn encode_color(color: vec3<f32>) -> vec3<f32> {
    return color / COLOR_ENCODE_VALUE;
}

fn luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn change_luminance(c_in: vec3<f32>, l_out: f32) -> vec3<f32> {
    let l_in = luminance(c_in);
    return c_in * (l_out / l_in);
}

fn reinhard_extended(v: vec3<f32>, max_white: f32) -> vec3<f32> {
    let max_white_sq = max_white * max_white;
    let numerator = v * (vec3<f32>(1.0) + (v / vec3<f32>(max_white_sq)));
    return numerator / (vec3<f32>(1.0) + v);
}

fn reinhard_extended_luminance(v: vec3<f32>, max_white_l: f32) -> vec3<f32> {
    let l_old = luminance(v);
    let max_white_l_sq = max_white_l * max_white_l;
    let numerator = l_old * (1.0 + (l_old / max_white_l_sq));
    let l_new = numerator / (1.0 + l_old);
    return change_luminance(v, l_new);
}

fn uncharted2_tonemap_partial(x: vec3<f32>) -> vec3<f32> {
    let A = 0.15;
    let B = 0.50;
    let C = 0.10;
    let D = 0.20;
    let E = 0.02;
    let F = 0.30;
    return ((x * (A * x + C * B) + D * E) / (x * (A * x + B) + D * F)) - E / F;
}

fn uncharted2_filmic(v: vec3<f32>) -> vec3<f32> {
    let exposure_bias = 2.0;
    let curr = uncharted2_tonemap_partial(v * exposure_bias);
    let W = vec3<f32>(11.2);
    let white_scale = vec3<f32>(1.0) / uncharted2_tonemap_partial(W);
    return curr * white_scale;
}

fn aces_approx(v: vec3<f32>) -> vec3<f32> {
    let modified_v = v * 0.6;
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((modified_v * (a * modified_v + b)) / (modified_v * (c * modified_v + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn tonemap(color: vec3<f32>) -> vec3<f32> {
    // return reinhard_extended(color, 5.0);
    return reinhard_extended_luminance(color, 5.0);
    // return uncharted2_filmic(color);
    // return aces_approx(color);
}

fn adjust_brightness(color: vec3<f32>, value: f32) -> vec3<f32> {
    return color + value;
}

fn adjust_contrast(color: vec3<f32>, value: f32) -> vec3<f32> {
    return vec3<f32>(0.5) + (1.0 + value) * (color - vec3<f32>(0.5));
}

fn adjust_exposure(color: vec3<f32>, value: f32) -> vec3<f32> {
    return (1.0 + value) * color;
}   

fn adjust_saturation(color: vec3<f32>, value: f32) -> vec3<f32> {
    let grayscale = luminance(color);
    return mix(vec3<f32>(grayscale), color, 1.0 + value);
}

@fragment
fn main(@builtin(position) pos: vec4<f32>) -> FragmentOutput {
    let tex_coord = vec2<i32>(pos.xy);
    let encoded_color = textureLoad(rendered_frame, tex_coord, 0).rgb;
    var color = decode_color(encoded_color);

    // color = adjust_saturation(color, 0.1);
    // color = adjust_contrast(color, 0.1);
    // color = adjust_exposure(color, 0.5);
    // color = tonemap(color);

    var out: FragmentOutput;
    out.frame_color = vec4<f32>(color, 1.0);
    return out;
}
