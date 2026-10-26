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

use std::sync::Arc;

use egui::{Color32, Galley, Image, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

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
        match FileName(name).extension().as_deref() {
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
        let ty = FileName(&self.name)
            .extension()
            .map(|e| e.to_ascii_uppercase());
        let sz = self.size_bytes.map(|b| Bytes(b).human());
        match (ty, sz) {
            (Some(t), Some(s)) => Some(format!("{t} · {s}")),
            (Some(t), None) => Some(t),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        }
    }

    /// Render the chip, returning its [`AttachmentResponse`].
    pub fn show(mut self, ui: &mut Ui) -> AttachmentResponse {
        let gap = crate::sizing::resolve(std::mem::take(&mut self.sizing_hook)).gap;
        let desc = self.resolved_description();
        let lay = ChipLayout {
            m: Metrics::of(self.size),
            gap,
            vertical: self.orientation == Orientation::Vertical,
            has_desc: desc.is_some(),
        };
        let (rect, response) = ui.allocate_exact_size(lay.size(self.width), Sense::click());
        // Off-screen chips paint nothing and, crucially, request no frames:
        // an invisible busy chip that keeps its shimmer cadence armed stops
        // the window from ever idling.
        if !ui.is_rect_visible(rect) {
            return AttachmentResponse {
                response,
                removed: false,
            };
        }

        let tokens = Tokens::get(ui);
        self.paint_chrome(ui, tokens, rect);
        let tile = lay.tile(rect);
        self.paint_media(ui, tokens, tile, lay.m.glyph);
        let removed = self.removable && Self::remove_button(ui, tokens, lay.close(rect), &response);
        let (left, name_cy, avail) = lay.text(rect, tile, self.removable);
        self.paint_content(
            ui,
            tokens,
            &lay.m,
            egui::pos2(left, name_cy),
            avail,
            desc.as_deref(),
        );

        // Via `Repaint` so the cadence is a real interval (a bare
        // `request_repaint_after` is only a deadline) and is visible to a
        // paint-caching host, which must not replay an animating chip.
        if self.state.is_busy() {
            crate::repaint::Repaint::request_after(
                ui.ctx(),
                "attachment.busy",
                crate::repaint::Repaint::GRADIENT,
            );
        }
        AttachmentResponse { response, removed }
    }

    /// Card chrome; a destructive border while in the error state.
    fn paint_chrome(&self, ui: &Ui, tokens: Tokens, rect: Rect) {
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
    }

    /// The × button. Returns `true` when clicked.
    fn remove_button(ui: &Ui, tokens: Tokens, btn: Rect, chip: &Response) -> bool {
        let r = ui.interact(btn, chip.id.with("rm"), Sense::click());
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
        let glyph = Rect::from_center_size(btn.center(), btn.size() * 0.6);
        Icon::new(X).color(tint).image(tokens).paint_at(ui, glyph);
        if r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        r.clicked()
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
            // A faint destructive wash behind the alert glyph.
            let wash = tokens.muted.lerp_to_gamma(tokens.destructive, 0.18);
            (ALERT, tokens.destructive, wash)
        } else {
            (kind.glyph(), tokens.muted_foreground, tokens.muted)
        };
        ui.painter().rect_filled(tile, tokens.radius_md(), fill);
        Icon::new(svg).color(tint).image(tokens).paint_at(
            ui,
            egui::Rect::from_center_size(tile.center(), Vec2::splat(glyph)),
        );
    }

    /// Paint the name (shimmering while busy) with the description beneath,
    /// the pair centred vertically on `name_anchor`.
    fn paint_content(
        &self,
        ui: &Ui,
        tokens: Tokens,
        m: &Metrics,
        name_anchor: egui::Pos2,
        avail: f32,
        desc: Option<&str>,
    ) {
        let name_color = if self.state.is_busy() {
            // 0..1, three radians a second.
            let pulse = ui.input(|i| (i.time * 3.0).sin().mul_add(0.5, 0.5));
            tokens
                .muted_foreground
                .lerp_to_gamma(tokens.foreground, Self::unit(pulse))
        } else {
            tokens.foreground
        };
        let name = Fit { ui, max: avail }.line(&self.name, m.name, name_color);
        let name_h = name.size().y;
        let block_h = desc.map_or(name_h, |_| name_h + 2.0 + m.desc);
        let top = name_anchor.y - block_h / 2.0;
        ui.painter()
            .galley(egui::pos2(name_anchor.x, top), name, name_color);

        if let Some(d) = desc {
            let color = if self.state == State::Error {
                tokens.destructive
            } else {
                tokens.muted_foreground
            };
            let g = Fit { ui, max: avail }.line(d, m.desc, color);
            ui.painter()
                .galley(egui::pos2(name_anchor.x, top + name_h + 2.0), g, color);
        }
    }

    /// A 0..1 animation value from `f64` time maths, as a lerp factor.
    #[allow(clippy::cast_possible_truncation)]
    const fn unit(v: f64) -> f32 {
        v as f32
    }
}

