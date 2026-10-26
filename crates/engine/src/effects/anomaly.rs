//! Anomalies and the Scanning Probes (Stinkyscape, issue #1855; the set plan's "Anomalies (M08)";
//! R1855000–R1855003): two verbs, proved on fixture scripts until the set's cards use them (S20).
//!
//! - `replace_board` — "exile your board and replace it with a precreated one": what an effect of the
//!   card can reach on its caster's side is exiled (a Dormant card and one the card cannot reach stay,
//!   and the preset lays nothing in their zones), the preset is summoned into its named zones, each
//!   entry Radiant or in Defense Position as it says and carrying its riders in its memory (R1855000,
//!   R1855002), and once the whole board is laid each laid Unit's Cry fires, its caster's choices drawn
//!   at random (R1855001, the designer's answer). Not `swap_board`, the name #87's exchange of the
//!   two sides' boards already has (`effects::swap`, R88, R171).
//! - `scan_for_anomaly` — the Scanning Probes' roll: a Radiant find, a plain one or a miss, each miss
//!   raising that player's later chances (R1855003).

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::combat::{SwitchPositionOptions, switch_position};
use crate::config::{
    ANOMALY_PLAIN_CHANCE, ANOMALY_PLAIN_STEP, ANOMALY_RADIANT_CHANCE, ANOMALY_RADIANT_STEP,
    ANOMALY_ROLL_SIDES,
};
use crate::cry_trigger::trigger_random_cry_of;
use crate::faces::card_type_of_face;
use crate::plague::permanents_on_field;
use crate::query::{luck_of, lucky_on};
use crate::resolve::lazy_part;
use crate::script::{Effect, EffectPart};
use crate::state::{CardInstance, GameState, find_instance_mut};
use crate::wire::{GameEvent, PLAYER_IDS, Position, Row, TargetAim};
use crate::zones::exile_to_pile;

use super::add_to_hand::{AddToHandArgs, add_to_hand};
use super::summon::{SummonPlacement, row_of, summon_fresh};
use super::targets::{BoardScope, ScopeSide, cards_in_scope_aimed};

/// One card of a preset board: its definition, the zone it is laid in, and how it lies there.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PresetCard {
    pub def_id: String,
    /// The card's own row (§5.1): an entry in the other row lays nothing.
    pub row: Row,
    pub lane: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    /// A Unit laid in Defense Position (`DEF`) is switched there by the effect (R20); Attack is the
    /// default a summon lands in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    /// R1855002: riders the card's own text reads, written into its memory as it lands (R862's shape;
    /// `effects::tray::RESOURCE_POOL_KEY` is one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<IndexMap<String, Value>>,
}

/// `replaceBoard`'s argument: the preset, laid in list order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ReplaceBoardArgs {
    pub board: Vec<PresetCard>,
}

/// R1855001: the memory key on a laid Unit whose Cry has not fired yet, holding its entry's place in
/// the preset and taken off as it fires, so the card that fires is the one the preset laid wherever it
/// stands by then.
pub const PRESET_CRY_OWED_KEY: &str = "presetCryOwed";

/// R1855001: take every owed-Cry mark off the field, so one an earlier replacement left on a card it
/// laid (buried by a Stack before its Cry, say) is never read as this replacement's.
fn clear_owed_cries(state: &mut GameState) {
    for player in PLAYER_IDS {
        let side = &mut state.players[player];
        let cards = side
            .units
            .iter_mut()
            .flatten()
            .flatten()
            .chain(side.backrow.iter_mut().flatten())
            .chain(side.backrow_piles.iter_mut().flatten().flatten())
            .chain(side.carried.iter_mut().flatten().flatten());
        for card in cards {
            card.memory.shift_remove(PRESET_CRY_OWED_KEY);
        }
    }
}

