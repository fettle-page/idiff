//! `library <results_dir> <receipts_dir>`: adds a catalog run's results to the Receipts Library
//! and prints what changed, as Markdown for the pull request.
//!
//! `library findings <results_dir> [<receipts_dir>]`: prints the Markdown for `receipts/FINDINGS.md`
//! from a catalog run's results. With the receipts dir, links use its `index.json`.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, results] if cmd == "findings" => findings(results, None),
        [cmd, results, receipts] if cmd == "findings" => findings(results, Some(receipts)),
        [results, receipts] => match library::apply(Path::new(results), Path::new(receipts)) {
            Ok(summary) => {
                print!("{}", library::summary_for(Path::new(receipts), &summary));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("library: {e}");
                ExitCode::from(1)
            }
        },
        _ => {
            eprintln!(
                "usage: library <results_dir> <receipts_dir>\n       library findings <results_dir> [<receipts_dir>]"
            );
            ExitCode::from(2)
        }
    }
}

fn findings(results: &str, receipts: Option<&String>) -> ExitCode {
    match library::findings::findings_md(Path::new(results), receipts.map(Path::new)) {
        Ok(md) => {
            print!("{md}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("library: {e}");
            ExitCode::from(1)
        }
    }
}
