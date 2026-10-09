//! R185: one concrete world consistent with what the seat knows. `redact` has already turned every
//! hidden card into a placeholder; this fills each one with a real non-token definition of any set
//! (R380) drawn from what the opponent has not shown, shuffles the seat's own library, and gives the
//! world a seed of its own, so no simulation can foresee a real draw or a real coin flip.
//!
//! docs/polish/3-ai.md's six steps, in order. `rng` is the AI's own stream and every draw below comes
//! from it in a fixed order, so the same public state and the same rng give the same world.
//!
//! Port of `packages/ai/src/determinize.ts`. TS set a placeholder's `defId` through the live object
//! while it tried candidate traps; here the card is addressed by where it sits (side, row, lane) and
//! written in place on the clone, in the same order.

use indexmap::IndexSet;
use jackioh_engine::prelude::json_as;
use jackioh_engine::{
    CardInstance, CardType, CatalogQueryArgs, CostOptions, GameState, PLAYER_IDS, PlayerId, RevealAt, Rng,
    SetAs, active_units_of, effective_cost, find_def, query, scripts_for, unit_view,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::AI_DETERMINIZE;
use crate::observe::HIDDEN_DEF_ID;

fn all_cards(state: &GameState) -> Vec<&CardInstance> {
    let mut out: Vec<&CardInstance> = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        out.extend(side.hand.iter());
        out.extend(side.library.iter());
        out.extend(side.graveyard.iter());
        out.extend(side.exile.iter());
        out.extend(side.resolving.iter());
        for pile in side.units.iter().flatten() {
            out.extend(pile.iter());
        }
        out.extend(side.backrow.iter().flatten());
        // B5 E21, R446: a backrow pile's dormant cards and a carrier's Unit are on the board too.
        for pile in side.backrow_piles.iter().flatten() {
            out.extend(pile.iter());
        }
        out.extend(side.carried.iter().flatten().flatten());
    }
    out
}

/// One def id, uniformly, from `pool` minus `seen` minus what this determinization already sampled;
/// with replacement from the whole pool once that is empty.
fn sample_def(
    pool: &[String],
    seen: &IndexSet<String>,
    sampled: &mut IndexSet<String>,
    rng: &mut Rng,
) -> String {
    let open: Vec<&String> = pool
        .iter()
        .filter(|id| !seen.contains(*id) && !sampled.contains(*id))
        .collect();
    if !open.is_empty() {
        let at = rng.int(open.len() as i32) as usize;
        let pick = open.get(at).map(|id| (*id).clone()).unwrap_or_default();
        sampled.insert(pick.clone());
        return pick;
    }
    let at = rng.int(pool.len() as i32) as usize;
    pool.get(at).cloned().unwrap_or_else(|| HIDDEN_DEF_ID.to_string())
}

fn is_placeholder(card: &CardInstance) -> bool {
    card.def_id == HIDDEN_DEF_ID
}

/// Every unit's Attack, Health and keywords as the board shows them, one string per unit.
fn shown_units(state: &GameState) -> String {
    PLAYER_IDS
        .into_iter()
        .flat_map(|player| {
            active_units_of(state, player)
                .into_iter()
                .map(|unit| {
                    let view = unit_view(state, unit);
                    let keywords = serde_json::to_string(&view.keywords).unwrap_or_default();
                    format!("{}:{}/{}/{}", unit.id, view.attack, view.max_health, keywords)
                })
                .collect::<Vec<String>>()
        })
        .collect::<Vec<String>>()
        .join("|")
}

/// Whether either face of a def projects an aura, which a face-down Trap does from the moment it is set (R403).
fn has_aura(state: &GameState, def_id: &str) -> bool {
    let scripts = scripts_for(state, def_id);
    scripts.base.aura.is_some() || scripts.radiant.aura.is_some()
}

