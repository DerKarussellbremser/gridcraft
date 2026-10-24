//! A tiny, self-contained PDF 1.7 writer for GridCraft's print and PDF export.
//!
//! - The 14 standard fonts (no embedding) with WinAnsiEncoding, plus approximate width tables
//!   ([`text_width`]) so callers can measure, align and wrap text. Characters outside
//!   WinAnsiEncoding (CJK and other scripts) are set in a predefined, non-embedded Adobe-GB1 CID
//!   font with the `UniGB-UTF16-H` CMap instead of being replaced by `?`.
//! - Vector drawing: rectangles, lines (solid or dashed), polygons, circles (Bézier), clipping.
//! - RGBA images as RGB image XObjects with a soft mask for alpha (run-length encoded).
//! - Coordinates are points with a **top-left origin** (y grows down); text `y` is the baseline.
//!
//! - TrueType fonts from the system, embedded as the subset of glyphs a document uses
//!   ([`EmbeddedFont`], [`Page::text_embedded`]), with bold and italic imitated where a face lacks
//!   them.
//!
//! Streams are written uncompressed (images use the trivial RunLengthDecode filter) so the crate
//! needs no dependencies.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

mod metrics;
mod truetype;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::Arc;

pub use truetype::{EmbeddedFont, FaceInfo, face_count, faces};

/// How text in a face that lacks the asked-for style imitates it, as Excel does: slanted by 1/3,
/// and outlined about 1/36 of its size thick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Synthetic {
    pub bold: bool,
    pub italic: bool,
}

/// RGB colour, 0–255 per channel.
pub type Rgb = [u8; 3];

/// The standard 14 fonts minus Symbol and ZapfDingbats.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Font {
    #[default]
    Helvetica,
    HelveticaBold,
    HelveticaOblique,
    HelveticaBoldOblique,
    TimesRoman,
    TimesBold,
    TimesItalic,
    TimesBoldItalic,
    Courier,
    CourierBold,
    CourierOblique,
    CourierBoldOblique,
}

/// A font family for [`Font::styled`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Family {
    #[default]
    Sans,
    Serif,
    Mono,
}

impl Font {
    pub const ALL: [Font; 12] = [
        Font::Helvetica,
        Font::HelveticaBold,
        Font::HelveticaOblique,
        Font::HelveticaBoldOblique,
        Font::TimesRoman,
        Font::TimesBold,
        Font::TimesItalic,
        Font::TimesBoldItalic,
        Font::Courier,
        Font::CourierBold,
        Font::CourierOblique,
        Font::CourierBoldOblique,
    ];

    /// PostScript base font name.
    pub fn base_name(self) -> &'static str {
        match self {
            Font::Helvetica => "Helvetica",
            Font::HelveticaBold => "Helvetica-Bold",
            Font::HelveticaOblique => "Helvetica-Oblique",
            Font::HelveticaBoldOblique => "Helvetica-BoldOblique",
            Font::TimesRoman => "Times-Roman",
            Font::TimesBold => "Times-Bold",
            Font::TimesItalic => "Times-Italic",
            Font::TimesBoldItalic => "Times-BoldItalic",
            Font::Courier => "Courier",
            Font::CourierBold => "Courier-Bold",
            Font::CourierOblique => "Courier-Oblique",
            Font::CourierBoldOblique => "Courier-BoldOblique",
        }
    }

    fn index(self) -> usize {
        Font::ALL.iter().position(|f| *f == self).unwrap_or(0)
    }

    /// The face's family name, weight and slant: (`Helvetica`, bold, italic).
    pub fn style(self) -> (&'static str, bool, bool) {
        let family = match self {
            Font::Helvetica | Font::HelveticaBold | Font::HelveticaOblique | Font::HelveticaBoldOblique => "Helvetica",
            Font::TimesRoman | Font::TimesBold | Font::TimesItalic | Font::TimesBoldItalic => "Times",
            _ => "Courier",
        };
        let name = self.base_name();
        (family, name.contains("Bold"), name.contains("Oblique") || name.contains("Italic"))
    }

    /// The face of `family` with the given weight and slant.
    pub fn styled(family: Family, bold: bool, italic: bool) -> Font {
        match (family, bold, italic) {
            (Family::Sans, false, false) => Font::Helvetica,
            (Family::Sans, true, false) => Font::HelveticaBold,
            (Family::Sans, false, true) => Font::HelveticaOblique,
            (Family::Sans, true, true) => Font::HelveticaBoldOblique,
            (Family::Serif, false, false) => Font::TimesRoman,
            (Family::Serif, true, false) => Font::TimesBold,
            (Family::Serif, false, true) => Font::TimesItalic,
            (Family::Serif, true, true) => Font::TimesBoldItalic,
            (Family::Mono, false, false) => Font::Courier,
            (Family::Mono, true, false) => Font::CourierBold,
            (Family::Mono, false, true) => Font::CourierOblique,
            (Family::Mono, true, true) => Font::CourierBoldOblique,
        }
    }

    /// Picks a standard family for a font name: serif faces map to Times, monospaced ones to
    /// Courier, everything else to Helvetica.
    pub fn family_for(name: &str) -> Family {
        let n = name.to_ascii_lowercase();
        const SERIF: [&str; 10] = ["times", "cambria", "georgia", "garamond", "serif", "book", "palatino", "baskerville", "tinos", "caladea"];
        const MONO: [&str; 8] = ["courier", "consolas", "mono", "menlo", "lucida console", "cousine", "code", "typewriter"];
        if MONO.iter().any(|m| n.contains(m)) {
            Family::Mono
        } else if SERIF.iter().any(|s| n.contains(s)) && !n.contains("sans") {
            Family::Serif
        } else {
            Family::Sans
        }
    }

    /// Advance width of a character in 1/1000 em.
    pub fn char_width(self, c: char) -> u16 {
        let table: Option<&[u16; 95]> = match self {
            Font::Helvetica | Font::HelveticaOblique => Some(&metrics::HELVETICA),
            Font::HelveticaBold | Font::HelveticaBoldOblique => Some(&metrics::HELVETICA_BOLD),
            Font::TimesRoman | Font::TimesItalic => Some(&metrics::TIMES),
            Font::TimesBold | Font::TimesBoldItalic => Some(&metrics::TIMES_BOLD),
            _ => None,
        };
        if needs_cid(c) {
            return CID_WIDTH;
        }
        let Some(table) = table else { return 600 };
        let ascii = |c: char| -> Option<u16> {
            let i = (c as u32).checked_sub(32)? as usize;
            table.get(i).copied()
        };
        if let Some(w) = ascii(c) {
            return w;
        }
        match c {
            '—' | '…' | '‰' | '™' => 1000,
            _ => metrics::fold(c).and_then(ascii).unwrap_or(match self {
                Font::TimesRoman | Font::TimesItalic | Font::TimesBold | Font::TimesBoldItalic => 500,
                _ => 556,
            }),
        }
    }
}

