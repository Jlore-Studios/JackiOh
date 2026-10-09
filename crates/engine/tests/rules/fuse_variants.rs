//! The Fuse variants of patch v0.2.0 (docs/classic-sets.md B5 E23; R77, R102, R179) and the three
//! rulings they needed: R468 (a fused id is bounded: past `FUSED_ID_CAP` it is a digest of its
//! ingredient list, which the definition keeps), R469 (an ingredient fused "as a Radiant card" puts
//! its Radiant face into both fused forms) and R470 (a fusion may keep a hand or deck card, which
//! stays where it is, and "its cost doesn't change" keeps the cost it had).
//!
//! TS read the live objects `fuse` changed; Rust reads a card back from the state by its id, and an
//! ingredient that ceased to exist (`{ z: "gone" }`, R86) is one the state holds in no pile. A digest
//! id's ingredients come off its fused definition in the state (SURFACE §6.6: no process-wide digest
//! table), so the catalog readers take the state; read without one (`None`) a digest id is no fused
//! id, which is what TS's process answered before it had seen the definition.
//!
//! Port of `packages/engine/test/fuse-variants.test.ts`.

use jackioh_engine::catalog::{
    CatalogQueryArgs, def_of, excluding_def_id, fused_id_parts, fused_id_specs, is_digest_id, self_def_ids,
};
use jackioh_engine::effects::{fuse_cards, fuse_generated, fuse_random_into};
use jackioh_engine::layers::unit_view;
use jackioh_engine::mana::effective_cost;
use jackioh_engine::plague::plague_multiplier_of;
use jackioh_engine::reduce::legal_actions;
use jackioh_engine::replay::hash_state;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::scripts::script_of;
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse, fused_digest, fused_ingredients};
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::{HIDDEN_ID, view_for};
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::generation::{
    GEN_SCRIPTS, LAB_POOL, Run, act, ai_spell, ai_unit, answer, big_unit, body, deck_fusion, felinor_a,
    felinor_b, felinor_c, field_trap, frozen, fuse_a, fuse_b, fuser, hand_card, immutable, lab, mutate, pick,
    plain_trap, playing, replayed, slime, slop, x_unit,
};
use crate::rules::fixtures::harness::{put, set_library, slot};

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// `JSON.parse(JSON.stringify(state))`.
fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).expect("a state survives JSON")
}

/// vitest's `toMatchObject`: every key the expected object names matches, recursively.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(a, e)| matches_object(a, e))
        }
        _ => actual == expected,
    }
}

/// TS `sinkFor(state)`: a sink whose rng starts at the state's cursor, as reduce does. The events and
/// the rng are kept beside the state, so the test can change the state between calls as TS's shared
/// objects let it.
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Sink {
    fn for_state(state: &GameState) -> Sink {
        Sink {
            events: Vec::new(),
            rng: Rng::new(&state.seed, state.rng_cursor),
        }
    }

    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

/// TS `run(sink, effect, self = null)`: the effect applied in a context of p1's.
fn run(state: &mut GameState, sink: &mut Sink, effect: Effect, self_: Option<&CardInstance>) {
    let mut engine = sink.on(state);
    let mut ctx = make_context(
        &mut engine,
        self_,
        HookOptions {
            controller: Some(P1),
            ..Default::default()
        },
    );
    (effect.apply)(&mut ctx);
}

/// `fuse(sink, args)`, `args` the TS object literal (its instances as the state holds them now).
fn fuse_in(state: &mut GameState, sink: &mut Sink, args: Value) -> Option<CardInstance> {
    let args: FuseArgs = json_as(args);
    fuse(&mut sink.on(state), args)
}

/// `fuse(sinkFor(state), args)`.
fn fuse_fresh(state: &mut GameState, args: Value) -> Option<CardInstance> {
    let mut sink = Sink::for_state(state);
    fuse_in(state, &mut sink, args)
}

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("expected {what}"))
}

/// The card under `id` as the state holds it now (TS's live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id}"))
}

fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn input(literal: Value) -> ActionInput {
    json_as(literal)
}

