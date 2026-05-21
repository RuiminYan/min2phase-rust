// m2p — min2phase Rust CLI
//
// Subcommands:
//   m2p solve <facelets>           one-shot solve, prints solution
//   m2p scramble "R U R' ..."      scramble string -> 54-char facelets
//   m2p random [seed]              print one random cube as facelets
//   m2p bench [n] [seed]           solve N random cubes, print stats
//   m2p daemon                     stdin: facelet line per request, stdout: solution per response

use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::time::Instant;

use m2p_core::{tools, Solver, SolverError, Tables};

fn print_usage() {
    eprintln!(
        "m2p — min2phase Rust solver

Usage:
  m2p solve <FACELETS>           solve a 54-char facelet string
  m2p scramble \"R U R' U' ...\"   convert scramble to facelets
  m2p random [seed]              emit one random cube
  m2p bench [n=100] [seed=42]    solve N random cubes
  m2p daemon                     line-driven solver on stdin/stdout
"
    );
}

fn build_solver() -> (Arc<Tables>, Solver) {
    let tables = Arc::new(Tables::build(true));
    let solver = Solver::with_tables(tables.clone());
    (tables, solver)
}

fn cmd_solve(facelets: &str) -> i32 {
    let (_t, mut solver) = build_solver();
    match solver.solve(facelets, 21, 100_000, 0, 0) {
        Ok(s) => {
            println!("{}", s);
            0
        }
        Err(e) => {
            eprintln!("{}: {}", err_name(e), e);
            1
        }
    }
}

fn err_name(e: SolverError) -> &'static str {
    match e {
        SolverError::FaceletParse => "FaceletParse",
        SolverError::EdgeMissing => "EdgeMissing",
        SolverError::EdgeFlip => "EdgeFlip",
        SolverError::CornerMissing => "CornerMissing",
        SolverError::CornerTwist => "CornerTwist",
        SolverError::Parity => "Parity",
        SolverError::NoSolutionInDepth => "NoSolutionInDepth",
        SolverError::ProbeLimitExceeded => "ProbeLimitExceeded",
    }
}

fn cmd_scramble(scramble: &str) -> i32 {
    let tables = Tables::build(false);
    let f = tools::from_scramble(scramble, &tables);
    println!("{}", f);
    0
}

fn cmd_random(seed: Option<u64>) -> i32 {
    let s = match seed {
        Some(s) => {
            use rand::SeedableRng;
            let mut rng = rand::rngs::StdRng::seed_from_u64(s);
            tools::random_cube(&mut rng)
        }
        None => tools::random_cube(&mut rand::thread_rng()),
    };
    println!("{}", s);
    0
}

fn cmd_bench(n: usize, seed: u64) -> i32 {
    use rand::SeedableRng;
    eprintln!("# building tables...");
    let t_init = Instant::now();
    let tables = Arc::new(Tables::build(true));
    let init_ms = t_init.elapsed().as_secs_f64() * 1000.0;
    eprintln!("# init {:.1} ms", init_ms);

    let mut solver = Solver::with_tables(tables);
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // warmup
    for _ in 0..10 {
        let c = tools::random_cube(&mut rng);
        let _ = solver.solve(&c, 21, 100_000, 0, 0);
    }
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    let mut times_us: Vec<u64> = Vec::with_capacity(n);
    let mut lengths: Vec<usize> = Vec::with_capacity(n);
    let mut fails = 0usize;

    let t0 = Instant::now();
    for _ in 0..n {
        let c = tools::random_cube(&mut rng);
        let ts = Instant::now();
        match solver.solve(&c, 21, 100_000, 0, 0) {
            Ok(_sol) => {
                let us = ts.elapsed().as_micros() as u64;
                times_us.push(us);
                lengths.push(solver.length());
            }
            Err(_) => fails += 1,
        }
    }
    let total_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let ok = times_us.len();
    if ok == 0 {
        eprintln!("all failed");
        return 1;
    }
    times_us.sort_unstable();

    let avg_us = times_us.iter().sum::<u64>() as f64 / ok as f64;
    let p50 = times_us[ok / 2] as f64;
    let p95 = times_us[ok * 95 / 100] as f64;
    let p99 = times_us[ok * 99 / 100] as f64;
    let max = *times_us.last().unwrap() as f64;
    let min = *times_us.first().unwrap() as f64;
    let avg_len = lengths.iter().sum::<usize>() as f64 / ok as f64;

    let mut len_hist = std::collections::BTreeMap::new();
    for &l in &lengths {
        *len_hist.entry(l).or_insert(0u32) += 1;
    }

    println!("solved   : {}/{}", ok, n);
    if fails > 0 {
        println!("failed   : {}", fails);
    }
    println!("total    : {:.1} ms", total_ms);
    println!("avg      : {:.3} ms", avg_us / 1000.0);
    println!("p50      : {:.3} ms", p50 / 1000.0);
    println!("p95      : {:.3} ms", p95 / 1000.0);
    println!("p99      : {:.3} ms", p99 / 1000.0);
    println!("min      : {:.3} ms", min / 1000.0);
    println!("max      : {:.3} ms", max / 1000.0);
    println!("avg-len  : {:.3}", avg_len);
    print!("len-hist :");
    for (l, c) in &len_hist {
        print!(" {}:{}", l, c);
    }
    println!();
    if fails > 0 {
        1
    } else {
        0
    }
}

fn cmd_daemon() -> i32 {
    eprintln!("# m2p daemon — building tables...");
    let t_init = Instant::now();
    let tables = Arc::new(Tables::build(true));
    eprintln!(
        "# ready ({:.1} ms)",
        t_init.elapsed().as_secs_f64() * 1000.0
    );
    let mut solver = Solver::with_tables(tables.clone());
    let mut rng = rand::thread_rng();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "quit" || line == "exit" {
            break;
        }
        let facelets: String = if line == "random" {
            tools::random_cube(&mut rng)
        } else if let Some(rest) = line.strip_prefix("scramble ") {
            tools::from_scramble(rest, &tables)
        } else {
            line.to_string()
        };

        let t = Instant::now();
        match solver.solve(&facelets, 21, 100_000, 0, 0) {
            Ok(sol) => {
                let us = t.elapsed().as_micros();
                writeln!(out, "OK {} {} us {}", solver.length(), us, sol).ok();
            }
            Err(e) => {
                writeln!(out, "ERR {}: {}", err_name(e), e).ok();
            }
        }
        out.flush().ok();
    }
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(2);
    }
    let code = match args[1].as_str() {
        "solve" => {
            if args.len() < 3 {
                eprintln!("solve: need <FACELETS>");
                1
            } else {
                cmd_solve(&args[2])
            }
        }
        "scramble" => {
            if args.len() < 3 {
                eprintln!("scramble: need scramble string");
                1
            } else {
                cmd_scramble(&args[2])
            }
        }
        "random" => {
            let seed = args.get(2).and_then(|s| s.parse::<u64>().ok());
            cmd_random(seed)
        }
        "bench" => {
            let n = args.get(2).and_then(|s| s.parse::<usize>().ok()).unwrap_or(100);
            let seed = args.get(3).and_then(|s| s.parse::<u64>().ok()).unwrap_or(42);
            cmd_bench(n, seed)
        }
        "daemon" => cmd_daemon(),
        "-h" | "--help" | "help" => {
            print_usage();
            0
        }
        other => {
            eprintln!("unknown subcommand: {}", other);
            print_usage();
            2
        }
    };
    std::process::exit(code);
}
