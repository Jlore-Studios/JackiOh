//! C #45 Nature Titan (SPEC §8.6 row 45). (2) Unit, Legendary, 6/6 → 12/12.
//!   Base:    "Tribute 1\nCry and whenever this attacks: Draw {draw} and heal your hero {heal}." — 1, 3
//!   Radiant: "Tribute 1\nCry and whenever this attacks: Draw {draw} and heal your hero {heal}." — 2, 6
//!   Engine:  "A Tribute cost (§6.3, R101), which may pay for its own zone (§3.2, R391). One script for
//!            the Cry and the attack trigger, as #22 Carnivorous Cube's Cry and Death share one; the
//!            trigger answers `attackDeclared` for this unit, forced attacks included. Tunes: draw 1 ↑;
//!            heal 3 ↑."
//!
//! Tribute 1 is the play validator's (`playChoices.ts`), read off `staticFlags.tribute` as #66 The
//! Rock's is: the units travel in the play action's `tributes` (R81, R90), a Sheep Token pays it alone
//! (R101), and a board with no Unit to Tribute refuses the play. Whether a full board may be played
//! into the zone the Tribute empties (R391) is the validator's as well; nothing here names a zone.
//! The 1 is the printed keyword's, not a declared number, so it is written here once.
//!
//! "Cry and whenever this attacks" is one effect list, `titan` below, run by the Cry and by a trigger
//! on `attackDeclared` whose attacker is this card. Every attack it makes emits one — declared by its
//! controller or forced (R53, #9 Moths to the Flame) — and the trigger answers each; an attack on it
//! names it as the target, not the attacker, so defending does nothing.
//!
//! "Heal your hero" is §6.3 Heal on a hero: no cap (§3). The two numbers are the declared `draw` and
//! `heal` (R386), read through `param`; the faces differ only in them, so both run this one script.

use jackioh_engine::effects::{draw, heal};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-045";

/// "Tribute 1" (§6.3): one of your units, or one Sheep Token.
const TRIBUTE_COST: i32 = 1;

/// "Draw {draw} and heal your hero {heal}", the text the Cry and the attack trigger share.
fn titan(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let count = param(&*ctx, "draw");
    let amount = param(&*ctx, "heal");
    vec![
        draw(json_as(json!({ "count": count }))),
        heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": amount }))),
    ]
}

