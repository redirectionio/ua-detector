//! What one compiled regex actually costs, by pattern kind.
//!
//!     cargo run --release --example regexcost

fn resident() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|line| line.split_whitespace().next()?.parse().ok())
        .unwrap_or(0)
}

fn main() {
    let samples = [
        ("noeud", r"Mozilla/5\.0 \(Linux; Android (?<os_version>[\d.]+); S"),
        ("entree ancree", r"^(?:Mozilla/5\.0 \(Linux; Android (?<os_version>[\d.]+); SM-A536B\) AppleWebKit/537\.36 \(KHTML, like Gecko\) Chrome/(?<client_version>[\d.]+) Mobile Safari/537\.36)$"),
        ("entree libre", r"Mozilla/5\.0 \(Linux; Android (?<os_version>[\d.]+); SM-A536B\) AppleWebKit/537\.36 \(KHTML, like Gecko\) Chrome/(?<client_version>[\d.]+) Mobile Safari/537\.36"),
        ("ancree, 1 capture", r"^(?:Mozilla/5\.0 \(Linux; Android (?<os_version>[\d.]+); SM-A536B\) AppleWebKit/537\.36 \(KHTML, like Gecko\) Chrome/537\.36 Mobile Safari/537\.36)$"),
        ("ancree, 0 capture", r"^(?:Mozilla/5\.0 \(Linux; Android 13; SM-A536B\) AppleWebKit/537\.36 \(KHTML, like Gecko\) Chrome/120 Mobile Safari/537\.36)$"),
        ("ancree, courte", r"^(?:Android (?<os_version>[\d.]+); SM-A536B)$"),
        ("litteral", r"Mozilla/5\.0 \(Linux; Android "),
    ];

    for (name, pattern) in samples {
        for (limits, label) in [(false, "par defaut"), (true, "bride")] {
            let before = resident();
            let mut kept = Vec::new();

            for _ in 0..2000 {
                if limits {
                    let configured = regex_automata::meta::Regex::builder()
                        .configure(
                            regex_automata::meta::Config::new()
                                .onepass_size_limit(Some(0))
                                .dfa_size_limit(Some(0))
                                .hybrid_cache_capacity(1 << 13),
                        )
                        .build(pattern)
                        .unwrap();

                    kept.push(Box::new(configured) as Box<dyn std::any::Any>);
                } else {
                    kept.push(Box::new(regex::Regex::new(pattern).unwrap()) as Box<dyn std::any::Any>);
                }
            }

            println!("{:>16} {:>10}  {:>6} Kio par regex", name, label, (resident() - before) / 2000);
            drop(kept);
        }
    }
}
