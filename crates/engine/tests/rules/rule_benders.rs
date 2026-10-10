//! The rule benders' engine proofs (issue #526): ME-TRIBAL's immunity ([[R940]]), the zeroed
//! refresh ([[R941]]), the backrow fill ([[R942]]), ME-CREATED ([[R943]]) and the aimed cast with
//! the lane watch ([[R946]]). Real cards through `scenario()` (registered first), the keyword
//! granted through `granted_keywords`.

use jackioh_engine::effects::{
    CardScope, CardScopeOptions, CardZone, FillBoardArgs, ScopeSide, TuneArgs, TuneDirection,
    cards_in_card_scope, fill_board, reached_cards,
};
use jackioh_engine::resolve::{CastOptions, HookOptions, cast_card, make_context};
use jackioh_engine::testkit::*;
use serde_json::json;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

const BIGOT: &str = "core-002";
const HIT_JOB: &str = "core-016";
const VANILLA: &str = "core-008"; // (1) 4/4 Human.
const PLAIN: &str = "core-025"; // (4) 7/7, no tags.
const GAOKAO: &str = "meditative-034";
const RCTA: &str = "meditative-035";
const FILLER: &str = "core-005";
const WELL: &str = "core-006";

fn library() -> Value {
    json!([FILLER, FILLER, FILLER, FILLER])
}

/// TS `sinkFor(state)`: a sink beside the state, so the test can apply verbs directly.
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

fn immune(s: &mut Scenario, id: &str) {
    s.card_mut(id).granted_keywords.push(Keyword::ImmuneToTribalHate);
}

fn books(s: &Scenario) -> usize {
    s.pile(P1, "graveyard")
        .into_iter()
        .filter(|card| card.def_id == "classicplus-071")
        .count()
}

mod r940 {
    use super::*;

