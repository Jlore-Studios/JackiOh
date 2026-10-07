//! C+ #44 Simplicity Audit (SPEC §8.7 row 44, E36, R280, R583). (2) Spell, Rare.
//!   Exile every permanent whose card has fewer lines of code than this one. Radiant: choose all
//!   permanents or only your opponent's (a mode declared at play, R81) — and "highlight targets".
//!
//! Each permanent's `loc` (§5: its script file's non-blank, non-comment lines, imports excluded; a fused
//! card's is its ingredients' sum) against this card's own, read off the running definition, so this
//! file's own length is the card's balance (R388). The permanents are the tops of piles and the backrow
//! cards, face-down ones included (`subsystems.auditTargets`); an equal `loc` stays. Exile fires no Death
//! and a token ceases to exist (§6.3, R11). The Radiant face's preview (R280, R583) is, for each choice,
//! the permanents it would exile now — never one its controller may not read (an enemy face-down card,
//! R177), which the exile still takes; the base face marks nothing.


use jackioh_engine::effects::{ForEachCardArgs, chosen_options, exile, for_each_card, unreadable_by};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-044";

const ALL: &str = "All permanents";
const THEIRS: &str = "Only your opponent's";

/// The permanents whose card has fewer lines of code than `defId`'s.
fn targets(
    state: &GameState,
    controller: PlayerId,
    active: PlayerId,
    def_id: &str,
    enemy_only: bool,
) -> Vec<CardInstance> {
    let loc = subsystems::lines_of_code(state, def_id);
    subsystems::audit_targets(state, subsystems::AuditArgs { controller, active, loc, more: false, enemy_only })
}

/// Which permanents a run of the Audit exiles: all of them, or only the opponent's.
type EnemyOnly = fn(&EffectContext<'_>) -> bool;

/// The `forEachCard` list of one run: the permanents below the running definition's `loc`, by id.
fn run_targets(run: &EffectContext<'_>, enemy_only: EnemyOnly) -> Vec<String> {
    let def_id = run
        .self_
        .as_ref()
        .map(|card| card.def_id.clone())
        .or_else(|| run.def_id.clone())
        .unwrap_or_else(|| ID.to_string());
    let enemy_only = enemy_only(run);
    let state: &GameState = run.state;
    targets(state, run.controller, state.active, &def_id, enemy_only).into_iter().map(|card| card.id).collect()
}

/// The `forEachCard` list, read once as the list reaches it (`run` is the running context).
fn cards_of(enemy_only: EnemyOnly) -> impl Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync + 'static {
    move |run| run_targets(run, enemy_only)
}

