use std::io::{self, Cursor, Read};

use criterion::{criterion_group, criterion_main, Criterion};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use rdiff::{
    signature::{read_signature, signature},
    weak_sum::{rabin_karp::RabinKarp, rollsum::Rollsum},
};

struct RandReader {
    rng: StdRng,
}

impl RandReader {
    pub fn seed_from_u64(state: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(state),
        }
    }
}

impl Read for RandReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.rng.fill_bytes(buf);
        Ok(buf.len())
    }
}

fn weak_sum(c: &mut Criterion) {
    let mut rnd_reader = RandReader::seed_from_u64(0);

    let mut buf = [0u8; 2048];
    rnd_reader.read_exact(&mut buf).unwrap();

    c.bench_function("rollsum 2048 update+digest", |b| {
        b.iter(|| {
            let mut r = Rollsum::new();
            r.update(&buf);
            let _ = r.digest();
        })
    });

    c.bench_function("rabinkarp 2048 update+digest", |b| {
        b.iter(|| {
            let mut r = RabinKarp::new();
            r.update(&buf);
            let _ = r.digest();
        })
    });
}

fn bench_read_signature(c: &mut Criterion) {
    let rnd_reader = RandReader::seed_from_u64(0);

    let mut output = vec![];
    signature(
        &mut rnd_reader.take(1_000_000),
        &mut Cursor::new(&mut output),
    )
    .unwrap();

    c.bench_function("read signature with input size", |b| {
        b.iter(|| {
            read_signature(&mut Cursor::new(&output), Some(output.len())).unwrap();
        })
    });
}

criterion_group!(benches, weak_sum, bench_read_signature);
criterion_main!(benches);
