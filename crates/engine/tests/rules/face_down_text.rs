//! R1487000: a face-down card's lasting text sleeps until it is revealed or face-up (SPEC §3.2,
//! §10.4 layer 5, §10.8; R33, R686, R1149000).
//!
//!   * Set face-down, a Trap's hero Armor, aura, draw limit and Infinite Reserves act on nothing, so
//!     the other seat's view shows what a textless Trap would leave.
//!   * Revealed (R686) or face-up, each of them acts, and so does every other lasting text a reader
//!     takes off the board: a carrier, a Tribute worth, a targeting cost, a Plague multiplier, an Echo.
//!   * A lasting text fused onto a face-down Trap (C+ #31, R77, R102) sleeps with it, and the Trap
//!     still fires on its trigger.
//!   * A kept script whose text worked face-down (`Script::live_face_down`, R767000) still does.
//!
//! Fixtures are this file's own: `fd-` defs indexed from 14871 (BUILD §0).

use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::unit_def;
use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// The Attack the aura fixture takes off each enemy Unit.
const AURA_DROP: i32 = 2;
/// The draws a turn the limit fixture leaves every player.
const LIMIT: i32 = 1;
/// The damage the firing fixture deals the enemy hero.
const FIRE_DAMAGE: i32 = 2;
/// The other statics fixture's numbers: its worth as a Tribute, the discards a targeting of it costs,
/// what it multiplies a Plague placement by and the Echo it gives a revealed Spell.
const WORTH: i32 = 2;
const DISCARDS: i32 = 1;
const PLAGUE_TIMES: i32 = 2;
const REVEAL_ECHO: i32 = 1;

// Fixtures.

fn def(name: &str, index: u32) -> CardDef {
    json_as(json!({
        "id": format!("fd-{name}"),
        "index": index.to_string(),
        "name": format!("fd-{name}"),
        "set": "Core",
        "type": "Trap",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    }))
}

/// #84 Going Long's lasting text: Armor for its controller's hero.
fn armor() -> CardDef {
    def("armor", 14871)
}
/// An aura on the other side's Units.
fn aura() -> CardDef {
    def("aura", 14872)
}
/// Classic #49's draw limit, on both players.
fn limit() -> CardDef {
    def("limit", 14873)
}
/// #75 Infinite Reserves: a Rush Token card in place of fatigue.
fn reserves() -> CardDef {
    def("reserves", 14874)
}
/// A Trap with no text, what the other seat should see in each one's place.
fn blank() -> CardDef {
    def("blank", 14875)
}
/// A Trap that fires once a played card has resolved (R17).
fn firing() -> CardDef {
    def("firing", 14876)
}
/// The aura on a kept script that works face-down, as C #88's v0.3.7b did (R767000).
fn kept() -> CardDef {
    def("kept", 14877)
}
fn body() -> CardDef {
    unit_def(14878, json!({ "attack": 4, "health": 4 }))
}
/// Other lasting texts a reader takes off the board: a carrier (R446), a Tribute worth (§3.2), a
/// targeting cost (Classic #89), a Plague multiplier (Classic #27, R471) and a reveal Echo (R1045).
fn statics() -> CardDef {
    def("statics", 14879)
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn flags(flags: StaticFlags) -> Script {
    Script {
        static_flags: Some(flags),
        ..Script::default()
    }
}

fn aura_script(live_face_down: bool) -> Script {
    Script {
        aura: Some(aura_hook(|args| {
            let me = args.self_.controller;
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| unit.controller != me),
                mod_: StatMod {
                    attack: Some(-AURA_DROP),
                    ..StatMod::default()
                },
            }]
        })),
        live_face_down,
        ..Script::default()
    }
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (
            armor().id,
            both(flags(StaticFlags {
                hero_armor: Some(FlagOrCount::Flag(true)),
                ..StaticFlags::default()
            })),
        ),
        (aura().id, both(aura_script(false))),
        (
            limit().id,
            both(Script {
                draw_limit: Some(read_hook(|_args| {
                    vec![DrawLimit {
                        player: DrawLimitPlayer::Both,
                        count: LIMIT,
                    }]
                })),
                ..Script::default()
            }),
        ),
        (
            reserves().id,
            both(flags(StaticFlags {
                infinite_reserves: Some(true),
                ..StaticFlags::default()
            })),
        ),
        (
            firing().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "fd-firing",
                    &[GameEventType::CardResolved],
                    |_ctx, _event| {
                        vec![effects::damage(json_as(
                            json!({ "to": { "of": "enemyHero" }, "amount": FIRE_DAMAGE }),
                        ))]
                    },
                )],
                ..Script::default()
            }),
        ),
        (kept().id, both(aura_script(true))),
        (
            statics().id,
            both(Script {
                static_flags: Some(StaticFlags {
                    carrier: Some(true),
                    tribute_worth: Some(WORTH),
                    ..StaticFlags::default()
                }),
                targeting_discards: Some(read_hook(|_args| DISCARDS)),
                plague_multiplier: Some(read_hook(|_args| PLAGUE_TIMES)),
                reveal_echo: Some(read_hook(|_args| REVEAL_ECHO)),
                ..Script::default()
            }),
        ),
    ]
}

