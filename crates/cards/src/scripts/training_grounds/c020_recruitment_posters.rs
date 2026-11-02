//! TG #20 Recruitment Posters (SPEC §8.11 row 20). (2) Field Spell, Rare. "Start of turn: Add a random
//! Training Grounds card to your hand." (Radiant: a random Radiant Training Grounds card.)
//! Core #59 Unbiased Immigration's `start_of_turn` with C+ #75.1's set-named pool: a pool that names a
//! set draws from it, shipped or not (R1420, R2194001, #2204), snapshots #41–#60 included. Tokens and
//! this card are left out (R387, R2199004). One match-rng pick (R60); a full hand burns it (§2.4).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-020";

fn recruit(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "set": "Training Grounds", "excludeDefId": ID },
                "count": 1,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: recruit(false),
        radiant: recruit(true),
    }
}

// TG #20 Recruitment Posters — SPEC §8.11 row 20, BUILD M13 row TG 20. The libraries hold Core cards
// only, so a Training Grounds card in hand came from the Posters.
#[cfg(test)]
mod tests {
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const POSTERS: &str = super::ID;
    const FILLER: &str = "core-010";

    fn board(radiant: bool, seed: &str, hand: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": hand,
                "library": [FILLER, FILLER],
                "backrow": [{ "def": POSTERS, "radiant": radiant, "faceUp": true, "lane": 1 }],
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    fn recruits(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| def_of(Some(s.state()), &card.def_id).set == SetName::TrainingGrounds)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r2194001_at_your_start_of_turn_one_random_training_grounds_card_is_added() {
            let mut s = board(false, "posters", json!([]));
            assert!(set_ships(SetName::TrainingGrounds));
            s.start_turn();

            assert_eq!(recruits(&s).len(), 1);
            assert_eq!(s.hand(P1).len(), 2, "the Posters' card and the turn's draw");
            let added = s.events().iter().position(|e| matches!(e, GameEvent::AddedToHand { .. })).unwrap();
            let drawn = s.events().iter().position(|e| matches!(e, GameEvent::Drawn { .. })).unwrap();
            assert!(added < drawn);
        }

        #[test]
        fn r2199004_only_cards_of_the_set_never_a_token_never_this_card() {
            for seed in 0..40 {
                let mut s = board(false, &format!("posters-{seed}"), json!([]));
                s.start_turn();
                for card in recruits(&s) {
                    let def = def_of(Some(s.state()), &card.def_id);
                    assert!(!def.is_token(), "{} is a token", card.def_id);
                    assert_ne!(card.def_id, POSTERS);
                }
                assert_eq!(recruits(&s).len(), 1, "seed {seed}");
            }
        }

        #[test]
        fn r68_none_at_the_opponents_start_of_turn() {
            let mut s = board(false, "posters", json!([]));
            s.state_mut().active = P2;
            s.start_turn();

            assert!(recruits(&s).is_empty());
            assert!(s.hand(P1).is_empty());
        }

        #[test]
        fn r4_a_full_hand_burns_it() {
            let hand = vec![json!(FILLER); HAND_CAP as usize];
            let mut s = board(false, "posters", json!(hand));
            s.start_turn();

            assert!(recruits(&s).is_empty());
            assert!(s.events().iter().any(|e| matches!(e, GameEvent::Burned { .. })));
        }

        #[test]
        fn r60_the_pick_is_one_match_rng_draw_the_same_seed_the_same_card() {
            let mut a = board(false, "posters-rng", json!([]));
            let mut b = board(false, "posters-rng", json!([]));
            a.start_turn();
            b.start_turn();

            assert_eq!(recruits(&a)[0].def_id, recruits(&b)[0].def_id);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_the_card_is_radiant() {
            let mut s = board(true, "posters", json!([]));
            s.start_turn();

            let recruit = recruits(&s);
            assert_eq!(recruit.len(), 1);
            assert!(recruit[0].radiant);
        }
    }
}
