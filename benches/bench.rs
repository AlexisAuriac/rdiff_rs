mod utils;

use std::{
    io::{Cursor, Read},
    time::Duration,
};

use criterion::{criterion_group, criterion_main, Criterion};
use rdiff::{
    signature::{read_signature, signature},
    weak_sum::{rabin_karp::RabinKarp, rollsum::Rollsum},
};

use utils::{RandReader, SlowReader};

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

    c.bench_function("read signature without input size", |b| {
        b.iter(|| {
            read_signature(&mut Cursor::new(&output), None).unwrap();
        })
    });

    c.bench_function("read signature with input size, slow reader", |b| {
        b.iter(|| {
            let mut input =
                SlowReader::with_interval(Cursor::new(&output), Duration::from_nanos(100));
            read_signature(&mut input, Some(output.len())).unwrap();
        })
    });
}

criterion_group!(benches, weak_sum, bench_read_signature);
criterion_main!(benches);
