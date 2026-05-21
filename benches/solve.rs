// Criterion benchmarks for hot paths.
//
//   cargo bench -p m2p-core
//
// Bench groups:
//   - tables_build: cold init time (full vs partial)
//   - solve_random: end-to-end solve on uniformly random cubes
//   - solve_super_flip: known hard case
//   - from_scramble: scramble string -> facelet conversion

use std::sync::Arc;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use m2p_core::{tools, verbose, Solver, Tables};
use rand::SeedableRng;

fn bench_tables_build(c: &mut Criterion) {
    let mut g = c.benchmark_group("tables_build");
    g.sample_size(10);
    g.bench_function("full_init", |b| {
        b.iter(|| Tables::build(true));
    });
    g.bench_function("partial_init", |b| {
        b.iter(|| Tables::build(false));
    });
    g.finish();
}

fn bench_solve(c: &mut Criterion) {
    let tables = Arc::new(Tables::build(true));
    let mut solver = Solver::with_tables(tables.clone());
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    let cubes: Vec<String> = (0..200).map(|_| tools::random_cube(&mut rng)).collect();

    for c in cubes.iter().take(20) {
        let _ = solver.solve(c, 21, 100_000, 0, verbose::INVERSE_SOLUTION);
    }

    let mut idx = 0;
    let mut g = c.benchmark_group("solve_random");
    g.sample_size(100);
    g.bench_function("max_depth_21", |b| {
        b.iter(|| {
            let cube = &cubes[idx % cubes.len()];
            idx += 1;
            solver
                .solve(cube, 21, 100_000, 0, verbose::INVERSE_SOLUTION)
                .unwrap()
        });
    });
    g.bench_function("max_depth_22", |b| {
        b.iter(|| {
            let cube = &cubes[idx % cubes.len()];
            idx += 1;
            solver
                .solve(cube, 22, 100_000, 0, verbose::INVERSE_SOLUTION)
                .unwrap()
        });
    });
    g.finish();
}

fn bench_super_flip(c: &mut Criterion) {
    let tables = Arc::new(Tables::build(true));
    let mut solver = Solver::with_tables(tables.clone());
    let sf = tools::super_flip();
    for _ in 0..5 {
        let _ = solver.solve(&sf, 21, 1_000_000, 0, verbose::INVERSE_SOLUTION);
    }
    let mut g = c.benchmark_group("solve_super_flip");
    g.sample_size(30);
    g.bench_function("max_depth_21", |b| {
        b.iter(|| {
            solver
                .solve(&sf, 21, 1_000_000, 0, verbose::INVERSE_SOLUTION)
                .unwrap()
        });
    });
    g.finish();
}

fn bench_from_scramble(c: &mut Criterion) {
    let tables = Tables::build(false);
    let scrambles = [
        "R U R' U' R' F R F'",
        "R U2 D' B D'",
        "F R U' R' U' R U R' F' R U R' U' R' F R F'",
        "U2 R2 F2 D' U F2 U2 B U B' R U' F L B R' F L2 D' B",
    ];
    let mut g = c.benchmark_group("from_scramble");
    for s in scrambles.iter() {
        let len = s.split_whitespace().count();
        g.bench_with_input(
            criterion::BenchmarkId::new("len", len),
            s,
            |b, s| {
                b.iter_batched(
                    || s.to_string(),
                    |s| tools::from_scramble(&s, &tables),
                    BatchSize::SmallInput,
                );
            },
        );
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_tables_build,
    bench_solve,
    bench_super_flip,
    bench_from_scramble
);
criterion_main!(benches);
