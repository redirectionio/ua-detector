//! What the index costs to hold, against what the budget buys.
//!
//!     cargo run --release --example memory [budget...]
//!
//! Resident size is the one thing `cargo bench` cannot answer: it is a
//! property of the process rather than of an operation, so there is nothing for criterion to
//! sample -- running the same allocation a hundred times measures the allocator, not the index.

fn resident() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();

    status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|line| line.split_whitespace().next()?.parse().ok())
        .unwrap_or(0)
}

fn main() {
    let budgets: Vec<u64> = match std::env::args().skip(1).collect::<Vec<_>>() {
        arguments if arguments.is_empty() => vec![0, 1_000, 5_000, 20_000],
        arguments => arguments.iter().filter_map(|argument| argument.parse().ok()).collect(),
    };

    let before = resident();
    let detector = device_detector::Detector::new();
    println!(
        "chargement   {} entrees  {} Mio",
        detector.len(),
        (resident() - before) / 1024
    );
    drop(detector);

    println!("\n  budget   compilees   memoire");

    for budget in budgets {
        let mut detector = device_detector::Detector::new();
        let loaded = resident();

        let left = detector.cache(budget);

        println!(
            "{:>8}  {:>9}  {:>5} Mio",
            budget,
            budget - left,
            resident().saturating_sub(loaded) / 1024
        );
    }
}