/// The `fused` events of a view's event list (`eventsOfType(view.events, "fused")`).
fn fused_events(events: &Value) -> Vec<Value> {
    events
        .as_array()
        .map(|events| {
            events
                .iter()
                .filter(|event| event["type"] == json!("fused"))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// A phantom ingredient: a definition only, in no pile (R86), as `fuseCards` makes one.
fn phantom(state: &mut GameState, def_id: &str) -> CardInstance {
    new_instance(state, def_id, P1, Zone::Gone { player: P1 })
}

/// `/^t-\d+:/`: the head a Fuse writes on every id it mints.
fn fused_head(id: &str) -> bool {
    id.strip_prefix("t-")
        .and_then(|rest| rest.split_once(':'))
        .is_some_and(|(count, _)| !count.is_empty() && count.bytes().all(|byte| byte.is_ascii_digit()))
}

/// `/^[0-9a-f]{16}$/`.
fn hex16(text: &str) -> bool {
    text.len() == 16
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// `/^t-\d+:#[0-9a-f]{16}$/`.
fn digest_shaped(id: &str) -> bool {
    id.strip_prefix("t-")
        .and_then(|rest| rest.split_once(":#"))
        .is_some_and(|(count, digest)| {
            !count.is_empty() && count.bytes().all(|byte| byte.is_ascii_digit()) && hex16(digest)
        })
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// `(attack, maxHealth)` as §10.4's layers read the card.
fn stats(state: &GameState, card: &CardInstance) -> (i32, i32) {
    let view = unit_view(state, card);
    (view.attack, view.max_health)
}

/// A copy of `run` whose state is `state` (TS `{ ...run, state }`).
fn with_state(run: &Run, state: GameState) -> Run {
    Run {
        start: run.start.clone(),
        log: run.log.clone(),
        state,
    }
}

// ---------------------------------------------------------------------------
// R468: bounded fused ids.
// ---------------------------------------------------------------------------

mod r468_a_fused_id_is_bounded_past_fused_id_cap_it_is_a_digest_of_the_ingredient_list {
    use super::*;

    #[test]
    fn r468_a_short_list_is_spelled_out_r179_a_card_fused_onto_again_and_again_switches_to_a_digest_once_the_list_outgrows_the_cap()
     {
        let mut state = playing("fuse-digest").state;
        let kept = put(&mut state, &slime.id, slot(P1, Row::Units, 1), json!({}));
        let mut sink = Sink::for_state(&state);

        let ingredient = phantom(&mut state, &slime.id);
        let target = live(&state, &kept.id);
        let first = must(
            fuse_in(
                &mut state,
                &mut sink,
                json!({ "ingredients": [ingredient], "target": target }),
            ),
            "a fusion",
        );
        assert_eq!(
            first.def_id,
            format!("t-1:{}+{}", slime.id.clone(), slime.id.clone())
        );

        let mut fusions: u32 = 1;
        while !is_digest_id(&live(&state, &kept.id).def_id) {
            let ingredient = phantom(&mut state, &slime.id);
            let target = live(&state, &kept.id);
            fuse_in(
                &mut state,
                &mut sink,
                json!({ "ingredients": [ingredient], "target": target }),
            );
            fusions += 1;
            if fusions > 40 {
                panic!("the id never switched to a digest");
            }
        }
        let id = live(&state, &kept.id).def_id;
        assert!(digest_shaped(&id));
        // The fused id before it spelled out everything and sat within the cap; this one would not have.
        let def = def_of(Some(&state), &id).clone();
        let spelled: Vec<String> = must(def.ingredients.clone(), "the ingredient list")
            .iter()
            .map(|entry| {
                if fused_head(&entry.def_id) {
                    format!("({})", entry.def_id)
                } else {
                    entry.def_id.clone()
                }
            })
            .collect();
        assert!(spelled.join("+").len() > FUSED_ID_CAP);
        assert_eq!(id, format!("t-{fusions}:#{}", fused_digest(&spelled.join("+"))));

        // The scripts are the list's: every Slime text multiplies (R471), fused onto itself `fusions` times.
        assert_eq!(
            plague_multiplier_of(&state, &live(&state, &kept.id)),
            2_i32.pow(fusions + 1)
        );
        // One more fusion onto it names the digest in parentheses, which stays short. The kept card is the
        // last ingredient, as R77's target always is (#85's played card fused onto its target).
        let ingredient = phantom(&mut state, &slime.id);
        let target = live(&state, &kept.id);
        fuse_in(
            &mut state,
            &mut sink,
            json!({ "ingredients": [ingredient], "target": target }),
        );
        let now = live(&state, &kept.id);
        assert_eq!(
            now.def_id,
            format!("t-{}:{}+({id})", fusions + 1, slime.id.clone())
        );
        assert_eq!(
            fused_id_parts(Some(&state), &now.def_id),
            Some(vec![slime.id.clone(), id.clone()])
        );
        assert_eq!(plague_multiplier_of(&state, &now), 2_i32.pow(fusions + 2));
    }

    #[test]
    fn r468_the_digest_is_a_pure_function_of_the_list_the_same_list_gives_the_same_digest_a_different_one_another()
     {
        let a = fused_digest("gen-a+gen-b");
        assert_eq!(fused_digest("gen-a+gen-b"), a);
        assert_ne!(fused_digest("gen-b+gen-a"), a);
        assert_ne!(fused_digest("gen-a+gen-b*"), a);
        assert!(hex16(&a));
    }

    #[test]
    fn r468_a_state_holding_a_digest_id_this_process_never_minted_rebuilds_its_scripts_from_the_definition() {
        let mut state = playing("fuse-digest-json").state;
        let id = format!("t-1:#{}", fused_digest("never-minted-here"));
        let mut fused = def_of(Some(&state), &slime.id).clone();
        fused.id = id.clone();
        fused.index = id.clone();
        fused.name = "Gen slime + Gen slime".to_string();
        fused.ingredients = Some(json_as(json!([{ "defId": slime.id }, { "defId": slime.id }])));
        state.transient_defs.insert(id.clone(), fused);
        let card = put(&mut state, &id, slot(P1, Row::Units, 1), json!({}));
        // Read without the state, a digest is no fused id: nothing in this process has seen its list.
        assert!(fused_id_specs(None, &id).is_none());
        assert!(registered_scripts().get(id.as_str()).is_none());

        let round = round_trip(&state);
        let _ = legal_actions(&round, P1);
        assert_eq!(
            json_of(fused_id_specs(Some(&round), &id)),
            json!([{ "defId": slime.id }, { "defId": slime.id }])
        );
        assert_eq!(
            fused_ingredients(&round, &id),
            Some(vec![slime.id.clone(), slime.id.clone()])
        );
        assert!(script_of(&round, id.as_str()).base.plague_multiplier.is_some());
        let again = must(find_instance(&round, &card.id).cloned(), "the card after JSON");
        assert_eq!(plague_multiplier_of(&round, &again), 4);

        // B4.1: a digest-named fused card never generates any of its ingredients either.
        assert_eq!(
            json_of(self_def_ids(Some(&round), &id)),
            json!([slime.id.clone(), slime.id.clone()])
        );
        let excluded: IndexSet<String> = json_of(excluding_def_id(
            Some(&round),
            &CatalogQueryArgs::default(),
            Some(id.as_str()),
        ))["excludeDefId"]
            .as_array()
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| id.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(excluded, IndexSet::from([slime.id.clone()]));

        // A registry replaced wholesale is repaired on the next entry, digest ids included.
        register_scripts(GEN_SCRIPTS.clone());
        assert!(registered_scripts().get(id.as_str()).is_none());
        let _ = view_for(&round, P1);
        assert!(script_of(&round, id.as_str()).base.plague_multiplier.is_some());
    }

    #[test]
    fn r468_every_fused_definition_keeps_its_ingredient_list_and_sums_its_ingredients_lines_of_code_e36() {
        let mut state = playing("fuse-loc").state;
        let kept = put(&mut state, &fuse_a.id, slot(P1, Row::Units, 1), json!({}));
        let mut sink = Sink::for_state(&state);
        let ingredient = phantom(&mut state, &fuse_b.id);
        let result = must(
            fuse_in(
                &mut state,
                &mut sink,
                json!({ "ingredients": [ingredient], "target": kept }),
            ),
            "a fusion",
        );
        let def = def_of(Some(&state), &result.def_id).clone();
        assert_eq!(
            json_of(&def.ingredients),
            json!([{ "defId": fuse_b.id }, { "defId": fuse_a.id }])
        );
        assert_eq!(def.loc, Some(17));

        let other = put(&mut state, &body.id, slot(P1, Row::Units, 2), json!({}));
        let ingredient = phantom(&mut state, &fuse_b.id);
        let plain = must(
            fuse_in(
                &mut state,
                &mut sink,
                json!({ "ingredients": [ingredient], "target": other }),
            ),
            "a fusion",
        );
        assert_eq!(def_of(Some(&state), &plain.def_id).loc, Some(7));
        let none = put(&mut state, &body.id, slot(P1, Row::Units, 3), json!({}));
        let ingredient = phantom(&mut state, &body.id);
        let bare = must(
            fuse_in(
                &mut state,
                &mut sink,
                json!({ "ingredients": [ingredient], "target": none }),
            ),
            "a fusion",
        );
        assert_eq!(def_of(Some(&state), &bare.def_id).loc, None);
    }
}

// ---------------------------------------------------------------------------
// R469: an ingredient fused as a Radiant card.
// ---------------------------------------------------------------------------

mod r469_an_ingredient_fused_on_its_radiant_face_goes_into_both_fused_forms {
    use super::*;

    #[test]
    fn r469_the_id_marks_it_with_a_star_and_the_base_form_carries_its_radiant_face_and_text() {
        let mut state = playing("fuse-radiant-ingredient").state;
        let kept = put(&mut state, &body.id, slot(P1, Row::Units, 1), json!({}));
        let ingredient = phantom(&mut state, &fuse_a.id);
        let result = must(
            fuse_fresh(
                &mut state,
                json!({ "ingredients": [ingredient], "target": kept, "radiantIngredients": [ingredient.id] }),
            ),
            "a fusion",
        );
        assert_eq!(
            result.def_id,
            format!("t-1:{}*+{}", fuse_a.id.clone(), body.id.clone())
        );
        assert_eq!(
            json_of(fused_id_specs(Some(&state), &result.def_id)),
            json!([{ "defId": fuse_a.id, "radiant": true }, { "defId": body.id }])
        );
        assert_eq!(
            fused_id_parts(Some(&state), &result.def_id),
            Some(vec![fuse_a.id.clone(), body.id.clone()])
        );
        let def = def_of(Some(&state), &result.def_id).clone();
        // body 1/1 on the base form, fuseA's Radiant 7/1 Taunt on both.
        assert!(matches_object(
            &json_of(&def.base),
            &json!({ "attack": 8, "health": 2 })
        ));
        assert_eq!(json_of(&def.base.keywords), json!([{ "kind": "Taunt" }]));
        assert!(def.base.text.contains("fuse-a radiant"));
        assert!(matches_object(
            &json_of(&def.radiant),
            &json!({ "attack": 9, "health": 3 })
        ));
        assert_eq!(stats(&state, &live(&state, &result.id)), (8, 2));
    }

    #[test]
    fn r469_its_radiant_script_runs_on_the_base_form_a_radiant_slime_triples_on_a_base_card() {
        let mut state = playing("fuse-radiant-script").state;
        let kept = put(&mut state, &body.id, slot(P1, Row::Units, 1), json!({}));
        let ingredient = phantom(&mut state, &slime.id);
        fuse_fresh(
            &mut state,
            json!({ "ingredients": [ingredient], "target": kept, "radiantIngredients": [ingredient.id] }),
        );
        let kept = live(&state, &kept.id);
        assert!(!kept.radiant);
        assert_eq!(plague_multiplier_of(&state, &kept), 3);
    }

    #[test]
    fn r469_fuse_cards_radiant_ingredients_the_opponents_played_card_fused_into_your_permanent_as_a_radiant_copy_classic_plus_74()
     {
        let mut state = playing("fuse-twice-forward").state;
        let trap = put(&mut state, &field_trap.id, slot(P1, Row::Backrow, 1), json!({}));
        let played = put(&mut state, &fuse_a.id, slot(P2, Row::Units, 1), json!({}));
        let mut sink = Sink::for_state(&state);
        run(
            &mut state,
            &mut sink,
            fuse_cards(json_as(json!({
                "instanceIds": [played.id],
                "targetInstanceId": trap.id,
                "radiantIngredients": true,
            }))),
            Some(&trap),
        );
        let trap_now = live(&state, &trap.id);
        let def = def_of(Some(&state), &trap_now.def_id).clone();
        // The kept instance and its type stay; the opponent's card ceases to exist (R77, R86).
        assert_eq!(json_of(def.type_), json!("Field Trap"));
        assert_eq!(
            trap_now.zone,
            Zone::Field {
                player: P1,
                row: Row::Backrow,
                lane: 1,
            }
        );
        assert!(find_instance(&state, &played.id).is_none());
        assert!(state.players.p2.units[0].is_none());
        assert!(def.base.text.contains("fuse-a radiant"));
        assert_eq!(
            trap_now.def_id,
            format!("t-1:{}*+{}", fuse_a.id.clone(), field_trap.id.clone())
        );
    }
}

// ---------------------------------------------------------------------------
// R470: fusing into a hand or deck card, keeping its cost.
// ---------------------------------------------------------------------------

mod r470_a_fusion_keeps_a_hand_or_deck_card_where_it_is_and_its_cost_doesnt_change {
    use super::*;

    #[test]
    fn r470_fusion_lab_a_random_card_of_the_pool_never_the_lab_b4_1_into_the_declared_hand_card_which_keeps_its_cost()
     {
        let mut start = playing("fuse-lab");
        let card = hand_card(&mut start.state, &fuse_b.id, P1);
        live_mut(&mut start.state, &card.id).cost_mod = 1;
        let lab_card = hand_card(&mut start.state, &lab.id, P1);
        let cost_before = effective_cost(&start.state, &live(&start.state, &card.id), Default::default());
        let mut run1 = frozen(&start);
        run1 = act(
            &run1,
            input(
                json!({ "type": "play", "instanceId": lab_card.id, "targets": [pick(&card.id)], "playerId": "p1" }),
            ),
        );

        let kept = must(find_instance(&run1.state, &card.id).cloned(), "the hand card");
        assert_eq!(kept.zone, Zone::Hand { player: P1 });
        let def = def_of(Some(&run1.state), &kept.def_id).clone();
        let parts = must(fused_id_parts(Some(&run1.state), &kept.def_id), "a fused id");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], fuse_b.id.clone());
        assert!(
            LAB_POOL
                .clone()
                .into_iter()
                .filter(|id| *id != lab.id)
                .any(|id| id == parts[0])
        );
        assert_eq!(json_of(def.type_), json!("Unit"));
        // Its own cost as it stood, printed 1, now its costOverride; its costMod is its own and stays.
        assert_eq!(kept.cost_override, Some(1));
        assert_eq!(kept.cost_mod, 1);
        assert_eq!(
            effective_cost(&run1.state, &kept, Default::default()),
            cost_before
        );
        assert_eq!(hash_state(&replayed(&run1)), hash_state(&run1.state));

        // The opponent learns that some card of that hand fused, never which or into what (§10.8).
        let their_view = json_of(view_for(&run1.state, P2));
        let theirs = fused_events(&their_view["events"])
            .last()
            .cloned()
            .unwrap_or(Value::Null);
        assert!(matches_object(
            &theirs,
            &json!({ "resultInstanceId": HIDDEN_ID, "defId": HIDDEN_ID })
        ));
        assert!(
            theirs["instanceIds"]
                .as_array()
                .is_some_and(|ids| ids.iter().all(|id| *id == json!(HIDDEN_ID)))
        );
        assert!(their_view["defs"].get(&kept.def_id).is_none());
        let my_view = json_of(view_for(&run1.state, P1));
        let mine = fused_events(&my_view["events"])
            .last()
            .cloned()
            .unwrap_or(Value::Null);
        assert!(matches_object(
            &mine,
            &json!({ "resultInstanceId": card.id, "defId": kept.def_id })
        ));
        assert_eq!(
            my_view["defs"][&kept.def_id]["ingredients"],
            json_of(&def.ingredients)
        );
    }

    #[test]
    fn r470_fusion_labs_radiant_face_fuses_a_radiant_card_r469() {
        let mut start = playing("fuse-lab-radiant");
        let card = hand_card(&mut start.state, &body.id, P1);
        let lab_card = hand_card(&mut start.state, &lab.id, P1);
        live_mut(&mut start.state, &lab_card.id).radiant = true;
        let mut run1 = frozen(&start);
        run1 = act(
            &run1,
            input(
                json!({ "type": "play", "instanceId": lab_card.id, "targets": [pick(&card.id)], "playerId": "p1" }),
            ),
        );
        let kept = must(find_instance(&run1.state, &card.id).cloned(), "the hand card");
        let specs = json_of(fused_id_specs(Some(&run1.state), &kept.def_id));
        assert_eq!(specs[0]["radiant"], json!(true));
        assert_eq!(specs[1], json!({ "defId": body.id }));
    }

    #[test]
    fn r470_an_x_cost_or_embiggen_hand_card_keeps_its_printed_form_a_card_with_an_override_keeps_the_override()
     {
        let mut state = playing("fuse-keep-forms").state;
        let mut sink = Sink::for_state(&state);
        let x = hand_card(&mut state, &x_unit.id, P1);
        let big = hand_card(&mut state, &big_unit.id, P1);
        let crafted = hand_card(&mut state, &fuse_a.id, P1);
        live_mut(&mut state, &crafted.id).cost_override = Some(0);

        let ingredient = phantom(&mut state, &fuse_a.id);
        let into = live(&state, &x.id);
        fuse_in(
            &mut state,
            &mut sink,
            json!({ "ingredients": [ingredient], "into": into, "keepCost": true }),
        );
        let ingredient = phantom(&mut state, &fuse_a.id);
        let into = live(&state, &big.id);
        fuse_in(
            &mut state,
            &mut sink,
            json!({ "ingredients": [ingredient], "into": into, "keepCost": true }),
        );
        let ingredient = phantom(&mut state, &fuse_b.id);
        let into = live(&state, &crafted.id);
        fuse_in(
            &mut state,
            &mut sink,
            json!({ "ingredients": [ingredient], "into": into, "keepCost": true }),
        );

        let x = live(&state, &x.id);
        let big = live(&state, &big.id);
        let crafted = live(&state, &crafted.id);
        assert_eq!(json_of(def_of(Some(&state), &x.def_id).cost), json!("X"));
        assert!(x.cost_override.is_none());
        assert_eq!(
            json_of(def_of(Some(&state), &big.def_id).cost),
            json!({ "base": 2, "embiggen": 4 })
        );
        assert!(big.cost_override.is_none());
        assert_eq!(crafted.cost_override, Some(0));
        assert_eq!(effective_cost(&state, &crafted, Default::default()), 0);

        // Without keepCost the kept card takes R77's fused cost.
        let plain = hand_card(&mut state, &fuse_a.id, P1);
        let ingredient = phantom(&mut state, &fuse_b.id);
        fuse_in(
            &mut state,
            &mut sink,
            json!({ "ingredients": [ingredient], "into": plain }),
        );
        let plain = live(&state, &plain.id);
        assert!(plain.cost_override.is_none());
        assert_eq!(effective_cost(&state, &plain, Default::default()), 3);
    }

    #[test]
    fn r470_into_takes_only_a_hand_or_library_card_target_only_a_field_card_and_an_immutable_card_refuses_either()
     {
        let mut state = playing("fuse-into-refusals").state;
        let mut sink = Sink::for_state(&state);
        let on_field = put(&mut state, &body.id, slot(P1, Row::Units, 1), json!({}));
        let held = hand_card(&mut state, &body.id, P1);
        let locked = hand_card(&mut state, &immutable.id, P1);
        let gy = new_instance(&mut state, &body.id, P1, Zone::Graveyard { player: P1 });
        state.players.p1.graveyard.push(gy.clone());

        let ingredient = phantom(&mut state, &fuse_b.id);
        assert!(
            fuse_in(
                &mut state,
                &mut sink,
                json!({ "ingredients": [ingredient], "into": on_field })
            )
            .is_none()
        );
        let ingredient = phantom(&mut state, &fuse_b.id);
        assert!(
            fuse_in(
                &mut state,
                &mut sink,
                json!({ "ingredients": [ingredient], "target": held })
            )
            .is_none()
        );
        let ingredient = phantom(&mut state, &fuse_b.id);
        assert!(
            fuse_in(
                &mut state,
                &mut sink,
                json!({ "ingredients": [ingredient], "into": gy })
            )
            .is_none()
        );
        let ingredient = phantom(&mut state, &fuse_b.id);
        assert!(
            fuse_in(
                &mut state,
                &mut sink,
                json!({ "ingredients": [ingredient], "into": locked })
            )
            .is_none()
        );
        assert!(sink.events.is_empty());
        assert!(state.transient_defs.is_empty());

        // A random fusion into a card the Fuse would refuse draws nothing for it (R129).
        let cursor = sink.rng.cursor();
        let targeted = |card: &CardInstance| -> Effect {
            fuse_random_into(json_as(json!({
                "into": { "target": { "of": "instance", "instanceId": card.id } },
                "query": { "defId": LAB_POOL.clone() },
            })))
        };
        run(&mut state, &mut sink, targeted(&locked), None);
        run(&mut state, &mut sink, targeted(&gy), None);
        assert_eq!(sink.rng.cursor(), cursor);
        assert!(sink.events.is_empty());
    }

    #[test]
    fn r470_the_deck_fusion_a_random_card_into_every_deck_card_each_keeping_its_cost_hidden_from_both_players_classic_plus_73()
     {
        let mut start = playing("fuse-deck");
        let library = set_library(
            &mut start.state,
            P1,
            &[fuse_a.id.clone(), x_unit.id.clone(), body.id.clone()],
        );
        let costs: Vec<i32> = library
            .iter()
            .map(|card| effective_cost(&start.state, card, Default::default()))
            .collect();
        let list_before = json_of(view_for(&start.state, P1))["you"]["ownLibrary"].clone();
        let spell = hand_card(&mut start.state, &deck_fusion.id, P1);
        let mut run1 = frozen(&start);
        run1 = act(
            &run1,
            input(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })),
        );

        let after = run1.state.players.p1.library.clone();
        assert_eq!(ids(&after), ids(&library));
        let second_parts: Vec<Option<String>> = after
            .iter()
            .map(|card| {
                fused_id_parts(Some(&run1.state), &card.def_id).and_then(|parts| parts.get(1).cloned())
            })
            .collect();
        assert_eq!(
            second_parts,
            library
                .iter()
                .map(|card| Some(card.def_id.clone()))
                .collect::<Vec<_>>()
        );
        let distinct: IndexSet<String> = after.iter().map(|card| card.def_id.clone()).collect();
        assert_eq!(distinct.len(), 3);
        assert_eq!(
            after
                .iter()
                .map(|card| effective_cost(&run1.state, card, Default::default()))
                .collect::<Vec<_>>(),
            costs
        );
        let second_def = after.get(1).map(|card| card.def_id.clone()).unwrap_or_default();
        assert_eq!(json_of(def_of(Some(&run1.state), &second_def).cost), json!("X"));
        // Nobody reads a change made inside a library: the owner's list is as it was (R311), and every
        // `fused` event is the sentinel in both views.
        assert_eq!(
            json_of(view_for(&run1.state, P1))["you"]["ownLibrary"],
            list_before
        );
        for viewer in [P1, P2] {
            let events = fused_events(&json_of(view_for(&run1.state, viewer))["events"]);
            assert_eq!(events.len(), 3);
            assert!(
                events
                    .iter()
                    .all(|event| event["resultInstanceId"] == json!(HIDDEN_ID)
                        && event["defId"] == json!(HIDDEN_ID))
            );
        }
        assert_eq!(hash_state(&replayed(&run1)), hash_state(&run1.state));

        // An empty library fuses nothing and draws nothing (R129).
        let mut empty = playing("fuse-deck-empty").state;
        empty.players.p1.library = vec![];
        let mut sink = Sink::for_state(&empty);
        let cursor = sink.rng.cursor();
        run(
            &mut empty,
            &mut sink,
            fuse_random_into(json_as(
                json!({ "into": { "pile": "library" }, "query": { "defId": LAB_POOL.clone() } }),
            )),
            None,
        );
        assert_eq!(sink.rng.cursor(), cursor);
        assert!(sink.events.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Three generated cards with no target (Classic+ #43 AI Slop).
// ---------------------------------------------------------------------------

mod e23_fuse_three_generated_cards_into_the_hand_classic_plus_43 {
    use super::*;

    #[test]
    fn r77_three_random_picks_of_the_pool_repeats_allowed_fused_with_no_target_a_token_at_0_in_the_hand() {
        let mut start = playing("fuse-slop");
        let spell = hand_card(&mut start.state, &slop.id, P1);
        let hand_before = start.state.players.p1.hand.len();
        let mut run1 = frozen(&start);
        run1 = act(
            &run1,
            input(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })),
        );

        let hand = &run1.state.players.p1.hand;
        assert_eq!(hand.len(), hand_before);
        let made = must(hand.last().cloned(), "the fused card");
        let def = def_of(Some(&run1.state), &made.def_id).clone();
        assert_eq!(def.ingredients.as_ref().map(Vec::len), Some(3));
        let generated = [ai_unit.id.clone(), ai_spell.id.clone()];
        assert!(
            def.ingredients
                .as_ref()
                .is_some_and(|list| list.iter().all(|entry| generated.contains(&entry.def_id)))
        );
        assert!(def.token);
        let types: Vec<Value> = def
            .ingredients
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|entry| json_of(def_of(Some(&run1.state), &entry.def_id).type_))
            .collect();
        // R102: the shared type, else the first pick's — the first pick's either way.
        assert_eq!(Some(json_of(def.type_)), types.first().cloned());
        assert_eq!(made.cost_override, Some(0));
        assert!(!made.radiant);
        assert_eq!(hash_state(&replayed(&run1)), hash_state(&run1.state));
    }

    #[test]
    fn r469_the_radiant_face_fuses_radiant_ai_cards_fewer_than_two_picks_or_an_empty_pool_fuses_and_draws_nothing()
     {
        let mut start = playing("fuse-slop-radiant");
        let spell = hand_card(&mut start.state, &slop.id, P1);
        live_mut(&mut start.state, &spell.id).radiant = true;
        let mut run1 = frozen(&start);
        run1 = act(
            &run1,
            input(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })),
        );
        let made = must(run1.state.players.p1.hand.last().cloned(), "the fused card");
        assert!(
            def_of(Some(&run1.state), &made.def_id)
                .ingredients
                .as_ref()
                .is_some_and(|list| list.iter().all(|entry| entry.radiant == Some(true)))
        );
        // Every ingredient Radiant and no kept card: the fused card is Radiant too.
        assert!(made.radiant);

        let mut sink = Sink::for_state(&start.state);
        let cursor = sink.rng.cursor();
        run(
            &mut start.state,
            &mut sink,
            fuse_generated(json_as(
                json!({ "count": 1, "query": { "defId": [ai_unit.id.clone()] } }),
            )),
            None,
        );
        run(
            &mut start.state,
            &mut sink,
            fuse_generated(json_as(json!({ "count": 3, "query": { "tags": ["Pancake"] } }))),
            None,
        );
        assert_eq!(sink.rng.cursor(), cursor);
        assert!(sink.events.is_empty());
    }
}

