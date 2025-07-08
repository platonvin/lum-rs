use crate::*;

pub(crate) const PERLIN_SEED: i32 = 1337;

pub(crate) const PERLIN_FREQUENCY: f32 = 0.02;

#[spirv(compute(threads(8, 8, 1)))]
#[unsafe(no_mangle)]
pub fn comp_2d(
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

pub(crate) const CELLULAR_SEED: i32 = 42;

pub(crate) const CELLULAR_BASE_FREQ: f32 = 3.0;

pub(crate) const CELLULAR_OCTAVES: i32 = 4;

pub(crate) const CELLULAR_LACUNARITY: f32 = 2.0;

pub(crate) const CELLULAR_GAIN: f32 = 0.5;

#[spirv(compute(threads(4, 4, 4)))]
#[unsafe(no_mangle)]
pub fn comp_3d(
    #[spirv(descriptor_set = 0, binding = 0)] noise_image: &Image!(
        3D,
        format = rgba32f,
        sampled = false
    ),

    #[spirv(global_invocation_id)] global_invocation_id: UVec3,
) {
    let fpix = global_invocation_id.xyz().as_vec3() + 0.5;
    let ipix = fpix.as_ivec3();

    let mut noise_generator = FastNoiseLite::with_seed(CELLULAR_SEED);
    noise_generator.set_noise_type(Some(NoiseType::Cellular));
    noise_generator.set_fractal_type(Some(FractalType::FBm));
    noise_generator.set_fractal_octaves(Some(CELLULAR_OCTAVES));
    noise_generator.set_fractal_lacunarity(Some(CELLULAR_LACUNARITY));
    noise_generator.set_fractal_gain(Some(CELLULAR_GAIN));

    let scaled_coords = fpix / 32.0;

    let mut noise_val_channels = Vec4::ZERO;

    noise_generator.set_frequency(Some(CELLULAR_BASE_FREQ));
    noise_val_channels.x =
        noise_generator.get_noise_3d(scaled_coords.x, scaled_coords.y, scaled_coords.z);

    noise_generator.set_frequency(Some(CELLULAR_BASE_FREQ * 2.0));
    noise_val_channels.y =
        noise_generator.get_noise_3d(scaled_coords.x, scaled_coords.y, scaled_coords.z);

    noise_generator.set_frequency(Some(CELLULAR_BASE_FREQ * 4.0));
    noise_val_channels.z =
        noise_generator.get_noise_3d(scaled_coords.x, scaled_coords.y, scaled_coords.z);

    noise_generator.set_frequency(Some(CELLULAR_BASE_FREQ * 8.0));
    noise_val_channels.w =
        noise_generator.get_noise_3d(scaled_coords.x, scaled_coords.y, scaled_coords.z);

    unsafe {
        noise_image.write(ipix, noise_val_channels);
    }
}
