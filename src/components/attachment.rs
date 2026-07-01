//! [`Attachment`] — a file/image attachment chip, mirroring shadcn's
//! `<Attachment>` (radix-rhea) family.
//!
//! An attachment chip previews a staged file: a leading **media** slot (a kind
//! icon in a rounded tile, or an image preview), the **file name**, a secondary
//! **description** (`PDF · 2.4 MB`), an optional **remove** action, and an
//! **upload state** that drives the styling — `uploading`/`processing` shimmer
//! the title, `error` switches to a destructive treatment.
//!
//! This collapses shadcn's part family (`AttachmentMedia`, `AttachmentContent`,
//! `AttachmentTitle`, `AttachmentDescription`, `AttachmentActions`) into one
//! builder, since egui favours a single painted widget over nested slots:
//!
//! - media — [`kind`](Attachment::kind) (auto from the extension) or
//!   [`image`](Attachment::image);
//! - content — the name plus a [`description`](Attachment::description), which
//!   defaults to `TYPE · SIZE` from the extension and [`size_bytes`];
//! - actions — [`removable`](Attachment::removable) adds a × button;
//! - [`state`](Attachment::state), [`size`](Attachment::size), and
//!   [`orientation`](Attachment::orientation) mirror the root props.
//!
//! ```no_run
//! use glazier::attachment::{Attachment, State};
//! # egui::__run_test_ui(|ui| {
//! let r = Attachment::new("sales-dashboard.pdf")
//!     .size_bytes(2_516_582)
//!     .state(State::Uploading)
//!     .removable(true)
//!     .show(ui);
//! if r.removed {
//!     // drop it from the staged list
//! }
//! # });
//! ```

use egui::{Color32, Image, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::components::icon::Icon;
use crate::sizing::{Sizeable, SizingHook};
use crate::tokens::Tokens;

// --- Bundled lucide glyphs -------------------------------------------------

const FILE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/></svg>"#;
const IMAGE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>"#;
const FILE_TEXT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/></svg>"#;
const MUSIC: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>"#;
const VIDEO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m22 8-6 4 6 4V8Z"/><rect width="14" height="12" x="2" y="6" rx="2" ry="2"/></svg>"#;
const ARCHIVE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/></svg>"#;
const X: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"#;
const ALERT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/></svg>"#;

/// The category of a staged file, selecting the leading glyph.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Kind {
    /// A generic file (the default).
    #[default]
    File,
    /// An image — `png`, `jpg`, `gif`, `webp`, `svg`, …
    Image,
    /// A document — `pdf`, `txt`, `md`, `doc`, …
    Document,
    /// Audio — `mp3`, `wav`, `flac`, `ogg`, …
    Audio,
    /// Video — `mp4`, `mov`, `webm`, `mkv`, …
    Video,
    /// An archive — `zip`, `tar`, `gz`, `rar`, `7z`.
    Archive,
}

impl Kind {
    /// Guess a kind from a file name's extension, defaulting to [`Kind::File`].
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match extension(name).as_deref() {
            Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "avif") => {
                Self::Image
            }
            Some("pdf" | "txt" | "md" | "doc" | "docx" | "rtf" | "odt" | "tex") => Self::Document,
            Some("mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus") => Self::Audio,
            Some("mp4" | "mov" | "webm" | "mkv" | "avi" | "m4v") => Self::Video,
            Some("zip" | "tar" | "gz" | "tgz" | "rar" | "7z" | "bz2" | "xz") => Self::Archive,
            _ => Self::File,
        }
    }

    /// The lucide glyph for this kind.
    const fn glyph(self) -> &'static str {
        match self {
            Self::File => FILE,
            Self::Image => IMAGE,
            Self::Document => FILE_TEXT,
            Self::Audio => MUSIC,
            Self::Video => VIDEO,
            Self::Archive => ARCHIVE,
        }
    }
}

/// The upload lifecycle state, driving styling and the title shimmer
/// (shadcn's `state` prop).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum State {
    /// Staged but not yet uploaded.
    Idle,
    /// Uploading — the title shimmers.
    Uploading,
    /// Processing after upload — the title shimmers.
    Processing,
    /// Upload failed — a destructive treatment.
    Error,
    /// Complete (the default).
    #[default]
    Done,
}

impl State {
    /// Whether this state animates the title shimmer.
    const fn is_busy(self) -> bool {
        matches!(self, Self::Uploading | Self::Processing)
    }
}

