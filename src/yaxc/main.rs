mod client;

use client::Client;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;

    Ok(())
}


