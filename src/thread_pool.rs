//! Implementation of ThreadPool
use std::thread;

use chrono::Utc;
use crossbeam::channel;

pub struct ThreadPool {
    threads: Vec<Worker>,
    // Worker 结构体需要从线程池 TreadPool 的队列中获取待执行的代码，对于这类场景，消息传递非常适合：我们将使用消息通道(channel)作为任务队列。
    // 这里sender时通道的发送端
    sender: Option<channel::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Create a ThreadPool
    ///
    /// size：the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The function will panic if the 'size' is zero.
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let mut threads = Vec::with_capacity(size);
        // let (sender, receiver) = mpsc::channel();
        // let receiver = Arc::new(Mutex::new(receiver));
        let (sender, receiver) = channel::unbounded::<Job>();

        for i in 0..size {
            threads.push(Worker::new(i, receiver.clone()));
        }

        ThreadPool {
            threads,
            sender: Some(sender),
        }
        // 由上可知，线程池 ThreadPool 持有通道的发送端，然后通过 execute 方法来发送任务。
        // 那么谁持有接收端呢？答案是 Worker，它的内部线程将接收任务，然后进行处理。
    }

    /// submit a task
    pub fn submit<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
        println!("Sent a job to worker at [{}]", Utc::now());
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // 为 sender 增加 Option 封装，这样可以用 take 拿走所有权，跟之前的 thread 一样
        // 主动调用 drop 关闭发送端 sender
        drop(self.sender.take());
        println!("Dropped sender at [{}]", Utc::now());
        for worker in &mut self.threads {
            if let Some(thread) = worker.thread.take() {
                println!("Shutting down worker {} at [{}]", worker.id, Utc::now());
                // 虽然调用了 join ，但是目标线程依然不会停止，原因在于它们在无限的 loop 循环等待，
                // 看起来需要借用 channel 的 drop 机制：释放 sender发送端后，receiver 接收端会收到报错，然后再退出即可。
                thread.join().unwrap();
                println!("Worker {} shutted down at [{}]", worker.id, Utc::now());
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, reciever: channel::Receiver<Job>) -> Self {
        let handle = thread::spawn(move || loop {
            // receiver关闭之后，接收端recv()会返回一个错误，这里根据接收的消息进行不同的处理
            let job = reciever.recv();
            if let Ok(job) = job {
                println!("Worker {id} got a job at [{}]; executing.", Utc::now());
                job();
            } else {
                println!(
                    "Worker {id} disconnected; shutting down at [{}]",
                    Utc::now()
                );
                break;
            }
        });
        Worker {
            id,
            thread: Some(handle),
        }
    }
}
