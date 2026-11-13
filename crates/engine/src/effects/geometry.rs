//! Board geometry and damage routing verbs (Stinkyscape+, issue #3554, R3554000–R3554009):
//! the Black Hole, the slashed board, line damage, chain lightning and damage chains. Card scripts
//! return these effects and never mutate state (CLAUDE.md rule 5).

use serde::{Deserialize, Serialize};

use crate::config::{BLACK_HOLE_LANE, SLASH_BASE_LANE, SLASH_LANES, UNIT_ZONES};
use crate::damage::{DamageArgs, DamageFlags, DamageTarget, deal_damage};
use crate::effects::choose::{ChooseModeArgs, choose_mode, chosen_options};
use crate::effects::targets::{
    BoardScope, PlayerSpec, ScopeSide, TargetSpec, cards_in_scope, resolve_target,
};
use crate::geometry::{BlackHole, ChainKind};
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance, find_instance_mut};
use crate::wire::{CardType, GameEvent, KeywordKind, PLAYER_IDS, Row, Zone, ZoneRef, opponent_of};

/// `openBlackHole`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OpenBlackHoleArgs {
    /// The Radiant face pulls only the opponent's permanents, at the start of their turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opponent_only: Option<bool>,
    /// R3561000: the permanents it closes after (Black Hole's `permanents`); `None`: the engine's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<i32>,
}

/// R3554000 (S+ #77 Black Hole): open a Black Hole in the middle lane. A new hole replaces an open
/// one, restarting the count.
pub fn open_black_hole(args: OpenBlackHoleArgs) -> Effect {
    Effect::new("openBlackHole", move |ctx| {
        let pulls = if args.opponent_only == Some(true) {
            Some(opponent_of(ctx.controller))
        } else {
            None
        };
        ctx.state.black_hole = Some(BlackHole {
            capacity: args.capacity,
            ..BlackHole::new(BLACK_HOLE_LANE, pulls)
        });
    })
}

/// `slashBoard`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SlashBoardArgs {
    /// The lane to slash: one of `SLASH_LANES`, defaulting to `SLASH_BASE_LANE`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<i32>,
}

/// R3554002 (S+ #78 Slashed Dream Cries in Red): exile the lane with every permanent in it
/// (Dormant cards included, a covering GIGA by its anchor), and void it for the rest of the game.
/// A second slash does nothing, and neither does a lane outside `SLASH_LANES`.
pub fn slash_board(args: SlashBoardArgs) -> Effect {
    Effect::new("slashBoard", move |ctx| {
        if ctx.state.slashed_lane.is_some() {
            return;
        }
        let lane = args.lane.unwrap_or(SLASH_BASE_LANE);
        if !SLASH_LANES.contains(&lane) {
            return;
        }
        let mut met: Vec<String> = Vec::new();
        for player in PLAYER_IDS {
            for row in [Row::Units, Row::Backrow] {
                let slot = crate::zones::ZoneSlot { player, row, lane };
                // A covering GIGA goes by its anchor, once; Dormant cards go too.
                if let Some(found) = crate::giga::covering(ctx.state, slot).cloned()
                    && !met.contains(&found.id)
                {
                    met.push(found.id.clone());
                    if let Some(at) = crate::zones::slot_of(ctx.state, &found) {
                        crate::geometry::exile_zone(ctx, at);
                    }
                    continue;
                }
                crate::geometry::exile_zone(ctx, slot);
            }
        }
        ctx.state.slashed_lane = Some(lane);
    })
}

/// R3554002: the Radiant face's pick of the slash lane, a `mode` prompt over `SLASH_LANES` so
/// `legal_actions`, the AI and the timeout answer it too.
pub fn ask_slash_lane(step: impl Into<String>) -> Effect {
    let options: Vec<String> = SLASH_LANES.iter().map(|lane| lane.to_string()).collect();
    let labels = SLASH_LANES
        .iter()
        .map(|lane| (lane.to_string(), format!("Lane {lane}")))
        .collect();
    choose_mode(ChooseModeArgs {
        options,
        step: step.into(),
        prompt: Some("Slash the board: choose the lane".to_string()),
        data: None,
        by: None,
        labels: Some(labels),
    })
}

