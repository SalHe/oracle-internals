mod dump;
mod filter;
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

        /// Filter expression for records and changes.
        ///
        /// Available variables (all integers):
        ///   - record: Record number (1-based)
        ///   - change: Change number within record (1-based)
        ///   - layer: Layer of change (5, 11, 13, etc)
        ///   - op: Operation type within layer
        ///   - scn: System Change Number (as integer)
        ///
        /// Expressions use standard operators: `==`, `!=`, `<`, `>`, `<=`, `>=`,
        /// logical `&&`, `||`, `!`, and arithmetic as needed.
        ///
        /// Examples:
        ///   "record == 1 || record == 2 || record == 3"
        ///   "record >= 1 && record <= 10 && layer == 11"
        ///   "op >= 1 && op <= 5 && layer == 11"
        #[arg(long)]
        filter: Option<String>,
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
        Commands::Dump { redo, force_vector, filter } => read_redo(redo, force_vector, filter),
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
