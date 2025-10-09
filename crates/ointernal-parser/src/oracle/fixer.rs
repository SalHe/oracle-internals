use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
    path::Path,
};

use crate::oracle::{
    reader::{ReaderError, RedoFileReader},
    redo::RedoBlockHead,
};

pub struct RedoFixer {
    reader: RedoFileReader,
    block_size: u32,
    block_count: u32,
    block_id: u32,
    initialized: bool,
}

impl RedoFixer {
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut reader = RedoFileReader::from_file(path)?;
        reader.set_options().ignore_checksum = true;

        Ok(Self {
            reader,
            initialized: false,
            block_size: 0,
            block_count: 0,
            block_id: 0,
        })
    }

    pub fn next_block(&mut self) -> Result<Option<BlockStatus>, ReaderError> {
        if !self.initialized {
            self.initialized = true;
            self.reader.read_file_header()?;

            let header = &self.reader.file_header().header;
            self.block_size = header.block_size;
            self.block_count = header.block_count;
            self.block_id = 0;
        }

        if self.block_id >= self.block_count {
            return Ok(None);
        }

        self.block_id += 1;
        self.reader.next_redo_block()?;

        let mut operations = Vec::new();
        let (actual, expected) = self.reader.calc_curr_block_checksum().unwrap();
        if actual != expected {
            operations.push(FixOperation::FixCheckSum { actual, expected });
        }

        Ok(Some(BlockStatus {
            block_id: self.block_id,
            operations,
        }))
    }

    pub fn block_size(&self) -> Option<u32> {
        if self.initialized {
            Some(self.block_size)
        } else {
            None
        }
    }
}

pub enum FixOperation {
    FixCheckSum { actual: u16, expected: u16 },
}

impl FixOperation {
    pub fn fix_it(&self, f: &mut File, block_id: u32, block_size: u32) -> std::io::Result<()> {
        match self {
            FixOperation::FixCheckSum { actual, .. } => {
                f.seek(SeekFrom::Start(
                    ((block_id * block_size) as usize
                        + std::mem::offset_of!(RedoBlockHead, checksum)) as u64,
                ))?;
                f.write_all(&actual.to_ne_bytes())?;
            }
        }
        Ok(())
    }
}

pub struct BlockStatus {
    pub block_id: u32,
    pub operations: Vec<FixOperation>,
}

impl BlockStatus {
    pub fn passed(&self) -> bool {
        self.operations.is_empty()
    }
}
