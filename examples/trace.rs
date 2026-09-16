//! Where one detection spends itself, regex by regex.
//!
//!     cargo run --release --example trace -- "<user agent>"
//!     cargo run --release --example trace -- "<user agent>" --budget 0 --top 20
//!     cargo run --release --example trace -- "<user agent>" --header sec-ch-ua-platform=Android
//!     cargo run --release --example trace -- "<user agent>" --warm   # budget spent on this one
//!
//! The plain run at the top is the number to quote. Everything under it is the instrumented
//! pass, which reads the clock twice per regex and so totals a little higher: it is there for
//! the shares, not for the sums.

use std::time::Duration;

use ua_detector::{DetectionTrace, MatchPath, RuleTrace, Trace};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let mut user_agent = None;
    let mut headers = Vec::new();
    let mut budget = ua_detector::Budget::regexes(1_000);
    let mut top = 10;
    let mut tree = false;
    let mut warm = false;

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--budget" => budget = arguments.next().and_then(|v| v.parse().ok()).unwrap_or(budget),
            "--top" => top = arguments.next().and_then(|v| v.parse().ok()).unwrap_or(top),
            "--tree" => tree = true,
            "--warm" => warm = true,
            "--header" => {
                let value = arguments.next().unwrap_or_default();
                let (name, value) = value.split_once('=').expect("--header name=value");

                headers.push((name.to_string(), value.to_string()));
            }
            _ => user_agent = Some(argument),
        }
    }

    let user_agent = user_agent.expect("usage: trace <user agent> [--budget n] [--top n] [--tree]");
    let headers: Vec<(&str, &str)> =
        headers.iter().map(|(name, value)| (name.as_str(), value.as_str())).collect();

    let mut detector = ua_detector::Detector::new();
    let left = if warm {
        detector.warm([user_agent.as_str()], budget)
    } else {
        detector.cache(budget)
    };
    let trace = detector.trace(&user_agent, &headers);

    println!("user agent   {}", trace.user_agent);
    println!("entries      {}, indexed under {} keys", detector.len(), detector.keys());
    println!(
        "budget       {} spent {}, {left} left over",
        budget.take(left.spent_from(budget)),
        if warm { "on this user agent" } else { "blind" },
    );

    match &trace.detection {
        None => println!("detection    nothing"),
        Some(detection) => println!("detection    {detection:?}"),
    }

    totals(&trace);
    index(&trace, top, tree);
    candidates(&trace, top);
    wildcards(&trace, &detector);
}

/// What the plain run cost, and how it splits between narrowing the database down and reading
/// the entries that survived.
fn totals(trace: &DetectionTrace) {
    let apply = trace.apply();
    let index = trace.index.elapsed();
    let instrumented = index + apply;

    println!("\nwhere the time goes          plain {}", time(trace.detect));
    println!("  index          {}  {}", time(trace.find), share(index, instrumented));
    println!(
        "  candidates     {}  {}",
        time(trace.detect.saturating_sub(trace.find)),
        share(apply, instrumented),
    );
    println!(
        "  of which compiling regexes outside the budget  {}  {}",
        time(trace.compile()),
        share(trace.compile(), instrumented),
    );
}

/// The walk over the index: how many regexes it ran, by which route, and the heaviest of them.
fn index(trace: &DetectionTrace, top: usize, print_tree: bool) {
    let walk = &trace.index;

    println!(
        "\nindex walk                   {} over {} regexes, {} matched",
        time(walk.elapsed()),
        walk.visited(),
        count_matched(walk),
    );

    for path in [
        MatchPath::Prefiltered,
        MatchPath::Literal,
        MatchPath::Cached,
        MatchPath::Compiled,
        MatchPath::Failed,
    ] {
        let count = walk.taking(path);

        if count == 0 {
            continue;
        }

        let mut elapsed = Duration::ZERO;
        walk.walk(&mut |node, _| {
            if node.measure.path == path {
                elapsed += node.measure.elapsed;
            }
        });

        println!("  {:<12} {count:>6}  {}", path.label(), time(elapsed));
    }

    let mut nodes = Vec::new();
    walk.walk(&mut |node, depth| nodes.push((node.measure, node.regex.clone(), node.count, depth)));
    nodes.sort_unstable_by_key(|(measure, ..)| std::cmp::Reverse(measure.elapsed));

    println!("\n  heaviest regexes of the walk");

    for (measure, regex, count, _) in nodes.iter().take(top) {
        println!(
            "  {}  {:<11} {:>5} rules  {}",
            time(measure.elapsed),
            measure.path.label(),
            count,
            cut(regex),
        );
    }

    if !print_tree {
        return;
    }

    println!("\n  the branches that matched");
    walk.walk(&mut |node, depth| {
        if !node.matched || node.is_leaf() {
            return;
        }

        println!(
            "  {:indent$}{}  {:>5} rules  {}",
            "",
            time(node.elapsed()),
            node.count,
            cut(&node.regex),
            indent = depth * 2,
        );
    });
}

