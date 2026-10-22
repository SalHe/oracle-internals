use ointernal_parser::oracle::change::{ExtentManagement, layers::layer14::LockExtent};

use crate::dump::Dump;

impl Dump for ExtentManagement<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::ClearExtentControlLock(_) => {
                println!("ktecush redo: clear extent control lock");
                Ok(true)
            }
            Self::LockExtent(Some(x)) => x.dump(),
            _ => Ok(false),
        }
    }
}

impl Dump for LockExtent<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!(
            "ktelk redo: xid:  0x{:04x}.{:03x}.{:08x}",
            self.xid.usn, self.xid.slot, self.xid.sequence,
        );
        Ok(true)
    }
}
