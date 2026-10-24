# gridcraft-de

A German-focused fork of [GridCraft](https://github.com/storytold/gridcraft), the clean-room,
open-source, Rust-native spreadsheet by storytold and its contributors. All of the original work,
its architecture, its tests and its rules are theirs; this fork only adds to it. GridCraft is
dual-licensed MIT / Apache-2.0, and so is everything here.

If the changes here grow too far from GridCraft to be merged back, this fork gets a name of its own
and keeps crediting GridCraft prominently (README, About window, packaging).

## Why it exists

To use GridCraft day to day in a German workshop: invoices (VK), purchases (EK) and bank sheets
that look up customers in another workbook, printed and saved as PDF, opened and saved again in
Excel. That needs German throughout and Excel's behaviour in the details.

## Branches

| Branch | What it is |
|---|---|
| `main` | A plain mirror of `storytold/gridcraft` `main`. Never committed to here. |
| `german-locale` | This fork: everything below, merged with `main` regularly. |
| `upstream/<topic>` | One focused change cut from `main`, offered to GridCraft as a pull request. |

Keeping up with GridCraft: `git fetch upstream`, then **merge** `upstream/main` into `main` and
`main` into `german-locale` (no rebase, so the history shows what came from where). Conflicts are
decided by the rules under "When GridCraft changes the same thing".

## What this fork changes

| Area | Change | Upstream status |
|---|---|---|
| Formulas | German syntax: function names (read from Excel de-DE), error values, `;` `,` separators, `{1,5.2;3.4}`, `WAHR`/`FALSCH` | overlaps open PRs #1201, #1522 |
| Numbers and dates | `Locale` (de): separators, month and day names, de-DE built-in formats, `/` as the date separator, German entry and editing | overlaps open PR #1569 |
| Interface | German catalog ~1,580 labels, function help for every function, panes and dialog labels routed through the catalog, default names Mappe1/Tabelle1, € accounting button | extends GridCraft's own catalog |
| Messages | German import notes, Analysis ToolPak and Solver reports | new |
| Links | `[1]Sheet!A1` evaluates from cached values; Data › Edit Links; update from the file, network paths, Change Source; values saved back | new: offered upstream |
| Pictures | EMF/WMF bitmaps; embedded-object previews | new: offered upstream |
| Printing | borders under overflowing text, repeat fill (`*`) | new: offered upstream |
| Editing | percent cells edit as `12.5%` | new: offered upstream |

## When GridCraft changes the same thing

1. **GridCraft's version is equivalent or better**: take theirs, drop ours, keep our tests if they
   still pass (they encode Excel de-DE observations).
2. **Ours is closer to Excel**: keep ours, and offer the difference upstream with the observation
   that proves it.
3. **Both differ and neither is clearly right**: check Excel (black-box, as GridCraft's rules
   allow) and follow Excel.
4. GridCraft's rules apply here too: clean-room, never crash, everything is a command, the UI is
   thin, quality gates before every commit.

## Never in this repository

Real workbooks (customers, invoices, bank data). `.gitignore` blocks spreadsheet files; test
workbooks are built in code.