// ---------------------------------------------------------------------------
// An enemy card onto one of yours of its type (Classic #78 Mutate Spell, Radiant).
// ---------------------------------------------------------------------------

struct MutateBoard {
    run: Run,
    enemy: CardInstance,
    field: CardInstance,
    in_hand: CardInstance,
    deck: Vec<CardInstance>,
}

fn mutate_board(seed: &str) -> MutateBoard {
    let mut start = playing(seed);
    let enemy = put(&mut start.state, &fuse_a.id, slot(P2, Row::Units, 1), json!({}));
    let field = put(&mut start.state, &body.id, slot(P1, Row::Units, 1), json!({}));
    put(
        &mut start.state,
        &immutable.id,
        slot(P1, Row::Units, 2),
        json!({}),
    );
    start.state.players.p1.hand = vec![];
    let in_hand = hand_card(&mut start.state, &fuse_b.id, P1);
    hand_card(&mut start.state, &slop.id, P1); // a Spell, not of the enemy card's type
    let deck = set_library(
        &mut start.state,
        P1,
        &[felinor_c.id.clone(), lab.id.clone(), felinor_a.id.clone()],
    );
    MutateBoard {
        run: start,
        enemy,
        field,
        in_hand,
        deck,
    }
}

mod e23_fuse_an_enemy_card_onto_one_of_yours_of_its_type_classic_78_radiant {
    use super::*;

