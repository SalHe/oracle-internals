use std::{
    cmp::min,
    fs::File,
    io::{ErrorKind, Read, Seek},
    path::Path,
};

use thiserror::Error;
use tracing::trace;
use zerocopy::{CastError, FromBytes};

use crate::oracle::{
    change::ChangeAndVector,
    checksum::calc_checksum,
    file::{FileHeader, FileHeaderBlock},
    redo::{
        Change, RedoBlockHead, RedoFileHead, RedoRecordHeadCommon, RedoRecordHeadV1,
        RedoRecordHeadV2, VldFlags,
    },
};

#[derive(Debug, Clone, Copy)]
pub enum Record {
    V1(RedoRecordHeadV1),
    V2(RedoRecordHeadV2),
}

#[derive(Debug)]
pub struct RBA {
    pub seq: u16,
    pub block_id: u32,
    pub offset: u16,
}

#[derive(Error, Debug)]
pub enum ReaderError {
    #[error("current record is being reading")]
    AlreadyInRecord,
    #[error("cannot read change when no record being read")]
    NotReadingRecord,
    #[error(
        "invalid checksum for block#{block_id}, expected 0x{expected:04x}, actual 0x{actual:04x}"
    )]
    InvalidChecksum {
        block_id: u32,
        expected: u16,
        actual: u16,
    },
    #[error("Magic number of file header is not correct")]
    InvalidFileMagicNumber,
    #[error("Magic number of redo block is not correct")]
    InvalidRedoBlockMagicNumber,
    #[error("read to EOF")]
    Eof,
    #[error("io error: {0:?}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Default)]
pub struct ReadOptions {
    pub ignore_checksum: bool,
}

pub struct RedoFileReader {
    file: File,

    /// FileHeader block
    block0: Vec<u8>,

    /// RedoFileHeader block
    block1: Vec<u8>,

    curr_block: Vec<u8>,
    in_record: bool,
    offset_in_block: usize,
    content_offset: usize,
    curr_record: Option<Record>,
    options: ReadOptions,
}

