mod server;
mod event;
mod tree;
mod wm;

use tree::{Node, Mode, Split};


fn main() {
    let mut root: Node<usize> = Node::root(0);

    println!("root: {:#?}", root);

    root.insert(420, Split::Horizontal, Mode::Deepest);

    println!("root: {:#?}", root);

    root.insert(69, Split::Horizontal, Mode::Deepest);

    println!("root: {:#?}", root);

    root.insert(1337, Split::Horizontal, Mode::Manual {
        point: 420,
    });

    println!("root: {:#?}", root);

    root.remove(&420);

    println!("root: {:#?}", root);
}


