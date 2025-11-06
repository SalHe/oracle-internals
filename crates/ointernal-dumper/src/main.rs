mod dump;
mod fix;
mod utils;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ointernal_parser::oracle::types::get_number_from_bytes;
use tracing_subscriber::EnvFilter;

use crate::{dump::read_redo, fix::fix_redo};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// dump redo file
    Dump {
        redo: PathBuf,

        /// Dump raw vector content.
        #[arg(short, long, default_value = "false")]
        force_vector: bool,
    },

    /// fix checksum of redo file(to modify redo file and dump it using Oracle Database)
    Fix {
        redo: PathBuf,

        /// Replacing byte sequence at specified offset of file.
        ///
        /// eg. `0x400,AABBCCDD` means in-replace bytes sequence at 0x400 from file head.
        ///
        /// Format: offset,[type:]content
        ///     offset: offset from head, in decimal(1024), or hex(0x400)
        ///     type: When specified one of [u16,u32,u64], content is an integer.
        ///           When ignored, content is a hex string representing byte sequence.
        #[arg(short, long)]
        replaces: Vec<String>,
    },

    /// Try represent bytes(hex) in Oracle Database's types.
    Represent { hex: String },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let args = Cli::parse();

    match args.command {
        Commands::Dump { redo, force_vector } => read_redo(redo, force_vector),
        Commands::Fix { redo, replaces } => {
            fix_redo(redo, replaces)?;
            Ok(())
        }
        Commands::Represent { hex } => {
            let hex = hex.replace(' ', "");
            let bytes = hex::decode(&hex)?;
            println!("Number: {:?}", get_number_from_bytes(&bytes));
            Ok(())
        }
    }
}
