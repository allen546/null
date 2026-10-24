//! `null-cli [--seed N] [--bots N] [--trees N] [--audit FILE] [--max-ticks N] [command...]`
//! Writes (`set-goal`, `halt`, `recall`) need `--reason "text"`; without it they are refused and audited. `help` lists the commands.
//! With a command: run it once (`set-goal` then runs the simulation to the end and prints the status).
//! Without: read commands from standard input, one per line (`run N`, `run-done` advance the simulation).
use null_cli::{Spine, SpineConfig};
use std::io::BufRead;

// Spec: specs/K4-human-interface.md §4.1
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut cfg = SpineConfig::new(1, 2);
    let (mut audit, mut max_ticks) = (None, 20_000u64);
    while args.first().is_some_and(|a| a.starts_with("--") && a != "--reason" && a != "--priority") {
        let flag = args.remove(0);
        if flag == "--help" {
            return println!("{}", null_cli::HELP);
        }
        if args.is_empty() {
            return eprintln!("{flag} needs a value");
        }
        let v = args.remove(0);
        let n = || v.parse::<u64>().unwrap_or_else(|_| panic!("{flag} needs a number"));
        match flag.as_str() {
            "--seed" => cfg.seed = n(),
            "--bots" => cfg.bots = n() as usize,
            "--trees" => cfg.trees = n() as usize,
            "--max-ticks" => max_ticks = n(),
            "--audit" => audit = Some(v),
            _ => return eprintln!("unknown flag {flag}"),
        }
    }
    let mut spine = Spine::new(&cfg);
    if let Some(p) = audit {
        spine.audit = null_cli::Audit::to_file(p.as_ref()).expect("audit file opens");
    }
    if args.is_empty() {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if !line.trim().is_empty() {
                println!("{}", spine.exec(&line).await);
            }
        }
        return;
    }
    // The shell removed the quotes around a multi-word reason; put them back for the parser.
    let line = args.iter().map(|a| if a.contains(char::is_whitespace) { format!("\"{a}\"") } else { a.clone() }).collect::<Vec<_>>().join(" ");
    println!("{}", spine.exec(&line).await);
    if line.starts_with("set-goal") {
        println!("{}", spine.exec(&format!("run-done {max_ticks}")).await);
        println!("{}", spine.exec("status").await);
    }
}