/// R762: the cost a trap of `def_id` would show in the zone `card` holds, read as the board reads a
/// face-down card's (R65, R351), without the shown number `redact` parked on the placeholder.
fn cost_in(state: &GameState, card: &CardInstance, def_id: &str) -> i32 {
    let mut slot = card.clone();
    slot.def_id = def_id.to_string();
    slot.cost_override = None;
    effective_cost(state, &slot, CostOptions::default())
}

/// How a determinization samples. `greedy_action` keeps the sampler the quality gates were fixed on.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeterminizeOptions {
    /// R762: a face-down card showing a cost takes a trap of that cost while an unseen one is left. Default true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_shown_cost: Option<bool>,
}

/// Where a face-down placeholder of one side sits: a backrow top, a card dormant under a backrow pile
/// (lane index, depth), or a card waiting in the resolving zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrapSlot {
    Top(usize),
    Dormant(usize, usize),
    Resolving(usize),
}

fn slot_card(state: &GameState, side: PlayerId, slot: TrapSlot) -> Option<&CardInstance> {
    let player = &state.players[side];
    match slot {
        TrapSlot::Top(lane) => player.backrow.get(lane).and_then(|card| card.as_ref()),
        TrapSlot::Dormant(lane, depth) => player
            .backrow_piles
            .as_ref()
            .and_then(|piles| piles.get(lane))
            .and_then(|pile| pile.get(depth)),
        TrapSlot::Resolving(at) => player.resolving.get(at),
    }
}

fn slot_card_mut(state: &mut GameState, side: PlayerId, slot: TrapSlot) -> Option<&mut CardInstance> {
    let player = &mut state.players[side];
    match slot {
        TrapSlot::Top(lane) => player.backrow.get_mut(lane).and_then(|card| card.as_mut()),
        TrapSlot::Dormant(lane, depth) => player
            .backrow_piles
            .as_mut()
            .and_then(|piles| piles.get_mut(lane))
            .and_then(|pile| pile.get_mut(depth)),
        TrapSlot::Resolving(at) => player.resolving.get_mut(at),
    }
}

/// ME-ALTPLAY, R1046: stamp a sampled Unit or Spell as a set card — a Unit at the start of the
/// next turn, a Spell at a uniformly sampled timing. With no permission nothing is stamped and no
/// extra rng draw happens.
fn stamp_sampled_set_as(
    state: &mut GameState,
    side: PlayerId,
    slot: TrapSlot,
    grants: &jackioh_engine::alt_play::FaceDownGrant,
    set_turn: i32,
    rng: &mut Rng,
) {
    let Some(def_id) = slot_card(state, side, slot).map(|card| card.def_id.clone()) else {
        return;
    };
    let Some(def) = find_def(Some(&*state), &def_id) else {
        return;
    };
    let is_unit = def.type_ == CardType::Unit && grants.units;
    let is_spell = def.type_ == CardType::Spell && grants.spells;
    if !is_unit && !is_spell {
        return;
    }
    // D4: a sampled Spell's timing is uniform; a Unit needs no draw.
    let reveal = if is_unit {
        RevealAt::StartOfNextTurn
    } else {
        RevealAt::ALL[rng.int(3) as usize]
    };
    let Some(card) = slot_card_mut(state, side, slot) else {
        return;
    };
    card.set_as = Some(SetAs {
        reveal,
        set_turn,
        echo: None,
        revealing: None,
    });
}

fn set_slot_def(state: &mut GameState, side: PlayerId, slot: TrapSlot, def_id: &str) {
    if let Some(card) = slot_card_mut(state, side, slot) {
        card.def_id = def_id.to_string();
    }
}

/// What `trapFor` closed over in TS: the trap pools, what the board shows, and the two id sets.
struct TrapSampler<'a> {
    trap_pool: &'a [String],
    aura_traps: &'a [String],
    shown: &'a str,
    match_shown_cost: bool,
}

