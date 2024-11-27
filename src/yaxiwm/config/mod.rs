use ipc::Direction;


#[derive(Debug, Clone, PartialEq)]
pub struct Insert {
    pub dir: Direction,
    pub ratio: i8,
}

impl Default for Insert {
    fn default() -> Insert {
        Insert {
            dir: Direction::East,
            ratio: 50,
        }
    }
}

impl Insert {
    pub fn new(dir: Direction, ratio: i8) -> Insert {
        Insert {
            dir,
            ratio,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PointerFocus {
    pub focus_follows: bool,
    pub pointer_follows: bool,
}

#[derive(Debug, Clone)]
pub struct Desktops {
    pub names: Vec<String>,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub gaps: u8,
}

#[derive(Debug, Clone)]
pub struct Border {
    pub normal: u32,
    pub focused: u32,
    pub width: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct Padding {
    pub top: u16,
    pub bottom: u16,
    pub left: u16,
    pub right: u16,
}

#[derive(Debug, Clone)]
pub struct Configuration {
    pub insert: Insert,
    pub pf: PointerFocus,
    pub desktops: Desktops,
    pub window: Window,
    pub border: Border,
    pub padding: Padding,
}

impl Configuration {
    pub fn new() -> Configuration {
        Configuration {
            insert: Insert::default(),
            pf: PointerFocus {
                focus_follows: false,
                pointer_follows: false,
            },
            desktops: Desktops {
                names: Vec::new(),
                pinned: false,
            },
            window: Window {
                gaps: 0,
            },
            border: Border {
                normal: 0x000000ff,
                focused: 0xffffffff,
                width: 1,
            },
            padding: Padding {
                top: 0,
                bottom: 0,
                left: 0,
                right: 0,
            },
        }
    }
}


