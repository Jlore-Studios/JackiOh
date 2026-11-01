//! A finished game's record for the card statistics (SPEC §9.11, R376): which cards each seat started
//! with, drew and played, and how the game ended, read off a replay of `(seed, decks, log)`.
//!
//! The state keeps no such history — `turnLog` is one turn's, and `applied` keeps only the last
//! actions' events — so this folds the log exactly as `replay::fold` does and reads the raw events
//! each step emits, which no view has redacted. It runs only where the whole game is already known:
//! the server once a match has its result, and the AI's development runs.
//!
//! What "played" means needs one bookkeeping device. `cardPlayed` is emitted for a play from hand and
//! for a cast alike (§2.4's cast on draw, R70), and only the first is the player's play. So each
//! seat's hand is followed through the events: the hand the state holds as an action begins, plus
//! every `addedToHand`, minus every card that leaves it. A `cardPlayed` naming a card in that hand is
//! a play from hand; a cast names a card that never entered it. A play countered in its announce
//! window has no `cardPlayed` (§10.5, R448), so it is none; a play step 3 replaced (R449) is followed
//! by its `transformed` to the card it resolves as, the one its `cardPlayed` names. A card a Counter
//! sends to the countering player's hand and that player's effect then casts in the same step (C #4
//! Palantir, R831001), or in the step that answers the prompt the cast opened (S #139 Wormhole asking
//! the cast's target), passes through a hand too, and is a cast all the same. Each play keeps whether
//! it was Radiant as `cardPlayed` placed it (R1147000), so step-3 Radiant (Gifted Program, Feng Shui
//! per R0983, C+ #68) counts.
//!
//! R2089000: a Created card (R943) is no dealt card, so it is left out of `opening`, `drawn` and
//! `played`, and each Created instance is credited instead to the seat that made it: the seat a
//! minting event's `creator` names (an effect that made it for the other seat), else the controller
//! of the card whose effect summoned it (R1088's `sourceId`), else the seat it went to (The Coin and
//! every other card a rule mints). It is used the first time it is played, from a hand or anywhere
//! else, cast, summoned or set by an effect, or takes a card's place on the field (a Transform), by
//! either seat, and counts once however often it is used again; the rest the seat made are its
//! never used ones. A Created card played from a hand is also kept as its player's (`createdPlayed`),
//! which the quests count beside `played`.
//!
//! Port of `packages/engine/src/gameSummary.ts` (part 5).

use indexmap::{IndexMap, IndexSet};

use crate::reduce::{begin_game, reduce};
use crate::replay::FoldArgs;
use crate::state::{CardInstance, CreateGameOptions, GameState, create_recorded_game, find_instance};
use crate::wire::{
    CounteredTo, CreatedUse, GameEvent, GameSummary, PLAYER_IDS, PerPlayer, PlayerId, SeatSummary, Zone,
};

/// Each seat's hand as instance id → catalog id, in hand order.
type Hands = PerPlayer<IndexMap<String, String>>;

struct Reading {
    hands: Hands,
    radiant: IndexSet<String>,
    /// The seat of the first `turnStarted`, and both hands at that moment; `None` until a turn has begun.
    first: Option<PlayerId>,
    opening: Option<PerPlayer<Vec<String>>>,
    drawn: PerPlayer<Vec<String>>,
    played: PerPlayer<Vec<String>>,
    played_turns: PerPlayer<Vec<i32>>,
    played_radiant: PerPlayer<Vec<bool>>,
    /// R1256000: the per-seat counts, read off the events both seats see.
    fatigue_draws: PerPlayer<i32>,
    fatigue_damage: PerPlayer<i32>,
    traps_set: PerPlayer<i32>,
    traps_fired: PerPlayer<i32>,
    position_switches: PerPlayer<i32>,
    radiant_upgrades: PerPlayer<i32>,
    /// R1256000: every instance id whose `radiantSet` already counted an upgrade.
    radiant_seen: IndexSet<String>,
    current_turn: i32,
    /// R2089000: every Created instance seen, by instance id: the seat that made it and its catalog id.
    made: IndexMap<String, (PlayerId, String)>,
    /// R2089000: the Created instances already used, a fresh face-down id (R227) beside its former one.
    used: IndexSet<String>,
    created_used: PerPlayer<Vec<CreatedUse>>,
    created_played: PerPlayer<Vec<CreatedUse>>,
    /// R831001: the cards countered into a hand by a step that ended on a prompt, whose cast the step
    /// that answers it may make.
    stolen: Vec<String>,
}

