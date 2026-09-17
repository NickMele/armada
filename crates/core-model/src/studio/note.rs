//! What a Note keeps of the moment it was captured. `#1290`,
//! `docs/concepts/studio.md`, *Notes*.
//!
//! **Fixed at capture.** Every field is set by the constructor and read back;
//! nothing here offers a write, which is what makes the rule hold.
//!
//! **The development layer's fields plus four.** The layer's shape is
//! `apps/desktop/src/shared/annotations.ts`; a Note adds the computed styles,
//! the trimmed markup, a stored frame and a source file path.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Where the element sat in the window, in CSS pixels. **Not `box`**, which
/// Rust reserves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureBounds {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

/// How big the window was, so the box above can be read against it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureWindow {
    pub width: i64,
    pub height: i64,
}

/// The element itself, as a person saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureElement {
    /// Lowercased, as the DOM spells it.
    pub tag: String,
    /// The visible text, collapsed and cut.
    pub text: String,
    /// The accessible name, where the element carries one.
    pub label: Option<String>,
}

/// The frame Fleet kept. **A file name and never a path**, so moving where
/// Fleet keeps its machine state does not rewrite every Note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureFrame {
    pub filename: String,
    pub byte_size: u64,
    /// The image's own pixels: the window's, times the device ratio.
    pub width: i64,
    pub height: i64,
}

/// Everything a Note keeps about where it was pointed.
///
/// Public fields because there is nothing to protect: no method writes one,
/// and a Note is only ever made from a capture that already happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioCapture {
    /// The innermost React component under the press, where one was found.
    pub component: Option<String>,
    /// The components above it, nearest first. **The parent chain**: a
    /// packaged Bridge runs a production React, which keeps no owners.
    pub owners: Vec<String>,
    /// A CSS selector that found the element, and finds it again.
    pub selector: String,
    pub element: CaptureElement,
    /// The rail item marked current, where one was.
    pub screen: Option<String>,
    /// The dialog or sheet the element sat in, by its accessible name.
    pub layer: Option<String>,
    /// Path, query and hash of the page.
    pub location: String,
    pub bounds: CaptureBounds,
    pub window: CaptureWindow,
    /// `getComputedStyle`, for the properties Bridge declares it reads.
    pub styles: BTreeMap<String, String>,
    /// `outerHTML`, trimmed by the capture to a length it declares.
    pub markup: String,
    /// The file the element's JSX is in, **only where the build says so**.
    /// React 19 fibers carry no `_debugSource`, so this is usually absent.
    pub source: Option<String>,
    /// The frame kept beside the Studio's records, where one was taken.
    pub frame: Option<CaptureFrame>,
}
