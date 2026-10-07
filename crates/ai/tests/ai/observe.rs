//! What the AI may know (SPEC §9.9, R185; docs/polish/3-ai.md B9–B12).
//!
//! `redact(state, seat)` is the only code that reads a true state, and `determinize` turns what it
//! leaves into one concrete world. The proofs here are all observable: two true states that differ
//! only in what the seat cannot see redact to the same hash and give the same decision; a
//! determinized world shows the seat exactly the view the true state shows it; and every card the
//! seat can read comes through a determinization untouched.
//!
//! The fixed pair (B9, B11) is the one the design names: p2's hand #2 Bigot and #11 Tempo Timmy
//! against #53 Reno and #20 Pointmaster, a face-down #41 Sheepish against a face-down #60 Bear
//! Honeypot, different p2 libraries, a reordered p1 library and different seeds. The mutation
//! property (fast-check) generalises it to random mutations of random real games.
//!
//! Port of `packages/ai/test/observe.test.ts`. fast-check has no Rust twin among the workspace's
//! crates (SURFACE §2), so the property runs its 150 cases from a seeded `Rng` (stream
//! `observe-property:185`, TS's `seed: 185`) and does not shrink. `REAL_STATES`, a module constant in
//! TS, is computed once per test binary (`OnceLock`). TS's per-test `{ timeout }` has no `cargo test`
//! twin.

use std::sync::OnceLock;

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{
    AI, act, ai_pool, card_by_id, clone, dealt_game, every_card, is_legal, random_decks, random_policy_states,
    trap_pool,
};

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

fn p1_side() -> Value {
    json!({
        "hand": ["core-008", "core-035"],
        "field": ["core-011"],
        "library": ["core-020", "core-053", "core-025"],
    })
}

fn p2_a() -> Value {
    json!({
        "hand": ["core-002", "core-011"],
        "field": ["core-019"],
        "backrow": [{ "def": "core-041", "faceUp": false }],
        "library": ["core-005", "core-016", "core-047"],
        "health": 20,
    })
}

fn p2_b() -> Value {
    json!({
        "hand": ["core-053", "core-020"],
        "field": ["core-019"],
        "backrow": [{ "def": "core-060", "faceUp": false }],
        "library": ["core-063", "core-044", "core-072"],
        "health": 20,
    })
}

/// TS's `{ ...base, ...over }` on two JSON objects.
fn spread(base: Value, over: Value) -> Value {
    let mut out = base;
    if let (Some(target), Value::Object(extra)) = (out.as_object_mut(), over) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
    out
}

/// A value as its JSON, for comparisons that pin the wire shape rather than a Rust type's name.
fn js<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// `determinize` with TS's default options (`{}`).
fn det(public: &GameState, seat: PlayerId, rng: &mut Rng) -> GameState {
    determinize(public, seat, rng, DeterminizeOptions::default())
}

/// TS's `{ rng: createRng(seed) }`: AI_BUDGET, no clock.
fn ai_options(seed: &str) -> AiOptions<'static> {
    AiOptions { rng: create_rng(seed, 0), budget: AI_BUDGET, should_stop: None }
}

/// TS's `build(seed, p2, p1 = P1_SIDE)`; the default is passed explicitly.
fn build(seed: &str, p2: Value, p1: Value) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 })).state().clone()
}

