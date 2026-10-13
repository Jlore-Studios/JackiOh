//! R645 (SPEC §9.9, §6): a ladder bot's persona chatter is cosmetic and cannot affect game state,
//! replays or decisions. R1344: its pools start with §6's emotes, add the MN03 emoji, and every roll
//! is limited to the dealt hand. R643's `emote_gate` judges the send instant; a blocked emote is
//! dropped, never queued.
//!
//! The port of `apps/web/src/practice/personas.test.ts`, case for case, against
//! `jackioh_server::bots::persona` (the server's port of `personas.ts`). TS's cases generated per
//! persona and trigger or reply row are one test each here, looping over the same tables.

use jackioh_engine::wire::{EMOTE_COOLDOWN_MS, EMOTE_IDS, EmoteId, GameEvent, PlayerId, PlayerView};
use jackioh_server::bots::persona::{
    AiEmote, EmotePersona, pick_bot_persona, pick_persona, reply_key_of, roll_delay, roll_for_reply,
    roll_for_trigger,
};
use jackioh_server::config::{
    AI_EMOTE, AI_PERSONAS, BOT_EMOTE_DELAY_FIT, BOT_EMOTE_REPLY_DELAY_FIT, BOT_PERSONA_WEIGHTS,
    EMOTE_REPLY_KEYS, EMOTE_TRIGGERS, EmoteDelayFit, EmoteReplyKey, EmoteTrigger, PERSONA_DEAL_STEPS,
    PERSONA_NAMES, PersonaCaps, PersonaName, PersonaRow, persona_spec,
};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------------------------
// Scripted draws
// ---------------------------------------------------------------------------------------------

type Rng = Box<dyn FnMut() -> f64 + Send>;

/// TS `low`: every draw is 0.
fn low() -> Rng {
    Box::new(|| 0.0)
}

/// TS `seq`: the draws in order, then the last one for ever (0 for none).
fn seq(draws: &[f64]) -> Rng {
    let draws = draws.to_vec();
    let mut next = 0;
    Box::new(move || {
        let draw = draws
            .get(next.min(draws.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0);
        next += 1;
        draw
    })
}

/// TS `exact`: the draws in order, and a panic for one past the script.
fn exact(draws: &[f64]) -> Rng {
    let draws = draws.to_vec();
    let mut next = 0;
    Box::new(move || {
        let draw = draws.get(next).copied();
        next += 1;
        match draw {
            Some(draw) => draw,
            None => panic!("draw #{next} was not scripted"),
        }
    })
}

// ---------------------------------------------------------------------------------------------
// Minimal watching-client views
// ---------------------------------------------------------------------------------------------

/// The bot.
const SEAT: PlayerId = PlayerId::P1;
/// The human, `opponent_of(SEAT)`: the view's own seat.
const FOE: PlayerId = PlayerId::P2;

/// TS's `{ ...base, ...over }`: `over`'s keys replace `base`'s.
fn merged(mut base: Value, over: Value) -> Value {
    if let (Some(fields), Value::Object(over)) = (base.as_object_mut(), over) {
        for (key, value) in over {
            fields.insert(key, value);
        }
    }
    base
}

fn unit(owner: PlayerId, attack: i32) -> Value {
    json!({
        "instanceId": format!("u-{owner}-{attack}"),
        "defId": "core-004",
        "radiant": false,
        "cost": 1,
        "owner": owner,
        "controller": owner,
        "attack": attack,
        "maxHealth": 2,
        "health": 2,
        "keywords": [],
        "armor": 0,
        "position": "ATK",
        "counters": {},
        "buried": 0,
        "canAct": true,
    })
}

fn hero(health: i32) -> Value {
    json!({ "health": health, "armor": 0, "powers": [], "power": null })
}

fn side(player: PlayerId, over: Value) -> Value {
    let lanes = json!([false, false, false, false, false]);
    merged(
        json!({
            "player": player,
            "hero": hero(30),
            "modifiers": [],
            "mana": { "current": 0, "max": 0 },
            "hand": { "count": 0 },
            "libraryCount": 0,
            "graveyard": [],
            "exile": [],
            "resolving": [],
            "units": [null, null, null, null, null],
            "backrow": [null, null, null, null, null],
            "locks": { "units": lanes, "backrow": lanes },
            "reserved": { "units": lanes, "backrow": lanes },
            "fatigueCount": 0,
        }),
        over,
    )
}

/// The human's view of an unremarkable main phase; `over` moves whatever the test moves.
fn view(over: Value) -> PlayerView {
    let base = json!({
        "viewer": FOE,
        "turn": 4,
        "active": FOE,
        "phase": "main",
        "you": side(FOE, json!({})),
        "opponent": side(SEAT, json!({})),
        "pending": null,
        "events": [],
        "result": null,
        "clockMs": null,
    });
    match serde_json::from_value(merged(base, over)) {
        Ok(view) => view,
        Err(error) => panic!("a fixture view must parse: {error}"),
    }
}

/// The two heroes' healths a turnStartAhead check reads: the bot's, then the human's.
fn hero_sides(ai_health: i32, human_health: i32) -> Value {
    json!({
        "you": side(FOE, json!({ "hero": hero(human_health) })),
        "opponent": side(SEAT, json!({ "hero": hero(ai_health) })),
    })
}

/// The view of the human's or the bot's turn `turn`, nothing else moved.
fn turn_view(active: PlayerId, turn: i32) -> PlayerView {
    view(json!({ "active": active, "turn": turn }))
}

/// A view whose mulligan window is open, both seats ready.
fn mulligan_view() -> PlayerView {
    view(json!({ "phase": "mulligan", "mulligan": { "youReady": true, "opponentReady": true } }))
}

/// A finished game's view.
fn over_view(winner: &str) -> PlayerView {
    view(json!({ "phase": "over", "result": { "winner": winner, "reason": "concede" } }))
}

fn event(value: Value) -> GameEvent {
    match serde_json::from_value(value) {
        Ok(event) => event,
        Err(error) => panic!("a fixture event must parse: {error}"),
    }
}

fn turn_started(player: PlayerId, turn: i32) -> GameEvent {
    event(json!({ "type": "turnStarted", "player": player, "turn": turn }))
}

fn hero_hit(player: PlayerId, amount: i32) -> GameEvent {
    event(json!({
        "type": "damage",
        "sourceId": "u-src",
        "targetId": format!("hero-{player}"),
        "amount": amount,
        "combat": true,
    }))
}

/// TS `heroHit(player)`: a hit of exactly `bigHit`.
fn big_hit(player: PlayerId) -> GameEvent {
    hero_hit(player, AI_EMOTE.big_hit)
}

fn unit_died_by(controller: PlayerId, attack: i32, killer_id: Option<&str>) -> GameEvent {
    event(json!({
        "type": "destroyed",
        "instanceId": format!("dead-{controller}-{attack}"),
        "defId": "core-004",
        "owner": controller,
        "controller": controller,
        "attack": attack,
        "maxHealth": 2,
        "killerId": killer_id,
    }))
}

/// TS `unitDied(controller, attack)`: killed by a unit.
fn unit_died(controller: PlayerId, attack: i32) -> GameEvent {
    unit_died_by(controller, attack, Some("u-killer"))
}

fn health_lost(player: PlayerId, amount: i32) -> GameEvent {
    event(json!({ "type": "healthLost", "player": player, "amount": amount }))
}

fn fatigue(player: PlayerId, amount: i32) -> GameEvent {
    event(json!({ "type": "fatigue", "player": player, "count": 1, "amount": amount }))
}

fn game_over(winner: &str) -> GameEvent {
    event(json!({ "type": "gameOver", "winner": winner, "reason": "concede" }))
}

fn emote_for_key(key: EmoteReplyKey) -> EmoteId {
    match key {
        EmoteReplyKey::Greetings => EmoteId::Greetings,
        EmoteReplyKey::Compliment => EmoteId::WellPlayed,
        EmoteReplyKey::Taunt => EmoteId::Threaten,
        EmoteReplyKey::Apology => EmoteId::Oops,
    }
}

fn session(persona: PersonaName, rng: Rng) -> EmotePersona {
    EmotePersona::new(persona, SEAT, rng, None)
}

/// `emote` after the shortest delay.
fn at_min(emote: EmoteId) -> AiEmote {
    AiEmote {
        emote,
        delay_ms: AI_EMOTE.delay_min_ms,
    }
}

/// TS `MIN_DELAY`: the greeting after the shortest delay.
fn min_delay() -> AiEmote {
    at_min(EmoteId::Greetings)
}

fn in_delay_window(delay_ms: f64) -> bool {
    (AI_EMOTE.delay_min_ms..=AI_EMOTE.delay_max_ms).contains(&delay_ms)
}

// ---------------------------------------------------------------------------------------------
// The deal
// ---------------------------------------------------------------------------------------------

#[test]
fn r645_deals_balanced_below_0_4_polite_below_0_65_bm_below_0_85_and_silent_for_the_rest() {
    let deals = [
        (0.0, PersonaName::Balanced),
        (0.3999, PersonaName::Balanced),
        (0.4, PersonaName::Polite),
        (0.6499, PersonaName::Polite),
        (0.65, PersonaName::Bm),
        (0.8499, PersonaName::Bm),
        (0.85, PersonaName::Silent),
        (0.999, PersonaName::Silent),
        (1.0, PersonaName::Silent),
    ];
    for (draw, expected) in deals {
        assert_eq!(pick_persona(&mut || draw), expected, "draw {draw}");
    }
}

#[test]
fn r645_the_weights_are_40_25_20_15_in_deal_order_and_sum_to_one() {
    let names: Vec<PersonaName> = AI_PERSONAS.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, PERSONA_NAMES, "the table is in deal order");
    let weights = PERSONA_NAMES.map(|persona| persona_spec(persona).weight);
    assert_eq!(weights, [0.4, 0.25, 0.2, 0.15]);
    let sum: f64 = weights.iter().sum();
    // TS `toBeCloseTo(1, 10)`: within half of 10^-10.
    assert!((sum - 1.0).abs() < 5e-11, "the weights sum to {sum}");
}

