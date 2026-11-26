//! A finished game's record for the card statistics (SPEC §9.11, R376): which cards each seat started
//! with, drew and played, and how the game ended, read off a replay of `(seed, decks, log)`. It reads
//! raw events no view has redacted, so it runs only where the whole game is already known.
//!
//! `cardPlayed` is emitted for a play from hand and a cast alike (§2.4's cast on draw, R70), so each seat's
//! hand is followed through the events, and only a `cardPlayed` naming a card in it is a play. A play
//! countered in its announce window has none (§10.5, R448); a play step 3 replaced (R449) is followed by
//! its `transformed`. A card a Counter sends to its player's hand that their effect then casts (C #4
//! Palantir, R831001; S #139 Wormhole) is a cast all the same. Each play keeps whether `cardPlayed` placed
//! it Radiant (R1147000), so step-3 Radiant (R0983, C+ #68) counts.
//!
//! R2089000: a Created card (R943) is left out of `opening`, `drawn` and `played` and credited to the seat
//! that made it: a minting event's `creator`, else the controller of the card whose effect summoned it
//! (R1088's `sourceId`), else the seat it went to (The Coin). It is used once, the first time it is
//! played, cast, summoned, set or takes a card's place; a hand play is its player's `createdPlayed`.
//!
//! R4279000: a dealt card an effect casts or summons from its seat's own deck or hand, rather than a play
//! from the hand, is that seat's `castOrSummoned`: cast on its draw (§2.4, the draw's `cardPlayed` says
//! `onDraw`) or from the deck (§6.3), or summoned from the deck or the hand (M #87 Tatches the Totem, a
//! Recruit). A play's own `summoned` follows its `cardPlayed` and is that play, not another.
//!
//! R4672000: each play keeps its variant: the X and the Embiggen its `cardPlayed` names, the choices it
//! declared (`cardPlayed.modes`), and the `mode` and `number` prompts its own script asks between its
//! `cardPlayed` and its `cardResolved` that the seat which played it answers, read off the answering
//! action (`step`'s `action`).
//! R4672001: each Created card names the card whose effect made it (`GameState.makers`). R4672002: a
//! card's `discover` or `pick` prompt is a pick, its offer and its answer as catalog ids.
//!
//! R4707000: each play's variant keeps what it rolled, read off the public reports: a `chaosRolled`'s
//! places in its card's table, a `rolled` batch's coins, and its dice by the faces their card checks for
//! (or by face, at most `ROLL_SPLIT_MAX_SIDES` sides). A roll joins the last entry the play of the card
//! that rolled it listed, whenever it comes (a Trap's coin as it fires, a unit's end-of-turn flip), and a
//! Call to Chaos a Recursion cast joins the play that began its chain (R28).

use indexmap::{IndexMap, IndexSet};

use crate::config::ROLL_SPLIT_MAX_SIDES;
use crate::reduce::{begin_game, reduce};
use crate::replay::FoldArgs;
use crate::state::{CardInstance, CreateGameOptions, GameState, create_recorded_game, find_instance};
use crate::wire::{
    Action, ActionBody, CardPick, CounteredTo, CreatedUse, GameEvent, GameSummary, MadeBy, PLAYER_IDS,
    PerPlayer, PlayVariant, PlayerId, PromptKind, RollKind, RolledOutcome, SeatSummary, Selection, Tag, Zone,
};

/// Each seat's hand as instance id → catalog id, in hand order.
type Hands = PerPlayer<IndexMap<String, String>>;

/// R4279000: each seat's library by instance id.
type Decks = PerPlayer<IndexSet<String>>;

/// R2089000: a Created instance's maker: the seat it is credited to and its catalog id. R4672001: and the
/// card whose effect made it, `None` for one a rule made.
struct Made {
    creator: PlayerId,
    card: String,
    source: Option<MadeBy>,
}

/// R4672000: a list entry a play pushed, whose variant the choices its card asks as it resolves extend.
#[derive(Clone, Copy)]
enum Slot {
    Played(PlayerId, usize),
    Cast(PlayerId, usize),
    CreatedUsed(PlayerId, usize),
    CreatedPlayed(PlayerId, usize),
}

/// R4672000: a play between its `cardPlayed` and its `cardResolved`: the seat that played it, its
/// entries, and the steps of its own script already answered, so an Echo repeat asking one again adds
/// nothing.
struct OpenPlay {
    player: PlayerId,
    /// R4707000: the card played, which a Recursion's cast names as its maker.
    def_id: String,
    slots: Vec<Slot>,
    answered: IndexSet<String>,
}