    #[test]
    fn r470_the_controller_picks_among_their_cards_of_its_type_on_the_field_in_hand_and_in_deck_the_answer_fuses_it_there()
     {
        let MutateBoard {
            run: mut start,
            enemy,
            field,
            in_hand,
            deck,
        } = mutate_board("fuse-mutate");
        let spell = hand_card(&mut start.state, &mutate.id, P1);
        let mut run1 = frozen(&start);
        let enemy_hero = run1.state.players.p2.hero.health;
        run1 = act(
            &run1,
            input(
                json!({ "type": "play", "instanceId": spell.id, "targets": [pick(&enemy.id)], "playerId": "p1" }),
            ),
        );

        let pending = must(run1.state.pending.clone(), "the pick");
        assert_eq!(pending.player_id, P1);
        // Field in lane order, hand in hand order, the deck's Units in an order of their own (name).
        let felinor_a_in_deck = must(deck.get(2).cloned(), "felinor-a");
        let felinor_c_in_deck = must(deck.first().cloned(), "felinor-c");
        assert_eq!(
            pending
                .options
                .iter()
                .map(|option| option.selection.clone())
                .collect::<Vec<_>>(),
            vec![
                pick(&field.id),
                pick(&in_hand.id),
                pick(&felinor_a_in_deck.id),
                pick(&felinor_c_in_deck.id),
            ]
        );
        // The rest of the Spell waits for the answer (R113).
        assert_eq!(run1.state.players.p2.hero.health, enemy_hero);

        // The other player sees an open prompt, nothing more; the chooser sees its deck cards (§10.8).
        assert_eq!(
            json_of(view_for(&run1.state, P2))["pending"],
            json!({ "forYou": false, "pendingFor": "p1" })
        );
        assert!(
            !serde_json::to_string(&view_for(&run1.state, P2))
                .expect("a view serialises")
                .contains(&felinor_a_in_deck.id)
        );
        let mine = json_of(view_for(&run1.state, P1))["pending"].clone();
        assert_eq!(mine["forYou"], json!(true), "expected p1's prompt");
        let offered = mine["options"]
            .as_array()
            .and_then(|options| {
                options
                    .iter()
                    .find(|option| option["instanceId"] == json!(felinor_a_in_deck.id))
            })
            .map(|option| option["defId"].clone());
        assert_eq!(offered, Some(json!(felinor_a.id.clone())));

        // JSON mid-prompt, and the answer: onto the deck card, which stays in the deck.
        let round = round_trip(&run1.state);
        assert_eq!(round, run1.state);
        let live_run = answer(&run1, pick(&felinor_a_in_deck.id), None);
        let from_json = answer(&with_state(&run1, round), pick(&felinor_a_in_deck.id), None);
        assert_eq!(hash_state(&from_json.state), hash_state(&live_run.state));
        assert_eq!(hash_state(&replayed(&live_run)), hash_state(&live_run.state));

        let kept = must(
            find_instance(&live_run.state, &felinor_a_in_deck.id).cloned(),
            "the deck card",
        );
        assert_eq!(kept.zone, Zone::Library { player: P1 });
        assert_eq!(
            fused_id_parts(Some(&live_run.state), &kept.def_id),
            Some(vec![fuse_a.id.clone(), felinor_a.id.clone()])
        );
        assert!(find_instance(&live_run.state, &enemy.id).is_none());
        assert!(live_run.state.players.p2.units[0].is_none());
        assert_eq!(live_run.state.players.p2.hero.health, enemy_hero - 1);
    }

