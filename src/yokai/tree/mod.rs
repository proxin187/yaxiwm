use crate::config::{Insert, Padding};
use crate::wm::Area;

use yaxi::window::{Window, WindowKind};

use ipc::Direction;


pub enum Point {
    Window(Window),
    Any,
}

#[derive(Debug, Clone)]
pub enum Node {
    Leaf {
        window: Window,
    },
    Internal {
        left: Box<Node>,
        right: Box<Node>,
        insert: Insert,
    },
}

impl Node {
    pub fn root(window: Window) -> Node {
        Node::Leaf { window }
    }

    pub fn collect(self) -> Vec<Window> {
        match self {
            Node::Leaf { window } => vec![window],
            Node::Internal { left, right, .. } => {
                [left.collect(), right.collect()].concat()
            },
        }
    }

    pub fn contains(&self, needle: &Window) -> bool {
        match self {
            Node::Leaf { window } => needle == window,
            Node::Internal { left, right, .. } => left.contains(needle) || right.contains(needle),
        }
    }

    pub fn traverse<F>(&self, mut f: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Clone + Copy + FnMut(&Window) -> Result<(), Box<dyn std::error::Error>>
    {
        match self {
            Node::Leaf { window } => f(window),
            Node::Internal { left, right, .. } => {
                left.traverse(f)?;

                right.traverse(f)
            },
        }
    }

    pub fn partition(&self, area: Area, gaps: u8) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Node::Leaf { window } => {
                window.mov_resize(
                    area.x + gaps as u16,
                    area.y + gaps as u16,
                    area.width - (gaps as u16 * 2),
                    area.height - (gaps as u16 * 2),
                )?;

                window.map(WindowKind::Window)?;
            },
            Node::Internal { left, right, insert } => {
                let factor = insert.ratio.min(100) as f64 / 100.0;

                match insert.dir {
                    Direction::North | Direction::South => {
                        left.partition(Area::new(area.x, area.y, area.width, (area.height as f64 * factor) as u16), gaps)?;

                        right.partition(Area::new(area.x, area.y + (area.height as f64 * factor) as u16, area.width, area.height - (area.height as f64 * factor) as u16), gaps)?;
                    },
                    Direction::West | Direction::East => {
                        left.partition(Area::new(area.x, area.y, (area.width as f64 * factor) as u16, area.height), gaps)?;

                        right.partition(Area::new(area.x + (area.width as f64 * factor) as u16, area.y, area.width - (area.width as f64 * factor) as u16, area.height), gaps)?;
                    },
                }
            },
        }

        Ok(())
    }

    pub fn remove(&mut self, needle: u32) -> bool {
        match self {
            Node::Leaf { window } => window.id() == needle,
            Node::Internal { left, right, .. } => {
                if left.remove(needle) {
                    *self = *right.clone();
                } else if right.remove(needle) {
                    *self = *left.clone();
                }

                false
            },
        }
    }

    pub fn map_internal<F>(&mut self, needle: u32, f: F) -> bool
    where
        F: Clone + Copy + Fn(Box<Node>, Box<Node>, Insert) -> Node
    {
        match self {
            Node::Leaf { window } => window.id() == needle,
            Node::Internal { left, right, insert } => {
                if left.map_internal(needle, f) || right.map_internal(needle, f) {
                    *self = f(left.clone(), right.clone(), *insert);
                }

                false
            },
        }
    }

    pub fn reverse(&mut self) {
        match self {
            Node::Leaf { .. } => {},
            Node::Internal { left, right, .. } => {
                std::mem::swap(left, right);

                left.reverse();

                right.reverse();
            },
        }
    }

    fn find(&mut self, point: &Point) -> Option<&mut Node> {
        match self {
            Node::Leaf { window } => match point {
                Point::Window(point) => (window == point).then(|| self),
                Point::Any => Some(self),
            },
            Node::Internal { left, right, .. } => {
                left.find(&point).or(right.find(&point))
            },
        }
    }

    pub fn insert(&mut self, window: Window, insert: Insert, point: Point) {
        if let Some(node) = self.find(&point) {
            *node = match insert.dir {
                Direction::East | Direction::South => Node::Internal {
                    left: Box::new(node.clone()),
                    right: Box::new(Node::Leaf { window }),
                    insert,
                },
                Direction::West | Direction::North => Node::Internal {
                    left: Box::new(Node::Leaf { window }),
                    right: Box::new(node.clone()),
                    insert,
                },
            }
        }
    }
}