struct Reading {
    hands: Hands,
    /// R4279000: each seat's library as an action begins, with the cards a cast took from it that an
    /// earlier action left resolving (`decks_of`).
    decks: Decks,
    radiant: IndexSet<String>,
    /// The seat of the first `turnStarted`, and both hands at that moment; `None` until a turn has begun.
    first: Option<PlayerId>,
    opening: Option<PerPlayer<Vec<String>>>,
    drawn: PerPlayer<Vec<String>>,
    played: PerPlayer<Vec<String>>,
    played_turns: PerPlayer<Vec<i32>>,
    played_radiant: PerPlayer<Vec<bool>>,
    /// R5054000: the dealt cards each seat held Radiant in its hand from its opening hand on, each once.
    held_radiant: PerPlayer<IndexSet<String>>,
    /// R4672000: each play's variant, beside `played`.
    played_variants: PerPlayer<Vec<Option<PlayVariant>>>,
    /// R4672000: the plays still resolving, by the instance id their card's prompts name.
    open: IndexMap<String, OpenPlay>,
    /// R4707000: the entry each played card's rolls join, by its instance id: its play's last.
    rollers: IndexMap<String, Slot>,
    /// R4707000: a Call to Chaos a Recursion cast, by instance id, and the play that began its chain.
    chains: IndexMap<String, String>,
    /// R4672002: each seat's Discover-like picks.
    picks: PerPlayer<Vec<CardPick>>,
    /// R1256000: the per-seat counts, read off the events both seats see.
    fatigue_draws: PerPlayer<i32>,
    fatigue_damage: PerPlayer<i32>,
    traps_set: PerPlayer<i32>,
    traps_fired: PerPlayer<i32>,
    position_switches: PerPlayer<i32>,
    radiant_upgrades: PerPlayer<i32>,
    /// R3335001: the Starship Pieces that fused into each seat's Starship, and the Starships it launched.
    starship_pieces_fused: PerPlayer<i32>,
    starships_launched: PerPlayer<i32>,
    /// R1256000: every instance id whose `radiantSet` already counted an upgrade.
    radiant_seen: IndexSet<String>,
    current_turn: i32,
    /// R2089000: every Created instance seen, by instance id: the seat that made it and its catalog id.
    made: IndexMap<String, Made>,
    /// R2089000: the Created instances already used, a fresh face-down id (R227) beside its former one.
    used: IndexSet<String>,
    created_used: PerPlayer<Vec<CreatedUse>>,
    created_played: PerPlayer<Vec<CreatedUse>>,
    /// R4279000: the dealt cards an effect cast or summoned from each seat's own deck or hand.
    cast_or_summoned: PerPlayer<Vec<CreatedUse>>,
    /// R831001: the cards countered into a hand by a step that ended on a prompt, whose cast the step
    /// that answers it may make.
    stolen: Vec<String>,
}

