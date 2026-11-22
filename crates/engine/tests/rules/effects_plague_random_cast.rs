//! R452 meets E19 (R471, R689): "Place N Plague Counters" inside a random cast (Classic+ #47 Jogg's Box
//! casting Classic #70 Book of Plague) asks its caster nothing — every placement goes on one random
//! permanent — as a random cast makes every choice at random. Through fixture cards; the real cards'
//! tests cover the same case again.

use jackioh_engine::testkit::*;

use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::effects::{
    CastRandomArgs, CastRandomCount, CastRandomQuery, cast_random, place_plague_tokens,
};
use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{CardScripts, EngineSink, Script, hook};
use jackioh_engine::scripts::registered_scripts;
use jackioh_engine::state::{GameState, new_instance};

use super::fixtures::harness::{new_game, put, set_library, slot};

fn spell(id: &str) -> CardDef {
    json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

fn plague_spell() -> CardDef {
    spell("pr-plague-fixture")
}

fn box_() -> CardDef {
    spell("pr-box-fixture")
}

const TOKENS: i32 = 3;

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in [plague_spell(), box_()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let placing = Script {
        cry: Some(hook(|_ctx| {
            vec![place_plague_tokens(json_as(json!({ "count": TOKENS })))]
        })),
        ..Script::default()
    };
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        plague_spell().id,
        CardScripts {
            base: placing.clone(),
            radiant: placing,
        },
    );
    register_scripts(scripts);
    state.turn = 4;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    set_library(&mut state, PlayerId::P1, &[] as &[&str]);
    state
}

fn cast_from_box(state: &mut GameState) {
    let self_ = new_instance(
        state,
        &box_().id,
        PlayerId::P1,
        Zone::Resolving { player: PlayerId::P1 },
    );
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            Some(&self_),
            HookOptions {
                controller: Some(PlayerId::P1),
                ..Default::default()
            },
        );
        apply_effects(
            &[cast_random(CastRandomArgs {
                query: CastRandomQuery::Fixed(json_as(json!({ "defId": plague_spell().id }))),
                count: Some(CastRandomCount::Fixed(1)),
                radiant: None,
                target_enemies: None,
                afterward: None,
            })],
            &mut ctx,
        );
    }
    state.rng_cursor = rng.cursor();
}

fn tokens_on_field(state: &GameState) -> i32 {
    [&state.players.p1, &state.players.p2]
        .iter()
        .flat_map(|side| side.units.iter().flatten().flatten())
        .map(|card| card.counters.plague.unwrap_or(0))
        .sum()
}

mod r452_r471_placements_inside_a_random_cast {
    use super::*;

    #[test]
    fn r452_r689_its_caster_is_never_asked_every_placement_lands_on_one_random_permanent_at_once() {
        let mut state = game("plague-random-cast");
        put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({}));
        cast_from_box(&mut state);
        assert!(state.pending.is_none());
        assert_eq!(tokens_on_field(&state), TOKENS);
    }

    #[test]
    fn r129_with_no_permanent_on_the_field_the_placements_fizzle_and_nothing_is_asked() {
        let mut state = game("plague-random-empty");
        cast_from_box(&mut state);
        assert!(state.pending.is_none());
        assert_eq!(tokens_on_field(&state), 0);
    }

    #[test]
    fn r471_outside_a_random_cast_the_same_spell_still_asks_its_caster() {
        let mut state = game("plague-asked");
        put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        let self_ = new_instance(
            &mut state,
            &plague_spell().id,
            PlayerId::P1,
            Zone::Resolving { player: PlayerId::P1 },
        );
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut events = Vec::new();
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                Some(&self_),
                HookOptions {
                    controller: Some(PlayerId::P1),
                    ..Default::default()
                },
            );
            apply_effects(
                &[place_plague_tokens(json_as(json!({ "count": TOKENS })))],
                &mut ctx,
            );
        }
        assert_eq!(
            state.pending.as_ref().map(|pending| pending.player_id),
            Some(PlayerId::P1)
        );
    }
}

/// A permanent put on the field Dormant, stamped Slumber by hand (white-box, as `board_geometry.rs`).
fn put_dormant(state: &mut GameState, def_id: &str, at: ZoneSlot) -> String {
    let card = put(state, def_id, at, json!({}));
    find_instance_mut(state, &card.id).expect("the permanent").slumber = Some(Slumber { turns: Some(2) });
    card.id
}

fn plague_of(state: &GameState, id: &str) -> i32 {
    find_instance(state, id)
        .and_then(|card| card.counters.plague)
        .unwrap_or(0)
}

/// R4322000: a placement drawn at random draws only among the permanents an effect reaches, so it
/// never lands on a Dormant card (§6.1 Dormant, R1839000), as a placement its caster chooses already
/// does not. A Mayor's placement (R1200) takes the same random branch over the same pool.
mod r4322000_a_random_placement_never_lands_on_a_dormant_permanent {
    use super::*;

