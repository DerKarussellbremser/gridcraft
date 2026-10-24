//! Links to other workbooks (`xl/externalLinks/externalLinkN.xml`): where each one is and the
//! values Excel last read from it, so `[1]Kunden!A2` evaluates without the other file. The parts
//! themselves are kept for saving by the pass-through.

use gridcraft_core::{CellError, CellRef, Value};
use gridcraft_model::{ExternalBook, ExternalSheet};

use crate::package::Rel;
use crate::read::Ctx;
use crate::xml::El;

/// Most cached cells taken from one file: a link part is input like any other.
const MAX_CELLS: usize = 4_000_000;

/// The linked workbooks in `<externalReferences>` order, which is the `n` of `[n]` in formulas.
/// A link that can't be read is an empty book: its references are `#REF!`.
pub(crate) fn read(cx: &mut Ctx<'_>, wb_xml: &El, wb_rels: &[Rel]) -> Vec<ExternalBook> {
    let Some(refs) = wb_xml.child("externalReferences") else { return vec![] };
    let mut budget = MAX_CELLS;
    let mut books = Vec::new();
    // The XML reader keeps local names: `r:id` is `id`.
    for id in refs.kids("externalReference").filter_map(|x| x.attr("id")) {
        let book = wb_rels.iter().find(|r| r.id == id).and_then(|r| book(cx, &r.target, &mut budget));
        books.push(book.unwrap_or_default());
    }
    books
}

fn book(cx: &mut Ctx<'_>, part: &str, budget: &mut usize) -> Option<ExternalBook> {
    let x = cx.optional_xml(part).ok()??;
    let b = x.child("externalBook")?;
    let rels = cx.pkg.rels(part).unwrap_or_default();
    // `file:///C:\Daten\x.xlsx` is the path `C:\Daten\x.xlsx`.
    let target = |id: Option<&str>| {
        id.and_then(|id| rels.iter().find(|r| r.id == id)).map(|r| {
            let t = percent_decode(&r.target);
            t.strip_prefix("file:///").filter(|p| p.as_bytes().get(1) == Some(&b':') || p.starts_with('\\')).map(str::to_string).unwrap_or(t)
        })
    };
    // The book's own target (often relative), then Excel 365's absolute alternate.
    let mut targets: Vec<String> = Vec::new();
    let absolute = b.path(&["alternateUrls", "absoluteUrl"]).and_then(|u| u.attr("id"));
    for t in [target(b.attr("id")), target(absolute)].into_iter().flatten() {
        if !targets.contains(&t) {
            targets.push(t);
        }
    }
    let names: Vec<String> =
        b.child("sheetNames").map(|s| s.kids("sheetName").filter_map(|n| n.attr("val")).map(str::to_string).collect()).unwrap_or_default();
    let mut sheets: Vec<ExternalSheet> = names.into_iter().map(|name| ExternalSheet { name, cells: vec![] }).collect();
    for data in b.child("sheetDataSet").map(|d| d.kids("sheetData").collect::<Vec<_>>()).unwrap_or_default() {
        let Some(sheet) = data.attr_u32("sheetId").and_then(|i| sheets.get_mut(i as usize)) else { continue };
        for cell in data.kids("row").flat_map(|r| r.kids("cell")) {
            if *budget == 0 {
                break;
            }
            let (Some(at), Some(v)) = (cell.attr("r").and_then(CellRef::parse), cell.child("v").map(|v| v.text.as_str())) else { continue };
            let value = match cell.attr("t") {
                Some("str" | "s" | "inlineStr") => Value::text(v),
                Some("b") => Value::Bool(v.trim() == "1"),
                Some("e") => Value::Error(CellError::parse(v.trim()).unwrap_or(CellError::NA)),
                _ => match v.trim().parse::<f64>() {
                    Ok(n) if n.is_finite() => Value::number(n),
                    _ => continue,
                },
            };
            sheet.cells.push((at, value));
            *budget -= 1;
        }
        sheet.cells.sort_by_key(|(c, _)| (c.row, c.col));
        sheet.cells.dedup_by_key(|(c, _)| *c);
    }
    Some(ExternalBook { targets, sheets })
}

