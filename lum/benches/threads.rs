use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rayon::ThreadPool;
use std::thread;
use lum::containers::Multiprocessor;

// A dummy function to simulate (no) dispatch work
// - oh but compiler can just remove all the function calls 
// - if it can for some crate, than that is a very good crate. Currently, its not
fn just_function() {}

// Mine: reuse the thread pool
fn bench_mine(pool: &Multiprocessor, thread_count: usize) {
    // mine always dispatches the same amount of threads
    pool.dispatch(thread_count, |_| just_function());
}

// Rayon: reuse the thread pool
fn bench_rayon(pool: &ThreadPool, thread_count: usize) {
    pool.scope(|s| {
        for _ in 0..thread_count {
            s.spawn(|_| just_function());
        }
    });
}

// fn bench_tokio(rt: &Runtime, barrier: Arc<Barrier>, thread_count: usize) {
//     rt.block_on(async {
//         let mut handles = vec![];

//         for _ in 0..thread_count {
//             let barrier = barrier.clone();
//             handles.push(tokio::spawn(async move {
//                 barrier.wait();
//                 just_function();
//             }));
//         }

//         for handle in handles {
//             handle.await.unwrap();
//         }
//     });
// }

// fn bench_async_std(barrier: Arc<Barrier>, thread_count: usize) {
//     task::block_on(async {
//         let mut handles = vec![];

//         for _ in 0..thread_count {
//             let barrier = barrier.clone();
//             handles.push(task::spawn(async move {
//                 barrier.wait();
//                 just_function();
//             }));
//         }

//         for handle in handles {
//             handle.await;
//         }
//     });
// }

// fn bench_std_threads(barrier: Arc<Barrier>, thread_count: usize) {
//     let mut handles = vec![];

//     for _ in 0..thread_count {
//         let barrier = barrier.clone();
//         handles.push(thread::spawn(move || {
//             barrier.wait();
//             just_function();
//         }));
//     }

//     for handle in handles {
//         handle.join().unwrap();
//     }
// }

// Dispatch benchmark group
fn bench_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("Thread Pool Dispatch");
    let native_thread_count = thread::available_parallelism().unwrap().get();
    let thread_count = native_thread_count - 2;
    let thread_counts = [
        thread_count, 
        // native_thread_count
        ];


    for &thread_count in &thread_counts {
        let mine_pool = Multiprocessor::new();
            bench_mine(&mine_pool, thread_count);
            group.bench_with_input(BenchmarkId::new("mine", thread_count), &thread_count, |b, &tc| {
                b.iter(|| bench_mine(&mine_pool, tc));
            });
        drop(mine_pool);

        let rayon_pool = rayon::ThreadPoolBuilder::new().num_threads(thread_count).build().unwrap();
            bench_rayon(&rayon_pool, thread_count);
            group.bench_with_input(
                BenchmarkId::new("rayon", thread_count),
                &thread_count,
                |b, &tc| {
                    b.iter(|| bench_rayon(&rayon_pool, tc));
                },
            );
        drop(rayon_pool);
    }

    group.finish();
}

criterion_group!(benches, bench_dispatch);
criterion_main!(benches);
