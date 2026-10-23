//! Labels the report tools write into cells (Data Analysis, Solver), in the session's language.
//! German Excel writes its reports in German; the wording here is GridCraft's own, in the usual
//! German statistics and optimisation terms. English needs no table: the labels are English.

use gridcraft_core::Locale;

const DE: &[(&str, &str)] = &[
    // Data Analysis.
    ("ANOVA", "ANOVA"),
    ("Adjusted R Square", "Adjustiertes Bestimmtheitsmaß"),
    ("Anova: Single Factor", "Einfaktorielle Varianzanalyse"),
    ("Average", "Mittelwert"),
    ("Between Groups", "Zwischen den Gruppen"),
    ("Bin", "Klasse"),
    ("Coefficients", "Koeffizienten"),
    ("Count", "Anzahl"),
    ("Cumulative %", "Kumuliert %"),
    ("F Critical one-tail", "Kritischer F-Wert (einseitig)"),
    ("F crit", "Kritischer F-Wert"),
    ("F-Test Two-Sample for Variances", "Zwei-Stichproben-F-Test für Varianzen"),
    ("Frequency", "Häufigkeit"),
    ("Groups", "Gruppen"),
    ("Hypothesized Mean Difference", "Hypothetische Differenz der Mittelwerte"),
    ("Intercept", "Achsenabschnitt"),
    ("Kurtosis", "Kurtosis"),
    ("MS", "MQ"),
    ("Maximum", "Maximum"),
    ("Mean", "Mittelwert"),
    ("Median", "Median"),
    ("Minimum", "Minimum"),
    ("Mode", "Modalwert"),
    ("More", "und größer"),
    ("Multiple R", "Multipler Korrelationskoeffizient"),
    ("Observation", "Beobachtung"),
    ("Observations", "Beobachtungen"),
    ("P(F<=f) one-tail", "P(F<=f) einseitig"),
    ("P(T<=t) one-tail", "P(T<=t) einseitig"),
    ("P(T<=t) two-tail", "P(T<=t) zweiseitig"),
    ("P-value", "p-Wert"),
    ("PROBABILITY OUTPUT", "WAHRSCHEINLICHKEITSAUSGABE"),
    ("Pearson Correlation", "Pearson-Korrelation"),
    ("Percent", "Prozent"),
    ("Percentile", "Perzentil"),
    ("Point", "Punkt"),
    ("Pooled Variance", "Gepoolte Varianz"),
    ("R Square", "Bestimmtheitsmaß"),
    ("RESIDUAL OUTPUT", "RESIDUENAUSGABE"),
    ("Range", "Spannweite"),
    ("Rank", "Rang"),
    ("Regression", "Regression"),
    ("Regression Statistics", "Regressionsstatistik"),
    ("Residual", "Residuum"),
    ("Residuals", "Residuen"),
    ("SS", "QS"),
    ("SUMMARY", "ZUSAMMENFASSUNG"),
    ("SUMMARY OUTPUT", "AUSGABE: ZUSAMMENFASSUNG"),
    ("Sample Variance", "Stichprobenvarianz"),
    ("Significance F", "Signifikanz F"),
    ("Skewness", "Schiefe"),
    ("Source of Variation", "Streuungsursache"),
    ("Standard Deviation", "Standardabweichung"),
    ("Standard Error", "Standardfehler"),
    ("Standard Residuals", "Standardisierte Residuen"),
    ("Sum", "Summe"),
    ("Total", "Gesamt"),
    ("Variance", "Varianz"),
    ("Within Groups", "Innerhalb der Gruppen"),
    ("df", "FG"),
    ("t Critical one-tail", "Kritischer t-Wert (einseitig)"),
    ("t Critical two-tail", "Kritischer t-Wert (zweiseitig)"),
    ("t Stat", "t-Statistik"),
    ("t-Test: Paired Two Sample for Means", "Zweistichproben-t-Test bei abhängigen Stichproben (Paarvergleich)"),
    ("t-Test: Two-Sample Assuming Equal Variances", "Zweistichproben-t-Test unter der Annahme gleicher Varianzen"),
    ("t-Test: Two-Sample Assuming Unequal Variances", "Zweistichproben-t-Test unter der Annahme unterschiedlicher Varianzen"),
    // Solver.
    ("Binary", "Binär"),
    ("Binding", "Bindend"),
    ("Cell", "Zelle"),
    ("Cell Value", "Zellwert"),
    ("Constraints", "Nebenbedingungen"),
    ("Contin", "Stetig"),
    ("Evolutionary", "Evolutionär"),
    ("Final Value", "Endwert"),
    ("Formula", "Formel"),
    ("GridCraft Answer Report", "GridCraft-Antwortbericht"),
    ("GridCraft Limits Report", "GridCraft-Grenzwertbericht"),
    ("GRG Nonlinear", "GRG-Nichtlinear"),
    ("Integer", "Ganzzahlig"),
    ("Lower Limit", "Untergrenze"),
    ("Name", "Name"),
    ("Not Binding", "Nicht bindend"),
    ("Objective Cell", "Zielzelle"),
    ("Objective Result", "Zielergebnis"),
    ("Original Value", "Ausgangswert"),
    ("Simplex LP", "Simplex-LP"),
    ("Slack", "Schlupf"),
    ("Status", "Status"),
    ("Unbounded", "Unbeschränkt"),
    ("Upper Limit", "Obergrenze"),
    ("Value", "Wert"),
    ("Variable Cell", "Variablenzelle"),
    ("Variable Cells", "Variablenzellen"),
    // Solver results, quoted in the Answer report.
    ("Solver could not find a solution that satisfies all constraints.", "Der Solver hat keine Lösung gefunden, die alle Nebenbedingungen erfüllt."),
    (
        "Solver converged to a locally optimal solution. All constraints are satisfied.",
        "Der Solver ist zu einer lokal optimalen Lösung konvergiert. Alle Nebenbedingungen sind erfüllt.",
    ),
    (
        "Solver found an optimal solution. All constraints are satisfied.",
        "Der Solver hat eine optimale Lösung gefunden. Alle Nebenbedingungen sind erfüllt.",
    ),
    (
        "Solver stopped at its evaluation limit without finding a feasible solution.",
        "Der Solver hat seine Höchstzahl an Auswertungen erreicht, ohne eine zulässige Lösung zu finden.",
    ),
    (
        "Solver stopped at its evaluation limit; the best solution found satisfies the constraints.",
        "Der Solver hat seine Höchstzahl an Auswertungen erreicht; die beste gefundene Lösung erfüllt die Nebenbedingungen.",
    ),
    (
        "Solver stopped when the best solution stopped improving. All constraints are satisfied.",
        "Der Solver hat angehalten, weil sich die beste Lösung nicht mehr verbessert hat. Alle Nebenbedingungen sind erfüllt.",
    ),
    (
        "The population converged. The best solution found satisfies all constraints.",
        "Die Population ist konvergiert. Die beste gefundene Lösung erfüllt alle Nebenbedingungen.",
    ),
    ("The objective can grow without limit: the problem is unbounded.", "Das Ziel kann unbegrenzt wachsen: Das Problem ist unbeschränkt."),
];

