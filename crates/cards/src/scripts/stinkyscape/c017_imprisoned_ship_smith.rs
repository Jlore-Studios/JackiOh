//! S #17 Imprisoned Ship Smith (SPEC §8.10 row 17): (1) Unit, Human, Common, 2/2 → 4/4, of the
//! Stinkyscape set.
//!   Base:    "Dormant for 2 turns. When this awakens, summon a Fiery Waraxe."
//!   Radiant: "Dormant for 2 turns. When this awakens, summon a Radiant Fiery Waraxe."
//! Engine: Dormant (R1839000), `StaticFlags.slumber { turns: 2 }`, counted at the start of its
//! controller's turns and awake at the start of the second, before the start-of-turn triggers,
//! summoning sick for that turn (R1839001, R2259000). `when_this_awakens` (R1839002): a fresh M #66 Fiery Waraxe into the
//! leftmost open backrow zone (R64), no Cry (R1); a full backrow summons nothing. It enters on its
//! controller's turn, so it animates as it enters (§6.1 "Animated on your turn"): an entry, so
//! summoning sick (R1062), its Rush letting it attack a Unit that turn. Radiant: a Radiant Waraxe.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-017";

/// M #66 Fiery Waraxe, what the awakening summons.
const WARAXE: &str = "meditative-066";

/// "Dormant for 2 turns": the status's printed count, no param (docs/stinkyscape-set.md, #17).
const DORMANT_TURNS: i32 = 2;

