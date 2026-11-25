//! IA #37 Bluff Called (SPEC §8.13 row 37), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Reveals when your opponent plays a Spell: Counter it."
//!   Radiant: "Reveals when your opponent plays a Spell: Counter it, then set a Figment."
//!
//! The opponent's `cardAnnounced` of a Spell in §10.5's window, before the card moves (R70, R448);
//! the Spell never resolves and goes to its graveyard. Every condition is in `when`, never `run`
//! (R61, R99). The Radiant face then sets a plain Figment (`set_figments`).
//!
//! Engine: `counter_play`, a Figment `summon` (R4586000). Built by C2 (#4586).

use jackioh_engine::prelude::*;

use crate::scripts::infinite_assault::c025_1_figment::set_figments;

pub const ID: &str = "infiniteassault-037";

/// The announce this card answers: the opponent's announced Spell.
fn opponents_spell(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let GameEvent::CardAnnounced {
        player,
        instance_id,
        card_type,
        ..
    } = event
    else {
        return None;
    };
    if *player == ctx.controller || *card_type != CardType::Spell {
        return None;
    }
    Some(instance_id.clone())
}

fn bluff_called(figment: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "bluff-called",
            &[GameEventType::CardAnnounced],
            move |ctx, event| {
                let Some(announced) = opponents_spell(ctx, event) else {
                    return vec![];
                };
                let mut effects = vec![counter_play(json_as(json!({
                    "target": { "of": "instance", "instanceId": announced },
                })))];
                if figment {
                    effects.extend(set_figments(1, false));
                }
                effects
            },
        )
        .with_when(|ctx, event| opponents_spell(ctx, event).is_some())],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bluff_called(false),
        radiant: bluff_called(true),
    }
}

// IA #37 Bluff Called — SPEC §8.13 row 37 (R4586000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIGMENT: &str = "infiniteassault-025-1";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn armed(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": [VANILLA],
                "library": crate::filler(4),
            },
            "p2": {
                "hand": [STOCKPILE, VANILLA],
                "library": crate::filler(4),
            },
        }))
    }

    fn figments(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .backrow
            .iter()
            .filter_map(|slot| slot.clone())
            .filter(|card| card.def_id == FIGMENT)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn the_opponents_spell_is_countered_and_never_resolves() {
            let mut s = armed(false, "bluff-base");
            let spell = s.card(STOCKPILE).id.clone();
            let hand_before = s.hand(PlayerId::P2).len();
            s.play(&spell, json!({}));
            s.expect_in_zone(&spell, "graveyard");
            assert_eq!(s.hand(PlayerId::P2).len(), hand_before - 1, "no draw");
            s.expect_events(json!(["cardAnnounced", "trapFired", "countered"]));
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn a_unit_play_leaves_it_set() {
            let mut s = armed(false, "bluff-unit");
            s.play(VANILLA, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "field");
            s.expect_in_zone(VANILLA, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_counter_then_sets_a_figment() {
            let mut s = armed(true, "bluff-radiant");
            let spell = s.card(STOCKPILE).id.clone();
            s.play(&spell, json!({}));
            s.expect_in_zone(&spell, "graveyard");
            assert_eq!(figments(&s).len(), 1, "then a Figment");
            assert!(!figments(&s)[0].radiant, "a plain one");
            s.expect_in_zone(ID, "graveyard");
        }
    }
}
