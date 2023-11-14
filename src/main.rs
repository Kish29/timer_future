use std::thread;
use std::time::Duration;
use timer_future::{new_executor_spawner, ThreadFuture};

fn main() {
    let (executor, spawner) = new_executor_spawner();
    spawner.spawn(ThreadFuture::new(|| {
        thread::sleep(Duration::from_secs(3));
        String::from("I love rust!")
    }));
    spawner.spawn(ThreadFuture::new(|| {
        thread::sleep(Duration::from_secs(2));
        String::from("I love golang!")
    }));
    spawner.spawn(ThreadFuture::new(|| {
        thread::sleep(Duration::from_secs(4));
        String::from("I love java!")
    }));
    spawner.spawn(ThreadFuture::new(|| {
        thread::sleep(Duration::from_secs(1));
        String::from("I love c++!")
    }));
    drop(spawner);
    executor.run();
}