use std::net::TcpStream;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum FSError {
    #[error("IO error")]
    IOError(#[from] std::io::Error),

    #[error("Command parse error")]
    CmdParseError,

    #[error("Block seek failure")]
    SeekFailure,

    #[error("{}", 0)]
    BlockError(String),

    #[error("Serialize error")]
    SerializeError(#[from] Box<bincode::ErrorKind>),

    #[error("A File System error has occured")]
    FileSysError,

    #[error("500 File is not a directory")]
    NotDirError,

    #[error("501 File is a directory")]
    NotAFileError,

    #[error("502 File exists")]
    FileExistsError,

    #[error("503 File does not exist")]
    FileNotFoundError,

    #[error("504 File name is too long")]
    FileNameTooLongError,

    #[error("505 Disk is full")]
    DiskFullError,

    #[error("506 Directory is full")]
    DirFullError,

    #[error("507 Directory is not empty")]
    DirNotEmptyError,

    #[error("508 Append exceeds maximum file size")]
    FileFullError,

}

pub type Result<T> = std::result::Result<T, FSError>;

