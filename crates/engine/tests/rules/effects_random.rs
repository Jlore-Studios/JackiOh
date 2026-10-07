//! The rng verbs and the two subsystem wrappers
//! (SPEC §6.1 Lucky X, §6.3 Fuse/Rotate/Make Radiant, §3.1's rotation-topology ruling, §10.4, §10.7;
//! R14, R32, R60, R77, R88, R102; BUILD M3-T1).
//!
//! Four verbs share this file because they share one question: does the verb delegate, and does it
//! take exactly the draws it is supposed to?
//!
//!   * `flipCoins` (#4 Gary the Gambler) and `radiantChance` (#42 Eugenics) own randomness, so every
//!     test here counts DRAWS as well as outcomes. §10.7 stores `rngCursor` in state, which makes
//!     the cursor part of the match: a verb that took a draw only sometimes would make every later
//!     draw in the game depend on the board at that moment, and §9.3's exact replay would be gone.
//!     So "how many draws" is a rule, not an implementation detail, and it is asserted directly.
//!   * `fuseCards` (#99 Craft a Card, #85 Unlicensed Experimentation) and `rotate` (#52 Silly Silas)
//!     own nothing: R77 and R14 live in `subsystems/fuse.ts` and `subsystems/rotation.ts`, which
//!     have their own test files. The tests here prove the wrapper reaches them — each one leans on
//!     a rule ONLY the subsystem implements (the capped fused cost, the Locked-destination bounce),
//!     so a wrapper that reimplemented the walk would fail them.
//!
//! The golden numbers are the coin and chance sequences of the named seeds at cursor 0. None of
//! these tests call `beginGame`, so nothing has drawn before them and the cursor really is 0 (the
//! same convention as `effects-radiant.test.ts`).
//!
//! Port of `packages/engine/test/effects-random.test.ts`.

use jackioh_engine::testkit::*;

use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::config::FUSE_COST_CAP;
use jackioh_engine::effects::{exile, flip_coin_keyword, flip_coins, fuse_cards, radiant_chance, rotate};
use jackioh_engine::layers::unit_view;
use jackioh_engine::mana::effective_cost;
use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{Effect, EngineSink};
use jackioh_engine::state::{CardInstance, GameState, find_instance, find_instance_mut};
use jackioh_engine::zones::{card_at, lock_zone};

use super::fixtures::harness::{new_game, put, set_library, slot};

// ---------------------------------------------------------------------------
// Fixture cards. Indices start above 1600 so they never collide with another test file's locals.
// ---------------------------------------------------------------------------

/// TS `unitDefOf(name, overrides)`, numbered from a module counter starting at 1600 in declaration
/// order (gary 1601, body 1602, backdrop 1603, alpha 1604, beta 1605); the number is passed here.
fn unit_def_of(name: &str, index: i32, overrides: Value) -> CardDef {
    let mut def = json!({
        "id": format!("rn-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (random)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 1, "health": 1, "keywords": [], "text": name },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": format!("{name} radiant") },
    });
    if let (Some(base), Some(overrides)) = (def.as_object_mut(), overrides.as_object()) {
        for (key, value) in overrides {
            base.insert(key.clone(), value.clone());
        }
    }
    json_as(def)
}

/// #4 Gary himself: a 1/1 whose radiant face is 2/2, so a coin buff is readable on top of it.
fn gary() -> CardDef {
    unit_def_of("gary", 1601, json!({}))
}

/// Plain bodies for the library pool and the rotation ring.
fn body() -> CardDef {
    unit_def_of("body", 1602, json!({}))
}

/// A backrow card for the rotation's second ring: a Unit never stands in a backrow zone (§3.2, R446).
fn backdrop() -> CardDef {
    unit_def_of(
        "backdrop",
        1603,
        json!({
            "type": "Field Spell",
            "base": { "keywords": [], "text": "backdrop" },
            "radiant": { "keywords": [], "text": "backdrop radiant" },
        }),
    )
}

/// #99's two Discover picks: distinct costs, stats, keywords and tags, so every R77 sum shows.
fn alpha() -> CardDef {
    unit_def_of(
        "alpha",
        1604,
        json!({
            "cost": 3,
            "tags": ["Human"],
            "rarity": "Rare",
            "base": { "attack": 2, "health": 3, "keywords": [{ "kind": "Taunt" }], "text": "alpha" },
            "radiant": { "attack": 4, "health": 6, "keywords": [{ "kind": "Taunt" }], "text": "alpha radiant" },
        }),
    )
}

fn beta() -> CardDef {
    unit_def_of(
        "beta",
        1605,
        json!({
            "cost": 2,
            "tags": ["Felinor"],
            "base": { "attack": 1, "health": 1, "keywords": [{ "kind": "Rush" }], "text": "beta" },
            "radiant": { "attack": 2, "health": 2, "keywords": [{ "kind": "Rush" }], "text": "beta radiant" },
        }),
    )
}

fn rn_defs() -> Vec<CardDef> {
    vec![gary(), body(), backdrop(), alpha(), beta()]
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in rn_defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state.turn = 3;
    state.active = PlayerId::P1;
    state
}

