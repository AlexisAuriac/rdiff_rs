use std::io::{self, Read};

use criterion::{criterion_group, criterion_main, Criterion};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use rdiff::weak_sum::{rabin_karp::RabinKarp, rollsum::Rollsum};

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

criterion_group!(benches, weak_sum);
criterion_main!(benches);
