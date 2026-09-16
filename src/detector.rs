//! Detection of a user agent against the entries of `src/devices/`.

use std::sync::LazyLock;
use std::time::{Duration, Instant};

#[cfg(feature = "embed")]
use include_dir::{Dir, include_dir};

use crate::budget::Budget;
use crate::device::{Detection, Partial};
use crate::regex::{LazyRegex, Measure, RegexOptions};
use crate::regex_radix_tree::{RegexTreeMap, Trace};

#[cfg(feature = "embed")]
static DEVICES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/devices");

/// Where the files are read from when they were not built in, and no directory was named.
#[cfg(not(feature = "embed"))]
const DEVICES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/devices");

#[derive(serde::Deserialize)]
struct Entry {
    regex: String,
    /// Where this entry sits in the merge: every entry that matches contributes, in ascending
    /// priority, so the highest is applied last and wins.
    #[serde(default)]
    priority: i8,
    /// Conditions on the request headers, on top of the user agent. A header a request does
    /// not carry never matches.
    #[serde(default)]
    headers: std::collections::BTreeMap<String, String>,
    #[serde(flatten)]
    detection: Partial,
}

struct Rule {
    /// Applied in ascending order, so the last wins: the declared priority, then the number of
    /// header conditions, then the position across the files. An entry that read the headers
    /// knows strictly more than one that did not.
    order: (i8, usize, usize),
    regex: LazyRegex,
    headers: Vec<(String, LazyRegex)>,
    detection: Partial,
}

/// Answers kept for the user agents seen most often.
///
/// A detection walks the whole index and costs about two hundred microseconds; reading one back
/// costs forty nanoseconds. Traffic is Zipf shaped -- a thousand user agents carry nine requests
/// in ten -- so a cache of a few thousand answers is the difference between the two for almost
/// everything, at some six hundred bytes an entry.
///
/// Admission matters more than size: the tail is a scan over millions of user agents seen once,
/// and a plain LRU would let it evict the hot set. `quick_cache` admits on frequency instead.
#[cfg(feature = "cache")]
type Answers = quick_cache::sync::Cache<Box<str>, Option<std::sync::Arc<Detection>>>;

pub struct Detector {
    /// Every entry, in the order they were read.
    rules: Vec<Rule>,
    /// Narrows a user agent down to the rules worth running, which is only an optimisation: a
    /// scan over every rule gives the same answer. An entry sits under every key it may be
    /// reached by, so one lookup can offer it more than once.
    tree: RegexTreeMap<usize>,
    /// What [`Detector::detect_cached`] reads and fills. Shared, not per thread: one cache
    /// answers for more of the traffic than the same memory split between threads.
    #[cfg(feature = "cache")]
    answers: Option<Answers>,
}

/// How many times [`Detector::trace`] runs what it measures plainly, keeping the quickest.
const PASSES: usize = 3;

/// The user agents the crate ships with, for [`Detector::warm`] to spend a budget on where a
/// caller has none of its own.
///
/// An even stride across the corpus rather than a sample of anyone's requests: it covers the
/// breadth of what the database answers rather than the shape of any one service's traffic. A
/// caller holding a dump of its own should warm on that instead, which is worth a great deal
/// more -- see [`Detector::warm_from_path`].
pub fn common_user_agents() -> impl Iterator<Item = &'static str> {
    include_str!("warm.txt").lines()
}

/// Shared detector, so callers do not pay for loading the entries more than once.
///
/// One for the whole process, whatever the number of threads: detection reads and never writes,
/// so nothing here contends, and a detector of its own per thread would multiply a hundred
/// megabytes by the thread count.
///
/// Warmed on [`common_user_agents`] rather than given a budget to spend blind, because a budget
/// spent blind is mostly spent wrong: it buys the branches of the index that hold the most
/// entries, which is not where a user agent goes. This is the modest default a library can
/// choose for a caller it knows nothing about, and a caller that knows its own traffic does far
/// better with [`Detector::warm`] and a budget of its own.
pub fn shared() -> &'static Detector {
    static SHARED: LazyLock<Detector> = LazyLock::new(|| {
        let mut detector = Detector::new();
        detector.warm(common_user_agents(), Budget::bytes(SHARED_BUDGET));

        #[cfg(feature = "cache")]
        detector.cache_answers(20_000);

        detector
    });

    &SHARED
}

