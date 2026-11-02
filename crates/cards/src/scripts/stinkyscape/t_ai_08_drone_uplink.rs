//! S T-AI-8 Drone Uplink (SPEC §7, §8.10 row T-AI-8; R62, R64, R78, R482, R1852000, R1881005): (2) Field
//! Spell, AI, Token (no printed rarity), an AI generated token of the Stinkyscape set, built
//! by S21.
//!   Base:    "End of turn: Summon a Drone Interceptor. It gains Starship Piece."
//!   Radiant: "End of turn: Summon a Radiant Drone Interceptor. It gains Starship Piece."
//! Engine: End of turn (R62), its controller's: a summon of the Drone Interceptor into their leftmost open
//!   unit zone (R64), its `keywords` rider granting it Starship Piece as it lands (kept until it leaves the
//!   field, R78), with a full unit row their leftmost open backrow zone (R2248000); with no open zone
//!   nothing. When it dies a copy fuses into its owner's Dormant Starship
//!   as a printed piece's does (R1852000), a plain Drone Interceptor, the granted keyword not carried
//!   (R1881005). Radiant: a Radiant Interceptor. Declares no numbers (R482).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-08";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";

fn face(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({
                "defId": INTERCEPTOR,
                "radiant": radiant,
                "keywords": [{ "kind": "Starship Piece" }],
            })))]
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

// S T-AI-8 Drone Uplink — SPEC §8.10 row T-AI-8, BUILD M12 row S T-AI-8 (R1881005, R2248000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const UPLINK: &str = "stinkyscape-t-ai-08";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.

    fn fielded(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "seed": "drone-uplink",
            "p1": {
                "hand": [FILLER, HIT_JOB],
                "mana": 10,
                "backrow": [{ "def": UPLINK, "radiant": radiant, "faceUp": true }],
                "field": field,
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }))
    }

    fn drones(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .filter(|unit| unit.def_id == INTERCEPTOR)
            .collect()
    }

    #[test]
    fn r62_at_its_controllers_end_of_turn_a_drone_with_starship_piece() {
        let mut s = fielded(false, json!([]));
        s.end_turn();
        let made = drones(&s, P1);
        assert_eq!(made.len(), 1);
        assert!(!made[0].radiant);
        assert!(is_starship_piece(s.state(), s.card(made[0].id.as_str())));
        s.end_turn();
        assert_eq!(drones(&s, P1).len(), 1, "none at the opponent's end of turn");
        assert!(drones(&s, P2).is_empty());
    }

    #[test]
    fn r1881005_its_drone_dying_fuses_a_plain_copy_into_its_owners_starship() {
        let mut s = fielded(false, json!([]));
        s.end_turn();
        s.end_turn();
        let drone = drones(&s, P1)[0].id.clone();
        s.play(HIT_JOB, json!({ "targets": pick::id(&drone) }));
        s.expect_in_zone(&drone, "gone");
        let ship = dormant_starship_of(s.state(), P1).expect("a Dormant Starship");
        let pieces = starship_pieces_of(ship);
        assert_eq!(pieces.len(), 1, "{pieces:?}");
        assert_eq!(pieces[0].def_id, INTERCEPTOR);
    }

    #[test]
    fn r2248000_a_full_unit_row_puts_the_drone_in_the_backrow() {
        let mut s = fielded(false, json!(vec![VANILLA; 5]));
        s.end_turn();
        assert!(drones(&s, P1).is_empty());
        let drone = s.backrow(P1, 2).expect("the Interceptor beside the Uplink");
        assert_eq!(drone.def_id, INTERCEPTOR);
        assert!(is_starship_piece(s.state(), s.card(drone.id.as_str())));
    }

    #[test]
    fn radiant_a_radiant_drone() {
        let mut s = fielded(true, json!([]));
        s.end_turn();
        let made = drones(&s, P1);
        assert_eq!(made.len(), 1);
        assert!(made[0].radiant);
        assert!(is_starship_piece(s.state(), s.card(made[0].id.as_str())));
    }
}
