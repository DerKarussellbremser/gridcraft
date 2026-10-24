//! TrueType fonts embedded in the PDF: the few tables a PDF needs (names, character map, advance
//! widths, glyph outlines) read from a font file, and the subset of glyphs a document uses written
//! back as a font program (ISO 32000-1 9.7.4 CIDFontType2, 9.9 FontFile2). Hand-written so the
//! crate keeps no dependencies; every offset read from the file is checked.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

type Dir = Vec<([u8; 4], usize, usize)>;

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*d.get(o)?, *d.get(o.checked_add(1)?)?]))
}
fn i16_at(d: &[u8], o: usize) -> Option<i16> {
    u16_at(d, o).map(|v| v as i16)
}
fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*d.get(o)?, *d.get(o.checked_add(1)?)?, *d.get(o.checked_add(2)?)?, *d.get(o.checked_add(3)?)?]))
}

/// Where face `index` starts: 0 for a plain font file, its entry for a collection (`ttcf`).
fn face_offset(data: &[u8], index: u32) -> Option<usize> {
    if data.get(0..4)? == b"ttcf" {
        let count = u32_at(data, 8)?;
        if index >= count {
            return None;
        }
        Some(u32_at(data, 12usize.checked_add(4 * index as usize)?)? as usize)
    } else {
        (index == 0).then_some(0)
    }
}

/// Number of faces in a font file (1 for a plain file, at most 64 read from a collection).
pub fn face_count(data: &[u8]) -> u32 {
    if data.get(0..4) == Some(b"ttcf") { u32_at(data, 8).unwrap_or(0).min(64) } else { 1 }
}

/// The table directory of the face at `off`: TrueType outlines only (`OTTO`, CFF, is refused).
fn directory(data: &[u8], off: usize) -> Option<Dir> {
    let version = data.get(off..off.checked_add(4)?)?;
    if version != [0, 1, 0, 0] && version != b"true" {
        return None;
    }
    let n = u16_at(data, off + 4)? as usize;
    let mut dir = Vec::with_capacity(n.min(64));
    for i in 0..n.min(64) {
        let r = off + 12 + 16 * i;
        let tag: [u8; 4] = data.get(r..r + 4)?.try_into().ok()?;
        let (o, l) = (u32_at(data, r + 8)? as usize, u32_at(data, r + 12)? as usize);
        if o.checked_add(l).is_some_and(|end| end <= data.len()) {
            dir.push((tag, o, l));
        }
    }
    Some(dir)
}

fn table<'a>(data: &'a [u8], dir: &Dir, tag: &[u8; 4]) -> Option<&'a [u8]> {
    dir.iter().find(|t| &t.0 == tag).and_then(|t| data.get(t.1..t.1 + t.2))
}

/// The best English name with `id` from a `name` table: Windows Unicode US English first, then any
/// Windows Unicode name, then a Macintosh Roman one.
fn name(t: &[u8], id: u16) -> Option<String> {
    let count = u16_at(t, 2)? as usize;
    let strings = u16_at(t, 4)? as usize;
    let mut best: Option<(u8, String)> = None;
    for i in 0..count.min(2000) {
        let r = 6 + 12 * i;
        let (platform, encoding, language, nid) = (u16_at(t, r)?, u16_at(t, r + 2)?, u16_at(t, r + 4)?, u16_at(t, r + 6)?);
        if nid != id {
            continue;
        }
        let (len, at) = (u16_at(t, r + 8)? as usize, u16_at(t, r + 10)? as usize);
        let Some(bytes) = t.get(strings + at..strings + at + len) else { continue };
        let (rank, text) = match (platform, encoding) {
            (3, 1 | 10) => {
                let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|p| u16::from_be_bytes(*p)).collect();
                (if language == 0x409 { 0 } else { 1 }, String::from_utf16_lossy(&units))
            }
            (1, 0) => (2, bytes.iter().map(|&b| b as char).collect()),
            _ => continue,
        };
        if !text.is_empty() && best.as_ref().is_none_or(|(r, _)| rank < *r) {
            best = Some((rank, text));
        }
    }
    best.map(|(_, s)| s)
}

