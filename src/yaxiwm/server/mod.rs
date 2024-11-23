use crate::event::{Queue, EventType};

use std::os::unix::net::UnixListener;
use std::sync::Arc;
use std::env;


pub struct Server {
    listener: UnixListener,
    events: Arc<Queue<EventType>>,
}

impl Server {
    pub fn new(events: Arc<Queue<EventType>>) -> Result<Server, Box<dyn std::error::Error>> {
        let home = env::var("HOME")?;

        Ok(Server {
            listener: UnixListener::bind(format!("{home}/.config/yaxiwm/ipc"))?,
            events,
        })
    }

    pub fn listen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for stream in self.listener.incoming() {
            // TODO: we need some sort of system to serialize and deserialize
        }

        Ok(())
    }
}


