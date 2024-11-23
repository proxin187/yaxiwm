use serde::{Serialize, Deserialize};

use std::os::unix::net::UnixStream;
use std::io::Write;
use std::env;


pub struct Client {
    stream: UnixStream,
}

impl Client {
    pub fn new() -> Result<Client, Box<dyn std::error::Error>> {
        let home = env::var("HOME")?;

        Ok(Client {
            stream: UnixStream::connect(format!("{home}/.config/yaxiwm/ipc"))?,
        })
    }

    pub fn send<T>(&mut self, object: T) -> Result<(), Box<dyn std::error::Error>>
    where
        T: Serialize + for<'a> Deserialize<'a>
    {
        let bytes = bincode::serialize(&object)?;

        self.stream.write_all(&bytes).map_err(|err| err.into())
    }
}


