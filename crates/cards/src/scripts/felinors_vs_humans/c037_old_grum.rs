//! SPEC §8.14 FvH #37 Old Grum ((2) Unit, Legendary, 20/20 → 40/40), a card of the held Felinors vs Humans
//! set (R5235000).
//!   Base:    "GIGA, Untributable, Reborn\nAura: This is summoning sick until it is Reborn."
//!   Radiant: "GIGA, Reborn\nAura: This is summoning sick until it is Reborn."
//!
//! Engine: GIGA (four zones, two lanes wide, taken as it enters, R3546000, R3546002), Untributable (the base
//! face alone) and Reborn are catalog data. The Aura is `Script.stays_sick` (R5237004) with the read "it has
//! not come back by Reborn" (`returned_by_reborn`): OR-ed into `combat::is_sick`, it cannot declare an attack
//! while the read holds, whatever turn it entered (FV2 #37); Rush or Charge would lift it as they lift any
//! sickness. Its Reborn body (R83) is marked returned, so it is sick only on the turn it returns, a new
//! arrival (R174), and attacks from the turn after. Declares no numbers. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-037";

fn old_grum() -> Script {
    Script {
        stays_sick: Some(read_hook(|args| !returned_by_reborn(args.self_))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same Aura on both faces; the Radiant face's want of Untributable and its line are catalog data.
    CardScripts {
        base: old_grum(),
        radiant: old_grum(),
    }
}

// FvH #37 Old Grum — SPEC §8.14 row 37 (R5237004, R83, R3546000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, next_turn_of, scenario};

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill that brings it back by Reborn
    const ROCK: &str = "core-066"; // The Rock: a Tribute 1 Unit
    const VANILLA: &str = "stand-in (1) 4/4";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// R5237004: standing since before this turn and never Reborn, it is sick now and on its
    /// controller's later turns alike.
    fn sick_until_reborn(radiant: bool) {
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "lane": 1, "radiant": radiant }], "library": filler(6) },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        let grum = s.unit_at(P1, 1);
        assert!(is_sick(s.state(), s.card(&grum)));
        assert!(s.attacks_of(&grum).is_empty(), "no attack is offered");
        s.expect_refused(|s| s.attack(&grum, "hero"));
        for turn in 1..=2 {
            next_turn_of(&mut s, P1);
            assert!(s.attacks_of(&grum).is_empty(), "still sick on its controller's next turn {turn}");
        }
        s.expect_health(P2, HERO_HEALTH);
    }

    /// R5237004, R83: played and killed, its Reborn body is sick the turn it returns and attacks from
    /// the turn after.
    fn reborn_body_attacks_a_turn_later(radiant: bool) {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB, FILLER],
                "library": filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        let grum = s.unit_at(P1, 1);
        assert!(is_giga(&grum), "it entered GIGA");
        s.play(HIT_JOB, pick::targets(pick::instance(&grum)));
        let back = s.unit_at(P1, 1);
        assert_eq!(back.def_id, ID);
        assert!(returned_by_reborn(&back), "it came back by Reborn");
        assert!(s.attacks_of(&back).is_empty(), "sick the turn it returns");
        next_turn_of(&mut s, P1);
        assert!(!is_sick(s.state(), s.card(&back)));
        assert!(!s.attacks_of(&back).is_empty(), "the Reborn body may attack");
    }

    /// Old Grum (`radiant` its face) in lanes 1 and 2 and a Vanilla in lane 3 on p1's side; p1 holds a
    /// The Rock, whose Tribute 1 may take a Unit of theirs.
    fn beside_a_tribute(radiant: bool) -> (Scenario, CardInstance, CardInstance) {
        let s = scenario(json!({
            "p1": {
                "hand": [ROCK, FILLER],
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }, { "def": VANILLA, "lane": 3 }],
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let grum = s.unit_at(P1, 1);
        let other = s.unit_at(P1, 3);
        (s, grum, other)
    }

    /// The ids of the Units The Rock's Tribute may take.
    fn tributable(s: &Scenario) -> Vec<String> {
        legal_tribute_units(s.state(), P1, s.card(ROCK))
            .into_iter()
            .map(|unit| unit.id.clone())
            .collect()
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r5237004_it_stays_summoning_sick_until_it_is_reborn() {
            sick_until_reborn(false);
        }

        #[test]
        fn r5237004_r83_its_reborn_body_attacks_from_the_turn_after_it_returns() {
            reborn_body_attacks_a_turn_later(false);
        }

        #[test]
        fn r1220_the_base_face_is_untributable() {
            let (s, grum, other) = beside_a_tribute(false);
            let units = tributable(&s);
            assert!(units.contains(&other.id), "the other Unit may be tributed");
            assert!(!units.contains(&grum.id), "Untributable: never offered");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5237004_it_stays_summoning_sick_until_it_is_reborn() {
            sick_until_reborn(true);
        }

        #[test]
        fn r5237004_r83_its_reborn_body_attacks_from_the_turn_after_it_returns() {
            reborn_body_attacks_a_turn_later(true);
        }

        #[test]
        fn r1220_the_radiant_face_prints_no_untributable() {
            let (s, grum, other) = beside_a_tribute(true);
            let units = tributable(&s);
            assert!(units.contains(&other.id));
            assert!(units.contains(&grum.id), "the Radiant face may be tributed");
        }
    }
}
