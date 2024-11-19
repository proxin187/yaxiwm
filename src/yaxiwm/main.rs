mod tree;

use tree::Node;


fn main() {
    let mut root: Node<usize> = Node::root(0);

    println!("root: {:#?}", root);

    root.insert(420);

    println!("root: {:#?}", root);

    root.insert(69);

    println!("root: {:#?}", root);

    root.insert(1337);

    println!("root: {:#?}", root);

    root.remove(&420);

    println!("root: {:#?}", root);
}


