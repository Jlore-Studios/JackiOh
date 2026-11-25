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

use super::fixtures::harness::{in_hand, new_game, put, set_library, slot};
use super::fixtures::mb22 as fx;

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
    for def in [plague_spell(), box_(), field_spell()] {
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

/// A Field Spell with no script, a placement's source in the backrow (R4353002 bars its aim as a Spell's).
fn field_spell() -> CardDef {
    json_as(json!({
        "id": "pr-field-fixture",
        "index": "pr-field-fixture",
        "name": "pr-field-fixture",
        "set": "Core",
        "type": "Field Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": "pr-field-fixture" },
        "radiant": { "keywords": [], "text": "pr-field-fixture" },
    }))
}

fn evasive(state: &mut GameState, id: &str) {
    find_instance_mut(state, id)
        .expect("the permanent")
        .granted_keywords
        .push(Keyword::Evasive);
}

/// `game` with Meditative batch 22's Mayor registered beside the plague fixtures (R1200).
fn mayor_game(seed: &str) -> GameState {
    let state = game(seed);
    fx::register();
    state
}

/// `place_plague_tokens` run straight by `self_` for `controller`: how many rolls it drew.
fn placement_draws(state: &mut GameState, self_: &CardInstance, controller: PlayerId) -> u32 {
    let before = state.rng_cursor;
    let mut rng = Rng::new(&state.seed, before);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            Some(self_),
            HookOptions {
                controller: Some(controller),
                ..Default::default()
            },
        );
        apply_effects(
            &[place_plague_tokens(json_as(json!({ "count": TOKENS })))],
            &mut ctx,
        );
    }
    state.rng_cursor = rng.cursor();
    state.rng_cursor - before
}

fn on_p2(state: &GameState) -> bool {
    state
        .players
        .p2
        .units
        .iter()
        .flatten()
        .flatten()
        .any(|card| card.counters.plague.unwrap_or(0) > 0)
}

/// A Mayor acting at P1's unit zone 3 (R1200), an open `fx-1` at P1's zone 1 and an Evasive `fx-2` at
/// P2's zone 1: the game, the Mayor's id, the open card's and the Evasive one's.
fn mayor_board(seed: &str) -> (GameState, String, String, String) {
    let mut state = mayor_game(seed);
    let mayor = put(
        &mut state,
        &fx::mayor.id,
        slot(PlayerId::P1, Row::Units, 3),
        json!({}),
    )
    .id;
    let open = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({})).id;
    let ghost = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
    evasive(&mut state, &ghost);
    (state, mayor, open, ghost)
}

/// A Mayor (Radiant or not) at P1's unit zone 1 and three permanents, two on each side: how many rolls
/// `caster`'s Spell placement drew under it, and whether it landed on P2's side.
fn placement_under_mayor(seed: &str, radiant: bool, caster: PlayerId) -> (u32, bool) {
    let mut state = mayor_game(seed);
    put(
        &mut state,
        &fx::mayor.id,
        slot(PlayerId::P1, Row::Units, 1),
        json!({ "radiant": radiant }),
    );
    put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 2), json!({}));
    put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({}));
    put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 2), json!({}));
    let spell = in_hand(&mut state, &plague_spell().id, caster, 1).remove(0);
    (placement_draws(&mut state, &spell, caster), on_p2(&state))
}

/// R4353002: "Spells and Field Spells can't target it". A placement nobody is asked for, a random cast's
/// (R452) or any while a Mayor acts (R1200), draws from the options its prompt would offer (R450), so a
/// Spell's or a Field Spell's never lands on an Evasive card in play, and the draw is recorded for I22
/// (#4540). A Unit's placement still reaches it, and a Spell's chosen placement never offers it.
mod r4353002_a_placement_nobody_is_asked_for_never_lands_on_an_evasive_permanent {
    use super::*;

