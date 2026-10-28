//! C #35 Prep (SPEC §8.6 row 35). (0) Spell, Common; Embiggened (2) since issue #2179.
//!   Base:    "Your next Spell this turn costs ({discount}) less.
//!             Embiggened (2): Also draw the highest cost Spell from your deck." — discount 2
//!   Radiant: the same — discount 4
//!   Engine:  "#35 Lunar Eclipse's modifier (`costDiscount`, `onlyType: "Spell"`, this turn, consumed
//!            on use or at cleanup; Cost, §6.3, R65). Tunes: discount 2 ↑."
//!
//! Core #35 Lunar Eclipse's player modifier: `onlyType: "Spell"` (a Unit, Field Spell or Trap play
//! neither pays less nor spends it), `oncePerTurn` (§10.5 step 2 removes it on the first Spell it
//! priced) and `{ until: "thisTurn" }` (§2.2's cleanup takes it unused). R65 floors the price at (0).
//! R70: a cast pays nothing and never uses a discount, so it is left for the next Spell played.
//! The amount is the declared `discount` (R386), read through `param`; both faces run this script.
//!
//! The Embiggened version (§6.3 Embiggen, R81: `instance.embiggened`) then draws the Spell (the card
//! type, not a Field Spell) of the highest current cost in its controller's library, as #30 Archivist
//! draws its highest card (R24: R65's costs, ties to the card nearest the top), by §6.3's
//! `drawFromLibrary`, so it is a draw: R55's counter, the hand cap, the cast-on-draw path. A library
//! with no Spell draws nothing. Through v0.4.12d it had one price and no draw, which `script_v0_4_12d`
//! keeps (R767000).

use jackioh_engine::effects::{add_player_modifier, draw_from_library};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-035";

/// "Your next Spell this turn costs ({discount}) less."
fn discount(ctx: &EffectContext<'_>) -> Effect {
    add_player_modifier(json_as(json!({
        "player": "self",
        "mod": {
            "kind": "costDiscount",
            "amount": param(ctx, "discount"),
            // "your next Spell": Spells only, and only the first one.
            "onlyType": "Spell",
            "oncePerTurn": true,
            // "this turn": §2.2's cleanup takes it if unused.
            "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
        },
    })))
}

/// R24: the library Spell with the highest current cost (R65), ties going to the card nearest the top.
fn highest_spell(ctx: &EffectContext<'_>) -> Option<CardInstance> {
    let mut best: Option<CardInstance> = None;
    let mut best_cost = 0;
    for card in zone_cards(ctx.state, ctx.controller, OffFieldZone::Library) {
        if card_type_of(ctx.state, &card) != CardType::Spell {
            continue;
        }
        let cost = effective_cost(ctx.state, &card, Default::default());
        // Strict, and top down, so a tie keeps the card already held — the one nearer the top (R24).
        if best.is_none() || cost > best_cost {
            best = Some(card.clone());
            best_cost = cost;
        }
    }
    best
}

