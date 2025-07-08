use crate::*;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct AoLut {
    pub world_shift_weight: Vec4,
    // pub weight_normalized: f32,
    pub screen_shift: Vec2,
    pub padding: Vec2,
}

#[repr(C)]
pub struct LutBuffer {
    pub lut: [AoLut; 8],
}

#[spirv(fragment)]
#[unsafe(no_mangle)]
pub fn main(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] ubo: &UniformBufferObject,
    #[spirv(uniform, descriptor_set = 0, binding = 1)] lut_buffer: &LutBuffer,
    #[spirv(input_attachment_index = 0, descriptor_set = 0, binding = 2)] mat_norm_input: &Image!(subpass, type=u32, sampled=false),
    #[spirv(descriptor_set = 0, binding = 3)] depth_buffer: &SampledImage<
        Image!(2D, type=f32, sampled),
    >,
    #[spirv(frag_coord)] gl_frag_coord: Vec4,

    frame_color: &mut Vec4,
) {
    let norm = load_norm(mat_norm_input);
    let initial_pix = gl_frag_coord.xy() / ubo.frame_size;
    let initial_depth = load_depth_sampled(depth_buffer, initial_pix);

    const SAMPLE_COUNT: i32 = 8;

    let mut total_ao = 0.0;

    for i in 0..SAMPLE_COUNT {
        let lut = lut_buffer.lut[i as usize];

        let screen_shift = lut.screen_shift;

        let current_depth = load_depth_sampled(depth_buffer, initial_pix + screen_shift);
        let depth_shift = current_depth - initial_depth;

        let relative_pos = lut.world_shift_weight.xyz() + ubo.camdir.xyz() * depth_shift;

        let direction = relative_pos.normalize();

        let ao = (direction.dot(norm)).max(0.0);

        let mut weight = lut.world_shift_weight.w;
        weight *= f32::sqrt((depth_shift + 8.0).clamp(0.0, 8.0) / 8.0);

        total_ao += ao * weight;
    }

    let obfuscation = total_ao;

    // *frame_color = encode_color(Vec3::splat(obfuscation)).extend(1.0);
    *frame_color = Vec3::ZERO.extend(obfuscation);
}