/// Labels built around a value: `Column 2`, `Lower 95.0%`, `Worksheet: Sales`… (prefix, suffix,
/// German prefix, German suffix, whether the value must be a number). The value keeps its text,
/// with a decimal comma in German; a numbered label never matches user text such as `Columnist`.
const DE_AROUND: &[(&str, &str, &str, &str, bool)] = &[
    ("Column ", "", "Spalte ", "", true),
    ("Column", "", "Spalte", "", true),
    ("Confidence Level(", "%)", "Konfidenzniveau (", " %)", true),
    ("Largest(", ")", "Größter(", ")", true),
    ("Smallest(", ")", "Kleinster(", ")", true),
    ("Lower ", "%", "Untere ", " %", true),
    ("Upper ", "%", "Obere ", " %", true),
    ("Predicted ", "", "Schätzung ", "", false),
    ("X Variable ", "", "X-Variable ", "", true),
    ("Worksheet: ", "", "Arbeitsblatt: ", "", false),
    ("Result: ", "", "Ergebnis: ", "", false),
    ("Engine: ", "", "Lösungsmethode: ", "", false),
    ("Objective Cell (", ")", "Zielzelle (", ")", false),
    ("Value Of ", "", "Wert von ", "", true),
    ("Answer Report ", "", "Antwortbericht ", "", true),
    ("Limits Report ", "", "Grenzwertbericht ", "", true),
];

