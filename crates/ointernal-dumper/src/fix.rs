use std::{fs::File, path::PathBuf};

use anyhow::Context;
use colored::Colorize;
use ointernal_parser::oracle::fixer::{FixOperation, RedoFixer};

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

pub fn fix_redo(file: PathBuf) -> anyhow::Result<()> {
    let fixed_file = file.with_file_name(format!(
        "{}.fixed",
        file.file_name().unwrap().to_str().unwrap()
    ));
    std::fs::copy(&file, &fixed_file).with_context(|| "failed to create fixed file")?;

    let mut f = File::options().write(true).open(&fixed_file)?;
    let mut fixer = RedoFixer::from_file(&file)?;

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
