//! #74 Adaptive UI (SPEC §8.3): cost X, "Deal X damage to a target, heal your hero X, draw X, summon
//! an X/X Ghoul Token"; radiant "2X damage, heal 3X, draw 2X, a 3X/3X token" — the radiant cell
//! changes only the four multipliers (§8 Conventions). The token is a Ghoul Token (§7, R353: X/X with
//! Pierce); X is at least 1 (R348).
//!
//! X is a PLAY choice, not a prompt (R81, §10.6): the play action carries it, `make_context` hands it
//! to the hook as `ctx.x`, and R65 makes an X-cost card cost exactly X. R348 (`MIN_CHOSEN_X`) keeps
//! X = 0 out of the hook; the guard below covers a run with no X. Clauses resolve in §8's order:
//! damage before heal, draw before summon (a draw into a full hand burns, R4, before the board grows).
//! "Heal your hero X" is `{ of: "selfHero" }`, not the target (R19's licence is #47 Fig of Life's);
//! §3 gives a hero no maximum. The token is the Ghoul Token plus a §7 `statsOverride`, summoned on its
//! base face on both faces (R349 would double the 3X); R64 places it in the leftmost empty unit zone.

use jackioh_engine::effects::{damage, draw, heal, summon};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-074";

/// §7, R353: the token is the catalog's Ghoul Token, named by id.
const GHOUL_TOKEN: &str = "core-t-ghoul";

/// R81: "a target" travels in the play action and never pauses resolution. §8's Conventions make it
/// "all legal units and heroes on either side"; a hero is always on the board, so this declaration
/// can never make a play illegal, and R90 fizzles it rather than refusing the play if it somehow has
/// no answer.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// §10.9: a hook may READ state to compute an effect's arguments; it never writes. This card's only
/// read is the declared X off the context — negative and fractional values are impossible (the play
/// validator refuses X < 0), and the clamp is belt and braces so no clause can go backwards.
fn chosen_x(ctx: &EffectContext<'_>) -> i32 {
    ctx.x.max(0)
}