/// A and B differ only in what p1 cannot see: p2's hidden cards, p1's library order and the seed.
fn pair() -> (GameState, GameState) {
    let a = build("observe-a", p2_a(), p1_side());
    let mut b = clone(&build("observe-b", p2_b(), p1_side()));
    b.players[PlayerId::P1].library.reverse();
    (a, b)
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// The seat-visible fields B12 says a determinization leaves alone.
fn readable(card: &CardInstance) -> Value {
    json!({
        "id": card.id,
        "defId": card.def_id,
        "zone": card.zone,
        "radiant": card.radiant,
        "damage": card.damage,
        "buffs": card.buffs,
    })
}

fn is_hidden_backrow(state: &GameState, card: &CardInstance, seat: PlayerId) -> bool {
    card.controller != seat && card.face_up != Some(true) && def_of(Some(state), &card.def_id).type_ != CardType::FieldSpell
}

/// B9's mutation: rewrite everything `seat` may not know and nothing else. The opponent's hand and
/// library get new identities (and Radiant flags) and a new order, every backrow card the seat
/// cannot read gets another Trap that shows the same cost (R351: the seat reads that much, since patch
/// v0.1.1 gave #85 a cost of its own) and leaves the seat's view unchanged (R403, R602: a live
/// face-down aura shows on the board), the seat's own library is reordered, and seed, cursor and the
/// nonce log are replaced.
fn mutate_hidden(state: &GameState, seat: PlayerId, seed: i64) -> GameState {
    let mut out = clone(state);
    let mut rng = create_rng(&format!("observe-mutate:{seed}"), 0);
    let opp = seat.opponent();
    let pool = ai_pool();
    let traps = trap_pool();
    {
        let side = &mut out.players[opp];
        for card in side.hand.iter_mut().chain(side.library.iter_mut()) {
            card.def_id = rng.pick(&pool).cloned().unwrap_or_default();
            card.radiant = rng.coin();
        }
    }
    for player in PLAYER_IDS {
        for lane in 0..out.players[player].backrow.len() {
            let Some(card) = out.players[player].backrow[lane].clone() else { continue };
            if !is_hidden_backrow(state, &card, seat) {
                continue;
            }
            // R403, R602: a face-down trap with no reveal condition is live, so a trap of the same cost
            // can still change what the seat reads off the board (C #88 Siphon Squad shrinks its units'
            // Attack). That is not hidden from it, so the swap must leave the seat's view as it was. The
            // card's own def always does, so there is always a trap to pick.
            let shown = effective_cost(&out, &card, Default::default());
            let original = card.def_id.clone();
            let seen = view_without_events(&out, seat);
            let mut unseen: Vec<String> = Vec::new();
            for id in &traps {
                let mut probe = card.clone();
                probe.def_id = id.clone();
                if effective_cost(&out, &probe, Default::default()) != shown {
                    continue;
                }
                if let Some(slot) = out.players[player].backrow[lane].as_mut() {
                    slot.def_id = id.clone();
                }
                let same = view_without_events(&out, seat) == seen;
                if let Some(slot) = out.players[player].backrow[lane].as_mut() {
                    slot.def_id = original.clone();
                }
                if same {
                    unseen.push(id.clone());
                }
            }
            let picked = rng.pick(&unseen).cloned().unwrap_or_default();
            if let Some(slot) = out.players[player].backrow[lane].as_mut() {
                slot.def_id = picked;
            }
        }
    }
    out.players[opp].hand = rng.shuffle(&out.players[opp].hand);
    out.players[opp].library = rng.shuffle(&out.players[opp].library);
    out.players[seat].library = rng.shuffle(&out.players[seat].library);
    out.seed = format!("observe-mutated-{seed}");
    out.rng_cursor = rng.int(10_000) as u32;
    out.applied = Vec::new();
    out
}

/// viewFor without its event tail, which B10 exempts.
fn view_without_events(state: &GameState, seat: PlayerId) -> Value {
    let mut view = js(view_for(state, seat));
    if let Some(map) = view.as_object_mut() {
        map.remove("events");
    }
    view
}

/// Real mid-game states for the property and for B10 (ids minted by createGame, both seats valid).
fn real_states() -> &'static [GameState] {
    static REAL_STATES: OnceLock<Vec<GameState>> = OnceLock::new();
    REAL_STATES.get_or_init(|| {
        jackioh_cards::register_all();
        let mut states = random_policy_states("observe-real-1", 7, 600);
        states.extend(random_policy_states("observe-real-2", 9, 600));
        states.extend(random_policy_states("observe-real-3", 5, 400));
        states
    })
}

/// A fresh real game, as TS's `beginGame(createGame({ seed, decks: randomDecks(seed) })).state`.
fn real_game(seed: &str) -> GameState {
    jackioh_cards::register_all();
    let created = create_game(&CreateGameArgs {
        seed: seed.to_string(),
        decks: random_decks(seed),
        ..Default::default()
    });
    begin_game(&created).state
}

fn sorted(mut items: Vec<String>) -> Vec<String> {
    items.sort();
    items
}

// ---------------------------------------------------------------------------------------------
// B9: redact
// ---------------------------------------------------------------------------------------------

mod redact_b9 {
    use super::*;

    /// R185 B9: two states differing only in p2's hidden hand, face-down trap, library, p1's library order and the seed redact to one hash
    #[test]
    fn r185_b9_two_states_differing_only_in_p2s_hidden_hand_face_down_trap_library_p1s_library_order_and_the_seed_redact_to_one_hash() {
        let (a, b) = pair();
        assert_ne!(hash_state(&a), hash_state(&b));
        assert_eq!(hash_state(&redact(&a, AI)), hash_state(&redact(&b, AI)));
    }

    /// R185 B9: redact does not mutate its input
    #[test]
    fn r185_b9_redact_does_not_mutate_its_input() {
        let (a, _) = pair();
        let before = serde_json::to_string(&a).expect("serialisable");
        let _ = redact(&a, AI);
        assert_eq!(serde_json::to_string(&a).expect("serialisable"), before);
    }

    /// R185 B9: the hidden set is the opponent's hand and library plus the backrow the seat cannot read
    #[test]
    fn r185_b9_the_hidden_set_is_the_opponents_hand_and_library_plus_the_backrow_the_seat_cannot_read() {
        let (a, _) = pair();
        let p2 = &a.players[PlayerId::P2];
        let expected: IndexSet<String> = ids_of(&p2.hand)
            .into_iter()
            .chain(ids_of(&p2.library))
            .chain(p2.backrow.iter().flatten().map(|card| card.id.clone()))
            .collect();
        let got: Vec<String> = hidden_instance_ids(&a, AI).into_iter().collect();
        assert_eq!(sorted(got), sorted(expected.into_iter().collect()));
    }

