use std::{
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Mutex,
    },
    thread,
    thread::JoinHandle,
};

// Struct representing the thread pool (aka multiprocessor)
// Why not rayon? Its too slow. $> cargo bench -p lum --bench threads
pub struct Multiprocessor {
    num_threads: usize,
    threads: Vec<JoinHandle<()>>,
    threads_active: Arc<AtomicI32>,
    should_stop: Arc<AtomicBool>,
    current_task: Arc<Mutex<Option<Box<dyn Fn(usize) + Send>>>>,
    thread_flags: Vec<Arc<AtomicBool>>,
}

impl Default for Multiprocessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Multiprocessor {
    pub fn new() -> Self {
        // at least 1 thread, but also leave one for the OS
        let num_threads = std::thread::available_parallelism().map(|n| n.get() - 1).unwrap_or(1);
        let threads_active = Arc::new(AtomicI32::new(0));
        let should_stop = Arc::new(AtomicBool::new(false));
        let current_task: Arc<Mutex<Option<Box<dyn Fn(usize) + Send>>>> =
            Arc::new(Mutex::new(None));

        let mut thread_flags = Vec::with_capacity(num_threads);
        let mut threads = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            thread_flags.push(Arc::new(AtomicBool::new(false)));
        }

        for i in 0..num_threads {
            let thread_flag = thread_flags[i].clone();
            let threads_active = threads_active.clone();
            let should_stop = should_stop.clone();
            let current_task = current_task.clone();

            let handle = thread::spawn(move || {
                // while not stopped explicitly
                while !should_stop.load(Ordering::Relaxed) {
                    // if corresponding flag is set to do some work
                    if thread_flag.swap(false, Ordering::Relaxed) {
                        // then do the work and set the flag back to false (work done)
                        if let Some(task) = current_task.lock().unwrap().as_ref() {
                            task(i);
                        }
                        // also substract one from the counter to let main thread know part of work is done
                        threads_active.fetch_sub(1, Ordering::Relaxed);
                    }
                }
            });
            threads.push(handle);
        }

        Multiprocessor {
            num_threads,
            threads,
            thread_flags,
            threads_active,
            should_stop,
            current_task,
        }
    }

    /// Dispatches a task to the thread pool
    /// NOTE: `func: F` must identify itself using the thread id
    /// (which means that work division is on your side)
    pub fn dispatch<F>(&self, dispatch_size: usize, func: F)
    where
        F: Fn(usize) + Send + Sync + 'static,
    {
        while self.threads_active.load(Ordering::Relaxed) != 0 {}

        {
            let mut task = self.current_task.lock().unwrap();
            *task = Some(Box::new(func));
        }

        // store how many threads will be working on the thing
        self.threads_active.store(dispatch_size as i32, Ordering::Relaxed);

        // set N threads to do work for workgroup_size of N
        for i in 0..dispatch_size {
            let flag = &self.thread_flags[i];
            flag.store(true, Ordering::Relaxed);
        }

        // while self.threads_active.load(Ordering::Relaxed) != 0 {}
        loop {
            let active = self.threads_active.load(Ordering::Relaxed);
            assert!(active >= 0, "threads_active should never be negative");
            assert!(
                active <= dispatch_size as i32,
                "threads_active should never be greater than num_threads"
            );
            if active == 0 {
                break;
            }
        }
    }

    /// Returns the number of threads in the pool
    pub fn used_thread_count(&self) -> usize {
        self.num_threads
    }

    /// Returns the dispatch size you should use for optimal perfomance
    pub fn optimal_dispatch_size(&self) -> usize {
        self.num_threads - 1 // one for sync
    }
}

