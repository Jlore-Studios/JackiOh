//! `drawWhile` (effects/drawWhile.ts; Classic #46 Divine Favor's "draw until"): one draw at a time while
//! a condition holds, read again before each draw, ending at the first draw that adds no card to the
//! hand — fatigue, a burn at the hand cap, a cast on draw, a draw a draw limit stops (§2.4, R58, B5 E3)
//! — or that leaves a question open.
//!
//! Port of `packages/engine/test/effects-drawWhile.test.ts`.

use jackioh_engine::effects::{DrawWhileArgs, damage, draw_while};
use std::sync::Arc;

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, slot};

fn spell(id: &str, cost: i32) -> CardDef {
    json_as(json!({
        "id": format!("dw-{id}"),
        "index": format!("dw-{id}"),
        "name": id,
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

/// TS `{ ...spell("limiter"), type: "Field Spell" }`.
fn limiter() -> CardDef {
    CardDef { type_: CardType::FieldSpell, ..spell("limiter", 1) }
}

fn cast() -> CardDef {
    spell("cast", 1)
}

const PLAIN: &str = "dw-plain";

fn setup(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    catalog.insert(limiter().id, limiter());
    catalog.insert(cast().id, cast());
    catalog.insert(PLAIN.to_string(), spell("plain", 1));
    register_catalog(catalog);
    let mut scripts: IndexMap<String, Script> = IndexMap::new();
    scripts.insert(
        limiter().id,
        Script {
            draw_limit: Some(read_hook(|_args| vec![DrawLimit { player: DrawLimitPlayer::Both, count: 1 }])),
            ..Script::default()
        },
    );
    scripts.insert(
        cast().id,
        Script {
            static_flags: Some(StaticFlags { cast_on_draw: Some(true), ..StaticFlags::default() }),
            cry: Some(hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))])),
            ..Script::default()
        },
    );
    scripts.insert(PLAIN.to_string(), Script::default());
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts {
        registry.insert(id, CardScripts { base: script.clone(), radiant: script });
    }
    register_scripts(registry);
    state.players[PlayerId::P1].hand = vec![];
    state.players[PlayerId::P2].hand = vec![];
    // A main phase of p1's (a draw during setup is no player's turn and counts toward no limit, B5 E3).
    state.turn = 3;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state
}

/// TS `drawTo(state, mark)` with a number: draw while p1's hand holds fewer than `mark` cards.
fn below(mark: usize) -> impl Fn(&EffectContext<'_>) -> bool + Send + Sync + 'static {
    move |c: &EffectContext<'_>| c.state.players[PlayerId::P1].hand.len() < mark
}

/// Run `drawWhile` for p1 while `more` holds; returns the sink's events. Like TS's, the sink's rng
/// starts at the state's cursor and is not written back.
fn draw_to(state: &mut GameState, more: impl Fn(&EffectContext<'_>) -> bool + Send + Sync + 'static) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx =
            make_context(&mut sink, None, HookOptions { controller: Some(PlayerId::P1), ..Default::default() });
        (draw_while(DrawWhileArgs { more: Arc::new(more) }).apply)(&mut ctx);
    }
    events
}

fn plains(count: usize) -> Vec<String> {
    vec![PLAIN.to_string(); count]
}

mod draw_while_classic_46_divine_favor_s_draw_until {
    use super::*;

    #[test]
    fn draws_one_card_at_a_time_asking_again_before_each_draw_until_the_condition_fails() {
        let mut state = setup("dw-basic");
        set_library(&mut state, PlayerId::P1, &plains(5));
        in_hand(&mut state, PLAIN, PlayerId::P1, 1);
        let events = draw_to(&mut state, below(4));
        assert_eq!(events_of_type(&events, GameEventType::Drawn).len(), 3);
        assert_eq!(state.players[PlayerId::P1].hand.len(), 4);
        assert_eq!(state.players[PlayerId::P1].library.len(), 2);
    }

    #[test]
    fn draws_nothing_when_the_condition_already_fails() {
        let mut state = setup("dw-none");
        set_library(&mut state, PlayerId::P1, &plains(1));
        in_hand(&mut state, PLAIN, PlayerId::P1, 3);
        assert_eq!(events_of_type(&draw_to(&mut state, below(3)), GameEventType::Drawn).len(), 0);
        assert_eq!(state.players[PlayerId::P1].library.len(), 1);
    }

    #[test]
    fn a_fatigue_hit_adds_no_card_and_ends_it_one_hit_not_one_per_missing_card() {
        let mut state = setup("dw-fatigue");
        set_library(&mut state, PlayerId::P1, &plains(1));
        let events = draw_to(&mut state, below(5));
        assert_eq!(events_of_type(&events, GameEventType::Drawn).len(), 1);
        assert_eq!(events_of_type(&events, GameEventType::Fatigue).len(), 1);
        assert_eq!(state.players[PlayerId::P1].fatigue_count, 1);
    }

    #[test]
    fn a_burn_at_the_hand_cap_adds_no_card_and_ends_it() {
        let mut state = setup("dw-burn");
        set_library(&mut state, PlayerId::P1, &plains(3));
        in_hand(&mut state, PLAIN, PlayerId::P1, HAND_CAP);
        let events = draw_to(&mut state, |_c: &EffectContext<'_>| true);
        assert_eq!(events_of_type(&events, GameEventType::Burned).len(), 1);
        assert_eq!(state.players[PlayerId::P1].library.len(), 2);
    }

    /// TS: "B5 E3 a draw a draw limit stops adds no card and ends it".
    #[test]
    fn b5_e3_a_draw_a_draw_limit_stops_adds_no_card_and_ends_it() {
        let mut state = setup("dw-limit");
        put(&mut state, &limiter().id, slot(PlayerId::P2, Row::Backrow, 1), json!({}));
        set_library(&mut state, PlayerId::P1, &plains(3));
        let events = draw_to(&mut state, below(3));
        assert_eq!(events_of_type(&events, GameEventType::Drawn).len(), 1);
        assert_eq!(events_of_type(&events, GameEventType::DrawLimited).len(), 1);
        assert_eq!(state.players[PlayerId::P1].library.len(), 2);
    }

    #[test]
    fn r58_a_card_cast_on_draw_adds_no_card_and_ends_it_even_when_its_chain_repeats_into_a_card() {
        let mut state = setup("dw-cast");
        set_library(
            &mut state,
            PlayerId::P1,
            &[cast().id, PLAIN.to_string(), PLAIN.to_string(), PLAIN.to_string()],
        );
        let events = draw_to(&mut state, below(3));
        let hero_hits = events
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { target_id, .. } if target_id == "hero-p2"))
            .count();
        assert_eq!(hero_hits, 1);
        // The chain's repeat drew one card (§2.4), and the draws stop there.
        assert_eq!(state.players[PlayerId::P1].hand.len(), 1);
        assert_eq!(state.players[PlayerId::P1].library.len(), 2);
    }
}
