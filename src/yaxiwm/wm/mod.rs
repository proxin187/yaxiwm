use crate::event::{Queue, EventType};

use yaxi::display::{self, Display};
use yaxi::window::Window;

use std::sync::Arc;
use std::thread;


pub struct WindowManager {
    display: Display,
    root: Window,
    events: Arc<Queue<EventType>>,
}

impl WindowManager {
    pub fn new() -> Result<WindowManager, Box<dyn std::error::Error>> {
        let display = display::open(None)?;
        let root = display.default_root_window()?;

        Ok(WindowManager {
            display,
            root,
            events: Arc::new(Queue::new()),
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let display = self.display.clone();
        let events = self.events.clone();

        thread::spawn(move || {
            listen(display, events).expect("failed to listen");
        });

        loop {
            match self.events.wait()? {
                EventType::XEvent(event) => {
                },
                EventType::Config() => {
                },
            }
        }
    }
}

fn listen(display: Display, events: Arc<Queue<EventType>>) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let event = display.next_event()?;

        events.push(EventType::XEvent(event))?;
    }
}


