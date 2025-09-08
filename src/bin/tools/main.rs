mod measure_wire_usage;

use anyhow::Error;
use clap::{Parser, Subcommand};

use measure_wire_usage::measure_wire_usage;

#[derive(Debug, Subcommand)]
/// miscelaneous tools for project management
enum Command {
    /// measure signature and delta size in various scenarios
    MeasureWireUsage,
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
    }

    Ok(())
}
