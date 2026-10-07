//! Port of `packages/engine/test/draw-complete.test.ts`.
//!
//! The draw-complete point of a cast-on-draw draw (SPEC §2.4, R58, R70; Classic #9 Income Tax's shape).
//!
//! R58: a cast-on-draw card is cast as it is drawn, and the draw is complete once that cast has
//! resolved, so a trap or trigger answering the draw answers it after the cast. The cast's own windows
//! (its announce, R448, and its step 4) offer every event so far to the traps, which used to hand them
//! the `drawn` that found the card mid-cast; the draw now holds that event back until its cast's
//! pipeline has finished, across a question the cast asks too.

use jackioh_engine::draw::draw;
use jackioh_engine::effects::damage;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{new_game, put, slot};

/// TS `def(name, type)`, its module `nextIndex` (from 5600) written out per definition.
fn def(name: &str, type_: &str, index: i32) -> CardDef {
    json_as(json!({
        "id": format!("dc-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (draw complete)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    }))
}

/// A cast-on-draw Spell whose Cry hits the enemy hero for 1.
fn bolt() -> CardDef {
    def("bolt", "Spell", 5601)
}

/// A cast-on-draw Spell that declares a target, so its cast asks its caster (R70, R81).
fn aimed() -> CardDef {
    def("aimed", "Spell", 5602)
}

/// p2's trap on the opponent's draws (Classic #9's moment): it hits the drawing player's hero for 2.
fn tax_trap() -> CardDef {
    def("tax", "Trap", 5603)
}

/// p2's Field Spell with an ordinary trigger on the opponent's draws: it hits that hero for 3.
fn watcher() -> CardDef {
    def("watcher", "Field Spell", 5604)
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn opponents_draw(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::Drawn { player, .. } if *player != ctx.controller)
}

fn hit_enemy_hero(amount: i32) -> Effect {
    damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        bolt().id,
        both(Script {
            static_flags: Some(json_as(json!({ "castOnDraw": true }))),
            cry: Some(hook(|_ctx| vec![hit_enemy_hero(1)])),
            ..Script::default()
        }),
    );
    scripts.insert(
        aimed().id,
        both(Script {
            static_flags: Some(json_as(json!({ "castOnDraw": true }))),
            targets: vec![json_as(
                json!({ "kind": "target", "min": 1, "max": 1, "filter": { "of": ["hero"] } }),
            )],
            cry: Some(hook(|_ctx| {
                vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 1 })))]
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        tax_trap().id,
        both(Script {
            triggers: vec![
                TriggerDef::new("tax", &[GameEventType::Drawn], |_ctx, _event| {
                    vec![hit_enemy_hero(2)]
                })
                .with_when(opponents_draw),
            ],
            ..Script::default()
        }),
    );
    scripts.insert(
        watcher().id,
        both(Script {
            triggers: vec![TriggerDef::new("watch", &[GameEventType::Drawn], |ctx, event| {
                if opponents_draw(ctx, event) {
                    vec![hit_enemy_hero(3)]
                } else {
                    vec![]
                }
            })],
            ..Script::default()
        }),
    );
    scripts
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for card in [bolt(), aimed(), tax_trap(), watcher()] {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    register_scripts(all);
    state.turn = 3;
    state.phase = Phase::Main;
    state
}

fn on_top(state: &mut GameState, def_id: &str) -> CardInstance {
    let card = new_instance(
        state,
        def_id,
        PlayerId::P1,
        Zone::Library { player: PlayerId::P1 },
    );
    state.players.p1.library.insert(0, card.clone());
    card
}

/// TS `events.findIndex(match)`: -1 when none matches.
fn index_of(events: &[GameEvent], found: impl Fn(&GameEvent) -> bool) -> i64 {
    events.iter().position(found).map_or(-1, |at| at as i64)
}

fn resolved_of(id: &str) -> impl Fn(&GameEvent) -> bool + '_ {
    move |event| matches!(event, GameEvent::CardResolved { instance_id, .. } if instance_id == id)
}

fn fired_of(id: &str) -> impl Fn(&GameEvent) -> bool + '_ {
    move |event| matches!(event, GameEvent::TrapFired { instance_id, .. } if instance_id == id)
}

/// TS `sinkFor(state)` held across `draw` and `settle`: one events list and one rng, lent with the state.
fn draw_and_settle(state: &mut GameState) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    draw(&mut sink, PlayerId::P1, 1);
    settle(&mut sink, Default::default());
    events
}

mod r58_a_draw_that_casts_is_complete_once_its_cast_has_resolved {
    use super::*;

    #[test]
    fn r58_a_trap_answering_a_cast_on_draw_draw_fires_after_the_cast_resolves_not_inside_it() {
        let mut state = game("r58-complete");
        let trap = put(
            &mut state,
            &tax_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            Default::default(),
        );
        let card = on_top(&mut state, &bolt().id);

        let events = draw_and_settle(&mut state);

        let resolved = index_of(&events, resolved_of(&card.id));
        let fired = index_of(&events, fired_of(&trap.id));
        assert!(resolved >= 0);
        assert!(fired > resolved);
        // Both happened: the cast's 1 to p2, the trap's 2 to p1.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH - 2);
        assert_eq!(state.held_draws, None);
    }

    #[test]
    fn r58_an_ordinary_trigger_on_the_draw_waits_for_the_cast_too() {
        let mut state = game("r58-trigger");
        put(
            &mut state,
            &watcher().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            Default::default(),
        );
        let card = on_top(&mut state, &bolt().id);

        let events = draw_and_settle(&mut state);

        let resolved = index_of(&events, resolved_of(&card.id));
        let hit = index_of(
            &events,
            |event| matches!(event, GameEvent::Damage { target_id, .. } if target_id == "hero-p1"),
        );
        assert!(hit > resolved);
        // The draw repeats once the cast has resolved (R58), so the watcher answers both draws.
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, GameEvent::Drawn { .. }))
                .count(),
            2
        );
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH - 6);
    }

    #[test]
    fn r58_a_plain_draw_is_answered_at_once_as_before() {
        let mut state = game("r58-plain");
        let trap = put(
            &mut state,
            &tax_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            Default::default(),
        );
        on_top(&mut state, "fx-1");
        let events = draw_and_settle(&mut state);
        assert!(events.iter().any(fired_of(&trap.id)));
        assert_eq!(state.held_draws, None);
    }

    #[test]
    fn r58_a_cast_that_asks_keeps_its_draw_held_across_the_answer_json_round_trip_included() {
        let mut state = game("r58-pause");
        let trap = put(
            &mut state,
            &tax_trap().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            Default::default(),
        );
        let card = on_top(&mut state, &aimed().id);

        let events = draw_and_settle(&mut state);
        assert_eq!(
            state.pending.as_ref().map(|pending| pending.player_id),
            Some(PlayerId::P1)
        );
        assert_eq!(state.held_draws, Some(vec![card.id.clone()]));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::TrapFired { .. }))
        );

        let round: GameState =
            serde_json::from_value(serde_json::to_value(&state).expect("a state serialises"))
                .expect("and parses");
        assert_eq!(hash_state(&round), hash_state(&state));
        let answer: Action = json_as(json!({
            "type": "answer",
            "choiceId": state.pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default(),
            "selection": [{ "pick": "hero", "player": "p2" }],
            "playerId": "p1",
            "nonce": "r58-answer",
        }));
        let live = reduce(&state, &answer);
        let revived = reduce(&round, &answer);
        assert_eq!(live.error, None);
        assert_eq!(hash_state(&revived.state), hash_state(&live.state));

        let resolved = index_of(&live.events, resolved_of(&card.id));
        let fired = index_of(&live.events, fired_of(&trap.id));
        assert!(resolved >= 0);
        assert!(fired > resolved);
        assert_eq!(live.state.players.p1.hero.health, HERO_HEALTH - 2);
        assert_eq!(live.state.held_draws, None);
    }
}
