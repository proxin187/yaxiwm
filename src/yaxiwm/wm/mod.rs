use crate::config::{Configuration, Insert};
use crate::event::{Queue, EventType};
use crate::tree::Node;
use crate::startup;

use yaxi::display::{self, Display};
use yaxi::window::Window;
use yaxi::proto::Event;

use std::sync::Arc;
use std::thread;

use ipc::{Arguments, Command, NodeCommand, ConfigCommand};


#[derive(Clone, Copy)]
pub struct Area {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Area {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Area {
        Area {
            x,
            y,
            width,
            height,
        }
    }
}

pub struct Desktop {
    clients: Node,
    area: Area,
}

impl Desktop {
    pub fn new(root: Window, area: Area) -> Desktop {
        Desktop {
            clients: Node::root(root),
            area,
        }
    }

    pub fn tile(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.clients.partition(self.area)
    }
}

pub struct Screen {
    desktops: Vec<Desktop>,
    area: Area,
}

impl Screen {
    pub fn new(area: Area) -> Screen {
        Screen {
            desktops: Vec::new(),
            area,
        }
    }
}

pub struct WindowManager {
    display: Display,
    root: Window,
    events: Arc<Queue<EventType>>,
    screens: Vec<Screen>,
    config: Configuration,
}

impl WindowManager {
    pub fn new() -> Result<WindowManager, Box<dyn std::error::Error>> {
        let display = display::open(None)?;
        let root = display.default_root_window()?;

        Ok(WindowManager {
            display,
            root,
            events: Arc::new(Queue::new()),
            screens: Vec::new(),
            config: Configuration::new(),
        })
    }

    fn load_screens(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let xinerama = self.display.query_xinerama()?;

        for screen in xinerama.query_screens()? {
            self.screens.push(Screen::new(Area::new(screen.x, screen.y, screen.width, screen.height)));
        }

        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            Event::MapRequest { window, .. } => {
            },
            Event::UnmapNotify { window, .. } => {
            },
            _ => {},
        }

        Ok(())
    }

    fn handle_config(&mut self, args: Arguments) -> Result<(), Box<dyn std::error::Error>> {
        match args.command {
            Command::Node(node) => match node {
                NodeCommand::Insert { dir, ratio, toggle } => {
                    let insert = Insert::new(dir, ratio.unwrap_or(self.config.insert.ratio));

                    self.config.insert = (insert == self.config.insert && toggle)
                        .then(|| Insert::default())
                        .unwrap_or(insert);
                },
                NodeCommand::Kill => {},
                NodeCommand::Close => {},
            },
            Command::Config(config) => match config {
                ConfigCommand::PointerFollowsFocus => {},
                ConfigCommand::FocusFollowsPointer => {},
            },
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let display = self.display.clone();
        let events = self.events.clone();

        self.load_screens()?;

        thread::spawn(move || {
            listen(display, events).expect("failed to listen");
        });

        startup::startup()?;

        loop {
            match self.events.wait()? {
                EventType::XEvent(event) => {
                    self.handle_event(event)?;
                },
                EventType::Config(args) => {
                    self.handle_config(args)?;
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


