# Working on this repository

## What the library is

A user agent, plus the client hints that come with it, goes in; the device, the operating system
and the client it names come out. `README.md` covers the shape of it. This file covers what an
agent needs to know before touching anything.

## The specification is the fixtures, and only the fixtures

`tests/matomo-device-detector/fixtures/*.yml` says what every one of 38 030 user agents means.
That corpus, pinned to one revision of matomo-org/device-detector in `castor.php`, is the whole
specification. A change is right when the tests pass and wrong when they do not; there is no other
authority to appeal to.

Matomo's own regexes are not vendored, but they may be read: this library is LGPL-3.0-or-later,
the same family as matomo's, so there is nothing to keep out. Read them for what they say about
*which* tokens carry meaning -- that is the question the fixtures answer only by example, one
user agent at a time.

They still cannot be copied. They are written for PCRE, which backtracks, while this library uses
the `regex` crate, which does not: lookarounds and backreferences are unavailable and a pattern
taken across would not compile. Write the pattern yourself, and let the fixtures say whether it
is right.

`git clone https://github.com/matomo-org/device-detector` at `MATOMO_REVISION` from `castor.php`,
somewhere outside this tree; `regexes/` is what you want.

## Entries

The database is `src/devices/*.yml`, one entry per regex, described in `src/devices/README.md`.
Read that before adding one. In short:

- the regex covers a whole user agent, anchored at both ends by the loader, and is matched
  ignoring case, as the corpus is. An entry that genuinely depends on case says so with
  `(?-i:...)` -- there are six model tokens in the database that differ from another only in
  their capitals, `UNIQ` against `UniQ` among them;
- result fields may hold `{placeholder}` references to the regex's named captures, optionally
  through filters, chained left to right: `{model|spaces|title}`;
- `headers:` adds conditions on request headers, searched rather than anchored, whose captures
  share the same namespace;
- every entry that matches contributes: they are applied in ascending order of `(priority, number
  of header conditions, position across the files in path order)`, each laying what it says over
  what came before, so **the last one wins**. An entry may therefore answer for part of a user
  agent and leave the rest to another;
- `priority:` defaults to 0 and is signed. An entry that matches anything of its shape goes
  *below* one that names something: -1 for a fallback, -2 for a catch-all, -3 for an entry that
  speaks to one axis alone;
- leaving a field out and writing `''` are different. Out means another entry may fill it; `''`
  means it is empty and overwrites one that was found.

Six files are not one entry per user agent shape but one axis each, and none of them describes a
whole user agent:

- `systems.yml` reads the platform token, and for the desktops says that the device is a desktop,
  without which a browser on one reads as a phone;
- `systems_fallback.yml` is every operating system the corpus knows, generated from matomo's
  `regexes/oss.yml`, a rung lower;
- `clients.yml` reads the token a client writes to name itself, wherever it sits in the string.
  Hand written, ordered least particular first because the last entry to match wins;
- `clients_fallback.yml` is every client the corpus knows, generated from `regexes/client/*.yml`,
  so it only answers where nothing above it does;
- `clients_promoted.yml` lifts the few dozen of those the traffic holds back above the generic
  browsers. Only tokens of seven characters or more, or a token with a version after it: shorter
  ones read inside other strings and cost more than they pay;
- `models.yml` reads a model token and says nothing else.

`bots.yml` is ordered by matomo's own rule order, reversed, since matomo stops at its first match
and merging keeps the last: a user agent naming two bots has to resolve to the one matomo names.
`bots_generic.yml` is the tokens a crawler gives itself away with, one entry each, below every
named bot, and the compatibility comment a crawler declares itself in.

A bot is the whole answer: an entry that names one wins over everything else that matched,
whatever its priority, which is matomo's own order of parsers said the other way round. Priority
among bot entries therefore decides which bot, and nothing else, and no later entry can take a
bot back off -- an over-wide bot regex is the one mistake this database cannot correct elsewhere.

Matomo compiles every one of its rules behind `(?:^|[^A-Z0-9_-]|[^A-Z0-9-]_|sprd-|MZ-)`
(`Parser/AbstractParser.php:392`), so a token only matches at the start of a string or after
something that is not a letter, digit, underscore or hyphen. That is deliberately *not* carried
over: the tree indexes entries by the text of their regexes, and a boundary in front gives every
one of them the same forty character prefix, so the index prunes nothing -- a corpus run goes from
seconds to over a minute. The handful of tokens short enough to read inside a longer word are
given a `\b` one at a time instead.

