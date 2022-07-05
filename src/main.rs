

mod server;
mod error;
mod virtual_file_sys;
mod client;
mod command;

use virtual_file_sys::file_sys::FileSys;
use error::{Result};
fn main() ->Result<()>{

    let mut fs = FileSys::mount()?;
    fs.create(b"file00001".to_owned())?;
    fs.create(b"file00002".to_owned())?;
    fs.create(b"file00003".to_owned())?;
    fs.create(b"file00004".to_owned())?;
    fs.mkdir(b"dir000001".to_owned())?;
    fs.mkdir(b"dir000002".to_owned())?;
    fs.mkdir(b"dir000003".to_owned())?;
    let res = fs.ls()?;
    println!("{res}");
    fs.append(b"file00001".to_owned(), &vec![b'h',b'e',b'l',b'l',b'o',b'\n',b'w',b'o',b'r',b'l',b'd'])?;
    let res = fs.cat(b"file00001".to_owned())?;
    println!("{res}");
    fs.cd(b"dir000001".to_owned())?;
    fs.create(b"file00011".to_owned())?;
    fs.create(b"file00022".to_owned())?;
    fs.create(b"file00033".to_owned())?;
    fs.create(b"file00044".to_owned())?;
    fs.mkdir(b"dir000011".to_owned())?;
    fs.mkdir(b"dir000022".to_owned())?;
    fs.mkdir(b"dir000033".to_owned())?;
    let res = fs.ls()?;
    
    print!("{res}");
    Ok(())
}
