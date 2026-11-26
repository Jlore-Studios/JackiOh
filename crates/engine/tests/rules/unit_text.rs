//! Copy a Unit's text (`subsystems/unit_text.rs`, R1850000; Stinkyscape #20 Tnack Trav): the copier
//! runs the text the target runs now, on its own stats, and leaving the field ends it. Through the
//! fixtures in `fixtures/card_on_card.rs` (#2273); the real card's tests cover the same cases again.

use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::P1;

use super::fixtures::card_on_card::{
    LEGEND_PING, LEGEND_RADIANT_PING, apply, at, cheap, copier, destroy, has, immutable_copier, instance,
    legend, on_field, pass, play, playing, radiant_copier, top,
};
use super::fixtures::harness::{act_ok, events_of_type, put, slot};

mod r1850000_copy_a_units_text {
    use super::*;

    #[test]
    fn r1850000_has_the_units_keywords_aura_and_triggers_on_its_own_stats() {
        let mut state = playing("copy");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let plain = put(&mut state, &cheap().id, slot(P1, Row::Units, 3), json!({}));
        let played = play(&state, P1, &copier().id, false, at("units", 2, &legend.id));
        let state = played.state;
        let me = top(&state, P1, 2).expect("the copier in lane 2");
        // Reported as a Vanilla is: an in-place `transformed`, its definition unchanged.
        assert!(played.events.iter().any(|event| matches!(
            event,
            GameEvent::Transformed { instance_id, new_instance_id, from_def_id, to_def_id, .. }
                if *instance_id == me.id && *new_instance_id == me.id && from_def_id == to_def_id
        )));
        assert_eq!(me.def_id, copier().id);
        // The Legendary's Taunt, on the copier's own 2/2 (+1 from the Legendary's aura).
        assert!(has(&state, &me.id, KeywordKind::Taunt));
        let view = unit_view(&state, &me);
        assert_eq!((view.attack, view.max_health), (3, 2));
        // Its copied aura lends +1 to its other Units, the Legendary among them.
        assert_eq!(unit_view(&state, &plain).attack, 1 + 2);
        assert_eq!(unit_view(&state, &legend).attack, 3 + 1);
        // Both "whenever you play a card" triggers answer the next play.
        let before = state.players.p2.hero.health;
        let state = play(
            &state,
            P1,
            &cheap().id,
            false,
            json!({ "zone": { "row": "units", "lane": 4 } }),
        )
        .state;
        assert_eq!(state.players.p2.hero.health, before - 2 * LEGEND_PING);
        // The copied Activate is the copier's to use.
        let before = state.players.p2.hero.health;
        let state = act_ok(
            &state,
            json!({ "type": "activate", "instanceId": me.id, "ability": "ping", "playerId": "p1" }),
        )
        .state;
        assert_eq!(state.players.p2.hero.health, before - LEGEND_PING);
        // Both end-of-turn hooks deal their number.
        let before = state.players.p2.hero.health;
        let ended = pass(&state);
        assert_eq!(ended.state.players.p2.hero.health, before - 2 * LEGEND_PING);
    }

    #[test]
    fn r1850000_a_copied_death_runs() {
        let mut state = playing("death");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let me = put(&mut state, &copier().id, slot(P1, Row::Units, 2), json!({}));
        apply(
            &mut state,
            Some(&me.id),
            P1,
            effects::copy_unit_text(json_as(json!({ "target": instance(&legend.id) }))),
        );
        let before = state.players.p2.hero.health;
        destroy(&mut state, &me.id);
        assert!(!on_field(&state, &me.id));
        assert_eq!(state.players.p2.hero.health, before - LEGEND_PING);
    }

    #[test]
    fn r1850000_the_radiant_option_copies_the_radiant_face() {
        let mut state = playing("radiant");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let played = play(
            &state,
            P1,
            &radiant_copier().id,
            false,
            at("units", 2, &legend.id),
        );
        let mut state = played.state;
        let me = top(&state, P1, 2).expect("the copier in lane 2");
        // The Radiant text's Lifesteal, on the copier's base face: it is no Radiant card.
        assert!(has(&state, &me.id, KeywordKind::Lifesteal));
        assert!(has(&state, &me.id, KeywordKind::Taunt));
        assert!(!me.radiant);
        assert_eq!(unit_view(&state, &me).max_health, 2);
        // Its Death reads the Radiant face's number.
        let before = state.players.p2.hero.health;
        destroy(&mut state, &me.id);
        assert_eq!(state.players.p2.hero.health, before - LEGEND_RADIANT_PING);
    }

