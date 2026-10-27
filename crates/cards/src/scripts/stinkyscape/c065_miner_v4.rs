//! Stinkyscape #65 Miner v4 (SPEC §8.10 row 65). Unit 0/6 → 0/12, cost 3, Epic; held (R1420).
//!   Base:    "Starship Piece, Equip
//!            Activate 3: Gain a random Stinkyscape Resource."
//!   Radiant: "Starship Piece, Equip
//!            Activate 3: Gain a random Radiant Stinkyscape Resource."
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1872000: the Activate, three uses a turn (R384), gains one Resource rolled over every Resource the
//! catalog holds (R1840002) into the tray of the controller of the Unit it is on, never the hand
//! (R1840000), a ninth sold at once (R1840001); the Radiant face's Resource is Radiant.

use jackioh_engine::effects::{GainResourcesArgs, gain_resources};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-065";

/// "Activate 3": three uses a turn (R384).
const USES: i32 = 3;

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

// Stinkyscape #65 Miner v4 — SPEC §8.10 row 65, BUILD M12 row S 65: "its Activate puts one Resource into its
// controller's tray, never the hand (R1840000), by the rarity roll that replays (R1840002); a ninth in the
// tray is sold at once (R1840001); three uses a turn, a fourth refused; equipped onto a Unit, the host takes
// the Activate; radiant 0/12 and a Radiant Resource".
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const REKNITE: &str = "stinkyscape-t-reknite"; // a Common Resource.

    /// p1's Miner v4 (Radiant on `radiant`) stands in lane 1.
    fn mining(radiant: bool, seed: &str) -> Scenario {
        weapon_board(seed, json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({}))
    }

    fn dig() -> Value {
        json!({ "ability": "mine" })
    }

    fn tray(s: &Scenario) -> Vec<CardInstance> {
        resource_tray_of(s.state(), P1)
    }

    #[test]
    fn r1872000_its_activate_gains_one_random_resource_into_the_tray_never_the_hand() {
        let mut s = mining(false, "s065");
        let hand = s.hand(P1).len();
        s.activate(ID, dig());
        let tray = tray(&s);
        assert_eq!(tray.len(), 1);
        assert!(crate::card_def(&tray[0].def_id).resource.is_some(), "{}", tray[0].def_id);
        assert!(!tray[0].radiant);
        assert_eq!(s.hand(P1).len(), hand);
        // R1840002: the roll replays from the seed.
        let mut again = mining(false, "s065");
        again.activate(ID, dig());
        assert_eq!(resource_tray_of(again.state(), P1)[0].def_id, tray[0].def_id);
    }

    #[test]
    fn r1840001_a_ninth_resource_is_sold_at_once() {
        let mut s = weapon_board(
            "s065-full",
            json!({ "field": [ID], "tray": [REKNITE, REKNITE, REKNITE, REKNITE, REKNITE, REKNITE, REKNITE, REKNITE] }),
            json!({}),
        );
        s.activate(ID, dig());
        assert_eq!(tray(&s).len(), 8);
        assert!(
            s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::ResourceSold)
        );
    }

    #[test]
    fn r384_3_uses_a_turn_a_fourth_use_refused_and_again_next_turn() {
        let mut s = mining(false, "s065-uses");
        for _ in 0..super::USES {
            s.activate(ID, dig());
        }
        let refusal = format!("used {} times this turn", super::USES);
        s.expect_refused_with(|s| s.activate(ID, dig()), &refusal);
        assert_eq!(tray(&s).len(), super::USES as usize);
        next_turn_of(&mut s, P1);
        s.activate(ID, dig());
        assert_eq!(tray(&s).len(), super::USES as usize + 1);
    }

    #[test]
    fn r1086_r1872000_equipped_onto_a_unit_the_host_takes_the_activate() {
        let mut s = weapon_board(
            "s065-equip",
            json!({ "hand": [ID], "field": [VANILLA], "mana": 10 }),
            json!({}),
        );
        let host = s.unit_at(P1, 1).id;
        s.play(ID, json!({ "zone": 1, "equip": true }));
        assert_eq!(s.unit_at(P1, 1).id, host);
        s.activate(host.as_str(), dig());
        assert_eq!(tray(&s).len(), 1);
    }

    #[test]
    fn radiant_the_resource_is_radiant() {
        let mut s = mining(true, "s065-radiant");
        s.activate(ID, dig());
        let tray = tray(&s);
        assert_eq!(tray.len(), 1);
        assert!(tray[0].radiant);
    }
}
