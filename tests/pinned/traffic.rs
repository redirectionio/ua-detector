//! User agents production answers differently from this library, heaviest first.
//!
//! Written by `cargo run --release --example traffic -- <dump.csv>` from the case file next to
//! this one. Regenerate it rather than editing it. The two columns are what production wrote
//! for the user agent, its type being the `DeviceType` of the log injector that
//! `examples/compare/injector.rs` reads a detection into.

#[path = "../../examples/compare/injector.rs"]
mod injector;

use injector::{Kind, read};

#[test]
fn the_heaviest_traffic_reads_as_production_reads_it() {
    // Typed, because the file is generated and starts out with nothing in it.
    let cases: &[(&str, &str, u16)] = &[
        // 5165429 requests
        (r"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_11_1) AppleWebKit/601.2.4 (KHTML, like Gecko) Version/9.0.1 Safari/601.2.4 facebookexternalhit/1.1 Facebot Twitterbot/1.0", r"Facebook Crawler", 3),
        // 3404765 requests
        (r"Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148", r"Mobile Safari", 2),
        // 1870150 requests
        (r"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 (compatible; AionBot/1.0)", r"Generic Bot", 3),
        // 55170 requests
        (r"Mozilla/5.0 (compatible; Meta-ExternalAgent/1.0; +https://developers.facebook.com/docs/sharing/webmasters/crawler)", r"Meta-ExternalAgent", 3),
        // 51091 requests
        (r"Mozilla/5.0 (Windows NT 6.2) AppleWebKit/537.1 (KHTML, like Gecko) Chrome/21.0.1180.75 Safari/537.1", r"Chrome", 1),
        // 42115 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F766N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 39993 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S931N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 39332 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S948N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 23532 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S938N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 22984 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S942N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 17201 requests
        (r"Mozilla/5.0 (Linux; Android 17; SM-F971N Build/CP2A.260605.016; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 16595 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S921N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 15005 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S906N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 14746 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S926N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 12883 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S928N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 12793 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F741N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 12762 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S936N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 12142 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S948N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 11420 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S911N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 11012 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F766N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 10823 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F731N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 10039 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S931N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 9375 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F766N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 8968 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F966N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 8368 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S947N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 7607 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S942N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 7128 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S916N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 6934 requests
        (r"Mozilla/5.0 (Linux; Android 17; SM-F971N Build/CP2A.260605.016; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 6921 requests
        (r"Mozilla/5.0 (Linux; Android 17; SM-F776N Build/CP2A.260605.016; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 6750 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S918N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 6611 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S921N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 5890 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F741N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 5655 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S908N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 5408 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S938N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 4636 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S937N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 4635 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S906N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 4557 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S901N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 4534 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S928N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 4206 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S926N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 4192 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F731N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 4042 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F741N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 3757 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F966N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 3605 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S938N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.29 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.22.10)", r"Naver", 2),
        // 3454 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F721N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 3399 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S936N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 2973 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S911N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 2923 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S931N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.29 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.22.50)", r"Naver", 2),
        // 2689 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S947N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 2512 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S937N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 2454 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S921N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 2447 requests
        (r"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0; 360Spider", r"360Spider", 4),
        // 2357 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F956N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 2102 requests
        (r"Mozilla/5.0 (Linux; Android 17; SM-F776N Build/CP2A.260605.016; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 2081 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S901N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 1988 requests
        (r"Mozilla/5.0 (Linux; Android 15; SM-G991N Build/AP3A.240905.015.A2; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1853 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S931N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1849 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S721N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.29 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.22.50)", r"Naver", 2),
        // 1771 requests
        (r"Mozilla/5.0 (Linux; Android 15; SM-F711N Build/AP3A.240905.015.A2; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1735 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F721N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 1735 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F766N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 1713 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S918N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 1653 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F966N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.29 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.22.10)", r"Naver", 2),
        // 1647 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S926N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1634 requests
        (r"mozilla/5.0 (compatible; meta-externalagent/1.0; +https://developers.facebook.com/docs/sharing/webmasters/crawler)", r"Meta-ExternalAgent", 3),
        // 1622 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S721N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1616 requests
        (r"User-Agent:Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0; 360Spider", r"360Spider", 4),
        // 1593 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S916N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 1589 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F741N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 1517 requests
        (r"Mozilla/5.0 (Linux; Android 12; SM-F711N Build/SP2A.220305.013; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1497 requests
        (r"Mozilla/5.0 (Linux; Android 17; SM-F971N Build/CP2A.260605.016; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 1438 requests
        (r"Mozilla/5.0 (Linux; Android 17; SM-F971N Build/CP2A.260605.016; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1397 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S731N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1339 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S911N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1317 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F731N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1300 requests
        (r"Mozilla/5.0 (Linux; Android 17; SM-F976N Build/CP2A.260605.016; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1285 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F966N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1256 requests
        (r"Mozilla/5.0 (Linux; Android 15; SM-G998N Build/AP3A.240905.015.A2; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 1185 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S928N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.17 Mobile Safari/537.36 NAVER(inapp; search; 2000; 12.15.50)", r"Naver", 2),
        // 1166 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F966N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.23 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.18.5)", r"Naver", 2),
        // 1135 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F731N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.29 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.22.50)", r"Naver", 2),
        // 992 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S938N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 978 requests
        (r"Mozilla/5.0 (Linux; Android 13; SM-N981N Build/TP1A.220624.014; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
        // 976 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S921N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.29 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.22.50)", r"Naver", 2),
        // 917 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F956N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 904 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S948N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.24 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.19.28)", r"Naver", 2),
        // 844 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F731N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.29 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.22.50)", r"Naver", 2),
        // 804 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-S908N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 777 requests
        (r"Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.1 (KHTML, like Gecko) Chrome/22.0.1207.1 Safari/537.1", r"Chrome", 1),
        // 773 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F956N Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.29 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.22.10)", r"Naver", 2),
        // 768 requests
        (r"Mozilla/5.0 (Linux; Android 16; SM-F731N Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 759 requests
        (r"Mozilla/5.0 (Linux; Android 15; SM-F711N Build/AP3A.240905.015.A2; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.50)", r"Naver", 2),
        // 743 requests
        (r"Mozilla/5.0 (Linux; Android 13; SM-G988N Build/TP1A.220624.014; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/128.0.0.0 Whale/1.0.0.0 Crosswalk/29.128.0.31 Mobile Safari/537.36 NAVER(inapp; search; 2100; 12.23.1)", r"Naver", 2),
    ];

    let detector = ua_detector::shared();
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(user_agent, name, kind)| {
            let want = Kind::from_column(*kind).expect("a type the log injector writes");
            let (found_kind, found_name) = read(detector, user_agent);

            (found_name != *name || found_kind != want).then(|| {
                format!(
                    "\n  {name} [{}] read as {found_name} [{}]\n    {user_agent}",
                    want.name(),
                    found_kind.name(),
                )
            })
        })
        .collect();

    assert!(wrong.is_empty(), "{} of {} still wrong{}", wrong.len(), cases.len(), wrong.join(""));
}
