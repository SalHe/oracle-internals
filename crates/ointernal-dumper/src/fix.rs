use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use anyhow::{Context, anyhow};
use colored::Colorize;
use num_traits::Num;
use ointernal_parser::oracle::fixer::{FixOperation, RedoFixer};

use crate::utils::Hex;

fn fix(f: &mut File, block_id: u32, block_size: u32, op: FixOperation) -> anyhow::Result<()> {
    print!("   ");
    match op {
        FixOperation::FixCheckSum { actual, expected } => {
            print!(
                "check sum expected=0x{:04x}, actual=0x{:04x}",
                expected, actual
            );
        }
    }

    op.fix_it(f, block_id, block_size)?;
    println!(" {}", "(FIXED)".bold());

    Ok(())
}

fn parse_int<T: Num>(v: &str) -> Result<T, <T as Num>::FromStrRadixErr> {
    if let Some(v) = v.strip_prefix("0x") {
        T::from_str_radix(v, 16)
    } else {
        T::from_str_radix(v, 10)
    }
}

fn parse_replace(r: &str) -> anyhow::Result<(u64, Vec<u8>)> {
    let (addr, bin) = r
        .split_once(',')
        .ok_or(anyhow!("incorrect replace string format."))?;

    let offset = parse_int(addr).with_context(|| anyhow!("Failed to parse offset {addr:}"))?;
    let bin = if let Some(uint16) = bin.strip_prefix("u16:") {
        parse_int::<u16>(uint16)?.to_ne_bytes().to_vec()
    } else if let Some(uint32) = bin.strip_prefix("u32:") {
        parse_int::<u32>(uint32)?.to_ne_bytes().to_vec()
    } else if let Some(uint64) = bin.strip_prefix("u64:") {
        parse_int::<u64>(uint64)?.to_ne_bytes().to_vec()
    } else {
        hex::decode(bin)?
    };

    Ok((offset, bin))
}

pub fn fix_redo(file: PathBuf, replaces: Vec<String>) -> anyhow::Result<()> {
    let fixed_file = file.with_file_name(format!(
        "{}.fixed",
        file.file_name().unwrap().to_str().unwrap()
    ));
    std::fs::copy(&file, &fixed_file).with_context(|| "failed to create fixed file")?;
    let mut f = File::options().read(true).write(true).open(&fixed_file)?;

    // replace
    for r in &replaces {
        let (addr, bin) = parse_replace(r)?;
        let mut old = vec![0u8; bin.len()];

        f.seek(SeekFrom::Start(addr))?;
        f.read_exact(&mut old)?;
        f.seek(SeekFrom::Start(addr))?;
        f.write_all(&bin)?;
        println!("{} bytes at 0x{addr:04x} (hex)", "replaced".green(),);
        println!("         old {}", Hex::spaced(&old));
        println!("         new {}", Hex::spaced(&bin));
    }
    f.flush()?;
    if !replaces.is_empty() {
        println!();
    }

    // do fix
    let mut fixer = RedoFixer::from_file(&fixed_file)?;
    while let Some(s) = fixer.next_block()? {
        print!("{}  ", format!("{}{}", "block#", s.block_id).bold());
        if s.passed() {
            println!("check passed");
        } else {
            println!();
            assert!(!s.operations.is_empty());
            for op in s.operations {
                fix(&mut f, s.block_id, fixer.block_size().unwrap(), op)?;
            }
        }
    }

    Ok(())
}
