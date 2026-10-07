//! C #43 Plague Nuke (SPEC §8.6 row 43). (3) Spell, Epic.
//!   Base:    "Destroy all Units. Gain {mana} mana for each Plague Counter that was on them." — 1
//!   Radiant: "Destroy all Units. Gain {mana} mana for each Plague Counter that was on them. Then summon,
//!            under your control, each of those Units that had a Plague Counter from its owner's
//!            graveyard." — 1
//!   Engine:  "Count the tokens on every unit first, destroy all (one state check, §4.5), then the
//!            temporary mana. An Indestructible unit survives, but its tokens count ("on them" is every
//!            Unit the Spell hit). Radiant: after that check, each non-token unit card that had a token
//!            and is now in a graveyard is summoned to your side (control yours, owner unchanged, §3.2;
//!            no Cry, R1), into your leftmost open zones in lane order (R64); a Reborn unit already
//!            back on the field is not summoned again; tokens are gone (R11). Tunes: mana per token 1 ↑."
//!
//! "Them" is every Unit on the field as the Spell resolves — the top of each unit pile on both sides,
//! never a card dormant under a Stack (R13) — read once, first: the Plague Counters on them all, and the
//! ones that carry any. Then every Unit is destroyed, and the deaths happen in ONE §4.5 check
//! (`afterStateCheck` runs it at this point of the list, R59), so an Indestructible unit survives with
//! its tokens counted all the same. After that check comes the temporary mana (§2.3): {mana} per token.
//!
//! Radiant, after the same check: each of those Units that had a token, is not a token (R11: a token is
//! gone) and now lies in a graveyard — not one Reborn already put back on the field, not one exiled
//! instead of dying — is summoned for the caster (§6.3 Summon: no Cry, R1; its owner unchanged, §3.2),
//! in the order they stood on the board (R68: the active side first, lane order), each into the
//! caster's leftmost open zone (R64); a full row leaves the rest where they are. The rest of the text
//! runs on a stay that begins after the check (R174), which is what lets it name a card in its
//! graveyard.
//!
//! R280: the preview is the mana it would give now — the Plague Counters on the Units on the field,
//! which are public (§10.8), times {mana} — read by the same count the resolution uses.
//!
//! The number is the declared `mana` (R386), read through `param`.

use jackioh_engine::prelude::*;
use jackioh_engine::effects::{after_state_check, destroy_all, gain_mana, summon};
use std::sync::Arc;

pub const ID: &str = "classic-043";

/// R280: the words of the text the preview's value follows, on both faces.
const MANA_LABEL: &str = "for each Plague Counter that was on them";

/// Every Unit on the field, R68's order from `first`'s side: the tops of the unit piles (R13).
fn units_on_field(state: &GameState, first: PlayerId) -> Vec<CardInstance> {
    let mut units: Vec<CardInstance> = active_units_of(state, first).into_iter().map(|unit| unit.clone()).collect();
    units.extend(active_units_of(state, opponent_of(first)).into_iter().map(|unit| unit.clone()));
    units
}

/// The Plague Counters on every Unit on the field now: what the mana counts.
fn tokens_on_units(state: &GameState, first: PlayerId) -> i32 {
    units_on_field(state, first).iter().fold(0, |sum, unit| sum + plague_on(unit))
}

