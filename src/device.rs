//! What a user agent resolves to.
//!
//! [`Detection`] is what the detector answers. [`Partial`] is what a single entry in
//! `src/devices/*.yml` says, which is not the same thing: several entries may match one user
//! agent -- one naming the client, another the model it runs on -- and what they say is merged.
//!
//! The difference between the two is that a field of a `Partial` can be absent as well as empty,
//! and the two mean opposite things. An entry that does not mention the operating system version
//! leaves it to whatever else matched; an entry that writes `version: ""` is saying this thing
//! has no version, and overwrites. Collapsing the two is what the result type does last.

use crate::kind::{BotCategory, ClientKind, DeviceKind};
use crate::template::{fill, placeholders};

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Detection {
    #[serde(default)]
    pub os: Option<Os>,
    #[serde(default)]
    pub client: Option<Client>,
    #[serde(default)]
    pub device: Option<Device>,
    #[serde(default)]
    pub os_family: String,
    #[serde(default)]
    pub browser_family: String,
    #[serde(default)]
    pub bot: Option<Bot>,
}

impl Detection {
    /// Whether a crawler sent this, in which case the bot is the whole answer and the other
    /// fields are all empty. See the documentation of [`Detection`].
    pub fn is_bot(&self) -> bool {
        self.bot.is_some()
    }

    /// Whether the client is a browser no desktop has.
    ///
    /// Matomo's `usesMobileBrowser`, and the exception the two below are built on: a system that
    /// runs on nothing but a desktop is still not one when what reads it is a browser only a
    /// phone can run. `X11; Linux i686 ... Puffin/1.3.2665MS` is a phone having a server render
    /// the page for it.
    pub fn uses_mobile_only_browser(&self) -> bool {
        self.client.as_ref().is_some_and(|client| mobile_only(client.kind, &client.name))
    }

    /// Whether this was sent from a desktop, by matomo's rule: the operating system is one of
    /// the nine that run on nothing else, and the client is not a browser only a phone has.
    ///
    /// The device is not consulted. Matomo reads this off the system alone, and so does the
    /// fallback that fills [`Device::kind`] in when no entry named the machine.
    ///
    /// A bot answers `false`, where matomo answers on a detection it expects nobody to ask.
    pub fn is_desktop(&self) -> bool {
        !self.is_bot()
            && self.os.as_ref().is_some_and(|os| !os.name.is_empty())
            && !self.uses_mobile_only_browser()
            && DESKTOP_FAMILIES.contains(&self.os_family.as_str())
    }