/// One face of a font file, as a font list sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceInfo {
    pub index: u32,
    /// Legacy family name (name ID 1): the family Excel styles bold and italic within, e.g.
    /// `Arial Black`, which has one face only.
    pub family: String,
    /// Typographic family (name ID 16), when it differs (`Arial` for Arial Black).
    pub typographic_family: Option<String>,
    /// The face is the family's bold / italic one (OS/2 `fsSelection`, else `head.macStyle`).
    pub bold: bool,
    pub italic: bool,
}

/// The TrueType faces of a font file; other files and damaged faces give nothing.
pub fn faces(data: &[u8]) -> Vec<FaceInfo> {
    (0..face_count(data))
        .filter_map(|index| {
            let off = face_offset(data, index)?;
            let dir = directory(data, off)?;
            let names = table(data, &dir, b"name")?;
            let family = name(names, 1)?;
            let typographic_family = name(names, 16).filter(|t| *t != family);
            let (bold, italic) = match table(data, &dir, b"OS/2").and_then(|t| u16_at(t, 62)) {
                Some(sel) => (sel & 0x20 != 0, sel & 0x201 != 0),
                None => {
                    let mac = table(data, &dir, b"head").and_then(|t| u16_at(t, 44)).unwrap_or(0);
                    (mac & 1 != 0, mac & 2 != 0)
                }
            };
            Some(FaceInfo { index, family, typographic_family, bold, italic })
        })
        .collect()
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

/// A TrueType face ready to set text in and to embed: shared between the pages that use it.
#[derive(Debug)]
pub struct EmbeddedFont {
    pub(crate) id: u32,
    data: Arc<[u8]>,
    dir: Dir,
    /// Absolute offset and format of the character map subtable.
    cmap: Option<(usize, u16)>,
    units_per_em: u16,
    num_glyphs: u16,
    num_hmetrics: u16,
    loca_long: bool,
    pub(crate) ps_name: String,
    /// Legacy family name (name ID 1) and whether this is its bold / italic face.
    family: String,
    bold: bool,
    pub(crate) italic: bool,
    ascent: i16,
    descent: i16,
    cap_height: i16,
    bbox: [i16; 4],
    italic_angle: f32,
}

impl EmbeddedFont {
    /// Reads face `index` of a font file. Fonts with CFF outlines, damaged tables or no Unicode
    /// character map give `None`.
    pub fn parse(data: Arc<[u8]>, index: u32) -> Option<Arc<EmbeddedFont>> {
        let off = face_offset(&data, index)?;
        let dir = directory(&data, off)?;
        let head = table(&data, &dir, b"head")?;
        let hhea = table(&data, &dir, b"hhea")?;
        let maxp = table(&data, &dir, b"maxp")?;
        table(&data, &dir, b"glyf")?;
        table(&data, &dir, b"loca")?;
        table(&data, &dir, b"hmtx")?;
        let units_per_em = u16_at(head, 18).filter(|u| (16..=16384).contains(u))?;
        let cmap = table(&data, &dir, b"cmap").and_then(|t| pick_cmap(&data, &dir, t));
        cmap?;
        let os2 = table(&data, &dir, b"OS/2");
        let names = table(&data, &dir, b"name");
        let ps_name: String = names
            .and_then(|t| name(t, 6))
            .unwrap_or_else(|| "Embedded".into())
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .take(60)
            .collect();
        let italic = os2.and_then(|t| u16_at(t, 62)).map_or(u16_at(head, 44).unwrap_or(0) & 2 != 0, |sel| sel & 0x201 != 0);
        let bold = os2.and_then(|t| u16_at(t, 62)).map_or(u16_at(head, 44).unwrap_or(0) & 1 != 0, |sel| sel & 0x20 != 0);
        let family: String = names.and_then(|t| name(t, 1)).unwrap_or_default().chars().take(128).collect();
        let ascent = i16_at(hhea, 4)?;
        let cap_height = os2.filter(|t| u16_at(t, 0).is_some_and(|v| v >= 2)).and_then(|t| i16_at(t, 88)).unwrap_or((ascent as f32 * 0.7) as i16);
        let italic_angle = table(&data, &dir, b"post").and_then(|t| u32_at(t, 4)).map_or(0.0, |f| (f as i32) as f32 / 65536.0);
        let font = EmbeddedFont {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            num_glyphs: u16_at(maxp, 4)?,
            num_hmetrics: u16_at(hhea, 34)?.max(1),
            loca_long: i16_at(head, 50)? == 1,
            bbox: [i16_at(head, 36)?, i16_at(head, 38)?, i16_at(head, 40)?, i16_at(head, 42)?],
            descent: i16_at(hhea, 6)?,
            ps_name: if ps_name.is_empty() { "Embedded".into() } else { ps_name },
            units_per_em,
            cmap,
            family,
            bold,
            italic,
            ascent,
            cap_height,
            italic_angle: if italic_angle.is_finite() { italic_angle.clamp(-60.0, 60.0) } else { 0.0 },
            dir,
            data,
        };
        Some(Arc::new(font))
    }

    /// The family this face belongs to (name ID 1: `Arial` for Arial Bold, `Arial Black`).
    pub fn family(&self) -> &str {
        &self.family
    }
    /// Whether this is its family's bold / italic face.
    pub fn is_bold(&self) -> bool {
        self.bold
    }
    pub fn is_italic(&self) -> bool {
        self.italic
    }

    /// The glyph for `c` (0, the missing-glyph box, when the face has none).
    pub fn glyph(&self, c: char) -> u16 {
        let Some((at, format)) = self.cmap else { return 0 };
        let g = match format {
            4 => cmap4(&self.data, at, c as u32),
            12 => cmap12(&self.data, at, c as u32),
            _ => None,
        };
        g.filter(|g| *g < self.num_glyphs).unwrap_or(0)
    }

    /// Advance width of a glyph in font units.
    pub fn advance(&self, gid: u16) -> u16 {
        let Some(hmtx) = table(&self.data, &self.dir, b"hmtx") else { return 0 };
        let i = gid.min(self.num_hmetrics - 1) as usize;
        u16_at(hmtx, 4 * i).unwrap_or(0)
    }

    /// Width of `text` in points at `size` points.
    pub fn text_width(&self, size: f32, text: &str) -> f32 {
        let units: u64 = text.chars().map(|c| u64::from(self.advance(self.glyph(c)))).sum();
        units as f32 * size / self.units_per_em as f32
    }

    /// Whether the face has a glyph of its own for every character of `text`.
    pub fn covers(&self, text: &str) -> bool {
        text.chars().all(|c| c.is_control() || self.glyph(c) != 0)
    }

    pub(crate) fn per_mille(&self, v: f32) -> i32 {
        (v * 1000.0 / self.units_per_em as f32).round() as i32
    }

    /// The FontDescriptor entries other than the font program, in 1/1000 em.
    pub(crate) fn descriptor(&self, base: &str) -> String {
        let [x0, y0, x1, y1] = self.bbox.map(|v| self.per_mille(v as f32));
        format!(
            "/Type /FontDescriptor /FontName /{base} /Flags {} /FontBBox [{x0} {y0} {x1} {y1}] /ItalicAngle {} /Ascent {} /Descent {} /CapHeight {} /StemV 80",
            if self.italic { 32 + 64 } else { 32 },
            crate::n(self.italic_angle),
            self.per_mille(self.ascent as f32),
            self.per_mille(self.descent as f32),
            self.per_mille(self.cap_height as f32),
        )
    }

    /// The `[start, end)` bytes of a glyph's outline in `glyf`.
    fn glyph_range(&self, gid: u16) -> Option<(usize, usize)> {
        let loca = table(&self.data, &self.dir, b"loca")?;
        let i = gid as usize;
        let (a, b) = if self.loca_long {
            (u32_at(loca, 4 * i)? as usize, u32_at(loca, 4 * i + 4)? as usize)
        } else {
            (u16_at(loca, 2 * i)? as usize * 2, u16_at(loca, 2 * i + 2)? as usize * 2)
        };
        (a <= b).then_some((a, b))
    }

    /// A font program holding only the outlines of `used` (and the glyphs their composites are
    /// built from), keeping every glyph's number so the text's glyph ids stay valid.
    pub(crate) fn subset(&self, used: &BTreeSet<u16>) -> Option<Vec<u8>> {
        let glyf = table(&self.data, &self.dir, b"glyf")?;
        let mut keep: BTreeSet<u16> = used.iter().copied().filter(|g| *g < self.num_glyphs).collect();
        keep.insert(0);
        let mut stack: Vec<u16> = keep.iter().copied().collect();
        let mut steps = 0usize;
        while let Some(g) = stack.pop() {
            steps += 1;
            if steps > 100_000 {
                break;
            }
            let Some((a, b)) = self.glyph_range(g) else { continue };
            let Some(outline) = glyf.get(a..b) else { continue };
            if outline.len() < 10 || i16_at(outline, 0).unwrap_or(0) >= 0 {
                continue;
            }
            // A composite glyph: its components follow the header (glyf §: flags, glyph index,
            // arguments, transform).
            let mut pos = 10;
            for _ in 0..1000 {
                let (Some(flags), Some(component)) = (u16_at(outline, pos), u16_at(outline, pos + 2)) else { break };
                if component < self.num_glyphs && keep.insert(component) {
                    stack.push(component);
                }
                pos += 4 + if flags & 0x0001 != 0 { 4 } else { 2 };
                pos += if flags & 0x0008 != 0 {
                    2
                } else if flags & 0x0040 != 0 {
                    4
                } else if flags & 0x0080 != 0 {
                    8
                } else {
                    0
                };
                if flags & 0x0020 == 0 {
                    break;
                }
            }
        }
        let mut new_glyf: Vec<u8> = Vec::new();
        let mut new_loca: Vec<u8> = Vec::with_capacity((self.num_glyphs as usize + 1) * 4);
        for gid in 0..self.num_glyphs {
            new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
            if keep.contains(&gid)
                && let Some((a, b)) = self.glyph_range(gid)
                && let Some(outline) = glyf.get(a..b)
            {
                new_glyf.extend_from_slice(outline);
                while !new_glyf.len().is_multiple_of(4) {
                    new_glyf.push(0);
                }
            }
        }
        new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
        let mut head = table(&self.data, &self.dir, b"head")?.to_vec();
        head.get_mut(8..12)?.copy_from_slice(&[0; 4]);
        head.get_mut(50..52)?.copy_from_slice(&1i16.to_be_bytes());
        let copy = |tag: &[u8; 4]| table(&self.data, &self.dir, tag).map(<[u8]>::to_vec);
        // Sorted by tag, as the directory must be.
        let parts: Vec<(&[u8; 4], Vec<u8>)> = [
            (b"cvt ", copy(b"cvt ")),
            (b"fpgm", copy(b"fpgm")),
            (b"glyf", Some(new_glyf)),
            (b"head", Some(head)),
            (b"hhea", copy(b"hhea")),
            (b"hmtx", copy(b"hmtx")),
            (b"loca", Some(new_loca)),
            (b"maxp", copy(b"maxp")),
            (b"prep", copy(b"prep")),
        ]
        .into_iter()
        .filter_map(|(tag, data)| data.map(|d| (tag, d)))
        .collect();
        Some(sfnt(&parts))
    }
}

/// A font file of `tables` (sorted by tag) with its directory and checksums.
fn sfnt(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let n = tables.len() as u16;
    let pow = 1u16 << (15 - n.max(1).leading_zeros() as u16).min(15);
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 1, 0, 0]);
    for v in [n, pow * 16, pow.trailing_zeros() as u16, (n * 16).saturating_sub(pow * 16)] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    let mut offset = 12 + 16 * tables.len();
    for (tag, data) in tables {
        let sum = data.chunks(4).fold(0u32, |s, c| {
            let mut w = [0u8; 4];
            w[..c.len()].copy_from_slice(c);
            s.wrapping_add(u32::from_be_bytes(w))
        });
        out.extend_from_slice(&tag[..]);
        out.extend_from_slice(&sum.to_be_bytes());
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        offset += data.len().div_ceil(4) * 4;
    }
    for (_, data) in tables {
        out.extend_from_slice(data);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
    }
    out
}

