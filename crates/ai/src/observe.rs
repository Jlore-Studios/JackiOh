//! What the AI's seat may know (R185, SPEC §9.9). This is the only module that reads a true
//! `GameState`: `decide` calls `ai_to_act` and `redact` on it and nothing else, and every later step
//! works on the redacted copy or on a determinization of it (`determinize.rs`).
//!
//! `redact` follows docs/polish/3-ai.md's seven steps in order. What it keeps is kept on purpose:
//! board cards (buried Stack cards included), damage and buffs, exertion, summoning turns, positions,
//! graveyards, exiles, `resolving`, turn logs, modifiers, delayed effects, counters, hero health and
//! armor, mana, draw offers and handicaps are public history the seat watched happen. Instance ids of
//! hidden cards are kept too: they give away a deck-list position, which the AI cannot map to an
//! identity.
//!
//! Port of `packages/ai/src/observe.ts`. TS rebuilt the queue entries it kept with object spreads;
//! here they are edited in place on the clone `redact` returns, which is the same state. The loose
//! JSON TS walked (a captured event, setup's owed mulligan) is walked as `serde_json::Value`.

use std::cmp::Ordering;

use indexmap::IndexSet;
use jackioh_engine::{
    ANSWER_KEY, AttackHealth, CardInstance, CardType, Counters, GameEvent, GameState, PLAYER_IDS,
    PendingChoice, PlayerId, Resume, Row, SETUP_WORK, Selection, Zone, active_units_of,
    announced_face_down_to, effective_cost, find_def, find_instance, find_instance_mut, handicap_of,
    mulligan_prompt_for, unclamped_attack, unit_view,
};
use jackioh_engine::{CostOptions, subsystems};
use serde_json::Value;

pub const HIDDEN_DEF_ID: &str = "ai:hidden";

/// Where one card instance sits in a state, so a walk can read a card and later write it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Hand(usize),
    Library(usize),
    Graveyard(usize),
    Exile(usize),
    Resolving(usize),
    /// A unit zone's pile (lane index, depth in the pile).
    Unit(usize, usize),
    Backrow(usize),
    /// B5 E21: a backrow pile's dormant card (lane index, depth).
    BackrowPile(usize, usize),
    /// R446: a carrier's Unit (lane index).
    Carried(usize),
}

/// Every card instance the state holds in a pile, a lane or a Stack, in a fixed order (TS
/// `everyInstance`), as places.
fn every_place(state: &GameState) -> Vec<(PlayerId, Place)> {
    let mut out: Vec<(PlayerId, Place)> = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        out.extend((0..side.hand.len()).map(|at| (player, Place::Hand(at))));
        out.extend((0..side.library.len()).map(|at| (player, Place::Library(at))));
        out.extend((0..side.graveyard.len()).map(|at| (player, Place::Graveyard(at))));
        out.extend((0..side.exile.len()).map(|at| (player, Place::Exile(at))));
        out.extend((0..side.resolving.len()).map(|at| (player, Place::Resolving(at))));
        for (lane, pile) in side.units.iter().enumerate() {
            if let Some(pile) = pile {
                out.extend((0..pile.len()).map(|depth| (player, Place::Unit(lane, depth))));
            }
        }
        for (lane, card) in side.backrow.iter().enumerate() {
            if card.is_some() {
                out.push((player, Place::Backrow(lane)));
            }
        }
        // B5 E21, R446: a backrow pile's dormant cards and a carrier's Unit are on the board too.
        for (lane, pile) in side.backrow_piles.iter().flatten().enumerate() {
            out.extend((0..pile.len()).map(|depth| (player, Place::BackrowPile(lane, depth))));
        }
        for (lane, card) in side.carried.iter().flatten().enumerate() {
            if card.is_some() {
                out.push((player, Place::Carried(lane)));
            }
        }
    }
    out
}

fn card_at(state: &GameState, player: PlayerId, place: Place) -> Option<&CardInstance> {
    let side = &state.players[player];
    match place {
        Place::Hand(at) => side.hand.get(at),
        Place::Library(at) => side.library.get(at),
        Place::Graveyard(at) => side.graveyard.get(at),
        Place::Exile(at) => side.exile.get(at),
        Place::Resolving(at) => side.resolving.get(at),
        Place::Unit(lane, depth) => side
            .units
            .get(lane)
            .and_then(|pile| pile.as_ref())
            .and_then(|pile| pile.get(depth)),
        Place::Backrow(lane) => side.backrow.get(lane).and_then(|card| card.as_ref()),
        Place::BackrowPile(lane, depth) => side
            .backrow_piles
            .as_ref()
            .and_then(|piles| piles.get(lane))
            .and_then(|pile| pile.get(depth)),
        Place::Carried(lane) => side
            .carried
            .as_ref()
            .and_then(|row| row.get(lane))
            .and_then(|card| card.as_ref()),
    }
}

