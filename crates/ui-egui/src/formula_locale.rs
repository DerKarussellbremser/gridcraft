//! UI formula syntax boundary. Engine commands, control/API callers and XLSX keep English.
use crate::i18n::Language;
use gridcraft_engine::formula::{self, FormulaLocale, ParseError};
use gridcraft_engine::model::Workbook;

pub(crate) fn canonical(
    text: &str,
    locale: FormulaLocale,
    wb: &Workbook,
    sheet: usize,
    cell: gridcraft_engine::core::CellRef,
) -> Result<String, ParseError> {
    // The editor hid the `_xludf.` prefixes of this cell's formula; confirming it unchanged keeps
    // the user-defined calls instead of binding them to the built-ins of the same name.
    if let Some(stored) = wb.sheet(sheet).and_then(|sh| sh.cell(cell)).filter(|c| c.formula.is_some()).map(|c| c.input_text())
        && has_udf_prefix(&stored)
        && text == display(&stored, locale, wb, sheet)
    {
        return Ok(stored);
    }
    if wb.sheet(sheet).is_some_and(|sh| wb.styles.get(sh.style_id(cell)).num_fmt.as_str() == "@") {
        return Ok(text.to_string());
    }
    // `'[Daten .xlsx]Kunden'!A2`, as the formula bar shows it, is `[1]Kunden!A2` in the file.
    let numbered;
    let text = if text.starts_with('=') && !wb.external_books.is_empty() {
        numbered = book_names(text, locale, wb, false);
        numbered.as_str()
    } else {
        text
    };
    if locale == FormulaLocale::En {
        return Ok(text.to_string());
    }
    // Same formula detection as the engine's `input_to_cell` (no trimming: " =1" is text).
    let body = text;
    if body.starts_with('=')
        || ((body.starts_with('+') || body.starts_with('-')) && body.chars().nth(1).is_some_and(|c| c.is_ascii_alphabetic() || c == '('))
    {
        let known = |name: &str| wb.name(name, sheet).is_some() || wb.table(name).is_some();
        // Match the engine's existing convenience for missing closing parentheses.
        let expr = formula::parse_closing_parens(body, |text| formula::parse_input(text, locale, known));
        return Ok(format!("={}", formula::print(&expr?)));
    }
    // German `#NV` enters the error `#N/A`; the English spelling is text there, as in Excel de-DE.
    if let Some(e) = locale.parse_error(body.trim()) {
        return Ok(e.as_str().to_string());
    }
    if gridcraft_engine::core::CellError::parse(body.trim()).is_some() {
        return Ok(format!("'{body}"));
    }
    if locale == FormulaLocale::De {
        return Ok(german_constant(body, wb).unwrap_or_else(|| text.to_string()));
    }
    // Decimal-comma number entry follows the same preference, without rewriting text or dates.
    let normalized = body.trim().replace(locale.decimal_separator(), ".");
    let number = normalized.strip_suffix('%').unwrap_or(&normalized);
    if number.parse::<f64>().is_ok_and(f64::is_finite) {
        return Ok(normalized);
    }
    Ok(text.to_string())
}

/// A constant typed as in Excel de-DE, in the engine's en-US entry syntax: `WAHR`, `1.234,5`,
/// `15.10.2026 13:30`, `15.10.` (this year), `15. Okt 26`. `1.5` is the 1st of May there, while
/// `1.234` is a number because 234 is no month. `None` keeps the text as typed.
fn german_constant(body: &str, wb: &Workbook) -> Option<String> {
    use gridcraft_engine::core::{Locale, Value, parse::parse_input};
    let t = body.trim();
    for value in [true, false] {
        if t.eq_ignore_ascii_case(Locale::De.boolean(value)) {
            return Some(Locale::En.boolean(value).to_string());
        }
        // English `TRUE` is text there.
        if t.eq_ignore_ascii_case(Locale::En.boolean(value)) {
            return Some(format!("'{body}"));
        }
    }
    let is_number = |s: &str| matches!(parse_input(s, wb.date_system).value, Value::Number(_));
    if let Some(date) = german_date(t).filter(|d| is_number(d)) {
        return Some(date);
    }
    let swapped: String = t
        .chars()
        .map(|c| match c {
            '.' => ',',
            ',' => '.',
            c => c,
        })
        .collect();
    is_number(&swapped).then_some(swapped)
}

