use std::time::{Duration, Instant};
use std::{fmt::Display, hash::Hash, sync::Arc};

use regex_automata::meta::{Config, Regex};
use regex_automata::util::syntax;
use regex_automata::{Input, PatternID};

/// How the regexes stored in a tree are matched against a haystack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegexOptions {
    pub ignore_case: bool,
    /// When set, a leaf only matches a haystack it covers entirely. Otherwise leaves and nodes
    /// are searched anywhere inside the haystack, which is what user agent patterns expect.
    pub anchored: bool,
}

impl RegexOptions {
    pub fn anchored(ignore_case: bool) -> Self {
        RegexOptions {
            ignore_case,
            anchored: true,
        }
    }

    pub fn search(ignore_case: bool) -> Self {
        RegexOptions {
            ignore_case,
            anchored: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LazyRegex {
    pub(crate) original: String,
    pub(crate) regex: String,
    pub(crate) compiled: Option<Arc<Regex>>,
    pub(crate) options: RegexOptions,
    /// Text the haystack has to hold, in that order, for the pattern to stand a chance. Looking
    /// for it costs a fraction of running the regex and rules most haystacks out without ever
    /// compiling anything.
    literals: Literals,
    /// What the pattern is anchored on: a node covers the start of the haystack, a leaf under
    /// [`RegexOptions::anchored`] covers all of it, and anything else is searched.
    anchoring: Anchoring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Anchoring {
    Anywhere,
    Start,
    Whole,
}

#[derive(Debug, Clone, Default)]
struct Literals {
    runs: Vec<Box<str>>,
    /// Whether the runs are the whole pattern, in which case they answer on their own.
    whole: bool,
    /// Folded when set, and compared against a folded haystack.
    ignore_case: bool,
}

impl Literals {
    /// Reads the literal runs a pattern is made of, keeping only what every match must contain:
    /// nothing under an alternation or a group, and nothing a quantifier may drop.
    fn of(pattern: &str, ignore_case: bool) -> Literals {
        let mut literals = Literals {
            runs: Vec::new(),
            whole: true,
            ignore_case,
        };
        let mut run = String::new();
        let mut depth = 0usize;
        let mut chars = pattern.chars().peekable();

        while let Some(char) = chars.next() {
            match char {
                '\\' => match chars.next() {
                    // An escaped letter or digit is a class, not the character itself.
                    Some(escaped) if escaped.is_alphanumeric() => literals.cut(&mut run),
                    Some(escaped) if depth == 0 => run.push(escaped),
                    Some(_) => literals.cut(&mut run),
                    None => literals.cut(&mut run),
                },
                // A group may be optional or hold alternatives, so nothing inside it is required.
                '(' => {
                    depth += 1;
                    literals.cut(&mut run);
                }
                ')' => depth = depth.saturating_sub(1),
                '[' => {
                    literals.cut(&mut run);

                    while chars.peek().is_some_and(|char| *char != ']') {
                        if chars.next() == Some('\\') {
                            chars.next();
                        }
                    }

                    chars.next();
                }
                // With a top level alternation the pattern has no required text at all.
                '|' if depth == 0 => return Literals::default(),
                '*' | '?' => {
                    // The character it applies to may not be there.
                    run.pop();
                    literals.cut(&mut run);
                }
                '{' => {
                    run.pop();
                    literals.cut(&mut run);

                    while chars.peek().is_some_and(|char| *char != '}') {
                        chars.next();
                    }

                    chars.next();
                }
                '.' | '^' | '$' | '+' | '|' => literals.cut(&mut run),
                _ if depth == 0 => run.push(char),
                _ => literals.cut(&mut run),
            }
        }

        if !run.is_empty() {
            literals.keep(&mut run);
        }

        literals
    }

    /// Ends the run being read. Whatever ends it is something the runs do not describe.
    fn cut(&mut self, run: &mut String) {
        self.whole = false;

        if run.len() > 1 {
            let mut run = std::mem::take(run);
            self.keep(&mut run);
        } else {
            run.clear();
        }
    }

    /// Files a run away, folded if the match is to ignore case.
    ///
    /// A run that is not ascii is dropped instead. Folding it would need the same rules the regex
    /// engine uses, and a run folded by weaker ones would rule out haystacks the pattern matches.
    /// Dropping it only makes the prefilter less selective, which costs time and never an answer.
    fn keep(&mut self, run: &mut String) {
        if !self.ignore_case {
            self.runs.push(std::mem::take(run).into_boxed_str());

            return;
        }

        if run.is_ascii() {
            run.make_ascii_lowercase();
            self.runs.push(std::mem::take(run).into_boxed_str());
        } else {
            self.whole = false;
            run.clear();
        }
    }

    /// Whether the haystack holds every run. Read backwards, because a tree walks patterns that
    /// grow at the end: what tells a pattern apart from the one above it is its last run, so
    /// looking there first usually settles it in a single scan.
    fn found_in(&self, haystack: &str) -> bool {
        self.runs.iter().rev().all(|run| {
            if self.ignore_case {
                contains_ignore_ascii_case(haystack, run)
            } else {
                haystack.contains(run.as_ref())
            }
        })
    }
}

/// Whether `haystack` holds `needle`, which is already folded, comparing a byte at a time.
///
/// Folding the haystack instead would mean allocating a copy of it on every lookup that reaches
/// a pattern, and there are a few of those per user agent.
fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());

    let Some(&first) = needle.first() else {
        return true;
    };
    let Some(last_start) = haystack.len().checked_sub(needle.len()) else {
        return false;
    };

    (0..=last_start).any(|start| {
        haystack[start].to_ascii_lowercase() == first
            && haystack[start..].iter().zip(needle).all(|(from, want)| from.to_ascii_lowercase() == *want)
    })
}

/// Whether `value` opens with `text`, which is folded when `ignore_case`.
fn starts_with(value: &str, text: &str, ignore_case: bool) -> bool {
    if !ignore_case {
        return value.starts_with(text);
    }

    let (value, text) = (value.as_bytes(), text.as_bytes());

    value.len() >= text.len() && value[..text.len()].eq_ignore_ascii_case(text)
}

/// What a haystack can be told without running a regex.
enum Prefilter {
    /// The answer, and what reached it.
    Answer(bool, MatchPath),
    /// Only the regex can say.
    Undecided,
}

/// The route a match took, which is what separates a lookup costing a byte scan from one
/// costing a compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MatchPath {
    /// A node with no pattern of its own, which covers everything under it.
    Empty,
    /// The text the pattern requires is not in the haystack, so no regex was run.
    Prefiltered,
    /// The pattern is text and nothing else, answered by comparing it.
    Literal,
    /// Ran a regex the cache budget had already compiled.
    Cached,
    /// Compiled the regex for this lookup alone, and will compile it again for the next one.
    Compiled,
    /// The pattern would not compile.
    Failed,
}

impl MatchPath {
    pub fn label(&self) -> &'static str {
        match self {
            MatchPath::Empty => "empty",
            MatchPath::Prefiltered => "prefiltered",
            MatchPath::Literal => "literal",
            MatchPath::Cached => "cached",
            MatchPath::Compiled => "compiled",
            MatchPath::Failed => "failed",
        }
    }
}