// ---------------------------------------------------------------------------------------------
// Trigger table
// ---------------------------------------------------------------------------------------------

/// Every persona × trigger pair, with its row when the table gives it one.
fn trigger_cells() -> Vec<(PersonaName, EmoteTrigger, Option<&'static PersonaRow>)> {
    let mut cells = Vec::new();
    for persona in PERSONA_NAMES {
        for trigger in EMOTE_TRIGGERS {
            cells.push((persona, trigger, persona_spec(persona).trigger(trigger)));
        }
    }
    cells
}

#[test]
fn r645_a_dash_in_the_trigger_table_rolls_nothing_and_never_even_draws() {
    let dashes: Vec<_> = trigger_cells()
        .into_iter()
        .filter(|(_, _, row)| row.is_none())
        .collect();
    assert!(!dashes.is_empty());
    for (persona, trigger, _) in dashes {
        assert_eq!(
            roll_for_trigger(persona, trigger, &mut exact(&[]), EMOTE_IDS),
            None,
            "{persona:?} × {trigger:?}"
        );
    }
}

#[test]
fn r645_a_trigger_hit_sends_the_rows_first_pool_entry_after_delay_min_ms() {
    for (persona, trigger, row) in trigger_cells() {
        let Some(row) = row else { continue };
        assert_eq!(
            roll_for_trigger(persona, trigger, &mut exact(&[0.0, 0.0, 0.0]), EMOTE_IDS),
            Some(at_min(row.pool[0])),
            "{persona:?} × {trigger:?}"
        );
    }
}

#[test]
fn r645_a_high_trigger_pool_draw_still_lands_in_the_pool_inside_the_delay_window() {
    for (persona, trigger, row) in trigger_cells() {
        let Some(row) = row else { continue };
        let label = format!("{persona:?} × {trigger:?}");
        let Some(roll) = roll_for_trigger(persona, trigger, &mut exact(&[0.0, 0.9999, 0.9999]), EMOTE_IDS)
        else {
            panic!("{label}: a hit must roll an emote");
        };
        assert!(row.pool.contains(&roll.emote), "{label}");
        assert_eq!(Some(&roll.emote), row.pool.last(), "{label}");
        assert!(in_delay_window(roll.delay_ms), "{label}: delay {}", roll.delay_ms);
    }
}

#[test]
fn r645_a_trigger_chance_is_a_hard_boundary_under_hits_at_or_over_misses() {
    for (persona, trigger, row) in trigger_cells() {
        let Some(row) = row else { continue };
        let label = format!("{persona:?} × {trigger:?}: the chance {}", row.chance);
        assert_eq!(
            roll_for_trigger(
                persona,
                trigger,
                &mut exact(&[row.chance - 0.0001, 0.0, 0.0]),
                EMOTE_IDS
            ),
            Some(at_min(row.pool[0])),
            "{label}"
        );
        assert_eq!(
            roll_for_trigger(persona, trigger, &mut exact(&[row.chance]), EMOTE_IDS),
            None,
            "{label}"
        );
        assert_eq!(
            roll_for_trigger(persona, trigger, &mut exact(&[1.0]), EMOTE_IDS),
            None,
            "{label}"
        );
    }
}