    fn hunting(seed: &str, hand: Value) -> Scenario {
        jackioh_cards::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "mana": 10, "hand": hand, "library": library() },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": PLAIN, "lane": 1 }, { "def": PLAIN, "lane": 2 }],
                "library": library(),
            },
        }))
    }

    #[test]
    fn r940_a_harmful_scope_naming_a_tribal_tag_passes_an_immune_unit_by() {
        let mut s = hunting(
            "benders-bigot",
            json!([{ "def": BIGOT, "radiant": true }, FILLER]),
        );
        let spared = s.unit(P2, 1).expect("victim").id.clone();
        immune(&mut s, &spared);
        s.play(BIGOT, json!({}));
        // Radiant Bigot's destroy_all over notTags [Human] passes the immune unit by.
        assert_eq!(s.unit(P2, 1).expect("stands").id, spared);
        assert!(s.unit(P2, 2).is_none(), "the plain unit is destroyed");
    }

    #[test]
    fn r940_a_scope_with_no_tribal_tag_still_reaches_it() {
        let mut s = hunting("benders-hitjob", json!([{ "def": HIT_JOB }, FILLER]));
        let target = s.unit(P2, 1).expect("victim").id.clone();
        immune(&mut s, &target);
        // Hit Job names no tag: the immunity does not apply.
        s.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": target }] }),
        );
        assert!(s.unit(P2, 1).is_none(), "a plain destroy still hits it");
    }

    #[test]
    fn r940_a_harm_declaration_cannot_pick_it_and_a_help_one_can() {
        let mut s = hunting(
            "benders-picks",
            json!([{ "def": BIGOT }, { "def": RCTA }, FILLER]),
        );
        let target = s.unit(P2, 1).expect("victim").id.clone();
        immune(&mut s, &target);
        // Bigot's pick aims harm at notTags [Human]: refused.
        s.expect_refused(|s| {
            s.play(
                BIGOT,
                json!({ "targets": [{ "pick": "instance", "instanceId": target }] }),
            )
        });
        // RCTA's pick aims help: allowed, and the CN tag lands.
        s.play(
            RCTA,
            json!({ "targets": [{ "pick": "instance", "instanceId": target }] }),
        );
        let after = s.unit(P2, 1).expect("stands");
        assert!(tags_of(s.state(), &after).contains(&Tag::Cn));
    }

    #[test]
    fn r940_a_degrade_over_a_tag_scope_skips_it_and_an_upgrade_reaches_it() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "benders-tune",
            "p1": { "mana": 10, "hand": [FILLER], "library": library() },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                "library": library(),
            },
        }));
        let spared = s.unit(P2, 1).expect("victim").id.clone();
        immune(&mut s, &spared);
        let scope = CardScope {
            side: Some(ScopeSide::Enemy),
            zones: vec![CardZone::Field],
            rows: None,
            types: None,
            tags: Some(vec![Tag::Human]),
            not_tags: None,
            exclude_self: None,
        };
        let args = TuneArgs {
            scope: Some(scope),
            ..Default::default()
        };
        let mut sink = Sink::for_state(s.state());
        let state = s.state_mut();
        let mut engine = sink.on(state);
        let mut ctx = make_context(
            &mut engine,
            None,
            HookOptions {
                controller: Some(P1),
                ..Default::default()
            },
        );
        let degraded: Vec<String> = reached_cards(&mut ctx, &args, TuneDirection::Degrade)
            .into_iter()
            .map(|reached| reached.card.id)
            .collect();
        let upgraded: Vec<String> = reached_cards(&mut ctx, &args, TuneDirection::Upgrade)
            .into_iter()
            .map(|reached| reached.card.id)
            .collect();
        assert!(!degraded.contains(&spared), "a Degrade passes it by");
        assert_eq!(degraded.len(), 1, "only the plain unit degrades");
        assert!(upgraded.contains(&spared), "an Upgrade still reaches it");
        assert_eq!(upgraded.len(), 2);
    }

    #[test]
    fn r940_it_protects_nothing_in_a_hand() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "benders-hand",
            "p1": { "mana": 10, "hand": [FILLER], "library": library() },
            "p2": {
                "hand": [{ "def": VANILLA }, { "def": VANILLA }],
                "library": library(),
            },
        }));
        let held = s.hand(P2)[0].id.clone();
        immune(&mut s, &held);
        // A harmful card-scope walk over the hand still matches the immune card: it is no unit there.
        let scope = CardScope {
            side: Some(ScopeSide::Enemy),
            zones: vec![CardZone::Hand],
            rows: None,
            types: None,
            tags: Some(vec![Tag::Human]),
            not_tags: None,
            exclude_self: None,
        };
        let mut sink = Sink::for_state(s.state());
        let state = s.state_mut();
        let mut engine = sink.on(state);
        let mut ctx = make_context(
            &mut engine,
            None,
            HookOptions {
                controller: Some(P1),
                ..Default::default()
            },
        );
        let matched: Vec<String> = cards_in_card_scope(
            &mut ctx,
            &scope,
            Some(&CardScopeOptions {
                aim: Some(TargetAim::Harm),
                ..Default::default()
            }),
        )
        .into_iter()
        .map(|scoped| scoped.card.id)
        .collect();
        assert!(matched.contains(&held), "in a hand the keyword protects nothing");
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn r940_gaokao_still_buffs_what_its_destroy_passes_by() {
        let mut s = hunting("benders-gaokao", json!([{ "def": GAOKAO }, FILLER]));
        let spared = s.unit(P2, 1).expect("victim").id.clone();
        immune(&mut s, &spared);
        let mark = s.events().len();
        s.play(GAOKAO, json!({}));
        // Neither-CN-nor-KY destroy passes the immune unit by and kills the plain one …
        assert_eq!(s.unit(P2, 1).expect("stands").id, spared);
        assert!(s.unit(P2, 2).is_none());
        // … but the help-aimed Buffs never reached either (neither is CN nor KY): no upgrades.
        let upgraded = s.events()[mark..]
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { .. }))
            .count();
        assert_eq!(upgraded, 0, "nothing tagged, nothing buffed");
    }
}

mod r941 {
    use super::*;

    #[test]
    fn r941_refreshed_mana_gives_only_the_rider_when_not_natural() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "benders-refresh",
            "p1": { "hand": [FILLER], "library": library() },
            "p2": { "hand": [FILLER], "library": library() },
        }));
        let mut side = s.state().players[P1].clone();
        side.mana.next_turn_mod = 2;
        let max = max_mana_for(&side);
        assert_eq!(
            refreshed_mana(&side, max, false),
            2,
            "no natural mana: the rider only"
        );
        assert_eq!(
            refreshed_mana(&side, max, true),
            max + 2,
            "natural mana: max plus the rider"
        );
        assert!(
            natural_mana_on(s.state()),
            "with no aura card acting, mana is natural"
        );
    }
}

mod r942 {
    use super::*;