/// Width of `text` in points when set in `font` at `size` points.
pub fn text_width(font: Font, size: f32, text: &str) -> f32 {
    // u64: a u32 sum overflows for ~8 million characters.
    let units: u64 = text.chars().map(|c| u64::from(font.char_width(c))).sum();
    units as f32 * size / 1000.0
}

/// Unicode → WinAnsiEncoding (CP1252) byte; unmappable characters become `?`.
pub fn win_ansi(c: char) -> u8 {
    let u = c as u32;
    if (0x20..0x7F).contains(&u) || (0xA0..=0xFF).contains(&u) {
        return u as u8;
    }
    match c {
        '€' => 0x80,
        '‚' => 0x82,
        'ƒ' => 0x83,
        '„' => 0x84,
        '…' => 0x85,
        '†' => 0x86,
        '‡' => 0x87,
        'ˆ' => 0x88,
        '‰' => 0x89,
        'Š' => 0x8A,
        '‹' => 0x8B,
        'Œ' => 0x8C,
        'Ž' => 0x8E,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '˜' => 0x98,
        '™' => 0x99,
        'š' => 0x9A,
        '›' => 0x9B,
        'œ' => 0x9C,
        'ž' => 0x9E,
        'Ÿ' => 0x9F,
        '\t' => b' ',
        _ => b'?',
    }
}

/// Index of the CID font in a page's font flags (after the [`Font::ALL`] faces).
const CID_FONT: usize = Font::ALL.len();
/// Number of font resources a page can use.
const FONT_SLOTS: usize = CID_FONT + 1;
/// Base font of the CID font used for characters WinAnsiEncoding lacks.
const CID_BASE_FONT: &str = "STSong-Light";
/// Default glyph width of the CID font (full-width), in 1/1000 em.
const CID_WIDTH: u16 = 1000;

/// True when `c` has no WinAnsiEncoding byte and is drawn with the CID font.
fn needs_cid(c: char) -> bool {
    !c.is_control() && c != '?' && win_ansi(c) == b'?'
}

/// A PDF hex string `< … >` of `text` in UTF-16BE (the `UniGB-UTF16-H` encoding).
fn pdf_utf16_string(text: &str) -> String {
    let mut s = String::with_capacity(text.len() * 4 + 2);
    s.push('<');
    for unit in text.encode_utf16() {
        let _ = write!(s, "{unit:04X}");
    }
    s.push('>');
    s
}

/// A PDF literal string `( … )` for `text` in WinAnsiEncoding.
fn pdf_string(text: &str) -> String {
    let mut s = String::with_capacity(text.len() + 2);
    s.push('(');
    for c in text.chars() {
        let b = win_ansi(c);
        match b {
            b'(' | b')' | b'\\' => {
                s.push('\\');
                s.push(b as char);
            }
            0x20..=0x7E => s.push(b as char),
            _ => {
                let _ = write!(s, "\\{b:03o}");
            }
        }
    }
    s.push(')');
    s
}

/// URI actions use byte strings. Percent-encode non-ASCII UTF-8 before PDF string escaping.
fn pdf_uri_string(uri: &str) -> String {
    let mut ascii = String::with_capacity(uri.len());
    for byte in uri.bytes() {
        if byte.is_ascii() {
            ascii.push(byte as char);
        } else {
            let _ = write!(ascii, "%{byte:02X}");
        }
    }
    pdf_string(&ascii)
}