impl Display for MatchPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// What one match against one pattern answered, and what it cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measure {
    pub matched: bool,
    pub path: MatchPath,
    /// Everything the call cost, compilation included.
    pub elapsed: Duration,
    /// What of it went on compiling a regex the cache budget did not cover.
    pub compile: Duration,
}

impl Measure {
    fn new(matched: bool, path: MatchPath, elapsed: Duration, compile: Duration) -> Measure {
        Measure {
            matched,
            path,
            elapsed,
            compile,
        }
    }

    /// What the match itself cost, once the compilation it had to pay for is taken off.
    pub fn running(&self) -> Duration {
        self.elapsed.saturating_sub(self.compile)
    }
}

impl Eq for LazyRegex {}

impl PartialEq for LazyRegex {
    fn eq(&self, other: &Self) -> bool {
        self.original == other.original && self.options == other.options
    }
}

impl Ord for LazyRegex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.original.cmp(&other.original)
    }
}

impl PartialOrd for LazyRegex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for LazyRegex {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.original.hash(state);
        self.options.hash(state);
    }
}

impl Display for LazyRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.original)
    }
}

impl LazyRegex {
    pub fn new_node(regex: String, options: RegexOptions) -> LazyRegex {
        let compiled_source = match (regex.is_empty(), options.anchored) {
            (true, _) => ".*".to_string(),
            (false, true) => ["^(?:", regex.as_str(), ")"].join(""),
            (false, false) => regex.clone(),
        };

        LazyRegex {
            literals: Literals::of(&regex, options.ignore_case),
            anchoring: if options.anchored { Anchoring::Start } else { Anchoring::Anywhere },
            regex: compiled_source,
            original: regex,
            compiled: None,
            options,
        }
    }

