//! SPEC §8.14 FvH #91 KY's Log Exp of the FFT ((X) Spell, KY, Legendary), a card of the Felinors vs Humans
//! set (R5235000).
//!   Base:    "Deal X√(log₂X) damage, rounded down, to an enemy. If X is a power of 2, deal it to every enemy
//!            instead."
//!   Radiant: "X rounds up to a power of 2, at least 2. Deal X√(log₂X) damage, rounded down, to every enemy."
//!
//! An X Spell (X at least 1, R348). The damage is `ky_fft_damage(X)`, the `KY_FFT_DAMAGE` table of
//! ⌊n·√log₂n⌋ worked out offline so no float runs (FV3 E4), X clamped at `KY_FFT_MAX_POINTS`; at X = 1 it
//! is 0, the joke (FV2 #91). Base: one declared enemy, Unit or hero, aimed Harm (R81), one `damage`; when
//! X is a power of 2 (1 = 2⁰ among them), `damage_all` over every enemy instead, each enemy Unit and the
//! enemy hero one hit each (R59). Radiant: no target; X padded up to a power of 2, at least 2, and
//! `damage_all` of its table value to every enemy. A 0 deals nothing (R5256000). Spell Damage adds per
//! hit. The table is a formula, so it declares no number (R1748003). Its `preview` is the damage at the
//! mana its player holds, the X a play now could pay (R280, R5256000).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-091";

/// The formula as both faces print it, the preview's label (R280: a substring of the face's text).
const FORMULA: &str = "X√(log₂X)";
/// R348: an X is at least 1.
const LEAST_X: i32 = 1;
/// The Radiant face's "at least 2": the smallest padded X, a printed number of the formula (R1748003).
const LEAST_PADDED: i32 = 2;

/// Whether `x` is a power of 2, 1 = 2⁰ included.
fn is_power_of_two(x: i32) -> bool {
    u32::try_from(x).is_ok_and(u32::is_power_of_two)
}

/// The Radiant face's X: rounded up to a power of 2, at least 2, never past the table's end.
fn padded(x: i32) -> i32 {
    u32::try_from(x.max(LEAST_PADDED))
        .ok()
        .and_then(|n| i32::try_from(n.next_power_of_two()).ok())
        .map_or(KY_FFT_MAX_POINTS, |p| p.min(KY_FFT_MAX_POINTS))
}

/// The points a resolution of X reads the table at, on its face.
fn points(x: i32, radiant: bool) -> i32 {
    if radiant { padded(x) } else { x }
}

/// Every enemy, the hero included, one hit each.
fn every_enemy(amount: i32) -> Effect {
    damage_all(json_as(json!({ "side": "enemy", "amount": amount, "heroes": true })))
}

/// The base face: the chosen enemy, or every enemy when X is a power of 2.
fn base_cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let amount = ky_fft_damage(ctx.x);
    if amount <= 0 {
        return Vec::new();
    }
    if is_power_of_two(ctx.x) {
        vec![every_enemy(amount)]
    } else {
        vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
    }
}

/// The Radiant face: every enemy, at the padded X.
fn radiant_cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let amount = ky_fft_damage(points(ctx.x, true));
    if amount <= 0 {
        return Vec::new();
    }
    vec![every_enemy(amount)]
}

/// R280, R5256000: the damage a play now would deal, X the mana its player holds (at least 1).
fn preview(radiant: bool) -> PreviewHook {
    condition_hook(move |ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        let x = unspent_mana_of(ctx.state, ctx.controller).max(LEAST_X);
        vec![PreviewValue {
            label: FORMULA.to_string(),
            value: ky_fft_damage(points(x, radiant)),
            display: None,
            ids: None,
        }]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: vec![TargetDecl {
                aim: Some(TargetAim::Harm),
                ..TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }))
            }],
            cry: Some(hook(base_cry)),
            preview: Some(preview(false)),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(radiant_cry)),
            preview: Some(preview(true)),
            ..Script::default()
        },
    }
}

// FvH #91 KY's Log Exp of the FFT — SPEC §8.14 row 91 (R5256000, R348, R280, R59).
#[cfg(test)]
mod tests {
    use super::{FORMULA, ID};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{hits, merged, preview_seen, weapon_board};

