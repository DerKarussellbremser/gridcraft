//! The notes a file import shows when it opens (`sheet "X": embedded objects … are kept for
//! saving`), in the session's language. Readers write them in English with values in them; each
//! template's `{}` stands for one value, which is carried over as is (a sheet name, a part, a
//! technical reason). The German wording is GridCraft's own.

use gridcraft_core::Locale;

const DE: &[(&str, &str)] = &[
    // XLSX.
    (
        "sheet \"{}\": embedded objects and form controls are not shown; they are kept for saving",
        "Blatt „{}“: Eingebettete Objekte und Formularsteuerelemente werden nicht angezeigt, bleiben beim Speichern aber erhalten.",
    ),
    (
        "sheet \"{}\": pictures in the header and footer are not shown; they are kept for saving",
        "Blatt „{}“: Bilder in Kopf- und Fußzeile werden nicht angezeigt, bleiben beim Speichern aber erhalten.",
    ),
    ("sheet \"{}\": header and footer pictures were dropped: {}", "Blatt „{}“: Bilder in Kopf- und Fußzeile wurden entfernt: {}"),
    (
        "sheet \"{}\": relationships could not be read ({}); unmodelled content was dropped",
        "Blatt „{}“: Verknüpfungen konnten nicht gelesen werden ({}); nicht unterstützte Inhalte wurden entfernt.",
    ),
    (
        "sheet \"{}\": controls and embedded objects could not be read and were dropped",
        "Blatt „{}“: Steuerelemente und eingebettete Objekte konnten nicht gelesen werden und wurden entfernt.",
    ),
    ("sheet \"{}\": controls and embedded objects were dropped: {}", "Blatt „{}“: Steuerelemente und eingebettete Objekte wurden entfernt: {}"),
    ("sheet \"{}\": printer settings were dropped: {}", "Blatt „{}“: Druckereinstellungen wurden entfernt: {}"),
    ("sheet \"{}\": hyperlinks past the first {} cells were dropped", "Blatt „{}“: Links nach den ersten {} Zellen wurden entfernt."),
    (
        "sheet \"{}\": {} cells or rows outside the 1,048,576 × 16,384 grid were dropped",
        "Blatt „{}“: {} Zellen oder Zeilen außerhalb des Rasters von 1.048.576 × 16.384 wurden entfernt.",
    ),
    ("sheet \"{}\": {} were dropped: {}", "Blatt „{}“: {} wurden entfernt: {}"),
    ("sheet \"{}\": {} was dropped: {}", "Blatt „{}“: {} wurde entfernt: {}"),
    ("sheet \"{}\" has no part; skipped", "Blatt „{}“ hat keinen Inhalt und wurde übersprungen."),
    ("sheet \"{}\" is a {} and was skipped", "Blatt „{}“ ist ein {} und wurde übersprungen."),
    ("sheet \"{}\" could not be read: {}", "Blatt „{}“ konnte nicht gelesen werden: {}"),
    ("sheet \"{}\" is damaged ({}); kept {} rows", "Blatt „{}“ ist beschädigt ({}); {} Zeilen wurden übernommen."),
    ("chart sheet \"{}\" was skipped and is not kept on save", "Diagrammblatt „{}“ wurde übersprungen und beim Speichern nicht übernommen."),
    ("chart sheet \"{}\" was dropped: {}", "Diagrammblatt „{}“ wurde entfernt: {}"),
    (
        "links to external workbooks were kept; they are not followed or refreshed",
        "Links auf andere Arbeitsmappen bleiben erhalten, werden aber weder verfolgt noch aktualisiert.",
    ),
    ("links to external workbooks were dropped: {}", "Links auf andere Arbeitsmappen wurden entfernt: {}"),
    ("macros (VBA project) were kept but not executed", "Makros (VBA-Projekt) bleiben erhalten, werden aber nicht ausgeführt."),
    (
        "macros (VBA project) reference a missing part and were dropped",
        "Makros (VBA-Projekt) verweisen auf einen fehlenden Teil und wurden entfernt.",
    ),
    ("macros (VBA project) could not be read and were dropped: {}", "Makros (VBA-Projekt) konnten nicht gelesen werden und wurden entfernt: {}"),
    ("an embedded object (SmartArt, OLE…) was skipped", "Ein eingebettetes Objekt (SmartArt, OLE …) wurde übersprungen."),
    ("a linked (not embedded) picture was skipped", "Ein verknüpftes (nicht eingebettetes) Bild wurde übersprungen."),
    ("grouped shapes are not supported and were skipped", "Gruppierte Formen werden nicht unterstützt und wurden übersprungen."),
    (
        "a histogram, waterfall, treemap… chart without its chart part was skipped",
        "Ein Histogramm-, Wasserfall- oder Treemap-Diagramm … ohne Diagrammteil wurde übersprungen.",
    ),
    ("chart {} has no supported plot and was skipped", "Diagramm {} hat keine unterstützte Darstellung und wurde übersprungen."),
    ("missing picture {}", "Bild {} fehlt."),
    ("missing part {}", "Teil {} fehlt."),
    ("skipped {}: {}", "{} übersprungen: {}"),
    ("{} part {} was dropped: {}", "{}-Teil {} wurde entfernt: {}"),
    ("{} was dropped: {}", "{} wurde entfernt: {}"),
    ("no styles part", "Die Datei enthält keine Formatvorlagen."),
    ("the workbook has no worksheets; added an empty sheet", "Die Arbeitsmappe enthält keine Arbeitsblätter; ein leeres Blatt wurde hinzugefügt."),
    ("some cells refer to missing shared strings", "Einige Zellen verweisen auf fehlende gemeinsame Zeichenfolgen."),
    ("some date cells could not be read", "Einige Datumszellen konnten nicht gelesen werden."),
    ("some numeric cells could not be read", "Einige Zahlenzellen konnten nicht gelesen werden."),
    ("shared strings are damaged ({}); kept {} strings", "Die gemeinsamen Zeichenfolgen sind beschädigt ({}); {} Zeichenfolgen wurden übernommen."),
    (
        "shared strings: {} phonetic readings were dropped because another <si> with the same text already had one",
        "Gemeinsame Zeichenfolgen: {} Lesungen (Furigana) wurden entfernt, weil derselbe Text bereits eine hatte.",
    ),
    (
        "shared strings: {} phonetic runs were dropped (bad sb/eb or no reading)",
        "Gemeinsame Zeichenfolgen: {} Lesungen (Furigana) wurden entfernt (ungültig oder leer).",
    ),
    (
        "a shared formula refers to a missing master cell; kept the cached value",
        "Eine gemeinsame Formel verweist auf eine fehlende Ausgangszelle; der gespeicherte Wert wurde übernommen.",
    ),
    (
        "what-if data tables are not supported; kept their values",
        "Was-wäre-wenn-Datentabellen werden nicht unterstützt; ihre Werte wurden übernommen.",
    ),
    (
        "What-if data tables are not supported; their values were kept.",
        "Was-wäre-wenn-Datentabellen werden nicht unterstützt; ihre Werte wurden übernommen.",
    ),
    ("date filters in AutoFilters are not supported", "Datumsfilter im AutoFilter werden nicht unterstützt."),
    (
        "some Excel 2010 conditional format extensions were skipped",
        "Einige Erweiterungen der bedingten Formatierung aus Excel 2010 wurden übersprungen.",
    ),
    ("conditional format type \"{}\" is not supported", "Die bedingte Formatierung vom Typ „{}“ wird nicht unterstützt."),
    (
        "unmodelled content of a Strict Open XML workbook (custom XML, links, controls…) is not kept on save",
        "Nicht unterstützte Inhalte einer Strict-Open-XML-Arbeitsmappe (benutzerdefiniertes XML, Links, Steuerelemente …) werden beim Speichern nicht übernommen.",
    ),
    (
        "Some formulas use {}; their saved results were kept as values.",
        "Einige Formeln verwenden {}; ihre gespeicherten Ergebnisse wurden als Werte übernommen.",
    ),
    ("Some defined names use {} and were skipped.", "Einige definierte Namen verwenden {} und wurden übersprungen."),
    ("{} cells outside the grid were skipped.", "{} Zellen außerhalb des Rasters wurden übersprungen."),
    (
        "Some cells refer to missing shared strings and were left empty.",
        "Einige Zellen verweisen auf fehlende gemeinsame Zeichenfolgen und blieben leer.",
    ),
    (
        "Some number formats were shorter than their declared length; the characters present were kept.",
        "Einige Zahlenformate waren kürzer als angegeben; die vorhandenen Zeichen wurden übernommen.",
    ),
    ("Macro names were skipped.", "Makronamen wurden übersprungen."),
    // PivotTables.
    ("calculated PivotTable fields are not supported", "Berechnete PivotTable-Felder werden nicht unterstützt."),
    (
        "a PivotTable's cache has no fields; kept as plain cells",
        "Der Cache einer PivotTable hat keine Felder; sie wurde als normale Zellen übernommen.",
    ),
    (
        "a PivotTable's cache has no worksheet source; kept as plain cells",
        "Der Cache einer PivotTable hat keine Quelle im Arbeitsblatt; sie wurde als normale Zellen übernommen.",
    ),
    (
        "a PivotTable's source is in another workbook; its source reference may not resolve",
        "Die Quelle einer PivotTable liegt in einer anderen Arbeitsmappe; ihr Quellbezug lässt sich eventuell nicht auflösen.",
    ),
    (
        "a PivotTable's source reference is missing or invalid; kept as plain cells",
        "Der Quellbezug einer PivotTable fehlt oder ist ungültig; sie wurde als normale Zellen übernommen.",
    ),
    (
        "PivotTables with a {} data source are not supported; kept as plain cells",
        "PivotTables mit einer Datenquelle vom Typ {} werden nicht unterstützt; sie wurden als normale Zellen übernommen.",
    ),
    ("PivotTable in {} has no cache; kept as plain cells", "Die PivotTable in {} hat keinen Cache; sie wurde als normale Zellen übernommen."),
    (
        "PivotTable grouping of \"{}\" is not supported; the field is ungrouped",
        "Die PivotTable-Gruppierung von „{}“ wird nicht unterstützt; die Gruppierung des Felds wurde aufgehoben.",
    ),
    ("PivotTable field \"{}\" (grouped items) is not supported", "Das PivotTable-Feld „{}“ (gruppierte Elemente) wird nicht unterstützt."),
    (
        "PivotTable \"{}\" has no valid location; kept as plain cells",
        "Die PivotTable „{}“ hat keine gültige Position; sie wurde als normale Zellen übernommen.",
    ),
    (
        "PivotTable \"{}\": calculated items and label/value filters are not supported",
        "PivotTable „{}“: Berechnete Elemente sowie Beschriftungs- und Wertfilter werden nicht unterstützt.",
    ),
    ("PivotTable function {} is not supported; using Sum", "Die PivotTable-Funktion {} wird nicht unterstützt; stattdessen wird Summe verwendet."),
    ("PivotTable show-values-as {} is not supported", "Die PivotTable-Wertanzeige {} wird nicht unterstützt."),
    ("PivotTables on sheet \"{}\" could not be read: {}", "Die PivotTables auf Blatt „{}“ konnten nicht gelesen werden: {}"),
    ("{} is not a PivotTable definition; skipped", "{} ist keine PivotTable-Definition und wurde übersprungen."),
    (
        "{} is not a pivot cache definition; its PivotTable was kept as plain cells",
        "{} ist keine PivotTable-Cache-Definition; die PivotTable wurde als normale Zellen übernommen.",
    ),
    // Cell pictures.
    (
        "a cell picture has an external or invalid image relationship; kept its scalar fallback",
        "Ein Bild in einer Zelle verweist auf ein externes oder ungültiges Bild; der Ersatzwert wurde übernommen.",
    ),
    (
        "a cell picture is not a supported PNG/JPEG or exceeds 8192 pixels per side / 16 million pixels; kept its scalar fallback",
        "Ein Bild in einer Zelle ist kein unterstütztes PNG/JPEG oder größer als 8192 Pixel je Seite bzw. 16 Millionen Pixel; der Ersatzwert wurde übernommen.",
    ),
    (
        "cell pictures exceed the 128 MiB workbook limit; remaining pictures kept their scalar fallbacks",
        "Die Bilder in Zellen überschreiten die Grenze von 128 MiB je Arbeitsmappe; für die übrigen Bilder wurden die Ersatzwerte übernommen.",
    ),
    (
        "formula-generated cell pictures were imported as static pictures; their formulas were not retained",
        "Per Formel erzeugte Bilder in Zellen wurden als feste Bilder übernommen; ihre Formeln nicht.",
    ),
    (
        "duplicate XLRICHVALUE metadata types; cell pictures were not loaded",
        "Doppelte XLRICHVALUE-Metadatentypen; Bilder in Zellen wurden nicht geladen.",
    ),
    ("missing XLRICHVALUE future metadata; cell pictures were not loaded", "XLRICHVALUE-Metadaten fehlen; Bilder in Zellen wurden nicht geladen."),
    (
        "missing rich-value data relationship; cell pictures were not loaded",
        "Die Verknüpfung zu den Rich-Value-Daten fehlt; Bilder in Zellen wurden nicht geladen.",
    ),
    (
        "missing rich-value structure or image relationships; cell pictures were not loaded",
        "Rich-Value-Struktur oder Bildverknüpfungen fehlen; Bilder in Zellen wurden nicht geladen.",
    ),
    (
        "some cells have unsupported or missing value metadata; kept their scalar fallback (cell pictures may be missing)",
        "Einige Zellen haben nicht unterstützte oder fehlende Wertmetadaten; ihre Ersatzwerte wurden übernommen (Bilder in Zellen fehlen eventuell).",
    ),
    (
        "some rich values are unsupported or malformed; their scalar fallbacks were retained (cell pictures may be missing)",
        "Einige Rich Values werden nicht unterstützt oder sind fehlerhaft; ihre Ersatzwerte wurden übernommen (Bilder in Zellen fehlen eventuell).",
    ),
    (
        "cell-picture data could not be read ({}); scalar fallbacks were retained",
        "Die Daten der Bilder in Zellen konnten nicht gelesen werden ({}); die Ersatzwerte wurden übernommen.",
    ),
    ("cell-picture {} count does not match its records", "Die Anzahl der Zellbild-{} passt nicht zu ihren Datensätzen."),
    ("conflicting {} relationships; cell pictures may be missing", "Widersprüchliche {}-Verknüpfungen; Bilder in Zellen fehlen eventuell."),
    (
        "external {} relationship is unsupported; cell pictures may be missing",
        "Externe {}-Verknüpfungen werden nicht unterstützt; Bilder in Zellen fehlen eventuell.",
    ),
    ("invalid cell-picture part {}", "Ungültiger Zellbild-Teil {}."),
    ("missing or damaged cell-picture part {}: {}", "Zellbild-Teil {} fehlt oder ist beschädigt: {}"),
    // XLS, XLSB, ODS.
    ("Cell comments in .xls files are not imported yet.", "Kommentare in .xls-Dateien werden noch nicht importiert."),
    ("Conditional formatting in .xls files is not imported yet.", "Bedingte Formatierungen in .xls-Dateien werden noch nicht importiert."),
    ("Data validation in .xls files is not imported yet.", "Datenüberprüfungen in .xls-Dateien werden noch nicht importiert."),
    ("Hyperlinks in .xls files are not imported yet.", "Links in .xls-Dateien werden noch nicht importiert."),
    ("Pictures, charts and shapes in .xls files are not imported yet.", "Bilder, Diagramme und Formen in .xls-Dateien werden noch nicht importiert."),
    (
        "PivotTables in .xls files are not imported yet; their values were kept.",
        "PivotTables in .xls-Dateien werden noch nicht importiert; ihre Werte wurden übernommen.",
    ),
    ("Dialog sheets and unreadable sheets were skipped.", "Dialogblätter und nicht lesbare Blätter wurden übersprungen."),
    ("Invalid XLSB merged ranges were skipped.", "Ungültige verbundene Bereiche aus der XLSB-Datei wurden übersprungen."),
    ("Unsupported XLSB future-record and alternate-content blocks were skipped.", "Nicht unterstützte XLSB-Datenblöcke wurden übersprungen."),
    ("XLSB module sheets were skipped.", "XLSB-Modulblätter wurden übersprungen."),
    ("XLSB non-worksheet sheets were skipped.", "XLSB-Blätter, die keine Arbeitsblätter sind, wurden übersprungen."),
    (
        "XLSB data-only import: formulas become their saved results and will not recalculate. Formatting, charts, images, macros and other features are not imported; dates, currency and percentages may display as numbers. Save a new .xlsx file; XLSB saving is not supported.",
        "XLSB-Import nur der Daten: Formeln werden zu ihren gespeicherten Ergebnissen und nicht neu berechnet. Formatierung, Diagramme, Bilder, Makros und weitere Funktionen werden nicht importiert; Datumsangaben, Währungen und Prozentwerte erscheinen eventuell als Zahlen. Speichern Sie eine neue .xlsx-Datei; das Speichern als XLSB wird nicht unterstützt.",
    ),
    (
        "ODS data-only import: formulas become their saved results and will not recalculate. Source formatting, charts, images and other features are not imported. Save a new .xlsx file; ODS saving is not supported.",
        "ODS-Import nur der Daten: Formeln werden zu ihren gespeicherten Ergebnissen und nicht neu berechnet. Formatierung, Diagramme, Bilder und weitere Funktionen werden nicht importiert. Speichern Sie eine neue .xlsx-Datei; das Speichern als ODS wird nicht unterstützt.",
    ),
    (
        "Some ODS sheet names were shortened or renamed to valid, unique XLSX sheet names.",
        "Einige ODS-Blattnamen wurden gekürzt oder in gültige, eindeutige Namen umbenannt.",
    ),
    (
        "Some ODS values or formula caches were unsupported or missing; their displayed text (or raw value) was kept as text. A formula without either is marked [Unavailable ODS formula result].",
        "Einige ODS-Werte oder gespeicherte Formelergebnisse wurden nicht unterstützt oder fehlten; ihr angezeigter Text (oder Rohwert) wurde als Text übernommen. Eine Formel ohne beides ist mit [Unavailable ODS formula result] markiert.",
    ),
    // The summary line of the open message.
    ("{} additional import warnings.", "{} weitere Importhinweise."),
];

