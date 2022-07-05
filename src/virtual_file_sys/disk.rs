use std::cell::RefCell;
use std::path::Path;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use crate::error::{FSError, Result};
use crate::virtual_file_sys::block::*;
#[derive()]
pub struct Disk {
    fd: RefCell<File>
}

impl Disk {
    pub fn mount<P: AsRef<Path>>(filename: P) -> Result<Self> {
        let fd = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(filename)?;

        Ok(Self{fd:RefCell::new(fd)})
    }

    pub fn unmount(self) {
        drop(self);
    }

    pub fn read_block(&self, block_num: usize, block: &mut Vec<u8>) -> Result<()>{
        if block_num >= NUM_BLOCKS {
            return Err(FSError::BlockError("Invalid block size".to_owned()));
        }
        let offset = block_num * BLOCK_SIZE;
        let new_offset = self.fd.borrow_mut()
                                .seek(SeekFrom::Start(offset as u64))?;
        if offset != new_offset as usize {
            return Err(FSError::SeekFailure);
        }

        self.fd.borrow_mut().read_exact(block)?;
        Ok(())
    }

    pub fn write_block(&self, block_num: usize, block: &Vec<u8>) -> Result<()>{
        if block_num >= NUM_BLOCKS {
            return Err(FSError::BlockError("Invalid block size".to_owned()));
        }
        let offset = block_num * BLOCK_SIZE;
        let new_offset = self.fd.borrow_mut()
                                .seek(SeekFrom::Start(offset as u64))?;
        if offset != new_offset as usize {
            return Err(FSError::SeekFailure);
        }
        self.fd.borrow_mut().write_all(block)?;
        Ok(())
    }
}

#[cfg(test)]
mod test{

    use super::*;
    #[test]
    fn test() -> Result<()>{
        let disk = Disk::mount("Disk")?;
        Ok(())
    }
}