/// The attachment size (shadcn's `size` prop).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Size {
    /// The roomy default.
    #[default]
    Default,
    /// Small.
    Sm,
    /// Extra small.
    Xs,
}

/// Whether the media sits beside or above the content (shadcn's `orientation`).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Orientation {
    /// Media left of the content (the default).
    #[default]
    Horizontal,
    /// Media stacked above the content.
    Vertical,
}

/// Overridable geometry for [`Attachment`] — reach in via
/// [`Attachment::sizing`]. Field names match the internal per-size record
/// this used to be, now merged with a user override, resolved per-size below.
#[derive(Clone, Copy, Debug)]
pub struct AttachmentMetrics {
    /// Gap between media, text, and actions.
    pub gap: f32,
}

impl Default for AttachmentMetrics {
    fn default() -> Self {
        Self { gap: 10.0 }
    }
}

/// Resolved per-size geometry.
#[derive(Clone, Copy)]
struct Metrics {
    /// Tile edge length.
    tile: f32,
    /// Name font size.
    name: f32,
    /// Description font size.
    desc: f32,
    /// Inner padding.
    pad: f32,
    /// Remove-button hit square.
    close: f32,
    /// Media glyph size.
    glyph: f32,
}

impl Metrics {
    const fn of(size: Size) -> Self {
        match size {
            Size::Default => Self {
                tile: 40.0,
                name: 14.0,
                desc: 12.0,
                pad: 12.0,
                close: 28.0,
                glyph: 20.0,
            },
            Size::Sm => Self {
                tile: 32.0,
                name: 13.0,
                desc: 11.5,
                pad: 10.0,
                close: 24.0,
                glyph: 17.0,
            },
            Size::Xs => Self {
                tile: 24.0,
                name: 12.0,
                desc: 11.0,
                pad: 8.0,
                close: 20.0,
                glyph: 14.0,
            },
        }
    }
}

/// The outcome of showing an [`Attachment`].
#[derive(Clone)]
#[must_use]
pub struct AttachmentResponse {
    /// The whole chip's response (clicks, hover, …) — shadcn's
    /// `AttachmentTrigger`.
    pub response: Response,
    /// `true` the frame the remove (×) button was clicked.
    pub removed: bool,
}

/// A file/image attachment chip.
#[must_use = "attachments do nothing unless you show them"]
pub struct Attachment {
    name: String,
    kind: Option<Kind>,
    image: Option<Image<'static>>,
    size_bytes: Option<u64>,
    description: Option<String>,
    state: State,
    size: Size,
    orientation: Orientation,
    removable: bool,
    width: Option<f32>,
    sizing_hook: SizingHook<AttachmentMetrics>,
}

impl Attachment {
    /// Create a chip for the file `name` (its extension picks the icon).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: None,
            image: None,
            size_bytes: None,
            description: None,
            state: State::Done,
            size: Size::Default,
            orientation: Orientation::Horizontal,
            removable: false,
            width: None,
            sizing_hook: SizingHook::default(),
        }
    }

    /// Force the file [`Kind`] instead of inferring it from the extension.
    pub const fn kind(mut self, kind: Kind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Use an image preview as the media instead of a kind icon (shadcn's
    /// `AttachmentMedia variant="image"`).
    pub fn image(mut self, image: impl Into<Image<'static>>) -> Self {
        self.image = Some(image.into());
        self
    }

    /// Set the file size in bytes; folded into the default description.
    pub const fn size_bytes(mut self, bytes: u64) -> Self {
        self.size_bytes = Some(bytes);
        self
    }

    /// Override the secondary description line (default: `TYPE · SIZE`).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the upload [`State`].
    pub const fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }

    /// Set the [`Size`].
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Set the [`Orientation`].
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Show a remove (×) button (shadcn's `AttachmentAction`).
    pub const fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }

    /// Set an explicit chip width.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl Sizeable<AttachmentMetrics> for Attachment {
    fn sizing_hook_mut(&mut self) -> &mut SizingHook<AttachmentMetrics> {
        &mut self.sizing_hook
    }
}

impl Attachment {

    /// The description to render: the explicit override, else `TYPE · SIZE`.
    fn resolved_description(&self) -> Option<String> {
        if let Some(d) = &self.description {
            return Some(d.clone());
        }
        let ty = extension(&self.name).map(|e| e.to_ascii_uppercase());
        let sz = self.size_bytes.map(human_size);
        match (ty, sz) {
            (Some(t), Some(s)) => Some(format!("{t} · {s}")),
            (Some(t), None) => Some(t),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        }
    }

