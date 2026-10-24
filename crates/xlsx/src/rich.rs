//! Rich text strings: a string item (`<si>`, `<is>`) set in more than one font, as `<r>` runs
//! with run properties (`<rPr>`, ISO/IEC 29500-1 §18.4.4 `r` and §18.4.7 `rPr` as CT_RPrElt).

use std::fmt::Write as _;

use gridcraft_model::rich::MAX_RUNS;
use gridcraft_model::{Font, RichText, TextRun, Underline, VertAlign};

use crate::styles::{DEFAULT_FONT_FAMILY, MAX_FONT_FAMILY, color_attrs, read_font};
use crate::xml::{El, decode_xstring, esc_attr, num, t_el};

/// The formatted runs of a string item, when at least one run has properties of its own. A run
/// without `<rPr>`, or a bare `<t>`, takes the cell's font. More than [`MAX_RUNS`] runs read as
/// plain text.
pub fn read(e: &El, palette: &[u32]) -> Option<RichText> {
    if !e.kids("r").any(|r| r.child("rPr").is_some()) {
        return None;
    }
    let mut text = String::new();
    let mut runs = Vec::new();
    for c in &e.children {
        let (piece, font) = match c.name.as_str() {
            "t" => (decode_xstring(&c.text), None),
            "r" => (decode_xstring(&c.kids("t").map(|t| t.text.as_str()).collect::<String>()), c.child("rPr").map(|p| read_font(p, palette))),
            _ => continue,
        };
        if runs.len() >= MAX_RUNS {
            return None;
        }
        runs.push(TextRun { len: u32::try_from(piece.chars().count()).unwrap_or(u32::MAX), font });
        text.push_str(&piece);
    }
    Some(RichText { text: text.into(), runs })
}

/// The body of a `<si>` for `text` set in `rich`'s runs, or `None` when the runs don't fit it.
pub fn si_body(text: &str, rich: &RichText) -> Option<String> {
    let spans = rich.spans(text)?;
    let mut s = String::new();
    for (piece, font) in spans.into_iter().filter(|(piece, _)| !piece.is_empty()) {
        s.push_str("<r>");
        if let Some(f) = font {
            s.push_str(&rpr(f));
        }
        s.push_str(&t_el(piece));
        s.push_str("</r>");
    }
    Some(s)
}

/// Run properties in CT_RPrElt order: rFont, charset, family, b, i, strike, color, sz, u, vertAlign.
fn rpr(f: &Font) -> String {
    let mut s = String::from("<rPr>");
    let _ = write!(s, "<rFont val=\"{}\"/>", esc_attr(&f.name));
    if let Some(charset) = f.charset {
        let _ = write!(s, "<charset val=\"{charset}\"/>");
    }
    let _ = write!(s, "<family val=\"{}\"/>", f.family.filter(|v| *v <= MAX_FONT_FAMILY).unwrap_or(DEFAULT_FONT_FAMILY));
    for (on, tag) in [(f.bold, "b"), (f.italic, "i"), (f.strike, "strike")] {
        if on {
            let _ = write!(s, "<{tag}/>");
        }
    }
    let _ = write!(s, "<color {}/>", color_attrs(&f.color));
    let _ = write!(s, "<sz val=\"{}\"/>", num(f.size as f64));
    match f.underline {
        Underline::None => {}
        Underline::Single => s.push_str("<u/>"),
        Underline::Double => s.push_str("<u val=\"double\"/>"),
        Underline::SingleAccounting => s.push_str("<u val=\"singleAccounting\"/>"),
        Underline::DoubleAccounting => s.push_str("<u val=\"doubleAccounting\"/>"),
    }
    match f.vert {
        VertAlign::Baseline => {}
        VertAlign::Superscript => s.push_str("<vertAlign val=\"superscript\"/>"),
        VertAlign::Subscript => s.push_str("<vertAlign val=\"subscript\"/>"),
    }
    s.push_str("</rPr>");
    s
}
