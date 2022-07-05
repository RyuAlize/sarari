use std::cell::{RefCell, RefMut};
use serde::{Serialize, Deserialize};
use crate::virtual_file_sys::block::*;

use crate::error::{Result, FSError};
use crate::virtual_file_sys::file_sys::WrappedFileSys;

pub const UNUSED_ID: u8 = 0;
pub const HOME_DIR_ID: u8 = 1;

/// Inode - index node for a data file
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Inode {
    magic: usize,
    size: usize,
    blocks: Vec<u8>,
}

impl BlockBinary for Inode {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        let bytes = bincode::serialize(self)?;
        Ok(bytes)
    }

    fn from_bytes(block_data: &[u8]) -> Result<Self> {
        let block = bincode::deserialize(block_data)?;
        Ok(block)
    }
}

#[derive(Serialize, Deserialize)]
pub struct FileInode {
    id: usize,
    magic: usize,
    size: usize,
    raw: RefCell<Inode>,
    blocks: Vec<DataBlock>,
}

impl FileInode {
    pub fn new(wrapped_file_sys: &WrappedFileSys) -> Result<Self> {
        let res =  wrapped_file_sys.file_sys().get_free_block();
        return match res {
            Some(id) => {
                let tmp_raw = Inode {
                    magic: INODE_MAGIC_NUM,
                    size: 0,
                    blocks: vec![UNUSED_ID; MAX_DATA_BLOCKS]
                };
                let file_inode = Self {
                    id,
                    magic: INODE_MAGIC_NUM,
                    raw: RefCell::new(Inode::default()),
                    size: 0,
                    blocks: Vec::new(),
                };
                file_inode.write_and_set_raw_block(wrapped_file_sys, tmp_raw)?;
                Ok(file_inode)
            },
            None => { Err(FSError::DiskFullError) }
        }
    }

    pub fn retrieve(wrapped_file_sys: &WrappedFileSys, id: usize) -> Result<Self> {
        let mut block_data = vec![0u8; BLOCK_SIZE];
        wrapped_file_sys.file_sys().read_block(id, &mut block_data)?;
        let inode: Inode = bincode::deserialize(&block_data)?;
        if inode.magic != INODE_MAGIC_NUM {
            return Err(FSError::FileSysError);
        }
        let mut blocks = vec![];
        for i in 0..MAX_DATA_BLOCKS {
            let block_id = inode.blocks[i];
            if block_id != UNUSED_ID {
                blocks.push(DataBlock::retrieve(wrapped_file_sys,block_id as usize)?);
            }
        }
        Ok(Self{
            id,
            magic: inode.magic,
            size: inode.size,
            raw: RefCell::new(inode),
            blocks,
        })
    }

    pub fn write_and_set_raw_block(&self, wrapped_file_sys: &WrappedFileSys, tmp_raw: Inode) -> Result<()> {
        wrapped_file_sys.file_sys().write_block(self.id, &tmp_raw.to_bytes()?)?;
        self.raw.replace(tmp_raw);
        Ok(())
    }
    pub fn get_raw(&self) -> RefMut<Inode> {
        self.raw.borrow_mut()
    }
    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn get_blocks(&self) -> &Vec<DataBlock> {
        &self.blocks
    }

    pub fn add_block(&mut self, wrapped_file_sys: &WrappedFileSys, block: DataBlock) -> Result<()>{
        let mut tmp_raw = self.get_raw();
        for i in 0..MAX_DATA_BLOCKS {
            if tmp_raw.blocks[i] == UNUSED_ID {
                tmp_raw.blocks[i] = block.get_id() as u8;
                wrapped_file_sys.file_sys().write_block(self.id, &tmp_raw.to_bytes()?)?;
                return Ok(())
            }
        }
        Err(FSError::FileFullError)
    }

    pub fn remove_block(&mut self, wrapped_file_sys: &WrappedFileSys, block: &DataBlock) -> Result<()> {
        let mut tmp_raw = self.raw.borrow_mut();
        return match tmp_raw.blocks.iter().position(|&i| i as usize == block.get_id()) {
            Some(index) => {
                tmp_raw.blocks.remove(index);
                tmp_raw.blocks.push(UNUSED_ID);
                if let Some(index) = self.blocks.iter()
                    .position(|b|b.get_id() == block.get_id()) {
                    self.blocks.remove(index);
                }
                wrapped_file_sys.file_sys().write_block(self.id, &tmp_raw.to_bytes()?)?;
                Ok(())
            },
            None => {
                Err(FSError::FileSysError)
            }
        }
    }

