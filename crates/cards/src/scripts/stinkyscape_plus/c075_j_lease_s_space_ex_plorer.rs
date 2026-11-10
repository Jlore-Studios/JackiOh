//! SPEC §8.12 S+ #75 J-lease S-Space EX-plorer (Unit 5/5 → 10/10, cost 3, Legendary), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Cloaked / Evolves when you launch a Starship. / Evolve: Put a J-lease S-Space EX-plorer Pack on
//!            top of your deck."
//!   Radiant: "Cloaked / Evolves when you launch a Starship. / Evolve: Put a Radiant J-lease S-Space EX-plorer
//!            Pack on top of your deck."
//!
//! R3536001: "Evolves when you launch a Starship" is the card's own trigger on `starshipLaunched`, evolving
//! itself when the launcher is its controller (an opponent's launch evolves nothing). Its Evolve text puts
//! one fresh Pack on top of its controller's library (`shuffle_into` at the top, so the next draw is the
//! Pack, which casts itself on draw); the Radiant face's Pack is Radiant. The card evolves once (R3536000),
//! but a multiplier such as High Evolutionary's runs the text again for another Pack. Cloaked is catalog
//! data (R783004).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-075";

/// The Pack its Evolve text puts on top of the deck (S+ #75.1).
const PACK: &str = "stinkyscapeplus-075-1";

/// How many Packs one run of its Evolve text puts on top ("a Pack").
const PACKS: i32 = 1;

fn explorer(radiant: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "explorer-launch",
            &[GameEventType::StarshipLaunched],
            |ctx, event| match event {
                GameEvent::StarshipLaunched { player, .. } if *player == ctx.controller => {
                    vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
                }
                _ => vec![],
            },
        )],
        evolve: Some(hook(move |_ctx| {
            let mut args = json!({ "defId": PACK, "count": PACKS, "position": "top" });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![shuffle_into(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: explorer(false),
        radiant: explorer(true),
    }
}

// S+ #75 J-lease S-Space EX-plorer — SPEC §8.12 row 75 (R3536000, R3536001, R1852003, R3542001).
#[cfg(test)]
mod tests {
    use super::{ID, PACK};
    use jackioh_engine::testkit::*;

    use crate::{built_starship, launch_starship};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const AP_ROUNDS: &str = "stinkyscape-100"; // AP Rounds: Starship Piece, keywords only

    /// p1's Starship, one EX-plorer on `side`'s field, and a stocked library for p1.
    fn board(seed: &str, radiant: bool, mine: bool) -> (Scenario, String) {
        let explorer = json!([{ "def": ID, "radiant": radiant }]);
        let (p1_field, p2_field) = if mine { (explorer, json!([])) } else { (json!([]), explorer) };
        let mut p1 = json!({ "field": [] });
        p1["field"] = p1_field;
        built_starship(seed, json!([{ "def": AP_ROUNDS, "lane": 1 }]), p1, json!({ "field": p2_field }))
    }

    mod base {
        use super::*;

        #[test]
        fn your_launch_evolves_it_and_a_pack_goes_on_top() {
            let (mut s, ship) = board("s075-base", false, true);
            let explorer = s.unit_at(P1, 2);
            launch_starship(&mut s, &ship);
            assert_eq!(s.card(&explorer).evolved, Some(true));
            assert_eq!(evolutions_of(s.state(), P1), 1);
            let top = &s.pile(P1, "library")[0];
            assert_eq!(top.def_id, PACK);
            assert!(!top.radiant);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_radiant_pack_goes_on_top() {
            let (mut s, ship) = board("s075-radiant", true, true);
            launch_starship(&mut s, &ship);
            let top = &s.pile(P1, "library")[0];
            assert_eq!(top.def_id, PACK);
            assert!(top.radiant);
        }
    }

    #[test]
    fn an_opponents_launch_evolves_nothing() {
        let (mut s, ship) = board("s075-theirs", false, false);
        let explorer = s.unit_at(P2, 1);
        launch_starship(&mut s, &ship);
        assert_ne!(s.card(&explorer).evolved, Some(true));
        assert_eq!(evolutions_of(s.state(), P2), 0);
        assert!(s.pile(P1, "library").iter().all(|card| card.def_id != PACK));
    }
}
