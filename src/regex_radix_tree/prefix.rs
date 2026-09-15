#[cfg(test)]
pub fn common_prefix(left: &str, right: &str) -> String {
    let size = common_prefix_char_size(left, right);

    get_prefix_with_char_size(left, size)
}

/// Where a regex may be cut to serve as a node prefix, read once.
///
/// Inserting a regex compares it against every node it walks through and every child of those,
/// and each comparison would otherwise walk the pattern again.
pub struct Cuts<'a> {
    regex: &'a str,
    /// Ascending, and empty when no prefix of the regex is usable at all.
    safe: Vec<u32>,
}

impl<'a> Cuts<'a> {
    pub fn of(regex: &'a str) -> Cuts<'a> {
        Cuts {
            regex,
            safe: safe_cuts(regex),
        }
    }

    pub fn regex(&self) -> &'a str {
        self.regex
    }

    /// Longest prefix this regex shares with `other`, usable as a node.
    pub fn shared_with(&self, other: &str) -> u32 {
        let raw = raw_common_prefix_char_size(self.regex, other);

        if raw == 0 {
            return 0;
        }

        self.at_most(raw).min(largest_safe_cut(other, raw))
    }

    /// Same, giving up as soon as the prefix cannot beat `least`.
    pub fn shared_with_over(&self, other: &str, least: u32) -> u32 {
        let raw = raw_common_prefix_char_size(self.regex, other);

        if raw <= least {
            return 0;
        }

        self.at_most(raw).min(largest_safe_cut(other, raw))
    }

    fn at_most(&self, max: u32) -> u32 {
        match self.safe.partition_point(|cut| *cut <= max) {
            0 => 0,
            after => self.safe[after - 1],
        }
    }
}

/// Longest prefix shared by both regexes that is safe to use as a tree node.
///
/// A node prefix `P` is only usable to prune a subtree if every string matched by a child
/// regex also matches `P` used as a search pattern. That holds when the child is the
/// concatenation `P . rest`, so the cut has to land on a boundary where appending text keeps
/// that relation:
///
/// * it must be syntactically complete: not inside a character class, a `{n,m}` quantifier,
///   a group, nor right after a backslash;
/// * the following token must not be a quantifier that can match zero times, otherwise `ab*`
///   would be pruned by `ab` even though it matches `a`;
/// * neither regex may contain a top level alternation, since `abc|def` matching `def` says
///   nothing about the prefix `abc`.
#[cfg(test)]
pub fn common_prefix_char_size(left: &str, right: &str) -> u32 {
    let raw = raw_common_prefix_char_size(left, right);

    if raw == 0 {
        return 0;
    }

    largest_safe_cut(left, raw).min(largest_safe_cut(right, raw))
}

fn raw_common_prefix_char_size(left: &str, right: &str) -> u32 {
    let mut size = 0;
    let mut left_chars = left.chars();
    let mut right_chars = right.chars();

    loop {
        match (left_chars.next(), right_chars.next()) {
            (Some(l), Some(r)) if l == r => size += 1,
            _ => return size,
        }
    }
}

fn safe_cuts(regex: &str) -> Vec<u32> {
    let mut cuts = Vec::new();
    let mut depth: i32 = 0;
    let mut in_class = false;
    let mut class_len = 0;
    let mut in_brace = false;
    let mut index: u32 = 0;
    let mut chars = regex.chars();

    while let Some(char) = chars.next() {
        if !in_class && !in_brace && depth == 0 && !is_quantifier_start(char) {
            cuts.push(index);
        }

        if in_class {
            match char {
                '\\' => {
                    class_len += 1;
                    index += 1 + u32::from(chars.next().is_some());
                    continue;
                }
                ']' if class_len > 0 => in_class = false,
                _ => class_len += 1,
            }

            index += 1;
            continue;
        }

        if in_brace {
            in_brace = char != '}';
            index += 1;
            continue;
        }

        match char {
            '\\' => {
                index += 1 + u32::from(chars.next().is_some());
                continue;
            }
            '[' => {
                in_class = true;
                class_len = 0;
            }
            '{' => in_brace = true,
            '(' => depth += 1,
            ')' => depth -= 1,
            // A top level alternation makes every non empty prefix unusable
            '|' if depth == 0 => return Vec::new(),
            _ => {}
        }

        index += 1;
    }

    if !in_class && !in_brace && depth == 0 {
        cuts.push(index);
    }

    cuts
}

