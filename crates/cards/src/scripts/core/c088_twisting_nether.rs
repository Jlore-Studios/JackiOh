//! #88 Twisting Nether (SPEC §8.5, §3.1, §4.5, R13, R46, R59, R68, R69, R81).
//!
//! Base: "Destroy all permanents". Radiant: "Choose: all enemy permanents, or all".
//! Engine cell: "Indestructibles survive; backrow included."
//!
//! §8's Conventions: the radiant cell restates the whole clause as a modal one, so the radiant face
//! is the same board-wide destroy with a side chosen at play time.
//!
//! The mode is a DECLARED play choice, not a prompt. R81's card list names #88, and §10.6 is
//! explicit: a card's own declared modes "travel in the `play` action" and are "not prompts";
//! `playChoices.ts` enumerates them for `legalActions` and refuses a play that answers none. So the
//! declaration below and `chosenOptions(ctx)` are the whole of the radiant choice — and they work
//! today, which is why they are written out rather than left for later.
//!
//! `destroyAll({ side, rows })` is the board-wide destroy (`effects/destroy.ts`), written in the
//! shared `BoardScope` of `effects/targets.ts`. It MARKS and never moves — `markedDestroyed` on
//! every card the scope matches, walked in R68's order — so §4.5 step 1 collects the whole board at
//! once and R59's single state check does the rest. Only cards ON the field are matched, so a card
//! dormant under a Stack pile is not (R13).
//!
//! INDESTRUCTIBLE IS NOT THIS CARD'S BUSINESS, and the scope does NOT pre-exclude it. §4.5's
//! `resolveIndestructibleMarks` drops the mark on an Indestructible unit, switches it to Attack
//! Position and suppresses its Taunt for the turn (R46) — effects that only happen if the mark was
//! actually applied. An Indestructible unit whose max health is already 0 or less dies anyway,
//! because no destroy effect is involved (R69). So "Indestructibles survive" falls out of the state
//! check, and asking the scope to skip them would quietly lose R46.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-088";

/// The radiant face's choice, in the order §8's cell lists it: enemy only, or everything.
const MODE_ENEMY: &str = "enemy";
const MODE_ALL: &str = "all";

/// TS `modes: ModeDecl[] = [{ kind: "mode", options: [MODE_ENEMY, MODE_ALL] }]`.
fn modes() -> Vec<ModeDecl> {
    vec![ModeDecl {
        kind: PromptKind::Mode,
        options: vec![MODE_ENEMY.to_string(), MODE_ALL.to_string()],
    }]
}

/// Which side the destroy reaches. The base face has no choice to read and always hits both sides;
/// the radiant face hits what the play named, and a play carrying no mode fizzles rather than
/// guessing — the spell still counts as played (§6.3, §8 Conventions).
///
/// `"any" | "enemy" | null`: the `BoardScope` side, as the literal the scope takes.
fn side_for(mode_name: Option<&str>, modal: bool) -> Option<&'static str> {
    if !modal {
        return Some("any");
    }
    if mode_name == Some(MODE_ENEMY) {
        return Some("enemy");
    }
    if mode_name == Some(MODE_ALL) {
        return Some("any");
    }
    None
}

