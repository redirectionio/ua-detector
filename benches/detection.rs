//! Where the time goes: loading the database, compiling a budget of it, and answering with it.
//!
//!     cargo bench
//!     cargo bench -- detect/20000        # one group of it
//!     DETECT_SAMPLE=38030 cargo bench    # the whole corpus, and a long wait
//!
//! The memory side of the same question is `cargo run --release --example memory`. Criterion
//! cannot answer it: resident size is a property of the process rather than of an operation, and
//! running the same allocation a hundred times measures the allocator instead of the index.
//!
//! Every group runs ten samples rather than the usual hundred, and detection runs over a stride
//! across the corpus rather than all of it. That is not thrift for its own sake. At a budget of
//! zero a lookup compiles whatever it reaches and throws it away, which costs about six
//! milliseconds an agent against half a millisecond once the entries are cached; a hundred
//! samples over thirty-eight thousand agents at that rate is several hours, and a benchmark
//! nobody runs measures nothing. Raise `DETECT_SAMPLE` when a number has to be trusted on its
//! own rather than compared against the last run.

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, SamplingMode, Throughput, criterion_group, criterion_main};
use device_detector::Detector;

/// What `Detector::cache` is given, in regexes compiled up front.
///
/// Zero is the interesting end, and the reason the rest of this file is shaped the way it is:
/// nothing is compiled, so the cost of an entry is paid again at every lookup that reaches it.
const BUDGETS: [u64; 5] = [0, 1_000, 5_000, 20_000, 100_000];

/// Agents to detect per sample, taken across the corpus at an even stride.
///
/// Small enough that the worst budget stays in seconds. `DETECT_SAMPLE` overrides it.
const SAMPLE: usize = 5;

/// The corpus, as the haystacks alone.
///
/// Read off the fixtures rather than off production traffic, so that a run means the same thing
/// on any checkout and so that it reaches the whole database rather than the few hundred entries
/// real traffic lands on.
fn agents() -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct Case {
        user_agent: String,
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/matomo-device-detector/fixtures");
    let mut files: Vec<_> = std::fs::read_dir(root)
        .expect("the fixtures are not where they should be; run `castor matomo:sync`")
        .flatten()
        .map(|entry| entry.path())
        .collect();
    files.sort();

    files
        .iter()
        .flat_map(|path| {
            let content = std::fs::read_to_string(path).unwrap();
            let cases: Vec<Case> = serde_yaml::from_str(&content).unwrap();

            cases.into_iter().map(|case| case.user_agent)
        })
        .collect()
}

/// A stride over the corpus, so that the sample keeps covering every typology in it.
fn sample() -> Vec<String> {
    let agents = agents();
    let wanted = std::env::var("DETECT_SAMPLE")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(SAMPLE)
        .min(agents.len())
        .max(1);

    let stride = agents.len().div_ceil(wanted);

    agents.into_iter().step_by(stride).collect()
}

/// Reading the yaml and building the radix tree over it. Nothing is compiled here.
fn load(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("load");
    group
        .sample_size(10)
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5));

    group.bench_function("new", |bencher| bencher.iter(|| black_box(Detector::new()).len()));

    group.finish();
}

/// Spending a budget of compilations, breadth first from the root.
///
/// A detector that has already spent its budget would answer instantly, so each sample gets a
/// freshly loaded one. That setup is not timed, but it is still a second of wall clock a sample.
fn cache(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("cache");
    // `cache` is quick at the small budgets, and criterion sizes a run by the measurement time
    // it is given rather than by the sample count -- sixty seconds of it asked for seventy
    // iterations of the thousand-regex budget, each preceded by loading a detector that had not
    // spent it yet. That setup is not timed, but it is most of a second, and it was most of the
    // run. Asking for no measurement time at all leaves exactly `sample_size` iterations, which
    // is what there is to measure; criterion says so on the way past.
    group
        .sample_size(10)
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(Duration::from_millis(1))
        .measurement_time(Duration::from_millis(1));

    for budget in BUDGETS.into_iter().filter(|&budget| budget > 0) {
        group.bench_with_input(BenchmarkId::from_parameter(budget), &budget, |bencher, &budget| {
            bencher.iter_batched(
                Detector::new,
                |mut detector| black_box(detector.cache(budget)),
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

/// Answering, at each budget.
///
/// One group per budget rather than one parameterised group, so that criterion keeps the budgets
/// apart in its baseline and a change in one of them reads on its own. The spread between them is
/// the whole point: it is what a budget too small costs, an entry the tree landed on having to be
/// compiled again on the way out.
fn detect(criterion: &mut Criterion) {
    let agents = sample();

    for budget in BUDGETS {
        let mut detector = Detector::new();
        detector.cache(budget);

        let mut group = criterion.benchmark_group(format!("detect/{budget}"));
        group
            .sample_size(10)
            .sampling_mode(SamplingMode::Flat)
            .warm_up_time(Duration::from_secs(1))
            .measurement_time(Duration::from_secs(5))
            .throughput(Throughput::Elements(agents.len() as u64));

        group.bench_function("detect", |bencher| {
            bencher.iter(|| agents.iter().filter(|agent| detector.detect(agent).is_some()).count())
        });

        group.finish();
    }
}

criterion_group!(benches, load, cache, detect);
criterion_main!(benches);