    #[test]
    fn r942_fill_board_fills_both_backrows_active_side_first_skipping_locked_zones() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "benders-fill",
            "p1": { "hand": [FILLER], "library": library() },
            "p2": { "hand": [FILLER], "library": library() },
        }));
        lock_zone(
            s.state_mut(),
            ZoneSlot {
                player: P2,
                row: Row::Backrow,
                lane: 5,
            },
        );
        let mut sink = Sink::for_state(s.state());
        let state = s.state_mut();
        let mut engine = sink.on(state);
        let mut ctx = make_context(
            &mut engine,
            None,
            HookOptions {
                controller: Some(P1),
                ..Default::default()
            },
        );
        (fill_board(FillBoardArgs {
            def_id: WELL.to_string(),
            player: None,
            radiant: None,
            stats_override: None,
            armor_override: None,
            row: Some(Row::Backrow),
            side: Some(ScopeSide::Any),
            radiant_for: None,
        })
        .apply)(&mut ctx);
        // Both backrows filled for their owners, the Locked zone skipped.
        for lane in 1..=5 {
            let well = s.backrow(P1, lane).expect("p1's well");
            assert_eq!(well.def_id, WELL);
            assert_eq!(well.owner, P1);
        }
        for lane in 1..=4 {
            let well = s.backrow(P2, lane).expect("p2's well");
            assert_eq!(well.def_id, WELL);
            assert_eq!(well.owner, P2);
        }
        assert!(s.backrow(P2, 5).is_none(), "the Locked zone is skipped");
    }
}

mod r943 {
    use super::*;

    #[test]
    fn r943_minted_cards_and_the_coin_are_created_and_dealt_cards_are_not() {
        jackioh_cards::register_all();
        let mut cursor: u32 = 1;
        let minted = new_instance(&mut cursor, "core-001", P1, Zone::Hand { player: P1 });
        assert_eq!(minted.created, Some(true), "a minted instance is Created");
        let dealt = new_dealt_instance(&mut cursor, "core-001", P1, Zone::Library { player: P1 });
        assert_eq!(dealt.created, None, "a dealt instance is not");
        // The Coin is minted by setup, so it is Created; scenario decks are dealt, so they are not.
        let mut s = scenario(json!({
            "seed": "benders-created",
            "p1": { "hand": [FILLER], "library": [{ "def": VANILLA }, FILLER] },
            "p2": { "hand": [FILLER], "library": library() },
        }));
        let mut sink = Sink::for_state(s.state());
        let coin = {
            let state = s.state_mut();
            let mut engine = sink.on(state);
            create_in_hand(&mut engine, P1, "core-t-coin")
        };
        assert_eq!(coin.created, Some(true), "The Coin is Created");
        assert!(
            s.state().players[P1]
                .library
                .iter()
                .all(|card| card.created.is_none()),
            "dealt deck cards are not"
        );
    }

    #[test]
    fn r943_the_flag_survives_a_reset_a_move_and_a_json_round_trip() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "benders-keep",
            "p1": {
                "hand": [FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }],
                "library": [{ "def": VANILLA, "created": true }, FILLER],
            },
            "p2": { "hand": [FILLER], "library": library() },
        }));
        let id = s.unit(P1, 1).expect("unit").id.clone();
        s.card_mut(&id).created = Some(true);
        // A reset keeps it.
        reset_instance(s.card_mut(&id));
        assert_eq!(s.card(&id).created, Some(true), "a reset keeps the mark");
        // A draw moves it to the hand with the mark on.
        let mut sink = Sink::for_state(s.state());
        {
            let state = s.state_mut();
            let mut engine = sink.on(state);
            draw(&mut engine, P1, 1);
        }
        let drawn = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == VANILLA)
            .expect("drawn");
        assert_eq!(drawn.created, Some(true), "a move keeps the mark");
        // A JSON round trip keeps it.
        let value = serde_json::to_value(&drawn).expect("serialises");
        assert_eq!(value.get("created"), Some(&json!(true)));
        let back: CardInstance = serde_json::from_value(value).expect("deserialises");
        assert_eq!(back.created, Some(true));
    }

    #[test]
    fn r943_views_show_it_only_where_the_viewer_reads_the_card_and_the_library_list_marks_it() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "benders-views",
            "p1": {
                "hand": [{ "def": VANILLA, "created": true }],
                "field": [{ "def": VANILLA, "lane": 1, "created": true }],
                "library": [{ "def": VANILLA, "created": true }, FILLER],
            },
            "p2": { "hand": [FILLER], "library": library() },
        }));
        // The owner knows the top library card (as an open shuffle would show it).
        let top = s.state().players[P1].library[0].clone();
        s.card_mut(&top.id).known_as = Some(KnownAs {
            def_id: top.def_id.clone(),
            radiant: false,
        });
        // The owner reads it everywhere …
        let own = view_for(s.state(), P1);
        let HandView::Cards(hand) = &own.you.hand else {
            panic!("the owner's hand is cards")
        };
        assert_eq!(
            hand.iter()
                .find(|card| card.def_id == VANILLA)
                .and_then(|card| card.created),
            Some(true)
        );
        assert_eq!(
            own.you.units[0].as_ref().and_then(|unit| unit.created),
            Some(true)
        );
        // … the opponent reads only the field …
        let foe = view_for(s.state(), P2);
        assert_eq!(
            foe.opponent.units[0].as_ref().and_then(|unit| unit.created),
            Some(true)
        );
        assert!(
            matches!(foe.opponent.hand, HandView::Count { .. }),
            "a hidden hand is a count: the mark goes with its face"
        );
        // … and the owner's library list marks it.
        let list = own_library_view(s.state(), P1);
        assert!(
            list.cards.iter().any(|entry| entry.created == Some(true)),
            "the library list marks Created cards the owner knows"
        );
    }
}

