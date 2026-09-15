//! One row of the dump, and the RFC 4180 it is written in.

use crate::injector::Kind;

#[derive(Clone)]
pub struct Row {
    pub user_agent: String,
    /// What production wrote into `user_agent_simplified`.
    pub name: String,
    /// What production wrote into `user_agent_type`.
    pub kind: Kind,
    /// Requests that carried this user agent, which is what orders the dump.
    pub count: u64,
    /// Where it sat in the dump, so a batch judged across threads reports in the order traffic
    /// cares about rather than in the order a thread finished.
    pub number: usize,
}

impl Row {
    /// Reads a row, or nothing where there is no comparison to make.
    pub fn parse(line: &str, number: usize) -> Option<Row> {
        let fields = fields(line)?;
        let [user_agent, name, kind, count] = fields.as_slice() else {
            return None;
        };

        // Production replaces any name holding "googlebot" with the verdict of a reverse DNS
        // lookup on the client address, which the dump does not carry.
        if name.starts_with("Googlebot") {
            return None;
        }

        Some(Row {
            user_agent: user_agent.clone(),
            name: name.clone(),
            kind: kind.parse().ok().and_then(Kind::from_column)?,
            count: count.parse().ok()?,
            number,
        })
    }
}

/// Splits a line into its fields. A field may be quoted, and a quote inside a quoted field is
/// doubled, which is all of the format the dump uses.
pub fn fields(line: &str) -> Option<Vec<String>> {
    let mut fields = Vec::with_capacity(4);
    let mut rest = line;

    loop {
        let mut value = String::new();

        if let Some(quoted) = rest.strip_prefix('"') {
            rest = quoted;

            loop {
                let end = rest.find('"')?;

                value.push_str(&rest[..end]);
                rest = &rest[end + 1..];

                match rest.strip_prefix('"') {
                    Some(tail) => {
                        value.push('"');
                        rest = tail;
                    }
                    None => break,
                }
            }
        } else {
            let end = rest.find(',').unwrap_or(rest.len());

            value.push_str(&rest[..end]);
            rest = &rest[end..];
        }

        fields.push(value);

        match rest.strip_prefix(',') {
            Some(tail) => rest = tail,
            None if rest.is_empty() => return Some(fields),
            None => return None,
        }
    }
}

pub fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