    #[test]
    fn r470_onto_a_field_card_it_is_r77s_target_a_trap_counts_as_a_field_trap_with_none_of_its_type_the_card_is_exiled()
     {
        let mut start = playing("fuse-mutate-trap");
        let enemy_trap = put(
            &mut start.state,
            &plain_trap.id,
            slot(P2, Row::Backrow, 2),
            json!({}),
        );
        let mine_trap = put(
            &mut start.state,
            &field_trap.id,
            slot(P1, Row::Backrow, 4),
            json!({}),
        );
        let spell = hand_card(&mut start.state, &mutate.id, P1);
        let mut run1 = frozen(&start);
        run1 = act(
            &run1,
            input(
                json!({ "type": "play", "instanceId": spell.id, "targets": [pick(&enemy_trap.id)], "playerId": "p1" }),
            ),
        );
        assert_eq!(
            run1.state.pending.as_ref().map(|pending| pending
                .options
                .iter()
                .map(|option| option.selection.clone())
                .collect::<Vec<_>>()),
            Some(vec![pick(&mine_trap.id)])
        );
        run1 = answer(&run1, pick(&mine_trap.id), None);
        let kept = must(
            find_instance(&run1.state, &mine_trap.id).cloned(),
            "the Field Trap",
        );
        assert_eq!(
            json_of(def_of(Some(&run1.state), &kept.def_id).type_),
            json!("Field Trap")
        );
        assert_eq!(
            kept.zone,
            Zone::Field {
                player: P1,
                row: Row::Backrow,
                lane: 4,
            }
        );

        let mut lonely = playing("fuse-mutate-none");
        let target = put(&mut lonely.state, &fuse_a.id, slot(P2, Row::Units, 3), json!({}));
        lonely.state.players.p1.library = vec![];
        lonely.state.players.p1.hand = vec![];
        let again = hand_card(&mut lonely.state, &mutate.id, P1);
        let mut run2 = frozen(&lonely);
        run2 = act(
            &run2,
            input(
                json!({ "type": "play", "instanceId": again.id, "targets": [pick(&target.id)], "playerId": "p1" }),
            ),
        );
        assert!(run2.state.pending.is_none());
        assert_eq!(ids(&run2.state.players.p2.exile), vec![target.id.clone()]);
    }
}

