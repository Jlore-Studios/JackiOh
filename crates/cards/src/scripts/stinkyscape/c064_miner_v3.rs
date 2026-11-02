//! Stinkyscape #64 Miner v3 (SPEC §8.10 row 64). Unit 0/4 → 0/8, cost 2, Rare.
//!   Base:    "Starship Piece, Equip
//!            Activate 2: Gain a random Stinkyscape Resource."
//!   Radiant: "Starship Piece, Equip
//!            Activate 2: Gain a random Radiant Stinkyscape Resource."
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1872000: the Activate, two uses a turn (R384), gains one Resource rolled over every Resource the
//! catalog holds (R1840002) into the hand of the controller of the Unit it is on, a full hand burning
//! it (R2250000); the Radiant face's Resource is Radiant.

use jackioh_engine::effects::{GainResourcesArgs, gain_resources};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-064";

/// "Activate 2": two uses a turn (R384).
const USES: i32 = 2;

/// R1872000: "Gain a random [Radiant] Stinkyscape Resource".
fn mine(radiant: bool) -> Script {
    let label = if radiant {
        "Gain a random Radiant Stinkyscape Resource"
    } else {
        "Gain a random Stinkyscape Resource"
    };
    let dig = ActivationDecl {
        id: "mine".to_string(),
        label: label.to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            vec![gain_resources(GainResourcesArgs {
                count: 1,
                radiant: radiant.then_some(true),
                ..GainResourcesArgs::default()
            })]
        }),
    };
    Script {
        activations: vec![dig],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mine(false),
        radiant: mine(true),
    }
}

// Stinkyscape #64 Miner v3 — SPEC §8.10 row 64, BUILD M12 row S 64 (R1840001, R1840002, R2250000).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const REKNITE: &str = "stinkyscape-t-reknite"; // a Common Resource.

    fn mining(radiant: bool, seed: &str) -> Scenario {
        weapon_board(seed, json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({}))
    }

    fn dig() -> Value {
        json!({ "ability": "mine" })
    }

    fn mined(s: &Scenario) -> Vec<CardInstance> {
        resources_in_hand_of(s.state(), P1)
    }

    #[test]
    fn r1872000_r2250000_its_activate_gains_one_random_resource_into_the_hand() {
        let mut s = mining(false, "s064");
        let hand = s.hand(P1).len();
        s.activate(ID, dig());
        let in_hand = mined(&s);
        assert_eq!(in_hand.len(), 1);
        assert!(crate::card_def(&in_hand[0].def_id).resource.is_some(), "{}", in_hand[0].def_id);
        assert!(!in_hand[0].radiant);
        assert_eq!(s.hand(P1).len(), hand + 1, "it takes a hand slot");
        // R1840002: the roll replays from the seed.
        let mut again = mining(false, "s064");
        again.activate(ID, dig());
        assert_eq!(resources_in_hand_of(again.state(), P1)[0].def_id, in_hand[0].def_id);
    }

    #[test]
    fn r1840001_r2250000_into_a_full_hand_the_resource_burns() {
        let full = vec![REKNITE; HAND_CAP as usize];
        let mut s = weapon_board(
            "s064-full",
            json!({ "field": [ID], "hand": full }),
            json!({}),
        );
        s.activate(ID, dig());
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        assert!(
            s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::Burned)
        );
    }

    #[test]
    fn r384_2_uses_a_turn_a_third_use_refused_and_again_next_turn() {
        let mut s = mining(false, "s064-uses");
        for _ in 0..super::USES {
            s.activate(ID, dig());
        }
        let refusal = format!("used {} times this turn", super::USES);
        s.expect_refused_with(|s| s.activate(ID, dig()), &refusal);
        assert_eq!(mined(&s).len(), super::USES as usize);
        next_turn_of(&mut s, P1);
        s.activate(ID, dig());
        assert_eq!(mined(&s).len(), super::USES as usize + 1);
    }

    #[test]
    fn r1086_r1872000_equipped_onto_a_unit_the_host_takes_the_activate() {
        let mut s = weapon_board(
            "s064-equip",
            json!({ "hand": [ID], "field": [VANILLA], "mana": 10 }),
            json!({}),
        );
        let host = s.unit_at(P1, 1).id;
        s.play(ID, json!({ "zone": 1, "equip": true }));
        assert_eq!(s.unit_at(P1, 1).id, host);
        s.activate(host.as_str(), dig());
        assert_eq!(mined(&s).len(), 1);
    }

    #[test]
    fn radiant_the_resource_is_radiant() {
        let mut s = mining(true, "s064-radiant");
        s.activate(ID, dig());
        let in_hand = mined(&s);
        assert_eq!(in_hand.len(), 1);
        assert!(in_hand[0].radiant);
    }
}