#[test]
fn r645_silent_sends_nothing_ever_no_trigger_no_reply_whatever_the_draw() {
    for trigger in EMOTE_TRIGGERS {
        assert_eq!(
            roll_for_trigger(PersonaName::Silent, trigger, &mut exact(&[]), EMOTE_IDS),
            None,
            "{trigger:?}"
        );
    }
    for &emote in EMOTE_IDS {
        assert_eq!(
            roll_for_reply(PersonaName::Silent, emote, &mut exact(&[]), EMOTE_IDS),
            None,
            "{emote}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Reply table
// ---------------------------------------------------------------------------------------------

#[test]
fn r645_r1344_each_emote_of_the_pool_lands_on_its_reply_row_a_yawn_a_shrug_a_thinking_face_and_a_gasp_on_none()
 {
    use EmoteReplyKey::{Apology, Compliment, Greetings, Taunt};
    let expected: [(EmoteId, Option<EmoteReplyKey>); 24] = [
        (EmoteId::Greetings, Some(Greetings)),
        (EmoteId::WellPlayed, Some(Compliment)),
        (EmoteId::Thanks, Some(Compliment)),
        (EmoteId::Threaten, Some(Taunt)),
        (EmoteId::Laugh, Some(Taunt)),
        (EmoteId::WahWah, Some(Taunt)),
        (EmoteId::Angry, Some(Taunt)),
        (EmoteId::Oops, Some(Apology)),
        (EmoteId::Sob, Some(Apology)),
        (EmoteId::Yawn, None),
        (EmoteId::Wave, Some(Greetings)),
        (EmoteId::Clap, Some(Compliment)),
        (EmoteId::ThumbsUp, Some(Compliment)),
        (EmoteId::Heart, Some(Compliment)),
        (EmoteId::Salute, Some(Compliment)),
        (EmoteId::Fire, Some(Taunt)),
        (EmoteId::Skull, Some(Taunt)),
        (EmoteId::Cool, Some(Taunt)),
        (EmoteId::Party, Some(Taunt)),
        (EmoteId::Facepalm, Some(Apology)),
        (EmoteId::Sweat, Some(Apology)),
        (EmoteId::Shrug, None),
        (EmoteId::Thinking, None),
        (EmoteId::Gasp, None),
    ];
    assert_eq!(EMOTE_IDS.len(), 24);
    for &emote in EMOTE_IDS {
        let Some((_, key)) = expected.iter().find(|(id, _)| *id == emote) else {
            panic!("{emote} has no expected reply row");
        };
        assert_eq!(reply_key_of(emote), *key, "{emote}");
    }
}

/// Every persona × reply row pair, with its row when the persona has one.
fn reply_cells() -> Vec<(PersonaName, EmoteReplyKey, Option<&'static PersonaRow>)> {
    let mut cells = Vec::new();
    for persona in PERSONA_NAMES {
        for key in EMOTE_REPLY_KEYS {
            cells.push((persona, key, persona_spec(persona).reply(key)));
        }
    }
    cells
}

#[test]
fn r645_no_reply_row_no_reply_and_no_draw_spent() {
    let missing: Vec<_> = reply_cells()
        .into_iter()
        .filter(|(_, _, row)| row.is_none())
        .collect();
    assert!(!missing.is_empty());
    for (persona, key, _) in missing {
        assert_eq!(
            roll_for_reply(persona, emote_for_key(key), &mut exact(&[]), EMOTE_IDS),
            None,
            "{persona:?} × {key:?}"
        );
    }
}

#[test]
fn r645_a_reply_hit_answers_from_the_pool_the_key_recorded_on_the_roll() {
    for (persona, key, row) in reply_cells() {
        let Some(row) = row else { continue };
        assert_eq!(
            roll_for_reply(
                persona,
                emote_for_key(key),
                &mut exact(&[0.0, 0.0, 0.0]),
                EMOTE_IDS
            ),
            Some((at_min(row.pool[0]), key)),
            "{persona:?} × {key:?}"
        );
    }
}

#[test]
fn r645_a_reply_draw_just_under_the_chance_still_answers_from_the_pools_far_end() {
    for (persona, key, row) in reply_cells() {
        let Some(row) = row else { continue };
        let label = format!("{persona:?} × {key:?}: the chance {}", row.chance);
        let rng = &mut exact(&[row.chance - 0.0001, 0.9999, 0.9999]);
        let Some((roll, rolled_key)) = roll_for_reply(persona, emote_for_key(key), rng, EMOTE_IDS) else {
            panic!("{label}: a hit must answer");
        };
        assert!(row.pool.contains(&roll.emote), "{label}");
        assert_eq!(Some(&roll.emote), row.pool.last(), "{label}");
        assert_eq!(rolled_key, key, "{label}");
        assert!(in_delay_window(roll.delay_ms), "{label}: delay {}", roll.delay_ms);
    }
}

#[test]
fn r645_a_reply_draw_at_the_chance_misses() {
    for (persona, key, row) in reply_cells() {
        let Some(row) = row else { continue };
        assert_eq!(
            roll_for_reply(persona, emote_for_key(key), &mut exact(&[row.chance]), EMOTE_IDS),
            None,
            "{persona:?} × {key:?}"
        );
    }
}

#[test]
fn r645_a_yawn_earns_no_reply_row_for_any_persona() {
    for persona in PERSONA_NAMES {
        assert_eq!(
            roll_for_reply(persona, EmoteId::Yawn, &mut exact(&[]), EMOTE_IDS),
            None,
            "{persona:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Polite pools
// ---------------------------------------------------------------------------------------------

#[test]
fn r645_r1344_polite_never_picks_a_taunt_emote_anywhere_it_may_draw() {
    let off_limits = [
        EmoteId::Threaten,
        EmoteId::Laugh,
        EmoteId::WahWah,
        EmoteId::Yawn,
        EmoteId::Angry,
        EmoteId::Fire,
        EmoteId::Skull,
        EmoteId::Cool,
        EmoteId::Party,
    ];
    let polite = persona_spec(PersonaName::Polite);
    let pools: Vec<&[EmoteId]> = polite
        .triggers
        .iter()
        .map(|(_, row)| row.pool)
        .chain(polite.replies.iter().map(|(_, row)| row.pool))
        .collect();
    assert!(!pools.is_empty());
    for pool in pools {
        for emote in off_limits {
            assert!(!pool.contains(&emote), "pool {pool:?} must not hold {emote}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Session trigger detection
// ---------------------------------------------------------------------------------------------

#[test]
fn r645_the_mulligans_end_sends_the_greeting_balanced_takes_it_every_time() {
    let mut s = session(PersonaName::Balanced, low());
    let prev = mulligan_view();
    let next = turn_view(FOE, 1);
    assert_eq!(s.on_events(&[], Some(&prev), &next, 1_000), vec![min_delay()]);
}

#[test]
fn r645_the_first_snapshot_is_not_a_mulligan_end_the_window_must_have_been_open_to_close() {
    let mut s = session(PersonaName::Balanced, low());
    let open = view(json!({ "phase": "mulligan", "mulligan": { "youReady": false, "opponentReady": true } }));
    assert_eq!(s.on_events(&[], None, &open, 1_000), vec![]);
    assert_eq!(s.on_events(&[], None, &turn_view(FOE, 1), 2_000), vec![]);
    let prev = mulligan_view();
    let next = turn_view(FOE, 1);
    assert_eq!(s.on_events(&[], Some(&prev), &next, 3_000), vec![min_delay()]);
    assert_eq!(s.on_events(&[], Some(&next), &turn_view(FOE, 2), 13_000), vec![]);
}

#[test]
fn r645_the_ais_turn_started_while_ahead_rolls_turn_start_ahead_on_health_or_on_units() {
    let mut s = session(PersonaName::Balanced, low());
    let health_ahead = view(merged(
        json!({ "active": SEAT, "turn": 5 }),
        hero_sides(30, 30 - AI_EMOTE.lead_health),
    ));
    assert_eq!(
        s.on_events(&[turn_started(SEAT, 5)], None, &health_ahead, 1_000),
        vec![at_min(EmoteId::Threaten)]
    );
    let unit_ahead = view(json!({
        "active": SEAT,
        "turn": 7,
        "opponent": side(SEAT, json!({ "units": [unit(SEAT, 2), unit(SEAT, 2), unit(SEAT, 2), null, null] })),
    }));
    assert_eq!(
        s.on_events(&[turn_started(SEAT, 7)], None, &unit_ahead, 11_000),
        vec![at_min(EmoteId::Threaten)]
    );
}

#[test]
fn r645_no_lead_no_turn_start_ahead_and_the_humans_turn_started_is_only_the_per_turn_reset() {
    let mut s = session(PersonaName::Balanced, low());
    let even = turn_view(SEAT, 5);
    assert_eq!(s.on_events(&[turn_started(SEAT, 5)], None, &even, 1_000), vec![]);
    let just_short = view(merged(
        json!({ "active": SEAT, "turn": 6 }),
        hero_sides(30, 30 - AI_EMOTE.lead_health + 1),
    ));
    assert_eq!(
        s.on_events(&[turn_started(SEAT, 6)], None, &just_short, 11_000),
        vec![]
    );
    let human_turn_ahead = view(merged(json!({ "active": FOE, "turn": 7 }), hero_sides(30, 1)));
    assert_eq!(
        s.on_events(&[turn_started(FOE, 7)], None, &human_turn_ahead, 21_000),
        vec![]
    );
}

#[test]
fn r645_a_big_hit_on_the_humans_hero_rolls_dealt_big_hit_under_big_hit_or_off_a_hero_it_rolls_nothing() {
    let v = turn_view(SEAT, 5);
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(&[big_hit(FOE)], None, &v, 1_000),
        vec![at_min(EmoteId::Laugh)]
    );
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(
            &[hero_hit(FOE, AI_EMOTE.big_hit - 1)],
            None,
            &v,
            1_000
        ),
        vec![]
    );
    let unit_hit = event(json!({
        "type": "damage",
        "sourceId": "u-a",
        "targetId": "u-b",
        "amount": AI_EMOTE.big_hit,
        "combat": true,
    }));
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(&[unit_hit], None, &v, 1_000),
        vec![]
    );
}

#[test]
fn r645_a_big_hit_on_the_ais_hero_rolls_took_big_hit_and_player_big_play_off_the_same_hit_where_the_row_exists()
 {
    let v = turn_view(FOE, 5);
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(&[big_hit(SEAT)], None, &v, 1_000),
        vec![at_min(EmoteId::Oops)]
    );
    let mut polite = session(PersonaName::Polite, seq(&[0.35, 0.0, 0.0, 0.0]));
    assert_eq!(
        polite.on_events(&[big_hit(SEAT)], None, &v, 1_000),
        vec![at_min(EmoteId::WellPlayed)]
    );
}

#[test]
fn r645_a_fat_health_lost_or_fatigue_hit_on_the_ai_counts_as_took_big_hit_too_on_the_human_it_does_not() {
    let v = turn_view(FOE, 5);
    let oops = vec![at_min(EmoteId::Oops)];
    let big = AI_EMOTE.big_hit;
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(&[health_lost(SEAT, big)], None, &v, 1_000),
        oops
    );
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(&[fatigue(SEAT, big)], None, &v, 1_000),
        oops
    );
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(
            &[health_lost(FOE, big), fatigue(FOE, big)],
            None,
            &v,
            1_000
        ),
        vec![]
    );
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(
            &[health_lost(SEAT, big - 1), fatigue(SEAT, big - 1)],
            None,
            &v,
            1_000
        ),
        vec![]
    );
}

#[test]
fn r645_the_dead_units_attack_must_reach_the_previous_views_top_for_killed_top_unit_and_lost_top_unit() {
    let prev = view(json!({
        "you": side(FOE, json!({ "units": [unit(FOE, 8), unit(FOE, 2), null, null, null] })),
        "opponent": side(SEAT, json!({ "units": [unit(SEAT, 6), null, null, null, null] })),
    }));
    let v = turn_view(SEAT, 5);
    let balanced = || session(PersonaName::Balanced, low());
    assert_eq!(
        balanced().on_events(&[unit_died(FOE, 8)], Some(&prev), &v, 1_000),
        vec![at_min(EmoteId::Laugh)]
    );
    assert_eq!(
        balanced().on_events(&[unit_died(FOE, 7)], Some(&prev), &v, 1_000),
        vec![]
    );
    assert_eq!(
        balanced().on_events(&[unit_died(SEAT, 6)], Some(&prev), &v, 1_000),
        vec![at_min(EmoteId::Sob)]
    );
    assert_eq!(
        balanced().on_events(&[unit_died(SEAT, 5)], Some(&prev), &v, 1_000),
        vec![]
    );
    assert_eq!(
        balanced().on_events(&[unit_died(FOE, 1)], None, &v, 1_000),
        vec![at_min(EmoteId::Laugh)]
    );
    assert_eq!(
        balanced().on_events(&[unit_died_by(FOE, 8, None)], Some(&prev), &v, 1_000),
        vec![]
    );
}

#[test]
fn r645_two_of_the_ais_units_dying_on_the_humans_turn_roll_player_big_play_once_at_the_second() {
    let mut s = session(PersonaName::Polite, low());
    let human_turn = turn_view(FOE, 7);
    let prev =
        view(json!({ "opponent": side(SEAT, json!({ "units": [unit(SEAT, 9), null, null, null, null] })) }));
    let well_played = vec![at_min(EmoteId::WellPlayed)];
    let two_dead = [unit_died(SEAT, 4), unit_died(SEAT, 4)];

    assert_eq!(
        s.on_events(&[unit_died(SEAT, 4)], Some(&prev), &human_turn, 1_000),
        vec![]
    );
    assert_eq!(
        s.on_events(&[unit_died(SEAT, 4)], Some(&prev), &human_turn, 11_000),
        well_played
    );
    assert_eq!(s.on_events(&two_dead, Some(&prev), &human_turn, 21_000), vec![]);
    assert_eq!(
        s.on_events(&two_dead, Some(&prev), &turn_view(SEAT, 8), 31_000),
        vec![]
    );
    let next_turn = turn_view(FOE, 9);
    assert_eq!(
        s.on_events(&[turn_started(FOE, 9)], Some(&prev), &next_turn, 41_000),
        vec![]
    );
    assert_eq!(
        s.on_events(&two_dead, Some(&prev), &next_turn, 51_000),
        well_played
    );
    let other_turn = turn_view(FOE, 10);
    assert_eq!(
        s.on_events(&[turn_started(FOE, 10)], Some(&prev), &other_turn, 61_000),
        vec![]
    );
    assert_eq!(
        s.on_events(
            &[unit_died(FOE, 4), unit_died(FOE, 4)],
            Some(&prev),
            &other_turn,
            71_000
        ),
        vec![]
    );
}

#[test]
fn r645_the_second_kill_roll_is_spent_whether_it_hits_or_misses() {
    let mut s = session(PersonaName::Polite, seq(&[0.9]));
    let human_turn = turn_view(FOE, 7);
    let prev =
        view(json!({ "opponent": side(SEAT, json!({ "units": [unit(SEAT, 9), null, null, null, null] })) }));
    let two_dead = [unit_died(SEAT, 4), unit_died(SEAT, 4)];
    assert_eq!(s.on_events(&two_dead, Some(&prev), &human_turn, 1_000), vec![]);
    assert_eq!(s.on_events(&two_dead, Some(&prev), &human_turn, 11_000), vec![]);
}

#[test]
fn r645_game_over_sends_match_won_when_the_seat_won_match_lost_when_the_human_did_nothing_on_a_draw() {
    let won = SEAT.as_str();
    let lost = FOE.as_str();
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(&[game_over(won)], None, &over_view(won), 1_000),
        vec![at_min(EmoteId::WahWah)]
    );
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(&[game_over(lost)], None, &over_view(lost), 1_000),
        vec![at_min(EmoteId::WellPlayed)]
    );
    assert_eq!(
        session(PersonaName::Balanced, low()).on_events(
            &[game_over("draw")],
            None,
            &over_view("draw"),
            1_000
        ),
        vec![]
    );
}

