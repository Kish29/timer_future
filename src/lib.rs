use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, sync_channel, SyncSender};
use std::task::{Context, Poll, Waker};
use std::thread;
use futures::future::BoxFuture;
use futures::FutureExt;
use futures::task::{ArcWake, waker_ref};

pub struct SharedState<T> {
    completed: bool,
    waker: Option<Waker>,
    data: Option<T>,
}

pub struct ThreadFuture<T> {
    shared_state: Arc<Mutex<SharedState<T>>>,
}

impl<T> Future for ThreadFuture<T> where T: Clone + Debug + Send + Sync + 'static {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut ss = self.shared_state.lock().unwrap();
        if ss.completed {
            Poll::Ready(ss.data.take().unwrap())
        } else {
            ss.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<T> ThreadFuture<T> where T: Clone + Debug + Send + Sync + 'static {
    pub fn new(f: fn() -> T) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
            data: None,
        }));
        let trd_ss = shared_state.clone();
        thread::spawn(move || {
            let d = f();
            let mut ss = trd_ss.lock().unwrap();
            ss.completed = true;
            ss.data = Some(d);
            if let Some(wk) = ss.waker.take() {
                wk.wake();
            }
        });
        Self {
            shared_state
        }
    }
}

pub struct Executor<T> {
    ready_queue: Receiver<Arc<Task<T>>>,
}

impl<T> Executor<T> where T: Clone + Debug + Send + Sync + 'static {
    pub fn run(&self) {
        while let Ok(task) = self.ready_queue.recv() {
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let cx = &mut Context::from_waker(&*waker);
                match future.as_mut().poll(cx) {
                    Poll::Ready(d) => {
                        println!("{:?}", d)
                    }
                    Poll::Pending => {
                        *future_slot = Some(future)
                    }
                }
            }
        }
    }
}


pub struct Spawner<T> {
    task_sender: SyncSender<Arc<Task<T>>>,
}

impl<T> Spawner<T> where T: Clone + Debug + Send + Sync + 'static {
    pub fn spawn(&self, future: impl Future<Output=T> + Send + 'static) {
        let f = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(Some(f)),
            task_sender: self.task_sender.clone(),
        });
        self.task_sender.send(task).expect("task queue is overflow")
    }
}

pub struct Task<T> {
    future: Mutex<Option<BoxFuture<'static, T>>>,
    task_sender: SyncSender<Arc<Task<T>>>,
}

impl<T> ArcWake for Task<T> where T: Clone + Debug + Send + Sync + 'static {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.task_sender.send(arc_self.clone()).expect("task queue is overflow")
    }
}

pub fn new_executor_spawner<T: Clone + Debug + Send + Sync + 'static>() -> (Executor<T>, Spawner<T>) {
    const MAX_QUEUE_TASKS: usize = 10_000;
    let (task_sender, ready_queue) = sync_channel(MAX_QUEUE_TASKS);
    (Executor { ready_queue }, Spawner { task_sender })
}