impl Reading {
    /// A reading that has read no step yet, from the game's hands, libraries and Radiant cards at
    /// `current_turn`.
    fn new(hands: Hands, decks: Decks, radiant: IndexSet<String>, current_turn: i32) -> Reading {
        let zero = || PerPlayer::new(0, 0);
        Reading {
            hands,
            decks,
            radiant,
            first: None,
            opening: None,
            drawn: per_seat(),
            played: per_seat(),
            played_turns: per_seat(),
            played_radiant: per_seat(),
            held_radiant: PerPlayer::new(IndexSet::new(), IndexSet::new()),
            played_variants: per_seat(),
            open: IndexMap::new(),
            rollers: IndexMap::new(),
            chains: IndexMap::new(),
            picks: per_seat(),
            fatigue_draws: zero(),
            fatigue_damage: zero(),
            traps_set: zero(),
            traps_fired: zero(),
            position_switches: zero(),
            radiant_upgrades: zero(),
            starship_pieces_fused: zero(),
            starships_launched: zero(),
            radiant_seen: IndexSet::new(),
            current_turn,
            made: IndexMap::new(),
            used: IndexSet::new(),
            created_used: per_seat(),
            created_played: per_seat(),
            cast_or_summoned: per_seat(),
            stolen: Vec::new(),
        }
    }
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

/// R5054000: notes the card `id` names as held Radiant by the seat whose hand holds it, when it is Radiant,
/// dealt (a Created card is never drawn, R2089000) and the opening hand has been dealt (a card mulliganed
/// away was never seen).
fn hold_radiant(reading: &mut Reading, before: &GameState, after: &GameState, id: &str) {
    if reading.opening.is_none() || !reading.radiant.contains(id) || is_created(before, after, &[id]) {
        return;
    }
    for player in PLAYER_IDS {
        if let Some(def_id) = reading.hands[player].get(id) {
            let def_id = def_id.clone();
            reading.held_radiant[player].insert(def_id);
        }
    }
}

/// Each seat's hand as an action begins: the state's, plus the cards of `following` (the last action's
/// hands) a play took to the resolving zone and has not placed, since a prompt can hold one there past
/// its action (§10.5: R449's step-3 choices, or C #4 Palantir's announce window).
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

/// R4279000: each seat's library as an action begins, plus the cards of `following` (the last action's
/// decks) a cast took to the resolving zone and has not announced, since a cast's choices come before its
/// `cardPlayed` (R453) and a prompt can hold them past its action, as `hands_of` keeps a hand's.
fn decks_of(state: &GameState, following: Option<&Decks>) -> Decks {
    let deck = |player: PlayerId| -> IndexSet<String> {
        let mut ids: IndexSet<String> = state.players[player]
            .library
            .iter()
            .map(|card| card.id.clone())
            .collect();
        for card in &state.players[player].resolving {
            if following.is_some_and(|following| following[player].contains(&card.id)) {
                ids.insert(card.id.clone());
            }
        }
        ids
    };
    PerPlayer::new(deck(PlayerId::P1), deck(PlayerId::P2))
}

/// R4279000: a dealt card of `player`'s an effect cast or summoned from its deck or hand, at the index it
/// is listed at. R4672000: a cast keeps its variant.
fn cast_or_summon(
    reading: &mut Reading,
    player: PlayerId,
    card: &str,
    radiant: bool,
    variant: Option<PlayVariant>,
) -> usize {
    let cast = &mut reading.cast_or_summoned[player];
    cast.push(CreatedUse {
        card: card.to_string(),
        turn: reading.current_turn,
        radiant,
        variant,
        source: None,
    });
    cast.len() - 1
}

/// R4672000: what a `cardPlayed` says was chosen; `None` when its card offered no choice.
fn play_variant(modes: Option<&[String]>, x: Option<i32>, embiggened: Option<bool>) -> Option<PlayVariant> {
    let modes = modes.map(<[String]>::to_vec).unwrap_or_default();
    if modes.is_empty() && x.is_none() && embiggened.is_none() {
        return None;
    }
    Some(PlayVariant {
        modes,
        x,
        embiggened,
        ..PlayVariant::default()
    })
}

/// R4672000: the variant a list entry holds.
fn variant_mut(reading: &mut Reading, slot: Slot) -> Option<&mut Option<PlayVariant>> {
    match slot {
        Slot::Played(player, at) => reading.played_variants[player].get_mut(at),
        Slot::Cast(player, at) => reading.cast_or_summoned[player]
            .get_mut(at)
            .map(|cast| &mut cast.variant),
        Slot::CreatedUsed(player, at) => reading.created_used[player]
            .get_mut(at)
            .map(|used| &mut used.variant),
        Slot::CreatedPlayed(player, at) => reading.created_played[player]
            .get_mut(at)
            .map(|played| &mut played.variant),
    }
}

/// R4672000: a play's entries, open until its `cardResolved`; a play that listed none is no open play.
/// R4707000: its card's rolls join its last entry from here on, as long as the card keeps its id.
fn open(reading: &mut Reading, instance_id: &str, player: PlayerId, def_id: &str, slots: Vec<Slot>) {
    match slots.last() {
        Some(slot) => {
            reading.rollers.insert(instance_id.to_string(), *slot);
        }
        None => {
            reading.rollers.shift_remove(instance_id);
        }
    }
    if slots.is_empty() {
        reading.open.shift_remove(instance_id);
    } else {
        reading.open.insert(
            instance_id.to_string(),
            OpenPlay {
                player,
                def_id: def_id.to_string(),
                slots,
                answered: IndexSet::new(),
            },
        );
    }
}

/// R4707000: whether a catalog card is of the Call to Chaos family (§10.7's recursion pool).
fn is_call_to_chaos(state: &GameState, def_id: &str) -> bool {
    crate::catalog::find_def(Some(state), def_id).is_some_and(|def| def.tags.contains(&Tag::CallToChaos))
}

/// R4707000, R28: the play whose chain a Call to Chaos cast by a Call to Chaos's Recursion is part of:
/// the innermost play still resolving of the card that made it, and that play's own chain's first when it
/// is a cast itself. `None` for any other cast.
fn chain_root(reading: &Reading, state: &GameState, def_id: &str, maker: Option<&MadeBy>) -> Option<String> {
    let maker = maker?;
    if !is_call_to_chaos(state, def_id) || !is_call_to_chaos(state, &maker.card) {
        return None;
    }
    let (parent, _) = reading
        .open
        .iter()
        .rev()
        .find(|(_, play)| play.def_id == maker.card)?;
    Some(reading.chains.get(parent).unwrap_or(parent).clone())
}

/// R4707000: a roll the card `instance_id` made joins its play's entry, or its chain's first play's; a
/// card no play the record keeps listed (one an effect summoned or set) records none.
fn credit_roll(reading: &mut Reading, instance_id: &str, roll: RolledOutcome) {
    let roller = reading
        .chains
        .get(instance_id)
        .map_or(instance_id, String::as_str);
    let Some(slot) = reading.rollers.get(roller).copied() else {
        return;
    };
    if let Some(variant) = variant_mut(reading, slot) {
        variant.get_or_insert_with(PlayVariant::default).rolled.push(roll);
    }
}

/// R4707000: what a `rolled` batch came up with, as the record keeps it: its coins' heads, or its dice by
/// the faces their card checks for, else by face when they have at most `ROLL_SPLIT_MAX_SIDES`; a larger
/// die its card checks no face of is no outcome of a fixed list.
fn rolled_outcome(
    def_id: &str,
    kind: RollKind,
    sides: i32,
    rolls: &[crate::wire::RollResult],
    hit: Option<crate::wire::RollHit>,
) -> Option<RolledOutcome> {
    let count = |n: usize| i32::try_from(n).unwrap_or(i32::MAX);
    // Every coin or die of a batch is thrown at one Lucky, 1 + X draws each (R1440).
    let lucky = rolls
        .first()
        .map_or(0, |roll| count(roll.faces.len()).saturating_sub(1));
    match kind {
        RollKind::Coin => Some(RolledOutcome::Coin {
            card: def_id.to_string(),
            coins: count(rolls.len()),
            heads: count(rolls.iter().filter(|roll| roll.heads()).count()),
            lucky,
        }),
        RollKind::Die if hit.is_some() || sides <= ROLL_SPLIT_MAX_SIDES => Some(RolledOutcome::Die {
            card: def_id.to_string(),
            sides,
            lucky,
            hit,
            kept: rolls.iter().map(crate::wire::RollResult::kept_face).collect(),
        }),
        RollKind::Die => None,
    }
}

/// R4672000, R4672002: an answer to a prompt a card's own script asked (`prompts::RESUME_HOOK`), read
/// before the step's events. A `mode` or `number` answer by the seat that played it joins the variant
/// of the play still resolving whose card asked it, each of its steps once; a `discover` or `pick` answer is a pick of the seat the
/// prompt's step runs as, when every option it offered is a card.
fn answer(reading: &mut Reading, before: &GameState, action: &Action) {
    let ActionBody::Answer { selection, .. } = &action.body else {
        return;
    };
    let Some(pending) = before.pending.as_ref() else {
        return;
    };
    let resume = &pending.resume;
    if resume.hook != crate::prompts::RESUME_HOOK {
        return;
    }
    // The options picked, in the prompt's (the card's) order.
    let chosen: Vec<&Selection> = pending
        .options
        .iter()
        .map(|option| &option.selection)
        .filter(|option| selection.contains(option))
        .collect();
    match pending.kind {
        PromptKind::Mode | PromptKind::Number => {
            let Some(play) = resume
                .instance_id
                .as_ref()
                .and_then(|id| reading.open.get_mut(id))
            else {
                return;
            };
            // A question another seat answers (Classic #8 Pickle's) is no choice of the play's.
            if pending.player_id != play.player {
                return;
            }
            if !play.answered.insert(resume.step.clone()) {
                return;
            }
            let modes: Vec<String> = chosen
                .iter()
                .filter_map(|option| match option {
                    Selection::Mode { option } => Some(option.clone()),
                    _ => None,
                })
                .collect();
            if modes.is_empty() {
                return;
            }
            for slot in play.slots.clone() {
                if let Some(variant) = variant_mut(reading, slot) {
                    variant
                        .get_or_insert_with(PlayVariant::default)
                        .modes
                        .extend(modes.iter().cloned());
                }
            }
        }
        PromptKind::Discover | PromptKind::Pick => {
            if resume.def_id.is_empty() {
                return;
            }
            // A definition offered by its catalog id, or an instance by its card's; an index or a code
            // offer (R247, R1183005) names no card, so it is no pick.
            let card = |option: &Selection| -> Option<String> {
                match option {
                    Selection::Mode { option } => {
                        crate::catalog::find_def(Some(before), option).map(|def| def.id.clone())
                    }
                    Selection::Instance { instance_id } => {
                        find_instance(before, instance_id).map(|card| card.def_id.clone())
                    }
                    _ => None,
                }
            };
            let Some(offered) = pending
                .options
                .iter()
                .map(|option| card(&option.selection))
                .collect::<Option<Vec<String>>>()
            else {
                return;
            };
            let picked: Vec<String> = chosen.iter().filter_map(|option| card(option)).collect();
            reading.picks[crate::prompts::prompt_owner_of(pending)].push(CardPick {
                source: MadeBy {
                    card: resume.def_id.clone(),
                    radiant: resume.radiant,
                },
                offered,
                picked,
            });
        }
        _ => {}
    }
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

/// The card one of `ids` names, as the step left it, else as the step found it.
fn card_of<'a>(before: &'a GameState, after: &'a GameState, ids: &[&str]) -> Option<&'a CardInstance> {
    ids.iter()
        .find_map(|id| find_instance(after, id))
        .or_else(|| ids.iter().find_map(|id| find_instance(before, id)))
}

/// R943: whether the card is Created. One neither state holds was made and gone within the step: no dealt
/// card is, for the deal put every one of them in the state before the first action.
fn is_created(before: &GameState, after: &GameState, ids: &[&str]) -> bool {
    card_of(before, after, ids).is_none_or(|card| card.created == Some(true))
}

fn is_radiant(before: &GameState, after: &GameState, ids: &[&str]) -> bool {
    card_of(before, after, ids).is_some_and(|card| card.radiant)
}

/// R4672001: the card whose effect made the Created card one of `ids` names, as the step left the
/// engine's record of it, else as the step found it.
fn maker_of(before: &GameState, after: &GameState, ids: &[&str]) -> Option<MadeBy> {
    ids.iter()
        .find_map(|id| after.makers.get(*id))
        .or_else(|| ids.iter().find_map(|id| before.makers.get(*id)))
        .cloned()
}

/// R2089000: a Created instance's maker, kept the first time an event names it.
fn make(reading: &mut Reading, id: &str, creator: PlayerId, card: &str, source: Option<MadeBy>) {
    reading.made.entry(id.to_string()).or_insert_with(|| Made {
        creator,
        card: card.to_string(),
        source,
    });
}

/// R2089000: a Created instance is used, counted once for the seat that made it (`maker` when no event
/// has named it before). `ids` are its id now and, for a card set face-down, the one it had (R227).
/// R4672001: its entry names the card that made it; R4672000: and keeps `variant` when `maker`, the seat
/// that played it, made it. Hands back the entry it listed.
fn use_created(
    reading: &mut Reading,
    ids: &[&str],
    maker: PlayerId,
    card: &str,
    radiant: bool,
    source: Option<MadeBy>,
    variant: Option<PlayVariant>,
) -> Option<(PlayerId, usize)> {
    let (creator, source) = match ids.iter().find_map(|id| reading.made.get(*id)) {
        Some(made) => (made.creator, made.source.clone().or(source)),
        None => (maker, source),
    };
    let first = !ids.iter().any(|id| reading.used.contains(*id));
    for id in ids {
        make(reading, id, creator, card, source.clone());
        reading.used.insert((*id).to_string());
    }
    if !first {
        return None;
    }
    let used = &mut reading.created_used[creator];
    used.push(CreatedUse {
        card: card.to_string(),
        turn: reading.current_turn,
        radiant,
        variant: variant.filter(|_| creator == maker),
        source,
    });
    Some((creator, used.len() - 1))
}

/// One step's events, in the order they happened.
fn read(reading: &mut Reading, events: &[GameEvent], before: &GameState, after: &GameState) {
    // The cards this step countered into a hand, and those an earlier step did before it ended on a
    // prompt: a `cardPlayed` naming one is the countering player's cast (R831001), never a play from hand.
    let mut stolen: Vec<String> = std::mem::take(&mut reading.stolen);
    // R4279000: the cards a `cardPlayed` of this step named, by both ids, whose `summoned` is that play's.
    let mut announced: IndexSet<String> = IndexSet::new();
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
                    // R5054000: a card Radiant in the opening hand was held Radiant from the first turn on.
                    let held: Vec<String> = PLAYER_IDS
                        .iter()
                        .flat_map(|&player| reading.hands[player].keys().cloned().collect::<Vec<_>>())
                        .collect();
                    for id in held {
                        hold_radiant(reading, before, after, &id);
                    }
                }
            }
            GameEvent::Drawn {
                player,
                instance_id,
                def_id,
                ..
            } => {
                // §2.4: a draw that reached the hand is followed at once by its `addedToHand` (a burn's
                // `burned` or a cast's `cardPlayed` instead); the opening hand's are no draws of the game.
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
                hold_radiant(reading, before, after, instance_id);
                if is_created(before, after, &[instance_id]) {
                    let source = maker_of(before, after, &[instance_id]);
                    make(reading, instance_id, creator.unwrap_or(*player), def_id, source);
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
                reading.decks[*player].insert(instance_id.clone());
                if is_created(before, after, &[instance_id]) {
                    let source = maker_of(before, after, &[instance_id]);
                    make(reading, instance_id, creator.unwrap_or(*player), def_id, source);
                }
            }
            GameEvent::MarketBought {
                player,
                instance_id,
                def_id,
                ..
            } => {
                if is_created(before, after, &[instance_id]) {
                    let source = maker_of(before, after, &[instance_id]);
                    make(reading, instance_id, *player, def_id, source);
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
                    let source = maker_of(before, after, &ids);
                    use_created(reading, &ids, maker, def_id, radiant, source, None);
                } else if !ids.iter().any(|id| announced.contains(*id)) {
                    // R4279000: a dealt card put onto its seat's field from that seat's deck or hand, by
                    // its own trigger (M #87) or another card's effect (a Recruit); a play's step-4
                    // summon is that play.
                    let id = former_id.as_deref().unwrap_or(instance_id.as_str());
                    if reading.decks[*player].contains(id) || reading.hands[*player].contains_key(id) {
                        let radiant = is_radiant(before, after, &ids);
                        cast_or_summon(reading, *player, def_id, radiant, None);
                    }
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
                hold_radiant(reading, before, after, instance_id);
                // R1256000: an instance's first cue counts for its controller when the step turned it
                // Radiant; the engine also cues a card Radiant already (R177), and re-cues dedupe to one.
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
                on_draw,
                x,
                embiggened,
                modes,
                ..
            } => {
                // R227: a Trap set face-down took a fresh id; the hand knew it by the old one.
                let id = former_id.as_ref().unwrap_or(instance_id);
                let from_hand = reading.hands[*player].contains_key(id);
                let ids = [instance_id.as_str(), id.as_str()];
                let created = is_created(before, after, &ids);
                // R2179002: a cast on draw (or from the deck, R3558007) is no play, from a hand included:
                // R748's card setup dealt uncast is cast from there.
                let on_draw = *on_draw == Some(true);
                announced.insert(instance_id.clone());
                announced.insert(id.clone());
                // R4672000: the play's variant, on each entry it lists, which its own prompts extend.
                let variant = play_variant(modes.as_deref(), *x, *embiggened);
                let mut slots: Vec<Slot> = Vec::new();
                if created {
                    // R2089000: a play from anywhere and a cast use it alike. A card no event named
                    // before was made in this step for the seat that casts it.
                    let radiant = if from_hand {
                        reading.radiant.contains(id)
                    } else {
                        is_radiant(before, after, &ids)
                    };
                    let source = maker_of(before, after, &ids);
                    if let Some((creator, at)) =
                        use_created(reading, &ids, *player, def_id, radiant, source, variant.clone())
                        && creator == *player
                    {
                        slots.push(Slot::CreatedUsed(creator, at));
                    }
                }
                if !from_hand || on_draw {
                    // R4279000: a dealt card cast on its draw, whichever deck it was drawn from (as
                    // `drawn` credits the drawer), or cast from its seat's own deck.
                    if !created && (on_draw || reading.decks[*player].contains(id)) {
                        let radiant = is_radiant(before, after, &ids);
                        let at = cast_or_summon(reading, *player, def_id, radiant, variant);
                        slots.push(Slot::Cast(*player, at));
                    }
                    if from_hand {
                        reading.hands[*player].shift_remove(id);
                    }
                    // R4707000, R28: a Recursion's cast rolls for the play that began its chain.
                    if created
                        && !from_hand
                        && let Some(root) =
                            chain_root(reading, after, def_id, maker_of(before, after, &ids).as_ref())
                    {
                        reading.chains.insert(instance_id.clone(), root);
                    }
                    open(reading, instance_id, *player, def_id, slots);
                    continue;
                }
                if stolen.iter().any(|countered| countered == id) {
                    leave(&mut reading.hands, id);
                    open(reading, instance_id, *player, def_id, slots);
                    continue;
                }
                let turn = reading.current_turn;
                if created {
                    let played = &mut reading.created_played[*player];
                    played.push(CreatedUse {
                        card: def_id.clone(),
                        turn,
                        radiant: reading.radiant.contains(id),
                        variant,
                        source: None,
                    });
                    slots.push(Slot::CreatedPlayed(*player, played.len() - 1));
                } else {
                    reading.played[*player].push(def_id.clone());
                    reading.played_turns[*player].push(turn);
                    reading.played_radiant[*player].push(reading.radiant.contains(id));
                    let variants = &mut reading.played_variants[*player];
                    variants.push(variant);
                    slots.push(Slot::Played(*player, variants.len() - 1));
                }
                // R1256000, R227: a play that put the card face-down set a Trap.
                if former_id.is_some() {
                    reading.traps_set[*player] += 1;
                }
                reading.hands[*player].shift_remove(id);
                open(reading, instance_id, *player, def_id, slots);
            }
            // R4672000: the play has resolved; a prompt its card asks from here on is no part of it.
            GameEvent::CardResolved { instance_id, .. } => {
                reading.open.shift_remove(instance_id);
            }
            // R4707000: a table's entries rolled, by their places; a list done whole drew none.
            GameEvent::ChaosRolled {
                instance_id,
                def_id,
                effects,
                indices: Some(picked),
                table_size: Some(of),
                ..
            } => {
                let roll = RolledOutcome::Effect {
                    card: def_id.clone(),
                    picked: picked.clone(),
                    labels: effects.clone(),
                    of: *of,
                };
                credit_roll(reading, instance_id, roll);
            }
            GameEvent::Rolled {
                instance_id,
                def_id,
                kind,
                sides,
                rolls,
                hit,
                ..
            } => {
                if let Some(roll) = rolled_outcome(def_id, *kind, *sides, rolls, *hit) {
                    credit_roll(reading, instance_id, roll);
                }
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
                    hold_radiant(reading, before, after, new_instance_id);
                }
                // R2089000: a Transform's new card is Created (a Vanilla keeps the card it changes).
                // Taking a card's place on the field puts it there, which uses it.
                let new = [new_instance_id.as_str()];
                if new_instance_id != instance_id
                    && is_created(before, after, &new)
                    && let Some(card) = card_of(before, after, &new)
                {
                    let maker = creator.unwrap_or(card.controller);
                    let source = maker_of(before, after, &new);
                    if matches!(card.zone, Zone::Field { .. }) {
                        let radiant = card.radiant;
                        use_created(reading, &new, maker, to_def_id, radiant, source, None);
                    } else {
                        make(reading, new_instance_id, maker, to_def_id, source);
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
            // R3335001: each fusion into a seat's Starship (§6.1) and each launch (§6.3), once.
            GameEvent::StarshipFused { player, .. } => {
                reading.starship_pieces_fused[*player] += 1;
            }
            GameEvent::StarshipLaunched { player, .. } => {
                reading.starships_launched[*player] += 1;
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
    let unused: Vec<&Made> = reading
        .made
        .iter()
        .filter(|(id, made)| made.creator == player && !reading.used.contains(*id))
        .map(|(_, made)| made)
        .collect();
    // R4672001: the makers beside them, absent when no card made any; R4672000: the variants likewise.
    let unused_sources: Vec<Option<MadeBy>> = unused.iter().map(|made| made.source.clone()).collect();
    let unused: Vec<String> = unused.iter().map(|made| made.card.clone()).collect();
    let variants = &reading.played_variants[player];
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
        // R5054000: always present on a fresh summary, empty when none, so the fold can tell it from an
        // older record's.
        held_radiant: Some(reading.held_radiant[player].iter().cloned().collect()),
        played_variants: Some(variants.clone()).filter(|variants| variants.iter().any(Option::is_some)),
        // R1256000: a fresh summary always knows the counts; `None` is for older records.
        fatigue_draws: Some(reading.fatigue_draws[player]),
        fatigue_damage: Some(reading.fatigue_damage[player]),
        traps_set: Some(reading.traps_set[player]),
        traps_fired: Some(reading.traps_fired[player]),
        position_switches: Some(reading.position_switches[player]),
        radiant_upgrades: Some(reading.radiant_upgrades[player]),
        created_used: listed(&reading.created_used[player]),
        created_unused: listed(&unused),
        created_unused_sources: Some(unused_sources).filter(|sources| sources.iter().any(Option::is_some)),
        created_played: listed(&reading.created_played[player]),
        cast_or_summoned: listed(&reading.cast_or_summoned[player]),
        // R3335001: absent when none, so a game without a Starship keeps its record's shape.
        starship_pieces_fused: Some(reading.starship_pieces_fused[player]).filter(|n| *n > 0),
        starships_launched: Some(reading.starships_launched[player]).filter(|n| *n > 0),
        picks: listed(&reading.picks[player]),
        // R4552000: the AI's fold writes the turn rows (`jackioh_ai::attach_swing_turns`).
        swing_turns: None,
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
        let decks = decks_of(start, None);
        let radiant = radiant_of(start, &hands);
        SummaryReader {
            reading: Reading::new(hands, decks, radiant, start.turn.max(1)),
        }
    }

    /// One step: the state as it began, the action `reduce` took it with (`None` for a step no action
    /// made, the game's beginning), the events it emitted, and the state it left.
    pub fn step(
        &mut self,
        before: &GameState,
        action: Option<&Action>,
        events: &[GameEvent],
        after: &GameState,
    ) {
        // The state's own hands as the step begins, so nothing the events do not spell out (a card
        // whose control changed, say) carries from one action into the next, but a play still in flight.
        self.reading.hands = hands_of(before, Some(&self.reading.hands));
        self.reading.decks = decks_of(before, Some(&self.reading.decks));
        self.reading.radiant = radiant_of(before, &self.reading.hands);
        if let Some(action) = action {
            answer(&mut self.reading, before, action);
        }
        read(&mut self.reading, events, before, after);
    }

    /// The seat's summary so far, `deck` its decklist.
    pub fn seat(&self, player: PlayerId, deck: Vec<String>) -> SeatSummary {
        seat_summary(&self.reading, deck, player)
    }
}

/// R376, R4552000: the state a recorded game began in, set up as it was played (`summarize_game` and
/// the AI's turn rows, `jackioh_ai::swing_turns`, both start here, so both read the game that was played).
pub fn recorded_start(input: &FoldArgs) -> GameState {
    // No `dealt` here, unlike `fold`: it changes no event.
    create_recorded_game(&CreateGameOptions {
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
        // R3049000: and the win-rate snapshot, or CN Tech and NA Tech pick other cards.
        win_rates: input.win_rates.clone(),
        // R4595000: and the deck sets, which the state keeps.
        deck_sets: input.deck_sets.clone(),
        ..Default::default()
    })
}

/// R376, R4552000: replays `log` from `start` (`recorded_start`), the game's beginning first, handing
/// each step to `step` as `(before, events, after)`, and returns the state it ends in. The fold is
/// `replay::fold`'s, so an action the log holds and the engine refuses is skipped as there.
pub fn read_steps(
    start: &GameState,
    log: &[Action],
    mut step: impl FnMut(&GameState, &[GameEvent], &GameState),
) -> GameState {
    read_acted_steps(start, log, |before, _, events, after| step(before, events, after))
}

/// R4672000: `read_steps`, handing each step the action that took it as well (`None` for the game's
/// beginning), which the record's reader reads a prompt's answer off (`SummaryReader::step`).
fn read_acted_steps(
    start: &GameState,
    log: &[Action],
    mut step: impl FnMut(&GameState, Option<&Action>, &[GameEvent], &GameState),
) -> GameState {
    let begun = begin_game(start);
    step(start, None, &begun.events, &begun.state);
    let mut state = begun.state;
    for action in log {
        let result = reduce(&state, action);
        if result.error.is_some() {
            continue;
        }
        step(&state, Some(action), &result.events, &result.state);
        state = result.state;
    }
    state
}

/// R376: what each seat's cards did in a finished game, and how it ended; `None` with no result. The
/// fold is `replay::fold`'s, so an action the log holds and the engine refuses is skipped as there.
pub fn summarize_game(input: &FoldArgs) -> Option<GameSummary> {
    summarize_game_with(input, |_, _, _| {})
}

/// R4552000: `summarize_game`, handing each step of its one fold to `step` as well, as `read_steps`
/// does, so another reading of the game (the AI's turn rows, `jackioh_ai::summarize_with_swing_turns`)
/// shares the fold rather than replay the log again (#2213: a finished game's log is folded once).
pub fn summarize_game_with(
    input: &FoldArgs,
    mut step: impl FnMut(&GameState, &[GameEvent], &GameState),
) -> Option<GameSummary> {
    let start = recorded_start(input);
    let mut reader = SummaryReader::new(&start);
    let state = read_acted_steps(&start, &input.log, |before, action, events, after| {
        reader.step(before, action, events, after);
        step(before, events, after);
    });

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
        Reading::new(
            PerPlayer::new(IndexMap::new(), IndexMap::new()),
            PerPlayer::new(IndexSet::new(), IndexSet::new()),
            IndexSet::new(),
            1,
        )
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

    /// R5054000: `hand_card`'s card as the reader's hand holds it, the opening hand dealt or not.
    fn holding(state: &GameState, dealt: bool) -> Reading {
        let card = &state.players[PlayerId::P1].hand[0];
        let mut reading = reading();
        reading.hands.p1.insert(card.id.clone(), card.def_id.clone());
        if dealt {
            reading.opening = Some(PerPlayer::new(vec![card.def_id.clone()], Vec::new()));
        }
        reading
    }

    #[test]
    fn r5054000_a_card_turned_radiant_in_hand_is_held_radiant_played_or_not() {
        let before = hand_card(false);
        let after = hand_card(true);
        let mut reading = holding(&before, true);
        read(&mut reading, &cue(&after), &before, &after);
        let reader = SummaryReader { reading };
        let seat = reader.seat(PlayerId::P1, vec!["core-008".to_string()]);
        assert_eq!(seat.held_radiant, Some(vec!["core-008".to_string()]));
        assert!(seat.played.is_empty(), "held, never played");
        assert_eq!(
            reader.seat(PlayerId::P2, Vec::new()).held_radiant,
            Some(Vec::new())
        );
    }

    #[test]
    fn r5054000_a_normal_card_or_one_upgraded_before_the_opening_hand_is_not_held_radiant() {
        let before = hand_card(false);
        let after = hand_card(true);
        // Never Radiant: present and empty, so the fold reads it as seen normal.
        let mut normal = holding(&before, true);
        read(&mut normal, &[], &before, &before);
        assert_eq!(
            SummaryReader { reading: normal }
                .seat(PlayerId::P1, Vec::new())
                .held_radiant,
            Some(Vec::new())
        );
        // Before the opening hand is dealt, a card may yet be mulliganed away: it is not held.
        let mut early = holding(&before, false);
        read(&mut early, &cue(&after), &before, &after);
        assert!(early.held_radiant.p1.is_empty());
    }

    #[test]
    fn r3335001_the_summary_counts_each_starship_fused_and_launched_event() {
        let state = hand_card(false);
        let fused = |player: PlayerId, pieces: u32| GameEvent::StarshipFused {
            player,
            instance_id: "ship".to_owned(),
            piece_id: format!("piece-{pieces}"),
            def_id: "stinkyscape-001".to_owned(),
            pieces,
        };
        let launched = |player: PlayerId| GameEvent::StarshipLaunched {
            player,
            instance_id: "ship".to_owned(),
            def_id: "stinkyscape-t-dormant-starship".to_owned(),
            name: "The Ninth Life".to_owned(),
            pieces: Vec::new(),
        };
        let mut reading = reading();
        read(
            &mut reading,
            &[
                fused(PlayerId::P1, 1),
                fused(PlayerId::P1, 2),
                fused(PlayerId::P2, 1),
            ],
            &state,
            &state,
        );
        read(&mut reading, &[launched(PlayerId::P1)], &state, &state);
        assert_eq!(
            (reading.starship_pieces_fused.p1, reading.starship_pieces_fused.p2),
            (2, 1)
        );
        assert_eq!(
            (reading.starships_launched.p1, reading.starships_launched.p2),
            (1, 0)
        );
        let p1 = seat_summary(&reading, Vec::new(), PlayerId::P1);
        let p2 = seat_summary(&reading, Vec::new(), PlayerId::P2);
        assert_eq!(
            (p1.starship_pieces_fused, p1.starships_launched),
            (Some(2), Some(1))
        );
        // None launched is absent from the record, as it is from one written before the count.
        assert_eq!((p2.starship_pieces_fused, p2.starships_launched), (Some(1), None));
        let json = serde_json::to_value(&p2).unwrap_or_default();
        assert!(json.get("starshipsLaunched").is_none(), "{json}");
        assert_eq!(json.get("starshipPiecesFused"), Some(&json!(1)));
    }
}
