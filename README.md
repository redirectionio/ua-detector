# device-detector

Reads a user agent, and the client hints that go with it, into the device, operating system and
client it names.

```rust
use device_detector::{Budget, Detector};

let mut detector = Detector::new();
detector.warm(device_detector::common_user_agents(), Budget::bytes(128 << 20));

let found = detector
    .detect("Mozilla/5.0 (Linux; Android 10; SM-G9650) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/80.0.3987.99 Safari/537.36")
    .unwrap();

assert!(found.is_mobile());
assert_eq!(found.device.unwrap().model, "Galaxy S9+");
assert_eq!(found.os.unwrap().version, "10");
```

One detector for the whole process. `device_detector::shared()` is that detector built for you,
warmed the same way, if you have nowhere better to keep one -- but see **What a lookup costs**
below before you leave the budget to it.

```
Detection {
    os: Some(Os { name: "Android", version: "10", platform: "" }),
    client: Some(Client {
        kind: Some(Browser), name: "Chrome", version: "80.0.3987.99",
        engine: "Blink", engine_version: "80.0.3987.99",
    }),
    device: Some(Device { kind: Some(Smartphone), brand: "Samsung", model: "Galaxy S9+" }),
    os_family: "Android",
    browser_family: "Chrome",
    bot: None,
}
```

Client hints travel in headers rather than in the user agent, so they go in through
`detect_with_headers`:

```rust
detector.detect_with_headers(user_agent, &[
    ("Sec-CH-UA-Platform", "\"Android\""),
    ("Sec-CH-UA-Model", "\"SM-G9650\""),
]);
```

## What comes out

`Device::kind` and `Client::kind` are the vocabularies matomo answers with, not text: fourteen
device kinds and six client kinds, `None` where the entries named a device without saying what
kind of one it is. `Bot::category` is the same over an open vocabulary, an unknown name reading
back as `BotCategory::Other`.

A bot is the whole answer -- where `bot` is set the other five fields are empty, always -- and
`Detection::named` is that invariant said as an enum, so the arm that reads a client cannot be
written where there is none:

```rust
match found.named() {
    Named::Bot(bot) => (bot.name.clone(), bot.category.as_ref().is_some_and(BotCategory::is_search_bot)),
    Named::Agent { client, .. } => (client.map(|c| c.name.clone()).unwrap_or_default(), false),
}
```

Matomo's rules for what a detection amounts to live here rather than in every application that
asks: `is_mobile`, `is_desktop`, `is_bot` and `uses_mobile_only_browser`, with `MOBILE_ONLY_BROWSERS`
public for whoever needs the list itself.

## What is in it

The database is `src/devices/*.yml`, embedded in the binary. One entry is one regex and the whole
answer it stands for:

```yaml
- regex: Mozilla/5\.0 \(Linux; Android (?<os_version>[\d.]+); SM-G9650\) AppleWebKit/537\.36 \(KHTML, like Gecko\) Chrome/(?<client_version>[\d.]+) Safari/537\.36
  os:
    name: Android
    version: '{os_version}'
    platform: ''
  client:
    type: browser
    name: Chrome
    version: '{client_version}'
    engine: Blink
    engine_version: '{client_version}'
  device:
    type: smartphone
    brand: Samsung
    model: Galaxy S9+
  os_family: Android
  browser_family: Chrome
```

Fields may hold `{placeholder}` references to the named captures of the regex, optionally through
filters (`{model|spaces|title}`), and an entry may add conditions on request headers.
[`src/devices/README.md`](src/devices/README.md) describes the format in full.

## How a lookup works

Thirty-seven thousand entries are far too many to try one by one, so they are held in a radix
tree over the regexes themselves: a node is a prefix its children share, and a lookup only walks
into the subtrees whose prefix matches. A handful of them come back for a common user agent.