    const FILLER: &str = "core-005"; // Stockpile: a Spell, each side's spare card (§2.5).
    /// An enemy Unit that outlives every hit here.
    const WALL: &str = "stand-in (1) 0/40";

    /// p1 holds the Spell (`radiant` its face) with 10 mana; p2 has a WALL in lane 1.
    fn fft(seed: &str, radiant: bool) -> Scenario {
        weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 10 }),
            json!({ "field": [WALL], "hand": [FILLER] }),
        )
    }

    /// The base face's play at `x`, aimed at the enemy hero.
    fn at_hero(x: i32) -> Value {
        merged(pick::targets(pick::hero("p2")), json!({ "x": x }))
    }

    /// p2's hero and WALL each took `amount`, once.
    fn expect_every_enemy(s: &mut Scenario, wall: &str, amount: i32) {
        s.expect_health(P2, 30 - amount);
        s.expect_stats(wall, json!({ "health": 40 - amount }));
        assert_eq!(hits(s.last_events()).len(), 2);
    }

    #[test]
    fn declares_no_numbers() {
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r5256000_x_3_deals_3_to_the_chosen_enemy_alone() {
            let mut s = fft("fvh091-three", false);
            let wall = s.unit_at(P2, 1).id;
            s.play(ID, at_hero(3));
            s.expect_health(P2, 30 - ky_fft_damage(3));
            assert_eq!(ky_fft_damage(3), 3);
            s.expect_stats(wall.as_str(), json!({ "health": 40 }));
        }

        #[test]
        fn r5256000_a_chosen_unit_takes_it() {
            let mut s = fft("fvh091-unit", false);
            let wall = s.unit_at(P2, 1).id;
            s.play(ID, merged(pick::targets(pick::id(&wall)), json!({ "x": 6 })));
            s.expect_stats(wall.as_str(), json!({ "health": 40 - 9 }));
            s.expect_health(P2, 30);
        }

        #[test]
        fn r5256000_a_power_of_2_deals_it_to_every_enemy_instead() {
            for (x, amount) in [(2, 2), (4, 5), (8, 13)] {
                let mut s = fft(&format!("fvh091-power-{x}"), false);
                let wall = s.unit_at(P2, 1).id;
                s.play(ID, at_hero(x));
                expect_every_enemy(&mut s, &wall, amount);
                s.expect_health(P1, 30);
            }
        }

        #[test]
        fn r5256000_x_1_deals_0_so_nothing_is_hit() {
            let mut s = fft("fvh091-one", false);
            s.play(ID, at_hero(1));
            assert!(hits(s.last_events()).is_empty());
            s.expect_health(P2, 30);
        }

        #[test]
        fn r280_r5256000_the_preview_is_the_damage_at_the_mana_held() {
            for (mana, amount) in [(3, 3), (4, 5), (1, 0)] {
                let s = weapon_board(
                    &format!("fvh091-preview-{mana}"),
                    json!({ "hand": [ID, FILLER], "mana": mana }),
                    json!({ "hand": [FILLER] }),
                );
                let id = s.card(ID).id.clone();
                assert_eq!(preview_seen(&s, P1, &id), vec![(FORMULA.to_string(), amount, None)], "at {mana}");
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5256000_x_rounds_up_to_a_power_of_2_at_least_2_and_hits_every_enemy() {
            for (x, amount) in [(1, 2), (3, 5), (5, 13), (8, 13)] {
                let mut s = fft(&format!("fvh091-radiant-{x}"), true);
                let wall = s.unit_at(P2, 1).id;
                s.play(ID, json!({ "x": x }));
                expect_every_enemy(&mut s, &wall, amount);
            }
        }

        #[test]
        fn r280_r5256000_the_preview_reads_the_padded_x() {
            let s = weapon_board(
                "fvh091-radiant-preview",
                json!({ "hand": [{ "def": ID, "radiant": true }, FILLER], "mana": 3 }),
                json!({ "hand": [FILLER] }),
            );
            let id = s.card(ID).id.clone();
            assert_eq!(preview_seen(&s, P1, &id), vec![(FORMULA.to_string(), 5, None)]);
        }
    }
}