/// The workbook as it is saved: its kept link parts carry the values in `external_books`, which
/// "Update Values" may have changed since the file was read, and a changed source's path. Only
/// the `<sheetDataSet>` of a part is rewritten. `None` when the workbook has no kept links.
pub(crate) fn with_current_links(wb: &gridcraft_model::Workbook) -> Option<gridcraft_model::Workbook> {
    let p = wb.preserved.as_ref()?;
    let refs = p.elements.iter().find(|e| e.slot == "externalReferences")?;
    let ids: Vec<&str> = refs.xml.split("r:id=\"").skip(1).filter_map(|s| s.split('"').next()).collect();
    let same = |a: &str, b: &str| a.trim_start_matches('/') == b.trim_start_matches('/');
    let mut parts = p.parts.clone();
    for (i, id) in ids.iter().enumerate() {
        let (Some(book), Some(rel)) = (wb.external_books.get(i), p.rels.iter().find(|r| r.id == *id)) else { continue };
        let Some(part) = parts.iter_mut().find(|q| same(&q.name, &rel.target)) else { continue };
        let xml = String::from_utf8_lossy(&part.data).into_owned();
        if let Some(new) = replace_sheet_data(&xml, book) {
            part.data = std::sync::Arc::from(new.into_bytes());
        }
        // A changed source (Change Source picks a file: an absolute path) is the book's own
        // relationship. Relative targets are left to Save As, which rebases them on the new folder.
        let own = xml.split("<externalBook").nth(1).and_then(|s| s.split("r:id=\"").nth(1)).and_then(|s| s.split('"').next());
        if let (Some(own), Some(target)) = (own, book.targets.first())
            && is_absolute(target)
            && let Some(r) = part.rels.iter_mut().find(|r| r.id == own && r.external)
            && r.target != to_uri(target)
        {
            r.target = to_uri(target);
        }
    }
    let mut out = wb.clone();
    out.preserved = Some(std::sync::Arc::new(gridcraft_model::Preserved { parts, ..(**p).clone() }));
    Some(out)
}

/// The part's XML with its `<sheetDataSet>` written from `book`; `None` when it has no place for
/// one GridCraft recognises (then the part is kept as it was).
fn replace_sheet_data(xml: &str, book: &ExternalBook) -> Option<String> {
    use std::fmt::Write as _;
    let mut set = String::from("<sheetDataSet>");
    for (i, sheet) in book.sheets.iter().enumerate() {
        if sheet.cells.is_empty() {
            let _ = write!(set, "<sheetData sheetId=\"{i}\"/>");
            continue;
        }
        let _ = write!(set, "<sheetData sheetId=\"{i}\">");
        let mut row = None;
        for (at, v) in &sheet.cells {
            if row != Some(at.row) {
                if row.is_some() {
                    set.push_str("</row>");
                }
                let _ = write!(set, "<row r=\"{}\">", at.row + 1);
                row = Some(at.row);
            }
            let (t, text) = match v {
                Value::Number(n) => ("", crate::xml::num(*n)),
                Value::Text(s) => (" t=\"str\"", crate::xml::esc(s)),
                Value::Bool(b) => (" t=\"b\"", if *b { "1" } else { "0" }.to_string()),
                Value::Error(e) => (" t=\"e\"", crate::xml::esc(e.as_str())),
                _ => continue,
            };
            let _ = write!(set, "<cell r=\"{}\"{t}><v>{text}</v></cell>", at.a1());
        }
        if row.is_some() {
            set.push_str("</row>");
        }
        set.push_str("</sheetData>");
    }
    set.push_str("</sheetDataSet>");
    if let Some(start) = xml.find("<sheetDataSet") {
        let rest = xml.get(start..)?;
        let len = match (rest.find("/>"), rest.find('>'), rest.find("</sheetDataSet>")) {
            (Some(a), Some(b), _) if a + 1 == b => a + 2,
            (_, _, Some(end)) => end + "</sheetDataSet>".len(),
            _ => return None,
        };
        return Some(format!("{}{set}{}", xml.get(..start)?, rest.get(len..)?));
    }
    let end = xml.find("</externalBook>")?;
    Some(format!("{}{set}{}", xml.get(..end)?, xml.get(end..)?))
}

/// `C:\…`, `\\server\…`, `/…` or `file:…`: a path that doesn't depend on the workbook's folder.
fn is_absolute(path: &str) -> bool {
    let b = path.as_bytes();
    path.starts_with(['/', '\\']) || path.starts_with("file:") || (b.get(1) == Some(&b':') && b.first().is_some_and(u8::is_ascii_alphabetic))
}

/// A path as a link target, as Excel writes it: `file:///C:\Daten\Kunden%20neu.xlsx`.
fn to_uri(path: &str) -> String {
    let mut out = String::from(if path.starts_with("file:") || path.starts_with('/') { "" } else { "file:///" });
    for b in path.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'\\' | b'.' | b'-' | b'_' | b':' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `Daten%20.xlsx` → `Daten .xlsx` (link targets are URIs).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while let Some(&c) = b.get(i) {
        let hex = b.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok());
        match (c, hex) {
            (b'%', Some(h)) => {
                out.push(h);
                i += 3;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::percent_decode;

    #[test]
    fn link_targets_are_decoded() {
        assert_eq!(percent_decode("Daten%20.xlsx"), "Daten .xlsx");
        assert_eq!(percent_decode("/Volumes/SYS/Firma/2026/Daten%20.xlsx"), "/Volumes/SYS/Firma/2026/Daten .xlsx");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("Gr%C3%B6%C3%9Fe.xlsx"), "Größe.xlsx");
    }
}
