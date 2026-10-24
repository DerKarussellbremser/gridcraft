use gridcraft_formula::{Expr, parse, print};

/// Another workbook's sheets print as Excel saves them: `[1]` never forces quotes.
#[test]
fn external_workbook_references_round_trip() {
    for (src, printed) in [
        ("IF($E$24,VLOOKUP($E$24,[1]Kunden!$A$2:$O$4914,7),)", "IF($E$24,VLOOKUP($E$24,[1]Kunden!$A$2:$O$4914,7),)"),
        ("'[1]Kunden Liste'!A1+[12]Tab.1!B2", "'[1]Kunden Liste'!A1+[12]Tab.1!B2"),
        ("'[1]Kunden'!A1", "[1]Kunden!A1"),
    ] {
        let e = parse(src).unwrap_or_else(|err| panic!("{src}: {err:?}"));
        assert_eq!(print(&e), printed, "{src}");
    }
    // Structured references keep their meaning.
    assert_eq!(print(&parse("SUM(Table1[Col])").unwrap()), "SUM(Table1[Col])");
}

#[test]
fn number_near_f64_max_round_trips() {
    for src in ["=1.7976931348623157E308", "=-1.7976931348623157E308"] {
        let e = parse(src).unwrap();
        let printed = print(&e);
        let back = parse(&printed).unwrap_or_else(|err| panic!("{printed}: {err:?}"));
        assert_eq!(print(&back), printed);
        match (&e, &back) {
            (Expr::Number(a), Expr::Number(b)) => assert_eq!(a, b),
            _ => assert_eq!(e, back),
        }
    }
}
