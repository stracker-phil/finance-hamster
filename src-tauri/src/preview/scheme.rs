//! `pdf-page://localhost/<kind>/<id>/<page>.png?w=<pixels>`: a page of a known PDF as PNG, for thumbnails.
//! Unknown ids, pages or malformed requests answer 404; nothing else is ever read.

use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, UriSchemeContext, UriSchemeResponder, Wry};

use super::{PdfSource, SourceKind};
use crate::error::AppResult;
use crate::platform::pdf::Pdf;
use crate::state::AppState;

pub const SCHEME: &str = "pdf-page";
const DEFAULT_WIDTH: f64 = 600.0;
const MAX_WIDTH: f64 = 2400.0;

/// What a request asks for: the source, the 0-based page and the width.
pub fn parse(path: &str, query: Option<&str>) -> Option<(PdfSource, usize, f64)> {
    let mut parts = path.trim_start_matches('/').split('/');
    let kind = SourceKind::parse(parts.next()?)?;
    let id = parts.next()?.to_string();
    let page: usize = parts.next()?.strip_suffix(".png")?.parse().ok()?;
    if parts.next().is_some() || id.is_empty() || page == 0 {
        return None;
    }
    let width = query
        .into_iter()
        .flat_map(|q| q.split('&'))
        .find_map(|pair| pair.strip_prefix("w="))
        .and_then(|w| w.parse::<f64>().ok())
        .unwrap_or(DEFAULT_WIDTH)
        .clamp(16.0, MAX_WIDTH);
    Some((PdfSource { kind, id }, page - 1, width))
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    fn source(kind: SourceKind, id: &str) -> PdfSource {
        PdfSource {
            kind,
            id: id.into(),
        }
    }

    #[test]
    fn a_plain_path_asks_for_the_first_page_at_the_default_width() {
        assert_eq!(
            parse("/document/abc/1.png", None),
            Some((source(SourceKind::Document, "abc"), 0, 600.0))
        );
    }

    #[test]
    fn pages_are_one_based_in_the_path_and_zero_based_in_the_result() {
        assert_eq!(
            parse("/statement/s1/3.png", None),
            Some((source(SourceKind::Statement, "s1"), 2, 600.0))
        );
    }

    #[test]
    fn the_width_comes_from_the_query_and_is_clamped() {
        for (query, expected) in [
            ("w=300", 300.0),
            ("x=1&w=300", 300.0),
            ("w=1", 16.0),
            ("w=99999", 2400.0),
            ("w=wide", 600.0),
            ("other=5", 600.0),
        ] {
            let (_, _, width) = parse("/document/abc/1.png", Some(query)).expect("parses");
            assert_eq!(width, expected, "{query}");
        }
    }

    #[test]
    fn malformed_requests_are_rejected() {
        for path in [
            "",
            "/document",
            "/document/abc",
            "/image/abc/1.png",
            "/document/abc/0.png",
            "/document/abc/1",
            "/document/abc/x.png",
            "/document/abc/1.png/extra",
            "/document//1.png",
        ] {
            assert_eq!(parse(path, None), None, "{path}");
        }
    }
}

fn not_found() -> Response<Vec<u8>> {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = StatusCode::NOT_FOUND;
    response
}

fn render(
    app: &tauri::AppHandle,
    source: &PdfSource,
    page: usize,
    width: f64,
) -> AppResult<Vec<u8>> {
    let state = app.state::<AppState>();
    let path = super::resolve(&state.db, &crate::profile_dir(), source)?;
    Pdf::open(&path)?.page_png(page, width)
}

pub fn handle(
    ctx: UriSchemeContext<'_, Wry>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    let uri = request.uri().clone();
    std::thread::spawn(move || {
        let response = match parse(uri.path(), uri.query()) {
            None => not_found(),
            Some((source, page, width)) => match render(&app, &source, page, width) {
                Ok(png) => Response::builder()
                    .header("Content-Type", "image/png")
                    .header("Cache-Control", "no-store")
                    .body(png)
                    .unwrap_or_else(|_| not_found()),
                Err(e) => {
                    log::info!("No page for {}: {e}", uri.path());
                    not_found()
                }
            },
        };
        responder.respond(response);
    });
}
