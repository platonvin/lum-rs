use containers::{array3d::RuntimeDims, Array3D};
use criterion::{criterion_group, criterion_main, Criterion};
use zarray::z3d::ZArray3D;

fn blur_array3d(src: &Array3D<u8, RuntimeDims>, radius: usize) -> Array3D<u16, RuntimeDims> {
    let (x_size, y_size, z_size) = src.dimensions();
    let mut result = Array3D::new_default(src.dims);

    for z in 0..z_size {
        for y in 0..y_size {
            for x in 0..x_size {
                let mut sum = 0u16;
                let mut count = 0u16;

                for dx in -(radius as isize)..=(radius as isize) {
                    for dy in -(radius as isize)..=(radius as isize) {
                        for dz in -(radius as isize)..=(radius as isize) {
                            let nx = x as isize + dx;
                            let ny = y as isize + dy;
                            let nz = z as isize + dz;

                            if nx >= 0
                                && nx < x_size as isize
                                && ny >= 0
                                && ny < y_size as isize
                                && nz >= 0
                                && nz < z_size as isize
                            {
                                sum += src[(nx as usize, ny as usize, nz as usize)] as u16;
                                count += 1;
                            }
                        }
                    }
                }

                result[(x, y, z)] = sum / count;
            }
        }
    }

    result
}

fn blur_zarray3d(src: &ZArray3D<u8>, radius: usize) -> ZArray3D<u16> {
    let (x_size, y_size, z_size) = src.dimensions();
    let mut result = ZArray3D::new_with_default(x_size, y_size, z_size);

    for z in 0..z_size as isize {
        for y in 0..y_size as isize {
            for x in 0..x_size as isize {
                let mut sum = 0u16;
                let mut count = 0u16;

                for dx in -(radius as isize)..=(radius as isize) {
                    for dy in -(radius as isize)..=(radius as isize) {
                        for dz in -(radius as isize)..=(radius as isize) {
                            // "safe" version
                            // if let Some(&value) = src.bounded_get(x + dx, y + dy, z + dz) {
                            //      sum += value as u16;
                            //     count += 1;
                            // }
                            let nx = x + dx;
                            let ny = y + dy;
                            let nz = z + dz;

                            if nx >= 0
                                && nx < x_size as isize
                                && ny >= 0
                                && ny < y_size as isize
                                && nz >= 0
                                && nz < z_size as isize
                            {
                                let value = src.get_unchecked(
                                    (x + dx) as usize,
                                    (y + dy) as usize,
                                    (z + dz) as usize,
                                );

                                sum += *value as u16;
                                count += 1;
                            }
                        }
                    }
                }

                result.set_unchecked(x as usize, y as usize, z as usize, sum / count)
                // .unwrap();
            }
        }
    }

    result
}

fn benchmark_blur(c: &mut Criterion) {
    let x_size = 48;
    let y_size = 48;
    let z_size = 16;
    let radius = 1;

    let dims = RuntimeDims {
        x: x_size,
        y: y_size,
        z: z_size,
    };

    let mut array3d = Array3D::new_default(dims);
    let mut zarray = ZArray3D::new_with_default(x_size, y_size, z_size);

    // Populate both arrays with sample data
    for x in 0..x_size {
        for y in 0..y_size {
            for z in 0..z_size {
                let value = ((x + y + z) as u16 % 256) as u8;
                array3d[(x, y, z)] = value;
                zarray.set(x, y, z, value).unwrap();
            }
        }
    }

    c.bench_function("blur_array3d", |b| {
        b.iter(|| blur_array3d(&array3d, radius))
    });
    c.bench_function("blur_zarray3d", |b| {
        b.iter(|| blur_zarray3d(&zarray, radius))
    });
}

criterion_group!(benches, benchmark_blur);
criterion_main!(benches);