#[test]
fn r645_silents_session_is_quiet_end_to_end_greeting_kills_lethal_replies_all_land_nothing() {
    let mut s = session(PersonaName::Silent, low());
    assert_eq!(
        s.on_events(&[], Some(&mulligan_view()), &turn_view(FOE, 1), 1_000),
        vec![]
    );
    assert_eq!(
        s.on_events(
            &[
                big_hit(FOE),
                big_hit(SEAT),
                unit_died(FOE, 9),
                unit_died(FOE, 9),
                unit_died(SEAT, 9),
            ],
            None,
            &turn_view(FOE, 2),
            2_000
        ),
        vec![]
    );
    assert_eq!(s.on_player_emote(EmoteId::Greetings, 3_000), vec![]);
    assert_eq!(s.on_player_turn_long(&turn_view(FOE, 2), 4_000), vec![]);
    assert_eq!(
        s.on_events(
            &[game_over(SEAT.as_str())],
            None,
            &over_view(SEAT.as_str()),
            5_000
        ),
        vec![]
    );
}

#[test]
fn r645_a_players_long_turn_rolls_player_turn_long_once_per_turn_never_on_the_ais_own() {
    let mut s = session(PersonaName::Balanced, low());
    let yawn = vec![at_min(EmoteId::Yawn)];
    assert_eq!(s.on_player_turn_long(&turn_view(FOE, 7), 1_000), yawn);
    assert_eq!(s.on_player_turn_long(&turn_view(FOE, 7), 2_000), vec![]);
    assert_eq!(s.on_player_turn_long(&turn_view(SEAT, 8), 3_000), vec![]);
    s.on_events(&[turn_started(FOE, 9)], None, &turn_view(FOE, 9), 4_000);
    assert_eq!(s.on_player_turn_long(&turn_view(FOE, 9), 5_000), yawn);
}