/// What [`shared`] spends. Enough to cover what the shipped user agents reach, and no more:
/// warming stops of its own accord once it has paid for them, so the figure is a ceiling rather
/// than an amount spent.
const SHARED_BUDGET: u64 = 400 << 20;

impl Detector {
    pub fn new() -> Detector {
        // The tree searches rather than anchors, so an entry written as `.*token.*` is indexed
        // on `token` alone. Case is ignored: roughly one request in ten arrives lowercased.
        let mut tree = RegexTreeMap::new(RegexOptions::search(true));

        let entries: Vec<(String, Entry)> = load();
        let keys: Vec<Vec<String>> =
            entries.iter().map(|(_, entry)| crate::index::keys(&entry.regex)).collect();
        let mut rules: Vec<Rule> = entries
            .into_iter()
            .enumerate()
            .map(|(index, (source, entry))| build(index, &source, entry))
            .collect();

        // Sorted by where they go in the merge, so the index a key holds is also the order the
        // rule is applied in: a lookup then only has to sort the indices it collected.
        let mut order: Vec<usize> = (0..rules.len()).collect();
        order.sort_unstable_by_key(|index| rules[*index].order);

        let mut placed = vec![0usize; rules.len()];

        for (position, index) in order.iter().enumerate() {
            placed[*index] = position;
        }

        rules.sort_unstable_by_key(|rule| rule.order);

        for (index, keys) in keys.iter().enumerate() {
            for (branch, key) in keys.iter().enumerate() {
                tree.insert(key, &format!("{index}/{branch}"), placed[index]);
            }
        }

        Detector {
            rules,
            tree,
            #[cfg(feature = "cache")]
            answers: None,
        }
    }

    /// Keeps the answers to `entries` user agents, which [`Detector::detect_cached`] then reads
    /// instead of detecting. Called once, before the detector is shared.
    #[cfg(feature = "cache")]
    pub fn cache_answers(&mut self, entries: usize) {
        self.answers = (entries > 0).then(|| Answers::new(entries));
    }

    /// Whether the cache already holds an answer for this user agent. For measurement.
    #[cfg(feature = "cache")]
    pub fn answers_hold(&self, user_agent: &str) -> bool {
        self.answers.as_ref().is_some_and(|answers| answers.peek(user_agent).is_some())
    }

    /// The same answer as [`Detector::detect`], kept for the next user agent that asks for it.
    ///
    /// The answer is shared rather than cloned: a [`Detection`] is ten strings, and copying them
    /// out costs several times what reading the cache does. Without [`Detector::cache_answers`]
    /// this is [`Detector::detect`] with an allocation on top.
    #[cfg(feature = "cache")]
    pub fn detect_cached(&self, user_agent: &str) -> Option<std::sync::Arc<Detection>> {
        let Some(answers) = &self.answers else {
            return self.detect(user_agent).map(std::sync::Arc::new);
        };

        if let Some(answer) = answers.get(user_agent) {
            return answer;
        }

        let answer = self.detect(user_agent).map(std::sync::Arc::new);

        // A user agent nothing answers for is worth keeping too: most of them are one crawler
        // sending the same broken string over and over.
        answers.insert(Box::from(user_agent), answer.clone());

        answer
    }

    /// Compiles up to `limit` regexes, guessing at which ones a caller will reach.
    ///
    /// Nothing is compiled until this is called, and one left out is compiled again at every
    /// lookup that reaches it, so a budget too small to cover what a caller matches costs far
    /// more than the memory it saves. The guess is in three parts:
    ///
    /// 1. the entries that require no text of a user agent at all, which are read for every
    ///    lookup there is. Half the budget at most;
    /// 2. the index, breadth first from the root, each level handing what it leaves over to the
    ///    branches in proportion to the rules they hold, weakest prefilter first;
    /// 3. whatever is left, on the rest of the entries, the least text each requires first.
    ///
    /// [`Detector::warm`] does not have to guess. Returns what was left unused.
    ///
    /// What a budget buys, over this database on one machine, `cargo run --release --example
    /// memory`. The database on its own is 111 MiB, and the resident column is what the budget
    /// adds to it:
    ///
    /// | budget | compiled | counted | resident |
    /// |---|---|---|---|
    /// | none | 0 | 0 | 0 |
    /// | 2 000 regexes | 2 000 | 36 MiB | 43 MiB |
    /// | 5 000 regexes | 5 000 | 95 MiB | 131 MiB |
    /// | 20 000 regexes | 20 000 | 508 MiB | 704 MiB |
    /// | 100 000 regexes | 89 959 | 3 471 MiB | 4 445 MiB |
    /// | [`Budget::bytes`] 900 MiB | 34 459 | 973 MiB | 1 327 MiB |
    ///
    /// Which is the answer to what a count of regexes costs, and why it is not a good way to ask
    /// the question: the average is 32 KiB and the spread around it is wide, so the only number
    /// that holds is the one a budget in bytes states outright. Resident runs about a third above
    /// what is counted, that third being the allocator and what is held around each regex.
    ///
    /// A small budget is worse than none: what it leaves out is compiled again at every lookup
    /// that reaches it, and it has bought the memory anyway. Spend enough to cover what you
    /// actually match, or [`Detector::warm`] on user agents of your own, which covers it at a
    /// fraction of both.
    pub fn cache(&mut self, limit: Budget) -> Budget {
        let certain = limit.take(limit.remaining() / 2);
        let spent = self.cache_rules(certain, 0).spent_from(certain);
        let left = limit.take(limit.remaining().saturating_sub(spent));
        let left = self.tree.cache(left);

        self.cache_rules(left, usize::MAX)
    }

