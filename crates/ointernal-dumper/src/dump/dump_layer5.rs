use ointernal_parser::oracle::change::{
    TransactionUndoManagement,
    layers::layer5::{ChangeTxSlotStatus, UndoHeaderRedo},
};

use crate::dump::DumpRedo;

impl DumpRedo for TransactionUndoManagement<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::UndoHeaderRedo(Some(b)) => b.dump(),
            Self::ChangeTxSlotStatus(Some(b)) => b.dump(),
            _ => Ok(false),
        }
    }
}

impl DumpRedo for UndoHeaderRedo<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!(
            "ktudh redo: slt: 0x{:04x} sqn: 0x{:08x} flg: 0x{:04x} siz: {} fbi: {}",
            self.data.slt,
            self.data.sqn,
            self.data.flag.into_bits(),
            self.data.size,
            self.data.fbi,
        );
        print!(
            "            uba: 0x{:08x}.{:04x}.{:02x}    pxid:  0x{:04x}.{:03x}.{:08x}",
            self.data.uba.dba.0,
            self.data.uba.seq,
            self.data.uba.record,
            self.data.parent_xid.usn,
            self.data.parent_xid.slot,
            self.data.parent_xid.sequence,
        );
        match self.pdbuid {
            Some(pdbuid) => println!("        pdbuid:{pdbuid}"),
            None => println!(),
        }
        Ok(true)
    }
}

impl DumpRedo for ChangeTxSlotStatus<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        print!(
            "ktucm redo: slt: 0x{:04x} sqn: 0x{:08x} srt: {} sta: {} flg: 0x{:x}",
            self.data.slot, self.data.seq, self.data.srt, self.data.sta, self.data.flags,
        );

        if let Some(d) = self.data2 {
            print!(
                " ktucf redo: uba: 0x{:08x}.{:04x}.{:02x} ext: {} spc: {} fbi: {} ",
                d.uba.dba.0, d.uba.seq, d.uba.seq, d.ext, d.spc, d.fbi
            );
        }

        if let Some(d) = self.scn {
            print!("\nscn:  0x{:016x}", d.full_scn());
        }

        println!();
        Ok(true)
    }
}
