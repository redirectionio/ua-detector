// `include_dir!` does not register the files it embeds as build dependencies, so cargo would
// happily reuse a stale binary after an entry is added or edited.
//
// Unless it is not embedded at all, or a directory is named to read instead, in which case what
// answers is the files on disk and watching them only buys a two minute rebuild for every yaml
// change.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=UA_DETECTOR_DEVICES");

    if std::env::var("CARGO_FEATURE_EMBED").is_err() || std::env::var("UA_DETECTOR_DEVICES").is_ok()
    {
        return;
    }

    println!("cargo:rerun-if-changed=src/devices");

    let Ok(entries) = std::fs::read_dir("src/devices") else {
        return;
    };

    for entry in entries.flatten() {
        println!("cargo:rerun-if-changed={}", entry.path().display());
    }
}
