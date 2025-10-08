use std::path::PathBuf;

use clap::Parser;
use ointernal_parser::oracle::reader::{ReaderError, RedoFileReader};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ODumper {
    #[arg()]
    redo: PathBuf,
}

fn read_redo(reader: &mut RedoFileReader) -> Result<(), ReaderError> {
    let mut i_record = 1usize;
    while let Some(record) = reader.begin_read_record()? {
        println!("Record#{i_record}: {record:?}");

        let mut i_change = 1usize;
        while let Some(change) = reader.read_change()? {
            println!("  Change#{i_change}: {change:?}");
            i_change += 1;
        }
        i_record += 1;
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = ODumper::parse();

    let mut reader = RedoFileReader::from_file(args.redo)?;
    reader.reload_header()?;

    println!("{:?}", reader.file_header().header);
    println!("{:?}", reader.redo_file_header());
    println!("{:?}", reader.curr_block_head());

    match read_redo(&mut reader) {
        Ok(_) => {}
        Err(ReaderError::Eof) => println!("!!!OVER!!!"),
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
