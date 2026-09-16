# Detection entries

Each file here is a list of entries. An entry pairs a regex with the result a user agent
matching it resolves to, and that result is exactly the shape the detector returns:

```yaml
-
  regex: 'Mozilla/5\.0 \(Linux; Android (?<os_version>[\d.]+); EK-GC(?<number>\d{3})\).*'
  os: {name: Android, version: "{os_version}", platform: ""}
  client: {type: browser, name: Chrome, version: "{v}", engine: Blink, engine_version: "{v}"}
  device: {type: camera, brand: Samsung, model: "Galaxy Camera {number}"}
  os_family: Android
  browser_family: Chrome
```

A regex has to cover the **whole** user agent: it is anchored on both ends. Use `.*` where a
tail does not matter.

## Placeholders

Any text field may hold `{name}`, filled with what the group `name` captured. A placeholder
that names no group is refused when the detector is built, so a typo fails loudly instead of
resolving to nothing.

Filters transform what was captured, and chain left to right:

| filter | for | example |
|---|---|---|
| `dots` | versions written with underscores or dashes | `10_10_3` → `10.10.3` |
| `spaces` | models written with underscores | `8227L_demo` → `8227L demo` |
| `upper` | models written in lower case | `k2001o` → `K2001O` |
| `title` | models shouted in capitals | `THOR PRO` → `Thor Pro` |

`{model|spaces|title}` turns `ZOEY_SMART` into `Zoey Smart`.

## The two `type:` fields

`device.type` and `client.type` are not free text: they are the vocabularies matomo answers with,
and anything else fails when the detector reads the file. Placeholders and filters do not apply
to them.

```
device.type   desktop, smartphone, tablet, phablet, feature phone, console, tv, car browser,
              smart display, smart speaker, camera, portable media player, wearable, peripheral
client.type   browser, mobile app, mediaplayer, feed reader, pim, library
```

`bot.category` is the one vocabulary left open, because matomo adds to it -- the AI crawlers all
arrived at once -- and a synchronisation that brings a new name must not leave the database
unloadable. A name this library does not know reads back as `BotCategory::Other`.

## Headers

An entry may also require request headers, which is where a browser puts the client hints that
refine or contradict its user agent. Every listed header has to match; a header the request
does not carry never matches. Named groups of a header condition feed the same placeholders as
the user agent ones.

```yaml
-
  regex: 'Mozilla/5\.0 \(Linux; Android [\d.]+; K\).*'
  headers:
    sec-ch-ua-platform-version: '^(?<os_version>[\d.]+)$'
    sec-ch-ua-model: '^SM-A105F$'
  os: {name: Android, version: "{os_version}", platform: ""}
```

Header conditions are searched, not anchored, so anchor them yourself when it matters.

## Priority, and the merge

Every entry that matches has its say. They are applied in ascending order and each lays what it
knows over what came before, so **the last one wins**: highest priority, then the most header
conditions, then the position of the entry across the files read in sorted order.

That is what lets an entry answer for part of a user agent rather than all of it. One entry may
name the client from a token it recognises, another the model it is running on, and the two
compose without either having been written with the other in mind.

`priority:` defaults to 0 and may be negative. An entry that matches anything of its shape goes
*below* the entries that name something in particular:

```
 0   an entry that names a device, a client and an operating system outright
-1   a fallback: the shape is right but the specifics are a guess
-2   a catch-all: matches a whole family, to be overwritten by anything that knows better
-3   an axis: says one thing about the user agent and nothing else
```

Header conditions outrank the file order because an entry that read them knows strictly more:
client hints exist to say what the user agent gets wrong.

## Saying nothing, and saying nothing in particular

Leaving a field out and writing `''` mean opposite things.

```yaml
device: {type: smartphone}              # says nothing about the brand: another entry may
device: {type: smartphone, brand: ''}   # says the brand is empty, and overwrites one that was found
```

That holds for `type:` as well, where `''` is how an entry says the thing has no kind -- a model
token names a device without saying what kind of device it is -- and reads back as `None` rather
than as a kind.

An entry cut from a single user agent usually wants the second: it knows the whole answer and
means the empty fields. An entry written for one axis wants the first, so the rest can be filled
in by whatever else matched.

## Bots

A bot entry carries a `bot` block instead of the three results, and nothing else. A bot that
matches is the whole answer, whatever else matched and whatever its priority, so an over-wide bot
regex is the one mistake the merge cannot correct elsewhere:

```yaml
-
  regex: '.*Googlebot.*'
  bot:
    name: 'Googlebot'
    category: 'Search bot'
    url: 'https://www.google.com/bot.html'
    producer: {name: 'Google Inc.', url: 'https://www.google.com'}
```
