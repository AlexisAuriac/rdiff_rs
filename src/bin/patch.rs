use std::{env, fs, path::PathBuf};

use anyhow::{anyhow, Error};
use rdiff::patch::patch;

#[derive(Debug)]
struct Opts {
    pub old_file: PathBuf,
    pub delta_file: PathBuf,
    pub new_file: PathBuf,
}

fn parse_opts() -> Result<Opts, Error> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.len() < 2 {
        return Err(anyhow!("not enough arguments"));
    }

    let old_file = PathBuf::from(args[0].clone());
    let delta_file = PathBuf::from(args[1].clone());
    let new_file = match &args[..] {
        [_, _, new] => PathBuf::from(new),
        _ => {
            let mut path = old_file.clone();
            path.set_extension("patched");
            path
        }
    };

    Ok(Opts {
        old_file,
        delta_file,
        new_file,
    })
}

fn main() -> Result<(), Error> {
    let opts = parse_opts()?;

    let mut old_file = fs::OpenOptions::new().read(true).open(&opts.old_file)?;
    let mut delta_file = fs::OpenOptions::new().read(true).open(&opts.delta_file)?;
    let mut new_file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&opts.new_file)?;

    patch(&mut old_file, &mut delta_file, &mut new_file)?;

    Ok(())
}
