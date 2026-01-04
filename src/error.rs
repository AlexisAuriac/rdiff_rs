use std::{
    error,
    fmt::{self, Display, Formatter},
    io,
};

use crate::op::OpKind;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    BadSigType(u32),
    BadHashName(String),
    BadRollsumName(String),
    ZeroBlockLen,
    BadStrongLen(u32),
    BadDeltaMagic(u32),
    UnexpectedCommand(OpKind),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "{}", err),
            Error::BadSigType(got) => write!(f, "bad signature type: {:#x}", got),
            Error::BadHashName(got) => write!(f, "bad hash name: {}", got),
            Error::BadRollsumName(got) => write!(f, "bad rollsum name: {}", got),
            Error::ZeroBlockLen => write!(f, "block len is 0"),
            Error::BadStrongLen(got) => write!(f, "bad strong len: {}", got),
            Error::BadDeltaMagic(magic) => write!(f, "bad delta magic: {magic:x}"),
            Error::UnexpectedCommand(kind) => write!(f, "unexpected command: {:?}", kind),
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}
