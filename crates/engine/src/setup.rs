//! Shuffle, the opening draw table, Quickdraw, the mulligan, The Coin and start-of-game effects
//! (SPEC §2.1, R9, R43, R244).
//!
//! Cast on draw cards sit out the deal and are shuffled in after the mulligans (R635, R748). The mulligan
//! is concurrent and an answer sealed until both are in (R265, R266), so the game is the same whichever
//! seat answered first. `work.rs`'s dispatcher calls `run_owed_setup` (SURFACE §6.6).

use indexmap::IndexSet;
use serde::Serialize;
use serde_json::{Value, json};

use crate::catalog::find_def;
use crate::config::{COIN_DEF_ID, MULLIGAN_ORDER, OPENING_COINS, OPENING_DRAW};
use crate::draw::{add_to_hand, cast_dealt_card, casts_on_draw, draw};
use crate::own_library::show_to_owner;
use crate::prompts::run_start_of_game;
use crate::script::EngineSink;
use crate::scripts::flags_of;
use crate::state::{
    CardInstance, EngineError, GameState, MulliganSeat, PendingChoice, PromptOption, Resume, WorkItem,
    find_instance, find_instance_mut, handicap_of, new_instance,
};
use crate::state_check::state_check;
use crate::turn::{clear_return_flags, start_turn};
use crate::wire::{GameEvent, PLAYER_IDS, PerPlayer, Phase, PlayerId, PromptKind, Selection, Zone};
use crate::work::{owe, paused};
use crate::zones::{LibraryPosition, MoveToZoneOptions, OffFieldZone, move_to_zone};

fn seat_of(player: PlayerId) -> usize {
    player.seat()
}

/// §2.1, R2368000: the seat's entry in the opening-draw table.
pub fn opening_draw_for(player: PlayerId) -> i32 {
    OPENING_DRAW
        .get(seat_of(player))
        .or(OPENING_DRAW.last())
        .copied()
        .unwrap_or_default()
}

/// §2.1, R182: the seat's opening-draw table entry plus its handicap's extra cards. Quickdraw cards
/// replace draws out of this total, so it is the size of the hand the mulligan sees.
pub fn opening_hand_size(state: &GameState, player: PlayerId) -> i32 {
    opening_draw_for(player) + handicap_of(&state.players[player]).extra_opening_cards
}

fn mulligan_prompt(sink: &EngineSink, player: PlayerId) -> PendingChoice {
    let hand = &sink.state.players[player].hand;
    let mut data = indexmap::IndexMap::new();
    data.insert("player".to_string(), json!(player));
    PendingChoice {
        id: format!("q{}", sink.state.next_id),
        player_id: player,
        kind: PromptKind::Mulligan,
        prompt: "Choose the cards to keep; the rest are returned and redrawn".to_string(),
        options: hand
            .iter()
            .map(|card| PromptOption {
                key: card.id.clone(),
                label: card.def_id.clone(),
                selection: Selection::Instance {
                    instance_id: card.id.clone(),
                },
                cost: None,
                radiant: None,
            })
            .collect(),
        min: 0,
        max: hand.len() as i32,
        budget: None,
        craft_mode: None,
        craft_offer: None,
        streak: None,
        resume: Resume {
            def_id: String::new(),
            hook: "mulligan".to_string(),
            step: "mulligan".to_string(),
            radiant: false,
            instance_id: None,
            data,
        },
    }
}

/// §2.1 step 3, R265: both seats' prompts, opened together in seat order. Not `state.pending`, which
/// stays the one prompt §10.1 allows, so they open only once nothing is waiting.
fn open_mulligans(sink: &mut EngineSink) {
    if sink.state.pending.is_some() || sink.state.mulligan.is_some() {
        return;
    }
    let mut open: Vec<MulliganSeat> = Vec::with_capacity(PLAYER_IDS.len());
    for player in PLAYER_IDS {
        let prompt = mulligan_prompt(sink, player);
        sink.state.next_id += 1;
        sink.events.push(GameEvent::PromptOpened {
            player,
            choice_id: Some(prompt.id.clone()),
            kind: Some(PromptKind::Mulligan),
        });
        open.push(MulliganSeat { prompt, keep: None });
    }
    let p2 = open.pop().expect("a mulligan seat for p2");
    let p1 = open.pop().expect("a mulligan seat for p1");
    sink.state.mulligan = Some(PerPlayer::new(p1, p2));
    sink.state.phase = Phase::Mulligan;
}

