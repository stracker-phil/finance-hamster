//! The native PDF preview (ADR 0037): a PDFKit `PDFView` placed above the main window's webview, over the
//! page's placeholder. Text can be selected and copied; links are swallowed and PDFKit runs no PDF
//! JavaScript. Every change runs on the main thread inside `with_webview`.

use std::cell::RefCell;
use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSView, NSWindowOrderingMode};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSURL};
use objc2_pdf_kit::{PDFDisplayMode, PDFDocument, PDFView, PDFViewDelegate};
use serde::Deserialize;
use tauri::WebviewWindow;
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::platform::pdf::file_url;

/// The placeholder's box in CSS pixels, relative to the webview's top left.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub enum Change {
    Show {
        path: PathBuf,
        frame: Frame,
    },
    Move(Frame),
    Hide,
    /// An overlay opened above the page; the view hides until every overlay has resumed it.
    Suspend,
    Resume,
}

define_class!(
    /// Answers PDFView's link clicks by doing nothing, so the preview never navigates or opens a URL.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    struct LinkBlocker;

    unsafe impl NSObjectProtocol for LinkBlocker {}

    unsafe impl PDFViewDelegate for LinkBlocker {
        #[unsafe(method(PDFViewWillClickOnLink:withURL:))]
        fn will_click_on_link(&self, _sender: &PDFView, _url: &NSURL) {}
    }
);

impl LinkBlocker {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        // SAFETY: `init` on a fresh NSObject subclass without ivars.
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

/// The view and its delegate, which PDFView holds only weakly.
struct NativeView {
    view: Retained<PDFView>,
    _link_blocker: Retained<LinkBlocker>,
}

#[derive(Default)]
struct Preview {
    native: Option<NativeView>,
    frame: Frame,
    shown: bool,
    suspended: u32,
}

fn create_view(webview: &NSView, mtm: MainThreadMarker) -> NativeView {
    let blocker = LinkBlocker::new(mtm);
    // SAFETY: configuring a fresh view on the main thread.
    let view = unsafe {
        let view = PDFView::initWithFrame(PDFView::alloc(mtm), NSRect::ZERO);
        view.setAutoScales(true);
        view.setDisplayMode(PDFDisplayMode::SinglePageContinuous);
        view.setDelegate(Some(ProtocolObject::from_ref(&*blocker)));
        view
    };
    view.setHidden(true);
    if let Some(parent) = superview(webview) {
        parent.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Above, Some(webview));
    }
    NativeView {
        view,
        _link_blocker: blocker,
    }
}

thread_local! {
    static PREVIEW: RefCell<Preview> = RefCell::new(Preview::default());
}

impl Preview {
    fn ensure_view(&mut self, webview: &NSView, mtm: MainThreadMarker) -> &PDFView {
        &self
            .native
            .get_or_insert_with(|| create_view(webview, mtm))
            .view
    }

    fn apply(&mut self, webview: &NSView, mtm: MainThreadMarker, change: Change) {
        match change {
            Change::Show { path, frame } => {
                let url = file_url(&path);
                let view = self.ensure_view(webview, mtm);
                // SAFETY: opens the file read-only; a file that isn't a PDF leaves the view empty.
                unsafe {
                    let document = PDFDocument::initWithURL(PDFDocument::alloc(), &url);
                    view.setDocument(document.as_deref());
                }
                self.frame = frame;
                self.shown = true;
            }
            Change::Move(frame) => self.frame = frame,
            Change::Hide => {
                self.shown = false;
                if let Some(NativeView { view, .. }) = &self.native {
                    // SAFETY: clearing the document on the main thread.
                    unsafe { view.setDocument(None) };
                }
            }
            Change::Suspend => self.suspended += 1,
            Change::Resume => self.suspended = self.suspended.saturating_sub(1),
        }
        self.layout(webview);
    }

    fn layout(&self, webview: &NSView) {
        let Some(NativeView { view, .. }) = &self.native else {
            return;
        };
        let visible =
            self.shown && self.suspended == 0 && self.frame.width > 0.0 && self.frame.height > 0.0;
        view.setHidden(!visible);
        if visible {
            view.setFrame(native_frame(webview, self.frame));
        }
    }
}

fn superview(view: &NSView) -> Option<Retained<NSView>> {
    // SAFETY: a getter, called on the main thread.
    unsafe { view.superview() }
}

/// The placeholder's box in the coordinates of the webview's superview, which may count from the bottom.
fn native_frame(webview: &NSView, frame: Frame) -> NSRect {
    let outer = webview.frame();
    let flipped = superview(webview).is_some_and(|parent| parent.isFlipped());
    let y = if flipped {
        outer.origin.y + frame.y
    } else {
        outer.origin.y + outer.size.height - frame.y - frame.height
    };
    NSRect::new(
        NSPoint::new(outer.origin.x + frame.x, y),
        NSSize::new(frame.width, frame.height),
    )
}

fn on_main(
    window: &WebviewWindow,
    f: impl FnOnce(&tauri::webview::PlatformWebview) + Send + 'static,
) -> AppResult<()> {
    window
        .with_webview(move |platform| f(&platform))
        .map_err(|e| {
            AppError::Internal(format!(
                "Couldn't reach the window for the PDF preview: {e}"
            ))
        })
}

/// Whether the preview is on screen now, asked on the main thread.
pub fn visible(window: &WebviewWindow) -> AppResult<bool> {
    let (tx, rx) = std::sync::mpsc::channel();
    on_main(window, move |_| {
        let visible = PREVIEW.with(|preview| {
            let preview = preview.borrow();
            preview.native.as_ref().is_some_and(|n| !n.view.isHidden())
        });
        let _ = tx.send(visible);
    })?;
    rx.recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| AppError::Internal("The window didn't answer about the PDF preview".into()))
}

/// Applies `change` on the main thread.
pub fn apply(window: &WebviewWindow, change: Change) -> AppResult<()> {
    on_main(window, move |platform| {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        // SAFETY: on macOS `inner` is the window's WKWebView, an NSView that lives as long as the window.
        let Some(webview) = (unsafe { platform.inner().cast::<NSView>().as_ref() }) else {
            return;
        };
        PREVIEW.with(|preview| preview.borrow_mut().apply(webview, mtm, change));
    })
}