    #[test]
    fn r1850000_an_immutable_copier_copies_nothing() {
        let mut state = playing("immutable");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let me = put(
            &mut state,
            &immutable_copier().id,
            slot(P1, Row::Units, 2),
            json!({}),
        );
        let events = apply(
            &mut state,
            Some(&me.id),
            P1,
            effects::copy_unit_text(json_as(json!({ "target": instance(&legend.id) }))),
        );
        assert!(events_of_type(&events, GameEventType::Transformed).is_empty());
        assert!(!has(&state, &me.id, KeywordKind::Taunt));

        // Nor does a copier copy a Vanilla Unit, which has no text, or itself.
        let other = put(&mut state, &copier().id, slot(P1, Row::Units, 3), json!({}));
        apply(
            &mut state,
            None,
            P1,
            effects::vanilla(json_as(json!({ "target": instance(&legend.id) }))),
        );
        for target in [&legend.id, &other.id] {
            let events = apply(
                &mut state,
                Some(&other.id),
                P1,
                effects::copy_unit_text(json_as(json!({ "target": instance(target) }))),
            );
            assert!(events_of_type(&events, GameEventType::Transformed).is_empty());
        }
        assert!(!has(&state, &other.id, KeywordKind::Taunt));
    }

    #[test]
    fn r1850000_leaving_the_field_ends_it() {
        let mut state = playing("leaves");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let plain = put(&mut state, &cheap().id, slot(P1, Row::Units, 3), json!({}));
        let me = put(&mut state, &copier().id, slot(P1, Row::Units, 2), json!({}));
        apply(
            &mut state,
            Some(&me.id),
            P1,
            effects::copy_unit_text(json_as(json!({ "target": instance(&legend.id) }))),
        );
        assert_eq!(unit_view(&state, &plain).attack, 3);
        apply(
            &mut state,
            None,
            P1,
            effects::bounce(json_as(json!({ "target": instance(&me.id) }))),
        );
        let held = state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.id == me.id)
            .cloned()
            .expect("the copier back in hand");
        assert!(held.memory.is_empty());
        assert!(!unit_has(&state, &held, KeywordKind::Taunt));
        // Its copied aura went with it.
        assert_eq!(unit_view(&state, &plain).attack, 2);
    }
}

mod r4839000_a_copied_aura_draws_its_keywords_as_it_lands {
    use super::*;
    use jackioh_engine::effects::{aura_combat_keywords, drawn_combat_keywords, random_pool_keywords};

    use crate::rules::fixtures::harness::both;

    /// A Legendary 1/1 → 2/2: "Aura: Has {keywords} random combat keywords", 3 on the base face and 6 on
    /// the Radiant one, following its number (R3778000) with no arrival draw of its own, so only the
    /// copy draws.
    const KEYED: &str = "ut-keyed-legend";
    const KEYED_BASE: usize = 3;
    const KEYED_RADIANT: usize = 6;

    fn keyed() -> CardDef {
        let text = "Aura: Has {keywords|random combat keyword|random combat keywords}.";
        json_as(json!({
            "id": KEYED,
            "index": "48390",
            "name": "Keyed Legend (#4839)",
            "set": "Core",
            "type": "Unit",
            "tags": [],
            "rarity": "Legendary",
            "token": false,
            "cost": 1,
            "base": { "attack": 1, "health": 1, "keywords": [], "text": text },
            "radiant": { "attack": 2, "health": 2, "keywords": [], "text": text },
            "params": [{
                "key": "keywords",
                "base": KEYED_BASE,
                "radiant": KEYED_RADIANT,
                "better": "up",
                "step": 1,
                "min": 1,
            }],
        }))
    }

    /// `playing`'s board with `KEYED` registered beside #1850's cards.
    fn keyed_game(seed: &str) -> GameState {
        let state = playing(seed);
        let mut catalog = registered_catalog().clone();
        catalog.insert(KEYED.to_string(), keyed());
        register_catalog(catalog);
        let mut scripts = registered_scripts().clone();
        scripts.insert(
            KEYED.to_string(),
            both(Script {
                combat_keywords_follow: Some("keywords"),
                conditional_keywords: Some(read_hook(|args| aura_combat_keywords(&args, "keywords", None))),
                ..Script::default()
            }),
        );
        register_scripts(scripts);
        state
    }

    /// `KEYED` in p1's lane 1 and `copier` in lane 2: their ids.
    fn board(seed: &str, copier: &CardDef) -> (GameState, String, String) {
        let mut state = keyed_game(seed);
        let source = put(&mut state, KEYED, slot(P1, Row::Units, 1), json!({})).id;
        let me = put(&mut state, &copier.id, slot(P1, Row::Units, 2), json!({})).id;
        (state, source, me)
    }

    /// `me` copies `target`'s text (its Radiant text when `radiant`).
    fn copy(state: &mut GameState, me: &str, target: &str, radiant: bool) -> Vec<GameEvent> {
        let mut args = json!({ "target": instance(target) });
        if radiant {
            args["radiant"] = json!(true);
        }
        apply(state, Some(me), P1, effects::copy_unit_text(json_as(args)))
    }

    /// The card's list, in draw order.
    fn drawn(state: &GameState, id: &str) -> Vec<KeywordKind> {
        let card = find_instance(state, id).cloned().expect("the card");
        drawn_combat_keywords(&card, None)
            .iter()
            .map(Keyword::kind)
            .collect()
    }

    /// The keywords the card acts with.
    fn acting(state: &GameState, id: &str) -> Vec<KeywordKind> {
        let card = find_instance(state, id).cloned().expect("the card");
        keywords_of(state, &card).iter().map(Keyword::kind).collect()
    }