fn card_at_mut(state: &mut GameState, player: PlayerId, place: Place) -> Option<&mut CardInstance> {
    let side = &mut state.players[player];
    match place {
        Place::Hand(at) => side.hand.get_mut(at),
        Place::Library(at) => side.library.get_mut(at),
        Place::Graveyard(at) => side.graveyard.get_mut(at),
        Place::Exile(at) => side.exile.get_mut(at),
        Place::Resolving(at) => side.resolving.get_mut(at),
        Place::Unit(lane, depth) => side
            .units
            .get_mut(lane)
            .and_then(|pile| pile.as_mut())
            .and_then(|pile| pile.get_mut(depth)),
        Place::Backrow(lane) => side.backrow.get_mut(lane).and_then(|card| card.as_mut()),
        Place::BackrowPile(lane, depth) => side
            .backrow_piles
            .as_mut()
            .and_then(|piles| piles.get_mut(lane))
            .and_then(|pile| pile.get_mut(depth)),
        Place::Carried(lane) => side
            .carried
            .as_mut()
            .and_then(|row| row.get_mut(lane))
            .and_then(|card| card.as_mut()),
    }
}

/// `c17` → 17; anything that is not a minted instance id sorts after every one that is.
fn instance_number(id: &str) -> f64 {
    match id.strip_prefix('c') {
        Some(digits) if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => {
            digits.parse::<f64>().unwrap_or(f64::INFINITY)
        }
        _ => f64::INFINITY,
    }
}