impl RedoFileReader {
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            file,
            block0: Default::default(),
            block1: Default::default(),
            curr_block: Default::default(),
            in_record: false,
            offset_in_block: 0,
            content_offset: 0,
            curr_record: None,
            options: Default::default(),
        })
    }

    /// 重新读取文件头，文件读取指针将被重置
    pub fn reload_header(&mut self) -> Result<(), ReaderError> {
        self.read_file_header()?;
        self.read_redo_file_header()?;
        self.next_redo_block()?;
        Ok(())
    }

    pub fn file_header(&self) -> &FileHeaderBlock {
        FileHeaderBlock::ref_from_bytes(&self.block0).unwrap()
    }

    pub fn redo_file_header(&self) -> &RedoFileHead {
        RedoFileHead::ref_from_bytes(&self.block1).unwrap()
    }

    pub fn curr_block_head(&self) -> &RedoBlockHead {
        RedoBlockHead::ref_from_bytes(&self.curr_block[0..std::mem::size_of::<RedoBlockHead>()])
            .unwrap()
    }

    pub fn set_options(&mut self) -> &mut ReadOptions {
        &mut self.options
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn read_file_header(&mut self) -> Result<(), ReaderError> {
        // 确认block大小
        self.file.seek(std::io::SeekFrom::Start(0))?;
        let mut buffer = [0u8; std::mem::size_of::<FileHeader>()];
        self.file.read_exact(&mut buffer)?;

        let header = FileHeader::ref_from_bytes(&buffer).unwrap();
        trace!(header.block_size, header.block_count, "file header is read",);

        trace!(
            expected = FileHeader::MAGIC_NUMBER,
            actual = header.magic_number,
            "magic number"
        );
        if header.magic_number != FileHeader::MAGIC_NUMBER {
            return Err(ReaderError::InvalidFileMagicNumber);
        }

        // 实际读取 block0
        self.file.seek(std::io::SeekFrom::Start(0))?;
        let mut block0 = vec![0u8; header.block_size as usize];
        self.file.read_exact(&mut block0)?;
        self.block0 = block0;
        Ok(())
    }

    fn read_redo_file_header(&mut self) -> Result<(), ReaderError> {
        self.block1 = self.read_block()?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn read_block(&mut self) -> Result<Vec<u8>, ReaderError> {
        let mut buffer = vec![0u8; self.file_header().header.block_size as usize];
        if let Err(err) = self.file.read_exact(&mut buffer)
            && err.kind() == ErrorKind::UnexpectedEof
        {
            return Err(ReaderError::Eof);
        };

        let (calc_checksum, expected_checksum) = calc_checksum(&mut buffer).unwrap();
        let (head, _) = RedoBlockHead::ref_from_prefix(&buffer).unwrap();
        trace!(
            head.id,
            calc_checksum,
            expected_checksum,
            matched = calc_checksum == expected_checksum
        );

        if calc_checksum != expected_checksum && !self.options.ignore_checksum {
            return Err(ReaderError::InvalidChecksum {
                block_id: head.id,
                expected: expected_checksum,
                actual: calc_checksum,
            });
        }

        if head.magic_number != RedoBlockHead::MAGIC_NUMBER {
            return Err(ReaderError::InvalidRedoBlockMagicNumber);
        }

        Ok(buffer)
    }

    pub(crate) fn next_redo_block(&mut self) -> Result<(), ReaderError> {
        self.curr_block = self.read_block()?;
        self.offset_in_block = std::mem::size_of::<RedoBlockHead>();
        Ok(())
    }

    pub fn calc_curr_block_checksum(
        &mut self,
    ) -> Result<(u16, u16), CastError<&mut [u8], RedoBlockHead>> {
        calc_checksum(&mut self.curr_block)
    }

    #[tracing::instrument(skip(self))]
    pub fn begin_read_record(&mut self) -> Result<Option<(RBA, Record)>, ReaderError> {
        if self.in_record {
            return Err(ReaderError::AlreadyInRecord);
        }

        self.in_record = true;

        let min_size = min(
            std::mem::size_of::<RedoRecordHeadV1>(),
            std::mem::size_of::<RedoRecordHeadV2>(),
        );
        if self.file_header().header.block_size as usize - self.offset_in_block < min_size {
            self.next_redo_block()?;
        }

        loop {
            let record = RedoRecordHeadCommon::ref_from_bytes(
                &self.curr_block[self.offset_in_block
                    ..(self.offset_in_block + std::mem::size_of::<RedoRecordHeadCommon>())],
            )
            .unwrap();
            let block_head = self.curr_block_head();
            let rba = RBA {
                seq: block_head.seq,
                block_id: block_head.id,
                offset: self.offset_in_block as _,
            };

            let flags = VldFlags::from_bits(record.vld).unwrap();
            trace!(?flags);
            let vld = flags;
            if vld.contains(VldFlags::KCRVOID) {
                let v2 = RedoRecordHeadV2::ref_from_bytes(
                    &self.curr_block[self.offset_in_block
                        ..(self.offset_in_block + std::mem::size_of::<RedoRecordHeadV2>())],
                )
                .unwrap();
                self.offset_in_block += std::mem::size_of::<RedoRecordHeadV2>();
                self.content_offset = std::mem::size_of::<RedoRecordHeadV2>();
                self.curr_record = Some(Record::V2(*v2));
            } else {
                let v1 = RedoRecordHeadV1::ref_from_bytes(
                    &self.curr_block[self.offset_in_block
                        ..(self.offset_in_block + std::mem::size_of::<RedoRecordHeadV1>())],
                )
                .unwrap();
                self.offset_in_block += std::mem::size_of::<RedoRecordHeadV1>();
                self.content_offset = std::mem::size_of::<RedoRecordHeadV1>();
                self.curr_record = Some(Record::V1(*v1));
            }

            if record.len == 0 {
                trace!("current record len is 0, skip and read next block");
                self.next_redo_block()?;
                continue;
            }

            return Ok(self.curr_record.map(|r| (rba, r)));
        }
    }

    #[tracing::instrument(skip(self, target))]
    pub fn read_content(&mut self, target: &mut [u8]) -> Result<(), ReaderError> {
        self.curr_record.ok_or(ReaderError::NotReadingRecord)?;

        let expected_size = target.len();
        let block_crossed =
            self.offset_in_block + expected_size >= self.file_header().header.block_size as usize;
        trace!(expected_size, block_crossed, "begin read data");

        if block_crossed {
            let mut rest = expected_size;
            while rest > 0 {
                let can_read = min(
                    self.file_header().header.block_size as usize - self.offset_in_block,
                    rest,
                );
                trace!(
                    self.content_offset,
                    self.offset_in_block, can_read, "read slice of data"
                );

                target[(expected_size - rest)..(expected_size - rest + can_read)].copy_from_slice(
                    &self.curr_block[self.offset_in_block..(self.offset_in_block + can_read)],
                );
                rest -= can_read;
                self.offset_in_block += can_read;

                if self.offset_in_block == self.file_header().header.block_size as usize {
                    self.next_redo_block()?;
                }
            }
        } else {
            trace!(
                self.content_offset,
                self.offset_in_block,
                can_read = expected_size,
                "read all data"
            );
            target.copy_from_slice(
                &self.curr_block[self.offset_in_block..(self.offset_in_block + expected_size)],
            );
            self.offset_in_block += expected_size;
        }

        self.content_offset += expected_size;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn read_change(&mut self) -> Result<Option<ChangeAndVector>, ReaderError> {
        if !self.in_record {
            return Err(ReaderError::NotReadingRecord);
        }
        let record = self.curr_record.ok_or(ReaderError::NotReadingRecord)?;
        let len = match record {
            Record::V1(RedoRecordHeadV1 { common, .. }) => common.len,
            Record::V2(RedoRecordHeadV2 { common, .. }) => common.len,
        };

        if self.content_offset >= len as usize {
            trace!(self.content_offset, len, "no more changes");
            self.in_record = false;
            return Ok(None);
        }

        trace!("reading change");
        let mut change_buffer = vec![0u8; std::mem::size_of::<Change>()];
        self.read_content(&mut change_buffer)?;

        let change = *Change::ref_from_bytes(&change_buffer).unwrap();
        trace!(?change, "change read");

        let mut vector_size = Vec::with_capacity(change.vector_count() as usize);
        if change.vector_count() > 0 {
            vector_size.push(change.first_vector_size);
        }

        for i in 1..change.vector_count() {
            trace!(vector_id = i, "reading vector data size");
            let mut buffer = [0u8; 2];
            self.read_content(&mut buffer)?;
            vector_size.push(u16::from_le_bytes(buffer));
        }
        trace!(?vector_size);

        if change.need_padding() {
            self.content_offset += 2;
            self.offset_in_block += 2;
            trace!("padding 2 bytes for change list");
            trace!(self.content_offset, self.offset_in_block);
        }

        let mut vector_data = Vec::with_capacity(change.vector_count() as usize);
        for (i, size) in vector_size.iter().enumerate() {
            trace!(vector_id = i, "reading vector data content");
            let mut buffer = vec![0u8; padding_size(*size as u64, 4) as usize];
            self.read_content(&mut buffer)?;
            vector_data.push(buffer);
        }

        Ok(Some(ChangeAndVector {
            change,
            vector: vector_data,
        }))
    }
}

fn padding_size(size: u64, alignment: u64) -> u64 {
    if size.is_multiple_of(alignment) {
        size
    } else {
        size + alignment - (size % alignment)
    }
}
