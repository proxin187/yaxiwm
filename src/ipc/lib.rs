use serde::{Serialize, Deserialize};
use clap::{Parser, Subcommand};


#[derive(Debug, Clone, PartialEq, Subcommand, Serialize, Deserialize)]
pub enum Direction {
    North,
    South,
    West,
    East,
}

#[derive(Clone, Subcommand, Serialize, Deserialize)]
pub enum NodeCommand {
    Insert {
        #[command(subcommand)]
        dir: Direction,

        #[arg(short, long)]
        ratio: Option<i8>,

        #[arg(short, long)]
        toggle: bool,
    },

    Close,
    Kill,
}

#[derive(Clone, Subcommand, Serialize, Deserialize)]
pub enum ConfigCommand {
    PointerFollowsFocus,
    FocusFollowsPointer,
}

#[derive(Clone, Subcommand, Serialize, Deserialize)]
pub enum Command {
    #[command(subcommand)]
    Node(NodeCommand),

    #[command(subcommand)]
    Config(ConfigCommand),
}

#[derive(Parser, Serialize, Deserialize)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}


