use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
};

/*=
| A well-designed communications protocol has a number of characteristics.
|
| - Everything is sent in well defined packets with a header and an optional body or data payload.
| - In each packet's header a type and or command specified.
| - Each packet has a definite length.
|
| Rsync's protocol has none of these good characteristics.
*/

use anyhow::{anyhow, Error};

// trait Wire {
// 	fn read_file_name()
// 	fn ()
// }

trait ToBytes<W> {
    fn to_bytes(&self) -> &[u8];
}

struct Server {
    tcp: TcpListener,
}

struct RsyncTcpServerConn {
    stream: TcpStream,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnvelopCode {
    Data = 0x00,
    Error = 0x01,
}

impl EnvelopCode {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, Error> {
        match buf {
            [b] if *b == EnvelopCode::Data as u8 => Ok(EnvelopCode::Data),
            [b] if *b == EnvelopCode::Error as u8 => Ok(EnvelopCode::Error),
            _ => Err(anyhow!("invalid envelop code")),
        }
    }
}

struct EnvelopHeader {
    code: EnvelopCode,
    size: u32,
}

impl EnvelopHeader {
    pub fn write<W: Write>(&self, w: &mut W) -> Result<(), Error> {
        w.write_all(&(self.code as u8).to_be_bytes())?;
        w.write_all(&self.size.to_be_bytes())?;
        Ok(())
    }

    pub fn read<R: Read>(r: &mut R) -> Result<Self, Error> {
        let mut buf = [0u8; size_of::<EnvelopCode>()];
        r.read_exact(&mut buf)?;
        let code = EnvelopCode::from_bytes(&buf)?;

        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        let size = u32::from_be_bytes(buf);

        Ok(Self { code, size })
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WireCode {
    SendFile,
    Found,
    NotFound,
    SignatureData,
    DeltaData,
    Done,
}

impl WireCode {
    pub fn from_bytes(buf: &[u8]) -> Result<Self, Error> {
        match buf {
            [b] if *b == WireCode::Found as u8 => Ok(WireCode::Found),
            [b] if *b == WireCode::NotFound as u8 => Ok(WireCode::NotFound),
            [b] if *b == WireCode::SignatureData as u8 => Ok(WireCode::SignatureData),
            [b] if *b == WireCode::DeltaData as u8 => Ok(WireCode::DeltaData),
            [b] if *b == WireCode::Done as u8 => Ok(WireCode::Done),
            _ => Err(anyhow!("invalid wire code")),
        }
    }
}

struct WireHeader {
    code: WireCode,
    size: u32,
}

impl WireHeader {
    pub fn write<W: Write>(&self, w: &mut W) -> Result<(), Error> {
        w.write_all(&(self.code as u8).to_be_bytes())?;
        w.write_all(&self.size.to_be_bytes())?;
        Ok(())
    }

    pub fn read<R: Read>(r: &mut R) -> Result<Self, Error> {
        let mut buf = [0u8; size_of::<WireCode>()];
        r.read_exact(&mut buf)?;
        let code = WireCode::from_bytes(&buf)?;

        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        let size = u32::from_be_bytes(buf);

        Ok(Self { code, size })
    }
}

#[derive(Debug)]
enum RsyncSenderStateMachine {
    Start,
    SentFileName,
    SendFile,
    RecvSignature,
    ComputeDelta,
    SendDelta,
    WaitPatch,
    Done,
}

impl RsyncSenderStateMachine {
    pub fn new() -> Self {
        RsyncSenderStateMachine::Start
    }

    pub fn send_file_name(&self) -> Result<Self, Error> {
        match self {
            RsyncSenderStateMachine::Start => Ok(RsyncSenderStateMachine::SentFileName),
            _ => Err(anyhow!(
                "state machine error: send_file_name from {:?}",
                self
            )),
        }
    }

    pub fn send_file(&self) -> Result<Self, Error> {
        match self {
            RsyncSenderStateMachine::SentFileName => Ok(RsyncSenderStateMachine::SendFile),
            _ => Err(anyhow!("state machine error: send_file from {:?}", self)),
        }
    }
}

#[derive(Debug)]
enum RsyncReceiverStateMachine {
    Start,
    WaitFileName,
    ReceivedFileName,
    Found,
    NotFound,
    RecvFile,
    ComputeSignature,
    SendSignature,
    RecvDelta,
    Patch,
    Done,
}

impl RsyncTcpServerConn {
    pub fn write_error(&mut self) -> Result<(), Error> {
        self.stream.write_all(&(Code::Error as u8).to_be_bytes())?;

        Ok(())
    }

    pub fn read_file_name(&mut self) -> Result<String, Error> {
        let mut name_len_buf = [0u8; 2];
        self.stream.read_exact(&mut name_len_buf)?;
        let name_len = u16::from_be_bytes(name_len_buf);

        let mut name_buf = vec![0; name_len as usize];
        self.stream.read_exact(&mut name_buf);
        let name = String::from_utf8(name_buf)?;

        Ok(name)
    }

    pub fn send_found(&mut self) -> Result<(), Error> {
        self.stream.write_all(&(Code::Found as u8).to_be_bytes())?;

        Ok(())
    }

    pub fn send_not_found(&mut self) -> Result<(), Error> {
        self.stream
            .write_all(&(Code::NotFound as u8).to_be_bytes())?;

        Ok(())
    }
}

impl Server {
    pub fn init() -> Result<Self, Error> {
        let tcp = TcpListener::bind("localhost:4000")?;

        Ok(Self { tcp })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            for stream in self.tcp.incoming() {
                let stream = stream?;
            }

            self.download(conn)?;
        }
    }
}

fn main() -> Result<(), Error> {
    let server = Server::init()?;

    server.run()?;

    Ok(())
}
