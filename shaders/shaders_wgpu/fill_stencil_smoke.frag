// fillStencilSmoke.frag (WGSL)

// Define structure for multiple render targets and fragment depth output
struct FragmentOutput {
    // Match the locations from GLSL 'out' variables
    @location(0) far_depth_out: f32,
    @location(1) near_depth_out: f32,
    // Built-in for writing fragment depth
    @builtin(frag_depth) frag_depth: f32,
};

// --- Fragment Entry Point ---
@fragment
fn main(
    @location(0) end_depth_in: f32, // Input from vertex shader
    @builtin(front_facing) is_front: bool // Built-in for front-facing check
) -> FragmentOutput {
    var output: FragmentOutput;

    // Set fragment depth based on front-facing property
    if (!is_front) {
        output.frag_depth = end_depth_in - 0.01;
    } else {
        output.frag_depth = end_depth_in;
    }

    // Write the input depth to both color attachments
    // The min/max blending happens in the pipeline state
    output.far_depth_out = end_depth_in;
    output.near_depth_out = end_depth_in;

    // Stencil value is written by the pipeline state, not explicitly here.
    return output;
}