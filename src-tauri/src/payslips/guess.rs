//! The month a payslip PDF is for, read from a "<month name> <year>" near its top (`Lohnset Juli 2025`).

/// How many lines from the top the month heading may sit.
const HEAD_LINES: usize = 12;

const MONTHS: [[&str; 2]; 12] = [
    ["january", "jänner"],
    ["february", "februar"],
    ["march", "märz"],
    ["april", "april"],
    ["may", "mai"],
    ["june", "juni"],
    ["july", "juli"],
    ["august", "august"],
    ["september", "september"],
    ["october", "oktober"],
    ["november", "november"],
    ["december", "dezember"],
];

fn month_of(word: &str) -> Option<u32> {
    let word = word.to_lowercase();
    let word = if word == "januar" {
        "jänner".to_string()
    } else {
        word
    };
    MONTHS
        .iter()
        .position(|names| names.contains(&word.as_str()))
        .map(|i| i as u32 + 1)
}

/// The first `(year, month)` named in the top lines.
pub fn month(lines: &[String]) -> Option<(i32, u32)> {
    lines.iter().take(HEAD_LINES).find_map(|line| {
        let words: Vec<&str> = line.split_whitespace().collect();
        words.windows(2).find_map(|pair| {
            let month = month_of(pair[0].trim_matches(|c: char| !c.is_alphabetic()))?;
            let year: i32 = pair[1]
                .trim_matches(|c: char| !c.is_ascii_digit())
                .parse()
                .ok()?;
            (1900..=2200).contains(&year).then_some((year, month))
        })
    })
}