/// Instance ids of the Radiant cards each seat holds in its hand or resolving zone as an action begins.
fn radiant_of(state: &GameState, hands: &Hands) -> IndexSet<String> {
    let mut radiant = IndexSet::new();
    for player in PLAYER_IDS {
        for card in state.players[player]
            .hand
            .iter()
            .chain(state.players[player].resolving.iter())
        {
            if card.radiant && hands[player].contains_key(&card.id) {
                radiant.insert(card.id.clone());
            }
        }
    }
    radiant
}

fn mark_radiant(reading: &mut Reading, after: &GameState, id: &str) {
    if find_instance(after, id).is_some_and(|c| c.radiant) {
        reading.radiant.insert(id.to_string());
    } else {
        reading.radiant.shift_remove(id);
    }
}

/// Each seat's hand as an action begins: the cards the state holds there, and those of `following`
/// (the hands the last action ended with) that a play has taken to the resolving zone and not yet
/// placed. A prompt can hold a play there past its action (§10.5: the choices of the card step 3 put
/// in its place, R449, or the announce window, C #4 Palantir), and its `cardPlayed` comes in a later one.
fn hands_of(state: &GameState, following: Option<&Hands>) -> Hands {
    let hand = |player: PlayerId| -> IndexMap<String, String> {
        let mut cards: IndexMap<String, String> = state.players[player]
            .hand
            .iter()
            .map(|card| (card.id.clone(), card.def_id.clone()))
            .collect();
        for card in &state.players[player].resolving {
            if following.is_some_and(|following| following[player].contains_key(&card.id)) {
                cards.insert(card.id.clone(), card.def_id.clone());
            }
        }
        cards
    };
    PerPlayer::new(hand(PlayerId::P1), hand(PlayerId::P2))
}

fn per_seat<T>() -> PerPlayer<Vec<T>> {
    PerPlayer::new(Vec::new(), Vec::new())
}

/// A card leaves whichever hand holds it.
fn leave(hands: &mut Hands, instance_id: &str) {
    for player in PLAYER_IDS {
        hands[player].shift_remove(instance_id);
    }
}

/// The card one of `ids` names, as the step left it or, for one that has left the state, as the step
/// found it.
fn card_of<'a>(before: &'a GameState, after: &'a GameState, ids: &[&str]) -> Option<&'a CardInstance> {
    ids.iter()
        .find_map(|id| find_instance(after, id))
        .or_else(|| ids.iter().find_map(|id| find_instance(before, id)))
}

/// R943: whether the card is Created. One neither state holds was made and gone within the step: no
/// dealt card is, for the deal put every one of them in the state before the first action.
fn is_created(before: &GameState, after: &GameState, ids: &[&str]) -> bool {
    card_of(before, after, ids).is_none_or(|card| card.created == Some(true))
}

fn is_radiant(before: &GameState, after: &GameState, ids: &[&str]) -> bool {
    card_of(before, after, ids).is_some_and(|card| card.radiant)
}

/// R2089000: a Created instance's maker, kept the first time an event names it.
fn make(reading: &mut Reading, id: &str, creator: PlayerId, card: &str) {
    reading
        .made
        .entry(id.to_string())
        .or_insert_with(|| (creator, card.to_string()));
}

/// R2089000: a Created instance is used, counted once for the seat that made it (`maker` when no event
/// has named it before). `ids` are its id now and, for a card set face-down, the one it had (R227).
fn use_created(reading: &mut Reading, ids: &[&str], maker: PlayerId, card: &str, radiant: bool) {
    let creator = ids
        .iter()
        .find_map(|id| reading.made.get(*id))
        .map_or(maker, |(creator, _)| *creator);
    let first = !ids.iter().any(|id| reading.used.contains(*id));
    for id in ids {
        make(reading, id, creator, card);
        reading.used.insert((*id).to_string());
    }
    if first {
        reading.created_used[creator].push(CreatedUse {
            card: card.to_string(),
            turn: reading.current_turn,
            radiant,
        });
    }
}

