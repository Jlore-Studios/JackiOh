//! Exile, Bounce, Discard and Counter: the moves that take a card off the field, out of a hand or off
//! the stack (SPEC §6.3, §3.2, §2.4, R11, R12, R16, R78, R448). Every zone change goes through `zones.rs`.
//!
//! Each verb comes in a single-card form and a sweep — `exile`/`exile_all`/`exile_adjacent_to`,
//! `bounce`/`bounce_all`, `discard`/`discard_hand`, plus `exile_hand` — and the sweeps are walks over
//! `cards_in_scope` or `adjacent_to` (§3.1, §3.2) down the same private per-card helper the
//! single-card form uses. One implementation per move, so "bounce" can only ever mean one thing.
//!
//! Port of `packages/engine/src/effects/move.ts` (`move` is a Rust keyword, hence `move_`).


use serde::{Deserialize, Serialize};

use crate::announce::{innermost_live_announce, is_announce_live, mark_countered};
use crate::config::HAND_CAP;
use crate::draw::add_to_hand;
use crate::echo::exile_on_landing;
use crate::effects::targets::{BoardScope, PlayerSpec, TargetSpec, adjacent_to, cards_in_scope, instance_of, player_of};
use crate::mana::{effective_cost, is_x_cost};
use crate::ownership::take_into_hand;
use crate::query::zone_cards;
use crate::script::{Effect, EffectContext, EngineSink};
use crate::state::{CardInstance, find_instance, find_instance_mut};
use crate::wire::{CounteredTo, GameEvent, Zone, ZoneName};
use crate::zones::{MoveResult, OffFieldZone, is_unit_token, move_to_zone, report_graveyard_landing};

/// One card to the exile pile: the whole of §6.3 Exile for a single card, so `exile`, `exile_all`,
/// `exile_adjacent_to` and `exile_hand` are four ways of naming cards over ONE implementation. A card
/// that reaches the pile feeds the game exile counter (R55); a unit token ceases to exist instead
/// and never enters it (R11), so it is not counted, while the event still reports the card leaving.
fn exile_card(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    if card.zone.z() == ZoneName::Exile {
        return;
    }
    // R178: a resolving Spell's "exile this" names where §10.5 step 7 sends it, so it stays itself
    // until then — for the rest of its text and for its Echo repeats (§6.2) — and lands in exile.
    let is_self = ctx.self_.as_ref().is_some_and(|me| me.id == card.id);
    if card.zone.z() == ZoneName::Resolving && is_self {
        if let Some(live) = find_instance_mut(ctx.state, &card.id) {
            exile_on_landing(live);
        }
        return;
    }

    let mut card = card.clone();
    let moved = move_to_zone(ctx.state, &mut card, OffFieldZone::Exile, Default::default());
    if matches!(moved, MoveResult::Moved) {
        ctx.state.counters.exiled += 1;
    }
    ctx.events.push(GameEvent::Exiled {
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        owner: card.owner,
    });
}

/// One card back to its controller's hand: the whole of §6.3 Bounce for a single card, shared by
/// `bounce` and `bounce_all`. A steal on the field changes control only (R15, R171), so for a card
/// another player controls the hand it returns to is the controller's, not the owner's — and it
/// lands there as the controller's own card, as a card taken off the field's reach by a steal does
/// (R12, R747): ownership moves with it, so every pile it reaches afterwards, and every hand rule
/// that reads the owner, is the holder's. A unit token vanishes (R11), the hand cap applies to the
/// hand it enters so a full hand burns it (§2.4), and the instance resets on the way out (R78).
pub fn bounce_card(ctx: &mut EngineSink<'_>, card: &CardInstance) {
    if card.zone.z() == ZoneName::Hand {
        return;
    }

    let mut card = card.clone();
    // R747: off the field control means nothing (R12), so only a card on the field can be controlled by
    // another player. It joins its controller's cards before the R78 reset makes them one again.
    if card.zone.z() == ZoneName::Field {
        card.owner = card.controller;
        if let Some(live) = find_instance_mut(ctx.state, &card.id) {
            live.owner = live.controller;
        }
    }
    let token = is_unit_token(ctx.state, &card);
    let event = GameEvent::Bounced {
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        owner: card.owner,
    };

    if token {
        // R11: it ceases to exist, so it never reaches a hand and the hand cap never sees it.
        move_to_zone(ctx.state, &mut card, OffFieldZone::Hand, Default::default());
        ctx.events.push(event);
        return;
    }

    ctx.events.push(event);
    add_to_hand(ctx, &mut card);
}

