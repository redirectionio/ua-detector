//! Reports what a fixture file still expects that the entries do not produce.
//!
//!     cargo run --example report -- smart_display          # only what fails
//!     cargo run --example report -- smart_display --shapes # failures grouped by shape

use std::collections::BTreeMap;

use device_detector::Detection;

#[derive(serde::Deserialize)]
struct Case {
    user_agent: String,
    #[serde(default)]
    headers: BTreeMap<String, serde_yaml::Value>,
    #[serde(flatten)]
    expected: serde_yaml::Value,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let name = args.next().expect("usage: report <fixture> [--shapes]");
    let shapes = args.any(|arg| arg == "--shapes");

    let path = format!("tests/matomo-device-detector/fixtures/{name}.yml");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let cases: Vec<Case> = serde_yaml::from_str(&content).expect("a valid fixture");

    let detector = device_detector::shared();
    let mut failures = Vec::new();

    for case in &cases {
        let expected: Detection = expected_of(&case.expected);
        let headers: Vec<(String, String)> = case
            .headers
            .iter()
            .map(|(name, value)| (name.clone(), header_of(value)))
            .collect();
        let headers: Vec<(&str, &str)> =
            headers.iter().map(|(name, value)| (name.as_str(), value.as_str())).collect();
        let detected = detector.detect_with_headers(&case.user_agent, &headers);

        if detected.as_ref() != Some(&expected) {
            failures.push((case, expected, detected));
        }
    }

    if shapes {
        let mut grouped: BTreeMap<String, usize> = BTreeMap::new();

        for (case, _, _) in &failures {
            *grouped.entry(shape_of(&case.user_agent)).or_default() += 1;
        }

        for (shape, count) in &grouped {
            println!("{count:>5}  {shape}");
        }

        println!("\n{} shape(s), {}/{} case(s) failing", grouped.len(), failures.len(), cases.len());

        return;
    }

    for (case, expected, detected) in &failures {
        println!("UA       {}", case.user_agent);
        println!("expected {expected:?}");

        match detected {
            None => println!("got      nothing\n"),
            Some(detected) => println!("got      {detected:?}\n"),
        }
    }

    println!("{}/{} case(s) failing", failures.len(), cases.len());
}

/// A few fixtures give a brand list as a structure rather than as the header text a browser
/// actually sends. Render it back to the wire form.
fn header_of(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(text) => text.clone(),
        serde_yaml::Value::Sequence(brands) => brands
            .iter()
            .map(|brand| {
                let text = |key| brand.get(key).and_then(|v| v.as_str()).unwrap_or_default();

                format!("\"{}\";v=\"{}\"", text("brand"), text("version"))
            })
            .collect::<Vec<String>>()
            .join(", "),
        _ => String::new(),
    }
}

/// A fixture spells an undetected operating system as an empty list and an undetected client
/// as null, neither of which our entry format uses.
fn expected_of(value: &serde_yaml::Value) -> Detection {
    let mut value = value.clone();

    if let serde_yaml::Value::Mapping(mapping) = &mut value {
        mapping.remove("headers");

        for key in ["os", "client", "device"] {
            if mapping.get(key).is_some_and(|value| value.is_null() || value.is_sequence()) {
                mapping.remove(key);
            }
        }
    }

    serde_yaml::from_value(value).expect("a fixture our types can hold")
}

/// Strips the parts of a user agent that vary between otherwise identical cases, so failures
/// can be counted per shape rather than one by one.
fn shape_of(user_agent: &str) -> String {
    let mut shape = String::with_capacity(user_agent.len());
    let mut chars = user_agent.chars().peekable();

    while let Some(char) = chars.next() {
        if !char.is_ascii_digit() {
            shape.push(char);
            continue;
        }

        while chars.peek().is_some_and(|next| next.is_ascii_digit() || *next == '.') {
            chars.next();
        }

        shape.push('#');
    }

    shape
}