/// TS `run`'s `options: { self?, controller?, radiant? }`.
#[derive(Default)]
struct RunOptions {
    self_: Option<CardInstance>,
    controller: Option<PlayerId>,
    radiant: Option<bool>,
}

/// What TS's `run` handed back (the sink): the events and the cursor the run left behind.
struct Ran {
    events: Vec<GameEvent>,
    cursor: u32,
}

/// Apply effects the way `resolve.ts` does and hand back the sink, so a test can read both the
/// events and the rng cursor the run left behind.
fn run(state: &mut GameState, effects: &[Effect], options: RunOptions) -> Ran {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            options.self_.as_ref(),
            HookOptions {
                controller: options.controller,
                ..Default::default()
            },
        );
        // TS `{ ...ctx, radiant }`: the same context with the running face overridden.
        if let Some(radiant) = options.radiant {
            ctx.radiant = radiant;
        }
        apply_effects(effects, &mut ctx);
    }
    state.rng_cursor = rng.cursor();
    Ran {
        events,
        cursor: rng.cursor(),
    }
}

/// Draws taken by one run, which is the only honest way to say "this verb rolled N times".
fn draws(ran: &Ran) -> u32 {
    ran.cursor
}

fn with_self(card: &CardInstance) -> RunOptions {
    RunOptions {
        self_: Some(card.clone()),
        ..Default::default()
    }
}

fn as_p1() -> RunOptions {
    RunOptions {
        controller: Some(PlayerId::P1),
        ..Default::default()
    }
}

/// `fixtures/harness.ts`'s `eventsOfType`, as JSON: the events of one type, each as TS writes it.
fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == kind)
        .collect()
}

