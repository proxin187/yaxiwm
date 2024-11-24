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
pub struct Configuration {
    pub insert: Insert,
}

impl Configuration {
    pub fn new() -> Configuration {
        Configuration {
            insert: Insert::default(),
        }
    }
}