/// The `forEachCard` step: exile that permanent.
fn exile_instance(instance_id: &str) -> Effect {
    exile(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
}

fn audit(enemy_only: EnemyOnly) -> Hook {
    hook(move |_ctx| {
        vec![for_each_card(ForEachCardArgs {
            cards: Arc::new(cards_of(enemy_only)),
            each: Arc::new(exile_instance),
        })]
    })
}

fn all_permanents(_ctx: &EffectContext<'_>) -> bool {
    false
}

fn only_theirs(ctx: &EffectContext<'_>) -> bool {
    chosen_options(ctx).first().map(String::as_str) == Some(THEIRS)
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(audit(all_permanents)),
        ..Script::default()
    };
    let radiant = Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![ALL.to_string(), THEIRS.to_string()],
        }],
        cry: Some(audit(only_theirs)),
        preview: Some(condition_hook(|c| {
            [("all permanents", false), ("only your opponent's", true)]
                .into_iter()
                .map(|(label, enemy_only)| {
                    let active = if c.your_turn { c.controller } else { opponent_of(c.controller) };
                    let seen = targets(c.state, c.controller, active, &c.self_.def_id, enemy_only);
                    let ids: Vec<String> = seen
                        .iter()
                        .filter(|card| !unreadable_by(c.state, card).contains(&c.controller))
                        .map(|card| card.id.clone())
                        .collect();
                    PreviewValue {
                        label: label.to_string(),
                        value: ids.len() as i32,
                        display: None,
                        ids: Some(ids),
                    }
                })
                .collect()
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C+ #44 Simplicity Audit — SPEC §8.7 row 44, E36, R11, R13, R177, R280, R583, BUILD M9 row C+ 44.
// The sweep is `subsystems.auditTargets` (packages/engine/test/audit.test.ts); its preview is pinned
// in test/preview.test.ts with R280's other cards. Every `loc` here is read off the catalog, so a
// script edit that moves one fails the guard that names it rather than a silent comparison.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const AUDIT: &str = "classicplus-044";
    const LOW: &str = "core-008"; // Mr. Vanilla
    const HIGH: &str = "core-022"; // Carnivorous Cube
    const LOW_TRAP: &str = "core-071"; // Intern Stimmy, a Field Trap
    const REBORN: &str = "core-003"; // Right-house defender: Taunt, Divine Shield, Reborn
    const RUSH: &str = "core-t-rush";
    const FIENDER: &str = "core-092"; // Felinor Fiender, Stack
    const INDESTRUCTIBLE: &str = "classic-041"; // State of the Game: Indestructible
    const FILLER: &str = "core-005";
    const ALL: &str = "All permanents";
    const THEIRS: &str = "Only your opponent's";

    /// TS `LOC = cardDef(AUDIT).loc ?? 0`.
    fn loc() -> i32 {
        crate::card_def(AUDIT).loc.unwrap_or(0)
    }

    /// Two Units whose `loc` sums to the Audit's: fused, they are a permanent of exactly its `loc`.
    fn equal_pair() -> (String, String) {
        let units: Vec<&CardDef> = crate::CATALOG
            .values()
            .filter(|card| card.type_ == CardType::Unit && !card.token && card.loc.is_some())
            .collect();
        for a in &units {
            let b = units.iter().find(|other| other.id != a.id && a.loc.unwrap_or(0) + other.loc.unwrap_or(0) == loc());
            if let Some(b) = b {
                return (a.id.clone(), b.id.clone());
            }
        }
        panic!("no two Units sum to {} lines", loc());
    }

    use crate::scenario;

    use crate::merged;

    fn audit(radiant: bool, p1: Value, p2: Value) -> Scenario {
        scenario(json!({
            "p1": merged(json!({ "hand": [{ "def": AUDIT, "radiant": radiant }, FILLER] }), p1),
            "p2": merged(json!({ "hand": [FILLER] }), p2),
        }))
    }

    /// Fuse the pair onto p2's unit in `lane`, as R77's kept target: a permanent of the Audit's `loc`.
    fn fuse_equal(s: &mut Scenario, lane: i32) -> CardInstance {
        let Some(kept) = s.unit(P2, lane) else {
            panic!("a unit to fuse onto");
        };
        let (first, second) = equal_pair();
        let other = if s.card(&kept).def_id == first { second } else { first };
        let seed = s.state().seed.clone();
        let cursor = s.state().rng_cursor;
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&seed, cursor);
        let state = s.state_mut();
        let phantom = new_instance(state, &other, P2, Zone::Gone { player: P2 });
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let fused = subsystems::fuse(
            &mut sink,
            subsystems::FuseArgs {
                ingredients: vec![kept.clone(), phantom],
                target: Some(kept),
                ..subsystems::FuseArgs::default()
            },
        );
        match fused {
            Some(card) => card,
            None => panic!("the fusion"),
        }
    }

    fn preview(s: &Scenario) -> Option<Value> {
        let view = serde_json::to_value(s.view(P1)).expect("a view is JSON");
        let audit_id = s.card(AUDIT).id.clone();
        view["you"]["hand"]
            .as_array()
            .and_then(|hand| hand.iter().find(|card| card["instanceId"] == audit_id.as_str()))
            .and_then(|card| card.get("preview").cloned())
    }

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| serde_json::to_value(event).expect("an event is JSON")).collect()
    }

    #[test]
    fn the_catalogs_loc_guards_the_cards_below_are_below_above_and_summed_to_the_audits() {
        assert_eq!(crate::card_def(AUDIT).id, AUDIT);
        assert!(loc() > 0);
        for id in [LOW, LOW_TRAP, REBORN, RUSH, FIENDER, INDESTRUCTIBLE] {
            assert!(crate::card_def(id).loc.unwrap_or(0) < loc());
        }
        assert!(crate::card_def(HIGH).loc.unwrap_or(0) > loc());
    }

    mod base {
        use super::*;

        #[test]
        fn e36_exiles_every_permanent_on_both_sides_whose_card_has_fewer_lines_of_code_a_higher_one_stays() {
            let mut s = audit(false, json!({ "field": [LOW, HIGH] }), json!({ "field": [LOW, HIGH] }));
            let (mine, theirs) = (s.unit(P1, 1).expect("Mr. Vanilla"), s.unit(P2, 1).expect("Mr. Vanilla"));
            s.play(AUDIT, json!({}));
            s.expect_in_zone(&mine, "exile").expect_in_zone(&theirs, "exile");
            assert_eq!(s.card(HIGH).zone.z(), ZoneName::Field);
            assert_eq!(s.unit(P2, 2).map(|card| card.def_id), Some(HIGH.to_string()));
            assert_eq!(s.unit(P1, 2).map(|card| card.def_id), Some(HIGH.to_string()));
        }

        #[test]
        fn e36_an_equal_loc_stays_a_fused_cards_loc_is_its_ingredients_sum_r77() {
            let (first, _) = equal_pair();
            let mut s = audit(false, json!({}), json!({ "field": [first, LOW] }));
            let fused = fuse_equal(&mut s, 1);
            assert_eq!(subsystems::lines_of_code(s.state(), &fused.def_id), loc());
            s.play(AUDIT, json!({}));
            assert_eq!(s.unit(P2, 1).map(|card| card.id), Some(fused.id.clone()));
            assert!(s.unit(P2, 2).is_none());
        }

        #[test]
        fn s3_2_face_down_backrow_cards_are_permanents_too_and_are_exiled() {
            let mut s = audit(false, json!({}), json!({ "backrow": [{ "def": LOW_TRAP, "faceUp": false }] }));
            s.play(AUDIT, json!({}));
            s.expect_in_zone(LOW_TRAP, "exile");
        }

        #[test]
        fn r11_6_3_an_exile_a_token_ceases_to_exist_reborn_never_returns_and_the_exile_counter_moves() {
            let mut s = audit(false, json!({}), json!({ "field": [RUSH, REBORN] }));
            let token = s.unit(P2, 1).expect("the Rush Token");
            let exiled = s.state().counters.exiled;
            s.play(AUDIT, json!({}));
            s.expect_in_zone(&token, "gone").expect_in_zone(REBORN, "exile");
            assert!(s.unit(P2, 2).is_none());
            assert!(!events_json(&s).iter().any(|event| event["type"] == "destroyed"));
            // The token never reached an exile pile (R11), so only the defender counts.
            assert_eq!(s.state().counters.exiled, exiled + 1);
        }

        #[test]
        fn s6_1_an_exile_is_no_destroy_an_indestructible_permanent_is_exiled_too() {
            let mut s = audit(false, json!({}), json!({ "field": [INDESTRUCTIBLE] }));
            s.play(AUDIT, json!({}));
            s.expect_in_zone(INDESTRUCTIBLE, "exile");
        }

        #[test]
        fn r13_a_card_dormant_under_a_stack_is_not_on_the_field_the_top_goes_and_the_card_beneath_resumes() {
            let mut s = audit(false, json!({}), json!({ "field": [HIGH, { "def": FIENDER, "stack": true }] }));
            s.play(AUDIT, json!({}));
            s.expect_in_zone(FIENDER, "exile");
            assert_eq!(s.unit(P2, 1).map(|card| card.def_id), Some(HIGH.to_string()));
        }

        #[test]
        fn nothing_below_its_loc_nothing_is_exiled_and_the_spell_still_resolves() {
            let mut s = audit(false, json!({}), json!({ "field": [HIGH] }));
            s.play(AUDIT, json!({}));
            assert_eq!(s.unit(P2, 1).map(|card| card.def_id), Some(HIGH.to_string()));
            s.expect_in_zone(AUDIT, "graveyard");
        }

        #[test]
        fn r280_the_base_face_marks_nothing_no_preview() {
            assert!(super::super::script().base.preview.is_none());
            let s = audit(false, json!({}), json!({ "field": [LOW] }));
            assert_eq!(preview(&s), None);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r81_chooses_with_the_play_all_permanents_or_only_the_opponents() {
            assert_eq!(
                serde_json::to_value(&super::super::script().radiant.modes).expect("modes are JSON"),
                json!([{ "kind": "mode", "options": [ALL, THEIRS] }])
            );
            let mut all = audit(true, json!({ "field": [LOW] }), json!({ "field": [LOW, HIGH] }));
            all.play(AUDIT, json!({ "modes": [ALL] }));
            assert!(all.unit(P1, 1).is_none());
            assert!(all.unit(P2, 1).is_none());
            let mut theirs = audit(true, json!({ "field": [LOW] }), json!({ "field": [LOW, HIGH] }));
            theirs.play(AUDIT, json!({ "modes": [THEIRS] }));
            assert_eq!(theirs.unit(P1, 1).map(|card| card.def_id), Some(LOW.to_string()));
            assert!(theirs.unit(P2, 1).is_none());
            assert_eq!(theirs.unit(P2, 2).map(|card| card.def_id), Some(HIGH.to_string()));
        }

        #[test]
        fn r280_r583_its_preview_is_for_each_choice_the_permanents_it_would_exile_now_and_that_is_what_the_play_exiles() {
            let mut s = audit(true, json!({ "field": [LOW, HIGH] }), json!({ "field": [LOW, HIGH] }));
            let mine = s.unit(P1, 1).map(|card| card.id).unwrap_or_default();
            let theirs = s.unit(P2, 1).map(|card| card.id).unwrap_or_default();
            assert_eq!(
                preview(&s),
                Some(json!([
                    { "label": "all permanents", "value": 2, "ids": [mine, theirs] },
                    { "label": "only your opponent's", "value": 1, "ids": [theirs] },
                ]))
            );
            let text = crate::card_def(AUDIT).radiant.text;
            for entry in preview(&s).and_then(|entries| entries.as_array().cloned()).unwrap_or_default() {
                assert!(text.contains(entry["label"].as_str().unwrap_or_default()));
            }
            s.play(AUDIT, json!({ "modes": [ALL] }));
            assert_eq!([s.card(&mine).zone.z(), s.card(&theirs).zone.z()], [ZoneName::Exile, ZoneName::Exile]);
        }

        #[test]
        fn r177_r583_the_preview_never_marks_an_enemy_face_down_card_which_the_exile_still_takes_its_own_face_down_card_it_marks() {
            let mut s = audit(
                true,
                json!({ "backrow": [{ "def": LOW_TRAP, "faceUp": false }] }),
                json!({ "backrow": [{ "def": LOW_TRAP, "faceUp": false }] }),
            );
            let mine = s.backrow(P1, 1).map(|card| card.id).unwrap_or_default();
            let theirs = s.backrow(P2, 1).map(|card| card.id).unwrap_or_default();
            assert_eq!(
                preview(&s),
                Some(json!([
                    { "label": "all permanents", "value": 1, "ids": [mine] },
                    { "label": "only your opponent's", "value": 0, "ids": [] },
                ]))
            );
            s.play(AUDIT, json!({ "modes": [THEIRS] }));
            s.expect_in_zone(&theirs, "exile");
            assert_eq!(s.backrow(P1, 1).map(|card| card.id), Some(mine));
        }

        #[test]
        fn r280_the_opponents_view_of_the_hand_card_carries_no_preview() {
            let s = audit(true, json!({}), json!({ "field": [LOW] }));
            assert!(!serde_json::to_string(&s.view(P2)).expect("a view is JSON").contains("all permanents"));
        }
    }
}