    pub fn new_leaf(regex: &str, options: RegexOptions) -> LazyRegex {
        LazyRegex {
            regex: if options.anchored {
                // The group matters: `^a|b$` would anchor each branch on its own.
                ["^(?:", regex, ")$"].join("")
            } else {
                regex.to_string()
            },
            original: regex.to_string(),
            compiled: None,
            options,
            literals: Literals::of(regex, options.ignore_case),
            anchoring: if options.anchored { Anchoring::Whole } else { Anchoring::Anywhere },
        }
    }

    /// What can be said about a haystack before any regex is run, and how.
    ///
    /// Shared with [`LazyRegex::measure_match`], which has to take exactly the same route as
    /// [`LazyRegex::is_match`] for what it reports to mean anything.
    fn prefilter(&self, value: &str) -> Prefilter {
        if self.original.is_empty() {
            return Prefilter::Answer(true, MatchPath::Empty);
        }

        if !self.literals.found_in(value) {
            return Prefilter::Answer(false, MatchPath::Prefiltered);
        }

        // A pattern made of nothing but text needs no regex at all.
        if self.literals.whole {
            let text = self.literals.runs.first().map_or("", |run| run.as_ref());

            let matched = match self.anchoring {
                Anchoring::Anywhere => true,
                Anchoring::Start => starts_with(value, text, self.options.ignore_case),
                Anchoring::Whole => {
                    if self.options.ignore_case {
                        value.eq_ignore_ascii_case(text)
                    } else {
                        value == text
                    }
                }
            };

            return Prefilter::Answer(matched, MatchPath::Literal);
        }

        Prefilter::Undecided
    }

    pub fn is_match(&self, value: &str) -> bool {
        match self.prefilter(value) {
            Prefilter::Answer(matched, _) => matched,
            Prefilter::Undecided => match &self.compiled {
                Some(regex) => regex.is_match(value),
                None => match self.create_regex() {
                    None => false,
                    Some(regex) => regex.is_match(value),
                },
            },
        }
    }

    /// The same answer as [`LazyRegex::is_match`], with what it cost and which route it took.
    ///
    /// Reading the clock twice around a call that often amounts to a `memchr` is not free, so
    /// this is for tracing one lookup, never for the lookups themselves.
    pub fn measure_match(&self, value: &str) -> Measure {
        let start = Instant::now();

        if let Prefilter::Answer(matched, path) = self.prefilter(value) {
            return Measure::new(matched, path, start.elapsed(), Duration::ZERO);
        }

        match &self.compiled {
            Some(regex) => {
                let matched = regex.is_match(value);

                Measure::new(matched, MatchPath::Cached, start.elapsed(), Duration::ZERO)
            }
            None => {
                let compiling = Instant::now();
                let regex = self.create_regex();
                let compile = compiling.elapsed();

                match regex {
                    None => Measure::new(false, MatchPath::Failed, start.elapsed(), compile),
                    Some(regex) => {
                        let matched = regex.is_match(value);

                        Measure::new(matched, MatchPath::Compiled, start.elapsed(), compile)
                    }
                }
            }
        }
    }