    pub fn set_size(&mut self, wrapped_file_sys: &WrappedFileSys, size: usize) -> Result<()>{
        let mut tmp_raw = self.get_raw();
        tmp_raw.size = size;
        wrapped_file_sys.file_sys().write_block(self.id, &tmp_raw.to_bytes()?)?;
        Ok(())
    }

    #[inline]
    pub fn has_free_block(&self) -> bool {
        MAX_DATA_BLOCKS - self.blocks.len() > 0
    }

    pub fn internal_flag_size(&self) -> usize {
        self.size % BLOCK_SIZE
    }

    pub fn destroy(self, wrapped_file_sys: &WrappedFileSys) {
        wrapped_file_sys.file_sys().reclaim_block(self.id);
    }

    pub fn get_id(&self) -> usize {
        self.id
    }
}

#[derive(Default,Serialize, Deserialize)]
pub struct DirInode {
    id: usize,
    magic: usize,
    num_entries: usize,
    raw:RefCell<DirBlock>,
    file_entries: Vec<DirEntry>,
    dir_entries: Vec<DirEntry>,
}

impl DirInode {
    pub fn new(wrapped_file_sys: &WrappedFileSys) -> Result<Self> {
        let res = wrapped_file_sys.file_sys().get_free_block();
        return match res {
            Some(id) => {
                let tmp_raw = DirBlock::new();
                wrapped_file_sys.file_sys().write_block(id, &tmp_raw.to_bytes()?)?;
                let dir_inode = Self {
                    id,
                    magic: DIR_MAGIC_NUM,
                    num_entries: 0,
                    raw: RefCell::new(tmp_raw),
                    file_entries: Vec::new(),
                    dir_entries: Vec::new(),
                };
                Ok(dir_inode)
            },
            None => { Err(FSError::DiskFullError) }
        }
    }

    pub fn retrieve(wrapped_file_sys: &WrappedFileSys, id: usize) -> Result<Self> {
        let mut block_data = vec![0u8; BLOCK_SIZE];
        wrapped_file_sys.file_sys().read_block(id, &mut block_data)?;
        let dir_block: DirBlock = bincode::deserialize(&block_data)?;
        if dir_block.magic != DIR_MAGIC_NUM {
            return Err(FSError::FileSysError);
        }
        let mut dir_node = DirInode::default();
        for i in 0..MAX_DIR_ENTRIES {
            let name = dir_block.dir_entries[i].name;
            let block_id = dir_block.dir_entries[i].block_num;
            if block_id != UNUSED_ID as usize{
                let mut block_data = vec![0u8; BLOCK_SIZE];
                wrapped_file_sys.file_sys().read_block(block_id, &mut block_data)?;
                if let Ok(entry) = bincode::deserialize::<Inode>(&block_data) {
                    if entry.magic == INODE_MAGIC_NUM {
                        dir_node.file_entries.push(DirEntry::new(name, block_id));
                    }
                }
                if let Ok(entry) = bincode::deserialize::<DirBlock>(&block_data) {
                    if entry.magic == DIR_MAGIC_NUM{
                         dir_node.dir_entries.push(DirEntry::new(name, block_id));
                    }
                }
            }
        }
        dir_node.id = id;
        dir_node.magic = dir_block.magic;
        dir_node.num_entries = dir_block.num_entries;
        dir_node.raw.replace(dir_block);
        Ok(dir_node)
    }

    pub fn write_and_set_raw_block(&self, wrapped_file_sys: &WrappedFileSys, tmp_raw: DirBlock) -> Result<()> {
        wrapped_file_sys.file_sys().write_block(self.id, &tmp_raw.to_bytes()?)?;
        self.raw.replace(tmp_raw);
        Ok(())
    }

    pub fn get_num_entries(&self) -> usize {
        self.num_entries
    }

    pub fn get_file_inode_entries(&self) -> &Vec<DirEntry> {
        &self.file_entries
    }

    pub fn get_dir_inode_entries(&self) -> &Vec<DirEntry> {
        &self.dir_entries
    }

