//! The practice sandbox's tools (issue #923, R923000–R923003): one action, `sandbox`, whose `op`
//! spawns a card for either seat, sets a hero's health and Armor or a player's mana, draws, or
//! destroys or removes a card. This module owns the rule: `why_sandbox_refused` is every refusal and
//! `apply_sandbox` every change, and `reduce` calls the one after the other.
//!
//! - **Only in a sandbox** (R923000): a game created with `CreateGameOptions.sandbox` holds
//!   `GameState.sandbox`, and no other game accepts a tool. `legal_actions` never lists one, so the
//!   AI and every policy leave them alone, and the server's match actor refuses them before the
//!   engine sees them. A tool is an action like any other: it is logged, deduped by its nonce and
//!   folded by a replay, which passes the same flag (`ReplayInput.sandbox`).
//! - **Placed, not played** (R923001): every tool goes through the engine's own verbs, so a card
//!   arrives as an effect's card does. On the field it is summoned (`effects::summon`: a Trap
//!   face-down, a Unit as summoning sick as any summon, no Cry), in the hand it is added (§2.4: a
//!   full hand burns it), on top of the library it is put there (`draw::put_into_library`), and in
//!   the graveyard it is a fresh card that enters it. No play is counted and nothing is paid.
//! - **Numbers in bounds** (R923002): a hero's health 1 to `SANDBOX_HERO_HEALTH_MAX`, its Armor 0 to
//!   `SANDBOX_ARMOR_MAX`, mana 0 to `SANDBOX_MANA_MAX`, a draw 1 to `SANDBOX_DRAW_MAX` cards.
//! - **Destroy and remove** (R923003): destroy marks a card on the field as a destroy effect does, so
//!   the state check after the action moves it and runs its Death; remove exiles a card from
//!   wherever it is, its Death not run.
//!
//! A tool runs between the turn's moves, so it waits on what blocks one: an open prompt and the
//! mulligan refuse it (`reduce`'s gates), while either seat may use one on either turn.

use crate::catalog::{find_def, set_is_open};
use crate::config::{SANDBOX_ARMOR_MAX, SANDBOX_DRAW_MAX, SANDBOX_HERO_HEALTH_MAX, SANDBOX_MANA_MAX};
use crate::draw::{draw, put_into_library};
use crate::effects::{AddToHandArgs, DestroyArgs, ExileArgs, PlayerSpec, SummonArgs, TargetSpec};
use crate::faces::card_type_of_face;
use crate::ownership::LibraryEnd;
use crate::resolve::{HookOptions, apply_effects, make_context};
use crate::script::EngineSink;
use crate::state::{EngineError, GameState, find_instance, new_instance};
use crate::wire::{CardDef, CardType, GameEvent, PlayerId, Row, SandboxOp, SpawnZone, Zone, ZoneName};
use crate::zones::{is_empty, is_reserved, row_size};

/// R923000: whether `state` is a sandbox game, whose tools it alone accepts.
pub fn is_sandbox(state: &GameState) -> bool {
    state.sandbox == Some(true)
}

/// R923001: the definition a spawn may make: a card or a token of a set a player may hold (R1420).
/// A transient definition (a Fuse's, a crafted card's) is no card to spawn.
fn spawnable<'a>(state: &'a GameState, def_id: &str) -> Option<&'a CardDef> {
    if state.transient_defs.contains_key(def_id) {
        return None;
    }
    find_def(Some(state), def_id).filter(|def| set_is_open(def.set))
}

/// R923001: the row a card of this type stands in on the field, or `None` for a Spell, which is cast
/// and never stands there (§5.1).
fn row_for(card_type: CardType) -> Option<Row> {
    match card_type {
        CardType::Unit => Some(Row::Units),
        CardType::Spell => None,
        _ => Some(Row::Backrow),
    }
}

fn bounded(what: &str, value: Option<i32>, min: i32, max: i32) -> Result<(), EngineError> {
    match value {
        Some(n) if n < min || n > max => Err(EngineError::new(format!("{what} must be from {min} to {max}"))),
        _ => Ok(()),
    }
}

/// R923000–R923003: why the tool `op` is refused in `state`, if it is.
pub fn why_sandbox_refused(state: &GameState, op: &SandboxOp) -> Result<(), EngineError> {
    if !is_sandbox(state) {
        return Err(EngineError::new(
            "the sandbox's tools work only in a sandbox game",
        ));
    }
    match op {
        SandboxOp::Spawn {
            player,
            def_id,
            radiant,
            to,
        } => {
            let Some(def) = spawnable(state, def_id) else {
                return Err(EngineError::new(format!(
                    "there is no card \"{def_id}\" to spawn"
                )));
            };
            if let SpawnZone::Field { row, lane } = to {
                let card_type = card_type_of_face(state, &def.id, *radiant == Some(true));
                let Some(wanted) = row_for(card_type) else {
                    return Err(EngineError::new(format!(
                        "{} is a Spell, which is cast and never stands on the field",
                        def.name
                    )));
                };
                if *row != wanted {
                    return Err(EngineError::new(format!(
                        "{} stands in the {} row",
                        def.name,
                        if wanted == Row::Units { "unit" } else { "backrow" }
                    )));
                }
                if *lane < 1 || *lane > row_size(*row) {
                    return Err(EngineError::new(format!("there is no lane {lane} in that row")));
                }
                let slot = crate::zones::ZoneSlot {
                    player: *player,
                    row: *row,
                    lane: *lane,
                };
                if !is_empty(state, slot) || is_reserved(state, slot) {
                    return Err(EngineError::new("that zone is taken"));
                }
            }
            Ok(())
        }
        SandboxOp::SetHero { health, armor, .. } => {
            if health.is_none() && armor.is_none() {
                return Err(EngineError::new("set a health or an Armor"));
            }
            bounded("a hero's health", *health, 1, SANDBOX_HERO_HEALTH_MAX)?;
            bounded("a hero's Armor", *armor, 0, SANDBOX_ARMOR_MAX)
        }
        SandboxOp::SetMana { current, max, .. } => {
            if current.is_none() && max.is_none() {
                return Err(EngineError::new("set a current or a maximum mana"));
            }
            bounded("current mana", *current, 0, SANDBOX_MANA_MAX)?;
            bounded("maximum mana", *max, 0, SANDBOX_MANA_MAX)
        }
        SandboxOp::Draw { count, .. } => bounded("a draw", Some(*count), 1, SANDBOX_DRAW_MAX),
        SandboxOp::Destroy { instance_id } => match find_instance(state, instance_id) {
            Some(card) if card.zone.z() == ZoneName::Field => Ok(()),
            _ => Err(EngineError::new("destroy a card on the field")),
        },
        SandboxOp::Remove { instance_id } => match find_instance(state, instance_id) {
            Some(card) if card.zone.z() != ZoneName::Resolving && card.zone.z() != ZoneName::Exile => Ok(()),
            _ => Err(EngineError::new(
                "remove a card in a hand, a library, a graveyard or on the field",
            )),
        },
    }
}

