use crate::config::{Configuration, Insert, Padding};
use crate::event::{Queue, EventType};
use crate::tree::{Node, Point};
use crate::startup;
use crate::server;

use yaxi::display::{self, Display, Atom};
use yaxi::window::{Window, WindowKind, WindowArguments, ValuesBuilder};
use yaxi::proto::{Event, EventMask, RevertTo, ClientMessageData, WindowClass};
use yaxi::ewmh::{EwmhWindowType, DesktopViewport};

use std::sync::Arc;
use std::thread;

use ipc::{Arguments, Command, NodeCommand, DesktopCommand, ConfigCommand, Change};

const DOCK: [EwmhWindowType; 3] = [EwmhWindowType::Dock, EwmhWindowType::Toolbar, EwmhWindowType::Menu];
const FLOAT: [EwmhWindowType; 3] = [EwmhWindowType::Splash, EwmhWindowType::Utility, EwmhWindowType::Dialog];

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Float,
    Dock,
}

impl Mode {
    // sometimes you just feel like using functions for everything, who needs if statements anyway
    fn from(types: &[EwmhWindowType]) -> Option<Mode> {
        DOCK.iter()
            .any(|type_| types.contains(type_))
            .then(|| Mode::Dock)
            .or_else(|| {
                FLOAT.iter()
                    .any(|type_| types.contains(type_))
                    .then(|| Mode::Float)
            })
    }
}

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

#[derive(Debug, Clone)]
pub struct AltClient {
    window: Window,
    mode: Mode,
}

impl AltClient {
    pub fn new(window: Window, mode: Mode) -> AltClient {
        AltClient {
            window,
            mode,
        }
    }
}

pub struct Desktop {
    clients: Option<Node>,
    alt: Vec<AltClient>,
    area: Area,
}

impl Desktop {
    pub fn new(area: Area) -> Desktop {
        Desktop {
            clients: None,
            alt: Vec::new(),
            area,
        }
    }

    pub fn contains(&self, window: &Window) -> bool {
        match &self.clients {
            Some(clients) => clients.contains(window),
            None => false,
        }
    }

    pub fn insert(&mut self, window: Window, insert: Insert, point: Point, mode: Option<Mode>) {
        if let Some(mode) = mode {
            self.alt.push(AltClient::new(window, mode));
        } else {
            match &mut self.clients {
                Some(clients) => clients.insert(window, insert, point),
                None => self.clients = Some(Node::root(window)),
            }
        }
    }

    pub fn remove(&mut self, wid: impl Into<u32>) -> Option<AltClient> {
        let wid = wid.into();

        if self.clients.as_mut().map(|clients| clients.remove(wid)).unwrap_or(false) {
            self.clients = None;
        }

        self.alt.iter()
            .position(|alt| alt.window.id() == wid)
            .and_then(|index| (index < self.alt.len()).then(|| self.alt.remove(index)))
    }