    /// R185 B9: hidden cards become placeholders; every public field, the seat's hand and the seed-free history stay
    #[test]
    fn r185_b9_hidden_cards_become_placeholders_every_public_field_the_seats_hand_and_the_seed_free_history_stay() {
        let (a, _) = pair();
        let public = redact(&a, AI);
        let hidden = hidden_instance_ids(&a, AI);

        assert_eq!(public.seed, "redacted");
        assert_eq!(public.rng_cursor, 0);
        assert!(public.applied.is_empty());
        for id in &hidden {
            let card = card_by_id(&public, id);
            assert!(card.is_some(), "{id}");
            assert_eq!(card.as_ref().map(|card| card.def_id.clone()), Some(HIDDEN_DEF_ID.to_string()), "{id}");
            assert_eq!(card.as_ref().map(|card| card.radiant), Some(false), "{id}");
        }
        // p2's hand is re-ordered by numeric id, which erases the true order.
        let hand_ids: Vec<i64> = public.players[PlayerId::P2]
            .hand
            .iter()
            .map(|card| card.id[1..].parse::<i64>().unwrap_or(i64::MAX))
            .collect();
        let mut ascending = hand_ids.clone();
        ascending.sort();
        assert_eq!(hand_ids, ascending);
        // Public cards and the seat's own hand are untouched.
        let def_ids = |cards: &[CardInstance]| -> Vec<String> { cards.iter().map(|card| card.def_id.clone()).collect() };
        assert_eq!(def_ids(&public.players[PlayerId::P1].hand), vec!["core-008", "core-035"]);
        let unit_defs = |player: PlayerId| -> Vec<String> {
            public.players[player].units.iter().flatten().flatten().map(|card| card.def_id.clone()).collect()
        };
        assert_eq!(unit_defs(PlayerId::P2), vec!["core-019"]);
        assert_eq!(unit_defs(PlayerId::P1), vec!["core-011"]);
        assert_eq!(public.players[PlayerId::P2].hero.health, 20);
        assert_eq!(public.players[PlayerId::P2].hand.len(), 2);
        assert_eq!(public.players[PlayerId::P2].library.len(), 3);
        assert_eq!(sorted(def_ids(&public.players[PlayerId::P1].library)), vec!["core-020", "core-025", "core-053"]);
    }

    /// R185 B9: with nothing in the opponent's hand, library or face-down backrow, nothing is hidden
    #[test]
    fn r185_b9_with_nothing_in_the_opponents_hand_library_or_face_down_backrow_nothing_is_hidden() {
        let state = build(
            "observe-nothing-hidden",
            json!({ "field": ["core-019"], "graveyard": ["core-044"] }),
            json!({ "hand": ["core-008"] }),
        );
        assert_eq!(hidden_instance_ids(&state, AI).len(), 0);
        let public = redact(&state, AI);
        for card in every_card(&state) {
            let seen = card_by_id(&public, &card.id).expect("every card stays");
            assert_eq!(readable(seen), readable(card));
        }
    }

    /// R185 B9: the seat's own face-down trap is not hidden from it
    #[test]
    fn r185_b9_the_seats_own_face_down_trap_is_not_hidden_from_it() {
        let state = build(
            "observe-own-trap",
            p2_a(),
            spread(p1_side(), json!({ "backrow": [{ "def": "core-041", "faceUp": false }] })),
        );
        let own = state.players[PlayerId::P1].backrow.iter().flatten().next().cloned();
        assert_eq!(own.as_ref().map(|card| card.def_id.clone()), Some("core-041".to_string()));
        let own = own.expect("p1's trap");
        assert!(!hidden_instance_ids(&state, AI).contains(&own.id));
        assert_eq!(
            card_by_id(&redact(&state, AI), &own.id).map(|card| card.def_id.clone()),
            Some("core-041".to_string())
        );
    }

    /// R185 B9: the opponent's Field Spell is public even when it is not flagged face-up
    #[test]
    fn r185_b9_the_opponents_field_spell_is_public_even_when_it_is_not_flagged_face_up() {
        let state = build(
            "observe-field-spell",
            spread(p2_a(), json!({ "backrow": [{ "def": "core-006", "faceUp": false }] })),
            p1_side(),
        );
        let well = state.players[PlayerId::P2].backrow.iter().flatten().next().cloned().expect("p2's Field Spell");
        assert_eq!(well.def_id, "core-006");
        assert!(!hidden_instance_ids(&state, AI).contains(&well.id));
        assert_eq!(
            card_by_id(&redact(&state, AI), &well.id).map(|card| card.def_id.clone()),
            Some("core-006".to_string())
        );
    }

