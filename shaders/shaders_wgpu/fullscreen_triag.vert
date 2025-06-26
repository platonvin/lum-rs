// (single) triagle, that covers entire screen.

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    var output: VertexOutput;

    let uv = vec2<f32>(f32((vertex_idx << 1u) & 2u), f32(vertex_idx & 2u));

    output.clip_position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);

    return output;
}