mod r946 {
    use super::*;

    #[test]
    fn r946_an_aimed_cast_takes_the_named_pick_and_falls_back_when_it_is_not_legal() {
        jackioh_cards::register_all();
        let setup = || {
            scenario(json!({
                "seed": "benders-aim",
                "p1": { "mana": 10, "hand": [{ "def": HIT_JOB }, FILLER], "library": library() },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": PLAIN, "lane": 1 }, { "def": PLAIN, "lane": 2 }],
                    "library": library(),
                },
            }))
        };
        // The named legal pick is taken with no prompt.
        let mut s = setup();
        let target = s.unit(P2, 1).expect("victim").id.clone();
        let card = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == HIT_JOB)
            .expect("Hit Job");
        let mut sink = Sink::for_state(s.state());
        {
            let state = s.state_mut();
            let mut engine = sink.on(state);
            cast_card(
                &mut engine,
                &card,
                CastOptions {
                    aim_at: Some(target.clone()),
                    ..Default::default()
                },
            );
        }
        assert!(s.unit(P2, 1).is_none(), "the named card is destroyed");
        assert!(s.unit(P2, 2).is_some(), "only the named card");
        // A name no option carries falls back to the random cast, which still resolves.
        let mut s = setup();
        let card = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == HIT_JOB)
            .expect("Hit Job");
        let mut sink = Sink::for_state(s.state());
        {
            let state = s.state_mut();
            let mut engine = sink.on(state);
            cast_card(
                &mut engine,
                &card,
                CastOptions {
                    aim_at: Some("c999".to_string()),
                    random: Some(true),
                    ..Default::default()
                },
            );
        }
        let standing = [s.unit(P2, 1).is_some(), s.unit(P2, 2).is_some()]
            .into_iter()
            .filter(|stood| *stood)
            .count();
        assert_eq!(standing, 1, "the fallback cast still destroys exactly one");
    }

    #[test]
    fn r946_a_lane_watch_answers_resolved_plays_and_summons_in_its_lane_only_this_turn() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "benders-watch",
            "p1": {
                "mana": 10,
                "hand": [{ "def": "meditative-098", "radiant": true }, { "def": VANILLA }, { "def": "core-069" }, FILLER],
                "library": [{ "def": VANILLA }, FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": library() },
        }));
        s.play("meditative-098", json!({ "modes": ["lane-2"] }));
        // A summon into the lane answers (first entry: the open lane 2) …
        s.play("core-069", json!({}));
        let summoned = s.unit(P1, 2).expect("the recruit lands in lane 2");
        assert_eq!(summoned.def_id, VANILLA);
        assert_eq!(books(&s), 1, "one Book for the summon");
        // … a play into the lane answers again …
        s.play(VANILLA, json!({ "zone": 2 }));
        assert_eq!(books(&s), 2, "one Book for the played card");
        // … and next turn the watch is gone.
        s.end_turn();
        s.end_turn();
        s.play(VANILLA, json!({ "zone": 2 }));
        assert_eq!(books(&s), 2, "no Book after the turn");
    }
}
