//! What redirection.io's log injector makes of a detection.
//!
//! It stores two columns per request, `user_agent_simplified` and `user_agent_type`, and both
//! come from a detection by rust-device-detector. Comparing this library against production
//! means replaying that derivation, so it lives here rather than in either of the two examples
//! that need it, and in `tests/traffic.rs`, which reaches for it across the tree.

#![allow(dead_code)]

use device_detector::{Detection, MOBILE_ONLY_BROWSERS};

/// `DeviceType` of the log injector, whose numbers are what the column holds.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Kind {
    Desktop = 1,
    Mobile = 2,
    Tool = 3,
    Bot = 4,
}

impl Kind {
    pub fn from_column(value: u16) -> Option<Kind> {
        match value {
            1 => Some(Kind::Desktop),
            2 => Some(Kind::Mobile),
            3 => Some(Kind::Tool),
            4 => Some(Kind::Bot),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Desktop => "desktop",
            Kind::Mobile => "mobile",
            Kind::Tool => "tool",
            Kind::Bot => "bot",
        }
    }
}

/// What the log injector writes into the two columns, given a detection.
pub fn classify(detection: &Detection, user_agent: &str) -> (Kind, String) {
    if let Some(bot) = &detection.bot {
        let kind = if bot.category.to_lowercase().contains("search bot") {
            Kind::Bot
        } else {
            Kind::Tool
        };

        return (kind, bot.name.clone());
    }

    let name = match &detection.client {
        Some(client) => client.name.clone(),
        None => user_agent.to_string(),
    };

    if is_mobile(detection) || is_tablet(detection) {
        (Kind::Mobile, name)
    } else if is_desktop(detection) {
        (Kind::Desktop, name)
    } else {
        (Kind::Tool, name)
    }
}

fn is_tablet(detection: &Detection) -> bool {
    detection.device.as_ref().is_some_and(|device| device.kind == "tablet")
}

fn is_desktop(detection: &Detection) -> bool {
    detection.device.as_ref().is_some_and(|device| device.kind == "desktop")
}

/// rust-device-detector's `KnownDevice::is_mobile`, minus the client hints, which a dump of user
/// agents alone cannot carry.
fn is_mobile(detection: &Detection) -> bool {
    if let Some(device) = &detection.device {
        match device.kind.as_str() {
            "feature phone" | "smartphone" | "tablet" | "phablet" | "camera"
            | "portable media player" => return true,
            "tv" | "smart display" | "console" => return false,
            _ => {}
        }
    }

    if let Some(client) = &detection.client
        && client.kind == "browser"
        && MOBILE_ONLY_BROWSERS.contains(&client.name.as_str())
    {
        return true;
    }

    if detection.os.is_none() {
        return false;
    }

    !is_desktop(detection)
}

/// What the log injector would have written for a user agent, detection or no detection.
pub fn read(detector: &device_detector::Detector, user_agent: &str) -> (Kind, String) {
    match detector.detect(user_agent) {
        Some(detection) => classify(&detection, user_agent),
        // What the log injector does with a detection error, and the closest thing to it.
        None => (Kind::Tool, user_agent.to_string()),
    }
}
