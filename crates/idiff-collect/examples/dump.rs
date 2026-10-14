//! Prints one source as JSON, to see what it reads on this PC: `cargo run -p idiff-collect --example dump -- Services`

#[cfg(windows)]
fn main() {
    use idiff_collect::{SnapshotOptions, snapshot};
    use idiff_core::model::SourceKind;

    let name = std::env::args().nth(1).unwrap_or_default();
    let Ok(kind) = serde_json::from_value::<SourceKind>(serde_json::Value::from(name.as_str())) else {
        eprintln!("usage: dump <Source>, one of {:?}", SourceKind::ALL);
        std::process::exit(2);
    };
    let s = snapshot(&SnapshotOptions {
        only: Some(vec![kind]),
        ..Default::default()
    });
    println!("{}", serde_json::to_string_pretty(&s.sources[&kind]).unwrap());
}

#[cfg(not(windows))]
fn main() {
    eprintln!("dump reads Windows, so it only runs there");
}
