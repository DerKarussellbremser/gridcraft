//! Rich text: a text cell set in more than one font (its formatted runs).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::style::Font;

/// Most runs one cell keeps; a string with more is read as plain text.
pub const MAX_RUNS: usize = 1000;

/// The formatted runs of a text cell. Each run covers `len` characters and is set in `font`
/// (`None`: the cell's own font). The runs belong to the `text` they were made for: when the cell
/// holds any other text they are stale, and nothing draws or saves them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RichText {
    pub text: Arc<str>,
    pub runs: Vec<TextRun>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextRun {
    /// Characters (Unicode scalar values) the run covers.
    pub len: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<Font>,
}

impl RichText {
    /// `text` cut into its runs, or `None` when the runs were made for other text, don't cover it
    /// exactly, or are too many.
    pub fn spans<'a>(&'a self, text: &'a str) -> Option<Vec<(&'a str, Option<&'a Font>)>> {
        if *self.text != *text || self.runs.len() > MAX_RUNS {
            return None;
        }
        let mut out = Vec::with_capacity(self.runs.len());
        let mut rest = text;
        for run in &self.runs {
            let n = run.len as usize;
            let cut = match rest.char_indices().nth(n) {
                Some((i, _)) => i,
                None if rest.chars().count() == n => rest.len(),
                None => return None,
            };
            let (head, tail) = rest.split_at(cut);
            out.push((head, run.font.as_ref()));
            rest = tail;
        }
        rest.is_empty().then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rich(text: &str, lens: &[u32]) -> RichText {
        RichText { text: text.into(), runs: lens.iter().map(|&len| TextRun { len, font: None }).collect() }
    }

    #[test]
    fn spans_cut_the_text_by_characters_and_refuse_what_does_not_fit() {
        let r = rich("Grüße aus Köln", &[6, 8]);
        let spans: Vec<&str> = r.spans("Grüße aus Köln").unwrap().into_iter().map(|(s, _)| s).collect();
        assert_eq!(spans, ["Grüße ", "aus Köln"]);
        // Other text, runs too short, runs too long.
        assert!(r.spans("Grüße aus Bonn").is_none());
        assert!(rich("abc", &[1, 1]).spans("abc").is_none());
        assert!(rich("abc", &[2, 2]).spans("abc").is_none());
        // Empty runs and a hostile length.
        assert_eq!(rich("ab", &[0, 2, 0]).spans("ab").unwrap().len(), 3);
        assert!(rich("ab", &[u32::MAX]).spans("ab").is_none());
        assert!(rich("", &[]).spans("").unwrap().is_empty());
    }
}