    /// Compiles the entries whose prefilter is no stronger than `strength`, weakest first.
    fn cache_rules(&mut self, limit: Budget, strength: usize) -> Budget {
        let mut order: Vec<usize> = (0..self.rules.len())
            .filter(|index| self.rules[*index].regex.prefilter_strength() <= strength)
            .collect();
        order.sort_by_key(|index| self.rules[*index].regex.prefilter_strength());

        let mut left = limit;

        for index in order {
            if left.is_spent() {
                break;
            }

            left = self.rules[index].cache(left);
        }

        left
    }

    /// Spends `limit` compilations on what these user agents actually run, and on nothing else.
    ///
    /// Where [`Detector::cache`] guesses which of seventy thousand regexes a caller will reach,
    /// this walks the index for each user agent exactly as a lookup would and pays for what the
    /// walk had to run, the index and the entries alike. A few hundred user agents of a caller's
    /// own traffic are worth more than any budget spent blind.
    ///
    /// Returns what was left unused, which can go to [`Detector::cache`] afterwards.
    pub fn warm<A: AsRef<str>>(
        &mut self,
        user_agents: impl IntoIterator<Item = A>,
        limit: Budget,
    ) -> Budget {
        let mut left = limit;

        for user_agent in user_agents {
            if left.is_spent() {
                break;
            }

            let user_agent = user_agent.as_ref();
            left = self.tree.warm(user_agent, left);

            // And the entries it reached: reading one out compiles its regex, with no prefilter
            // in front of it to say otherwise.
            for index in self.reached_by(user_agent) {
                if left.is_spent() {
                    break;
                }

                left = self.rules[index].cache(left);
            }
        }

        left
    }

