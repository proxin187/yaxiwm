use crate::config::{Configuration, Insert};
use crate::event::{Queue, EventType};
use crate::tree::Node;
use crate::startup;

use yaxi::display::{self, Display};
use yaxi::window::{Window, WindowKind};
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
    clients: Option<Node>,
    area: Area,
}

impl Desktop {
    pub fn new(area: Area) -> Desktop {
        Desktop {
            clients: None,
            area,
        }
    }

    pub fn contains(&self, window: &Window) -> bool {
        match &self.clients {
            Some(clients) => clients.contains(window),
            None => false,
        }
    }

    pub fn insert(&mut self, window: Window, insert: Insert, point: &Window) {
        if let Some(clients) = &mut self.clients {
            clients.insert(window, insert, point);
        }
    }

    pub fn hide(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(clients) = &self.clients {
            clients.traverse(|window| {
                window.unmap(WindowKind::Window).map_err(|err| err.into())
            })?;
        }

        Ok(())
    }

    pub fn tile(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(clients) = &self.clients {
            clients.partition(self.area)?;
        }

        Ok(())
    }
}

pub struct Screen {
    desktops: Vec<Desktop>,
    current: usize,
    area: Area,
}

impl Screen {
    pub fn new(area: Area) -> Screen {
        Screen {
            desktops: Vec::new(),
            current: 0,
            area,
        }
    }

    pub fn contains(&self, window: &Window) -> bool {
        self.desktops.iter().any(|desktop| desktop.contains(window))
    }

    pub fn insert(&mut self, window: Window, insert: Insert, point: &Window) {
        if let Some(desktop) = self.desktops.get_mut(self.current) {
            desktop.insert(window, insert, point);
        }
    }

    pub fn tile(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(desktop) = self.desktops.get(self.current) {
            desktop.tile()?;
        }

        for (index, desktop) in self.desktops.iter().enumerate() {
            if index != self.current {
                desktop.hide()?;
            }
        }

        Ok(())
    }
}

pub struct WindowManager {
    display: Display,
    root: Window,
    focus: Window,
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
            root: root.clone(),
            focus: root,
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

    fn all<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(&mut Screen) -> Result<(), Box<dyn std::error::Error>>
    {
        for screen in self.screens.iter_mut() {
            f(screen)?;
        }

        Ok(())
    }

    fn focused<F>(&mut self, f: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(&mut Screen) -> Result<(), Box<dyn std::error::Error>>
    {
        for screen in self.screens.iter_mut() {
            if screen.contains(&self.focus) {
                f(screen)?;

                return Ok(());
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            Event::MapRequest { window, .. } => {
                let point = self.focus.clone();

                // TODO: fix this borrowing issue
                self.focused(|screen| {
                    let window = self.display.window_from_id(window)?;
                    let insert = self.config.insert.clone();

                    screen.insert(window, insert, &point);

                    Ok(())
                })?;
            },
            Event::UnmapNotify { window, .. } => {
                let window = self.display.window_from_id(window)?;
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


