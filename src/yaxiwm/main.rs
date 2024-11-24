mod startup;
mod config;
mod server;
mod event;
mod tree;
mod wm;

use wm::WindowManager;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut wm = WindowManager::new()?;

    wm.run()
}


