# ua-detector

This library parses a user agent and gives you information about it, from a database embedded in
the binary.

```rust
let found = ua_detector::shared()
    .detect("Mozilla/5.0 (Linux; Android 10; SM-G9650) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/80.0.3987.99 Safari/537.36")
    .unwrap();

assert!(found.is_mobile());
assert_eq!(found.device.unwrap().model, "Galaxy S9+");
assert_eq!(found.os.unwrap().version, "10");
```

This library is based on the [matomo device detector library](https://github.com/matomo-org/device-detector):
its test fixtures, 38 030 user agents and what each one means, are the specification this database
is written against. Matomo's own regexes are not vendored and none of them is copied here.

The main change is how the parsing is done, with a specific structure to ensure better speed with
minimal memory footprint, so it can be embedded with fewer constraints.

## Detection

Here is a sample of a result of such detection:

```rust
Detection {
    os: Some(Os { name: "Android", version: "10", platform: "" }),
    client: Some(Client {
        kind: Some(Browser), 
        name: "Chrome", 
        version: "80.0.3987.99",
        engine: "Blink", 
        engine_version: "80.0.3987.99",
    }),
    device: Some(Device { 
        kind: Some(Smartphone), 
        brand: "Samsung", 
        model: "Galaxy S9+" 
    }),
    os_family: "Android",
    browser_family: "Chrome",
    bot: None,
}
```

- `device.kind` is a `DeviceKind` -- smartphone, tablet, desktop, tv, console, wearable and eight
  more -- and `None` where a device was named without its kind being said.
- `client.kind` is a `ClientKind`: browser, mobile app, mediaplayer, feed reader, pim or library.
- `bot` is set for a crawler, and then **it is the whole answer**: the other five fields are
  empty. `Detection::named()` hands that back as an enum, so a `match` cannot read a client where
  there is none. `bot.category` is a `BotCategory`, with `is_search_bot()` on it.
- The text fields are empty rather than absent when nothing was found.

## Client hints

You can pass extra headers to the detection, which help it answer better. Those headers may or may
not be present, depending on the browser.

```rust
detector.detect_with_headers(user_agent, &[
    ("Sec-CH-UA-Platform", "\"Android\""),
    ("Sec-CH-UA-Model", "\"SM-G9650\""),
]);
```

## Speed

The database consists of almost 40 000 entries, one regex each. Compiling all of them takes a lot
of time and consumes a lot of memory.

To avoid that, it is organised around a giant regex tree, in order to quickly skip a whole set of
results when they share the same prefix.

By default there is no regex compilation: a regex is built when a lookup reaches it and thrown
away again, which makes it slow -- around 7 ms for a common user agent, and more than twice that
averaged over the whole corpus, most of which is devices nobody sends.

To improve this time we provide several options, depending on your use case:

 * `cache_answers` method: a basic system that keeps a simple key-value cache of the answers
   (nothing on the first request for a user agent, everything on the next one)
 * `warm` method: you give it a set of user agents and a budget, and it will compile the regexes
   those user agents reach and nothing else, so it is fast from the first request. The budget
   caps what it costs.
 * `cache` method: give it a budget and it will compile part of the tree blind, taking the top
   levels first and the branches holding the most entries before the rest. A guess, where `warm`
   has traffic to go on.

In most cases we recommend using both `cache_answers` and `warm`, like the following:

```rust
use ua_detector::{Budget, Detector};

let mut detector = Detector::new();

detector.warm(ua_detector::common_user_agents(), Budget::bytes(128 << 20));
detector.cache_answers(20_000);

let found = detector.detect_cached(user_agent);
```

## Licence

LGPL-3.0-or-later, the same as matomo's, whose fixtures this library is written against. See
[LICENSE](LICENSE), and [LICENSE.GPL](LICENSE.GPL) for the terms it incorporates.

The fixtures under `tests/matomo-device-detector/fixtures/` are copied verbatim from
matomo-org/device-detector, Copyright (C) Matomo Analytics, and `src/devices/*.yml` was first
derived from them.
