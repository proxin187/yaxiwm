use serde::{Serialize, Deserialize};
use clap::{Parser, Subcommand};


#[derive(Debug, Clone, PartialEq, Subcommand, Serialize, Deserialize)]
pub enum Direction {
    North,
    South,
    West,
    East,
}

#[derive(Debug, Clone, Subcommand, Serialize, Deserialize)]
pub enum NodeCommand {
    Insert {
        #[command(subcommand)]
        dir: Direction,

        #[arg(short, long)]
        ratio: Option<i8>,

        #[arg(short, long)]
        toggle: bool,
    },
    Desktop {
        #[arg(short, long)]
        desktop: usize,
    },

    Close,
    Kill,
}

#[derive(Debug, Clone, Subcommand, Serialize, Deserialize)]
pub enum DesktopCommand {
    Focus {
        #[arg(short, long)]
        desktop: usize,
    },
}

#[derive(Debug, Clone, Subcommand, Serialize, Deserialize)]
pub enum ConfigCommand {
    Desktops {
        #[arg(short, long)]
        names: Vec<String>,

        #[arg(short, long)]
        pinned: bool,
    },
    Window {
        #[arg(short, long)]
        gaps: u8,
    },
    Border {
        #[arg(short, long)]
        normal: String,

        #[arg(short, long)]
        focused: String,

        #[arg(short, long)]
        width: u16,
    },
    Padding {
        #[arg(short, long)]
        top: u16,

        #[arg(short, long)]
        bottom: u16,

        #[arg(short, long)]
        left: u16,

        #[arg(short, long)]
        right: u16,
    },

    PointerFollowsFocus,
    FocusFollowsPointer,
}

#[derive(Debug, Clone, Subcommand, Serialize, Deserialize)]
pub enum Command {
    #[command(subcommand)]
    Node(NodeCommand),

    #[command(subcommand)]
    Desktop(DesktopCommand),

    #[command(subcommand)]
    Config(ConfigCommand),
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}


