//! The vocabularies a detection answers with.
//!
//! Matomo's parsers answer with a fixed set of device and client types, and the fixtures never
//! name another, so these are enums rather than text: against a `String` a caller had nothing
//! but the corpus to tell a misspelling from a device nobody sends, and a `match` arm that never
//! fires says nothing at all.
//!
//! Bot categories are the exception. Matomo adds one from time to time -- the AI crawlers all
//! arrived at once -- and a synchronisation that brought a new name must not stop the database
//! from loading, so an unknown one is kept as text in [`BotCategory::Other`].

use std::str::FromStr;

use serde::Deserialize;

/// A name that is not one of the kinds its vocabulary holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKind {
    /// What was asked of it.
    pub vocabulary: &'static str,
    /// The name it does not hold.
    pub name: String,
}

impl std::fmt::Display for UnknownKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?} is not a {}", self.name, self.vocabulary)
    }
}

impl std::error::Error for UnknownKind {}

/// A closed vocabulary: the enum, the name matomo gives each of its kinds, and the four ways of
/// going between the two. Declared in one place so that the text, the parse and the display
/// cannot drift apart.
macro_rules! vocabulary {
    (
        $(#[$meta:meta])*
        $name:ident is $vocabulary:literal { $($variant:ident => $text:literal,)* }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[non_exhaustive]
        pub enum $name {
            $($variant,)*
        }

        impl $name {
            /// Every kind of this vocabulary, in the order they are declared.
            pub const ALL: &'static [$name] = &[$($name::$variant,)*];

            /// The name matomo gives it, which is what the fixtures and the database write.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text,)*
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = UnknownKind;

            fn from_str(name: &str) -> Result<$name, UnknownKind> {
                match name {
                    $($text => Ok($name::$variant),)*
                    _ => Err(UnknownKind { vocabulary: $vocabulary, name: name.to_string() }),
                }
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<$name, D::Error> {
                String::deserialize(deserializer)?.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

vocabulary! {
    /// What a user agent is running on.
    ///
    /// An entry may name a device without naming its kind -- `models.yml` reads a model token and
    /// says nothing else -- which is why [`crate::Device::kind`] is an option. Matomo answers with
    /// these fourteen and no others.
    DeviceKind is "device kind" {
        Desktop => "desktop",
        Smartphone => "smartphone",
        Tablet => "tablet",
        Phablet => "phablet",
        FeaturePhone => "feature phone",
        Console => "console",
        Tv => "tv",
        CarBrowser => "car browser",
        SmartDisplay => "smart display",
        SmartSpeaker => "smart speaker",
        Camera => "camera",
        PortableMediaPlayer => "portable media player",
        Wearable => "wearable",
        Peripheral => "peripheral",
    }
}

impl DeviceKind {
    /// Whether a device of this kind is carried, where the kind settles it on its own.
    ///
    /// `None` where it does not, which is matomo's answer as much as the other two: a desktop is
    /// told from the operating system rather than from the device, and a peripheral, a wearable,
    /// a car browser, a smart speaker or a camera says nothing either way. [`crate::Detection::is_mobile`]
    /// is where the rest of the question is answered.
    pub const fn is_mobile(self) -> Option<bool> {
        match self {
            DeviceKind::FeaturePhone
            | DeviceKind::Smartphone
            | DeviceKind::Tablet
            | DeviceKind::Phablet
            | DeviceKind::Camera
            | DeviceKind::PortableMediaPlayer => Some(true),
            DeviceKind::Tv | DeviceKind::SmartDisplay | DeviceKind::Console => Some(false),
            _ => None,
        }
    }
}

vocabulary! {
    /// What kind of software a client is, which is matomo's parser said as a value: a feed
    /// reader, a mobile application, a media player, a personal information manager, a library,
    /// or a browser.
    ClientKind is "client kind" {
        Browser => "browser",
        MobileApp => "mobile app",
        MediaPlayer => "mediaplayer",
        FeedReader => "feed reader",
        Pim => "pim",
        Library => "library",
    }
}

/// What a bot is there for, as matomo files it.
///
/// Unlike the other two this vocabulary is open: matomo adds a category from time to time, and a
/// name it has not seen is kept rather than refused, so that `castor matomo:sync` cannot leave
/// the database unloadable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum BotCategory {
    Crawler,
    SearchBot,
    SecuritySearchBot,
    SearchTools,
    SiteMonitor,
    NetworkMonitor,
    SecurityChecker,
    ServiceAgent,
    ServiceBot,
    SocialMediaAgent,
    FeedFetcher,
    FeedReader,
    FeedParser,
    ReadItLaterService,
    Validator,
    Benchmark,
    AiAgent,
    AiAssistant,
    AiDataScraper,
    AiSearchCrawler,
    /// A category matomo has added since this database was last synchronised.
    Other(String),
}

impl BotCategory {
    /// The categories this library knows by name, in the order they are declared.
    pub const NAMED: &'static [BotCategory] = &[
        BotCategory::Crawler,
        BotCategory::SearchBot,
        BotCategory::SecuritySearchBot,
        BotCategory::SearchTools,
        BotCategory::SiteMonitor,
        BotCategory::NetworkMonitor,
        BotCategory::SecurityChecker,
        BotCategory::ServiceAgent,
        BotCategory::ServiceBot,
        BotCategory::SocialMediaAgent,
        BotCategory::FeedFetcher,
        BotCategory::FeedReader,
        BotCategory::FeedParser,
        BotCategory::ReadItLaterService,
        BotCategory::Validator,
        BotCategory::Benchmark,
        BotCategory::AiAgent,
        BotCategory::AiAssistant,
        BotCategory::AiDataScraper,
        BotCategory::AiSearchCrawler,
    ];

    pub fn as_str(&self) -> &str {
        match self {
            BotCategory::Crawler => "Crawler",
            BotCategory::SearchBot => "Search bot",
            BotCategory::SecuritySearchBot => "Security search bot",
            BotCategory::SearchTools => "Search tools",
            BotCategory::SiteMonitor => "Site Monitor",
            BotCategory::NetworkMonitor => "Network Monitor",
            BotCategory::SecurityChecker => "Security Checker",
            BotCategory::ServiceAgent => "Service Agent",
            BotCategory::ServiceBot => "Service bot",
            BotCategory::SocialMediaAgent => "Social Media Agent",
            BotCategory::FeedFetcher => "Feed Fetcher",
            BotCategory::FeedReader => "Feed Reader",
            BotCategory::FeedParser => "Feed Parser",
            BotCategory::ReadItLaterService => "Read-it-later Service",
            BotCategory::Validator => "Validator",
            BotCategory::Benchmark => "Benchmark",
            BotCategory::AiAgent => "AI Agent",
            BotCategory::AiAssistant => "AI Assistant",
            BotCategory::AiDataScraper => "AI Data Scraper",
            BotCategory::AiSearchCrawler => "AI Search Crawler",
            BotCategory::Other(name) => name,
        }
    }

    /// Whether this is a search engine's crawler, which is the one distinction a log usually
    /// keeps: an AI assistant fetching a page on someone's behalf is a bot, but it is not the
    /// one that puts a site in an index.
    pub fn is_search_bot(&self) -> bool {
        matches!(self, BotCategory::SearchBot | BotCategory::SecuritySearchBot)
    }
}

impl std::fmt::Display for BotCategory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BotCategory {
    type Err = std::convert::Infallible;

    fn from_str(name: &str) -> Result<BotCategory, std::convert::Infallible> {
        let found = BotCategory::NAMED.iter().find(|category| category.as_str() == name);

        Ok(found.cloned().unwrap_or_else(|| BotCategory::Other(name.to_string())))
    }
}

impl<'de> Deserialize<'de> for BotCategory {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<BotCategory, D::Error> {
        let name = String::deserialize(deserializer)?;

        Ok(name.parse().expect("a bot category cannot fail to parse"))
    }
}

/// A kind, or nothing: an entry writes `type: ''` where it means the field has none, and the
/// fixtures write the same where matomo answered nothing.
pub(crate) fn kind<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let name = String::deserialize(deserializer)?;

    if name.is_empty() {
        return Ok(None);
    }

    name.parse().map(Some).map_err(serde::de::Error::custom)
}

/// The same, one level down, for what an entry *said*: absent and empty are opposite there, so
/// the outer option is whether the entry mentioned the field at all. See the module
/// documentation of [`crate::device`].
pub(crate) fn said<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: FromStr,
    T::Err: std::fmt::Display,
{
    kind(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_kind_by_the_name_matomo_gives_it() {
        assert_eq!("portable media player".parse(), Ok(DeviceKind::PortableMediaPlayer));
        assert_eq!("mobile app".parse(), Ok(ClientKind::MobileApp));
        assert_eq!(DeviceKind::CarBrowser.to_string(), "car browser");
    }

    #[test]
    fn refuses_a_device_kind_matomo_does_not_have() {
        let error = "smart phone".parse::<DeviceKind>().unwrap_err();

        assert_eq!(error.to_string(), "\"smart phone\" is not a device kind");
    }

    /// The open end of the three: a synchronisation that brings a new category has to load.
    #[test]
    fn keeps_a_bot_category_it_does_not_know() {
        let found: BotCategory = "Sandwich Fetcher".parse().unwrap();

        assert_eq!(found, BotCategory::Other(String::from("Sandwich Fetcher")));
        assert_eq!(found.as_str(), "Sandwich Fetcher");
        assert!(!found.is_search_bot());
    }

    /// `category.contains("search bot")`, which is what a caller wrote against the string, took
    /// this one with it. Whether that is wanted is now something the caller says.
    #[test]
    fn a_security_search_bot_is_a_search_bot() {
        assert!(BotCategory::SecuritySearchBot.is_search_bot());
        assert!(!BotCategory::AiSearchCrawler.is_search_bot());
    }
}