fn ids_of_type(events: &[GameEvent], kind: &str) -> Vec<String> {
    of_type(events, kind)
        .iter()
        .map(|event| event["instanceId"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// The card under `id` as it stands in the state now (TS read the live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

/// `toMatchObject` on a unit view's `attack` and `maxHealth`.
fn stats_of(state: &GameState, card: &CardInstance) -> (i32, i32) {
    let view = unit_view(state, live(state, &card.id));
    (view.attack, view.max_health)
}

fn transient_ids(state: &GameState) -> Vec<String> {
    state.transient_defs.keys().cloned().collect()
}

fn bodies(n: usize) -> Vec<&'static str> {
    vec!["rn-body"; n]
}

// ---------------------------------------------------------------------------
// flipCoins (#4 Gary the Gambler)
// ---------------------------------------------------------------------------

fn coins(args: Value) -> Effect {
    flip_coins(json_as(args))
}

mod r32_flip_coins_s8_1_c4_s10_4_layer_4_s10_7 {
    use super::*;

    #[test]
    fn s8_1_takes_exactly_coins_draws_and_buffs_plus_1_attack_per_heads_plus_1_max_health_per_tails() {
        let mut state = game("coins-known");
        let unit = put(
            &mut state,
            &gary().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        assert_eq!(stats_of(&state, &unit), (1, 1));

        let ran = run(
            &mut state,
            &[coins(json!({
                "target": { "of": "self" }, "coins": 5, "perHeads": { "attack": 1 }, "perTails": { "health": 1 }
            }))],
            with_self(&unit),
        );

        // Five flips, no more and no fewer: the cursor is state, so this is a rule (§10.7).
        assert_eq!(draws(&ran), 5);
        // "coins-known" at cursor 0 flips T H H H H: four heads, one tail.
        let buffs = live(&state, &unit.id).buffs;
        assert_eq!(buffs, AttackHealth { attack: 4, health: 1 });
        // Heads and tails always account for every flip.
        assert_eq!(buffs.attack + buffs.health, 5);
        // §10.4: the buff is layer 4, so the totals are computed on read, on top of the printed 1/1.
        assert_eq!(stats_of(&state, &unit), (5, 2));
        // One `buffed` event carrying the whole result (§10.10 animates it).
        assert_eq!(
            of_type(&ran.events, "buffed"),
            vec![json!({ "type": "buffed", "instanceId": unit.id, "attack": 4, "health": 1 })]
        );
    }

    #[test]
    fn s8_1_the_radiant_face_is_the_same_flip_at_7_coins_and_2_a_side() {
        let mut state = game("coins-known");
        let unit = put(
            &mut state,
            &gary().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({ "radiant": true }),
        );

        let ran = run(
            &mut state,
            &[coins(json!({
                "target": { "of": "self" }, "coins": 7, "perHeads": { "attack": 2 }, "perTails": { "health": 2 }
            }))],
            with_self(&unit),
        );

        // "coins-known" at cursor 0 flips T H H H H H T: five heads, two tails.
        assert_eq!(draws(&ran), 7);
        assert_eq!(
            live(&state, &unit.id).buffs,
            AttackHealth {
                attack: 10,
                health: 4
            }
        );
        // On the 2/2 radiant face (§5.2), read through the layers.
        assert_eq!(stats_of(&state, &unit), (12, 6));
    }

    #[test]
    fn s10_7_takes_no_draws_at_all_when_the_target_is_missing_so_the_cursor_is_untouched() {
        let mut state = game("coins-known");
        assert_eq!(state.rng_cursor, 0);

        // `{ of: "self" }` with no self, and `{ of: "chosen" }` with nothing chosen: both fizzle.
        let no_self = run(
            &mut state,
            &[coins(
                json!({ "target": { "of": "self" }, "coins": 5, "perHeads": { "attack": 1 } }),
            )],
            as_p1(),
        );
        assert_eq!(draws(&no_self), 0);
        assert!(no_self.events.is_empty());

        let no_pick = run(
            &mut state,
            &[coins(
                json!({ "target": { "of": "chosen" }, "coins": 7, "perTails": { "health": 2 } }),
            )],
            as_p1(),
        );
        assert_eq!(draws(&no_pick), 0);
        assert_eq!(state.rng_cursor, 0);

        // And a fizzle really is total: a unit that arrives afterwards gets the draws that were saved.
        let unit = put(
            &mut state,
            &gary().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        run(
            &mut state,
            &[coins(json!({
                "target": { "of": "self" }, "coins": 5, "perHeads": { "attack": 1 }, "perTails": { "health": 1 }
            }))],
            with_self(&unit),
        );
        assert_eq!(
            live(&state, &unit.id).buffs,
            AttackHealth { attack: 4, health: 1 }
        );
    }

    #[test]
    fn s9_3_the_same_seed_and_cursor_give_the_same_flips_twice_and_another_seed_does_not() {
        let flips = |seed: &str, cursor: u32| -> AttackHealth {
            let mut state = game(seed);
            state.rng_cursor = cursor;
            let unit = put(
                &mut state,
                &gary().id,
                slot(PlayerId::P1, Row::Units, 1),
                json!({}),
            );
            run(
                &mut state,
                &[coins(json!({
                    "target": { "of": "self" }, "coins": 5, "perHeads": { "attack": 1 }, "perTails": { "health": 1 }
                }))],
                with_self(&unit),
            );
            live(&state, &unit.id).buffs
        };

        assert_eq!(flips("coins-a", 0), flips("coins-a", 0));
        assert_ne!(flips("coins-a", 0), flips("coins-b", 0));
        // The cursor is the other half of the pair: the same seed further along reads other draws.
        // ("coins-a" flips H H T T T from cursor 0 and T H H T H from cursor 5.)
        assert_eq!(flips("coins-a", 0), AttackHealth { attack: 2, health: 3 });
        assert_eq!(flips("coins-a", 5), AttackHealth { attack: 3, health: 2 });
    }

    #[test]
    fn s6_3_a_card_that_pays_nothing_for_a_side_of_the_coin_still_flips_it() {
        let mut state = game("coins-a");
        let unit = put(
            &mut state,
            &gary().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );

        // Heads only: "coins-a" flips H H T T T, so two heads and three ignored tails.
        let ran = run(
            &mut state,
            &[coins(
                json!({ "target": { "of": "self" }, "coins": 5, "perHeads": { "attack": 1 } }),
            )],
            with_self(&unit),
        );

        assert_eq!(draws(&ran), 5);
        assert_eq!(
            live(&state, &unit.id).buffs,
            AttackHealth { attack: 2, health: 0 }
        );
    }
}

// ---------------------------------------------------------------------------
// flipCoinKeyword (#4 Gary the Gambler's rider)
// ---------------------------------------------------------------------------

mod r32_r130_flip_coin_keyword_s8_1_c4_s10_4_s10_7 {
    use super::*;

    fn rider() -> Effect {
        flip_coin_keyword(json_as(json!({
            "target": { "of": "self" },
            "headsKeyword": { "kind": "Divine Shield" },
            "tailsKeyword": { "kind": "Rush" },
        })))
    }

    #[test]
    fn s8_1_takes_exactly_one_draw_tails_at_cursor_0_grants_rush() {
        let mut state = game("coins-known");
        let unit = put(
            &mut state,
            &gary().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );

        let ran = run(&mut state, &[rider()], with_self(&unit));

        // "coins-known" at cursor 0 flips T H H H H: the first flip is tails.
        assert_eq!(draws(&ran), 1);
        assert_eq!(live(&state, &unit.id).granted_keywords, vec![Keyword::Rush]);
        // §10.4: the grant is a granted keyword, cued by one `keywordGranted` event (§10.10).
        assert_eq!(
            of_type(&ran.events, "keywordGranted"),
            vec![json!({ "type": "keywordGranted", "instanceId": unit.id, "keyword": { "kind": "Rush" } })]
        );
    }

    #[test]
    fn s8_1_heads_at_cursor_1_grants_divine_shield() {
        let mut state = game("coins-known");
        state.rng_cursor = 1;
        let unit = put(
            &mut state,
            &gary().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );

        let ran = run(&mut state, &[rider()], with_self(&unit));

        // The second flip of the "coins-known" stream is heads. The sink cursor is absolute, so
        // one draw from cursor 1 leaves it at 2.
        assert_eq!(draws(&ran) - 1, 1);
        assert_eq!(
            live(&state, &unit.id).granted_keywords,
            vec![Keyword::DivineShield]
        );
    }

    #[test]
    fn s10_7_takes_no_draws_at_all_when_the_target_is_missing_so_the_cursor_is_untouched() {
        let mut state = game("coins-known");
        assert_eq!(state.rng_cursor, 0);

        // `{ of: "self" }` with no self: the coin is never flipped.
        let ran = run(&mut state, &[rider()], as_p1());

        assert_eq!(draws(&ran), 0);
        assert!(ran.events.is_empty());
        assert_eq!(state.rng_cursor, 0);
    }

    #[test]
    fn s9_3_the_same_seed_and_cursor_grant_the_same_keyword_twice() {
        let granted = |seed: &str| -> Vec<Keyword> {
            let mut state = game(seed);
            let unit = put(
                &mut state,
                &gary().id,
                slot(PlayerId::P1, Row::Units, 1),
                json!({}),
            );
            run(&mut state, &[rider()], with_self(&unit));
            live(&state, &unit.id).granted_keywords.clone()
        };

        assert_eq!(granted("gary-rider-a"), granted("gary-rider-a"));
        assert_eq!(granted("gary-rider-a").len(), 1);
    }
}

// ---------------------------------------------------------------------------
// radiantChance (#42 Eugenics)
// ---------------------------------------------------------------------------

/// The library cards that came out Radiant, by index in the library as it was set up.
fn radiant_at(state: &GameState, cards: &[CardInstance]) -> Vec<usize> {
    cards
        .iter()
        .enumerate()
        .filter(|(_, card)| find_instance(state, &card.id).is_some_and(|now| now.radiant))
        .map(|(at, _)| at)
        .collect()
}

fn chance(args: Value) -> Effect {
    radiant_chance(json_as(args))
}

mod r32_r60_radiant_chance_s8_2_c42_s6_1_lucky_x_s10_7 {
    use super::*;

    #[test]
    fn s8_2_rolls_once_per_non_radiant_library_card_and_flags_the_ones_that_hit() {
        let mut state = game("eug-b");
        let library = set_library(&mut state, PlayerId::P1, &bodies(6));

        let ran = run(
            &mut state,
            &[chance(json!({ "zone": "library", "chance": 0.3 }))],
            as_p1(),
        );

        // One independent roll per card in the pool: six cards, six draws (§10.7).
        assert_eq!(draws(&ran), 6);
        // "eug-b" at cursor 0 puts three of its first six draws under 0.3, at library indices 1, 3, 4.
        assert_eq!(radiant_at(&state, &library), vec![1, 3, 4]);
        assert_eq!(
            ids_of_type(&ran.events, "radiantSet"),
            [1usize, 3, 4]
                .iter()
                .map(|&at| library[at].id.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn r177_s8_c42_rolls_every_remaining_card_a_radiant_one_too_and_cues_every_success_on_a_hidden_card() {
        let mut state = game("eug-b");
        let library = set_library(&mut state, PlayerId::P1, &bodies(6));
        // Flag two of them up front. #42 is no random pick (R60), so they are rolled like the rest.
        let already_radiant: [usize; 2] = [0, 2];
        for at in already_radiant {
            if let Some(card) = library.get(at) {
                find_instance_mut(&mut state, &card.id)
                    .expect("a library card")
                    .radiant = true;
            }
        }

        let ran = run(
            &mut state,
            &[chance(json!({ "zone": "library", "chance": 1 }))],
            as_p1(),
        );

        // Six rolls for the six cards: how many draws the effect takes does not hang on how many of a
        // library nobody may read were Radiant already (§9.1).
        assert_eq!(draws(&ran), 6);
        // At chance 1 every roll hits, and a library card is hidden from both seats, so every success is
        // cued, changed or not (R177): the cues cannot count the Radiant ones either.
        assert!(library.iter().all(|card| live(&state, &card.id).radiant));
        assert_eq!(
            ids_of_type(&ran.events, "radiantSet"),
            library.iter().map(|card| card.id.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn s6_1_lucky_1_takes_two_rolls_a_card_and_keeps_the_success_so_c42s_radiant_face_flags_more() {
        let plain = || -> (Ran, GameState, Vec<CardInstance>) {
            let mut state = game("eug-b");
            let library = set_library(&mut state, PlayerId::P1, &bodies(6));
            let ran = run(
                &mut state,
                &[chance(json!({ "zone": "library", "chance": 0.3 }))],
                as_p1(),
            );
            (ran, state, library)
        };
        let lucky = || -> (Ran, GameState, Vec<CardInstance>) {
            let mut state = game("eug-b");
            let library = set_library(&mut state, PlayerId::P1, &bodies(6));
            let ran = run(
                &mut state,
                &[chance(json!({ "zone": "library", "chance": 0.4, "lucky": 1 }))],
                as_p1(),
            );
            (ran, state, library)
        };

        let (base_ran, base_state, base_library) = plain();
        let (face_ran, face_state, face_library) = lucky();

        // The draw count is the hard property: (lucky + 1) rolls per card, so twice as many.
        assert_eq!(draws(&base_ran), 6);
        assert_eq!(draws(&face_ran), 12);

        // On this seed #42's radiant face ("Lucky 1 at 40%") really does flag strictly more than its
        // base face ("30%"): 5 cards against 3. That is a per-seed fact, not a theorem — the two faces
        // read different draws off the same stream, so no seed-independent ordering exists to assert.
        assert_eq!(radiant_at(&base_state, &base_library), vec![1, 3, 4]);
        assert_eq!(radiant_at(&face_state, &face_library), vec![0, 1, 2, 3, 4]);
        assert!(radiant_at(&face_state, &face_library).len() > radiant_at(&base_state, &base_library).len());
    }

    #[test]
    fn s6_1_lucky_takes_its_extra_rolls_but_never_invents_a_success() {
        let mut state = game("eug-c");
        let library = set_library(&mut state, PlayerId::P1, &bodies(4));

        // Nothing can hit at chance 0, however many rolls are kept — and the rolls still happen.
        let ran = run(
            &mut state,
            &[chance(json!({ "zone": "library", "chance": 0, "lucky": 10 }))],
            as_p1(),
        );

        assert_eq!(draws(&ran), 4 * 11);
        assert!(!library.iter().any(|card| live(&state, &card.id).radiant));
        assert!(ran.events.is_empty());
    }

    #[test]
    fn s8_2_a_card_an_earlier_effect_in_the_same_list_exiled_is_never_rolled() {
        let mut state = game("eug-b");
        let library = set_library(&mut state, PlayerId::P1, &bodies(6));
        let doomed = library.get(2).cloned().expect("expected a third library card");

        // #42's own order: the exile runs first and the chance rolls over what is LEFT. §4.5 and R59
        // put the state check after the whole list, never between two of its effects.
        let ran = run(
            &mut state,
            &[
                exile(json_as(
                    json!({ "target": { "of": "instance", "instanceId": doomed.id } }),
                )),
                chance(json!({ "zone": "library", "chance": 1 })),
            ],
            as_p1(),
        );

        let doomed_now = live(&state, &doomed.id);
        assert_eq!(doomed_now.zone, Zone::Exile { player: PlayerId::P1 });
        // Five cards left, so five rolls: the exiled card cost the pool a draw as well as a flag.
        assert_eq!(draws(&ran), 5);
        assert!(!doomed_now.radiant);
        assert!(state.players.p1.library.iter().all(|card| card.radiant));
        assert!(!ids_of_type(&ran.events, "radiantSet").contains(&doomed.id));
    }

    #[test]
    fn s8_2_nothing_is_rolled_and_no_draw_is_taken_when_the_zone_holds_no_non_radiant_card() {
        let mut state = game("eug-d");
        set_library(&mut state, PlayerId::P1, &[] as &[&str]);

        let ran = run(
            &mut state,
            &[chance(json!({ "zone": "library", "chance": 0.4, "lucky": 1 }))],
            as_p1(),
        );

        assert_eq!(draws(&ran), 0);
        assert!(ran.events.is_empty());
    }

    #[test]
    fn reads_the_player_the_effect_names_so_enemy_rolls_the_opponents_library() {
        let mut state = game("eug-b");
        let mine = set_library(&mut state, PlayerId::P1, &bodies(3));
        let theirs = set_library(&mut state, PlayerId::P2, &bodies(3));

        run(
            &mut state,
            &[chance(
                json!({ "zone": "library", "chance": 1, "player": "enemy" }),
            )],
            as_p1(),
        );

        assert!(theirs.iter().all(|card| live(&state, &card.id).radiant));
        assert!(!mine.iter().any(|card| live(&state, &card.id).radiant));
    }
}

// ---------------------------------------------------------------------------
// fuseCards (#99 Craft a Card, #85 Unlicensed Experimentation)
// ---------------------------------------------------------------------------

/// The events a zone change makes; a phantom ingredient must produce none of them.
const ZONE_EVENTS: &[&str] = &[
    "addedToHand",
    "enteredGraveyard",
    "exiled",
    "burned",
    "bounced",
    "summoned",
    "destroyed",
];

fn fusing(args: Value) -> Effect {
    fuse_cards(json_as(args))
}

mod r77_r102_fuse_cards_s6_3_fuse_s8_5_c99_s8_4_c85 {
    use super::*;

    #[test]
    fn r77_c85_radiant_fuses_one_ingredient_onto_every_target_one_fusion_at_a_time() {
        let mut state = game("fuse-each");
        // #85's played permanent, plus two of the controller's matching permanents.
        let played = put(
            &mut state,
            &alpha().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let first = put(
            &mut state,
            &beta().id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        let second = put(
            &mut state,
            &beta().id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );

        run(
            &mut state,
            &[fusing(
                json!({ "instanceIds": [played.id], "targetInstanceIds": [first.id, second.id] }),
            )],
            as_p1(),
        );

        // R77: each target is its own fusion with its own transient definition. The second fusion is
        // the whole point — the played card ceased to exist in the first one, and an ingredient only
        // ever contributes its DEFINITION, so it still fuses. Re-resolving it by id would have found
        // nothing, dropped below FUSE_MIN_INGREDIENTS and silently changed nothing.
        let mut keys = transient_ids(&state);
        keys.sort();
        assert_eq!(keys, vec!["t-1:rn-alpha+rn-beta", "t-2:rn-alpha+rn-beta"]);

        // Both targets survived as the fused cards, in their own lanes (R77 keeps the instance).
        let kept_first = card_at(&state, slot(PlayerId::P1, Row::Units, 2)).cloned();
        let kept_second = card_at(&state, slot(PlayerId::P1, Row::Units, 3)).cloned();
        assert_eq!(kept_first.as_ref().map(|c| c.id.clone()), Some(first.id.clone()));
        assert_eq!(
            kept_second.as_ref().map(|c| c.id.clone()),
            Some(second.id.clone())
        );
        // Each kept card now IS a fused card, and the two fusions are distinct definitions.
        assert_ne!(kept_first.as_ref().map(|c| c.def_id.clone()), Some(beta().id));
        assert_ne!(kept_second.as_ref().map(|c| c.def_id.clone()), Some(beta().id));
        assert_ne!(
            kept_first.as_ref().map(|c| c.def_id.clone()),
            kept_second.as_ref().map(|c| c.def_id.clone())
        );

        // Both fusions summed alpha onto beta, so both read the same R77 total (2+1 / 3+1).
        for kept in [kept_first, kept_second] {
            let kept = kept.expect("expected a kept instance");
            let view = unit_view(&state, &kept);
            assert_eq!(view.attack, 3);
            assert_eq!(view.max_health, 4);
        }

        // The played permanent is gone exactly once, with no death (R77).
        assert!(find_instance(&state, &played.id).is_none());
        assert!(of_type(&run(&mut state, &[], as_p1()).events, "destroyed").is_empty());
    }

    #[test]
    fn s8_4_c85_base_picks_one_target_at_random_taking_exactly_one_draw_inside_apply() {
        let mut state = game("fuse-random");
        let played = put(
            &mut state,
            &alpha().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let first = put(
            &mut state,
            &beta().id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        let second = put(
            &mut state,
            &beta().id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );

        // Building the effect must take no draw: the pick belongs inside apply (§9.3, §10.7).
        let effect = fusing(json!({
            "instanceIds": [played.id],
            "targetInstanceIds": [first.id, second.id],
            "pick": "random",
        }));
        assert_eq!(state.rng_cursor, 0);

        let ran = run(&mut state, &[effect], as_p1());

        assert_eq!(draws(&ran), 1);
        // Exactly ONE fusion happened, so exactly one target was consumed into a fused card.
        assert_eq!(transient_ids(&state), vec!["t-1:rn-alpha+rn-beta"]);
        let fused_count = [&first, &second]
            .iter()
            .filter(|card| live(&state, &card.id).def_id != beta().id)
            .count();
        assert_eq!(fused_count, 1);
    }

    #[test]
    fn s6_3_takes_no_draw_and_fuses_nothing_when_every_named_target_has_left_the_field() {
        let mut state = game("fuse-none");
        let played = put(
            &mut state,
            &alpha().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let victim = put(
            &mut state,
            &beta().id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        let gone_id = victim.id.clone();
        // The target leaves before the fusion resolves (R61: the trap fired and did nothing). TS also
        // set the detached object's zone to `{ z: "gone" }`, which no pile holds; the state is the same.
        state.players.p1.units[1] = None;

        let ran = run(
            &mut state,
            &[fusing(
                json!({ "instanceIds": [played.id], "targetInstanceIds": [gone_id], "pick": "random" }),
            )],
            as_p1(),
        );

        assert_eq!(draws(&ran), 0);
        assert!(state.transient_defs.is_empty());
        // The ingredient is untouched: nothing was consumed for a fusion that never happened.
        assert_eq!(
            find_instance(&state, &played.id).map(|c| c.def_id.clone()),
            Some(alpha().id)
        );
    }

    #[test]
    fn r77_c99_crafts_the_fused_definition_from_two_def_ids_and_lands_it_in_the_named_hand_at_cost_0() {
        let mut state = game("fuse-craft");

        let ran = run(
            &mut state,
            &[fusing(
                json!({ "defIds": [alpha().id, beta().id], "toHand": "self" }),
            )],
            as_p1(),
        );

        // The transient definition is the subsystem's, in match state where `defOf` finds it (§10.1).
        assert_eq!(transient_ids(&state), vec!["t-1:rn-alpha+rn-beta"]);
        let fused = state
            .transient_defs
            .get("t-1:rn-alpha+rn-beta")
            .cloned()
            .expect("expected a transient def");

        // R77's sums, on BOTH faces, so Make Radiant still works on the result (§5.2).
        assert_eq!((fused.base.attack, fused.base.health), (Some(3), Some(4)));
        assert_eq!((fused.radiant.attack, fused.radiant.health), (Some(6), Some(8)));
        let mut kinds: Vec<&str> = fused.base.keywords.iter().map(|k| k.kind().as_str()).collect();
        kinds.sort();
        assert_eq!(kinds, vec!["Rush", "Taunt"]);
        let mut tags: Vec<&str> = fused.tags.iter().map(|tag| tag.as_str()).collect();
        tags.sort();
        assert_eq!(tags, vec!["Felinor", "Human"]);
        // R77's cap: 3 + 2 is 5, which is more than FUSE_COST_CAP, so the definition costs 4.
        assert_eq!(
            serde_json::to_value(fused.cost).expect("a cost serialises"),
            json!(FUSE_COST_CAP)
        );

        // "the result costs 0 and goes to your hand": a fresh, non-Radiant instance with an override,
        // so the printed 4 stands on the definition and R65 reads 0 off the card.
        assert_eq!(state.players.p1.hand.len(), 1);
        let result = state
            .players
            .p1
            .hand
            .first()
            .cloned()
            .expect("expected the crafted card in hand");
        assert_eq!(result.def_id, "t-1:rn-alpha+rn-beta");
        assert!(!result.radiant);
        assert_eq!(result.cost_override, Some(0));
        assert_eq!(effective_cost(&state, &result, Default::default()), 0);
        assert!(state.players.p2.hand.is_empty());

        // One `fused` event, naming the ingredients and the result, and nothing else the wrapper made.
        let fused_events = of_type(&ran.events, "fused");
        assert_eq!(fused_events.len(), 1);
        let event = &fused_events[0];
        assert!(event["instanceIds"].is_array());
        let mut rest = event.clone();
        if let Some(fields) = rest.as_object_mut() {
            fields.remove("instanceIds");
        }
        assert_eq!(
            rest,
            json!({ "type": "fused", "resultInstanceId": result.id, "defId": "t-1:rn-alpha+rn-beta" })
        );
    }

    #[test]
    fn r86_c99s_def_id_ingredients_sit_in_no_pile_so_no_zone_event_fires_and_nothing_can_reach_them() {
        let mut state = game("fuse-phantom");

        let ran = run(
            &mut state,
            &[fusing(
                json!({ "defIds": [alpha().id, beta().id], "toHand": "self" }),
            )],
            as_p1(),
        );

        let fused_event = of_type(&ran.events, "fused")
            .into_iter()
            .next()
            .expect("expected a fused event");
        let result = state.players.p1.hand.first().cloned();
        let result_id = result.as_ref().map(|card| card.id.clone());
        let phantoms: Vec<String> = fused_event["instanceIds"]
            .as_array()
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| id.as_str().map(str::to_string))
                    .filter(|id| Some(id) != result_id.as_ref())
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(phantoms.len(), 2);

        // `{ z: "gone" }` is in no pile, so `findInstance` cannot reach one (R11, R86).
        for id in &phantoms {
            assert!(find_instance(&state, id).is_none());
        }

        // And no zone event names one: the only card that entered a zone is the crafted result.
        let zone_events: Vec<GameEvent> = ran
            .events
            .iter()
            .filter(|event| {
                let kind = serde_json::to_value(event).expect("an event serialises")["type"].clone();
                ZONE_EVENTS.iter().any(|zone_kind| kind == *zone_kind)
            })
            .cloned()
            .collect();
        assert_eq!(
            ids_of_type(&zone_events, "addedToHand"),
            result_id.iter().cloned().collect::<Vec<_>>()
        );
        for event in &zone_events {
            let named = serde_json::to_value(event).expect("an event serialises")["instanceId"].clone();
            assert!(!phantoms.iter().any(|id| named == id.as_str()));
        }
    }

    #[test]
    fn r77_c85_keeps_the_targets_instance_when_the_ingredients_are_cards_on_the_field() {
        let mut state = game("fuse-onto");
        let victim = put(
            &mut state,
            &alpha().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let played = put(
            &mut state,
            &beta().id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        find_instance_mut(&mut state, &victim.id)
            .expect("the victim")
            .damage = 1;

        run(
            &mut state,
            &[fusing(
                json!({ "instanceIds": [played.id], "targetInstanceId": victim.id }),
            )],
            as_p1(),
        );

        // R77: the fused card IS the target, in its own zone, with its damage intact.
        let now = live(&state, &victim.id);
        assert_eq!(now.def_id, "t-1:rn-beta+rn-alpha");
        assert_eq!(
            now.zone,
            Zone::Field {
                player: PlayerId::P1,
                row: Row::Units,
                lane: 1
            }
        );
        assert_eq!(now.damage, 1);
        let view = unit_view(&state, now);
        assert_eq!((view.attack, view.max_health, view.health), (3, 4, 3));
        // The other ingredient ceased to exist: no graveyard, no death, no pile.
        assert!(card_at(&state, slot(PlayerId::P1, Row::Units, 2)).is_none());
        assert!(find_instance(&state, &played.id).is_none());
        assert!(state.players.p1.graveyard.is_empty());
    }

    #[test]
    fn s6_3_fizzles_and_changes_nothing_when_the_subsystem_refuses_the_fusion() {
        let mut state = game("fuse-fizzle");

        // R77: "Fewer is not a fusion", and a call with no target and no hand has nowhere to put one.
        let too_few = run(
            &mut state,
            &[fusing(json!({ "defIds": [alpha().id], "toHand": "self" }))],
            as_p1(),
        );
        let nowhere = run(
            &mut state,
            &[fusing(json!({ "defIds": [alpha().id, beta().id] }))],
            as_p1(),
        );

        assert!(state.transient_defs.is_empty());
        assert!(state.players.p1.hand.is_empty());
        assert!(of_type(&too_few.events, "fused").is_empty());
        assert!(of_type(&nowhere.events, "fused").is_empty());
    }
}

// ---------------------------------------------------------------------------
// rotate (#52 Silly Silas)
// ---------------------------------------------------------------------------

/// Where a card sits now, as "p2 units 5", for readable assertions.
fn where_is(state: &GameState, card: &CardInstance) -> String {
    match find_instance(state, &card.id).map(|now| now.zone.clone()) {
        None => "gone".to_string(),
        Some(Zone::Field { player, row, lane }) => format!("{} {} {lane}", player.as_str(), row.as_str()),
        Some(zone) => zone.z().as_str().to_string(),
    }
}

fn turning(direction: &str) -> Effect {
    rotate(json_as(json!({ "direction": direction })))
}

mod r14_r88_rotate_s6_3_rotate_s3_1s_rotation_topology_s8_3_c52 {
    use super::*;

    #[test]
    fn s3_1_moves_every_card_one_step_around_its_ring_control_changing_on_the_crossing() {
        let mut state = game("rotate-verb");
        let lane1 = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let lane5 = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 5),
            json!({}),
        );
        let trap = put(
            &mut state,
            &backdrop().id,
            slot(PlayerId::P1, Row::Backrow, 5),
            json!({}),
        );

        let ran = run(&mut state, &[turning("right")], as_p1());

        // The ring is the controller's lanes 1→5, then the opponent's 5→1, and back (§3.1).
        assert_eq!(where_is(&state, &lane1), "p1 units 2");
        assert_eq!(where_is(&state, &lane5), "p2 units 5");
        assert_eq!(live(&state, &lane1.id).controller, PlayerId::P1);
        assert_eq!(live(&state, &lane5.id).controller, PlayerId::P2);
        // R14: both rings turn together, so the backrow moved too.
        assert_eq!(where_is(&state, &trap), "p2 backrow 5");

        assert_eq!(
            of_type(&ran.events, "rotated"),
            vec![json!({ "type": "rotated", "direction": "right" })]
        );
        let mut changed = ids_of_type(&ran.events, "controlChanged");
        changed.sort();
        let mut expected = vec![lane5.id.clone(), trap.id.clone()];
        expected.sort();
        assert_eq!(changed, expected);
    }

    #[test]
    fn s3_1_reads_left_and_right_from_the_rotating_players_seat_which_is_the_controller() {
        let mut state = game("rotate-seat");
        let mine = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );

        // p1 rotating right sends lane 1 to lane 2; p2 rotating right sends p1's lane 1 the other way,
        // because the ring is written from the rotating player's seat.
        run(&mut state, &[turning("right")], as_p1());
        assert_eq!(where_is(&state, &mine), "p1 units 2");

        run(
            &mut state,
            &[turning("right")],
            RunOptions {
                controller: Some(PlayerId::P2),
                ..Default::default()
            },
        );
        assert_eq!(where_is(&state, &mine), "p1 units 1");
    }

    #[test]
    fn r14_r88_delegates_the_locked_destination_bounce_rather_than_walking_the_ring_itself() {
        let mut state = game("rotate-locked");
        let blocked = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        lock_zone(&mut state, slot(PlayerId::P1, Row::Units, 2));

        let ran = run(&mut state, &[turning("right")], as_p1());

        // A rule only `subsystems/rotation.ts` implements: the card goes to its OWNER's hand (R12).
        assert_eq!(where_is(&state, &blocked), "hand");
        assert_eq!(
            state
                .players
                .p1
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect::<Vec<_>>(),
            vec![blocked.id.clone()]
        );
        assert_eq!(ids_of_type(&ran.events, "bounced"), vec![blocked.id.clone()]);
    }

    #[test]
    fn s8_3_c52_picks_the_radiant_bounce_up_from_the_running_face_with_nothing_passed_for_it() {
        let mut state = game("rotate-radiant");
        let crosser = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 5),
            json!({}),
        );
        let stayer = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );

        // The card's two faces share one hook and pass only the direction, so the radiant behaviour
        // has to come from `ctx.radiant`.
        let ran = run(
            &mut state,
            &[turning("right")],
            RunOptions {
                controller: Some(PlayerId::P1),
                radiant: Some(true),
                ..Default::default()
            },
        );

        // "Cards that would move to the opponent are bounced to their owner's hand costing 0 instead."
        assert_eq!(where_is(&state, &crosser), "hand");
        let crossed = live(&state, &crosser.id);
        assert_eq!(crossed.cost_override, Some(0));
        assert_eq!(crossed.controller, PlayerId::P1);
        assert!(of_type(&ran.events, "controlChanged").is_empty());
        // The card that stayed on this side still rotated.
        assert_eq!(where_is(&state, &stayer), "p1 units 2");
    }
}