/// R265: the seats whose mulligan is open and unanswered, in seat order.
pub fn mulligan_owed(state: &GameState) -> Vec<PlayerId> {
    let Some(open) = state.mulligan.as_ref() else {
        return Vec::new();
    };
    PLAYER_IDS
        .into_iter()
        .filter(|&player| open[player].keep.is_none())
        .collect()
}

/// R265: this seat's own mulligan prompt while it is open and the seat has not answered it.
pub fn mulligan_prompt_for(state: &GameState, player: PlayerId) -> Option<&PendingChoice> {
    let seat = &state.mulligan.as_ref()?[player];
    if seat.keep.is_some() {
        return None;
    }
    Some(&seat.prompt)
}

/// R265, R266: why this seat's mulligan answer is refused (SURFACE §4.4.9): the reducer asks before it
/// answers, and `legal_actions` offers the mulligan exactly when there is no reason.
pub fn why_mulligan_refused(state: &GameState, player: PlayerId, keep: &[String]) -> Result<(), EngineError> {
    if state.pending.is_some() || state.mulligan.is_none() {
        return Err(EngineError::new("no mulligan is open"));
    }
    let Some(prompt) = mulligan_prompt_for(state, player) else {
        return Err(EngineError::new("you have already answered your mulligan"));
    };
    let offered: IndexSet<&str> = prompt.options.iter().map(|option| option.key.as_str()).collect();
    for id in keep {
        if !offered.contains(id.as_str()) {
            return Err(EngineError::new(format!("{id} is not in your hand")));
        }
    }
    Ok(())
}

/// R113: the `resume.hook` of what setup still owes when a clause asks during it: only a start-of-game
/// clause on arrival (R151) can, as the opening draw and R9's replacement draws arrive (R224), since a
/// dealt card is never cast while it deals (R635, R748). §10.1 allows one prompt at a time, so setup owes
/// the rest of itself and the answer's drain brings it back (R122).
pub const SETUP_WORK: &str = "@setup";

/// Which part of setup is owed, each then the seats after it: the deal from a seat on, a seat's
/// Quickdraw cards, or the end of a seat's mulligan (R265).
const DEAL_STEP: &str = "deal";
const QUICKDRAW_STEP: &str = "quickdraw";
const MULLIGAN_STEP: &str = "mulligan";
const START_OF_GAME_STEP: &str = "startOfGame";
const SUSPENDED_STEP: &str = "suspended";

/// R748: the mark on a cast-on-draw card setup dealt uncast, on the card's memory, which no view
/// carries (§9.1), so it says nothing the `drawn` it was dealt with did not (R225).
const SUSPENDED_CAST_KEY: &str = "@suspendedCast";

/// §2.1: deal both opening hands, then open both mulligans (R265).
pub fn begin_setup(sink: &mut EngineSink) {
    deal_from(sink, 0);
}

fn is_quickdraw(state: &GameState, card: &CardInstance) -> bool {
    flags_of(state, card).quickdraw == Some(true)
}

/// R635: how many cards setup may still draw, the set-aside cast-on-draw cards left out, so it never
/// casts one by drawing it nor draws from an empty library (R3's fatigue).
fn drawable_count(state: &GameState, player: PlayerId) -> i32 {
    state.players[player]
        .library
        .iter()
        .filter(|card| !casts_on_draw(state, card))
        .count() as i32
}