/// R3554002: the slash lane an answer picked, if any.
pub fn chosen_slash_lane(ctx: &EffectContext<'_>) -> Option<i32> {
    chosen_options(ctx)
        .iter()
        .filter_map(|option| option.parse::<i32>().ok())
        .find(|lane| SLASH_LANES.contains(lane))
}

/// R3554005 (#109 Lancer, #126 Vela, #127 Corvus): the rules a line runs under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineRules {
    /// The damage each Unit on the line takes.
    pub amount: i32,
    /// How many enemy tiles are offered (`None`: every one; Lancer Discovers from
    /// `DISCOVER_OPTIONS`).
    pub offers: Option<i32>,
    /// Vela's and Corvus's Radiant faces hit each neighbour too, `via` Cleave.
    pub cleave: bool,
    /// Corvus draws the line as a laser.
    pub laser: bool,
    /// Whether each hit pierces Armor: Lancer, Vela and Corvus print Pierce; KY's Line Search Bot (S+ T-AI-7) does not (R3571000).
    pub pierce: bool,
}

/// R3554005: the resume step the tile answer re-enters.
pub const LINE_STEP: &str = "lineTile";

/// R3554005: a cell as the prompt names it, `"x,y"` from the caster's seat.
fn line_option(x: i32, y: i32) -> String {
    format!("{x},{y}")
}