Matomo's rule order is precedence: it tries its parsers FeedReader, MobileApp, MediaPlayer, PIM,
Browser, Library, and inside each its rules top to bottom, stopping at the first that matches.
Merging keeps the *last*, so both orders are reversed on the way in. Where a rule of theirs uses a
lookaround the regex crate will not compile, it can be said either as an ordering --
`HMSCore(?!.*HuaweiBrowser)` is Huawei Mobile Services placed above the browser that must beat it
-- or written out as the alternation of everything the lookaround leaves, which is what the
generic bot tokens and the Electron applications do.

Widen a pattern only as far as the fixtures allow: `RG\d+` looks like a fair reading of ANBERNIC's
range and takes RugGear's phones with it.

## Running things

```
cargo test --release --no-default-features       # the whole suite, seconds
cargo test --release                             # the same, against the database as it ships
cargo test --release --lib                       # the unit tests alone
cargo test --release --no-default-features --test corpus bots::   # one fixture
cargo test --release --no-default-features --test pinned          # the hand written cases
cargo run --release --example report <fixture>   # the failing cases of one fixture, in full
cargo run --release --example all                # every fixture in one process, a count per file
cargo bench                     # loading, compiling and detecting, per cache budget
cargo run --release --example traffic -- <dump.csv>   # what production answers that this does not
cargo run --release --example compare -- <dump.tsv>   # the same, over a dump of raw rows
cargo run --release --example memory [budget...] # what the index costs to hold, per budget
cargo run --release --example trace -- "<user agent>" # where one detection spends itself
castor matomo:sync              # pull the fixtures again from the pinned revision
```

Nothing about the corpus is compiled: `tests/corpus` reads the fixtures at startup and hands one
case at a time to the harness through `libtest-mimic`, so the names and the filters read as a
`#[test]` would -- `bots::17` is the seventeenth case of `bots.yml`. `tests/pinned` is the rest,
the two files that are not read from the fixtures.

The database is embedded with `include_dir!`, which registers every yaml with rustc, so a
one-line change to a regex recompiles whatever links the library. Without the `embed` feature
nothing is embedded and the files are read from `src/devices` at startup, which takes a change to
an entry down to the second the detector spends loading -- four seconds against two minutes.
`DEVICE_DETECTOR_DEVICES` names another directory to read, which works either way round;
`build.rs` stops watching the yaml whenever what answers is not the embedded copy. Toggling the
feature does rebuild, so stay on one side of it while working.

`cargo bench` is criterion, so it keeps the last run and tells you what moved; `--save-baseline`
and `--baseline` name one to keep. It detects over a stride across the corpus rather than all
38 030 agents, because at a budget of zero a lookup costs about six milliseconds against half a
millisecond cached, and ten samples of the whole corpus at that rate is most of an hour. That
makes the default good for comparing two runs and not for quoting on its own; `DETECT_SAMPLE`
raises it when a number has to stand by itself. Criterion sizes a run by the measurement time it
is given rather than by the sample count, so a group whose setup costs more than what it measures
-- `cache` reloads a detector that has not spent its budget -- asks for no measurement time at
all. Memory is the one question criterion cannot answer, resident size belonging to the process
rather than to an operation sampled ten times, which is what the `memory` example is left for.

`cargo build` does not rebuild examples, so pass `--examples` when you mean to.

`tests/pinned/production.rs` is written by hand: it pins user agents the corpus shows once, or
not at all. Add a case there when a production divergence is fixed, and check the case fails
against the database as it was.

## The traffic loop

`examples/traffic` reads a dump of production traffic -- one row per distinct user agent, the
name and the type the log injector gave it, and how many requests carried it, heaviest first --
and reports where this library answers differently:

```
cargo run --release --no-default-features --example traffic -- ~/user_agent.csv
cargo run --release --no-default-features --example traffic -- ~/user_agent.csv --keep-going --top 20
```

Plainly, it stops at the first row it cannot answer as production did and writes that row to
`tests/pinned/traffic/cases.csv`, from which `tests/pinned/traffic.rs` is generated, so the cases
accumulate in the order the traffic cares about. `--keep-going` surveys instead, printing the
divergences grouped by the pair of answers with the requests behind each summed, which is what
says where the next entry is worth writing. Either way it ends on what the pass came to in
requests rather than in rows, which is the only unit that says whether a change was worth making.

Production runs a device detector some years old, so a divergence is not automatically this
library's to fix: a bot released since, an application it never knew, a name matomo has changed.
Those go in `tests/pinned/traffic/false-negatives.yml`, one entry per reason, which is also where
a reader finds out why a row was let through. An entry states any of `pattern` (a regex searched
in the user agent), `says` (production's name), `says_nothing` (production named nothing, and the
log injector wrote the user agent into the name column), `names_agree` (both name the same client
and only the type differs), `we_say`, `we_say_type`, and every condition it states has to hold.

