//! Every fixture case, in one process, with a count per file and an exit code.
//!
//!     cargo run --release --example all
//!
//! `cargo test --test corpus` asserts the same thing one case at a time, which is what you want
//! to read a failure. This is the count, and an exit code to go with it.

mod fixtures;

use std::process::ExitCode;

use crate::fixtures::Case;

fn main() -> ExitCode {
    let mut detector = device_detector::Detector::new();
    detector.cache(device_detector::Budget::regexes(200_000));

    let (mut total, mut failing) = (0usize, 0usize);

    for path in fixtures::files() {
        let cases = fixtures::read(&path);
        let bad = cases.iter().filter(|case| !passes(&detector, case)).count();

        total += cases.len();
        failing += bad;

        if bad > 0 {
            println!("{bad:>5}/{:<5} {}", cases.len(), path.file_stem().unwrap().to_string_lossy());
        }
    }

    println!("\n{}/{total} cases pass", total - failing);

    if failing == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn passes(detector: &device_detector::Detector, case: &Case) -> bool {
    let headers = case.headers();
    let headers: Vec<(&str, &str)> = headers
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();

    detector.detect_with_headers(&case.user_agent, &headers).as_ref() == Some(&case.expected())
}
