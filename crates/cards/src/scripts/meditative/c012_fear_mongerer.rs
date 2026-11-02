//! Meditative #12 Fear Mongerer (SPEC §8.8 row 12; docs/meditative-set.md M6 #12; ME-TRIG (c); R824).
//! (2) Unit, Human, Epic, 5/7 → 10/14.
//!   Base:    "Cry: Trigger your End of turn effects."
//!   Radiant: "Cry: Trigger your End of turn effects {times|time|times}." (times 2)
//!
//! `trigger_turn_hooks` queues its controller's end-of-turn hooks as §2.2's end-of-turn trigger step
//! would — field cards and the return-flagged Spells in their graveyard (R153, R155), Meditative #9's
//! multiplier included (R821) — and the loop resolves them after the Cry's list. The turn goes on; the
//! trap window and the delayed effects do not run (R62). The rounds run one after another (R824).
//! `times` is tuned on the Radiant face only (R749): the base face's "your End of turn effects" is once.

use jackioh_engine::effects::{TriggerTurnHooksArgs, trigger_turn_hooks};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-012";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![trigger_turn_hooks(TriggerTurnHooksArgs {
                times: param(&*ctx, "times"),
            })]
        })),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// Meditative #12 Fear Mongerer — SPEC §8.8 row 12, BUILD M10 row M 12, R1, R68.
// Core #23 Reoccurring Dream stands in for M 8; Core #13, #39 and #18 are as their consts say.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHREDDER: &str = "core-013"; // Unit. End of turn: 2 damage to each enemy Unit and hero.
    const DREAM: &str = "core-023"; // Spell. End of turn: return this to hand, from the graveyard.
    const RECYCLING: &str = "core-039"; // Spell. Exile this. End of turn: copy each other card played.
    const VANILLA: &str = "core-008"; // a (1) 4/4 with no text.
    const BREAD_AND_BUTTER: &str = "core-018"; // Field Trap: fires in the end-of-turn trap window.
    const BREAD_TOKEN: &str = "core-t-bread";
    const FILING: &str = "meditative-009"; // Joint Filing: each end-of-turn hook runs twice.
    const PEA: &str = "meditative-013"; // Gatling Pea: end of turn, 1 to the enemy hero, then grows.
    const SPARE: &str = "core-010"; // (0) Spell, never played: it only keeps the turn open (R82).

    /// A side with mana to play and a stocked library, so no turn auto-ends (§2.5) or fatigues.
    fn side(extra: Value) -> Value {
        crate::merged(
            json!({ "mana": 10, "hand": [SPARE], "library": [VANILLA, VANILLA, VANILLA] }),
            extra,
        )
    }

    fn bread_tokens(s: &Scenario, player: PlayerId) -> usize {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .filter(|unit| unit.def_id == BREAD_TOKEN)
            .count()
    }

    /// A card's declared number as its tuning leaves it (R386).
    fn number(s: &Scenario, card: &str, key: &str) -> i32 {
        let instance = s.card(card).clone();
        jackioh_engine::prelude::param_value(
            s.state(),
            Some(&instance),
            key,
            jackioh_engine::prelude::ParamValueOptions::default(),
        )
    }

    mod base {
        use super::*;

        #[test]
        fn r824_with_a_shredder_the_enemy_takes_2_now_and_2_again_at_the_end_of_turn() {
            let mut s = scenario(json!({
                "p1": side(json!({ "field": [SHREDDER], "hand": [ID, SPARE] })),
                "p2": side(json!({})),
            }));
            s.play(ID, json!({}));
            s.expect_health(P2, 28);
            assert_eq!(s.state().active, P1);
            assert!(s.state().pending.is_none());
            s.end_turn();
            s.expect_health(P2, 26);
        }

        #[test]
        fn r824_a_graveyard_dream_returns_now_and_can_be_played_again() {
            let mut s = scenario(json!({
                "p1": side(json!({ "hand": [DREAM, ID, SPARE] })),
                "p2": side(json!({})),
            }));
            // Played this turn, the Dream lands return-flagged (R155); one set in the graveyard
            // would not be a return Spell at all (R153).
            s.play(DREAM, json!({}));
            s.expect_in_zone(DREAM, "graveyard");
            s.play(ID, json!({}));
            s.expect_in_zone(DREAM, "hand");
            s.play(DREAM, json!({}));
            s.expect_in_zone(DREAM, "graveyard");
        }

        #[test]
        fn r824_no_bread_token_and_no_delayed_copy() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [BREAD_AND_BUTTER],
                    "hand": [RECYCLING, VANILLA, ID, SPARE],
                })),
                "p2": side(json!({})),
            }));
            s.play(RECYCLING, json!({}));
            s.play(VANILLA, json!({}));
            s.play(ID, json!({}));
            assert_eq!(bread_tokens(&s, P1), 0);
            assert_eq!(
                s.hand(P1).iter().filter(|card| card.def_id == VANILLA).count(),
                0
            );
            s.end_turn();
            assert_eq!(
                s.hand(P1).iter().filter(|card| card.def_id == VANILLA).count(),
                1
            );
        }

        #[test]
        fn r821_under_joint_filing_each_hook_runs_twice() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [FILING], "field": [SHREDDER], "hand": [ID, SPARE] })),
                "p2": side(json!({})),
            }));
            s.play(ID, json!({}));
            s.expect_health(P2, 26);
        }

        #[test]
        fn r825_gatling_pea_hits_and_grows_by_one_per_run() {
            let mut s = scenario(json!({
                "p1": side(json!({ "field": [PEA], "hand": [ID, SPARE] })),
                "p2": side(json!({})),
            }));
            s.play(ID, json!({}));
            s.expect_health(P2, 29);
            assert_eq!(number(&s, PEA, "damage"), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r824_two_rounds_with_gatling_pea_hit_1_then_2() {
            let mut s = scenario(json!({
                "p1": side(json!({ "field": [PEA], "hand": [{ "def": ID, "radiant": true }, SPARE] })),
                "p2": side(json!({})),
            }));
            s.play(ID, json!({}));
            s.expect_health(P2, 27);
            assert_eq!(number(&s, PEA, "damage"), 3);
        }

        #[test]
        fn r824_upgraded_times_runs_three_rounds() {
            let mut s = scenario(json!({
                "p1": side(json!({ "field": [SHREDDER], "hand": [{ "def": ID, "radiant": true }, SPARE] })),
                "p2": side(json!({})),
            }));
            assert_eq!(crate::upgrade_number(&mut s, ID, "times"), 3);
            s.play(ID, json!({}));
            s.expect_health(P2, 24);
        }
    }
}
