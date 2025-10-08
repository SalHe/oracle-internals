use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
};

use anyhow::{Context, anyhow};
use ointernal_parser::oracle::{reader::RedoFileReader, redo::RedoBlockHead};

pub fn fix_redo(file: PathBuf) -> anyhow::Result<()> {
    let fixed_file = file.with_file_name(format!(
        "{}.fixed",
        file.file_name().unwrap().to_str().unwrap()
    ));
    std::fs::copy(&file, &fixed_file).with_context(|| "failed to create fixed file")?;

    let mut f = File::options().write(true).open(&fixed_file)?;
    let mut reader = RedoFileReader::from_file(&file)?;
    reader.set_options().ignore_checksum = true;
    unsafe { reader.read_file_header()? };

    let header = reader.file_header();
    let block_size = header.header.block_size;

    for i_block in 1..=header.header.block_count {
        unsafe { reader.next_redo_block().map_err(|e| anyhow!("{:?}", e))? };
        let (actual, expected) = reader
            .calc_curr_block_checksum()
            .map_err(|e| anyhow!("{:?}", e))?;
        print!("block#{i_block}: actual=0x{actual:04x}, expected=0x{expected:04x}");

        if actual != expected {
            print!(" (NOT EQUAL) ");
            f.seek(SeekFrom::Start(
                ((i_block * block_size) as usize + std::mem::offset_of!(RedoBlockHead, checksum))
                    as u64,
            ))?;
            f.write_all(&actual.to_ne_bytes())?;

            print!(" (FIXED) ")
        } else {
            print!(" (EQUAL) ")
        }
        println!()
    }

    Ok(())
}
