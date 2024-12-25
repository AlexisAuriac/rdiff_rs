mod measure_wire_usage;
mod rabinkarp_precompute_data;

use anyhow::Error;
use clap::{Parser, Subcommand};

use measure_wire_usage::measure_wire_usage;
use rabinkarp_precompute_data::rabinkarp_precompute_data;

#[derive(Debug, Subcommand)]
/// miscelaneous tools for project management
enum Command {
    /// measure signature and delta size in various scenarios
    MeasureWireUsage,
    /// precompute values to speed up rabinkarp (default weak sum)
    RabinkarpPrecomputeData {
        /// number of powers to precompute
        n: u32,
        #[arg(short, long, default_value = None)]
        /// write output to path
        output: Option<String>,
    },
}

#[derive(Debug, Parser)]
#[command(about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::MeasureWireUsage => measure_wire_usage()?,
        Command::RabinkarpPrecomputeData { n, output } => rabinkarp_precompute_data(n, output)?,
    }

    Ok(())
}