    /// R185 B9: the opponent's open prompt reaches the seat with no options
    #[test]
    fn r185_b9_the_opponents_open_prompt_reaches_the_seat_with_no_options() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "observe-their-prompt",
            "active": "p2",
            "turn": 10,
            "p1": { "hand": ["core-008"] },
            "p2": { "hand": ["core-072"], "graveyard": ["core-044", "core-008"], "library": ["core-011"] },
        }));
        s.play("core-072", json!({}));
        let state = s.state().clone();
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P2));
        assert!(state.pending.as_ref().map_or(0, |pending| pending.options.len()) > 0);

        let public = redact(&state, AI);
        assert_eq!(public.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P2));
        assert_eq!(public.pending.as_ref().map(|pending| pending.options.len()), Some(0));
        // The seat's own prompt keeps its options: seen from p2 nothing is removed.
        assert_eq!(
            js(redact(&state, PlayerId::P2).pending.map(|pending| pending.options)),
            js(state.pending.as_ref().map(|pending| pending.options.clone()))
        );
    }

    /// R266 B9: while both mulligans are open, the opponent's sealed answer and its options reach the seat as nothing
    #[test]
    fn r266_b9_while_both_mulligans_are_open_the_opponents_sealed_answer_and_its_options_reach_the_seat_as_nothing() {
        let dealt = dealt_game("observe-their-mulligan");
        let everything: Vec<String> = dealt.players[PlayerId::P2].hand.iter().map(|card| card.id.clone()).collect();
        let kept_all = act(&dealt, PlayerId::P2, &ActionBody::Mulligan { keep: everything });
        let kept_none = act(&dealt, PlayerId::P2, &ActionBody::Mulligan { keep: Vec::new() });

        // Whatever p2 kept, the seat's state is the same: only that p2 has answered (R265, R266).
        assert_eq!(hash_state(&redact(&kept_none, AI)), hash_state(&redact(&kept_all, AI)));
        let public = redact(&kept_all, AI);
        assert_eq!(public.mulligan.as_ref().and_then(|open| open.p2.keep.clone()), Some(Vec::<String>::new()));
        assert_eq!(public.mulligan.as_ref().map(|open| open.p2.prompt.options.len()), Some(0));
        // The seat's own mulligan keeps its options, and it answers it without waiting.
        assert_eq!(
            js(public.mulligan.as_ref().map(|open| open.p1.prompt.options.clone())),
            js(kept_all.mulligan.as_ref().map(|open| open.p1.prompt.options.clone()))
        );
        assert_eq!(
            decide(&kept_all, AI, &mut ai_options("observe-their-mulligan")).map(|decision| decision.reason),
            Some(DecisionReason::Mulligan)
        );
    }

    /// R266 B9: a sealed answer owed behind the seat's own paused resolution reaches it as keeping everything
    #[test]
    fn r266_b9_a_sealed_answer_owed_behind_the_seats_own_paused_resolution_reaches_it_as_keeping_everything() {
        // A cast-on-draw Spell that asks its caster (no Core one asks, so a fixture): p1's replacement
        // draw casts it, and p2's sealed answer waits in setup's owed item until p1 answers (R224, R265).
        jackioh_cards::register_all();
        let asking = "ai-r266-cod-asks";
        let script = Script {
            static_flags: Some(StaticFlags { cast_on_draw: Some(true), ..Default::default() }),
            cry: Some(hook(|_ctx| {
                vec![effects::choose_mode(json_as(json!({
                    "options": ["ok"],
                    "step": "ok",
                    "prompt": "the cast's question",
                })))]
            })),
            resume: IndexMap::from_iter([("ok", hook(|_ctx| Vec::new()))]),
            ..Script::default()
        };
        let mut scripts = registered_scripts().clone();
        scripts.insert(asking.to_string(), CardScripts { base: script.clone(), radiant: script });
        register_scripts(scripts);
        let paused = |p2_keeps_all: bool| -> GameState {
            let mut dealt = dealt_game("observe-owed-mulligan");
            dealt.transient_defs.insert(
                asking.to_string(),
                json_as(json!({
                    "id": asking,
                    "index": asking,
                    "name": asking,
                    "set": "Core",
                    "type": "Spell",
                    "tags": [],
                    "rarity": "Common",
                    "token": false,
                    "cost": 0,
                    "base": { "keywords": [], "text": asking },
                    "radiant": { "keywords": [], "text": asking },
                })),
            );
            let card = new_instance(&mut dealt, asking, PlayerId::P1, Zone::Library { player: PlayerId::P1 });
            dealt.players[PlayerId::P1].library.insert(0, card);
            let keep: Vec<String> = if p2_keeps_all {
                dealt.players[PlayerId::P2].hand.iter().map(|card| card.id.clone()).collect()
            } else {
                Vec::new()
            };
            let sealed = act(&dealt, PlayerId::P2, &ActionBody::Mulligan { keep });
            let keep_p1: Vec<String> =
                sealed.players[PlayerId::P1].hand.iter().skip(1).map(|card| card.id.clone()).collect();
            act(&sealed, PlayerId::P1, &ActionBody::Mulligan { keep: keep_p1 })
        };
        let kept_all = paused(true);
        let kept_none = paused(false);
        assert_eq!(kept_all.pending.as_ref().map(|pending| pending.player_id), Some(AI));
        assert!(!kept_all.work.is_empty());

        // The seat is asked now, and what it may know is the same whatever p2 kept.
        assert_eq!(hash_state(&redact(&kept_none, AI)), hash_state(&redact(&kept_all, AI)));
        assert_eq!(
            decide(&kept_none, AI, &mut ai_options("observe-owed")).map(|decision| decision.action),
            decide(&kept_all, AI, &mut ai_options("observe-owed")).map(|decision| decision.action),
        );
    }

    /// R185 B9: a card in the seat's own library that was minted for the opponent's deck (R73) is hidden too
    #[test]
    fn r185_b9_a_card_in_the_seats_own_library_that_was_minted_for_the_opponents_deck_r73_is_hidden_too() {
        let mut state = clone(&real_game("observe-r73"));
        // #87's library swap, reduced to its effect on two cards: they now sit in p1's library.
        let mut moved: Vec<CardInstance> = state.players[PlayerId::P2].library.drain(..2).collect();
        for card in &mut moved {
            card.zone = Zone::Library { player: PlayerId::P1 };
        }
        state.players[PlayerId::P1].library.extend(moved.iter().cloned());
        let moved_ids = ids_of(&moved);

        let hidden = hidden_instance_ids(&state, AI);
        for id in &moved_ids {
            assert!(hidden.contains(id), "{id}");
        }
        for card in &state.players[PlayerId::P1].library {
            if moved_ids.contains(&card.id) {
                continue;
            }
            assert!(!hidden.contains(&card.id), "{} is p1's own card", card.id);
        }
        for card in &state.players[PlayerId::P1].hand {
            assert!(!hidden.contains(&card.id));
        }

        let public = redact(&state, AI);
        for id in &moved_ids {
            assert_eq!(card_by_id(&public, id).map(|card| card.def_id.clone()), Some(HIDDEN_DEF_ID.to_string()));
        }
    }

    /// R312 a card in the seat's own library it was never shown is hidden, so what it is cannot move a decision
    #[test]
    fn r312_a_card_in_the_seats_own_library_it_was_never_shown_is_hidden_so_what_it_is_cannot_move_a_decision() {
        let mut a = clone(&real_game("observe-r312"));
        // #83's library replacements (or a library #87 swapped away and back), reduced to two cards:
        // p1 was never shown them, so its own `viewFor` counts them unknown (R312) and so must the AI.
        let replaced: Vec<String> = a.players[AI].library.iter().take(2).map(|card| card.id.clone()).collect();
        for card in a.players[AI].library.iter_mut().take(2) {
            card.known_as = None;
        }
        let mut b = clone(&a);
        let legendary = "core-052";
        for card in b.players[AI].library.iter_mut().take(2) {
            card.def_id = legendary.to_string();
            card.radiant = true;
        }

        let hidden = hidden_instance_ids(&a, AI);
        for id in &replaced {
            assert!(hidden.contains(id), "{id}");
        }
        for card in a.players[AI].library.iter().skip(2) {
            assert!(!hidden.contains(&card.id), "{} is known", card.id);
        }
        assert_eq!(view_for(&a, AI).you.own_library.map(|library| library.unknown), Some(2));
        assert_eq!(hash_state(&redact(&b, AI)), hash_state(&redact(&a, AI)));
        for id in &replaced {
            assert_eq!(card_by_id(&redact(&b, AI), id).map(|card| card.def_id.clone()), Some(HIDDEN_DEF_ID.to_string()));
        }
    }

    /// R185 B9: a different public unit on the opponent's field changes the redacted hash
    #[test]
    fn r185_b9_a_different_public_unit_on_the_opponents_field_changes_the_redacted_hash() {
        let a = build("observe-a", p2_a(), p1_side());
        let c = build("observe-a", spread(p2_a(), json!({ "field": ["core-025"] })), p1_side());
        assert_ne!(hash_state(&redact(&c, AI)), hash_state(&redact(&a, AI)));
    }

    /// R185 B9: a face-up backrow card is readable, so its identity changes the redacted hash
    #[test]
    fn r185_b9_a_face_up_backrow_card_is_readable_so_its_identity_changes_the_redacted_hash() {
        let a = build("observe-faceup", spread(p2_a(), json!({ "backrow": [{ "def": "core-041", "faceUp": true }] })), p1_side());
        let b = build("observe-faceup", spread(p2_a(), json!({ "backrow": [{ "def": "core-060", "faceUp": true }] })), p1_side());
        assert_ne!(hash_state(&redact(&a, AI)), hash_state(&redact(&b, AI)));
    }

    /// R185 B9: the seat's own hand is known, so a different own hand changes the redacted hash
    #[test]
    fn r185_b9_the_seats_own_hand_is_known_so_a_different_own_hand_changes_the_redacted_hash() {
        let a = build("observe-a", p2_a(), p1_side());
        let b = build("observe-a", p2_a(), spread(p1_side(), json!({ "hand": ["core-008", "core-044"] })));
        assert_ne!(hash_state(&redact(&a, AI)), hash_state(&redact(&b, AI)));
    }

    /// R185 B9: the seat knows its own library's contents, so different contents change the redacted hash
    #[test]
    fn r185_b9_the_seat_knows_its_own_librarys_contents_so_different_contents_change_the_redacted_hash() {
        let a = build("observe-a", p2_a(), p1_side());
        let b = build(
            "observe-a",
            p2_a(),
            spread(p1_side(), json!({ "library": ["core-020", "core-053", "core-019"] })),
        );
        assert_ne!(hash_state(&redact(&a, AI)), hash_state(&redact(&b, AI)));
    }

    /// R185 B9: the opponent's hand size is public, so one more hidden card changes the redacted hash
    #[test]
    fn r185_b9_the_opponents_hand_size_is_public_so_one_more_hidden_card_changes_the_redacted_hash() {
        let a = build("observe-a", p2_a(), p1_side());
        let b = build(
            "observe-a",
            spread(p2_a(), json!({ "hand": ["core-002", "core-011", "core-005"] })),
            p1_side(),
        );
        assert_ne!(hash_state(&redact(&a, AI)), hash_state(&redact(&b, AI)));
    }

    /// R185 B9: from the other seat the same two states are not alike (p2 reads its own hand)
    #[test]
    fn r185_b9_from_the_other_seat_the_same_two_states_are_not_alike_p2_reads_its_own_hand() {
        let (a, b) = pair();
        assert_ne!(hash_state(&redact(&a, PlayerId::P2)), hash_state(&redact(&b, PlayerId::P2)));
    }

    /// R185 B9: property: any mutation of what the seat cannot know leaves the redacted hash unchanged
    #[test]
    fn r185_b9_property_any_mutation_of_what_the_seat_cannot_know_leaves_the_redacted_hash_unchanged() {
        let (fixed, _) = pair();
        let mut bases: Vec<(&GameState, Vec<PlayerId>)> = vec![(&fixed, vec![AI])];
        bases.extend(real_states().iter().map(|state| (state, PLAYER_IDS.to_vec())));
        // fast-check's `{ numRuns: 150, seed: 185 }` over `nat(bases - 1)`, `nat(1)` and `integer()`.
        let mut runs = create_rng("observe-property:185", 0);
        for _ in 0..150 {
            let base_at = runs.int(bases.len() as i32) as usize;
            let seat_at = runs.int(2) as usize;
            let mutation = i64::from(runs.int(i32::MAX)) - i64::from(runs.int(i32::MAX));
            let (base, seats) = &bases[base_at];
            let seat = seats[seat_at % seats.len()];
            let mutated = mutate_hidden(base, seat, mutation);
            // The mutation really changed the true state (the seed alone guarantees it).
            assert_ne!(hash_state(&mutated), hash_state(base));
            assert_eq!(hash_state(&redact(&mutated, seat)), hash_state(&redact(base, seat)));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B10: no information is lost
// ---------------------------------------------------------------------------------------------

mod determinize_loses_nothing_the_seat_can_see_b10 {
    use super::*;

    /// R185 B10: viewFor of a determinized redaction equals the true view, events aside, from both seats
    #[test]
    fn r185_b10_view_for_of_a_determinized_redaction_equals_the_true_view_events_aside_from_both_seats() {
        // R434: a finished game shows both hands to both seats, which redaction hides by design and no
        // decision reads (`aiToAct` is false once the game is over), so B10 holds over games in progress.
        let states = real_states();
        assert!(states.iter().filter(|state| state.result.is_none()).count() > 20);
        for (at, state) in states.iter().enumerate() {
            // R434: a finished game's view shows the opponent's hand in full, which a redaction cannot
            // keep. No seat is to act in one, so the AI never determinizes it; a sampled game that happens
            // to end on a sampled step is not a state this check is about.
            if state.result.is_some() {
                continue;
            }
            for seat in PLAYER_IDS {
                let world = det(&redact(state, seat), seat, &mut create_rng(&format!("observe-b10:{at}:{seat}"), 0));
                assert_eq!(view_without_events(&world, seat), view_without_events(state, seat), "state {at}, seat {seat}");
            }
        }
    }

    /// R185 B10: the fixed scenario's view survives redaction and determinization
    #[test]
    fn r185_b10_the_fixed_scenarios_view_survives_redaction_and_determinization() {
        let (a, _) = pair();
        let world = det(&redact(&a, AI), AI, &mut create_rng("observe-b10-fixed", 0));
        assert_eq!(view_without_events(&world, AI), view_without_events(&a, AI));
    }
}

// ---------------------------------------------------------------------------------------------
// B11: the same decision
// ---------------------------------------------------------------------------------------------

mod decide_cannot_see_hidden_cards_b11 {
    use super::*;

    /// R185 B11: decide gives deep-equal decisions for the fixed pair under the same AI rng
    #[test]
    fn r185_b11_decide_gives_deep_equal_decisions_for_the_fixed_pair_under_the_same_ai_rng() {
        let (a, b) = pair();
        for k in ["observe-k1", "observe-k2", "observe-k3"] {
            let from_a = decide(&a, AI, &mut ai_options(k));
            let from_b = decide(&b, AI, &mut ai_options(k));
            assert!(from_a.is_some(), "{k}");
            assert_eq!(from_b, from_a, "{k}");
            let action = from_a.map(|decision| decision.action).expect("a decision");
            assert!(is_legal(&a, AI, &action), "{k}");
        }
    }

    /// R185 B11: decide gives deep-equal decisions for random mutations of real states
    #[test]
    fn r185_b11_decide_gives_deep_equal_decisions_for_random_mutations_of_real_states() {
        let candidates: Vec<&GameState> = real_states()
            .iter()
            .filter(|state| state.result.is_none() && seat_to_act(state) == Some(state.active))
            .take(4)
            .collect();
        assert!(!candidates.is_empty());
        for (at, state) in candidates.iter().enumerate() {
            let seat = seat_to_act(state).expect("a seat to act");
            let mutated = mutate_hidden(state, seat, at as i64 + 1);
            let rng_seed = format!("observe-b11-real-{at}");
            let original = decide(state, seat, &mut ai_options(&rng_seed));
            assert_eq!(decide(&mutated, seat, &mut ai_options(&rng_seed)), original, "state {at}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B12: determinize
// ---------------------------------------------------------------------------------------------

mod determinize_b12 {
    use super::*;

    /// B12: every card the seat can read keeps its id, def, zone, Radiant flag, damage and buffs
    #[test]
    fn b12_every_card_the_seat_can_read_keeps_its_id_def_zone_radiant_flag_damage_and_buffs() {
        let (a, _) = pair();
        let hidden = hidden_instance_ids(&a, AI);
        let world = det(&redact(&a, AI), AI, &mut create_rng("observe-b12-readable", 0));
        for card in every_card(&a) {
            if hidden.contains(&card.id) {
                continue;
            }
            let after = card_by_id(&world, &card.id);
            assert!(after.is_some(), "{}", card.id);
            let after = after.expect("the card");
            assert_eq!(readable(after), readable(card), "{}", card.id);
        }
    }

    /// B12: readable cards survive determinization in real mid-game states, from both seats
    #[test]
    fn b12_readable_cards_survive_determinization_in_real_mid_game_states_from_both_seats() {
        for (at, state) in real_states().iter().take(12).enumerate() {
            for seat in PLAYER_IDS {
                let hidden = hidden_instance_ids(state, seat);
                let world = det(&redact(state, seat), seat, &mut create_rng(&format!("observe-b12-real:{at}:{seat}"), 0));
                for card in every_card(state) {
                    if hidden.contains(&card.id) {
                        continue;
                    }
                    let after = card_by_id(&world, &card.id);
                    assert!(after.is_some(), "{at} {seat} {}", card.id);
                    let after = after.expect("the card");
                    assert_eq!(readable(after), readable(card), "{at} {seat} {}", card.id);
                }
            }
        }
    }

    /// R380 B12: every hidden card gets a real non-token def of any set; opponent samples are distinct and unseen; face-down ones are traps
    #[test]
    fn r380_b12_every_hidden_card_gets_a_real_non_token_def_of_any_set_opponent_samples_are_distinct_and_unseen_face_down_ones_are_traps() {
        let (a, _) = pair();
        let hidden = hidden_instance_ids(&a, AI);
        let pool: IndexSet<String> = ai_pool().into_iter().collect();
        let traps: IndexSet<String> = trap_pool().into_iter().collect();
        let opponent_public: IndexSet<String> = every_card(&a)
            .iter()
            .filter(|card| card.owner == PlayerId::P2 && !hidden.contains(&card.id))
            .map(|card| card.def_id.clone())
            .collect();
        let face_down_ids: IndexSet<String> =
            a.players[PlayerId::P2].backrow.iter().flatten().map(|card| card.id.clone()).collect();

        for k in 0..40 {
            let world = det(&redact(&a, AI), AI, &mut create_rng(&format!("observe-b12-samples:{k}"), 0));
            let mut samples: Vec<String> = Vec::new();
            for id in &hidden {
                let card = card_by_id(&world, id);
                assert!(card.is_some(), "{id}");
                let def_id = card.map(|card| card.def_id.clone()).unwrap_or_default();
                assert_ne!(def_id, HIDDEN_DEF_ID);
                assert!(pool.contains(&def_id), "{id} sampled {def_id}");
                if face_down_ids.contains(id) {
                    assert!(traps.contains(&def_id), "{id} sampled {def_id}");
                }
                samples.push(def_id);
            }
            let distinct: IndexSet<&String> = samples.iter().collect();
            assert_eq!(distinct.len(), samples.len(), "seed {k}: {}", samples.join(","));
            for def_id in &samples {
                assert!(!opponent_public.contains(def_id), "seed {k}: {def_id}");
            }
        }
    }

    /// R762 B12: each face-down card takes an unseen trap of the cost it shows; with none of that cost left, the sampler falls back to the trap pool
    #[test]
    fn r762_b12_each_face_down_card_takes_an_unseen_trap_of_the_cost_it_shows_with_none_of_that_cost_left_the_sampler_falls_back_to_the_trap_pool() {
        jackioh_cards::register_all();
        let face_down = ["core-018", "core-041", "core-060", "core-071", "core-085"];
        // Every trap of every set is shown but four, so four unseen traps are left for five lanes.
        let unshown = ["core-018", "core-060", "core-071", "core-085"];
        let shown: Vec<String> = trap_pool().into_iter().filter(|id| !unshown.contains(&id.as_str())).collect();
        let backrow: Vec<Value> = face_down.iter().map(|def| json!({ "def": def, "faceUp": false })).collect();
        let state = build("observe-trap-exhaust", json!({ "backrow": backrow, "graveyard": shown }), p1_side());
        let lanes: Vec<Option<String>> =
            state.players[PlayerId::P2].backrow.iter().map(|card| card.as_ref().map(|card| card.id.clone())).collect();
        assert!(lanes.iter().all(Option::is_some));
        let traps: IndexSet<String> = trap_pool().into_iter().collect();
        let costs: Vec<i32> = state.players[PlayerId::P2]
            .backrow
            .iter()
            .map(|card| effective_cost(&state, card.as_ref().expect("a face-down card"), Default::default()))
            .collect();
        assert_eq!(costs, vec![1, 1, 1, 1, 2]);

        for k in 0..20 {
            let world = det(&redact(&state, AI), AI, &mut create_rng(&format!("observe-trap-exhaust:{k}"), 0));
            let samples: Vec<String> = lanes
                .iter()
                .map(|id| {
                    card_by_id(&world, id.as_deref().unwrap_or_default()).map(|card| card.def_id.clone()).unwrap_or_default()
                })
                .collect();
            for def_id in &samples {
                assert!(traps.contains(def_id), "seed {k}: {def_id}");
            }
            // In lane order: the three unseen (1) traps fill the first three lanes that show (1).
            assert_eq!(sorted(samples[..3].to_vec()), vec!["core-018", "core-060", "core-071"], "seed {k}");
            // Lane 4 shows (1) with no unseen (1) left, so it falls back to the pool's only unseen trap.
            assert_eq!(samples[3], "core-085", "seed {k}");
            // Lane 5, showing (2) with nothing unseen left, takes any trap.
        }
    }

    /// B12: with more hidden cards than the pool, every card is used before any repeat, and nothing throws
    #[test]
    fn b12_with_more_hidden_cards_than_the_pool_every_card_is_used_before_any_repeat_and_nothing_throws() {
        jackioh_cards::register_all();
        let pool = ai_pool();
        let mut library = pool.clone();
        library.extend(["core-008", "core-011", "core-019"].map(String::from));
        let state = build("observe-pool-exhaust", json!({ "library": library }), json!({}));
        let excluded: IndexSet<String> = query(&Default::default())
            .iter()
            .filter(|def| AI_DETERMINIZE.exclude_def_ids.contains(&def.id.as_str()))
            .map(|def| def.id.clone())
            .collect();
        let world = det(&redact(&state, AI), AI, &mut create_rng("observe-pool-exhaust", 0));
        let samples: Vec<String> = world.players[PlayerId::P2].library.iter().map(|card| card.def_id.clone()).collect();
        assert_eq!(samples.len(), pool.len() + 3);
        let pool_set: IndexSet<String> = pool.iter().cloned().collect();
        for def_id in &samples {
            assert!(pool_set.contains(def_id), "{def_id}");
        }
        let used: IndexSet<&String> = samples.iter().collect();
        for id in &pool {
            if excluded.contains(id) {
                continue;
            }
            assert!(used.contains(id), "{id} was never sampled");
        }
    }

    /// B12: a state with nothing hidden comes through determinization card for card
    #[test]
    fn b12_a_state_with_nothing_hidden_comes_through_determinization_card_for_card() {
        let state = build(
            "observe-public-det",
            json!({ "field": ["core-019", "core-011"], "graveyard": ["core-044"] }),
            p1_side(),
        );
        assert_eq!(hidden_instance_ids(&state, AI).len(), 0);
        let world = det(&redact(&state, AI), AI, &mut create_rng("observe-public-det", 0));
        assert_eq!(every_card(&world).len(), every_card(&state).len());
        for card in every_card(&state) {
            let after = card_by_id(&world, &card.id).expect("every card stays");
            assert_eq!(readable(after), readable(card), "{}", card.id);
        }
    }

    /// B12: the seat's own library keeps its contents; only its order may change
    #[test]
    fn b12_the_seats_own_library_keeps_its_contents_only_its_order_may_change() {
        let (a, _) = pair();
        let world = det(&redact(&a, AI), AI, &mut create_rng("observe-b12-own", 0));
        assert_eq!(sorted(ids_of(&world.players[PlayerId::P1].library)), sorted(ids_of(&a.players[PlayerId::P1].library)));
        let def_ids = |cards: &[CardInstance]| -> Vec<String> { cards.iter().map(|card| card.def_id.clone()).collect() };
        assert_eq!(
            sorted(def_ids(&world.players[PlayerId::P1].library)),
            sorted(def_ids(&a.players[PlayerId::P1].library))
        );
    }

    /// B12: the determinized seed is the AI's own, never the match's
    #[test]
    fn b12_the_determinized_seed_is_the_ais_own_never_the_matchs() {
        let (a, _) = pair();
        for k in 0..10 {
            let world = det(&redact(&a, AI), AI, &mut create_rng(&format!("observe-b12-seed:{k}"), 0));
            assert_ne!(world.seed, a.seed);
            assert!(world.seed.starts_with("ai:"), "{}", world.seed);
            assert_eq!(world.rng_cursor, 0);
        }
    }

    /// B12: determinize is pure given its rng, and different rngs give different worlds
    #[test]
    fn b12_determinize_is_pure_given_its_rng_and_different_rngs_give_different_worlds() {
        let (a, _) = pair();
        let public = redact(&a, AI);
        let before = serde_json::to_string(&public).expect("serialisable");
        let one = det(&public, AI, &mut create_rng("observe-b12-pure", 0));
        let two = det(&public, AI, &mut create_rng("observe-b12-pure", 0));
        assert_eq!(two, one);
        assert_eq!(serde_json::to_string(&public).expect("serialisable"), before);

        let mut hands: IndexSet<String> = IndexSet::new();
        for k in 0..10 {
            let world = det(&public, AI, &mut create_rng(&format!("observe-b12-vary:{k}"), 0));
            let hand: Vec<&str> = world.players[PlayerId::P2].hand.iter().map(|card| card.def_id.as_str()).collect();
            hands.insert(hand.join(","));
        }
        assert!(hands.len() > 1);
    }

    /// B12: a hidden hand or library slot never samples an excluded card (#98's memory, R43)
    #[test]
    fn b12_a_hidden_hand_or_library_slot_never_samples_an_excluded_card_98s_memory_r43() {
        // Freshly dealt: p2's four-card hand and sixteen-card library are all hidden from p1.
        let state = dealt_game("observe-b12-exclude");
        let excluded: IndexSet<String> = query(&Default::default())
            .iter()
            .filter(|def| AI_DETERMINIZE.exclude_def_ids.contains(&def.id.as_str()))
            .map(|def| def.id.clone())
            .collect();
        assert!(!excluded.is_empty());
        let p2 = &state.players[PlayerId::P2];
        let slots: Vec<String> = ids_of(&p2.hand).into_iter().chain(ids_of(&p2.library)).collect();
        assert!(slots.len() > 10);
        for k in 0..100 {
            let world = det(&redact(&state, AI), AI, &mut create_rng(&format!("observe-b12-exclude:{k}"), 0));
            for id in &slots {
                let def_id = card_by_id(&world, id).map(|card| card.def_id.clone()).unwrap_or_default();
                assert!(!excluded.contains(&def_id), "seed {k}: {id} sampled {def_id}");
            }
        }
    }
}
