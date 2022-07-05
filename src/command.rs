use serde::{Serialize, Deserialize};
use crate::error::{Result, FSError};
#[derive(Serialize, Deserialize)]
pub enum CommandType {
    mkdir,
    ls,
    cd,
    home,
    rmdir,
    create,
    append,
    cat,
    rm,
}

#[derive(Serialize, Deserialize)]
pub struct Command {
    pub cmd: CommandType,
    pub parameter: Vec<u8>,
}

pub fn parse_from_string(buf: String) -> Result<Command> {
    let mut cmd;
    let mut parameter;
    let mut bytes = buf.into_bytes();
    if bytes.starts_with(b"mkdir ") {
        cmd = CommandType::mkdir;
        bytes.drain(0.."mkdir ".len());
        parameter = bytes;
        return Ok(Command{ cmd,parameter});

    }
    else if bytes.starts_with(b"ls") {
        cmd = CommandType::ls;
        bytes.drain(0.."ls ".len());
        parameter = bytes;
        return Ok(Command{ cmd,parameter});
    }
    else if bytes.starts_with(b"cd ") {
        cmd = CommandType::cd;
        bytes.drain(0.."cd ".len());
        parameter = bytes;
        return Ok(Command{ cmd,parameter});
    }
    else if bytes.starts_with(b"home") {
        cmd = CommandType::home;
        bytes.drain(0.."home".len());
        parameter = bytes;
        return Ok(Command{ cmd,parameter});
    }
    else if bytes.starts_with(b"rmdir ") {
        cmd = CommandType::rmdir;
        bytes.drain(0.."rmdir ".len());
        parameter = bytes;
        return Ok(Command{ cmd,parameter});
    }
    else if bytes.starts_with(b"create ") {
        cmd = CommandType::create;
        bytes.drain(0.."create ".len());
        parameter = bytes;
        return Ok(Command{ cmd,parameter});
    }
    else if bytes.starts_with(b"rm ") {
        cmd = CommandType::rm;
        bytes.drain(0.."rm ".len());
        parameter = bytes;
        return Ok(Command{ cmd,parameter});
    }

    Err(FSError::CmdParseError)
}