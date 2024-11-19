

pub enum Mode<T> {
    Manual {
        point: T,
    },
    Deepest,
    Balance,
}

#[derive(Debug, Clone)]
pub enum Node<T: Clone + PartialEq> {
    Leaf {
        value: T,
    },
    Internal {
        left: Box<Node<T>>,
        right: Box<Node<T>>,
    },
}

impl<T> Node<T> where T: Clone + PartialEq {
    pub fn root(value: T) -> Node<T> {
        Node::Leaf { value }
    }

    fn deepest(&mut self, depth: usize) -> (usize, &mut Node<T>) {
        match self {
            Node::Leaf { .. } => (depth, self),
            Node::Internal { left, right } => {
                let (ld, left) = left.deepest(depth);
                let (rd, right) = right.deepest(depth);

                (ld > rd).then(|| (ld, left)).unwrap_or((rd, right))
            },
        }
    }

    pub fn remove(&mut self, needle: &T) -> bool {
        match self {
            Node::Leaf { value } => value == needle,
            Node::Internal { left, right } => {
                if left.remove(needle) {
                    *self = *right.clone();
                } else if right.remove(needle) {
                    *self = *left.clone();
                }

                false
            },
        }
    }

    pub fn insert(&mut self, value: T) {
        let (_, node) = self.deepest(0);

        *node = Node::Internal {
            left: Box::new(node.clone()),
            right: Box::new(Node::Leaf { value }),
        };
    }
}