// ---------------------------------------------------------------------------------------------
// Session replies
// ---------------------------------------------------------------------------------------------

#[test]
fn r645_a_players_emote_can_earn_a_reply_each_reply_rule_at_most_once_a_match() {
    let mut s = session(PersonaName::Balanced, low());
    s.on_events(&[turn_started(FOE, 3)], None, &turn_view(FOE, 3), 1_000);
    assert_eq!(s.on_player_emote(EmoteId::Greetings, 11_000), vec![min_delay()]);
    assert_eq!(s.on_player_emote(EmoteId::Greetings, 21_000), vec![]);
    s.on_events(&[turn_started(FOE, 4)], None, &turn_view(FOE, 4), 31_000);
    assert_eq!(
        s.on_player_emote(EmoteId::WellPlayed, 41_000),
        vec![at_min(EmoteId::Thanks)]
    );
    s.on_events(&[turn_started(FOE, 5)], None, &turn_view(FOE, 5), 51_000);
    assert_eq!(s.on_player_emote(EmoteId::Thanks, 61_000), vec![]);
    s.on_events(&[turn_started(FOE, 6)], None, &turn_view(FOE, 6), 71_000);
    assert_eq!(
        s.on_player_emote(EmoteId::Threaten, 81_000),
        vec![at_min(EmoteId::Laugh)]
    );
    s.on_events(&[turn_started(FOE, 7)], None, &turn_view(FOE, 7), 91_000);
    assert_eq!(s.on_player_emote(EmoteId::Oops, 101_000), vec![]);
    assert_eq!(s.on_player_emote(EmoteId::Yawn, 102_000), vec![]);
}

#[test]
fn r645_a_missed_roll_does_not_spend_the_rule_the_same_emote_can_still_earn_the_reply_later() {
    let mut s = session(PersonaName::Balanced, seq(&[0.7, 0.0, 0.0, 0.0]));
    s.on_events(&[turn_started(FOE, 3)], None, &turn_view(FOE, 3), 1_000);
    assert_eq!(s.on_player_emote(EmoteId::Greetings, 11_000), vec![]);
    assert_eq!(s.on_player_emote(EmoteId::Greetings, 21_000), vec![min_delay()]);
    assert_eq!(s.on_player_emote(EmoteId::Greetings, 31_000), vec![]);
}