    #[test]
    fn r4353002_a_random_casts_placement_lands_on_the_other_permanent() {
        for n in 0..16 {
            let mut state = game(&format!("pr-evasive-{n}"));
            let open = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({})).id;
            let ghost = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
            evasive(&mut state, &ghost);
            cast_from_box(&mut state);
            assert!(state.pending.is_none(), "seed {n}");
            assert_eq!(plague_of(&state, &ghost), 0, "seed {n}: none on the Evasive card");
            assert_eq!(plague_of(&state, &open), TOKENS, "seed {n}: all on the other one");
        }
    }

    #[test]
    fn r4353002_r129_with_only_an_evasive_permanent_a_random_casts_placements_fizzle() {
        let mut state = game("pr-evasive-only");
        let ghost = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
        evasive(&mut state, &ghost);
        start_target_draws();
        cast_from_box(&mut state);
        let draws = take_target_draws();
        assert!(state.pending.is_none());
        assert_eq!(tokens_on_field(&state), 0);
        assert!(draws.is_empty(), "nothing drawn: {}", draws.len());
    }

    #[test]
    fn r4353002_r1200_under_a_mayor_a_spells_placement_never_lands_on_an_evasive_permanent() {
        for n in 0..16 {
            let (mut state, mayor, open, ghost) = mayor_board(&format!("pr-mayor-evasive-{n}"));
            let spell = in_hand(&mut state, &plague_spell().id, PlayerId::P1, 1).remove(0);
            placement_draws(&mut state, &spell, PlayerId::P1);
            assert!(state.pending.is_none(), "seed {n}");
            assert_eq!(plague_of(&state, &ghost), 0, "seed {n}: none on the Evasive card");
            assert_eq!(
                plague_of(&state, &open) + plague_of(&state, &mayor),
                TOKENS,
                "seed {n}: all on one of the others"
            );
        }
    }

    #[test]
    fn r4353002_r1200_under_a_mayor_a_field_spells_placement_never_lands_on_an_evasive_permanent() {
        for n in 0..24 {
            let (mut state, mayor, open, ghost) = mayor_board(&format!("pr-mayor-field-{n}"));
            let field = put(
                &mut state,
                &field_spell().id,
                slot(PlayerId::P1, Row::Backrow, 1),
                json!({}),
            );
            placement_draws(&mut state, &field, PlayerId::P1);
            assert!(state.pending.is_none(), "seed {n}");
            assert_eq!(plague_of(&state, &ghost), 0, "seed {n}: none on the Evasive card");
            // The Field Spell is a permanent on the field, so it may take them itself.
            assert_eq!(
                plague_of(&state, &open) + plague_of(&state, &mayor) + plague_of(&state, &field.id),
                TOKENS,
                "seed {n}: all on one of the others"
            );
        }
    }

    #[test]
    fn r4353002_r1200_a_units_placement_under_a_mayor_still_reaches_an_evasive_permanent() {
        let mut reached = false;
        for n in 0..16 {
            let (mut state, _mayor, open, ghost) = mayor_board(&format!("pr-mayor-unit-{n}"));
            let me = find_instance(&state, &open).cloned().expect("the unit");
            placement_draws(&mut state, &me, PlayerId::P1);
            assert!(state.pending.is_none(), "seed {n}");
            assert_eq!(tokens_on_field(&state), TOKENS, "seed {n}: every placement lands");
            reached |= plague_of(&state, &ghost) == TOKENS;
        }
        assert!(
            reached,
            "a Unit's placement reaches the Evasive card on some seed"
        );
    }

    #[test]
    fn r4353002_a_spells_chosen_placement_offers_no_evasive_permanent_and_a_units_does() {
        let mut state = game("pr-evasive-asked");
        let open = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        let ghost = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
        evasive(&mut state, &ghost);
        let spell = in_hand(&mut state, &plague_spell().id, PlayerId::P1, 1).remove(0);
        let pending = place_by(&mut state.clone(), &spell).expect("the Spell's placement prompt");
        assert_eq!(offered(&pending), vec![open.id.clone()]);
        let me = find_instance(&state, &open.id).cloned().expect("the unit");
        let pending = place_by(&mut state, &me).expect("the Unit's placement prompt");
        assert_eq!(offered(&pending), vec![open.id.clone(), ghost]);

        // With only the Evasive card standing, a Spell asks nothing and places nothing (R129).
        let mut alone = game("pr-evasive-asked-alone");
        let ghost = put(&mut alone, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
        evasive(&mut alone, &ghost);
        let spell = in_hand(&mut alone, &plague_spell().id, PlayerId::P1, 1).remove(0);
        assert!(place_by(&mut alone, &spell).is_none());
        assert_eq!(tokens_on_field(&alone), 0);
    }

    #[test]
    fn r4353002_r452_a_random_casts_placement_draw_is_recorded_for_i22_and_lawful() {
        let mut state = game("pr-evasive-i22");
        let open = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({})).id;
        let ghost = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({})).id;
        evasive(&mut state, &ghost);
        start_target_draws();
        cast_from_box(&mut state);
        let draws = take_target_draws();
        assert_eq!(draws.len(), 1, "one draw: the placement's");
        let draw = &draws[0];
        assert_eq!(draw.by, TargetDrawBy::RandomCast);
        assert_eq!(draw.seat, PlayerId::P1);
        assert_eq!(
            draw.source.as_ref().map(|card| card.def_id.clone()),
            Some(plague_spell().id)
        );
        assert_eq!(draw.picks, vec![Selection::Instance { instance_id: open }]);
        assert_eq!(target_draw_findings(draw), Vec::<String>::new());
    }

    #[test]
    fn r4353002_r1200_a_mayors_placement_draw_is_recorded_for_i22_and_lawful() {
        for n in 0..8 {
            let (mut state, _mayor, _open, ghost) = mayor_board(&format!("pr-mayor-i22-{n}"));
            let spell = in_hand(&mut state, &plague_spell().id, PlayerId::P1, 1).remove(0);
            start_target_draws();
            placement_draws(&mut state, &spell, PlayerId::P1);
            let draws = take_target_draws();
            assert_eq!(draws.len(), 1, "seed {n}: one draw, the placement's");
            let draw = &draws[0];
            assert_eq!(draw.by, TargetDrawBy::Mayor, "seed {n}");
            assert_eq!(
                draw.source.as_ref().map(|card| card.def_id.clone()),
                Some(plague_spell().id),
                "seed {n}"
            );
            assert!(
                !draw.picks.contains(&Selection::Instance { instance_id: ghost }),
                "seed {n}: the draw never names the Evasive card"
            );
            assert_eq!(target_draw_findings(draw), Vec::<String>::new(), "seed {n}");
        }
    }
}

