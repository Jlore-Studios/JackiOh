//! Port of `packages/cards/test/combat-windows.test.ts` (part 27.1).
//!
//! Who killed a unit, what My Pawn projects, and the AI turn My Pawn hands over (SPEC §4.2 step 4,
//! §4.3, §5.1, §10.3, R42, R44, R89, R152, R176). Found by the polish-4 edge-case hunt
//! (docs/polish/4-edge-cases.md, lenses L5 and L7); every case here failed before its fix.
//!
//!  - R42, R89: a destroy effect is no damage instance, so an earlier non-lethal hit is not the kill.
//!  - R176: My Pawn's lethal projection follows the combat — an attacker a First Strike defender
//!    kills first lands nothing, and Trample excess from the attack's Cleave hits counts.
//!  - §5.1: My Pawn fires once, even while its own AI turn is still being played out.
//!  - R44, R152: the AI plays out the rest of the turn My Pawn took, not the player's next turn.
//!  - §10.3: the AI turn's events are dispatched once, not again by the enclosing action.
//!  - R220 (round 6): a declared attack resolves only while it stands as it was declared — a trap in
//!    §4.2 step 4's window that destroyed, stole or moved the attacker or its target, or swapped the
//!    boards, ends it, and a My Pawn later in the window has nothing to answer. No Core trap but My
//!    Pawn answers a declaration, and My Pawn cancels, so the trap in the window is a fixture (a
//!    transient def and its script in the registry, as paused-sequences.test.ts builds them).
//!  - R176 (round 6): a defender's Lifesteal strike back heals its hero in the same combat, so a
//!    Trample swing it outheals is not lethal.
//!  - R212 (round 7): the ordinary triggers on a declaration are queued after the window, and meet
//!    the board as it stood when the attack was declared: a unit a trap in the window stole answers
//!    for the player who controlled it then, and one a trap summoned there answers nothing.
//!  - R176 (round 8): that strike back heals what it really deals — with Trample, only up to the
//!    attacker's health on the unit, and its excess through the attacking hero's Armor.
//!  - R220, §10.3 (round 8): a trap answering what a trap in the window did (its hit on the attacker)
//!    fires inside the window, before step 5, with or without a question first.

use jackioh_engine::testkit::*;

const TIMMY: &str = "core-011";
const BIGOT: &str = "core-002";
const RIGHT_HOUSE: &str = "core-003";
const SCARAB: &str = "core-007";
const GARY: &str = "core-004";
const BIG_D: &str = "core-001";
const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const HIT_JOB: &str = "core-016";
const MENACE: &str = "core-019";
const POINTMASTER: &str = "core-020";
const SEVEN_SEVEN: &str = "core-025";
const GIGA: &str = "core-029";
const PANTHER: &str = "core-032";
const CLONE_MACHINE: &str = "core-033";
const DUELIST: &str = "core-045";
const RENO: &str = "core-053";
const SORCERER: &str = "core-068";
const MY_PAWN: &str = "core-096";
const JILLIAX: &str = "core-056";
const GOING_LONG: &str = "core-084";
const WINDOW_LIBRARY: [&str; 6] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

