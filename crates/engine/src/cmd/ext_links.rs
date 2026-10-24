//! Links to other workbooks (Data › Edit Links): which files a workbook's formulas read from,
//! and reading their values again. A link comes from the document, so a file is only read when
//! the user (or a script) asks: opening a workbook never reads another one by itself.

use std::path::{Path, PathBuf};

use gridcraft_model::{ExternalBook, ExternalSheet};
use serde_json::{Value as Json, json};

use super::*;

/// Most cells read from one linked workbook, as for the values cached in the file.
const MAX_CELLS: usize = 4_000_000;

pub(crate) fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "data.editLinks",
            "Edit Links",
            ["Data", "Queries & Connections"],
            None,
            "{} → [{book, source, path, sheets}]: path is where the file was found, or null",
            has_links,
            list
        ),
        cmd!(
            "data.updateLinks",
            "Update Values",
            ["Data", "Queries & Connections"],
            None,
            "{book?: n, the [n] in formulas; all when omitted} → reads the linked files again",
            has_links,
            update
        ),
        cmd!(
            "data.changeLinkSource",
            "Change Source",
            ["Data", "Queries & Connections"],
            None,
            "{path, book?: n (1)} → links the workbook to another file and reads it",
            has_links,
            change_source
        ),
    ]
}

fn has_links(s: &Session) -> std::result::Result<(), String> {
    has_doc(s)?;
    if s.active().is_some_and(|d| d.wb.external_books.is_empty()) { Err("this workbook has no links to other workbooks".into()) } else { Ok(()) }
}

/// Where a linked workbook is on this machine: a saved path that exists (a relative one is next
/// to this workbook, a network one `\\server\share\…` is tried as it is), else a file of the
/// same name next to this workbook, which is where it is after both were moved or copied from
/// another computer (`/Volumes/…` saved on a Mac).
pub(crate) fn locate(book: &ExternalBook, dir: Option<&Path>) -> Option<PathBuf> {
    let paths: Vec<String> = book.targets.iter().filter_map(|t| local_path(t)).collect();
    for t in &paths {
        let p = Path::new(t);
        let p = if p.is_absolute() { p.to_path_buf() } else { dir?.join(p) };
        if p.is_file() {
            return Some(p);
        }
    }
    let dir = dir?;
    paths.iter().filter_map(|t| t.rsplit(['/', '\\']).next()).map(|name| dir.join(name)).find(|p| p.is_file())
}

/// A link target as a file path: `file:///C:\x.xlsx` → `C:\x.xlsx`, `file://server/share/x.xlsx`
/// → `\\server\share\x.xlsx`; web addresses (`https://…`) are not files and give `None`.
fn local_path(target: &str) -> Option<String> {
    if let Some(rest) = target.strip_prefix("file:///") {
        return Some(rest.to_string());
    }
    if let Some(rest) = target.strip_prefix("file://") {
        return Some(format!("\\\\{}", rest.replace('/', "\\")));
    }
    (!target.contains("://")).then(|| target.to_string())
}