/// `15.10.2026` → `10/15/2026`, `15.10.` → `10/15`, `15. Okt 2026` → `15-Oct-2026`, each
/// with an optional time after a space.
fn german_date(t: &str) -> Option<String> {
    use gridcraft_engine::core::Locale;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let mut words = t.split_whitespace();
    let first = words.next()?;
    let date = match first.split('.').collect::<Vec<_>>().as_slice() {
        [d, m, y] if digits(d) && digits(m) && digits(y) => format!("{m}/{d}/{y}"),
        [d, m] | [d, m, ""] if digits(d) && digits(m) => format!("{m}/{d}"),
        [d, ""] if digits(d) => {
            let name = words.next()?.trim_end_matches('.').to_lowercase();
            let month = (1..=12).find(|&i| Locale::De.month(i).to_lowercase() == name || Locale::De.month_short(i).to_lowercase() == name)?;
            let en = Locale::En.month_short(month);
            match words.clone().next().filter(|y| digits(y)) {
                Some(y) => {
                    words.next();
                    format!("{d}-{en}-{y}")
                }
                None => format!("{d}-{en}"),
            }
        }
        _ => return None,
    };
    match words.collect::<Vec<_>>().as_slice() {
        [] => Some(date),
        [time] if time.contains(':') => Some(format!("{date} {}", time.replace(',', "."))),
        _ => None,
    }
}

pub(crate) fn display(text: &str, locale: FormulaLocale, wb: &Workbook, sheet: usize) -> String {
    let shown = display_syntax(text, locale, wb, sheet);
    if wb.external_books.is_empty() || !shown.starts_with('=') { shown } else { book_names(&shown, locale, wb, true) }
}

fn display_syntax(text: &str, locale: FormulaLocale, wb: &Workbook, sheet: usize) -> String {
    // English (and every language without a formula table) shows the stored text, `_xludf.`
    // prefixes aside, and skips a parse per frame.
    if locale == FormulaLocale::En {
        return hide_udf_prefix(text, locale);
    }
    let Some(body) = text.strip_prefix('=') else { return text.to_string() };
    let shown = formula::parse(body)
        .and_then(|expr| formula::print_input(&expr, locale, |name| wb.name(name, sheet).is_some() || wb.table(name).is_some()))
        .map(|body| format!("={body}"))
        .unwrap_or_else(|_| text.to_string());
    hide_udf_prefix(&shown, locale)
}

/// Swaps the `[1]` of another workbook's sheet for its file name (`show`), as Excel shows it:
/// `[1]Kunden!A2` ↔ `'[Daten .xlsx]Kunden'!A2`. Only sheet prefixes change, never text in quotes.
fn book_names(text: &str, locale: FormulaLocale, wb: &Workbook, show: bool) -> String {
    let Some(body) = text.strip_prefix('=') else { return text.to_string() };
    let Ok(tokens) = formula::lexer::tokenize_locale(body, locale) else { return text.to_string() };
    let file = |book: &gridcraft_engine::model::ExternalBook| {
        book.targets.first().map(|t| t.rsplit(['/', '\\']).next().unwrap_or(t).to_string()).unwrap_or_default()
    };
    let mut out = String::from("=");
    let mut from = 0;
    for t in tokens {
        let formula::lexer::Tok::Sheet(name, None) = &t.tok else { continue };
        let Some((inside, sheet)) = name.strip_prefix('[').and_then(|r| r.split_once(']')) else { continue };
        let replacement = if show {
            let Some(book) = inside.parse::<usize>().ok().and_then(|n| wb.external_books.get(n.checked_sub(1)?)) else { continue };
            format!("'[{}]{}'!", file(book).replace('\'', "''"), sheet.replace('\'', "''"))
        } else {
            if inside.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            let Some(n) = wb.external_books.iter().position(|b| file(b).eq_ignore_ascii_case(inside)) else { continue };
            let numbered = format!("[{}]{sheet}", n + 1);
            if formula::printer::needs_quotes(sheet) { format!("'{}'!", numbered.replace('\'', "''")) } else { format!("{numbered}!") }
        };
        out.push_str(body.get(from..t.start).unwrap_or(""));
        out.push_str(&replacement);
        from = t.end;
    }
    out.push_str(body.get(from..).unwrap_or(""));
    out
}

