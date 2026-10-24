//! Rich text strings (`<r>` runs with `<rPr>`, ISO/IEC 29500-1 §18.4.4 and §18.4.7) are read onto
//! the sheet, written back as rich shared strings, and dropped when the cell's text changed.

use gridcraft_core::{CellRef, Value};
use gridcraft_model::{Cell, Color, StyleId, Underline};

use super::minimal;
use crate::package::Package;
use crate::{read_xlsx, write_xlsx};

/// A rich item (a plain run, then an underlined 8 pt Arial run), a plain item, and a rich item
/// whose runs have no properties at all.
const SST: &str = r#"<?xml version="1.0"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="3" uniqueCount="3"><si><r><t xml:space="preserve">Firma: </t></r><r><rPr><u/><sz val="8"/><rFont val="Arial"/><family val="2"/></rPr><t>Muster GmbH</t></r></si><si><t>schlicht</t></si><si><r><t>ohne </t></r><r><t>Format</t></r></si></sst>"#;

/// A1 and A2 share the rich item, B1 is plain, C1 has runs without properties, D1 is an inline
/// rich string with a bold red run.
const SHEET_BODY: &str = r#"<sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c><c r="C1" t="s"><v>2</v></c><c r="D1" t="inlineStr"><is><r><t>Summe </t></r><r><rPr><b/><color rgb="FFFF0000"/><sz val="10"/><rFont val="Arial"/></rPr><t>netto</t></r></is></c></row><row r="2"><c r="A2" t="s"><v>0</v></c></row></sheetData>"#;

const SST_RELS: &str = r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>"#;

fn at(a: &str) -> CellRef {
    CellRef::parse(a).unwrap()
}

fn part(bytes: &[u8], name: &str) -> String {
    let mut pkg = Package::open(bytes).unwrap();
    String::from_utf8(pkg.read(name).unwrap().unwrap()).unwrap()
}

#[test]
fn rich_strings_are_read_onto_their_cells() {
    let (wb, _) = read_xlsx(&minimal(SHEET_BODY, &[("xl/sharedStrings.xml", SST)], SST_RELS, "")).unwrap();
    let sheet = wb.sheet(0).unwrap();
    assert_eq!(sheet.value(at("A1")), Value::text("Firma: Muster GmbH"));
    let spans = sheet.rich_spans(at("A1")).unwrap();
    assert_eq!(spans.iter().map(|(s, _)| *s).collect::<Vec<_>>(), ["Firma: ", "Muster GmbH"]);
    assert!(spans[0].1.is_none(), "a run without rPr takes the cell's font");
    let f = spans[1].1.unwrap();
    assert_eq!((f.name.as_str(), f.size, f.underline, f.bold), ("Arial", 8.0, Underline::Single, false));
    assert!(sheet.rich_spans(at("A2")).is_some(), "every cell using the item gets its runs");
    // Plain items, and runs that set nothing, are plain text.
    assert!(!sheet.rich.contains_key(&at("B1")) && !sheet.rich.contains_key(&at("C1")));
    // Inline rich strings too.
    let inline = sheet.rich_spans(at("D1")).unwrap();
    let f = inline[1].1.unwrap();
    assert!(f.bold && f.color == Color::Rgb(0xFF0000), "{f:?}");
}

#[test]
fn rich_strings_are_saved_and_stale_runs_are_not() {
    let (mut wb, _) = read_xlsx(&minimal(SHEET_BODY, &[("xl/sharedStrings.xml", SST)], SST_RELS, "")).unwrap();
    let bytes = write_xlsx(&wb).unwrap();
    let sst = part(&bytes, "xl/sharedStrings.xml");
    assert!(
        sst.contains(r#"<si><r><t xml:space="preserve">Firma: </t></r><r><rPr><rFont val="Arial"/><family val="2"/><color auto="1"/><sz val="8"/><u/></rPr><t>Muster GmbH</t></r></si>"#),
        "{sst}"
    );
    assert_eq!(sst.matches("Muster GmbH").count(), 1, "A1 and A2 share one item: {sst}");
    assert!(sst.contains(r#"<rPr><rFont val="Arial"/><family val="2"/><b/><color rgb="FFFF0000"/><sz val="10"/></rPr><t>netto</t>"#), "{sst}");
    // Open again: the same runs; save again: the same table.
    let (back, _) = read_xlsx(&bytes).unwrap();
    assert_eq!(back.sheet(0).unwrap().rich, wb.sheet(0).unwrap().rich);
    assert_eq!(part(&write_xlsx(&back).unwrap(), "xl/sharedStrings.xml"), sst);

    // A1 gets other text: its runs are stale, and only A2 keeps the rich item.
    std::sync::Arc::make_mut(&mut wb.sheets[0])
        .cells
        .set(at("A1"), Cell { value: Value::text("Firma: Beispiel AG"), formula: None, style: StyleId::DEFAULT });
    let sst = part(&write_xlsx(&wb).unwrap(), "xl/sharedStrings.xml");
    assert!(sst.contains("<si><t>Firma: Beispiel AG</t></si>"), "{sst}");
    assert_eq!(sst.matches("<r>").count(), 4, "two runs for A2, two for D1: {sst}");
}
