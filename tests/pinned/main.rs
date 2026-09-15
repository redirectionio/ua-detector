//! What the corpus does not speak for: user agents from production traffic, which the corpus
//! shows once or not at all. `production` is written by hand, `traffic` is generated from
//! `tests/pinned/traffic/cases.csv` by `cargo run --release --example traffic`.

mod production;
mod traffic;