/// `exile`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExileArgs {
    pub target: TargetSpec,
}

/// §6.3 Exile: to the exile pile from anywhere, with no Death trigger.
pub fn exile(args: ExileArgs) -> Effect {
    Effect::new("exile", move |ctx| {
        let Some(card) = instance_of(ctx, &args.target) else {
            return;
        };
        exile_card(ctx, &card);
    })
}

/// §6.3 Exile over a scope: every card the scope names, in `cards_in_scope` order, down the same path
/// a single exile takes (#100 Ceaseless Void, "Cry: exile all other permanents on both sides" —
/// `{ side: "any", rows: ["units", "backrow"], excludeSelf: true }`). `excludeSelf` is what leaves
/// the running card standing; without it the Void would exile itself mid-Cry.
pub fn exile_all(args: BoardScope) -> Effect {
    Effect::new("exileAll", move |ctx| {
        let cards: Vec<CardInstance> = cards_in_scope(ctx, &args).into_iter().collect();
        for card in &cards {
            exile_card(ctx, card);
        }
    })
}

/// `exileAdjacentTo`'s arguments: the target, and the scope its neighbours must match
/// (TS `{ target: TargetSpec } & BoardScope`, the scope's keys beside `target`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExileAdjacentToArgs {
    pub target: TargetSpec,
    #[serde(flatten)]
    pub scope: BoardScope,
}

/// §3.1 Adjacent exile: lanes N-1 and N+1 on the target's own side and row (#34 Collateral Damage
/// radiant, "also the permanents adjacent to the target in its row"). Adjacency never crosses rows,
/// so "in its row" needs no argument of its own — a backrow target has backrow neighbours and a
/// unit has units. The card pairs this with a plain `exile` on the target itself.
pub fn exile_adjacent_to(args: ExileAdjacentToArgs) -> Effect {
    Effect::new("exileAdjacentTo", move |ctx| {
        let cards: Vec<CardInstance> = adjacent_to(ctx, &args.target, &args.scope).into_iter().collect();
        for card in &cards {
            exile_card(ctx, card);
        }
    })
}

// ---------------------------------------------------------------------------
// Exile out of the off-field zones, by cost (§6.3, R26, R66, R135)
// ---------------------------------------------------------------------------

/// The three zones a card can be exiled out of by a sweep; the exile pile is where they go.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum ExileZone {
    Library,
    Hand,
    Graveyard,
}

impl ExileZone {
    /// The off-field pile this names (`zones::OffFieldZone`, whose literals these are).
    fn pile(self) -> OffFieldZone {
        match self {
            ExileZone::Library => OffFieldZone::Library,
            ExileZone::Hand => OffFieldZone::Hand,
            ExileZone::Graveyard => OffFieldZone::Graveyard,
        }
    }
}

/// R135's order, and §8 #94's: library, then hand, then graveyard.
pub const EXILE_ZONE_ORDER: &[ExileZone] = &[ExileZone::Library, ExileZone::Hand, ExileZone::Graveyard];

/// `CostFilter.parity`: "odd" | "even".
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum CostParity {
    Odd,
    Even,
}

/// Which cards in those zones a sweep takes, by what they cost NOW (R65, R66). The cost is
/// `effective_cost` and never `query_cost`: `query_cost` reads a DEFINITION, so it cannot see the
/// `costMod` #7 Jewelosco Scarab left on an instance or the discount #95 Call to Chaos put across a
/// whole library, and R66 asks for each card's cost at resolution.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CostFilter {
    /// The parity of the card's current cost (#94's "every odd-cost card", R26).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parity: Option<CostParity>,
    /// An exact current cost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<i32>,
    /// R66: X-cost cards are exempt, since R65 reads one out of play as 0 and it is nobody's number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exempt_x_cost: Option<bool>,
}