fn list(s: &mut Session, _: &Json) -> Result<Json> {
    let wb = &s.doc()?.wb;
    let dir = wb.source_dir.as_deref();
    let books: Vec<Json> = wb
        .external_books
        .iter()
        .enumerate()
        .map(|(i, b)| {
            json!({
                "book": i + 1,
                "source": b.targets.first(),
                "path": locate(b, dir).map(|p| p.display().to_string()),
                "sheets": b.sheets.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(json!(books))
}

fn update(s: &mut Session, p: &Json) -> Result<Json> {
    let which = book_param(s, p, "data.updateLinks")?;
    refresh(s, which, None)
}

fn change_source(s: &mut Session, p: &Json) -> Result<Json> {
    let path = str_param(p, "path").ok_or_else(|| bad("data.changeLinkSource", "missing `path` (the workbook to link to)"))?;
    let which = book_param(s, p, "data.changeLinkSource")?.unwrap_or(1);
    refresh(s, Some(which), Some(PathBuf::from(path)))
}

/// `book`, checked against the workbook's links; `None` = all of them.
fn book_param(s: &Session, p: &Json, cmd: &str) -> Result<Option<usize>> {
    let count = s.doc()?.wb.external_books.len();
    match p.get("book") {
        None | Some(Json::Null) => Ok(None),
        Some(v) => match v.as_u64().and_then(|n| usize::try_from(n).ok()).filter(|n| (1..=count).contains(n)) {
            Some(n) => Ok(Some(n)),
            None => Err(bad(cmd, format!("`book` must be 1 to {count}"))),
        },
    }
}

/// Reads the linked files again (book `which`, or all) and recalculates. `source` replaces the
/// book's saved path first. A file that isn't found or can't be read keeps its cached values.
fn refresh(s: &mut Session, which: Option<usize>, source: Option<PathBuf>) -> Result<Json> {
    let wb = &s.doc()?.wb;
    let dir = wb.source_dir.clone();
    let mut books = wb.external_books.clone();
    let (mut updated, mut missing) = (Vec::new(), Vec::new());
    for (i, book) in books.iter_mut().enumerate() {
        if which.is_some_and(|w| w != i + 1) {
            continue;
        }
        if let Some(path) = &source {
            book.targets = vec![path.display().to_string()];
        }
        let found = match &source {
            Some(path) => path.is_file().then(|| path.clone()),
            None => locate(book, dir.as_deref()),
        };
        let Some(path) = found else {
            missing.push(json!({"book": i + 1, "source": book.targets.first(), "error": "not found"}));
            continue;
        };
        match read_values(&path, book) {
            Ok(sheets) => {
                book.sheets = sheets;
                updated.push(json!({"book": i + 1, "path": path.display().to_string()}));
            }
            Err(e) => missing.push(json!({"book": i + 1, "path": path.display().to_string(), "error": e.to_string()})),
        }
    }
    if source.is_some() || !updated.is_empty() {
        edit(s, |cx| {
            cx.wb.external_books = books;
            cx.structural = true;
            Ok(())
        })?;
    }
    Ok(json!({"updated": updated, "missing": missing}))
}

/// The values of the sheets the formulas name, read from the file at `path`. A sheet the file
/// no longer has comes back empty, so its references are blank, not stale.
fn read_values(path: &Path, book: &ExternalBook) -> Result<Vec<ExternalSheet>> {
    let name = path.display().to_string();
    let bytes = crate::io::read_file(&name)?;
    let (source, _) = crate::io::open_bytes_with(&name, &bytes, None, false)?;
    let mut budget = MAX_CELLS;
    Ok(book
        .sheets
        .iter()
        .map(|sheet| {
            let cells = source
                .sheets
                .iter()
                .find(|s| s.name.eq_ignore_ascii_case(&sheet.name))
                .map(|s| s.cells.iter().filter(|(_, c)| !c.value.is_empty()).take(budget).map(|(at, c)| (at, c.value.clone())).collect::<Vec<_>>())
                .unwrap_or_default();
            budget = budget.saturating_sub(cells.len());
            ExternalSheet { name: sheet.name.clone(), cells }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use gridcraft_core::Value;

    use super::*;

    /// The saved path, else the same file name next to this workbook (a Mac path on Windows).
    #[test]
    fn linked_files_are_found_where_they_are_or_next_to_the_workbook() {
        let dir = std::env::temp_dir().join(format!("gridcraft-links-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Daten .xlsx");
        std::fs::write(&file, b"x").unwrap();
        let book = |t: &[&str]| ExternalBook { targets: t.iter().map(|s| s.to_string()).collect(), sheets: vec![] };
        assert_eq!(locate(&book(&["Daten .xlsx"]), Some(&dir)), Some(file.clone()));
        assert_eq!(locate(&book(&["/Volumes/SYS/Firma/2026/Daten .xlsx"]), Some(&dir)), Some(file.clone()));
        assert_eq!(locate(&book(&[&file.display().to_string()]), None), Some(file.clone()));
        assert_eq!(locate(&book(&["https://example.com/Daten .xlsx"]), Some(&dir)), None);
        assert_eq!(locate(&book(&["Anders.xlsx"]), Some(&dir)), None);
        // Network paths, in each form a link can carry them.
        assert_eq!(local_path("file://server/SYS/Firma/Daten .xlsx").as_deref(), Some(r"\\server\SYS\Firma\Daten .xlsx"));
        assert_eq!(local_path(r"\\server\SYS\Daten .xlsx").as_deref(), Some(r"\\server\SYS\Daten .xlsx"));
        assert_eq!(local_path(r"file:///\\server\SYS\Daten .xlsx").as_deref(), Some(r"\\server\SYS\Daten .xlsx"));
        assert_eq!(local_path("https://example.com/x.xlsx"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Updating reads the other workbook's current values and recalculates the lookups.
    #[test]
    fn update_reads_the_linked_workbook_and_recalculates() {
        let dir = std::env::temp_dir().join(format!("gridcraft-update-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // The data workbook: customer 300 is new since the cache was saved.
        let mut data = Session::new();
        data.new_workbook();
        data.execute("sheet.rename", json!({"name": "Kunden"})).unwrap();
        data.execute("range.setValues", json!({"range": "A2", "values": [[100, "Frank"], [300, "Neu"]]})).unwrap();
        let path = dir.join("Daten .xlsx");
        data.execute("file.saveAs", json!({"path": path.display().to_string()})).unwrap();
        // The invoice: its cache only knows customer 100.
        let mut s = Session::new();
        s.new_workbook();
        let kunden = ExternalSheet {
            name: "Kunden".into(),
            cells: vec![(CellRef::new(1, 0), Value::number(100.0)), (CellRef::new(1, 1), Value::text("Frank"))],
        };
        s.doc_mut().unwrap().wb = std::sync::Arc::new(Workbook {
            external_books: vec![ExternalBook { targets: vec!["/Volumes/SYS/Daten .xlsx".into()], sheets: vec![kunden] }],
            source_dir: Some(dir.clone()),
            ..(*s.doc().unwrap().wb).clone()
        });
        s.execute("range.setValues", json!({"range": "A1", "values": [[300]]})).unwrap();
        s.execute("cell.set", json!({"cell": "B1", "input": "=VLOOKUP(A1,[1]Kunden!$A$2:$B$100,2,FALSE)"})).unwrap();
        let b1 = |s: &Session| s.doc().unwrap().wb.sheet(0).unwrap().value(CellRef::new(0, 1));
        assert_eq!(b1(&s), Value::Error(gridcraft_core::CellError::NA), "not in the cache yet");
        let listed = s.execute("data.editLinks", json!({})).unwrap();
        assert_eq!(listed[0]["path"], json!(path.display().to_string()));
        let r = s.execute("data.updateLinks", json!({})).unwrap();
        assert_eq!(r["updated"].as_array().map(Vec::len), Some(1), "{r}");
        assert_eq!(b1(&s), Value::text("Neu"));
        assert!(s.execute("data.updateLinks", json!({"book": 2})).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