/// One step's events, in the order they happened.
fn read(reading: &mut Reading, events: &[GameEvent], before: &GameState, after: &GameState) {
    // The cards this step countered into a hand, and those an earlier step did before it ended on a
    // prompt: a `cardPlayed` naming one is the countering player's cast (R831001), never a play from hand.
    let mut stolen: Vec<String> = std::mem::take(&mut reading.stolen);
    for (at, event) in events.iter().enumerate() {
        match event {
            GameEvent::Countered {
                instance_id,
                to: CounteredTo::Hand,
                ..
            } => {
                stolen.push(instance_id.clone());
            }
            GameEvent::TurnStarted { player, turn, .. } => {
                reading.current_turn = *turn;
                // §2.1: the mulligans have resolved and The Coin is dealt; the first turn's draw comes next.
                if reading.opening.is_none() {
                    reading.first = Some(*player);
                    // R2089000: the dealt cards alone; The Coin is Created.
                    let dealt = |hand: &IndexMap<String, String>| -> Vec<String> {
                        hand.iter()
                            .filter(|(id, _)| !is_created(before, after, &[id.as_str()]))
                            .map(|(_, def_id)| def_id.clone())
                            .collect()
                    };
                    reading.opening =
                        Some(PerPlayer::new(dealt(&reading.hands.p1), dealt(&reading.hands.p2)));
                }
            }
            GameEvent::Drawn {
                player,
                instance_id,
                def_id,
                ..
            } => {
                // §2.4: a draw that reached the hand is followed at once by its `addedToHand`; a burned
                // card's `burned` or a cast's `cardPlayed` follows instead. The opening hand's draws are the
                // opening hand's, not draws of the game.
                let reached_hand = matches!(
                    events.get(at + 1),
                    Some(GameEvent::AddedToHand { instance_id: next_id, .. }) if next_id == instance_id
                );
                if reading.opening.is_some() && reached_hand && !is_created(before, after, &[instance_id]) {
                    reading.drawn[*player].push(def_id.clone());
                }
            }
            GameEvent::AddedToHand {
                player,
                instance_id,
                def_id,
                creator,
            } => {
                reading.hands[*player].insert(instance_id.clone(), def_id.clone());
                mark_radiant(reading, after, instance_id);
                if is_created(before, after, &[instance_id]) {
                    make(reading, instance_id, creator.unwrap_or(*player), def_id);
                }
            }
            GameEvent::ShuffledIn {
                player,
                instance_id,
                def_id,
                creator,
                ..
            } => {
                leave(&mut reading.hands, instance_id);
                if is_created(before, after, &[instance_id]) {
                    make(reading, instance_id, creator.unwrap_or(*player), def_id);
                }
            }
            GameEvent::MarketBought {
                player,
                instance_id,
                def_id,
                ..
            } => {
                if is_created(before, after, &[instance_id]) {
                    make(reading, instance_id, *player, def_id);
                }
            }
            GameEvent::Summoned {
                player,
                instance_id,
                def_id,
                former_id,
                source_id,
                creator,
                ..
            } => {
                let ids: Vec<&str> = std::iter::once(instance_id.as_str())
                    .chain(former_id.as_deref())
                    .collect();
                if is_created(before, after, &ids) {
                    // The seat whose effect summoned it for the other side, else the controller of the
                    // card whose effect summoned it (R1088), as it stood when the step began.
                    let maker = creator.unwrap_or_else(|| {
                        source_id
                            .as_deref()
                            .and_then(|source| {
                                find_instance(before, source).or_else(|| find_instance(after, source))
                            })
                            .map_or(*player, |source| source.controller)
                    });
                    let radiant = is_radiant(before, after, &ids);
                    use_created(reading, &ids, maker, def_id, radiant);
                }
            }
            GameEvent::Fatigue { player, amount, .. } => {
                // R1256000: one draw for the seat that took it, and the nominal pre-Armor hit.
                reading.fatigue_draws[*player] += 1;
                reading.fatigue_damage[*player] += *amount;
            }
            GameEvent::TrapFired { controller, .. } => {
                // R1256000: the trap's controller, whoever's effect fired it.
                reading.traps_fired[*controller] += 1;
            }
            GameEvent::PositionSwitched { instance_id, .. } => {
                // R1256000: the unit's controller; unresolvable (the unit ceased) is skipped.
                if let Some(card) = find_instance(after, instance_id) {
                    reading.position_switches[card.controller] += 1;
                }
            }
            GameEvent::RadiantSet { instance_id, .. } => {
                reading.radiant.insert(instance_id.clone());
                // R1256000: an instance's first cue counts for its controller when the step turned it
                // Radiant: not Radiant before the step, Radiant after. The engine also cues a card that
                // was Radiant already (R177), and a hidden card's re-cues dedupe to one.
                if reading.radiant_seen.insert(instance_id.clone())
                    && find_instance(before, instance_id).is_none_or(|card| !card.radiant)
                    && let Some(card) = find_instance(after, instance_id)
                    && card.radiant
                {
                    reading.radiant_upgrades[card.controller] += 1;
                }
            }
            GameEvent::Deradianted { instance_id, .. } => {
                reading.radiant.shift_remove(instance_id);
            }
            GameEvent::CardPlayed {
                player,
                instance_id,
                def_id,
                former_id,
                ..
            } => {
                // R227: a Trap set face-down took a fresh id; the hand knew it by the old one.
                let id = former_id.as_ref().unwrap_or(instance_id);
                let from_hand = reading.hands[*player].contains_key(id);
                let ids = [instance_id.as_str(), id.as_str()];
                let created = is_created(before, after, &ids);
                if created {
                    // R2089000: a play from anywhere and a cast use it alike. A card no event named
                    // before was made in this step for the seat that casts it.
                    let radiant = if from_hand {
                        reading.radiant.contains(id)
                    } else {
                        is_radiant(before, after, &ids)
                    };
                    use_created(reading, &ids, *player, def_id, radiant);
                }
                if !from_hand {
                    continue;
                }
                if stolen.iter().any(|countered| countered == id) {
                    leave(&mut reading.hands, id);
                    continue;
                }
                let turn = reading.current_turn;
                if created {
                    reading.created_played[*player].push(CreatedUse {
                        card: def_id.clone(),
                        turn,
                        radiant: reading.radiant.contains(id),
                    });
                } else {
                    reading.played[*player].push(def_id.clone());
                    reading.played_turns[*player].push(turn);
                    reading.played_radiant[*player].push(reading.radiant.contains(id));
                }
                // R1256000, R227: a play that put the card face-down set a Trap.
                if former_id.is_some() {
                    reading.traps_set[*player] += 1;
                }
                reading.hands[*player].shift_remove(id);
            }
            GameEvent::Transformed {
                instance_id,
                to_def_id,
                new_instance_id,
                creator,
                ..
            } => {
                for player in PLAYER_IDS {
                    let hand = &mut reading.hands[player];
                    if !hand.contains_key(instance_id) {
                        continue;
                    }
                    hand.shift_remove(instance_id);
                    hand.insert(new_instance_id.clone(), to_def_id.clone());
                    mark_radiant(reading, after, new_instance_id);
                }
                // R2089000: a Transform's new card is Created (a Vanilla keeps the card it changes).
                // Taking a card's place on the field puts it there, which uses it.
                let new = [new_instance_id.as_str()];
                if new_instance_id != instance_id
                    && is_created(before, after, &new)
                    && let Some(card) = card_of(before, after, &new)
                {
                    let maker = creator.unwrap_or(card.controller);
                    if matches!(card.zone, Zone::Field { .. }) {
                        let radiant = card.radiant;
                        use_created(reading, &new, maker, to_def_id, radiant);
                    } else {
                        make(reading, new_instance_id, maker, to_def_id);
                    }
                }
            }
            GameEvent::Fused { instance_ids, .. } => {
                for id in instance_ids {
                    leave(&mut reading.hands, id);
                }
            }
            GameEvent::Discarded { instance_id, .. }
            | GameEvent::Exiled { instance_id, .. }
            | GameEvent::Bounced { instance_id, .. }
            | GameEvent::EnteredGraveyard { instance_id, .. } => {
                leave(&mut reading.hands, instance_id);
            }
            _ => {}
        }
    }
    // R831001: a cast that asks a choice of its caster finishes in the step that answers it.
    if after.pending.is_some() {
        reading.stolen = stolen;
    }
}

