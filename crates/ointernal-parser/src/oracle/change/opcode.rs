use std::marker::PhantomData;

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use thiserror::Error;

use crate::oracle::change::cv::FromCV;

use super::{ChangeAndVector, ConvertError, VectorData, layers};

#[derive(Error, Debug)]
pub enum OpDataError {
    #[error("unrecognized op {0}.{1}")]
    UnrecognizedOp(u8, u8),
    #[error("{0:?}")]
    ConvertError(#[from] ConvertError),
    #[error("change has been encrypted")]
    Encrypted,
}

macro_rules! codes_for_layer {
    (
        $(
            $(#[$layer_meta:meta])*
            $layer_name:ident = $layer_code:literal {
                $(
                    $(#[$op_meta:meta])*
                    $op_name:ident = $op_code:literal $(! $($op_data_type:ident)::*)?,
                )*
            }
        )+
    ) => {
        // op code and data enum
        #[derive(Debug)]
        pub enum Op<'a> {
            $(
                #[doc = concat!("Layer: ", stringify!($layer_code), "\n\n")]
                $layer_name($layer_name<'a>),
            )*
        }

        impl Op<'_> {
            pub fn parse(value: (u8, u8)) -> Option<Self> {
                let (in_layer, in_op) = value;
                match LayerCode::from_u8(in_layer) {
                    Some(parsed_layer_code) => match parsed_layer_code {
                        $(
                            LayerCode::$layer_name => $layer_name::from_u8(in_op).map(Op::$layer_name),
                        )*
                    },
                    None => None,
                }
            }
        }

        // layer code enum
        #[derive(Debug, FromPrimitive, ToPrimitive, PartialEq, Eq)]
        enum LayerCode {
            $(
                $(#[$layer_meta])*
                $layer_name = $layer_code,
            )*
        }

        // opcode in all layers
        $(
            #[doc = concat!("Layer: ", stringify!($layer_code), "\n\n")]
            $(#[$layer_meta])*
            #[allow(unused_parens)]
            #[derive(Debug)]
            pub enum $layer_name<'a> {
                $(
                    #[doc = concat!("OP: ", stringify!($layer_code), ".", stringify!($op_code), "\n\n")]
                    $(#[$op_meta])*

                    $op_name(Option<(
                        $($($op_data_type)::*<'a>)?
                    )>),
                )*
                Unknown(&'a PhantomData<()>)
            }

            impl<'a> FromPrimitive for $layer_name<'a> {
                fn from_i64(n: i64) -> Option<Self> {
                    Self::from_u64(n as _)
                }

                fn from_u64(n: u64) -> Option<Self> {
                    match n {
                        $(
                            $op_code => Some($layer_name::$op_name(None)),
                        )*
                        _ => None,
                    }
                }
            }
        )*

        // op data parsing
        impl ChangeAndVector {
            pub fn parse<'a>(&'a self) -> Result<Op<'a>, OpDataError> {
                if self.change.type_.encrypted() {
                    Err(OpDataError::Encrypted)
                } else {
                    parse_impl(self.change.layer, self.change.op, &self.vector)
                }
            }
        }

        pub fn parse_impl(layer: u8, op: u8, vectors: &[VectorData]) -> Result<Op<'_>, OpDataError> {
            let op = Op::parse((layer, op))
                        .ok_or(OpDataError::UnrecognizedOp(layer, op))?;
            match op {
                $(
                    Op::$layer_name(op_code) => match op_code {
                        $(
                            $layer_name::$op_name(_) => {
                                $(
                                    return Ok(Op::$layer_name($layer_name::$op_name(Some(<$($op_data_type)::*<'_>>::from_vectors(vectors)?))));
                                )?
                                #[allow(unreachable_code)]
                                Ok(Op::$layer_name($layer_name::$op_name(None)))
                            }
                        )*
                        $layer_name::Unknown(_) => unreachable!(),
                    }
                )*
            }
        }
    };
}

codes_for_layer! {
    /// Transaction Control operations
    TransactionControl = 1 {}

    /// Transaction read operations
    TransactionRead = 2 {}

    /// Transaction update operations
    TransactionUpdate = 3 {}

    BlockCleanOut = 4 {
        /// Block cleanout record
        BlockCleanOutRecord = 1,
        /// Physical cleanout
        PhysicalCleanOut = 2,
        /// Single array change
        SingleArrayChanges = 3,
        /// Multiple array changes
        MultiArrayChanges = 4,
        /// Format block
        FormatBlock = 5,
        /// ktbcc redo - Commit Time Block Cleanout Change
        CommitTimeBlockCleanOutChange = 6,
    }

    TransactionUndoManagement = 5 {
        /// ktudb redo (undo block redo)
        UndoBlockRedo = 1 !layers::layer5::UndoBlockRedo,
        /// ktudh redo (undo header redo)
        UndoHeaderRedo = 2 !layers::layer5::UndoHeaderRedo,
        /// Rollout a transaction begin
        RolloutTxBegin = 3,
        /// ktucm redo (change transaction slot state (commit/rollback))
        ChangeTxSlotStatus = 4 !layers::layer5::ChangeTxSlotStatus,
        /// Create rollback segment
        CreateRollbackSeg = 5,
        /// ktubu redo: mark undo record "user undo done"
        MarkUndoRecordDone = 6,
        /// Restore transaction table slot.
        /// In the ktubl for 'dbms_transaction.local_transaction_id'
        /// (begin transaction) - also arrives for incoming distributed
        /// tx, no data change but TT slot acquired. Also for recursive
        /// transaction (e.g. truncate). txn start scn: 0xffff.ffffffff
        /// Also prior to securefile LOB insertion.
        RestoreTxTableSlot = 7,
        /// Mark transaction as dead
        MarkTxDead = 8,
        /// Rollback extension of rollback seg
        RollbackExtension = 9,
        /// Rollback segment header change for extension of rollback seg
        RollbackSegmentHeaderForRollbackSeg = 10,
        /// Set current undo block (uba) in undo header during rollback
        SetUndoBlockInHeader = 11,
        /// Transaction audit record - first
        TxAuditRecordFirst = 19,
        /// Transaction audit record - subsequent
        TxAuditRecordSub = 20,
        /// ktudbr redo: disable block level recovery (reports XID)
        DisableBlockLevelRecovery = 23,
        /// ktfbhundo - File Space Header Undo
        FileSpaceHeaderUndo = 24,
        /// ktuuntp - ??
        Kttuubtp = 28,
    }

    /// Control file operations
    ControlFile = 6 {}

    Index = 10 {
        /// SQL load index block
        SqlLoadIndexBlock = 1,
        /// Insert Leaf Row
        InsertLeafRow = 2,
        /// Purge Leaf Row
        PurgeLeafRow = 3,
        /// Delete Leaf Row
        DeleteLeafRow = 4,
        /// Restore Leaf Row during rollback
        RestoreLeafRowDuringRollback = 5,
        /// (kdxlok) Lock block (pre-split)
        LockBlockPreSplit = 6,
        /// (kdxulo) unlock block (clear block opcode during rollback)
        UnlockBlock = 7,
        /// (kdxlne) initialize (and load) newly allocated leaf block
        InitializeNewLeafBlock = 8,
        /// (kdxair) apply XAT do to ITL 1 -- (service ITL) related to leaf block split
        ApplyXatToItl = 9,
        /// Set leaf block next pointer
        SetLeafBlockNextPointer = 10,
        /// (kdxlpr) set kdxleprv / kdxlenxt (previous / next leaf pointer)
        SetLeafPointers = 11,
        /// Initialize root block after split
        InitializeRootBlockAfterSplit = 12,
        /// index redo (kdxlem): (REDO) make leaf block empty
        MakeLeafBlockEmpty = 13,
        /// Restore block before image
        RestoreBlockBeforeImage = 14,
        /// (kdxbin) Insert branch block row
        InsertBranchBlockRow = 15,
        /// Purge branch row
        PurgeBranchRow = 16,
        /// Initialize new branch block
        InitializeNewBranchBlock = 17,
        /// Update key data in row -- index redo (kdxlup): update keydata
        UpdateKeyDataInRow = 18,
        /// Clear split flag
        ClearSplitFlag = 19,
        /// Set split flag
        SetSplitFlag = 20,
        /// index general undo (branch) operations
        IndexGeneralUndoBranch = 21,
        /// Undo leaf operation
        UndoLeafOperation = 22,
        /// restore block to tree
        RestoreBlockToTree = 23,
        /// Shrink ITL
        ShrinkItl = 24,
        /// format root block
        FormatRootBlock = 25,
        /// format root block (undo)
        FormatRootBlockUndo = 26,
        /// format root block (redo)
        FormatRootBlockRedo = 27,
        /// Migrating block (undo)
        MigratingBlockUndo = 28,
        /// Migrating block (redo)
        MigratingBlockRedo = 29,
        /// Update nonkey value
        UpdateNonkeyValue = 30,
        /// index root block redo (kdxdlr): create/load index
        IndexRootBlockRedo = 31,
        /// make block empty
        MakeBlockEmpty = 34,
        /// index redo (kdxlcnu): update nonkey
        UpdateNonkey = 35,
        /// undo index change (kdxIndexlogicalNonkeyUpdate) -- bitmap index
        UndoIndexChangeBitmap = 37,
        /// index change (kdxIndexlogicalNonkeyUpdate) -- bitmap index
        IndexChangeBitmap = 38,
        /// index redo (kdxbur) : branch block update range
        BranchBlockUpdateRange = 39,
        /// index redo (kdxbdu) : branch block DBA update
        BranchBlockDbaUpdate = 40,
    }

    Table = 11 {
        /// undo row operation
        UndoRowOperation = 1 !layers::layer11::KtbRedo,
        /// IRP insert row piece
        InsertRowPiece = 2 !layers::layer11::KtbRedo,
        /// DRP delete row piece
        DeleteRowPiece = 3 !layers::layer11::KtbRedo,
        /// lock row piece
        LockRowPiece = 4 !layers::layer11::KtbRedo,
        /// URP update row piece
        UpdateRowPiece = 5 !layers::layer11::KtbRedo,
        /// overwrite row piece
        OverwriteRowPiece = 6,
        /// manipulate first column
        ManipulateFirstColumn = 7,
        /// change forwarding address - migration
        ChangeForwardingAddress = 8,
        /// change cluster key index
        ChangeClusterKeyIndex = 9,
        /// Set Cluster key pointers
        SetClusterKeyPointers = 10,
        /// Insert multiple rows
        InsertMultipleRows = 11 !layers::layer11::KtbRedo,
        /// Delete multiple rows
        DeleteMultipleRows = 12,
        /// toggle block header flags
        ToggleBlockHeaderFlags = 13,
        /// Marked: KDODSC
        Kdodsc = 14,
        /// Marked: KDOMBC
        Kdombc = 15,
        /// Marked: Logminer support - RM for rowpiece with only logminer columns - KDOLMN
        LogminerRowpiece = 16,
        /// Update multiple rows
        UpdateMultipleRows = 17,
        /// Marked: Logminer support - RM for LOB operation errors - KDOLBE
        LogminerLobErrors = 18,
        /// Array update ?
        ArrayUpdate = 19 !layers::layer11::KtbRedo,
        /// SHK (mark as shrunk?)
        MarkAsShrunk = 20,
        /// Marked: Logminer support - KDOURP2
        Kdourp2 = 21,
        /// Marked: Logminer support - KDOCMP
        Kdocmp = 22,
        /// Marked: Logminer support - KDODCU
        Kdodcu = 23,
        /// HCC update rowid map ?
        HccUpdateRowidMap = 24,
        /// Marked: Logminer support - KDOAIR
        Kdoair = 25,
        /// "CMP2" Row Store Compress - partial compression ?
        RowStoreCompressPartial = 28,
    }

    /// Cluster operations
    Cluster = 12 {}

    SegmentManagement = 13 {
        /// ktsfm redo: -- allocate space ??
        AllocateSpace = 1,
        /// KTSFRBFMT (block format) redo
        BlockFormatRedo = 5 !layers::layer13::BlockFormat,
        /// (block link modify) (? index) (options: lock clear, lock set)
        BlockLinkModify = 6 !layers::layer13::BlockLinkModify,
        /// KTSFRGRP (fgb/shdr modify freelist) redo: (options unlink block, move HWM)
        FreelistModify = 7 !layers::layer13::FreelistModify,
        Unknown11 = 11,
        /// ktsbbu undo - undo operation on bitmap block
        BitmapBlockUndo1 = 13,
        /// ktsbbu undo - undo operation on bitmap block
        BitmapBlockUndo2 = 14,
        Unknown15 = 15,
        /// ktsphfredo - Format Pagetable Segment Header
        FormatPagetableSegmentHeader = 17,
        /// ktspffredo - Format Level 1 Bitmap Block
        FormatLevel1BitmapBlock = 18,
        /// ktspsfredo - Format Level 2 Bitmap Block
        FormatLevel2BitmapBlock = 19,
        /// ktspbfredo - Format Pagetable Datablock
        FormatPagetableDatablock = 21,
        /// State change on level 1 bitmap block
        StateChangeLevel1Bitmap = 22,
        /// Undo on level 1 bitmap block
        UndoLevel1Bitmap = 23,
        /// State change on level 2 bitmap block
        StateChangeLevel2Bitmap = 24,
        /// Undo on level 2 bitmap block
        UndoLevel2Bitmap = 25,
        /// State change on level 3 bitmap block
        StateChangeLevel3Bitmap = 26,
        /// Undo on level 3 bitmap block
        UndoLevel3Bitmap = 27,
        /// Update LHWM and HHWM on segment header
        UpdateLhwmHhwm = 28,
        /// Undo on segment header
        UndoSegmentHeader = 29,
        /// Segment shrink redo for L1 bitmap block
        SegmentShrinkL1Bitmap = 31,
        /// Segment shrink redo for segment header block
        SegmentShrinkSegmentHeader = 32,
        /// Segment shrink redo for L2 bitmap block
        SegmentShrinkL2Bitmap = 36,
        /// KTSL - Format LOB Extent Header Block (EHB)
        FormatLobExtentHeaderBlock = 41,
        /// KTSL - Format LOB Persisten Undo Block (PUB)
        FormatLobPersistentUndoBlock = 42,
        /// KTSL - Format LOB Hash Bucket Block (UFS) -- uncommitted free space
        FormatLobHashBucketBlock = 43,
        /// KTSL - Format LOB Free Space Block (FSB) -- also CFS, committed free space
        FormatLobFreeSpaceBlock = 44,
        /// KTSL - Format LOB Segment Header Block (SHB)
        FormatLobSegmentHeaderBlock = 45,
        /// KTSL - HBB redo record
        LobHashBucketBlockRedo = 49,
        /// KTSL - LOB Hash Bucket Block (HBB) undo
        LobHashBucketBlockUndo = 50,
        /// KTSL - FSB redo record (Free Space Block)
        LobFreeSpaceBlockRedo = 51,
        /// KTSL - LOB Free Space Block (FSB) undo record
        LobFreeSpaceBlockUndo = 52,
        /// KTSL - PUA redo record: (Persistent Undo Amend ?)
        LobPersistentUndoAmendRedo = 53,
        /// KTSL - LOB Persistent Undo Block (PUB) undo record
        LobPersistentUndoBlockUndo = 54,
        /// KTSL - Segment Header (SHB) redo record
        LobSegmentHeaderRedo = 55,
        /// KTSL - LOB Segment Header Block (SHB) undo record
        LobSegmentHeaderUndo = 56,
    }

    ExtentManagement = 14 {
        /// ktecush redo: clear extent control lock
        ClearExtentControlLock = 1,
        /// ktelk redo - lock extent (map)
        LockExtent = 2,
        /// Extent de-allocate
        ExtentDeallocate = 3,
        /// kteop redo - redo operation on extent map
        ExtentMapRedo = 4,
        /// kteopu undo - undo operation on extent map
        ExtentMapUndo = 5,
        /// kteopf redo - format an extent map block
        FormatExtentMapBlock = 6,
        /// kteoputrn - undo operation for flush for truncate
        UndoForTruncateFlush = 8,
        /// kteuUpdredo redo - Propagate Extent commit timetmep
        PropagateExtentCommitTimestamp = 12,
    }

    /// Tablespace operations
    Tablespace = 15 {}

    /// Row cache operations
    RowCache = 16 {}

    RecoveryManagement = 17 {
        /// End backup mode marker
        EndBackupModeMarker = 1,
        /// Crash Recovery at scn: 0x0000.02429111
        CrashRecoveryAtScn = 3,
        /// STANDBY METADATA CACHE INVALIDATION (after commit on sys table update)
        StandbyMetadataCacheInvalidation = 28,
        /// RC Invalidation Marker
        RcInvalidationMarker = 51,
    }

    BlockImage = 18 {
        /// Block image
        BlockImage = 1,
        /// Object / Range reuse
        ObjectRangeReuse = 3,
    }

    DirectLoader = 19 {
        /// Direct Loader block redo entry (for logged direct path writes)
        DirectLoaderBlockRedo = 1,
        /// Direct Loader invalidate block range redo entry (for nologging direct path writes)
        DirectLoaderInvalidateBlockRange = 2,
        /// Direct block relogging - KCBLCRLB
        DirectBlockRelogging = 3,
        /// Invalidate range relogging - KCBLCRIR
        InvalidateRangeRelogging = 4,
    }

    CompatibilitySegment = 20 {
        /// Media Recovery Marker - related to LOB load (KDLI)
        MediaRecoveryMarkerLob = 24,
    }

    LobSegment = 21 {
        /// kdlop (Long Feild) redo: \[sic\] (insert basicfile clob)
        LongFieldRedo = 1,
    }

    LocallyManagedTablespace = 22 {
        /// ktfbhredo - File Space Header Redo:
        FileSpaceHeaderRedo = 2,
        /// ktfbhundo - File Space Header Undo:
        FileSpaceHeaderUndo = 3,
        /// ktfbbredo - File BitMap Block Redo:
        FileBitmapBlockRedo = 5,
        /// File Property Map Block (FPM)
        FilePropertyMapBlock = 16,
    }

    BlockWrites = 23 {
        /// MEDIA RECOVERY MARKER (Block written record 12c)
        MediaRecoveryMarker12c = 1,
        /// Block read record (BRR) - written when db_lost_write_protect enabled
        BlockReadRecord = 2,
        /// MEDIA RECOVERY MARKER (Block written record 19c)
        MediaRecoveryMarker19c = 3,
    }

    DdlStatements = 24 {
        /// MEDIA RECOVERY MARKER - DDL
        MediaRecoveryMarkerDdl = 1,
        /// Direct load block end mark
        DirectLoadBlockEndMark = 2,
        /// Media recovery marker
        MediaRecoveryMarker = 4,
        /// Unknown operation 1
        Unknown1 = 10,
        /// Unknown operation 2
        Unknown2 = 11,
    }

    /// Queues operations
    Queues = 25 {}

    Lob = 26 {
        /// Invalidate LOB (?) (KDLI)
        InvalidateLob = 4,
        /// Load LOB (KDLI)
        LoadLob = 6,
    }

    /// Block Change Tracking operations
    BlockChangeTracking = 27 {}
}

#[cfg(test)]
mod test {
    use num_traits::FromPrimitive;

    use crate::oracle::change::opcode::{BlockCleanOut, LayerCode, Op};

    #[test]
    fn undefined_layer_code() {
        assert!(LayerCode::from_u8(0).is_none());
    }

    #[test]
    fn normal_op() {
        assert!(matches!(Op::parse((1, 2)), None));
        assert!(matches!(
            Op::parse((4, 5)),
            Some(Op::BlockCleanOut(BlockCleanOut::FormatBlock(_)))
        ));
        assert!(matches!(
            Op::parse((4, 2)),
            Some(Op::BlockCleanOut(BlockCleanOut::PhysicalCleanOut(_)))
        ));
    }
}