impl TrapSampler<'_> {
    /// R602: whether the board would show every unit as it does now with `def_id` in this slot.
    fn agrees(&self, next: &mut GameState, side: PlayerId, slot: TrapSlot, def_id: &str) -> bool {
        set_slot_def(next, side, slot, def_id);
        let same = shown_units(next) == self.shown;
        set_slot_def(next, side, slot, HIDDEN_DEF_ID);
        same
    }

    #[allow(clippy::too_many_arguments)]
    fn trap_for(
        &self,
        next: &mut GameState,
        side: PlayerId,
        slot: TrapSlot,
        shown_cost: Option<i32>,
        seen: &IndexSet<String>,
        sampled: &mut IndexSet<String>,
        rng: &mut Rng,
    ) -> String {
        let open: Vec<String> = if self.aura_traps.is_empty() {
            self.trap_pool.to_vec()
        } else {
            let mut open: Vec<String> = Vec::new();
            for id in self.trap_pool {
                if !self.aura_traps.contains(id) || self.agrees(next, side, slot, id) {
                    open.push(id.clone());
                }
            }
            open
        };
        let pool: Vec<String> = if !open.is_empty() {
            open
        } else {
            self.trap_pool.to_vec()
        };
        // R762: the board shows this card's cost (R351), so a trap that would show another is no world the
        // seat could be in; with no unseen trap of that cost left, the pool falls back as before.
        if self.match_shown_cost
            && let Some(shown_cost) = shown_cost
        {
            let board: &GameState = next;
            let priced: Vec<String> = match slot_card(board, side, slot) {
                Some(card) => pool
                    .iter()
                    .filter(|id| cost_in(board, card, id) == shown_cost)
                    .cloned()
                    .collect(),
                None => Vec::new(),
            };
            if priced
                .iter()
                .any(|id| !seen.contains(id) && !sampled.contains(id))
            {
                return sample_def(&priced, seen, sampled, rng);
            }
        }
        sample_def(&pool, seen, sampled, rng)
    }
}

/// `rng.int(2 ** 31)`: 2³¹ is past `Rng::int`'s `i32`, so the same draw is made here by hand
/// (`Math.floor(next() * n) % n`, rng.ts).
fn int_2_31(rng: &mut Rng) -> u64 {
    const N: f64 = 2_147_483_648.0;
    ((rng.next() * N).floor() as u64) % (N as u64)
}

/// The ids a catalog query answers, in its order.
fn query_ids(args: serde_json::Value) -> Vec<String> {
    let args: CatalogQueryArgs = json_as(args);
    query(&args).into_iter().map(|def| def.id.clone()).collect()
}