    /// Render the chip, returning its [`AttachmentResponse`].
    pub fn show(mut self, ui: &mut Ui) -> AttachmentResponse {
        let tokens = Tokens::get(ui);
        let m = Metrics::of(self.size);
        let desc = self.resolved_description();
        let gap = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook)).gap;
        let vertical = self.orientation == Orientation::Vertical;

        let width = self.width.unwrap_or(if vertical { 132.0 } else { 240.0 });
        let height = if vertical {
            // padded tile + gap + name + desc.
            let lines = m.name + if desc.is_some() { 2.0 + m.desc } else { 0.0 };
            2.0f32.mul_add(m.pad, m.tile + gap + lines)
        } else {
            2.0f32.mul_add(m.pad, m.tile)
        };

        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
        let mut removed = false;

        if ui.is_rect_visible(rect) {
            // Card chrome — destructive border while in the error state.
            let border = if self.state == State::Error {
                tokens.destructive
            } else {
                tokens.border
            };
            ui.painter().rect(
                rect,
                tokens.radius_md(),
                tokens.card,
                Stroke::new(1.0, border),
                StrokeKind::Inside,
            );

            // --- Media tile -------------------------------------------------
            let tile = if vertical {
                egui::Rect::from_min_size(
                    egui::pos2(rect.center().x - m.tile / 2.0, rect.top() + m.pad),
                    Vec2::splat(m.tile),
                )
            } else {
                egui::Rect::from_min_size(
                    egui::pos2(rect.left() + m.pad, rect.center().y - m.tile / 2.0),
                    Vec2::splat(m.tile),
                )
            };
            self.paint_media(ui, tokens, tile, m.glyph);

            // --- Remove action ---------------------------------------------
            let mut content_right = rect.right() - m.pad;
            if self.removable {
                let btn = egui::Rect::from_min_size(
                    egui::pos2(rect.right() - m.pad - m.close, rect.top() + m.pad),
                    Vec2::splat(m.close),
                );
                let r = ui.interact(btn, response.id.with("rm"), Sense::click());
                removed = r.clicked();
                let hover_t = ui.ctx().animate_bool_with_time(r.id, r.hovered(), 0.12);
                if hover_t > 0.0 {
                    ui.painter().rect_filled(
                        btn,
                        tokens.radius_md(),
                        tokens.accent.gamma_multiply(hover_t),
                    );
                }
                let tint = tokens
                    .muted_foreground
                    .lerp_to_gamma(tokens.foreground, hover_t);
                Icon::new(X).color(tint).image(tokens).paint_at(
                    ui,
                    egui::Rect::from_center_size(btn.center(), Vec2::splat(m.close * 0.6)),
                );
                if r.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if !vertical {
                    content_right -= m.close + gap;
                }
            }

            // --- Content (name + description) -------------------------------
            let (text_left, name_center_y) = if vertical {
                (rect.left() + m.pad, tile.bottom() + gap + m.name / 2.0)
            } else {
                (tile.right() + gap, rect.center().y)
            };
            let avail = (content_right - text_left).max(0.0);
            self.paint_content(
                ui,
                tokens,
                &m,
                text_left,
                name_center_y,
                avail,
                desc.as_deref(),
            );
        }

        if self.state.is_busy() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }

        AttachmentResponse { response, removed }
    }

    /// Paint the media tile: a rounded image preview or a kind glyph on a muted
    /// square.
    fn paint_media(&self, ui: &Ui, tokens: Tokens, tile: egui::Rect, glyph: f32) {
        if let Some(image) = &self.image {
            ui.painter()
                .rect_filled(tile, tokens.radius_md(), tokens.muted);
            image
                .clone()
                .corner_radius(tokens.radius_md())
                .paint_at(ui, tile);
            return;
        }
        let kind = self.kind.unwrap_or_else(|| Kind::from_name(&self.name));
        // Error swaps the glyph + tint for a destructive alert.
        let (svg, tint, fill) = if self.state == State::Error {
            (ALERT, tokens.destructive, destructive_wash(tokens))
        } else {
            (kind.glyph(), tokens.muted_foreground, tokens.muted)
        };
        ui.painter().rect_filled(tile, tokens.radius_md(), fill);
        Icon::new(svg).color(tint).image(tokens).paint_at(
            ui,
            egui::Rect::from_center_size(tile.center(), Vec2::splat(glyph)),
        );
    }

    /// Paint the name (with shimmer while busy) and the description line.
    #[allow(clippy::too_many_arguments)]
    fn paint_content(
        &self,
        ui: &Ui,
        tokens: Tokens,
        m: &Metrics,
        left: f32,
        name_center_y: f32,
        avail: f32,
        desc: Option<&str>,
    ) {
        let name_font = egui::FontId::proportional(m.name);
        let desc_font = egui::FontId::proportional(m.desc);

        // The name shimmers between muted and foreground while uploading.
        let name_color = if self.state.is_busy() {
            let t = ui.input(|i| i.time);
            #[allow(clippy::cast_possible_truncation)]
            let pulse = 0.5f32.mul_add((t as f32 * 3.0).sin(), 0.5);
            tokens
                .muted_foreground
                .lerp_to_gamma(tokens.foreground, pulse)
        } else {
            tokens.foreground
        };

        let name = truncate_to_width(ui, &self.name, &name_font, avail);
        let g = ui.painter().layout_no_wrap(name, name_font, name_color);
        let name_h = g.size().y;

        // Centre the name line on `name_center_y`, lifting it to make room for
        // the description so name + desc straddle that anchor.
        let block_h = if desc.is_some() {
            name_h + 2.0 + m.desc
        } else {
            name_h
        };
        let name_top = name_center_y - block_h / 2.0;
        ui.painter()
            .galley(egui::pos2(left, name_top), g, name_color);

        if let Some(d) = desc {
            let color = if self.state == State::Error {
                tokens.destructive
            } else {
                tokens.muted_foreground
            };
            let dg = truncate_to_width(ui, d, &desc_font, avail);
            let g = ui.painter().layout_no_wrap(dg, desc_font, color);
            ui.painter()
                .galley(egui::pos2(left, name_top + name_h + 2.0), g, color);
        }
    }
}

