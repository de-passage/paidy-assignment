use std::sync::{mpsc, Arc, Mutex};
use std::thread;

/// Simple threadpool, joining all threads on drop.
///
/// Heavily inspired by the one in the Rust book:
/// https://doc.rust-lang.org/book/ch20-02-multithreaded.html
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    /// Create a new ThreadPool with `size` threads.
    ///
    /// 'size' must be greater than 0.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0, "ThreadPool size must be greater than 0");

        let mut workers = Vec::with_capacity(size);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver)));
        }
        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Queue a task to run on the threadpool when a worker is available.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
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
type Job = Box<dyn FnOnce() + Send + 'static>;

/// Worker struct, holding a thread handle.
struct Worker {
    handle: Option<thread::JoinHandle<()>>,
}

/// Create a new worker that will execute jobs from the given receiver until this one is closed.
impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let handle = thread::spawn(move || loop {
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

    #[test]
    fn test_threadpool() {
        // Put this somewhere else when possible, it's very unlikely that it will fail,
        // but it's slow and not super reliable (I have seen it fail).
        let pool = super::ThreadPool::new(10);
        let results = Arc::new(Mutex::new(Vec::<u64>::new()));

        for i in 0..10 {
            let vec_handle = Arc::clone(&results);
            pool.execute(move || {
                thread::sleep(std::time::Duration::from_millis(10 - i));
                vec_handle.lock().unwrap().push(i);
            });
        }

        while results.lock().unwrap().len() < 10 {
            thread::sleep(std::time::Duration::from_millis(1));
        }

        let results = results.lock().unwrap().clone();
        assert_eq!(results.len(), 10);
        assert_eq!(results, vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0])
    }

}