Nothing is compiled when the database is read. A pattern first meets a haystack through the literal
text it must contain, which a substring search settles far quicker than the regex engine, and
answers on its own when the pattern is nothing but text. What is left is compiled on demand, and
kept only for as long as a budget was spent on holding it.

## What a lookup costs

A regex left out of the budget is compiled again at **every** lookup that reaches it, so a budget
too small to cover what you match costs far more than the memory it saves. Over two hundred user
agents from the corpus, on one machine:

| budget | a detection | memory | to start |
|---|---|---|---|
| `Budget::none()` | 16.4 ms | +0 MiB | 0.8 s |
| `Budget::regexes(5_000)` | 4.0 ms | +131 MiB | 1.4 s |
| `Budget::regexes(20_000)` | 2.1 ms | +704 MiB | 3.7 s |
| `warm(common_user_agents(), Budget::bytes(128 << 20))` | 1.5 ms | +47 MiB | 1.6 s |

A budget spent blind is mostly spent wrong: it buys the branches of the index that hold the most
entries, which is not where a user agent goes. `Detector::warm` walks the index for user agents
you hand it and pays for exactly the regexes they reach, which is why the last row beats the one
above it at a fifteenth of the memory.

`common_user_agents()` is the two thousand heaviest user agents of a dump of redirection.io's
traffic, in that order. It is one service's mix, which is the honest thing to say about it -- but
real traffic resembles other real traffic far more than it resembles a corpus written to cover
every device ever made, and the corpus row above is the unfavourable measurement for it. Over
rows of the dump itself that the list does not hold it answers in 272 µs, and 944 µs across the
long tail.

**A dump of your own is worth more still**, and `Detector::warm_from_path` reads one a line at a
time.

A budget is said in the unit you have it in -- `Budget::regexes(20_000)` or `Budget::bytes(1 << 30)`
-- and reads from text, so `--budget 900M` can come out of your own configuration.
`cargo run --release --example memory` prints what each one comes to, and `cargo bench` times them.

## Repeat traffic

A detection walks the whole index; reading an answer back does not. Traffic is Zipf shaped -- a
thousand user agents carry nine requests in ten -- so keeping the answers to a few thousand of
them turns almost every request into a hash lookup:

```rust
let detector = {
    let mut detector = Detector::new();
    detector.warm_from_path("user-agents.txt", Budget::bytes(128 << 20))?;
    detector.cache_answers(20_000);
    detector
};

let found = detector.detect_cached(user_agent);   // Option<Arc<Detection>>
```

```
detection                      200 µs
cached answer                   40 ns
```

The answer is shared rather than cloned -- a `Detection` is ten strings, and copying them out
costs several times what reading the cache does. `Arc` dereferences, so `found.is_mobile()` and
`found.device` read straight through it; a `match` on the fields wants `found.as_deref()` first.

One detector for the whole process, shared by `Arc` or through `shared()`: detection reads and
never writes, so nothing contends, and a detector of its own per thread would multiply a hundred
megabytes by the thread count. The cache is shared for the same reason -- one of a given size
answers for more of the traffic than the same memory split between threads.

## The corpus

What a user agent means is not something this library decides. It is taken from the test fixtures
of [matomo-org/device-detector](https://github.com/matomo-org/device-detector), 38 030 cases
pinned to one revision in `castor.php`:

```
castor matomo:sync       # pull the fixtures again, and rewrite src/warm.txt off them
cargo test --release     # 38 030 cases, read off disk, all green
```

The entries are written against those cases and nothing else: matomo's own regexes are not
vendored, and none of them is copied here.

## Licence

LGPL-3.0-or-later, the same as the corpus it is written against. See [LICENSE](LICENSE), and
[LICENSE.GPL](LICENSE.GPL) for the terms it incorporates.

The fixtures under `tests/matomo-device-detector/fixtures/` are copied verbatim from
matomo-org/device-detector, Copyright (C) Matomo Analytics, and `src/devices/*.yml` was first
derived from them.
