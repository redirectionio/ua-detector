//! The rows this library answers wrongly, and the test generated from them.
//!
//! `tests/pinned/traffic/cases.csv` holds them in the shape of the dump they came from, heaviest
//! first, so the test reads in the order the traffic cares about. `tests/pinned/traffic.rs` is
//! generated from it and is output, not source.

use std::io::Write;

use crate::Divergence;
use crate::row::{fields, quote};

/// One case: a row of the dump, kept verbatim.
struct Case {
    user_agent: String,
    name: String,
    kind: u16,
    count: u64,
}

/// Adds the divergences to the case file and writes the test again from all of them.
///
/// Returns how many cases the file holds. A user agent already in it is replaced rather than
/// added twice: a run reaching it again means the count moved, not that there are two of it.
pub fn record(cases: &str, tests: &str, divergences: &[Divergence]) -> std::io::Result<usize> {
    let mut kept = read(cases)?;

    for divergence in divergences {
        let row = &divergence.row;

        kept.retain(|case| case.user_agent != row.user_agent);
        kept.push(Case {
            user_agent: row.user_agent.clone(),
            name: row.name.clone(),
            kind: row.kind as u16,
            count: row.count,
        });
    }

    kept.sort_by_key(|case| std::cmp::Reverse(case.count));

    if let Some(parent) = std::path::Path::new(cases).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = std::fs::File::create(cases)?;

    for case in &kept {
        writeln!(
            file,
            "{},{},{},{}",
            quote(&case.user_agent),
            quote(&case.name),
            case.kind,
            case.count
        )?;
    }

    std::fs::write(tests, source(&kept))?;

    Ok(kept.len())
}

fn read(path: &str) -> std::io::Result<Vec<Case>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };

    Ok(text
        .lines()
        .filter_map(|line| {
            let fields = fields(line)?;
            let [user_agent, name, kind, count] = fields.as_slice() else {
                return None;
            };

            Some(Case {
                user_agent: user_agent.clone(),
                name: name.clone(),
                kind: kind.parse().ok()?,
                count: count.parse().ok()?,
            })
        })
        .collect())
}

fn source(cases: &[Case]) -> String {
    let mut out = String::from(
        r#"//! User agents production answers differently from this library, heaviest first.
//!
//! Written by `cargo run --release --example traffic -- <dump.csv>` from the case file next to
//! this one. Regenerate it rather than editing it. The two columns are what production wrote
//! for the user agent, its type being the `DeviceType` of the log injector that
//! `examples/compare/injector.rs` reads a detection into.

#[path = "../../examples/compare/injector.rs"]
mod injector;

use injector::{Kind, read};

#[test]
fn the_heaviest_traffic_reads_as_production_reads_it() {
    // Typed, because the file is generated and starts out with nothing in it.
    let cases: &[(&str, &str, u16)] = &[
"#,
    );

    for case in cases {
        out.push_str(&format!(
            "        // {} requests\n        ({}, {}, {}),\n",
            case.count,
            literal(&case.user_agent),
            literal(&case.name),
            case.kind
        ));
    }

    out.push_str(
        r#"    ];

    let detector = device_detector::shared();
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(user_agent, name, kind)| {
            let want = Kind::from_column(*kind).expect("a type the log injector writes");
            let (found_kind, found_name) = read(detector, user_agent);

            (found_name != *name || found_kind != want).then(|| {
                format!(
                    "\n  {name} [{}] read as {found_name} [{}]\n    {user_agent}",
                    want.name(),
                    found_kind.name(),
                )
            })
        })
        .collect();

    assert!(wrong.is_empty(), "{} of {} still wrong{}", wrong.len(), cases.len(), wrong.join(""));
}
"#,
    );

    out
}

/// A rust raw string literal, with enough hashes to survive whatever the dump holds.
fn literal(value: &str) -> String {
    let hashes = std::iter::successors(Some(String::new()), |hashes| Some(format!("{hashes}#")))
        .find(|hashes| !value.contains(&format!("\"{hashes}")))
        .expect("a value shorter than infinity");

    format!("r{hashes}\"{value}\"{hashes}")
}
