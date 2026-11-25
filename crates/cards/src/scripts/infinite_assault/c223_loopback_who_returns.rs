//! IA #223 Loopback, Who Returns (SPEC §8.13 row 223), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Reborn. When this returns by Reborn, it gains +{buff}/+{buff} and Reborn, up to three
//!            times a game." (3)
//!   Radiant: "Reborn. When this returns by Reborn, it gains +{buff}/+{buff} and Reborn, up to three
//!            times a game." (6)
//!
//! Reborn is catalog data. The return lands as a reset instance at 1 health, its Cry unfired
//! (§4.5 step 4, R83, R78): the trigger answers its own `summoned` when the return set
//! `returned_by_reborn` — a plain play or summon is no return — and gains the reset body +N/+N
//! with Reborn again, the first three returns of each instance a game, each on its own
//! once-a-game key (R4594004). The regained Reborn returns the card again (R78's reset cleared the
//! spent mark when the granted Reborn landed).
//!
//! Engine: `returned_by_reborn`, `markOnceAGame`, `buff`, `grantKeyword` (R4594004). Built by C10
//! (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-223";

/// How many returns a game gain.
const RETURNS: i32 = 3;

/// This card arrived on the field: its `summoned` names its own id.
fn arrived(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

fn loopback() -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "loopback",
            &[GameEventType::Summoned],
            move |ctx, event| {
                if !arrived(ctx, event) {
                    return vec![];
                }
                let Some(me) = ctx.live_self().cloned() else {
                    return vec![];
                };
                // R4594004: only a Reborn return gains; a plain play does nothing.
                if !returned_by_reborn(&me) {
                    return vec![];
                }
                let Some(n) = (1..=RETURNS).find(|n| {
                    !once_a_game_used(
                        &*ctx.state,
                        ctx.controller,
                        &format!("{ID}:{}:{n}", me.id),
                    )
                }) else {
                    return vec![];
                };
                let key = format!("{ID}:{}:{n}", me.id);
                let gain = param(&*ctx, "buff");
                vec![
                    mark_once_a_game(json_as(json!({ "key": key }))),
                    buff(json_as(json!({
                        "target": { "of": "instance", "instanceId": me.id },
                        "attack": gain,
                        "health": gain,
                    }))),
                    grant_keyword(json_as(json!({
                        "target": { "of": "instance", "instanceId": me.id },
                        "keyword": { "kind": "Reborn" },
                    }))),
                ]
            },
        )
        .with_when(|ctx, event| arrived(ctx, event))],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's +6/+6 is its declared `buff`.
    let base = loopback();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #223 Loopback, Who Returns — SPEC §8.13 row 223 (R4594004).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: destroy the Loopback.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn stats(s: &Scenario) -> (i32, i32) {
        let view = unit_view(s.state(), &s.unit_at(P1, 1));
        (view.attack, view.health)
    }

    fn has_reborn(s: &Scenario) -> bool {
        unit_view(s.state(), &s.unit_at(P1, 1))
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Reborn)
    }

    fn standing(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "field": [{ "def": ID, "radiant": radiant }], "hand": [STOCKPILE], "library": crate::filler(4) },
            "p2": { "hand": [HIT_JOB, HIT_JOB, HIT_JOB, HIT_JOB, STOCKPILE], "library": crate::filler(4), "mana": 10 },
        }))
    }

    fn kill(s: &mut Scenario) {
        s.end_turn();
        let victim = s.unit_at(P1, 1).id.clone();
        s.play(HIT_JOB, json!({ "targets": pick::id(&victim) }));
        s.end_turn();
    }

    mod base {
        use super::*;

        #[test]
        fn r4594004_a_plain_play_gains_nothing() {
            let s = standing(false, "loopback-play");
            assert_eq!(stats(&s), (6, 6));
            assert!(has_reborn(&s), "its printed Reborn");
        }

        #[test]
        fn r4594004_the_return_gains_3_3_and_reborn() {
            let mut s = standing(false, "loopback-return");
            kill(&mut s);
            // The reset body at 1 health, +3/+3.
            assert_eq!(stats(&s), (6 + 3, 1 + 3));
            assert!(has_reborn(&s), "Reborn again");
        }

        #[test]
        fn r4594004_the_fourth_return_gains_nothing() {
            let mut s = standing(false, "loopback-fourth");
            for _ in 0..3 {
                kill(&mut s);
            }
            assert!(has_reborn(&s), "three gains, Reborn again");
            kill(&mut s);
            assert_eq!(stats(&s), (6, 1), "the reset body, no gain");
            assert!(!has_reborn(&s), "no fourth Reborn");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594004_the_return_gains_6_6() {
            let mut s = standing(true, "loopback-radiant");
            kill(&mut s);
            assert_eq!(stats(&s), (12 + 6, 1 + 6));
            assert!(has_reborn(&s), "Reborn again");
        }
    }
}