/// The harness with the real catalog and every card script registered (TS's `_harness.ts` import
/// ran `registerAll()`; the engine's testkit cannot name the cards crate, so the cards test does).
fn setup(opts: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn count(s: &Scenario, type_: GameEventType) -> usize {
    s.events()
        .iter()
        .filter(|event| event.event_type() == type_)
        .count()
}

fn aim(card: &CardInstance) -> Value {
    json!([{ "pick": "instance", "instanceId": card.id }])
}

fn exertion_json(s: &Scenario, card: &CardInstance) -> Value {
    serde_json::to_value(s.card(card).exertion).unwrap()
}

fn keyword_kinds(s: &Scenario, card: &CardInstance) -> Vec<String> {
    s.stats(card)
        .keywords
        .iter()
        .map(|k| {
            serde_json::to_value(k).unwrap()["kind"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

/// R42 and R89: the unit whose damage instance was lethal
mod r42_and_r89_the_unit_whose_damage_instance_was_lethal {
    use super::*;

    #[test]
    fn r42_a_unit_prem_panther_damaged_earlier_and_a_hit_job_later_destroyed_was_not_destroyed_by_the_panther()
     {
        let mut s = setup(json!({
            "seed": "hunt-cw-panther-stale",
            "p1": { "field": [PANTHER], "hand": [HIT_JOB, STOCKPILE], "library": [TIMMY, TIMMY, TIMMY] },
            "p2": { "field": [BIG_D] },
        }));
        let dfender = s.card(BIG_D).clone();

        // 5 damage on a 0/7: it survives, and it deals nothing back.
        s.attack(PANTHER, &dfender);
        s.expect_stats(&dfender, json!({ "health": 2 }));
        let hand_before = s.hand("p1").len();

        // Hit Job destroys it: no damage instance at all, so no killer and no draw.
        s.play(HIT_JOB, json!({ "targets": aim(&dfender) }));

        s.expect_in_zone(&dfender, "graveyard");
        let destroyed = s
            .last_events()
            .iter()
            .find(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == dfender.id))
            .cloned()
            .expect("a destroyed event for the D-fender");
        assert!(
            matches!(destroyed, GameEvent::Destroyed { killer_id: None, .. }),
            "{destroyed:?} should match {{ killerId: null }}"
        );
        assert_eq!(s.hand("p1").len(), hand_before - 1);
    }
}

/// R176: My Pawn's projection follows the combat
mod r176_my_pawns_projection_follows_the_combat {
    use super::*;

    #[test]
    fn r176_r93_an_attacker_a_first_strike_defender_kills_first_never_lands_its_trample_hit_so_it_is_not_lethal_s4_3()
     {
        // p1's Twisted Sorcerer (5/5) with Trample swings at p2's Pointmaster (7/1, First Strike) with
        // p2 at 3. Pointmaster strikes first for 7 and the Sorcerer deals nothing.
        let mut s = setup(json!({
            "seed": "hunt-cw-pawn-first-strike",
            "p1": { "field": [SORCERER], "hand": [STOCKPILE], "library": [TIMMY, TIMMY] },
            "p2": { "health": 3, "field": [POINTMASTER], "backrow": [{ "def": MY_PAWN, "faceUp": false }], "library": [GIGA, GIGA] },
        }));
        s.card_mut(SORCERER).granted_keywords = vec![Keyword::Trample];
        let sorcerer = s.card(SORCERER).clone();
        let pointmaster = s.card(POINTMASTER).clone();

        s.attack(&sorcerer, &pointmaster);

        assert_eq!(count(&s, GameEventType::AttackCancelled), 0);
        s.expect_in_zone(&sorcerer, "graveyard");
        s.expect_stats(&pointmaster, json!({ "health": 1 }));
        s.expect_health("p2", 3);
        s.expect_in_zone(MY_PAWN, "field");
    }

    #[test]
    fn r176_r63_trample_excess_from_the_attacks_cleave_hits_counts_toward_lethal_s4_4_step_10() {
        // p1's radiant Prem Panther (10 attack, Cleave) with Trample attacks p2's Right-house defender
        // (Taunt, Divine Shield) between two 1/1s. The shield stops the hit on the defender, but Cleave
        // hits each 1/1 for 10 and Trample sends 9 + 9 = 18 to p2's hero at 18: My Pawn cancels it.
        let mut s = setup(json!({
            "seed": "hunt-cw-pawn-cleave",
            "p1": { "field": [{ "def": PANTHER, "radiant": true }], "hand": [STOCKPILE], "library": [TIMMY, TIMMY] },
            "p2": {
                "health": 18,
                "field": [SCARAB, RIGHT_HOUSE, GARY],
                "backrow": [{ "def": MY_PAWN, "faceUp": false }],
                "hand": [STOCKPILE],
                "library": [GIGA, GIGA],
            },
        }));
        let panther = s.card(PANTHER).clone();
        s.card_mut(&panther).granted_keywords = vec![Keyword::Trample];

        s.attack(&panther, RIGHT_HOUSE);

        let cancelled = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::AttackCancelled { attacker_id, .. } if *attacker_id == panther.id))
            .count();
        assert_eq!(cancelled, 1);
        assert!(s.state().result.is_none());
    }

    #[test]
    fn r176_one_hit_short_of_lethal_through_cleave_is_not_lethal_so_the_attack_goes_through() {
        // The same board with p2 at 19: 18 through Trample is not enough, so My Pawn stays armed.
        let mut s = setup(json!({
            "seed": "hunt-cw-pawn-cleave",
            "p1": { "field": [{ "def": PANTHER, "radiant": true }], "hand": [STOCKPILE], "library": [TIMMY, TIMMY] },
            "p2": {
                "health": 19,
                "field": [SCARAB, RIGHT_HOUSE, GARY],
                "backrow": [{ "def": MY_PAWN, "faceUp": false }],
                "hand": [STOCKPILE],
                "library": [GIGA, GIGA],
            },
        }));
        let panther = s.card(PANTHER).clone();
        s.card_mut(&panther).granted_keywords = vec![Keyword::Trample];

        s.attack(&panther, RIGHT_HOUSE);

        assert_eq!(count(&s, GameEventType::AttackCancelled), 0);
        s.expect_health("p2", 1);
        s.expect_in_zone(MY_PAWN, "field");
    }
}

/// §5.1, R44, R152: My Pawn and the AI turn it hands over
mod s5_1_r44_r152_my_pawn_and_the_ai_turn_it_hands_over {
    use super::*;

    #[test]
    fn s5_1_my_pawn_fires_once_it_cannot_fire_again_on_an_attack_its_own_ai_turn_declares() {
        // My Pawn cancels p1's lethal Sorcerer and hands p1's turn to the AI. p1's Deft Duelist, which
        // has already switched, can still attack the hero for 4, which is lethal at 4 — and a Trap that
        // has fired is spent (§5.1), so nothing cancels it.
        let mut s = setup(json!({
            "seed": "hunt-cw-pawn-twice-a",
            "p1": { "field": [SORCERER, DUELIST], "library": [GIGA, GIGA] },
            "p2": { "health": 4, "backrow": [{ "def": MY_PAWN, "faceUp": false }], "library": [GIGA, GIGA] },
        }));
        s.card_mut(DUELIST).exertion.switched = true;

        s.attack(SORCERER, "hero");

        assert_eq!(count(&s, GameEventType::TrapFired), 1);
        // The Duelist's swing ends the game inside the AI turn, and nothing happens after that (R216):
        // the trap that fired once is not consumed afterwards, so it is still in the backrow, face-up.
        assert_eq!(
            serde_json::to_value(s.state().result).unwrap(),
            json!({ "winner": "p1", "reason": "hero-death" })
        );
        let to_graveyard = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::EnteredGraveyard { def_id, .. } if def_id == MY_PAWN))
            .count();
        assert_eq!(to_graveyard, 0);
        let pawn = s.backrow("p2", 1).expect("My Pawn in p2's backrow lane 1");
        assert_eq!(pawn.def_id, MY_PAWN);
        assert_eq!(pawn.face_up, Some(true));
    }

    #[test]
    fn r152_r44_r82_the_ai_plays_out_only_the_rest_of_the_turn_my_pawn_took_not_the_players_next_turn() {
        // My Pawn cancels p1's lethal swing on turn 9 and the AI ends p1's turn. p2 has nothing it can
        // do, so R82 auto-ends turn 10 inside that same reduction and p1's turn 11 begins — p1's own.
        let mut s = setup(json!({
            "seed": "hunt-cw-pawn-next-turn",
            "p1": { "field": [SORCERER], "library": [GIGA, GIGA, GIGA] },
            "p2": { "health": 5, "backrow": [{ "def": MY_PAWN, "faceUp": false }], "library": [GIGA, GIGA] },
        }));
        let sorcerer = s.card(SORCERER).clone();

        s.attack(&sorcerer, "hero");

        assert!(s.state().result.is_none());
        assert_eq!(s.state().turn, 11);
        assert_eq!(s.state().active, PlayerId::P1);
        assert!(!s.state().players.p1.ai_turn);
        assert_eq!(
            exertion_json(&s, &sorcerer),
            json!({ "attacked": false, "switched": false })
        );
    }

    #[test]
    fn r42_s10_3_an_event_the_ai_turn_emitted_is_dispatched_once_prem_panther_draws_2_per_kill_not_4() {
        // On this seed the AI sends p1's Prem Panther into p2's 1/1 Scarab and kills it.
        let mut s = setup(json!({
            "seed": "hunt-cw-redispatch-d",
            "p1": { "field": [MENACE, PANTHER], "library": [GIGA, GIGA, GIGA, GIGA, GIGA, GIGA] },
            "p2": {
                "health": 9,
                "field": [SCARAB],
                "backrow": [{ "def": MY_PAWN, "faceUp": false }],
                "hand": [STOCKPILE],
                "library": [GIGA, GIGA],
            },
        }));
        let panther = s.card(PANTHER).clone();

        s.attack(MENACE, "hero");

        let kills = s
            .events()
            .iter()
            .filter(|event| {
                matches!(event, GameEvent::Destroyed { killer_id: Some(killer), .. } if *killer == panther.id)
            })
            .count();
        let draws = s
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    GameEvent::Drawn {
                        player: PlayerId::P1,
                        ..
                    }
                )
            })
            .count();
        // The seed's AI does make the kill; without it this test proves nothing.
        assert_eq!(kills, 1);
        assert_eq!(draws, 2);
        assert_eq!(s.hand("p1").len(), 2);
    }

    #[test]
    fn s10_3_an_ai_turns_play_reaches_unstable_clone_machine_once_three_copies_not_six_s8_c33() {
        // p1 swings lethal; p2's My Pawn cancels and the AI plays out p1's turn, playing its one hand
        // card, Mr. Vanilla, with p1's Unstable Clone Machine out.
        let mut s = setup(json!({
            "seed": "pawn-dispatch",
            "p1": {
                "hand": [VANILLA],
                "field": [{ "def": POINTMASTER, "lane": 1 }],
                "backrow": [{ "def": CLONE_MACHINE, "lane": 1 }],
                "library": [RENO, RENO, RENO, RENO, RENO, RENO],
            },
            "p2": {
                "hand": [RENO, RENO],
                "health": 5,
                "backrow": [{ "def": MY_PAWN, "lane": 2 }],
                "field": [{ "def": SEVEN_SEVEN, "lane": 3 }],
                "library": [RENO, RENO, RENO, RENO, RENO, RENO],
            },
        }));

        s.attack(POINTMASTER, "hero");

        let vanilla_plays = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == VANILLA))
            .count();
        assert_eq!(vanilla_plays, 1);
        s.expect_health("p2", 5);
        assert_eq!(
            s.pile("p1", "library")
                .iter()
                .filter(|card| card.def_id == VANILLA)
                .count(),
            3
        );
    }
}

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("expected {what}"))
}