fn seat_summary(reading: &Reading, deck: Vec<String>, player: PlayerId) -> SeatSummary {
    fn listed<T: Clone>(list: &[T]) -> Option<Vec<T>> {
        if list.is_empty() {
            None
        } else {
            Some(list.to_vec())
        }
    }
    // R2089000: the Created cards the seat made that no use reached, one per instance; a card set
    // face-down is one instance under both its ids, which `used` holds together.
    let unused: Vec<String> = reading
        .made
        .iter()
        .filter(|(id, (creator, _))| *creator == player && !reading.used.contains(*id))
        .map(|(_, (_, card))| card.clone())
        .collect();
    SeatSummary {
        deck,
        opening: reading
            .opening
            .as_ref()
            .map(|opening| opening[player].clone())
            .unwrap_or_default(),
        drawn: reading.drawn[player].clone(),
        played: reading.played[player].clone(),
        played_turns: listed(&reading.played_turns[player]),
        played_radiant: listed(&reading.played_radiant[player]),
        // R1256000: a fresh summary always knows the counts; `None` stays reserved for
        // records written before the change.
        fatigue_draws: Some(reading.fatigue_draws[player]),
        fatigue_damage: Some(reading.fatigue_damage[player]),
        traps_set: Some(reading.traps_set[player]),
        traps_fired: Some(reading.traps_fired[player]),
        position_switches: Some(reading.position_switches[player]),
        radiant_upgrades: Some(reading.radiant_upgrades[player]),
        created_used: listed(&reading.created_used[player]),
        created_unused: listed(&unused),
        created_played: listed(&reading.created_played[player]),
    }
}

