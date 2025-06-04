// benches/index_lookup.rs

use std::collections::HashMap;
use std::ptr;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_hashmap(c: &mut Criterion) {
    // Two values (could be anything; we only care about their addresses)
    let a = 0u32;
    let b = 1u32;

    // Array of references to them
    let array: [&u32; 2] = [&a, &b];

    // Build a HashMap<*const u32, usize> so that &a → 0, &b → 1
    let mut map: HashMap<*const u32, usize> = HashMap::new();
    map.insert(array[0] as *const u32, 0);
    map.insert(array[1] as *const u32, 1);

    // Pick a “target” pointer (here: address of `b`)
    let target: *const u32 = &b as *const u32;

    c.bench_function("hashmap_lookup", |bencher| {
        bencher.iter(|| {
            // black_box(target) prevents the compiler from folding this into a constant
            let idx = map.get(&black_box(target)).expect("must find target in hashmap");
            // use black_box on the result so it doesn’t optimize away
            black_box(idx);
        })
    });
}

fn bench_iter_find(c: &mut Criterion) {
    // Same setup: two values and an array of references
    let a = 0u32;
    let b = 1u32;
    let array: [&u32; 2] = [&a, &b];

    // Target pointer: &b
    let target: *const u32 = &b as *const u32;

    c.bench_function("iter_find", |bencher| {
        bencher.iter(|| {
            let mut idx: usize = 0;
            for (i, &item_ref) in array.iter().enumerate() {
                // Compare raw pointers
                if (item_ref as *const u32) == black_box(target) {
                    idx = i;
                    break;
                }
            }
            black_box(idx);
        })
    });
}

criterion_group!(benches, bench_hashmap, bench_iter_find);
criterion_main!(benches);
