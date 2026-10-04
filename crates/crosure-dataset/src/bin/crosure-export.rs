//! `crosure-export [options] [OUT_DIR]`: write recorded investigations as a
//! training dataset (trajectories, SFT, DPO). See `crosure_dataset`.

use std::path::PathBuf;
use std::process::ExitCode;

use crosure_dataset::{export, ExportOptions};
use crosure_recorder::Store;

const USAGE: &str = "usage: crosure-export [--session ID]... [--all-steps] [--humans-only] \
[--history N] [--allow-unverified] [--test-percent N] [OUT_DIR]

Reads $CROSURE_HOME/crosure.db (default ~/.crosure) and writes trajectories.jsonl,
sft.jsonl, dpo.jsonl and manifest.json to OUT_DIR (default $CROSURE_HOME/datasets/latest).";

fn home() -> PathBuf {
    std::env::var_os("CROSURE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".crosure")))
        .unwrap_or_else(|| PathBuf::from(".crosure"))
}

fn parse(args: &[String]) -> Result<(ExportOptions, Option<PathBuf>), String> {
    let mut opts = ExportOptions::default();
    let mut out = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--session" => opts
                .sessions
                .push(it.next().ok_or("--session needs an id")?.clone()),
            "--all-steps" => opts.sft.key_path_only = false,
            "--humans-only" => opts.sft.humans_only = true,
            "--allow-unverified" => opts.allow_unverified = true,
            "--test-percent" => {
                opts.test_percent = it
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|n| *n <= 100)
                    .ok_or("--test-percent needs a number from 0 to 100")?
            }
            "--history" => {
                opts.sft.history = it
                    .next()
                    .and_then(|n| n.parse().ok())
                    .ok_or("--history needs a number")?
            }
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => out = Some(PathBuf::from(s)),
        }
    }
    Ok((opts, out))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (opts, out) = match parse(&args) {
        Ok(p) => p,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}");
            }
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let home = home();
    let out = out.unwrap_or_else(|| home.join("datasets").join("latest"));
    let result = Store::open(&home.join("crosure.db"))
        .map_err(|e| e.to_string())
        .and_then(|store| export(&store, &out, &opts).map_err(|e| e.to_string()));
    match result {
        Ok(m) => {
            for s in &m.sessions {
                let status = s.skipped.as_deref().unwrap_or("ok");
                println!(
                    "{} {:<24} {:>4} steps {:>4} sft {:>3} dpo  {:<5} {status}",
                    s.session_id, s.binary, s.steps, s.sft, s.dpo, s.split
                );
            }
            println!(
                "{} trajectories, {} SFT examples, {} DPO pairs -> {}",
                m.trajectories,
                m.sft_examples,
                m.dpo_pairs,
                out.display()
            );
            println!(
                "split by binary and shared code: train {} / test {} trajectories{}",
                m.train.trajectories,
                m.test.trajectories,
                if m.sft_duplicates > 0 {
                    format!(", {} duplicate SFT examples dropped", m.sft_duplicates)
                } else {
                    String::new()
                }
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("crosure-export: {e}");
            ExitCode::FAILURE
        }
    }
}
