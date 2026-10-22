use ointernal_parser::oracle::change::{BlockCleanOut, layers::layer4::BlockCleanOutRecord};

use crate::dump::Dump;

impl Dump for BlockCleanOut<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::BlockCleanOutRecord(Some(bc)) => bc.dump(),
            _ => Ok(false),
        }
    }
}

impl Dump for BlockCleanOutRecord<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!(
            "Block cleanout record, scn:  0x{:016x} ver: 0x{:02x} opt: 0x{:02x} bigscn: {} compact: {} spare: {:08x}, entries follow...",
            self.scn.full_scn(),
            self.flags.version(),
            self.opt.into_bits(),
            if self.flags.big_scn() { "Y" } else { "N" },
            if self.flags.compact() { "Y" } else { "N" },
            self.spare,
        );
        for e in self.entries {
            println!(
                "  itli: {}  flg: (opt={} whr={})  scn:  0x{:016x}",
                e.itli,
                e.flag.opt(),
                e.flag.whr(),
                e.scn().full_scn(),
            );
        }
        Ok(true)
    }
}
