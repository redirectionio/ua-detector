//! Reports what a fixture file still expects that the entries do not produce.
//!
//!     cargo run --example report -- smart_display          # only what fails
//!     cargo run --example report -- smart_display --shapes # failures grouped by shape

#[path = "all/fixtures.rs"]
mod fixtures;

use std::collections::BTreeMap;

fn main() {
    let mut args = std::env::args().skip(1);
    let name = args.next().expect("usage: report <fixture> [--shapes]");
    let shapes = args.any(|arg| arg == "--shapes");

    let cases = fixtures::read(std::path::Path::new(&format!("{}/{name}.yml", fixtures::FIXTURES)));

    let detector = ua_detector::shared();
    let mut failures = Vec::new();

    for case in &cases {
        let expected = case.expected();
        let headers = case.headers();
        let headers: Vec<(&str, &str)> =
            headers.iter().map(|(name, value)| (name.as_str(), value.as_str())).collect();
        let detected = detector.detect_with_headers(&case.user_agent, &headers);

        if detected.as_ref() != Some(&expected) {
            failures.push((case, expected, detected));
        }
    }

    if shapes {
        let mut grouped: BTreeMap<String, usize> = BTreeMap::new();

        for (case, _, _) in &failures {
            *grouped.entry(shape_of(&case.user_agent)).or_default() += 1;
        }

        for (shape, count) in &grouped {
            println!("{count:>5}  {shape}");
        }

        println!("\n{} shape(s), {}/{} case(s) failing", grouped.len(), failures.len(), cases.len());

        return;
    }

    for (case, expected, detected) in &failures {
        println!("UA       {}", case.user_agent);
        println!("expected {expected:?}");

        match detected {
            None => println!("got      nothing\n"),
            Some(detected) => println!("got      {detected:?}\n"),
        }
    }

    println!("{}/{} case(s) failing", failures.len(), cases.len());
}

/// Strips the parts of a user agent that vary between otherwise identical cases, so failures
/// can be counted per shape rather than one by one.
fn shape_of(user_agent: &str) -> String {
    let mut shape = String::with_capacity(user_agent.len());
    let mut chars = user_agent.chars().peekable();

    while let Some(char) = chars.next() {
        if !char.is_ascii_digit() {
            shape.push(char);
            continue;
        }

        while chars.peek().is_some_and(|next| next.is_ascii_digit() || *next == '.') {
            chars.next();
        }

        shape.push('#');
    }

    shape
}
