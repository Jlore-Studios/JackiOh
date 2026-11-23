// The verbs and riders Stinkyscape+ S24 (issue #3570) added for Call to Chaos (Mindustrial-Swarm
// Edition), each through a fixture script before a card uses it: the Evolved rider (R3570001), the GIGA
// rider (R3570002), a summon that grants keywords as the card lands (R3570006), "summon or cast" (R3570003)
// and the launch watch with its caps (R3570004). The cards' own tests play them in the real scripts;
// these pin the verbs themselves, so the engine never imports `crates/cards`. Ids are `s24-`.

use jackioh_engine::config::{CHAOS_MS_LAUNCH_WATCH_ANSWERS_PER_TURN, CHAOS_MS_LAUNCH_WATCH_CAP};
use jackioh_engine::effects;
use jackioh_engine::subsystems::{
    CHAOS_MS_EFFECTS, CHAOS_MS_LAUNCH_WATCH_HOOK, launch_watch_answers_this_turn, launch_watch_step,
    launch_watches_held,
};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::triggers::{LAUNCH_WATCH, dispatch_event};
use jackioh_engine::work::script_step_for;

use crate::rules::fixtures::harness::{apply_now, run_effect, run_on_sink, setup_catalog};

/// A Unit whose text prints an Evolve hook (it grants itself Flying), a Unit that prints none, a vanilla
/// Spell, a Unit whose Cry adds the marker card to its controller's hand, and that marker.
const EVOLVER: &str = "s24-evolver";
const PLAIN: &str = "s24-plain";
const SPELL: &str = "s24-spell";
const CRY_UNIT: &str = "s24-cry-unit";
const MARKER: &str = "s24-marker";

fn card(index: u32, id: &str, type_: &str, stats: Option<(i32, i32)>) -> CardDef {
    let face = match stats {
        Some((attack, health)) => {
            json!({ "attack": attack, "health": health, "keywords": [], "text": id })
        }
        None => json!({ "keywords": [], "text": id }),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }))
}

