// fullscreenTriag.vert (WGSL)

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    // Although GLSL had 'outUV' commented out, if you need UVs for a fullscreen pass:
    // @location(0) uv: vec2<f32>,
};

@vertex
fn main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    var output: VertexOutput;

    // Generate UV coordinates for a fullscreen triangle
    // (0,0), (2,0), (0,2) -> covers the viewport corners
    let uv = vec2<f32>(f32((vertex_idx << 1u) & 2u), f32(vertex_idx & 2u));

    // Convert UVs [0,2] to clip space coordinates [-1,1]
    // Resulting coordinates are (-1,-1), (3,-1), (-1, 3) which covers the screen.
    output.clip_position = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);

    // If UVs are needed in the fragment shader:
    // output.uv = uv; // Pass UVs (range 0.0 to 2.0)

    return output;
}