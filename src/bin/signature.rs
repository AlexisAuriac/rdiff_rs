use std::{env, fs, path::PathBuf};

use anyhow::{anyhow, Error};
use rdiff::signature::{signature, SigType};

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

const BLOCK_LEN: u32 = 2048;
const STRONG_LEN: u32 = 32;
const SIGTYPE: SigType = SigType::Blake2B;

fn main() -> Result<(), Error> {
    let opts = parse_opts()?;

    let mut in_file = fs::OpenOptions::new().read(true).open(&opts.in_file)?;
    let mut out_file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&opts.out_file)?;

    signature(&mut in_file, &mut out_file, BLOCK_LEN, STRONG_LEN, SIGTYPE)?;

    Ok(())
}
