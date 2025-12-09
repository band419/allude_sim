use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use allude_sim::sim_env::{IsaExtensions, SimConfig, SimEnv, TestResult};

const PREFIX: &str = "rv32um-p-";

fn main() {
    let filter = env::args().nth(1);
    if let Err(err) = run_suite(filter.as_deref()) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run_suite(filter: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new("isa_test");
    if !root.exists() {
        return Err(format!("{} does not exist", root.display()).into());
    }

    let cases = collect_cases(root, filter)?;
    if cases.is_empty() {
        match filter {
            Some(pattern) => println!(
                "No {PREFIX}* tests matching '{pattern}' under {}",
                root.display()
            ),
            None => println!("No {PREFIX}* tests found under {}", root.display()),
        }
        return Ok(());
    }

    match filter {
        Some(pattern) => println!(
            "Discovered {} {PREFIX}* tests matching '{pattern}'",
            cases.len()
        ),
        None => println!("Discovered {} {PREFIX}* tests", cases.len()),
    }

    let mut pass = 0usize;
    let mut fail = Vec::new();

    for case in &cases {
        let name = case.file_name().unwrap().to_string_lossy().into_owned();
        print!("[RUN] {name} ... ");
        let start = Instant::now();
        match run_case(case) {
            Ok((TestResult::Pass, executed)) => {
                pass += 1;
                println!("PASS ({} instr, {:?})", executed, start.elapsed());
            }
            Ok((result, executed)) => {
                println!("{:?} ({} instr, {:?})", result, executed, start.elapsed());
                fail.push((name, result));
            }
            Err(err) => {
                println!("ERROR: {err}");
                fail.push((name, TestResult::Timeout));
            }
        }
    }

    println!("\nSummary: {} passed / {} failed", pass, fail.len());
    if !fail.is_empty() {
        println!("Failed cases:");
        for (name, result) in &fail {
            println!("  {name}: {:?}", result);
        }
        return Err("rv32um suite has failures".into());
    }

    Ok(())
}

fn collect_cases(root: &Path, filter: Option<&str>) -> io::Result<Vec<PathBuf>> {
    let mut cases = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with(PREFIX) && !name.ends_with(".dump") {
            if let Some(pattern) = filter {
                if !name.contains(pattern) {
                    continue;
                }
            }
            cases.push(path);
        }
    }
    cases.sort();
    Ok(cases)
}

fn run_case(path: &Path) -> Result<(TestResult, u64), Box<dyn std::error::Error>> {
    let config = SimConfig::new()
        .with_elf_path(path.to_string_lossy().into_owned())
        .with_memory("ram", 0x8000_0000, 512 * 1024)
        .with_extensions(IsaExtensions::rv32g())
        .with_verbose(false);

    let mut env = SimEnv::from_config(config)?;
    let result = env.run_isa_test(2_000_000);
    Ok(result)
}
