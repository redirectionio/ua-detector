//! Writes `src/warm.txt`, the user agents the crate ships to warm itself on.
//!
//!     cargo run --release --example warmlist -- ~/user_agent.csv [rows]
//!
//! The dump is the one `examples/traffic` reads: one row per distinct user agent, heaviest
//! first. The heaviest thousand of it are what a caller with no traffic of its own is best off
//! compiling, since a user agent that carries a million requests carries them for everyone.
//!
//! It is redirection.io's traffic and nobody else's, which is the honest thing to say about it:
//! another service's mix would warm on a different five thousand. It is still worth more than a
//! budget spent on the shape of the index, which belongs to no traffic at all -- and real traffic
//! resembles other real traffic far more than it resembles a corpus written to cover every device
//! ever made.
//!
//! Only the heaviest rows are taken, each of which carries a great many requests, so what ships
//! is the strings browsers and crawlers send rather than anything one visitor sent.
//!
//! The run ends on what the list buys, measured over rows of the same dump that it does not
//! hold -- the thousand just past the cut, and a stride over the whole tail behind them.

#[path = "compare/injector.rs"]
mod injector;

#[path = "traffic/row.rs"]
mod row;

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::Instant;

use device_detector::{Budget, Detector};

/// How many to keep, unless a second argument says otherwise. What each size buys, over the same
/// dump: what ships, what warming on it compiles, what `shared()` then takes to answer its first
/// question, and a detection over a stride across everything the list does not hold.
///
///      rows   shipped   compiled           start      tail
///     1 000    103 KiB   1 311, 21 MiB    ~1.3 s   1.32 ms
///     2 000    210 KiB   1 726, 28 MiB     1.7 s    963 µs
///     3 000    325 KiB   2 030, 35 MiB     2.1 s    843 µs
///     5 000    578 KiB   2 632, 45 MiB     3.1 s    659 µs
///
/// Two thousand is where this sits. The tail keeps improving past it, but warming walks the
/// index once per user agent whether it compiles anything or not, so the start column is a
/// straight line and a service that redeploys pays it on its first request.
const KEEP: usize = 2_000;

/// Rows read past the cut to measure against, and how many of them to keep from the tail.
const MEASURE: usize = 1_000;

fn main() {
    let path = std::env::args().nth(1).expect("usage: warmlist <dump.csv> [rows]");
    let keep = std::env::args().nth(2).and_then(|rows| rows.parse().ok()).unwrap_or(KEEP);
    let Dump { kept, near, tail, rows } = read(&path, keep);

    let into = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/warm.txt");
    let mut file = std::fs::File::create(&into).expect("src/warm.txt is writable");

    for agent in &kept {
        writeln!(file, "{agent}").expect("a line of it");
    }

    let size = file.metadata().map(|data| data.len()).unwrap_or_default();
    println!("{} of {rows} rows, {} KiB into {}", kept.len(), size / 1024, into.display());

    report(&kept, &near, &tail);
}

/// What the dump holds, without holding the dump: the heaviest rows go in the list, the ones
/// just past them and a stride over everything behind are what it is measured against, and the
/// other three and a half million are counted and dropped.
struct Dump {
    kept: Vec<String>,
    near: Vec<String>,
    tail: Vec<String>,
    rows: usize,
}

/// One row in a thousand is kept from the tail, which leaves a few thousand to take [`MEASURE`]
/// of rather than the whole file.
const TAIL: usize = 1_000;

fn read(path: &str, keep: usize) -> Dump {
    let file =
        std::fs::File::open(path).unwrap_or_else(|error| panic!("cannot read {path}: {error}"));
    let mut seen = HashSet::new();
    let mut dump = Dump { kept: Vec::new(), near: Vec::new(), tail: Vec::new(), rows: 0 };

    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let Some(fields) = row::fields(line.trim_end_matches('\n')) else {
            continue;
        };

        let Some(agent) = fields.into_iter().next().filter(|agent| !agent.trim().is_empty()) else {
            continue;
        };

        dump.rows += 1;

        match dump.kept.len() {
            // The dump is one row per distinct user agent, but the list is what ships, so it is
            // the one place worth making sure of.
            length if length < keep => {
                if seen.insert(agent.clone()) {
                    dump.kept.push(agent);
                }
            }
            _ if dump.near.len() < MEASURE => dump.near.push(agent),
            _ if dump.rows % TAIL == 0 => dump.tail.push(agent),
            _ => {}
        }
    }

    dump
}

/// What the list costs, and what a detection costs with it, over rows it does not hold.
///
/// Two samples, because they are two different questions: the rows just past the cut are what a
/// user agent one build behind the traffic costs, and a stride over the tail is what the
/// long tail of crawlers and oddities costs. Neither is what production pays, since the answers
/// cache takes nine requests in ten before any of this is reached.
fn report(kept: &[String], near: &[String], tail: &[String]) {
    let budget = Budget::bytes(400 << 20);
    let mut detector = Detector::new();
    let left = detector.warm(kept, budget);

    println!(
        "warming      {} regexes, {} MiB, {} of {budget} left over",
        detector.compiled(),
        detector.compiled_size() / (1 << 20),
        left,
    );

    let stride = (tail.len() / MEASURE).max(1);
    let tail: Vec<&String> = tail.iter().step_by(stride).take(MEASURE).collect();
    let near: Vec<&String> = near.iter().collect();

    for (name, sample) in [("just past the cut", near), ("across the tail", tail)] {
        let started = Instant::now();
        let answered = sample.iter().filter(|agent| detector.detect(agent).is_some()).count();
        let each = started.elapsed() / sample.len().max(1) as u32;

        println!("{name:>18}   {each:?} a detection over {} rows, {answered} answered", sample.len());
    }
}
