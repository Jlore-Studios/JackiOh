//! #24 Efficiency Dividend (SPEC §8.2, R65, R81, R68, §5.1).
//!
//! Base: "Choose one: Deal X+1 damage, heal a target 4X, or gain X mana next turn. End of turn: Return
//! this to hand". Radiant: "Deal 2X+1 damage, heal a target 8X, or gain X+1 mana next turn", the modes,
//! target and return to hand kept (§8 Conventions). Patch v0.3.10 (#831) set these numbers (X, 2X and
//! floor(X/2) before; 2X, 4X and X on the Radiant face). The price is
//! still X, bounded by the play validator, so nothing here re-checks it (R65).
//!
//! R81: X, the mode and the target all travel in the `play` action and never pause resolution, so the
//! hook reads `ctx.x`, `ctx.modes` (through `chosenOptions`) and `{ of: "chosen" }`. "Gain X mana next
//! turn" is a positive `mana.nextTurnMod` (§2.3), the one-shot modifier Hinder makes negative.
//! §5.1 and R68: the spell returns from the graveyard at the end of its turn. No verb sets
//! `returnToHandAtEndOfTurn` yet, so the hook gates on the flag OR this turn's play log (see #23).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-024";

// The three §8.2 modes, in the order the cell lists them. A card file exports only
// `{ ID, script }` (§10.9), so the mode names live in the declaration below and the test
// reads them back off `base.modes`.
const MODE_DAMAGE: &str = "damage";
const MODE_HEAL: &str = "heal";
const MODE_MANA: &str = "mana";

fn modes() -> Vec<ModeDecl> {
    vec![ModeDecl {
        kind: PromptKind::Mode,
        options: vec![MODE_DAMAGE.to_string(), MODE_HEAL.to_string(), MODE_MANA.to_string()],
    }]
}

/// R81: the target travels with the play, for the damage and heal modes alone (`forModes`). §8's
/// Conventions pick "a target" from every legal unit and hero and only an EMPTY set fizzles; a hero
/// always stands, so those modes always name one and the mana mode names none (R90). With `min: 0`
/// for every mode, a damage or heal play could name nobody and pay its X for nothing.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        for_modes: Some(vec![MODE_DAMAGE.to_string(), MODE_HEAL.to_string()]),
        ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))
    }]
}

/// Patch v0.3.10 (#831): "Deal X+1" (Radiant "2X+1") damage: the declared multiple of X, plus this.
const DIVIDEND_DAMAGE_FLAT: i32 = 1;

/// "Gain X mana next turn" on the base face, "X+1" on the Radiant: this much more there.
const DIVIDEND_RADIANT_MANA: i32 = 1;

