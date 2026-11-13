//! Rebuilds a page's text with its layout: glyphs that share a baseline form a line, and each glyph sits in the
//! column its position gives, so a fixed-width statement reads back with its columns aligned.

/// A visible character and its box in page points (origin bottom left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub c: char,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(0.0)
}

/// The page's lines from top to bottom; columns come from the typical glyph width, measured from the left
/// edge of the page so the indentation of every line is kept.
pub fn lines(glyphs: &[Glyph]) -> Vec<String> {
    let visible: Vec<&Glyph> = glyphs
        .iter()
        .filter(|g| !g.c.is_whitespace() && g.width > 0.0)
        .collect();
    if visible.is_empty() {
        return Vec::new();
    }
    let advance = median(visible.iter().map(|g| g.width).collect());
    let tolerance = median(visible.iter().map(|g| g.height).collect()) / 2.0;
    rows(visible, tolerance)
        .into_iter()
        .map(|row| render_row(row, advance))
        .collect()
}

/// Glyphs sharing a baseline (within `tolerance`), top row first.
fn rows(mut glyphs: Vec<&Glyph>, tolerance: f64) -> Vec<Vec<&Glyph>> {
    glyphs.sort_by(|a, b| b.y.total_cmp(&a.y));
    let mut rows: Vec<(f64, Vec<&Glyph>)> = Vec::new();
    for glyph in glyphs {
        match rows.last_mut() {
            Some((y, row)) if (*y - glyph.y).abs() <= tolerance => row.push(glyph),
            _ => rows.push((glyph.y, vec![glyph])),
        }
    }
    rows.into_iter().map(|(_, row)| row).collect()
}

/// The row as text, each glyph in the column `x / advance` gives.
fn render_row(mut row: Vec<&Glyph>, advance: f64) -> String {
    row.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut line = String::new();
    let mut column = 0usize;
    for glyph in row {
        let at = (glyph.x / advance).round().max(0.0) as usize;
        while column < at {
            line.push(' ');
            column += 1;
        }
        line.push(glyph.c);
        column += 1;
    }
    line
}

/// Removes the left margin all lines share, so the text starts at column 0; blank lines become empty.
pub fn trim_margin(lines: Vec<String>) -> Vec<String> {
    let margin = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .into_iter()
        .map(|l| {
            if l.trim().is_empty() {
                String::new()
            } else {
                l.chars().skip(margin).collect()
            }
        })
        .collect()
}
