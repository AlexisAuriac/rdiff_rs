use anyhow::Error;
use clap::{Parser, Subcommand};

// mod gen_test_data;

use gen_test_data::gen_test_data;
use rand::{rngs::StdRng, RngCore, SeedableRng};

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
        #[arg(short, long, value_parser, num_args = 1.., value_delimiter = ',', default_value = "add_start,add_end,add_middle,add_throughout,add_all,rm_start,rm_end,rm_middle,rm_throughout,rm_all")]
        /// types of modifications that will be applied to the data
        modifs: Vec<String>,
        #[arg(short, long, value_parser, num_args = 1.., value_delimiter = ',', default_value = "100,1000,10000,100000")] // default: 1kB, 10kB, 100kB
        /// input sizes
        input_sizes: Vec<u32>,
    },
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

struct GenTestDataParams {}

const MIN_INPUT_SIZE: u64 = 100;
const MAX_INPUT_SIZE: u64 = 1_000_000;

const MIN_BLOCK_LEN: u32 = 12;
const MAX_BLOCK_LEN: u32 = 65536;

#[derive(Debug, Clone, Copy)]
enum StrongSumSize {
    Default,
    Min,
    Max,
    N(u32),
}

impl StrongSumSize {
    pub fn rand(rng: &mut StdRng, max: u32) -> StrongSumSize {
        match rng.next_u32() % 4 {
            0 => StrongSumSize::Default,
            1 => StrongSumSize::Min,
            2 => StrongSumSize::Max,
            3 => StrongSumSize::N(rng.next_u32() % max),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BlockSize {
    Default,
    N(u32),
}

impl BlockSize {
    pub fn rand(rng: &mut StdRng) -> BlockSize {
        match rng.next_u32() % 2 {
            0 => BlockSize::Default,
            1 => BlockSize::N(rng.next_u32() % MAX_BLOCK_LEN + MIN_BLOCK_LEN),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Weaksum {
    Rollsum,
    RabinKarp,
}

impl Weaksum {
    pub fn rand(rng: &mut StdRng) -> Self {
        match rng.next_u32() % 2 {
            0 => Weaksum::Rollsum,
            1 => Weaksum::RabinKarp,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum StrongSum {
    Md4,
    Blake2,
}

impl StrongSum {
    pub fn max_size(&self) -> u32 {
        match self {
            StrongSum::Md4 => 16,
            StrongSum::Blake2 => 32,
        }
    }

    pub fn rand(rng: &mut StdRng) -> Self {
        match rng.next_u32() % 2 {
            0 => StrongSum::Md4,
            1 => StrongSum::Blake2,
            _ => unreachable!(),
        }
    }
}

struct TestParams {
    input_size: u64,
    weaksum: Weaksum,
    strong_sum: StrongSum,
    strong_sum_size: StrongSumSize,
    block_size: BlockSize,
    give_input_size: bool,
}

impl TestParams {
    pub fn rand(rng: &mut StdRng) -> Self {
        let input_size = rng.next_u64() % MAX_INPUT_SIZE + MIN_INPUT_SIZE;

        let weaksum = Weaksum::rand(rng);
        let strong_sum = StrongSum::rand(rng);
        let strong_sum_size = StrongSumSize::rand(rng, strong_sum.max_size());
        let block_size = BlockSize::rand(rng);

        let give_input_size = rng.next_u32() % 2 == 0;

        Self {
            input_size,
            weaksum,
            strong_sum,
            strong_sum_size,
            block_size,
            give_input_size,
        }
    }
}

fn gen_test_data(params: &GenTestDataParams, sample_size: usize) -> Result<Vec<TestParams>, Error> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut params_list = Vec::with_capacity(sample_size);

    for _ in 0..sample_size {
        let params = TestParams::rand(&mut rng);
        params_list.push(params);
    }

    Ok(params_list)
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::GenTestData {
            bin,
            out_dir,
            rollsums,
        } => {
            println!("{:?}",);
            gen_test_data(bin, out_dir)?
        }
    }

    Ok(())
}