/// §2.1 steps 1 and 2 for each seat from `seat` on, then both mulligans (R265).
///
/// R225, R640: each Quickdraw card replaces one of the last opening draws (§2.1, §6.2), at most as many
/// as the hand holds; the others stay in the library as ordinary cards. Each goes to the hand as the draw
/// it replaces (R55's counter), so nothing the opponent sees says whether the hand holds one: #100's
/// price, the deal's events, the counts while a clause is asking (R224), the hand's size (§9.1). R635: the
/// cast-on-draw cards wait just above them, out of the draws' reach.
fn deal_from(sink: &mut EngineSink, seat: usize) {
    for (at, &player) in PLAYER_IDS.iter().enumerate().skip(seat) {
        let size = opening_hand_size(sink.state, player);
        let shuffled = sink.rng.shuffle(&sink.state.players[player].library);
        let (quickdraw, set_aside, drawable) = {
            let state: &GameState = &*sink.state;
            let quickdraw: Vec<CardInstance> = shuffled
                .iter()
                .filter(|card| is_quickdraw(state, card))
                .take(size.max(0) as usize)
                .cloned()
                .collect();
            let dealt: IndexSet<&str> = quickdraw.iter().map(|card| card.id.as_str()).collect();
            let rest: Vec<&CardInstance> = shuffled
                .iter()
                .filter(|card| !dealt.contains(card.id.as_str()))
                .collect();
            let set_aside: Vec<CardInstance> = rest
                .iter()
                .filter(|card| casts_on_draw(state, card))
                .map(|card| (*card).clone())
                .collect();
            let drawable: Vec<CardInstance> = rest
                .iter()
                .filter(|card| !casts_on_draw(state, card))
                .map(|card| (*card).clone())
                .collect();
            (quickdraw, set_aside, drawable)
        };
        let quickdraw_count = quickdraw.len() as i32;
        let drawable_len = drawable.len() as i32;
        let mut library = drawable;
        library.extend(set_aside);
        library.extend(quickdraw);
        sink.state.players[player].library = library;

        // R748: what the other cards cannot fill comes from the set-aside cards, uncast.
        let others = 0_i32.max((size - quickdraw_count).min(drawable_len));
        deal_suspended(sink, player, size - quickdraw_count - others);
        draw(sink, player, others);
        // R158: an arrival clause of the opening draw is asking, so this seat's Quickdraw cards, the
        // seats after it and the mulligan wait behind it.
        if paused(sink) {
            if sink.state.result.is_none() {
                owe_setup(sink, json!({ "step": QUICKDRAW_STEP, "seat": at }));
            }
            return;
        }
        deal_quickdraw(sink, player);
    }

    open_mulligans(sink);
}

/// R225, R640: the Quickdraw cards `deal_from` left at the very bottom of the library, read off it so
/// the owed `quickdraw` step (R113) names no card the other seat's AI could read in `state.work` (R185).
fn dealt_quickdraw(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    let library = &state.players[player].library;
    let size = opening_hand_size(state, player);
    let mut dealt: Vec<CardInstance> = Vec::new();
    for card in library.iter().rev() {
        if (dealt.len() as i32) >= size {
            break;
        }
        if !is_quickdraw(state, card) {
            break;
        }
        dealt.insert(0, card.clone());
    }
    dealt
}

/// R225: a card the deal puts into the hand from the library arrives as a draw does, counted and reported.
fn deal_as_draw(sink: &mut EngineSink, player: PlayerId, mut card: CardInstance) {
    move_to_zone(
        sink.state,
        &mut card,
        OffFieldZone::Hand,
        MoveToZoneOptions::default(),
    );
    sink.state.counters.drawn += 1;
    sink.events.push(GameEvent::Drawn {
        player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        turn_draw: None,
        emptied: None,
    });
    sink.events.push(GameEvent::AddedToHand {
        player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        creator: None,
    });
}

/// R225: the seat's dealt Quickdraw cards, each as the opening draw it replaces.
fn deal_quickdraw(sink: &mut EngineSink, player: PlayerId) {
    for card in dealt_quickdraw(sink.state, player) {
        deal_as_draw(sink, player, card);
    }
}

