//! The native PDF preview (ADR 0037): a PDFKit `PDFView` placed above the main window's webview, over the
//! page's placeholder. Text can be selected and copied; links are swallowed and PDFKit runs no PDF
//! JavaScript. Overlays the page draws above it cut holes into it (ADR 0057). Every change runs on the main
//! thread inside `with_webview`.

use std::cell::RefCell;
use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSResponder, NSView, NSWindowOrderingMode};
use objc2_core_graphics::CGMutablePath;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSURL};
use objc2_pdf_kit::{PDFDisplayMode, PDFDocument, PDFView, PDFViewDelegate};
use objc2_quartz_core::{CAShapeLayer, kCAFillRuleEvenOdd};
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

/// What the page draws above the preview right now.
#[derive(Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Covers {
    /// A sheet is open: the view hides.
    pub hidden: bool,
    /// Overlays the view leaves out and lets clicks through, in the same coordinates as `Frame`.
    pub holes: Vec<Frame>,
    /// Every click on the view goes to the page, so a click beside an open menu closes it.
    pub click_through: bool,
}

pub enum Change {
    Show { path: PathBuf, frame: Frame },
    Move(Frame),
    Hide,
    Cover(Covers),
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

/// Where clicks pass through the view to the page, in the coordinates of the view's superview.
#[derive(Default)]
struct PassThrough {
    holes: Vec<NSRect>,
    click_through: bool,
}

fn contains(rect: NSRect, point: NSPoint) -> bool {
    point.x >= rect.origin.x
        && point.x < rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y < rect.origin.y + rect.size.height
}

define_class!(
    /// A PDFView that hands clicks to the page where an overlay sits above it.
    #[unsafe(super(PDFView, NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = RefCell<PassThrough>]
    struct PreviewView;

    unsafe impl NSObjectProtocol for PreviewView {}

    impl PreviewView {
        /// `point` is in the superview's coordinates; None lets the view below, the webview, take it.
        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> Option<Retained<NSView>> {
            let passes = {
                let through = self.ivars().borrow();
                through.click_through || through.holes.iter().any(|&hole| contains(hole, point))
            };
            if passes {
                None
            } else {
                // SAFETY: NSView's own hit test with the same argument.
                unsafe { msg_send![super(self), hitTest: point] }
            }
        }
    }
);

impl PreviewView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(RefCell::new(PassThrough::default()));
        // SAFETY: PDFView's designated initializer on a fresh instance.
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }
}

/// The view and its delegate, which PDFView holds only weakly.
struct NativeView {
    view: Retained<PreviewView>,
    _link_blocker: Retained<LinkBlocker>,
}

#[derive(Default)]
struct Preview {
    native: Option<NativeView>,
    frame: Frame,
    shown: bool,
    covers: Covers,
}

fn create_view(webview: &NSView, mtm: MainThreadMarker) -> NativeView {
    let blocker = LinkBlocker::new(mtm);
    let view = PreviewView::new(mtm);
    // SAFETY: configuring a fresh view on the main thread.
    unsafe {
        view.setAutoScales(true);
        view.setDisplayMode(PDFDisplayMode::SinglePageContinuous);
        view.setDelegate(Some(ProtocolObject::from_ref(&*blocker)));
    }
    view.setWantsLayer(true);
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
            Change::Cover(covers) => self.covers = covers,
        }
        self.layout(webview);
    }

    fn layout(&self, webview: &NSView) {
        let Some(NativeView { view, .. }) = &self.native else {
            return;
        };
        let visible =
            self.shown && !self.covers.hidden && self.frame.width > 0.0 && self.frame.height > 0.0;
        view.setHidden(!visible);
        if !visible {
            return;
        }
        view.setFrame(native_frame(webview, self.frame));
        self.apply_covers(view, webview);
    }

    /// Cuts the overlays out of the view and lets their clicks through.
    fn apply_covers(&self, view: &PreviewView, webview: &NSView) {
        let holes: Vec<NSRect> = self
            .covers
            .holes
            .iter()
            .map(|&hole| native_frame(webview, hole))
            .collect();
        cut_out(view, &holes);
        *view.ivars().borrow_mut() = PassThrough {
            holes,
            click_through: self.covers.click_through,
        };
    }
}

/// Masks the view so it draws nothing inside `holes` (superview coordinates); no holes, no mask.
fn cut_out(view: &NSView, holes: &[NSRect]) {
    let Some(layer) = view.layer() else {
        return;
    };
    let Some(parent) = superview(view) else {
        return;
    };
    if holes.is_empty() {
        // SAFETY: removing the mask on the main thread.
        unsafe { layer.setMask(None) };
        return;
    }
    let path = CGMutablePath::new();
    // SAFETY: rectangles added to a fresh path without a transform.
    unsafe {
        CGMutablePath::add_rect(Some(&path), std::ptr::null(), view.bounds());
        for &hole in holes {
            let local = view.convertRect_fromView(hole, Some(&parent));
            CGMutablePath::add_rect(Some(&path), std::ptr::null(), local);
        }
    }
    let mask = CAShapeLayer::new();
    mask.setFrame(view.bounds());
    mask.setPath(Some(&path));
    // SAFETY: a static string constant of QuartzCore.
    mask.setFillRule(unsafe { kCAFillRuleEvenOdd });
    // SAFETY: a fresh layer as the view's mask, on the main thread.
    unsafe { layer.setMask(Some(&mask)) };
}

fn superview(view: &NSView) -> Option<Retained<NSView>> {
    // SAFETY: a getter, called on the main thread.
    unsafe { view.superview() }
}

/// A box of the page in the coordinates of the webview's superview, which may count from the bottom.
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
            AppError::Internal(crate::tr!(
                "Couldn't reach the window for the PDF preview: {e}",
                "Das Fenster für die PDF-Vorschau war nicht erreichbar: {e}"
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
        .map_err(|_| {
            AppError::Internal(crate::tr!(
                "The window didn't answer about the PDF preview",
                "Das Fenster hat zur PDF-Vorschau nicht geantwortet"
            ))
        })
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