/// Whether one card passes a cost filter, read at the moment the effect applies (R66).
fn matches_cost(ctx: &EffectContext<'_>, card: &CardInstance, filter: &CostFilter) -> bool {
    if filter.exempt_x_cost == Some(true) && is_x_cost(ctx.state, card) {
        return false;
    }
    let cost = effective_cost(ctx.state, card, Default::default());
    if let Some(wanted) = filter.cost
        && cost != wanted
    {
        return false;
    }
    if filter.parity == Some(CostParity::Odd) && cost % 2 == 0 {
        return false;
    }
    if filter.parity == Some(CostParity::Even) && cost % 2 != 0 {
        return false;
    }
    true
}

/// `exileMatching`'s arguments (TS `{ zones?; player? } & CostFilter`, the filter's keys beside them).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExileMatchingArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zones: Option<Vec<ExileZone>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(flatten)]
    pub filter: CostFilter,
}

/// §6.3 Exile over a player's off-field zones, by cost: #94 Genn's Greed's "exile every odd-cost card
/// in your library, hand and GY (X-cost cards exempt)".
///
/// R135 is the whole shape of it. The zones are walked in the order §8 names — library, then hand,
/// then graveyard, which `EXILE_ZONE_ORDER` holds so the order is stated once — and EACH CARD IS ITS
/// OWN EXILE, down the same `exile_card` a single `exile` uses: `state.counters.exiled` moves once per
/// card (R55) and anything watching sees one `exiled` event per card rather than a batch. Each pile is
/// snapshotted before it is walked (`zone_cards` copies), because exiling splices the pile underneath.
///
/// The scope is one player's zones, never both: §8 #94 says "YOUR library, hand and GY".
pub fn exile_matching(args: ExileMatchingArgs) -> Effect {
    Effect::new("exileMatching", move |ctx| {
        let owner = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let zones: Vec<ExileZone> = args.zones.clone().unwrap_or_else(|| EXILE_ZONE_ORDER.to_vec());
        // R66: "every odd-cost card" is one set, read as the clause resolves — each card's cost per
        // R65 at that moment — and then exiled a card at a time (R135). Read again after each exile, a
        // cost that counts the exiles (#100 Ceaseless Void, R55) flipped its parity halfway through.
        let mut matching: Vec<CardInstance> = Vec::new();
        for zone in zones {
            let pile: Vec<CardInstance> = zone_cards(ctx.state, owner, zone.pile()).into_iter().collect();
            for card in pile {
                if matches_cost(ctx, &card, &args.filter) {
                    matching.push(card);
                }
            }
        }
        for card in &matching {
            exile_card(ctx, card);
        }
    })
}

/// `bounce`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BounceArgs {
    pub target: TargetSpec,
}

/// §6.3 Bounce: return the card to its controller's hand (R747).
pub fn bounce(args: BounceArgs) -> Effect {
    Effect::new("bounce", move |ctx| {
        let Some(card) = instance_of(ctx, &args.target) else {
            return;
        };
        bounce_card(ctx, &card);
    })
}

/// §6.3 Bounce over a scope: every card the scope names, in `cards_in_scope` order (#17 Flood,
/// "Bounce all units on both sides", which is `{ side: "any" }` — `BoardScope`'s default row is
/// `["units"]`). §3.2 spells out the token case for exactly this card: "Bounce all units" clears
/// unit tokens because they cease to exist rather than reaching a hand (R11).
pub fn bounce_all(args: BoardScope) -> Effect {
    Effect::new("bounceAll", move |ctx| {
        let cards: Vec<CardInstance> = cards_in_scope(ctx, &args).into_iter().collect();
        for card in &cards {
            bounce_card(ctx, card);
        }
    })
}