/// A number for a content stream: finite, short.
fn n(v: f32) -> String {
    let v = if v.is_finite() { v.clamp(-1.0e7, 1.0e7) } else { 0.0 };
    let r = (v * 1000.0).round() / 1000.0;
    if r == r.trunc() {
        format!("{}", r as i64)
    } else {
        let s = format!("{r:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// True when every value is finite (NaN and infinities are rejected).
fn finite(v: &[f32]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// A positive, finite extent (a width, height or radius).
fn extent(v: f32) -> bool {
    v > 0.0 && v.is_finite()
}

/// A stroke width: negative, NaN and infinite become 0 (the thinnest line).
fn stroke_width(w: f32) -> f32 {
    if w.is_finite() { w.max(0.0) } else { 0.0 }
}

/// The entries of a dash array (`[on, off, …]`, at most 8). A pattern the PDF specification
/// forbids (any non-finite entry, or no entry that survives rounding to a positive number) is
/// replaced by an empty array, i.e. a solid line.
fn dash_values(dash: &[f32]) -> String {
    let d: Vec<f32> = dash.iter().take(8).map(|v| if v.is_finite() { v.max(0.0) } else { f32::NAN }).collect();
    if !finite(&d) || !d.iter().any(|v| (v.min(1.0e7) * 1000.0).round() > 0.0) {
        return String::new();
    }
    d.iter().map(|v| n(*v)).collect::<Vec<_>>().join(" ")
}

fn rgb_op(c: Rgb, stroke: bool) -> String {
    let f = |v: u8| n(v as f32 / 255.0);
    format!("{} {} {} {}", f(c[0]), f(c[1]), f(c[2]), if stroke { "RG" } else { "rg" })
}

struct ImageData {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
    alpha: Option<Vec<u8>>,
}

enum LinkTarget {
    Uri(String),
    Page { index: usize, x: f32, y: f32 },
}

struct LinkAnnotation {
    /// PDF coordinates, with the origin at the bottom left.
    rect: [f32; 4],
    target: LinkTarget,
}

/// One thing a page draws, kept when its document records ([`PdfDoc::recording`]) so a preview can
/// paint exactly what prints without reading the PDF back. Coordinates as for drawing: points,
/// top-left origin, text `y` the baseline.
#[derive(Clone, Debug)]
pub enum Op {
    Save,
    Restore,
    Clip {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    FillRect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: Rgb,
    },
    StrokeRect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        width: f32,
        color: Rgb,
    },
    /// An open line through `pts`; `dash` empty for solid.
    Line {
        pts: Vec<[f32; 2]>,
        width: f32,
        color: Rgb,
        dash: Vec<f32>,
    },
    Polygon {
        pts: Vec<[f32; 2]>,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    },
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    },
    Text(TextOp),
    /// RGBA pixels (`pw` × `ph`, straight alpha) drawn into the box.
    Image {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rgba: Arc<[u8]>,
        pw: u32,
        ph: u32,
    },
}

/// Text as drawn: the family it is set in (`Arial`, `Helvetica` for the standard sans) and the
/// weight and slant it shows, whether its face has them or they are imitated.
#[derive(Clone, Debug, PartialEq)]
pub struct TextOp {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    /// Clockwise, in radians, about `(x, y)`.
    pub angle: f32,
    pub color: Rgb,
    pub text: String,
    pub family: String,
    pub bold: bool,
    pub italic: bool,
}

/// Most operations a page records; a hostile workbook can't exhaust memory through a preview.
const MAX_PAGE_OPS: usize = 1_000_000;

/// A page under construction. Obtain one with [`PdfDoc::add_page`].
pub struct Page {
    width: f32,
    height: f32,
    content: String,
    fonts: [bool; FONT_SLOTS],
    /// Embedded faces used on the page, each with its glyphs and the character each stands for.
    embedded: Vec<(Arc<EmbeddedFont>, BTreeMap<u16, char>)>,
    images: Vec<ImageData>,
    links: Vec<LinkAnnotation>,
    depth: u32,
    /// What the page draws, when its document records.
    ops: Option<Vec<Op>>,
    /// The picture bytes a recording page holds so far (see [`MAX_RECORDED_IMAGE_BYTES`]).
    recorded_image_bytes: usize,
}

/// Most picture bytes (RGBA) a recording page keeps, with its own PDF copies of them; past it a
/// picture is recorded as an outline only, so a workbook full of large pictures can't exhaust
/// memory through a preview. Ordinary documents are not limited by it.
#[cfg(not(test))]
const MAX_RECORDED_IMAGE_BYTES: usize = 256 * 1024 * 1024;
/// Small in tests, so the budget can be reached without hundreds of megabytes.
#[cfg(test)]
const MAX_RECORDED_IMAGE_BYTES: usize = 1024 * 1024;

/// Most bytes of content per page before further drawing is dropped (keeps hostile inputs from
/// exhausting memory).
const MAX_PAGE_CONTENT: usize = 64 * 1024 * 1024;
/// Largest image (pixels) embedded.
const MAX_IMAGE_PIXELS: u64 = 40_000_000;
const MAX_PAGE_LINKS: usize = 4096;
const MAX_LINK_URI_BYTES: usize = 4096;

impl Page {
    pub fn width(&self) -> f32 {
        self.width
    }
    pub fn height(&self) -> f32 {
        self.height
    }
    /// What the page draws, in order; empty unless its document records.
    pub fn ops(&self) -> &[Op] {
        self.ops.as_deref().unwrap_or(&[])
    }
    /// Records `op` when the document records, unless the page is full: then the PDF leaves it
    /// out too. Call it before the operation writes its content.
    fn rec(&mut self, op: Op) {
        let full = self.full();
        if let Some(ops) = &mut self.ops
            && !full
            && ops.len() < MAX_PAGE_OPS
        {
            ops.push(op);
        }
    }
    fn y(&self, y: f32) -> f32 {
        self.height - y
    }
    fn full(&self) -> bool {
        self.content.len() > MAX_PAGE_CONTENT
    }
    fn op(&mut self, s: &str) {
        if !self.full() {
            self.content.push_str(s);
            self.content.push('\n');
        }
    }

    fn link_rect(&self, x: f32, y: f32, w: f32, h: f32) -> Option<[f32; 4]> {
        if !finite(&[x, y, w, h]) || !extent(w) || !extent(h) || self.links.len() >= MAX_PAGE_LINKS {
            return None;
        }
        let (right, bottom) = (x + w, y + h);
        if !finite(&[right, bottom]) {
            return None;
        }
        let (left, right) = (x.max(0.0), right.min(self.width));
        let (top, bottom) = (y.max(0.0), bottom.min(self.height));
        (right > left && bottom > top).then(|| [left, self.y(bottom), right, self.y(top)])
    }

    /// Adds an invisible clickable area for an external URI. Invalid or excessive links are ignored.
    pub fn link_uri(&mut self, x: f32, y: f32, w: f32, h: f32, uri: &str) {
        let allowed = uri
            .split_once(':')
            .is_some_and(|(scheme, rest)| matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https" | "mailto") && !rest.is_empty());
        if !allowed || uri.len() > MAX_LINK_URI_BYTES || uri.chars().any(char::is_control) {
            return;
        }
        if let Some(rect) = self.link_rect(x, y, w, h) {
            self.links.push(LinkAnnotation { rect, target: LinkTarget::Uri(uri.to_string()) });
        }
    }

    /// Adds a link to a location on another page of this document (zero-based page index).
    pub fn link_page(&mut self, x: f32, y: f32, w: f32, h: f32, page: usize, target_x: f32, target_y: f32) {
        if !finite(&[target_x, target_y]) {
            return;
        }
        if let Some(rect) = self.link_rect(x, y, w, h) {
            self.links.push(LinkAnnotation { rect, target: LinkTarget::Page { index: page, x: target_x, y: target_y } });
        }
    }

    /// Saves the graphics state (pair with [`Page::restore`]).
    pub fn save(&mut self) {
        self.depth = self.depth.saturating_add(1);
        self.rec(Op::Save);
        self.content.push_str("q\n");
    }
    /// Restores the graphics state saved by [`Page::save`]; extra calls are ignored.
    pub fn restore(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
            self.rec(Op::Restore);
            self.content.push_str("Q\n");
        }
    }
    /// Intersects the clip with a rectangle (until the next [`Page::restore`]).
    pub fn clip_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.rec(Op::Clip { x, y, w: w.max(0.0), h: h.max(0.0) });
        let s = format!("{} {} {} {} re W n", n(x), n(self.y(y + h)), n(w.max(0.0)), n(h.max(0.0)));
        self.op(&s);
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Rgb) {
        if !(extent(w) && extent(h) && finite(&[x, y])) {
            return;
        }
        self.rec(Op::FillRect { x, y, w, h, color });
        let s = format!("{}\n{} {} {} {} re f", rgb_op(color, false), n(x), n(self.y(y + h)), n(w), n(h));
        self.op(&s);
    }

    pub fn rect_stroke(&mut self, x: f32, y: f32, w: f32, h: f32, width: f32, color: Rgb) {
        if !finite(&[x, y, w, h]) {
            return;
        }
        self.rec(Op::StrokeRect { x, y, w, h, width: stroke_width(width), color });
        let s = format!("{}\n{} w [] 0 d\n{} {} {} {} re S", rgb_op(color, true), n(stroke_width(width)), n(x), n(self.y(y + h)), n(w), n(h));
        self.op(&s);
    }

    pub fn stroke_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, color: Rgb) {
        self.stroke_line_dashed(x0, y0, x1, y1, width, color, &[]);
    }

    /// A line with a dash pattern (`[on, off, …]` in points; empty = solid).
    pub fn stroke_line_dashed(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, color: Rgb, dash: &[f32]) {
        if !finite(&[x0, y0, x1, y1]) {
            return;
        }
        self.rec(Op::Line { pts: vec![[x0, y0], [x1, y1]], width: stroke_width(width), color, dash: dash.to_vec() });
        let s = format!(
            "{}\n{} w [{}] 0 d\n{} {} m {} {} l S",
            rgb_op(color, true),
            n(stroke_width(width)),
            dash_values(dash),
            n(x0),
            n(self.y(y0)),
            n(x1),
            n(self.y(y1))
        );
        self.op(&s);
    }

    /// An open polyline.
    pub fn polyline(&mut self, pts: &[[f32; 2]], width: f32, color: Rgb, dash: &[f32]) {
        if pts.len() < 2 || !pts.iter().all(|p| finite(p)) {
            return;
        }
        self.rec(Op::Line { pts: pts.to_vec(), width: stroke_width(width), color, dash: dash.to_vec() });
        let mut s = format!("{}\n{} w [{}] 0 d 1 j\n", rgb_op(color, true), n(stroke_width(width)), dash_values(dash));
        self.path(&mut s, pts, false);
        s.push_str(" S");
        self.op(&s);
    }

    fn path(&self, s: &mut String, pts: &[[f32; 2]], close: bool) {
        for (i, p) in pts.iter().enumerate() {
            let _ = write!(s, "{} {} {} ", n(p[0]), n(self.y(p[1])), if i == 0 { "m" } else { "l" });
        }
        if close {
            s.push('h');
        }
    }

    /// A closed polygon, filled and/or stroked.
    pub fn polygon(&mut self, pts: &[[f32; 2]], fill: Option<Rgb>, stroke: Option<(Rgb, f32)>) {
        if pts.len() < 2 || (fill.is_none() && stroke.is_none()) || !pts.iter().all(|p| finite(p)) {
            return;
        }
        self.rec(Op::Polygon { pts: pts.to_vec(), fill, stroke: stroke.map(|(c, w)| (c, stroke_width(w))) });
        let mut s = String::new();
        if let Some(f) = fill {
            s.push_str(&rgb_op(f, false));
            s.push('\n');
        }
        if let Some((c, w)) = stroke {
            let _ = writeln!(s, "{}\n{} w [] 0 d 1 j", rgb_op(c, true), n(stroke_width(w)));
        }
        self.path(&mut s, pts, true);
        s.push_str(match (fill.is_some(), stroke.is_some()) {
            (true, true) => " B",
            (true, false) => " f",
            _ => " S",
        });
        self.op(&s);
    }

    /// A circle approximated by four cubic Béziers.
    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, fill: Option<Rgb>, stroke: Option<(Rgb, f32)>) {
        if !extent(r) || !finite(&[cx, cy]) || (fill.is_none() && stroke.is_none()) {
            return;
        }
        self.rec(Op::Circle { cx, cy, r, fill, stroke: stroke.map(|(c, w)| (c, stroke_width(w))) });
        const K: f32 = 0.552_284_8;
        let k = r * K;
        let y = self.y(cy);
        let mut s = String::new();
        if let Some(f) = fill {
            s.push_str(&rgb_op(f, false));
            s.push('\n');
        }
        if let Some((c, w)) = stroke {
            let _ = writeln!(s, "{}\n{} w [] 0 d", rgb_op(c, true), n(stroke_width(w)));
        }
        let _ = write!(
            s,
            "{} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c h",
            n(cx + r),
            n(y),
            n(cx + r),
            n(y + k),
            n(cx + k),
            n(y + r),
            n(cx),
            n(y + r),
            n(cx - k),
            n(y + r),
            n(cx - r),
            n(y + k),
            n(cx - r),
            n(y),
            n(cx - r),
            n(y - k),
            n(cx - k),
            n(y - r),
            n(cx),
            n(y - r),
            n(cx + k),
            n(y - r),
            n(cx + r),
            n(y - k),
            n(cx + r),
            n(y)
        );
        s.push_str(match (fill.is_some(), stroke.is_some()) {
            (true, true) => " B",
            (true, false) => " f",
            _ => " S",
        });
        self.op(&s);
    }

    /// Text with its baseline starting at `(x, y)`.
    pub fn text(&mut self, x: f32, y: f32, size: f32, font: Font, color: Rgb, text: &str) {
        self.text_rotated(x, y, size, font, color, text, 0.0);
    }

    /// Text rotated by `angle` radians (clockwise on the page, like the y-down coordinate
    /// system) about its baseline origin.
    pub fn text_rotated(&mut self, x: f32, y: f32, size: f32, font: Font, color: Rgb, text: &str, angle: f32) {
        if text.is_empty() || !extent(size) || !finite(&[x, y]) {
            return;
        }
        let (family, bold, italic) = font.style();
        let angle_cw = if angle.is_finite() { angle } else { 0.0 };
        self.rec(Op::Text(TextOp {
            x,
            y,
            size: size.min(10_000.0),
            angle: angle_cw,
            color,
            text: text.to_string(),
            family: family.to_string(),
            bold,
            italic,
        }));
        let fi = font.index();
        let size = n(size.min(10_000.0));
        let (sn, cs) = if angle.is_finite() { angle.sin_cos() } else { (0.0, 1.0) };
        // Clockwise in y-down = counter-clockwise negated in PDF's y-up space.
        let mut s = format!("BT\n{}\n{} {} {} {} {} {} Tm\n", rgb_op(color, false), n(cs), n(-sn), n(sn), n(cs), n(x), n(self.y(y)));
        // Runs of WinAnsi text use `font`; runs it cannot encode use the CID font. Each `Tj`
        // advances the text position, so the runs follow each other on the baseline.
        let mut rest = text;
        while let Some(first) = rest.chars().next() {
            let cid = needs_cid(first);
            let end = rest.char_indices().find(|(_, c)| needs_cid(*c) != cid).map_or(rest.len(), |(i, _)| i);
            let (run, tail) = rest.split_at(end);
            let (slot, string) = if cid { (CID_FONT, pdf_utf16_string(run)) } else { (fi, pdf_string(run)) };
            if let Some(f) = self.fonts.get_mut(slot) {
                *f = true;
            }
            let _ = writeln!(s, "/F{} {} Tf\n{} Tj", slot + 1, size, string);
            rest = tail;
        }
        s.push_str("ET");
        self.op(&s);
    }

    /// Text in an embedded TrueType face, its baseline starting at `(x, y)`. Characters the face
    /// has no glyph for show its missing-glyph box; `synthetic` slants or thickens the text.
    pub fn text_embedded(&mut self, x: f32, y: f32, size: f32, font: &Arc<EmbeddedFont>, synthetic: Synthetic, color: Rgb, text: &str) {
        if text.is_empty() || !extent(size) || !finite(&[x, y]) || self.full() {
            return;
        }
        self.rec(Op::Text(TextOp {
            x,
            y,
            size: size.min(10_000.0),
            angle: 0.0,
            color,
            text: text.to_string(),
            family: font.family().to_string(),
            bold: font.is_bold() || synthetic.bold,
            italic: font.is_italic() || synthetic.italic,
        }));
        let at = match self.embedded.iter().position(|(f, _)| f.id == font.id) {
            Some(i) => i,
            None => {
                self.embedded.push((font.clone(), BTreeMap::new()));
                self.embedded.len() - 1
            }
        };
        let mut hex = String::with_capacity(text.len() * 4);
        if let Some((_, used)) = self.embedded.get_mut(at) {
            for c in text.chars() {
                let g = font.glyph(c);
                used.entry(g).or_insert(c);
                let _ = write!(hex, "{g:04X}");
            }
        }
        let size = size.min(10_000.0);
        let mut s = format!("q\nBT\n{}\n", rgb_op(color, false));
        if synthetic.bold {
            let _ = writeln!(s, "{}\n{} w\n2 Tr", rgb_op(color, true), n(size / 36.0));
        }
        let skew = if synthetic.italic { 0.333 } else { 0.0 };
        let _ = write!(s, "1 0 {} 1 {} {} Tm\n/G{} {} Tf\n<{hex}> Tj\nET\nQ", n(skew), n(x), n(self.y(y)), font.id, n(size));
        self.op(&s);
    }

    /// Draws an RGBA image (`pw`×`ph` pixels, row-major, straight alpha) into the box
    /// `(x, y, w, h)`. Malformed or oversized buffers are ignored.
    pub fn image_rgba(&mut self, x: f32, y: f32, w: f32, h: f32, rgba: &[u8], pw: u32, ph: u32) {
        let px = pw as u64 * ph as u64;
        if pw == 0 || ph == 0 || px > MAX_IMAGE_PIXELS || (rgba.len() as u64) < px * 4 || !(extent(w) && extent(h) && finite(&[x, y])) {
            return;
        }
        let px = px as usize;
        if let Some(ops) = &self.ops {
            let bytes = px * 4;
            let room = ops.len() < MAX_PAGE_OPS && self.recorded_image_bytes.saturating_add(bytes) <= MAX_RECORDED_IMAGE_BYTES;
            if !room {
                // Over the budget: the picture's place is outlined, and neither the preview nor
                // the recording document's PDF copies its pixels.
                self.rec(Op::StrokeRect { x, y, w, h, width: 0.5, color: [160, 160, 160] });
                return;
            }
            if !self.full() {
                self.recorded_image_bytes += bytes;
                let rgba: Arc<[u8]> = rgba.get(..bytes).unwrap_or(rgba).into();
                self.rec(Op::Image { x, y, w, h, rgba, pw, ph });
            }
        }
        let mut rgb = Vec::with_capacity(px * 3);
        let mut alpha = Vec::with_capacity(px);
        for p in rgba.as_chunks::<4>().0.iter().take(px) {
            rgb.extend_from_slice(&[p[0], p[1], p[2]]);
            alpha.push(p[3]);
        }
        let alpha = if alpha.iter().all(|a| *a == 255) { None } else { Some(alpha) };
        let idx = self.images.len();
        self.images.push(ImageData { width: pw, height: ph, rgb, alpha });
        let s = format!("q {} 0 0 {} {} {} cm /Im{} Do Q", n(w), n(h), n(x), n(self.y(y + h)), idx + 1);
        self.op(&s);
    }
}