fn ship_smith(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: Some(DORMANT_TURNS),
                wakes_by: None,
            }),
            ..StaticFlags::default()
        }),
        triggers: vec![when_this_awakens("awakens", move |_ctx, _event| {
            vec![summon(json_as(json!({ "defId": WARAXE, "radiant": radiant })))]
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: ship_smith(false),
        radiant: ship_smith(true),
    }
}

// S #17 Imprisoned Ship Smith — SPEC §8.10 row 17, BUILD M12 row S 17.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{DORMANT_TURNS, ID, WARAXE};

    const FILLER: &str = "core-010"; // (0) Spell: a play always open, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const RESERVES: &str = "core-075"; // (0) Field Spell that does nothing here: a backrow zone taken.
    const FARM: &str = "core-058"; // (2) Field Spell: Start of turn: Summon a Rush Token.
    const RUSH_TOKEN: &str = "core-t-rush";
    const WHIRLWIND: &str = "classicplus-021"; // (0) Spell: "Pierce. Deal {damage} damage to all Units."
    const NETHER: &str = "core-088"; // (4) Spell: "Destroy all permanents."

    fn played(radiant: bool, p1_backrow: Value) -> Scenario {
        let mut s = crate::scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "backrow": p1_backrow,
                "library": crate::filler(4),
            },
            "p2": { "hand": [HIT_JOB, FILLER], "field": [VANILLA], "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn to_its_awakening(s: &mut Scenario) {
        s.end_turn().end_turn().end_turn().end_turn();
        assert_eq!(s.state().active, P1);
    }

    fn dormant_for(turns: i32) -> Option<Slumber> {
        Some(Slumber { turns: Some(turns) })
    }

    fn named_by_a_legal_action(s: &Scenario, player: PlayerId, id: &str) -> bool {
        legal_actions(s.state(), player)
            .iter()
            .any(|action| serde_json::to_string(action).expect("serializes").contains(&format!("\"{id}\"")))
    }

    fn first(events: &[GameEvent], matches: impl Fn(&GameEvent) -> bool) -> Option<usize> {
        events.iter().position(matches)
    }

    fn awakened(event: &GameEvent) -> bool {
        matches!(event, GameEvent::Awakened { .. })
    }

    fn summoned_waraxes(s: &Scenario) -> Vec<(Row, i32)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, row, lane, .. } if def_id == WARAXE => Some((*row, *lane)),
                _ => None,
            })
            .collect()
    }

    fn awake_beside_a_dormant_one(radiant: bool) -> Scenario {
        let mut s = crate::scenario(json!({
            "p1": {
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 2 }));
        to_its_awakening(&mut s);
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r2271000_another_cards_awakening_summons_nothing() {
            // "When THIS awakens": the awake Smith let every awakening through, so each one summoned
            // a Waraxe for it too, until its condition was read (R2271000).
            let s = awake_beside_a_dormant_one(false);
            assert_eq!(summoned_waraxes(&s).len(), 1);
        }

        #[test]
        fn r1839000_played_it_lies_dormant_for_2_turns_no_attack_no_switch_no_target() {
            let mut s = played(false, json!([]));
            let smith = s.card(ID).id.clone();
            assert_eq!(s.card(&smith).slumber, dormant_for(DORMANT_TURNS));
            assert!(!named_by_a_legal_action(&s, P1, &smith));
            s.expect_refused(|s| s.attack(&smith, "hero"));

            s.end_turn();
            assert!(!named_by_a_legal_action(&s, P2, &smith));
            s.expect_refused(|s| s.attack(VANILLA, &smith));
        }

        #[test]
        fn r1839000_while_dormant_an_untargeted_effect_passes_it_by() {
            let mut s = crate::scenario(json!({
                "p1": { "hand": [ID, FILLER], "library": crate::filler(4) },
                "p2": { "hand": [WHIRLWIND, NETHER, FILLER], "field": [VANILLA], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            let smith = s.card(ID).id.clone();
            let health = s.stats(&smith).health;
            s.end_turn();
            s.state_mut().players[P2].mana.current = 10;

            let vanilla_health = s.stats(VANILLA).health;
            s.play(WHIRLWIND, json!({}));
            assert!(
                s.stats(VANILLA).health < vanilla_health,
                "the Whirlwind reached the Vanilla"
            );
            assert_eq!(s.stats(&smith).health, health);
            assert!(!s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Damage { target_id, .. } if *target_id == smith
            )));

            let vanilla = s.unit_at(P2, 1).id;
            s.play(NETHER, json!({}));
            s.expect_in_zone(&vanilla, "graveyard");
            assert!(matches!(
                s.card(&smith).zone,
                Zone::Field {
                    player: P1,
                    row: Row::Units,
                    ..
                }
            ));
            assert_eq!(s.card(&smith).slumber, dormant_for(DORMANT_TURNS));
        }

        #[test]
        fn r1839001_r2259000_awakens_at_the_start_of_its_controllers_second_turn_before_the_turns_triggers_sick() {
            let mut s = played(false, json!([FARM]));
            let smith = s.card(ID).id.clone();
            s.end_turn().end_turn();
            assert_eq!(s.card(&smith).slumber, dormant_for(DORMANT_TURNS - 1));
            assert!(first(s.events(), awakened).is_none());

            s.end_turn().end_turn();
            assert_eq!(s.card(&smith).slumber, None);
            let woke = first(s.last_events(), awakened);
            let farm_token = first(
                s.last_events(),
                |event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == RUSH_TOKEN),
            );
            assert!(woke.is_some() && woke < farm_token);
            // Summoning sick the turn it awakens (R2259000); it attacks on its controller's next turn.
            s.expect_refused(|s| s.attack(&smith, "hero"));
            s.end_turn().end_turn();
            s.attack(&smith, "hero");
            s.expect_health(P2, HERO_HEALTH - 2);
        }

        #[test]
        fn r64_its_awakening_summons_a_fiery_waraxe_into_the_leftmost_open_backrow_zone_and_it_animates() {
            let mut s = played(false, json!([{ "def": RESERVES, "lane": 1 }]));
            to_its_awakening(&mut s);

            assert_eq!(summoned_waraxes(&s), vec![(Row::Backrow, 2)]);
            let woke = first(s.events(), awakened);
            let summoned = first(
                s.events(),
                |event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == WARAXE),
            );
            assert!(woke.is_some() && woke < summoned);
            // §6.1: on its controller's turn it animates as it enters, into its own lane's unit zone.
            let waraxe = s.unit_at(P1, 2);
            assert_eq!(waraxe.def_id, WARAXE);
            assert!(!waraxe.radiant);
            s.expect_stats(&waraxe, json!({ "attack": 3, "health": 2 }));
            // An entry, so sick (R1062): its Rush lets it hit a Unit, not the hero.
            s.expect_refused(|s| s.attack(&waraxe, "hero"));
            s.attack(&waraxe, VANILLA);
            s.expect_stats(VANILLA, json!({ "health": 1 }));
        }

        #[test]
        fn a_full_backrow_summons_no_waraxe() {
            let reserves: Vec<Value> = (1..=5).map(|lane| json!({ "def": RESERVES, "lane": lane })).collect();
            let mut s = played(false, json!(reserves));
            to_its_awakening(&mut s);

            assert!(first(s.events(), awakened).is_some());
            assert!(summoned_waraxes(&s).is_empty());
            assert!((1..=5).all(|lane| s.unit(P1, lane).is_none_or(|unit| unit.def_id != WARAXE)));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2271000_another_cards_awakening_summons_nothing_on_the_radiant_face() {
            let s = awake_beside_a_dormant_one(true);
            assert_eq!(summoned_waraxes(&s).len(), 1);
        }

        #[test]
        fn r275_a_4_4_whose_awakening_summons_a_radiant_fiery_waraxe_6_4_with_pierce() {
            let mut s = played(true, json!([]));
            s.expect_stats(ID, json!({ "attack": 4, "health": 4 }));
            to_its_awakening(&mut s);

            assert_eq!(summoned_waraxes(&s), vec![(Row::Backrow, 1)]);
            let waraxe = s.unit_at(P1, 2);
            assert!(waraxe.radiant);
            s.expect_stats(&waraxe, json!({ "attack": 6, "health": 4 }));
            assert!(has_keyword(&s.stats(&waraxe).keywords, KeywordKind::Pierce));
        }
    }
}
