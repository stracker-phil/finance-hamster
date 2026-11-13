//! PDFs through PDFKit (ADR 0018, ADR 0037): the text of each page with its layout kept, and pages rendered
//! to PNG. Files are opened read-only and never changed.

pub mod layout;
pub mod write;

#[cfg(test)]
mod tests;

use std::path::Path;

use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep};
use objc2_foundation::{NSData, NSDictionary, NSRange, NSSize, NSString, NSURL};
use objc2_pdf_kit::{PDFDisplayBox, PDFDocument, PDFPage};

use crate::error::{AppError, AppResult};
use layout::Glyph;

/// A PDF opened by PDFKit.
pub struct Pdf {
    document: Retained<PDFDocument>,
}

// SAFETY: a `PDFDocument` used from one thread at a time is safe; `Pdf` is only moved, never shared.
unsafe impl Send for Pdf {}

impl Pdf {
    pub fn open(path: &Path) -> AppResult<Pdf> {
        let unreadable =
            || AppError::InvalidInput(format!("{} isn't a readable PDF", path.display()));
        let url = file_url(path);
        // SAFETY: `initWithURL` reads the file; it returns None when it isn't a PDF.
        let document = unsafe { PDFDocument::initWithURL(PDFDocument::alloc(), &url) };
        document
            .map(|document| Pdf { document })
            .ok_or_else(unreadable)
    }

    pub fn from_bytes(bytes: &[u8]) -> AppResult<Pdf> {
        let data = NSData::with_bytes(bytes);
        // SAFETY: `initWithData` parses the bytes; it returns None when they aren't a PDF.
        let document = unsafe { PDFDocument::initWithData(PDFDocument::alloc(), &data) };
        document
            .map(|document| Pdf { document })
            .ok_or_else(|| AppError::InvalidInput("The data isn't a readable PDF".into()))
    }

    pub fn page_count(&self) -> usize {
        // SAFETY: a plain getter.
        unsafe { self.document.pageCount() }
    }

    fn page(&self, index: usize) -> AppResult<Retained<PDFPage>> {
        // SAFETY: `pageAtIndex` is only called below `pageCount`.
        (index < self.page_count())
            .then(|| unsafe { self.document.pageAtIndex(index) })
            .flatten()
            .ok_or_else(|| AppError::NotFound(format!("The PDF has no page {}", index + 1)))
    }

    /// Each page's lines, laid out as fixed-width text (`layout::lines`).
    pub fn text_pages(&self) -> AppResult<Vec<Vec<String>>> {
        (0..self.page_count())
            .map(|i| {
                let page = self.page(i)?;
                Ok(layout::trim_margin(layout::lines(&glyphs(&page))))
            })
            .collect()
    }

    /// The page as a PNG no wider than `max_width` pixels, keeping its proportions.
    pub fn page_png(&self, index: usize, max_width: f64) -> AppResult<Vec<u8>> {
        let page = self.page(index)?;
        // SAFETY: plain getters and PDFKit's own thumbnail renderer.
        let image = unsafe {
            let bounds = page.boundsForBox(PDFDisplayBox::CropBox);
            let scale = max_width / bounds.size.width.max(1.0);
            let size = NSSize::new(max_width, (bounds.size.height * scale).round());
            page.thumbnailOfSize_forBox(size, PDFDisplayBox::CropBox)
        };
        let failed = || AppError::Internal("Couldn't render the PDF page".into());
        let tiff = image.TIFFRepresentation().ok_or_else(failed)?;
        let bitmap = NSBitmapImageRep::imageRepWithData(&tiff).ok_or_else(failed)?;
        // SAFETY: an empty property dictionary is valid for every file type.
        let png = unsafe {
            bitmap.representationUsingType_properties(
                NSBitmapImageFileType::PNG,
                &NSDictionary::new(),
            )
        };
        Ok(png.ok_or_else(failed)?.to_vec())
    }
}

/// Every visible character of the page with its box. The characters come from the page's text, where PDFKit
/// marks line breaks; each box comes from a one-character selection, which matches that text index (the
/// `characterBoundsAtIndex` index does not count PDFKit's line breaks).
fn glyphs(page: &PDFPage) -> Vec<Glyph> {
    // SAFETY: read-only queries with indexes below the page text's length.
    unsafe {
        let Some(text) = page.string() else {
            return Vec::new();
        };
        let units: Vec<u16> = text.to_string().encode_utf16().collect();
        let mut out = Vec::new();
        let mut index = 0usize;
        for c in char::decode_utf16(units.iter().copied()) {
            let c = c.unwrap_or(char::REPLACEMENT_CHARACTER);
            let at = index;
            index += c.len_utf16();
            if c.is_whitespace() {
                continue;
            }
            let Some(selection) = page.selectionForRange(NSRange::new(at, c.len_utf16())) else {
                continue;
            };
            let r = selection.boundsForPage(page);
            if r.size.width > 0.0 {
                out.push(Glyph {
                    c,
                    x: r.origin.x,
                    y: r.origin.y,
                    width: r.size.width,
                    height: r.size.height,
                });
            }
        }
        out
    }
}

/// A `file://` URL for a local path.
pub fn file_url(path: &Path) -> Retained<NSURL> {
    NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()))
}