/// Document metadata.
#[derive(Clone, Debug, Default)]
pub struct Info {
    pub title: String,
    pub author: String,
    pub creator: String,
}

/// A PDF document under construction.
#[derive(Default)]
pub struct PdfDoc {
    pages: Vec<Page>,
    pub info: Info,
    /// Pages keep what they draw ([`Page::ops`]).
    record: bool,
}

/// PackBits-style run-length encoding (PDF `RunLengthDecode`).
fn run_length(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2 + 16);
    let mut i = 0;
    while i < data.len() {
        let b = data[i];
        let mut run = 1;
        while i + run < data.len() && run < 128 && data[i + run] == b {
            run += 1;
        }
        if run >= 2 {
            out.push((257 - run) as u8);
            out.push(b);
            i += run;
            continue;
        }
        // Literal run until the next repeat of 3+ or 128 bytes.
        let start = i;
        let mut len = 0;
        while i < data.len() && len < 128 {
            if i + 2 < data.len() && data[i] == data[i + 1] && data[i] == data[i + 2] {
                break;
            }
            i += 1;
            len += 1;
        }
        if len == 0 {
            // Next is a repeat; loop will handle it.
            continue;
        }
        out.push((len - 1) as u8);
        out.extend_from_slice(&data[start..start + len]);
    }
    out.push(128);
    out
}

