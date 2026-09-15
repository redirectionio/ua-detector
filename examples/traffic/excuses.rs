//! What production is forgiven for.
//!
//! The dump comes from a device detector some years old. Where it disagrees with this library,
//! it is often the one that is wrong: a bot released since, an application it never knew, a name
//! it simply got wrong. Those rows are declared, one entry per reason, so that a run stops only
//! on rows this library has to answer for.

use regex::Regex;

use crate::injector::Kind;
use crate::row::Row;

/// One reason to let a divergence stand. Every condition an entry states must hold, and an entry
/// that states none would excuse the whole dump, so it is refused.
struct Excuse {
    /// Searched in the user agent, ignoring case, as the corpus is.
    pattern: Option<Regex>,
    /// Only the rows where production gave this name.
    says: Option<String>,
    /// Only the rows production could not name, which the log injector writes as the user agent
    /// itself.
    says_nothing: Option<bool>,
    /// Only the rows where the two libraries give the same name and differ on the type alone.
    names_agree: Option<bool>,
    /// Only the rows where this library gives this name.
    we_say: Option<String>,
    /// Only the rows where this library gives this type.
    we_say_type: Option<Kind>,
    reason: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    says: Option<String>,
    #[serde(default)]
    says_nothing: Option<bool>,
    #[serde(default)]
    names_agree: Option<bool>,
    #[serde(default)]
    we_say: Option<String>,
    #[serde(default)]
    we_say_type: Option<u16>,
    reason: String,
}

#[derive(Default)]
pub struct Excuses {
    entries: Vec<Excuse>,
}

impl Excuses {
    /// A missing file is no error: it means nothing is excused yet.
    pub fn read(path: &str) -> Result<Excuses, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Excuses::default()),
            Err(error) => return Err(error.to_string()),
        };

        let entries: Vec<Entry> = serde_yaml::from_str(&text).map_err(|error| error.to_string())?;
        let entries = entries
            .into_iter()
            .map(|entry| {
                if entry.pattern.is_none()
                    && entry.says.is_none()
                    && entry.says_nothing.is_none()
                    && entry.names_agree.is_none()
                    && entry.we_say.is_none()
                    && entry.we_say_type.is_none()
                {
                    return Err(format!("{}: an entry that states no condition", entry.reason));
                }

                Ok(Excuse {
                    pattern: entry
                        .pattern
                        .map(|pattern| Regex::new(&format!("(?i){pattern}")))
                        .transpose()
                        .map_err(|error| error.to_string())?,
                    says: entry.says,
                    says_nothing: entry.says_nothing,
                    names_agree: entry.names_agree,
                    we_say: entry.we_say,
                    we_say_type: entry
                        .we_say_type
                        .map(|kind| {
                            Kind::from_column(kind).ok_or_else(|| format!("{kind} is no type"))
                        })
                        .transpose()?,
                    reason: entry.reason,
                })
            })
            .collect::<Result<Vec<Excuse>, String>>()?;

        Ok(Excuses { entries })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn reason(&self, excuse: usize) -> &str {
        &self.entries[excuse].reason
    }

    /// Which entry excuses this row, if one does.
    pub fn excuses(&self, row: &Row, found_name: &str, found_kind: Kind) -> Option<usize> {
        self.entries.iter().position(|excuse| {
            excuse.says.as_ref().is_none_or(|says| *says == row.name)
                && excuse
                    .says_nothing
                    .is_none_or(|nothing| nothing == (row.name == row.user_agent))
                && excuse.names_agree.is_none_or(|agree| agree == (row.name == found_name))
                && excuse.we_say.as_ref().is_none_or(|name| name == found_name)
                && excuse.we_say_type.is_none_or(|kind| kind == found_kind)
                && excuse
                    .pattern
                    .as_ref()
                    .is_none_or(|pattern| pattern.is_match(&row.user_agent))
        })
    }
}