/// A fixture card's definition, as TS's object literal (`face` is the printed face of both sides).
fn fixture_def(id: &str, type_: &str, face: Value) -> CardDef {
    json_as(json!({
        "id": id,
        "index": id,
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

/// TS `registerScripts({ ...registeredScripts(), [id]: { base: script, radiant: script } })`.
fn register_fixture_script(id: &str, script: Script) {
    let mut all: IndexMap<String, CardScripts> = scripts::registered_scripts().clone();
    all.insert(
        id.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(all);
}

/// A fixture card: a transient def in the match state and its script in the registry.
fn fixture(s: &mut Scenario, id: &str, type_: &str, script: Script) {
    let face = json!({ "keywords": [], "text": id });
    s.state_mut()
        .transient_defs
        .insert(id.to_string(), fixture_def(id, type_, face));
    register_fixture_script(id, script);
}

/// A face-down trap of `player`'s in a backrow lane (R33).
fn set_trap(s: &mut Scenario, def_id: &str, player: PlayerId, lane: i32) -> CardInstance {
    let row = Row::Backrow;
    let state = s.state_mut();
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    if !zones::place_on_field(
        state,
        &mut card,
        ZoneRef { player, row, lane },
        Default::default(),
    ) {
        panic!("could not place {def_id}");
    }
    let live = find_instance_mut(state, &card.id).unwrap_or_else(|| panic!("could not place {def_id}"));
    live.face_up = Some(false);
    live.clone()
}

/// The `attackDeclared` event's fields, when the event is one and was not forced.
fn unforced_declaration(event: &GameEvent) -> Option<(&String, &String)> {
    match event {
        GameEvent::AttackDeclared {
            attacker_id,
            target_id,
            forced: false,
            ..
        } => Some((attacker_id, target_id)),
        _ => None,
    }
}

/// "When a unit is declared as an attacker (not forced): destroy it."
fn vaporize() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "edge-r6-vaporize",
                &[GameEventType::AttackDeclared],
                |_ctx, event| match event {
                    GameEvent::AttackDeclared { attacker_id, .. } => vec![effects::destroy(json_as(json!({
                        "target": { "of": "instance", "instanceId": attacker_id }
                    })))],
                    _ => vec![],
                },
            )
            .with_when(|_ctx, event| unforced_declaration(event).is_some()),
        ],
        ..Script::default()
    }
}

/// "When a unit is declared as an attack's target (not forced): destroy it."
fn shatter() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "edge-r6-shatter",
                &[GameEventType::AttackDeclared],
                |_ctx, event| match event {
                    GameEvent::AttackDeclared { target_id, .. } => vec![effects::destroy(json_as(json!({
                        "target": { "of": "instance", "instanceId": target_id }
                    })))],
                    _ => vec![],
                },
            )
            .with_when(|_ctx, event| {
                unforced_declaration(event).is_some_and(|(_, target_id)| !target_id.starts_with("hero-"))
            }),
        ],
        ..Script::default()
    }
}

