use anyhow::Error;
use clap::{Parser, Subcommand};

mod gen_test_data;

use gen_test_data::gen_test_data;

#[derive(Debug, Subcommand)]
/// tools for project management
enum Command {
    /// generate test data using the original rdiff implementation
    GenTestData {
        #[arg(short, long)]
        /// rdiff binary path, (default: PATH)
        bin: Option<String>,
        #[arg(short, long, default_value = "testdata")]
        /// output directory
        out_dir: String,
        #[arg(short = 'R', long, value_parser, num_args = 1.., value_delimiter = ',', default_value = "rabinkarp,rollsum")]
        /// rollsums
        rollsums: Vec<String>,
        #[arg(short = 'H', long, value_parser, num_args = 1.., value_delimiter = ',', default_value = "blake2,md4")]
        /// hashes
        hashes: Vec<String>,
        #[arg(short = 'B', long, value_parser, num_args = 1.., value_delimiter = ',', default_value = "256,2033,2048,4096")]
        /// block sizes
        block_sizes: Vec<u32>,
        #[arg(short, long, value_parser, num_args = 1.., value_delimiter = ',', default_value = "12,18,31,32")]
        /// strong sizes
        strong_sizes: Vec<u32>,
    },
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::GenTestData { bin, out_dir, .. } => gen_test_data(bin, out_dir)?,
    }

    Ok(())
}
