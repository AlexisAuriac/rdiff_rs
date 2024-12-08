use std::{env, fs, path::PathBuf};

use anyhow::{anyhow, Error};
use blake2::{
    digest::consts::U32, Blake2b, Blake2b512, Blake2s, Blake2s256, Blake2sMac256, Digest,
};

#[derive(Debug)]
struct Opts {
    pub in_file: PathBuf,
    pub out_file: PathBuf,
}

fn parse_opts() -> Result<Opts, Error> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.len() < 1 {
        return Err(anyhow!("not enough arguments"));
    }

    let in_file = PathBuf::from(args[0].clone());
    let out_file = match &args[..] {
        [_, out, ..] => PathBuf::from(out),
        _ => {
            let mut path = in_file.clone();
            path.set_extension("sig");
            path
        }
    };

    Ok(Opts { in_file, out_file })
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
enum SigType {
    Blake2B = 0x72730137,
}

type Blake2b256 = Blake2b<U32>;

impl SigType {
    pub fn to_bytes(self) -> [u8; 4] {
        (self as u32).to_be_bytes()
    }

    pub fn strong_sum(self, data: &[u8], strong_len: u32) -> Vec<u8> {
        match self {
            Self::Blake2B => {
                let mut hasher = Blake2b256::new();
                hasher.update(data);
                hasher
                    .finalize()
                    .to_vec()
                    .drain(..(strong_len as usize))
                    .collect()
            }
        }
    }
}

#[derive(Debug)]
struct Rollsum {
    count: usize,
    s1: u16,
    s2: u16,
}

const ROLLSUM_CHAR_OFFSET: u16 = 31;

impl Rollsum {
    pub fn new() -> Self {
        return Rollsum {
            count: 0,
            s1: 0,
            s2: 0,
        };
    }

    pub fn update(&mut self, p: &[u8]) {
        let l = p.len();

        let mut n = 0;
        while n < l {
            // ??? what does this if branch even do ?
            if n + 15 < l {
                for i in 0..16 {
                    self.s1 = self.s1.wrapping_add(p[n + i] as u16);
                    self.s2 = self.s2.wrapping_add(self.s1);
                }
                n += 16;
            } else {
                self.s1 = self.s1.wrapping_add(p[n] as u16);
                self.s2 = self.s2.wrapping_add(self.s1);
                n += 1;
            }
        }

        self.s1 = self
            .s1
            .wrapping_add((l as u16).wrapping_mul(ROLLSUM_CHAR_OFFSET));
        self.s2 = self
            .s2
            .wrapping_add((((l + 1) * l / 2) as u16).wrapping_mul(ROLLSUM_CHAR_OFFSET));
        self.count += l;
    }

    pub fn digest(&self) -> u32 {
        return ((self.s2 as u32) << 16) | ((self.s1 as u32) & 0xffff);
    }
}

fn compute_weak_checksum(data: &[u8]) -> u32 {
    let mut sum = Rollsum::new();
    sum.update(data);
    return sum.digest();
}

fn signature<I, O>(input: &mut I, output: &mut O) -> Result<(), Error>
where
    I: std::io::Read,
    O: std::io::Write,
{
    const block_len: u32 = 2048;
    const strong_len: u32 = 32;
    const sigtype: SigType = SigType::Blake2B;

    // todo: check max strong len

    output.write(&sigtype.to_bytes())?;
    output.write(&block_len.to_be_bytes())?;
    output.write(&strong_len.to_be_bytes())?;

    let mut block = vec![0u8; block_len as usize];

    loop {
        let n = input.read(&mut block[..])?;
        if n == 0 {
            break;
        }

        let data = &block[..n];

        let weak = compute_weak_checksum(data);
        output.write(&weak.to_be_bytes())?;

        let strong = sigtype.strong_sum(data, strong_len);
        output.write(&strong)?;
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    let opts = parse_opts()?;

    let mut in_file = fs::OpenOptions::new().read(true).open(&opts.in_file)?;
    let mut out_file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&opts.out_file)?;

    signature(&mut in_file, &mut out_file)?;
    println!("{:?}", opts);

    Ok(())
}