/// R748: `count` of the set-aside cards (never a Quickdraw card, R635), dealt uncast as draws (R225)
/// and marked to be cast at the start of the game.
fn deal_suspended(sink: &mut EngineSink, player: PlayerId, count: i32) {
    if count <= 0 {
        return;
    }
    let waiting: Vec<CardInstance> = {
        let state: &GameState = &*sink.state;
        state.players[player]
            .library
            .iter()
            .filter(|card| casts_on_draw(state, card) && !is_quickdraw(state, card))
            .take(count as usize)
            .cloned()
            .collect()
    };
    for card in waiting {
        let instance_id = card.id.clone();
        deal_as_draw(sink, player, card);
        if let Some(live) = find_instance_mut(sink.state, &instance_id) {
            live.memory
                .insert(SUSPENDED_CAST_KEY.to_string(), Value::Bool(true));
        }
    }
}

/// R265, R266: a seat's answer is sealed, read by nothing until the other seat has answered as well;
/// the second answer resolves both.
pub fn answer_mulligan(sink: &mut EngineSink, player: PlayerId, keep: &[String]) {
    let Some(open) = sink.state.mulligan.as_mut() else {
        return;
    };
    let seat = &mut open[player];
    if seat.keep.is_some() {
        return;
    }
    // An answer is a set: stored in the order the prompt offered it (R221), so two spellings of one
    // answer are one state.
    let chosen: IndexSet<&str> = keep.iter().map(String::as_str).collect();
    let kept: Vec<String> = seat
        .prompt
        .options
        .iter()
        .map(|option| option.key.clone())
        .filter(|key| chosen.contains(key.as_str()))
        .collect();
    seat.keep = Some(kept);
    let choice_id = seat.prompt.id.clone();
    // §10.6: the answer names the prompt it answers. That a seat has answered is public, and what it
    // kept is not (R266, §9.1).
    sink.events.push(GameEvent::PromptAnswered {
        player,
        choice_id: Some(choice_id),
    });
    if !mulligan_owed(sink.state).is_empty() {
        return;
    }
    resolve_mulligans(sink);
}

/// R267: what a sealed answer returns, read against the hand as it stands when that seat's turn to
/// resolve comes: the cards the prompt offered and the answer did not keep that are still there. A
/// card the other seat's resolution put in this hand was never offered, so it stays.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SealedMulligan {
    player: PlayerId,
    offered: Vec<String>,
    keep: Vec<String>,
}

/// R265: both answers are in, so the seats resolve in seat order.
fn resolve_mulligans(sink: &mut EngineSink) {
    let Some(open) = sink.state.mulligan.as_ref() else {
        return;
    };
    let sealed: Vec<SealedMulligan> = PLAYER_IDS
        .into_iter()
        .map(|player| {
            let offered: Vec<String> = open[player]
                .prompt
                .options
                .iter()
                .map(|option| option.key.clone())
                .collect();
            let keep = open[player].keep.clone().unwrap_or_else(|| offered.clone());
            SealedMulligan {
                player,
                offered,
                keep,
            }
        })
        .collect();
    sink.state.mulligan = None;
    resolve_from(sink, &sealed);
}

// R9: the one `MULLIGAN_ORDER` this file plays.
const _: () = assert!(
    matches!(MULLIGAN_ORDER.as_bytes(), b"draw-then-shuffle"),
    "R9: setup.rs draws the mulligan's replacements, then shuffles the returned cards back"
);