// ---------------------------------------------------------------------------
// Discover and fuse onto self (Classic+ #30 Felinor Fuser).
// ---------------------------------------------------------------------------

mod e23_discover_twice_and_fuse_both_onto_this_classic_plus_30 {
    use super::*;

    #[test]
    fn r77_two_chained_discovers_of_felinor_units_never_the_fuser_itself_b4_1_fused_onto_the_fuser_on_the_field()
     {
        let mut start = playing("fuse-felinor");
        let card = hand_card(&mut start.state, &fuser.id, P1);
        let mut run1 = frozen(&start);
        run1 = act(
            &run1,
            input(json!({ "type": "play", "instanceId": card.id, "playerId": "p1" })),
        );

        let first = must(run1.state.pending.clone(), "the first Discover");
        let offered: Vec<String> = first
            .options
            .iter()
            .map(|option| match &option.selection {
                Selection::Mode { option } => option.clone(),
                _ => String::new(),
            })
            .collect();
        assert!(!offered.contains(&fuser.id));
        let felinors = [felinor_a.id.clone(), felinor_b.id.clone(), felinor_c.id.clone()];
        assert!(offered.iter().all(|id| felinors.contains(id)));
        assert_eq!(
            json_of(view_for(&run1.state, P2))["pending"],
            json!({ "forYou": false, "pendingFor": "p1" })
        );

        let first_pick = must(first.options.first().cloned(), "an option").selection;
        run1 = answer(&run1, first_pick, None);
        let second = must(run1.state.pending.clone(), "the second Discover");
        let second_pick = must(second.options.get(1).cloned(), "an option").selection;
        run1 = answer(&run1, second_pick, None);

        let kept = must(find_instance(&run1.state, &card.id).cloned(), "the Fuser");
        assert!(matches!(kept.zone, Zone::Field { .. }));
        let parts = must(fused_id_parts(Some(&run1.state), &kept.def_id), "a fused id");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[2], fuser.id.clone());
        let summed: i32 = parts
            .iter()
            .map(|id| def_of(Some(&run1.state), id).base.attack.unwrap_or(0))
            .sum();
        assert_eq!(unit_view(&run1.state, &kept).attack, summed);
        assert_eq!(hash_state(&replayed(&run1)), hash_state(&run1.state));
    }
}

