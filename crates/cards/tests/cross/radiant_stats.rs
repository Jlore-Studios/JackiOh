//! SPEC §8's rows hold a card's Radiant stats in two cells: the Stats column's right side (`8/8 →
//! 16/16`) and, when the Radiant effect cell opens with them, that cell (`16/16, plus Pierce`). A
//! balance pass that moves one and not the other leaves the spec saying two things (#3125 left S #94
//! Tmack Trav at "18/18, plus Pierce" against its 16/16, #3795, #3804), so this holds the two cells of
//! each row to each other. It compares the spec with itself, never with the catalog: the catalog is
//! transcribed from the spec, and `catalog.rs` holds that.
//!
//! A Radiant cell written in prose ("Same", "Rush; same", "+4 Armor") is not read; only one that opens
//! with `A/B` is.

/// SPEC §8, read at compile time (`include_str!`: CLAUDE.md rule 4).
const CATALOG_NOTE: &str = include_str!("../../../../spec/08-catalog.md");

/// The header of every §8 card table.
const TABLE_HEADER: &str =
    "| # | Name | Rarity | Cost | Type, tags | Stats | Base effect | Radiant effect | Engine |";

/// The `A/B` a cell opens with, when it opens with one: `18/18` in "18/18, plus Pierce", none in
/// "Same" or in "+4 Armor".
fn leading_stats(cell: &str) -> Option<(u32, u32)> {
    let (attack, rest) = cell.split_once('/')?;
    let attack: u32 = attack.parse().ok()?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let after = rest[digits.len()..].chars().next();
    if after.is_some_and(|c| c.is_alphanumeric()) {
        return None;
    }
    Some((attack, digits.parse().ok()?))
}

/// Every row of `note`'s card tables whose Radiant cell opens with stats that are not its Stats
/// column's right side, as `#<n> <name>: …`; and every row that is not cut into the header's cells.
fn mismatches(note: &str) -> Vec<String> {
    let header: Vec<&str> = TABLE_HEADER
        .trim_matches('|')
        .split(" | ")
        .map(str::trim)
        .collect();
    let stats_at = header.iter().position(|cell| *cell == "Stats").unwrap();
    let radiant_at = header.iter().position(|cell| *cell == "Radiant effect").unwrap();
    let mut problems = Vec::new();
    let mut in_table = false;
    for line in note.lines() {
        if line == TABLE_HEADER {
            in_table = true;
            continue;
        }
        if !line.starts_with('|') {
            in_table = false;
            continue;
        }
        if !in_table || line.starts_with("| ---") {
            continue;
        }
        // An empty cell is written `|  |` and now and then `| |` (S+ #28's Stats): read both as empty.
        let mut row = line.to_string();
        while row.contains("| |") {
            row = row.replace("| |", "|  |");
        }
        let cells: Vec<&str> = row
            .strip_prefix("| ")
            .and_then(|row| row.strip_suffix(" |"))
            .unwrap_or(&row)
            .split(" | ")
            .collect();
        if cells.len() != header.len() {
            problems.push(format!(
                "a row of {} cells, not {}: {line}",
                cells.len(),
                header.len()
            ));
            continue;
        }
        let Some(radiant) = leading_stats(cells[radiant_at]) else {
            continue;
        };
        let right = cells[stats_at]
            .split_once('→')
            .and_then(|(_, right)| leading_stats(right.trim()));
        if right != Some(radiant) {
            problems.push(format!(
                "#{} {}: the Radiant cell says {}/{}, the Stats cell \"{}\"",
                cells[0], cells[1], radiant.0, radiant.1, cells[stats_at]
            ));
        }
    }
    problems
}

#[test]
fn every_radiant_cell_that_opens_with_stats_holds_its_stats_columns_right_side() {
    assert_eq!(mismatches(CATALOG_NOTE), Vec::<String>::new());
    // The rule reads something: §8's rows hold more than twenty such cells.
    let read = CATALOG_NOTE
        .lines()
        .filter(|line| line.starts_with("| ") && line.split(" | ").nth(7).and_then(leading_stats).is_some())
        .count();
    assert!(read > 20, "{read}");
}

#[test]
fn tmack_trav_s_radiant_cell_put_back_to_18_18_fails() {
    let row = CATALOG_NOTE
        .lines()
        .find(|line| line.starts_with("| 94 | Tmack Trav |"))
        .expect("§8.10 row 94");
    assert!(row.contains("| 16/16, plus Pierce"), "{row}");
    let header = TABLE_HEADER;
    let stale = row.replace("| 16/16, plus Pierce", "| 18/18, plus Pierce");
    let stale = format!("{header}\n| --- |\n{stale}\n");
    assert_eq!(
        mismatches(&stale),
        vec!["#94 Tmack Trav: the Radiant cell says 18/18, the Stats cell \"8/8 → 16/16\"".to_string()]
    );
}

#[test]
fn a_cell_opens_with_stats_only_when_it_starts_with_numbers_around_a_slash() {
    assert_eq!(leading_stats("18/18, plus Pierce"), Some((18, 18)));
    assert_eq!(leading_stats("0/14"), Some((0, 14)));
    assert_eq!(leading_stats("Same"), None);
    assert_eq!(leading_stats("Rush; same"), None);
    assert_eq!(leading_stats("+4 Armor"), None);
    assert_eq!(leading_stats("Cry: deal 2/3 of it"), None);
    assert_eq!(leading_stats("3/4x"), None);
}

#[test]
fn an_empty_cell_written_with_one_space_still_counts_as_a_cell() {
    let one_space = format!(
        "{TABLE_HEADER}\n| --- |\n| 28 | Box | Rare | 0 | Spell | | Text | 2/2, plus Rush | Engine |\n"
    );
    assert_eq!(
        mismatches(&one_space),
        vec!["#28 Box: the Radiant cell says 2/2, the Stats cell \"\"".to_string()]
    );
}