    pub fn add_file_entry(&mut self, wrapped_file_sys: &WrappedFileSys, entry: DirEntry) -> Result<()>{
        let mut tmp_raw = self.raw.borrow_mut();
        for i in 0..MAX_DIR_ENTRIES {
            if tmp_raw.dir_entries[i].block_num == UNUSED_ID as usize{
                tmp_raw.dir_entries[i].block_num = entry.get_id();
                tmp_raw.dir_entries[i].name = entry.get_name();
                tmp_raw.num_entries += 1;
                self.num_entries = tmp_raw.num_entries;
                self.file_entries.push(entry);
                wrapped_file_sys.file_sys().write_block(self.id, &tmp_raw.to_bytes()?)?;
                return Ok(())
            }
        }
        Err(FSError::DirFullError)
    }

    pub fn add_dir_entry(&mut self, wrapped_file_sys: &WrappedFileSys, entry: DirEntry) -> Result<()>{
        let mut tmp_raw = self.raw.borrow_mut();
        for i in 0..MAX_DIR_ENTRIES {
            if tmp_raw.dir_entries[i].block_num == UNUSED_ID as usize{
                tmp_raw.dir_entries[i].block_num = entry.get_id();
                tmp_raw.dir_entries[i].name = entry.get_name();
                tmp_raw.num_entries += 1;
                self.num_entries = tmp_raw.num_entries;
                self.dir_entries.push(entry);
                wrapped_file_sys.file_sys().write_block(self.id, &tmp_raw.to_bytes()?)?;
                return Ok(())
            }
        }
        Err(FSError::DirFullError)
    }

    pub fn remove_file_entry(&mut self, wrapped_file_sys: &WrappedFileSys, block_id: usize) -> Result<()> {
        let mut tmp_raw = self.raw.borrow_mut();
        for i in 0..MAX_DIR_ENTRIES {
            if tmp_raw.dir_entries[i].block_num == block_id {
                tmp_raw.num_entries -= 1;
                self.num_entries = tmp_raw.num_entries;
                wrapped_file_sys.file_sys().write_block(self.id, &tmp_raw.to_bytes()?)?;
                if let Some(index) = self.file_entries.iter().position(|e| e.get_id() == block_id) {
                    self.file_entries.remove(index);
                }
            }
        }
        Ok(())
    }

    pub fn remove_dir_entry(&mut self, wrapped_file_sys: &WrappedFileSys, block_id: usize) -> Result<()> {
        let mut tmp_raw = self.raw.borrow_mut();
        for i in 0..MAX_DIR_ENTRIES {
            if tmp_raw.dir_entries[i].block_num == block_id {
                tmp_raw.num_entries -= 1;
                self.num_entries = tmp_raw.num_entries;
                wrapped_file_sys.file_sys().write_block(self.id, &tmp_raw.to_bytes()?)?;
                if let Some(index) = self.dir_entries.iter().position(|e| e.get_id() == block_id) {
                    self.dir_entries.remove(index);
                }
            }
        }
        Ok(())
    }

    pub fn has_free_entry(&self) -> bool {
        MAX_DIR_ENTRIES - self.num_entries > 0
    }

    pub fn destroy(self, wrapped_file_sys: &WrappedFileSys) {
        wrapped_file_sys.file_sys().reclaim_block(self.id);
    }

    pub fn get_id(&self) -> usize {
        self.id
    }
}

#[derive(Serialize, Deserialize)]
pub struct DirEntry {
    name: [u8; MAX_FNAME_SIZE],
    inode_id: usize,

}

impl DirEntry {

    pub fn new(name: [u8; MAX_FNAME_SIZE], inode_id: usize) -> Self {
        Self{
            name,
            inode_id
        }
    }

    #[inline]
    pub fn get_name(&self) -> [u8; MAX_FNAME_SIZE] {
        self.name
    }

    #[inline]
    pub fn get_id(&self) -> usize {
        self.inode_id
    }

}

/// Directory block - represents a directory
#[derive(Debug, Default,Serialize, Deserialize)]
pub struct DirBlock {
    magic: usize,
    num_entries: usize,
    dir_entries: Vec<Entry>
}

impl BlockBinary for DirBlock {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        let bytes = bincode::serialize(self)?;
        Ok(bytes)
    }

    fn from_bytes(block_data: &[u8]) -> Result<Self> {
        let block = bincode::deserialize(block_data)?;
        Ok(block)
    }
}

impl DirBlock {
    pub fn new() -> Self {
        Self{
            magic: DIR_MAGIC_NUM,
            num_entries: 0,
            dir_entries: vec![Entry::default(); MAX_DIR_ENTRIES]
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct Entry {
    name: [u8; MAX_FNAME_SIZE],
    block_num: usize,
}
