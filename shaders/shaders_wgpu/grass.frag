// grass.frag (WGSL)

// Define input structure matching vertex shader output
struct FragmentInput {
    // Flat interpolation for integer/packed data
    @location(0) @interpolate(flat) mat_norm: vec4<u32>,
};

// Define output structure
struct FragmentOutput {
    // Match GLSL output location and type (uvec4 -> vec4<u32>)
    @location(0) outMatNorm: vec4<u32>,
};

// Apply early depth testing, equivalent to layout(early_fragment_tests) in;
@fragment @early_depth_test
fn main(input: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;

    // Pass the input directly to the output
    output.outMatNorm = input.mat_norm;

    return output;
}