/// R9: the replacements are drawn first, then the returned cards are shuffled back, which is what
/// "without replacement" means. One seat at a time, then the game.
fn resolve_from(sink: &mut EngineSink, sealed: &[SealedMulligan]) {
    let Some((next, rest)) = sealed.split_first() else {
        finish_setup(sink);
        return;
    };
    let offered: IndexSet<&str> = next.offered.iter().map(String::as_str).collect();
    let kept: IndexSet<&str> = next.keep.iter().map(String::as_str).collect();
    let returned: Vec<CardInstance> = sink.state.players[next.player]
        .hand
        .iter()
        .filter(|card| offered.contains(card.id.as_str()) && !kept.contains(card.id.as_str()))
        .cloned()
        .collect();

    for card in &returned {
        let hand = &mut sink.state.players[next.player].hand;
        if let Some(at) = hand.iter().position(|c| c.id == card.id) {
            hand.remove(at);
        }
    }

    // R635, R748: what the library cannot replace besides the set-aside cards is dealt from them, uncast,
    // and never as a fatigue draw.
    let returned_count = returned.len() as i32;
    let others = returned_count.min(drawable_count(sink.state, next.player));
    deal_suspended(sink, next.player, returned_count - others);
    draw(sink, next.player, others);
    // R224: an arrival clause of a replacement is asking, so the shuffle-back, the seats after this one
    // and the game wait in the owed item, with the returned cards, which are in no pile until then.
    if paused(sink) {
        if sink.state.result.is_none() {
            owe_setup(
                sink,
                json!({
                    "step": MULLIGAN_STEP,
                    "player": next.player,
                    "returned": returned,
                    "rest": rest,
                }),
            );
        }
        return;
    }

    finish_mulligan(sink, next.player, &returned, rest);
}

/// R9's second half — the returned cards shuffled back — then the next seat's mulligan, or the game.
fn finish_mulligan(
    sink: &mut EngineSink,
    player: PlayerId,
    returned: &[CardInstance],
    rest: &[SealedMulligan],
) {
    for card in returned {
        let mut card = card.clone();
        // R748: a card dealt uncast that the mulligan returned is a set-aside card again, shuffled in
        // with the rest of them (R635), and not cast.
        card.memory.shift_remove(SUSPENDED_CAST_KEY);
        let position = sink.rng.int(sink.state.players[player].library.len() as i32 + 1);
        move_to_zone(
            sink.state,
            &mut card,
            OffFieldZone::Library,
            MoveToZoneOptions {
                position: Some(LibraryPosition::At(position)),
                ..MoveToZoneOptions::default()
            },
        );
        // R311: the player returned it from their own hand, so they know what went back.
        if let Some(live) = find_instance_mut(sink.state, &card.id) {
            show_to_owner(live);
        }
        sink.events.push(GameEvent::ShuffledIn {
            player,
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            position,
            creator: None,
        });
    }

    if !sink.state.mulliganed.contains(&player) {
        sink.state.mulliganed.push(player);
    }
    resolve_from(sink, rest);
}

/// R224, §9.1: the cards a mulligan returned that wait in the owed item for their shuffle-back, in no
/// pile (`find_instance` misses them). §10.8 reads them as the library cards they are about to be: nobody
/// reads them (§3), their owner included.
pub fn returned_awaiting_shuffle(state: &GameState) -> Vec<String> {
    state
        .work
        .iter()
        .flat_map(|item| {
            if item.resume.hook != SETUP_WORK || item.resume.step != MULLIGAN_STEP {
                return Vec::new();
            }
            let Some(owed) = item.resume.data.get("owed").and_then(Value::as_object) else {
                return Vec::new();
            };
            let Some(returned) = owed.get("returned").and_then(Value::as_array) else {
                return Vec::new();
            };
            returned
                .iter()
                .filter_map(|card| card.as_object()?.get("id")?.as_str().map(str::to_string))
                .collect()
        })
        .collect()
}

fn owe_setup(sink: &mut EngineSink, owed: Value) {
    let step = owed
        .get("step")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let mut data = indexmap::IndexMap::new();
    data.insert("owed".to_string(), owed);
    owe(
        sink,
        Resume {
            def_id: String::new(),
            hook: SETUP_WORK.to_string(),
            step,
            radiant: false,
            instance_id: None,
            data,
        },
    );
}

