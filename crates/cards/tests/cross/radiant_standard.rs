//! R275 and R276: the Radiant power standard (SPEC §5.2), catalog-wide.
//! R275's standard has two halves. The stat half is mechanical, and this file holds every Unit face
//! to it: a Radiant Unit's attack and health are each at least twice its base face's, a 0 staying 0
//! (#1 Big D-fender, #65.1 Spikey Pillow), and a token summoned X/X (the Bread Token's printed 0/0)
//! passing on its printed face because the card that summons it scales X itself. A "[3X/3X]" face
//! (Classic+ #69 Buff Billy, B2.7) is held to the same factor on its X multiples. The effect half —
//! 100–150% stronger, a broader scope, or an added rider — is a judgement, recorded card by card in
//! docs/radiant-audit.md, one row per entry or, for a card added since, one id-keyed row with a verdict
//! and its reasoning (#3111), and this file proves that document covers every entry.
//! R276: every entry's Radiant face changes it, so no Make Radiant is spent on a card it leaves as it was.
//! The audit is read at compile time (`include_str!`, SURFACE §3: a pure crate's tests read no files).

use jackioh_cards::CATALOG;
use jackioh_engine::{AttackHealth, CardDef, CardType, FaceKind, GLITCH_DEF_ID, fill_params};

fn entries() -> Vec<&'static CardDef> {
    CATALOG.values().collect()
}

/// R275: the factor a Radiant Unit's attack and health are held to.
const STAT_FACTOR: i32 = 2;

/// R275's named exceptions to the stat half, by id, with the reason; none remain.
const STAT_EXCEPTIONS: &[(&str, &str)] = &[];

/// `docs/radiant-audit.md`, the card-by-card record of R275's effect half.
const AUDIT: &str = include_str!("../../../../docs/radiant-audit.md");

/// The header of the audit's last table, the one a card added since #3111 gets its row in:
/// `| <id> | <name> | <verdict> | <reasoning> |`. `cargo jackioh names` writes the name cell.
const LATER_HEADER: &str = "| Id | Card | Verdict | Reasoning |";

/// The id-keyed rows under `LATER_HEADER`, each as its four trimmed cells, the last holding any `|` the
/// reasoning has; a row of fewer than four cells has the missing ones empty.
fn later_rows(audit: &str) -> Vec<Vec<&str>> {
    let lines: Vec<&str> = audit.lines().collect();
    let Some(head) = lines.iter().position(|line| *line == LATER_HEADER) else {
        return Vec::new();
    };
    lines
        .get(head + 2..)
        .unwrap_or_default()
        .iter()
        .take_while(|line| line.starts_with("| "))
        .map(|line| {
            let inner = line.trim().strip_prefix('|').unwrap_or(line);
            let inner = inner.strip_suffix('|').unwrap_or(inner);
            let mut cells: Vec<&str> = inner.splitn(4, '|').map(str::trim).collect();
            cells.resize(4, "");
            cells
        })
        .collect()
}

/// R275: what keeps `audit` from covering the catalog, one line per gap. A card is covered by its row in
/// the sections above (`| <index> | <name> |`) or by an id-keyed row in the last table, and a row there
/// holds the judgment: a verdict and its reasoning, which no tool writes.
fn audit_gaps(audit: &str) -> Vec<String> {
    let later = later_rows(audit);
    let mut gaps: Vec<String> = Vec::new();
    for card in entries() {
        let covered = audit.contains(&format!("| {} | {} |", card.index, card.name))
            || later.iter().any(|row| row[0] == card.id);
        if !covered {
            gaps.push(format!("{}: no row", card.id));
        }
    }
    for (at, row) in later.iter().enumerate() {
        let id = row[0];
        let Some(card) = CATALOG.get(id) else {
            gaps.push(format!("{id}: not a catalog id"));
            continue;
        };
        if later[..at].iter().any(|before| before[0] == id) {
            gaps.push(format!("{id}: two rows"));
        }
        if row[1] != card.name {
            gaps.push(format!(
                "{id}: card cell {:?}, catalog name {:?}: run cargo jackioh names",
                row[1], card.name
            ));
        }
        if row[2].is_empty() {
            gaps.push(format!("{id}: no verdict"));
        }
        if row[3].is_empty() {
            gaps.push(format!("{id}: no reasoning"));
        }
    }
    gaps
}

