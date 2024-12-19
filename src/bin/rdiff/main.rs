mod mock;
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
        #[arg(short, long, default_value_t = 2048)]
        /// Signature block size
        block_size: u32,
        #[arg(short = 'S', long, default_value_t = 32)]
        /// Set signature strength
        sum_size: u32,
        #[arg(short = 'H', long, default_value = "blake2")]
        /// Hash algorithm: blake2, md4
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

fn signature_options_from_args(cmd: &Command) -> Result<SignatureOptions, Error> {
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
        _ => Err(anyhow!(
            "can't call this function for non-signature command"
        )),
    }
}

impl Cli {
    pub fn run(self) -> Result<(), Error> {
        match &self.command {
            Command::Signature {
                basis,
                signature: sig_file,
                force,
                ..
            } => {
                let opts = signature_options_from_args(&self.command)?;
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
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    cli.run()
}

#[cfg(test)]
mod tests {
    use anyhow::Error;

    use super::*;

    fn cli_from_argv<T: AsRef<str>>(argv: &[T]) -> Result<Cli, Error> {
        let cli = Cli::try_parse_from(
            argv.iter()
                .map(|s| s.as_ref().to_string())
                .collect::<Vec<_>>(),
        )?;
        Ok(cli)
    }

    fn argv_format<T: AsRef<str>>(argv: &[T]) -> String {
        argv.iter()
            .map(|s| {
                if s.as_ref().is_ascii() {
                    s.as_ref().to_string()
                } else {
                    format!("'{}'", s.as_ref())
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn assert_cli_argv_err<T, U>(argv: &[T], msg: U)
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        assert!(
            cli_from_argv(argv).is_err(),
            "{}: `{}`",
            msg.as_ref(),
            argv_format(argv),
        );
    }

    #[test]
    fn basic_error_handling() -> Result<(), Error> {
        assert_cli_argv_err(&["rdiff"], "no arguments should fail");
        assert_cli_argv_err(&["rdiff", "a"], "bad subcommand should fail");

        assert_cli_argv_err(&["rdiff", "signature"], "signature: 0 args should fail");
        assert_cli_argv_err(
            &["rdiff", "signature", "basis"],
            "signature: missing signature should fail",
        );

        assert_cli_argv_err(
            &["rdiff", "delta"],
            "delta: missing sig, new, delta should fail",
        );
        assert_cli_argv_err(
            &["rdiff", "delta", "sig"],
            "delta: missing new, delta should fail",
        );
        assert_cli_argv_err(
            &["rdiff", "delta", "sig", "new"],
            "delta: missing delta should fail",
        );

        assert_cli_argv_err(
            &["rdiff", "patch"],
            "delta: missing old, delta, new should fail",
        );
        assert_cli_argv_err(
            &["rdiff", "patch", "old"],
            "delta: missing delta, new should fail",
        );
        assert_cli_argv_err(
            &["rdiff", "patch", "old", "delta"],
            "delta: missing new should fail",
        );

        Ok(())
    }
}