/// R1087 (M #75): a keyword a fusion brings in applies at once — a fused "Animated on your turn"
/// animates the kept card at its controller's next start of turn, while a fused plain Animated
/// waits until the card enters the field again.
mod r1087_a_fused_animated_keyword_applies_at_once {
    use super::*;
    use jackioh_engine::animated::animate_at_turn_start;
    use jackioh_engine::effects::move_::bounce_card;

    /// A plain Field Spell keeper (2/3), an "Animated on your turn" Field Spell (4/5) and a plain
    /// Animated Field Spell (1/1).
    fn animated_defs() -> Vec<CardDef> {
        let def = |id: &str, index: i32, attack: i32, health: i32, keywords: Value| -> CardDef {
            json_as(json!({
                "id": id,
                "index": index.to_string(),
                "name": format!("{id} (animated fusion)"),
                "set": "Core",
                "type": "Field Spell",
                "tags": [],
                "rarity": "Common",
                "token": false,
                "cost": 1,
                "base": { "attack": attack, "health": health, "keywords": keywords, "text": id },
                "radiant": {
                    "attack": attack * 2,
                    "health": health * 2,
                    "keywords": keywords,
                    "text": id,
                },
            }))
        };
        vec![
            def("fv-keep", 1971, 2, 3, json!([])),
            def(
                "fv-turn",
                1972,
                4,
                5,
                json!([{ "kind": "Animated on your turn" }, { "kind": "Rush" }]),
            ),
            def("fv-plain", 1973, 1, 1, json!([{ "kind": "Animated" }])),
        ]
    }

