use gridcraft_core::CellError;
use gridcraft_formula::{FormulaLocale, parse, parse_input, print, print_input};

const DE: FormulaLocale = FormulaLocale::De;

fn input(src: &str) -> String {
    print(&parse_input(src, DE, |_| false).unwrap())
}

fn shown(english: &str) -> String {
    print_input(&parse(english).unwrap(), DE, |_| false).unwrap()
}

/// Spellings and separators as Excel de-DE shows them (observed through its FormulaLocal).
#[test]
fn german_error_values_separators_and_booleans() {
    for (german, english) in [
        ("WENN(1;{1,5.2;3.4};#NV)", "IF(1,{1.5,2;3,4},#N/A)"),
        ("ISTFEHLER(#WERT!)+#ZAHL!", "ISERROR(#VALUE!)+#NUM!"),
        ("#BEZUG!", "#REF!"),
        ("#DATEN_ABRUFEN", "#GETTING_DATA"),
        ("#NULL!+#DIV/0!+#NAME?", "#NULL!+#DIV/0!+#NAME?"),
        ("SVERWEIS(A2;D:E;2;FALSCH)", "VLOOKUP(A2,D:E,2,FALSE)"),
        ("ZÄHLENWENNS(A:A;\">0\")+OBERGRENZE.MATHEMATIK(2,5)", "COUNTIFS(A:A,\">0\")+CEILING.MATH(2.5)"),
        ("MAX(A1:A3)+ABS(-1)", "MAX(A1:A3)+ABS(-1)"),
        ("WENN(WAHR;FALSCH;0,5)", "IF(TRUE,FALSE,0.5)"),
    ] {
        assert_eq!(input(german), english, "{german}");
        assert_eq!(shown(english), german, "{english}");
    }
    // Typed in any case; the English spelling is still understood.
    assert_eq!(input("#nv+#wert!+#N/A"), "#N/A+#VALUE!+#N/A");
    assert_eq!(input("=summe(1;2)+zählenwenn(A:A;1)"), "SUM(1,2)+COUNTIF(A:A,1)");
}

#[test]
fn german_error_names_parse_both_ways() {
    for e in CellError::ALL {
        assert_eq!(DE.parse_error(DE.error_name(e)), Some(e), "{e}");
        assert_eq!(FormulaLocale::En.parse_error(e.as_str()), Some(e));
    }
    assert_eq!(DE.error_name(CellError::Spill), "#ÜBERLAUF!");
    assert_eq!(DE.error_name(CellError::Calc), "#CALC!");
    assert_eq!(DE.parse_error("#n/a"), None);
}
