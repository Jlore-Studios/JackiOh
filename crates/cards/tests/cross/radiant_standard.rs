//! R275 and R276: the Radiant power standard (SPEC §5.2), catalog-wide.
//!
//! R275's standard has two halves. The stat half is mechanical, and this file holds every Unit face
//! to it: a Radiant Unit's attack and health are each at least twice its base face's, a 0 staying 0
//! (#1 Big D-fender, #65.1 Spikey Pillow), and a token summoned X/X (the Bread Token's printed 0/0)
//! passing on its printed face because the card that summons it scales X itself. A "[3X/3X]" face
//! (Classic+ #69 Buff Billy, B2.7) is held to the same factor on its X multiples. The effect half —
//! 100–150% stronger, a broader scope, or an added rider — is a judgement, recorded card by card in
//! docs/radiant-audit.md, and this file proves that document covers every entry.
//!
//! R276: every entry's Radiant face changes it, so no Make Radiant is spent on a card it leaves as it was.
//! The audit is read at compile time (`include_str!`, SURFACE §3: a pure crate's tests read no files).

use jackioh_cards::CATALOG;
use jackioh_engine::{AttackHealth, CardDef, CardType, FaceKind, GLITCH_DEF_ID, fill_params};

fn entries() -> Vec<&'static CardDef> {
    CATALOG.values().collect()
}

/// R275: the factor a Radiant Unit's attack and health are held to.
const STAT_FACTOR: i32 = 2;

/// R275's named exceptions to the stat half, by id, with the reason. None remain, so the record
/// stays empty.
const STAT_EXCEPTIONS: &[(&str, &str)] = &[];

/// `docs/radiant-audit.md`, the card-by-card record of R275's effect half.
const AUDIT: &str = include_str!("../../../../docs/radiant-audit.md");

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
    fn r275_is_recorded_card_by_card_docs_radiant_audit_md_has_one_row_for_every_catalog_entry() {
        let missing: Vec<String> = entries()
            .into_iter()
            .filter(|card| !AUDIT.contains(&format!("| {} | {} |", card.index, card.name)))
            .map(|card| format!("#{} {}", card.index, card.name))
            .collect();
        assert_eq!(missing, Vec::<String>::new());
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