impl Drop for Multiprocessor {
    fn drop(&mut self) {
        self.should_stop.store(true, Ordering::Relaxed);

        for handle in self.threads.drain(..) {
            handle.join().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{atomic::AtomicUsize, Arc, Mutex},
        time::Duration,
    };

    use super::*;

    #[test]
    fn test_thread_count() {
        let pool = Multiprocessor::new();
        let num_threads = pool.used_thread_count();
        assert!(num_threads > 0, "Thread count should be greater than zero");
    }

    #[test]
    fn test_single_task_execution() {
        let pool = Multiprocessor::new();
        let result = Arc::new(Mutex::new(None));
        let result_clone = result.clone();

        pool.dispatch(1, move |thread_id| {
            let mut result = result_clone.lock().unwrap();
            *result = Some(thread_id);
        });

        assert_eq!(
            *result.lock().unwrap(),
            Some(0),
            "Task should execute on thread 0"
        );
    }

    #[test]
    fn test_multiple_task_execution() {
        let pool = Multiprocessor::new();
        let num_threads = pool.used_thread_count();
        let results = Arc::new(Mutex::new(vec![false; num_threads]));
        let results_clone = results.clone();

        pool.dispatch(num_threads, move |thread_id| {
            let mut results = results_clone.lock().unwrap();
            results[thread_id] = true;
        });

        let results = results.lock().unwrap();
        assert!(
            results.iter().all(|&x| x),
            "All threads should have completed their tasks"
        );
    }

    #[test]
    fn test_task_order() {
        let pool = Multiprocessor::new();
        let num_threads = pool.used_thread_count();
        let results = Arc::new(Mutex::new(vec![]));
        let results_clone = results.clone();

        pool.dispatch(num_threads, move |thread_id| {
            let mut results = results_clone.lock().unwrap();
            results.push(thread_id);
        });

        let results = results.lock().unwrap();
        assert_eq!(results.len(), num_threads, "All threads should have run");
        assert!(
            results.iter().copied().all(|id| id < num_threads),
            "Thread IDs should be within range"
        );
    }

    #[test]
    fn test_concurrent_execution() {
        let pool = Multiprocessor::new();
        let num_threads = pool.used_thread_count();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        pool.dispatch(num_threads, move |_| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(100)); // Simulate work
        });

        assert_eq!(
            counter.load(Ordering::Relaxed),
            num_threads,
            "All threads should have completed their tasks"
        );
    }

    #[test]
    fn test_no_active_threads_after_completion() {
        let pool = Multiprocessor::new();
        let num_threads = pool.used_thread_count();

        pool.dispatch(num_threads, |_| {
            std::thread::sleep(Duration::from_millis(10));
        });

        assert_eq!(
            pool.threads_active.load(Ordering::Relaxed),
            0,
            "All threads should be idle after task completion"
        );
    }

    #[test]
    fn test_stress_multiple_dispatches() {
        let pool = Multiprocessor::new();
        let num_threads = pool.used_thread_count();
        let iterations = 100;
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..iterations {
            let counter_clone = counter.clone();
            pool.dispatch(num_threads, move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
        }

        assert_eq!(
            counter.load(Ordering::SeqCst),
            num_threads * iterations,
            "All tasks should have been executed across multiple dispatches"
        );
    }

    #[test]
    fn test_partial_thread_dispatch() {
        let pool = Multiprocessor::new();
        let workgroup_size = 2; // Use fewer threads than available
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        pool.dispatch(workgroup_size, move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(
            counter.load(Ordering::SeqCst),
            workgroup_size,
            "Only the specified number of threads should execute the task"
        );
    }

    // TODO: this test fails when multiple tests run in parallel and interfere with each other causing slowdown
    // global test mutex? --test-threads=1 ?

    // #[test]
    // fn test_task_timing() {
    //     let pool = ThreadPoolThatDoesNotSuck::new();
    //     let num_threads = pool.thread_count();

    //     let start_time = Instant::now();

    //     pool.dispatch(2, |_| {
    //         std::thread::sleep(Duration::from_millis(200));
    //     });
    //     let elapsed = start_time.elapsed();
    //     println!("elapsed: {elapsed:?}");

    //     if elapsed > Duration::from_secs(1) {
    //         panic!("Tasks should complete within a reasonable time");
    //     }
    // }
}