/// "Whenever this attacks": every `attackDeclared` whose attacker is this card, forced ones included.
/// The test is in `run`: a `when` is read for traps only (R99, `traps.ts`), and an ordinary trigger's
/// `run` is its whole condition, as #32 Prem Panther's is.
fn on_attack() -> TriggerDef {
    TriggerDef::new("45-whenever-this-attacks", &[GameEventType::AttackDeclared], |ctx, event| {
        let GameEvent::AttackDeclared { attacker_id, .. } = event else {
            return vec![];
        };
        let mine = ctx.self_.as_ref().is_some_and(|me| me.id == *attacker_id);
        if !mine {
            return vec![];
        }
        titan(ctx)
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        cry: Some(hook(titan)),
        triggers: vec![on_attack()],
        ..Script::default()
    };

    // The same script: the Radiant face's draw 2 and heal 6 are its declared numbers, which `param` reads off the running face.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// C #45 Nature Titan — SPEC §8.6 row 45, BUILD M9 Classic row C 45: "Tribute 1 (§6.3, R101): with no
// Unit to Tribute it can't be played; on a full board it may take the tributed Unit's zone when that
// pile is the one card, without Reborn and not Locked (R391), never the top of a Stack pile; a Sheep
// overpays (R101); Cry, and whenever it attacks (each `attackDeclared` for it, forced attacks
// included): draw 1 and heal your hero 3 with no cap (§6.3 Heal); defending does nothing; radiant
// 12/12: draw 2, heal 6; its tuned numbers (draw, heal) read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TITAN: &str = "classic-045";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const SHEEP: &str = "core-t-sheep"; // Sheep Token, worth 2 Tributes.
    const DEFENDER: &str = "core-003"; // Right-house defender 1/1: Taunt, Divine Shield, Reborn.
    const FIENDER: &str = "core-092"; // Felinor Fiender: Stack.
    const MOTHS: &str = "core-009"; // 1/14: Start of turn: every enemy Unit attacks this.
    const TEMPO: &str = "core-011"; // (1) Unit 3/3 Rush.
    const ANCHOR: &str = "core-010";
    const A: &str = "core-020";
    const B: &str = "core-001";
    const C: &str = "core-045";

    /// The TS default for `player` is "p1"; every caller passes it.
    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    /// The TS default for `player` is "p1"; every caller passes it.
    fn unit_defs(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        [1, 2, 3, 4, 5].into_iter().map(|lane| s.unit(player, lane).map(|card| card.def_id)).collect()
    }

    /// TS `stepParam(s.card(ref), key, steps)`: the step written on the card as it stands in the state.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        let instance = find_instance_mut(s.state_mut(), &id).expect("the card to step is in the state");
        step_param(instance, key, steps);
    }

    mod c45_nature_titan {
        use super::*;

        #[test]
        fn declares_tribute_1_and_runs_one_script_on_both_faces() {
            crate::register_all();
            assert_eq!(registered_catalog()[ID].id, TITAN);
            let scripts = script();
            assert_eq!(scripts.base.static_flags.as_ref().and_then(|flags| flags.tribute), Some(1));
            // TS `expect(radiant).toBe(base)`: the Radiant face is the same script.
            assert_eq!(scripts.radiant.static_flags, scripts.base.static_flags);
            assert_eq!(scripts.radiant.cry.is_some(), scripts.base.cry.is_some());
            assert_eq!(
                scripts.radiant.triggers.iter().map(|trigger| trigger.id.clone()).collect::<Vec<_>>(),
                scripts.base.triggers.iter().map(|trigger| trigger.id.clone()).collect::<Vec<_>>(),
            );
        }

        mod base {
            use super::*;

            #[test]
            fn r101_with_no_unit_to_tribute_it_cant_be_played() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [TITAN, ANCHOR], "library": [A] }, "p2": { "hand": [ANCHOR] } }));

                s.expect_refused_with(|s| s.play(TITAN, json!({})), "Tribute");
                s.expect_in_zone(TITAN, "hand");
            }

            #[test]
            fn tribute_1_then_its_cry_draw_1_and_heal_your_hero_3() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TITAN, ANCHOR], "field": [VANILLA], "library": [A, B], "health": 20 },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(TITAN, json!({ "tributes": [VANILLA] }));

                s.expect_in_zone(VANILLA, "graveyard").expect_in_zone(TITAN, "field");
                s.expect_stats(TITAN, json!({ "attack": 6, "health": 6 }));
                assert_eq!(hand_defs(&s, P1), vec![ANCHOR, A]);
                s.expect_health(P1, 23);
            }

            #[test]
            fn s6_3_the_heal_has_no_cap_a_hero_at_30_reaches_33() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TITAN, ANCHOR], "field": [VANILLA], "library": [A] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(TITAN, json!({ "tributes": [VANILLA] }));

                s.expect_health(P1, 33);
            }

            #[test]
            fn r101_a_sheep_token_overpays_the_tribute_1_alone() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TITAN, ANCHOR], "field": [SHEEP], "library": [A] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(TITAN, json!({ "tributes": [SHEEP] }));

                s.expect_in_zone(TITAN, "field");
                assert_eq!(unit_defs(&s, P1).iter().filter(|id| id.as_deref() == Some(SHEEP)).count(), 0);
            }

            #[test]
            fn r101_only_a_card_that_says_so_tributes_the_opponents_units_an_enemy_unit_cant_pay_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TITAN, ANCHOR], "field": [VANILLA], "library": [A] },
                    "p2": { "hand": [ANCHOR], "field": [TEMPO] },
                }));

                // TS `toThrow(/tribute/i)`: "ribute" is in the message whichever case its first letter takes.
                s.expect_refused_with(|s| s.play(TITAN, json!({ "tributes": [TEMPO] })), "ribute");
                s.expect_in_zone(TITAN, "hand").expect_in_zone(TEMPO, "field");
            }

            #[test]
            fn r391_on_a_full_board_it_may_take_the_zone_its_tribute_empties() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TITAN, ANCHOR], "field": [MENACE, MENACE, VANILLA, MENACE, MENACE], "library": [A] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(TITAN, json!({ "tributes": [VANILLA], "zone": 3 }));

                assert_eq!(json!(unit_defs(&s, P1)), json!([MENACE, MENACE, TITAN, MENACE, MENACE]));
            }

            #[test]
            fn r391_r64_on_a_full_board_it_cant_take_the_zone_of_a_tributed_reborn_unit_which_is_reserved() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TITAN, ANCHOR], "field": [MENACE, MENACE, DEFENDER, MENACE, MENACE], "library": [A] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.expect_refused(|s| s.play(TITAN, json!({ "tributes": [DEFENDER], "zone": 3 })));
                s.expect_in_zone(TITAN, "hand");
            }

            #[test]
            fn r391_r13_on_a_full_board_it_never_takes_the_zone_of_a_tributed_stack_piles_top_the_card_beneath_resumes() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [TITAN, ANCHOR],
                        "field": [VANILLA, { "def": FIENDER, "stack": true }, MENACE, MENACE, MENACE, MENACE],
                        "library": [A],
                    },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.expect_refused(|s| s.play(TITAN, json!({ "tributes": [FIENDER], "zone": 1 })));
                s.expect_in_zone(TITAN, "hand");
            }

            #[test]
            fn r391_on_a_full_board_it_cant_take_a_locked_zone_even_the_one_its_tribute_empties() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TITAN, ANCHOR], "field": [MENACE, MENACE, VANILLA, MENACE, MENACE], "library": [A] },
                    "p2": { "hand": [ANCHOR] },
                }));
                // Lane 3 is Locked with its Vanilla in it, set directly as Core #60's test sets a Lock.
                s.state_mut().players.p1.locks.units[2] = true;

                s.expect_refused(|s| s.play(TITAN, json!({ "tributes": [VANILLA], "zone": 3 })));
                s.expect_in_zone(TITAN, "hand").expect_in_zone(VANILLA, "field");
            }

            #[test]
            fn whenever_it_attacks_draw_1_and_heal_your_hero_3_after_the_attack() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [ANCHOR], "field": [TITAN], "library": [A, B], "health": 20 },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.attack(TITAN, "hero");

                s.expect_health(P2, 24).expect_health(P1, 23);
                assert_eq!(hand_defs(&s, P1), vec![ANCHOR, A]);
                s.expect_events(json!(["attackDeclared", "damage", "drawn", "healed"]));
            }

            #[test]
            fn r53_a_forced_attack_is_an_attack_too_moths_to_the_flames_start_of_turn_makes_it_attack_and_it_draws_and_heals() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [ANCHOR], "field": [TITAN], "library": [A, B], "health": 20 },
                    "p2": { "hand": [ANCHOR], "field": [MOTHS], "library": [C] },
                }));

                s.end_turn();

                assert_eq!(s.state().active, P2);
                assert!(s.events().iter().any(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. })));
                assert_eq!(hand_defs(&s, P1), vec![ANCHOR, A]);
                s.expect_health(P1, 23);
            }

            #[test]
            fn r212_the_attack_trigger_is_answered_after_that_combats_state_check_so_a_titan_that_died_in_it_draws_and_heals_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [ANCHOR], "field": [TITAN], "library": [A, B], "health": 20 },
                    "p2": { "hand": [ANCHOR], "field": [MENACE] },
                }));

                s.attack(TITAN, MENACE);

                s.expect_in_zone(TITAN, "graveyard");
                assert_eq!(hand_defs(&s, P1), vec![ANCHOR]);
                s.expect_health(P1, 20);
            }

            #[test]
            fn defending_does_nothing_an_attack_on_it_draws_and_heals_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [ANCHOR], "field": [TITAN], "library": [A], "health": 20 },
                    "p2": { "hand": [ANCHOR], "field": [TEMPO] },
                }));

                s.attack(TEMPO, TITAN);

                assert_eq!(hand_defs(&s, P1), vec![ANCHOR]);
                s.expect_health(P1, 20);
            }

            #[test]
            fn r386_an_upgrade_of_draw_makes_it_draw_2_of_heal_heal_4() {
                crate::register_all();
                let mut draw_up = scenario(json!({
                    "p1": { "hand": [TITAN, ANCHOR], "field": [VANILLA], "library": [A, B, C], "health": 20 },
                    "p2": { "hand": [ANCHOR] },
                }));
                step(&mut draw_up, TITAN, "draw", 1);
                draw_up.play(TITAN, json!({ "tributes": [VANILLA] }));
                assert_eq!(hand_defs(&draw_up, P1), vec![ANCHOR, A, B]);
                draw_up.expect_health(P1, 23);

                let mut heal_up = scenario(json!({
                    "p1": { "hand": [TITAN, ANCHOR], "field": [VANILLA], "library": [A, B, C], "health": 20 },
                    "p2": { "hand": [ANCHOR] },
                }));
                step(&mut heal_up, TITAN, "heal", 1);
                heal_up.play(TITAN, json!({ "tributes": [VANILLA] }));
                heal_up.expect_health(P1, 24);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_12_12_whose_cry_draws_2_and_heals_6() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TITAN, "radiant": true }, ANCHOR], "field": [VANILLA], "library": [A, B, C], "health": 20 },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(TITAN, json!({ "tributes": [VANILLA] }));

                s.expect_stats(TITAN, json!({ "attack": 12, "health": 12 }));
                assert_eq!(hand_defs(&s, P1), vec![ANCHOR, A, B]);
                s.expect_health(P1, 26);
            }

            #[test]
            fn whenever_it_attacks_it_draws_2_and_heals_6() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [ANCHOR], "field": [{ "def": TITAN, "radiant": true }], "library": [A, B, C], "health": 20 },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.attack(TITAN, "hero");

                s.expect_health(P2, 18).expect_health(P1, 26);
                assert_eq!(hand_defs(&s, P1), vec![ANCHOR, A, B]);
            }

            #[test]
            fn r101_still_needs_its_tribute_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": TITAN, "radiant": true }, ANCHOR], "library": [A] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.expect_refused_with(|s| s.play(TITAN, json!({})), "Tribute");
            }

            #[test]
            fn r386_an_upgrade_of_heal_on_the_radiant_face_steps_6_to_7() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [ANCHOR], "field": [{ "def": TITAN, "radiant": true }], "library": [A, B, C], "health": 20 },
                    "p2": { "hand": [ANCHOR] },
                }));
                step(&mut s, TITAN, "heal", 1);

                s.attack(TITAN, "hero");

                s.expect_health(P1, 27);
            }
        }
    }
}
