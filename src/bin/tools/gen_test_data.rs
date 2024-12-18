use std::{
    fs::metadata,
    path::{Path, PathBuf},
};

use anyhow::Error;
use which::which;

fn bin_from_env() -> Result<PathBuf, Error> {
    let p = which("rdiff")?;
    Ok(p)
}

fn get_bin_path<P>(bin: Option<P>) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    match bin {
        Some(p) => Ok(PathBuf::from(p.as_ref())),
        None => bin_from_env(),
    }
}

fn check_out_dir<P>(out_dir: P) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    metadata(out_dir)?;
    Ok(())
}

pub fn gen_test_data<P1, P2>(bin: Option<P1>, out_dir: P2) -> Result<(), Error>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let bin_path = get_bin_path(bin)?;

    check_out_dir(out_dir)?;

    todo!()
}