/// "When a unit is declared as an attacker (not forced): take control of it."
fn turncoat() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "edge-r6-turncoat",
                &[GameEventType::AttackDeclared],
                |_ctx, event| match event {
                    GameEvent::AttackDeclared { attacker_id, .. } => {
                        vec![effects::steal(json_as(json!({ "instanceId": attacker_id })))]
                    }
                    _ => vec![],
                },
            )
            .with_when(|_ctx, event| unforced_declaration(event).is_some()),
        ],
        ..Script::default()
    }
}

/// "When a unit is declared as an attack's target (not forced): take control of it."
fn defector() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "edge-r6-defector",
                &[GameEventType::AttackDeclared],
                |_ctx, event| match event {
                    GameEvent::AttackDeclared { target_id, .. } => {
                        vec![effects::steal(json_as(json!({ "instanceId": target_id })))]
                    }
                    _ => vec![],
                },
            )
            .with_when(|_ctx, event| {
                unforced_declaration(event).is_some_and(|(_, target_id)| !target_id.starts_with("hero-"))
            }),
        ],
        ..Script::default()
    }
}

/// "When a unit is declared as an attacker: swap the boards."
fn swapper() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "edge-r6-swapper",
                &[GameEventType::AttackDeclared],
                |_ctx, _event| vec![effects::swap_board()],
            )
            .with_when(|_ctx, event| unforced_declaration(event).is_some()),
        ],
        ..Script::default()
    }
}

/// The attacker id a trap stored in its question's data (TS `String(ctx.data.attackerId)`).
fn stored_attacker_id(ctx: &EffectContext<'_>) -> String {
    match ctx.data.get("attackerId") {
        Some(Value::String(id)) => id.clone(),
        Some(other) => other.to_string(),
        None => "undefined".to_string(),
    }
}

/// A face-down trap in the §4.2 step-4 window that asks its controller about the opponent's declared
/// attack, and whose answer then does `then` to the attacker — so the window pauses, and step 5 is
/// owed to the answer's action (R113).
fn asking_window_trap(s: &mut Scenario, id: &str, then: &'static str) {
    let prompt = format!("{id}: a question");
    fixture(
        s,
        id,
        "Trap",
        Script {
            triggers: vec![
                TriggerDef::new(
                    format!("{id}:asks"),
                    &[GameEventType::AttackDeclared],
                    move |_ctx, event| {
                        let attacker_id = match event {
                            GameEvent::AttackDeclared { attacker_id, .. } => attacker_id.clone(),
                            _ => String::new(),
                        };
                        vec![effects::choose_mode(json_as(json!({
                            "options": ["ok"],
                            "step": "answered",
                            "prompt": prompt,
                            "data": { "attackerId": attacker_id },
                        })))]
                    },
                )
                .with_when(|ctx, event| {
                    unforced_declaration(event).is_some() && ctx.state.active != ctx.controller
                }),
            ],
            resume: [(
                "answered",
                hook(move |ctx| {
                    let attacker_id = stored_attacker_id(ctx);
                    if then == "steal" {
                        vec![effects::steal(json_as(json!({ "instanceId": attacker_id })))]
                    } else {
                        vec![effects::destroy(json_as(json!({
                            "target": { "of": "instance", "instanceId": attacker_id }
                        })))]
                    }
                }),
            )]
            .into_iter()
            .collect(),
            ..Script::default()
        },
    );
}

/// R220: §4.2 step 5 resolves the attack only as it was declared
mod r220_s4_2_step_5_resolves_the_attack_only_as_it_was_declared {
    use super::*;

    #[test]
    fn r220_r174_r83_an_attacker_a_trap_in_the_window_destroyed_does_not_attack_with_the_reborn_body_that_came_back()
     {
        let mut s = setup(json!({
            "seed": "edge-r6-vaporize-reborn",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
            "p2": { "health": 20, "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
        }));
        fixture(&mut s, "edge-r6-vaporize", "Trap", vaporize());
        set_trap(&mut s, "edge-r6-vaporize", PlayerId::P2, 1);
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");
        s.card_mut(&sorcerer).granted_keywords.push(Keyword::Reborn);

        s.attack(&sorcerer, "hero");

        // The trap destroyed the declared attacker and Reborn put a new arrival in its zone (R83).
        assert!(s.events().iter().any(
            |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == sorcerer.id)
        ));
        let body = must(s.unit("p1", 1), "the Reborn body");
        assert_eq!(body.id, sorcerer.id);
        // The attack was the stay that died; the body declared nothing and deals nothing.
        s.expect_health("p2", 20);
        assert_eq!(count(&s, GameEventType::Damage), 0);
    }