/// R923000–R923003: apply the tool `op` (refused first, as `reduce` refuses any other action).
pub fn apply_sandbox(sink: &mut EngineSink<'_>, op: &SandboxOp) -> Result<(), EngineError> {
    why_sandbox_refused(sink.state, op)?;
    match op {
        SandboxOp::Spawn {
            player,
            def_id,
            radiant,
            to,
        } => spawn(sink, *player, def_id, *radiant == Some(true), *to),
        SandboxOp::SetHero {
            player,
            health,
            armor,
        } => {
            let hero = &mut sink.state.players[*player].hero;
            if let Some(armor) = armor {
                hero.armor = *armor;
            }
            if let Some(health) = health {
                hero.health = *health;
                sink.events.push(GameEvent::HealthSet {
                    player: *player,
                    health: *health,
                    source_id: None,
                });
            }
        }
        SandboxOp::SetMana { player, current, max } => {
            let mana = &mut sink.state.players[*player].mana;
            if let Some(max) = max {
                mana.max = *max;
            }
            if let Some(current) = current {
                mana.current = *current;
            }
            let (current, max) = (mana.current, mana.max);
            sink.events.push(GameEvent::ManaChanged {
                player: *player,
                current,
                max,
            });
        }
        SandboxOp::Draw { player, count } => {
            draw(sink, *player, *count);
        }
        SandboxOp::Destroy { instance_id } => {
            let controller = controller_of(sink.state, instance_id);
            run_verb(
                sink,
                controller,
                crate::effects::destroy(DestroyArgs {
                    target: TargetSpec::Instance {
                        instance_id: instance_id.clone(),
                    },
                }),
            );
        }
        SandboxOp::Remove { instance_id } => {
            let controller = controller_of(sink.state, instance_id);
            run_verb(
                sink,
                controller,
                crate::effects::exile(ExileArgs {
                    target: TargetSpec::Instance {
                        instance_id: instance_id.clone(),
                    },
                }),
            );
        }
    }
    Ok(())
}

fn controller_of(state: &GameState, instance_id: &str) -> PlayerId {
    find_instance(state, instance_id).map_or(state.active, |card| card.controller)
}

/// One of the engine's verbs, run for `controller` with no card behind it: the sandbox is no card.
fn run_verb(sink: &mut EngineSink<'_>, controller: PlayerId, effect: crate::script::Effect) {
    let mut ctx = make_context(
        sink,
        None,
        HookOptions {
            controller: Some(controller),
            ..HookOptions::default()
        },
    );
    apply_effects(&[effect], &mut ctx);
}

/// R923001: a fresh card of `def_id` for `player`, placed by the engine's own verb for `to`.
fn spawn(sink: &mut EngineSink<'_>, player: PlayerId, def_id: &str, radiant: bool, to: SpawnZone) {
    let radiant_flag = radiant.then_some(true);
    match to {
        SpawnZone::Hand => run_verb(
            sink,
            player,
            crate::effects::add_to_hand(AddToHandArgs {
                def_id: Some(def_id.to_string()),
                player: Some(PlayerSpec::SelfSide),
                radiant: radiant_flag,
                ..AddToHandArgs::default()
            }),
        ),
        SpawnZone::Field { lane, .. } => run_verb(
            sink,
            player,
            crate::effects::summon(SummonArgs {
                player: Some(PlayerSpec::SelfSide),
                def_id: Some(def_id.to_string()),
                lane: Some(lane),
                radiant: radiant_flag,
                ..SummonArgs::default()
            }),
        ),
        SpawnZone::LibraryTop => {
            let mut card = new_instance(&mut *sink.state, def_id, player, Zone::Resolving { player });
            card.radiant = radiant;
            put_into_library(sink, &mut card, false, None, LibraryEnd::Top);
        }
        SpawnZone::Graveyard => {
            let mut card = new_instance(&mut *sink.state, def_id, player, Zone::Graveyard { player });
            card.radiant = radiant;
            sink.events.push(GameEvent::EnteredGraveyard {
                instance_id: card.id.clone(),
                def_id: card.def_id.clone(),
                owner: player,
            });
            sink.state.players[player].graveyard.push(card);
        }
    }
}
