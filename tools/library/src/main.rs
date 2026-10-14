//! `library <results_dir> <receipts_dir>`: adds a catalog run's results to the Receipts Library
//! and prints what changed, as Markdown for the pull request.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [results, receipts] = args.as_slice() else {
        eprintln!("usage: library <results_dir> <receipts_dir>");
        return ExitCode::from(2);
    };
    match library::apply(Path::new(results), Path::new(receipts)) {
        Ok(summary) => {
            print!("{}", library::summary_for(Path::new(receipts), &summary));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("library: {e}");
            ExitCode::from(1)
        }
    }
}