/// Where the chip's parts sit, for one size and orientation.
struct ChipLayout {
    m: Metrics,
    gap: f32,
    vertical: bool,
    has_desc: bool,
}

impl ChipLayout {
    /// The chip's size: `width` or the orientation's default, around a
    /// padded tile (with the text stacked beneath it when vertical).
    fn size(&self, width: Option<f32>) -> Vec2 {
        let m = self.m;
        let (default_w, below) = if self.vertical {
            let desc = if self.has_desc { 2.0 + m.desc } else { 0.0 };
            (132.0, self.gap + m.name + desc)
        } else {
            (240.0, 0.0)
        };
        Vec2::new(
            width.unwrap_or(default_w),
            m.pad.mul_add(2.0, m.tile + below),
        )
    }

    /// The media tile: centred at the top when vertical, else at the left.
    fn tile(&self, rect: Rect) -> Rect {
        let m = self.m;
        let min = if self.vertical {
            egui::pos2(rect.center().x - m.tile / 2.0, rect.top() + m.pad)
        } else {
            egui::pos2(rect.left() + m.pad, rect.center().y - m.tile / 2.0)
        };
        Rect::from_min_size(min, Vec2::splat(m.tile))
    }

    /// The remove button, in the padded top-right corner.
    fn close(&self, rect: Rect) -> Rect {
        let m = self.m;
        let min = egui::pos2(rect.right() - m.pad - m.close, rect.top() + m.pad);
        Rect::from_min_size(min, Vec2::splat(m.close))
    }

    /// `(left, name centre y, width)` for the text block. Beside the tile
    /// it stops short of the remove button; beneath it the button is above.
    fn text(&self, rect: Rect, tile: Rect, removable: bool) -> (f32, f32, f32) {
        let (m, gap) = (self.m, self.gap);
        let (left, name_cy) = if self.vertical {
            (rect.left() + m.pad, tile.bottom() + gap + m.name / 2.0)
        } else {
            (tile.right() + gap, rect.center().y)
        };
        let reserve = if removable && !self.vertical {
            m.close + gap
        } else {
            0.0
        };
        let right = rect.right() - m.pad - reserve;
        (left, name_cy, (right - left).max(0.0))
    }
}

/// A file name.
struct FileName<'a>(&'a str);

impl FileName<'_> {
    /// The lowercase extension, if any.
    fn extension(&self) -> Option<String> {
        self.0
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .filter(|e| !e.is_empty() && e.len() <= 5)
    }
}

/// A byte count.
struct Bytes(u64);

impl Bytes {
    /// A short human string (`284 KB`, `1.4 MB`).
    fn human(&self) -> String {
        const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
        if self.0 < 1024 {
            return format!("{} B", self.0);
        }
        let mut size = self.float() / 1024.0;
        let mut unit = 1;
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

    /// The count as a float; above 2⁵³ it rounds, far below display precision.
    #[allow(clippy::cast_precision_loss)]
    const fn float(&self) -> f64 {
        self.0 as f64
    }
}

/// Lays text out on one line no wider than `max`.
struct Fit<'a> {
    ui: &'a Ui,
    max: f32,
}

