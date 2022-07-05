#[macro_use]
use std::cell::RefCell;
use std::cell::RefMut;
use std::net::{TcpStream, UdpSocket};
use serde::{Deserialize, Serialize};
use crate::virtual_file_sys::disk::Disk;
use crate::error::{FSError, Result};
use super::block::*;
use super::inode::*;


pub struct FileSys {
    wrapped_file_sys: WrappedFileSys,
    curr_dir: usize,
}

impl FileSys {
    pub fn mount() -> Result<Self>{
        let mut file_sys = Self{
            wrapped_file_sys: WrappedFileSys::new()?,
            curr_dir: 1,
        };
        let home_dir = DirInode::retrieve(&file_sys.wrapped_file_sys, HOME_DIR_ID as usize)?;
        file_sys.set_working_dir(home_dir);
        Ok(file_sys)
    }

    pub fn unmount(self){
        self.wrapped_file_sys.into_inner().unmount()

    }

    pub fn mkdir(&self, name: [u8; MAX_FNAME_SIZE]) -> Result<()> {
        let mut working_dir = self.get_working_dir()?;
        self.validate_before_new_entry(&working_dir, name)?;
        let new_dir = DirInode::new(&self.wrapped_file_sys)?;
        let entry = DirEntry::new(name, new_dir.get_id());
        working_dir.add_dir_entry(&self.wrapped_file_sys,entry)?;

        Ok(())
    }

    pub fn cd(&mut self, name: [u8; MAX_FNAME_SIZE]) -> Result<()>{
        let mut working_dir = self.get_working_dir()?;
        return match working_dir.get_dir_inode_entries()
            .iter().find(|&e| e.get_name().eq(&name)) {
            Some(entry) => {
                self.set_working_dir(DirInode::retrieve(&self.wrapped_file_sys, entry.get_id())?);
                Ok(())
            },
            None => { Err(FSError::FileNotFoundError) }
        }

    }

    pub fn home(&mut self) -> Result<()>{
        self.set_working_dir(
            DirInode::retrieve(&self.wrapped_file_sys,HOME_DIR_ID as usize)?);
        Ok(())
    }

    pub fn rmdir(&self, name: [u8; MAX_FNAME_SIZE]) -> Result<()>{
        let mut working_dir = self.get_working_dir()?;
        match working_dir.get_dir_inode_entries()
            .iter().find(|&e| e.get_name().eq(&name)) {
            Some(entry) => {
                let dir = DirInode::retrieve(&self.wrapped_file_sys, entry.get_id())?;
                match dir.has_free_entry() {
                    true => {
                        working_dir.remove_dir_entry(&self.wrapped_file_sys, entry.get_id())?;
                    },
                    false => {return Err(FSError::DirNotEmptyError);}
                }
            },
            None => {}
        }
        Ok(())
    }

    pub fn ls(&self) -> Result<String> {
        let working_dir = self.get_working_dir()?;
        let mut names = Vec::new();
        for entry in working_dir.get_dir_inode_entries() {
            let mut name = std::str::from_utf8(&entry.get_name()).unwrap().to_owned();
            name.push('/');
            names.push(name);
        }
        for entry in working_dir.get_file_inode_entries() {
            names.push(std::str::from_utf8(&entry.get_name()).unwrap().to_owned());
        }
        let res = names.join(" ");
        Ok(res)

    }

    pub fn create(&self, name: [u8; MAX_FNAME_SIZE]) -> Result<()>{
        let mut working_dir = self.get_working_dir()?;
        self.validate_before_new_entry(&working_dir, name)?;
        let new_file = FileInode::new(&self.wrapped_file_sys)?;
        working_dir.add_file_entry(&self.wrapped_file_sys, DirEntry::new(name, new_file.get_id()))
    }

    pub fn append(&self, name:[u8; MAX_FNAME_SIZE], data:&[u8]) -> Result<()> {
        let mut working_dir = self.get_working_dir()?;
        if let Some(_) = working_dir.get_dir_inode_entries()
            .iter().find(|&e| e.get_name().eq(&name)) {
            return Err(FSError::NotAFileError);
        }
        match working_dir.get_file_inode_entries()
            .iter().find(|&e| e.get_name().eq(&name)) {
            Some(entry) => {
                let mut file = FileInode::retrieve(&self.wrapped_file_sys, entry.get_id())?;
                let new_total_size = data.len() + file.get_size();
                let new_total_blocks = new_total_size / BLOCK_SIZE;
                if new_total_blocks > MAX_DATA_BLOCKS {
                    return Err(FSError::FileFullError);
                }
                let frag_size = file.internal_flag_size();
                let mut pos = 0;
                let mut last_block_id = None;
                let mut last_block_data_backup = None;
                if frag_size > 0 {         
                    let last_block = file.get_blocks().last().unwrap();
                    last_block_id = Some(last_block.get_id());
                    last_block_data_backup = Some(last_block.get_data().clone());
                    let mut fragmented_block_data = last_block.get_raw();
                    pos = BLOCK_SIZE - frag_size;
                    match data.len() <= pos {
                        true => {fragmented_block_data.extend_from_slice(data); pos = data.len()},
                        false => {fragmented_block_data.extend_from_slice(&data[..pos]);}
                    }

                }
                if pos < data.len() {
                    let mut new_blocks = vec![];
                    for chunk in data[pos..].chunks(BLOCK_SIZE) {
                        match DataBlock::new(&self.wrapped_file_sys) {
                            Ok(new_block) => {
                                new_block.write_and_set_raw_block(&self.wrapped_file_sys, chunk.to_vec())?;
                                new_blocks.push(new_block);
                            },
                            Err(e) => {
                                if last_block_data_backup.is_some() {
                                    self.wrapped_file_sys.file_sys().write_block(last_block_id.unwrap(), &last_block_data_backup.unwrap())?;
                                }                
                                for new_block in new_blocks {
                                    self.wrapped_file_sys.file_sys().reclaim_block(new_block.get_id())?;
                                }
                                return Err(e)
                            },
                        }           
                    }
                    for new_block in new_blocks {
                        file.add_block(&self.wrapped_file_sys, new_block)?;
                    }
                }
                file.set_size(&self.wrapped_file_sys, new_total_blocks)?;
            },
            None => {return Err(FSError::FileNotFoundError);}
        }
        Ok(())
    }


