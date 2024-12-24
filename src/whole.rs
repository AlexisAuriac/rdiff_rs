use std::{
    fs::{remove_file, OpenOptions},
    io::{BufReader, BufWriter},
    path::Path,
};

use crate::{
    buf_reader_with_retry::BufReaderWithRetry,
    delta::delta as io_delta,
    error::Error,
    patch::patch as io_patch,
    signature::{read_signature, SignatureOptions},
};

pub fn signature<P1, P2>(basis: P1, sig_file: P2, force: bool) -> Result<(), Error>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    signature_opts(basis, sig_file, SignatureOptions::new(), force)
}

pub fn signature_opts<P1, P2>(
    basis: P1,
    sig_file: P2,
    mut opts: SignatureOptions,
    force: bool,
) -> Result<(), Error>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let in_file = OpenOptions::new().read(true).open(&basis)?;
    let out_file = OpenOptions::new()
        .create(force)
        .create_new(!force)
        .truncate(true)
        .write(true)
        .open(&sig_file)?;

    let input_size = in_file.metadata()?.len();
    opts.input_size(input_size as usize);

    let res = opts.signature(
        &mut BufReaderWithRetry::new(&in_file), // dramatically improves perf for small block len
        &mut BufWriter::new(&out_file),
    );
    if let Err(err) = res {
        drop(out_file);
        remove_file(&sig_file).unwrap_or_else(|err| eprintln!("{:?}: {}", sig_file.as_ref(), err));
        return Err(err);
    }

    let res = out_file.sync_data();
    if let Err(err) = res {
        drop(out_file);
        remove_file(&sig_file).unwrap_or_else(|err| eprintln!("{:?}: {}", sig_file.as_ref(), err));
        return Err(err.into());
    }

    Ok(())
}

pub fn delta<P1, P2, P3>(
    sig_file: P1,
    new_file: P2,
    delta_path: P3,
    force: bool,
) -> Result<(), Error>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    let sig = {
        let mut sig_file = OpenOptions::new().read(true).open(&sig_file)?;
        let input_size = sig_file.metadata()?.len() as usize;

        read_signature(&mut sig_file, Some(input_size))?
    };

    let new_file = OpenOptions::new().read(true).open(&new_file)?;
    let mut delta_file = OpenOptions::new()
        .create(force)
        .create_new(!force)
        .truncate(true)
        .write(true)
        .open(&delta_path)?;

    let res = io_delta(&sig, &mut BufReader::new(new_file), &mut delta_file);
    if let Err(err) = res {
        drop(delta_file);
        remove_file(&delta_path)
            .unwrap_or_else(|err| eprintln!("{:?}: {}", delta_path.as_ref(), err));
        return Err(err);
    }

    let res = delta_file.sync_data();
    if let Err(err) = res {
        drop(delta_file);
        remove_file(&delta_path)
            .unwrap_or_else(|err| eprintln!("{:?}: {}", delta_path.as_ref(), err));
        return Err(err.into());
    }

    Ok(())
}

pub fn patch<P1, P2, P3>(basis: P1, delta_file: P2, new_path: P3, force: bool) -> Result<(), Error>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    let mut old_file = OpenOptions::new().read(true).open(&basis)?;
    let mut delta_file = OpenOptions::new().read(true).open(&delta_file)?;
    let mut new_file = OpenOptions::new()
        .create(force)
        .create_new(!force)
        .truncate(true)
        .write(true)
        .open(&new_path)?;

    let res = io_patch(&mut old_file, &mut delta_file, &mut new_file);
    if let Err(err) = res {
        drop(new_file);
        remove_file(&new_path).unwrap_or_else(|err| eprintln!("{:?}: {}", new_path.as_ref(), err));
        return Err(err);
    }

    let res = new_file.sync_data();
    if let Err(err) = res {
        drop(new_file);
        remove_file(&new_path).unwrap_or_else(|err| eprintln!("{:?}: {}", new_path.as_ref(), err));
        return Err(err.into());
    }

    Ok(())
}
