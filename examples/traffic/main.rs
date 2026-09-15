//! Every user agent production has seen, heaviest first, against what this library answers.
//!
//! The dump is one row per distinct user agent -- the string, the name and the type production
//! gave it, and how many requests carried it -- heaviest first:
//!
//!     cargo run --release --example traffic -- ~/user_agent.csv
//!
//! It stops at the first row it cannot answer as production did and writes that row to
//! `tests/pinned/traffic/cases.csv`, from which the test beside it is generated, so the cases
//! accumulate in the order the traffic cares about.
//!
//! Production runs a device detector some years old, so a divergence is not automatically this
//! library's to fix: a bot released since, an application it never knew, a name it got wrong.
//! Those go in `tests/pinned/traffic/false-negatives.yml`, which says which rows to excuse and
//! why.
//!
//!     --false-negatives P  what to excuse production for
//!     --cases P            where the failing rows accumulate
//!     --tests P            the test generated from them
//!     --budget N           regexes to compile up front (default 20000)
//!     --warm N             rows to spend that budget on, the heaviest first (default 5000)
//!     --limit N            stop after N rows
//!     --keep-going         survey every divergence rather than stopping at the first, and
//!                          report them grouped rather than writing a case for each
//!     --top N              how many groups a --keep-going run prints (default 40)

#[path = "../compare/injector.rs"]
mod injector;

mod cases;
mod excuses;
mod row;

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::ExitCode;

use device_detector::Detector;
use indicatif::{ProgressBar, ProgressStyle};

use crate::excuses::Excuses;
use crate::injector::{Kind, read};
use crate::row::Row;

/// Rows read before the batch is handed to the threads. Enough to keep them all busy, small
/// enough that stopping at the first divergence does not first detect a hundred thousand rows
/// that come after it.
const BATCH: usize = 4096;

struct Options {
    path: String,
    false_negatives: String,
    cases: String,
    tests: String,
    budget: u64,
    warm: usize,
    limit: usize,
    keep_going: bool,
    top: usize,
}

/// Rows, and the requests behind them. A hundred rows of a user agent nobody sends are worth
/// less than one of `Chrome/152`, which is the whole reason the dump carries a count.
#[derive(Clone, Copy, Default)]
struct Tally {
    rows: u64,
    requests: u64,
}

impl Tally {
    fn add(&mut self, row: &Row) {
        self.rows += 1;
        self.requests += row.count;
    }

    fn merge(&mut self, other: Tally) {
        self.rows += other.rows;
        self.requests += other.requests;
    }
}

/// A row this library answers differently from production, and what it answered.
struct Divergence {
    row: Row,
    found_name: String,
    found_kind: Kind,
}