    /// The same, over a file of user agents, one per line -- a dump of a service's own traffic,
    /// heaviest first, being the best thing a budget can be spent on and the one thing an
    /// application does not have at startup.
    ///
    /// Read a line at a time: a dump worth warming on is larger than the index it pays for.
    pub fn warm_from_path(
        &mut self,
        path: impl AsRef<std::path::Path>,
        limit: Budget,
    ) -> std::io::Result<Budget> {
        use std::io::BufRead;

        let file = std::io::BufReader::new(std::fs::File::open(path)?);
        let mut left = limit;

        for line in file.lines() {
            let line = line?;

            if left.is_spent() {
                break;
            }

            left = self.warm([line.as_str()], left);
        }

        Ok(left)
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// How many regexes are being held compiled, the index and the entries together. What a
    /// budget came to, rather than what it was asked for.
    pub fn compiled(&self) -> usize {
        let rules: usize = self
            .rules
            .iter()
            .map(|rule| {
                usize::from(rule.regex.is_compiled())
                    + rule.headers.iter().filter(|(_, regex)| regex.is_compiled()).count()
            })
            .sum();

        self.tree.cached_len() + rules
    }

    /// What those cost to hold, in bytes. The automata and what hangs off them; see [`Budget`]
    /// for what this does not count.
    pub fn compiled_size(&self) -> u64 {
        let rules: u64 = self
            .rules
            .iter()
            .map(|rule| {
                rule.headers
                    .iter()
                    .fold(rule.regex.compiled_size(), |total, (_, regex)| {
                        total + regex.compiled_size()
                    })
            })
            .sum();

        self.tree.cached_size() + rules
    }

    /// How many keys the index holds: an alternation has no prefix and sits under one per
    /// branch.
    pub fn keys(&self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Every entry's pattern, for tooling that reads the database as text rather than running
    /// it.
    pub fn patterns(&self) -> impl Iterator<Item = &str> {
        self.rules.iter().map(|rule| rule.regex.original.as_str())
    }

    /// How many rules the tree offers for a user agent, which measures how well the index
    /// narrows the database down.
    pub fn candidates(&self, user_agent: &str) -> usize {
        self.candidates_of(user_agent).len()
    }

    /// The rules worth trying, in the order they are applied. An entry indexed under several
    /// keys is offered once per key its haystack reached.
    fn candidates_of(&self, user_agent: &str) -> Vec<&Rule> {
        self.reached_by(user_agent).into_iter().map(|index| &self.rules[index]).collect()
    }

    /// Where in `rules` the index sends this user agent, each entry once.
    fn reached_by(&self, user_agent: &str) -> Vec<usize> {
        let mut found: Vec<usize> = self.tree.find(user_agent).into_iter().copied().collect();
        found.sort_unstable();
        found.dedup();

        found
    }

    /// What the user agent resolves to, or `None` when no entry covers it.
    ///
    /// When several entries match, the first one in file order wins.
    pub fn detect(&self, user_agent: &str) -> Option<Detection> {
        self.detect_with_headers(user_agent, &[])
    }

    /// Same as [`Detector::detect`], also reading the request headers, which is where a browser
    /// puts the client hints that refine or contradict its user agent.
    pub fn detect_with_headers(
        &self,
        user_agent: &str,
        headers: &[(&str, &str)],
    ) -> Option<Detection> {
        let mut detection: Option<Partial> = None;

        // Every entry that matches has its say, in ascending priority: one may name the client,
        // another the model it is running on.
        for rule in self.candidates_of(user_agent) {
            let Some(found) = rule.apply(user_agent, headers) else {
                continue;
            };

            match &mut detection {
                None => detection = Some(found),
                Some(detection) => detection.merge(found),
            }
        }

        detection.map(Detection::from)
    }

    /// The same detection, told step by step, with a clock around every regex it runs.
    ///
    /// Two kinds of pass: the plain one says what a detection costs, the instrumented one says
    /// where it went and reads the clock twice per regex to do it.
    ///
    /// Everything is warmed first and then run [`PASSES`] times, the quickest kept: the first
    /// detection of a process costs several times the ones after it, which is the cost of
    /// starting rather than the cost of a lookup.
    pub fn trace(&self, user_agent: &str, headers: &[(&str, &str)]) -> DetectionTrace {
        let mut detection = self.detect_with_headers(user_agent, headers);
        let mut detect = Duration::MAX;

        drop(self.tree.find(user_agent));

        for _ in 0..PASSES {
            let start = Instant::now();
            detection = self.detect_with_headers(user_agent, headers);
            detect = detect.min(start.elapsed());
        }

        let mut candidates = Vec::new();
        let mut find = Duration::MAX;

        for _ in 0..PASSES {
            let start = Instant::now();
            candidates = self.candidates_of(user_agent);
            find = find.min(start.elapsed());
        }

        let index = self.tree.trace(user_agent).erased();
        let rules = candidates.iter().map(|rule| rule.trace(user_agent, headers)).collect();

        DetectionTrace {
            user_agent: user_agent.to_string(),
            detection,
            detect,
            find,
            index,
            rules,
        }
    }
}

/// What one detection did, and what each part of it cost. See [`Detector::trace`].
#[derive(Debug)]
pub struct DetectionTrace {
    pub user_agent: String,
    pub detection: Option<Detection>,
    /// A plain detection, measured with nothing in the way.
    pub detect: Duration,
    /// What of it went on narrowing the database down to the candidates.
    pub find: Duration,
    /// That same walk, run again with every regex of it measured.
    pub index: Trace<'static, ()>,
    /// Every rule the index offered, in the order they are applied.
    pub rules: Vec<RuleTrace>,
}

impl DetectionTrace {
    /// What reading the candidates cost, which is where an entry's own regex is run.
    pub fn apply(&self) -> Duration {
        self.rules.iter().map(RuleTrace::elapsed).sum()
    }

    /// What went on compiling regexes the cache budget did not cover. Every one of them is
    /// compiled again at the next lookup that reaches it.
    pub fn compile(&self) -> Duration {
        self.index.compile() + self.rules.iter().map(RuleTrace::compile).sum::<Duration>()
    }
}

/// What one candidate rule cost to read.
#[derive(Debug)]
pub struct RuleTrace {
    pub regex: String,
    pub order: (i8, usize, usize),
    /// Reading the captures out of the user agent.
    pub measure: Measure,
    /// The conditions on the request headers, in the order they were checked. `None` where the
    /// request does not carry the header at all, which no regex has to answer.
    pub headers: Vec<(String, Option<Measure>)>,
    /// The text the pattern requires of a user agent: what a prefilter would have had to work
    /// with, had one stood in front of the entry regex.
    pub prefilter: Vec<String>,
    /// Whether the rule had its say in the merge.
    pub applied: bool,
}

impl RuleTrace {
    pub fn elapsed(&self) -> Duration {
        self.headers
            .iter()
            .flat_map(|(_, measure)| measure.map(|measure| measure.elapsed))
            .fold(self.measure.elapsed, |total, elapsed| total + elapsed)
    }

    pub fn compile(&self) -> Duration {
        self.headers
            .iter()
            .flat_map(|(_, measure)| measure.map(|measure| measure.compile))
            .fold(self.measure.compile, |total, compile| total + compile)
    }

    /// What of it was the regexes actually running, rather than being built again.
    pub fn running(&self) -> Duration {
        self.elapsed().saturating_sub(self.compile())
    }
}

impl Rule {
    fn apply(&self, user_agent: &str, headers: &[(&str, &str)]) -> Option<Partial> {
        let mut captured = self.regex.named_captures(user_agent)?;

        for (name, condition) in &self.headers {
            let value = headers
                .iter()
                .find(|(header, _)| header.eq_ignore_ascii_case(name))
                .map(|(_, value)| *value)?;

            captured.extend(condition.named_captures(value)?);
        }

        Some(self.detection.resolve(&|name: &str| {
            captured
                .iter()
                .find(|(captured, _)| captured == name)
                .map(|(_, value)| value.clone())
        }))
    }

    /// The same route [`Rule::apply`] takes, measured. It stops where `apply` stops: a rule
    /// whose user agent regex missed never reads a header.
    fn trace(&self, user_agent: &str, headers: &[(&str, &str)]) -> RuleTrace {
        let (measure, captured) = self.regex.measure_captures(user_agent);
        let mut measures = Vec::new();
        let mut applied = captured.is_some();

        if applied {
            for (name, condition) in &self.headers {
                let value = headers
                    .iter()
                    .find(|(header, _)| header.eq_ignore_ascii_case(name))
                    .map(|(_, value)| *value);

                let Some(value) = value else {
                    measures.push((name.clone(), None));
                    applied = false;

                    break;
                };

                let (measure, captured) = condition.measure_captures(value);
                measures.push((name.clone(), Some(measure)));

                if captured.is_none() {
                    applied = false;

                    break;
                }
            }
        }

        RuleTrace {
            regex: self.regex.original.clone(),
            order: self.order,
            measure,
            headers: measures,
            prefilter: self.regex.required_literals().into_iter().map(str::to_string).collect(),
            applied,
        }
    }

    fn cache(&mut self, mut left: Budget) -> Budget {
        let regexes = std::iter::once(&mut self.regex).chain(self.headers.iter_mut().map(|(_, regex)| regex));

        for regex in regexes {
            if left.is_spent() {
                break;
            }

            if !regex.is_compiled() {
                *regex = regex.compile();
                left = left.pay(regex);
            }
        }

        left
    }
}

impl Default for Detector {
    fn default() -> Detector {
        Detector::new()
    }
}

/// Directory to read the database from instead of the copy built into the binary, which is what
/// keeps a change to an entry from recompiling everything that links the library. `build.rs`
/// stops watching the files while it is set.
pub const DEVICES_DIRECTORY: &str = "DEVICE_DETECTOR_DEVICES";

fn load() -> Vec<(String, Entry)> {
    let mut files: Vec<(String, String)> = match std::env::var(DEVICES_DIRECTORY) {
        Ok(directory) => read(&directory),
        Err(_) => embedded(),
    };

    // Sorted, so the order entries are given in is the same on every platform, and the same
    // whichever of the two the database was read from.
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut entries = Vec::new();

    for (source, content) in files {
        let parsed: Vec<Entry> = serde_yaml::from_str(&content)
            .unwrap_or_else(|error| panic!("cannot parse {source}: {error}"));

        entries.extend(parsed.into_iter().map(|entry| (source.clone(), entry)));
    }

    entries
}

#[cfg(feature = "embed")]
fn embedded() -> Vec<(String, String)> {
    DEVICES
        .files()
        .filter(|file| file.path().extension().is_some_and(|extension| extension == "yml"))
        .map(|file| {
            let source = file.path().display().to_string();
            let content =
                file.contents_utf8().unwrap_or_else(|| panic!("{source} is not valid utf8"));

            (source, content.to_string())
        })
        .collect()
}

/// Without the database built in, the tree it was built from is what answers.
#[cfg(not(feature = "embed"))]
fn embedded() -> Vec<(String, String)> {
    read(DEVICES)
}

/// The same files, read from a directory. An entry is reported under its file name alone, as
/// `include_dir` gives it, so a failure reads the same whichever copy answered.
fn read(directory: &str) -> Vec<(String, String)> {
    let listing = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {DEVICES_DIRECTORY}={directory}: {error}"));

    listing
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "yml"))
        .map(|path| {
            let source = path
                .file_name()
                .expect("a file that has a name")
                .display()
                .to_string();
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));

