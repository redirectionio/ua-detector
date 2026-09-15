//! What an entry is indexed under, which is not the same thing as what it matches.
//!
//! The tree is a radix tree over the text of the patterns, so it can only group entries that
//! open on the same characters, and a child of the root is run on every lookup there is. What
//! stands between an entry and a usable prefix and says nothing about the haystack comes off
//! here: the wildcards it opens and closes with, a wrapping group, a leading token that is
//! optional anyway. What is left is sometimes an alternation, which has no prefix by nature,
//! and is split into one key per branch.
//!
//! The one rule all of it answers to: a haystack the entry matches must still reach one of its
//! keys, or the index prunes an entry that would have matched. Taking off something optional
//! keeps that, taking off something required does not.

/// The keys `regex` is indexed under. Never empty.
pub fn keys(regex: &str) -> Vec<String> {
    let mut keys = Vec::new();

    collect(trim(regex), &mut keys);

    keys
}

fn collect(pattern: &str, keys: &mut Vec<String>) {
    let pattern = unwrap(pattern);

    if let Some(branches) = alternatives(&pattern) {
        for branch in branches {
            collect(&branch, keys);
        }

        return;
    }

    if let Some(spliced) = splice(&pattern) {
        for key in spliced {
            collect(&key, keys);
        }

        return;
    }

    keys.push(pattern);
}

/// Reads a leading group that has to match into the key itself, handing what follows it to
/// each branch: `(?:OPR|OPiOS)/(?<v>...)` is indexed under `OPR/(?<v>...)` and
/// `OPiOS/(?<v>...)`, which sit with the rest of the entries that open on those names instead
/// of at the root with everything whose first character is a parenthesis.
///
/// Every key is shorter than what it came from, so this settles.
fn splice(pattern: &str) -> Option<Vec<String>> {
    let group = leading_group(pattern)?;
    let rest = &pattern[group.close + 1..];

    // How many times the group is there is a question a key made of text cannot carry.
    if rest.starts_with(['?', '*', '+', '{']) {
        return None;
    }

    let inner = &pattern[group.inner..group.close];
    let branches = split(inner);

    // A branch of nothing matches everywhere, and a key that matches everywhere is one the
    // walk can never skip. The group is better left where it is.
    if branches.iter().any(String::is_empty) {
        return None;
    }

    Some(branches.iter().map(|branch| format!("{branch}{rest}")).collect())
}

/// The part of a pattern worth indexing: what is left once the wildcards it opens and closes
/// with are taken off, since they match anywhere by definition.
fn trim(regex: &str) -> &str {
    let stripped = regex.strip_prefix(".*").unwrap_or(regex);

    match stripped.strip_suffix(".*") {
        // A trailing `.*` only counts when the dot is not escaped.
        Some(shorter) if !shorter.ends_with('\\') => shorter,
        _ => stripped,
    }
}

/// Takes the wrappers off a pattern, as far as they go.
///
/// A group around the whole of it -- which is how every entry generated from matomo's client
/// rules is written -- leaves the tree nothing to index on but `(?:`, shared by a thousand
/// others and matching none of them. A leading group that may match zero times is not text the
/// haystack has to hold, so the key does not have to ask for it.
fn unwrap(pattern: &str) -> String {
    let mut pattern = pattern.to_string();

    loop {
        let Some(group) = leading_group(&pattern) else {
            return pattern;
        };

        if group.close == pattern.len() - 1 {
            pattern = pattern[group.inner..group.close].to_string();
            continue;
        }

        let rest = &pattern[group.close + 1..];

        // `*` and `?` both allow none of it. The `?` that may follow either is laziness, which
        // says when to stop and nothing about what is there.
        if rest.starts_with(['?', '*']) {
            let rest = &rest[1..];
            pattern = rest.strip_prefix('?').unwrap_or(rest).to_string();
            continue;
        }

        return pattern;
    }
}

/// Where the group a pattern opens on ends, when it opens on one this may be read through.
///
/// Only a plain group and a named capture qualify. Anything else `(?` introduces changes how
/// what follows is read -- `(?-i:UNIQ)` is the six entries the database spells in capitals --
/// and unwrapping one would hand the tree a key that matches something else.
struct Group {
    /// Where the text inside the group starts.
    inner: usize,
    /// Where its closing parenthesis is.
    close: usize,
}

fn leading_group(pattern: &str) -> Option<Group> {
    let bytes = pattern.as_bytes();

    if bytes.first() != Some(&b'(') {
        return None;
    }

    let inner = match bytes.get(1) {
        Some(b'?') => {
            let opening = ["(?:", "(?<", "(?P<"]
                .into_iter()
                .find(|opening| pattern.starts_with(opening))?;

            match opening {
                "(?:" => 3,
                // A name is written in ascii between two delimiters, so this lands on a
                // character boundary whatever the pattern holds after it.
                _ => pattern.find('>')? + 1,
            }
        }
        _ => 1,
    };

    let mut depth = 0i32;
    let mut in_class = false;
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 1,
            b'[' if !in_class => in_class = true,
            b']' if in_class => in_class = false,
            b'(' if !in_class => depth += 1,
            b')' if !in_class => {
                depth -= 1;

                if depth == 0 {
                    return (index >= inner).then_some(Group { inner, close: index });
                }
            }
            _ => {}
        }

        index += 1;
    }

    None
}

