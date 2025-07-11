#define RAYS_PER_PROBE (32)
const ivec3 world_size = ivec3(48,48,16);
const float COLOR_ENCODE_VALUE = 6.0;

layout(constant_id = 0) const int BLOCK_PALETTE_SIZE_X = 64;
layout(constant_id = 1) const int STATIC_BLOCK_COUNT = 64;

const float PI = 3.1415926535;