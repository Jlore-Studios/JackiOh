//! M #45 Knowledge Breaker (SPEC §8.8 row 45): (1) Unit, CN, KY, Catalyst, Legendary, 1/3 → 2/6.
//!
//! Base:    "Cry: Nerf every other CN or KY card on the field, in each hand and in each deck.
//!           Aura: You may play your Units face-down into your backrow as Animated Field Traps that
//!           reveal at the start of your turn.
//!           Death: Shuffle a Knowledge Breaker Prime into your deck."
//! Radiant: "Divine Shield
//!           Cry: Nerf every other CN or KY card on the field, in each hand and in each deck
//!           {times|time|times}.
//!           Aura: You may play your Units face-down into your backrow as Animated Field Traps that
//!           reveal at the start of your turn.
//!           Death: Shuffle a Radiant Knowledge Breaker Prime into your deck."
//! Engine:
//! - Cry: `degrade` over every other CN or KY card on fields, hands and decks, `times`.
//! - Death: `shuffle_into` the Prime, Radiant on the Radiant face (C+ #38's shape).
//! - Aura: ME-ALTPLAY permission for Units while acting.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-045";

const PRIME: &str = "meditative-045-1";

fn face_down_units() -> FaceDownPlayHook {
    Arc::new(|_args| {
        vec![FaceDownPlayPermission {
            units: Some(true),
            ..FaceDownPlayPermission::default()
        }]
    })
}

fn breaker(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let times = param(&*ctx, "times");
            vec![degrade(json_as(json!({
                "scope": {
                    "side": "any",
                    "zones": ["field", "hand", "library"],
                    "tags": ["CN", "KY"],
                    "excludeSelf": true,
                },
                "times": times,
            })))]
        })),
        death: Some(hook(move |_ctx| {
            vec![shuffle_into(json_as(json!({
                "defId": PRIME,
                "count": 1,
                "radiant": radiant,
            })))]
        })),
        face_down_play: Some(face_down_units()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: breaker(false),
        radiant: breaker(true),
    }
}