/// R376: reads a game's record off its steps, one `(before, events, after)` at a time, as
/// `summarize_game` folds a log; a test reads a `scenario()`'s verbs through it the same way.
pub struct SummaryReader {
    reading: Reading,
}

impl SummaryReader {
    /// A reader of the game that begins at `start`.
    pub fn new(start: &GameState) -> SummaryReader {
        let hands = hands_of(start, None);
        let radiant = radiant_of(start, &hands);
        fn zero() -> PerPlayer<i32> {
            PerPlayer::new(0, 0)
        }
        SummaryReader {
            reading: Reading {
                hands,
                radiant,
                first: None,
                opening: None,
                drawn: per_seat(),
                played: per_seat(),
                played_turns: per_seat(),
                played_radiant: per_seat(),
                fatigue_draws: zero(),
                fatigue_damage: zero(),
                traps_set: zero(),
                traps_fired: zero(),
                position_switches: zero(),
                radiant_upgrades: zero(),
                radiant_seen: IndexSet::new(),
                current_turn: start.turn.max(1),
                made: IndexMap::new(),
                used: IndexSet::new(),
                created_used: per_seat(),
                created_played: per_seat(),
                stolen: Vec::new(),
            },
        }
    }

    /// One step: the state as it began, the events it emitted, and the state it left.
    pub fn step(&mut self, before: &GameState, events: &[GameEvent], after: &GameState) {
        // The state's own hands as the step begins, so nothing the events do not spell out (a card
        // whose control changed, say) carries from one action into the next, but a play still in flight.
        self.reading.hands = hands_of(before, Some(&self.reading.hands));
        self.reading.radiant = radiant_of(before, &self.reading.hands);
        read(&mut self.reading, events, before, after);
    }

    /// The seat's summary so far, `deck` its decklist.
    pub fn seat(&self, player: PlayerId, deck: Vec<String>) -> SeatSummary {
        seat_summary(&self.reading, deck, player)
    }
}

