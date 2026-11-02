//! The rule benders' engine proofs (issue #526): ME-TRIBAL's immunity ([[R940]]), the zeroed
//! refresh ([[R941]]), the backrow fill ([[R942]]), ME-CREATED ([[R943]]) and the aimed cast with
//! the lane watch ([[R946]]), Showdown's empty lane ([[R831008]]) and, from issue #1956, a Created
//! card's generator ([[R1956001]]). Real cards through `scenario()` (registered first), the keyword
//! granted through `granted_keywords`.

use jackioh_engine::effects::{
    CardScope, CardScopeOptions, CardZone, FillBoardArgs, ScopeSide, TuneArgs, TuneDirection,
    cards_in_card_scope, fill_board, reached_cards,
};
use jackioh_engine::resolve::{CastOptions, HookOptions, cast_card, make_context};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::Bench;
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

fn books(s: &Scenario) -> usize {
    s.pile(P1, "graveyard")
        .into_iter()
        .filter(|card| card.def_id == "classicplus-071")
        .count()
}

mod r940 {
    use super::*;

    const FELINOR: &str = "meditative-025";

    fn hunting(seed: &str, hand: Value) -> Scenario {
        jackioh_cards::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "mana": 10, "hand": hand, "library": library() },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": FELINOR, "lane": 1 }, { "def": PLAIN, "lane": 2 }],
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
        s.play(BIGOT, json!({}));
        // Radiant Bigot's destroy_all over notTags [Human] passes the immune unit by.
        assert_eq!(s.unit(P2, 1).expect("stands").id, spared);
        assert!(s.unit(P2, 2).is_none(), "the plain unit is destroyed");
    }

    #[test]
    fn r940_a_scope_with_no_tribal_tag_still_reaches_it() {
        let mut s = hunting("benders-hitjob", json!([{ "def": HIT_JOB }, FILLER]));
        let target = s.unit(P2, 1).expect("victim").id.clone();
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
                "field": [{ "def": FELINOR, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                "library": library(),
            },
        }));
        let spared = s.unit(P2, 1).expect("victim").id.clone();
        s.card_mut(&spared).granted_tags = Some(vec![Tag::Human]);
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
        let mut sink = Bench::new(s.state());
        let state = s.state_mut();
        let mut engine = sink.sink(state);
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
                "hand": [{ "def": FELINOR }, { "def": VANILLA }],
                "library": library(),
            },
        }));
        let held = s.hand(P2)[0].id.clone();
        s.card_mut(&held).granted_tags = Some(vec![Tag::Human]);
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
        let mut sink = Bench::new(s.state());
        let state = s.state_mut();
        let mut engine = sink.sink(state);
        let ctx = make_context(
            &mut engine,
            None,
            HookOptions {
                controller: Some(P1),
                ..Default::default()
            },
        );
        let matched: Vec<String> = cards_in_card_scope(
            &ctx,
            &scope,
            Some(&CardScopeOptions {
                aim: Some(TargetAim::Harm),
                ..Default::default()
            }),
        )
        .into_iter()
        .map(|scoped| scoped.card.id)
        .collect();
        assert!(matched.contains(&held), "in a hand the ability protects nothing");
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn r940_gaokao_still_buffs_what_its_destroy_passes_by() {
        let mut s = hunting("benders-gaokao", json!([{ "def": GAOKAO }, FILLER]));
        let spared = s.unit(P2, 1).expect("victim").id.clone();
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

    #[test]
    fn r940_a_shipped_catalog_that_printed_the_keyword_still_reads_and_the_view_leaves_it_out() {
        // Through v0.4.4b the Felinor printed the immunity as a keyword; a game folded under such a catalog
        // (R767000) reads it and the unit stays immune, but the client is not shown it.
        jackioh_cards::register_all();
        let shipped = jackioh_cards::catalog_at("v0.4.4").expect("v0.4.4 is carried");
        jackioh_engine::with_shipped(Some(shipped), || {
            let mut s = hunting(
                "benders-shipped",
                json!([{ "def": BIGOT, "radiant": true }, FILLER]),
            );
            let felinor = s.unit(P2, 1).expect("victim").id.clone();
            assert!(
                s.stats(&felinor)
                    .keywords
                    .iter()
                    .any(|keyword| keyword.kind() == KeywordKind::ImmuneToTribalHate),
                "that catalog prints it"
            );
            let shown = s.view(P1).opponent.units[0]
                .clone()
                .expect("the Felinor")
                .keywords;
            assert!(
                shown
                    .iter()
                    .all(|keyword| keyword.kind() != KeywordKind::ImmuneToTribalHate)
            );
            s.play(BIGOT, json!({}));
            assert_eq!(s.unit(P2, 1).expect("stands").id, felinor);
        });
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
        let mut sink = Bench::new(s.state());
        let state = s.state_mut();
        let mut engine = sink.sink(state);
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
            chinese: None,
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
        let mut sink = Bench::new(s.state());
        let coin = {
            let state = s.state_mut();
            let mut engine = sink.sink(state);
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

    /// D14: once the Meditative set ships, a game's setup with nothing previewed hands out a Created
    /// Coin and deals every deck card unmarked.
    #[test]
    fn r943_since_the_meditative_set_shipped_every_game_marks_the_coin_and_no_dealt_card() {
        jackioh_cards::register_all();
        assert!(set_ships(SetName::Meditative));
        let mut cursor: u32 = 1;
        let minted = new_instance(&mut cursor, "core-001", P1, Zone::Hand { player: P1 });
        assert_eq!(minted.created, Some(true), "a minted instance is Created");
        let deck: Vec<String> = (1..=DECK_SIZE).map(|n| format!("core-{n:03}")).collect();
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: "benders-created-shipped".to_string(),
            decks: (deck.clone(), deck),
            ..Default::default()
        }))
        .state;
        // The Coin is dealt once both mulligans are answered (R244).
        for player in PLAYER_IDS {
            let keep: Vec<String> = state.players[player]
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect();
            let action = Action::new(ActionBody::Mulligan { keep }, player, format!("created-{player}"));
            let result = reduce(&state, &action);
            assert_eq!(result.error, None);
            state = result.state;
        }
        let cards: Vec<&CardInstance> = PLAYER_IDS
            .iter()
            .flat_map(|player| {
                state.players[*player]
                    .hand
                    .iter()
                    .chain(&state.players[*player].library)
            })
            .collect();
        let coins: Vec<&&CardInstance> = cards.iter().filter(|card| card.def_id == COIN_DEF_ID).collect();
        assert_eq!(coins.len(), 1, "the second seat holds The Coin");
        assert_eq!(coins[0].created, Some(true), "The Coin is Created");
        assert!(
            cards
                .iter()
                .filter(|card| card.def_id != COIN_DEF_ID)
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
        reset_instance(s.card_mut(&id));
        assert_eq!(s.card(&id).created, Some(true), "a reset keeps the mark");
        let mut sink = Bench::new(s.state());
        {
            let state = s.state_mut();
            let mut engine = sink.sink(state);
            draw(&mut engine, P1, 1);
        }
        let drawn = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == VANILLA)
            .expect("drawn");
        assert_eq!(drawn.created, Some(true), "a move keeps the mark");
        let value = serde_json::to_value(&drawn).expect("serializes");
        assert_eq!(value.get("created"), Some(&json!(true)));
        let back: CardInstance = serde_json::from_value(value).expect("deserializes");
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
        let mut sink = Bench::new(s.state());
        {
            let state = s.state_mut();
            let mut engine = sink.sink(state);
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
        let mut sink = Bench::new(s.state());
        {
            let state = s.state_mut();
            let mut engine = sink.sink(state);
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
        // Felinor Fiender has Stack (§3.2), so each play below goes onto the taken lane-2 zone.
        const FIENDER: &str = "core-092";
        let mut s = scenario(json!({
            "seed": "benders-watch",
            "p1": {
                "mana": 10,
                "hand": [{ "def": "meditative-098", "radiant": true }, { "def": FIENDER }, { "def": FIENDER }, { "def": "core-069" }, FILLER],
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
        s.play(FIENDER, json!({ "zone": 2 }));
        assert_eq!(s.unit(P1, 2).expect("the pile's top").def_id, FIENDER);
        assert_eq!(books(&s), 2, "one Book for the played card");
        // … and next turn the watch is gone.
        s.end_turn();
        s.end_turn();
        s.play(FIENDER, json!({ "zone": 2 }));
        assert_eq!(books(&s), 2, "no Book after the turn");
    }
}

mod r831008 {
    use super::*;

    const PAWN: &str = "core-096"; // My Pawn, a Trap, set face down.

    #[test]
    fn r831008_lane_is_empty_counts_every_zone() {
        // One card in each of the four zones a lane has, in lanes 1 to 4 (a face-down Trap occupies its
        // zone as any card does). Lane 5 alone is empty.
        jackioh_cards::register_all();
        let s = scenario(json!({
            "p1": {
                "field": [{ "def": VANILLA, "lane": 1 }],
                "backrow": [{ "def": WELL, "faceUp": true, "lane": 2 }],
                "library": library(),
            },
            "p2": {
                "field": [{ "def": VANILLA, "lane": 3 }],
                "backrow": [{ "def": PAWN, "lane": 4 }],
                "library": library(),
            },
        }));
        let empty: Vec<i32> = (1..=5).filter(|lane| lane_is_empty(s.state(), *lane)).collect();
        assert_eq!(empty, vec![5]);
    }
}

mod r1956001 {
    use super::*;
    use jackioh_engine::effects::{
        AddToHandArgs, FlickerArgs, PlayerSpec, SummonArgs, add_to_hand, flicker, summon,
    };
    use jackioh_engine::resolve::apply_effects;
    use jackioh_engine::testkit::golden;

    const MR_TOKEN: &str = "core-015"; // Me and Mr Token: "Cry: Summon a Rush Token."
    const SHEEPISH: &str = "core-041"; // A Trap, set face down.

    /// Run `effect` as `maker`'s script, its controller's.
    fn run_as(s: &mut Scenario, maker: &CardInstance, effect: Effect) {
        let mut sink = Bench::new(s.state());
        let state = s.state_mut();
        let mut engine = sink.sink(state);
        let mut ctx = make_context(
            &mut engine,
            Some(maker),
            HookOptions {
                controller: Some(maker.controller),
                ..Default::default()
            },
        );
        apply_effects(&[effect], &mut ctx);
    }

    fn generator(s: &Scenario, id: &str) -> Option<String> {
        s.state().generators.get(id).cloned()
    }

    fn created_in_hand(s: &Scenario, player: PlayerId) -> CardInstance {
        s.hand(player)
            .into_iter()
            .find(|card| card.created == Some(true))
            .expect("a Created card in the hand")
    }

    fn json_text(view: &PlayerView) -> String {
        serde_json::to_string(view).expect("a view serializes")
    }

    #[test]
    fn r1956001_a_token_a_cry_summons_carries_the_card_that_summoned_it() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "benders-generator-cry",
            "p1": { "mana": 10, "hand": [MR_TOKEN], "library": library() },
            "p2": { "hand": [FILLER], "library": library() },
        }));
        s.play(MR_TOKEN, json!({ "zone": 1 }));
        let played = s.unit(P1, 1).expect("Me and Mr Token");
        let token = s.unit(P1, 2).expect("its Rush Token");
        assert_eq!(token.created, Some(true));
        assert_eq!(generator(&s, &token.id).as_deref(), Some(MR_TOKEN));
        assert_eq!(generator(&s, &played.id), None, "a dealt card has no generator");
        let own = s.view(P1);
        assert_eq!(
            own.you.units[1]
                .as_ref()
                .and_then(|unit| unit.created_by.as_deref()),
            Some(MR_TOKEN)
        );
        assert_eq!(
            own.you.units[0].as_ref().and_then(|unit| unit.created_by.clone()),
            None
        );
        let foe = s.view(P2);
        let seen = foe.opponent.units[1].as_ref().expect("the token, public");
        assert_eq!(seen.created, Some(true), "Created is public on the field (R943)");
        assert_eq!(seen.created_by, None, "its generator is its owner's alone");
    }

    #[test]
    fn r1956001_a_created_trap_keeps_its_generator_through_each_fresh_face_down_id() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "benders-generator-face-down",
            "p1": { "mana": 10, "hand": [VANILLA], "library": library() },
            "p2": { "hand": [FILLER], "library": library() },
        }));
        let a = s.hand(P1)[0].clone();
        run_as(
            &mut s,
            &a,
            add_to_hand(AddToHandArgs {
                def_id: Some(SHEEPISH.to_string()),
                ..Default::default()
            }),
        );
        let made = created_in_hand(&s, P1);
        assert_eq!(generator(&s, &made.id).as_deref(), Some(VANILLA));

        // Set face-down from hand, it takes a fresh id (R227), and its generator follows it.
        s.play(made.id.as_str(), json!({ "zone": 2 }));
        let set = s.backrow(P1, 2).expect("the Trap, set");
        assert_ne!(set.id, made.id, "a face-down card takes a fresh id");
        assert_eq!(generator(&s, &set.id).as_deref(), Some(VANILLA));
        assert_eq!(generator(&s, &made.id), None, "the id it had names nothing");
        let own = s.view(P1);
        let Some(Some(BackrowView::Public(own_trap))) = own.you.backrow.get(1) else {
            panic!("the owner reads its own Trap")
        };
        assert_eq!(own_trap.created_by.as_deref(), Some(VANILLA));

        // Flickered, it re-enters face-down with another fresh id, still A's.
        run_as(&mut s, &set, flicker(FlickerArgs::default()));
        let back = s.backrow(P1, 2).expect("the Trap, flickered");
        assert_ne!(back.id, set.id, "a flickered Trap takes a fresh id");
        assert_eq!(generator(&s, &back.id).as_deref(), Some(VANILLA));
        assert_eq!(generator(&s, &set.id), None);

        // What the Trap makes carries A, the root, and not the Trap.
        run_as(
            &mut s,
            &back,
            summon(SummonArgs {
                def_id: Some(PLAIN.to_string()),
                ..Default::default()
            }),
        );
        let unit = s.unit(P1, 1).expect("the unit the Trap made");
        assert_eq!(generator(&s, &unit.id).as_deref(), Some(VANILLA));
    }

    #[test]
    fn r1956001_a_created_card_carries_its_root_generator_and_only_its_owner_is_told() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "benders-generators",
            "p1": { "hand": [VANILLA], "library": library() },
            "p2": { "hand": [FILLER], "library": library() },
        }));
        let a = s.hand(P1)[0].clone();
        // A makes a card in its owner's hand: Created, by A.
        run_as(
            &mut s,
            &a,
            add_to_hand(AddToHandArgs {
                def_id: Some(FILLER.to_string()),
                ..Default::default()
            }),
        );
        let made = created_in_hand(&s, P1);
        assert_eq!(generator(&s, &made.id).as_deref(), Some(VANILLA));
        // The card A made summons a unit and sets a Trap: each carries A, the root, and never the card
        // between.
        for def_id in [PLAIN, SHEEPISH] {
            run_as(
                &mut s,
                &made,
                summon(SummonArgs {
                    def_id: Some(def_id.to_string()),
                    ..Default::default()
                }),
            );
        }
        let unit = s.unit(P1, 1).expect("the unit");
        let trap = s.backrow(P1, 1).expect("the Trap");
        assert_eq!(generator(&s, &unit.id).as_deref(), Some(VANILLA));
        assert_eq!(generator(&s, &trap.id).as_deref(), Some(VANILLA));
        run_as(
            &mut s,
            &a,
            add_to_hand(AddToHandArgs {
                def_id: Some(FILLER.to_string()),
                player: Some(PlayerSpec::Enemy),
                ..Default::default()
            }),
        );
        let given = created_in_hand(&s, P2);
        assert_eq!(
            generator(&s, &given.id),
            None,
            "a card given away records nothing"
        );
        assert_eq!(generator(&s, &a.id), None, "a dealt card records nothing");

        // The owner is told on every card it reads: in its hand, on the field and in its backrow.
        let own = s.view(P1);
        let HandView::Cards(hand) = &own.you.hand else {
            panic!("the owner's hand is cards")
        };
        let in_hand = |id: &str| {
            hand.iter()
                .find(|card| card.instance_id == id)
                .and_then(|card| card.created_by.clone())
        };
        assert_eq!(in_hand(&made.id).as_deref(), Some(VANILLA));
        assert_eq!(in_hand(&a.id), None, "a card nothing generated carries none");
        assert_eq!(
            own.you.units[0]
                .as_ref()
                .and_then(|unit| unit.created_by.as_deref()),
            Some(VANILLA)
        );
        let Some(Some(BackrowView::Public(own_trap))) = own.you.backrow.first() else {
            panic!("the owner reads its own Trap")
        };
        assert_eq!(own_trap.created_by.as_deref(), Some(VANILLA));
        assert!(json_text(&own).contains("createdBy"));

        // The other player is told nothing.
        let foe = s.view(P2);
        let seen = foe.opponent.units[0].as_ref().expect("the unit is public");
        assert_eq!(seen.created, Some(true), "Created is public on the field (R943)");
        assert_eq!(seen.created_by, None);
        assert!(matches!(foe.opponent.hand, HandView::Count { .. }));
        assert!(matches!(
            foe.opponent.backrow.first(),
            Some(Some(BackrowView::FaceDown(_)))
        ));
        assert!(!json_text(&foe).contains("createdBy"));

        // It plays no part in the game: the hash and the golden views leave it out, and a JSON round
        // trip keeps it.
        let mut bare = s.state().clone();
        bare.generators.clear();
        assert_eq!(hash_state(s.state()), hash_state(&bare));
        assert_eq!(
            golden::hashed_state_text(s.state()),
            golden::hashed_state_text(&bare)
        );
        let texts = golden::view_texts(s.state());
        assert_eq!(texts, golden::view_texts(&bare));
        assert!(!texts[0].contains("createdBy"));
        let back: GameState =
            serde_json::from_value(serde_json::to_value(s.state()).expect("a state serializes"))
                .expect("and deserializes");
        assert_eq!(back.generators, s.state().generators);
    }
}
