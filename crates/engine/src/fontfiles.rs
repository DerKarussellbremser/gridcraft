//! The installed TrueType fonts PDF export embeds: the system's font files catalogued by family
//! name (read once, on the first export that needs them), and the face a cell's font is set in.
//! A family without the asked-for bold or italic face is imitated, as Excel does (Arial Black has
//! one face: bold italic Arial Black prints that face slanted and outlined). None on wasm.

use std::sync::{Arc, Mutex, OnceLock};

use gridcraft_pdf::{EmbeddedFont, FaceInfo, Synthetic};

/// Font files larger than this are skipped (no font face is that big; a hostile file might be).
#[cfg(not(target_arch = "wasm32"))]
const MAX_FONT_FILE: u64 = 64 << 20;
/// Files read for the catalog at most.
#[cfg(not(target_arch = "wasm32"))]
const MAX_FONT_FILES: usize = 10_000;

#[cfg_attr(test, allow(dead_code))]
struct Entry {
    path: std::path::PathBuf,
    info: FaceInfo,
}

/// The face of `family` to set text in that is bold and italic as asked, and what of that has
/// to be imitated. `None` when no installed TrueType face has the name.
pub fn face(family: &str, bold: bool, italic: bool) -> Option<(Arc<EmbeddedFont>, Synthetic)> {
    #[cfg(test)]
    {
        let _ = family;
        TEST_FACE.with(|t| t.borrow().clone()).map(|f| (f, Synthetic { bold, italic }))
    }
    #[cfg(not(test))]
    {
        let cat = catalog();
        let infos: Vec<&FaceInfo> = cat.iter().map(|e| &e.info).collect();
        let (i, synthetic) = pick(&infos, family, bold, italic)?;
        let entry = cat.get(i)?;
        load(&entry.path, entry.info.index).map(|f| (f, synthetic))
    }
}

/// Which of `faces` sets `family` bold and italic as asked: the family's own face with that style,
/// else its regular (or other) face with the missing style imitated. The legacy family name (what
/// a workbook stores, `Arial Black`) is matched first, then the typographic one.
fn pick(faces: &[&FaceInfo], family: &str, bold: bool, italic: bool) -> Option<(usize, Synthetic)> {
    let by = |typographic: bool| -> Vec<usize> {
        (0..faces.len())
            .filter(|&i| {
                let f = faces[i];
                if typographic {
                    f.typographic_family.as_deref().is_some_and(|t| t.eq_ignore_ascii_case(family))
                } else {
                    f.family.eq_ignore_ascii_case(family)
                }
            })
            .collect()
    };
    let group = Some(by(false)).filter(|g| !g.is_empty()).unwrap_or_else(|| by(true));
    let find = |b: bool, it: bool| group.iter().copied().find(|&i| faces[i].bold == b && faces[i].italic == it);
    // Asked-for style first; then drop italic, then bold, then both, imitating what is missing.
    for (b, it) in [(bold, italic), (false, italic), (bold, false), (false, false)] {
        if let Some(i) = find(b, it) {
            return Some((i, Synthetic { bold: bold && !b, italic: italic && !it }));
        }
    }
    let i = *group.first()?;
    Some((i, Synthetic { bold: bold && !faces[i].bold, italic: italic && !faces[i].italic }))
}

/// Faces read so far, by file and index (each file is read once).
type Loaded = std::collections::HashMap<(std::path::PathBuf, u32), Option<Arc<EmbeddedFont>>>;

#[cfg_attr(test, allow(dead_code))]
fn load(path: &std::path::Path, index: u32) -> Option<Arc<EmbeddedFont>> {
    static LOADED: OnceLock<Mutex<Loaded>> = OnceLock::new();
    let mut loaded = LOADED.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    loaded
        .entry((path.to_path_buf(), index))
        .or_insert_with(|| {
            let data = std::fs::read(path).ok()?;
            EmbeddedFont::parse(data.into(), index)
        })
        .clone()
}

#[cfg_attr(test, allow(dead_code))]
fn catalog() -> &'static [Entry] {
    static CATALOG: OnceLock<Vec<Entry>> = OnceLock::new();
    CATALOG.get_or_init(scan)
}

