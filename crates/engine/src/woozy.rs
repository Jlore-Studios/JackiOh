//! Woozy X (SPEC §6.1, Stinkyscape+ E5; R3548000–R3548001): a player's public counter, a
//! stein the client draws beside Jade and Credits. Playing a card with Woozy X adds X to its
//! controller's counter; the counter drops by `WOOZY_DECAY` at the start of its owner's turn and
//! is gone at 0. While it is up, a card the player newly plays, summons or draws is stamped with
//! the counter's level and turn, and the stamp may blur one of the card's words — a number, a
//! keyword or a word of its name — on its owner's view alone. The blur is drawn from a seed stream
//! of the game's seed and the card's instance, never from the game's `Rng`, so a reload or a
//! replay shows the same blur and `rng_cursor` never moves for one (R3548001).

use crate::config::{PERCENT, WOOZY_BLUR_CHANCE_CAP, WOOZY_BLUR_CHANCE_PER_STACK, WOOZY_DECAY};
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, WoozyCounter, WoozyStamp};
use crate::wire::{CardType, CounterKind, GameEvent, KeywordKind, PlayerId, WoozyBlur, Zone};

/// What the blur's seed stream is keyed under: the game's seed, this, the card's id and its stamp's
/// turn, so the same card blurs the same way however often its view is built. It keeps the keyword's
/// old name, so a game played while it was called Tipsy blurs the same words on replay (R5181000).
pub const WOOZY_BLUR_STREAM: &str = ":tipsy-blur";

/// R3548000: the stamp a card arriving for `player` carries while their Woozy counter is up: whose,
/// how high and on which turn. `None` at none, so a card arriving sober carries nothing.
pub fn stamp_for(state: &GameState, player: PlayerId) -> Option<WoozyStamp> {
    let counter = state.players[player].woozy?;
    Some(WoozyStamp {
        player,
        level: counter.count,
        turn: state.turn,
    })
}

/// R3548000: playing a card with Woozy X adds X to its controller's counter, reported as one
/// `counterChanged` of the `woozy` kind on `hero-<player>` with the X it added, as Jade reports
/// its rises. A card without Woozy changes and reports nothing.
pub fn add_on_play(sink: &mut EngineSink<'_>, player: PlayerId, card: &CardInstance) {
    let Some(added) = crate::tuning::numbered_sum(
        &crate::layers::card_keywords(sink.state, card),
        KeywordKind::Woozy,
    ) else {
        return;
    };
    if added <= 0 {
        return;
    }
    let counter = sink.state.players[player].woozy;
    let value = counter.map_or(added, |held| held.count + added);
    sink.state.players[player].woozy = Some(WoozyCounter {
        count: value,
        since: counter.map_or(sink.state.turn, |held| held.since),
    });
    sink.events.push(GameEvent::CounterChanged {
        instance_id: format!("hero-{player}"),
        counter: CounterKind::Woozy,
        value,
        placed: Some(added),
    });
}

/// R3548000: at the start of `player`'s turn their Woozy counter drops by `WOOZY_DECAY`, reported
/// as a `counterChanged` of its new value; at 0 or below the counter is gone, reported as 0.
pub fn decay_at_turn_start(sink: &mut EngineSink<'_>, player: PlayerId) {
    let Some(counter) = sink.state.players[player].woozy else {
        return;
    };
    let value = counter.count - WOOZY_DECAY;
    sink.state.players[player].woozy = if value <= 0 {
        None
    } else {
        Some(WoozyCounter {
            count: value,
            since: counter.since,
        })
    };
    sink.events.push(GameEvent::CounterChanged {
        instance_id: format!("hero-{player}"),
        counter: CounterKind::Woozy,
        value: value.max(0),
        placed: None,
    });
}

/// R3548001: the chance, in percent, that a card stamped at `level` blurs: ten points a level to a
/// cap of fifty.
pub fn blur_chance(level: i32) -> i32 {
    (level.max(0) * WOOZY_BLUR_CHANCE_PER_STACK).min(WOOZY_BLUR_CHANCE_CAP)
}

/// The faces a stamped card may blur, in the order the draw picks among them: each whitespace word
/// of its definition's name, its cost, its attack and health when its running face has attack, then
/// each distinct keyword kind its face carries.
fn blur_targets(state: &GameState, card: &CardInstance) -> Vec<WoozyBlur> {
    let mut targets: Vec<WoozyBlur> = Vec::new();
    let name = crate::catalog::def_of(Some(state), &card.def_id).name.clone();
    for (index, _) in name.split_whitespace().enumerate() {
        targets.push(WoozyBlur::Name { word: index as i32 });
    }
    targets.push(WoozyBlur::Cost);
    if crate::faces::card_type_of(state, card) == CardType::Unit {
        targets.push(WoozyBlur::Attack);
        targets.push(WoozyBlur::Health);
    }
    let mut kinds: Vec<KeywordKind> = Vec::new();
    for keyword in crate::layers::card_keywords(state, card) {
        if !kinds.contains(&keyword.kind()) {
            kinds.push(keyword.kind());
        }
    }
    for kind in kinds {
        targets.push(WoozyBlur::Keyword { keyword: kind });
    }
    targets
}

/// R3548001: what of a stamped card its viewer sees blurred, or nothing. Only the stamp's own
/// player ever reads a blur, only in a hand or on the field, and only while the counter that
/// stamped it is still the one that is up: a stamp older than the counter's latest rise (`since`)
/// stays clear, so a counter that fell to 0 and rose again blurs only what arrived since. The pick
/// is drawn from a stream of the game's seed and the card's id and stamped turn, so a reload or a
/// replay shows the same blur and the game's `Rng` is never touched.
pub fn blur_for(state: &GameState, card: &CardInstance, viewer: PlayerId) -> Option<WoozyBlur> {
    let stamp = card.woozy?;
    if stamp.player != viewer {
        return None;
    }
    if !matches!(card.zone, Zone::Hand { .. } | Zone::Field { .. }) {
        return None;
    }
    let counter = state.players[stamp.player].woozy?;
    if stamp.turn < counter.since {
        return None;
    }
    let mut rng = crate::rng::Rng::new(
        &format!("{}{WOOZY_BLUR_STREAM}{}:{}", state.seed, card.id, stamp.turn),
        0,
    );
    if rng.int(PERCENT) >= blur_chance(stamp.level) {
        return None;
    }
    let targets = blur_targets(state, card);
    let pick = rng.int(targets.len() as i32) as usize;
    targets.get(pick).cloned()
}
