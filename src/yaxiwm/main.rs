mod tree;
mod wm;

use tree::{Node, Mode};


fn main() {
    let mut root: Node<usize> = Node::root(0);

    println!("root: {:#?}", root);

    root.insert(420, Mode::Deepest);

    println!("root: {:#?}", root);

    root.insert(69, Mode::Deepest);

    println!("root: {:#?}", root);

    root.insert(1337, Mode::Manual {
        point: 420,
    });

    println!("root: {:#?}", root);

    root.remove(&420);

    println!("root: {:#?}", root);
}