/// The lowercase extension of `name`, if any.
fn extension(name: &str) -> Option<String> {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .filter(|e| !e.is_empty() && e.len() <= 5)
}

/// A faint destructive tint for the error media tile.
fn destructive_wash(tokens: Tokens) -> Color32 {
    tokens.muted.lerp_to_gamma(tokens.destructive, 0.18)
}

/// Format a byte count as a short human string (`284 KB`, `1.4 MB`).
fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    #[allow(clippy::cast_precision_loss)]
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if size >= 100.0 {
        format!("{size:.0} {}", UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

/// Trim `text` with a trailing `…` so it fits within `max` points.
fn truncate_to_width(ui: &Ui, text: &str, font: &egui::FontId, max: f32) -> String {
    let measure = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
    };
    if measure(text) <= max {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let candidate: String = chars[..mid].iter().collect::<String>() + "…";
        if measure(&candidate) <= max {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo == 0 {
        "…".to_owned()
    } else {
        chars[..lo].iter().collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_from_extension() {
        assert_eq!(Kind::from_name("photo.PNG"), Kind::Image);
        assert_eq!(Kind::from_name("report.pdf"), Kind::Document);
        assert_eq!(Kind::from_name("song.flac"), Kind::Audio);
        assert_eq!(Kind::from_name("clip.mp4"), Kind::Video);
        assert_eq!(Kind::from_name("bundle.tar.gz"), Kind::Archive);
        assert_eq!(Kind::from_name("README"), Kind::File);
        assert_eq!(Kind::from_name("notes"), Kind::File);
    }

    #[test]
    fn human_size_units() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(284_103), "277 KB");
        assert_eq!(human_size(1_572_864), "1.5 MB");
    }

    #[test]
    fn description_defaults_to_type_and_size() {
        let a = Attachment::new("sales-dashboard.pdf").size_bytes(2_516_582);
        assert_eq!(a.resolved_description().as_deref(), Some("PDF · 2.4 MB"));
        let b = Attachment::new("notes").size_bytes(2048);
        assert_eq!(b.resolved_description().as_deref(), Some("2.0 KB"));
        let c = Attachment::new("logo.svg");
        assert_eq!(c.resolved_description().as_deref(), Some("SVG"));
        let d = Attachment::new("plain").description("Custom");
        assert_eq!(d.resolved_description().as_deref(), Some("Custom"));
    }
}
