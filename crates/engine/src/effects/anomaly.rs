//! Anomalies and the Scanning Probes (Stinkyscape, issue #1855; the set plan's "Anomalies (M08)";
//! R1855000–R1855003): two verbs, proved on fixture scripts until the set's cards use them (S20).
//!
//! - `replace_board`: "exile your board and replace it with a precreated one"; once the whole board
//!   is laid each laid Unit's Cry fires, its choices drawn at random (R1855001, the designer's answer).
//!   Not `swap_board`, #87's exchange of the two sides' boards (`effects::swap`, R88, R171).
//! - `scan_for_anomaly`: the Scanning Probes' roll: a Radiant find, a plain one or a miss, each miss
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
use crate::faces::{card_type_of_face, running_face_of};
use crate::plague::permanents_on_field;
use crate::query::{luck_of, lucky_on};
use crate::resolve::lazy_part;
use crate::script::{Effect, EffectPart};
use crate::state::{CardInstance, GameState, find_instance_mut};
use crate::wire::{CardType, GameEvent, KeywordKind, PLAYER_IDS, Position, Row, TargetAim};
use crate::zones::{ZoneSlot, card_at, exile_to_pile};

use super::add_to_hand::{AddToHandArgs, add_to_hand};
use super::summon::{SummonPlacement, row_of, summon_fresh, summon_fresh_standing};
use super::targets::{BoardScope, ScopeSide, cards_in_scope_aimed};

/// One card of a preset board: its definition, the zone it is laid in, and how it lies there.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PresetCard {
    pub def_id: String,
    /// The card's own row (§5.1): an entry in the other row lays nothing, but a Unit with Flying named
    /// in the backrow stands there (R1081000, R1879001).
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
    /// R1879002: laid on top of the Unit an earlier entry of this replacement laid in its unit zone, a
    /// layered pile's later card (R1842000); in an empty zone it is laid as any entry, and on a card
    /// the replacement did not lay (a Dormant one, say) nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<bool>,
}

/// `replaceBoard`'s argument: the preset, laid in list order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ReplaceBoardArgs {
    pub board: Vec<PresetCard>,
}

/// R1855001: the mark on a laid Unit whose Cry is owed, its entry's place in the preset, taken off as
/// it fires, so the Cry finds the card the preset laid wherever it stands by then.
pub const PRESET_CRY_OWED_KEY: &str = "presetCryOwed";

/// R1855001: take every owed-Cry mark off the field, so one an earlier replacement left (on a card a
/// Stack buried) is never read as this replacement's.
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

/// R1855000–R1855002: the part's list (the exile, one lay per entry, one Cry per entry) is built from
/// `args` alone, so a rebuild after a pause resumes where it stopped (R113).
pub fn replace_board(args: ReplaceBoardArgs) -> Effect {
    lazy_part("replaceBoard", move |ctx, _memo| {
        let player = ctx.controller;
        let source_id = ctx.self_.as_ref().map(|me| me.id.clone());
        let mut effects = vec![Effect::new("replaceBoardExile", move |at| {
            at.events.push(GameEvent::BoardReplaced { player });
            clear_owed_cries(at.state);
            // R1855000: Dormant cards (R1839000) and ones immune to the card (E35) stay. A card beneath
            // a Stack is reached on a later pass; `seen` ends the walk on a card that does not leave.
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
                let card_type = card_type_of_face(at.state, &entry.def_id, radiant);
                let is_unit = card_type == CardType::Unit;
                // R1879001: a Flying Unit named in the backrow stands there, as a Flying play may.
                let stands =
                    entry.row == Row::Backrow && is_unit && prints_flying(at.state, &entry.def_id, radiant);
                if row_of(card_type) != Some(entry.row) && !stands {
                    return;
                }
                let slot = ZoneSlot {
                    player,
                    row: entry.row,
                    lane: entry.lane,
                };
                // R1879002: on top only of a card this replacement laid, which still bears its mark.
                let stack = entry.stack == Some(true)
                    && entry.row == Row::Units
                    && card_at(at.state, slot)
                        .is_some_and(|top| top.memory.contains_key(PRESET_CRY_OWED_KEY));
                let placement = SummonPlacement {
                    lane: Some(entry.lane),
                    stack: stack.then_some(true),
                    radiant: entry.radiant,
                    source_id: source_id.clone(),
                    ..SummonPlacement::default()
                };
                // R1855000: a summon (R1), into the named zone or nowhere (R47, R64, R688).
                let laid = if stands {
                    summon_fresh_standing(at, &entry.def_id, player, &placement)
                } else {
                    summon_fresh(at, &entry.def_id, player, &placement)
                };
                let Some(card) = laid else {
                    return;
                };
                if let Some(live) = find_instance_mut(at.state, &card.id) {
                    for (key, value) in entry.memory.iter().flatten() {
                        live.memory.insert(key.clone(), value.clone());
                    }
                    if is_unit {
                        live.memory.insert(PRESET_CRY_OWED_KEY.to_string(), json!(index));
                    }
                }
                if is_unit && entry.position == Some(Position::Def) {
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
        // One per entry: only an entry that laid a Unit left a mark to find (R1879001).
        effects.extend(args.board.iter().enumerate().map(|(index, _)| {
            Effect::new("replaceBoardCry", move |at| {
                // R1855001: a card that has left the field lost its mark with its memory (R78, R174).
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
                // R1855001: a triggered Cry (R467), every choice random (R452's), across a pause.
                trigger_random_cry_of(&mut at.sink, &card, player);
            })
        }));
        EffectPart { effects, memo: None }
    })
}

/// R1879001: whether a face of `def_id` prints Flying.
fn prints_flying(state: &GameState, def_id: &str, radiant: bool) -> bool {
    running_face_of(state, def_id, radiant)
        .keywords
        .iter()
        .any(|keyword| keyword.kind() == KeywordKind::Flying)
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

/// R1855003: what `roll` finds after `misses` misses: the lowest rolls Radiant, the next plain.
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

/// R1855003: the Scanning Probes' one roll, a luck-based one (R1438): the card's Lucky and its
/// controller's Luck roll again and keep the best. A miss counts for the controller
/// (`PlayerState.anomaly_misses`), silently; a find adds the card and keeps the count.
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
