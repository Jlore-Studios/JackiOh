//! IA #26 Phantom Lancer (SPEC §8.13 row 26), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Cloaked. When this is revealed, deal {damage} damage to an enemy Unit." (3)
//!   Radiant: "Cloaked. When this is revealed, deal {damage} damage to an enemy Unit." (6)
//!
//! Cloaked is catalog data. The reveal fires as a trap firing (R4586002): a trigger on its own
//! `trapFired` opens the enemy-Unit pick as it resolves, and the resume deals the declared
//! `damage`. A face-up play never fires it, and with no target nothing happens (R61).
//!
//! Engine: `chooseTarget` + `damage` (R4586002). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-026";

/// The resume step answering the reveal's pick.
const LANCER: &str = "lancer";

/// Its own Cloaked reveal's firing.
fn own_reveal(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = &ctx.self_ else {
        return false;
    };
    matches!(
        event,
        GameEvent::TrapFired { instance_id, .. } if *instance_id == me.id
    )
}

fn lancer() -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "phantom-lancer",
            &[GameEventType::TrapFired],
            |ctx, event| {
                if !own_reveal(ctx, event) {
                    return vec![];
                }
                vec![choose_target(json_as(json!({
                    "step": LANCER,
                    "scope": { "side": "enemy", "of": ["unit"] },
                    "prompt": "Phantom Lancer: deal damage to an enemy Unit",
                })))]
            },
        )
        .with_when(own_reveal)],
        resume: IndexMap::from([(
            LANCER,
            hook(|ctx| {
                vec![damage(json_as(json!({
                    "to": { "of": "chosen" },
                    "amount": param(&*ctx, "damage"),
                })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: lancer(),
        // The same script: the Radiant face's 6 is its declared `damage`.
        radiant: lancer(),
    }
}

// IA #26 Phantom Lancer — SPEC §8.13 row 26 (R4586002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::{ID, LANCER};
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";


    fn health_of(s: &Scenario, lane: i32) -> i32 {
        unit_view(s.state(), &s.unit_at(PlayerId::P2, lane)).health
    }

    fn revealed(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "field": [VANILLA], "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        let set = crate::set_cloaked(&mut s, P1, ID, 1);
        s.activate(&set, json!({ "ability": jackioh_engine::alt_play::CLOAKED_REVEAL }));
        s
    }

    fn answer_prompt(s: &mut Scenario) {
        let victim = s.unit_at(PlayerId::P2, 1).id.clone();
        s.answer(json!(victim));
    }

    mod base {
        use super::*;

        #[test]
        fn r4586002_the_reveal_deals_3_to_an_enemy_unit_you_choose() {
            let mut s = revealed(false, "lancer-base");
            assert!(s.state().pending.is_some(), "the reveal opens the pick");
            let victim = s.unit_at(PlayerId::P2, 1).id.clone();
            let before = health_of(&s, 1);
            s.answer(json!(victim));
            assert_eq!(health_of(&s, 1), before - 3);
        }

        #[test]
        fn r4586002_a_face_up_play_never_fires_it() {
            let mut s = scenario(json!({
                "seed": "lancer-faceup",
                "p1": { "hand": [ID, STOCKPILE], "library": crate::filler(4) },
                "p2": { "field": [VANILLA], "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            let before = health_of(&s, 1);
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none(), "no pick opens");
            assert_eq!(health_of(&s, 1), before);
        }

        #[test]
        fn r4586002_with_no_enemy_unit_nothing_happens() {
            let mut s = scenario(json!({
                "seed": "lancer-no-target",
                "p1": {
                    "hand": [{ "def": ID, "radiant": false }, STOCKPILE],
                    "library": crate::filler(4),
                },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            let set = crate::set_cloaked(&mut s, P1, ID, 1);
            s.activate(&set, json!({ "ability": jackioh_engine::alt_play::CLOAKED_REVEAL }));
            assert!(s.state().pending.is_none(), "no target, no prompt (R61)");
        }

        #[test]
        fn the_prompt_is_the_lancer_step() {
            let s = revealed(false, "lancer-step");
            let pending = s.state().pending.clone().expect("the pick");
            assert_eq!(pending.resume.step, LANCER);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586002_the_reveal_deals_6() {
            let mut s = revealed(true, "lancer-radiant");
            let before = health_of(&s, 1);
            answer_prompt(&mut s);
            assert_eq!(health_of(&s, 1), before - 6);
        }
    }
}