/// The six capital letters that mark a subset font's name (ISO 32000-1 9.6.4), from its glyphs.
fn subset_tag(id: u32, gids: &BTreeSet<u16>) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ u64::from(id);
    for g in gids {
        h ^= u64::from(*g);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    (0..6).map(|i| (b'A' + ((h >> (i * 7)) % 26) as u8) as char).collect()
}

/// Longest Info string written, in characters; longer values are cut on a character boundary.
const MAX_INFO_CHARS: usize = 1024;

/// A PDF text string (document Info entries) for `text`: a literal `( … )` for printable ASCII,
/// otherwise UTF-16BE with a byte-order mark as a hex string (ISO 32000-1 7.9.2.2).
fn pdf_text_string(text: &str) -> String {
    let text: String = text.chars().take(MAX_INFO_CHARS).collect();
    if text.chars().all(|c| matches!(c, ' '..='~')) {
        return pdf_string(&text);
    }
    let mut s = String::with_capacity(text.len() * 4 + 6);
    s.push_str("<FEFF");
    for unit in text.encode_utf16() {
        let _ = write!(s, "{unit:04X}");
    }
    s.push('>');
    s
}

impl PdfDoc {
    pub fn new() -> PdfDoc {
        PdfDoc::default()
    }

    /// A document whose pages also keep what they draw, for a preview ([`Page::ops`]).
    pub fn recording() -> PdfDoc {
        PdfDoc { record: true, ..PdfDoc::default() }
    }