/// `work.rs`'s handler for `SETUP_WORK`: setup, continued where a question stopped it (R113, R122).
pub fn run_owed_setup(sink: &mut EngineSink, item: &WorkItem) {
    let Some(owed) = item.resume.data.get("owed").and_then(Value::as_object) else {
        return;
    };
    let step = owed.get("step").and_then(Value::as_str);
    let seat = owed.get("seat").and_then(Value::as_u64).map(|seat| seat as usize);
    if step == Some(DEAL_STEP)
        && let Some(seat) = seat
    {
        deal_from(sink, seat);
        return;
    }
    if step == Some(QUICKDRAW_STEP)
        && let Some(seat) = seat
    {
        if let Some(&player) = PLAYER_IDS.get(seat) {
            deal_quickdraw(sink, player);
        }
        deal_from(sink, seat + 1);
        return;
    }
    let player = owed
        .get("player")
        .and_then(Value::as_str)
        .and_then(|text| text.parse::<PlayerId>().ok());
    if step == Some(MULLIGAN_STEP)
        && let Some(player) = player
    {
        let returned: Vec<CardInstance> = owed
            .get("returned")
            .and_then(Value::as_array)
            .map(|cards| {
                cards
                    .iter()
                    .filter_map(|card| serde_json::from_value::<CardInstance>(card.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        let rest = sealed_in(owed.get("rest"));
        finish_mulligan(sink, player, &returned, &rest);
        return;
    }
    if step == Some(START_OF_GAME_STEP) {
        let ids: Vec<String> = owed
            .get("ids")
            .and_then(Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| id.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        start_of_game_from(sink, &ids);
        return;
    }
    if step == Some(SUSPENDED_STEP) {
        resume_suspended(sink);
    }
}

/// The seats still to resolve, read back out of an owed item's JSON (R113).
fn sealed_in(raw: Option<&Value>) -> Vec<SealedMulligan> {
    let Some(entries) = raw.and_then(Value::as_array) else {
        return Vec::new();
    };
    let ids = |value: Option<&Value>| -> Vec<String> {
        value
            .and_then(Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| id.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    entries
        .iter()
        .filter_map(|entry| {
            let seat = entry.as_object()?;
            let player = seat.get("player")?.as_str()?.parse::<PlayerId>().ok()?;
            Some(SealedMulligan {
                player,
                offered: ids(seat.get("offered")),
                keep: ids(seat.get("keep")),
            })
        })
        .collect()
}

/// §2.1 step 3's close, R244: each seat's `OPENING_COINS`, dealt after both mulligans so a Coin is never
/// returned, redrawn or shuffled in (R9), whatever the seat's handicap (R182). §6.3's add to hand, not a
/// draw (§2.4, R4, R55), and no rng draw, so `(seed, decks, handicaps, log)` folds exactly (§9.3, R187).
/// A catalog without The Coin (the engine's partial fixtures) deals none.
pub fn deal_coins(sink: &mut EngineSink) {
    if sink.state.result.is_some() {
        return;
    }
    if find_def(Some(&*sink.state), COIN_DEF_ID).is_none() {
        return;
    }
    for (seat, player) in PLAYER_IDS.into_iter().enumerate() {
        let count = OPENING_COINS.get(seat).copied().unwrap_or(0);
        for _dealt in 0..count {
            let mut coin = new_instance(&mut *sink.state, COIN_DEF_ID, player, Zone::Hand { player });
            add_to_hand(sink, &mut coin);
        }
    }
}

/// R635: each seat's cast-on-draw cards go back one at a time at a uniform random place. Nothing is
/// reported: a `shuffledIn` per card would tell the other seat how many the deck holds (§9.1, R97).
fn shuffle_in_set_aside(sink: &mut EngineSink) {
    for player in PLAYER_IDS {
        let waiting: Vec<CardInstance> = {
            let state: &GameState = &*sink.state;
            state.players[player]
                .library
                .iter()
                .filter(|card| casts_on_draw(state, card))
                .cloned()
                .collect()
        };
        if waiting.is_empty() {
            continue;
        }
        let out: IndexSet<String> = waiting.iter().map(|card| card.id.clone()).collect();
        let side = &mut sink.state.players[player];
        side.library.retain(|card| !out.contains(&card.id));
        for card in waiting {
            let at = sink.rng.int(side.library.len() as i32 + 1) as usize;
            side.library.insert(at, card);
        }
    }
}

/// R635's shuffle-in, R244's Coin, the start-of-game effects, R748's casts, then player 1 takes the
/// first turn and draws (§2.1, R10).
pub fn finish_setup(sink: &mut EngineSink) {
    shuffle_in_set_aside(sink);
    deal_coins(sink);
    let cards: Vec<String> = PLAYER_IDS
        .into_iter()
        .flat_map(|player| {
            let side = &sink.state.players[player];
            side.hand
                .iter()
                .chain(side.library.iter())
                .map(|card| card.id.clone())
                .collect::<Vec<String>>()
        })
        .collect();
    start_of_game_from(sink, &cards);
}

/// §2.1 step 4 over the cards from `ids` on, then R748's casts and turn 1. A clause that asks owes the
/// clauses after it and the first turn behind its tail (R113, R117), so turn 1 never begins with a
/// question of setup's still open.
fn start_of_game_from(sink: &mut EngineSink, ids: &[String]) {
    for at in 0..ids.len() {
        let Some(card) = find_instance(sink.state, &ids[at]).cloned() else {
            continue;
        };
        let player = match card.zone {
            Zone::Hand { player } | Zone::Library { player } => player,
            _ => card.owner,
        };
        run_start_of_game(sink, &card, player);
        if paused(sink) {
            if sink.state.result.is_none() {
                owe_setup(
                    sink,
                    json!({ "step": START_OF_GAME_STEP, "ids": ids[at + 1..].to_vec() }),
                );
            }
            return;
        }
    }
    cast_suspended(sink);
}

/// R748: the next card still waiting in a hand to be cast, Player 1's hand first, in hand order.
fn next_suspended(state: &GameState) -> Option<CardInstance> {
    for player in PLAYER_IDS {
        let card = state.players[player]
            .hand
            .iter()
            .find(|card| card.memory.get(SUSPENDED_CAST_KEY) == Some(&Value::Bool(true)));
        if let Some(card) = card {
            return Some(card.clone());
        }
    }
    None
}

/// R748: each card a hand had to take uncast is cast as its draw would have cast it, with a state check
/// after each (R59) and no draw repeated; a Unit with no open zone stays in the hand (R459). A cast that
/// asks owes the rest and turn 1 (R113, R224).
///
/// Setup is turn 0 (BUILD M1-T1), no player's turn (§2.1 step 5), so a Spell cast in it has its §10.5
/// step 7 return ended before turn 1 as a turn's cleanup ends it (R155); `start_turn` empties the logs
/// that cleanup reads, so setup clears it first.
fn cast_suspended(sink: &mut EngineSink) {
    while let Some(mut card) = next_suspended(sink.state) {
        card.memory.shift_remove(SUSPENDED_CAST_KEY);
        if let Some(live) = find_instance_mut(sink.state, &card.id) {
            live.memory.shift_remove(SUSPENDED_CAST_KEY);
        }
        cast_dealt_card(sink, &card);
        if paused(sink) {
            if sink.state.result.is_none() {
                owe_setup(sink, json!({ "step": SUSPENDED_STEP }));
            }
            return;
        }
    }
    clear_return_flags(sink.state);
    start_turn(sink, PLAYER_IDS[0]);
}

/// R748: a cast that asked has resolved, so its state check runs (R59) and the casts go on.
fn resume_suspended(sink: &mut EngineSink) {
    state_check(sink);
    if paused(sink) {
        if sink.state.result.is_none() {
            owe_setup(sink, json!({ "step": SUSPENDED_STEP }));
        }
        return;
    }
    cast_suspended(sink);
}

/// A fresh instance of `def_id` in a player's hand, for setup-time card creation.
pub fn create_in_hand(sink: &mut EngineSink, player: PlayerId, def_id: &str) -> CardInstance {
    let card = new_instance(&mut *sink.state, def_id, player, Zone::Hand { player });
    sink.state.players[player].hand.push(card.clone());
    card
}
