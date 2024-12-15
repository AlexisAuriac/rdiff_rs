use std::{
    error,
    fmt::{self, Display, Formatter},
    io,
};

use crate::op::OpKind;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    BadMagic {
        expect_name: String,
        expect_value: u32,
        got: u32,
    },
    BadSigType(u32),
    BadSigName(String),
    BadStrongLen(u32),
    UnexpectedCommand(OpKind),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "{}", err),
            Error::BadMagic {
                expect_name,
                expect_value,
                got,
            } => {
                write!(
                    f,
                    "bad magic number: expected {} ({:#x}), got {:#x}",
                    expect_name, expect_value, got
                )
            }
            Error::BadSigType(got) => write!(f, "bad signature type: {:#x}", got),
            Error::BadSigName(got) => write!(f, "bad signature name: {}", got),
            Error::BadStrongLen(got) => write!(f, "bad strong len: {}", got),
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