    /// `count` different keywords of R21's pool.
    fn assert_pool_draw(kinds: &[KeywordKind], count: usize) {
        assert_eq!(kinds.len(), count, "{kinds:?}");
        let pool: Vec<KeywordKind> = random_pool_keywords().iter().map(Keyword::kind).collect();
        assert!(kinds.iter().all(|kind| pool.contains(kind)), "{kinds:?}");
        let mut unique = kinds.to_vec();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), count, "no repeats: {kinds:?}");
    }

    #[test]
    fn r4839000_a_copier_draws_the_copied_auras_number_as_the_text_lands() {
        let (mut state, source, me) = board("keyed", &copier());
        assert!(drawn(&state, &source).is_empty());
        let events = copy(&mut state, &me, &source, false);
        assert_eq!(events_of_type(&events, GameEventType::Transformed).len(), 1);
        let list = drawn(&state, &me);
        assert_pool_draw(&list, KEYED_BASE);
        let shown = acting(&state, &me);
        assert!(
            list.iter().all(|kind| shown.contains(kind)),
            "{list:?} in {shown:?}"
        );
        assert!(
            drawn(&state, &source).is_empty(),
            "the copier drew its own; the target's list is its draw"
        );
    }

    #[test]
    fn r4839000_the_radiant_option_draws_the_radiant_number() {
        let (mut state, source, me) = board("keyed-radiant", &copier());
        copy(&mut state, &me, &source, true);
        let list = drawn(&state, &me);
        assert_pool_draw(&list, KEYED_RADIANT);
        let shown = acting(&state, &me);
        assert!(
            list.iter().all(|kind| shown.contains(kind)),
            "{list:?} in {shown:?}"
        );
    }

    #[test]
    fn r4839000_the_draw_replays_under_the_same_seed() {
        let draw = || {
            let (mut state, source, me) = board("keyed-replay", &copier());
            copy(&mut state, &me, &source, false);
            (drawn(&state, &me), state.rng_cursor)
        };
        let first = draw();
        assert_eq!(first.0.len(), KEYED_BASE);
        assert_eq!(first, draw());
    }

    #[test]
    fn r4839000_a_text_with_no_such_aura_shows_none_of_the_list_and_the_aura_back_draws_nothing_new() {
        let (mut state, source, me) = board("keyed-back", &copier());
        let other = put(&mut state, &legend().id, slot(P1, Row::Units, 3), json!({})).id;
        copy(&mut state, &me, &source, false);
        let list = drawn(&state, &me);
        assert_eq!(list.len(), KEYED_BASE);

        copy(&mut state, &me, &other, false);
        assert!(has(&state, &me, KeywordKind::Taunt), "the legend's printed Taunt");
        let shown = acting(&state, &me);
        for kind in list.iter().filter(|kind| **kind != KeywordKind::Taunt) {
            assert!(!shown.contains(kind), "{kind:?}: no text gives it now: {shown:?}");
        }
        assert_eq!(drawn(&state, &me), list, "the list stays whole");

        let cursor = state.rng_cursor;
        copy(&mut state, &me, &source, false);
        assert_eq!(drawn(&state, &me), list, "the same list, nothing new drawn");
        assert_eq!(state.rng_cursor, cursor, "no draw");
        let shown = acting(&state, &me);
        assert!(
            list.iter().all(|kind| shown.contains(kind)),
            "{list:?} in {shown:?}"
        );
    }

    #[test]
    fn r4839000_a_later_copy_of_a_higher_number_draws_only_the_rest() {
        let (mut state, source, me) = board("keyed-higher", &copier());
        copy(&mut state, &me, &source, false);
        let first = drawn(&state, &me);
        assert_eq!(first.len(), KEYED_BASE);
        copy(&mut state, &me, &source, true);
        let now = drawn(&state, &me);
        assert_pool_draw(&now, KEYED_RADIANT);
        assert_eq!(now[..KEYED_BASE], first[..], "the first 3 stay, in their order");
        let shown = acting(&state, &me);
        assert!(
            now.iter().all(|kind| shown.contains(kind)),
            "{now:?} in {shown:?}"
        );
    }

    #[test]
    fn r4839000_a_copier_made_radiant_keeps_the_copied_number_and_draws_nothing() {
        let (mut state, source, me) = board("keyed-flip", &copier());
        copy(&mut state, &me, &source, false);
        let list = drawn(&state, &me);
        assert_eq!(list.len(), KEYED_BASE);
        apply(
            &mut state,
            None,
            P1,
            effects::set_radiant(json_as(json!({ "instanceId": me }))),
        );
        assert!(find_instance(&state, &me).is_some_and(|card| card.radiant));
        assert_eq!(
            drawn(&state, &me),
            list,
            "R1850000: the copied face, so the copied number"
        );
    }

    #[test]
    fn r4839000_an_immutable_copier_draws_nothing() {
        let (mut state, source, me) = board("keyed-immutable", &immutable_copier());
        let cursor = state.rng_cursor;
        copy(&mut state, &me, &source, false);
        assert!(drawn(&state, &me).is_empty());
        assert_eq!(state.rng_cursor, cursor);
    }
}
