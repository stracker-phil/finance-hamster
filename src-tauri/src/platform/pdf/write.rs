//! A small PDF writer: A4 pages of positioned text in the standard fonts, WinAnsi-encoded. Enough for
//! synthetic statements and the year summary; no images, no compression.

use std::fmt::Write as _;

pub const A4_WIDTH: f32 = 595.0;
pub const A4_HEIGHT: f32 = 842.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Font {
    Courier,
    CourierBold,
    Helvetica,
    HelveticaBold,
}

impl Font {
    const ALL: [Font; 4] = [
        Font::Courier,
        Font::CourierBold,
        Font::Helvetica,
        Font::HelveticaBold,
    ];

    fn base_name(self) -> &'static str {
        match self {
            Font::Courier => "Courier",
            Font::CourierBold => "Courier-Bold",
            Font::Helvetica => "Helvetica",
            Font::HelveticaBold => "Helvetica-Bold",
        }
    }

    fn resource(self) -> &'static str {
        match self {
            Font::Courier => "F1",
            Font::CourierBold => "F2",
            Font::Helvetica => "F3",
            Font::HelveticaBold => "F4",
        }
    }
}

/// One piece of text; `x` and `y` are points from the page's bottom left, `y` the baseline.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub font: Font,
    pub text: String,
}

pub type Page = Vec<Run>;

/// A character in WinAnsiEncoding; anything it lacks becomes `?`.
fn win_ansi(c: char) -> u8 {
    match c {
        ' '..='~' => c as u8,
        '\u{a0}'..='\u{ff}' => c as u32 as u8,
        '€' => 0x80,
        '‚' => 0x82,
        '„' => 0x84,
        '…' => 0x85,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '−' => b'-',
        _ => b'?',
    }
}

/// A PDF string literal: `(` and `)` and `\` escaped, bytes outside ASCII written in octal.
fn literal(text: &str) -> String {
    let mut out = String::from("(");
    for byte in text.chars().map(win_ansi) {
        match byte {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(byte as char);
            }
            0x20..=0x7e => out.push(byte as char),
            _ => {
                let _ = write!(out, "\\{byte:03o}");
            }
        }
    }
    out.push(')');
    out
}

fn content(page: &Page) -> String {
    let mut out = String::new();
    for run in page {
        let _ = writeln!(
            out,
            "BT /{} {:.2} Tf {:.2} {:.2} Td {} Tj ET",
            run.font.resource(),
            run.size,
            run.x,
            run.y,
            literal(&run.text)
        );
    }
    out
}

const FIRST_FONT_OBJECT: usize = 3;

/// The document's bytes. Object numbers: 1 catalog, 2 page tree, then the fonts, then a page and its content
/// stream per page.
pub fn write(pages: &[Page]) -> Vec<u8> {
    serialize(&objects(pages))
}

fn objects(pages: &[Page]) -> Vec<String> {
    let first_page = FIRST_FONT_OBJECT + Font::ALL.len();
    let mut objects: Vec<String> = Vec::new();
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".into());
    let kids: Vec<String> = (0..pages.len())
        .map(|i| format!("{} 0 R", first_page + 2 * i))
        .collect();
    objects.push(format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        pages.len()
    ));
    for font in Font::ALL {
        objects.push(format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /{} /Encoding /WinAnsiEncoding >>",
            font.base_name()
        ));
    }
    let fonts: Vec<String> = Font::ALL
        .iter()
        .enumerate()
        .map(|(i, f)| format!("/{} {} 0 R", f.resource(), FIRST_FONT_OBJECT + i))
        .collect();
    for (i, page) in pages.iter().enumerate() {
        let stream = content(page);
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {A4_WIDTH} {A4_HEIGHT}] /Resources << /Font << {} >> >> /Contents {} 0 R >>",
            fonts.join(" "),
            first_page + 2 * i + 1
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{stream}endstream",
            stream.len()
        ));
    }
    objects
}

fn serialize(objects: &[String]) -> Vec<u8> {
    let size = objects.len() + 1;
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{object}\nendobj\n", i + 1);
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}