/// Hand to GY for one named card, with the events §10.3 gives a discard. Exported for the one engine
/// path that discards as a cost rather than as an effect: a targeting cost (R450, `targeting_point.rs`).
pub fn discard_from_hand(ctx: &mut EngineSink<'_>, card: &CardInstance) {
    discard_card(ctx, card);
}

fn discard_card(ctx: &mut EngineSink<'_>, card: &CardInstance) {
    if card.zone.z() != ZoneName::Hand {
        return;
    }

    let mut card = card.clone();
    let moved = move_to_zone(ctx.state, &mut card, OffFieldZone::Graveyard, Default::default());
    ctx.events.push(GameEvent::Discarded {
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        owner: card.owner,
    });
    // R11: a unit-token card leaving a hand ceases to exist and reaches no graveyard; B5 E5: a card a
    // replacement sent elsewhere is reported where it went.
    report_graveyard_landing(ctx, &card, moved);
}

/// `discard`'s arguments: the target defaults to the first chosen selection.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscardArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
}

/// §6.3 Discard of a named card: R16 makes the discard the player's choice, so the choice arrives
/// here as a selection the script already prompted for (`{ of: "chosen" }`).
pub fn discard(args: DiscardArgs) -> Effect {
    Effect::new("discard", move |ctx| {
        let spec = args.target.clone().unwrap_or(TargetSpec::Chosen { index: None });
        let Some(card) = instance_of(ctx, &spec) else {
            return;
        };
        discard_card(ctx, &card);
    })
}

/// `discardRandom`'s arguments: `count` defaults to 1, `player` to "self".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscardRandomArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// §6.3 Discard at random (R16: only when the card says "random"), drawn from the match rng.
pub fn discard_random(args: DiscardRandomArgs) -> Effect {
    Effect::new("discardRandom", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let count = args.count.unwrap_or(1).max(0);
        for _ in 0..count {
            if ctx.state.players[player].hand.is_empty() {
                return;
            }
            let Some(card) = ctx.sink.rng.pick(&ctx.sink.state.players[player].hand).cloned() else {
                return;
            };
            discard_card(ctx, &card);
        }
    })
}

/// `discardHand`'s and `exileHand`'s arguments: `player` defaults to "self".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscardHandArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// §6.3 Discard of a whole hand: every card in the hand to the graveyard, in hand order (#76 Field
/// of Dreams, "Replace your hand with the same number of Reminisce" — R31 sends the replaced cards
/// to the graveyard, which is how a later Reminisce can find them there).
///
/// Deterministic, and drawing NOTHING from `ctx.rng`, for two reasons:
///   - A whole-hand sweep involves no choice at all, so R16's "the player chooses unless random is
///     stated" does not arise. There is nothing to prompt for: every card goes.
///   - `discard_random({ count: hand.len() })` would reach the same end state while burning
///     `hand.len()` rng draws. §10.7 makes `rngCursor` part of state, so those draws would advance
///     it and change every downstream replay hash — a different shuffle, a different Discover, a
///     different fuzz game — for no reason a rule asks for.
pub fn discard_hand(args: DiscardHandArgs) -> Effect {
    Effect::new("discardHand", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        // A snapshot: `discard_card` splices the hand, so iterating it live would skip cards.
        let hand = ctx.state.players[player].hand.clone();
        for card in &hand {
            discard_card(ctx, card);
        }
    })
}

/// `exileHand`'s arguments: `player` defaults to "self".
pub type ExileHandArgs = DiscardHandArgs;

/// §6.3 Exile of a whole hand: the same sweep, to the exile pile (#78 /fullsend, "at end of turn,
/// exile your hand"). Each card that reaches the pile bumps `counters.exiled` (R55) and emits one
/// `exiled` event; a unit-token card ceases to exist instead of reaching the pile and is not
/// counted (R11, §3.2). No choice and no rng draw, for the reasons on `discard_hand`.
pub fn exile_hand(args: ExileHandArgs) -> Effect {
    Effect::new("exileHand", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let hand = ctx.state.players[player].hand.clone();
        for card in &hand {
            exile_card(ctx, card);
        }
    })
}

