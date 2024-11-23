

pub enum Mode<T> {
    Manual {
        point: T,
    },
    Deepest,
    Balance,
}

#[derive(Debug, Clone)]
pub enum Split {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub enum Node<T: Clone + PartialEq> {
    Leaf {
        value: T,
    },
    Internal {
        left: Box<Node<T>>,
        right: Box<Node<T>>,
        split: Split,
    },
}

impl<T> Node<T> where T: Clone + PartialEq {
    pub fn root(value: T) -> Node<T> {
        Node::Leaf { value }
    }

    fn deepest(&mut self, depth: usize) -> (usize, &mut Node<T>) {
        match self {
            Node::Leaf { .. } => (depth, self),
            Node::Internal { left, right, .. } => {
                let (ld, left) = left.deepest(depth);
                let (rd, right) = right.deepest(depth);

                (ld > rd).then(|| (ld, left)).unwrap_or((rd, right))
            },
        }
    }

    pub fn remove(&mut self, needle: &T) -> bool {
        match self {
            Node::Leaf { value } => value == needle,
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

    fn manual(&mut self, point: &T) -> Option<&mut Node<T>> {
        match self {
            Node::Leaf { value } => (value == point).then(|| self),
            Node::Internal { left, right, .. } => {
                left.manual(&point).or(right.manual(&point))
            },
        }
    }

    pub fn get_node(&mut self, mode: Mode<T>) -> Option<&mut Node<T>> {
        match mode {
            Mode::Manual { point } => {
                self.manual(&point)
            },
            Mode::Deepest => {
                let (_, node) = self.deepest(0);

                Some(node)
            },
            Mode::Balance => None,
        }
    }

    pub fn insert(&mut self, value: T, split: Split, mode: Mode<T>) {
        if let Some(node) = self.get_node(mode) {
            *node = Node::Internal {
                left: Box::new(node.clone()),
                right: Box::new(Node::Leaf { value }),
                split,
            };
        }
    }
}