fn plague_nuke(resummons: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // Read once, before anything dies: the tokens, and the non-token Units that carry any.
            let first = ctx.state.active;
            let mana = tokens_on_units(&ctx.state, first) * param(ctx, "mana");
            let plagued: Vec<String> = units_on_field(&ctx.state, first)
                .into_iter()
                .filter(|unit| plague_on(unit) > 0 && !def_of(&ctx.state, &unit.def_id).token)
                .map(|unit| unit.id)
                .collect();
            vec![
                destroy_all(json_as(json!({ "side": "any" }))),
                after_state_check(hook(move |after| {
                    let mut effects = vec![gain_mana(json_as(json!({ "amount": mana })))];
                    if resummons {
                        effects.extend(
                            plagued
                                .iter()
                                .filter(|id| {
                                    find_instance(&after.state, id.as_str())
                                        .is_some_and(|card| matches!(card.zone, Zone::Graveyard { .. }))
                                })
                                .map(|instance_id| {
                                    summon(json_as(json!({
                                        "instance": { "of": "instance", "instanceId": instance_id },
                                        "player": "self",
                                    })))
                                }),
                        );
                    }
                    effects
                })),
            ]
        })),
        // R280: the mana it would give now.
        preview: Some(Arc::new(|ctx: &ConditionContext| -> Vec<PreviewValue> {
            vec![json_as(json!({
                "label": MANA_LABEL,
                "value": tokens_on_units(&ctx.state, ctx.controller) * param(ctx, "mana"),
            }))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = plague_nuke(false);

    let radiant = plague_nuke(true);

    CardScripts { base, radiant }
}

// C #43 Plague Nuke — SPEC §8.6 row 43, BUILD M9 Classic row C 43: "Counts the Plague Counters on every
// Unit first, then destroys all Units in one state check (§4.5), then gives 1 mana this turn per token
// counted, an Indestructible survivor's tokens included; its preview is that mana (R280); radiant:
// after that check, each non-token Unit card that had a token and now lies in a graveyard is summoned
// to your side under your control, its owner unchanged, into your leftmost open zones in lane order,
// without a Cry; a Reborn Unit already back is not summoned again; tokens are gone (R11); a full board
// leaves the rest; a Unit exiled instead of dying into a graveyard (C #50) is not summoned; its tuned
// number (mana per token) reads through `param()` (R386)".
//
// The preview's proofs (R280) are in `../preview.test.ts`, with the other cards'. The C #50 cases use
// C #50 Voidwalker's real script, whose Aura exiles what would go to a graveyard while it is on the
// field; a Voidwalker the Nuke kills leaves the field with the others and exiles none of them (R398, R463).
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const NUKE: &str = "classic-043";
    const STATE: &str = "classic-041"; // State of the Game: 3/3 Indestructible.
    const VOIDWALKER: &str = "classic-050"; // Aura: cards that would go to a graveyard are exiled instead.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const FELINORS: &str = "core-012"; // Cry: summon a copy of this.
    const DEFENDER: &str = "core-003"; // Right-house defender: Taunt, Divine Shield, Reborn.
    const RUSH_TOKEN: &str = "core-t-rush";
    const BIG_FELINOR: &str = "core-043"; // (4) Unit 3/10, under the Stack pile below.
    const FIENDER: &str = "core-092"; // Felinor Fiender: Stack.
    const ANCHOR: &str = "core-010";

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// TS `plagued(defId, n)` (no caller passes the TS helper's optional `extra`).
    fn plagued(def_id: &str, n: i32) -> Value {
        json!({ "def": def_id, "counters": { "plague": n } })
    }

    fn mana_gained(s: &Scenario) -> i32 {
        // The cast paid (4) from 4; anything above 0 is the Spell's gain (read off p1's own view, §10.8).
        s.view(PlayerId::P1).you.mana.current
    }

    /// Each lane's def id, `null` for an empty lane, as the JSON array the TS compares.
    fn unit_defs(s: &Scenario, player: PlayerId) -> Value {
        Value::Array(
            [1, 2, 3, 4, 5]
                .into_iter()
                .map(|lane| s.unit(player, lane).map_or(Value::Null, |unit| json!(unit.def_id)))
                .collect(),
        )
    }

    mod c_43_plague_nuke {
        use super::*;

        #[test]
        fn has_a_script_per_face_each_with_a_preview() {
            crate::register_all();
            assert_eq!(ID, NUKE);
            let scripts = script();
            assert!(scripts.base.preview.is_some());
            assert!(scripts.radiant.preview.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn destroys_all_units_on_both_sides_then_gains_1_mana_per_plague_counter_that_was_on_them() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [NUKE, ANCHOR], "field": [plagued(VANILLA, 2)] },
                    "p2": { "hand": [ANCHOR], "field": [plagued(MENACE, 3), VANILLA] },
                }));

                s.play(NUKE, json!({}));

                assert_eq!(unit_defs(&s, PlayerId::P1), json!([null, null, null, null, null]));
                assert_eq!(unit_defs(&s, PlayerId::P2), json!([null, null, null, null, null]));
                assert_eq!(mana_gained(&s), 5);
            }

            #[test]
            fn c4_5_one_state_check_every_death_comes_first_and_the_mana_after_them() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [NUKE, ANCHOR], "field": [plagued(VANILLA, 1)] },
                    "p2": { "hand": [ANCHOR], "field": [plagued(MENACE, 1), VANILLA] },
                }));

                s.play(NUKE, json!({}));

                let kinds: Vec<String> =
                    s.last_events().iter().map(|event| js(event)["type"].as_str().unwrap_or("").to_string()).collect();
                let destroyed: Vec<usize> =
                    kinds.iter().enumerate().filter_map(|(at, kind)| if kind == "destroyed" { Some(at) } else { None }).collect();
                assert_eq!(destroyed.len(), 3);
                // TS `lastIndexOf`: none is -1, which no index is less than.
                let gain = kinds.iter().rposition(|kind| kind == "manaChanged");
                for at in destroyed {
                    assert!(gain.is_some_and(|gain| at < gain));
                }
            }

            #[test]
            fn no_tokens_on_the_board_it_destroys_all_and_gives_nothing() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [NUKE, ANCHOR], "field": [VANILLA] }, "p2": { "hand": [ANCHOR], "field": [MENACE] } }));

                s.play(NUKE, json!({}));

                assert_eq!(mana_gained(&s), 0);
                s.expect_in_zone(VANILLA, "graveyard");
                s.expect_in_zone(MENACE, "graveyard");
            }

            #[test]
            fn r46_an_indestructible_unit_survives_and_its_tokens_count_all_the_same() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [NUKE, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [plagued(STATE, 2), plagued(VANILLA, 1)] },
                }));

                s.play(NUKE, json!({}));

                s.expect_in_zone(STATE, "field");
                s.expect_in_zone(VANILLA, "graveyard");
                assert_eq!(mana_gained(&s), 3);
            }

            #[test]
            fn r13_a_card_dormant_under_a_stack_pile_is_not_a_unit_on_the_field_its_tokens_dont_count_and_it_resumes_and_survives() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [NUKE, ANCHOR] },
                    "p2": {
                        "hand": [ANCHOR],
                        "field": [plagued(BIG_FELINOR, 2), { "def": FIENDER, "stack": true, "counters": { "plague": 1 } }],
                    },
                }));

                s.play(NUKE, json!({}));

                s.expect_in_zone(FIENDER, "graveyard");
                assert_eq!(s.unit(PlayerId::P2, 1).map(|unit| unit.def_id.clone()), Some(BIG_FELINOR.to_string()));
                assert_eq!(mana_gained(&s), 1);
            }

            #[test]
            fn the_base_face_summons_nothing_back() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [NUKE, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": [plagued(VANILLA, 1)] } }));

                s.play(NUKE, json!({}));

                s.expect_in_zone(VANILLA, "graveyard");
                assert_eq!(unit_defs(&s, PlayerId::P1), json!([null, null, null, null, null]));
            }

            #[test]
            fn r386_an_upgrade_gives_2_mana_per_token() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [NUKE, ANCHOR] }, "p2": { "hand": [ANCHOR], "field": [plagued(VANILLA, 2)] } }));
                step_param(s.card_mut(NUKE), "mana", 1);

                s.play(NUKE, json!({}));

                assert_eq!(mana_gained(&s), 4);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn summons_each_unit_that_had_a_token_from_its_owners_graveyard_under_your_control_owner_unchanged_in_lane_order() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": NUKE, "radiant": true }, ANCHOR], "field": [plagued(VANILLA, 1)] },
                    "p2": { "hand": [ANCHOR], "field": [MENACE, plagued(MENACE, 2)] },
                }));
                let mine = s.card(VANILLA).id.clone();
                let theirs = s.unit(PlayerId::P2, 2).expect("a plagued Menace in lane 2").id.clone();

                s.play(NUKE, json!({}));

                assert_eq!(mana_gained(&s), 3);
                assert_eq!(unit_defs(&s, PlayerId::P1), json!([VANILLA, MENACE, null, null, null]));
                assert_eq!(s.unit(PlayerId::P1, 1).map(|unit| unit.id.clone()), Some(mine.clone()));
                assert_eq!(s.unit(PlayerId::P1, 2).map(|unit| unit.id.clone()), Some(theirs.clone()));
                assert_eq!(s.card(&theirs).owner, PlayerId::P2);
                assert_eq!(s.card(&theirs).controller, PlayerId::P1);
                // The Menace with no token stays in its owner's graveyard.
                assert_eq!(
                    s.pile(PlayerId::P2, "graveyard").iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
                    vec![MENACE],
                );
            }

            #[test]
            fn r1_r78_a_summoned_unit_returns_reset_and_fires_no_cry_a_duplicating_felinors_makes_no_copy_and_its_tokens_are_gone() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": NUKE, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [plagued(FELINORS, 1)] },
                }));

                s.play(NUKE, json!({}));

                assert_eq!(unit_defs(&s, PlayerId::P1), json!([FELINORS, null, null, null, null]));
                assert_eq!(s.unit(PlayerId::P1, 1).and_then(|unit| unit.counters.plague).unwrap_or(0), 0);
            }

            #[test]
            fn r64_r83_a_reborn_unit_the_check_already_put_back_is_not_summoned_again() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": NUKE, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [plagued(DEFENDER, 1)] },
                }));
                let defender = s.card(DEFENDER).id.clone();

                s.play(NUKE, json!({}));

                assert_eq!(s.card(&defender).controller, PlayerId::P2);
                assert_eq!(s.unit(PlayerId::P2, 1).map(|unit| unit.id.clone()), Some(defender.clone()));
                assert_eq!(unit_defs(&s, PlayerId::P1), json!([null, null, null, null, null]));
            }

            #[test]
            fn r11_a_token_that_had_a_token_is_gone_not_summoned() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": NUKE, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [plagued(RUSH_TOKEN, 2)] },
                }));
                let token = s.unit(PlayerId::P2, 1).expect("a token").id.clone();

                s.play(NUKE, json!({}));

                s.expect_in_zone(&token, "gone");
                assert_eq!(unit_defs(&s, PlayerId::P1), json!([null, null, null, null, null]));
                assert_eq!(mana_gained(&s), 2);
            }

            #[test]
            fn an_indestructible_survivor_with_tokens_stays_where_it_is_with_its_controller() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": NUKE, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [plagued(STATE, 1)] },
                }));

                s.play(NUKE, json!({}));

                assert_eq!(s.card(STATE).controller, PlayerId::P2);
                assert_eq!(unit_defs(&s, PlayerId::P1), json!([null, null, null, null, null]));
            }

            #[test]
            fn r13_only_the_piles_top_was_hit_the_stack_top_that_had_a_token_is_summoned_and_the_card_beneath_stays_with_its_owner() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": NUKE, "radiant": true }, ANCHOR] },
                    "p2": {
                        "hand": [ANCHOR],
                        "field": [plagued(BIG_FELINOR, 2), { "def": FIENDER, "stack": true, "counters": { "plague": 1 } }],
                    },
                }));

                s.play(NUKE, json!({}));

                assert_eq!(unit_defs(&s, PlayerId::P1), json!([FIENDER, null, null, null, null]));
                assert_eq!(s.card(FIENDER).owner, PlayerId::P2);
                assert_eq!(unit_defs(&s, PlayerId::P2), json!([BIG_FELINOR, null, null, null, null]));
            }

            #[test]
            fn a_full_board_leaves_the_rest_in_the_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": NUKE, "radiant": true }, ANCHOR], "field": [STATE, STATE, STATE, STATE] },
                    "p2": { "hand": [ANCHOR], "field": [plagued(VANILLA, 1), plagued(MENACE, 1)] },
                }));

                s.play(NUKE, json!({}));

                assert_eq!(unit_defs(&s, PlayerId::P1), json!([STATE, STATE, STATE, STATE, VANILLA]));
                assert_eq!(
                    s.pile(PlayerId::P2, "graveyard").iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
                    vec![MENACE],
                );
            }

            #[test]
            fn c_50_a_unit_exiled_instead_of_dying_into_a_graveyard_is_not_summoned() {
                crate::register_all();
                // C #50 Voidwalker's base Aura: "Cards that would go to a graveyard are exiled instead", live while
                // it is on the field (SPEC §8.6 row 50). An Indestructible Voidwalker survives the Nuke (R46), so
                // the plagued Vanilla dying beside it goes to exile instead (R461: it has not died).
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": NUKE, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [plagued(VANILLA, 1), VOIDWALKER] },
                }));
                s.card_mut(VOIDWALKER).granted_keywords.push(json_as(json!({ "kind": "Indestructible" })));

                s.play(NUKE, json!({}));

                assert!(s.pile(PlayerId::P2, "exile").iter().any(|card| card.def_id == VANILLA));
                assert!(s.pile(PlayerId::P2, "graveyard").is_empty());
                assert_eq!(unit_defs(&s, PlayerId::P1), json!([null, null, null, null, null]));
                assert_eq!(unit_defs(&s, PlayerId::P2), json!([null, VOIDWALKER, null, null, null]));
            }

            #[test]
            fn r398_r463_a_voidwalker_the_nuke_kills_leaves_with_the_others_and_exiles_none_the_unit_is_summoned() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": NUKE, "radiant": true }, ANCHOR] },
                    "p2": { "hand": [ANCHOR], "field": [plagued(VANILLA, 1), VOIDWALKER] },
                }));

                s.play(NUKE, json!({}));

                assert!(s.pile(PlayerId::P2, "exile").is_empty());
                assert_eq!(
                    s.pile(PlayerId::P2, "graveyard").iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
                    vec![VOIDWALKER],
                );
                assert_eq!(unit_defs(&s, PlayerId::P1), json!([VANILLA, null, null, null, null]));
                assert_eq!(s.card(VANILLA).owner, PlayerId::P2);
            }
        }
    }
}