    #[test]
    fn r220_r174_r83_an_attack_whose_target_a_trap_in_the_window_destroyed_does_not_land_on_the_targets_reborn_body()
     {
        let mut s = setup(json!({
            "seed": "edge-r6-shatter-reborn",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
            "p2": { "field": [{ "def": RIGHT_HOUSE, "lane": 2 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
        }));
        fixture(&mut s, "edge-r6-shatter", "Trap", shatter());
        set_trap(&mut s, "edge-r6-shatter", PlayerId::P1, 1);
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");
        let defender = must(s.unit("p2", 2), "p2's Right-house defender");

        s.attack(&sorcerer, &defender);

        assert!(s.events().iter().any(
            |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == defender.id)
        ));
        let body = must(s.unit("p2", 2), "the Reborn body");
        assert_eq!(body.id, defender.id);
        // The body is a new arrival with a fresh Divine Shield; the attack that named the old stay is
        // over, so nothing strikes it and it strikes nothing back.
        assert_eq!(count(&s, GameEventType::DivineShieldLost), 0);
        assert_eq!(s.card(&sorcerer).damage, 0);
        assert!(keyword_kinds(&s, &body).contains(&"Divine Shield".to_string()));
    }

    #[test]
    fn r220_r173_r171_an_attacker_a_trap_in_the_window_stole_does_not_go_on_to_hit_its_new_controllers_hero_s4_2_step_2()
     {
        let mut s = setup(json!({
            "seed": "edge-r6-turncoat",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
            "p2": { "health": 20, "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
        }));
        fixture(&mut s, "edge-r6-turncoat", "Trap", turncoat());
        set_trap(&mut s, "edge-r6-turncoat", PlayerId::P2, 1);
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");

        s.attack(&sorcerer, "hero");

        assert_eq!(s.card(&sorcerer).controller, PlayerId::P2);
        // p2's own unit does not attack p2's hero: an attack is made on an enemy (§4.2 step 2).
        s.expect_health("p2", 20);
        // The attack is over, but the attacker changed sides: R171 gives it a fresh exertion for p2,
        // and nothing spends it again as the attack ends.
        assert_eq!(
            exertion_json(&s, &sorcerer),
            json!({ "attacked": false, "switched": false })
        );
    }

    #[test]
    fn r220_r173_r171_an_attack_whose_target_a_trap_in_the_window_moved_to_the_attackers_side_does_not_hit_it_s4_2_step_2()
     {
        let mut s = setup(json!({
            "seed": "edge-r6-defector",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
            "p2": { "field": [{ "def": VANILLA, "lane": 3 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
        }));
        fixture(&mut s, "edge-r6-defector", "Trap", defector());
        set_trap(&mut s, "edge-r6-defector", PlayerId::P1, 1);
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");
        let vanilla = must(s.unit("p2", 3), "p2's Mr. Vanilla");

        s.attack(&sorcerer, &vanilla);

        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::ControlChanged { instance_id, controller: PlayerId::P1, .. } if *instance_id == vanilla.id
        )));
        // Both are p1's now: no friendly combat, so neither strikes the other.
        assert_eq!(count(&s, GameEventType::Damage), 0);
        s.expect_in_zone(&vanilla, "field");
        assert_eq!(s.card(&vanilla).controller, PlayerId::P1);
        assert_eq!(s.card(&sorcerer).damage, 0);
        // The attacker stayed p1's, so its exertion stays spent (R44); the target that changed sides has
        // a fresh one (R171).
        assert!(s.card(&sorcerer).exertion.attacked);
        assert_eq!(
            exertion_json(&s, &vanilla),
            json!({ "attacked": false, "switched": false })
        );
    }

    #[test]
    fn r220_r73_r171_a_board_swap_in_the_window_leaves_the_declaring_players_attacker_on_the_other_side_no_combat()
     {
        let mut s = setup(json!({
            "seed": "edge-r6-swapper",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
            "p2": { "field": [{ "def": VANILLA, "lane": 3 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
        }));
        fixture(&mut s, "edge-r6-swapper", "Trap", swapper());
        set_trap(&mut s, "edge-r6-swapper", PlayerId::P2, 5);
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");
        let vanilla = must(s.unit("p2", 3), "p2's Mr. Vanilla");

        s.attack(&sorcerer, &vanilla);

        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::ControlChanged { instance_id, controller: PlayerId::P2, .. } if *instance_id == sorcerer.id
        )));
        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::ControlChanged { instance_id, controller: PlayerId::P1, .. } if *instance_id == vanilla.id
        )));
        // p1 declared with a unit it no longer controls: that attack is over, and neither unit strikes.
        assert_eq!(count(&s, GameEventType::Damage), 0);
        s.expect_in_zone(&vanilla, "field");
        // Both changed sides, so both have a fresh exertion for their new controller (R171).
        assert_eq!(
            exertion_json(&s, &sorcerer),
            json!({ "attacked": false, "switched": false })
        );
        assert_eq!(
            exertion_json(&s, &vanilla),
            json!({ "attacked": false, "switched": false })
        );
    }

    #[test]
    fn r220_r113_r173_an_attacker_the_windows_trap_stole_after_a_question_never_strikes_its_new_controllers_hero()
     {
        let mut s = setup(json!({
            "p1": { "field": [TIMMY], "hand": [RENO] },
            "p2": { "hand": [RENO] },
        }));
        asking_window_trap(&mut s, "edge-r6-l7-window-steal", "steal");
        set_trap(&mut s, "edge-r6-l7-window-steal", PlayerId::P2, 1);
        let timmy = must(s.unit("p1", 1), "p1's Tempo Timmy");

        s.attack(&timmy, "hero");
        assert_eq!(
            must(s.state().pending.as_ref(), "the window trap's question").player_id,
            PlayerId::P2
        );
        s.answer(json!("ok"));

        // The window's trap took Timmy: it is p2's now, and the attack p1 declared at p2's hero would be
        // p2's own unit hitting p2's own hero. Every attack is made on an enemy (§4.2 step 2, R173).
        assert_eq!(s.card(&timmy).controller, PlayerId::P2);
        s.expect_health("p2", 30);
    }

