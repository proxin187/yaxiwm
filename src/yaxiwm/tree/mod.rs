use crate::config::{Insert, Padding};
use crate::wm::Area;

use yaxi::window::Window;

use ipc::Direction;


#[derive(Clone)]
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

    pub fn partition(&self, area: Area, padding: Padding) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Node::Leaf { window } => {
                window.mov_resize(
                    area.x + padding.left,
                    area.y - padding.top,
                    area.width - padding.left - padding.right,
                    area.height - padding.top - padding.bottom
                )?;
            },
            Node::Internal { left, right, insert } => {
                let factor = insert.ratio.min(100) as f64 / 100.0;

                match insert.dir {
                    Direction::North | Direction::South => {
                        left.partition(Area::new(area.x, area.y, area.width, (area.height as f64 * factor) as u16), padding)?;

                        right.partition(Area::new(area.x, area.y + (area.height as f64 * factor) as u16, area.width, area.height - (area.height as f64 * factor) as u16), padding)?;
                    },
                    Direction::West | Direction::East => {
                        left.partition(Area::new(area.x, area.y, (area.width as f64 * factor) as u16, area.height), padding)?;

                        right.partition(Area::new(area.x + (area.width as f64 * factor) as u16, area.y, area.width - (area.width as f64 * factor) as u16, area.height), padding)?;
                    },
                }
            },
        }

        Ok(())
    }

    pub fn remove(&mut self, needle: &Window) -> bool {
        match self {
            Node::Leaf { window } => window == needle,
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

    fn find(&mut self, point: &Window) -> Option<&mut Node> {
        match self {
            Node::Leaf { window } => (window == point).then(|| self),
            Node::Internal { left, right, .. } => {
                left.find(&point).or(right.find(&point))
            },
        }
    }

    pub fn insert(&mut self, window: Window, insert: Insert, point: &Window) {
        if let Some(node) = self.find(point) {
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


