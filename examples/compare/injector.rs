//! What redirection.io's log injector makes of a detection.
//!
//! It stores two columns per request, `user_agent_simplified` and `user_agent_type`, and both
//! come from a detection. Comparing this library against production
//! means replaying that derivation, so it lives here rather than in either of the two examples
//! that need it, and in `tests/traffic.rs`, which reaches for it across the tree.

#![allow(dead_code)]

use ua_detector::{ClientKind, Detection, Named};

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
    let name = match detection.named() {
        Named::Bot(bot) => bot.name.clone(),
        Named::Agent { client: Some(client), .. } => client.name.clone(),
        Named::Agent { client: None, .. } => user_agent.to_string(),
    };

    (kind(detection), name)
}

/// The log injector's `device_type`, from `server/log-injector/src/enrichment.rs`.
fn kind(detection: &Detection) -> Kind {
    match detection.named() {
        Named::Bot(bot) => match &bot.category {
            Some(category) if category.is_search_bot() => Kind::Bot,
            _ => Kind::Tool,
        },
        Named::Agent { os, client, device, .. } => {
            let driven_by_a_person = client
                .and_then(|client| client.kind)
                .is_some_and(|kind| matches!(kind, ClientKind::Browser | ClientKind::MobileApp | ClientKind::Pim));
            let device_kind = device.and_then(|device| device.kind);
            let platform_named =
                os.is_some_and(|os| !os.name.is_empty()) || device_kind.is_some_and(|kind| kind.is_mobile().is_some());

            match device_kind {
                _ if !driven_by_a_person || !platform_named => Kind::Tool,
                // A television, a console or a smart display is neither carried nor a desktop.
                Some(kind) if kind.is_mobile() == Some(false) => Kind::Tool,
                _ if detection.is_mobile() => Kind::Mobile,
                _ if detection.is_desktop() => Kind::Desktop,
                _ => Kind::Tool,
            }
        }
    }
}

/// What the log injector would have written for a user agent, detection or no detection.
pub fn read(detector: &ua_detector::Detector, user_agent: &str) -> (Kind, String) {
    match detector.detect(user_agent) {
        Some(detection) => classify(&detection, user_agent),
        // What the log injector does with a detection error, and the closest thing to it.
        None => (Kind::Tool, user_agent.to_string()),
    }
}
