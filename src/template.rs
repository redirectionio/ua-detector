//! Substitution of `{name}` placeholders from the named captures of a matched regex.
//!
//! A placeholder may carry a filter, written `{name|filter}`, for the cases where the user
//! agent spells a value differently from the way it is reported.

/// Applies the filter a placeholder asks for. Unknown filters are left to the caller to
/// reject, which [`placeholders`] does at load time.
pub const FILTERS: [&str; 4] = ["dots", "spaces", "title", "upper"];

fn apply(filter: &str, value: &str) -> String {
    match filter {
        // `Mac OS X 10_10_3` is reported as `10.10.3`
        "dots" => value.replace(['_', '-'], "."),
        // `8227L_demo` is reported as `8227L demo`
        "spaces" => value.replace('_', " "),
        // `QUAD-CORE T3 k2001o` is reported as `QUAD-CORE T3 K2001O`
        "upper" => value.to_uppercase(),
        // `THOR PRO` is reported as `Thor Pro`
        "title" => value
            .split(' ')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect(),
                }
            })
            .collect::<Vec<String>>()
            .join(" "),
        _ => value.to_string(),
    }
}

/// Splits `name|filter|filter` into the capture name and the filters to apply, in order.
fn split(reference: &str) -> (&str, Vec<&str>) {
    let mut parts = reference.split('|');

    (parts.next().unwrap_or(reference), parts.collect())
}

/// Replaces every `{name}` of a template by what `lookup` returns for it.
///
/// A pattern can capture from the user agent and from several headers at once, so the groups
/// do not all come from the same match.
pub fn fill(template: &str, lookup: &dyn Fn(&str) -> Option<String>) -> String {
    if !template.contains('{') {
        return template.to_string();
    }

    let mut filled = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        filled.push_str(&rest[..open]);

        let Some(close) = rest[open..].find('}').map(|end| open + end) else {
            break;
        };

        let (name, filters) = split(&rest[open + 1..close]);
        let captured = lookup(name).unwrap_or_default();

        filled.push_str(&filters.iter().fold(captured, |value, filter| apply(filter, &value)));

        rest = &rest[close + 1..];
    }

    filled.push_str(rest);
    filled
}

/// Names every `{placeholder}` a template refers to, with the filters it asks for.
pub fn placeholders(template: &str) -> Vec<(&str, Vec<&str>)> {
    let mut names = Vec::new();
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|end| open + end) else {
            break;
        };

        names.push(split(&rest[open + 1..close]));
        rest = &rest[close + 1..];
    }

    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn captures_of(regex: &str, haystack: &str) -> impl Fn(&str) -> Option<String> {
        let captures = regex::Regex::new(regex)
            .unwrap()
            .captures(haystack)
            .expect("the regex to match")
            .iter()
            .zip(regex::Regex::new(regex).unwrap().capture_names())
            .filter_map(|(group, name)| Some((name?.to_string(), group?.as_str().to_string())))
            .collect::<std::collections::HashMap<String, String>>();

        move |name: &str| captures.get(name).cloned()
    }

    #[test]
    fn fills_a_placeholder_from_a_named_capture() {
        let captures = captures_of(r"Android (?<version>[\d.]+)", "Linux; Android 12.1; Pixel");

        assert_eq!(fill("{version}", &captures), "12.1");
        assert_eq!(fill("Android {version} release", &captures), "Android 12.1 release");
        assert_eq!(fill("no placeholder", &captures), "no placeholder");
    }

    #[test]
    fn leaves_an_unknown_placeholder_empty() {
        let captures = captures_of(r"Android (?<version>[\d.]+)", "Android 12.1");

        assert_eq!(fill("{missing}", &captures), "");
    }

    #[test]
    fn lists_the_placeholders_a_template_refers_to() {
        assert_eq!(
            placeholders("Galaxy {model} {version|dots}"),
            vec![("model", vec![]), ("version", vec!["dots"])]
        );
        assert_eq!(placeholders("none"), Vec::new());
    }

    #[test]
    fn applies_chained_filters_in_order() {
        let captures = captures_of(r"; (?<model>\S+)\)", "Android 9; ZOEY_SMART)");

        assert_eq!(fill("{model|spaces|title}", &captures), "Zoey Smart");
    }

    #[test]
    fn rewrites_a_model_the_user_agent_spells_differently() {
        let captures = captures_of(r"Android 9; (?<model>\S+)", "Android 9; 8227L_demo)");

        assert_eq!(fill("{model|spaces}", &captures), "8227L demo)");
        assert_eq!(fill("{model|upper}", &captures), "8227L_DEMO)");
    }

    #[test]
    fn rewrites_a_version_the_user_agent_spells_with_underscores() {
        let captures = captures_of(r"Mac OS X (?<version>[\d_]+)", "Intel Mac OS X 10_10_3)");

        assert_eq!(fill("{version|dots}", &captures), "10.10.3");
    }
}