/// `modal` is the whole of the radiant text.
fn nether(modal: bool) -> Script {
    Script {
        modes: if modal { modes() } else { vec![] },
        cry: Some(hook(move |ctx| {
            let chosen = chosen_options(ctx);
            let Some(side) = side_for(chosen.first().map(String::as_str), modal) else {
                return vec![];
            };
            // §6.3: a permanent is a Unit, Field Spell, Trap or Field Trap, so both rows.
            vec![destroy_all(json_as(json!({ "side": side, "rows": ["units", "backrow"] })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = nether(false);
    let radiant = nether(true);
    CardScripts { base, radiant }
}

// #88 Twisting Nether (SPEC §8.5, BUILD M4-T4 row 88): "Every permanent on both rows destroyed,
// Indestructibles survive; radiant enemy-only mode".
//
// These six cases were held as `it.todo` with their bodies intact while the board-wide destroy was
// missing from the effects barrel. `destroyAll({ side, rows })` has since landed in
// `engine/src/effects/destroy.ts` and is re-exported from the barrel, so every case below is live:
// the assertions are unchanged from the ones written as the acceptance for that verb.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const NETHER: &str = "core-088";

    // Inert fixtures: each unit below has a Cry and nothing else, and the harness's `field` setup never
    // fires a Cry. Mana Well and Sheepish have no Indestructible, so both rows are destructible.
    const GARY: &str = "core-004"; // 1/1
    const RENO: &str = "core-053"; // 4/6
    const ROCK: &str = "core-066"; // 10/10, printed Indestructible
    const MANA_WELL: &str = "core-006"; // Field Spell
    const SHEEPISH: &str = "core-041"; // Trap; watches for a Unit the opponent plays, so a Spell is safe

    // §2.5: one always-playable card per hand keeps a scenario on the turn it started on.
    const FILLER: &str = "core-005";

    const SEED: &str = "nether-88";

    /// An engine value as the JSON the TS test compares it with.
    fn js<T: serde::Serialize>(v: &T) -> Value {
        serde_json::to_value(v).expect("serialises")
    }

    fn destroyed_in(events: &[GameEvent]) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Destroyed { .. }))
            .count()
    }

    mod n88_twisting_nether_declared_play_choices_r81_s10_6 {
        use super::*;

        #[test]
        fn r81_the_base_face_has_no_choice_to_make() {
            crate::register_all();
            assert!(script().base.modes.is_empty());
        }

        #[test]
        fn r81_the_radiant_face_declares_the_side_as_a_play_time_mode_not_a_prompt() {
            crate::register_all();
            assert_eq!(
                js(&script().radiant.modes),
                json!([{ "kind": "mode", "options": ["enemy", "all"] }])
            );
        }
    }

    mod n88_twisting_nether_base {
        use super::*;

        #[test]
        fn destroys_every_permanent_on_both_rows_the_backrow_included() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": {
                    "hand": [NETHER, FILLER],
                    "field": [{ "def": GARY, "lane": 1 }],
                    "backrow": [{ "def": MANA_WELL, "lane": 1 }],
                },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": RENO, "lane": 3 }],
                    "backrow": [{ "def": SHEEPISH, "lane": 2 }],
                },
            }));
            let gary = s.card(GARY).clone();
            let reno = s.card(RENO).clone();
            let well = s.card(MANA_WELL).clone();
            let trap = s.card(SHEEPISH).clone();

            s.play(NETHER, json!({}));

            s.expect_in_zone(&gary, "graveyard")
                .expect_in_zone(&reno, "graveyard")
                .expect_in_zone(&well, "graveyard")
                .expect_in_zone(&trap, "graveyard");
            assert!(s.unit(P1, 1).is_none());
            assert!(s.unit(P2, 3).is_none());
            assert!(s.backrow(P1, 1).is_none());
            assert!(s.backrow(P2, 2).is_none());

            // R12: each card went to ITS OWN owner's graveyard.
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.id == gary.id));
            assert!(s.pile(P2, "graveyard").iter().any(|card| card.id == reno.id));
        }

        #[test]
        fn r59_every_permanent_dies_together_in_one_state_check() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [NETHER, FILLER], "field": [{ "def": GARY, "lane": 1 }, { "def": RENO, "lane": 2 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));
            let before = s.state().counters.destroyed;

            s.play(NETHER, json!({}));

            assert_eq!(s.state().counters.destroyed, before + 3);
            assert_eq!(destroyed_in(s.last_events()), 3);
        }

        #[test]
        fn r46_an_indestructible_unit_survives_switches_to_atk_and_loses_taunt() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": {
                    "hand": [NETHER, FILLER],
                    "field": [
                        { "def": ROCK, "lane": 1, "position": "DEF" },
                        { "def": GARY, "lane": 2 },
                    ],
                },
                "p2": { "hand": [FILLER] },
            }));
            let rock = s.card(ROCK).clone();
            let gary = s.card(GARY).clone();

            s.play(NETHER, json!({}));

            // R46: the destroy mark is dropped, the unit stays, and Defense Position is given up — which
            // is why the filter must mark Indestructibles rather than skipping them.
            s.expect_in_zone(&rock, "field");
            assert_eq!(s.unit(P1, 1).map(|card| card.id.clone()), Some(rock.id.clone()));
            assert_eq!(js(&s.stats(&rock).position), json!("ATK"));
            assert!(!s
                .stats(&rock)
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Taunt));

            // Everything destructible still died.
            s.expect_in_zone(&gary, "graveyard");
        }

        #[test]
        fn destroys_nothing_and_still_counts_as_played_when_both_boards_are_empty() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": SEED, "p1": { "hand": [NETHER, FILLER] }, "p2": { "hand": [FILLER] } }));

            s.play(NETHER, json!({}));

            s.expect_in_zone(NETHER, "graveyard")
                .expect_events(json!(["cardPlayed"]));
            assert_eq!(destroyed_in(s.last_events()), 0);
        }
    }

    mod n88_twisting_nether_radiant {
        use super::*;

        #[test]
        fn enemy_only_the_opponent_s_permanents_are_destroyed_both_rows() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": {
                    "hand": [{ "def": NETHER, "radiant": true }, FILLER],
                    "field": [{ "def": GARY, "lane": 1 }],
                    "backrow": [{ "def": MANA_WELL, "lane": 1 }],
                },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": RENO, "lane": 1 }],
                    "backrow": [{ "def": SHEEPISH, "lane": 1 }],
                },
            }));
            let mine = s.card(GARY).clone();
            let my_well = s.card(MANA_WELL).clone();
            let theirs = s.card(RENO).clone();
            let their_trap = s.card(SHEEPISH).clone();

            s.play(NETHER, json!({ "modes": ["enemy"] }));

            s.expect_in_zone(&theirs, "graveyard")
                .expect_in_zone(&their_trap, "graveyard");
            s.expect_in_zone(&mine, "field").expect_in_zone(&my_well, "field");
            assert_eq!(s.unit(P1, 1).map(|card| card.id.clone()), Some(mine.id.clone()));
            assert_eq!(s.backrow(P1, 1).map(|card| card.id.clone()), Some(my_well.id.clone()));
        }

        #[test]
        fn all_the_radiant_face_can_still_sweep_both_sides() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": NETHER, "radiant": true }, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": RENO, "lane": 1 }] },
            }));
            let mine = s.card(GARY).clone();
            let theirs = s.card(RENO).clone();

            s.play(NETHER, json!({ "modes": ["all"] }));

            s.expect_in_zone(&mine, "graveyard").expect_in_zone(&theirs, "graveyard");
        }

        #[test]
        fn r46_indestructibles_survive_the_enemy_only_mode_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": NETHER, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": ROCK, "lane": 1 }, { "def": GARY, "lane": 2 }] },
            }));
            let rock = s.card(ROCK).clone();
            let gary = s.card(GARY).clone();

            s.play(NETHER, json!({ "modes": ["enemy"] }));

            s.expect_in_zone(&rock, "field").expect_in_zone(&gary, "graveyard");
        }
    }
}
