mod utils;

use std::io::{sink, Cursor, Read};

use criterion::{criterion_group, criterion_main, Criterion};
use rdiff::{
    signature::{read_signature, signature, SignatureOptions},
    weak_sum::{rabin_karp::RabinKarp, rollsum::Rollsum},
};

use utils::RandReader;

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

fn bench_signature(c: &mut Criterion) {
    let mut rnd_reader = RandReader::seed_from_u64(0);

    c.bench_function("signature 1kb", |b| {
        b.iter(|| {
            let rnd_reader = &mut rnd_reader;
            signature(&mut rnd_reader.take(1_000), &mut sink()).unwrap();
        })
    });

    c.bench_function("signature 1Mb", |b| {
        b.iter(|| {
            let rnd_reader = &mut rnd_reader;
            signature(&mut rnd_reader.take(1_000_000), &mut sink()).unwrap();
        })
    });

    c.bench_function("signature 100Mb", |b| {
        b.iter(|| {
            let rnd_reader = &mut rnd_reader;
            signature(&mut rnd_reader.take(100_000_000), &mut sink()).unwrap();
        })
    });
}

fn bench_signature_with_size_hint(c: &mut Criterion) {
    let mut rnd_reader = RandReader::seed_from_u64(0);

    c.bench_function("signature (size hint) 1kb", |b| {
        b.iter(|| {
            let rnd_reader = &mut rnd_reader;

            SignatureOptions::new()
                .input_size(1_000)
                .signature(&mut rnd_reader.take(1_000), &mut sink())
                .unwrap();
        })
    });

    c.bench_function("signature (size hint) 1Mb", |b| {
        b.iter(|| {
            let rnd_reader = &mut rnd_reader;

            SignatureOptions::new()
                .input_size(1_000_000)
                .signature(&mut rnd_reader.take(1_000_000), &mut sink())
                .unwrap();
        })
    });

    c.bench_function("signature (size hint) 100Mb", |b| {
        b.iter(|| {
            let rnd_reader = &mut rnd_reader;

            SignatureOptions::new()
                .input_size(100_000_000)
                .signature(&mut rnd_reader.take(100_000_000), &mut sink())
                .unwrap();
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
}

criterion_group!(
    benches,
    weak_sum,
    bench_signature,
    bench_signature_with_size_hint,
    bench_read_signature
);
criterion_main!(benches);