fn by_instance_id(a: &CardInstance, b: &CardInstance) -> Ordering {
    let na = instance_number(&a.id);
    let nb = instance_number(&b.id);
    if na != nb {
        return if na < nb {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }
    if a.id == b.id {
        return Ordering::Equal;
    }
    if a.id < b.id {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

/// §10.8 and R33, as `view_for` reads them: a backrow Trap or Field Trap is readable by its current
/// controller only until it flips face-up; a Field Spell is public. A placeholder is never readable.
fn backrow_hidden_from(state: &GameState, card: &CardInstance, seat: PlayerId) -> bool {
    if card.controller == seat || card.face_up == Some(true) {
        return false;
    }
    let Some(def) = find_def(Some(state), &card.def_id) else {
        return true;
    };
    def.type_ == CardType::Trap || def.type_ == CardType::FieldTrap
}

/// The instance ids `redact` hides from `seat` (step 2): the opponent's hand and library, every
/// backrow card the seat cannot read, the cards in the seat's own library that were minted for
/// the opponent's opening deck (#87 Pocket Chaos's library swap, R73), and every card in the seat's
/// own library it was never shown (no `knownAs`, R312: a library swapped in, or swapped away and
/// back, and #83 Transmogulate's picks), which its own `viewFor` list counts as unknown too. A card
/// the seat's own open prompt offers as an option is shown to the seat by `viewFor` (§10.8), so it
/// is not hidden.
pub fn hidden_instance_ids(state: &GameState, seat: PlayerId) -> IndexSet<String> {
    let opp = seat.opponent();
    let mut hidden: IndexSet<String> = IndexSet::new();

    for card in &state.players[opp].hand {
        hidden.insert(card.id.clone());
    }
    for card in &state.players[opp].library {
        hidden.insert(card.id.clone());
    }

    for player in PLAYER_IDS {
        // B5 E21: a face-down card dormant under a backrow pile is as hidden as one on top (R33, R447).
        let side = &state.players[player];
        let tops = side.backrow.iter().flatten();
        let dormant = side.backrow_piles.iter().flatten().flatten();
        for card in tops.chain(dormant) {
            if backrow_hidden_from(state, card, seat) {
                hidden.insert(card.id.clone());
            }
        }
        // R448: a card waiting in the resolving zone to be set face-down is its player's alone.
        for card in &state.players[player].resolving {
            if announced_face_down_to(state, card, seat) {
                hidden.insert(card.id.clone());
            }
        }
    }

    // createGame mints c1..c{n1} for p1's deck and c{n1+1}..c{n1+n2} for p2's (R184's deck sizes).
    let n1 = f64::from(handicap_of(&state.players.p1).deck_size);
    let n2 = f64::from(handicap_of(&state.players.p2).deck_size);
    let low = if opp == PlayerId::P1 { 1.0 } else { n1 + 1.0 };
    let high = if opp == PlayerId::P1 { n1 } else { n1 + n2 };
    for card in &state.players[seat].library {
        let n = instance_number(&card.id);
        if n >= low && n <= high {
            hidden.insert(card.id.clone());
        }
        // R312: a card of the seat's own library it was never shown is as hidden as the opponent's.
        if card.known_as.is_none() {
            hidden.insert(card.id.clone());
        }
    }

    let pending: Option<&PendingChoice> = state
        .pending
        .as_ref()
        .or_else(|| mulligan_prompt_for(state, seat));
    if let Some(pending) = pending
        && pending.player_id == seat
    {
        for option in &pending.options {
            if let Selection::Instance { instance_id } = &option.selection {
                hidden.shift_remove(instance_id);
            }
        }
    }

    hidden
}

/// Step 3: the card keeps its id, owner, controller, zone and backrow lane, and nothing else (a
/// face-down backrow card's shown cost is put back by `redact`, R351).
fn to_placeholder(card: &mut CardInstance) {
    card.def_id = HIDDEN_DEF_ID.to_string();
    card.radiant = false;
    card.cost_mod = 0;
    card.memory = Default::default();
    card.counters = Counters::default();
    card.granted_keywords = Vec::new();
    card.buffs = AttackHealth { attack: 0, health: 0 };
    card.damage = 0;
    card.cost_override = None;
    card.x = None;
    card.embiggened = None;
    card.return_to_hand_at_end_of_turn = None;
    // R311: what the card's owner was shown of it going into their library names it too.
    card.known_as = None;
    // R385, R386, B5 E39 (patch v0.2.0): its Brittle count, what Degrade, Upgrade and KY's Constant
    // changed on it and the enchantments riding it are the card's as much as its face is.
    card.tuning = None;
    card.brittle = None;
    card.enchantments = None;
}

/// `to_placeholder` on a card held as loose JSON (setup's owed mulligan record), as TS ran it on a
/// `JSON.parse(JSON.stringify(card))` copy.
fn to_placeholder_json(card: &mut Value) {
    let Some(object) = card.as_object_mut() else {
        return;
    };
    object.insert("defId".to_string(), Value::from(HIDDEN_DEF_ID));
    object.insert("radiant".to_string(), Value::Bool(false));
    object.insert("costMod".to_string(), Value::from(0));
    object.insert("memory".to_string(), Value::Object(serde_json::Map::new()));
    object.insert("counters".to_string(), Value::Object(serde_json::Map::new()));
    object.insert("grantedKeywords".to_string(), Value::Array(Vec::new()));
    object.insert(
        "buffs".to_string(),
        serde_json::json!({ "attack": 0, "health": 0 }),
    );
    object.insert("damage".to_string(), Value::from(0));
    for key in [
        "costOverride",
        "x",
        "embiggened",
        "returnToHandAtEndOfTurn",
        // R311: what the card's owner was shown of it going into their library names it too.
        "knownAs",
        // R385, R386, B5 E39: its Brittle count, its tuning and its enchantments.
        "tuning",
        "brittle",
        "enchantments",
    ] {
        object.remove(key);
    }
}

fn is_hidden_id(value: Option<&str>, hidden: &IndexSet<String>) -> bool {
    value.is_some_and(|id| hidden.contains(id))
}

fn is_hidden_value(value: Option<&Value>, hidden: &IndexSet<String>) -> bool {
    is_hidden_id(value.and_then(Value::as_str), hidden)
}

/// Step 5: every `defId` that sits next to a hidden instance id becomes the placeholder id.
fn scrub_event_value(event: &mut Value, hidden: &IndexSet<String>) {
    let Some(copy) = event.as_object_mut() else {
        return;
    };
    if is_hidden_value(copy.get("instanceId"), hidden) {
        if copy.contains_key("defId") {
            copy.insert("defId".to_string(), Value::from(HIDDEN_DEF_ID));
        }
        if copy.contains_key("fromDefId") {
            copy.insert("fromDefId".to_string(), Value::from(HIDDEN_DEF_ID));
        }
    }
    if is_hidden_value(copy.get("newInstanceId"), hidden) && copy.contains_key("toDefId") {
        copy.insert("toDefId".to_string(), Value::from(HIDDEN_DEF_ID));
    }
    if is_hidden_value(copy.get("resultInstanceId"), hidden) && copy.contains_key("defId") {
        copy.insert("defId".to_string(), Value::from(HIDDEN_DEF_ID));
    }
}

/// `scrub_event_value` on a typed event: through its JSON, back into the event.
fn scrub_event(event: &GameEvent, hidden: &IndexSet<String>) -> GameEvent {
    let Ok(mut value) = serde_json::to_value(event) else {
        return event.clone();
    };
    scrub_event_value(&mut value, hidden);
    serde_json::from_value::<GameEvent>(value).unwrap_or_else(|_| event.clone())
}

/// Whether a queue entry belongs to a hidden card, by its `instanceId` or its `resume.instanceId`.
fn belongs_to_hidden(instance_id: Option<&str>, resume: Option<&Resume>, hidden: &IndexSet<String>) -> bool {
    is_hidden_id(instance_id, hidden) || is_hidden_id(resume.and_then(|r| r.instance_id.as_deref()), hidden)
}

/// A kept entry's captured event (a trigger's `resume.data.event`) is scrubbed like a dispatch event,
/// so a face-up card's trigger cannot carry a hidden card's identity through the queue.
fn scrub_resume(resume: &mut Resume, hidden: &IndexSet<String>) {
    if let Some(event) = resume.data.get_mut("event")
        && event.is_object()
    {
        scrub_event_value(event, hidden);
    }
}

/// R465: a multiple-choice problem's key (`ANSWER_KEY`, Classic+ #42) rides in the resume data of the
/// prompt that asks it, and of the tail a pause inside its answered step parks. It never leaves the
/// engine, so the seat reads no key, its own prompt's included: it answers from what the prompt shows,
/// as a human does, and a simulated answer is judged right by nothing (`answeredCorrectly`).
fn without_answer_key(resume: &mut Resume) {
    resume.data.shift_remove(ANSWER_KEY);
}

/// R266, R185: setup's owed mulligan item (R224, R265) carries two things the seat may not read: the
/// sealed answers of the seats still to resolve (`rest`), and, while a seat's own resolution waits on
/// a question (an arrival clause's, R151), the cards it returned (`returned`, full instances until
/// they go back). The opponent's sealed answer becomes "keeps everything it was offered", which says
/// nothing, and its returned cards become placeholders.
fn scrub_owed_mulligan(resume: &mut Resume, opp: PlayerId) {
    if resume.hook != SETUP_WORK {
        return;
    }
    let Some(owed) = resume.data.get_mut("owed") else {
        return;
    };
    let Some(copy) = owed.as_object_mut() else {
        return;
    };
    let opp_value = Value::from(opp.as_str());
    if let Some(Value::Array(rest)) = copy.get_mut("rest") {
        for entry in rest.iter_mut() {
            let Some(seat) = entry.as_object_mut() else {
                continue;
            };
            if seat.get("player") != Some(&opp_value) {
                continue;
            }
            let Some(Value::Array(offered)) = seat.get("offered") else {
                continue;
            };
            let keep = Value::Array(offered.clone());
            seat.insert("keep".to_string(), keep);
        }
    }
    let opps_own = copy.get("player") == Some(&opp_value);
    if opps_own && let Some(Value::Array(returned)) = copy.get_mut("returned") {
        for card in returned.iter_mut() {
            to_placeholder_json(card);
        }
    }
}

/// R185: the state as `seat` may know it. Pure; the input is not mutated.
pub fn redact(state: &GameState, seat: PlayerId) -> GameState {
    let opp = seat.opponent();
    // Step 2 reads the true state: which cards are hidden is itself decided by public facts.
    let hidden = hidden_instance_ids(state, seat);
    let mut next = state.clone();

    // Step 1: the seed, the cursor and the nonce log (which carries unredacted events).
    next.seed = "redacted".to_string();
    next.rng_cursor = 0;
    next.applied = Vec::new();
    // R417: a last board is its own seat's alone (§10.8); the other seat's never reaches the AI.
    if let Some(last_boards) = next.last_boards.as_mut() {
        *last_boards.slot(opp) = None;
        if last_boards.get(seat).is_none() {
            next.last_boards = None;
        }
    }
    // R419: C+ #35 Rollback's history holds whole instances — face-down traps, cards since gone to a hand.
    // ponytail: dropped whole, so the AI simulates a Rollback as restoring nothing; redact each snapshot's
    // hidden cards instead if the AI should ever plan around one.
    next.board_history = None;
    // R676, R678: the decks the match began with and the boards a Glitch may lay down are no seat's to
    // read; dropped whole, so the AI simulates a Glitch reset as nothing and its boards as empty fields.
    next.opening = None;
    next.glitch_boards = None;

    // Step 3: every hidden card becomes a placeholder. R351: a face-down backrow card's cost is shown
    // to both players, so its placeholder keeps that number as its price, and whatever trap
    // `determinize` puts there shows the seat the same cost the true board does. A card that is a
    // placeholder already (the state was redacted before: the training arena sends each agent
    // `redact(state, seat)`, and `decide` redacts what it is given, SURFACE §14.1) has no definition
    // to price, and already shows its price, so redacting twice is redacting once.
    for (player, place) in every_place(&next) {
        let Some(card) = card_at(&next, player, place) else {
            continue;
        };
        if !hidden.contains(&card.id) {
            continue;
        }
        let shown_cost = if !matches!(
            card.zone,
            Zone::Field {
                row: Row::Backrow,
                ..
            }
        ) {
            None
        } else if card.def_id == HIDDEN_DEF_ID {
            card.cost_override
        } else {
            Some(effective_cost(&next, card, CostOptions::default()))
        };
        if let Some(card) = card_at_mut(&mut next, player, place) {
            to_placeholder(card);
            if let Some(shown) = shown_cost {
                card.cost_override = Some(shown);
            }
        }
    }

    // Step 4: erase the true order of the piles the seat cannot see into.
    next.players[opp].hand.sort_by(by_instance_id);
    next.players[opp].library.sort_by(by_instance_id);
    next.players[seat].library.sort_by(by_instance_id);

    // Step 5: queue entries of hidden cards go; events naming one lose the definition.
    next.trigger_queue
        .retain(|entry| !belongs_to_hidden(Some(entry.instance_id.as_str()), Some(&entry.resume), &hidden));
    for entry in next.trigger_queue.iter_mut() {
        scrub_resume(&mut entry.resume, &hidden);
        without_answer_key(&mut entry.resume);
    }

    let mut cursor = next.work_cursor as i64;
    let mut kept_work = Vec::with_capacity(next.work.len());
    for (index, mut item) in std::mem::take(&mut next.work).into_iter().enumerate() {
        if belongs_to_hidden(None, Some(&item.resume), &hidden) {
            if index < next.work_cursor {
                cursor -= 1;
            }
            continue;
        }
        scrub_resume(&mut item.resume, &hidden);
        scrub_owed_mulligan(&mut item.resume, opp);
        without_answer_key(&mut item.resume);
        kept_work.push(item);
    }
    next.work_cursor = cursor.min(kept_work.len() as i64).max(0) as usize;
    next.work = kept_work;

    next.echo_queue
        .retain(|entry| !belongs_to_hidden(Some(entry.instance_id.as_str()), None, &hidden));
    next.delayed
        .retain(|entry| !belongs_to_hidden(None, Some(&entry.resume), &hidden));
    for entry in next.delayed.iter_mut() {
        scrub_resume(&mut entry.resume, &hidden);
        without_answer_key(&mut entry.resume);
    }
    for entry in next.dispatch.iter_mut() {
        entry.event = scrub_event(&entry.event, &hidden);
    }
    if let Some(pending) = next.pending.as_mut() {
        scrub_resume(&mut pending.resume, &hidden);
        without_answer_key(&mut pending.resume);
    }

    // Step 6: a transient definition only a hidden card uses would name that card.
    let mut referenced: IndexSet<String> = IndexSet::new();
    for (player, place) in every_place(&next) {
        if let Some(card) = card_at(&next, player, place)
            && !hidden.contains(&card.id)
        {
            referenced.insert(card.def_id.clone());
            // R399, R546: a copier the seat can see has its copy's text, fixed on it once it is played.
            if let Some(copy) = subsystems::copied_text_of(&next, card) {
                referenced.insert(copy.def_id);
            }
        }
    }
    // R451, R185: the play records are public history, and the engine reads a definition by the id
    // they keep: a copier's text is the last Spell's (R399), wherever the card that was played went
    // since, and T-AI-5 adds a copy of a player's last face-up card. A fused card that went back into a
    // hidden library or hand is still named by them, so its definition stays.
    if let Some(last) = &next.last_spell {
        referenced.insert(last.def_id.clone());
    }
    for player in PLAYER_IDS {
        if let Some(record) = next.players[player]
            .game_log
            .as_ref()
            .and_then(|log| log.last_face_up_play.as_ref())
        {
            referenced.insert(record.def_id.clone());
        }
    }
    // A kept continuation runs by the definition it names (`Resume.defId`), which outlives its card: a
    // fused unit token's Death, its card gone (R11). One of a hidden card's own is not the seat's to read.
    let resumes = next
        .trigger_queue
        .iter()
        .map(|entry| &entry.resume)
        .chain(next.work.iter().map(|item| &item.resume))
        .chain(next.delayed.iter().map(|entry| &entry.resume))
        .chain(next.pending.iter().map(|pending| &pending.resume));
    for resume in resumes {
        if !belongs_to_hidden(None, Some(resume), &hidden) {
            referenced.insert(resume.def_id.clone());
        }
    }
    // A kept fusion of a fusion names its older ingredient in its id (R179), so that ingredient's
    // definition stays too: the id already says what it is, and its faces are public, summed into
    // the kept def's own.
    let mut transient: IndexSet<String> = IndexSet::new();
    let mut keep: Vec<String> = referenced
        .into_iter()
        .filter(|def_id| next.transient_defs.contains_key(def_id))
        .collect();
    while let Some(def_id) = keep.pop() {
        if !next.transient_defs.contains_key(&def_id) || transient.contains(&def_id) {
            continue;
        }
        transient.insert(def_id.clone());
        for id in subsystems::fused_ingredients(&next, &def_id).unwrap_or_default() {
            if next.transient_defs.contains_key(&id) {
                keep.push(id);
            }
        }
    }
    next.transient_defs.retain(|def_id, _| transient.contains(def_id));

    // R602: what a hidden card visibly does stays. A face-down trap's aura is live (R403), and a unit's
    // Attack and Health are on the board for both players to read, so every unit keeps the stats it
    // shows: the difference its placeholder made goes on the unit's buffs.
    for player in PLAYER_IDS {
        let ids: Vec<String> = active_units_of(&next, player)
            .iter()
            .map(|unit| unit.id.clone())
            .collect();
        for id in ids {
            let Some(shown) = find_instance(state, &id) else {
                continue;
            };
            let truth = unit_view(state, shown);
            let attack_delta = match find_instance(&next, &id) {
                Some(unit) => truth.attack - unclamped_attack(&next, unit),
                None => continue,
            };
            if let Some(unit) = find_instance_mut(&mut next, &id) {
                unit.buffs.attack += attack_delta;
            }
            let health_delta = match find_instance(&next, &id) {
                Some(unit) => truth.max_health - unit_view(&next, unit).max_health,
                None => continue,
            };
            if let Some(unit) = find_instance_mut(&mut next, &id) {
                unit.buffs.health += health_delta;
            }
        }
    }

    // Step 7: the opponent's prompt shows that it is open and whose it is, nothing more (R81). The
    // same goes for its mulligan while both are open (R265, R266): that it has answered is public,
    // and what it was offered and what it kept are not.
    if let Some(pending) = next.pending.as_mut()
        && pending.player_id == opp
    {
        pending.options = Vec::new();
    }
    if let Some(open) = next.mulligan.as_mut() {
        let theirs = &mut open[opp];
        theirs.prompt.options = Vec::new();
        theirs.prompt.max = 0;
        if theirs.keep.is_some() {
            theirs.keep = Some(Vec::new());
        }
    }

    next
}

/// R188: the opponent offered a draw this turn and nobody has answered it yet.
pub fn unanswered_draw_offer(state: &GameState, seat: PlayerId) -> bool {
    let off = seat.opponent();
    let side = &state.players[off];
    let offer = &side.draw_offer;
    state.active == off
        && state.phase == jackioh_engine::Phase::Main
        && state.pending.is_none()
        && offer.offered_turn == Some(state.turn)
        && offer.blocked_until.unwrap_or(0) <= side.turns_started
}

/// Whether `seat` owes an action: its prompt, its own mulligan while both are open (R265) — which it
/// answers without waiting for the other seat's — its main phase, or an unanswered draw offer.
pub fn ai_to_act(state: &GameState, seat: PlayerId) -> bool {
    if state.result.is_some() {
        return false;
    }
    if let Some(pending) = &state.pending
        && pending.player_id == seat
    {
        return true;
    }
    if state.pending.is_none() && mulligan_prompt_for(state, seat).is_some() {
        return true;
    }
    if state.pending.is_none() && state.active == seat && state.phase == jackioh_engine::Phase::Main {
        return true;
    }
    unanswered_draw_offer(state, seat)
}