/// R3554005: the cell an option names, if any.
fn option_point(option: &str) -> Option<(i32, i32)> {
    let (x, y) = option.split_once(',')?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

/// R3554005: the Discover of the enemy tile, drawn as the prompt opens: the grid's enemy cells,
/// minus the void lane and any lane outside the source's half.
pub fn line_damage_begin(rules: LineRules) -> Vec<Effect> {
    vec![Effect::new("lineTile", move |ctx| {
        let half = ctx
            .self_
            .as_ref()
            .and_then(|source| crate::geometry::half_of_card(ctx.state, source));
        let void = ctx.state.slashed_lane;
        let mut free: Vec<(i32, i32)> = Vec::new();
        for lane in 1..=UNIT_ZONES {
            if void == Some(lane) || !crate::geometry::lane_in_half(ctx.state, lane, half) {
                continue;
            }
            // y 2 and 3 from the caster's seat: the enemy Units, then the enemy backrow.
            free.push((lane - 1, 2));
            free.push((lane - 1, 3));
        }
        let mut offered: Vec<(i32, i32)> = match rules.offers {
            Some(n) => ctx
                .sink
                .rng
                .shuffle(&free)
                .into_iter()
                .take(n.max(0) as usize)
                .collect(),
            None => free,
        };
        offered.sort();
        let options: Vec<String> = offered.iter().map(|(x, y)| line_option(*x, *y)).collect();
        let labels = offered
            .iter()
            .map(|(x, y)| {
                let side = if *y == 2 { "enemy Units" } else { "enemy backrow" };
                (line_option(*x, *y), format!("Lane {}, {side}", x + 1))
            })
            .collect();
        let ask = choose_mode(ChooseModeArgs {
            options,
            step: LINE_STEP.to_string(),
            prompt: Some("Discover an enemy tile".to_string()),
            data: None,
            by: None,
            labels: Some(labels),
        });
        (ask.apply)(ctx);
    })]
}

/// Whether `card` is a Unit the line meets: a Unit face on the field, not Dormant.
fn is_line_unit(state: &crate::state::GameState, card: &CardInstance) -> bool {
    crate::zones::is_unit_face(state, card) && !crate::slumber::is_slumbering(card)
}

/// R3554005: the answer: the line between the source and the tile, drawn as KY's papaya draws it,
/// hitting every Unit on it — friend or foe — piercing Armor when `pierce`, and with `cleave` each
/// neighbour too.
pub fn line_damage_answered(rules: LineRules, ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(source) = ctx.live_self().cloned() else {
        return Vec::new();
    };
    let Some(at) = crate::zones::slot_of(ctx.state, &source) else {
        return Vec::new();
    };
    let Some((x, y)) = chosen_options(ctx)
        .iter()
        .filter_map(|option| option_point(option))
        .next()
    else {
        return Vec::new();
    };
    let to = crate::subsystems::papaya::zone_of_point(
        ctx.controller,
        &crate::subsystems::papaya::PapayaPoint { x, y },
    );
    let from = ZoneRef {
        player: at.player,
        row: at.row,
        lane: at.lane,
    };
    let cells = crate::geometry::line_cells(&from, &to);
    let mut drawn: Vec<ZoneRef> = vec![from];
    drawn.extend(cells.iter().copied());
    let player = ctx.controller;
    let laser = rules.laser;
    let mut effects = vec![Effect::new("lineDrawn", move |ctx| {
        ctx.events.push(GameEvent::LineDrawn {
            player,
            cells: drawn.clone(),
            laser: laser.then_some(true),
        });
    })];
    // The Units on the line, each once, in lane order.
    let mut ids: Vec<String> = Vec::new();
    for cell in cells {
        let slot = crate::zones::ZoneSlot {
            player: cell.player,
            row: cell.row,
            lane: cell.lane,
        };
        if let Some(found) = crate::giga::occupant_at(ctx.state, slot).cloned()
            && is_line_unit(ctx.state, &found)
            && !ids.contains(&found.id)
        {
            ids.push(found.id.clone());
        }
    }
    let amount = rules.amount;
    let cleave = rules.cleave;
    let pierce = rules.pierce;
    effects.push(Effect::new("lineDamage", move |ctx| {
        let Some(source) = ctx.live_self().cloned().or(ctx.self_.clone()) else {
            return;
        };
        let mut hit: Vec<String> = Vec::new();
        for id in &ids {
            let Some(live) = find_instance(ctx.state, id).cloned() else {
                continue;
            };
            if !is_line_unit(ctx.state, &live) {
                continue;
            }
            deal_damage(
                ctx,
                DamageArgs {
                    source: Some(source.clone()),
                    target: DamageTarget::Unit {
                        instance: live.clone(),
                    },
                    amount,
                    flags: Some(DamageFlags {
                        ignore_armor: pierce.then_some(true),
                        ..DamageFlags::default()
                    }),
                },
            );
            hit.push(live.id.clone());
            if !cleave {
                continue;
            }
            // Vela's and Corvus's Radiant faces: each neighbour not yet hit, `via` Cleave.
            let mut neighbours: Vec<CardInstance> = Vec::new();
            for slot in crate::giga::adjacent_slots(ctx.state, &live) {
                if let Some(found) = crate::giga::occupant_at(ctx.state, slot).cloned()
                    && is_line_unit(ctx.state, &found)
                    && crate::zones::is_adjacent_card(ctx.state, &live, &found)
                    && !hit.contains(&found.id)
                    && !ids.contains(&found.id)
                {
                    neighbours.push(found);
                }
            }
            for found in neighbours {
                deal_damage(
                    ctx,
                    DamageArgs {
                        source: Some(source.clone()),
                        target: DamageTarget::Unit {
                            instance: found.clone(),
                        },
                        amount,
                        flags: Some(DamageFlags {
                            ignore_armor: pierce.then_some(true),
                            via: Some(KeywordKind::Cleave),
                            ..DamageFlags::default()
                        }),
                    },
                );
                hit.push(found.id.clone());
            }
        }
    }));
    effects
}

/// `chainLightning`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ChainLightningArgs {
    pub to: TargetSpec,
    pub amount: i32,
}