    /// Adds a page of `width_pt` × `height_pt` points and returns it for drawing.
    pub fn add_page(&mut self, width_pt: f32, height_pt: f32) -> &mut Page {
        let width = if width_pt.is_finite() { width_pt.clamp(3.0, 14_400.0) } else { 612.0 };
        let height = if height_pt.is_finite() { height_pt.clamp(3.0, 14_400.0) } else { 792.0 };
        self.pages.push(Page {
            width,
            height,
            content: String::new(),
            fonts: [false; FONT_SLOTS],
            embedded: Vec::new(),
            images: Vec::new(),
            links: Vec::new(),
            depth: 0,
            ops: self.record.then(Vec::new),
            recorded_image_bytes: 0,
        });
        let i = self.pages.len() - 1;
        &mut self.pages[i]
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// The page at `i`.
    pub fn page(&self, i: usize) -> Option<&Page> {
        self.pages.get(i)
    }

    /// The page at `i`, to keep drawing on it.
    pub fn page_mut(&mut self, i: usize) -> Option<&mut Page> {
        self.pages.get_mut(i)
    }

    /// Serializes the document. A document without pages gets one blank Letter page.
    pub fn finish(mut self) -> Vec<u8> {
        if self.pages.is_empty() {
            self.add_page(612.0, 792.0);
        }
        // Object numbering: 1 catalog, 2 page tree, 3 info, then fonts, then per page.
        let mut used = [false; FONT_SLOTS];
        for p in &self.pages {
            for (u, f) in used.iter_mut().zip(p.fonts.iter()) {
                *u |= *f;
            }
        }
        let mut next = 4u32;
        let mut font_ids = [0u32; FONT_SLOTS];
        for (i, u) in used.iter().enumerate() {
            if *u {
                font_ids[i] = next;
                // The CID font also needs its descendant font and font descriptor.
                next += if i == CID_FONT { 3 } else { 1 };
            }
        }
        let mut objects: Vec<(u32, Vec<u8>)> = Vec::new();
        let mut kids = Vec::new();
        let mut annotations = Vec::new();
        let page_count = self.pages.len();
        let mut font_dict = String::new();
        for (i, id) in font_ids.iter().enumerate() {
            if *id != 0 {
                let _ = write!(font_dict, "/F{} {} 0 R ", i + 1, id);
                if i == CID_FONT {
                    let (cid_id, desc_id) = (id + 1, id + 2);
                    objects.push((
                        *id,
                        format!(
                            "<< /Type /Font /Subtype /Type0 /BaseFont /{CID_BASE_FONT} /Encoding /UniGB-UTF16-H /DescendantFonts [{cid_id} 0 R] >>"
                        )
                        .into_bytes(),
                    ));
                    objects.push((
                        cid_id,
                        format!(
                            "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /{CID_BASE_FONT} /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 4 >> /FontDescriptor {desc_id} 0 R /DW {CID_WIDTH} >>"
                        )
                        .into_bytes(),
                    ));
                    objects.push((
                        desc_id,
                        format!(
                            "<< /Type /FontDescriptor /FontName /{CID_BASE_FONT} /Flags 6 /FontBBox [-25 -254 1000 880] /ItalicAngle 0 /Ascent 880 /Descent -120 /CapHeight 880 /StemV 93 >>"
                        )
                        .into_bytes(),
                    ));
                    continue;
                }
                let name = Font::ALL.get(i).copied().unwrap_or_default().base_name();
                let enc = if name.starts_with("Symbol") { "" } else { " /Encoding /WinAnsiEncoding" };
                objects.push((*id, format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name}{enc} >>").into_bytes()));
            }
        }
        // Embedded faces: a Type0 font over a CIDFontType2 with glyph ids as character codes, its
        // descriptor, the subset font program and a ToUnicode map, holding what every page used.
        let mut faces: Vec<(Arc<EmbeddedFont>, BTreeMap<u16, char>)> = Vec::new();
        for p in &self.pages {
            for (font, used) in &p.embedded {
                match faces.iter_mut().find(|(f, _)| f.id == font.id) {
                    Some((_, all)) => {
                        for (g, c) in used {
                            all.entry(*g).or_insert(*c);
                        }
                    }
                    None => faces.push((font.clone(), used.clone())),
                }
            }
        }
        for (font, used) in &faces {
            let gids: BTreeSet<u16> = used.keys().copied().collect();
            let (type0, cid, desc, file, tu) = (next, next + 1, next + 2, next + 3, next + 4);
            next += 5;
            let base = format!("{}+{}", subset_tag(font.id, &gids), font.ps_name);
            let widths: String = gids.iter().map(|g| format!("{g} [{}] ", font.per_mille(font.advance(*g) as f32))).collect();
            let _ = write!(font_dict, "/G{} {type0} 0 R ", font.id);
            objects.push((
                type0,
                format!("<< /Type /Font /Subtype /Type0 /BaseFont /{base} /Encoding /Identity-H /DescendantFonts [{cid} 0 R] /ToUnicode {tu} 0 R >>")
                    .into_bytes(),
            ));
            objects.push((
                cid,
                format!(
                    "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{base} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {desc} 0 R /W [{widths}] /CIDToGIDMap /Identity >>"
                )
                .into_bytes(),
            ));
            // A program that can't be cut (a damaged face) leaves the font for the viewer to substitute.
            let program = font.subset(&gids).unwrap_or_default();
            let file_ref = if program.is_empty() { String::new() } else { format!(" /FontFile2 {file} 0 R") };
            objects.push((desc, format!("<< {}{file_ref} >>", font.descriptor(&base)).into_bytes()));
            let mut o = format!("<< /Length {} /Length1 {} >>\nstream\n", program.len(), program.len()).into_bytes();
            o.extend_from_slice(&program);
            o.extend_from_slice(b"\nendstream");
            objects.push((file, o));
            let cmap = truetype::to_unicode(used);
            let mut o = format!("<< /Length {} >>\nstream\n", cmap.len()).into_bytes();
            o.extend_from_slice(cmap.as_bytes());
            o.extend_from_slice(b"\nendstream");
            objects.push((tu, o));
        }
        for page in &mut self.pages {
            while page.depth > 0 {
                page.restore();
            }
            let mut xobjects = String::new();
            for (k, img) in page.images.iter().enumerate() {
                let img_id = next;
                next += 1;
                let smask = match &img.alpha {
                    Some(a) => {
                        let id = next;
                        next += 1;
                        let data = run_length(a);
                        let mut o = format!(
                            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /RunLengthDecode /Length {} >>\nstream\n",
                            img.width,
                            img.height,
                            data.len()
                        )
                        .into_bytes();
                        o.extend_from_slice(&data);
                        o.extend_from_slice(b"\nendstream");
                        objects.push((id, o));
                        format!(" /SMask {id} 0 R")
                    }
                    None => String::new(),
                };
                let data = run_length(&img.rgb);
                let mut o = format!(
                    "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8{} /Filter /RunLengthDecode /Length {} >>\nstream\n",
                    img.width,
                    img.height,
                    smask,
                    data.len()
                )
                .into_bytes();
                o.extend_from_slice(&data);
                o.extend_from_slice(b"\nendstream");
                objects.push((img_id, o));
                let _ = write!(xobjects, "/Im{} {} 0 R ", k + 1, img_id);
            }
            let content_id = next;
            let page_id = next + 1;
            next += 2;
            let body = std::mem::take(&mut page.content);
            let mut o = format!("<< /Length {} >>\nstream\n", body.len()).into_bytes();
            o.extend_from_slice(body.as_bytes());
            o.extend_from_slice(b"endstream");
            objects.push((content_id, o));
            let xo = if xobjects.is_empty() { String::new() } else { format!(" /XObject << {xobjects}>>") };
            let mut link_refs = String::new();
            for link in std::mem::take(&mut page.links) {
                if matches!(&link.target, LinkTarget::Page { index, .. } if *index >= page_count) {
                    continue;
                }
                let id = next;
                next += 1;
                let _ = write!(link_refs, "{id} 0 R ");
                annotations.push((id, link));
            }
            let annots = if link_refs.is_empty() { String::new() } else { format!(" /Annots [{link_refs}]") };
            objects.push((
                page_id,
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /ProcSet [/PDF /Text /ImageB /ImageC] /Font << {}>>{} >> /Contents {} 0 R{} >>",
                    n(page.width),
                    n(page.height),
                    font_dict,
                    xo,
                    content_id,
                    annots
                )
                .into_bytes(),
            ));
            kids.push(page_id);
        }
        for (id, link) in annotations {
            let [x0, y0, x1, y1] = link.rect;
            let action = match link.target {
                LinkTarget::Uri(uri) => format!("/A << /S /URI /URI {} >>", pdf_uri_string(&uri)),
                LinkTarget::Page { index, x, y } => {
                    let Some(&page_id) = kids.get(index) else { continue };
                    let height = self.pages.get(index).map_or(0.0, |p| p.height);
                    format!("/Dest [{page_id} 0 R /XYZ {} {} null]", n(x), n(height - y))
                }
            };
            objects.push((
                id,
                format!("<< /Type /Annot /Subtype /Link /Rect [{} {} {} {}] /Border [0 0 0] {action} >>", n(x0), n(y0), n(x1), n(y1)).into_bytes(),
            ));
        }
        objects.push((1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()));
        let kids_s: Vec<String> = kids.iter().map(|k| format!("{k} 0 R")).collect();
        objects.push((2, format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids_s.join(" "), kids.len()).into_bytes()));
        let creator = if self.info.creator.is_empty() { "GridCraft" } else { &self.info.creator };
        let mut info = format!("<< /Producer {} /Creator {}", pdf_text_string("GridCraft PDF"), pdf_text_string(creator));
        if !self.info.title.is_empty() {
            let _ = write!(info, " /Title {}", pdf_text_string(&self.info.title));
        }
        if !self.info.author.is_empty() {
            let _ = write!(info, " /Author {}", pdf_text_string(&self.info.author));
        }
        info.push_str(" >>");
        objects.push((3, info.into_bytes()));
        objects.sort_by_key(|(id, _)| *id);

        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
        let mut offsets = vec![0usize; next as usize];
        for (id, body) in &objects {
            if let Some(slot) = offsets.get_mut(*id as usize) {
                *slot = out.len();
            }
            out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        let mut x = format!("xref\n0 {}\n0000000000 65535 f \n", next);
        for off in offsets.iter().skip(1) {
            let _ = writeln!(x, "{off:010} 00000 n ");
        }
        let _ = write!(x, "trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{}\n%%EOF\n", next, xref);
        out.extend_from_slice(x.as_bytes());
        out
    }
}

#[cfg(test)]
mod tests;
