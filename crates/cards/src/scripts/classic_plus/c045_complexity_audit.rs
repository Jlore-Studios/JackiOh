//! C+ #45 Complexity Audit (SPEC §8.7 row 45, E36, R280, R583). (2) Spell, Rare.
//!   Exile every permanent whose card has more lines of code than this one. Radiant: choose all
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
use jackioh_engine::subsystems;

pub const ID: &str = "classicplus-045";

const ALL: &str = "All permanents";
const THEIRS: &str = "Only your opponent's";

/// The permanents whose card has more lines of code than `defId`'s.
fn targets(
    state: &GameState,
    controller: PlayerId,
    active: PlayerId,
    def_id: &str,
    enemy_only: bool,
) -> Vec<CardInstance> {
    let loc = subsystems::lines_of_code(state, def_id);
    subsystems::audit_targets(
        state,
        subsystems::audit::AuditArgs {
            controller,
            active,
            loc,
            more: true,
            enemy_only,
        },
    )
    .into_iter()
    .map(|card| CardInstance::clone(&card))
    .collect()
}

/// TS `audit(enemyOnly): Script["cry"]`: one `forEachCard` over the permanents the run would exile, read
/// once as the list reaches it, each exiled by id.
fn audit(enemy_only: impl Fn(&EffectContext<'_>) -> bool + Send + Sync + 'static) -> Hook {
    let enemy_only: Arc<dyn Fn(&EffectContext<'_>) -> bool + Send + Sync> = Arc::new(enemy_only);
    hook(move |_ctx| {
        let enemy_only = enemy_only.clone();
        vec![for_each_card(ForEachCardArgs {
            cards: Arc::new(move |run: &mut EffectContext<'_>| -> Vec<String> {
                let def_id = run
                    .self_
                    .as_ref()
                    .map(|card| card.def_id.clone())
                    .or_else(|| run.def_id.clone())
                    .unwrap_or_else(|| ID.to_string());
                targets(&*run.state, run.controller, run.state.active, &def_id, enemy_only(run))
                    .into_iter()
                    .map(|card| card.id)
                    .collect()
            }),
            each: Arc::new(|instance_id: &str| {
                exile(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
            }),
        })]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(audit(|_ctx| false)),
        ..Script::default()
    };

    let radiant = Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![ALL.to_string(), THEIRS.to_string()],
        }],
        cry: Some(audit(|ctx| chosen_options(ctx).first().map(String::as_str) == Some(THEIRS))),
        preview: Some(condition_hook(|ctx| -> Vec<PreviewValue> {
            [("all permanents", false), ("only your opponent's", true)]
                .into_iter()
                .map(|(label, enemy_only)| {
                    let active = if ctx.your_turn {
                        ctx.controller
                    } else {
                        opponent_of(ctx.controller)
                    };
                    let seen = targets(ctx.state, ctx.controller, active, &ctx.self_.def_id, enemy_only);
                    let ids: Vec<String> = seen
                        .iter()
                        .filter(|card| !unreadable_by(ctx.state, card).contains(&ctx.controller))
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

// C+ #45 Complexity Audit — SPEC §8.7 row 45, E36, R13, R177, R280, R583, BUILD M9 row C+ 45: C+ #44
// with the comparison reversed. The sweep is `subsystems.auditTargets` (packages/engine/test/audit.test.ts);
// its preview is pinned in test/preview.test.ts. Every `loc` is read off the catalog and guarded.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const AUDIT: &str = "classicplus-045";
    const LOW: &str = "core-008"; // Mr. Vanilla
    const HIGH: &str = "core-022"; // Carnivorous Cube, whose Death an exile never fires
    const HIGH_TRAP: &str = "core-060"; // Bear Honeypot
    const FIENDER: &str = "core-092"; // Felinor Fiender, Stack
    const FILLER: &str = "core-005";
    const ALL: &str = "All permanents";
    const THEIRS: &str = "Only your opponent's";

    /// TS `const LOC = cardDef(AUDIT).loc ?? 0`.
    fn loc() -> i32 {
        crate::card_def(AUDIT).loc.unwrap_or(0)
    }

    /// Two Units whose `loc` sums to the Audit's: fused, they are a permanent of exactly its `loc`.
    fn equal_pair() -> (String, String) {
        let target = loc();
        let units: Vec<&CardDef> = crate::CATALOG
            .values()
            .filter(|card| card.type_ == CardType::Unit && !card.token && card.loc.is_some())
            .collect();
        for a in &units {
            let b = units
                .iter()
                .find(|other| other.id != a.id && a.loc.unwrap_or(0) + other.loc.unwrap_or(0) == target);
            if let Some(b) = b {
                return (a.id.clone(), b.id.clone());
            }
        }
        panic!("no two Units sum to {target} lines");
    }

    /// TS `{ ...base, ...extra }` on two object literals.
    fn merged(mut base: Value, extra: Value) -> Value {
        if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
            for (key, value) in from {
                into.insert(key, value);
            }
        }
        base
    }

    /// TS `audit({ radiant?, p1?, p2? })`: the Audit and a filler in p1's hand, a filler in p2's, and
    /// each side's extra setup spread over that.
    fn audit(radiant: bool, p1: Value, p2: Value) -> Scenario {
        scenario(json!({
            "p1": merged(json!({ "hand": [{ "def": AUDIT, "radiant": radiant }, FILLER] }), p1),
            "p2": merged(json!({ "hand": [FILLER] }), p2),
        }))
    }

    fn fuse_equal(s: &mut Scenario, lane: i32) -> CardInstance {
        let kept = s.unit(P2, lane).expect("a unit to fuse onto");
        let pair = equal_pair();
        let state = s.state_mut();
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let phantom = new_instance(&mut *state, &pair.1, P2, Zone::Gone { player: P2 });
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let fused = subsystems::fuse(
            &mut sink,
            subsystems::FuseArgs {
                ingredients: vec![kept.clone(), phantom],
                target: Some(kept),
                ..Default::default()
            },
        );
        fused.expect("the fusion")
    }

    fn preview(s: &Scenario) -> Option<Vec<PreviewValue>> {
        let id = s.card(AUDIT).id.clone();
        match s.view(P1).you.hand {
            HandView::Cards(hand) => hand
                .into_iter()
                .find(|card| card.instance_id == id)
                .and_then(|card| card.preview),
            HandView::Count { .. } => None,
        }
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    fn def_at(s: &Scenario, seat: PlayerId, lane: i32) -> Option<String> {
        s.unit(seat, lane).map(|unit| unit.def_id)
    }

    #[test]
    fn the_catalog_s_loc_guards_the_cards_below_are_above_and_below_the_audit_s() {
        crate::register_all();
        assert_eq!(crate::card_def(super::ID).id, AUDIT);
        for id in [HIGH, HIGH_TRAP] {
            assert!(crate::card_def(id).loc.unwrap_or(0) > loc());
        }
        for id in [LOW, FIENDER] {
            assert!(crate::card_def(id).loc.unwrap_or(0) < loc());
        }
    }

    mod base {
        use super::*;

        #[test]
        fn e36_exiles_every_permanent_on_both_sides_whose_card_has_more_lines_of_code_a_lower_one_stays() {
            crate::register_all();
            let mut s = audit(false, json!({ "field": [HIGH, LOW] }), json!({ "field": [HIGH, LOW] }));
            let (mine, theirs) = (s.unit(P1, 1).unwrap(), s.unit(P2, 1).unwrap());
            s.play(AUDIT, json!({}));
            s.expect_in_zone(&mine, "exile").expect_in_zone(&theirs, "exile");
            assert_eq!(def_at(&s, P1, 2).as_deref(), Some(LOW));
            assert_eq!(def_at(&s, P2, 2).as_deref(), Some(LOW));
            // An exile fires no Death (§6.3): the Cubes' Deaths never ran.
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::Destroyed { .. })));
        }

        #[test]
        fn e36_an_equal_loc_stays_a_fused_card_s_loc_is_its_ingredients_sum_r77() {
            crate::register_all();
            let mut s = audit(false, json!({}), json!({ "field": [equal_pair().0, HIGH] }));
            let fused = fuse_equal(&mut s, 1);
            assert_eq!(subsystems::lines_of_code(s.state(), &fused.def_id), loc());
            s.play(AUDIT, json!({}));
            assert_eq!(s.unit(P2, 1).map(|unit| unit.id), Some(fused.id.clone()));
            assert!(s.unit(P2, 2).is_none());
        }

        #[test]
        fn s3_2_face_down_backrow_cards_are_permanents_too_and_are_exiled() {
            crate::register_all();
            let mut s = audit(false, json!({}), json!({ "backrow": [{ "def": HIGH_TRAP, "faceUp": false }] }));
            s.play(AUDIT, json!({}));
            s.expect_in_zone(HIGH_TRAP, "exile");
        }

        #[test]
        fn r13_a_card_dormant_under_a_stack_is_not_on_the_field_and_is_not_exiled() {
            crate::register_all();
            let mut s = audit(false, json!({}), json!({ "field": [HIGH, { "def": FIENDER, "stack": true }] }));
            let buried = s.card(HIGH).clone();
            s.play(AUDIT, json!({}));
            assert_eq!(def_at(&s, P2, 1).as_deref(), Some(FIENDER));
            assert_eq!(s.card(&buried).zone.z(), ZoneName::Field);
        }

        #[test]
        fn nothing_above_its_loc_nothing_is_exiled_and_the_spell_still_resolves() {
            crate::register_all();
            let mut s = audit(false, json!({}), json!({ "field": [LOW] }));
            s.play(AUDIT, json!({}));
            assert_eq!(def_at(&s, P2, 1).as_deref(), Some(LOW));
            s.expect_in_zone(AUDIT, "graveyard");
        }

        #[test]
        fn r280_the_base_face_marks_nothing_no_preview() {
            crate::register_all();
            assert!(script().base.preview.is_none());
            assert!(preview(&audit(false, json!({}), json!({ "field": [HIGH] }))).is_none());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r81_chooses_with_the_play_all_permanents_or_only_the_opponent_s() {
            crate::register_all();
            assert_eq!(js(&script().radiant.modes), json!([{ "kind": "mode", "options": [ALL, THEIRS] }]));
            let mut theirs = audit(true, json!({ "field": [HIGH] }), json!({ "field": [HIGH, LOW] }));
            theirs.play(AUDIT, json!({ "modes": [THEIRS] }));
            assert_eq!(def_at(&theirs, P1, 1).as_deref(), Some(HIGH));
            assert!(theirs.unit(P2, 1).is_none());
            assert_eq!(def_at(&theirs, P2, 2).as_deref(), Some(LOW));
            let mut all = audit(true, json!({ "field": [HIGH] }), json!({ "field": [HIGH] }));
            all.play(AUDIT, json!({ "modes": [ALL] }));
            assert_eq!([all.unit(P1, 1).is_none(), all.unit(P2, 1).is_none()], [true, true]);
        }

        #[test]
        fn r280_r583_its_preview_is_for_each_choice_the_permanents_it_would_exile_now_and_that_is_what_the_play_exiles() {
            crate::register_all();
            let mut s = audit(true, json!({ "field": [HIGH, LOW] }), json!({ "field": [HIGH, LOW] }));
            let (mine, theirs) = (
                s.unit(P1, 1).map(|unit| unit.id).unwrap_or_default(),
                s.unit(P2, 1).map(|unit| unit.id).unwrap_or_default(),
            );
            assert_eq!(
                js(&preview(&s)),
                json!([
                    { "label": "all permanents", "value": 2, "ids": [mine, theirs] },
                    { "label": "only your opponent's", "value": 1, "ids": [theirs] },
                ])
            );
            for entry in preview(&s).unwrap_or_default() {
                assert!(crate::card_def(AUDIT).radiant.text.contains(&entry.label));
            }
            s.play(AUDIT, json!({ "modes": [ALL] }));
            assert_eq!([s.card(&mine).zone.z(), s.card(&theirs).zone.z()], [ZoneName::Exile, ZoneName::Exile]);
        }

        #[test]
        fn r177_r583_the_preview_never_marks_an_enemy_face_down_card_which_the_exile_still_takes() {
            crate::register_all();
            let mut s = audit(true, json!({}), json!({ "backrow": [{ "def": HIGH_TRAP, "faceUp": false }] }));
            assert_eq!(
                js(&preview(&s)),
                json!([
                    { "label": "all permanents", "value": 0, "ids": [] },
                    { "label": "only your opponent's", "value": 0, "ids": [] },
                ])
            );
            s.play(AUDIT, json!({ "modes": [THEIRS] }));
            s.expect_in_zone(HIGH_TRAP, "exile");
        }
    }
}