/// The entries the index offered, which is where an entry's own regex is run, captures and all.
fn candidates(trace: &DetectionTrace, top: usize) {
    let applied = trace.rules.iter().filter(|rule| rule.applied).count();
    let mut rules: Vec<&RuleTrace> = trace.rules.iter().collect();
    rules.sort_unstable_by_key(|rule| std::cmp::Reverse(rule.elapsed()));

    println!(
        "\ncandidates                   {} over {} entries, {applied} applied",
        time(trace.apply()),
        trace.rules.len(),
    );
    println!("  running    compiling  route        used  pattern");

    for rule in rules.iter().take(top) {
        println!(
            "  {}  {}  {:<11}  {}  {}",
            time(rule.running()),
            time(rule.compile()),
            rule.measure.path.label(),
            if rule.applied { "yes " } else { "no  " },
            cut(&rule.regex),
        );
    }

    // An entry regex is run without the literal check the index puts in front of everything it
    // holds, so a candidate that could have been ruled out by a byte scan is run in full.
    let blind: Vec<&&RuleTrace> = rules
        .iter()
        .filter(|rule| !rule.applied && !rule.prefilter.is_empty())
        .collect();
    let wasted: Duration = blind.iter().map(|rule| rule.elapsed()).sum();

    println!(
        "\n  {} of the {} missed while requiring text a scan could have looked for first: {}",
        blind.len(),
        trace.rules.len(),
        time(wasted),
    );
}

/// The question this was written for: what the patterns opening on `.*` actually cost.
fn wildcards(trace: &DetectionTrace, detector: &ua_detector::Detector) {
    let opens = |regex: &str| regex.starts_with(".*");
    let (wild, rest): (Vec<&RuleTrace>, Vec<&RuleTrace>) =
        trace.rules.iter().partition(|rule| opens(&rule.regex));
    let elapsed = |rules: &[&RuleTrace]| rules.iter().map(|rule| rule.elapsed()).sum::<Duration>();

    let database = detector.patterns().count();
    let wildcards = detector.patterns().filter(|regex| opens(regex)).count();

    println!("\npatterns opening on .*");
    println!("  database     {wildcards:>6} of {database}");
    println!(
        "  candidates   {:>6} of {}   {}  {}",
        wild.len(),
        trace.rules.len(),
        time(elapsed(&wild)),
        share(elapsed(&wild), elapsed(&wild) + elapsed(&rest)),
    );
    println!(
        "  per entry    {} against {} for the others",
        time(mean(elapsed(&wild), wild.len())),
        time(mean(elapsed(&rest), rest.len())),
    );
}

fn count_matched(walk: &Trace<'static, ()>) -> usize {
    let mut matched = 0;
    walk.walk(&mut |node, _| matched += usize::from(node.matched));

    matched
}

fn mean(total: Duration, count: usize) -> Duration {
    total.checked_div(count as u32).unwrap_or(Duration::ZERO)
}

fn time(duration: Duration) -> String {
    format!("{:>9.2} us", duration.as_secs_f64() * 1e6)
}

fn share(part: Duration, whole: Duration) -> String {
    if whole.is_zero() {
        return "   0%".to_string();
    }

    format!("{:>4.0}%", 100.0 * part.as_secs_f64() / whole.as_secs_f64())
}

/// A pattern is often longer than a terminal, and its head says which it is.
fn cut(regex: &str) -> String {
    match regex.char_indices().nth(96) {
        None => regex.to_string(),
        Some((end, _)) => format!("{}...", &regex[..end]),
    }
}