    pub fn cat(&self, name: [u8; MAX_FNAME_SIZE]) -> Result<String> {
        let max_file_size = BLOCK_SIZE * BLOCK_SIZE;
        let mut working_dir = self.get_working_dir()?;
        if let Some(entry) = working_dir.get_dir_inode_entries()
            .iter().find(|&e| e.get_name().eq(&name)) {
            return Err(FSError::NotAFileError);
        }

        match  working_dir.get_file_inode_entries()
            .iter().find(|&e| e.get_name().eq(&name)) {
            Some(entry) => {
                let file = FileInode::retrieve(&self.wrapped_file_sys, entry.get_id())?;
                let mut content = String::new();
                for block in file.get_blocks().iter() {
                    content.push_str(std::str::from_utf8(&block.get_data()).unwrap());
                }
                return Ok(content);
            },
            None => {return Err(FSError::FileNotFoundError);}
        }
    }

    pub fn rm(&mut self, name: [u8; MAX_FNAME_SIZE]) -> Result<()> {
        let mut working_dir = self.get_working_dir()?;
        if let Some(entry) = working_dir.get_dir_inode_entries()
            .iter().find(|&e| e.get_name().eq(&name)) {
            return Err(FSError::NotAFileError);
        }
        match  working_dir.get_file_inode_entries()
            .iter().find(|&e| e.get_name().eq(&name)) {
            Some(entry) => {
                working_dir.remove_file_entry(&self.wrapped_file_sys, entry.get_id())?;
            },
            None => {}
        }
        Ok(())
    }

    pub fn set_working_dir(&mut self, dir: DirInode) {
        self.curr_dir = dir.get_id();
    }

    pub fn get_working_dir(&self) -> Result<DirInode> {
        DirInode::retrieve(&self.wrapped_file_sys, self.curr_dir)
    }

    pub fn validate_before_new_entry(&self, dir: &DirInode, name: [u8; MAX_FNAME_SIZE]) -> Result<()> {
        if !dir.has_free_entry() {
            return Err(FSError::DirFullError);
        }
        if dir.get_dir_inode_entries().iter().find(|&e| e.get_name().eq(&name)).is_some() {
            return Err(FSError::FileExistsError)
        }
        if dir.get_file_inode_entries().iter().find(|&e| e.get_name().eq(&name)).is_some() {
            return Err(FSError::FileExistsError)
        }
        Ok(())
    }
}


pub struct WrappedFileSys {
    bfs: RefCell<BasicFileSys>
}

impl WrappedFileSys {
    pub fn new() -> Result<Self> {
        Ok(Self{bfs:RefCell::new(BasicFileSys::mount()?)})
    }

    pub fn file_sys(&self) -> RefMut<BasicFileSys> {
        self.bfs.borrow_mut()
    }

    pub fn into_inner(self) -> BasicFileSys {
        self.bfs.into_inner()
    }
}

pub struct BasicFileSys {
    disk: Disk
}

impl BasicFileSys {
    pub fn mount() -> Result<Self>{
        let disk = Disk::mount("DISK")?;

        let mut super_block = vec![0u8; BLOCK_SIZE];
        super_block[0]=0x3;
        disk.write_block(0, &super_block)?;

        let dir_block = DirBlock::new();
       
        disk.write_block(1, &dir_block.to_bytes()?)?;
        let mut buf = vec![0u8;BLOCK_SIZE];
        disk.read_block(1, &mut buf);
        let dir = bincode::deserialize::<DirBlock>(&buf)?;
        let data_block = vec![0u8; BLOCK_SIZE];;
        for i in 2..NUM_BLOCKS {
            disk.write_block(i, &data_block);
        }
        Ok(Self{disk})
    }

    pub fn unmount(self) {
        self.disk.unmount();
    }

    pub fn get_free_block(&self) -> Option<usize>{
        let mut super_block = vec![0u8; BLOCK_SIZE];
        self.disk.read_block(0,&mut super_block);

        for byte in 0..BLOCK_SIZE {
            if super_block[byte] != 0xFF {
                for bit in 0..8 {
                    let mask = 1 << bit;
                    if mask & !super_block[byte] > 0 {
                        super_block[byte] |= mask;
                        self.disk.write_block(0, &super_block);
                        return Some(byte * 8 + bit);
                    }
                }
            }
        }
        None
    }

    pub fn reclaim_block(&self, block_num: usize) -> Result<()>{
        let mut super_block = vec![0u8; BLOCK_SIZE];
        self.disk.read_block(0, &mut super_block)?;

        let byte = block_num / 8;
        let bit = block_num % 8;
        let mask = !(1<<bit);
        super_block[byte] &= mask;
        self.disk.write_block(0, &super_block)?;
        Ok(())
    }

    pub fn read_block(&self, block_num: usize, block: &mut Vec<u8>) -> Result<()> {
        self.disk.read_block(block_num, block)
    }

    pub fn write_block(&self, block_num: usize, block: &Vec<u8>) -> Result<()> {
        self.disk.write_block(block_num, block)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test() -> Result<()> {
        Ok(())
    }
}