//! What this library answers, against what rust-device-detector answered in production.
//!
//! redirection.io's log injector stores two columns per request, `user_agent_simplified` and
//! `user_agent_type`, both derived from a detection. This replays the same derivation on a dump
//! of those columns and reports where the two libraries disagree.
//!
//!     clickhouse-client --query "
//!         SELECT user_agent, user_agent_simplified, user_agent_type
//!         FROM ua_compare.user_agent FORMAT TSV" > /tmp/ua_compare.tsv
//!     cargo run --release --example compare -- /tmp/ua_compare.tsv
//!
//!     --top N        how many divergence groups to print, per category (default 40)
//!     --limit N      stop after N rows
//!     --budget N     regexes to compile up front (default 200000, all of them)
//!     --unmatched P  write every user agent no entry covers to P, one per line

mod injector;

use std::collections::HashMap;
use std::process::ExitCode;

use device_detector::Detector;
use injector::{Kind, classify};

/// Which of the two columns disagree, which is also how the report is split.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Divergence {
    /// No entry covers the user agent at all.
    Unmatched,
    Name,
    Type,
    Both,
}

impl Divergence {
    const ALL: [Divergence; 4] =
        [Divergence::Unmatched, Divergence::Name, Divergence::Type, Divergence::Both];

    fn title(self) -> &'static str {
        match self {
            Divergence::Unmatched => "no entry matches the user agent",
            Divergence::Name => "a different name, same type",
            Divergence::Type => "a different type, same name",
            Divergence::Both => "a different name and a different type",
        }
    }
}

/// One divergence, counted over every row that shows it, with a user agent to reproduce it.
#[derive(Default)]
struct Group {
    count: u64,
    sample: String,
}

#[derive(Default)]
struct Report {
    rows: u64,
    agreed: u64,
    /// Rows the log injector rewrote after a reverse DNS lookup we cannot replay, counted
    /// apart rather than reported as divergences.
    googlebot: u64,
    groups: HashMap<(Divergence, String, Kind, String, Kind), Group>,
    /// Only filled when the run is asked to dump them.
    unmatched: Vec<String>,
    /// Rows that diverged, verbatim, so a run can be narrowed to what is still wrong.
    errors: Vec<String>,
}

impl Report {
    fn merge(&mut self, other: Report) {
        self.rows += other.rows;
        self.agreed += other.agreed;
        self.googlebot += other.googlebot;

        self.unmatched.extend(other.unmatched);
        self.errors.extend(other.errors);

        for (key, group) in other.groups {
            let entry = self.groups.entry(key).or_default();

            entry.count += group.count;

            if entry.sample.is_empty() {
                entry.sample = group.sample;
            }
        }
    }
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let mut path = None;
    let (mut top, mut limit, mut budget) = (40usize, usize::MAX, 200_000u64);
    let mut unmatched = None;
    let mut errors = None;

    while let Some(argument) = arguments.next() {
        let mut value = |name: &str| {
            arguments
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(|| panic!("{name} wants a number"))
        };

        match argument.as_str() {
            "--top" => top = value("--top") as usize,
            "--limit" => limit = value("--limit") as usize,
            "--budget" => budget = value("--budget"),
            "--unmatched" => unmatched = arguments.next(),
            "--errors" => errors = arguments.next(),
            _ => path = Some(argument),
        }
    }

    let Some(path) = path else {
        eprintln!(
            "usage: compare <dump.tsv> [--top N] [--limit N] [--budget N] [--unmatched PATH] [--errors PATH]"
        );

        return ExitCode::FAILURE;
    };