fn is_exception(id: &str) -> bool {
    STAT_EXCEPTIONS.iter().any(|(exception, _)| *exception == id)
}

mod r275_the_radiant_power_standard_spec_5_2 {
    use super::*;

    #[test]
    fn r275_gives_every_radiant_unit_at_least_twice_its_base_attack_and_health_a_0_staying_0() {
        let mut short: Vec<String> = Vec::new();
        for card in entries() {
            if card.type_ != CardType::Unit || is_exception(&card.id) {
                continue;
            }
            let mut base = AttackHealth {
                attack: card.base.attack.unwrap_or(0),
                health: card.base.health.unwrap_or(0),
            };
            let mut radiant = AttackHealth {
                attack: card.radiant.attack.unwrap_or(0),
                health: card.radiant.health.unwrap_or(0),
            };
            // B2.7: a "[3X/3X]" face's stats are its X multiples.
            let base_x = card.base.x_stats;
            let radiant_x = card.radiant.x_stats;
            if base_x.is_some() || radiant_x.is_some() {
                base = base_x.unwrap_or_default();
                radiant = radiant_x.unwrap_or_default();
            }
            for (stat, base_value, radiant_value) in [
                ("attack", base.attack, radiant.attack),
                ("health", base.health, radiant.health),
            ] {
                if radiant_value < STAT_FACTOR * base_value {
                    short.push(format!(
                        "{} {}: radiant {stat} {radiant_value} < {STAT_FACTOR} × {base_value}",
                        card.id, card.name
                    ));
                }
            }
        }
        assert_eq!(short, Vec::<String>::new());
    }

