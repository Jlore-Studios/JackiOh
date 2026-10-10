//! M #70 I'M WILL BE YOUR DOOM (SPEC §8.8 row 70): (1) Unit, Common, 5/8 → 10/16.
//!   Base:    "Cry: Summon a Ready… I'm for your opponent."
//!   Radiant: "Cry: Summon a Ready… I'm for your opponent and a Radiant Ready… I'm for yourself."
//! Engine: summons the token under the other seat (R360, MD-D35, R1066), then a Radiant one for you.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-070";

const READY: &str = "meditative-070-1";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![summon(json_as(json!({ "defId": READY, "player": "enemy" })))]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![
                    summon(json_as(json!({ "defId": READY, "player": "enemy" }))),
                    summon(json_as(json!({ "defId": READY, "radiant": true }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// M #70 I'M WILL BE YOUR DOOM — SPEC §8.8 row 70, BUILD M10 row M 70.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const DOOM: &str = "meditative-070";
    const READY: &str = "meditative-070-1";
    const FILLER: &str = "core-005";

    #[test]
    fn r1066_cry_summons_a_ready_im_owned_and_controlled_by_the_opponent_leftmost() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [DOOM, FILLER] },
            "p2": { "hand": [FILLER], "field": [{ "def": "core-008", "lane": 2 }] },
        }));
        s.play(DOOM, json!({ "zone": 1 }));
        let token = s.unit(PlayerId::P2, 1).expect("the opponent's Ready");
        assert_eq!(token.def_id, READY);
        assert_eq!(token.controller, PlayerId::P2);
        assert_eq!(token.owner, PlayerId::P1);
        assert!(!token.radiant);
    }

    #[test]
    fn r1066_their_full_row_takes_none() {
        crate::register_all();
        let full = ["core-008", "core-008", "core-008", "core-008", "core-008"];
        let mut s = scenario(json!({
            "p1": { "hand": [DOOM, FILLER] },
            "p2": { "hand": [FILLER], "field": full },
        }));
        s.play(DOOM, json!({ "zone": 1 }));
        assert!(s.unit(PlayerId::P2, 1).is_some());
        assert_eq!(s.unit(PlayerId::P2, 1).map(|unit| unit.def_id), Some("core-008".to_string()));
    }

    #[test]
    fn r1_cast_it_still_summons() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [DOOM, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = jackioh_engine::rng::create_rng(&s.state().seed, s.state().rng_cursor);
        {
            let mut sink = jackioh_engine::script::EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = jackioh_engine::resolve::make_context(
                    &mut sink,
                    None,
                    HookOptions { controller: Some(PlayerId::P1), ..HookOptions::default() },
                );
                jackioh_engine::resolve::apply_effects(
                    &[jackioh_engine::effects::cast_new(json_as(json!({ "def": DOOM })))],
                    &mut ctx,
                );
            }
            jackioh_engine::triggers::settle(&mut sink, jackioh_engine::triggers::SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
        assert!(s.unit(PlayerId::P2, 1).is_some());
    }

    #[test]
    fn radiant_also_a_radiant_one_for_you() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": DOOM, "radiant": true }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(DOOM, json!({ "zone": 1 }));
        let theirs = s.unit(PlayerId::P2, 1).expect("theirs");
        let yours = s.unit(PlayerId::P1, 2).expect("yours");
        assert_eq!(theirs.def_id, READY);
        assert!(!theirs.radiant);
        assert_eq!(yours.def_id, READY);
        assert!(yours.radiant);
    }
}
