use std::{env, fs, path::PathBuf};

use anyhow::{anyhow, Error};
use rdiff::{delta::delta, signature::read_signature};

#[derive(Debug)]
struct Opts {
    pub sig_file: PathBuf,
    pub new_file: PathBuf,
    pub delta_file: PathBuf,
}

fn parse_opts() -> Result<Opts, Error> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.len() < 2 {
        return Err(anyhow!("not enough arguments"));
    }

    let sig_file = PathBuf::from(args[0].clone());
    let new_file = PathBuf::from(args[1].clone());
    let delta_file = match &args[..] {
        [_, _, delta] => PathBuf::from(delta),
        _ => {
            let mut path = new_file.clone();
            path.set_extension("delta");
            path
        }
    };

    Ok(Opts {
        sig_file,
        new_file,
        delta_file,
    })
}

fn main() -> Result<(), Error> {
    let opts = parse_opts()?;

    let mut sig_file = fs::OpenOptions::new().read(true).open(&opts.sig_file)?;
    let sig = read_signature(&mut sig_file)?;

    let mut new_file = fs::OpenOptions::new().read(true).open(&opts.new_file)?;
    let mut delta_file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&opts.delta_file)?;

    delta(&sig, &mut new_file, &mut delta_file)?;

    Ok(())
}
