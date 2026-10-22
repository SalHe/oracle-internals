use ointernal_parser::oracle::change::{
    Index,
    layers::layer10::{KdxMetaRef, KdxRedo},
};

use crate::{dump::Dump, utils::Hex};

impl Dump for Index<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::InsertLeafRow(Some(kdx)) => {
                println!("index redo (kdxlin):  insert leaf row");
                kdx.dump()
            }
            Self::DeleteLeafRow(Some(kdx)) => {
                println!("index redo (kdxlde):  delete leaf row");
                kdx.dump()
            }
            _ => Ok(false),
        }
    }
}

impl Dump for KdxRedo<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        // `vector#0`: the same `KtbRedo` block printed by layer 11 changes.
        self.op.dump()?;

        let (itl, flags, sno, row_size) = match &self.meta.meta {
            KdxMetaRef::Single(m) => (m.itl, m.flags, m.sno, m.row_size),
            KdxMetaRef::Array(m) => (m.itl, m.flags, m.sno, m.row_size),
        };

        println!(
            "REDO: 0x{:x} {} / -- / -- ",
            flags.into_bits(),
            if flags.array() { "ARRAY" } else { "SINGLE" },
        );
        println!("itl: {}, sno: {}, row size {}", itl, sno, row_size);

        if let KdxMetaRef::Array(meta) = &self.meta.meta {
            println!("number of keys: {}", meta.num_keys);
            // The vector can carry one slot more than `num_keys` (seen on
            // ARRAY deletes); Oracle prints exactly `num_keys` of them.
            let slots = meta
                .slots
                .get(..meta.num_keys as usize)
                .unwrap_or(&meta.slots);
            print_list("slots", slots);
        }

        if !self.meta.keys.is_empty() {
            let key: Vec<u8> = self.meta.keys.concat();
            // Oracle prints two spaces before a key that fits in a single line,
            // and only one (then breaks the line) for longer ones.
            print!("insert key: ({}): ", key.len());
            if key.len() <= 25 {
                print!(" ");
            }
            println!("{}", Hex::spaced(&key).auto_break_line());
        }

        if !self.meta.key_sizes.is_empty() {
            print_list("each key size is", &self.meta.key_sizes);
        }

        Ok(true)
    }
}

/// Prints `<label>: ` followed by the values on the next line, prefixed and
/// separated by single spaces.
fn print_list<T: std::fmt::Display>(label: &str, items: &[T]) {
    println!("{}: ", label);
    print!(" ");
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", item);
    }
    println!();
}