fn prep(draws: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![discount(ctx)];
            if draws
                && ctx.embiggened
                && let Some(card) = highest_spell(ctx)
            {
                effects.push(draw_from_library(json_as(json!({ "instanceId": card.id }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's 4 is its declared `discount`, which `param` reads off the running face.
    let base = prep(true);
    CardScripts { radiant: base.clone(), base }
}

/// R767000: through v0.4.12d it had one price and drew nothing; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_4_12d() -> CardScripts {
    let base = prep(false);
    CardScripts { radiant: base.clone(), base }
}

// C #35 Prep — SPEC §8.6 row 35, BUILD M9 Classic row C 35: "Your next Spell this turn costs (2) less
// (#35 Lunar Eclipse's modifier: the Spell type only, floored at (0), R65); a Unit, Field Spell or
// Trap play doesn't consume it and the first Spell does; it expires at cleanup; a cast never uses it
// (R70); radiant: (4) less; its tuned number (discount) reads through `param()` (R386)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const PREP: &str = "classic-035";
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ARMOR: &str = "core-073"; // (2) Field Spell; Cry: Draw 1.
    const SHEEPISH: &str = "core-041"; // (1) Trap.
    const VIRUS: &str = "core-090-1"; // CN-Virus: Cast on draw: take 1 damage.
    const ANCHOR: &str = "core-010"; // (0) Spell, Combo 3 — a free play that keeps a turn open.
    const MENACE: &str = "core-019";

    use crate::js;

    /// Every `cardPlayed` so far, as `{ defId, costPaid }`.
    fn spells_paid(s: &Scenario) -> Vec<Value> {
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "cardPlayed")
            .map(|event| json!({ "defId": event["defId"], "costPaid": event["costPaid"] }))
            .collect()
    }

    fn paid_for(s: &Scenario, def_id: &str) -> Vec<i64> {
        spells_paid(s)
            .iter()
            .filter(|play| play["defId"] == def_id)
            .map(|play| play["costPaid"].as_i64().expect("a cardPlayed event's costPaid"))
            .collect()
    }

    mod c_35_prep {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r65_the_next_spell_costs_2_less_floored_at_0_a_1_spell_is_free_and_gives_no_mana_back() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, STOCKPILE, ANCHOR], "library": [MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, STOCKPILE), vec![0]);
                s.expect_mana(PlayerId::P1, 4);
            }

            #[test]
            fn the_next_spell_is_the_next_one_whatever_it_costs_a_0_spell_played_next_consumes_it_and_the_spell_after_pays_in_full() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, ANCHOR, STOCKPILE, FLOOD], "library": [MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(ANCHOR, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, ANCHOR), vec![0]);
                assert_eq!(paid_for(&s, STOCKPILE), vec![1]);
                s.expect_mana(PlayerId::P1, 3);
            }

            #[test]
            fn only_the_next_spell_the_first_spell_consumes_it_and_the_second_pays_in_full() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, STOCKPILE, STOCKPILE, ANCHOR], "library": [MENACE, MENACE, MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));
                let (first, second) = {
                    let hand = s.hand(PlayerId::P1);
                    match (hand.get(1), hand.get(2)) {
                        (Some(first), Some(second)) => (first.id.clone(), second.id.clone()),
                        _ => panic!("two Stockpiles in hand"),
                    }
                };

                s.play(PREP, json!({}));
                s.play(&first, json!({}));
                s.play(&second, json!({}));

                assert_eq!(paid_for(&s, STOCKPILE), vec![0, 1]);
                s.expect_mana(PlayerId::P1, 3);
            }

            #[test]
            fn a_unit_play_pays_in_full_and_does_not_consume_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, VANILLA, STOCKPILE, ANCHOR], "library": [MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(VANILLA, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, VANILLA), vec![1]);
                assert_eq!(paid_for(&s, STOCKPILE), vec![0]);
                s.expect_mana(PlayerId::P1, 3);
            }

            #[test]
            fn a_trap_play_pays_in_full_and_does_not_consume_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, SHEEPISH, STOCKPILE, ANCHOR], "library": [MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(SHEEPISH, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, SHEEPISH), vec![1]);
                assert_eq!(paid_for(&s, STOCKPILE), vec![0]);
            }

            #[test]
            fn r70_a_field_spell_play_does_not_consume_it_nor_does_the_spell_its_draw_casts_the_next_spell_played_is_still_cheaper() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, ARMOR, STOCKPILE, ANCHOR], "library": [VIRUS, MENACE, MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(ARMOR, json!({}));

                assert_eq!(paid_for(&s, ARMOR), vec![2]);
                // The Field Spell's Cry drew the CN-Virus, which cast itself for free (R70) and drew again.
                assert_eq!(paid_for(&s, VIRUS), vec![0]);
                s.expect_health(PlayerId::P1, 29);

                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, STOCKPILE), vec![0]);
                s.expect_mana(PlayerId::P1, 2);
            }

            #[test]
            fn s2_2_it_expires_at_cleanup_a_spell_on_your_next_turn_pays_in_full() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, STOCKPILE, ANCHOR], "library": [MENACE, MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR], "library": [MENACE, MENACE] },
                }));

                s.play(PREP, json!({}));
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().active, PlayerId::P1);
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, STOCKPILE), vec![1]);
            }

            #[test]
            fn r386_an_upgrade_makes_it_3_less_a_degrade_1_less_a_4_flood_costs_1_then_3() {
                crate::register_all();
                let mut up = scenario(json!({ "p1": { "hand": [PREP, FLOOD, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));
                step_param(up.card_mut(PREP), "discount", 1);
                up.play(PREP, json!({}));
                up.play(FLOOD, json!({}));
                assert_eq!(paid_for(&up, FLOOD), vec![1]);

                let mut down = scenario(json!({ "p1": { "hand": [PREP, FLOOD, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));
                step_param(down.card_mut(PREP), "discount", -1);
                down.play(PREP, json!({}));
                down.play(FLOOD, json!({}));
                assert_eq!(paid_for(&down, FLOOD), vec![3]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_next_spell_costs_4_less_a_4_flood_is_free() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": PREP, "radiant": true }, FLOOD, ANCHOR] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(FLOOD, json!({}));

                assert_eq!(paid_for(&s, FLOOD), vec![0]);
                s.expect_mana(PlayerId::P1, 4);
            }

            #[test]
            fn still_only_the_next_spell_and_still_not_a_unit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": PREP, "radiant": true }, VANILLA, FLOOD, STOCKPILE, ANCHOR],
                        "library": [MENACE, MENACE],
                        "mana": 10,
                    },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(PREP, json!({}));
                s.play(VANILLA, json!({}));
                s.play(FLOOD, json!({}));
                s.play(STOCKPILE, json!({}));

                assert_eq!(paid_for(&s, VANILLA), vec![1]);
                assert_eq!(paid_for(&s, FLOOD), vec![0]);
                assert_eq!(paid_for(&s, STOCKPILE), vec![1]);
            }

            #[test]
            fn r386_a_degrade_on_the_radiant_face_steps_4_to_3_a_4_flood_costs_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": PREP, "radiant": true }, FLOOD, ANCHOR] },
                    "p2": { "hand": [ANCHOR] },
                }));
                step_param(s.card_mut(PREP), "discount", -1);

                s.play(PREP, json!({}));
                s.play(FLOOD, json!({}));

                assert_eq!(paid_for(&s, FLOOD), vec![1]);
            }
        }
    }

    /// Issue #2179's Embiggened version: the discount, and the highest cost Spell from the deck (R24).
    mod c_35_prep_embiggened {
        use super::*;

        fn drawn(s: &Scenario) -> Vec<String> {
            s.events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Drawn { player: PlayerId::P1, instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect()
        }

        #[test]
        fn r81_r24_embiggened_it_draws_the_highest_cost_spell_and_the_discount_still_applies() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [PREP, ANCHOR], "library": [VANILLA, STOCKPILE, MENACE, FLOOD, ARMOR], "mana": 6 },
                "p2": { "hand": [ANCHOR] },
            }));
            let flood = s.card(FLOOD).id.clone();

            s.play(PREP, json!({ "embiggen": true }));
            assert_eq!(drawn(&s), vec![flood.clone()]);
            s.expect_in_zone(flood.as_str(), "hand");
            // The (4) Flood is the next Spell: it costs (2) less.
            s.play(flood.as_str(), json!({}));
            assert_eq!(paid_for(&s, FLOOD), vec![2]);
        }

        #[test]
        fn r24_a_tie_goes_to_the_spell_nearest_the_top() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [PREP, ANCHOR], "library": [STOCKPILE, FLOOD, FLOOD], "mana": 6 },
                "p2": { "hand": [ANCHOR] },
            }));
            let top = s.pile(PlayerId::P1, "library")[1].id.clone();
            s.play(PREP, json!({ "embiggen": true }));
            assert_eq!(drawn(&s), vec![top]);
        }

        #[test]
        fn a_deck_with_no_spell_draws_nothing_and_the_plain_version_never_draws() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [PREP, ANCHOR], "library": [VANILLA, MENACE, ARMOR], "mana": 6 },
                "p2": { "hand": [ANCHOR] },
            }));
            s.play(PREP, json!({ "embiggen": true }));
            assert!(drawn(&s).is_empty());

            let mut plain = scenario(json!({
                "p1": { "hand": [PREP, ANCHOR], "library": [FLOOD, STOCKPILE], "mana": 6 },
                "p2": { "hand": [ANCHOR] },
            }));
            plain.play(PREP, json!({}));
            assert!(drawn(&plain).is_empty());
        }

        #[test]
        fn r81_radiant_embiggened_it_draws_the_spell_which_costs_4_less() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": PREP, "radiant": true }, ANCHOR], "library": [STOCKPILE, FLOOD], "mana": 6 },
                "p2": { "hand": [ANCHOR] },
            }));
            let flood = s.card(FLOOD).id.clone();
            s.play(PREP, json!({ "embiggen": true }));
            assert_eq!(drawn(&s), vec![flood.clone()]);
            s.play(flood.as_str(), json!({}));
            assert_eq!(paid_for(&s, FLOOD), vec![0]);
        }

        /// R767000: the script kept for the replays of v0.4.12d and before, which drew nothing.
        #[test]
        fn r767000_a_game_on_v0_4_12d_draws_nothing() {
            crate::under_kept(PREP, crate::scripts::classic::c035_prep::script_v0_4_12d(), || {
                let mut s = scenario(json!({
                    "p1": { "hand": [PREP, ANCHOR], "library": [STOCKPILE, FLOOD], "mana": 6 },
                    "p2": { "hand": [ANCHOR] },
                }));
                s.play(PREP, json!({ "embiggen": true }));
                assert!(drawn(&s).is_empty());
            });
        }
    }
}
