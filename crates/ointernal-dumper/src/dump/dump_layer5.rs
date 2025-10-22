use ointernal_parser::oracle::change::{
    TransactionUndoManagement,
    layers::layer5::{
        self, ChangeTxSlotStatus, UndoBlockRedo, UndoBlockRedoData2CommonRest, UndoHeaderRedo,
    },
};

use crate::dump::{DumpRedo, print_record};

impl DumpRedo for TransactionUndoManagement<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::UndoBlockRedo(Some(b)) => b.dump(),
            Self::UndoHeaderRedo(Some(b)) => b.dump(),
            Self::ChangeTxSlotStatus(Some(b)) => b.dump(),
            _ => Ok(false),
        }
    }
}

fn yes_or_no(y: bool) -> &'static str {
    if y { "Yes" } else { "No" }
}

impl DumpRedo for UndoBlockRedo<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!(
            "ktudb redo: siz: {} spc: {} flg: 0x{:04x} seq: 0x{:04x} rec: 0x{:02x}",
            self.data.size, self.data.space, self.data.flags, self.data.seq, self.data.rec,
        );
        println!(
            "            xid:  0x{:04x}.{:03x}.{:08x}  ",
            self.data.xid.usn, self.data.xid.slot, self.data.xid.sequence,
        );
        println!(
            "ktub{} redo: slt: {} wrp: {} flg: 0x{:04x} prev dba:  0x{:08x} rci: {} opc: {}.{} [objn: {} objd: {} tsn: {}]",
            if self.data2.flags.is_ktubl() {
                "l"
            } else {
                "u"
            },
            self.data2.slot,
            self.data2.wrap,
            self.data2.flags.into_bits(),
            self.data2.dba.0,
            self.data2.rci,
            self.data2.record_layer_code,
            self.data2.record_op_code,
            self.data2.objn,
            self.data2.objd,
            self.data2.tsn,
        );
        println!(
            "[Undo type  ] {:<14}[User undo done   ]  {:<3} [Last buffer split]  {:<3}",
            match self.data2.flags.undo_type() {
                0 => "Regular undo",
                1 => "MBU - HEAD",
                2 => "MBU - TAIL",
                3 => "MBU - HEAD",
                _ => unreachable!(),
            },
            yes_or_no(self.data2.flags.user_undo_done()),
            yes_or_no(self.data2.flags.last_buffer_split()),
        );
        println!(
            "[Temp object]           {:<3} [Tablespace Undo  ]  {:<3} [User only        ]  {:<3}",
            yes_or_no(self.data2.flags.temp_object()),
            yes_or_no(self.data2.flags.tablespace_undo()),
            yes_or_no(self.data2.flags.user_only()),
        );

        let print_common = |c: &UndoBlockRedoData2CommonRest| {
            println!("Begin trans    ");
            println!(
                " prev ctl uba: 0x{:08x}.{:04x}.{:02x} prev ctl max cmt scn:  0x{:016x} ",
                c.prev_ctl_uba.dba.0,
                c.prev_ctl_uba.seq,
                c.prev_ctl_uba.record,
                c.prev_ctl_max_cmt_scn.full_scn(),
            );
            println!(
                " prev tx cmt scn:  0x{:016x} ",
                c.prev_tx_cmt_scn.full_scn()
            );
            println!(
                " txn start scn:  0x{:016x}  logon user: {}",
                c.txn_start_scn.full_scn(),
                c.logon_user
            );
            println!(
                " prev brb:  0x{:08x}  prev bcl:  0x{:08x}",
                c.prev_brb, c.prev_bcl
            );
        };

        if self.data2.flags.is_ktubl() {
            match self.data2.rest()? {
                layer5::UndoBlockRedoData2Rest::Standard(r) => print_common(r),
                layer5::UndoBlockRedoData2Rest::Extended(r) => {
                    print_common(&r.common);
                    println!("BuExt idx: {} flg2: {}", r.buext_idx, r.flags2);
                }
            };
        }

        if let Some(op) = &self.op {
            print_record(op)?;
        }
        Ok(true)
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
                d.uba.dba.0, d.uba.seq, d.uba.record, d.ext, d.spc, d.fbi
            );
        }

        if let Some(d) = self.scn {
            print!("\nscn:  0x{:016x}", d.full_scn());
        }

        println!();
        Ok(true)
    }
}