    #[test]
    fn r220_r174_r83_r113_an_attacker_the_windows_trap_killed_after_a_question_does_not_attack_with_its_reborn_body()
     {
        let mut s = setup(json!({
            "p1": { "field": [RIGHT_HOUSE], "hand": [RENO] },
            "p2": { "hand": [RENO] },
        }));
        asking_window_trap(&mut s, "edge-r6-l7-window-kill", "destroy");
        set_trap(&mut s, "edge-r6-l7-window-kill", PlayerId::P2, 1);
        let defender = must(s.unit("p1", 1), "p1's Right-house defender");

        s.attack(&defender, "hero");
        s.answer(json!("ok"));

        // The declared attacker died in the window and came back through Reborn: a new arrival, sick
        // this turn (R83), and not the stay that declared the attack (R174). Nothing hits p2's hero.
        assert_eq!(s.card(&defender).reborn_spent, Some(true));
        s.expect_health("p2", 30);
    }

    #[test]
    fn r220_r44_r99_my_pawn_does_not_fire_on_a_declaration_whose_attacker_an_earlier_trap_in_the_window_destroyed()
     {
        // p2's fixture trap in lane 1 destroys p1's attacking Twisted Sorcerer before My Pawn in lane 2
        // is offered the declaration (R68). The attacker is in its graveyard: no attack is left to be
        // lethal, so My Pawn stays armed and p1 keeps its turn.
        let mut s = setup(json!({
            "seed": "edge-r6-vaporize-pawn",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": [GIGA, GIGA, GIGA] },
            "p2": {
                "health": 5,
                "hand": [STOCKPILE],
                "backrow": [{ "def": MY_PAWN, "lane": 2, "faceUp": false }],
                "library": [GIGA, GIGA],
            },
        }));
        fixture(&mut s, "edge-r6-vaporize", "Trap", vaporize());
        set_trap(&mut s, "edge-r6-vaporize", PlayerId::P2, 1);
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");

        s.attack(&sorcerer, "hero");

        s.expect_in_zone(&sorcerer, "graveyard");
        let pawn_fired = s
            .events()
            .iter()
            .any(|event| matches!(event, GameEvent::TrapFired { def_id, .. } if def_id == MY_PAWN));
        assert!(!pawn_fired);
        assert!(!s.state().players.p1.ai_turn);
        assert_eq!(s.backrow("p2", 2).map(|c| c.def_id), Some(MY_PAWN.to_string()));
        assert_eq!(s.backrow("p2", 2).and_then(|c| c.face_up), Some(false));
    }
}

/// R176: the hero My Pawn projects is the one the combat leaves
mod r176_the_hero_my_pawn_projects_is_the_one_the_combat_leaves {
    use super::*;

    #[test]
    fn r176_a_trample_swing_the_defenders_lifesteal_strike_back_outheals_in_the_same_combat_is_not_lethal_s4_3_s4_4_step_8()
     {
        // p1's 5/5 Twisted Sorcerer with Trample attacks p2's Jilliax (3/2, Lifesteal, Taunt; its Divine
        // Shield spent) with p2 at 3. Trample sends 3 through, and Jilliax's strike back heals p2 for 3
        // in the same combat, so the check after it finds p2 at 3: the attack would not be lethal.
        let mut s = setup(json!({
            "seed": "edge-r6-pawn-lifesteal",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": [GIGA, GIGA, GIGA] },
            "p2": {
                "health": 3,
                "field": [{ "def": JILLIAX, "lane": 1 }],
                "backrow": [{ "def": MY_PAWN, "lane": 2, "faceUp": false }],
                "hand": [STOCKPILE],
                "library": [GIGA, GIGA],
            },
        }));
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");
        let jilliax = must(s.unit("p2", 1), "p2's Jilliax");
        s.card_mut(&sorcerer).granted_keywords.push(Keyword::Trample);
        s.card_mut(&jilliax).divine_shield_spent = Some(true);

        s.attack(&sorcerer, &jilliax);

        assert_eq!(count(&s, GameEventType::AttackCancelled), 0);
        assert!(s.state().result.is_none());
        s.expect_health("p2", 3);
        s.expect_in_zone(&jilliax, "graveyard");
    }
}

// ---------------------------------------------------------------------------
// Round 7 (lens "combat windows"): the ordinary triggers on a declaration meet the board as it stood
// when the attack was declared (R212), whatever a trap in the window did to it since.
// ---------------------------------------------------------------------------

/// A fixture unit with a face of its own: a transient def and its script in the registry.
fn fixture_unit(s: &mut Scenario, id: &str, script: Script, stats: AttackHealth) {
    let face = json!({ "attack": stats.attack, "health": stats.health, "keywords": [], "text": id });
    s.state_mut()
        .transient_defs
        .insert(id.to_string(), fixture_def(id, "Unit", face));
    register_fixture_script(id, script);
}

/// A fixture unit of `player`'s in Attack Position in a unit lane.
fn place_unit(s: &mut Scenario, def_id: &str, player: PlayerId, lane: i32) -> CardInstance {
    let state = s.state_mut();
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    let slot = ZoneRef {
        player,
        row: Row::Units,
        lane,
    };
    if !zones::place_on_field(state, &mut card, slot, Default::default()) {
        panic!("could not place {def_id}");
    }
    let live = find_instance_mut(state, &card.id).unwrap_or_else(|| panic!("could not place {def_id}"));
    live.position = Some(Position::Atk);
    live.clone()
}