    #[test]
    fn r275_names_its_exceptions_and_each_one_is_a_unit_that_really_is_below_the_stat_half() {
        for (id, reason) in STAT_EXCEPTIONS {
            let card = CATALOG.get(*id);
            assert_eq!(
                card.map(|card| card.type_),
                Some(CardType::Unit),
                "{id}: {reason}"
            );
            let below = card.is_some_and(|card| {
                card.radiant.attack.unwrap_or(0) < STAT_FACTOR * card.base.attack.unwrap_or(0)
                    || card.radiant.health.unwrap_or(0) < STAT_FACTOR * card.base.health.unwrap_or(0)
            });
            assert!(below, "{id} is below the stat half, or it needs no exception");
        }
        assert_eq!(
            STAT_EXCEPTIONS.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn r1149002_jlockeed_shredder_10_is_no_exception_since_issue_1219_its_radiant_12_16_doubles_its_6_8() {
        let card = CATALOG.get("core-013").expect("Jlockeed Shredder-10");
        // Issue #1219 raised the Radiant Attack from 8 to 12, so both halves double again; issue #3136
        // lowered the Health from 10 (Radiant 20) to 8 (Radiant 16), doubled still.
        assert_eq!((card.base.attack, card.radiant.attack), (Some(6), Some(12)));
        assert_eq!((card.base.health, card.radiant.health), (Some(8), Some(16)));
        assert!(!is_exception(&card.id));
    }

    #[test]
    fn r275_is_recorded_card_by_card_docs_radiant_audit_md_has_one_row_for_every_catalog_entry() {
        assert_eq!(audit_gaps(AUDIT), Vec::<String>::new());
    }

    #[test]
    fn r275_a_card_with_no_row_fails_the_gate() {
        let gaps = audit_gaps("");
        assert_eq!(gaps.len(), CATALOG.len());
        assert!(gaps.contains(&"core-001: no row".to_string()));
    }

    #[test]
    fn r275_an_id_keyed_row_covers_its_card_only_with_a_verdict_and_a_reasoning() {
        let table = |rows: &[&str]| {
            let mut text = format!("{LATER_HEADER}\n| --- | --- | --- | --- |\n");
            for row in rows {
                text.push_str(row);
                text.push('\n');
            }
            text
        };
        let gaps_of = |rows: &[&str]| -> Vec<String> {
            audit_gaps(&table(rows))
                .into_iter()
                .filter(|gap| gap.starts_with("core-001") || gap.starts_with("core-9999"))
                .collect()
        };
        let name = &CATALOG.get("core-001").expect("Big D-fender").name;

        let covered =
            format!("| core-001 | {name} | Meets | Its Aura is standing, so it is raised in scope. |");
        assert_eq!(gaps_of(&[covered.as_str()]), Vec::<String>::new());

        let no_verdict = format!("| core-001 | {name} |  | A reason. |");
        assert_eq!(
            gaps_of(&[no_verdict.as_str()]),
            vec!["core-001: no verdict".to_string()]
        );

        let no_reasoning = format!("| core-001 | {name} | Meets |  |");
        assert_eq!(
            gaps_of(&[no_reasoning.as_str()]),
            vec!["core-001: no reasoning".to_string()]
        );

        let cut_short = format!("| core-001 | {name} | Meets |");
        assert_eq!(
            gaps_of(&[cut_short.as_str()]),
            vec!["core-001: no reasoning".to_string()]
        );

        let wrong_name = "| core-001 | Old name | Meets | A reason. |";
        let gaps = gaps_of(&[wrong_name]);
        assert_eq!(gaps.len(), 1);
        assert!(gaps[0].starts_with("core-001: card cell"), "{gaps:?}");

        let unknown = "| core-9999 | Nobody | Meets | A reason. |";
        assert_eq!(
            gaps_of(&[unknown]),
            vec![
                "core-001: no row".to_string(),
                "core-9999: not a catalog id".to_string()
            ]
        );

        let twice = [covered.as_str(), covered.as_str()];
        assert_eq!(gaps_of(&twice), vec!["core-001: two rows".to_string()]);

        let bare_header = LATER_HEADER;
        assert!(audit_gaps(bare_header).contains(&"core-001: no row".to_string()));
    }
}

mod r276_every_card_has_a_radiant_face_spec_5_2 {
    use super::*;

    #[test]
    fn r276_changes_every_card_by_making_it_radiant_text_stats_or_keywords_differ() {
        // R349: a card that prints no Radiant form (`radiantFallback`, the Ghoul Token) changes by
        // doubling its stats, the X/X it is summoned with included, so its printed 0/0 reads alike on
        // both faces; `t_ghoul.rs`'s tests prove the doubling in play. Every other card differs in
        // print. A face's text is read with its own `params` values filled in (B3.4 rule 5).
        let unchanged: Vec<String> = entries()
            .into_iter()
            .filter(|card| {
                card.radiant_fallback != Some(true)
                    // R674: Glitch is blank on both faces, the one card a Make Radiant leaves as it was.
                    && card.id != GLITCH_DEF_ID
                    && fill_params(card, FaceKind::Radiant, None) == fill_params(card, FaceKind::Base, None)
                    && card.radiant.x_stats == card.base.x_stats
                    && card.radiant.attack == card.base.attack
                    && card.radiant.health == card.base.health
                    && card.radiant.keywords == card.base.keywords
            })
            .map(|card| format!("{} {}", card.id, card.name))
            .collect();
        assert_eq!(unchanged, Vec::<String>::new());
    }

    #[test]
    fn r276_gives_a_radiant_text_to_every_card_whose_base_face_has_one() {
        let blank: Vec<String> = entries()
            .into_iter()
            .filter(|card| !card.base.text.is_empty() && card.radiant.text.is_empty())
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(blank, Vec::<String>::new());
    }
}
