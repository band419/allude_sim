use std::error::Error;
use std::time::Instant;

use allude_sim::cpu::CpuState;
use allude_sim::sim_env::{IsaExtensions, SimConfig, SimEnv, TestResult};

const TEST_PATH: &str = "isa_test/rv32ui-p-ma_data";
const MAX_STEPS: u64 = 10_000;

fn main() -> Result<(), Box<dyn Error>> {
    if !std::path::Path::new(TEST_PATH).exists() {
        eprintln!("missing test ELF: {TEST_PATH}");
        std::process::exit(1);
    }

    let config = SimConfig::new()
        .with_elf_path(TEST_PATH)
        .with_memory("ram", 0x8000_0000, 512 * 1024)
        .with_extensions(IsaExtensions::rv32g())
        .with_verbose(true);

    let mut env = SimEnv::from_config(config)?;
    println!("Loaded {TEST_PATH}, entry pc=0x{:08x}", env.cpu.pc());

    let start = Instant::now();
    for step in 0..MAX_STEPS {
        let pc = env.cpu.pc();
        let state = env.step();

        if let Some(value) = env.check_tohost() {
            let result = TestResult::from_tohost(value);
            println!(
                "tohost write at step {step}: value=0x{value:08x} -> {:?}",
                result
            );
            println!("Instructions executed: {}", env.instructions_executed);
            env.dump();
            println!("Elapsed: {:?}", start.elapsed());
            return Ok(());
        }

        if state != CpuState::Running {
            println!("CPU entered state {:?} at step {}", state, step);
            env.dump();
            println!("Elapsed: {:?}", start.elapsed());
            return Ok(());
        }

        // Light progress output to track flow
        if step % 100 == 0 {
            println!("step {step:5} pc=0x{pc:08x} state={state:?}");
        }
    }

    println!("Reached MAX_STEPS without tohost write");
    env.dump();
    println!("Elapsed: {:?}", start.elapsed());
    Ok(())
}