    /// Whether this was sent from something carried, by matomo's rule: the device kind where it
    /// settles the question, then the browser, and failing both, whatever is not a desktop.
    ///
    /// That last step is why this is not `!is_desktop()`: a television settles it the other way,
    /// and a user agent naming no system at all is counted as mobile rather than as neither.
    ///
    /// A bot answers `false`. Matomo answers `true` there -- a crawler names no system, so
    /// nothing rules the desktop in -- which is a reading of its own rule rather than an answer
    /// about the traffic, and a caller asking this of a bot means the question, not the rule.
    pub fn is_mobile(&self) -> bool {
        if self.is_bot() {
            return false;
        }

        if let Some(settled) = self.device.as_ref().and_then(|device| device.kind?.is_mobile()) {
            return settled;
        }

        self.uses_mobile_only_browser() || !self.is_desktop()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Os {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub platform: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    #[serde(rename = "type", default, deserialize_with = "crate::kind::kind")]
    pub kind: Option<ClientKind>,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub engine_version: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    #[serde(rename = "type", default, deserialize_with = "crate::kind::kind")]
    pub kind: Option<DeviceKind>,
    #[serde(default)]
    pub brand: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bot {
    pub name: String,
    #[serde(default, deserialize_with = "crate::kind::kind")]
    pub category: Option<BotCategory>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub producer: Producer,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
}

/// What one entry says. See the module documentation for why this is not a [`Detection`].
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Partial {
    #[serde(default)]
    pub os: Option<PartialOs>,
    #[serde(default)]
    pub client: Option<PartialClient>,
    #[serde(default)]
    pub device: Option<PartialDevice>,
    #[serde(default)]
    pub os_family: Option<String>,
    #[serde(default)]
    pub browser_family: Option<String>,
    #[serde(default)]
    pub bot: Option<PartialBot>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialOs {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialClient {
    #[serde(rename = "type", default, deserialize_with = "crate::kind::said")]
    pub kind: Option<Option<ClientKind>>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub engine_version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialDevice {
    #[serde(rename = "type", default, deserialize_with = "crate::kind::said")]
    pub kind: Option<Option<DeviceKind>>,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialBot {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "crate::kind::said")]
    pub category: Option<Option<BotCategory>>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub producer: Option<PartialProducer>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialProducer {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Applies `f` to every text an entry holds, which is how an entry becomes a result: every
/// `{placeholder}` is replaced by what the regex captured. A field the entry left out stays out.
///
/// The kinds are carried across untouched: they are drawn from a closed vocabulary rather than
/// written, so no entry has a placeholder to fill in one.
macro_rules! walk {
    ($partial:expr, $f:expr) => {{
        let partial: &Partial = $partial;
        #[allow(unused_mut)]
        let mut f = $f;
        let mut g = |text: &Option<String>| text.as_deref().map(&mut f);

        Partial {
            os: partial.os.as_ref().map(|os| PartialOs {
                name: g(&os.name),
                version: g(&os.version),
                platform: g(&os.platform),
            }),
            client: partial.client.as_ref().map(|client| PartialClient {
                kind: client.kind,
                name: g(&client.name),
                version: g(&client.version),
                engine: g(&client.engine),
                engine_version: g(&client.engine_version),
            }),
            device: partial.device.as_ref().map(|device| PartialDevice {
                kind: device.kind,
                brand: g(&device.brand),
                model: g(&device.model),
            }),
            os_family: g(&partial.os_family),
            browser_family: g(&partial.browser_family),
            bot: partial.bot.as_ref().map(|bot| PartialBot {
                name: g(&bot.name),
                category: bot.category.clone(),
                url: g(&bot.url),
                producer: bot.producer.as_ref().map(|producer| PartialProducer {
                    name: g(&producer.name),
                    url: g(&producer.url),
                }),
            }),
        }
    }};
}

/// Takes `from` when it has something to say, which is any value at all, empty included.
fn take<T>(into: &mut Option<T>, from: Option<T>) {
    if from.is_some() {
        *into = from;
    }
}

impl Partial {
    /// Fills every `{placeholder}` from what `lookup` captured for it.
    pub fn resolve(&self, lookup: &dyn Fn(&str) -> Option<String>) -> Partial {
        walk!(self, |text: &str| fill(text, lookup))
    }

    /// Every `{placeholder}` the entry refers to, with the filters it asks for, so both can be
    /// checked against the regex before the detector is ever used.
    pub fn placeholders(&self) -> Vec<(String, Vec<String>)> {
        let mut names = Vec::new();

        walk!(self, |text: &str| {
            names.extend(placeholders(text).into_iter().map(|(name, filters)| {
                (name.to_string(), filters.into_iter().map(str::to_string).collect())
            }));
            String::new()
        });

        names
    }

    /// Lays `later` over this one, field by field, keeping whatever it has to say.
    ///
    /// Field by field rather than block by block: an entry that names a device by its model says
    /// nothing about the device kind, and an entry that reads the kind off the client says
    /// nothing about the brand. Neither should erase the other.
    ///
    /// Last wins, which is why matches are merged in ascending priority -- the entry that knows
    /// most goes on top.
    pub fn merge(&mut self, later: Partial) {
        match (&mut self.os, later.os) {
            (slot @ None, found) => *slot = found,
            (Some(os), Some(found)) => {
                take(&mut os.name, found.name);
                take(&mut os.version, found.version);
                take(&mut os.platform, found.platform);
            }
            _ => {}
        }

        match (&mut self.client, later.client) {
            (slot @ None, found) => *slot = found,
            (Some(client), Some(found)) => {
                take(&mut client.kind, found.kind);
                take(&mut client.name, found.name);
                take(&mut client.version, found.version);
                take(&mut client.engine, found.engine);
                take(&mut client.engine_version, found.engine_version);
            }
            _ => {}
        }

        match (&mut self.device, later.device) {
            (slot @ None, found) => *slot = found,
            (Some(device), Some(found)) => {
                take(&mut device.kind, found.kind);
                take(&mut device.brand, found.brand);
                take(&mut device.model, found.model);
            }
            _ => {}
        }

        take(&mut self.os_family, later.os_family);
        take(&mut self.browser_family, later.browser_family);

        if later.bot.is_some() {
            self.bot = later.bot;
        }
    }
}

/// The browsers matomo marks as mobile only. A system that runs on nothing but a desktop is
/// still not one when what reads it is a browser no desktop has, which is the exception matomo
/// makes to the rule below -- `X11; Linux i686 ... Puffin/1.3.2665MS` is a phone rendering a
/// page on a server.
///
/// Matomo holds them by short code; these are the names this library answers with.
pub const MOBILE_ONLY_BROWSERS: &[&str] = &[
    "18+ Privacy Browser",
    "1DM Browser",
    "1DM+ Browser",
    "360 Phone Browser",
    "ALVA",
    "APN Browser",
    "APUS Browser",
    "AdBlock Browser",
    "Adult Browser",
    "Airfind Secure Browser",
    "Aloha Browser Lite",
    "Amerigo",
    "AppBrowzer",
    "Arvin",
    "Ask.com",
    "Azka Browser",
    "B-Line",
    "BF Browser",
    "Bang",
    "Bangla Browser",
    "Belva Browser",
    "Beyond Private Browser",
    "Bitchute Browser",
    "Black Lion Browser",
    "Bloket",
    "BroKeep Browser",
    "Browlser",
    "BrowsBit",
    "CG Browser",
    "COS Browser",
    "Cave Browser",
    "Coast",
    "CoolBrowser",
    "Cornowser",
    "DUC Browser",
    "Dark Browser",
    "Dark Web",
    "Dark Web Private",
    "Delta Browser",
    "Desi Browser",
    "Dezor",
    "Diigo Browser",
    "DuckDuckGo Privacy Browser",
    "EUI Browser",
    "Every Browser",
    "Faux Browser",
    "Fiery Browser",
    "Fire Browser",
    "Firefox Focus",
    "Firefox Mobile",
    "Firefox Rocket",
    "Float Browser",
    "Freedom Browser",
    "Frost",
    "Frost+",
    "GO Browser",
    "Ghostery Privacy Browser",
    "GinxDroid Browser",
    "GoBrowser",
    "Godzilla Browser",
    "Good Browser",
    "HTC Browser",
    "Habit Browser",
    "Hawk Quick Browser",
    "Hawk Turbo Browser",
    "Hi Browser",
    "Holla Web Browser",
    "HotBrowser",
    "Huawei Browser Mobile",
    "IVVI Browser",
    "InBrowser",
    "Incognito Browser",
    "Insta Browser",
    "Involt Go",
    "Isivioo",
    "Japan Browser",
    "KUN",
    "Keyboard Browser",
    "Kids Safe Browser",
    "Kitt",
    "Kode Browser",
    "Legan Browser",
    "Lexi Browser",
    "Lilo",
    "MarsLab Web Browser",
    "MaxBrowser",
    "Meizu Browser",
    "Minimo",
    "MixerBox AI",
    "Mmx Browser",
    "Mobile Safari",
    "Monument Browser",
    "Motorola Internet Browser",
    "NOOK Browser",
    "Naked Browser Pro",
    "NextWord Browser",
    "Nova Video Downloader Pro",
    "Nox Browser",
    "Nuviu",
    "Ocean Browser",
    "Oculus Browser",
    "Odd Browser",
    "OnBrowser Lite",
    "Onion Browser",
    "Open Browser",
    "Opera Mini",
    "Opera Mobile",
    "OrNET Browser",
    "Orbitum",
    "Owl Browser",
    "PICO Browser",
    "Pawxy",
    "Perfect Browser",
    "Phantom.me",
    "Photon",
    "PocketBook Browser",
    "PrivacyWall",
    "Private Internet Browser",
    "PronHub Browser",
    "Proxy Browser",
    "Proxyium",
    "Proxynet",
    "Puffin Cloud Browser",
    "Puffin Incognito Browser",
    "Puffin Web Browser",
    "Puffin Web Browser Pro",
    "Qmamu",
    "Quark",
    "Realme Browser",
    "Reqwireless WebViewer",
    "SOTI Surf",
    "SP Browser",
    "START Internet Browser",
    "SX Browser",
    "Sailfish Browser",
    "Samsung Browser",
    "Savannah Browser",
    "SavySoda",
    "SecureX",
    "SkyLeap",
    "Skyfire",
    "Smart Browser",
    "Smooz",
    "Soundy Browser",
    "Stampy Browser",
    "Stargon",
    "Stealth Browser",
    "Streamy",
    "Sunflower Browser",
    "Super Fast Browser",
    "Surf Browser",
    "Sweet Browser",
    "TUC Mini Browser",
    "TalkTo",
    "Thor",
    "TrueLocation Browser",
    "U Browser",
    "UC Browser HD",
    "UC Browser Mini",
    "UC Browser Turbo",
    "UPhone Browser",
    "Ui Browser Mini",
    "VD Browser",
    "Vast Browser",
    "Venus Browser",
    "Vertex Surf",
    "Vuhuv",
    "Wear Internet Browser",
    "Web Explorer",
    "World Browser",
    "Wukong Browser",
    "X Browser Lite",
    "X-VPN",
    "XNX Browser",
    "Xooloo Internet",
    "XtremeCast",
    "YAGI",
    "Yaani Browser",
    "YouBrowser",
    "YouCare",
    "ZTE Browser",
    "Zordo Browser",
    "dbrowser",
    "eZ Browser",
    "fGet",
    "iBrowser",
    "iBrowser Mini",
    "lightning Browser Plus",
    "mCent",
    "vBrowser",
    "vivo Browser",
    "xBrowser Pro Super Fast",
];

/// Whether a client is a browser only a phone runs, which both [`Detection::is_mobile`] and the
/// desktop fallback below have to ask, one of a finished detection and one part way through it.
fn mobile_only(kind: Option<ClientKind>, name: &str) -> bool {
    kind == Some(ClientKind::Browser) && MOBILE_ONLY_BROWSERS.contains(&name)
}

/// The operating systems matomo says run on nothing but a desktop, which is what it falls back
/// on when nothing named the machine.
const DESKTOP_FAMILIES: [&str; 9] =
    ["AmigaOS", "IBM", "GNU/Linux", "Mac", "Unix", "Windows", "BeOS", "Chrome OS", "OpenVMS"];

/// Collapses what the entries said into the answer, absent and empty becoming the same thing
/// again: a caller has no use for the difference, only the merge did.
impl From<Partial> for Detection {
    fn from(partial: Partial) -> Detection {
        fn text(value: Option<String>) -> String {
            value.unwrap_or_default()
        }

        // A bot is the whole answer. Every bot in the corpus reports no system, no client and no
        // device, and an entry that names one says nothing about the rest -- so without this the
        // platform axis would leave an Android version on a crawler that merely mentions Android.
        if let Some(bot) = partial.bot {
            return Detection {
                bot: Some(Bot {
                    name: text(bot.name),
                    category: bot.category.flatten(),
                    url: text(bot.url),
                    producer: bot.producer.map_or_else(Producer::default, |producer| Producer {
                        name: text(producer.name),
                        url: text(producer.url),
                    }),
                }),
                ..Default::default()
            };
        }

        let os_family = text(partial.os_family);
        let mut device = partial.device.map(|device| Device {
            kind: device.kind.flatten(),
            brand: text(device.brand),
            model: text(device.model),
        });

        // Matomo's last word on the device: a system that runs on nothing else says desktop,
        // whatever the rest of the string left out. Entries say it themselves where they read
        // the platform token, and cannot where they read something else -- `Microsoft-WebDAV-
        // MiniRedir/10.0.19045` names Windows and no machine at all.
        let handheld = partial.client.as_ref().is_some_and(|client| {
            mobile_only(client.kind.flatten(), client.name.as_deref().unwrap_or_default())
        });

        if DESKTOP_FAMILIES.contains(&os_family.as_str())
            && !handheld
            && device.as_ref().is_none_or(|device| device.kind.is_none())
        {
            device = Some(Device {
                kind: Some(DeviceKind::Desktop),
                ..device.unwrap_or_default()
            });
        }

        Detection {
            os: partial.os.map(|os| Os {
                name: text(os.name),
                version: text(os.version),
                platform: text(os.platform),
            }),
            client: partial.client.map(|client| Client {
                kind: client.kind.flatten(),
                name: text(client.name),
                version: text(client.version),
                engine: text(client.engine),
                engine_version: text(client.engine_version),
            }),
            device,
            os_family,
            browser_family: text(partial.browser_family),
            bot: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(yaml: &str) -> Partial {
        serde_yaml::from_str(yaml).expect("a valid entry")
    }

    fn detection(yaml: &str) -> Detection {
        serde_yaml::from_str(yaml).expect("a valid detection")
    }

    #[test]
    fn resolves_the_placeholders_of_an_entry() {
        let entry = entry(
            r#"
            os: {name: Android, version: "{os_version}", platform: ""}
            device: {type: camera, brand: Samsung, model: "Galaxy Camera {number}"}
            "#,
        );
        let regex = regex::Regex::new(r"Android (?<os_version>[\d.]+); EK-GC(?<number>\d{3})").unwrap();
        let haystack = "Android 4.1.2; EK-GC110 Build";
        let captures = regex.captures(haystack).unwrap();
        let lookup = |name: &str| captures.name(name).map(|group| group.as_str().to_string());

        let resolved = Detection::from(entry.resolve(&lookup));

        assert_eq!(resolved.os.unwrap().version, "4.1.2");
        assert_eq!(resolved.device.unwrap().model, "Galaxy Camera 110");
    }

    #[test]
    fn lists_the_placeholders_of_an_entry() {
        let entry = entry(
            r#"
            os: {name: Android, version: "{os_version}"}
            client: {type: browser, name: Chrome, version: "{client_version}", engine_version: "{client_version}"}
            "#,
        );

        let mut names: Vec<String> =
            entry.placeholders().into_iter().map(|(name, _)| name).collect();
        names.sort();
        names.dedup();

        assert_eq!(names, vec!["client_version", "os_version"]);
    }

    /// The three steps of matomo's `isMobile`, one test each: the kind settles it, the browser
    /// settles it, and failing both, whatever is not a desktop.
    #[test]
    fn a_device_kind_settles_whether_it_is_carried() {
        let phone = detection(r#"{device: {type: smartphone}, os_family: Android}"#);
        let television = detection(r#"{device: {type: tv}, os_family: Android}"#);

        assert!(phone.is_mobile() && !phone.is_desktop());
        assert!(!television.is_mobile() && !television.is_desktop());
    }

    /// A system that runs on nothing but a desktop is still not one when what reads it is a
    /// browser no desktop has -- matomo's own example, a phone having a server render the page.
    #[test]
    fn a_browser_only_a_phone_has_outranks_the_system() {
        let puffin = detection(
            r#"{client: {type: browser, name: Puffin Web Browser}, os: {name: GNU/Linux}, os_family: GNU/Linux}"#,
        );

        assert!(puffin.uses_mobile_only_browser());
        assert!(puffin.is_mobile() && !puffin.is_desktop());
    }

    #[test]
    fn a_desktop_system_and_an_ordinary_browser_is_a_desktop() {
        let chrome = detection(
            r#"{client: {type: browser, name: Chrome}, os: {name: Windows}, device: {type: desktop}, os_family: Windows}"#,
        );

        assert!(chrome.is_desktop() && !chrome.is_mobile());
    }

    /// Matomo's last step, and the reason this is not `!is_desktop()` written the other way
    /// round: a user agent that names no system is counted as carried rather than as neither.
    #[test]
    fn what_names_no_system_counts_as_carried() {
        let bare = detection(r#"{client: {type: library, name: curl}}"#);

        assert!(bare.is_mobile() && !bare.is_desktop());
    }

    /// Where this library parts with matomo, which answers that a crawler is mobile because
    /// nothing about it rules the desktop in.
    #[test]
    fn a_bot_is_neither_carried_nor_a_desktop() {
        let bot = detection(r#"{bot: {name: Googlebot, category: Search bot}}"#);

        assert!(bot.is_bot());
        assert!(!bot.is_mobile() && !bot.is_desktop());
    }

    /// The distinction the merge is built on: saying nothing leaves the earlier value alone,
    /// saying nothing *in particular* replaces it.
    #[test]
    fn an_entry_can_say_a_field_is_empty() {
        let mut merged = entry(r#"os: {name: "Android", version: "11"}"#);
        merged.merge(entry(r#"os: {name: "Wear OS", version: ""}"#));

        let os = Detection::from(merged).os.unwrap();

        assert_eq!(os.name, "Wear OS");
        assert_eq!(os.version, "");
    }

    #[test]
    fn a_field_an_entry_leaves_out_keeps_what_was_found_before() {
        let mut merged = entry(r#"device: {type: smartphone, brand: Xiaomi, model: "Mi 11 Lite 5G"}"#);
        merged.merge(entry(r#"device: {type: tablet}"#));

        let device = Detection::from(merged).device.unwrap();

        assert_eq!((device.kind, device.brand.as_str()), (Some(DeviceKind::Tablet), "Xiaomi"));
    }
}