fn main() -> ExitCode {
    let Some(options) = options() else {
        eprintln!(
            "usage: traffic <dump.csv> [--false-negatives P] [--cases P] [--tests P] \
             [--budget N] [--warm N] [--limit N] [--keep-going] [--top N]"
        );

        return ExitCode::FAILURE;
    };

    let excuses = match Excuses::read(&options.false_negatives) {
        Ok(excuses) => excuses,
        Err(error) => {
            eprintln!("cannot read {}: {error}", options.false_negatives);

            return ExitCode::FAILURE;
        }
    };

    let mut detector = Detector::new();
    let warmed = match warm(&mut detector, &options) {
        Ok(warmed) => warmed,
        Err(error) => {
            eprintln!("cannot read {}: {error}", options.path);

            return ExitCode::FAILURE;
        }
    };

    println!(
        "{warmed} regexes compiled from the {} heaviest rows, {} excuse{} for production",
        options.warm,
        excuses.len(),
        if excuses.len() == 1 { "" } else { "s" },
    );

    let (divergences, agreed, excused) = match compare(&detector, &excuses, &options) {
        Ok(pass) => pass,
        Err(error) => {
            eprintln!("cannot read {}: {error}", options.path);

            return ExitCode::FAILURE;
        }
    };

    if options.keep_going {
        summarise(&divergences, options.top);
    } else {
        for divergence in &divergences {
            report(divergence);
        }
    }

    // A `--keep-going` run is a survey rather than a fix, and four hundred cases written in one
    // go are a list nobody works through. It still rewrites the test from the cases already
    // there, which is what keeps the two in step.
    let record: &[Divergence] = if options.keep_going { &[] } else { &divergences };

    match cases::record(&options.cases, &options.tests, record) {
        Ok(total) => {
            println!("{} holds {total} case(s)", options.cases);

            weigh(&divergences, agreed, &excused);
        }
        Err(error) => {
            eprintln!("cannot write the case: {error}");

            return ExitCode::FAILURE;
        }
    }

    if divergences.is_empty() {
        println!("every row reads as production reads it");

        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn options() -> Option<Options> {
    let mut arguments = std::env::args().skip(1);
    let mut path = None;
    let mut options = Options {
        path: String::new(),
        false_negatives: String::from("tests/pinned/traffic/false-negatives.yml"),
        cases: String::from("tests/pinned/traffic/cases.csv"),
        tests: String::from("tests/pinned/traffic.rs"),
        budget: 100_000,
        warm: 0,
        limit: usize::MAX,
        keep_going: false,
        top: 40,
    };

    while let Some(argument) = arguments.next() {
        let mut text =
            |name: &str| arguments.next().unwrap_or_else(|| panic!("{name} wants a path"));

        match argument.as_str() {
            "--false-negatives" => options.false_negatives = text("--false-negatives"),
            "--cases" => options.cases = text("--cases"),
            "--tests" => options.tests = text("--tests"),
            "--budget" => options.budget = text("--budget").parse().expect("a number"),
            "--warm" => options.warm = text("--warm").parse().expect("a number"),
            "--limit" => options.limit = text("--limit").parse().expect("a number"),
            "--keep-going" => options.keep_going = true,
            "--top" => options.top = text("--top").parse().expect("a number"),
            _ => path = Some(argument),
        }
    }

    options.path = path?;

    Some(options)
}

/// Spends the budget on the user agents the traffic actually holds, which is the whole point of
/// having them in order: the heaviest rows are the ones every later row is measured against.
fn warm(detector: &mut Detector, options: &Options) -> std::io::Result<u64> {
    if options.warm == 0 {
        return Ok(detector.cache(options.budget));
    }

    let file = BufReader::new(std::fs::File::open(&options.path)?);
    let heaviest: Vec<String> = file
        .lines()
        .take(options.warm)
        .map_while(Result::ok)
        .filter_map(|line| Row::parse(&line, 0).map(|row| row.user_agent))
        .collect();

    Ok(detector.warm(heaviest.iter().map(String::as_str), options.budget))
}

fn compare(
    detector: &Detector,
    excuses: &Excuses,
    options: &Options,
) -> std::io::Result<(Vec<Divergence>, Tally, Vec<Tally>)> {
    let total = std::fs::metadata(&options.path)?.len();
    let mut file = BufReader::new(std::fs::File::open(&options.path)?);
    let progress = ProgressBar::new(total);

    progress.set_style(
        ProgressStyle::with_template(
            "{bar:40} {bytes}/{total_bytes} {percent:>3}% {eta:>5} left  {msg}",
        )
        .expect("a template that parses"),
    );

    let (mut read_bytes, mut rows) = (0u64, 0usize);
    let mut agreed = Tally::default();
    let mut excused = vec![Tally::default(); excuses.len()];
    let mut divergences = Vec::new();
    let mut batch: Vec<Row> = Vec::with_capacity(BATCH);
    let mut line = String::new();

    loop {
        line.clear();

        let bytes = file.read_line(&mut line)?;

        read_bytes += bytes as u64;

        if bytes > 0 && rows < options.limit {
            rows += 1;

            if let Some(row) = Row::parse(line.trim_end_matches('\n'), rows) {
                batch.push(row);
            }
        }

        let last = bytes == 0 || rows >= options.limit;

        if batch.len() < BATCH && !last {
            continue;
        }

        let (found, agreements, excusals) = judge(detector, excuses, &batch);

        agreed.merge(agreements);
        divergences.extend(found);

        for (total, excusals) in excused.iter_mut().zip(excusals) {
            total.merge(excusals);
        }

        batch.clear();

        progress.set_position(read_bytes);
        progress.set_message(format!(
            "{rows} rows, {} agree, {} excused, {} wrong",
            agreed.rows,
            excused.iter().map(|tally| tally.rows).sum::<u64>(),
            divergences.len()
        ));

        if last || (!divergences.is_empty() && !options.keep_going) {
            break;
        }
    }

    progress.finish_and_clear();

    let forgiven: u64 = excused.iter().map(|tally| tally.rows).sum();
    let compared = agreed.rows + forgiven + divergences.len() as u64;

    println!(
        "{rows} rows, {compared} compared, {} agree, {forgiven} excused production, {} wrong",
        agreed.rows,
        divergences.len()
    );

    // Which excuse carried how many rows: an entry that suddenly answers for a hundred thousand
    // of them is excusing something it was not written for.
    for (excuse, tally) in excused.iter().enumerate().filter(|(_, tally)| tally.rows > 0) {
        println!(
            "{:>7} rows {:>12} requests  {}",
            tally.rows,
            tally.requests,
            excuses.reason(excuse)
        );
    }

    // A batch is judged whole, so stopping at the first divergence can still bring back several.
    if !options.keep_going {
        divergences.truncate(1);
    }

    Ok((divergences, agreed, excused))
}

/// Detects a batch across every core there is, and puts the answers back in the order of the
/// dump: the row that stops the run has to be the heaviest one, not the first a thread reached.
fn judge(
    detector: &Detector,
    excuses: &Excuses,
    batch: &[Row],
) -> (Vec<Divergence>, Tally, Vec<Tally>) {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let chunk = batch.len().div_ceil(threads).max(1);
    let mut divergences = Vec::new();
    let mut agreed = Tally::default();
    let mut excused = vec![Tally::default(); excuses.len()];

    std::thread::scope(|scope| {
        let handles: Vec<_> = batch
            .chunks(chunk)
            .map(|chunk| {
                scope.spawn(move || {
                    let mut divergences = Vec::new();
                    let mut agreed = Tally::default();
                    let mut excused = vec![Tally::default(); excuses.len()];

                    for row in chunk {
                        let (found_kind, found_name) = read(detector, &row.user_agent);

                        if found_name == row.name && found_kind == row.kind {
                            agreed.add(row);
                        } else if let Some(excuse) = excuses.excuses(row, &found_name, found_kind) {
                            excused[excuse].add(row);
                        } else {
                            divergences.push(Divergence {
                                row: row.clone(),
                                found_name,
                                found_kind,
                            });
                        }
                    }

                    (divergences, agreed, excused)
                })
            })
            .collect();

        for handle in handles {
            let (found, agreements, excusals) = handle.join().expect("a chunk that did not panic");

            divergences.extend(found);
            agreed.merge(agreements);

            for (total, excusals) in excused.iter_mut().zip(excusals) {
                total.merge(excusals);
            }
        }
    });

    divergences.sort_by_key(|divergence| divergence.row.number);

    (divergences, agreed, excused)
}

fn report(divergence: &Divergence) {
    let Divergence { row, found_name, found_kind } = divergence;

    println!();
    println!("row {}, {} requests", row.number, row.count);
    println!("  {}", row.user_agent);
    println!("  production  {} [{}]", row.name, row.kind.name());
    println!("  this        {found_name} [{}]", found_kind.name());
    println!();
    println!("  cargo run --release --example trace -- {:?}", row.user_agent);
}

/// What a `--keep-going` run has to say: one line per pair of answers, the traffic behind it
/// summed, heaviest first. Eight hundred divergences read one at a time say nothing; the same
/// eight hundred grouped say which single entry is worth writing next.
fn summarise(divergences: &[Divergence], top: usize) {
    /// What production said and what this library said, against the requests behind the pair,
    /// the user agents that showed it, and one of them to reproduce it with.
    type Groups<'a> = HashMap<(&'a str, Kind, &'a str, Kind), (u64, u64, &'a str)>;

    let mut groups = Groups::new();

    for divergence in divergences {
        let key = (
            divergence.row.name.as_str(),
            divergence.row.kind,
            divergence.found_name.as_str(),
            divergence.found_kind,
        );
        let group = groups.entry(key).or_insert((0, 0, divergence.row.user_agent.as_str()));

        group.0 += divergence.row.count;
        group.1 += 1;
    }

    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_unstable_by_key(|((name, ..), (count, ..))| (std::cmp::Reverse(*count), *name));

    println!();
    println!("{} groups, heaviest first", groups.len());

    for ((name, kind, found_name, found_kind), (count, rows, sample)) in groups.iter().take(top) {
        println!();
        println!(
            "{count:>12} requests over {rows} user agent(s)  {name} [{}]  ->  {found_name} [{}]",
            kind.name(),
            found_kind.name(),
        );
        println!("             {sample}");
    }

    if groups.len() > top {
        println!();
        println!("{:>12}  … {} more", "", groups.len() - top);
    }
}

/// What the pass came to in requests rather than in rows, which is the number that says whether
/// a change was worth making: one user agent of `Chrome/152` outweighs a thousand nobody sends.
fn weigh(divergences: &[Divergence], agreed: Tally, excused: &[Tally]) {
    let forgiven = excused.iter().fold(Tally::default(), |mut total, tally| {
        total.merge(*tally);

        total
    });
    let wrong = divergences.iter().fold(Tally::default(), |mut total, divergence| {
        total.add(&divergence.row);

        total
    });
    let total = agreed.requests + forgiven.requests + wrong.requests;
    let share =
        |requests: u64| if total == 0 { 0.0 } else { requests as f64 * 100.0 / total as f64 };

    println!();
    println!("{total:>14} requests behind the rows compared");
    println!("{:>14} agree ({:.3}%)", agreed.requests, share(agreed.requests));
    println!(
        "{:>14} excused production ({:.3}%)",
        forgiven.requests,
        share(forgiven.requests)
    );
    println!("{:>14} wrong ({:.3}%)", wrong.requests, share(wrong.requests));
}