// ---------------------------------------------------------------------------------------------
// Session caps and rate limit
// ---------------------------------------------------------------------------------------------

#[test]
fn r645_the_caps_live_in_config_balanced_1_1_8_polite_1_1_6_bm_2_2_20_silent_0_0_0() {
    let caps = |own_turn, other_turn, per_match| PersonaCaps {
        own_turn,
        other_turn,
        per_match,
    };
    assert_eq!(persona_spec(PersonaName::Balanced).caps, caps(1, 1, 8));
    assert_eq!(persona_spec(PersonaName::Polite).caps, caps(1, 1, 6));
    assert_eq!(persona_spec(PersonaName::Bm).caps, caps(2, 2, 20));
    assert_eq!(persona_spec(PersonaName::Silent).caps, caps(0, 0, 0));
}

#[test]
fn r645_balanceds_turn_caps_are_one_emote_on_each_sides_turn_refilled_as_the_turns_move() {
    let mut s = session(PersonaName::Balanced, low());
    let ai_turn = turn_view(SEAT, 5);
    assert_eq!(
        s.on_events(&[turn_started(SEAT, 5), big_hit(FOE)], None, &ai_turn, 1_000),
        vec![at_min(EmoteId::Laugh)]
    );
    assert_eq!(
        s.on_events(&[big_hit(FOE)], Some(&ai_turn), &ai_turn, 11_000),
        vec![]
    );
    let ai_turn6 = turn_view(SEAT, 6);
    assert_eq!(
        s.on_events(
            &[turn_started(SEAT, 6), big_hit(FOE)],
            Some(&ai_turn),
            &ai_turn6,
            21_000
        ),
        vec![at_min(EmoteId::Laugh)]
    );
    let human_turn7 = turn_view(FOE, 7);
    assert_eq!(
        s.on_events(
            &[turn_started(FOE, 7), big_hit(SEAT)],
            Some(&ai_turn6),
            &human_turn7,
            31_000
        ),
        vec![at_min(EmoteId::Oops)]
    );
    assert_eq!(
        s.on_events(&[big_hit(SEAT)], Some(&human_turn7), &human_turn7, 41_000),
        vec![]
    );
}

#[test]
fn r645_bm_gets_two_a_turn_the_third_trigger_of_the_same_turn_is_the_caps() {
    let mut s = session(PersonaName::Bm, low());
    let ai_turn = turn_view(SEAT, 5);
    assert_eq!(
        s.on_events(&[turn_started(SEAT, 5), big_hit(FOE)], None, &ai_turn, 1_000),
        vec![at_min(EmoteId::Laugh)]
    );
    assert_eq!(
        s.on_events(&[big_hit(FOE)], Some(&ai_turn), &ai_turn, 11_000),
        vec![at_min(EmoteId::Laugh)]
    );
    assert_eq!(
        s.on_events(&[big_hit(FOE)], Some(&ai_turn), &ai_turn, 21_000),
        vec![]
    );
}

/// The eight turns, alternating from the bot's, that spend balanced's match cap.
const CAP_SEATS: [PlayerId; 8] = [SEAT, FOE, SEAT, FOE, SEAT, FOE, SEAT, FOE];

#[test]
fn r645_the_match_cap_counts_only_the_non_exempt_emotes_the_greeting_and_the_end_emotes_land_past_it() {
    let mut s = session(PersonaName::Balanced, low());
    let mut now = 0;
    let mut step = || {
        now += 10_000;
        now
    };
    let mulligan = mulligan_view();

    assert_eq!(
        s.on_events(&[], Some(&mulligan), &turn_view(FOE, 1), step()),
        vec![min_delay()]
    );
    for (turn, player) in (2..).zip(CAP_SEATS) {
        let v = turn_view(player, turn);
        assert_eq!(
            s.on_events(&[turn_started(player, turn), big_hit(FOE)], None, &v, step()),
            vec![at_min(EmoteId::Laugh)],
            "turn {turn}"
        );
    }
    let turn10 = turn_view(SEAT, 10);
    assert_eq!(
        s.on_events(&[turn_started(SEAT, 10), big_hit(FOE)], None, &turn10, step()),
        vec![]
    );
    assert_eq!(
        s.on_events(&[], Some(&mulligan), &turn_view(SEAT, 10), step()),
        vec![min_delay()]
    );
    assert_eq!(
        s.on_events(
            &[game_over(SEAT.as_str())],
            Some(&turn10),
            &over_view(SEAT.as_str()),
            step()
        ),
        vec![at_min(EmoteId::WahWah)]
    );
}

#[test]
fn r645_the_end_of_match_emotes_are_exempt_at_the_flag_not_only_via_the_over_phase() {
    let mut s = session(PersonaName::Balanced, low());
    let mut now = 0;
    let mut step = || {
        now += 10_000;
        now
    };
    for (turn, player) in (2..).zip(CAP_SEATS) {
        let v = turn_view(player, turn);
        assert_eq!(
            s.on_events(&[turn_started(player, turn), big_hit(FOE)], None, &v, step())
                .len(),
            1,
            "turn {turn}"
        );
    }
    let turn11 = turn_view(FOE, 11);
    assert_eq!(
        s.on_events(&[turn_started(FOE, 11)], None, &turn11, step()),
        vec![]
    );
    assert_eq!(
        s.on_events(&[game_over(FOE.as_str())], Some(&turn11), &turn11, step()),
        vec![at_min(EmoteId::WellPlayed)]
    );
}

#[test]
fn r645_the_shared_rate_limit_judges_the_send_instant_a_fast_follow_up_is_dropped_a_delayed_one_lands() {
    let ai_turn = turn_view(SEAT, 5);
    let mut fast = session(PersonaName::Bm, low());
    assert_eq!(
        fast.on_events(&[big_hit(FOE), big_hit(FOE)], None, &ai_turn, 1_000),
        vec![at_min(EmoteId::Laugh)]
    );
    assert_eq!(
        fast.on_events(&[big_hit(FOE)], Some(&ai_turn), &ai_turn, 31_000),
        vec![at_min(EmoteId::Laugh)]
    );
    let mut spaced = session(PersonaName::Bm, seq(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.99]));
    let out = spaced.on_events(&[big_hit(FOE), big_hit(FOE)], None, &ai_turn, 1_000);
    assert_eq!(out.len(), 2);
    let Some(second) = out.get(1) else {
        panic!("two emotes land");
    };
    assert_eq!(second.emote, EmoteId::Laugh);
    assert!(
        second.delay_ms >= AI_EMOTE.delay_min_ms + EMOTE_COOLDOWN_MS as f64,
        "delay {}",
        second.delay_ms
    );
}