#[cfg(target_arch = "wasm32")]
fn scan() -> Vec<Entry> {
    Vec::new()
}

/// Every TrueType face in the system's and the user's font folders, recursing into subfolders.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(test, allow(dead_code))]
fn scan() -> Vec<Entry> {
    use std::path::PathBuf;
    let mut dirs: Vec<PathBuf> = Vec::new();
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    if cfg!(target_os = "macos") {
        dirs.extend(["/System/Library/Fonts", "/System/Library/Fonts/Supplemental", "/Library/Fonts"].map(PathBuf::from));
        dirs.extend(home.iter().map(|h| h.join("Library/Fonts")));
    } else if cfg!(windows) {
        let root = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Windows"));
        dirs.push(root.join("Fonts"));
        dirs.extend(std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("Microsoft\\Windows\\Fonts")));
    } else {
        dirs.extend(["/usr/share/fonts", "/usr/local/share/fonts"].map(PathBuf::from));
        dirs.extend(home.iter().flat_map(|h| [h.join(".fonts"), h.join(".local/share/fonts")]));
    }
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut files = 0usize;
    while let Some(dir) = dirs.pop() {
        if !seen.insert(std::fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone())) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let font = path.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "ttc"));
            if !font || std::fs::metadata(&path).map(|m| m.len() > MAX_FONT_FILE).unwrap_or(true) {
                continue;
            }
            files += 1;
            if files > MAX_FONT_FILES {
                return out;
            }
            let Ok(data) = std::fs::read(&path) else { continue };
            out.extend(gridcraft_pdf::faces(&data).into_iter().map(|info| Entry { path: path.clone(), info }));
        }
    }
    out
}

#[cfg(test)]
thread_local! {
    /// Unit tests set text in this face, or in the standard fonts when it is `None` (their PDFs
    /// are checked as text, which embedding would turn into glyph numbers).
    static TEST_FACE: std::cell::RefCell<Option<Arc<EmbeddedFont>>> = const { std::cell::RefCell::new(None) };
}

/// Sets every family in `font` for the rest of this test thread (`None`: standard fonts again).
#[cfg(test)]
pub(crate) fn set_test_face(font: Option<Arc<EmbeddedFont>>) {
    TEST_FACE.with(|t| *t.borrow_mut() = font);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(family: &str, typographic: Option<&str>, bold: bool, italic: bool) -> FaceInfo {
        FaceInfo { index: 0, family: family.into(), typographic_family: typographic.map(Into::into), bold, italic }
    }

    #[test]
    fn a_cell_font_finds_its_face_and_imitates_a_missing_style() {
        let arial =
            [info("Arial", None, false, false), info("Arial", None, true, false), info("Arial", None, false, true), info("Arial", None, true, true)];
        let black = info("Arial Black", Some("Arial"), false, false);
        let all: Vec<&FaceInfo> = arial.iter().chain([&black]).collect();
        let none = Synthetic::default();
        assert_eq!(pick(&all, "arial", true, true), Some((3, none)), "the family's own bold italic");
        assert_eq!(pick(&all, "Arial", false, true), Some((2, none)));
        // Arial Black has one face: bold italic is that face slanted and outlined (as Excel prints it).
        assert_eq!(pick(&all, "Arial Black", true, true), Some((4, Synthetic { bold: true, italic: true })));
        assert_eq!(pick(&all, "Arial Black", false, false), Some((4, none)));
        // A family with a regular face only: italic is imitated, bold too.
        let solo = [info("Solo", None, false, false)];
        let solo: Vec<&FaceInfo> = solo.iter().collect();
        assert_eq!(pick(&solo, "Solo", false, true), Some((0, Synthetic { bold: false, italic: true })));
        // Only a bold face: plain text uses it as it is.
        let heavy = [info("Heavy", None, true, false)];
        let heavy: Vec<&FaceInfo> = heavy.iter().collect();
        assert_eq!(pick(&heavy, "Heavy", false, false), Some((0, none)));
        assert_eq!(pick(&all, "Not Installed", false, false), None);
    }
}