    let dump = match std::fs::read_to_string(&path) {
        Ok(dump) => dump,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");

            return ExitCode::FAILURE;
        }
    };

    let mut detector = Detector::new();
    detector.cache(device_detector::Budget::regexes(budget));

    let lines: Vec<&str> = dump.lines().take(limit).collect();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let chunk = lines.len().div_ceil(threads).max(1);
    let mut report = Report::default();

    std::thread::scope(|scope| {
        let handles: Vec<_> = lines
            .chunks(chunk)
            .map(|chunk| scope.spawn(|| run(&detector, chunk, unmatched.is_some(), errors.is_some())))
            .collect();

        for handle in handles {
            report.merge(handle.join().expect("a chunk that did not panic"));
        }
    });

    print(&report, top);

    for (path, dump) in [(unmatched, &report.unmatched), (errors, &report.errors)] {
        let Some(path) = path else {
            continue;
        };

        if let Err(error) = std::fs::write(&path, dump.join("\n")) {
            eprintln!("cannot write {path}: {error}");

            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

/// Names the log injector gives when it recognises nothing in particular, which is also what an
/// older device detector gives for a client released after it. A row where the two libraries
/// differ only because this one knows a newer name is not a row to go and fix.
const UNINFORMATIVE: [&str; 2] = ["Generic Bot", "Amazon Bot"];

/// What a browser is called when nothing more particular is known about it. The dump comes from
/// a device detector some years old, so where it gives one of these and this library gives an
/// application, the difference is almost always an app released since -- `com.google.Maps` read
/// as Mobile Safari, `ChatGPT/1.2026` read as Mobile Safari. Those are not rows to go and fix
/// either, and there are enough of them to bury the ones that are.
const GENERIC: [&str; 9] = [
    "Chrome", "Chrome Mobile", "Chrome Webview", "Chrome Mobile iOS", "Safari", "Mobile Safari",
    "Firefox", "Android Browser", "Microsoft Edge",
];

fn run(detector: &Detector, lines: &[&str], keep_unmatched: bool, keep_errors: bool) -> Report {
    let mut report = Report::default();

    for line in lines {
        let mut fields = line.split('\t');
        let (Some(user_agent), Some(simplified), Some(kind), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            continue;
        };

        let user_agent = unescape(user_agent);
        let expected_name = unescape(simplified);
        let Some(expected_kind) = kind.parse().ok().and_then(Kind::from_column) else {
            continue;
        };

        report.rows += 1;

        // The log injector replaces any name holding "googlebot" with the verdict of a reverse
        // DNS lookup on the client address, which the dump does not carry.
        if expected_name.starts_with("Googlebot") {
            report.googlebot += 1;

            continue;
        }

        let detection = detector.detect(&user_agent);
        let (found_kind, found_name) = match &detection {
            Some(detection) => classify(detection, &user_agent),
            // What the log injector does with a detection error, and the closest thing to it.
            None => (Kind::Tool, user_agent.clone()),
        };

        if found_name == expected_name && found_kind == expected_kind {
            report.agreed += 1;

            continue;
        }

        let divergence = match (detection.is_none(), found_name == expected_name) {
            (true, _) => Divergence::Unmatched,
            (false, true) => Divergence::Type,
            (false, false) if found_kind == expected_kind => Divergence::Name,
            (false, false) => Divergence::Both,
        };
        if keep_unmatched && detection.is_none() {
            report.unmatched.push(user_agent.clone());
        }

        // The row as it was read, so the file a run writes is a dump the next run can be given.
        // What the log injector could not name either is left out: those are not this library's
        // to answer, and on a dump this old most of them are clients it simply predates.
        let newer = GENERIC.contains(&expected_name.as_str())
            && !GENERIC.contains(&found_name.as_str())
            && detection.is_some();

        if keep_errors && !UNINFORMATIVE.contains(&expected_name.as_str()) && !newer {
            report.errors.push((*line).to_string());
        }

        // A user agent of its own is no divergence anyone can act on: the group would hold one
        // row and the report thousands of them.
        let found_name = if detection.is_none() { String::from("<unmatched>") } else { found_name };
        let expected_name = if divergence == Divergence::Unmatched && expected_name == user_agent {
            String::from("<the user agent itself>")
        } else {
            expected_name
        };

        let group = report
            .groups
            .entry((divergence, expected_name, expected_kind, found_name, found_kind))
            .or_default();

        group.count += 1;

        if group.sample.is_empty() {
            group.sample = user_agent;
        }
    }

    report
}

/// ClickHouse's TSV escaping, which is the only thing between a user agent and a line.
fn unescape(field: &str) -> String {
    if !field.contains('\\') {
        return field.to_string();
    }

    let mut unescaped = String::with_capacity(field.len());
    let mut characters = field.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            unescaped.push(character);

            continue;
        }

        match characters.next() {
            Some('n') => unescaped.push('\n'),
            Some('t') => unescaped.push('\t'),
            Some('r') => unescaped.push('\r'),
            Some('b') => unescaped.push('\u{8}'),
            Some('f') => unescaped.push('\u{c}'),
            Some('0') => unescaped.push('\0'),
            Some(other) => unescaped.push(other),
            None => unescaped.push('\\'),
        }
    }

    unescaped
}

fn print(report: &Report, top: usize) {
    let diverging: u64 = report.groups.values().map(|group| group.count).sum();
    let compared = report.rows - report.googlebot;
    let share = |count: u64| {
        if compared == 0 { 0.0 } else { count as f64 * 100.0 / compared as f64 }
    };

    println!("{:>12} rows", report.rows);
    println!("{:>12} skipped, the log injector rewrote the name from a reverse DNS lookup", report.googlebot);
    println!("{:>12} compared", compared);
    println!("{:>12} agree ({:.2}%)", report.agreed, share(report.agreed));
    println!("{:>12} diverge ({:.2}%)", diverging, share(diverging));

    for divergence in Divergence::ALL {
        let mut groups: Vec<_> = report
            .groups
            .iter()
            .filter(|((kind, ..), _)| *kind == divergence)
            .map(|((_, expected_name, expected_kind, found_name, found_kind), group)| {
                (group.count, expected_name, expected_kind, found_name, found_kind, &group.sample)
            })
            .collect();

        if groups.is_empty() {
            continue;
        }

        let total: u64 = groups.iter().map(|(count, ..)| count).sum();

        groups.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(right.1)));

        println!();
        println!(
            "{} — {total} rows ({:.2}%), {} distinct",
            divergence.title(),
            share(total),
            groups.len()
        );
        println!();

        for (count, expected_name, expected_kind, found_name, found_kind, sample) in
            groups.iter().take(top)
        {
            println!(
                "{count:>10}  {expected_name} [{}]  ->  {found_name} [{}]",
                expected_kind.name(),
                found_kind.name(),
            );
            println!("            {sample}");
        }

        if groups.len() > top {
            println!("{:>10}  … {} more", "", groups.len() - top);
        }
    }
}