impl Fit<'_> {
    /// `text` at `size` and `color`, trimmed with a trailing `…` to fit.
    fn line(&self, text: &str, size: f32, color: Color32) -> Arc<Galley> {
        let font = egui::FontId::proportional(size);
        let layout = |s: String| self.ui.painter().layout_no_wrap(s, font.clone(), color);
        let full = layout(text.to_owned());
        if full.size().x <= self.max {
            return full;
        }
        // The longest prefix that fits with the ellipsis, by binary search.
        let chars: Vec<char> = text.chars().collect();
        let with_dots = |n: usize| chars[..n].iter().collect::<String>() + "…";
        let fits = |n: usize| layout(with_dots(n)).size().x <= self.max;
        let (mut lo, mut hi) = (0, chars.len());
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if fits(mid) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        layout(with_dots(lo))
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
        let human = |b| Bytes(b).human();
        assert_eq!(human(512), "512 B");
        assert_eq!(human(1024), "1.0 KB");
        assert_eq!(human(284_103), "277 KB");
        assert_eq!(human(1_572_864), "1.5 MB");
        assert_eq!(human(3 << 40), "3.0 TB");
        assert_eq!(human(u64::MAX), "16777216 TB");
    }

    #[test]
    fn every_kind_has_a_glyph() {
        for kind in [
            Kind::File,
            Kind::Image,
            Kind::Document,
            Kind::Audio,
            Kind::Video,
            Kind::Archive,
        ] {
            assert!(kind.glyph().starts_with("<svg"));
        }
    }

    /// Render one chip and return its paint output, its rect, and the frame's
    /// texts.
    fn render(build: impl Fn() -> Attachment) -> (Rect, Vec<String>) {
        let ui = crate::test_ui::TestUi::new();
        let mut rect = Rect::NOTHING;
        let out = ui.frame(|ui| rect = build().show(ui).response.rect);
        let texts = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        (rect, texts)
    }

    /// A narrow chip trims its name with an ellipsis.
    #[test]
    fn long_names_are_truncated() {
        let (_, texts) = render(|| {
            Attachment::new("a-very-long-quarterly-report-final-v2.pdf")
                .size(Size::Xs)
                .removable(true)
                .width(120.0)
        });
        assert!(
            texts.iter().any(|t| t.ends_with('…') && t.len() > 3),
            "{texts:?}"
        );
        assert!(texts.contains(&"PDF".to_owned()));
    }

    /// A vertical chip stacks the text beneath its tile, so it is taller.
    #[test]
    fn vertical_chips_stack() {
        let (wide, _) = render(|| Attachment::new("a.png").size(Size::Sm));
        let (tall, texts) = render(|| {
            Attachment::new("a.png")
                .size(Size::Sm)
                .orientation(Orientation::Vertical)
                .state(State::Error)
                .description("Upload failed")
                .removable(true)
        });
        assert!(tall.height() > wide.height());
        assert!((tall.width() - 132.0).abs() < 1e-3);
        assert!(texts.contains(&"Upload failed".to_owned()));
    }

    /// Clicking × reports a removal.
    #[test]
    fn the_remove_button_reports_removal() {
        let ui = crate::test_ui::TestUi::new();
        let rect = std::cell::Cell::new(Rect::NOTHING);
        let mut removed = false;
        let mut build = |ui: &mut Ui| {
            let r = Attachment::new("logo.svg")
                .image(egui::Image::from_bytes("bytes://logo.svg", FILE.as_bytes()))
                .state(State::Processing)
                .removable(true)
                .show(ui);
            rect.set(r.response.rect);
            removed |= r.removed;
        };
        ui.frame(&mut build);
        let m = Metrics::of(Size::Default);
        let r = rect.get();
        let close = egui::pos2(
            r.right() - m.pad - m.close / 2.0,
            r.top() + m.pad + m.close / 2.0,
        );
        ui.click(close, &mut build);
        drop(build);
        assert!(removed);
    }

    /// A busy attachment that is scrolled OFF-SCREEN must go quiet.
    ///
    /// The regression this pins: the shimmer's `request_after` sat outside
    /// the `is_rect_visible` branch, so a busy chip below the fold kept its
    /// 30fps cadence armed while painting nothing. In the gallery that read
    /// as "the CPU explodes only after I scroll": the scroll woke the page
    /// once, the invisible chip re-armed every pass from then on, and the
    /// window never idled again. Off-screen must mean silent — the same rule
    /// the painting already follows.
    #[test]
    fn an_offscreen_busy_attachment_requests_no_frames() {
        let ui = crate::test_ui::TestUi::new();

        // Several passes: the first frames legitimately carry startup
        // repaints (font atlas, initial layout), so judge only the last.
        let mut delay = std::time::Duration::ZERO;
        for _ in 0..6 {
            let output = ui.frame(|ui| {
                // Push the chip far past the 600pt-tall test viewport; its
                // rect stays real but `is_rect_visible` is false, exactly as
                // for a card below the fold of a scrolled page.
                ui.add_space(5_000.0);
                let _ = Attachment::new("design-system.fig")
                    .size_bytes(8_490_000)
                    .state(State::Uploading)
                    .width(280.0)
                    .show(ui);
            });
            delay = crate::test_ui::TestUi::repaint_delay(&output);
        }

        assert!(
            delay > std::time::Duration::from_secs(1000),
            "an off-screen busy attachment still scheduled a frame ({delay:?}) \
             — it should paint nothing and request nothing"
        );
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
