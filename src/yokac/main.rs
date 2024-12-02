mod client;

use client::Client;
use ipc::Arguments;

use clap::Parser;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Arguments::parse();

    let mut client = Client::new()?;

    client.send(args)?;

    Ok(())
}


