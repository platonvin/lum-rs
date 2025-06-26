struct FragmentOutput {
    // These are just depth textures
    // But to utilize min/max hw, we use color with (min/max) blending
    @location(0) far_depth_output: f32,
    @location(1) near_depth_output: f32,
};

@fragment
fn main(
    @location(0) end_depth_input: f32,
    @builtin(front_facing) is_front_facing: bool
) -> FragmentOutput {
    var output: FragmentOutput;

    output.far_depth_output = end_depth_input;
    output.near_depth_output = end_depth_input;

    // If not discarded, stencil value is written.
    // We use it as cheap "culling" of expensive shader (GPUs have really fast hw stencil tests).
    return output;
}
