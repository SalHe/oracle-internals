mod dump;
mod fix;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
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
        #[arg(short, long, default_value = "false")]
        force_vector: bool,
    },

    /// fix checksum of redo file(to modify redo file and dump it using Oracle Database)
    Fix { redo: PathBuf },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let args = Cli::parse();

    match args.command {
        Commands::Dump { redo, force_vector } => read_redo(redo, force_vector),
        Commands::Fix { redo } => {
            fix_redo(redo)?;
            Ok(())
        }
    }
}