// M #45 Knowledge Breaker — SPEC §8.8 row 45, BUILD M10 row M 45.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BREAKER: &str = "meditative-045";
    const PRIME: &str = "meditative-045-1";
    const CNKY: &str = "meditative-038";
    const VANILLA: &str = "core-008";
    const MENACE: &str = "core-019"; // 9/9 Taunt.

    fn degraded_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Degraded { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Breaker in p1's hand; CN/KY cards spread over both fields, hands and decks; a vanilla
    /// control on each side.
    fn spread(seed: &str, radiant_face: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 8,
                "hand": [
                    { "def": BREAKER, "radiant": radiant_face },
                    { "def": CNKY },
                    { "def": VANILLA },
                ],
                "field": [{ "def": CNKY, "lane": 1 }],
                "library": [{ "def": CNKY }, { "def": VANILLA }],
            },
            "p2": {
                "hand": [{ "def": CNKY }],
                "field": [{ "def": CNKY, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                "library": [{ "def": CNKY }],
            },
        }))
    }

    fn cnky_ids(s: &Scenario, player: PlayerId) -> Vec<String> {
        let state = s.state();
        let side = &state.players[player];
        side.units
            .iter()
            .flatten()
            .flatten()
            .chain(side.hand.iter())
            .chain(side.library.iter())
            .filter(|card| card.def_id == CNKY)
            .map(|card| card.id.clone())
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1042_cry_nerfs_every_other_cn_or_ky_card_on_fields_hands_decks() {
            let mut s = spread("breaker-cry", false);
            let breaker = s.hand(P1).iter().find(|card| card.def_id == BREAKER).cloned().unwrap();
            let before_p1 = cnky_ids(&s, P1);
            let before_p2 = cnky_ids(&s, P2);
            assert_eq!(before_p1.len(), 3);
            assert_eq!(before_p2.len(), 3);
            s.play(&breaker.id, json!({ "zone": 2 }));
            let degraded = degraded_ids(&s);
            // Every other CN or KY card exactly once, on fields, hands and decks alike.
            for id in before_p1.iter().chain(before_p2.iter()) {
                assert_eq!(degraded.iter().filter(|hit| *hit == id).count(), 1, "{id}");
            }
            // Itself is excluded, and the vanilla controls are untouched.
            assert!(!degraded.contains(&breaker.id));
            let state = s.state();
            let vanilla_hit = state.players[P1]
                .hand
                .iter()
                .chain(state.players[P1].library.iter())
                .chain(state.players[P2].hand.iter())
                .any(|card| card.def_id == VANILLA && degraded.contains(&card.id));
            assert!(!vanilla_hit);
        }

        #[test]
        fn r1040_r1041_a_set_unit_reveals_and_attacks() {
            let mut s = scenario(json!({
                "seed": "breaker-set",
                "p1": {
                    "mana": 8,
                    "hand": [BREAKER, VANILLA],
                    "field": [{ "def": BREAKER, "lane": 1 }],
                    "library": [VANILLA, VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
            }));
            let vanilla = s.hand(P1).iter().find(|card| card.def_id == VANILLA).cloned().unwrap();
            s.play(&vanilla.id, json!({ "zone": 2, "row": "backrow", "faceDown": "startOfNextTurn" }));
            assert!(s.state().pending.is_none());
            // The opponent's turn passes quietly; ours reveals with no prompt (no Cry to run).
            s.end_turn();
            assert!(s.state().pending.is_none());
            s.end_turn();
            assert!(s.state().pending.is_none());
            let set = s.backrow(P1, 2).expect("the set unit");
            assert_eq!(set.face_up, Some(true));
            assert_eq!(set.set_as, None);
            // It kept its set turn, so it may attack at once.
            let attacks: Vec<_> = legal_actions(s.state(), P1)
                .into_iter()
                .filter(|action| {
                    matches!(action, ActionBody::Attack { attacker_id, .. } if attacker_id == &set.id)
                })
                .collect();
            assert!(!attacks.is_empty());
        }

        #[test]
        fn death_shuffles_a_prime_radiant_on_radiant() {
            for radiant_face in [false, true] {
                let mut s = scenario(json!({
                    "seed": if radiant_face { "breaker-death-r" } else { "breaker-death" },
                    "p1": {
                        "mana": 8,
                        "hand": [{ "def": BREAKER, "radiant": radiant_face }],
                        "field": [{ "def": CNKY, "lane": 1 }],
                        "library": [VANILLA],
                    },
                    "p2": { "hand": [VANILLA], "field": [{ "def": MENACE, "lane": 1 }] },
                }));
                let breaker = s.hand(P1).iter().find(|card| card.def_id == BREAKER).cloned().unwrap();
                s.play(&breaker.id, json!({ "zone": 2 }));
                // Suicide into the 9/9 Taunt: its Death shuffles exactly one Prime.
                s.attack(BREAKER, MENACE);
                s.expect_in_zone(BREAKER, "graveyard");
                let library = s.pile(P1, "library");
                let primes: Vec<_> =
                    library.iter().filter(|card| card.def_id == PRIME).collect();
                assert_eq!(primes.len(), 1);
                assert_eq!(primes[0].radiant, radiant_face);
            }
        }

        #[test]
        fn the_opponents_units_get_no_permission() {
            let s = scenario(json!({
                "seed": "breaker-foe",
                "p1": { "mana": 8, "hand": [VANILLA], "field": [{ "def": BREAKER, "lane": 1 }], "library": [VANILLA] },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let enemy = s.hand(P2).iter().find(|card| card.def_id == VANILLA).cloned().unwrap();
            let plays: Vec<ActionBody> = legal_actions(s.state(), P2)
                .into_iter()
                .filter(|action| {
                    matches!(action, ActionBody::Play { instance_id, .. } if instance_id == &enemy.id)
                })
                .collect();
            assert!(!plays.is_empty());
            assert!(plays.iter().all(|action| {
                matches!(action, ActionBody::Play { face_down: None, .. })
            }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1042_radiant_nerfs_twice_with_divine_shield() {
            // The Radiant face carries Divine Shield.
            let def = crate::card_def(BREAKER);
            assert!(def.radiant.keywords.contains(&Keyword::DivineShield));
            let mut s = spread("breaker-cry-radiant", true);
            let breaker = s.hand(P1).iter().find(|card| card.def_id == BREAKER).cloned().unwrap();
            let before_p1 = cnky_ids(&s, P1);
            let before_p2 = cnky_ids(&s, P2);
            s.play(&breaker.id, json!({ "zone": 2 }));
            let degraded = degraded_ids(&s);
            for id in before_p1.iter().chain(before_p2.iter()) {
                assert_eq!(degraded.iter().filter(|hit| *hit == id).count(), 2, "{id}");
            }
            assert!(!degraded.contains(&breaker.id));
        }
    }
}

