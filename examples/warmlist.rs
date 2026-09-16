//! Writes `src/warm.txt`, the user agents the crate ships to warm itself on.
//!
//!     cargo run --release --example warmlist
//!
//! An even stride across the corpus, one user agent per line. Not a sample of anyone's traffic
//! -- there is none to take here -- but of what the database answers, so that a caller with no
//! dump of its own still pays for regexes real user agents reach rather than for whichever
//! branch of the index happened to come first. `castor warm:build` runs this after
//! `castor matomo:sync`, since the corpus moving moves what is worth compiling with it.

#[path = "all/fixtures.rs"]
mod fixtures;

use std::io::Write;
use std::path::PathBuf;

/// How many to keep. A thousand costs some 150 KiB of crate and covers the index as widely as
/// several thousand do -- past this the stride mostly lands on user agents whose regexes another
/// line already paid for.
const KEEP: usize = 1_000;

fn main() {
    let agents: Vec<String> = fixtures::files()
        .iter()
        .flat_map(|path| fixtures::read(path))
        .map(|case| case.user_agent)
        .filter(|agent| !agent.contains('\n'))
        .collect();

    let stride = agents.len().div_euclid(KEEP).max(1);
    let kept: Vec<&String> = agents.iter().step_by(stride).take(KEEP).collect();

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/warm.txt");
    let mut file = std::fs::File::create(&path).expect("src/warm.txt is writable");

    for agent in &kept {
        writeln!(file, "{agent}").expect("a line of it");
    }

    let size = file.metadata().map(|data| data.len()).unwrap_or_default();

    println!(
        "{} of {} user agents, one in {stride}, {} KiB into {}",
        kept.len(),
        agents.len(),
        size / 1024,
        path.display()
    );
}
