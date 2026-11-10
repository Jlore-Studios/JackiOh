//! S+ #95 JC Backed (SPEC §8.12 row 95). (3) Spell, Legendary.
//!   Base:    "Fill your hand with random S tier cards (or A tier if none exist). Summon {jlarna}
//!            Jlarna and lose {mana} mana for the rest of the game."
//!   Radiant: "Fill your hand with random S tier cards (or A tier if none exist). Summon {radiant}
//!            Radiant Jlarna for the rest of the game."
//!
//! Cry (R1): add S-tier cards (A-tier when no S card exists, R3538001) until the hand is full, then
//! summon the declared Jlarna (M #89), Radiant on the Radiant face. The base face only ends by
//! lowering max mana for the rest of the game (`change_max_mana`, R3538006).
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

use crate::scripts::meditative::c089_jlarna;

pub const ID: &str = "stinkyscapeplus-095";

/// R3538001: `first` unless its pool is empty, then `then`.
fn tier_or(first: CardTier, then: CardTier) -> CardTier {
    if query(&json_as::<CatalogQueryArgs>(json!({ "tier": first }))).is_empty() {
        then
    } else {
        first
    }
}

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let tier = tier_or(CardTier::S, CardTier::A);
            let mut effects = Vec::new();
            let room = room_in(ctx.state, ctx.controller, OffFieldZone::Hand);
            if room > 0 {
                effects.push(add_random_from_catalog(json_as(json!({
                    "query": { "tier": tier },
                    "count": room,
                }))));
            }
            let jlarna = if radiant {
                param(&*ctx, "radiant")
            } else {
                param(&*ctx, "jlarna")
            };
            for _ in 0..jlarna {
                effects.push(summon(json_as(json!({ "defId": c089_jlarna::ID, "radiant": radiant }))));
            }
            if !radiant {
                effects.push(change_max_mana(ChangeMaxManaArgs {
                    amount: -param(&*ctx, "mana"),
                    player: None,
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S+ #95 JC Backed — SPEC §8.12 row 95 (R3538001, R3538006).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile: a Spell
    const JLARNA: &str = "meditative-089";

    fn table(cards: Vec<(&str, i32, i32)>) -> WinRateTable {
        WinRateTable {
            patch: "v0.3.X".to_string(),
            source: WinRateSource::Provisional,
            cards: cards
                .into_iter()
                .map(|(id, wins, games)| WinRateRow {
                    id: id.to_string(),
                    wins,
                    games,
                })
                .collect(),
        }
    }

    /// (800, 1000) is S tier and (550, 1000) A tier of record.
    fn tiers() -> Vec<(&'static str, i32, i32)> {
        vec![(FILLER, 800, 1000), ("core-008", 550, 1000), (JLARNA, 800, 1000)]
    }

    fn board(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [{ "def": "core-008", "lane": 1 }],
                "library": filler(8),
                "mana": 3,
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": "core-008", "lane": 1 }],
                "library": filler(8),
            },
        }))
    }

    fn jlarna_on_field(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.backrow(player, lane))
            .filter(|card| card.def_id == JLARNA)
            .collect()
    }

    #[test]
    fn r3538001_base_fills_the_hand_from_s_tier_then_summons_two_jlarna() {
        crate::register_all();
        register_win_rates(table(tiers()));
        let mut s = board(false, "jc-backed");
        s.play(ID, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
        assert!(
            added.iter().all(|card| tier_of_record(&card.def_id) == Some(CardTier::S)),
            "every card filled is S tier"
        );
        let jlarna = jlarna_on_field(&s, P1);
        assert_eq!(jlarna.len(), 2, "two Jlarna summoned");
        assert!(jlarna.iter().all(|unit| !unit.radiant));
        clear_overrides();
    }

    #[test]
    fn r3538001_with_no_s_tier_it_fills_from_a_tier() {
        crate::register_all();
        register_win_rates(table(vec![("core-008", 550, 1000), (FILLER, 540, 1000)]));
        let mut s = board(false, "jc-backed-a-tier");
        s.play(ID, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
        let added: Vec<CardInstance> = s.hand(P1).into_iter().skip(1).collect();
        assert!(
            added.iter().all(|card| tier_of_record(&card.def_id) == Some(CardTier::A)),
            "every card filled is A tier"
        );
        clear_overrides();
    }

    #[test]
    fn r3538006_base_loses_max_mana_for_the_rest_of_the_game() {
        crate::register_all();
        register_win_rates(table(tiers()));
        let mut s = board(false, "jc-backed-mana");
        let max = s.state().players[P1].mana.max;
        s.play(ID, json!({}));
        assert_eq!(s.state().players[P1].mana.max, max - 1, "one less max mana");
        assert_eq!(s.state().players[P1].mana.perm_mod, -1);
        s.end_turn();
        s.end_turn();
        let side = &s.state().players[P1];
        assert_eq!(side.mana.perm_mod, -1, "the modifier lasts");
        assert_eq!(
            side.mana.max,
            jackioh_engine::mana::max_mana_for(side),
            "max still recomputed from it past the refresh"
        );
        clear_overrides();
    }

    #[test]
    fn r3538006_radiant_summons_radiant_jlarna_and_keeps_its_mana() {
        crate::register_all();
        register_win_rates(table(tiers()));
        let mut s = board(true, "jc-backed-radiant");
        let max = s.state().players[P1].mana.max;
        s.play(ID, json!({}));
        let jlarna = jlarna_on_field(&s, P1);
        assert_eq!(jlarna.len(), 2, "two Jlarna summoned");
        assert!(jlarna.iter().all(|unit| unit.radiant), "both Radiant");
        assert_eq!(s.state().players[P1].mana.max, max, "no mana lost");
        clear_overrides();
    }
}
