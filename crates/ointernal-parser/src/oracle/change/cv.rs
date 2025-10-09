use crate::oracle::{
    opcode::{Op, SegmentManagement},
    redo::Change,
};

#[derive(Debug)]
pub struct ChangeAndVector {
    pub change: Change,
    pub vector: Vec<Vec<u8>>,
}

impl ChangeAndVector {
    pub fn parse(&self) -> Option<()> {
        let op = Op::parse((self.change.layer, self.change.op))?;
        match op {
            Op::TransactionControl(_) => todo!(),
            Op::TransactionRead(_) => todo!(),
            Op::TransactionUpdate(_) => todo!(),
            Op::BlockCleanOut(_) => todo!(),
            Op::TransactionUndoManagement(_) => todo!(),
            Op::ControlFile(_) => todo!(),
            Op::Index(_) => todo!(),
            Op::Table(_) => todo!(),
            Op::Cluster(_) => todo!(),
            Op::SegmentManagement(o) => match o {
                SegmentManagement::AllocateSpace => todo!(),
                SegmentManagement::BlockFormatRedo => {
                    Some(())
                }
                SegmentManagement::BlockLinkModify => todo!(),
                SegmentManagement::FreelistModify => todo!(),
                SegmentManagement::BitmapBlockUndo1 => todo!(),
                SegmentManagement::BitmapBlockUndo2 => todo!(),
                SegmentManagement::FormatPagetableSegmentHeader => todo!(),
                SegmentManagement::FormatLevel1BitmapBlock => todo!(),
                SegmentManagement::FormatLevel2BitmapBlock => todo!(),
                SegmentManagement::FormatPagetableDatablock => todo!(),
                SegmentManagement::StateChangeLevel1Bitmap => todo!(),
                SegmentManagement::UndoLevel1Bitmap => todo!(),
                SegmentManagement::StateChangeLevel2Bitmap => todo!(),
                SegmentManagement::UndoLevel2Bitmap => todo!(),
                SegmentManagement::StateChangeLevel3Bitmap => todo!(),
                SegmentManagement::UndoLevel3Bitmap => todo!(),
                SegmentManagement::UpdateLhwmHhwm => todo!(),
                SegmentManagement::UndoSegmentHeader => todo!(),
                SegmentManagement::SegmentShrinkL1Bitmap => todo!(),
                SegmentManagement::SegmentShrinkSegmentHeader => todo!(),
                SegmentManagement::SegmentShrinkL2Bitmap => todo!(),
                SegmentManagement::FormatLobExtentHeaderBlock => todo!(),
                SegmentManagement::FormatLobPersistentUndoBlock => todo!(),
                SegmentManagement::FormatLobHashBucketBlock => todo!(),
                SegmentManagement::FormatLobFreeSpaceBlock => todo!(),
                SegmentManagement::FormatLobSegmentHeaderBlock => todo!(),
                SegmentManagement::LobHashBucketBlockRedo => todo!(),
                SegmentManagement::LobHashBucketBlockUndo => todo!(),
                SegmentManagement::LobFreeSpaceBlockRedo => todo!(),
                SegmentManagement::LobFreeSpaceBlockUndo => todo!(),
                SegmentManagement::LobPersistentUndoAmendRedo => todo!(),
                SegmentManagement::LobPersistentUndoBlockUndo => todo!(),
                SegmentManagement::LobSegmentHeaderRedo => todo!(),
                SegmentManagement::LobSegmentHeaderUndo => todo!(),
            },
            Op::ExtentManagement(_) => todo!(),
            Op::Tablespace(_) => todo!(),
            Op::RowCache(_) => todo!(),
            Op::RecoveryManagement(_) => todo!(),
            Op::BlockImage(_) => todo!(),
            Op::DirectLoader(_) => todo!(),
            Op::CompatibilitySegment(_) => todo!(),
            Op::LobSegment(_) => todo!(),
            Op::LocallyManagedTablespace(_) => todo!(),
            Op::BlockWrites(_) => todo!(),
            Op::DdlStatements(_) => todo!(),
            Op::Queues(_) => todo!(),
            Op::Lob(_) => todo!(),
            Op::BlockChangeTracking(_) => todo!(),
        }
    }
}
