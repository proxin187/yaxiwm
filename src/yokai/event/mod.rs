use yaxi::proto::Event;

use std::sync::{Mutex, Condvar};
use std::collections::VecDeque;

use ipc::Arguments;

macro_rules! lock {
    ($mutex:expr) => {
        $mutex.lock().map_err(|_| Into::<Box<dyn std::error::Error>>::into("failed to lock"))
    }
}

pub enum EventType {
    XEvent(Event),
    Config(Arguments),
}

pub struct Queue<T> {
    queue: Mutex<VecDeque<T>>,
    cond: Condvar,
}

impl<T> Queue<T> {
    pub fn new() -> Queue<T> {
        Queue {
            queue: Mutex::new(VecDeque::new()),
            cond: Condvar::new(),
        }
    }

    pub fn push(&self, value: T) -> Result<(), Box<dyn std::error::Error>> {
        lock!(self.queue)?.push_back(value);

        self.cond.notify_all();

        Ok(())
    }

    pub fn wait(&self) -> Result<T, Box<dyn std::error::Error>> {
        let mut guard = lock!(self.queue)?;

        loop {
            if let Some(value) = guard.pop_front() {
                return Ok(value);
            } else {
                guard = self.cond.wait(guard).map_err(|_| Into::<Box<dyn std::error::Error>>::into("failed to wait"))?;
            }
        }
    }
}