/// What the editor and formula bar start from for a constant: an error value in the formula
/// syntax's spelling (`#NV`), and text that would enter as one quoted (`'#NV`). German shows
/// numbers, dates and booleans as they are typed there (`1234,5`, `15.10.2026 13:30:00`, `WAHR`).
pub(crate) fn edit_text(wb: &Workbook, sh: &gridcraft_engine::model::Sheet, at: gridcraft_engine::core::CellRef, locale: FormulaLocale) -> String {
    use gridcraft_engine::core::{Locale, Value};
    use gridcraft_engine::numfmt::{NumberFormat, format_value_in};
    let code = wb.styles.get(sh.style_id(at)).num_fmt.as_str();
    let de = locale == FormulaLocale::De;
    match sh.cell(at).filter(|c| c.formula.is_none()).map(|c| &c.value) {
        Some(Value::Error(e)) => locale.error_name(*e).to_string(),
        Some(Value::Text(t))
            if code != "@" && (locale.parse_error(t.trim()).is_some() || de && german_constant(t, wb).is_some_and(|c| !c.starts_with('\''))) =>
        {
            format!("'{t}")
        }
        Some(Value::Bool(b)) if de => Locale::De.boolean(*b).to_string(),
        Some(Value::Number(n)) if de && NumberFormat::parse_cached(code).is_date() => {
            let edit = match (*n >= 1.0, n.fract() != 0.0) {
                (true, true) => "dd.mm.yyyy hh:mm:ss",
                (true, false) => "dd.mm.yyyy",
                (false, _) => "hh:mm:ss",
            };
            format_value_in(&Value::Number(*n), &NumberFormat::parse_cached(edit), wb.date_system, Locale::De).text
        }
        Some(Value::Number(_)) if de => {
            let raw = gridcraft_engine::display::edit_text(wb, sh, at);
            raw.chars()
                .map(|c| match c {
                    '.' => ',',
                    ',' => '.',
                    c => c,
                })
                .collect()
        }
        _ => gridcraft_engine::display::edit_text(wb, sh, at),
    }
}

const UDF_PREFIX: &str = "_xludf.";

fn has_udf_prefix(text: &str) -> bool {
    text.to_ascii_lowercase().contains(UDF_PREFIX)
}

/// Excel shows a user-defined call such as `_xludf.EOMONTH()` as `=EOMONTH()`. Only function
/// names lose the prefix: text literals and names keep it.
fn hide_udf_prefix(text: &str, locale: FormulaLocale) -> String {
    let Some(body) = text.strip_prefix('=').filter(|b| has_udf_prefix(b)) else { return text.to_string() };
    let Ok(tokens) = formula::lexer::tokenize_locale(body, locale) else { return text.to_string() };
    let mut out = String::from("=");
    let mut from = 0;
    for t in tokens {
        if matches!(t.tok, formula::lexer::Tok::Func(_))
            && body.get(t.start..t.start + UDF_PREFIX.len()).is_some_and(|p| p.eq_ignore_ascii_case(UDF_PREFIX))
        {
            out.push_str(body.get(from..t.start).unwrap_or(""));
            from = t.start + UDF_PREFIX.len();
        }
    }
    out.push_str(body.get(from..).unwrap_or(""));
    out
}

/// A function's argument hint. The name and separators follow the formula syntax; the argument
/// names follow the interface language, all of them or none, so a hint never mixes two languages.
pub(crate) fn signature(name: &str, locale: FormulaLocale, language: Language) -> Option<String> {
    let english = crate::formula_bar::function_signature(locale.canonical_name(name))?;
    let Some((head, args)) = split_signature(&english) else { return Some(english) };
    let separator = format!("{} ", locale.list_separator());
    let args = match args.iter().map(|arg| translate_argument(head, arg, language)).collect::<Option<Vec<_>>>() {
        Some(translated) => translated.join(&separator),
        None => args.join(&separator),
    };
    Some(format!("{}({args})", locale.function_name(head)))
}

/// A function's description in the interface language (English while untranslated).
pub(crate) fn description(name: &str, locale: FormulaLocale, language: Language) -> Option<String> {
    let english = crate::formula_bar::function_description(locale.canonical_name(name))?;
    Some(language.function_description(&english).map_or(english, str::to_string))
}

/// `SUM(number1, [number2], ...)` -> `SUM` and its arguments as written.
fn split_signature(signature: &str) -> Option<(&str, Vec<&str>)> {
    let (head, rest) = signature.split_once('(')?;
    let inner = rest.strip_suffix(')')?;
    Some((head, if inner.is_empty() { Vec::new() } else { inner.split(", ").collect() }))
}

/// The catalog names of a signature's arguments (`number` for `[number2]`), without `...`; the
/// function help tables are checked against them.
#[cfg(test)]
pub(crate) fn argument_names(signature: &str) -> Vec<&str> {
    split_signature(signature).map(|(_, args)| args.into_iter().map(argument_name).filter(|name| *name != "...").collect()).unwrap_or_default()
}

/// `[number2]` -> `number`. A bracket can open or close on its own (SWITCH's
/// `[default_or_value2, result2]`, LAMBDA's `[parameter1, ...]`).
fn argument_name(arg: &str) -> &str {
    let arg = arg.strip_prefix('[').unwrap_or(arg);
    let arg = arg.strip_suffix(']').unwrap_or(arg);
    arg.trim_end_matches(|c: char| c.is_ascii_digit())
}

