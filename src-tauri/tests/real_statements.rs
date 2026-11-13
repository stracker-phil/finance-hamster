//! Reads the owner's real statement PDFs from the folder named by `HAMSTER_REAL_FIXTURES`. Slow and outside
//! the repo's data: run with `--ignored`. Only counts are printed, never statement content.

use hamster_lib::platform::pdf::Pdf;
use hamster_lib::statements::check::check;
use hamster_lib::statements::parse::parse;

#[test]
#[ignore]
fn every_real_statement_parses_and_passes_the_checks() {
    let Some(folder) = std::env::var_os("HAMSTER_REAL_FIXTURES") else {
        eprintln!("HAMSTER_REAL_FIXTURES is not set; skipping");
        return;
    };
    let mut paths: Vec<_> = std::fs::read_dir(&folder)
        .expect("read the fixtures folder")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "the fixtures folder has no PDF");

    let mut lines = 0;
    let mut failures: Vec<String> = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let pages = match Pdf::open(path).and_then(|pdf| pdf.text_pages()) {
            Ok(pages) => pages,
            Err(_) => {
                failures.push(format!("#{i}: not readable as text"));
                continue;
            }
        };
        let parsed = match parse(&pages) {
            Ok(parsed) => parsed,
            Err(_) => {
                failures.push(format!("#{i}: not parsed"));
                continue;
            }
        };
        if check(&parsed).is_err() {
            failures.push(format!("#{i}: failed the checks"));
            continue;
        }
        lines += parsed.lines.len();
    }
    println!(
        "{} statements, {} read and checked, {lines} lines",
        paths.len(),
        paths.len() - failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("; "));
}
