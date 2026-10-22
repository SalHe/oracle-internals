use ointernal_parser::oracle::change::{
    RecoveryManagement, layers::layer17::StandbyMetadataCacheInvalidation,
};

use crate::dump::Dump;

impl Dump for RecoveryManagement<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::StandbyMetadataCacheInvalidation(Some(x)) => x.dump(),
            _ => Ok(false),
        }
    }
}

impl Dump for StandbyMetadataCacheInvalidation<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!("STANDBY METADATA CACHE INVALIDATION");
        println!("KQR INFO:");
        for e in self.kqr {
            println!("[{:x}, {}, {}]", e.id, e.kind, e.value);
        }
        println!("KGL INFO:");
        for e in self.kgl {
            println!(
                "[{:x}.{:x}.{:x}.{:x}, {}]",
                e.id[0],
                e.id[1],
                e.id[2],
                e.id[3],
                e.count & 0xff
            );
        }
        Ok(true)
    }
}
