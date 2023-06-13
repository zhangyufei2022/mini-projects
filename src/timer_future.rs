//! Timer future.
//! When the timer is created, we will start a thread and put the thread to sleep, and then notify the Future after the sleep is over.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

/// Share state between Future and waiting threads
struct SharedState {
    /// Whether the sleep is over
    completed: bool,
    /// When the sleep is over, the thread can use `waker` to notify `TimerFuture` to wake up the task
    waker: Option<Waker>,
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.shared_state.lock().unwrap();
        if state.completed {
            // 睡眠结束，在当前 poll 中， Future 可以被完成，则会返回 Poll::Ready(result)
            Poll::Ready(())
        } else {
            // 设置`waker`，这样新线程在睡眠结束后可以唤醒当前的任务，接着再次对`Future`进行`poll`操作,
            //
            // 下面的`clone`每次被`poll`时都会发生一次，实际上，应该是只`clone`一次更加合理。
            // 选择每次都`clone`的原因是： `TimerFuture`可以在执行器的不同任务间移动，如果只克隆一次，
            // 那么获取到的`waker`可能已经被篡改并指向了其它任务，最终导致执行器运行了错误的任务
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl TimerFuture {
    /// 创建一个新的`TimerFuture`，在指定的时间结束后，该`TimerFuture`可以完成
    pub fn new(duration: Duration) -> Self {
        let state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        let thread_state = state.clone();
        thread::spawn(move || {
            thread::sleep(duration);
            let mut shared_state = thread_state.lock().unwrap();
            shared_state.completed = true;
            if let Some(waker) = shared_state.waker.take() {
                waker.wake();
            }
        });
        TimerFuture {
            shared_state: state,
        }
    }
}
