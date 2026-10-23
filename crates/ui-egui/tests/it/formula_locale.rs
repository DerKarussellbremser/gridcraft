//! Language selection must affect UI entry, never the canonical engine/file boundary.
use gridcraft_engine::{
    Session,
    core::{CellError, CellRef, Value},
};
use gridcraft_ui_egui::{SheetApp, UiState, i18n::Language};
use serde_json::json;

fn app() -> SheetApp {
    let mut session = Session::new();
    session.new_workbook();
    let mut app = SheetApp::new(session, Default::default());
    app.ui.language = Language::En;
    app
}
fn formula(app: &SheetApp) -> String {
    app.session.active().unwrap().wb.active().unwrap().cell(CellRef::new(0, 0)).unwrap().formula.as_ref().unwrap().text.to_string()
}
fn enter(app: &mut SheetApp, text: &str) -> bool {
    app.begin_edit(Some(text.into()), false);
    app.commit_edit(0, 0, false, false)
}

#[test]
fn language_controls_entry_display_persistence_and_xlsx_roundtrip() {
    let mut app = app();
    assert!(!enter(&mut app, "=SUM(1;2)"));
    app.run("app.language.set", json!({"language":"es"})).unwrap();
    assert!(enter(&mut app, "=SI(VERDADERO;SUMA(1,5;2,25);0)"));
    assert_eq!(formula(&app), "IF(TRUE,SUM(1.5,2.25),0)");
    let saved = app.run("file.save", json!({"format":"xlsx"})).unwrap();
    app.run("file.open", json!({"name":"locale.xlsx", "base64":saved["base64"]})).unwrap();
    app.begin_edit(None, true);
    assert_eq!(app.editor.as_ref().unwrap().text, "=SI(VERDADERO;SUMA(1,5;2,25);0)");
    assert!(app.commit_edit(0, 0, false, false));
    assert_eq!(formula(&app), "IF(TRUE,SUM(1.5,2.25),0)");
    let state: UiState = serde_json::from_str(&serde_json::to_string(&app.ui).unwrap()).unwrap();
    assert_eq!(state.language, Language::Es);
    app.run("app.language.set", json!({"language":"en"})).unwrap();
    app.begin_edit(None, false);
    assert_eq!(app.editor.as_ref().unwrap().text, "=IF(TRUE,SUM(1.5,2.25),0)");
}

#[test]
fn language_does_not_change_api_syntax_or_an_edit_already_in_progress() {
    let mut app = app();
    app.run("app.language.set", json!({"language":"es"})).unwrap();
    assert!(app.run("cell.set", json!({"cell":"A1","input":"=SUM(1,2)"})).is_ok());
    assert!(app.run("cell.set", json!({"cell":"A2","input":"=SUMA(1;2)"})).is_err());
    app.begin_edit(Some("=SUMA(1,5;2)".into()), false);
    app.run("app.language.set", json!({"language":"en"})).unwrap();
    assert!(app.commit_edit(0, 0, false, false));
    assert_eq!(formula(&app), "SUM(1.5,2)");
}

#[test]
fn defined_and_local_names_survive_localized_editing() {
    let mut app = app();
    app.run("formulas.defineName", json!({"name":"FALSO", "refersTo":"=7"})).unwrap();
    app.run("app.language.set", json!({"language":"es"})).unwrap();
    assert!(enter(&mut app, "=LET(SUMA;LAMBDA(x;x+1);SUMA(2))+FALSO"));
    let original = formula(&app);
    app.begin_edit(None, false);
    assert_eq!(app.editor.as_ref().unwrap().text, "=LET(SUMA;LAMBDA(x;x+1);SUMA(2))+FALSO");
    assert!(app.commit_edit(0, 0, false, false));
    assert_eq!(formula(&app), original);
    assert_eq!(app.session.active().unwrap().wb.active().unwrap().value(CellRef::new(0, 0)), Value::Number(10.0));
}

#[test]
fn localized_argument_hints_ignore_decimal_commas_arrays_and_nested_calls() {
    let mut app = app();
    app.run("app.language.set", json!({"language":"es"})).unwrap();
    for (text, expected) in [("=SI(1,5;SUMA(2;3);", 2), ("=SUMA({1,5\\2;3\\4};", 1), ("=SUMA(Table1[[a],[b]];", 1)] {
        app.begin_edit(Some(text.into()), false);
        assert_eq!(app.editor.as_ref().unwrap().current_function().unwrap().1, expected);
    }
}

