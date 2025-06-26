#![feature(portable_simd)]
use containers::{
    array3d::{ConstDims, Dim3},
    Array3D, BitArray3d, Multiprocessor,
};
use criterion::{criterion_group, criterion_main, Criterion};
use lum::{
    assert_assume, for_zyx,
    types::{i8vec4, uvec3},
};
use qvek::i8vec4;

use std::{cell::UnsafeCell, simd::cmp::SimdPartialOrd, sync::Arc};
use std::{simd::i32x32, sync::Mutex};

type BlockId = i16;

struct Settings {
    world_size: uvec3,
}

type WorldSize = ConstDims<48, 48, 16>;
const WORLD_SIZE: WorldSize = WorldSize {};

struct World {
    blocks: Array3D<BlockId, WorldSize>,
    radiance_updates: Vec<i8vec4>,
}

impl World {
    fn new(size: uvec3) -> Self {
        World {
            blocks: Array3D::new_filled(WORLD_SIZE, 0),
            radiance_updates: Vec::with_capacity(
                size.x as usize * size.y as usize * size.z as usize,
            ),
        }
    }

    fn generate(&mut self) {
        // let mut rng = rand::rng();
        let (x_size, y_size, z_size) = (
            self.blocks.dimensions().0,
            self.blocks.dimensions().1,
            self.blocks.dimensions().2,
        );

        for z in 0..z_size {
            for y in 0..y_size {
                for x in 0..x_size {
                    // Higher Z has more air (surface-like distribution)
                    if (x + y + z) % 5 == 0 {
                        // 20% chance of block (simplified from weights)
                        self.blocks[(x, y, z)] =
                            std::hint::black_box((x + y * 2 + z * 4) as i16 % 10 + 1);
                    } else {
                        self.blocks[(x, y, z)] = std::hint::black_box(0);
                    }
                }
            }
        }
    }
}

// Original implementation
unsafe fn update_radiance_original(world: &mut World, settings: &Settings) {
    let mut set = Array3D::<bool, WorldSize>::new_filled(
        WORLD_SIZE,
        false, // each value in set corresponds to "if the block is already updated"
    );

    for z in 0..settings.world_size.z {
        for y in 0..settings.world_size.y {
            for x in 0..settings.world_size.x {
                let mut sum = 0;

                for dz in -1..=1 {
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let nx =
                                (x as i32 + dx).clamp(0, settings.world_size.x as i32 - 1) as usize;
                            let ny =
                                (y as i32 + dy).clamp(0, settings.world_size.y as i32 - 1) as usize;
                            let nz =
                                (z as i32 + dz).clamp(0, settings.world_size.z as i32 - 1) as usize;

                            sum += world.blocks[(nx, ny, nz)];
                        }
                    }
                }

                if sum > 0 {
                    world.radiance_updates.push(i8vec4!(x, y, z, 0));
                    set[(x as usize, y as usize, z as usize)] = true;
                }
            }
        }
    }

    std::hint::black_box(set);
}