/// The damage and heal modes' multiples are the declared numbers `damage` and `heal` (R386): X and 4X,
/// 2X and 8X on the Radiant face, where `damage` is tuned alone (R749: the base face's "Deal X+1
/// damage" prints no multiple and is always 1X). `bonus_mana` is the mana mode's Radiant difference.
fn dividend(bonus_mana: i32) -> Script {
    Script {
        modes: modes(),
        targets: targets(),
        cry: Some(hook(move |ctx| {
            // The X paid, never below 0.
            let paid = ctx.x.max(0);
            let mode = chosen_options(ctx).into_iter().next();
            match mode.as_deref() {
                Some(MODE_DAMAGE) => {
                    let amount = param(&*ctx, "damage") * paid + DIVIDEND_DAMAGE_FLAT;
                    vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
                }
                Some(MODE_HEAL) => {
                    let amount = param(&*ctx, "heal") * paid;
                    vec![heal(json_as(json!({ "target": { "of": "chosen" }, "amount": amount })))]
                }
                Some(MODE_MANA) => {
                    vec![next_turn_mana(json_as(json!({ "amount": paid + bonus_mana, "player": "self" })))]
                }
                // No mode named: nothing to resolve, and the spell still counts as played (§8 Conventions).
                _ => vec![],
            }
        })),
        end_of_turn: Some(hook(|ctx| {
            // The live `ctx.self`: the card as it stands now.
            let Some(self_) = ctx.live_self() else {
                return vec![];
            };
            let played = was_played_this_turn(ctx.state, self_.controller, self_);
            if self_.return_to_hand_at_end_of_turn != Some(true) && !played {
                return vec![];
            }
            vec![bounce(json_as(json!({ "target": { "of": "self" } })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: dividend(0),
        radiant: dividend(DIVIDEND_RADIANT_MANA),
    }
}

// #24 Efficiency Dividend — SPEC §8.2, BUILD M4-T4 row 24: "X chosen with the play, bounded by
// mana (R81); three modes; next-turn mana +X; returns to hand". Radiant (R275) deals 2X+1, heals 8X and
// gains X+1, still for X paid; each radiant mode is checked at an odd and an even X, and X = 0 is
// refused (R348).
// Nothing prompts (R81), so `state.pending` stays null. Both sides keep a unit and a card in hand so
// no turn auto-ends. The default board is turn 9: 4 mana now and a MAX_MANA 4 refresh, so a positive
// `mana.nextTurnMod` shows as current mana above 4 while max mana stays 4 (§2.3: temporary mana).
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const DIVIDEND: &str = "core-024"; // X-cost Spell.
    const TIMMY: &str = "core-011"; // 3/3, the unit target.
    const FILLER: &str = "core-005";

    fn dividend_in(seed: &str, is_radiant: bool, health: Option<i32>) -> Scenario {
        let mut p1 = json!({
            "hand": [{ "def": DIVIDEND, "radiant": is_radiant }, FILLER],
            "field": [TIMMY],
        });
        if let Some(health) = health {
            p1["health"] = json!(health);
        }
        scenario(json!({
            "seed": seed,
            "p1": p1,
            "p2": { "hand": [FILLER], "field": ["core-008"] },
        }))
    }

    /// p1's mana at their next refresh, two turn hand-overs away; max mana is untouched (§2.3).
    fn next_refresh(s: &mut Scenario) -> i32 {
        s.end_turn(); // p2's turn.
        s.end_turn(); // p1's turn: the refresh.
        assert_eq!(s.state().players.p1.mana.max, MAX_MANA);
        s.state().players.p1.mana.current
    }

    fn at_hero(player: &str) -> Value {
        json!([{ "pick": "hero", "player": player }])
    }

    mod n24_efficiency_dividend {
        use super::*;

        #[test]
        fn r81_declares_three_modes_and_the_damage_and_heal_modes_target_both_read_off_the_play() {
            let scripts = script();
            let base = &scripts.base;
            assert_eq!(
                serde_json::to_value(&base.modes).unwrap(),
                json!([{ "kind": "mode", "options": ["damage", "heal", "mana"] }])
            );
            assert_eq!(base.targets.len(), 1);
            let decl = &base.targets[0];
            assert_eq!(decl.kind, PromptKind::Target);
            // The target belongs to the damage and heal modes, which must name one (§8 Conventions: a hero
            // is always a legal target); the mana mode names none (R90).
            assert_eq!(decl.for_modes, Some(vec!["damage".to_string(), "heal".to_string()]));
            assert_eq!(decl.min, 1);
            assert_eq!(decl.max, 1);
            let filter = decl.filter.as_ref().expect("the target's filter");
            assert_eq!(filter.side, Some(FilterSide::Any));
            assert_eq!(filter.of, Some(vec![FilterOf::Unit, FilterOf::Hero]));
            // "Uses 2X" restates the arithmetic only, so both declarations are kept (§8 Conventions).
            assert_eq!(scripts.radiant.modes, base.modes);
            assert_eq!(scripts.radiant.targets, base.targets);
        }

        mod base {
            use super::*;

            #[test]
            fn r65_r81_x_travels_with_the_play_bounded_by_current_mana_and_costs_exactly_x() {
                crate::register_all();
                let mut s = dividend_in("dividend-x", false, None);
                // §2.3: 0 ≤ X ≤ current mana, so X = 5 on 4 mana is refused.
                s.expect_refused_with(
                    |s| s.play(DIVIDEND, json!({ "x": 5, "modes": ["damage"], "targets": at_hero("p2") })),
                    "mana",
                );

                s.play(DIVIDEND, json!({ "x": 3, "modes": ["damage"], "targets": at_hero("p2") }));

                s.expect_mana(P1, 1);
                assert!(s.state().pending.is_none());
            }

            #[test]
            fn the_damage_mode_deals_x_plus_1_damage_to_the_chosen_hero() {
                crate::register_all();
                let mut s = dividend_in("dividend-damage", false, None);
                s.play(DIVIDEND, json!({ "x": 3, "modes": ["damage"], "targets": at_hero("p2") }));

                s.expect_health(P2, 26);
            }

            #[test]
            fn the_damage_mode_can_pick_a_unit_instead_s8_conventions_any_unit_or_hero() {
                crate::register_all();
                let mut s = dividend_in("dividend-damage-unit", false, None);
                let timmy = s.card(TIMMY).clone();
                s.play(
                    DIVIDEND,
                    json!({ "x": 1, "modes": ["damage"], "targets": [{ "pick": "instance", "instanceId": timmy.id }] }),
                );

                s.expect_stats(&timmy, json!({ "health": 1, "maxHealth": 3 }));
            }

            #[test]
            fn r19_the_heal_mode_heals_the_chosen_target_4x() {
                crate::register_all();
                let mut s = dividend_in("dividend-heal", false, Some(20));
                s.play(DIVIDEND, json!({ "x": 2, "modes": ["heal"], "targets": at_hero("p1") }));

                // §3: a hero has no maximum health, so the heal is a flat 4X.
                s.expect_health(P1, 28);
            }

            #[test]
            fn the_mana_mode_gains_x_mana_next_turn_s2_3() {
                crate::register_all();
                let mut s = dividend_in("dividend-mana", false, None);
                s.play(DIVIDEND, json!({ "x": 3, "modes": ["mana"] }));

                assert_eq!(s.state().players.p1.mana.next_turn_mod, 3);
                assert_eq!(next_refresh(&mut s), MAX_MANA + 3);
                s.expect_mana(P1, MAX_MANA + 3);
            }

            #[test]
            fn the_mana_mode_at_x_1_gains_1() {
                crate::register_all();
                let mut s = dividend_in("dividend-mana-odd", false, None);
                s.play(DIVIDEND, json!({ "x": 1, "modes": ["mana"] }));

                assert_eq!(s.state().players.p1.mana.next_turn_mod, 1);
                assert_eq!(next_refresh(&mut s), MAX_MANA + 1);
            }

            #[test]
            fn s5_1_r68_at_the_end_of_the_turn_it_returns_to_your_hand() {
                crate::register_all();
                let mut s = dividend_in("dividend-return", false, None);
                s.play(DIVIDEND, json!({ "x": 2, "modes": ["damage"], "targets": at_hero("p2") }));
                let spell = s.card(DIVIDEND).clone();
                s.expect_in_zone(&spell, "graveyard");

                s.end_turn();

                s.expect_in_zone(&spell, "hand");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r275_the_damage_mode_deals_2x_plus_1_x_2_deals_5_and_costs_2() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-damage", true, None);
                s.play(DIVIDEND, json!({ "x": 2, "modes": ["damage"], "targets": at_hero("p2") }));

                // R65: X = 2 is what it cost; 5 is what it dealt.
                s.expect_health(P2, 25);
                s.expect_mana(P1, 2);
            }

            #[test]
            fn r275_the_damage_mode_at_an_odd_x_x_3_deals_7() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-damage-odd", true, None);
                s.play(DIVIDEND, json!({ "x": 3, "modes": ["damage"], "targets": at_hero("p2") }));

                s.expect_health(P2, 23);
                s.expect_mana(P1, 1);
            }

            #[test]
            fn r348_the_damage_mode_at_x_0_is_refused_and_nothing_happens() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-damage-zero", true, None);
                s.expect_refused_with(
                    |s| s.play(DIVIDEND, json!({ "x": 0, "modes": ["damage"], "targets": at_hero("p2") })),
                    "X must be at least 1",
                );

                s.expect_health(P2, 30);
                s.expect_mana(P1, 4);
                s.expect_in_zone(DIVIDEND, "hand");
            }

            #[test]
            fn r275_the_damage_mode_can_pick_a_unit_x_1_deals_3_to_it() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-damage-unit", true, None);
                let timmy = s.card(TIMMY).clone();
                s.play(
                    DIVIDEND,
                    json!({ "x": 1, "modes": ["damage"], "targets": [{ "pick": "instance", "instanceId": timmy.id }] }),
                );

                s.expect_in_zone(&timmy, "graveyard");
            }

            #[test]
            fn r275_the_heal_mode_heals_8x_x_2_heals_16() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-heal", true, Some(20));
                s.play(DIVIDEND, json!({ "x": 2, "modes": ["heal"], "targets": at_hero("p1") }));

                s.expect_health(P1, 36);
            }

            #[test]
            fn r275_the_heal_mode_at_an_odd_x_x_3_heals_24() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-heal-odd", true, Some(10));
                s.play(DIVIDEND, json!({ "x": 3, "modes": ["heal"], "targets": at_hero("p1") }));

                s.expect_health(P1, 34);
            }

            #[test]
            fn r348_the_heal_mode_at_x_0_is_refused() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-heal-zero", true, Some(20));
                s.expect_refused_with(
                    |s| s.play(DIVIDEND, json!({ "x": 0, "modes": ["heal"], "targets": at_hero("p1") })),
                    "X must be at least 1",
                );

                s.expect_health(P1, 20);
            }

            #[test]
            fn r275_the_mana_mode_gains_x_plus_1_at_an_odd_x_x_3_gains_4_where_the_base_face_gains_3() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-mana", true, None);
                s.play(DIVIDEND, json!({ "x": 3, "modes": ["mana"] }));

                assert_eq!(s.state().players.p1.mana.next_turn_mod, 4);
                assert_eq!(next_refresh(&mut s), MAX_MANA + 4);
            }

            #[test]
            fn r275_the_mana_mode_at_x_1_gains_2() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-mana-one", true, None);
                s.play(DIVIDEND, json!({ "x": 1, "modes": ["mana"] }));

                assert_eq!(s.state().players.p1.mana.next_turn_mod, 2);
                assert_eq!(next_refresh(&mut s), MAX_MANA + 2);
            }

            #[test]
            fn r275_the_mana_mode_at_an_even_x_x_4_gains_5() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-mana-even", true, None);
                s.play(DIVIDEND, json!({ "x": 4, "modes": ["mana"] }));

                assert_eq!(s.state().players.p1.mana.next_turn_mod, 5);
                assert_eq!(next_refresh(&mut s), MAX_MANA + 5);
            }

            #[test]
            fn r348_the_mana_mode_at_x_0_is_refused_so_the_next_refresh_is_untouched() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-mana-zero", true, None);
                s.expect_refused_with(|s| s.play(DIVIDEND, json!({ "x": 0, "modes": ["mana"] })), "X must be at least 1");

                assert_eq!(s.state().players.p1.mana.next_turn_mod, 0);
                assert_eq!(next_refresh(&mut s), MAX_MANA);
            }

            #[test]
            fn the_return_to_hand_is_kept_s8_conventions() {
                crate::register_all();
                let mut s = dividend_in("dividend-radiant-return", true, None);
                s.play(DIVIDEND, json!({ "x": 1, "modes": ["damage"], "targets": at_hero("p2") }));
                let spell = s.card(DIVIDEND).clone();

                s.end_turn();

                s.expect_in_zone(&spell, "hand");
                assert_eq!(
                    s.hand(P1).iter().find(|card| card.id == spell.id).map(|card| card.radiant),
                    Some(true)
                );
            }

            #[test]
            fn r386_an_upgrade_deals_3x_plus_1_and_a_degrade_1x_plus_1() {
                for (upgrade, per_x) in [(true, 3), (false, 1)] {
                    crate::register_all();
                    let mut s = dividend_in("dividend-radiant-tuned", true, None);
                    let moved = if upgrade {
                        crate::upgrade_number(&mut s, DIVIDEND, "damage")
                    } else {
                        crate::degrade_number(&mut s, DIVIDEND, "damage")
                    };
                    assert_eq!(moved, per_x);
                    s.play(DIVIDEND, json!({ "x": 2, "modes": ["damage"], "targets": at_hero("p2") }));
                    s.expect_health(P2, 30 - (2 * per_x + 1));
                }
            }
        }

        #[test]
        fn r386_an_upgrade_heals_5x_and_a_radiant_degrade_7x() {
            for (radiant, upgrade, per_x) in [(false, true, 5), (true, false, 7)] {
                crate::register_all();
                let mut s = dividend_in("dividend-heal-tuned", radiant, Some(10));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, DIVIDEND, "heal")
                } else {
                    crate::degrade_number(&mut s, DIVIDEND, "heal")
                };
                assert_eq!(moved, per_x);
                s.play(DIVIDEND, json!({ "x": 2, "modes": ["heal"], "targets": at_hero("p1") }));
                s.expect_health(P1, 10 + 2 * per_x);
            }
        }

        #[test]
        fn r749_the_base_face_s_1x_plus_1_damage_is_never_tuned() {
            crate::register_all();
            let s = dividend_in("dividend-base-untuned", false, None);
            assert!(!crate::can_upgrade_number(&s, DIVIDEND, "damage"));
            assert!(!crate::can_degrade_number(&s, DIVIDEND, "damage"));
        }
    }
}