/// `text` as a report written in `loc` shows it; `None` keeps it as is (English, user data, or
/// a label this table doesn't know).
pub(crate) fn localize(loc: Locale, text: &str) -> Option<String> {
    if loc == Locale::En {
        return None;
    }
    if let Some((_, de)) = DE.iter().find(|(en, _)| *en == text) {
        return Some((*de).to_string());
    }
    // Solver's constraint column: `$B$2=Integer`, `$B$2<=2.5`.
    if let Some((cell, rhs)) = text.split_once('=').filter(|(c, _)| c.starts_with('$')) {
        return Some(match rhs {
            "Integer" => format!("{cell}=Ganzzahl"),
            "Binary" => format!("{cell}=Binär"),
            _ => format!("{cell}={}", decimal_comma(rhs)),
        });
    }
    for (prefix, suffix, de_prefix, de_suffix, numeric) in DE_AROUND {
        let Some(inner) = text.strip_prefix(prefix).and_then(|t| t.strip_suffix(suffix)) else { continue };
        if inner.is_empty() || *numeric && inner.parse::<f64>().is_err() {
            continue;
        }
        // The inner part may itself be a label (`Result: Solver found…`, `Objective Cell (Max)`).
        let inner = localize(loc, inner).unwrap_or_else(|| decimal_comma(inner));
        return Some(format!("{de_prefix}{inner}{de_suffix}"));
    }
    None
}

/// `95.0` → `95,0`; text with letters (a sheet name, `Y`) stays as it is.
fn decimal_comma(text: &str) -> String {
    if text.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | 'E' | 'e' | '+')) { text.replace('.', ",") } else { text.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn german_report_labels() {
        let de = |t: &str| localize(Locale::De, t).unwrap_or_else(|| t.to_string());
        assert_eq!(de("Mean"), "Mittelwert");
        assert_eq!(de("Lower 95.0%"), "Untere 95,0 %");
        assert_eq!(de("Confidence Level(95.0%)"), "Konfidenzniveau (95,0 %)");
        assert_eq!(de("Column 2"), "Spalte 2");
        assert_eq!(de("X Variable 1"), "X-Variable 1");
        assert_eq!(de("GridCraft Answer Report"), "GridCraft-Antwortbericht");
        assert_eq!(de("Worksheet: Sales"), "Arbeitsblatt: Sales");
        assert_eq!(de("Objective Cell (Value Of 2.5)"), "Zielzelle (Wert von 2,5)");
        assert_eq!(de("Engine: Simplex LP"), "Lösungsmethode: Simplex-LP");
        assert_eq!(
            de("Result: Solver found an optimal solution. All constraints are satisfied."),
            "Ergebnis: Der Solver hat eine optimale Lösung gefunden. Alle Nebenbedingungen sind erfüllt."
        );
        assert_eq!(de("$B$2=Integer"), "$B$2=Ganzzahl");
        assert_eq!(de("$B$2<=2.5"), "$B$2<=2,5");
        assert_eq!(de("$B$2>=$C$4"), "$B$2>=$C$4");
        assert_eq!(localize(Locale::De, "Columnist"), None);
        assert_eq!(de("Answer Report 1"), "Antwortbericht 1");
        // User data and English stay.
        assert_eq!(localize(Locale::De, "Q1"), None);
        assert_eq!(localize(Locale::En, "Mean"), None);
    }
}
