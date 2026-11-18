//! S #105 Disintegrex (SPEC §8.10 row 105): (2) Unit, Epic, 6/3 → 12/6, of the Stinkyscape set.
//!   Base:    "Starship Piece / Aura: Anything that would be destroyed is exiled instead."
//!   Radiant: "Starship Piece / Aura: Any enemy permanent that would be destroyed is exiled instead."
//! Engine: Starship Piece (R1852000) is a catalog keyword. `StaticFlags.destroyed_exiled` (R1849002):
//! while it acts on the field, every card §4.5 step 1 collects that no would-die replacement took is
//! exiled instead of going to its graveyard, this card too when it dies in the same pass, every card
//! judged before any moves. Each was destroyed all the same (R4207000): its Death fires, Reborn returns
//! it from exile, `destroyed` reports it, and this card, a Starship Piece, still fuses. A Tribute, a
//! discard and a spent Trap are not destroyed and still reach the graveyard. Radiant:
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

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const SAINTESS: &str = "core-081"; // Radiant Saintess: "Death: Make your other Units Radiant."
    const DEFENDER: &str = "core-003"; // Right-house defender: Taunt, Divine Shield, Reborn
    const MENACE: &str = "core-019"; // Midrange Menace: Taunt
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.
    const SHATTER: &str = "core-036"; // Magic Jammed: Destroy a backrow card. Lock its zone.
    const ARMAGEDDON: &str = "core-088"; // Twisting Nether: Destroy all permanents.
    const RESERVES: &str = "core-075"; // Infinite Reserves: a backrow card to destroy
    const ROCK: &str = "core-066"; // The Rock: Tribute 1
    const BAUBLE: &str = "classicplus-061"; // Bauble Bubble

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

    /// R4207000: p2's backrow card with Death, destroyed under either face (an enemy permanent to the
    /// Radiant one), is exiled, destroyed all the same, and its Death adds its two Stockpiles.
    fn a_destroyed_backrow_card_with_death_fires_it_and_is_exiled(radiant: bool) {
        let mut s = crate::scenario(json!({
            "p1": {
                "hand": [SHATTER, FILLER],
                "field": [{ "def": ID, "radiant": radiant }],
                "library": crate::filler(4),
            },
            "p2": { "hand": [FILLER], "backrow": [BAUBLE], "library": crate::filler(4) },
        }));
        let bauble = s.backrow(P2, 1).expect("p2's Bauble Bubble");
        s.play(SHATTER, json!({ "targets": pick::instance(&bauble) }));
        s.expect_in_zone(&bauble, "exile");
        assert!(exiled(&s, &bauble) && destroyed(&s, &bauble));
        assert_eq!(s.hand(P2).len(), 3, "its Death added two Stockpiles");
    }

    mod base {
        use super::*;

        #[test]
        fn r4207000_a_dead_unit_and_a_destroyed_backrow_card_are_exiled_destroyed_and_the_units_death_fires() {
            let mut s = board(false, json!([HIT_JOB, SHATTER]), json!([SAINTESS, VANILLA]), json!([DEFENDER]));
            let saintess = s.unit_at(P1, 2);
            let vanilla = s.unit_at(P1, 3);
            s.play(HIT_JOB, json!({ "targets": pick::instance(&saintess) }));
            s.expect_in_zone(&saintess, "exile");
            assert!(exiled(&s, &saintess) && destroyed(&s, &saintess));
            assert!(s.card(&vanilla.id).radiant, "its Death fired");

            s.state_mut().players.p1.mana.current = 4;
            let reserves = s.backrow(P2, 1).expect("p2's Reserves");
            s.play(SHATTER, json!({ "targets": pick::instance(&reserves) }));
            s.expect_in_zone(&reserves, "exile");
            assert!(exiled(&s, &reserves) && destroyed(&s, &reserves));
        }

        #[test]
        fn r4207000_a_unit_at_0_health_is_exiled_and_its_death_fires() {
            let mut s = board(false, json!([]), json!([SAINTESS]), json!([MENACE]));
            let saintess = s.unit_at(P1, 2);
            s.attack(&saintess, s.unit_at(P2, 1));
            s.expect_in_zone(&saintess, "exile");
            assert!(exiled(&s, &saintess) && destroyed(&s, &saintess));
            assert!(s.card(&s.unit_at(P1, 1).id).radiant, "its Death fired");
        }

        #[test]
        fn r4207000_itself_in_the_same_pass_every_death_fires_the_reborn_returns_and_it_fuses() {
            let mut s = board(false, json!([ARMAGEDDON]), json!([VANILLA]), json!([DEFENDER]));
            let disintegrex = s.unit_at(P1, 1);
            let vanilla = s.unit_at(P1, 2);
            let defender = s.unit_at(P2, 1);
            let reserves = s.backrow(P2, 1).expect("p2's Reserves");
            s.play(ARMAGEDDON, json!({}));
            for card in [&disintegrex, &vanilla, &reserves] {
                s.expect_in_zone(card, "exile");
                assert!(destroyed(&s, card), "{} was destroyed", card.def_id);
            }
            // Reborn takes the Defender back out of the exile pile it died into.
            assert!(destroyed(&s, &defender));
            assert_eq!(s.unit_at(P2, 1).id, defender.id, "the Reborn body is back");
            s.expect_in_zone(&defender, "field");
            // Destroyed, so it died: this Starship Piece fuses (R1852000).
            assert!(s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
        }

        #[test]
        fn r4207000_a_destroyed_backrow_card_with_death_fires_it_and_is_exiled() {
            super::a_destroyed_backrow_card_with_death_fires_it_and_is_exiled(false);
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
            // R4207000: the exiled enemy was destroyed all the same.
            assert!(destroyed(&s, &theirs));
        }

        #[test]
        fn r4207000_a_destroyed_backrow_card_with_death_fires_it_and_is_exiled() {
            super::a_destroyed_backrow_card_with_death_fires_it_and_is_exiled(true);
        }

        #[test]
        fn r4207000_an_enemy_unit_destroyed_is_exiled_its_death_fires_and_its_reborn_returns_it() {
            let mut s = board(true, json!([HIT_JOB, HIT_JOB]), json!([]), json!([SAINTESS, VANILLA, DEFENDER]));
            let saintess = s.unit_at(P2, 1);
            let vanilla = s.unit_at(P2, 2);
            let defender = s.unit_at(P2, 3);
            s.play(HIT_JOB, json!({ "targets": pick::instance(&saintess) }));
            s.expect_in_zone(&saintess, "exile");
            assert!(exiled(&s, &saintess) && destroyed(&s, &saintess));
            assert!(s.card(&vanilla.id).radiant, "its Death fired");

            s.state_mut().players.p1.mana.current = 4;
            s.play(HIT_JOB, json!({ "targets": pick::instance(&defender) }));
            assert!(exiled(&s, &defender) && destroyed(&s, &defender));
            // Reborn takes it back out of the exile pile it died into.
            assert_eq!(s.unit_at(P2, 3).id, defender.id, "the Reborn body is back");
            s.expect_in_zone(&defender, "field");
        }
    }
}