/// "Whenever a unit is declared as an attacker (not forced): you draw a card." A unit, not a trap.
fn watcher() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "edge-r7-watcher",
                &[GameEventType::AttackDeclared],
                |_ctx, _event| vec![effects::draw(json_as(json!({ "count": 1 })))],
            )
            .with_when(|_ctx, event| unforced_declaration(event).is_some()),
        ],
        ..Script::default()
    }
}

/// A trap: "When a unit is declared as an attacker (not forced): summon a Watcher."
fn caller() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "edge-r7-caller",
                &[GameEventType::AttackDeclared],
                |_ctx, _event| vec![effects::summon(json_as(json!({ "defId": "edge-r7-watcher" })))],
            )
            .with_when(|_ctx, event| unforced_declaration(event).is_some()),
        ],
        ..Script::default()
    }
}

/// R212: a declaration is answered as the board stood when it was declared
mod r212_a_declaration_is_answered_as_the_board_stood_when_it_was_declared {
    use super::*;

    #[test]
    fn r212_r171_a_unit_a_trap_in_the_window_stole_answers_the_declaration_for_the_player_who_controlled_it_then_s10_3()
     {
        // p1's Sorcerer attacks p2's Watcher. p1's own trap in the window steals the Watcher, so the
        // attack is over (R220). The Watcher's "whenever a unit attacks, you draw" answers a declaration
        // made while p2 controlled it: p2 draws, not p1 (R212's "a card whose controller has changed
        // since answers for the one it had").
        let mut s = setup(json!({
            "seed": "edge-r7-watcher-stolen",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
            "p2": { "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
        }));
        fixture_unit(
            &mut s,
            "edge-r7-watcher",
            watcher(),
            AttackHealth { attack: 2, health: 2 },
        );
        fixture(&mut s, "edge-r6-defector", "Trap", defector());
        let watcher_card = place_unit(&mut s, "edge-r7-watcher", PlayerId::P2, 2);
        set_trap(&mut s, "edge-r6-defector", PlayerId::P1, 1);
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");
        let p1_hand = s.hand("p1").len();
        let p2_hand = s.hand("p2").len();

        s.attack(&sorcerer, &watcher_card);

        assert_eq!(s.card(&watcher_card).controller, PlayerId::P1);
        let drawn: Vec<PlayerId> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn { player, .. } => Some(*player),
                _ => None,
            })
            .collect();
        assert_eq!(drawn, [PlayerId::P2]);
        assert_eq!(s.hand("p2").len(), p2_hand + 1);
        assert_eq!(s.hand("p1").len(), p1_hand);
    }

    #[test]
    fn r212_r174_a_unit_a_trap_in_the_window_summoned_does_not_answer_the_declaration_made_before_it_arrived_s10_3()
     {
        // p1's Sorcerer attacks p2's hero. p2's trap in the window summons a Watcher for p2. The
        // Watcher was not on the field when the attack was declared, so it draws nothing for it.
        let mut s = setup(json!({
            "seed": "edge-r7-watcher-arrives",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
            "p2": { "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
        }));
        fixture_unit(
            &mut s,
            "edge-r7-watcher",
            watcher(),
            AttackHealth { attack: 2, health: 2 },
        );
        fixture(&mut s, "edge-r7-caller", "Trap", caller());
        set_trap(&mut s, "edge-r7-caller", PlayerId::P2, 1);
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");
        let p2_hand = s.hand("p2").len();

        s.attack(&sorcerer, "hero");

        assert_eq!(
            s.unit("p2", 1).map(|c| c.def_id),
            Some("edge-r7-watcher".to_string())
        );
        assert_eq!(count(&s, GameEventType::Drawn), 0);
        assert_eq!(s.hand("p2").len(), p2_hand);
        // The attack itself went through.
        s.expect_health("p2", 25);
    }
}

// ---------------------------------------------------------------------------------------------
// Round 8: the strike back's Trample split, and the window's own chain before step 5
// ---------------------------------------------------------------------------------------------

/// R176: a Lifesteal strike back that tramples heals what it really deals
mod r176_a_lifesteal_strike_back_that_tramples_heals_what_it_really_deals {
    use super::*;

    #[test]
    fn r176_r63_a_defenders_lifesteal_heals_only_what_its_trample_strike_back_really_deals_so_the_swing_is_lethal_s4_4_steps_2_8_9()
     {
        // p1's Bigot (6/1) with Trample attacks p2's Jilliax (3/2, Taunt, Lifesteal, shield spent) with
        // Trample, p2 at 3, and p1's hero behind Going Long (Armor 2). The swing sends 6 - 2 = 4 through
        // to p2. Jilliax strikes back 3 into a 1-health Bigot: 1 lands and heals p2 for 1, and the 2 that
        // tramples on is stopped by p1's Armor 2 (the zero rule), so it heals nothing. p2 ends the combat
        // at 3 - 4 + 1 = 0: the attack is lethal, and My Pawn cancels it.
        let mut s = setup(json!({
            "seed": "cw8-pawn-trample-strikeback",
            "p1": {
                "field": [{ "def": BIGOT, "lane": 1 }],
                "backrow": [{ "def": GOING_LONG, "lane": 1 }],
                "hand": [STOCKPILE],
                "library": [GIGA, GIGA, GIGA],
            },
            "p2": {
                "health": 3,
                "field": [{ "def": JILLIAX, "lane": 1 }],
                "backrow": [{ "def": MY_PAWN, "lane": 2, "faceUp": false }],
                "hand": [STOCKPILE],
                "library": [GIGA, GIGA],
            },
        }));
        let bigot = must(s.unit("p1", 1), "p1's Bigot");
        let jilliax = must(s.unit("p2", 1), "p2's Jilliax");
        s.card_mut(&bigot).granted_keywords.push(Keyword::Trample);
        s.card_mut(&jilliax).granted_keywords.push(Keyword::Trample);
        s.card_mut(&jilliax).divine_shield_spent = Some(true);

        s.attack(&bigot, &jilliax);

        assert_eq!(count(&s, GameEventType::AttackCancelled), 1);
        assert!(s.state().result.is_none());
        s.expect_health("p2", 3);
    }
}