    /// `playing` with the fusion-Animated defs registered (default scripts: no hooks).
    fn animated_game(seed: &str) -> Run {
        let run = playing(seed);
        let mut catalog = registered_catalog().clone();
        let mut registry = registered_scripts().clone();
        for def in animated_defs() {
            registry.insert(
                def.id.clone(),
                CardScripts {
                    base: Script::default(),
                    radiant: Script::default(),
                },
            );
            catalog.insert(def.id.clone(), def);
        }
        register_catalog(catalog);
        register_scripts(registry);
        run
    }

    fn animated_events(events: &[GameEvent]) -> Vec<GameEvent> {
        events
            .iter()
            .filter(|event| event.event_type() == GameEventType::Animated)
            .cloned()
            .collect()
    }

    #[test]
    fn r1087_fused_animated_on_your_turn_animates_next_start_of_turn() {
        let mut run = animated_game("fused-turn");
        let keep = put(&mut run.state, "fv-keep", slot(P1, Row::Backrow, 1), json!({}));
        let turn = put(&mut run.state, "fv-turn", slot(P1, Row::Backrow, 2), json!({}));
        let mut sink = Sink::for_state(&run.state);
        let kept = must(
            fuse_in(
                &mut run.state,
                &mut sink,
                json!({ "ingredients": [turn], "target": keep }),
            ),
            "the fusion",
        );

        // The fusion brings the keyword in but animates nothing yet: the keeper stays backrow.
        assert_eq!(
            card_at(&run.state, slot(P1, Row::Backrow, 1)).map(|card| card.id.clone()),
            Some(kept.id.clone())
        );
        assert!(!is_animated(&run.state, &kept));
        assert!(animated_events(&sink.events).is_empty());

        // At its controller's next start of turn it animates, with the summed stats.
        let mut sink = Sink::for_state(&run.state);
        animate_at_turn_start(&mut sink.on(&mut run.state), P1);
        let kept = must(find_instance(&run.state, &kept.id).cloned(), "the keeper");
        assert!(is_animated(&run.state, &kept));
        assert_eq!(animated_events(&sink.events).len(), 1);
        assert_eq!(unit_view(&run.state, &kept).attack, 6);
        assert_eq!(unit_view(&run.state, &kept).health, 8);
    }

    #[test]
    fn r1087_fused_plain_animated_waits_for_entry() {
        let mut run = animated_game("fused-plain");
        let keep = put(&mut run.state, "fv-keep", slot(P1, Row::Backrow, 1), json!({}));
        let plain = put(&mut run.state, "fv-plain", slot(P1, Row::Backrow, 2), json!({}));
        let mut sink = Sink::for_state(&run.state);
        let kept = must(
            fuse_in(
                &mut run.state,
                &mut sink,
                json!({ "ingredients": [plain], "target": keep }),
            ),
            "the fusion",
        );

        // A fused plain Animated waits until the card enters the field again: no animation now,
        // and none at the next start of turn either.
        assert!(!is_animated(&run.state, &kept));
        assert!(animated_events(&sink.events).is_empty());
        let mut sink = Sink::for_state(&run.state);
        animate_at_turn_start(&mut sink.on(&mut run.state), P1);
        let kept = must(find_instance(&run.state, &kept.id).cloned(), "the keeper");
        assert!(!is_animated(&run.state, &kept));
        assert!(animated_events(&sink.events).is_empty());

        // Bounced and played again, it enters animated.
        let kept_id = kept.id.clone();
        let mut sink = Sink::for_state(&run.state);
        let bounce = Effect::new("bounce-back", move |ctx| {
            if let Some(card) = find_instance(ctx.state, &kept_id).cloned() {
                bounce_card(&mut ctx.sink, &card);
            }
        });
        run(&mut run.state, &mut sink, bounce, None);
        run = act(
            &run,
            json!({
                "type": "play",
                "instanceId": kept.id,
                "zone": { "row": "backrow", "lane": 1 },
                "playerId": "p1",
            }),
        );
        let kept = must(find_instance(&run.state, &kept.id).cloned(), "the keeper");
        assert!(is_animated(&run.state, &kept));
    }
}
