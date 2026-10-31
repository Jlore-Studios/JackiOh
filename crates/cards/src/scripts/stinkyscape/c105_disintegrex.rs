//! S #105 Disintegrex (SPEC §8.10 row 105): (2) Unit, Epic, 6/3 → 12/6, of the Stinkyscape set.
//!   Base:    "Starship Piece / Aura: Anything that would be destroyed is exiled instead."
//!   Radiant: "Starship Piece / Aura: Any enemy permanent that would be destroyed is exiled instead."
//! Engine: Starship Piece (R1852000) is a catalog keyword. `StaticFlags.destroyed_exiled` (R1849002):
//! while it acts on the field, every card §4.5 step 1 collects that no would-die replacement took is
//! exiled instead — no Death, no Reborn, no `destroyed` (R461) — this card too when it dies in the same
//! pass, every card judged before any moves, so it fuses into no Starship (only a death fuses). A
//! Tribute, a discard and a spent Trap are not destroyed and still reach the graveyard. Radiant:
//! `enemy_destroyed_exiled`, its controller's opponent's cards only, its own side destroyed as usual.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-105";

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            destroyed_exiled: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    let radiant = Script {
        static_flags: Some(StaticFlags {
            enemy_destroyed_exiled: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// S #105 Disintegrex — SPEC §8.10 row 105, BUILD M12 row S 105.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const SAINTESS: &str = "core-081"; // (1) 2/2, "Death: Make your other Units Radiant."
    const DEFENDER: &str = "core-003"; // (1) 1/1 Taunt, Divine Shield, Reborn.
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const SHATTER: &str = "core-036"; // (1) Spell: Destroy a backrow card. Lock its zone.
    const ARMAGEDDON: &str = "core-088"; // (4) Spell: Destroy all permanents.
    const RESERVES: &str = "core-075"; // (0) Field Spell: a backrow card to destroy.
    const ROCK: &str = "core-066"; // (4) 10/10 Tribute 1.

    fn board(radiant: bool, p1_hand: Value, p1_field: Value, p2_field: Value) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend(p1_field.as_array().cloned().unwrap_or_default());
        let mut hand = p1_hand.as_array().cloned().unwrap_or_default();
        hand.push(json!(FILLER));
        crate::scenario(json!({
            "p1": { "hand": hand, "field": field, "library": crate::filler(4) },
            "p2": {
                "hand": [FILLER],
                "field": p2_field,
                "backrow": [RESERVES],
                "library": crate::filler(4),
            },
        }))
    }

    fn exiled(s: &Scenario, card: &CardInstance) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == card.id))
    }

    fn destroyed(s: &Scenario, card: &CardInstance) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == card.id))
    }

    mod base {
        use super::*;

        #[test]
        fn a_dead_unit_and_a_destroyed_backrow_card_are_exiled_with_no_death() {
            let mut s = board(false, json!([HIT_JOB, SHATTER]), json!([SAINTESS, VANILLA]), json!([DEFENDER]));
            let saintess = s.unit_at(P1, 2);
            let vanilla = s.unit_at(P1, 3);
            s.play(HIT_JOB, json!({ "targets": pick::instance(&saintess) }));
            s.expect_in_zone(&saintess, "exile");
            assert!(exiled(&s, &saintess) && !destroyed(&s, &saintess));
            assert!(!s.card(&vanilla.id).radiant, "no Death fired");

            s.state_mut().players.p1.mana.current = 4;
            let reserves = s.backrow(P2, 1).expect("p2's Reserves");
            s.play(SHATTER, json!({ "targets": pick::instance(&reserves) }));
            s.expect_in_zone(&reserves, "exile");
            assert!(!destroyed(&s, &reserves));
        }

        #[test]
        fn a_unit_at_0_health_is_exiled() {
            let mut s = board(false, json!([]), json!([SAINTESS]), json!([MENACE]));
            let saintess = s.unit_at(P1, 2);
            s.attack(&saintess, s.unit_at(P2, 1));
            s.expect_in_zone(&saintess, "exile");
            assert!(exiled(&s, &saintess) && !destroyed(&s, &saintess));
            assert!(!s.card(&s.unit_at(P1, 1).id).radiant, "no Death fired");
        }

        #[test]
        fn itself_in_the_same_pass() {
            let mut s = board(false, json!([ARMAGEDDON]), json!([VANILLA]), json!([DEFENDER]));
            let disintegrex = s.unit_at(P1, 1);
            let vanilla = s.unit_at(P1, 2);
            let defender = s.unit_at(P2, 1);
            let reserves = s.backrow(P2, 1).expect("p2's Reserves");
            s.play(ARMAGEDDON, json!({}));
            for card in [&disintegrex, &vanilla, &reserves] {
                s.expect_in_zone(card, "exile");
            }
            s.expect_in_zone(&defender, "exile");
            assert!(s.unit(P2, 1).is_none(), "no Reborn body");
            // Exiled, so no death: it fuses into no Starship (R1852000).
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
        }

        #[test]
        fn a_tribute_still_reaches_the_graveyard() {
            let mut s = board(false, json!([ROCK]), json!([VANILLA]), json!([]));
            let vanilla = s.unit_at(P1, 2);
            s.play(ROCK, json!({ "zone": 3, "tributes": [vanilla.id] }));
            s.expect_in_zone(&vanilla, "graveyard");
            assert!(!exiled(&s, &vanilla));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn only_the_enemys_are_exiled() {
            let mut s = board(true, json!([ARMAGEDDON]), json!([VANILLA]), json!([VANILLA]));
            let disintegrex = s.unit_at(P1, 1);
            s.expect_stats(&disintegrex, json!({ "attack": 12, "health": 6 }));
            let mine = s.unit_at(P1, 2);
            let theirs = s.unit_at(P2, 1);
            let reserves = s.backrow(P2, 1).expect("p2's Reserves");
            s.play(ARMAGEDDON, json!({}));
            s.expect_in_zone(&theirs, "exile").expect_in_zone(&reserves, "exile");
            s.expect_in_zone(&mine, "graveyard").expect_in_zone(&disintegrex, "graveyard");
            assert!(destroyed(&s, &disintegrex));
        }
    }
}
