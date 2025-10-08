mod dump;
mod fix;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ointernal_parser::oracle::reader::ReaderError;

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
    Dump { redo: PathBuf },

    /// fix checksum of redo file(to modify redo file and dump it using Oracle Database)
    Fix { redo: PathBuf },
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Dump { redo } => match read_redo(redo) {
            Ok(_) => Ok(()),
            Err(ReaderError::Eof) => {
                println!("END OF REDO DUMP");
                Ok(())
            }
            Err(e) => Err(e.into()),
        },
        Commands::Fix { redo } => {
            fix_redo(redo)?;
            Ok(())
        }
    }
}
