use ointernal_parser::oracle::change::{
    BlockImage,
    layers::layer18::{ReuseRedoEntry, ReuseType},
};

use crate::dump::Dump;

impl Dump for BlockImage<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::ObjectRangeReuse(Some(r)) => r.dump(),
            _ => Ok(false),
        }
    }
}

impl Dump for ReuseRedoEntry<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        let head = self.head;
        println!("Reuse redo entry");
        match self.reuse_type() {
            ReuseType::Object => {
                println!(
                    "Object reuse: pdb={}, tsn={} objd={}",
                    head.pdb, head.tsn, head.objd,
                );
            }
            ReuseType::Range => {
                println!(
                    "Range reuse: pdb = {} tsn={} base=0x{:08x} nblks={}",
                    head.pdb, head.tsn, head.base, head.nblks,
                );
            }
            ReuseType::PrewarmRange => {
                println!(
                    "Prewarm range reuse: pdb = {} tsn={} base=0x{:08x} nblks={}",
                    head.pdb, head.tsn, head.base, head.nblks,
                );
            }
            ReuseType::LocalRange(_) => {
                println!(
                    "Local range reuse: pdb = {} tsn={} base=0x{:08x} nblks={}",
                    head.pdb, head.tsn, head.base, head.nblks,
                );
            }
        }
        Ok(true)
    }
}
