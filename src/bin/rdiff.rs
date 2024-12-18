use std::error::Error;

use clap::{Parser, Subcommand};
use rdiff::{
    signature::SignatureOptions,
    strong_sum::StrongType,
    weak_sum::weak_sum::WeakSumType,
    whole::{delta, patch, signature_opts},
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
        /// Hash algorithm: blake3, blake2, md4
        hash: String,
        #[arg(short = 'R', long, default_value = "rabinkarp")]
        /// Rollsum algorithm: rabinkarp, rollsum
        rollsum: String,
        #[arg(short, long, default_value_t = false)]
        /// overwrite existing files
        force: bool,
    },
    /// calculates the binary diff between old and new files
    Delta {
        /// input signature file
        signature: String,
        /// input new file
        new_file: String,
        /// output delta file
        delta: String,
        #[arg(short, long, default_value_t = false)]
        /// overwrite existing files
        force: bool,
    },
    /// uses the delta file and old file to produce the new file
    Patch {
        /// input base file
        basis: String,
        /// input delta file
        delta: String,
        /// output new file
        new_file: String,
        #[arg(short, long, default_value_t = false)]
        /// overwrite existing files
        force: bool,
    },
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn signature_options_from_args(cmd: &Command) -> Result<SignatureOptions, Box<dyn Error>> {
    match cmd {
        Command::Signature {
            block_size,
            sum_size,
            hash,
            rollsum,
            ..
        } => {
            let weak = WeakSumType::try_from_str(rollsum)?;
            let strong = StrongType::try_from_str(hash)?;

            Ok(SignatureOptions::new()
                .block_len(*block_size)
                .strong_len(*sum_size)
                .weak_type(weak)
                .strong_type(strong))
        }
        _ => Err("can't call this function for non-signature command".into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Command::Signature {
            basis,
            signature: sig_file,
            force,
            ..
        } => {
            let opts = signature_options_from_args(&cli.command)?;
            signature_opts(basis, sig_file, opts, *force)?
        }
        Command::Delta {
            signature: sig_file,
            new_file,
            delta: delta_file,
            force,
        } => delta(sig_file, new_file, delta_file, *force)?,
        Command::Patch {
            basis,
            delta: delta_file,
            new_file,
            force,
        } => patch(basis, delta_file, new_file, *force)?,
    }

    Ok(())
}