// ---------------------------------------------------------------------------------------------
// R1344: dealt-hand choices
// ---------------------------------------------------------------------------------------------

/// One persona's §6 rows: each row's name as the wire spells it, and its pool.
type Section6Rows = Vec<(&'static str, Vec<EmoteId>)>;

/// §6's own pools, before R1344's emoji, by persona and row (TS `SECTION_6_POOLS`).
fn section_6_pools() -> Vec<(PersonaName, Section6Rows)> {
    use EmoteId::*;
    vec![
        (
            PersonaName::Balanced,
            vec![
                ("mulliganEnd", vec![Greetings]),
                ("turnStartAhead", vec![Threaten, Laugh, Yawn]),
                ("dealtBigHit", vec![Laugh, Threaten, WahWah]),
                ("killedTopUnit", vec![Laugh, WahWah]),
                ("tookBigHit", vec![Oops, Sob, Angry]),
                ("lostTopUnit", vec![Sob, Angry, Oops]),
                ("playerTurnLong", vec![Yawn]),
                ("matchWon", vec![WahWah, Laugh, WellPlayed]),
                ("matchLost", vec![WellPlayed]),
                ("greetings", vec![Greetings]),
                ("compliment", vec![Thanks]),
                ("taunt", vec![Laugh, WahWah, Yawn, Threaten]),
            ],
        ),
        (
            PersonaName::Polite,
            vec![
                ("mulliganEnd", vec![Greetings]),
                ("dealtBigHit", vec![Oops]),
                ("tookBigHit", vec![WellPlayed, Oops]),
                ("lostTopUnit", vec![WellPlayed]),
                ("playerBigPlay", vec![WellPlayed]),
                ("matchWon", vec![WellPlayed]),
                ("matchLost", vec![WellPlayed]),
                ("greetings", vec![Greetings]),
                ("compliment", vec![Thanks]),
                ("taunt", vec![Oops, Greetings]),
                ("apology", vec![Thanks]),
            ],
        ),
        (
            PersonaName::Bm,
            vec![
                ("mulliganEnd", vec![Threaten, Laugh]),
                ("turnStartAhead", vec![Threaten, Laugh, Yawn, WahWah]),
                ("dealtBigHit", vec![Laugh, WahWah, Threaten]),
                ("killedTopUnit", vec![Laugh, WahWah, Yawn]),
                ("tookBigHit", vec![Angry, Threaten]),
                ("lostTopUnit", vec![Angry]),
                ("playerTurnLong", vec![Yawn]),
                ("matchWon", vec![WahWah, Laugh]),
                ("matchLost", vec![Sob, Angry]),
                ("greetings", vec![Threaten, Laugh]),
                ("compliment", vec![Yawn]),
                ("taunt", vec![Laugh, WahWah, Yawn, Threaten]),
                ("apology", vec![Laugh, WahWah]),
            ],
        ),
    ]
}

/// A row key as the wire spells it (`mulliganEnd`, `greetings`).
fn wire_name(key: impl serde::Serialize) -> String {
    match serde_json::to_value(key) {
        Ok(Value::String(name)) => name,
        other => panic!("a row key serialises as a string, not {other:?}"),
    }
}

#[test]
fn r1344_every_pool_is_section_6s_own_emotes_first_verbatim_then_emoji_of_the_pool_alone_none_twice() {
    for (persona, rows) in section_6_pools() {
        let spec = persona_spec(persona);
        let widened: Vec<(String, &[EmoteId])> = spec
            .triggers
            .iter()
            .map(|(key, row)| (wire_name(key), row.pool))
            .chain(spec.replies.iter().map(|(key, row)| (wire_name(key), row.pool)))
            .collect();
        let mut widened_keys: Vec<&str> = widened.iter().map(|(key, _)| key.as_str()).collect();
        let mut original_keys: Vec<&str> = rows.iter().map(|(key, _)| *key).collect();
        widened_keys.sort_unstable();
        original_keys.sort_unstable();
        assert_eq!(widened_keys, original_keys, "{persona:?}");
        for (key, original) in &rows {
            let pool: &[EmoteId] = widened
                .iter()
                .find(|(name, _)| name == key)
                .map_or(&[], |(_, pool)| pool);
            assert_eq!(
                pool.get(..original.len()),
                Some(original.as_slice()),
                "{persona:?}.{key}"
            );
            for (at, emote) in pool.iter().enumerate() {
                assert!(!pool[..at].contains(emote), "{persona:?}.{key} repeats an emote");
                assert!(EMOTE_IDS.contains(emote), "{persona:?}.{key}");
            }
        }
    }
}

#[test]
fn r1344_a_trigger_roll_picks_only_from_what_the_hand_holds_in_the_pools_order() {
    use EmoteId::*;
    let no_greeting = [Oops, Thanks, Threaten, Sob, Wave, Heart, Salute, Party];
    assert_eq!(
        roll_for_trigger(
            PersonaName::Balanced,
            EmoteTrigger::MulliganEnd,
            &mut low(),
            &no_greeting
        ),
        Some(at_min(Wave))
    );
    assert_eq!(
        roll_for_trigger(
            PersonaName::Balanced,
            EmoteTrigger::MulliganEnd,
            &mut seq(&[0.0, 0.999, 0.0]),
            &no_greeting
        )
        .map(|roll| roll.emote),
        Some(Salute)
    );
}

#[test]
fn r1344_a_hit_on_a_row_the_hand_holds_none_of_sends_nothing() {
    use EmoteId::*;
    let quiet = [WellPlayed, Oops, Thanks, Sob, Yawn, Laugh, Angry, WahWah];
    assert_eq!(
        roll_for_trigger(
            PersonaName::Balanced,
            EmoteTrigger::MulliganEnd,
            &mut low(),
            &quiet
        ),
        None
    );
    assert_eq!(
        roll_for_reply(PersonaName::Polite, Greetings, &mut low(), &quiet),
        None
    );
    let with_wave: Vec<EmoteId> = std::iter::once(Wave).chain(quiet[1..].iter().copied()).collect();
    let answered = roll_for_reply(PersonaName::Polite, Greetings, &mut low(), &with_wave);
    assert_eq!(
        answered.map(|(roll, key)| (roll.emote, key)),
        Some((Wave, EmoteReplyKey::Greetings))
    );
}

#[test]
fn r1344_a_session_made_with_a_hand_sends_from_that_hand_alone_greeting_and_replies_included() {
    use EmoteId::*;
    let hand = vec![WellPlayed, Oops, Thanks, Sob, Clap, Skull, Salute, Gasp];
    let mut s = EmotePersona::new(PersonaName::Balanced, SEAT, low(), Some(hand.clone()));
    assert_eq!(
        s.on_events(&[], Some(&mulligan_view()), &turn_view(FOE, 1), 1_000),
        vec![at_min(Salute)]
    );
    let mut replies = EmotePersona::new(PersonaName::Balanced, SEAT, low(), Some(hand));
    let sent: Vec<EmoteId> = replies
        .on_player_emote(WellPlayed, 60_000)
        .into_iter()
        .map(|intent| intent.emote)
        .collect();
    assert_eq!(sent, vec![Thanks]);
    assert_eq!(replies.on_player_emote(Laugh, 90_000), vec![]);
}

#[test]
fn r1344_over_dealt_hands_nothing_any_persona_rolls_is_outside_its_hand() {
    use EmoteId::*;
    let hands = [
        [Greetings, Thanks, Threaten, Laugh, WahWah, Wave, ThumbsUp, Party],
        [WellPlayed, Oops, Thanks, Sob, Clap, Skull, Cool, Gasp],
        [Greetings, Oops, Threaten, Yawn, Facepalm, Shrug, Fire, Sweat],
        [
            WellPlayed, Thanks, Threaten, Angry, Thinking, Heart, Salute, Party,
        ],
    ];
    let draws = [0.0, 0.13, 0.27, 0.41, 0.58, 0.72, 0.86, 0.99];
    for hand in &hands {
        for persona in PERSONA_NAMES {
            for trigger in EMOTE_TRIGGERS {
                for draw in draws {
                    if let Some(roll) = roll_for_trigger(persona, trigger, &mut seq(&[0.0, draw, 0.0]), hand)
                    {
                        assert!(hand.contains(&roll.emote), "{persona:?} {trigger:?}");
                    }
                }
            }
            for &emote in EMOTE_IDS {
                for draw in draws {
                    if let Some((roll, _)) = roll_for_reply(persona, emote, &mut seq(&[0.0, draw, 0.0]), hand)
                    {
                        assert!(hand.contains(&roll.emote), "{persona:?} reply to {emote}");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Not a TS case: the server holds a persona in a match actor's tokio task, so it must be `Send`.
// ---------------------------------------------------------------------------------------------

#[test]
fn r645_a_persona_can_live_in_a_match_actors_task() {
    fn assert_send<T: Send>(_: &T) {}
    assert_send(&session(PersonaName::Balanced, low()));
}

// ---------------------------------------------------------------------------------------------
// R639000: a ladder bot's refit rows
// ---------------------------------------------------------------------------------------------

/// Two refit windows, apart from the practice AI's and from each other.
const TRIGGER_WINDOW: EmoteDelayFit = EmoteDelayFit {
    samples: 40,
    min_ms: 1_200.0,
    max_ms: 4_000.0,
};
const REPLY_WINDOW: EmoteDelayFit = EmoteDelayFit {
    samples: 35,
    min_ms: 2_000.0,
    max_ms: 6_000.0,
};

fn ladder_session(persona: PersonaName, rng: Rng) -> EmotePersona {
    session(persona, rng).with_delays(TRIGGER_WINDOW, REPLY_WINDOW)
}

#[test]
fn r639000_the_refit_windows_feed_the_rolls_and_the_caps_and_rate_limit_hold() {
    // Each delay is the window's floor plus the draw's share of its span, for each window.
    for window in [
        TRIGGER_WINDOW,
        REPLY_WINDOW,
        BOT_EMOTE_DELAY_FIT,
        BOT_EMOTE_REPLY_DELAY_FIT,
    ] {
        for draw in [0.0, 0.25, 0.5, 0.999] {
            assert_eq!(
                roll_delay(&mut || draw, &window),
                window.min_ms + draw * (window.max_ms - window.min_ms)
            );
        }
    }
    // Until a fit lands, the ladder's rows are the practice AI's.
    for window in [BOT_EMOTE_DELAY_FIT, BOT_EMOTE_REPLY_DELAY_FIT] {
        if window.samples == 0 {
            assert_eq!(
                (window.min_ms, window.max_ms),
                (AI_EMOTE.delay_min_ms, AI_EMOTE.delay_max_ms)
            );
        }
    }
    if BOT_PERSONA_WEIGHTS.samples == 0 {
        assert_eq!(
            BOT_PERSONA_WEIGHTS.weights,
            PERSONA_NAMES.map(|persona| persona_spec(persona).weight)
        );
    }
    // A new bot is dealt by the ladder's weights, on the deal's whole thousandths.
    let mut steps = 0.0;
    for (name, weight) in PERSONA_NAMES.iter().zip(BOT_PERSONA_WEIGHTS.weights) {
        if weight > 0.0 {
            assert_eq!(
                pick_bot_persona(&mut || steps / PERSONA_DEAL_STEPS),
                *name,
                "{name:?}"
            );
        }
        steps += (weight * PERSONA_DEAL_STEPS).round();
    }

    // A triggered emote waits inside the trigger window: the chance, the pool, then the delay.
    let ai_turn = turn_view(SEAT, 5);
    let mut triggered = ladder_session(PersonaName::Bm, seq(&[0.0, 0.0, 0.25]));
    assert_eq!(
        triggered.on_events(&[big_hit(FOE)], None, &ai_turn, 1_000),
        vec![AiEmote {
            emote: EmoteId::Laugh,
            delay_ms: 1_200.0 + 0.25 * 2_800.0,
        }]
    );
    // A reply waits inside the reply window.
    let mut replying = ladder_session(PersonaName::Balanced, seq(&[0.0, 0.0, 0.75]));
    replying.on_events(&[turn_started(FOE, 3)], None, &turn_view(FOE, 3), 1_000);
    assert_eq!(
        replying.on_player_emote(EmoteId::Greetings, 11_000),
        vec![AiEmote {
            emote: EmoteId::Greetings,
            delay_ms: 2_000.0 + 0.75 * 4_000.0,
        }]
    );

    // The caps hold: Balanced's one emote on its own turn, and nothing more that turn.
    let mut capped = ladder_session(PersonaName::Balanced, low());
    assert_eq!(
        capped.on_events(&[turn_started(SEAT, 5), big_hit(FOE)], None, &ai_turn, 1_000),
        vec![AiEmote {
            emote: EmoteId::Laugh,
            delay_ms: TRIGGER_WINDOW.min_ms,
        }]
    );
    assert_eq!(
        capped.on_events(&[big_hit(FOE)], Some(&ai_turn), &ai_turn, 11_000),
        vec![]
    );
    // And R643's rate limit: two of BM's at one instant, the second dropped.
    let mut fast = ladder_session(PersonaName::Bm, low());
    assert_eq!(
        fast.on_events(&[big_hit(FOE), big_hit(FOE)], None, &ai_turn, 1_000),
        vec![AiEmote {
            emote: EmoteId::Laugh,
            delay_ms: TRIGGER_WINDOW.min_ms,
        }]
    );
}
