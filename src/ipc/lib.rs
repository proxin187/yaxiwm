use serde::{Serialize, Deserialize};
use clap::Parser;


#[derive(Serialize, Deserialize)]
pub enum Node {
    Close,
    Kill,
}

#[derive(Serialize, Deserialize)]
pub enum Config {
    PointerFollowsFocus,
    FocusFollowsPointer,
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    Node(Node),
    Config(Config),
}


