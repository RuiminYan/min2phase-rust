// Apples-to-apples Rust solver against same facelet input as Java Shootout.
// stdin: one 54-char facelet line per cube
// stdout: TSV "input\tsolution\tlen\ttime_us\tok"
// stderr: aggregate stats (same keys as Java Shootout)

use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::time::Instant;

use m2p_core::{tools, verbose, Solver, Tables};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let max_len: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(21);
    let probe_max: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let warmup_n: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(100);

    let t0 = Instant::now();
    let tables = Arc::new(Tables::build(true));
    let init_us = t0.elapsed().as_micros();
    eprintln!("init_us\t{}", init_us);

    let mut solver = Solver::with_tables(tables.clone());

    // Read all inputs
    let stdin = io::stdin();
    let mut inputs: Vec<String> = Vec::new();
    for line in stdin.lock().lines().flatten() {
        let f = line.split('\t').next().unwrap_or("").trim().to_string();
        if f.len() == 54 {
            inputs.push(f);
        }
    }

    // Warmup
    use rand::SeedableRng;
    let mut warm_rng = rand::rngs::StdRng::seed_from_u64(0);
    for _ in 0..warmup_n {
        let c = tools::random_cube(&mut warm_rng);
        let _ = solver.solve(&c, max_len, probe_max, 0, verbose::INVERSE_SOLUTION);
    }

    let mut times = Vec::with_capacity(inputs.len());
    let mut lens = Vec::with_capacity(inputs.len());
    let mut ok = 0usize;

    let stdout = io::stdout();
    let mut out = stdout.lock();

    for cube in &inputs {
        let ts = Instant::now();
        let res = solver.solve(cube, max_len, probe_max, 0, verbose::INVERSE_SOLUTION);
        let us = ts.elapsed().as_micros() as u64;
        match res {
            Ok(sol) => {
                let len = solver.length();
                // Verify: applying solution as scramble should reproduce the cube
                let back = tools::from_scramble(&sol, &tables);
                let ok_this = back == *cube;
                if ok_this {
                    ok += 1;
                }
                times.push(us);
                lens.push(len);
                writeln!(out, "{}\t{}\t{}\t{}\t{}", cube, sol.trim(), len, us, ok_this).ok();
            }
            Err(e) => {
                writeln!(out, "{}\tERROR:{:?}\t0\t{}\tfalse", cube, e, us).ok();
            }
        }
    }
    out.flush().ok();

    if times.is_empty() {
        eprintln!("no valid input");
        std::process::exit(1);
    }

    let n = times.len();
    let mut sorted = times.clone();
    sorted.sort_unstable();
    let avg = times.iter().sum::<u64>() as f64 / n as f64;
    let avg_len = lens.iter().sum::<usize>() as f64 / n as f64;

    let mut hist: std::collections::BTreeMap<usize, u32> = Default::default();
    for &l in &lens {
        *hist.entry(l).or_insert(0) += 1;
    }

    eprintln!("solved\t{}/{}", ok, n);
    eprintln!("avg_us\t{}", avg as u64);
    eprintln!("p50_us\t{}", sorted[n / 2]);
    eprintln!("p95_us\t{}", sorted[n * 95 / 100]);
    eprintln!("p99_us\t{}", sorted[n * 99 / 100]);
    eprintln!("min_us\t{}", sorted[0]);
    eprintln!("max_us\t{}", sorted[n - 1]);
    eprintln!("avg_len\t{}", avg_len);
    let mut s = String::from("len_hist");
    for (l, c) in &hist {
        s.push_str(&format!("\t{}:{}", l, c));
    }
    eprintln!("{}", s);
}
