#![no_std]
use common::*;

pub(crate) const PERLIN_SEED: i32 = 1337;

pub(crate) const PERLIN_FREQUENCY: f32 = 0.02;

#[spirv(compute(threads(8, 8, 1)))]
#[unsafe(no_mangle)]
pub fn main(
    #[spirv(descriptor_set = 0, binding = 0)] noise_image: &Image!(
        2D,
        format = r32f,
        sampled = false
    ),

    #[spirv(global_invocation_id)] global_invocation_id: UVec3,
) {
    let fpix = global_invocation_id.xy().as_vec2() + 0.5;
    let ipix = fpix.as_ivec2();

    let mut noise_generator = FastNoiseLite::with_seed(PERLIN_SEED);
    noise_generator.set_noise_type(Some(NoiseType::Perlin));
    noise_generator.set_frequency(Some(PERLIN_FREQUENCY));

    let scaled_coords = fpix / (2.0 * WORLD_SIZE.xy().as_vec2());

    let noise_val = noise_generator.get_noise_2d(scaled_coords.x, scaled_coords.y);

    unsafe {
        noise_image.write(ipix, Vec4::new(noise_val, 0.0, 0.0, 0.0));
    }
}