fn register() {
    setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in [
        card(35700, EVOLVER, "Unit", Some((1, 1))),
        card(35701, PLAIN, "Unit", Some((1, 1))),
        card(35702, SPELL, "Spell", None),
        card(35703, CRY_UNIT, "Unit", Some((1, 1))),
        card(35704, MARKER, "Spell", None),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts: IndexMap<String, CardScripts> = registered_scripts().clone();
    let evolver = Script {
        evolve: Some(hook(|_ctx| {
            vec![effects::grant_keyword(json_as(
                json!({ "target": { "of": "self" }, "keyword": { "kind": "Flying" } }),
            ))]
        })),
        ..Script::default()
    };
    let cry_unit = Script {
        cry: Some(hook(|_ctx| {
            vec![effects::add_to_hand(json_as(json!({ "defId": MARKER })))]
        })),
        ..Script::default()
    };
    for (id, script) in [(EVOLVER, evolver), (CRY_UNIT, cry_unit)] {
        scripts.insert(
            id.to_string(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
    }
    register_scripts(scripts);
}

fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register();
    let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

/// A pool of exactly these definitions (`defId`, "a pool a script names card by card").
fn pool(ids: &[&str]) -> Value {
    json!({ "defId": ids })
}

mod r3570001_the_evolved_rider {
    use super::*;

    #[test]
    fn r3570001_add_random_from_catalog_marks_only_a_card_that_can_evolve() {
        let mut s = board("s24-add", json!({}), json!({}));
        apply_now(
            s.state_mut(),
            effects::add_random_from_catalog(json_as(json!({
                "query": pool(&[EVOLVER, PLAIN]),
                "count": 8,
                "entersEvolved": true,
            }))),
        );
        let hand = s.hand(P1);
        assert_eq!(hand.len(), 8);
        let evolvers: Vec<&CardInstance> = hand.iter().filter(|card| card.def_id == EVOLVER).collect();
        let plains: Vec<&CardInstance> = hand.iter().filter(|card| card.def_id == PLAIN).collect();
        assert!(!evolvers.is_empty() && !plains.is_empty(), "both were drawn");
        for card in evolvers {
            assert_eq!(card.enters_evolved, Some(true), "it can evolve, so it is marked");
        }
        for card in plains {
            assert_eq!(card.enters_evolved, None, "no Evolve text, so no mark");
        }
    }

    #[test]
    fn r3570001_add_random_from_catalog_marks_nothing_without_the_rider() {
        let mut s = board("s24-add-plain", json!({}), json!({}));
        apply_now(
            s.state_mut(),
            effects::add_random_from_catalog(json_as(json!({ "query": pool(&[EVOLVER]), "count": 2 }))),
        );
        assert!(s.hand(P1).iter().all(|card| card.enters_evolved.is_none()));
    }

    #[test]
    fn r3570001_summon_random_lands_a_marked_card_evolved() {
        let mut s = board("s24-evolve", json!({}), json!({}));
        apply_now(
            s.state_mut(),
            effects::summon_random_with(json_as(json!({
                "query": pool(&[EVOLVER, PLAIN]),
                "entersEvolved": true,
            }))),
        );
        // The pick is one of the two: an Evolve-hooked card lands evolved and its Evolve text runs.
        let landed = s.unit_at(P1, 1);
        if landed.def_id == EVOLVER {
            assert_eq!(landed.evolved, Some(true));
            assert!(
                has_keyword(&s.stats(&landed).keywords, KeywordKind::Flying),
                "its Evolve text ran"
            );
        } else {
            assert_eq!(landed.evolved, None, "a card with no Evolve text stays as it is");
        }
        // Forcing the Evolve-hooked one: the pool names it alone.
        let mut s = board("s24-evolve-only", json!({}), json!({}));
        apply_now(
            s.state_mut(),
            effects::summon_random_with(json_as(
                json!({ "query": pool(&[EVOLVER]), "entersEvolved": true }),
            )),
        );
        let landed = s.unit_at(P1, 1);
        assert_eq!(landed.def_id, EVOLVER);
        assert_eq!(landed.evolved, Some(true));
        assert!(
            has_keyword(&s.stats(&landed).keywords, KeywordKind::Flying),
            "its Evolve text ran"
        );
        // And with no rider the same card lands plain.
        let mut s = board("s24-evolve-none", json!({}), json!({}));
        apply_now(
            s.state_mut(),
            effects::summon_random(json_as(json!({ "query": pool(&[EVOLVER]) }))),
        );
        let landed = s.unit_at(P1, 1);
        assert_eq!(landed.evolved, None);
        assert!(!has_keyword(&s.stats(&landed).keywords, KeywordKind::Flying));
    }
}

mod r3570002_the_giga_rider {
    use super::*;

    #[test]
    fn r3570002_summon_random_lands_a_giga_card_across_four_zones() {
        let mut s = board("s24-giga", json!({}), json!({}));
        apply_now(
            s.state_mut(),
            effects::summon_random_with(json_as(json!({ "query": pool(&[PLAIN]), "giga": true }))),
        );
        let landed = s.unit_at(P1, 1);
        assert_eq!(landed.def_id, PLAIN);
        assert_eq!(
            landed.giga,
            Some(true),
            "it prints no GIGA, the summon makes it so"
        );
        // Lane 2 is covered by it (R3546000): the next plain summon lands in lane 3.
        apply_now(
            s.state_mut(),
            effects::summon_random(json_as(json!({ "query": pool(&[PLAIN]) }))),
        );
        assert!(s.unit(P1, 2).is_none());
        assert_eq!(s.unit_at(P1, 3).giga, None);
    }

    #[test]
    fn r3570002_with_no_four_zone_fit_a_giga_summon_lands_nothing_and_draws_nothing() {
        let mut s = board(
            "s24-giga-none",
            json!({ "field": [{ "def": PLAIN, "lane": 2 }, { "def": PLAIN, "lane": 4 }] }),
            json!({}),
        );
        let cursor = s.state().rng_cursor;
        apply_now(
            s.state_mut(),
            effects::summon_random_with(json_as(json!({ "query": pool(&[PLAIN]), "giga": true }))),
        );
        // Lanes 1 and 3 and 5 are open, but no two neighbours are: nothing is summoned and (R129) no draw is made.
        assert_eq!(s.state().rng_cursor, cursor);
        let units = (1..=5).filter(|lane| s.unit(P1, *lane).is_some()).count();
        assert_eq!(units, 2);
        // The same board takes a plain summon, so the pool was not the reason.
        apply_now(
            s.state_mut(),
            effects::summon_random(json_as(json!({ "query": pool(&[PLAIN]) }))),
        );
        assert_eq!((1..=5).filter(|lane| s.unit(P1, *lane).is_some()).count(), 3);
    }
}

mod r3570006_the_keywords_rider {
    use super::*;

    #[test]
    fn r3570006_summon_random_grants_its_keywords_as_the_card_lands() {
        let mut s = board("s24-keywords", json!({}), json!({}));
        apply_now(
            s.state_mut(),
            effects::summon_random_with(json_as(json!({
                "query": pool(&[PLAIN]),
                "keywords": [{ "kind": "Can't attack" }],
            }))),
        );
        let landed = s.unit_at(P1, 1);
        assert!(has_keyword(&s.stats(&landed).keywords, KeywordKind::CantAttack));
        // The next one, with no rider, takes none.
        apply_now(
            s.state_mut(),
            effects::summon_random(json_as(json!({ "query": pool(&[PLAIN]) }))),
        );
        assert!(!has_keyword(
            &s.stats(s.unit_at(P1, 2)).keywords,
            KeywordKind::CantAttack
        ));
    }
}

mod r3570003_summon_or_cast {
    use super::*;

    #[test]
    fn r3570003_summon_or_cast_casts_the_spell_and_summons_the_unit_without_its_cry() {
        let mut s = board("s24-summon-or-cast", json!({}), json!({}));
        // One run, so its tally of unique draws (R3563000) holds both picks.
        let both = Effect::new("s24:both", |ctx| {
            for _ in 0..2 {
                let one = effects::summon_or_cast_one_random(json_as(pool(&[SPELL, CRY_UNIT])));
                (one.apply)(ctx);
            }
        });
        let events = with_unique_draws(true, || run_effect(s.state_mut(), both, P1));
        // Two unique picks of two cards: one each.
        let summoned: Vec<&GameEvent> = events
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == CRY_UNIT))
            .collect();
        let played: Vec<&GameEvent> = events
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == SPELL))
            .collect();
        assert_eq!(summoned.len(), 1, "the Unit is summoned");
        assert_eq!(played.len(), 1, "the Spell is cast");
        assert_eq!(s.unit_at(P1, 1).def_id, CRY_UNIT);
        // R1: a summon fires no Cry, so the marker its Cry adds is nowhere.
        assert!(s.hand(P1).iter().all(|card| card.def_id != MARKER));
        // The cast Spell ends in its owner's graveyard (§10.5), a summoned Unit never was a play.
        assert!(
            s.state()
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.def_id == SPELL)
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == CRY_UNIT))
        );
    }

    #[test]
    fn r3570003_a_permanent_with_no_zone_lands_nowhere_and_a_spell_is_cast_all_the_same() {
        let mut s = board(
            "s24-summon-or-cast-full",
            json!({ "field": (1..=5).map(|lane| json!({ "def": PLAIN, "lane": lane })).collect::<Vec<_>>() }),
            json!({}),
        );
        let events = run_effect(
            s.state_mut(),
            effects::summon_or_cast_one_random(json_as(pool(&[CRY_UNIT]))),
            P1,
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::Summoned { .. }))
        );
        assert_eq!((1..=5).filter(|lane| s.unit(P1, *lane).is_some()).count(), 5);
        let events = run_effect(
            s.state_mut(),
            effects::summon_or_cast_one_random(json_as(pool(&[SPELL]))),
            P1,
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == SPELL))
        );
    }
}