/// The slots a chain may jump to from `unit`: its same-row neighbours and the other row of its
/// lanes, read off the unit as it stood when hit.
fn chain_slots(state: &crate::state::GameState, unit: &CardInstance) -> Vec<crate::zones::ZoneSlot> {
    let mut slots = crate::giga::adjacent_slots(state, unit);
    if let Zone::Field { player, row, .. } = unit.zone {
        let other = crate::giga::other_row(row);
        for lane in crate::giga::lanes_of(unit) {
            slots.push(crate::zones::ZoneSlot {
                player,
                row: other,
                lane,
            });
        }
    }
    slots
}

/// R3554006 (#38 Arc): deal `amount` to a Unit, then jump at random to an adjacent Unit — the next
/// lane or the backrow behind it — never the same Unit twice in one Activate, until no Unit is left
/// to jump to: each jump goes to an unhit Unit beside any Unit hit so far, so the whole connected
/// component is hit whatever the draws. Every draw comes from the game's Rng.
pub fn chain_lightning(args: ChainLightningArgs) -> Effect {
    Effect::new("chainLightning", move |ctx| {
        let Some(DamageTarget::Unit { instance }) = resolve_target(ctx, &args.to) else {
            return;
        };
        let source = ctx.self_.clone();
        let mut struck: Vec<CardInstance> = Vec::new();
        let mut next = Some(instance);
        while let Some(last) = next {
            if let Some(live) = find_instance(ctx.state, &last.id).cloned()
                && is_line_unit(ctx.state, &live)
            {
                deal_damage(
                    ctx,
                    DamageArgs {
                        source: source.clone(),
                        target: DamageTarget::Unit { instance: live },
                        amount: args.amount,
                        flags: None,
                    },
                );
            }
            // The unit as it stood when hit anchors the next jumps, dead or alive.
            struck.push(last);
            let hit: Vec<&str> = struck.iter().map(|unit| unit.id.as_str()).collect();
            let mut candidates: Vec<CardInstance> = Vec::new();
            for done in &struck {
                for slot in chain_slots(ctx.state, done) {
                    if let Some(found) = crate::giga::occupant_at(ctx.state, slot).cloned()
                        && is_line_unit(ctx.state, &found)
                        && crate::zones::acts_on_field(ctx.state, &found)
                        && !hit.contains(&found.id.as_str())
                        && !candidates.iter().any(|unit: &CardInstance| unit.id == found.id)
                        && crate::geometry::reaches(ctx.state, ctx.self_.as_ref(), &found)
                    {
                        candidates.push(found);
                    }
                }
            }
            next = ctx.sink.rng.pick(&candidates).cloned();
        }
    })
}

/// `chainUnits`' argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ChainUnitsArgs {
    pub side: PlayerSpec,
    pub kind: ChainKind,
}

/// R3554007 (S+ #7 Churnwalker): chain every Unit of `side` — chained enemies all take any damage
/// one of them takes, chained allies split it, rounded down. Only the Units on the field when this
/// resolves are in it; a chain lasts for the rest of the game.
pub fn chain_units(args: ChainUnitsArgs) -> Effect {
    Effect::new("chainUnits", move |ctx| {
        let next = ctx.state.chains_made.unwrap_or(0) + 1;
        ctx.state.chains_made = Some(next);
        let side = match args.side {
            PlayerSpec::SelfSide => ScopeSide::SelfSide,
            PlayerSpec::Enemy => ScopeSide::Enemy,
        };
        let scope = BoardScope {
            side: Some(side),
            rows: None,
            types: Some(vec![CardType::Unit]),
            tags: None,
            not_tags: None,
            exclude_self: None,
            win_rate_below: None,
        };
        let ids: Vec<String> = cards_in_scope(ctx, &scope)
            .iter()
            .map(|unit| unit.id.clone())
            .collect();
        for id in ids {
            let Some(live) = find_instance_mut(ctx.state, &id) else {
                continue;
            };
            match args.kind {
                ChainKind::Shared => live.shared_chain = Some(next),
                ChainKind::Split => live.split_chain = Some(next),
            }
        }
    })
}
