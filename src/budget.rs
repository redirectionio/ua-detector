//! What a caller is prepared to spend on compiled regexes.

use crate::regex::LazyRegex;

/// A budget for [`Detector::cache`](crate::Detector::cache) and
/// [`Detector::warm`](crate::Detector::warm), in either of the two units the question is asked
/// in.
///
/// A count of regexes is what the index spends. Bytes are what a caller has. Nothing relates the
/// two without measuring -- a compiled pattern averages some 32 KiB, but the spread across a
/// database of this shape is wide enough that the average says little about any given budget --
/// so both are offered and the accounting is kept in whichever was given.
///
/// ```
/// # use device_detector::{Budget, Detector};
/// let mut detector = Detector::new();
/// detector.cache(Budget::bytes(1 << 30));
/// ```
///
/// Bytes are a target rather than a bound. What a pattern costs is only known once it has been
/// compiled, so the branch that spends the last of a budget overshoots it by one regex. What a
/// branch overspends is carried back to its siblings rather than lost, which is why what is left
/// is held signed.
///
/// What is counted is `regex_automata`'s `memory_usage`: the automata and what hangs off them,
/// and not the lazy DFA caches, which `src/regex.rs` caps at 8 KiB and which are allocated per
/// thread at the first search rather than here. [`Detector::compiled_size`](crate::Detector::compiled_size)
/// reports the same figure for a whole detector, which is what says whether a budget came to
/// what it was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    left: i64,
    unit: Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Regexes,
    Bytes,
}

/// A budget is held signed so that a branch can overshoot its share, and no budget a caller can
/// mean comes near the top of that.
const fn signed(value: u64) -> i64 {
    if value > i64::MAX as u64 { i64::MAX } else { value as i64 }
}

impl Budget {
    /// A budget of `count` compiled regexes, which is what the index counts as it walks.
    pub const fn regexes(count: u64) -> Budget {
        Budget { left: signed(count), unit: Unit::Regexes }
    }

    /// A budget of `size` bytes of compiled regex.
    pub const fn bytes(size: u64) -> Budget {
        Budget { left: signed(size), unit: Unit::Bytes }
    }

    /// Nothing at all, which leaves every regex to be compiled again at every lookup that
    /// reaches it.
    pub const fn none() -> Budget {
        Budget::regexes(0)
    }

    /// What is left of it, in the unit it was given in. Nothing, where the last regex
    /// overshot it.
    pub const fn remaining(&self) -> u64 {
        if self.left < 0 { 0 } else { self.left as u64 }
    }

    pub const fn is_spent(&self) -> bool {
        self.left <= 0
    }

    /// How much of `before` was spent getting here, in the unit both are in. More than `before`
    /// held, where the last regex overshot it.
    pub const fn spent_from(&self, before: Budget) -> u64 {
        let spent = before.left - self.left;

        if spent < 0 { 0 } else { spent as u64 }
    }

    /// `amount` of this budget's unit, which is how a share is handed to a branch and how what
    /// was spent is read back in the unit it was spent in.
    pub const fn take(self, amount: u64) -> Budget {
        Budget { left: signed(amount), unit: self.unit }
    }

    /// What one regex that has just been compiled costs. A pattern that failed to compile costs
    /// nothing, there being nothing to hold.
    pub(crate) fn pay(self, regex: &LazyRegex) -> Budget {
        let cost = match self.unit {
            Unit::Regexes => u64::from(regex.is_compiled()),
            Unit::Bytes => regex.compiled_size(),
        };

        Budget { left: self.left - signed(cost), unit: self.unit }
    }
}

/// Text that does not read as a budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableBudget(pub String);

impl std::fmt::Display for UnreadableBudget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?} is not a budget: a count of regexes, or a size such as 900M", self.0)
    }
}

impl std::error::Error for UnreadableBudget {}

/// A bare number is a count of regexes; a number with a size on it is bytes. `20000`, `900M`,
/// `1GiB`, `2000000B`. Which is to say that the unit is the caller's to choose at the point
/// where the number is written down, which is where they know which one they meant.
impl std::str::FromStr for Budget {
    type Err = UnreadableBudget;

    fn from_str(text: &str) -> Result<Budget, UnreadableBudget> {
        let unreadable = || UnreadableBudget(text.to_string());
        let text = text.trim();
        let digits = text.trim_end_matches(|character: char| !character.is_ascii_digit());
        let count: u64 = digits.parse().map_err(|_| unreadable())?;

        let scale = match text[digits.len()..].trim().to_ascii_uppercase().as_str() {
            "" => return Ok(Budget::regexes(count)),
            "B" => 1,
            "K" | "KB" | "KIB" => 1 << 10,
            "M" | "MB" | "MIB" => 1 << 20,
            "G" | "GB" | "GIB" => 1 << 30,
            _ => return Err(unreadable()),
        };

        Ok(Budget::bytes(count.saturating_mul(scale)))
    }
}

impl std::fmt::Display for Budget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SIZES: [(u64, &str); 3] = [(1 << 30, "GiB"), (1 << 20, "MiB"), (1 << 10, "KiB")];

        match self.unit {
            Unit::Regexes => write!(formatter, "{} regexes", self.remaining()),
            Unit::Bytes => match SIZES.iter().find(|(size, _)| self.remaining() >= *size) {
                Some((size, name)) => {
                    write!(formatter, "{:.1} {name}", self.remaining() as f64 / *size as f64)
                }
                None => write!(formatter, "{} B", self.remaining()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_budget_in_either_unit() {
        assert_eq!("20000".parse(), Ok(Budget::regexes(20_000)));
        assert_eq!("900M".parse(), Ok(Budget::bytes(900 << 20)));
        assert_eq!("1 GiB".parse(), Ok(Budget::bytes(1 << 30)));
        assert_eq!("512B".parse(), Ok(Budget::bytes(512)));
    }

    #[test]
    fn refuses_a_size_it_cannot_read() {
        assert!("1.5G".parse::<Budget>().is_err());
        assert!("plenty".parse::<Budget>().is_err());
    }

    #[test]
    fn prints_what_is_left_in_the_unit_it_was_given() {
        assert_eq!(Budget::regexes(2_000).to_string(), "2000 regexes");
        assert_eq!(Budget::bytes(924 << 20).to_string(), "924.0 MiB");
        assert_eq!(Budget::bytes(512).to_string(), "512 B");
    }
}