/// The branches of a top level alternation, or `None` when the pattern is not one.
///
/// A branch is a key of its own: a haystack matching `A|B` holds what `A` asks for or what `B`
/// does, so indexing under both prunes nothing.
fn alternatives(pattern: &str) -> Option<Vec<String>> {
    let branches = split(pattern);

    (branches.len() > 1 && !branches.iter().any(String::is_empty)).then_some(branches)
}

/// Cuts a pattern at every `|` of its top level. One part when it has none.
fn split(pattern: &str) -> Vec<String> {
    let bytes = pattern.as_bytes();
    let mut branches = Vec::new();
    let mut depth = 0i32;
    let mut in_class = false;
    let mut start = 0;
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 1,
            b'[' if !in_class => in_class = true,
            b']' if in_class => in_class = false,
            b'(' if !in_class => depth += 1,
            b')' if !in_class => depth -= 1,
            b'|' if !in_class && depth == 0 => {
                branches.push(pattern[start..index].to_string());
                start = index + 1;
            }
            _ => {}
        }

        index += 1;
    }

    branches.push(pattern[start..].to_string());

    branches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_the_wildcards_a_pattern_opens_and_closes_with() {
        assert_eq!(keys(r".*Chrome/(?<v>[\d.]+).*"), [r"Chrome/(?<v>[\d.]+)"]);
        // An escaped dot is text, not a wildcard.
        assert_eq!(keys(r".*Version/1\.*"), [r"Version/1\.*"]);
    }

    /// Every entry generated from matomo's client rules is wrapped in a group, which leaves the
    /// tree `(?:` to index on and nothing else.
    #[test]
    fn unwraps_a_group_around_the_whole_pattern() {
        assert_eq!(keys("(?:nscurl)"), ["nscurl"]);
        assert_eq!(keys(r"(?:KlHttpClientCurl(?:/(?<v>\d+))?)"), [r"KlHttpClientCurl(?:/(?<v>\d+))?"]);
        assert_eq!(keys(r"(?<model>SM-\w+)"), [r"SM-\w+"]);
    }

    #[test]
    fn drops_a_leading_group_that_may_match_nothing() {
        assert_eq!(keys(r"(?:Bing)?Sapphire/(?<v>[\d.]+)"), [r"Sapphire/(?<v>[\d.]+)"]);
        assert_eq!(keys(r"(?:\w+ )*Dorado"), ["Dorado"]);
        // The `?` of a lazy quantifier goes with it, or the key opens on a dangling one.
        assert_eq!(keys(r"(?:a)*?Dorado"), ["Dorado"]);
    }

    /// An alternation has no prefix at all, so each branch is indexed on its own.
    #[test]
    fn splits_a_top_level_alternation_into_one_key_each() {
        assert_eq!(keys(r"(?:OPR|OPiOS)/(?<v>[\d.]+)"), [r"OPR/(?<v>[\d.]+)", r"OPiOS/(?<v>[\d.]+)"]);
        assert_eq!(keys("A|B"), ["A", "B"]);
        // Nested, because the branch of one may be another.
        assert_eq!(keys("(?:(?:a|b)X|c)"), ["aX", "bX", "c"]);
    }

    #[test]
    fn leaves_alone_what_it_cannot_read_through() {
        // A branch of nothing matches everywhere, which is no key at all.
        assert_eq!(keys("(?:a|)X"), ["(?:a|)X"]);
        // Case is the whole point of these six entries.
        assert_eq!(keys("(?-i:UNIQ)"), ["(?-i:UNIQ)"]);
        assert_eq!(keys(r"(?:a|b)+X"), [r"(?:a|b)+X"]);
        assert_eq!(keys(r"(?:ab){2}X"), [r"(?:ab){2}X"]);
        assert_eq!(keys("[abc]X"), ["[abc]X"]);
    }

    /// A group that has to match is text the haystack has to hold, so the key may say so.
    #[test]
    fn reads_through_a_group_that_has_to_match() {
        assert_eq!(keys("(?:Mobile )Safari"), ["Mobile Safari"]);
        assert_eq!(keys(r"(?<brand>Nokia) (?<model>\d+)"), [r"Nokia (?<model>\d+)"]);
    }

    #[test]
    fn never_answers_nothing() {
        for pattern in ["", ".*", ".*.*", "(", "(?:", "|", "(?:|)"] {
            assert!(!keys(pattern).is_empty(), "{pattern} was indexed under nothing");
        }
    }
}
