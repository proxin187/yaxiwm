use crate::config::{Configuration, Insert, Padding};
use crate::event::{Queue, EventType};
use crate::tree::{Node, Point};
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

    pub fn contains(&self, x: u16, y: u16) -> bool {
        (x > self.x && x < self.x + self.width) && (y > self.y && y < self.y + self.height)
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

    pub fn insert(&mut self, window: Window, insert: Insert, point: Point) {
        match &mut self.clients {
            Some(clients) => clients.insert(window, insert, point),
            None => self.clients = Some(Node::root(window)),
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
            let excess = self.desktops.drain(size..self.desktops.len())
                .filter_map(|desktop| desktop.clients)
                .flat_map(|client| client.collect())
                .collect::<Vec<Window>>();

            for window in excess {
                self.desktops[size - 1].insert(window, Insert::default(), Point::Any);
            }
        }
    }

    pub fn insert(&mut self, window: Window, insert: Insert, point: Point) {
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
    focus: Option<Window>,
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
            root,
            focus: None,
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
        let pointer = self.root.query_pointer()?;

        for screen in self.screens.iter_mut() {
            let cond = match &self.focus {
                Some(focus) => screen.contains(focus),
                None => screen.area.contains(pointer.root_x, pointer.root_y),
            };

            if cond {
                f(screen)?;

                return Ok(());
            }
        }

        Ok(())
    }

    fn map_focus<F>(&self, mut f: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&Window) -> Result<(), Box<dyn std::error::Error>>
    {
        match &self.focus {
            Some(focus) if focus != &self.root => f(focus),
            _ => Ok(()),
        }
    }

    fn handle_event(&mut self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        println!("event: {:?}", event);

        match event {
            Event::MapRequest { window, .. } => {
                let focus = self.focus.clone();
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
                    screen.insert(window.clone(), insert.clone(), focus.clone().map(|focus| Point::Window(focus)).unwrap_or(Point::Any));

                    screen.tile(padding)
                })?;
            },
            Event::UnmapNotify { window, .. } => {
                let window = self.display.window_from_id(window)?;
                let padding = self.config.padding.clone();

                self.all(|_, screen| {
                    screen.remove(&window);

                    screen.tile(padding)
                })?;

                if self.focus == Some(window) {
                    self.focus.replace(self.root.clone());
                }
            },
            Event::EnterNotify { window, .. } => {
                if window != self.root.id() && window > 1 && self.config.pf.focus_follows {
                    let window = self.display.window_from_id(window)?;

                    window.set_input_focus(RevertTo::Parent)?;
                }
            },
            Event::FocusIn { window, .. } => {
                // TODO: we should have a function that checks whether a window is managed by us or
                // not in order to minimize edge cases
                if window != self.root.id() {
                    let window = self.display.window_from_id(window)?;

                    window.set_border_pixel(self.config.border.focused)?;

                    if let Some(focus) = &self.focus {
                        focus.set_border_pixel(self.config.border.normal)?;
                    }

                    self.focus.replace(window.clone());
                }
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
                    self.map_focus(|focus| {
                        focus.kill().map_err(|err| err.into())
                    })?;
                },
                NodeCommand::Close => {
                    let wm_protocols = self.atoms.wm_protocols.clone();
                    let wm_delete = self.atoms.wm_delete.clone();

                    self.map_focus(|focus| {
                        focus.send_event(Event::ClientMessage {
                            format: 32,
                            window: focus.id(),
                            type_: wm_protocols,
                            data: ClientMessageData::Long([
                                wm_delete.id(),
                                0,
                                0,
                                0,
                                0,
                            ]),
                        }, Vec::new(), false)?;

                        Ok(())
                    })?;
                },
            },
            Command::Desktop(desktop) => match desktop {
                DesktopCommand::Focus { desktop } => {
                    let padding = self.config.padding.clone();

                    if self.config.desktops.pinned {
                        self.focused(|screen| {
                            screen.current = desktop.min(screen.desktops.len());

                            screen.tile(padding)
                        })?;
                    } else {
                        self.all(|index, screen| {
                            if desktop > screen.desktops.len() * index {
                                screen.current = (desktop - screen.desktops.len() * index).min(screen.desktops.len());

                                screen.tile(padding)?;
                            }

                            Ok(())
                        })?;
                    }
                },
            },
            Command::Config(config) => match config {
                ConfigCommand::Desktops { names, pinned } => {
                    let length = names.len();

                    self.config.desktops = crate::config::Desktops {
                        names,
                        pinned,
                    };

                    self.all(|_, screen| {
                        screen.resize(length);

                        Ok(())
                    })?;
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


