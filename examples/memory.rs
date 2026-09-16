//! What the index costs to hold, against what the budget buys.
//!
//!     cargo run --release --example memory [budget...]
//!     cargo run --release --example memory 200M 900M 2G
//!
//! A budget is a count of regexes, or a size: `20000`, `900M`, `1GiB`. Both columns are printed
//! either way, which is the only thing that says whether a count of regexes came to what it was
//! expected to. `warm:128M` spends the budget on the user agents the crate ships with instead of
//! on the shape of the index, which is the comparison worth making.
//!
//! One budget per process, because the allocator does not give a freed index back to the system:
//! a second budget measured in the same process reuses what the first one released and reads far
//! too low. The parent run spawns itself once per budget and prints what each child reports.
//!
//! Resident size is the one thing `cargo bench` cannot answer: it is a
//! property of the process rather than of an operation, so there is nothing for criterion to
//! sample -- running the same allocation a hundred times measures the allocator, not the index.

use device_detector::{Budget, Detector, common_user_agents};

fn resident() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();

    status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|line| line.split_whitespace().next()?.parse().ok())
        .unwrap_or(0)
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    if let Some(budget) = arguments.strip_prefix(&[String::from("--one")]) {
        return one(budget.first().expect("--one takes a budget"));
    }

    let budgets = match arguments.is_empty() {
        true => ["0", "2000", "20000", "900M", "warm:128M"].map(String::from).to_vec(),
        false => arguments,
    };

    let before = resident();
    let detector = Detector::new();
    println!("chargement   {} entrees  {} Mio", detector.len(), (resident() - before) / 1024);
    drop(detector);

    println!("\n          budget     compilees      reste      mesure    resident");

    for budget in budgets {
        let run = std::process::Command::new(std::env::current_exe().expect("our own path"))
            .args(["--one", &budget])
            .status()
            .expect("a run of our own");

        assert!(run.success(), "the run for {budget} failed");
    }
}

/// One budget, in a process of its own.
fn one(text: &str) {
    let warm = text.strip_prefix("warm:");
    let budget: Budget = warm.unwrap_or(text).parse().unwrap_or_else(|error| panic!("{error}"));
    let mut detector = Detector::new();
    let loaded = resident();

    let left = match warm {
        Some(_) => detector.warm(common_user_agents(), budget),
        None => detector.cache(budget),
    };

    println!(
        "{:>16}  {:>12}  {:>10}  {:>6} Mio  {:>6} Mio",
        text,
        detector.compiled(),
        left.to_string(),
        detector.compiled_size() / (1 << 20),
        resident().saturating_sub(loaded) / 1024
    );
}