The type column is the log injector's `DeviceType`, which `examples/compare/injector.rs` reads a
detection into and the test shares: 1 desktop, 2 mobile, 3 tool, 4 search bot. A bot lands in 4
only when its category holds "search bot", which is why a crawler in matomo's "AI Assistant"
category shows up as a type divergence rather than a name one.

## Performance, and the shape of the trap

Entries are held in a radix tree over the regexes themselves, and nothing is compiled when the
database is read.

The tree indexes an entry under the keys `src/index.rs` reads off its pattern, which is not the
pattern itself. It can only group two entries that open on the same characters, and a child of
the root is run on every lookup there is, so anything standing between an entry and a prefix
worth sharing is taken off first: the wildcards it opens and closes with, a wrapping group, a
leading token that was optional anyway. What is left is sometimes an alternation, which has no
prefix by nature, and is indexed under one key per branch instead -- `(?:OPR|OPiOS)/(?<v>...)`
sits under `OPR/` and `OPiOS/` rather than at the root. An entry is therefore offered once per key
its haystack reached, and `Detector::candidates_of` drops the repeats.

Everything there answers to one rule: a haystack the entry matches must still reach at least one
of its keys. Taking off something optional keeps that; taking off something required does not,
which is why a group is only dropped when it may match nothing, and why `(?-i:UNIQ)` is left
alone. Without it the index prunes an entry that would have matched, which
`the_index_never_prunes_a_rule_that_matches` and the corpus are there to catch.

Two more consequences worth keeping in mind:

- a regex outside the cache budget is compiled again at *every* lookup that reaches it, so a small
  budget can be far slower than none of the memory it saves is worth. `Detector::cache` spends its
  budget breadth first from the root, and hands what a level leaves over to the branches under it
  in proportion to the number of rules each holds, since that is roughly where the haystacks go;
- a long pattern with a capture group builds a dense one pass table of a quarter of a megabyte,
  which is why the engines are configured the way they are in `src/regex.rs`. `cargo run --release
  --example regexcost` is the measurement that decision rests on.

`Detector::trace` answers where one user agent went, which is the question a benchmark cannot:
the same walk as `find`, with a clock around every regex and the route each of them took --
ruled out by the literal prefilter, answered as text, run from the cache, or compiled on the
spot. `cargo run --release --example trace` reads one out. Two passes, a plain detection for the
total and an instrumented one for the shares, because reading the clock twice per regex is not
free; compare the shares, never the sums.

The budget is spent on the index first and the entries get what is left, so a budget large enough
to cover the tree and no more leaves every candidate entry compiling its own regex on every
lookup. That shows up in the trace as a candidate list where every route reads `compiled`, and it
is usually the largest single item in a detection.

A budget spent blind is mostly spent wrong, and no ordering fixes that. A lookup walks some 1 300
keys, the literal prefilter answers all but fifty without a regex, and which fifty depends
entirely on the user agent: a corpus of 951 runs 5 415 of the 74 709 regexes the database holds,
and 500 of those account for nineteen runs in twenty. A compiled regex is about 32 KiB, so
holding the database outright is over a gigabyte and not an option.

`Detector::warm` is the answer where there is traffic to hand: it walks the index for each user
agent exactly as a lookup would and pays for the regexes the walk had to run, index and entries
alike. Ninety-five user agents and a budget of 500 leave 93% of the index and 85% of the entries
waiting, at 1.7 ms a detection; the same 500 spent blind leave 50% and 13%, at 6.7 ms. Blind at
20 000 -- 640 MiB -- is still slower than warm at 500. One user agent on its own costs 61
compilations to cover completely, which `cargo run --release --example trace -- "<ua>" --warm`
will show.

Three things decide where `Detector::cache` puts a budget it has to spend blind, all of them
following from the prefilter rather than from the tree:

- a pattern the literals answer on their own never reaches the regex engine, so it is never
  compiled, whatever the budget. That is 4 720 of the keys;
- an entry is compiled the moment the index offers it as a candidate, with no prefilter in front
  of it, so an entry whose pattern requires no text at all is read for every user agent there
  is. Those are paid for before the index gets its share -- half the budget at most, since what
  the index prunes is never read at all;
- within a level, the weakest prefilter goes first, for the same reason. It only decides where
  the budget runs out, which is somewhere in a level every time, and it is worth about a fifth
  of a detection at a budget of 5 000.

## Conventions

- Commits: conventional prefixes, a subject that says what changed for a reader of the code, a body
  that says why when the why is not obvious. **No co-author trailer.**
- Comments: only where they explain something the code cannot say — a constraint, a trade-off, the
  reason a bound is where it is. Never restate the next line, and keep it to a line or two.
- Work one typology at a time and commit between each, so a regression can be bisected to a file.
- Licence is LGPL-3.0-or-later. `src/devices` descends from matomo's fixtures and this stays in
  step with them; nothing more permissive can be assumed.