    #[test]
    fn r4322000_beside_an_awake_permanent_every_placement_lands_on_the_awake_one() {
        for n in 0..16 {
            let mut state = game(&format!("pr-dormant-{n}"));
            let awake = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({})).id;
            let dormant = put_dormant(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1));
            cast_from_box(&mut state);
            assert!(state.pending.is_none(), "seed {n}");
            assert_eq!(
                plague_of(&state, &dormant),
                0,
                "seed {n}: none on the Dormant card"
            );
            assert_eq!(
                plague_of(&state, &awake),
                TOKENS,
                "seed {n}: all on the awake one"
            );
        }
    }

    #[test]
    fn r4322000_with_only_dormant_permanents_the_placements_fizzle() {
        let mut state = game("pr-dormant-only");
        put_dormant(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1));
        put_dormant(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1));
        cast_from_box(&mut state);
        assert!(state.pending.is_none());
        assert_eq!(tokens_on_field(&state), 0);
    }
}

fn immune(state: &mut GameState, id: &str) {
    find_instance_mut(state, id)
        .expect("the permanent")
        .granted_keywords
        .push(Keyword::ImmuneToSpells);
}

/// `place_plague_tokens` run straight, outside a random cast, by `self_` for P1 (R471's case): the
/// placement prompt it opens, if any.
fn place_by(state: &mut GameState, self_: &CardInstance) -> Option<PendingChoice> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            Some(self_),
            HookOptions {
                controller: Some(PlayerId::P1),
                ..Default::default()
            },
        );
        apply_effects(
            &[place_plague_tokens(json_as(json!({ "count": TOKENS })))],
            &mut ctx,
        );
    }
    state.pending.clone()
}

fn offered(pending: &PendingChoice) -> Vec<String> {
    pending
        .options
        .iter()
        .filter_map(|option| match &option.selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

/// R4453000: a Spell's placement, drawn at random or chosen, passes a Unit Immune to Spells by (§6.1:
/// "Spells can't target it or affect it"), as R4322000's passes a Dormant card by; a Unit's still
/// reaches it.
mod r4453000_a_spells_placement_passes_a_unit_immune_to_spells_by {
    use super::*;

    #[test]
    fn r4453000_a_random_placement_lands_on_the_other_permanent() {
        for n in 0..16 {
            let mut state = game(&format!("pr-immune-{n}"));
            let open = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({})).id;
            let warded = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
            immune(&mut state, &warded);
            cast_from_box(&mut state);
            assert!(state.pending.is_none(), "seed {n}");
            assert_eq!(plague_of(&state, &warded), 0, "seed {n}: none on the immune Unit");
            assert_eq!(plague_of(&state, &open), TOKENS, "seed {n}: all on the other one");
        }
    }

    #[test]
    fn r4453000_with_only_an_immune_unit_the_placements_fizzle() {
        let mut state = game("pr-immune-only");
        let warded = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
        immune(&mut state, &warded);
        cast_from_box(&mut state);
        assert!(state.pending.is_none());
        assert_eq!(tokens_on_field(&state), 0);
    }

    #[test]
    fn r4453000_a_spells_chosen_placement_offers_no_immune_unit() {
        // The Spell is in no zone here, so the targeting point (R450), which finds a prompt's source by
        // its id, has no Spell to judge by: the placement's own pool leaves the immune Unit out.
        let mut state = game("pr-immune-asked");
        let open = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({})).id;
        let warded = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
        immune(&mut state, &warded);
        let me = new_instance(
            &mut state,
            &plague_spell().id,
            PlayerId::P1,
            Zone::Resolving { player: PlayerId::P1 },
        );
        let pending = place_by(&mut state.clone(), &me).expect("the placement prompt");
        assert_eq!(offered(&pending), vec![open.clone()]);

        // With only the immune Unit standing, nothing is asked and nothing placed (R129).
        let mut alone = game("pr-immune-asked-alone");
        let warded = put(&mut alone, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
        immune(&mut alone, &warded);
        let me = new_instance(
            &mut alone,
            &plague_spell().id,
            PlayerId::P1,
            Zone::Resolving { player: PlayerId::P1 },
        );
        assert!(place_by(&mut alone, &me).is_none());
        assert_eq!(tokens_on_field(&alone), 0);
    }

    #[test]
    fn r4453000_a_units_placement_still_offers_it() {
        let mut state = game("pr-immune-unit");
        let open = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        let warded = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
        immune(&mut state, &warded);
        let me = find_instance(&state, &open.id).cloned().expect("the unit");
        let pending = place_by(&mut state, &me).expect("the placement prompt");
        assert_eq!(offered(&pending), vec![open.id.clone(), warded]);
    }
}
