use std::io;
use std::io::{Read, Stdin, Stdout, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use crate::error::{Result, FSError};
use crate::command::{parse_from_string};
pub struct Shell{
    connection: TcpStream,
    stdin: Stdin,
    stdout: Stdout,
} 

impl Shell {
    pub fn connect<A: ToSocketAddrs>(address: A) -> Result<Self> {
        let mut stdout = io::stdout();
        let mut stdin = io::stdin();
        Ok(Self{
            connection: TcpStream::connect(address)?,
            stdout: io::stdout(),
            stdin: io::stdin(),
        })
    }

    pub fn run(mut self) -> Result<()> {
        loop {
            write!(self.stdout, ">>")?;
            let mut buf = String::new();
            self.stdin.read_line(&mut buf)?;
            match buf.as_str() {
                "--help" => {self.help()},
                "exit" => {break;},
                _ => {
                    match parse_from_string(buf){
                        Ok(cmd) => {
                            let bytes = bincode::serialize(&cmd)?;

                            self.connection.write_all(&bytes)?;
                            let mut res = vec![];
                            self.connection.read_exact(&mut res)?;
                            self.stdout.write_all(&res)?;
                        }
                        Err(e) => {self.stdout.write_all(b"Command error, use \"--help\" to see help.")?;}
                    }
                }
            }
        }
        self.connection.shutdown(Shutdown::Both)?;
        Ok(())
    }

    pub fn help(&self) {

    }



}

fn main() -> Result<()>{
    let shell = Shell::connect("127.0.0.1:6000")?;
    shell.run()?;
    Ok(())
}