/// The character map subtable to use: full Unicode (format 12) first, then BMP Unicode (format 4).
fn pick_cmap(data: &[u8], dir: &Dir, cmap: &[u8]) -> Option<(usize, u16)> {
    let base = dir.iter().find(|t| &t.0 == b"cmap")?.1;
    let count = u16_at(cmap, 2)? as usize;
    let mut best: Option<(u8, usize, u16)> = None;
    for i in 0..count.min(64) {
        let r = 4 + 8 * i;
        let (platform, encoding, off) = (u16_at(cmap, r)?, u16_at(cmap, r + 2)?, u32_at(cmap, r + 4)? as usize);
        let at = base.checked_add(off)?;
        let format = u16_at(data, at)?;
        let rank = match (platform, encoding, format) {
            (3, 10, 12) | (0, 4, 12) => 0,
            (3, 1, 4) | (0, _, 4) => 1,
            _ => continue,
        };
        if best.is_none_or(|(r, _, _)| rank < r) {
            best = Some((rank, at, format));
        }
    }
    best.map(|(_, at, format)| (at, format))
}

/// Format 4 lookup (segments of the BMP).
fn cmap4(d: &[u8], at: usize, c: u32) -> Option<u16> {
    if c > 0xFFFF {
        return None;
    }
    let segs = (u16_at(d, at + 6)? / 2) as usize;
    let ends = at + 14;
    let starts = ends + 2 * segs + 2;
    let deltas = starts + 2 * segs;
    let ranges = deltas + 2 * segs;
    // The first segment whose end is at or past `c`.
    let (mut lo, mut hi) = (0usize, segs);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if (u16_at(d, ends + 2 * mid)? as u32) < c { lo = mid + 1 } else { hi = mid }
    }
    if lo >= segs {
        return None;
    }
    let start = u16_at(d, starts + 2 * lo)? as u32;
    if start > c {
        return None;
    }
    let delta = u16_at(d, deltas + 2 * lo)?;
    let range = u16_at(d, ranges + 2 * lo)? as usize;
    if range == 0 {
        return Some((c as u16).wrapping_add(delta));
    }
    let g = u16_at(d, ranges + 2 * lo + range + 2 * (c - start) as usize)?;
    (g != 0).then(|| g.wrapping_add(delta))
}