/// R185: one concrete world consistent with `public_state` (the output of redact). Pure given rng.
pub fn determinize(
    public_state: &GameState,
    seat: PlayerId,
    rng: &mut Rng,
    options: DeterminizeOptions,
) -> GameState {
    let opp = seat.opponent();
    let mut next = public_state.clone();

    // Step 1: a stream the match never uses.
    next.seed = format!("ai:{}", int_2_31(rng));
    next.rng_cursor = 0;

    // Step 2: what the opponent has shown, in any zone.
    let mut seen: IndexSet<String> = IndexSet::new();
    for card in all_cards(&next) {
        if card.owner == opp && !is_placeholder(card) {
            seen.insert(card.def_id.clone());
        }
    }
    let mut sampled: IndexSet<String> = IndexSet::new();

    // Step 3: face-down backrow placeholders, in lane order, from the Trap and Field Trap pool. R602: a
    // face-down Trap's aura is live (R403) and the units it changes are on the board for the seat to read,
    // so a candidate whose aura would change any unit's shown stats is not in the pool for that card.
    let trap_pool = query_ids(json!({ "type": ["Trap", "Field Trap"] }));
    let aura_traps: Vec<String> = trap_pool
        .iter()
        .filter(|id| has_aura(&next, id))
        .cloned()
        .collect();
    let shown = if !aura_traps.is_empty() {
        shown_units(&next)
    } else {
        String::new()
    };
    let base_sampler = TrapSampler {
        trap_pool: &trap_pool,
        aura_traps: &aura_traps,
        shown: &shown,
        match_shown_cost: options.match_shown_cost.unwrap_or(true),
    };
    for side in [opp, seat] {
        // ME-ALTPLAY, R1046: while a face-down permission acts, a hidden backrow card may be a set
        // Unit or Spell, so that side's pool widens by the Unit and Spell pools.
        let grants = jackioh_engine::alt_play::grants_of(&next, side);
        let mut pool = trap_pool.clone();
        if grants.units {
            pool.extend(query_ids(json!({ "type": ["Unit"] })));
        }
        if grants.spells {
            pool.extend(query_ids(json!({ "type": ["Spell"] })));
        }
        let sampler = if grants.units || grants.spells {
            TrapSampler {
                trap_pool: &pool,
                aura_traps: &aura_traps,
                shown: &shown,
                match_shown_cost: options.match_shown_cost.unwrap_or(true),
            }
        } else {
            TrapSampler {
                trap_pool: base_sampler.trap_pool,
                aura_traps: base_sampler.aura_traps,
                shown: base_sampler.shown,
                match_shown_cost: base_sampler.match_shown_cost,
            }
        };
        // B5 E21: then the face-down cards dormant under each backrow pile, lane by lane.
        let mut slots: Vec<TrapSlot> = (0..next.players[side].backrow.len()).map(TrapSlot::Top).collect();
        for (lane, pile) in next.players[side].backrow_piles.iter().flatten().enumerate() {
            slots.extend((0..pile.len()).map(|depth| TrapSlot::Dormant(lane, depth)));
        }
        for slot in slots {
            let Some(card) = slot_card(&next, side, slot) else {
                continue;
            };
            if !is_placeholder(card) {
                continue;
            }
            let set_turn = card.summoned_turn.unwrap_or(next.turn);
            // R762: a top card shows its cost (R351); a dormant one beneath shows only that it is there (R447).
            let shown_cost = if matches!(slot, TrapSlot::Top(_)) {
                card.cost_override
            } else {
                None
            };
            let def_id = sampler.trap_for(&mut next, side, slot, shown_cost, &seen, &mut sampled, rng);
            set_slot_def(&mut next, side, slot, &def_id);
            stamp_sampled_set_as(&mut next, side, slot, &grants, set_turn, rng);
        }
        // R448: a card being set face-down waits in the resolving zone as a placeholder; it is a trap too.
        for at in 0..next.players[side].resolving.len() {
            let slot = TrapSlot::Resolving(at);
            let Some(card) = slot_card(&next, side, slot) else {
                continue;
            };
            if !is_placeholder(card) {
                continue;
            }
            let set_turn = card.summoned_turn.unwrap_or(next.turn);
            if slot_card(&next, side, slot).is_some_and(is_placeholder) {
                let def_id = sampler.trap_for(&mut next, side, slot, None, &seen, &mut sampled, rng);
                set_slot_def(&mut next, side, slot, &def_id);
                stamp_sampled_set_as(&mut next, side, slot, &grants, set_turn, rng);
            }
        }
    }

    // Step 4: the opponent's hand, then its library, in (sorted) order, from the non-token pool of
    // every set.
    let pool = query_ids(json!({ "excludeDefId": AI_DETERMINIZE.exclude_def_ids }));
    for at in 0..next.players[opp].hand.len() {
        if is_placeholder(&next.players[opp].hand[at]) {
            next.players[opp].hand[at].def_id = sample_def(&pool, &seen, &mut sampled, rng);
        }
    }
    for at in 0..next.players[opp].library.len() {
        if is_placeholder(&next.players[opp].library[at]) {
            next.players[opp].library[at].def_id = sample_def(&pool, &seen, &mut sampled, rng);
        }
    }

    // Step 5: the seat's own library — the opponent's cards in it sampled as in step 4, then shuffled.
    for at in 0..next.players[seat].library.len() {
        if is_placeholder(&next.players[seat].library[at]) {
            next.players[seat].library[at].def_id = sample_def(&pool, &seen, &mut sampled, rng);
        }
    }
    next.players[seat].library = rng.shuffle(&next.players[seat].library);

    // Step 6: every sampled card kept its instance id, owner, controller and zone above.
    next
}
