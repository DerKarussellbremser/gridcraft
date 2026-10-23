//! How numbers, dates and booleans are written for display: the regional conventions Excel takes
//! from the system. Format codes, formulas and files never depend on it.

use crate::date::{MONTHS, WEEKDAYS};

/// Display conventions. `De` is Excel de-DE as observed: `1.234,56`, `15. Okt 26`, `WAHR`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Locale {
    #[default]
    En,
    De,
}

const DE_MONTHS: [&str; 12] = ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"];
// Observed `mmm`: March is `Mrz`, not the first three letters of `März`.
const DE_MONTHS_SHORT: [&str; 12] = ["Jan", "Feb", "Mrz", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez"];
const DE_WEEKDAYS: [&str; 7] = ["Sonntag", "Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag"];

impl Locale {
    pub fn decimal(self) -> char {
        if self == Self::De { ',' } else { '.' }
    }
    pub fn thousands(self) -> char {
        if self == Self::De { '.' } else { ',' }
    }
    /// `mmmm`; `month` is 1-based.
    pub fn month(self, month: u32) -> &'static str {
        let i = (month as usize).wrapping_sub(1);
        if self == Self::De { DE_MONTHS.get(i) } else { MONTHS.get(i) }.copied().unwrap_or("")
    }
    /// `mmm`.
    pub fn month_short(self, month: u32) -> &'static str {
        let i = (month as usize).wrapping_sub(1);
        if self == Self::De { DE_MONTHS_SHORT.get(i).copied().unwrap_or("") } else { MONTHS.get(i).and_then(|m| m.get(..3)).unwrap_or("") }
    }
    /// `dddd`; `weekday` is 0 for Sunday.
    pub fn weekday(self, weekday: u32) -> &'static str {
        if self == Self::De { DE_WEEKDAYS.get(weekday as usize) } else { WEEKDAYS.get(weekday as usize) }.copied().unwrap_or("")
    }
    /// `ddd`: `So`, `Mo`… in German, `Sun`, `Mon`… in English.
    pub fn weekday_short(self, weekday: u32) -> &'static str {
        let name = self.weekday(weekday);
        name.get(..if self == Self::De { 2 } else { 3 }).unwrap_or(name)
    }
    pub fn boolean(self, value: bool) -> &'static str {
        match (self, value) {
            (Self::De, true) => "WAHR",
            (Self::De, false) => "FALSCH",
            (Self::En, true) => "TRUE",
            (Self::En, false) => "FALSE",
        }
    }
    /// Default names: `Mappe1` and `Tabelle1` in German Excel.
    pub fn book_name(self) -> &'static str {
        if self == Self::De { "Mappe" } else { "Book" }
    }
    pub fn sheet_name(self) -> &'static str {
        if self == Self::De { "Tabelle" } else { "Sheet" }
    }
    /// General output (`1234.5`, `1.23457E-05`) with this decimal separator. General never
    /// groups thousands, so the point is the only character that changes.
    pub fn general(self, text: String) -> String {
        if self == Self::De { text.replace('.', ",") } else { text }
    }
}

#[cfg(test)]
mod tests {
    use super::Locale;

    #[test]
    fn names_and_separators() {
        let de = Locale::De;
        assert_eq!((de.decimal(), de.thousands()), (',', '.'));
        assert_eq!((de.month(3), de.month_short(3), de.month_short(5)), ("März", "Mrz", "Mai"));
        assert_eq!((de.weekday(4), de.weekday_short(4), de.weekday_short(0)), ("Donnerstag", "Do", "So"));
        assert_eq!((Locale::En.month_short(9), Locale::En.weekday_short(4)), ("Sep", "Thu"));
        assert_eq!((de.month(0), de.month(13), de.weekday(7)), ("", "", ""));
        assert_eq!(de.general("-1.23457E-05".into()), "-1,23457E-05");
    }
}