/// Format 12 lookup (sequential groups over all of Unicode).
fn cmap12(d: &[u8], at: usize, c: u32) -> Option<u16> {
    let groups = u32_at(d, at + 12)? as usize;
    let (mut lo, mut hi) = (0usize, groups.min(1 << 20));
    while lo < hi {
        let mid = (lo + hi) / 2;
        let g = at + 16 + 12 * mid;
        let (start, end) = (u32_at(d, g)?, u32_at(d, g + 4)?);
        if c < start {
            hi = mid;
        } else if c > end {
            lo = mid + 1;
        } else {
            return u16::try_from(u32_at(d, g + 8)?.checked_add(c - start)?).ok();
        }
    }
    None
}

/// The outline size of every glyph of a font program (from its long `loca`), for tests.
#[cfg(test)]
pub(crate) fn outline_sizes(program: &[u8]) -> Option<Vec<usize>> {
    let dir = directory(program, 0)?;
    let loca = table(program, &dir, b"loca")?;
    (0..(loca.len() / 4).saturating_sub(1)).map(|i| Some(u32_at(loca, 4 * i + 4)?.checked_sub(u32_at(loca, 4 * i)?)? as usize)).collect()
}

/// A ToUnicode CMap (ISO 32000-1 9.10.3) mapping each used glyph back to its character, so text
/// in the PDF can be searched and copied.
pub(crate) fn to_unicode(map: &BTreeMap<u16, char>) -> String {
    let mut s = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let pairs: Vec<(&u16, &char)> = map.iter().collect();
    for chunk in pairs.chunks(100) {
        let _ = writeln!(s, "{} beginbfchar", chunk.len());
        for (g, c) in chunk {
            let mut units = [0u16; 2];
            let hex: String = c.encode_utf16(&mut units).iter().map(|u| format!("{u:04X}")).collect();
            let _ = writeln!(s, "<{g:04X}> <{hex}>");
        }
        s.push_str("endbfchar\n");
    }
    s.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
    s
}