    /// The same answer as [`LazyRegex::named_captures`], with what it cost.
    ///
    /// Reading the captures out is where an entry regex is actually run, and where a pattern
    /// that opens on `.*` is paid for: no prefilter stands in front of it.
    pub fn measure_captures(&self, haystack: &str) -> (Measure, Option<Vec<(String, String)>>) {
        let start = Instant::now();
        let (compiled, path, compile) = match &self.compiled {
            Some(compiled) => (Some(compiled.clone()), MatchPath::Cached, Duration::ZERO),
            None => {
                let compiling = Instant::now();
                let compiled = self.create_regex();
                let compile = compiling.elapsed();
                let path = if compiled.is_some() { MatchPath::Compiled } else { MatchPath::Failed };

                (compiled, path, compile)
            }
        };

        let Some(compiled) = compiled else {
            return (Measure::new(false, MatchPath::Failed, start.elapsed(), compile), None);
        };

        let captured = self.captures_of(&compiled, haystack);

        (
            Measure::new(captured.is_some(), path, start.elapsed(), compile),
            captured,
        )
    }

    #[cfg(test)]
    pub fn original(&self) -> &str {
        self.original.as_str()
    }

    pub fn is_compiled(&self) -> bool {
        self.compiled.is_some()
    }

    /// Named captures of the first match, or `None` when the regex does not match.
    ///
    /// The regex is compiled on the fly unless it has been cached. Values are owned so that
    /// nothing borrows a regex that may only live for the call.
    pub fn named_captures(&self, haystack: &str) -> Option<Vec<(String, String)>> {
        let compiled = match &self.compiled {
            Some(compiled) => compiled.clone(),
            None => self.create_regex()?,
        };

        self.captures_of(&compiled, haystack)
    }

    fn captures_of(&self, compiled: &Regex, haystack: &str) -> Option<Vec<(String, String)>> {
        let mut captures = compiled.create_captures();
        compiled.captures(Input::new(haystack), &mut captures);

        if !captures.is_match() {
            return None;
        }

        let names = compiled.group_info().pattern_names(PatternID::ZERO).flatten();

        Some(
            names
                .filter_map(|name| {
                    let span = captures.get_group_by_name(name)?;

                    Some((name.to_string(), haystack[span].to_string()))
                })
                .collect(),
        )
    }

    /// Whether matching this never reaches the regex engine, the literals answering it on
    /// their own. Compiling one is budget spent on a regex no lookup will ever run.
    pub fn matches_without_a_regex(&self) -> bool {
        self.original.is_empty() || self.literals.whole
    }

    /// Whether answering this haystack would reach the regex engine, rather than being settled
    /// by the literals. Only then is a compiled copy of the pattern worth holding for it.
    pub fn needs_regex_for(&self, value: &str) -> bool {
        matches!(self.prefilter(value), Prefilter::Undecided)
    }

    /// How much of a haystack the literals can rule out before the regex is reached, as the
    /// length of the longest run they require. Zero means nothing rules it out, and a pattern
    /// nothing rules out is run by every lookup that reaches it.
    pub fn prefilter_strength(&self) -> usize {
        self.literals.runs.iter().map(|run| run.len()).max().unwrap_or(0)
    }

    /// The text every haystack this pattern matches has to hold. It is what rules most of them
    /// out before a regex is run, so a pattern with none is one that is run every time it is
    /// reached.
    pub fn required_literals(&self) -> Vec<&str> {
        self.literals.runs.iter().map(|run| run.as_ref()).collect()
    }

    /// Names of the capture groups, read off the pattern rather than off a compiled regex, so
    /// that a pattern can be checked without paying for its compilation.
    pub fn capture_names(&self) -> Vec<&str> {
        let mut names = Vec::new();
        let bytes = self.original.as_bytes();
        let mut index = 0;

        while index < bytes.len() {
            if bytes[index] == b'\\' {
                index += 2;
                continue;
            }

            // A group name is written between two ascii delimiters, so its bounds are always on
            // a character boundary even when the pattern holds text that is not.
            let opening = [b"(?<".as_slice(), b"(?P<".as_slice()]
                .into_iter()
                .find(|token| bytes[index..].starts_with(token));

            if let Some(opening) = opening {
                let start = index + opening.len();

                if let Some(end) = bytes[start..].iter().position(|byte| *byte == b'>') {
                    names.push(&self.original[start..start + end]);
                }
            }

            index += 1;
        }

        names
    }