/// R376: what each seat's cards did in a finished game, and how it ended. The fold is
/// `replay::fold`'s, so an action the log holds and the engine refuses is skipped as it is there.
/// `None` when the log leaves the game without a result: a record is only ever made of a finished
/// game.
pub fn summarize_game(input: &FoldArgs) -> Option<GameSummary> {
    // TS hands `createGame` no `dealt` here (only `fold` does); kept so, since it changes no event.
    let start = create_recorded_game(&CreateGameOptions {
        seed: input.seed.clone(),
        decks: input.decks.clone(),
        catalog: input.catalog.clone(),
        handicaps: input.handicaps.clone(),
        dealt: None,
        // R417, R678: setup like the decks, so the fold is the game that was played.
        last_boards: input.last_boards.clone(),
        glitch_boards: input.glitch_boards.clone(),
        // R2702000: the marks too, or the fold deals other cards.
        deck_marks: input.deck_marks.clone(),
        ..Default::default()
    });
    let mut reader = SummaryReader::new(&start);

    let begun = begin_game(&start);
    reader.step(&start, &begun.events, &begun.state);
    let mut state = begun.state;

    for action in &input.log {
        let result = reduce(&state, action);
        if result.error.is_some() {
            continue;
        }
        reader.step(&state, &result.events, &result.state);
        state = result.state;
    }

    let outcome = state.result?;
    Some(GameSummary {
        // §2.1: Player 1 takes the first turn. A game that ended before any turn began names it too.
        first: reader.reading.first.unwrap_or(PLAYER_IDS[0]),
        winner: outcome.winner,
        reason: outcome.reason,
        turns: state.turn,
        seats: PerPlayer::new(
            reader.seat(PlayerId::P1, input.decks.0.clone()),
            reader.seat(PlayerId::P2, input.decks.1.clone()),
        ),
    })
}

#[cfg(all(test, feature = "testkit"))]
mod tests {
    use super::*;
    use crate::testkit::{register_catalog, scenario};
    use serde_json::json;

    fn reading() -> Reading {
        let zero = || PerPlayer::new(0, 0);
        Reading {
            hands: PerPlayer::new(IndexMap::new(), IndexMap::new()),
            radiant: IndexSet::new(),
            first: None,
            opening: None,
            drawn: per_seat(),
            played: per_seat(),
            played_turns: per_seat(),
            played_radiant: per_seat(),
            fatigue_draws: zero(),
            fatigue_damage: zero(),
            traps_set: zero(),
            traps_fired: zero(),
            position_switches: zero(),
            radiant_upgrades: zero(),
            radiant_seen: IndexSet::new(),
            current_turn: 1,
            made: IndexMap::new(),
            used: IndexSet::new(),
            created_used: per_seat(),
            created_played: per_seat(),
            stolen: Vec::new(),
        }
    }

    /// core-008 "Vanilla": a hand card in a game whose catalog the engine's own tests install.
    fn hand_card(radiant: bool) -> GameState {
        if crate::testkit::catalog_override().is_none() {
            let defs = serde_json::from_str(include_str!("../../cards/catalog.json"))
                .expect("crates/cards/catalog.json parses as CardDefs");
            register_catalog(defs);
        }
        scenario(json!({ "p1": { "hand": [{ "def": "core-008", "radiant": radiant }] } }))
            .state()
            .clone()
    }

    fn cue(state: &GameState) -> Vec<GameEvent> {
        let card = &state.players[PlayerId::P1].hand[0];
        vec![GameEvent::RadiantSet {
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            zone: card.zone.clone(),
        }]
    }

    #[test]
    fn r1256000_a_cue_for_a_card_radiant_before_the_step_is_no_upgrade() {
        let already = hand_card(true);
        let mut reading = reading();
        read(&mut reading, &cue(&already), &already, &already);
        assert_eq!(reading.radiant_upgrades.p1, 0);
    }

    #[test]
    fn r1256000_a_cue_for_a_card_the_step_made_radiant_is_one_upgrade() {
        let before = hand_card(false);
        let after = hand_card(true);
        let mut reading = reading();
        // The id is the scenario's first instance in both; the cue is cued twice, as a hidden card's is.
        let events = cue(&after);
        assert_eq!(
            before.players[PlayerId::P1].hand[0].id,
            after.players[PlayerId::P1].hand[0].id
        );
        read(&mut reading, &events, &before, &after);
        read(&mut reading, &events, &before, &after);
        assert_eq!(reading.radiant_upgrades.p1, 1);
    }
}