/// "When a unit is declared as an attacker (not forced): deal 1 damage to it."
fn prick() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "cw8-prick",
                &[GameEventType::AttackDeclared],
                |_ctx, event| match event {
                    GameEvent::AttackDeclared { attacker_id, .. } => vec![effects::damage(json_as(json!({
                        "to": { "of": "instance", "instanceId": attacker_id },
                        "amount": 1,
                    })))],
                    _ => vec![],
                },
            )
            .with_when(|_ctx, event| unforced_declaration(event).is_some()),
        ],
        ..Script::default()
    }
}

/// "When an enemy unit takes damage: destroy it."
fn snap() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("cw8-snap", &[GameEventType::Damage], |_ctx, event| match event {
                GameEvent::Damage { target_id, .. } => vec![effects::destroy(json_as(json!({
                    "target": { "of": "instance", "instanceId": target_id }
                })))],
                _ => vec![],
            })
            .with_when(|ctx, event| {
                let GameEvent::Damage { target_id, .. } = event else {
                    return false;
                };
                if target_id.starts_with("hero-") {
                    return false;
                }
                match find_instance(&*ctx.state, target_id) {
                    Some(unit) => unit.zone.z() == ZoneName::Field && unit.controller != ctx.controller,
                    None => false,
                }
            }),
        ],
        ..Script::default()
    }
}

/// As PRICK, but it asks its controller a question first, so the window pauses (R113).
fn ask_prick() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "cw8-ask-prick",
                &[GameEventType::AttackDeclared],
                |_ctx, event| {
                    let attacker_id = match event {
                        GameEvent::AttackDeclared { attacker_id, .. } => attacker_id.clone(),
                        _ => String::new(),
                    };
                    vec![effects::choose_mode(json_as(json!({
                        "options": ["ok"],
                        "step": "answered",
                        "prompt": "cw8-ask-prick: a question",
                        "data": { "attackerId": attacker_id },
                    })))]
                },
            )
            .with_when(|_ctx, event| unforced_declaration(event).is_some()),
        ],
        resume: [(
            "answered",
            hook(|ctx| {
                let attacker_id = stored_attacker_id(ctx);
                vec![effects::damage(json_as(json!({
                    "to": { "of": "instance", "instanceId": attacker_id },
                    "amount": 1,
                })))]
            }),
        )]
        .into_iter()
        .collect(),
        ..Script::default()
    }
}

/// R220, §10.3: a trap answers what a trap in the attack's window did before the combat
mod r220_s10_3_a_trap_answers_what_a_trap_in_the_attacks_window_did_before_the_combat {
    use super::*;

    #[test]
    fn r220_a_trap_answering_the_window_traps_hit_on_the_attacker_fires_before_step_5_so_the_destroyed_attacker_never_swings_s10_3_s4_2_step_4()
     {
        // p1's Twisted Sorcerer attacks p2's hero. p2's first trap answers the declaration by dealing
        // the attacker 1 damage; p2's second trap answers that damage by destroying the unit. Traps are
        // responses that fire immediately (§10.3), and both resolve inside step 4's window, before any
        // damage of the attack (§4.2 step 4), so the attacker is gone when step 5 comes.
        let mut s = setup(json!({
            "seed": "cw8-window-chain",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
            "p2": { "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
        }));
        fixture(&mut s, "cw8-prick", "Trap", prick());
        fixture(&mut s, "cw8-snap", "Trap", snap());
        set_trap(&mut s, "cw8-prick", PlayerId::P2, 1);
        set_trap(&mut s, "cw8-snap", PlayerId::P2, 2);
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");

        s.attack(&sorcerer, "hero");

        s.expect_in_zone(&sorcerer, "graveyard");
        s.expect_health("p2", 30);
    }

    #[test]
    fn r220_r113_r122_after_a_window_traps_question_a_trap_answering_its_hit_on_the_attacker_still_fires_before_step_5_s10_3()
     {
        // The same two traps, but the first asks p2 a question before it deals the attacker 1 damage, so
        // the window pauses and step 5 is owed to the answer (R113). The answer finishes the window: the
        // second trap is a response to the hit and resolves before the combat the declaration still owes.
        let mut s = setup(json!({
            "seed": "cw8-window-chain-asked",
            "p1": { "field": [{ "def": SORCERER, "lane": 1 }], "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
            "p2": { "hand": [STOCKPILE], "library": WINDOW_LIBRARY },
        }));
        fixture(&mut s, "cw8-ask-prick", "Trap", ask_prick());
        fixture(&mut s, "cw8-snap", "Trap", snap());
        set_trap(&mut s, "cw8-ask-prick", PlayerId::P2, 1);
        set_trap(&mut s, "cw8-snap", PlayerId::P2, 2);
        let sorcerer = must(s.unit("p1", 1), "p1's Twisted Sorcerer");

        s.attack(&sorcerer, "hero");
        assert_eq!(
            must(s.state().pending.as_ref(), "the window trap's question").player_id,
            PlayerId::P2
        );
        s.answer(json!("ok"));

        s.expect_in_zone(&sorcerer, "graveyard");
        s.expect_health("p2", 30);
    }
}
