use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::thread::spawn;
use crate::command::{Command, CommandType};
use crate::virtual_file_sys::{file_sys::FileSys, MAX_FNAME_SIZE};
use crate::error::{FSError, Result};

pub struct NFServer {
    fs: RefCell<FileSys>,
    connection: TcpListener,
}

impl NFServer {
    pub fn bind<A: ToSocketAddrs>(address: A) -> Result<Self>{
        Ok(Self{
            fs: RefCell::new(FileSys::mount()?),
            connection: TcpListener::bind(address)?
        })
    }

    pub fn run(&self) -> Result<()>{
        for stream in self.connection.incoming() {
            match stream {
                Ok(stream) => {self.handle_stream(stream)?;}
                Err(_) => {}
            }
        }
        Ok(())
    }

    pub fn handle_stream(&self, mut stream: TcpStream) -> Result<()> {
        while let Ok(cmd) = bincode::deserialize_from::<&TcpStream, Command>(&stream) {
            let mut response = String::new();
            match cmd.cmd {
                CommandType::create => {
                    if cmd.parameter.len() > MAX_FNAME_SIZE || cmd.parameter.len() == 0 {
                        response = "Invalid file name size.".to_owned();
                    }
                    else {
                        let mut name = [0u8; MAX_FNAME_SIZE];
                        for (i, ch) in cmd.parameter.iter().enumerate() {name[i] = ch.to_owned()};
                        match self.fs.borrow().create(name) {
                            Ok(res) => {},
                            Err(FSError::DirFullError) => {response = "Directory is full.".to_owned()},
                            Err(FSError::FileExistsError) => {response = "File is allour already exist.".to_owned()},
                            _ => unreachable!()
                        }
                    }
                },
                CommandType::ls => {
                    match self.fs.borrow().ls() {
                        Ok(res) => {response = res;},
                        Err(_) => {response = "File system error.".to_owned();}
                    }
                },
                CommandType::cd => {
                    if cmd.parameter.len() > MAX_FNAME_SIZE || cmd.parameter.len() == 0 {
                        response = "Invalid file name size.".to_owned();
                    }
                    else {
                        let mut name = [0u8; MAX_FNAME_SIZE];
                        for (i, ch) in cmd.parameter.iter().enumerate() {name[i] = ch.to_owned()};
                        match self.fs.borrow_mut().cd(name) {
                            Ok(_) => {},
                            Err(FSError::FileNotFoundError) => {response = "Directory not find.".to_owned();},
                            Err(_) => {response = "File system error.".to_owned();}
                        }
                    }
                },
                CommandType::home => {},
                CommandType::append => {},
                CommandType::cat => {},
                CommandType::rm => {},
                _ =>{response = "Error command.".to_owned();}
            }
            stream.write_all(response.as_bytes())?;
        }
        Ok(())
    }

}

fn main() ->Result<()>{
    let nfs = NFServer::bind("127.0.0.1:6000")?;
    nfs.run();

    Ok(())
}