/// `[number2]` -> `[número2]`: brackets, index and `...` stay. A `FUNCTION|argument` entry wins
/// over the plain name (PERCENTRANK's `significance` is a digit count, CEILING's a step).
fn translate_argument(function: &str, arg: &str, language: Language) -> Option<String> {
    let name = argument_name(arg);
    if name == "..." {
        return Some(arg.to_string());
    }
    let translated = language.function_argument(&format!("{function}|{name}")).or_else(|| language.function_argument(name))?;
    // `arg` is `[`? + `name` + index + `]`?, so the first match is the name itself.
    Some(arg.replacen(name, translated, 1))
}

/// Avoid suggesting an alias that would call a workbook/table name instead of the function.
pub(crate) fn completion_name<'a>(name: &'a str, locale: FormulaLocale, wb: &Workbook, sheet: usize) -> &'a str {
    let alias = locale.function_name(name);
    if wb.name(alias, sheet).is_some() || wb.table(alias).is_some() { name } else { alias }
}

/// Engine-generated edit templates use canonical syntax. The only incomplete template is
/// the bare function opening emitted by Insert Function; typed user input never enters here.
pub(crate) fn display_template(text: &str, locale: FormulaLocale, wb: &Workbook, sheet: usize) -> String {
    if let Some(name) = text.strip_prefix('=').and_then(|s| s.strip_suffix('('))
        && !name.is_empty()
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '.'))
    {
        return format!("={}(", completion_name(name, locale, wb, sheet));
    }
    display(text, locale, wb, sheet)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hint for `name`, empty when there is none.
    fn sig(name: &str, locale: FormulaLocale, language: Language) -> String {
        signature(name, locale, language).unwrap_or_default()
    }

    /// Hints keep the formula syntax's name and separators and translate the argument names
    /// only when the interface language has them all.
    #[test]
    fn function_help_follows_the_interface_language_and_the_formula_syntax() {
        use FormulaLocale::{En, Es};
        // Spanish help over English formulas: the name and `,` stay as typed; brackets, indexes
        // and `...` survive.
        assert_eq!(sig("IF", En, Language::Es), "IF(prueba_lógica, [valor_si_verdadero], [valor_si_falso])");
        assert_eq!(sig("SUM", En, Language::Es), "SUM(número1, [número2], ...)");
        assert_eq!(description("IF", En, Language::Es).as_deref(), Some("Devuelve un valor según una condición."));
        // Spanish names and separators, from a localized name.
        assert_eq!(sig("suma", Es, Language::Es), "SUMA(número1; [número2]; ...)");
        assert_eq!(description("SUMA", Es, Language::Es).as_deref(), Some("Suma los números."));
        // An argument list Spanish covers only in part stays English instead of mixing (the name
        // is whatever the Spanish formula table calls SUMIFS).
        let sumifs = sig("SUMIFS", Es, Language::Es);
        assert!(sumifs.ends_with("(sum_range; criteria_range1; criteria1; ...)"), "{sumifs}");
        assert_eq!(description("SUMIFS", Es, Language::Es).as_deref(), Some("Adds the cells that meet every condition."));
        assert_eq!(sig("IF", En, Language::En), "IF(logical_test, [value_if_true], [value_if_false])");
        // German: every function has help, in German names and separators.
        use FormulaLocale::De;
        assert_eq!(sig("wenn", De, Language::De), "WENN(Prüfung; [Dann_Wert]; [Sonst_Wert])");
        assert_eq!(sig("SVERWEIS", De, Language::De), "SVERWEIS(Suchwert; Matrix; Spaltenindex; [Bereich_Verweis])");
        assert_eq!(sig("QUANTILSRANG", De, Language::De), "QUANTILSRANG(Matrix; x; [Genauigkeit])");
        assert!(sig("OBERGRENZE", De, Language::De).ends_with("(Zahl; Schritt)"), "{}", sig("OBERGRENZE", De, Language::De));
        assert_eq!(description("SUMME", De, Language::De).as_deref(), Some("Addiert alle Zahlen der Argumente."));
        assert_eq!(description("ERFC", De, Language::De).as_deref(), Some("Komplementäre Fehlerfunktion, 1 - GAUSSFEHLER(x)."));
        for f in gridcraft_engine::cmd::formulas::function_list() {
            let name = f["name"].as_str().unwrap_or_default();
            let english = f["description"].as_str().unwrap_or_default();
            assert_ne!(description(name, De, Language::De).as_deref(), Some(english), "{name} has no German description");
            // Every argument has a German name, so no hint falls back to English.
            let english_signature = crate::formula_bar::function_signature(name).unwrap_or_default();
            for argument in argument_names(&english_signature) {
                assert!(Language::De.function_argument(argument).is_some(), "{name}: no German name for {argument}");
            }
        }
    }
}