// Optimized implementation (your version)
unsafe fn update_radiance_unrolled(world: &mut World, settings: &Settings) {
    let (sx, sy, sz) = (
        settings.world_size.x as i8,
        settings.world_size.y as i8,
        settings.world_size.z as i8,
    );
    let wx = sx - 1;
    let wy = sy - 1;
    let wz = sz - 1;
    let x_size = world.blocks.dimensions().0;
    let y_size = world.blocks.dimensions().1;
    assert_assume!(x_size == settings.world_size.x as usize);
    assert_assume!(y_size == settings.world_size.y as usize);

    for_zyx!(settings.world_size, |xx, yy, zz| {
        let x = xx as i8;
        let y = yy as i8;
        let z = zz as i8;
        let mut found = false;

        // Check center first (most likely case)
        let (nx, ny, nz) = (x, y, z);
        if nx >= 0 && nx <= wx && ny >= 0 && ny <= wy && nz >= 0 && nz <= wz {
            let idx = nx as usize + ny as usize * x_size + nz as usize * x_size * y_size;
            found = world.blocks.data[idx] > 0;
        }

        // Early exit pattern for neighbor checks
        macro_rules! check_neighbor {
            ($dx:expr, $dy:expr, $dz:expr) => {
                if !found {
                    let nx = (x + $dx).clamp(0, wx);
                    let ny = (y + $dy).clamp(0, wy);
                    let nz = (z + $dz).clamp(0, wz);
                    let idx = nx as usize + ny as usize * x_size + nz as usize * x_size * y_size;
                    found = world.blocks.data[idx] > 0;
                }
            };
        }

        // Manual unrolling with spatial coherence (same z)
        // Layer 0
        check_neighbor!(1, 0, 0);
        check_neighbor!(-1, 0, 0);
        check_neighbor!(0, 1, 0);
        check_neighbor!(0, -1, 0);
        check_neighbor!(1, 1, 0);
        check_neighbor!(1, -1, 0);
        check_neighbor!(-1, 1, 0);
        check_neighbor!(-1, -1, 0);

        // Layer +1
        check_neighbor!(0, 0, 1);
        check_neighbor!(1, 0, 1);
        check_neighbor!(-1, 0, 1);
        check_neighbor!(0, 1, 1);
        check_neighbor!(0, -1, 1);
        check_neighbor!(1, 1, 1);
        check_neighbor!(1, -1, 1);
        check_neighbor!(-1, 1, 1);
        check_neighbor!(-1, -1, 1);

        // Layer -1
        check_neighbor!(0, 0, -1);
        check_neighbor!(1, 0, -1);
        check_neighbor!(-1, 0, -1);
        check_neighbor!(0, 1, -1);
        check_neighbor!(0, -1, -1);
        check_neighbor!(1, 1, -1);
        check_neighbor!(1, -1, -1);
        check_neighbor!(-1, 1, -1);
        check_neighbor!(-1, -1, -1);

        if found {
            world.radiance_updates.push(i8vec4!(x, y, z, 0));
        }
    });
}

#[derive(Debug, Default)]
struct FakeSyncBool(UnsafeCell<bool>);
unsafe impl Sync for FakeSyncBool {}

