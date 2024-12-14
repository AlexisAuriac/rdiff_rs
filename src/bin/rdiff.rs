use std::{
    fs::{remove_file, OpenOptions},
    io::{BufReader, BufWriter},
};

use anyhow::Error;
use clap::{Parser, Subcommand};
use rdiff::{
    buf_reader_with_retry::BufReaderWithRetry,
    delta::delta,
    patch::patch,
    signature::{read_signature, signature, SigType},
};

#[derive(Debug, Subcommand)]
/// binary diff utility
enum Command {
    /// creates a signature of the input file
    Signature {
        /// input file
        basis: String,
        /// output signature file
        signature: String,
        #[arg(short, long, default_value_t = 2048)]
        /// Signature block size
        block_size: u32,
        #[arg(short = 'S', long, default_value_t = 32)]
        /// Set signature strength
        sum_size: u32,
        #[arg(short = 'H', long, default_value = "blake2")]
        /// Hash algorithm: blake2, md4
        hash: String,
    },
    /// calculates the binary diff between old and new files
    Delta {
        /// input signature file
        signature: String,
        /// input new file
        new_file: String,
        /// output delta file
        delta: String,
    },
    /// uses the delta file and old file to produce the new file
    Patch {
        /// input base file
        basis: String,
        /// input delta file
        delta: String,
        /// output new file
        new_file: String,
    },
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn run_signature(
    basis: String,
    sig_file: String,
    block_size: u32,
    sum_size: u32,
    hash: String,
) -> Result<(), Error> {
    let sigtype = SigType::try_from_str(&hash)?;

    let in_file = OpenOptions::new().read(true).open(&basis)?;
    let out_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&sig_file)?;

    let res = signature(
        &mut BufReaderWithRetry::new(&in_file), // dramatically improves perf for small block len
        &mut BufWriter::new(&out_file),
        block_size,
        sum_size,
        sigtype,
    );
    if let Err(err) = res {
        drop(out_file);
        remove_file(&sig_file).unwrap_or_else(|err| eprintln!("{}: {}", sig_file, err));
        return Err(err);
    }

    let res = out_file.sync_data();
    if let Err(err) = res {
        drop(out_file);
        remove_file(&sig_file).unwrap_or_else(|err| eprintln!("{}: {}", sig_file, err));
        return Err(err.into());
    }

    Ok(())
}

fn run_delta(sig_file: String, new_file: String, delta_path: String) -> Result<(), Error> {
    let mut sig_file = OpenOptions::new().read(true).open(&sig_file)?;
    let sig = read_signature(&mut sig_file)?;

    let new_file = OpenOptions::new().read(true).open(&new_file)?;
    let mut delta_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&delta_path)?;

    let res = delta(&sig, &mut BufReader::new(new_file), &mut delta_file);
    if let Err(err) = res {
        drop(delta_file);
        remove_file(&delta_path).unwrap_or_else(|err| eprintln!("{}: {}", delta_path, err));
        return Err(err);
    }

    let res = delta_file.sync_data();
    if let Err(err) = res {
        drop(delta_file);
        remove_file(&delta_path).unwrap_or_else(|err| eprintln!("{}: {}", delta_path, err));
        return Err(err.into());
    }

    Ok(())
}

fn run_patch(basis: String, delta_file: String, new_path: String) -> Result<(), Error> {
    let mut old_file = OpenOptions::new().read(true).open(&basis)?;
    let mut delta_file = OpenOptions::new().read(true).open(&delta_file)?;
    let mut new_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&new_path)?;

    let res = patch(&mut old_file, &mut delta_file, &mut new_file);
    if let Err(err) = res {
        drop(new_file);
        remove_file(&new_path).unwrap_or_else(|err| eprintln!("{}: {}", new_path, err));
        return Err(err);
    }

    let res = new_file.sync_data();
    if let Err(err) = res {
        drop(new_file);
        remove_file(&new_path).unwrap_or_else(|err| eprintln!("{}: {}", new_path, err));
        return Err(err.into());
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::Signature {
            basis,
            signature: sig_file,
            block_size,
            sum_size,
            hash,
        } => run_signature(basis, sig_file, block_size, sum_size, hash)?,
        Command::Delta {
            signature: sig_file,
            new_file,
            delta,
        } => run_delta(sig_file, new_file, delta)?,
        Command::Patch {
            basis,
            delta: delta_file,
            new_file,
        } => run_patch(basis, delta_file, new_file)?,
    }

    Ok(())
}
