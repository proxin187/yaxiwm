use crate::event::{Queue, EventType};

use std::os::unix::net::UnixListener;
use std::sync::Arc;
use std::io::Read;
use std::thread;
use std::env;
use std::fs;

use ipc::Arguments;


pub struct Server {
    listener: UnixListener,
    events: Arc<Queue<EventType>>,
}

impl Server {
    pub fn new(events: Arc<Queue<EventType>>) -> Result<Server, Box<dyn std::error::Error>> {
        let path = format!("{}/.config/yokai/ipc", env::var("HOME")?);

        if fs::exists(&path)? {
            fs::remove_file(&path)?;
        }

        Ok(Server {
            listener: UnixListener::bind(path)?,
            events,
        })
    }

    pub fn listen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for stream in self.listener.incoming() {
            let mut buffer: Vec<u8> = Vec::new();

            stream?.read_to_end(&mut buffer)?;

            let args: Arguments = bincode::deserialize(&buffer)?;

            self.events.push(EventType::Config(args))?;
        }

        Ok(())
    }
}

pub fn spawn(events: Arc<Queue<EventType>>) {
    thread::spawn(move || {
        if let Ok(mut server) = Server::new(events) {
            let _ = server.listen();
        }
    });
}