unsafe fn get_block_with_neighbors_parallel(
    world: &Array3D<i16, WorldSize>,
    multiprocessor: &Multiprocessor,
) -> Vec<i8vec4> {
    let mut thread_count = multiprocessor.optimal_dispatch_size();
    let chunk_size = WORLD_SIZE.z().div_ceil(thread_count);

    // Wrap Array3D<bool> in UnsafeCell to allow interior mutability
    let visited = Arc::new(Array3D::<FakeSyncBool, WorldSize>::from_fn(
        WORLD_SIZE,
        || FakeSyncBool(UnsafeCell::new(false)),
    ));

    let world: &'static World = unsafe { std::mem::transmute(world) };

    // Shared vector wrapped in a mutex for final collection
    let radiance_updates = Arc::new(Mutex::new(Vec::new()));

    // when too many cores, overhead is just too high
    thread_count = thread_count.clamp(1, 2);

    multiprocessor.dispatch(thread_count, {
        let radiance_updates = radiance_updates.clone();
        let visited: Arc<Array3D<_, WorldSize>> = Arc::clone(&visited);

        move |thread_id| {
            let z_start = thread_id * chunk_size;
            let z_end = ((thread_id + 1) * chunk_size).min(WORLD_SIZE.z());

            // Each thread accumulates its own small vec
            let mut local_updates = Vec::with_capacity(
                chunk_size * (WORLD_SIZE.x() as usize * WORLD_SIZE.y() as usize) / 2,
            );

            for zz in z_start..z_end {
                for yy in 0..WORLD_SIZE.y() {
                    for xx in 0..WORLD_SIZE.x() {
                        'free: for dz in -1..=1 {
                            for dy in -1..=1 {
                                for dx in -1..=1 {
                                    let x =
                                        (xx as isize + dx).clamp(0, WORLD_SIZE.x() as isize - 1);
                                    let y =
                                        (yy as isize + dy).clamp(0, WORLD_SIZE.y() as isize - 1);
                                    let z =
                                        (zz as isize + dz).clamp(0, WORLD_SIZE.z() as isize - 1);

                                    if world.blocks[(x as usize, y as usize, z as usize)] != 0 {
                                        unsafe {
                                            *visited.get(xx, yy, zz).0.get() = true;
                                        }
                                        break 'free;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            for zz in z_start..z_end {
                for yy in 0..WORLD_SIZE.y() as usize {
                    for xx in 0..WORLD_SIZE.x() as usize {
                        if unsafe { *visited.get(xx, yy, zz).0.get() } {
                            local_updates.push(i8vec4::new(xx as i8, yy as i8, zz as i8, 0));
                        }
                    }
                }
            }

            // Append local_updates to the main vector in one locked operation
            radiance_updates.lock().unwrap().append(&mut local_updates);
        }
    });

    // idk why its needed
    let result = { radiance_updates.lock().unwrap().clone() };
    result
}

// turned to be the fastest
unsafe fn update_radiance_separated(world: &Array3D<BlockId, WorldSize>) -> Vec<i8vec4> {
    let mut included = Array3D::new_filled(WORLD_SIZE, false);
    let (sx, sy, sz) = (WORLD_SIZE.x(), WORLD_SIZE.y(), WORLD_SIZE.z());

    let mut count = 0;
    // First pass: Mark all positions in 3x3x3 neighborhoods around non-zero blocks
    for zz in 0..sz {
        for yy in 0..sy {
            for xx in 0..sx {
                'free: for dz in -1isize..=1 {
                    for dy in -1isize..=1 {
                        for dx in -1isize..=1 {
                            let x = (xx as isize + dx).max(0).min(sx as isize - 1);
                            let y = (yy as isize + dy).max(0).min(sy as isize - 1);
                            let z = (zz as isize + dz).max(0).min(sz as isize - 1);
                            let block = world.get_unchecked(x as usize, y as usize, z as usize);

                            assert_assume!((*block > 0) == (*block != 0));

                            if *block > 0 {
                                included.set_unchecked(xx, yy, zz, true);
                                count += 1;
                                //i want to
                                break 'free;
                            }
                        }
                    }
                }
            }
        }
    }

    // Second pass: Collect all marked positions
    let mut result = Vec::with_capacity(count);
    for x in 0..sx {
        for y in 0..sy {
            for z in 0..sz {
                if *included.get_unchecked(x, y, z) {
                    result.push(i8vec4::new(x as i8, y as i8, z as i8, 0));
                }
            }
        }
    }

    result
}

unsafe fn update_radiance_separated_bitarray(world: &Array3D<BlockId, WorldSize>) -> Vec<i8vec4> {
    let (sx, sy, sz) = (WORLD_SIZE.x(), WORLD_SIZE.y(), WORLD_SIZE.z());
    let mut included = BitArray3d::<u32, WorldSize>::new_filled(WORLD_SIZE, false);

    // First pass: Mark all positions in 3x3x3 neighborhoods around non-zero blocks

    for_zyx!(sx, sy, sz, |xx: usize, yy, zz| {
        if world[(xx, yy, zz)] != 0 {
            // effectively instead of clamp
            let start_x = xx.saturating_sub(1);
            let end_x = (xx + 1).min(sx - 1);
            let start_y = yy.saturating_sub(1);
            let end_y = (yy + 1).min(sy - 1);
            let start_z = zz.saturating_sub(1);
            let end_z = (zz + 1).min(sz - 1);

            for nx in start_x..=end_x {
                for ny in start_y..=end_y {
                    for nz in start_z..=end_z {
                        // we could push to stack in here. But it leads to worse asm, and also
                        included.set_unchecked(nx, ny, nz, true);
                    }
                }
            }
        }
    });

    // Second pass: Collect all marked positions
    let mut result = Vec::with_capacity(sx * sy * sz);
    for x in 0..sx {
        for y in 0..sy {
            for z in 0..sz {
                if included.get_unchecked(x, y, z) {
                    result.push(i8vec4::new(x as i8, y as i8, z as i8, 0));
                }
            }
        }
    }

    result
}

// not faster
#[allow(clippy::erasing_op)] // LOL
unsafe fn update_radiance_simd(world: &mut World, settings: &Settings) {
    // Precomputed memory offsets for neighbors (relative to current position)
    // They should be precomputed at init-time
    // TODO: NOTE: use different functions for different sizes - 48x48x16 barely fits in i16
    #[rustfmt::skip]
    const MEM_OFFSETS: [i32; 32] = [
        // -Z layer
        -1-48-48*48, (-1-48), -1-48+48*48,
        -1-48*48, -1, -1+48*48,
        -1+48-48*48, (-1+48), -1+48+48*48,
        
        // Same Z layer
        0-48-48*48, (0-48), 0-48+48*48,
        (0*48)-48*48,                 (48*48),
        48-48*48, 48, 48+48*48,
        
        // +Z layer
        1-48-48*48, (1-48), 1-48+48*48,
        1-48*48, 1, 1+48*48,
        1+48-48*48, (1+48), 1+48+48*48,
        
        // Padding (including self-check)
        0, 0, 0, 0, 0, 0
    ];

    let data = &world.blocks.data;

    for_zyx!(WORLD_SIZE.xyz(), |xx, yy, zz| {
        let base_idx = world.blocks.index_internal(xx, yy, zz) as i32;

        // Load memory offsets into SIMD vector
        let offsets = i32x32::from_array(MEM_OFFSETS);

        // Calculate neighbor indices relative to base index
        let neighbor_indices = offsets + i32x32::splat(base_idx);

        // Gather block value (not using SIMD. Is there "load from vector of pointers"?
        let mut blocks = [0i32; 32];
        for i in 0..32 {
            let idx = neighbor_indices[i] as usize;
            if idx < data.len() {
                blocks[i] = *data.get_unchecked(idx) as i32;
            }
        }

        // Check for any non-zero neighbors
        let block_vec = i32x32::from_array(blocks);
        if block_vec.simd_gt(i32x32::splat(0)).any() {
            world.radiance_updates.push(i8vec4::new(xx as i8, yy as i8, zz as i8, 0));
        }
    });
}

unsafe fn update_radiance_smearing(world: &Array3D<BlockId, WorldSize>, result: &mut Vec<i8vec4>) {
    let (sx, sy, sz) = (WORLD_SIZE.x(), WORLD_SIZE.y(), WORLD_SIZE.z());

    // 1. Initial pass: mark all non-zero blocks.
    let mut pass1 = BitArray3d::<u32, WorldSize>::new_filled(WORLD_SIZE, false);
    for z in 0..sz {
        for y in 0..sy {
            for x in 0..sx {
                if world[(x, y, z)] != 0 {
                    unsafe { pass1.set_unchecked(x, y, z, true) };
                }
            }
        }
    }

    let mut pass2 = BitArray3d::<u32, WorldSize>::new_filled(WORLD_SIZE, false);

    // 2. X-axis smear: pass1 -> pass2
    for z in 0..sz {
        for y in 0..sy {
            for x in 0..sx {
                let smear = (x > 0 && unsafe { pass1.get_unchecked(x - 1, y, z) })
                    || (x < sx - 1 && unsafe { pass1.get_unchecked(x + 1, y, z) });
                if smear {
                    unsafe { pass2.set_unchecked(x, y, z, true) };
                }
            }
        }
    }

    // 3. Y-axis smear: pass2 -> pass1
    pass1.fill(false); // Reuse pass1 as the destination
    for z in 0..sz {
        for y in 0..sy {
            for x in 0..sx {
                let smear = (y > 0 && unsafe { pass2.get_unchecked(x, y - 1, z) })
                    || (y < sy - 1 && unsafe { pass2.get_unchecked(x, y + 1, z) });
                if smear {
                    unsafe { pass1.set_unchecked(x, y, z, true) };
                }
            }
        }
    }

    // 4. Z-axis smear: pass1 -> pass2
    pass2.fill(false); // Reuse pass2 as the destination
    for z in 0..sz {
        for y in 0..sy {
            for x in 0..sx {
                let smear = (z > 0 && unsafe { pass1.get_unchecked(x, y, z - 1) })
                    || (z < sz - 1 && unsafe { pass1.get_unchecked(x, y, z + 1) });
                if smear {
                    unsafe { pass2.set_unchecked(x, y, z, true) };
                }
            }
        }
    }

    for z in 0..sz {
        for y in 0..sy {
            for x in 0..sx {
                if unsafe { pass2.get_unchecked(x, y, z) } {
                    result.push(i8vec4::new(x as i8, y as i8, z as i8, 0));
                }
            }
        }
    }
}

unsafe fn update_radiance_smearing_bool(
    world: &Array3D<BlockId, WorldSize>,
    result: &mut Vec<i8vec4>,
) {
    let mut mask = Array3D::<bool, WorldSize>::new_filled(WORLD_SIZE, false);
    let (sx, sy, sz) = (WORLD_SIZE.x(), WORLD_SIZE.y(), WORLD_SIZE.z());

    // initial mark
    for z in 0..sz {
        for y in 0..sy {
            for x in 0..sx {
                unsafe {
                    if *world.get_unchecked(x, y, z) != 0 {
                        mask.set_unchecked(x, y, z, true);
                    }
                }
            }
        }
    }

    for z in 0..sz {
        for y in 0..sy {
            for x in 1..sx {
                unsafe {
                    if *mask.get_unchecked(x - 1, y, z) {
                        mask.set_unchecked(x, y, z, true);
                    }
                }
            }
            for x in (0..sx - 1).rev() {
                unsafe {
                    if *mask.get_unchecked(x + 1, y, z) {
                        mask.set_unchecked(x, y, z, true);
                    }
                }
            }
        }
    }

    for z in 0..sz {
        for x in 0..sx {
            for y in 1..sy {
                unsafe {
                    if *mask.get_unchecked(x, y - 1, z) {
                        mask.set_unchecked(x, y, z, true);
                    }
                }
            }
            for y in (0..sy - 1).rev() {
                unsafe {
                    if *mask.get_unchecked(x, y + 1, z) {
                        mask.set_unchecked(x, y, z, true);
                    }
                }
            }
        }
    }

    for y in 0..sy {
        for x in 0..sx {
            for z in 1..sz {
                unsafe {
                    if *mask.get_unchecked(x, y, z - 1) {
                        mask.set_unchecked(x, y, z, true);
                    }
                }
            }
            for z in (0..sz - 1).rev() {
                unsafe {
                    if *mask.get_unchecked(x, y, z + 1) {
                        mask.set_unchecked(x, y, z, true);
                    }
                }
            }
        }
    }

    for z in 0..sz {
        for y in 0..sy {
            for x in 0..sx {
                unsafe {
                    if *mask.get_unchecked(x, y, z) {
                        result.push(i8vec4::new(x as i8, y as i8, z as i8, 0));
                    }
                }
            }
        }
    }
}

fn benchmark_radiance(c: &mut Criterion) {
    let mut group = c.benchmark_group("radiance-updates");

    let (name, size) = ("small", uvec3::new(48, 48, 16));

    group.bench_function(&format!("SIMD-{}", name), |b| {
        let mut world = World::new(size);
        world.generate();
        let settings = Settings { world_size: size };

        b.iter(|| {
            world.radiance_updates.clear();
            unsafe {
                update_radiance_simd(
                    std::hint::black_box(&mut world),
                    std::hint::black_box(&settings),
                )
            };
        })
    });

    // group.bench_function(format!("parallel-{}", name), |b| {
    //     let mut world = World::new(size);
    //     world.generate();
    //     let settings = Settings { world_size: size };
    //     let multiprocessor = Multiprocessor::new();

    //     b.iter(|| {
    //         world.radiance_updates.clear();
    //         get_block_with_neighbors_parallel(
    //             std::hint::black_box(&world.blocks),
    //             size,
    //             &multiprocessor,
    //         );
    //     })
    // });

    group.bench_function(format!("separated-{}", name), |b| {
        let mut world = World::new(size);
        world.generate();

        b.iter(|| {
            world.radiance_updates.clear();
            unsafe { update_radiance_separated(std::hint::black_box(&world.blocks)) };
        })
    });

    group.bench_function(format!("separated-bits-{}", name), |b| {
        let mut world = World::new(size);
        world.generate();

        b.iter(|| {
            world.radiance_updates.clear();
            unsafe { update_radiance_separated_bitarray(std::hint::black_box(&world.blocks)) };
        })
    });

    group.bench_function(format!("original-{}", name), |b| {
        let mut world = World::new(size);
        world.generate();
        let settings = Settings { world_size: size };

        b.iter(|| {
            world.radiance_updates.clear();
            unsafe {
                update_radiance_original(
                    std::hint::black_box(&mut world),
                    std::hint::black_box(&settings),
                )
            };
        })
    });

    group.bench_function(format!("unrolled-{}", name), |b| {
        let mut world = World::new(size);
        world.generate();
        let settings = Settings { world_size: size };

        b.iter(|| {
            world.radiance_updates.clear();
            unsafe {
                update_radiance_unrolled(
                    std::hint::black_box(&mut world),
                    std::hint::black_box(&settings),
                )
            };
        })
    });

    group.bench_function(format!("smearing-{}", name), |b| {
        let mut world = World::new(size);
        world.generate();
        b.iter(|| {
            world.radiance_updates.clear();
            unsafe {
                update_radiance_smearing_bool(
                    std::hint::black_box(&world.blocks),
                    &mut world.radiance_updates,
                )
            };
        })
    });

    group.bench_function(format!("smearing-bits-{}", name), |b| {
        let mut world = World::new(size);
        world.generate();
        b.iter(|| {
            world.radiance_updates.clear();
            unsafe {
                update_radiance_smearing(
                    std::hint::black_box(&world.blocks),
                    &mut world.radiance_updates,
                )
            };
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_radiance);
criterion_main!(benches);
