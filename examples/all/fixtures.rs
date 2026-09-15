//! The fixtures, read as they are rather than compiled into a test.
//!
//! `tests/matomo-device-detector/fixtures/*.yml` is the specification: 38 030 user agents and
//! what each one means. Generating a `#[test]` per case out of them cost forty megabytes of rust
//! and two minutes of build for every change to an entry; reading them at startup costs the
//! second the yaml takes to parse.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use device_detector::Detection;

pub const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/matomo-device-detector/fixtures");

#[derive(serde::Deserialize)]
pub struct Case {
    pub user_agent: String,
    #[serde(default)]
    headers: BTreeMap<String, serde_yaml::Value>,
    #[serde(flatten)]
    expected: serde_yaml::Value,
}

/// Every fixture file, in a stable order.
pub fn files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(FIXTURES)
        .unwrap_or_else(|error| panic!("cannot read {FIXTURES}: {error}"))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "yml"))
        .collect();
    files.sort();

    files
}

pub fn read(path: &std::path::Path) -> Vec<Case> {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));

    serde_yaml::from_str(&content)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()))
}

impl Case {
    /// The headers as a browser sends them.
    pub fn headers(&self) -> Vec<(String, String)> {
        self.headers.iter().map(|(name, value)| (name.clone(), header_of(value))).collect()
    }

    pub fn expected(&self) -> Detection {
        expected_of(&self.expected)
    }
}

/// A header as a browser sends it. A few fixtures give a brand list as a structure instead.
fn header_of(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(text) => text.clone(),
        serde_yaml::Value::Sequence(brands) => brands
            .iter()
            .map(|brand| {
                let text = |key| brand.get(key).and_then(|value| value.as_str()).unwrap_or_default();

                format!("\"{}\";v=\"{}\"", text("brand"), text("version"))
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    }
}

fn expected_of(value: &serde_yaml::Value) -> Detection {
    let mut value = value.clone();

    if let serde_yaml::Value::Mapping(mapping) = &mut value {
        mapping.remove("headers");

        // A fixture writes an absent result as null, or as an empty list.
        for key in ["os", "client", "device"] {
            if mapping.get(key).is_some_and(|value| value.is_null() || value.is_sequence()) {
                mapping.remove(key);
            }
        }
    }

    serde_yaml::from_value(value).expect("a fixture our types can hold")
}
