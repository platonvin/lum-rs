struct FragmentInput {
    @location(0) @interpolate(flat) mat_norm: vec4<u32>,
};

struct FragmentOutput {
    @location(0) @interpolate(flat) outMatNorm: vec4<u32>,
};

@fragment
fn main(input: FragmentInput) -> FragmentOutput {
    var output: FragmentOutput;

    output.outMatNorm = input.mat_norm;

    return output;
}