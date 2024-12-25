mod utils;

use std::{
    fmt::{self, Display, Formatter},
    io::{sink, Read},
};

use rdiff::{error::Error, signature::SignatureOptions};
use utils::{BytesFmt, CountWriter, RandReader};

#[derive(Debug)]
struct SignatureReport {
    pub input_size: usize,
    pub output_size: usize,
    pub cmp: f64,
}

impl Display for SignatureReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "input size: {}", BytesFmt(self.input_size))?;
        writeln!(f, "output size: {}", BytesFmt(self.output_size))?;
        write!(f, "output size: {:.2}%", self.cmp * 100.0)?;
        Ok(())
    }
}

fn signature_size(input_size: usize, opts: &SignatureOptions) -> Result<SignatureReport, Error> {
    let rnd_reader = RandReader::seed_from_u64(0);
    let mut output = CountWriter::new(sink());

    opts.signature(&mut rnd_reader.take(input_size as u64), &mut output)?;

    let cmp = output.count as f64 / input_size as f64;

    Ok(SignatureReport {
        input_size,
        output_size: output.count,
        cmp,
    })
}

pub fn measure_wire_usage() -> Result<(), Error> {
    let sizes = [1_000, 1_000_000, 10_000_000, 100_000_000];

    let opts = SignatureOptions::new();
    for size in sizes {
        let report = signature_size(size, &opts)?;
        println!(
            "--- signature size for input_size={}:\n{}",
            BytesFmt(size),
            report
        );
    }

    for size in sizes {
        println!(
            "--- signature size for input_size={} (with input size hint):",
            BytesFmt(size),
        );

        let opts = SignatureOptions::new().input_size(size).to_owned();
        let report = signature_size(size, &opts)?;

        println!("{}", report);
    }

    for size in sizes {
        println!(
            "--- signature size for input_size={} (with input size hint) and min strong_len:",
            BytesFmt(size),
        );

        let opts = SignatureOptions::new()
            .input_size(size)
            .min_strong_len()
            .to_owned();
        let report = signature_size(size, &opts)?;

        println!("{}", report);
    }

    Ok(())
}