    pub fn map_internal<F>(&mut self, wid: impl Into<u32>, f: F)
    where
        F: Clone + Copy + Fn(Box<Node>, Box<Node>, Insert) -> Node
    {
        if let Some(clients) = &mut self.clients {
            clients.map_internal(wid.into(), f);
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

    pub fn tile(&self, padding: Padding, gaps: u8) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(clients) = &self.clients {
            let area = Area::new(
                self.area.x + padding.left,
                self.area.y + padding.top,
                self.area.width - padding.left - padding.right,
                self.area.height - padding.top - padding.bottom,
            );

            clients.partition(area, gaps)?;
        }

        // this should map all alt window too, havent tested it yet
        for alt in self.alt.iter() {
            alt.window.map(WindowKind::Window)?;
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
            // TODO: we also need to collect alt

            let excess = self.desktops.drain(size..self.desktops.len())
                .filter_map(|desktop| desktop.clients)
                .flat_map(|client| client.collect())
                .collect::<Vec<Window>>();

            for window in excess {
                self.desktops[size - 1].insert(window, Insert::default(), Point::Any, None);
            }
        }
    }

    pub fn insert(&mut self, window: Window, insert: Insert, point: Point, mode: Option<Mode>) {
        if let Some(desktop) = self.desktops.get_mut(self.current) {
            desktop.insert(window, insert, point, mode);
        }
    }

    pub fn remove(&mut self, wid: impl Into<u32>) -> Option<AltClient> {
        self.desktops.get_mut(self.current)
            .and_then(|desktop| desktop.remove(wid))
    }

    pub fn map_internal<F>(&mut self, wid: impl Into<u32>, f: F)
    where
        F: Clone + Copy + Fn(Box<Node>, Box<Node>, Insert) -> Node
    {
        if let Some(desktop) = self.desktops.get_mut(self.current) {
            desktop.map_internal(wid, f);
        }
    }

    pub fn tile(&self, padding: Padding, gaps: u8) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(desktop) = self.desktops.get(self.current) {
            desktop.tile(padding, gaps)?;
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
    should_close: bool,
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
            should_close: false,
        })
    }

    fn load_screens(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let xinerama = self.display.query_xinerama()?;

        for screen in xinerama.query_screens()? {
            self.screens.push(Screen::new(Area::new(screen.x, screen.y, screen.width, screen.height)));
        }

        self.update_viewport()?;

        Ok(())
    }

    fn update_viewport(&self) -> Result<(), Box<dyn std::error::Error>> {
        let viewport = self.screens.iter()
            .map(|screen| DesktopViewport::new(screen.area.x as u32, screen.area.y as u32))
            .collect::<Vec<DesktopViewport>>();

        self.display
            .use_ewmh(&self.root)
            .set_desktop_viewport(&viewport)?;

        Ok(())
    }

    fn set_supporting_ewmh(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let window = self.root.create_window(WindowArguments {
            depth: self.root.depth(),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            class: WindowClass::InputOutput,
            border_width: 0,
            visual: self.root.visual(),
            values: ValuesBuilder::new(vec![]),
        })?;

        let ewmh = self.display.use_ewmh(&window);

        ewmh.set_supporting_wm_check(window.id())?;

        ewmh.set_wm_name("yokai")?;

        let root = self.display.use_ewmh(&self.root);

        root.set_supporting_wm_check(window.id())?;

        // TODO: support for _NET_WM_STATE and _NET_WM_STATE_FULLSCREEN

        root.set_supported(&[
            self.display.intern_atom("WM_PROTOCOLS", false)?,
            self.display.intern_atom("WM_DELETE_WINDOW", false)?,
            self.display.intern_atom("_NET_ACTIVE_WINDOW", false)?,
            self.display.intern_atom("_NET_NUMBER_OF_DESKTOPS", false)?,
            self.display.intern_atom("_NET_CURRENT_DESKTOP", false)?,
            self.display.intern_atom("_NET_WM_WINDOW_TYPE", false)?,
            self.display.intern_atom("_NET_WM_WINDOW_TYPE_DESKTOP", false)?,
            self.display.intern_atom("_NET_WM_WINDOW_TYPE_DOCK", false)?,
            self.display.intern_atom("_NET_WM_WINDOW_TYPE_TOOLBAR", false)?,
            self.display.intern_atom("_NET_WM_WINDOW_TYPE_MENU", false)?,
            self.display.intern_atom("_NET_WM_WINDOW_TYPE_UTILITY", false)?,
            self.display.intern_atom("_NET_WM_WINDOW_TYPE_SPLASH", false)?,
            self.display.intern_atom("_NET_WM_WINDOW_TYPE_DIALOG", false)?,
            self.display.intern_atom("_NET_WM_WINDOW_TYPE_NORMAL", false)?,
        ])?;

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

    fn focused<F, R>(&mut self, mut f: F) -> Result<R, Box<dyn std::error::Error>>
    where
        F: FnMut(usize, &mut Screen) -> Result<R, Box<dyn std::error::Error>>,
        R: Default,
    {
        let pointer = self.root.query_pointer()?;

        for (index, screen) in self.screens.iter_mut().enumerate() {
            if self.focus.as_ref().map(|focus| screen.contains(focus)).unwrap_or(screen.area.contains(pointer.root_x, pointer.root_y)) {
                return f(index, screen);
            }
        }

        Ok(R::default())
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

    fn is_managed(&self, window: &Window) -> bool {
        self.screens.iter().any(|screen| screen.contains(window))
    }

    fn unmanage(&mut self, window: u32) -> Result<Option<AltClient>, Box<dyn std::error::Error>> {
        let padding = self.config.padding.clone();
        let gaps = self.config.gaps.clone();

        self.all(|_, screen| {
            screen.remove(window);

            screen.tile(padding, gaps)
        })?;

        if self.focus.as_ref().map(|window| window.id()) == Some(window) {
            self.focus = None;
        }

        self.focused(|_, screen| {
            Ok(screen.remove(window))
        })
    }

    fn handle_event(&mut self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        println!("event: {:?}", event);

        match event {
            Event::MapRequest { window, .. } => {
                let focus = self.focus.clone();
                let window = self.display.window_from_id(window)?;
                let insert = self.config.insert.clone();
                let padding = self.config.padding.clone();
                let gaps = self.config.gaps.clone();

                window.select_input(&[
                    EventMask::SubstructureNotify,
                    EventMask::SubstructureRedirect,
                    EventMask::EnterWindow,
                    EventMask::FocusChange,
                ])?;

                window.set_border_pixel(self.config.border.normal)?;

                window.set_border_width(self.config.border.width)?;

                let types = self.display
                    .use_ewmh(&window)
                    .get_wm_window_type()?;

                self.focused(|_, screen| {
                    screen.insert(
                        window.clone(),
                        insert.clone(),
                        focus.clone()
                            .map(|focus| Point::Window(focus))
                            .unwrap_or(Point::Any),
                        Mode::from(&types),
                    );

                    screen.tile(padding, gaps)
                })?;
            },
            Event::UnmapNotify { window, .. } => {
                self.unmanage(window)?;
            },
            Event::EnterNotify { window, .. } => {
                let window = self.display.window_from_id(window)?;

                if self.is_managed(&window) && self.config.pf.focus_follows {
                    window.set_input_focus(RevertTo::Parent)?;

                }
            },
            Event::FocusIn { window, .. } => {
                let window = self.display.window_from_id(window)?;

                if self.is_managed(&window) {
                    window.set_border_pixel(self.config.border.focused)?;

                    window.raise()?;

                    if let Some(focus) = self.focus.replace(window.clone()) {
                        if focus.id() != window.id() {
                            focus.set_border_pixel(self.config.border.normal)?;
                        }
                    }
                }
            },
            _ => {},
        }

        Ok(())
    }

    fn handle_config(&mut self, args: Arguments) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: we need to implement node selection

        // TODO: floating and dock windows

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
                    if let Some(focus) = self.focus.clone() {
                        let insert = self.config.insert.clone();

                        if self.focused(|_, screen| Ok(desktop < screen.desktops.len() && screen.current != desktop))? {
                            let mode = self.unmanage(focus.id())?.map(|alt| alt.mode);

                            self.focused(move |_, screen| {
                                screen.desktops[desktop].insert(focus.clone(), insert, Point::Any, mode);

                                Ok(())
                            })?;
                        }
                    }
                },
                NodeCommand::Ratio { change } => {
                    if let Some(focus) = self.focus.clone() {
                        let padding = self.config.padding.clone();
                        let gaps = self.config.gaps.clone();

                        self.focused(move |_, screen| {
                            screen.map_internal(focus.id(), |left, right, insert| {
                                Node::Internal {
                                    left,
                                    right,
                                    insert: Insert {
                                        dir: insert.dir,
                                        ratio: match change {
                                            Change::Add { value } => insert.ratio + value,
                                            Change::Sub { value } => insert.ratio - value.min(insert.ratio),
                                            Change::Set { value } => value,
                                        }.min(90).max(10),
                                    },
                                }
                            });

                            screen.tile(padding, gaps)
                        })?;
                    }
                },
                NodeCommand::Reverse =>  {
                    if let Some(focus) = self.focus.clone() {
                        let padding = self.config.padding.clone();
                        let gaps = self.config.gaps.clone();

                        self.focused(move |_, screen| {
                            screen.map_internal(focus.id(), |mut left, mut right, insert| {
                                right.reverse();

                                left.reverse();

                                Node::Internal {
                                    left: right,
                                    right: left,
                                    insert,
                                }
                            });

                            screen.tile(padding, gaps)
                        })?;
                    }
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
                    let gaps = self.config.gaps.clone();
                    let ewmh = self.display.use_ewmh(&self.root);

                    if self.config.desktops.pinned {
                        self.focused(|index, screen| {
                            screen.current = desktop.min(screen.desktops.len());

                            ewmh.set_current_desktop((screen.current + screen.desktops.len() * index) as u32)?;

                            screen.tile(padding, gaps)
                        })?;
                    } else {
                        self.all(|index, screen| {
                            if desktop > screen.desktops.len() * index {
                                screen.current = (desktop - screen.desktops.len() * index).min(screen.desktops.len());

                                ewmh.set_current_desktop((screen.current + screen.desktops.len() * index) as u32)?;

                                screen.tile(padding, gaps)?;
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

                    self.display
                        .use_ewmh(&self.root)
                        .set_number_of_desktops((length * self.screens.len()) as u32)?;

                    self.update_viewport()?;
                },
                ConfigCommand::Window { gaps } => {
                    let padding = self.config.padding.clone();

                    self.config.gaps = gaps;

                    self.all(|_, screen| screen.tile(padding, gaps))?;
                },
                ConfigCommand::Border { normal, focused, width } => {
                    let padding = self.config.padding.clone();
                    let gaps = self.config.gaps.clone();

                    self.config.border = crate::config::Border {
                        normal: u32::from_str_radix(&normal, 16)?,
                        focused: u32::from_str_radix(&focused, 16)?,
                        width,
                    };

                    self.all(|_, screen| screen.tile(padding, gaps))?;
                },
                ConfigCommand::Padding { top, bottom, left, right } => {
                    self.config.padding = crate::config::Padding {
                        top,
                        bottom,
                        left,
                        right,
                    };

                    let padding = self.config.padding.clone();
                    let gaps = self.config.gaps.clone();

                    self.all(|_, screen| screen.tile(padding, gaps))?;
                },
                ConfigCommand::PointerFollowsFocus => self.config.pf.pointer_follows ^= true,
                ConfigCommand::FocusFollowsPointer => self.config.pf.focus_follows ^= true,
            },
            Command::Exit => {
                self.should_close = true;
            },
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let display = self.display.clone();
        let events = self.events.clone();

        self.load_screens()?;

        self.set_supporting_ewmh()?;

        server::spawn(events.clone());

        thread::spawn(move || {
            listen(display, events).expect("failed to listen");
        });

        startup::startup()?;

        while !self.should_close {
            match self.events.wait()? {
                EventType::XEvent(event) => {
                    self.handle_event(event)?;
                },
                EventType::Config(args) => {
                    self.handle_config(args)?;
                },
            }
        }

        Ok(())
    }
}

fn listen(display: Display, events: Arc<Queue<EventType>>) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let event = display.next_event()?;

        events.push(EventType::XEvent(event))?;
    }
}


