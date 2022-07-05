use std::cell::{RefCell, RefMut, Ref};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::error::{FSError, Result};
use crate::virtual_file_sys::file_sys::WrappedFileSys;

pub const BLOCK_SIZE: usize = 1024;
pub const NUM_BLOCKS: usize = BLOCK_SIZE * 8;
pub const MAX_FNAME_SIZE: usize = 9;
pub const MAX_DIR_ENTRIES: usize = (BLOCK_SIZE - 8) / 32;
pub const MAX_DATA_BLOCKS: usize = (BLOCK_SIZE - 8) / 4;
pub const MAX_FILE_SIZE: usize	= MAX_DATA_BLOCKS * BLOCK_SIZE;

pub const DIR_MAGIC_NUM: usize = 0xFFFFFFFF;
pub const INODE_MAGIC_NUM: usize = 0xFFFFFFFE;

#[derive(Serialize, Deserialize)]
pub struct DataBlock {
    id: usize,
    raw: RefCell<Vec<u8>>
}

impl DataBlock {
    pub fn new(wrapped_file_sys: &WrappedFileSys) -> Result<Self> {
        match wrapped_file_sys.file_sys().get_free_block() {
            Some(id) => {
                return Ok(Self {
                    id,
                    raw: RefCell::new(vec![0u8; BLOCK_SIZE]),
                });
            },
            None => { return Err(FSError::DiskFullError); }
        }
    }

    pub fn retrieve(wrapped_file_sys: &WrappedFileSys, id: usize) -> Result<Self> {
        let mut block_data = vec![0u8; BLOCK_SIZE];
        wrapped_file_sys.file_sys().read_block(id, &mut block_data)?;
        Ok(Self {
            id,
            raw: RefCell::new(block_data)
        })
    }

    pub fn destroy(self, wrapped_file_sys: &WrappedFileSys) {
        wrapped_file_sys.file_sys().reclaim_block(self.id);
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn get_raw(&self) -> RefMut<Vec<u8>> {
        self.raw.borrow_mut()
    }

    pub fn get_data(&self) ->Ref<Vec<u8>> {
        self.raw.borrow()
    }

    pub fn write_and_set_raw_block(&self, wrapped_file_sys: &WrappedFileSys, tmp_raw: Vec<u8>) -> Result<()> {
        wrapped_file_sys.file_sys().write_block(self.id, &tmp_raw)?;
        self.raw.replace(tmp_raw);
        Ok(())
    }
}


pub trait BlockBinary {
    fn to_bytes(&self) -> Result<Vec<u8>>;

    fn from_bytes(block_data: &[u8]) -> Result<Self> where Self: Sized;
}








