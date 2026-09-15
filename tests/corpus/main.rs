//! Every user agent the fixtures name, one case at a time.
//!
//! `tests/matomo-device-detector/fixtures/*.yml` is the specification. The cases are read at
//! startup and handed to the harness by `libtest-mimic`, so nothing about the corpus is
//! compiled and the names and filters read as a `#[test]` would:
//!
//!     cargo test --release --no-default-features --test corpus
//!     cargo test --release --no-default-features --test corpus bots::
//!     cargo test --release --no-default-features --test corpus -- --exact smartphone_2::38

#[path = "../../examples/all/fixtures.rs"]
mod fixtures;

use std::sync::Arc;

use libtest_mimic::{Arguments, Failed, Trial};

fn main() {
    let arguments = Arguments::from_args();
    let mut trials = Vec::new();

    for path in fixtures::files() {
        let name = path.file_stem().expect("a file that has a name").to_string_lossy().to_string();
        let cases = Arc::new(fixtures::read(&path));

        for index in 0..cases.len() {
            let cases = Arc::clone(&cases);

            // One-based, as the generated tests were, so a case keeps the name it had.
            trials.push(Trial::test(format!("{name}::{}", index + 1), move || check(&cases[index])));
        }
    }

    libtest_mimic::run(&arguments, trials).exit();
}

fn check(case: &fixtures::Case) -> Result<(), Failed> {
    let headers = case.headers();
    let headers: Vec<(&str, &str)> =
        headers.iter().map(|(name, value)| (name.as_str(), value.as_str())).collect();

    let found = device_detector::shared().detect_with_headers(&case.user_agent, &headers);
    let expected = case.expected();

    if found.as_ref() == Some(&expected) {
        return Ok(());
    }

    Err(format!("{}\n  expected {expected:?}\n     found {found:?}", case.user_agent).into())
}
