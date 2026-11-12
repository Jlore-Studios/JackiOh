//! SPEC §8.12 S+ #11 Energy Booster (Unit 1/1 → 2/2, Starship Piece, Equip, cost 3, Epic), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece, Equip
//!            Activate 1: Refresh"
//!   Radiant: "Starship Piece, Equip
//!            Activate 2: Refresh"
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it fuses
//! into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one (R1852001).
//! R3548003: Refresh gives back this turn's uses of every Activate on your permanents except Activates
//! that Refresh, so the Booster stays spent; the engine's `refresh_ability` is that Activate, its id the
//! `refresh` that `is_refresh_ability` names. Its uses are 1 a turn on the base face and 2 on the Radiant
//! one (R384, R1857003), script literals, since `ActivationUses` has only `Count` and `Unlimited`.

use jackioh_engine::effects::refresh_ability;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-011";

/// "Activate": 1 use a turn on the base face, a label and no param (R384, R482, R1857003).
const USES: i32 = 1;

/// "Activate 2": 2 uses a turn on the Radiant face.
const RADIANT_USES: i32 = 2;

/// R3548003: the engine's Refresh Activate with `uses` uses a turn.
fn refresh(uses: i32) -> Script {
    Script {
        activations: vec![refresh_ability(ActivationUses::Count(uses))],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: refresh(USES),
        radiant: refresh(RADIANT_USES),
    }
}

// S+ #11 Energy Booster — SPEC §8.12 row 11 (R3548003; R384, R1086, R1751000, R1852001).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const ASSEMBLER: &str = "stinkyscapeplus-008"; // Drone Assembler v3: Activate 1: summon a Drone Interceptor.
    /// A durable enemy: it survives any drone's hit.
    const TOUGH: &str = "stand-in (1) 2/30";

    fn offers(s: &Scenario, id: &str) -> usize {
        activate_actions_for(s.state(), P1, s.card(id)).len()
    }

    fn board(radiant: bool) -> Scenario {
        weapon_board(
            "s011",
            json!({ "field": [{ "def": ID, "radiant": radiant }, ASSEMBLER] }),
            json!({ "field": [TOUGH, TOUGH, TOUGH] }),
        )
    }

    fn assemble(s: &mut Scenario) {
        let target = s.unit_at(PlayerId::P2, 1).id;
        s.activate(ASSEMBLER, json!({ "ability": "assemble", "targets": pick::id(&target) }));
    }

    mod base {
        use super::*;

        #[test]
        fn r3548003_it_gives_back_a_spent_activate_and_stays_spent_itself() {
            let mut s = board(false);
            assert_eq!(offers(&s, ASSEMBLER), 3);
            assemble(&mut s);
            assert_eq!(offers(&s, ASSEMBLER), 0);
            assert_eq!(offers(&s, ID), 1);
            s.activate(ID, json!({}));
            assert_eq!(offers(&s, ASSEMBLER), 3, "the Assembler is offered again");
            assert_eq!(offers(&s, ID), 0, "a Refresh is never given back (R3548003)");
            s.expect_refused(|s| s.activate(ID, json!({})));
            assemble(&mut s);
            assert_eq!(offers(&s, ASSEMBLER), 0);
        }

        #[test]
        fn r384_its_use_comes_back_the_next_turn() {
            let mut s = board(false);
            s.activate(ID, json!({}));
            assert_eq!(offers(&s, ID), 0);
            next_turn_of(&mut s, P1);
            assert_eq!(offers(&s, ID), 1);
        }

        #[test]
        fn r1086_r1751000_an_equipped_host_takes_the_refresh() {
            let mut s = weapon_board(
                "s011-equip",
                json!({ "hand": [ID], "field": ["stand-in (1) 4/4 R12/12", ASSEMBLER], "mana": 10 }),
                json!({ "field": [TOUGH, TOUGH, TOUGH] }),
            );
            let host = s.unit_at(P1, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            assemble(&mut s);
            assert_eq!(offers(&s, ASSEMBLER), 0);
            s.activate(host.as_str(), json!({ "ability": "refresh" }));
            assert_eq!(offers(&s, ASSEMBLER), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_it_may_refresh_twice_a_turn() {
            let mut s = board(true);
            assemble(&mut s);
            s.activate(ID, json!({}));
            assert_eq!(offers(&s, ASSEMBLER), 3);
            assert_eq!(offers(&s, ID), 1, "one use left");
            assemble(&mut s);
            s.activate(ID, json!({}));
            assert_eq!(offers(&s, ASSEMBLER), 3);
            assert_eq!(offers(&s, ID), 0);
            s.expect_refused(|s| s.activate(ID, json!({})));
        }
    }
}
