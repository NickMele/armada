//! Studio capture on the wire: what a Note keeps of where a person pointed,
//! and the one act that makes one. `#1290`, `docs/concepts/studio.md`, *Notes*.
//!
//! **The frame crosses as a staged file, not as bytes.** Bridge's main process
//! writes the PNG where `stage_attachment` writes one and names the path;
//! Fleet copies it into its own keeping and the Note names only what it kept.
//! A base64 image inside a JSON body would sit in every `studio.changed` for
//! the life of the Studio.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::StudioNodeId;
use crate::studio::StudioPosition;

/// Where the element sat in the window, in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureBounds {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

/// How big the window was, so the box above reads against it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureWindow {
    pub width: i64,
    pub height: i64,
}

/// The element itself, as a person saw it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureElement {
    pub tag: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// The frame Fleet kept beside the Studio's records: its file name under the
/// Studio's own directory, and what it weighs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureFrame {
    pub filename: String,
    pub byte_size: u64,
    pub width: i64,
    pub height: i64,
}

/// The PNG Bridge took, written to disk before the request. **Never read back
/// to a client**: it is an input to `capture_studio_note` alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedFrame {
    pub staged_path: String,
    pub width: i64,
    pub height: i64,
}

/// Everything a Note keeps about where it was pointed, as `get_studio`
/// answers it under a `note` node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioCapture {
    /// The innermost React component under the press, where one was found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    /// The components above it, nearest first — the parent chain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<String>,
    pub selector: String,
    pub element: CaptureElement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    pub location: String,
    pub bounds: CaptureBounds,
    pub window: CaptureWindow,
    /// `getComputedStyle`, for the properties Bridge declares it reads.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub styles: BTreeMap<String, String>,
    pub markup: String,
    /// **Absent unless the build exposes one.** React 19 fibers carry no
    /// `_debugSource`, so Bridge sends this only where it has a path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// What Fleet stored. Absent where no frame could be taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<CaptureFrame>,
}

/// `capture_studio_note`: a Note fixed at capture, placed on the Studio.
///
/// **No `frame` on the Note here.** The request carries the staged PNG and
/// Fleet answers with the Note naming the file it kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureStudioNote {
    /// What the person said, verbatim.
    pub said: String,
    /// Where they pointed, without the frame.
    pub capture: StudioCapture,
    pub position: StudioPosition,
    /// The PNG Bridge took, staged on disk. Absent where nothing could take
    /// one — the Note is still a Note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<StagedFrame>,
    /// The node the capture was made from, where one was. The Studio draws the
    /// `produced` edge itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced_by: Option<StudioNodeId>,
}

impl StudioCapture {
    pub fn to_domain(self) -> core_model::StudioCapture {
        core_model::StudioCapture {
            component: self.component,
            owners: self.owners,
            selector: self.selector,
            element: core_model::CaptureElement {
                tag: self.element.tag,
                text: self.element.text,
                label: self.element.label,
            },
            screen: self.screen,
            layer: self.layer,
            location: self.location,
            bounds: core_model::CaptureBounds {
                x: self.bounds.x,
                y: self.bounds.y,
                width: self.bounds.width,
                height: self.bounds.height,
            },
            window: core_model::CaptureWindow {
                width: self.window.width,
                height: self.window.height,
            },
            styles: self.styles,
            markup: self.markup,
            source: self.source,
            frame: self.frame.map(|frame| core_model::CaptureFrame {
                filename: frame.filename,
                byte_size: frame.byte_size,
                width: frame.width,
                height: frame.height,
            }),
        }
    }
}

impl From<&core_model::StudioCapture> for StudioCapture {
    fn from(capture: &core_model::StudioCapture) -> StudioCapture {
        StudioCapture {
            component: capture.component.clone(),
            owners: capture.owners.clone(),
            selector: capture.selector.clone(),
            element: CaptureElement {
                tag: capture.element.tag.clone(),
                text: capture.element.text.clone(),
                label: capture.element.label.clone(),
            },
            screen: capture.screen.clone(),
            layer: capture.layer.clone(),
            location: capture.location.clone(),
            bounds: CaptureBounds {
                x: capture.bounds.x,
                y: capture.bounds.y,
                width: capture.bounds.width,
                height: capture.bounds.height,
            },
            window: CaptureWindow {
                width: capture.window.width,
                height: capture.window.height,
            },
            styles: capture.styles.clone(),
            markup: capture.markup.clone(),
            source: capture.source.clone(),
            frame: capture.frame.as_ref().map(|frame| CaptureFrame {
                filename: frame.filename.clone(),
                byte_size: frame.byte_size,
                width: frame.width,
                height: frame.height,
            }),
        }
    }
}