// Harness.

fn game(seed: &str) -> GameState {
    let mut state = new_game(&format!("face-down-text-{seed}"), None);
    let mut catalog = registered_catalog().clone();
    for def in [
        armor(),
        aura(),
        limit(),
        reserves(),
        blank(),
        firing(),
        kept(),
        body(),
        statics(),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    // Past setup, where no draw is counted and so no draw limit binds.
    state.turn = 2;
    state.phase = Phase::Main;
    state
}

#[derive(Clone, Copy, Debug)]
enum Face {
    Down,
    Revealed,
    Up,
}

/// The Trap set at p1's backrow 1, as `face` says, and p2's 4/4 at units 1.
fn board(seed: &str, trap: &str, face: Face) -> (GameState, CardInstance) {
    let mut state = game(seed);
    let card = put(&mut state, trap, slot(P1, Row::Backrow, 1), json!({}));
    put(&mut state, &body().id, slot(P2, Row::Units, 1), json!({}));
    let live = find_instance_mut(&mut state, &card.id).expect("the trap is set");
    match face {
        Face::Down => {}
        Face::Revealed => live.revealed = Some(true),
        Face::Up => live.face_up = Some(true),
    }
    let card = live.clone();
    (state, card)
}

/// What p2's seat is shown: p1's hero Armor, and the Attack and Health of p2's own Units.
fn shown_to_p2(state: &GameState) -> (i32, Vec<(i32, i32)>) {
    let view = view_for(state, P2);
    let units = view
        .you
        .units
        .iter()
        .flatten()
        .map(|unit| (unit.attack, unit.health))
        .collect();
    (view.opponent.hero.armor, units)
}

/// p1 draws from an empty library.
fn draw_from_empty(state: &mut GameState) -> DrawOutcome {
    state.players[P1].library.clear();
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    draw_one(&mut EngineSink::new(state, &mut events, &mut rng), P1, None)
}

/// How many cards reach p2's hand when p2 draws two.
fn two_draws(state: &mut GameState) -> usize {
    let before = state.players[P2].hand.len();
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    draw(&mut EngineSink::new(state, &mut events, &mut rng), P2, 2);
    state.players[P2].hand.len() - before
}

/// §10.5 step 7's event for a played permanent (R61), built by hand as `trap_cardresolved.rs` does.
fn resolved(instance: &CardInstance) -> GameEvent {
    GameEvent::CardResolved {
        player: instance.controller,
        instance_id: instance.id.clone(),
        def_id: instance.def_id.clone(),
        permanent: true,
        cost_paid: 0,
        radiant: None,
        arrived_during: None,
        exits_from: None,
        answer: None,
    }
}

mod r1487000_a_face_down_cards_lasting_text_sleeps_until_it_is_revealed_or_face_up {
    use super::*;

    #[test]
    fn r1487000_set_face_down_none_acts_and_the_other_seat_sees_what_a_textless_trap_leaves() {
        let (state, _) = board("blank", &blank().id, Face::Down);
        let textless = shown_to_p2(&state);
        assert_eq!(textless, (0, vec![(4, 4)]));

        for trap in [armor(), aura(), limit(), reserves()] {
            let (state, card) = board(&trap.id, &trap.id, Face::Down);
            assert!(lasting_text_dormant(&state, &card), "{} sleeps", trap.id);
            assert_eq!(shown_to_p2(&state), textless, "{} shows nothing", trap.id);
            assert_eq!(hero_armor_of(&state, P1), 0, "{} gives no Armor", trap.id);
            assert_eq!(draw_limit_of(&state, P2), None, "{} limits no draw", trap.id);
        }

        let (mut state, _) = board("reserves-draw", &reserves().id, Face::Down);
        assert_eq!(draw_from_empty(&mut state), DrawOutcome::Fatigue);
        let (mut state, _) = board("limit-draw", &limit().id, Face::Down);
        assert_eq!(two_draws(&mut state), 2);
    }

    #[test]
    fn r1487000_revealed_or_face_up_each_trap_acts() {
        for face in [Face::Revealed, Face::Up] {
            let (state, card) = board("armor", &armor().id, face);
            assert!(!lasting_text_dormant(&state, &card), "{face:?} wakes it");
            assert_eq!(hero_armor_of(&state, P1), HERO_ARMOR.base.normal, "{face:?}");
            assert_eq!(shown_to_p2(&state).0, HERO_ARMOR.base.normal, "{face:?}");

            let (state, _) = board("aura", &aura().id, face);
            assert_eq!(shown_to_p2(&state).1, vec![(4 - AURA_DROP, 4)], "{face:?}");

            let (mut state, _) = board("limit", &limit().id, face);
            assert_eq!(draw_limit_of(&state, P2), Some(LIMIT), "{face:?}");
            assert_eq!(two_draws(&mut state), 1, "{face:?}");

            let (mut state, _) = board("reserves", &reserves().id, face);
            assert_eq!(draw_from_empty(&mut state), DrawOutcome::Token, "{face:?}");
        }
    }

    #[test]
    fn r1487000_a_face_down_trap_fused_with_hero_armor_gives_none_and_still_fires_on_its_trigger() {
        let mut state = game("fused");
        let trap = put(&mut state, &firing().id, slot(P2, Row::Backrow, 1), json!({}));
        let played = put(&mut state, &body().id, slot(P1, Row::Units, 1), json!({}));
        let ingredient = in_hand(&mut state, &armor().id, P2, 1)
            .into_iter()
            .next()
            .expect("no ingredient");
        let on = find_instance(&state, &trap.id).cloned();
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let fused = fuse(
            &mut EngineSink::new(&mut state, &mut events, &mut rng),
            FuseArgs {
                ingredients: vec![ingredient],
                target: on,
                ..Default::default()
            },
        );

        // R77: the Trap keeps its id and takes the Armor text, which sleeps while it is face-down.
        assert_eq!(fused.map(|card| card.id), Some(trap.id.clone()));
        let live = find_instance(&state, &trap.id).expect("the fused trap is set");
        assert!(lasting_text_dormant(&state, live));
        assert_eq!(hero_armor_of(&state, P2), 0);
        assert_eq!(view_for(&state, P1).opponent.hero.armor, 0);
        let mut shown = state.clone();
        find_instance_mut(&mut shown, &trap.id)
            .expect("the fused trap is set")
            .revealed = Some(true);
        assert_eq!(hero_armor_of(&shown, P2), HERO_ARMOR.base.normal);

        // Its trigger is live face-down: it fires as a Trap and is spent.
        let event = resolved(&played);
        let watching: Vec<String> = traps_watching(&state, &event)
            .iter()
            .map(|matched| matched.trap.id.clone())
            .collect();
        assert_eq!(watching, vec![trap.id.clone()]);
        let before = state.players[P1].hero.health;
        let mut events = vec![event];
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        settle(
            &mut EngineSink::new(&mut state, &mut events, &mut rng),
            SettleOptions::default(),
        );
        assert_eq!(state.players[P1].hero.health, before - FIRE_DAMAGE);
        assert!(state.players[P2].graveyard.iter().any(|card| card.id == trap.id));
    }

    #[test]
    fn r1487000_its_other_static_texts_sleep_face_down_and_wake_revealed() {
        let read = |state: &GameState, card: &CardInstance| {
            (
                is_carrier(state, card),
                tribute_value_of(state, card),
                targeting_discards_of(state, card),
                plague_multiplier_of(state, card),
                jackioh_engine::alt_play::reveal_echo_bonus(state, P1),
            )
        };
        let (state, card) = board("statics-down", &statics().id, Face::Down);
        assert_eq!(read(&state, &card), (false, 1, 0, 1, 0));
        for face in [Face::Revealed, Face::Up] {
            let (state, card) = board("statics", &statics().id, face);
            assert_eq!(
                read(&state, &card),
                (true, WORTH, DISCARDS, PLAGUE_TIMES, REVEAL_ECHO),
                "{face:?}"
            );
        }
    }

    #[test]
    fn r1487000_r767000_a_script_kept_live_face_down_acts_while_set() {
        let (state, card) = board("kept", &kept().id, Face::Down);
        assert!(!lasting_text_dormant(&state, &card));
        assert_eq!(shown_to_p2(&state).1, vec![(4 - AURA_DROP, 4)]);
    }
}