/// `text` in `loc`; `None` keeps it (English, or a note this table doesn't know).
pub(crate) fn localize(loc: Locale, text: &str) -> Option<String> {
    if loc == Locale::En {
        return None;
    }
    DE.iter().find_map(|(en, de)| fill(de, &captures(en, text)?))
}

/// The values `text` puts where `template` has `{}`, or `None` when it doesn't match. Literal
/// pieces are matched left to right; each value is the shortest text up to the next piece.
fn captures(template: &str, text: &str) -> Option<Vec<String>> {
    let pieces: Vec<&str> = template.split("{}").collect();
    let (first, rest) = pieces.split_first()?;
    let mut tail = text.strip_prefix(first)?;
    let mut values = Vec::new();
    for (i, piece) in rest.iter().enumerate() {
        let last = i + 1 == rest.len();
        let at = if last { tail.len().checked_sub(piece.len()).filter(|&a| tail.get(a..) == Some(*piece))? } else { tail.find(piece)? };
        values.push(tail.get(..at)?.to_string());
        tail = tail.get(at + piece.len()..)?;
    }
    (rest.is_empty() && tail.is_empty() || !rest.is_empty()).then_some(values)
}

fn fill(template: &str, values: &[String]) -> Option<String> {
    let mut out = String::new();
    let mut values = values.iter();
    let mut pieces = template.split("{}").peekable();
    while let Some(piece) = pieces.next() {
        out.push_str(piece);
        if pieces.peek().is_some() {
            out.push_str(values.next()?);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn german_import_notes() {
        let de = |t: &str| localize(Locale::De, t);
        assert_eq!(
            de("sheet \"Rechnung KFZ (2)\": embedded objects and form controls are not shown; they are kept for saving").as_deref(),
            Some(
                "Blatt „Rechnung KFZ (2)“: Eingebettete Objekte und Formularsteuerelemente werden nicht angezeigt, bleiben beim Speichern aber erhalten."
            )
        );
        assert_eq!(
            de("links to external workbooks were kept; they are not followed or refreshed").as_deref(),
            Some("Links auf andere Arbeitsmappen bleiben erhalten, werden aber weder verfolgt noch aktualisiert.")
        );
        // A sheet name containing the template's own words still matches as a whole.
        assert_eq!(de("sheet \"a: b\" could not be read: bad zip").as_deref(), Some("Blatt „a: b“ konnte nicht gelesen werden: bad zip"));
        assert_eq!(de("3 additional import warnings.").as_deref(), Some("3 weitere Importhinweise."));
        assert_eq!(de("something new"), None);
        assert_eq!(localize(Locale::En, "no styles part"), None);
        // Every template has as many values in German as in English.
        for (en, de) in DE {
            assert_eq!(en.matches("{}").count(), de.matches("{}").count(), "{en}");
        }
    }
}