mod r3570004_the_launch_watch {
    use super::*;

    fn entry(name: &str) -> Effect {
        let found = CHAOS_MS_EFFECTS
            .iter()
            .find(|effect| effect.name == name)
            .unwrap_or_else(|| panic!("no entry {name}"));
        (found.build)()
    }

    /// An entry resolved for `player` as a played card of theirs would resolve it.
    fn run(state: &mut GameState, effect: Effect, player: PlayerId) -> Vec<GameEvent> {
        let me = new_instance(&mut *state, SPELL, player, Zone::Resolving { player });
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
            let options = HookOptions {
                controller: Some(player),
                ..HookOptions::default()
            };
            {
                let mut ctx = make_context(&mut sink, Some(&me), options);
                apply_effects(&[effect], &mut ctx);
            }
            state_check(&mut sink);
        }
        state.rng_cursor = rng.cursor();
        events
    }

    fn launched(player: PlayerId) -> GameEvent {
        GameEvent::StarshipLaunched {
            player,
            instance_id: "s24-ship".to_string(),
            def_id: PLAIN.to_string(),
            name: "Ship".to_string(),
            pieces: vec![],
        }
    }

    #[test]
    fn r3570004_a_launch_watch_answers_its_holder_s_launch_and_not_the_opponent_s() {
        let mut s = board("s24-watch", json!({}), json!({}));
        run(s.state_mut(), entry("launchWatch"), P1);
        assert_eq!(launch_watches_held(s.state(), P1), 1);
        // The opponent's launch is not the holder's: nothing is queued.
        let queued = run_on_sink(s.state_mut(), |sink| dispatch_event(sink, &launched(P2))).0;
        assert!(queued.is_empty(), "{queued:?}");
        // Its holder's own launch queues one answer, run as the holder, naming the ship.
        let queued = run_on_sink(s.state_mut(), |sink| dispatch_event(sink, &launched(P1))).0;
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].hook, LAUNCH_WATCH);
        assert_eq!(queued[0].resume.hook, CHAOS_MS_LAUNCH_WATCH_HOOK);
        assert_eq!(queued[0].resume.data.get("watcher"), Some(&json!("p1")));
        assert_eq!(queued[0].resume.data.get("instanceId"), Some(&json!("s24-ship")));
        // The opponent holding one answers theirs alone.
        run(s.state_mut(), entry("launchWatch"), P2);
        let queued = run_on_sink(s.state_mut(), |sink| dispatch_event(sink, &launched(P2))).0;
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].resume.data.get("watcher"), Some(&json!("p2")));
    }

    #[test]
    fn r3570004_a_player_holds_at_most_three_launch_watches() {
        let mut s = board("s24-watch-cap", json!({}), json!({}));
        for held in 1..=CHAOS_MS_LAUNCH_WATCH_CAP {
            run(s.state_mut(), entry("launchWatch"), P1);
            assert_eq!(launch_watches_held(s.state(), P1), held);
        }
        let mods = s.state().players.p1.mods.len();
        run(s.state_mut(), entry("launchWatch"), P1);
        assert_eq!(
            s.state().players.p1.mods.len(),
            mods,
            "a roll at the cap adds nothing"
        );
        assert_eq!(launch_watches_held(s.state(), P1), CHAOS_MS_LAUNCH_WATCH_CAP);
        assert_eq!(launch_watches_held(s.state(), P2), 0);
        // Each is badged with the clause as the card prints it (R169).
        let labels: Vec<String> = s
            .view(P2)
            .opponent
            .modifiers
            .iter()
            .map(|view| view.label.clone())
            .collect();
        assert_eq!(labels.len() as i32, CHAOS_MS_LAUNCH_WATCH_CAP);
        assert!(
            labels
                .iter()
                .all(|label| label.starts_with("For the rest of the game, when you launch a Starship"))
        );
    }

    #[test]
    fn r3570004_the_engine_answers_the_watch_hook_for_any_script() {
        let resume = Resume {
            def_id: "fx-1".to_string(),
            hook: CHAOS_MS_LAUNCH_WATCH_HOOK.to_string(),
            step: "chaosMsLaunchWatch".to_string(),
            radiant: false,
            instance_id: None,
            data: IndexMap::new(),
        };
        assert!(script_step_for(&Script::default(), &resume).is_some());
    }

    #[test]
    fn r3570004_the_step_answers_at_most_five_times_a_turn() {
        let mut s = board("s24-watch-answers", json!({}), json!({}));
        let step = launch_watch_step();
        let state = s.state_mut();
        let answer = |state: &mut GameState| -> usize {
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
            let options = HookOptions {
                controller: Some(P1),
                ..HookOptions::default()
            };
            let mut ctx = make_context(&mut sink, None, options);
            step(&mut ctx).len()
        };
        for answered in 1..=CHAOS_MS_LAUNCH_WATCH_ANSWERS_PER_TURN {
            // A base Call to Chaos cast, then the Weapon summon: two effects an answer.
            assert_eq!(answer(state), 2);
            assert_eq!(launch_watch_answers_this_turn(state, P1), answered);
        }
        assert_eq!(answer(state), 0, "the terminator");
        assert_eq!(launch_watch_answers_this_turn(state, P2), 0);
        state.turn += 1;
        assert_eq!(
            launch_watch_answers_this_turn(state, P1),
            0,
            "a new turn counts again"
        );
        assert_eq!(answer(state), 2);
    }
}