/// The four multipliers are the whole of the radiant text: the declared numbers `damage`, `heal`,
/// `draw` and `ghoul` (R386), 2, 3, 2 and 3 on the Radiant face and tuned there alone — the base face
/// prints a bare X, which is always 1X (R749).
fn adaptive_ui() -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let x = chosen_x(ctx);
            // R348 keeps a play from choosing X = 0; a run with none makes nothing, and no 0/0 token.
            if x == 0 {
                return vec![];
            }
            let stats = param(&*ctx, "ghoul") * x;
            let amount = param(&*ctx, "damage") * x;
            let healed = param(&*ctx, "heal") * x;
            let drawn = param(&*ctx, "draw") * x;
            vec![
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount }))),
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": healed }))),
                draw(json_as(json!({ "count": drawn }))),
                summon(json_as(json!({ "defId": GHOUL_TOKEN, "statsOverride": { "attack": stats, "health": stats } }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = adaptive_ui();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #74 Adaptive UI — SPEC §8.3, BUILD M4-T4: "X=2: 2 damage, heal 2, draw 2, a 2/2 Ghoul Token;
// radiant 4 / 6 / 4 / 6-6; X=0 refused (R348)".
// X is a play choice (R81): every test passes it as `play(..., { x })` with the declared target,
// never as an answered prompt (§10.6: no Core card opens an `x` prompt).
// Each hero starts below HERO_HEALTH (`health: 20`) so the heal shows as a number.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const X_REFUSAL: &str = "X must be at least 1";

    /// `scenario(opts)` with the shipped cards registered first.
    fn setup(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    use crate::js;

    /// The X values `legal_actions` offers for p1's Adaptive UI, in order (R348).
    fn offered_x(s: &Scenario) -> Vec<i64> {
        let card = s.card("core-074").id.clone();
        let xs: IndexSet<i64> = legal_actions(s.state(), P1)
            .iter()
            .map(js)
            .filter(|action| action["type"] == "play" && action["instanceId"] == card.as_str())
            .filter_map(|action| action["x"].as_i64())
            .collect();
        let mut xs: Vec<i64> = xs.into_iter().collect();
        xs.sort();
        xs
    }

    fn types_of_last_step(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .map(js)
            .filter_map(|event| event["type"].as_str().map(str::to_string))
            .collect()
    }

    fn at_enemy_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    mod adaptive_ui_base {
        use super::*;

        #[test]
        fn x_2_2_damage_to_the_target_heal_your_hero_2_draw_2_and_a_2_2_ghoul_token_in_s8s_order() {
            let mut s = setup(json!({
                "seed": "core-074-x2",
                "p1": {
                    "hand": ["core-074", "core-005"],
                    "library": ["core-035", "core-036", "core-013"],
                    "health": 20,
                },
                "p2": { "hand": ["core-005"], "health": 30 },
            }));

            s.play("core-074", json!({ "x": 2, "targets": at_enemy_hero() }));

            // Deal X damage to a target.
            s.expect_health(P2, 28);
            // Heal your hero X.
            s.expect_health(P1, 22);
            // Draw X: one card played out of two, two drawn.
            assert_eq!(s.hand(P1).len(), 3);
            assert_eq!(s.pile(P1, "library").len(), 1);
            // Summon an X/X Ghoul Token: the catalog's Ghoul Token with a §7 stat override (R353).
            let token = s.unit(P1, 1);
            assert_eq!(token.as_ref().map(|card| card.def_id.as_str()), Some("core-t-ghoul"));
            let token = token.expect("a Ghoul Token in lane 1");
            s.expect_stats(&token, json!({ "attack": 2, "health": 2, "maxHealth": 2 }));
            assert_eq!(js(&s.stats(&token).keywords), json!([{ "kind": "Pierce" }]));
            // R65: an X-cost card being played costs exactly X.
            s.expect_mana(P1, 2);
            // The four clauses resolve in the order §8 writes them.
            s.expect_events(json!(["cardPlayed", "damage", "healed", "drawn", "drawn", "summoned"]));
        }

        #[test]
        fn the_damage_goes_to_the_chosen_unit_while_the_heal_always_goes_to_your_own_hero() {
            let mut s = setup(json!({
                "seed": "core-074-unit-target",
                "p1": { "hand": ["core-074", "core-005"], "library": ["core-035", "core-036"], "health": 20 },
                "p2": { "field": ["core-013"], "hand": ["core-005"], "health": 30 },
            }));
            let shredder = s.unit(P2, 1).expect("setup: Jlockeed Shredder-10 in p2's lane 1");

            s.play("core-074", json!({ "x": 2, "targets": [{ "pick": "instance", "instanceId": shredder.id }] }));

            s.expect_stats(&shredder, json!({ "health": 8, "maxHealth": 10 }))
                .expect_health(P1, 22)
                // "Heal your hero X" is never the chosen target's hero.
                .expect_health(P2, 30);
        }

        #[test]
        fn r348_x_0_is_refused_legal_actions_offers_x_from_1_to_current_mana_and_the_play_is_turned_away() {
            let mut s = setup(json!({
                "seed": "core-074-x0",
                "p1": { "hand": ["core-074", "core-005"], "library": ["core-035"], "health": 20 },
                "p2": { "hand": ["core-005"], "health": 30 },
            }));

            assert_eq!(offered_x(&s), [1, 2, 3, 4]);
            s.expect_refused_with(|s| s.play("core-074", json!({ "x": 0, "targets": at_enemy_hero() })), X_REFUSAL);
            // A play that names no X is X = 0 too, and refused the same way.
            s.expect_refused_with(|s| s.play("core-074", json!({ "targets": at_enemy_hero() })), X_REFUSAL);

            // Nothing happened: no damage, no heal, no draw, no token, and the card is still in hand.
            s.expect_health(P2, 30).expect_health(P1, 20);
            let hand: Vec<String> = s.hand(P1).into_iter().map(|card| card.def_id).collect();
            assert_eq!(hand, ["core-074", "core-005"]);
            assert!(s.unit(P1, 1).is_none());
            assert_eq!(s.state().players.p1.turn_log.cards_played, 0);
            s.expect_mana(P1, 4);
        }

        #[test]
        fn r348_with_0_mana_there_is_no_x_to_choose_so_adaptive_ui_is_not_playable_at_all() {
            let mut s = setup(json!({
                "seed": "core-074-no-mana",
                "p1": { "hand": ["core-074", "core-005"], "mana": 0 },
                "p2": { "hand": ["core-005"] },
            }));
            assert!(offered_x(&s).is_empty());
            s.expect_refused_with(|s| s.play("core-074", json!({ "x": 0, "targets": at_enemy_hero() })), X_REFUSAL);
        }

        #[test]
        fn a_full_unit_row_fizzles_only_the_summon_the_other_three_clauses_still_happen_r64_s8_conventions() {
            let mut s = setup(json!({
                "seed": "core-074-full-board",
                "p1": {
                    "field": ["core-001", "core-002", "core-003", "core-004", "core-007"],
                    "hand": ["core-074", "core-005"],
                    "library": ["core-035", "core-036"],
                    "health": 20,
                },
                "p2": { "hand": ["core-005"], "health": 30 },
            }));

            s.play("core-074", json!({ "x": 2, "targets": at_enemy_hero() }));

            s.expect_health(P2, 28).expect_health(P1, 22);
            assert_eq!(s.hand(P1).len(), 3);
            // Five lanes, five units already: no sixth.
            assert_eq!(s.unit(P1, 5).map(|card| card.def_id), Some("core-007".to_string()));
            assert!(!types_of_last_step(&s).contains(&"summoned".to_string()));
        }
    }

    mod adaptive_ui_radiant {
        use super::*;

        #[test]
        fn x_2_radiant_4_damage_heal_6_draw_4_and_a_6_6_ghoul_token() {
            let mut s = setup(json!({
                "seed": "core-074-radiant-x2",
                "p1": {
                    "hand": [{ "def": "core-074", "radiant": true }, "core-005"],
                    "library": ["core-035", "core-036", "core-013", "core-019", "core-043"],
                    "health": 20,
                },
                "p2": { "hand": ["core-005"], "health": 30 },
            }));
            assert_eq!(s.hand(P1).first().map(|card| card.radiant), Some(true));

            s.play("core-074", json!({ "x": 2, "targets": at_enemy_hero() }));

            // 2X damage.
            s.expect_health(P2, 26);
            // Heal 3X.
            s.expect_health(P1, 26);
            // Draw 2X.
            assert_eq!(s.hand(P1).len(), 5);
            assert_eq!(s.pile(P1, "library").len(), 1);
            // A 3X/3X token.
            let token = s.unit(P1, 1);
            assert_eq!(token.as_ref().map(|card| card.def_id.as_str()), Some("core-t-ghoul"));
            let token = token.expect("a Ghoul Token in lane 1");
            s.expect_stats(&token, json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
            // The token is summoned by an effect, so it is not itself Radiant (§5.2, R74): the 3X is the
            // card's multiplier, and R349's fallback does not double it again.
            assert!(!token.radiant);
            s.expect_mana(P1, 2)
                .expect_events(json!(["cardPlayed", "damage", "healed", "drawn", "summoned"]));
        }

        #[test]
        fn x_1_radiant_2_damage_heal_3_draw_2_a_3_3_token() {
            let mut s = setup(json!({
                "seed": "core-074-radiant-x1",
                "p1": {
                    "hand": [{ "def": "core-074", "radiant": true }, "core-005"],
                    "library": ["core-035", "core-036", "core-013"],
                    "health": 20,
                },
                "p2": { "hand": ["core-005"], "health": 30 },
            }));

            s.play("core-074", json!({ "x": 1, "targets": at_enemy_hero() }));

            s.expect_health(P2, 28).expect_health(P1, 23);
            assert_eq!(s.hand(P1).len(), 3);
            let token = s.unit(P1, 1).expect("a Ghoul Token in lane 1");
            s.expect_stats(&token, json!({ "attack": 3, "health": 3, "maxHealth": 3 }))
                .expect_mana(P1, 3);
        }

        #[test]
        fn r348_x_0_radiant_is_refused_the_same_way() {
            let mut s = setup(json!({
                "seed": "core-074-radiant-x0",
                "p1": {
                    "hand": [{ "def": "core-074", "radiant": true }, "core-005"],
                    "library": ["core-035"],
                    "health": 20,
                },
                "p2": { "hand": ["core-005"], "health": 30 },
            }));

            assert_eq!(offered_x(&s), [1, 2, 3, 4]);
            s.expect_refused_with(|s| s.play("core-074", json!({ "x": 0, "targets": at_enemy_hero() })), X_REFUSAL);
            assert!(s.unit(P1, 1).is_none());
            assert_eq!(s.hand(P1).len(), 2);
            assert_eq!(s.state().players.p1.turn_log.cards_played, 0);
        }

        #[test]
        fn r346_the_radiant_cards_ghoul_pierces_a_3_3_from_x_1_hits_a_hero_behind_going_long_for_3() {
            let mut s = setup(json!({
                "seed": "core-074-radiant-pierce",
                "p1": { "hand": [{ "def": "core-074", "radiant": true }, "core-005"], "library": ["core-035", "core-036"] },
                "p2": { "backrow": ["core-084"], "hand": ["core-005"], "health": 30 },
            }));
            s.play("core-074", json!({ "x": 1, "targets": at_enemy_hero() }));
            // 2X = 2 to the hero, Armor 2 takes it all.
            s.expect_health(P2, 30);
            let ghoul = s.unit(P1, 1).expect("a Ghoul Token in lane 1");
            assert_eq!(js(&s.stats(&ghoul).keywords), json!([{ "kind": "Pierce" }]));
            // Back to p1's next turn, where the Ghoul is no longer summoning sick (§4.1).
            s.end_turn().end_turn();
            s.attack(&ghoul, "hero");
            s.expect_health(P2, 27);
        }
    }

    #[test]
    fn r386_each_radiant_multiplier_moves_one_step_and_the_base_face_has_none_to_move() {
        // Radiant at X = 1: 2 damage, heal 3, draw 2, a 3/3 Ghoul. One Upgrade of each makes it 3, 4,
        // 3 and 4/4; one Degrade 1, 2, 1 and 2/2.
        for (upgrade, damage, healed, drawn, ghoul) in [(true, 3, 4, 3, 4), (false, 1, 2, 1, 2)] {
            let mut s = setup(json!({
                "seed": "core-074-tuned",
                "p1": {
                    "hand": [{ "def": "core-074", "radiant": true }, "core-005"],
                    "library": ["core-035", "core-036", "core-013", "core-011"],
                    "health": 20,
                },
                "p2": { "hand": ["core-005"], "health": 30 },
            }));
            for key in ["damage", "heal", "draw", "ghoul"] {
                if upgrade {
                    crate::upgrade_number(&mut s, "core-074", key);
                } else {
                    crate::degrade_number(&mut s, "core-074", key);
                }
            }
            s.play("core-074", json!({ "x": 1, "targets": at_enemy_hero() }));
            s.expect_health(P2, 30 - damage);
            s.expect_health(P1, 20 + healed);
            assert_eq!(s.pile(P1, "library").len(), 4 - drawn as usize);
            let token = s.unit(P1, 1).expect("a Ghoul Token in lane 1");
            s.expect_stats(&token, json!({ "attack": ghoul, "health": ghoul }));
        }
        let s = setup(json!({ "p1": { "hand": ["core-074"] } }));
        for key in ["damage", "heal", "draw", "ghoul"] {
            assert!(!crate::can_upgrade_number(&s, "core-074", key));
            assert!(!crate::can_degrade_number(&s, "core-074", key));
        }
    }
}