/// R1855000–R1855002: replace the caster's board with `args.board`. The part's list is built from
/// `args` alone, so a rebuild after a pause is the same list and resumes where it stopped (R113): the
/// exile, one lay per entry, then one Cry per unit-row entry, each finding its card by its mark.
pub fn replace_board(args: ReplaceBoardArgs) -> Effect {
    lazy_part("replaceBoard", move |ctx, _memo| {
        let player = ctx.controller;
        let source_id = ctx.self_.as_ref().map(|me| me.id.clone());
        let mut effects = vec![Effect::new("replaceBoardExile", move |at| {
            at.events.push(GameEvent::BoardReplaced { player });
            clear_owed_cries(at.state);
            // R1855000: what an effect of the card reaches on its side, Dormant cards (R1839000) and
            // the ones immune to it (E35) passed by. A card beneath a Stack is reached on a later pass
            // once the card above it has gone; `seen` ends the walk on a card that does not leave.
            let scope = BoardScope {
                side: Some(ScopeSide::SelfSide),
                rows: Some(vec![Row::Units, Row::Backrow]),
                ..BoardScope::default()
            };
            let mut seen: IndexSet<String> = IndexSet::new();
            loop {
                let fresh: Vec<CardInstance> = cards_in_scope_aimed(at, &scope, TargetAim::Harm)
                    .into_iter()
                    .filter(|card| seen.insert(card.id.clone()))
                    .collect();
                if fresh.is_empty() {
                    break;
                }
                for card in fresh {
                    exile_to_pile(at, card);
                }
            }
        })];
        effects.extend(args.board.iter().enumerate().map(|(index, entry)| {
            let entry = entry.clone();
            let source_id = source_id.clone();
            Effect::new("replaceBoardLay", move |at| {
                let radiant = entry.radiant == Some(true);
                if row_of(card_type_of_face(at.state, &entry.def_id, radiant)) != Some(entry.row) {
                    return;
                }
                let placement = SummonPlacement {
                    lane: Some(entry.lane),
                    radiant: entry.radiant,
                    source_id: source_id.clone(),
                    ..SummonPlacement::default()
                };
                // R1855000: a summon (R1), into the named zone or nowhere (R47, R64, R688).
                let Some(card) = summon_fresh(at, &entry.def_id, player, &placement) else {
                    return;
                };
                if let Some(live) = find_instance_mut(at.state, &card.id) {
                    for (key, value) in entry.memory.iter().flatten() {
                        live.memory.insert(key.clone(), value.clone());
                    }
                    if entry.row == Row::Units {
                        live.memory.insert(PRESET_CRY_OWED_KEY.to_string(), json!(index));
                    }
                }
                if entry.row == Row::Units && entry.position == Some(Position::Def) {
                    let _ = switch_position(
                        at,
                        &card,
                        SwitchPositionOptions {
                            spend_exertion: Some(false),
                            to: Some(Position::Def),
                        },
                    );
                }
            })
        }));
        effects.extend(
            args.board
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.row == Row::Units)
                .map(|(index, _)| {
                    Effect::new("replaceBoardCry", move |at| {
                        // R1855001: the card this entry laid, wherever it acts now; one that has left
                        // the field lost its mark with the rest of its memory (R78, R174).
                        let mark = json!(index);
                        let Some(id) = permanents_on_field(at.state, player)
                            .into_iter()
                            .find(|card| card.memory.get(PRESET_CRY_OWED_KEY) == Some(&mark))
                            .map(|card| card.id.clone())
                        else {
                            return;
                        };
                        let Some(card) = find_instance_mut(at.state, &id).map(|card| {
                            card.memory.shift_remove(PRESET_CRY_OWED_KEY);
                            card.clone()
                        }) else {
                            return;
                        };
                        // R1855001: a triggered Cry (R467) whose caster's every choice is random
                        // (R452's), before a pause and after one.
                        trigger_random_cry_of(&mut at.sink, &card, player);
                    })
                }),
        );
        EffectPart { effects, memo: None }
    })
}

/// R1855003: what a scan finds, worst first, so a Lucky roll keeps the greatest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnomalyScan {
    Miss,
    Plain,
    Radiant,
}

/// R1855003: a player's Radiant and plain chances, out of `ANOMALY_ROLL_SIDES`, after `misses` misses:
/// each miss adds its step to both, and the two never sum past the roll.
pub fn anomaly_chances(misses: i32) -> (i32, i32) {
    let misses = misses.max(0);
    let radiant = ANOMALY_RADIANT_CHANCE
        .saturating_add(misses.saturating_mul(ANOMALY_RADIANT_STEP))
        .clamp(0, ANOMALY_ROLL_SIDES);
    let plain = ANOMALY_PLAIN_CHANCE
        .saturating_add(misses.saturating_mul(ANOMALY_PLAIN_STEP))
        .clamp(0, ANOMALY_ROLL_SIDES - radiant);
    (radiant, plain)
}

/// R1855003: what a roll of `roll` (0 to `ANOMALY_ROLL_SIDES` − 1) finds after `misses` misses: the
/// lowest rolls the Radiant one, the next the plain one.
pub fn anomaly_scan_of(roll: i32, misses: i32) -> AnomalyScan {
    let (radiant, plain) = anomaly_chances(misses);
    if roll < radiant {
        AnomalyScan::Radiant
    } else if roll < radiant + plain {
        AnomalyScan::Plain
    } else {
        AnomalyScan::Miss
    }
}

/// `scanForAnomaly`'s argument: the card a find adds to the hand (Anomaly Found!).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ScanForAnomalyArgs {
    pub def_id: String,
}

/// R1855003: the Scanning Probes' "5% chance to add a Radiant Anomaly Found! to your hand, 20% chance
/// to add an Anomaly Found!, else future Scanning Probes have +1% and +4%": one roll, a luck-based one
/// (R1438), so the card's Lucky and its controller's Luck roll again and keep the best. A miss counts
/// for the controller (`PlayerState.anomaly_misses`), silently; a find adds the card and keeps the
/// count.
pub fn scan_for_anomaly(args: ScanForAnomalyArgs) -> Effect {
    Effect::new("scanForAnomaly", move |ctx| {
        let player = ctx.controller;
        let misses = ctx.sink.state.players[player].anomaly_misses.unwrap_or(0);
        let lucky = ctx
            .self_
            .as_ref()
            .map_or(0, |card| lucky_on(ctx.sink.state, card))
            + luck_of(ctx.sink.state, player);
        let found = ctx.sink.rng.lucky(
            lucky,
            |rng| anomaly_scan_of(rng.int(ANOMALY_ROLL_SIDES), misses),
            |a, b| a.max(b),
        );
        if found == AnomalyScan::Miss {
            ctx.sink.state.players[player].anomaly_misses = Some(misses.saturating_add(1));
            return;
        }
        let add = add_to_hand(AddToHandArgs {
            def_id: Some(args.def_id.clone()),
            radiant: (found == AnomalyScan::Radiant).then_some(true),
            ..AddToHandArgs::default()
        });
        (add.apply)(ctx);
    })
}