    pub fn create_regex(&self) -> Option<Arc<Regex>> {
        // The engines left out here are the ones that trade a lot of memory for speed: a single
        // long pattern with a capture group builds a dense one pass table of a few hundred
        // kilobytes, which a database of this size cannot afford to hold for every entry.
        let configuration = Config::new()
            .onepass_size_limit(Some(0))
            .dfa_size_limit(Some(0))
            .hybrid_cache_capacity(1 << 13);

        match Regex::builder()
            .configure(configuration)
            .syntax(syntax::Config::new().case_insensitive(self.options.ignore_case))
            .build(self.regex.as_str())
        {
            Ok(regex) => Some(Arc::new(regex)),
            Err(e) => {
                tracing::error!("cannot create regex: {:?}", e);

                None
            }
        }
    }

    pub fn compile(&self) -> Self {
        let compiled = self.create_regex();

        LazyRegex {
            regex: self.regex.clone(),
            original: self.original.clone(),
            compiled,
            options: self.options,
            literals: self.literals.clone(),
            anchoring: self.anchoring,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A measure has to describe the lookup that actually happens, so it may never answer
    /// something [`LazyRegex::is_match`] would not.
    #[test]
    fn a_measure_answers_what_a_match_answers() {
        let patterns = ["Nokia[ _](\\d+)", "Mozilla/5\\.0 \\(Linux", "iPhone", "", "a|b"];
        let haystacks = ["Mozilla/5.0 (Linux; Android 13)", "Nokia 6120c", "iPhone", "b", ""];

        for pattern in patterns {
            for options in [RegexOptions::search(true), RegexOptions::anchored(true)] {
                let regex = LazyRegex::new_leaf(pattern, options);

                for haystack in haystacks {
                    assert_eq!(
                        regex.measure_match(haystack).matched,
                        regex.is_match(haystack),
                        "{pattern} against {haystack}",
                    );
                }
            }
        }
    }

    /// The route is the whole point of the measure: it is what tells a lookup that cost a byte
    /// scan from one that cost a compilation.
    #[test]
    fn a_measure_reports_the_route_it_took() {
        let regex = LazyRegex::new_leaf("Android (?<version>[\\d.]+)", RegexOptions::search(false));

        assert_eq!(regex.measure_match("Mozilla/5.0 (Linux)").path, MatchPath::Prefiltered);
        assert_eq!(regex.measure_match("Android 13").path, MatchPath::Compiled);
        assert!(regex.measure_match("Android 13").compile > Duration::ZERO);

        let compiled = regex.compile();

        assert_eq!(compiled.measure_match("Android 13").path, MatchPath::Cached);
        assert_eq!(compiled.measure_match("Android 13").compile, Duration::ZERO);

        // A pattern that is text and nothing else never needs a regex at all.
        let text = LazyRegex::new_leaf("iPhone", RegexOptions::search(false));

        assert_eq!(text.measure_match("an iPhone").path, MatchPath::Literal);
    }

    /// Reading the captures out is a second, heavier run, and it is measured the same way.
    #[test]
    fn a_measured_capture_reads_what_it_captured() {
        let regex = LazyRegex::new_leaf("Android (?<version>[\\d.]+)", RegexOptions::search(false));
        let (measure, captured) = regex.measure_captures("Android 13");

        assert!(measure.matched);
        assert_eq!(measure.path, MatchPath::Compiled);
        assert_eq!(captured.unwrap(), vec![("version".to_string(), "13".to_string())]);

        let (measure, captured) = regex.measure_captures("Linux");

        assert!(!measure.matched);
        assert!(captured.is_none());
    }
}
