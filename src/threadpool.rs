use std::sync::{mpsc, Arc, Mutex};
use std::thread::{Scope, ScopedJoinHandle};

/// Simple threadpool, joining all threads on drop.
///
/// Heavily inspired by the one in the Rust book:
/// https://doc.rust-lang.org/book/ch20-02-multithreaded.html
pub struct ThreadPool<'a> {
    workers: Vec<Worker<'a>>,
    sender: Option<mpsc::Sender<Job<'a>>>,
}

impl<'a> ThreadPool<'a> {
    /// Create a new ThreadPool with `size` threads.
    ///
    /// 'size' must be greater than 0.
    pub fn new(size: usize, scope: &'a Scope<'a, '_>) -> ThreadPool<'a> {
        assert!(size > 0, "ThreadPool size must be greater than 0");

        let mut workers = Vec::with_capacity(size);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver), scope));
        }
        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Queue a task to run on the threadpool when a worker is available.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'a,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl<'a> Drop for ThreadPool<'a> {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in &mut self.workers {
            if let Some(thread) = worker.handle.take() {
                thread.join().unwrap();
            }
        }
    }
}

/// Type of jobs to be executed by the threadpool.
type Job<'a> = Box<dyn FnOnce() + Send + 'a>;

/// Worker struct, holding a thread handle.
struct Worker<'a> {
    handle: Option<ScopedJoinHandle<'a, ()>>,
}

/// Create a new worker that will execute jobs from the given receiver until this one is closed.
impl<'a> Worker<'a> {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Job<'a>>>>, scope: &'a Scope<'a, '_>) -> Worker<'a> {
        let handle = scope.spawn(move || loop {
            let message = receiver.lock().unwrap().recv();
            match message {
                Ok(job) => job(),
                Err(_) => break,
            }
        });
        Worker {
            handle: Some(handle),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread::{scope,sleep};

    #[test]
    fn test_threadpool() {
        scope(|scope| {
            // Put this somewhere else when possible, it's very unlikely that it will fail,
            // but it's slow and not super reliable (I have seen it fail).
            let pool = super::ThreadPool::new(10, scope);
            let results = Arc::new(Mutex::new(Vec::<u64>::new()));

            for i in 0..10 {
                let vec_handle = Arc::clone(&results);
                pool.execute(move || {
                    sleep(std::time::Duration::from_millis(10 - i));
                    vec_handle.lock().unwrap().push(i);
                });
            }

            while results.lock().unwrap().len() < 10 {
                sleep(std::time::Duration::from_millis(1));
            }

            let results = results.lock().unwrap().clone();
            assert_eq!(results.len(), 10);
            assert_eq!(results, vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0])
        });
    }
}
