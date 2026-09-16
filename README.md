# device-detector

Reads a user agent, and the client hints that go with it, into the device, operating system and
client it names.

```rust
let found = device_detector::shared()
    .detect("Mozilla/5.0 (Linux; Android 10; SM-G9650) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/80.0.3987.99 Safari/537.36")
    .unwrap();

assert_eq!(found.device.unwrap().model, "Galaxy S9+");
assert_eq!(found.os.unwrap().version, "10");
```

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
device_detector::shared().detect_with_headers(user_agent, &[
    ("Sec-CH-UA-Platform", "\"Android\""),
    ("Sec-CH-UA-Model", "\"SM-G9650\""),
]);
```

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
kept only when `Detector::cache` is asked to spend a budget on it, breadth first from the root of
the index.

```
loading                        0.85 s, 171 MiB, nothing compiled yet
detection                      96 µs per agent, everything compiled
memory, everything compiled    1.3 GiB
```

`cargo bench` times loading, compiling and detecting at a range of budgets, and
`cargo run --release --example memory` prints what each of them costs to hold. A regex left out of
the budget is compiled again at every lookup that reaches it, so a budget too small to cover what
you actually match costs far more than the memory it saves.

A budget spent blind is mostly spent wrong: `Detector::warm` walks the index for user agents you
hand it and pays for exactly the regexes they reach. A thousand of your own is worth more than
ten times the budget spent guessing.

## Repeat traffic

A detection walks the whole index; reading an answer back does not. Traffic is Zipf shaped -- a
thousand user agents carry nine requests in ten -- so keeping the answers to a few thousand of
them turns almost every request into a hash lookup:

```rust
let detector = {
    let mut detector = Detector::new();
    detector.warm(your_thousand_heaviest_user_agents, 2_000);
    detector.cache_answers(20_000);
    detector
};

let found = detector.detect_cached(user_agent);   // Option<Arc<Detection>>
```

```
detection                      200 µs
cached answer                   40 ns
```

One detector for the whole process, shared by `Arc` or through `shared()`: detection reads and
never writes, so nothing contends, and a detector of its own per thread would multiply a hundred
megabytes by the thread count. The cache is shared for the same reason -- one of a given size
answers for more of the traffic than the same memory split between threads.

## The corpus

What a user agent means is not something this library decides. It is taken from the test fixtures
of [matomo-org/device-detector](https://github.com/matomo-org/device-detector), 38 030 cases
pinned to one revision in `castor.php`:

```
castor matomo:sync       # pull the fixtures again
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
