use num_traits::FromPrimitive;
use ointernal_parser::oracle::change::{
    Table,
    layers::layer11::{self, InsertRowPiece},
};

use crate::dump::Dump;

impl Dump for Table<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::UndoRowOperation(Some(irp)) => irp.dump(),
            Self::InsertRowPiece(Some(irp)) => irp.dump(),
            _ => Ok(false),
        }
    }
}

impl Dump for InsertRowPiece<'_> {
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
                layer11::InsertRowPieceOpRest::Z => println!("op: Z"),
                layer11::InsertRowPieceOpRest::L => println!("op: L ??????????????TODO"),
                layer11::InsertRowPieceOpRest::R => println!("op: R ??????????????TODO"),
                layer11::InsertRowPieceOpRest::N => println!("op: N ??????????????TODO"),
                layer11::InsertRowPieceOpRest::G => println!("op: G ??????????????TODO"),
            },
            None => println!("op: [ktbdir] invalid op: 0x{:02X} ", self.op.b1.into_bits()),
        }

        let print_kdo_op = |op: Option<&'static str>| {
            println!(
                "KDO Op code: {} row dependencies {}",
                match op {
                    Some(x) => x.to_string(),
                    None => format!("{}", self.meta.op.op()),
                },
                // TODO verify?
                if self.meta.op.row_dependency() {
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
        };

        let kdo_data = self.meta.rest()?;
        match kdo_data {
            Some(op) => match op {
                layer11::RPMetaRest::IUR => {
                    print_kdo_op(Some("IUR"));
                }
                layer11::RPMetaRest::IRP(m) => {
                    print_kdo_op(Some("IRP"));

                    println!(
                        "tabn: {} slot: {}(0x{:x}) size/delt: {}",
                        m.tabn as i32, // seems dumped as i32 in Oracle
                        m.slot,
                        m.slot,
                        m.delta_size,
                    );
                    println!(
                        "fb: {}{}{}{}{}{}{}{} lb: 0x{:x}  cc: {}",
                        if m.fb_flags.k() { 'K' } else { '-' },
                        if m.fb_flags.c() { 'C' } else { '-' },
                        if m.fb_flags.h() { 'H' } else { '-' },
                        if m.fb_flags.d() { 'D' } else { '-' },
                        if m.fb_flags.f() { 'F' } else { '-' },
                        if m.fb_flags.l() { 'L' } else { '-' },
                        if m.fb_flags.p() { 'P' } else { '-' },
                        if m.fb_flags.n() { 'N' } else { '-' },
                        m.lb,
                        m.column_count,
                    );

                    print!("null: ");
                    for c in 0..m.column_count {
                        let n = match m.is_col_null(c) {
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

                    for (i, col) in (0..m.column_count)
                        .map(|i| self.col_data(m, i as _).expect("Column must exist"))
                        .enumerate()
                    {
                        println!(
                            "col {:>2}: {} {}",
                            i,
                            if let Some(c) = col {
                                &format!("[{:>2}] ", c.len())
                            } else {
                                ""
                            },
                            col.map(hex::encode).unwrap_or("*NULL*".to_string())
                        );
                    }
                }
                layer11::RPMetaRest::DRP(m) => {
                    print_kdo_op(Some("DRP"));
                    println!(
                        "tabn: {} slot: {}(0x{:x})",
                        m.tabn as i32, // seems dumped as i32 in Oracle
                        m.slot,
                        m.slot,
                    );
                }
                layer11::RPMetaRest::LKR => {
                    print_kdo_op(Some("LKR"));
                }
                layer11::RPMetaRest::URP => {
                    print_kdo_op(Some("URP"));
                }
                layer11::RPMetaRest::ORP => {
                    print_kdo_op(Some("ORP"));
                }
                layer11::RPMetaRest::MFC => {
                    print_kdo_op(Some("MFC"));
                }
                layer11::RPMetaRest::CFA => {
                    print_kdo_op(Some("CFA"));
                }
                layer11::RPMetaRest::CKI => {
                    print_kdo_op(Some("CKI"));
                }
                layer11::RPMetaRest::SKL => {
                    print_kdo_op(Some("SKL"));
                }
                layer11::RPMetaRest::QMI => {
                    print_kdo_op(Some("QMI"));
                }
                layer11::RPMetaRest::QMD => {
                    print_kdo_op(Some("QMD"));
                }
                layer11::RPMetaRest::DSC => {
                    print_kdo_op(Some("DSC"));
                }
                layer11::RPMetaRest::LMN => {
                    print_kdo_op(Some("LMN"));
                }
                layer11::RPMetaRest::LLB => {
                    print_kdo_op(Some("LLB"));
                }
                layer11::RPMetaRest::SHK => {
                    print_kdo_op(Some("SHK"));
                }
                layer11::RPMetaRest::CMP => {
                    print_kdo_op(Some("CMP"));
                }
                layer11::RPMetaRest::DCU => {
                    print_kdo_op(Some("DCU"));
                }
                layer11::RPMetaRest::MRK => {
                    print_kdo_op(Some("MRK"));
                }
                layer11::RPMetaRest::IRP2 => {
                    print_kdo_op(Some("IRP2"));
                }
                layer11::RPMetaRest::DRP2 => {
                    print_kdo_op(Some("DRP2"));
                }
                layer11::RPMetaRest::CMP2 => {
                    print_kdo_op(Some("CMP2"));
                }
                layer11::RPMetaRest::DGD => {
                    print_kdo_op(Some("DGD"));
                }
            },
            None => print_kdo_op(None),
        };
        Ok(true)
    }
}