#[test]
fn localized_autocomplete_is_rendered_and_inserts_the_selected_name() {
    let mut app = app();
    app.run("app.language.set", json!({"language":"es"})).unwrap();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h.state_mut().begin_edit(Some("=SU".into()), true);
    h.run_steps(4);
    h.input_mut().events.push(egui::Event::Text("M".into()));
    h.run_steps(4);
    assert!(h.state().editor.as_ref().unwrap().autocomplete.iter().any(|n| n == "SUMA"));
    assert!(!h.state().editor.as_ref().unwrap().autocomplete.iter().any(|n| n == "SUM"));
    if let Ok(path) = std::env::var("GRIDCRAFT_LOCALE_SCREENSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
    let ed = h.state_mut().editor.as_mut().unwrap();
    ed.ac_index = ed.autocomplete.iter().position(|n| n == "SUMA").unwrap();
    assert!(ed.accept_autocomplete());
    assert_eq!(ed.text, "=SUMA(");
    h.run_steps(4);
    h.input_mut().events.push(egui::Event::Text("1,5;".into()));
    h.run_steps(4);
    assert_eq!(h.state().editor.as_ref().unwrap().current_function(), Some(("SUMA".into(), 1)));
    if let Ok(path) = std::env::var("GRIDCRAFT_LOCALE_SCREENSHOT") {
        h.render().unwrap().save(std::path::Path::new(&path).with_file_name("spanish-hints.png")).unwrap();
        h.state_mut().cancel_edit();
        h.state_mut().ui.ribbon_tab = "View".into();
        h.run_steps(4);
        h.render().unwrap().save(std::path::Path::new(&path).with_file_name("spanish-setting.png")).unwrap();
    }
    h.state_mut().cancel_edit();
    h.state_mut().run("app.language.set", json!({"language":"en"})).unwrap();
    h.state_mut().begin_edit(Some("=SU".into()), true);
    h.run_steps(4);
    h.input_mut().events.push(egui::Event::Text("M".into()));
    h.run_steps(4);
    assert!(h.state().editor.as_ref().unwrap().autocomplete.iter().any(|n| n == "SUM"));
    assert!(!h.state().editor.as_ref().unwrap().autocomplete.iter().any(|n| n == "SUMA"));
}

#[test]
fn locale_preserves_text_cells_and_existing_entry_conveniences() {
    let mut app = app();
    app.run("app.language.set", json!({"language":"es"})).unwrap();
    assert!(enter(&mut app, "=SUMA(1,5;2"));
    assert_eq!(formula(&app), "SUM(1.5,2)");
    assert!(enter(&mut app, "-1,5"));
    assert_eq!(app.session.active().unwrap().wb.active().unwrap().value(CellRef::new(0, 0)), Value::Number(-1.5));
    app.run("home.numberFormat", json!({"format":"@"})).unwrap();
    assert!(enter(&mut app, "=SUM(1,2)"));
    assert_eq!(app.session.active().unwrap().wb.active().unwrap().value(CellRef::new(0, 0)), Value::text("=SUM(1,2)"));
    app.begin_edit(None, true);
    assert_eq!(app.editor.as_ref().unwrap().text, "=SUM(1,2)");
}

#[test]
fn canonical_engine_templates_enter_the_selected_formula_language() {
    let mut app = app();
    app.run("app.language.set", json!({"language":"es"})).unwrap();
    app.run("formulas.insertFunction", json!({"name":"SUM"})).unwrap();
    assert_eq!(app.editor.as_ref().unwrap().text, "=SUMA(");
    app.cancel_edit();
    app.run("cell.set", json!({"cell":"A1","input":"2"})).unwrap();
    app.run("selection.set", json!({"range":"A2"})).unwrap();
    app.run("formulas.autoSum", json!({"enter":false})).unwrap();
    assert!(app.editor.as_ref().unwrap().text.starts_with("=SUMA("));
    app.cancel_edit();
    // Text supplied by the user is already in the selected UI syntax; do not translate it twice.
    app.begin_edit(Some("=SUMA(1,5;2)".into()), false);
    assert_eq!(app.editor.as_ref().unwrap().text, "=SUMA(1,5;2)");
    assert!(app.commit_edit(0, 0, false, false));
    if let Ok(path) = std::env::var("GRIDCRAFT_LOCALE_SCREENSHOT") {
        app.ui.ribbon_tab = "Formulas".into();
        app.run("formulas.insertFunction", json!({"name":"SUM"})).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
            |ui, app: &mut SheetApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(5);
        h.render().unwrap().save(std::path::Path::new(&path).with_file_name("spanish-ribbon-template.png")).unwrap();
    }
}

#[test]
fn portuguese_interface_retains_its_canonical_formula_fallback() {
    let mut app = app();
    app.run("app.language.set", json!({"code":"pt-BR"})).unwrap();
    assert_eq!(app.ui.language, Language::PtBr);
    assert_eq!(app.ui.language.tr("Data"), "Dados");
    assert!(enter(&mut app, "=SUM(1.5,2.25)"));
    app.begin_edit(None, true);
    assert_eq!(app.editor.as_ref().unwrap().text, "=SUM(1.5,2.25)");
    assert!(!enter(&mut app, "=SUMA(1;2)"));
    let restored: UiState = serde_json::from_str(&serde_json::to_string(&app.ui).unwrap()).unwrap();
    assert_eq!(restored.language, Language::PtBr);
    assert_eq!(Language::Es.tr("Page Layout|Orientation"), "Orientación");
}

#[test]
fn czech_interface_retains_its_canonical_formula_fallback() {
    // The Czech table translates interface labels only: formulas keep canonical English names,
    // `,` separators and `.` decimals until a Czech formula locale exists.
    let mut app = app();
    app.run("app.language.set", json!({"code":"cs-CZ"})).unwrap();
    assert_eq!(app.ui.language, Language::Cs);
    assert_eq!(app.ui.language.tr("Formulas"), "Vzorce");
    assert!(enter(&mut app, "=SUM(1.5,2.25)"));
    app.begin_edit(None, true);
    assert_eq!(app.editor.as_ref().unwrap().text, "=SUM(1.5,2.25)");
    assert!(!enter(&mut app, "=SUMA(1;2)"));
    let restored: UiState = serde_json::from_str(&serde_json::to_string(&app.ui).unwrap()).unwrap();
    assert_eq!(restored.language, Language::Cs);
}

#[test]
fn user_defined_prefix_is_hidden_while_editing_and_kept_on_commit() {
    // Excel shows `_xludf.EOMONTH()` as `=EOMONTH()`; the cell still holds the user-defined call
    // (#726), and committing the edit unchanged must not turn it into the built-in.
    let mut app = app();
    app.run("cell.set", json!({"cell":"A1","input":"=_xludf.EOMONTH()&_xludf.CONCAT(\"_xludf.x\")"})).unwrap();
    app.begin_edit(None, true);
    assert_eq!(app.editor.as_ref().unwrap().text, "=EOMONTH()&CONCAT(\"_xludf.x\")");
    assert!(app.commit_edit(0, 0, false, false));
    assert_eq!(formula(&app), "_xludf.EOMONTH()&_xludf.CONCAT(\"_xludf.x\")");
    assert_eq!(app.session.active().unwrap().wb.active().unwrap().value(CellRef::new(0, 0)), Value::Error(CellError::Name));
    app.run("app.language.set", json!({"language":"es"})).unwrap();
    app.begin_edit(None, true);
    assert_eq!(app.editor.as_ref().unwrap().text, "=EOMONTH()&CONCAT(\"_xludf.x\")");
    assert!(app.commit_edit(0, 0, false, false));
    assert_eq!(formula(&app), "_xludf.EOMONTH()&_xludf.CONCAT(\"_xludf.x\")");
}

#[test]
fn turkish_interface_retains_its_canonical_formula_fallback() {
    let mut app = app();
    app.run("app.language.set", json!({"code":"tr-TR"})).unwrap();
    assert_eq!(app.ui.language, Language::Tr);
    assert_eq!(app.ui.language.tr("Data"), "Veri");
    assert_eq!(app.ui.language.tr("Formulas"), "Formüller");
    assert!(enter(&mut app, "=SUM(1.5,2.25)"));
    app.begin_edit(None, true);
    assert_eq!(app.editor.as_ref().unwrap().text, "=SUM(1.5,2.25)");
    assert!(!enter(&mut app, "=TOPLA(1;2)"));
    let restored: UiState = serde_json::from_str(&serde_json::to_string(&app.ui).unwrap()).unwrap();
    assert_eq!(restored.language, Language::Tr);
}

/// Excel de-DE: `#NV` typed into a cell is the error `#N/A`, edits back as `#NV`, and the
/// English spelling is text there. Files and the API keep English.
#[test]
fn german_error_values_enter_and_edit_in_german() {
    let mut app = app();
    app.run("app.language.set", json!({"language":"de"})).unwrap();
    let a1 = |app: &SheetApp| app.session.active().unwrap().wb.active().unwrap().value(CellRef::new(0, 0));
    assert!(enter(&mut app, "#nv"));
    assert_eq!(a1(&app), Value::Error(CellError::NA));
    app.begin_edit(None, false);
    assert_eq!(app.editor.as_ref().unwrap().text, "#NV");
    app.cancel_edit();
    assert!(enter(&mut app, "=WENN(WAHR;SUMME(1,5;2);#WERT!)"));
    assert_eq!(formula(&app), "IF(TRUE,SUM(1.5,2),#VALUE!)");
    app.begin_edit(None, false);
    assert_eq!(app.editor.as_ref().unwrap().text, "=WENN(WAHR;SUMME(1,5;2);#WERT!)");
    app.cancel_edit();
    assert!(enter(&mut app, "#N/A"));
    assert_eq!(a1(&app), Value::Text("#N/A".into()));
    assert!(enter(&mut app, "'#NV"));
    app.begin_edit(None, false);
    assert_eq!(app.editor.as_ref().unwrap().text, "'#NV");
}

fn a1_text(app: &SheetApp) -> String {
    let d = app.session.active().unwrap();
    let sh = d.wb.active().unwrap();
    let c = CellRef::new(0, 0);
    gridcraft_engine::display::format(&sh.value(c), d.wb.styles.get(sh.style_id(c)).num_fmt.as_str(), &d.wb).text
}

/// Typed and shown as in Excel de-DE (observed): `1.5` is the 1st of May, `1.234` a number,
/// `1.234,5` grouped, `WAHR` a boolean, the English spellings text.
#[test]
fn german_numbers_dates_and_booleans_enter_show_and_edit_in_german() {
    let mut app = app();
    app.run("app.language.set", json!({"language":"de"})).unwrap();
    let a1 = |app: &SheetApp| app.session.active().unwrap().wb.active().unwrap().value(CellRef::new(0, 0));
    let edit = |app: &mut SheetApp| {
        app.begin_edit(None, false);
        let text = app.editor.as_ref().unwrap().text.clone();
        app.cancel_edit();
        text
    };
    for (typed, value, shown, edited) in [
        ("1234,5", Value::Number(1234.5), "1234,5", "1234,5"),
        ("1.234,5", Value::Number(1234.5), "1.234,50", "1234,5"),
        ("1.234", Value::Number(1234.0), "1.234", "1234"),
        ("15.10.2026", Value::Number(46310.0), "15.10.2026", "15.10.2026"),
        ("15.10.26 13:30", Value::Number(46310.5625), "15.10.2026 13:30", "15.10.2026 13:30:00"),
        ("15. Okt 2026", Value::Number(46310.0), "15. Okt 26", "15.10.2026"),
        ("12,5%", Value::Number(0.125), "12,50%", "12,5%"),
        ("wahr", Value::Bool(true), "WAHR", "WAHR"),
        ("TRUE", Value::Text("TRUE".into()), "TRUE", "'TRUE"),
    ] {
        app.run("edit.clearAll", json!({"range":"A1"})).unwrap();
        assert!(enter(&mut app, typed), "{typed}");
        assert_eq!(a1(&app), value, "{typed}");
        assert_eq!(a1_text(&app), shown, "{typed}");
        assert_eq!(edit(&mut app), edited, "{typed}");
        assert!(enter(&mut app, edited), "{typed} again");
        assert_eq!(a1(&app), value, "{typed} edited back");
    }
    // Switching back paints the same values in English, and never dirties the workbook.
    assert!(enter(&mut app, "1.234,5"));
    let doc = app.session.active_mut().unwrap();
    doc.saved = doc.wb.clone();
    app.run("app.language.set", json!({"language":"en"})).unwrap();
    assert_eq!(a1_text(&app), "1,234.50");
    assert!(!app.session.active().unwrap().is_dirty());
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(app.session.active().unwrap().wb.locale, gridcraft_engine::core::Locale::En, "undo brings back no German");
}

/// German Excel starts with `Mappe1` and `Tabelle1`; only an untouched blank workbook is renamed.
#[test]
fn german_default_names() {
    let mut app = app();
    let names = |app: &SheetApp| {
        let d = app.session.active().unwrap();
        (d.title.clone(), d.wb.sheets.iter().map(|s| s.name.clone()).collect::<Vec<_>>())
    };
    app.run("app.language.set", json!({"language":"de"})).unwrap();
    assert_eq!(names(&app), ("Mappe1".to_string(), vec!["Tabelle1".to_string()]));
    assert!(!app.session.active().unwrap().is_dirty());
    app.run("home.insertSheet", json!({})).unwrap();
    assert_eq!(names(&app).1, ["Tabelle1", "Tabelle2"]);
    app.session.new_workbook();
    assert_eq!(names(&app).0, "Mappe2");
    assert!(enter(&mut app, "1"));
    app.run("app.language.set", json!({"language":"en"})).unwrap();
    assert_eq!(names(&app), ("Mappe2".to_string(), vec!["Tabelle1".to_string()]), "a workbook with data keeps its names");
}