/// R1201: the Radiant Mayor's Lucky 1 rolls its controller's random placement again, as it does their
/// other random targets, and keeps the later roll only when it lands on the enemy side (R656: an unaimed
/// pick harms). The opponent's placement under the same Mayor rolls once.
mod r1201_the_radiant_mayors_lucky_rolls_a_placement_again {
    use super::*;

    #[test]
    fn r1201_lucky_keeps_the_enemy_side_and_the_opponent_rolls_once() {
        // Four permanents, two on each side. The plain and the Radiant Mayor's games share their first
        // roll, so Lucky 1 is one more roll, kept only when it lands on the enemy side where the first
        // did not: never worse, and better over enough seeds.
        let (mut plain_hits, mut lucky_hits) = (0, 0);
        for n in 0..40 {
            let seed = format!("pr-mayor-lucky-{n}");
            let (plain_draws, plain_enemy) = placement_under_mayor(&seed, false, PlayerId::P1);
            let (lucky_draws, lucky_enemy) = placement_under_mayor(&seed, true, PlayerId::P1);
            assert!(lucky_draws > plain_draws, "{seed}: Lucky rolls again");
            assert!(!plain_enemy || lucky_enemy, "{seed}: a kept roll is never worse");
            plain_hits += i32::from(plain_enemy);
            lucky_hits += i32::from(lucky_enemy);

            // The opponent's placement under the same Mayor rolls once: Lucky is its controller's alone.
            let (theirs_plain, _) = placement_under_mayor(&seed, false, PlayerId::P2);
            let (theirs_lucky, _) = placement_under_mayor(&seed, true, PlayerId::P2);
            assert_eq!(theirs_lucky, theirs_plain, "{seed}: the opponent rolls once");
        }
        assert!(
            lucky_hits > plain_hits,
            "Lucky {lucky_hits} vs plain {plain_hits} of 40"
        );
    }
}