            (source, content)
        })
        .collect()
}

fn build(index: usize, source: &str, entry: Entry) -> Rule {
    // Entries describe a whole user agent, so the regex has to cover it entirely. Header
    // conditions are searched instead, an entry anchoring them itself when it needs to.
    let regex = LazyRegex::new_leaf(&entry.regex, RegexOptions::anchored(true));
    let headers: Vec<(String, LazyRegex)> = entry
        .headers
        .iter()
        .map(|(name, condition)| {
            (name.clone(), LazyRegex::new_leaf(condition, RegexOptions::search(true)))
        })
        .collect();

    for (placeholder, filters) in entry.detection.placeholders() {
        let known = regex
            .capture_names()
            .into_iter()
            .chain(headers.iter().flat_map(|(_, regex)| regex.capture_names()));

        assert!(
            known.into_iter().any(|name| name == placeholder),
            "{source}: {{{placeholder}}} has no matching capture group in {}",
            entry.regex,
        );

        for filter in filters {
            assert!(
                crate::template::FILTERS.contains(&filter.as_str()),
                "{source}: unknown filter {filter} on {{{placeholder}}}, known filters are {:?}",
                crate::template::FILTERS,
            );
        }
    }

    Rule {
        order: (entry.priority, headers.len(), index),
        regex,
        headers,
        detection: entry.detection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_is_readable_and_its_placeholders_resolve() {
        assert!(!Detector::new().is_empty());
    }

    /// Nothing is compiled until a caller asks for it.
    #[test]
    fn compiles_nothing_until_asked_to() {
        let mut detector = Detector::new();

        assert!(detector.rules.iter().all(|rule| !rule.regex.is_compiled()));

        let left = detector.cache(Budget::regexes(10));

        assert!(left.is_spent(), "the whole budget should have been spent");
    }

    /// The tree is an index, not a filter: narrowing a lookup down must never drop a rule that
    /// would have matched.
    #[test]
    fn the_index_never_prunes_a_rule_that_matches() {
        let detector = shared();

        for (index, rule) in detector.rules.iter().enumerate() {
            let Some(sample) = sample_of(&rule.regex) else {
                continue;
            };

            assert!(
                detector.tree.find(&sample).contains(&&index),
                "the tree pruned {} for {sample}",
                rule.regex.original(),
            );
        }
    }

    /// A trace has to describe the detection it measures, and account for all of it: the same
    /// answer, and one candidate per leaf the index walk reached.
    #[test]
    fn a_trace_reads_the_same_detection_it_measures() {
        let detector = shared();
        let user_agent = "Mozilla/5.0 (Linux; Android 13; SM-A536B) AppleWebKit/537.36 \
            (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";
        let trace = detector.trace(user_agent, &[]);

        assert_eq!(trace.detection, detector.detect(user_agent));
        assert_eq!(trace.rules.len(), detector.candidates(user_agent));
        assert!(trace.rules.iter().any(|rule| rule.applied), "nothing had its say");
        assert!(trace.apply() > Duration::ZERO);

        let mut reached = 0;
        trace.index.walk(&mut |node, _| {
            if node.matched && node.is_leaf() {
                reached += node.count as usize;
            }
        });

        assert_eq!(reached, trace.rules.len(), "the walk reached something the rules do not hold");
    }

    /// Warming pays for what a user agent actually runs: afterwards nothing on its way is
    /// compiled again, and it answers what it answered before.
    #[test]
    fn warming_pays_for_what_a_user_agent_runs() {
        use crate::regex::MatchPath;

        let user_agent = "Mozilla/5.0 (Linux; Android 13; SM-A536B) AppleWebKit/537.36 \
            (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";
        let mut detector = Detector::new();
        let budget = Budget::regexes(10_000);
        let left = detector.warm([user_agent], budget);

        assert!(!left.is_spent(), "one user agent should not exhaust a budget of ten thousand");
        assert!(left.spent_from(budget) > 0, "warming spent nothing at all");

        let trace = detector.trace(user_agent, &[]);

        assert_eq!(trace.index.taking(MatchPath::Compiled), 0, "the index still compiles on the way");
        assert!(
            trace.rules.iter().all(|rule| rule.measure.path != MatchPath::Compiled),
            "an entry is still compiled at every lookup",
        );
        assert_eq!(trace.detection, shared().detect(user_agent), "warming changed the answer");
    }

    /// The list the crate ships is what [`shared`] spends its budget on, so the budget has to
    /// cover it: warming stops where the budget does, and every lookup past that point compiles
    /// again on the way out.
    #[test]
    fn the_shipped_user_agents_fit_the_shared_budget() {
        let agents: Vec<&str> = common_user_agents().collect();

        assert!(agents.len() > 500, "only {} user agents shipped", agents.len());
        assert!(agents.iter().all(|agent| !agent.trim().is_empty()), "a blank line in warm.txt");

        let mut detector = Detector::new();
        let left = detector.warm(&agents, Budget::bytes(SHARED_BUDGET));

        assert!(!left.is_spent(), "the shared budget does not cover what it is spent on");
    }

    /// A budget in bytes is a target rather than a bound -- what a regex costs is only known
    /// once it has been compiled -- but the overshoot has to stay small, which is what carrying
    /// a branch's overspend back to its siblings is for. It was a third of the budget without.
    #[test]
    fn a_budget_in_bytes_comes_to_about_what_it_asked_for() {
        let budget = Budget::bytes(100 << 20);
        let mut detector = Detector::new();
        detector.cache(budget);

        let held = detector.compiled_size();

        assert!(held > 90 << 20, "{held} bytes held against a budget of 100 MiB");
        assert!(held < 115 << 20, "{held} bytes held against a budget of 100 MiB");
    }

    /// What a budget buys is only ever an optimisation: the answer is the same without one.
    #[test]
    fn a_budget_never_changes_an_answer() {
        let user_agent = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
            (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
        let cold = Detector::new();
        let mut warmed = Detector::new();
        warmed.cache(Budget::regexes(2_000));

        assert_eq!(cold.detect(user_agent), warmed.detect(user_agent));
    }

    /// A cached answer is the answer, and the second one is the first one over again rather
    /// than a second detection.
    #[cfg(feature = "cache")]
    #[test]
    fn a_cached_answer_is_the_same_answer() {
        let user_agent = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
            (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
        let mut detector = Detector::new();
        detector.cache_answers(16);

        let first = detector.detect_cached(user_agent).expect("an answer");
        let second = detector.detect_cached(user_agent).expect("an answer");

        assert_eq!(*first, detector.detect(user_agent).expect("an answer"));
        assert!(std::sync::Arc::ptr_eq(&first, &second), "the answer was detected twice");

        // Nothing kept, and it still answers.
        let plain = Detector::new();

        assert_eq!(plain.detect_cached(user_agent).map(|answer| (*answer).clone()), plain.detect(user_agent));
    }

    /// A user agent the regex matches, built by keeping only its literal parts. Returns `None`
    /// when the regex holds a construct that cannot be turned back into text.
    fn sample_of(regex: &LazyRegex) -> Option<String> {
        let mut sample = String::new();
        let mut chars = regex.original().chars();

        while let Some(char) = chars.next() {
            match char {
                '\\' => sample.push(chars.next()?),
                '(' | '[' | '|' | '?' | '*' | '+' | '{' | '.' => return None,
                _ => sample.push(char),
            }
        }

        Some(sample)
    }
}
