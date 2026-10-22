use ointernal_parser::oracle::change::{
    LocallyManagedTablespace,
    layers::layer22::{FileSpaceHeaderOp, FileSpaceHeaderRedo, FileSpaceHeaderSave},
};

use crate::dump::Dump;

impl Dump for LocallyManagedTablespace<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::FileSpaceHeaderRedo(Some(x)) => x.dump(),
            _ => Ok(false),
        }
    }
}

impl Dump for FileSpaceHeaderRedo<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!("ktfbhredo - File Space Header Redo:");
        // The source of this field is not in the payload; every observed record
        // prints an empty value.
        println!("Header Opcode: ");
        let save = match self.save() {
            FileSpaceHeaderSave::Save => "Save",
            FileSpaceHeaderSave::NoSave => "NoSave",
            FileSpaceHeaderSave::Clear => "Clear",
        };
        match self.op() {
            FileSpaceHeaderOp::NoPending => println!("{}: No Pending Op ", save),
            FileSpaceHeaderOp::UsingExtent => {
                println!("{}: Using Extent: ", save);
                dump_begin(self);
            }
            FileSpaceHeaderOp::FreeingExtent => {
                println!("{}: Freeing Extent: ", save);
                dump_begin(self);
            }
            FileSpaceHeaderOp::FreeExtent => {
                println!("{}: Free Extent: ", save);
                dump_begin(self);
            }
            FileSpaceHeaderOp::UseExtent => {
                println!("{}: Use Extent: ", save);
                dump_begin(self);
            }
            FileSpaceHeaderOp::FileResize => {
                println!("{}: File Resize: ", save);
                println!("New End: {}, New Size: {} ", self.begin(), self.length());
            }
            FileSpaceHeaderOp::StatsOn => {
                println!("{}: Stats On: ", save);
                println!("First: {}, Free: {} ", self.begin(), self.length());
            }
            FileSpaceHeaderOp::FileAutoExtendOn => {
                println!("{}: File AutoExtend On: ", save);
                println!("MaxSize: {}, Increment: {} ", self.begin(), self.length());
            }
            FileSpaceHeaderOp::FileAutoExtendOff => {
                println!("{}: File AutoExtend Off: ", save);
            }
            FileSpaceHeaderOp::StatsOff => println!("{}: Stats Off: ", save),
            FileSpaceHeaderOp::InvalidateRange => {
                println!("{}: Invalidate Range: ", save);
                println!("BeginBlock: {}, Length: {} ", self.begin(), self.length());
            }
            FileSpaceHeaderOp::Unknown(_) => println!("{}: Unknown OPcode: ", save),
        }
        Ok(true)
    }
}

fn dump_begin(x: &FileSpaceHeaderRedo) {
    println!(
        "Begin: {}, Length: {}, Instance: {} ",
        x.begin(),
        x.length(),
        x.instance()
    );
}
