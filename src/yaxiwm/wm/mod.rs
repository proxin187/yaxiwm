use crate::config::{Configuration, Insert, Padding};
use crate::event::{Queue, EventType};
use crate::tree::Node;
use crate::startup;
use crate::server;

use yaxi::display::{self, Display, Atom};
use yaxi::window::{Window, WindowKind};
use yaxi::proto::{Event, EventMask, EventKind, RevertTo, ClientMessageData};

use std::sync::Arc;
use std::thread;

use ipc::{Arguments, Command, NodeCommand, DesktopCommand, ConfigCommand};


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

    pub fn remove(&mut self, window: &Window) {
        if let Some(clients) = &mut self.clients {
            clients.remove(window);
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

    pub fn tile(&self, padding: Padding) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(clients) = &self.clients {
            clients.partition(self.area, padding)?;
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

    pub fn resize(&mut self, size: usize) {
        if size >= self.desktops.len() {
            self.desktops.resize_with(size, || Desktop::new(self.area));
        } else {
            // TODO: finish this
            let excess = self.desktops.drain(size..self.len()).collect::<>;
        }
    }

    pub fn insert(&mut self, window: Window, insert: Insert, point: &Window) {
        if let Some(desktop) = self.desktops.get_mut(self.current) {
            desktop.insert(window, insert, point);
        }
    }

    pub fn remove(&mut self, window: &Window) {
        if let Some(desktop) = self.desktops.get_mut(self.current) {
            desktop.remove(window);
        }
    }

    pub fn tile(&self, padding: Padding) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(desktop) = self.desktops.get(self.current) {
            desktop.tile(padding)?;
        }

        for (index, desktop) in self.desktops.iter().enumerate() {
            if index != self.current {
                desktop.hide()?;
            }
        }

        Ok(())
    }
}

pub struct Atoms {
    wm_protocols: Atom,
    wm_delete: Atom,
}

impl Atoms {
    pub fn new(display: &Display) -> Result<Atoms, Box<dyn std::error::Error>> {
        Ok(Atoms {
            wm_protocols: display.intern_atom("WM_PROTOCOLS", false)?,
            wm_delete: display.intern_atom("WM_DELETE_WINDOW", false)?,
        })
    }
}

pub struct WindowManager {
    display: Display,
    root: Window,
    focus: Window,
    events: Arc<Queue<EventType>>,
    screens: Vec<Screen>,
    config: Configuration,
    atoms: Atoms,
}

impl WindowManager {
    pub fn new() -> Result<WindowManager, Box<dyn std::error::Error>> {
        let display = display::open(None)?;
        let root = display.default_root_window()?;

        root.select_input(&[
            EventMask::SubstructureNotify,
            EventMask::SubstructureRedirect,
            EventMask::EnterWindow,
            EventMask::FocusChange,
        ])?;

        let atoms = Atoms::new(&display)?;

        Ok(WindowManager {
            display,
            root: root.clone(),
            focus: root,
            events: Arc::new(Queue::new()),
            screens: Vec::new(),
            config: Configuration::new(),
            atoms,
        })
    }

    fn load_screens(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let xinerama = self.display.query_xinerama()?;

        for screen in xinerama.query_screens()? {
            self.screens.push(Screen::new(Area::new(screen.x, screen.y, screen.width, screen.height)));
        }

        Ok(())
    }

    fn all<F>(&mut self, mut f: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(usize, &mut Screen) -> Result<(), Box<dyn std::error::Error>>
    {
        for (index, screen) in self.screens.iter_mut().enumerate() {
            f(index, screen)?;
        }

        Ok(())
    }

    fn focused<F>(&mut self, mut f: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&mut Screen) -> Result<(), Box<dyn std::error::Error>>
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
        println!("event: {:?}", event);

        match event {
            Event::MapRequest { window, .. } => {
                let point = self.focus.clone();
                let window = self.display.window_from_id(window)?;
                let insert = self.config.insert.clone();
                let padding = self.config.padding.clone();

                window.select_input(&[
                    EventMask::SubstructureNotify,
                    EventMask::SubstructureRedirect,
                    EventMask::EnterWindow,
                    EventMask::FocusChange,
                ])?;

                window.set_border_pixel(self.config.border.normal)?;

                window.set_border_width(self.config.border.width)?;

                self.focused(|screen| {
                    screen.insert(window.clone(), insert.clone(), &point);

                    // TODO: we never reach here
                    println!("inserted");

                    screen.tile(padding)
                })?;
            },
            Event::UnmapNotify { window, .. } => {
                let window = self.display.window_from_id(window)?;

                self.all(|_, screen| {
                    screen.remove(&window);

                    Ok(())
                })?;

                if window == self.focus {
                    self.focus = self.root.clone();
                }
            },
            Event::EnterNotify { window, .. } => {
                if window != self.root.id() && window > 1 && self.config.pf.focus_follows {
                    let window = self.display.window_from_id(window)?;

                    window.set_input_focus(RevertTo::Parent)?;
                }
            },
            Event::FocusIn { window, .. } => {
                let window = self.display.window_from_id(window)?;

                self.focus.set_border_pixel(self.config.border.normal)?;

                window.set_border_pixel(self.config.border.focused)?;

                self.focus = window.clone();
            },
            _ => {},
        }

        Ok(())
    }

    fn handle_config(&mut self, args: Arguments) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: we need to retile when we get commands that affect tiling

        println!("config: {:?}", args);

        match args.command {
            Command::Node(node) => match node {
                NodeCommand::Insert { dir, ratio, toggle } => {
                    let insert = Insert::new(dir, ratio.unwrap_or(self.config.insert.ratio));

                    self.config.insert = (insert == self.config.insert && toggle)
                        .then(|| Insert::default())
                        .unwrap_or(insert);
                },
                NodeCommand::Desktop { desktop } => {
                    // TODO: send the node to another desktop
                },
                NodeCommand::Kill => {
                    if self.focus != self.root {
                        self.focus.kill()?;
                    }
                },
                NodeCommand::Close => {
                    if self.focus != self.root {
                        self.focus.send_event(Event::ClientMessage {
                            format: 32,
                            window: self.focus.id(),
                            type_: self.atoms.wm_protocols,
                            data: ClientMessageData::Long([
                                self.atoms.wm_delete.id(),
                                0,
                                0,
                                0,
                                0,
                            ]),
                        }, Vec::new(), false)?;
                    }
                },
            },
            Command::Desktop(desktop) => match desktop {
                DesktopCommand::Focus { desktop } => {
                    if self.config.desktops.pinned {
                        self.focused(|screen| {
                            screen.current = desktop.min(screen.desktops.len());

                            Ok(())
                        })?;
                    } else {
                        self.all(|index, screen| {
                            if desktop > screen.desktops.len() * index {
                                screen.current = desktop - screen.desktops.len() * index;
                            }

                            Ok(())
                        })?;
                    }
                },
            },
            Command::Config(config) => match config {
                ConfigCommand::Desktops { names, pinned } => {
                    self.config.desktops = crate::config::Desktops {
                        names,
                        pinned,
                    };

                    // TODO: we need to update desktops on each screen
                },
                ConfigCommand::Window { gaps } => {
                    self.config.window.gaps = gaps;
                },
                ConfigCommand::Border { normal, focused, width } => {
                    self.config.border = crate::config::Border {
                        normal: u32::from_str_radix(&normal, 16)?,
                        focused: u32::from_str_radix(&focused, 16)?,
                        width,
                    };
                },
                ConfigCommand::Padding { top, bottom, left, right } => {
                    self.config.padding = crate::config::Padding {
                        top,
                        bottom,
                        left,
                        right,
                    };
                },
                ConfigCommand::PointerFollowsFocus => self.config.pf.pointer_follows ^= true,
                ConfigCommand::FocusFollowsPointer => self.config.pf.focus_follows ^= true,
            },
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let display = self.display.clone();
        let events = self.events.clone();

        self.load_screens()?;

        server::spawn(events.clone());

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