// ---------------------------------------------------------------------------
// Counter (B5 E1, E2; R448)
// ---------------------------------------------------------------------------

/// Where a countered card goes: its owner's graveyard, exile when the text says so (Classic #10), or
/// the hand of the player whose effect countered it, as their own (E2's steal off the stack: Classic
/// #4, #72 Radiant).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum CounterDestination {
    Graveyard,
    Exile,
    Thief,
}

/// `counterPlay`'s arguments: `to` defaults to "graveyard"; with no `target` it is the innermost
/// announce still live.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CounterPlayArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<CounterDestination>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
}

/// B5 E1, R448: §6.3 Counter — cancel the play an announce window is answering. The card never
/// resolves or enters the field: no Cry, no Death, no `cardPlayed` or `cardResolved`, no Echo repeat,
/// and it counts for nothing that counts plays, because §10.5 step 4, where every count is made, never
/// runs for it. Its mana and Tributes stay spent. `target` names the announced card (a response reads
/// it off its `cardAnnounced`); with none it is the innermost announce still live. The first Counter to
/// resolve cancels the play, and a later one finds no card and does nothing (a trap on a cancelled
/// announce is not even offered it, and stays set).
///
/// `to: "thief"` is E2: the card moves to the countering player's hand and its owner becomes that
/// player (`ownership::take_into_hand`); a full hand burns it into their graveyard (§2.4). A unit-token
/// card ceases to exist wherever it was going (R11). `countered` goes out first, naming where the card
/// went, then the move's own events.
pub fn counter_play(args: CounterPlayArgs) -> Effect {
    Effect::new("counterPlay", move |ctx| {
        let record: Option<String> = match &args.target {
            None => innermost_live_announce(ctx.state).map(|record| record.instance_id.clone()),
            Some(target) => match instance_of(ctx, target) {
                Some(named) if is_announce_live(ctx.state, &named.id) => Some(named.id),
                _ => None,
            },
        };
        let Some(record) = record else {
            return;
        };
        let Some(card) = find_instance(ctx.state, &record).cloned() else {
            return;
        };
        let Zone::Resolving { player } = card.zone else {
            return;
        };
        if !mark_countered(ctx.state, &card.id) {
            return;
        }

        let to = args.to.unwrap_or(CounterDestination::Graveyard);
        let thief = ctx.controller;
        let token = is_unit_token(ctx.state, &card);
        let lands = if token {
            CounteredTo::Gone
        } else {
            match to {
                CounterDestination::Thief => {
                    if ctx.state.players[thief].hand.len() as i32 >= HAND_CAP {
                        CounteredTo::Graveyard
                    } else {
                        CounteredTo::Hand
                    }
                }
                CounterDestination::Exile => CounteredTo::Exile,
                CounterDestination::Graveyard => CounteredTo::Graveyard,
            }
        };
        let by_instance_id = ctx.self_.as_ref().map(|me| me.id.clone());
        ctx.events.push(GameEvent::Countered {
            player,
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            by_instance_id,
            to: lands,
        });

        let mut card = card;
        match to {
            CounterDestination::Thief => {
                take_into_hand(ctx, &card, thief);
            }
            CounterDestination::Exile => exile_card(ctx, &card),
            CounterDestination::Graveyard => {
                // B5 E5: its graveyard, or wherever a replacement sends it instead (R460).
                let moved = move_to_zone(ctx.state, &mut card, OffFieldZone::Graveyard, Default::default());
                report_graveyard_landing(ctx, &card, moved);
            }
        }
    })
}

/// `counter`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CounterArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
}

/// §6.3 Counter by its rules name: `counter_play` to its owner's graveyard, aimed at `target` or at the
/// innermost live announce. The one Counter there is (R448).
pub fn counter(args: CounterArgs) -> Effect {
    counter_play(CounterPlayArgs {
        to: Some(CounterDestination::Graveyard),
        target: args.target,
    })
}