fn largest_safe_cut(regex: &str, max: u32) -> u32 {
    let mut best = 0;
    let mut depth: i32 = 0;
    let mut in_class = false;
    let mut class_len = 0;
    let mut in_brace = false;
    let mut index: u32 = 0;
    let mut chars = regex.chars();

    // Read once, without keeping the pattern around: inserting a regex compares it against
    // every child of every node it walks through, so this runs millions of times.
    while let Some(char) = chars.next() {
        if !in_class && !in_brace && depth == 0 && index <= max && !is_quantifier_start(char) {
            best = index;
        }

        if in_class {
            match char {
                '\\' => {
                    class_len += 1;
                    index += 1 + u32::from(chars.next().is_some());
                    continue;
                }
                ']' if class_len > 0 => in_class = false,
                _ => class_len += 1,
            }

            index += 1;
            continue;
        }

        if in_brace {
            in_brace = char != '}';
            index += 1;
            continue;
        }

        match char {
            '\\' => {
                index += 1 + u32::from(chars.next().is_some());
                continue;
            }
            '[' => {
                in_class = true;
                class_len = 0;
            }
            '{' => in_brace = true,
            '(' => depth += 1,
            ')' => depth -= 1,
            // A top level alternation makes every non empty prefix unusable
            '|' if depth == 0 => return 0,
            _ => {}
        }

        index += 1;
    }

    if !in_class && !in_brace && depth == 0 && index <= max {
        best = index;
    }

    best
}

fn is_quantifier_start(char: char) -> bool {
    matches!(char, '*' | '+' | '?' | '{')
}

pub fn get_prefix_with_char_size(str: &str, size: u32) -> String {
    if size == 0 {
        return "".to_string();
    }

    let mut chars = str.chars();
    let mut prefix = Vec::new();

    for _i in 0..size {
        match chars.next() {
            Some(char) => prefix.push(char),
            None => {
                return prefix.into_iter().collect();
            }
        }
    }

    prefix.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_on_a_plain_boundary() {
        assert_eq!(common_prefix("/a/b", "/a/c"), "/a/");
    }

    #[test]
    fn never_cuts_inside_a_character_class() {
        assert_eq!(common_prefix("a[bc]d", "a[bd]e"), "a");
    }

    #[test]
    fn never_cuts_inside_a_brace_quantifier() {
        assert_eq!(common_prefix("a{2,3}b", "a{2,4}c"), "");
        assert_eq!(common_prefix("a{2,3}bc", "a{2,3}bd"), "a{2,3}b");
    }

    #[test]
    fn never_cuts_inside_a_group() {
        assert_eq!(common_prefix("(?:ab)c", "(?:ad)e"), "");
        assert_eq!(common_prefix("(?:ab)c", "(?:ab)e"), "(?:ab)");
    }

    #[test]
    fn never_cuts_right_before_a_nullable_quantifier() {
        assert_eq!(common_prefix("ab*c", "abd"), "a");
        assert_eq!(common_prefix("a(?:b)?c", "a(?:b)d"), "a");
    }

    #[test]
    fn never_cuts_a_regex_holding_a_top_level_alternation() {
        assert_eq!(common_prefix("abc|def", "abcd"), "");
        assert_eq!(common_prefix("abcd", "abc|def"), "");
        assert_eq!(common_prefix("a(?:bc|de)f", "a(?:bc|de)g"), "a(?:bc|de)");
    }

    #[test]
    fn never_cuts_after_a_backslash() {
        assert_eq!(common_prefix("a\\.b", "a\\.c"), "a\\.");
        assert_eq!(common_prefix("a\\db", "a\\wc"), "a");
    }
}
