mod whole;

use anyhow::{anyhow, Error};
use clap::{Parser, Subcommand};
use rdiff::{signature::SignatureOptions, strong_sum::StrongType, weak_sum::WeakSumType};
use whole::{delta, patch, signature_opts};

#[derive(Debug, Subcommand)]
/// binary diff utility
enum Command {
    /// creates a signature of the input file
    Signature {
        /// input file
        basis: String,
        /// output signature file
        signature: String,
        #[arg(short, long, default_value = None)]
        /// Signature block size
        block_size: Option<u32>,
        #[arg(short = 'S', long, default_value = None)]
        /// Set signature strength (-1 for minimum, max by default)
        sum_size: Option<i64>,
        #[arg(short = 'H', long, default_value = "blake2")]
        /// Hash algorithm: blake2, md4
        hash: String,
        #[arg(short = 'R', long, default_value = "rabinkarp")]
        /// Rollsum algorithm: rabinkarp, rollsum
        rollsum: String,
        #[arg(short, long, default_value_t = false)]
        /// overwrite existing files
        force: bool,
        #[arg(short = 'I', long, default_value = None)]
        /// input size in bytes
        input_size: Option<usize>,
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
        #[arg(short = 'I', long, default_value = None)]
        /// signature size in bytes
        input_size: Option<usize>,
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

fn signature_options_from_args(cmd: &Command) -> Result<SignatureOptions, Error> {
    match cmd {
        Command::Signature {
            block_size,
            sum_size,
            hash,
            rollsum,
            input_size,
            ..
        } => {
            let weak = WeakSumType::try_from_str(rollsum)?;
            let strong = StrongType::try_from_str(hash)?;

            let mut opts = SignatureOptions::new()
                .weak_type(weak)
                .strong_type(strong)
                .to_owned();

            if let Some(block_size) = block_size {
                opts.block_len(*block_size);
            }

            if let Some(input_size) = input_size {
                opts.input_size(*input_size);
            }

            // clumsy attempt to be compatible with the original rdiff
            // -1 => min, 0 => max, n => n
            match sum_size {
                Some(-1) => {
                    opts.min_strong_len();
                }
                Some(0) => {
                    opts.max_strong_len();
                }
                Some(sum_size) if *sum_size > u32::MAX as i64 => {
                    return Err(anyhow!("sum size is too big"));
                }
                Some(sum_size) if *sum_size < -1 => {
                    return Err(anyhow!("sum size is too small"));
                }
                Some(n) => {
                    opts.strong_len(*n as u32);
                }
                None => (),
            }

            Ok(opts)
        }
        _ => Err(anyhow!(
            "can't call this function for non-signature command"
        )),
    }
}

fn main() -> Result<(), Error> {
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
            input_size,
        } => delta(sig_file, new_file, delta_file, *force, *input_size)?,
        Command::Patch {
            basis,
            delta: delta_file,
            new_file,
            force,
        } => patch(basis, delta_file, new_file, *force)?,
    }

    Ok(())
}
