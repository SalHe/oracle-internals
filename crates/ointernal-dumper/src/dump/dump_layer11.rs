use num_traits::FromPrimitive;
use ointernal_parser::oracle::change::{
    Table,
    layers::layer11::{self, InsertRowPiece},
};

use crate::dump::DumpRedo;

impl DumpRedo for Table<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::InsertRowPiece(Some(irp)) => irp.dump(),
            _ => Ok(false),
        }
    }
}

impl DumpRedo for InsertRowPiece<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!("KTB Redo ");
        println!(
            "op: 0x{:02}  ver: 0x{:02}  ",
            self.op.b1.into_bits(), // display byte1, not only op code's bitfields
            self.op.b2.version(),
        );
        println!(
            "compat bit: {} ({}) padding: {}",
            layer11::IRPByte2::new()
                .with_post_11(self.op.b2.post_11())
                .into_bits(),
            if self.op.b2.post_11() {
                "post-11"
            } else {
                "pre-11"
            },
            if self.op.b2.post_11() { 1 } else { 0 }
        );

        match self.op.rest()? {
            Some(r) => match r {
                layer11::InsertRowPieceOpRest::F(f) => println!(
                    "op: F  xid:  0x{:04x}.{:03x}.{:08x}    uba: 0x{:08x}.{:04x}.{:02x}",
                    f.xid.usn, f.xid.slot, f.xid.sequence, f.uba.dba.0, f.uba.seq, f.uba.record,
                ),
                layer11::InsertRowPieceOpRest::C(uba) => println!(
                    "op: C  uba: 0x{:08x}.{:04x}.{:02x}",
                    uba.dba.0, uba.seq, uba.record,
                ),
                layer11::InsertRowPieceOpRest::Z => println!("op: Z ??????????????"),
                layer11::InsertRowPieceOpRest::L => println!("op: L ??????????????"),
                layer11::InsertRowPieceOpRest::R => println!("op: R ??????????????"),
                layer11::InsertRowPieceOpRest::N => println!("op: N ??????????????"),
                layer11::InsertRowPieceOpRest::G => println!("op: G ??????????????"),
            },
            None => println!("op: [ktbdir] invalid op: 0x{:02X} ", self.op.b1.into_bits()),
        }

        println!(
            "KDO Op code: {} row dependencies {}",
            match layer11::KdoOp::from_u8(self.meta.byte1.op()) {
                Some(op) => match op {
                    layer11::KdoOp::IUR => "IUR",
                    layer11::KdoOp::IRP => "IRP",
                    layer11::KdoOp::DRP => "DRP",
                    layer11::KdoOp::LKR => "LKR",
                    layer11::KdoOp::URP => "URP",
                    layer11::KdoOp::ORP => "ORP",
                    layer11::KdoOp::MFC => "MFC",
                    layer11::KdoOp::CFA => "CFA",
                    layer11::KdoOp::CKI => "CKI",
                    layer11::KdoOp::SKL => "SKL",
                    layer11::KdoOp::QMI => "QMI",
                    layer11::KdoOp::QMD => "QMD",
                    layer11::KdoOp::DSC => "DSC",
                    layer11::KdoOp::LMN => "LMN",
                    layer11::KdoOp::LLB => "LLB",
                    layer11::KdoOp::SHK => "SHK",
                    layer11::KdoOp::CMP => "CMP",
                    layer11::KdoOp::DCU => "DCU",
                    layer11::KdoOp::MRK => "MRK",
                    layer11::KdoOp::IRP2 => "IRP2",
                    layer11::KdoOp::DRP2 => "DRP2",
                    layer11::KdoOp::CMP2 => "CMP2",
                    layer11::KdoOp::DGD => "DGD",
                }
                .to_string(),
                None => format!("{}", self.meta.byte1.op()),
            },
            // TODO verify?
            if self.meta.byte1.row_dependency() {
                "Enabled"
            } else {
                "Disabled"
            }
        );

        println!(
            "  xtype: {} flags: 0x{:08x}  bdba: 0x{:08x}  hdba: 0x{:08x}",
            match layer11::IRPXtype::from_u8(self.meta.xtype.typ()) {
                Some(xt) => match xt {
                    layer11::IRPXtype::XA => "XA",
                    layer11::IRPXtype::XR => "XR",
                    layer11::IRPXtype::CR => "CR",
                }
                .to_string(),
                None => format!("{}", self.meta.xtype.typ()),
            },
            self.meta.xtype.with_typ(0).into_bits(), // clear type, flags remained
            self.meta.bdba.0,
            self.meta.hdba.0,
        );
        println!(
            "itli: {}  ispac: {}  maxfr: {}",
            self.meta.itli, self.meta.ispac, self.meta.maxfr,
        );
        println!(
            "tabn: {} slot: {}(0x{:x}) size/delt: {}",
            self.meta.tabn as i32, // seems dumped as i32 in Oracle
            self.meta.slot,
            self.meta.slot,
            self.meta.delta_size,
        );
        println!(
            "fb: {}{}{}{}{}{}{}{} lb: 0x{:x}  cc: {}",
            if self.meta.fb_flags.k() { 'K' } else { '-' },
            if self.meta.fb_flags.c() { 'C' } else { '-' },
            if self.meta.fb_flags.h() { 'H' } else { '-' },
            if self.meta.fb_flags.d() { 'D' } else { '-' },
            if self.meta.fb_flags.f() { 'F' } else { '-' },
            if self.meta.fb_flags.l() { 'L' } else { '-' },
            if self.meta.fb_flags.p() { 'P' } else { '-' },
            if self.meta.fb_flags.n() { 'N' } else { '-' },
            self.meta.lb,
            self.meta.column_count,
        );

        print!("null: ");
        for c in 0..self.meta.column_count {
            let n = match self.meta.is_col_null(c) {
                Some(is_null) => {
                    if is_null {
                        "N"
                    } else {
                        "-"
                    }
                }
                None => "?",
            };
            print!("{}", n);
        }
        println!();

        for (i, col) in (0..self.meta.column_count)
            .map(|i| self.col_data(i as _).expect("Column must exist"))
            .enumerate()
        {
            println!(
                "col {:>2}: {}{}",
                i,
                if let Some(c) = col {
                    &format!("[{}] ", c.len())
                } else {
                    ""
                },
                col.map(hex::encode).unwrap_or("*NULL*".to_string())
            );
        }

        Ok(true)
    }
}
