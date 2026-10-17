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
    CardInstance, CatalogQueryArgs, CostOptions, GameState, PLAYER_IDS, PerPlayer, PlayerId, RevealAt, Rng,
    SecretChoice, SetAs, active_units_of, effective_cost, query, scripts_for, unit_view,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::AI_DETERMINIZE;
use crate::observe::HIDDEN_DEF_ID;
use crate::shadow_ban::SHADOW_BAN_IDS;

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
    /// A dealt AI deck holds none of R186's shadow-banned cards, so a world consistent with the game
    /// being played holds none either: the hidden-card pools drop SHADOW_BAN_IDS, and the seat's own
    /// library, dealt singletons (`build_ai_deck`), samples no second copy of a def the seat already
    /// shows. Default true; the greedy baseline passes false so its sampler stays put.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consistent_pools: Option<bool>,
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

/// R783009, R1046: stamp a sampled card that may be set by its Cloaked (`cloakable`) as a set card
/// (`cloaked`, which has no timing), with no rng draw. Any other sampled card is a Trap or a Field
/// Trap and stays unstamped.
fn stamp_sampled_set_as(
    state: &mut GameState,
    side: PlayerId,
    slot: TrapSlot,
    cloakable: &[String],
    set_turn: i32,
) {
    let Some(def_id) = slot_card(state, side, slot).map(|card| card.def_id.clone()) else {
        return;
    };
    if !cloakable.contains(&def_id) {
        return;
    }
    let Some(card) = slot_card_mut(state, side, slot) else {
        return;
    };
    card.set_as = Some(SetAs {
        reveal: RevealAt::Cloaked,
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

/// What a determinization reads off `public_state` before its first draw: the opponent's shown def
/// ids (step 2's `seen`), the trap pools, the aura baseline `shown` and each side's widened slot
/// pool (step 3's head), and step 4/5's card pool. Every determinization of one decision starts from
/// the same `public_state`, so all of it is identical call to call; `decide` computes it once
/// (`det_ground`) and hands it to each (`determinize_on`). A side's `cloakable` reads only the auras
/// that reach that side's own hand (R783007's `cloak_hand_aura` applies where `player == controller`),
/// so the walk's sampling of the other side's slots first does not change it.
pub struct DetGround {
    seen: IndexSet<String>,
    /// The seat's own shown def ids (DeterminizeOptions.consistent_pools' `seen_own`): a dealt deck
    /// holds one of a def, so the seat's own library holds no second copy of one it already shows.
    seen_own: IndexSet<String>,
    trap_pool: Vec<String>,
    aura_traps: Vec<String>,
    shown: String,
    cloakable: PerPlayer<Vec<String>>,
    slot_pool: PerPlayer<Vec<String>>,
    card_pool: Vec<String>,
    /// `trap_pool`, `slot_pool` and `card_pool` minus SHADOW_BAN_IDS (consistent_pools): a dealt AI
    /// deck can hold none of them, so a consistent world samples none.
    open_trap_pool: Vec<String>,
    open_slot_pool: PerPlayer<Vec<String>>,
    open_card_pool: Vec<String>,
}

/// The `DetGround` of a public state: step 2's `seen`, step 3's pools and aura baseline, steps 4/5's
/// card pool — every read made on the state as it was handed in, before any sampling.
pub fn det_ground(public_state: &GameState, seat: PlayerId) -> DetGround {
    let opp = seat.opponent();
    let mut seen: IndexSet<String> = IndexSet::new();
    let mut seen_own: IndexSet<String> = IndexSet::new();
    for card in all_cards(public_state) {
        if is_placeholder(card) {
            continue;
        }
        if card.owner == opp {
            seen.insert(card.def_id.clone());
        }
        if card.owner == seat {
            seen_own.insert(card.def_id.clone());
        }
    }
    let trap_pool = query_ids(json!({ "type": ["Trap", "Field Trap"] }));
    let aura_traps: Vec<String> = trap_pool
        .iter()
        .filter(|id| has_aura(public_state, id))
        .cloned()
        .collect();
    let shown = if !aura_traps.is_empty() {
        shown_units(public_state)
    } else {
        String::new()
    };
    // R783009: a hidden backrow card may also be a card set by its Cloaked — one that prints it, or
    // one an aura lends it to in that side's hand now.
    let printed_cloaked = jackioh_engine::alt_play::printed_cloaked_def_ids();
    let mut cloakable: PerPlayer<Vec<String>> = PerPlayer::default();
    let mut slot_pool: PerPlayer<Vec<String>> = PerPlayer::default();
    for side in PLAYER_IDS {
        // R1046: Knowledge Breaker's and Paranoia's auras lend Cloaked to a hand's Units and Spells,
        // so while one acts those cards are `lent_cloaked_def_ids` too.
        let mut side_cloakable = printed_cloaked.clone();
        for id in jackioh_engine::alt_play::lent_cloaked_def_ids(public_state, side) {
            if !side_cloakable.contains(&id) {
                side_cloakable.push(id);
            }
        }
        let mut pool = trap_pool.clone();
        for id in &side_cloakable {
            if !pool.contains(id) {
                pool.push(id.clone());
            }
        }
        cloakable[side] = side_cloakable;
        slot_pool[side] = pool;
    }
    let card_pool = query_ids(json!({ "excludeDefId": AI_DETERMINIZE.exclude_def_ids }));
    let unbanned = |ids: &[String]| -> Vec<String> {
        let open: Vec<String> = ids
            .iter()
            .filter(|id| !SHADOW_BAN_IDS.contains(&id.as_str()))
            .cloned()
            .collect();
        // A pool banned whole samples as before rather than going dry.
        if open.is_empty() { ids.to_vec() } else { open }
    };
    let open_trap_pool = unbanned(&trap_pool);
    let mut open_slot_pool: PerPlayer<Vec<String>> = PerPlayer::default();
    for side in PLAYER_IDS {
        open_slot_pool[side] = unbanned(&slot_pool[side]);
    }
    let open_card_pool = unbanned(&card_pool);
    DetGround {
        seen,
        seen_own,
        trap_pool,
        aura_traps,
        shown,
        cloakable,
        slot_pool,
        card_pool,
        open_trap_pool,
        open_slot_pool,
        open_card_pool,
    }
}

/// R185: one concrete world consistent with `public_state` (the output of redact). Pure given rng.
pub fn determinize(
    public_state: &GameState,
    seat: PlayerId,
    rng: &mut Rng,
    options: DeterminizeOptions,
) -> GameState {
    determinize_on(public_state, seat, rng, options, &det_ground(public_state, seat))
}

/// `determinize` on a `DetGround` already computed: the same world `determinize` would build.
pub fn determinize_on(
    public_state: &GameState,
    seat: PlayerId,
    rng: &mut Rng,
    options: DeterminizeOptions,
    ground: &DetGround,
) -> GameState {
    let opp = seat.opponent();
    let mut next = public_state.clone();

    // Step 1: a stream the match never uses.
    next.seed = format!("ai:{}", int_2_31(rng));
    next.rng_cursor = 0;

    // Step 2: what the opponent has shown, in any zone (det_ground's `seen`).
    let seen = &ground.seen;
    let mut sampled: IndexSet<String> = IndexSet::new();

    let consistent = options.consistent_pools.unwrap_or(true);

    // Step 3: face-down backrow placeholders, in lane order, from the Trap and Field Trap pool. R602: a
    // face-down Trap's aura is live (R403) and the units it changes are on the board for the seat to read,
    // so a candidate whose aura would change any unit's shown stats is not in the pool for that card.
    let base_sampler = TrapSampler {
        trap_pool: if consistent {
            &ground.open_trap_pool
        } else {
            &ground.trap_pool
        },
        aura_traps: &ground.aura_traps,
        shown: &ground.shown,
        match_shown_cost: options.match_shown_cost.unwrap_or(true),
    };
    for side in [opp, seat] {
        let cloakable = &ground.cloakable[side];
        let pool = if consistent {
            &ground.open_slot_pool[side]
        } else {
            &ground.slot_pool[side]
        };
        let sampler = if !cloakable.is_empty() {
            TrapSampler {
                trap_pool: pool,
                aura_traps: &ground.aura_traps,
                shown: &ground.shown,
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
            let def_id = sampler.trap_for(&mut next, side, slot, shown_cost, seen, &mut sampled, rng);
            set_slot_def(&mut next, side, slot, &def_id);
            stamp_sampled_set_as(&mut next, side, slot, cloakable, set_turn);
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
                let def_id = sampler.trap_for(&mut next, side, slot, None, seen, &mut sampled, rng);
                set_slot_def(&mut next, side, slot, &def_id);
                stamp_sampled_set_as(&mut next, side, slot, cloakable, set_turn);
            }
        }
    }

    // Step 4: the opponent's hand, then its library, in (sorted) order, from the non-token pool of
    // every set (det_ground's card_pool; consistent_pools' open_card_pool).
    let pool = if consistent {
        &ground.open_card_pool
    } else {
        &ground.card_pool
    };
    for at in 0..next.players[opp].hand.len() {
        if is_placeholder(&next.players[opp].hand[at]) {
            next.players[opp].hand[at].def_id = sample_def(pool, seen, &mut sampled, rng);
        }
    }
    for at in 0..next.players[opp].library.len() {
        if is_placeholder(&next.players[opp].library[at]) {
            next.players[opp].library[at].def_id = sample_def(pool, seen, &mut sampled, rng);
        }
    }

    // Step 5: the seat's own library — the opponent's cards in it sampled as in step 4, then
    // shuffled. A card the seat owns is dealt one copy (consistent_pools): the seat's own library
    // holds no second copy of a def it already shows (`seen_own`), while one of the opponent's in
    // it samples as in step 4 (`seen`).
    for at in 0..next.players[seat].library.len() {
        let card = &next.players[seat].library[at];
        if is_placeholder(card) {
            let seen_for = if consistent && card.owner == seat {
                &ground.seen_own
            } else {
                seen
            };
            next.players[seat].library[at].def_id = sample_def(pool, seen_for, &mut sampled, rng);
        }
    }
    next.players[seat].library = rng.shuffle(&next.players[seat].library);

    // R865, the last draw: every dropped secret choice becomes one of the three, uniformly, so no
    // simulation reads the true one.
    if let Some(secrets) = next.secrets.as_mut() {
        for secret in secrets.iter_mut() {
            if secret.choice.is_none() {
                secret.choice = Some(SecretChoice::ALL[rng.int(3) as usize]);
            }
        }
    }

    // Step 6: every sampled card kept its instance id, owner, controller and zone above.
    next
}
