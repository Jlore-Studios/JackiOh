//! Player modifiers as a verb (SPEC §2.2's cleanup, §2.3, §6.3 Cost, §10.1, R30, R48, R65).
//!
//! A modifier is the one kind of effect that hangs off a PLAYER rather than a card: "your next Spell
//! costs 1 less" (#35 Lunar Eclipse), "your 4-cost cards cost 1 less through your next turn" (#77
//! Professor Curvature), "your cards cost 1 less this turn" and "gain Combo: draw 1" (#78 /fullsend)
//! and "your next spell resolves twice" (#79 Twinspell). `state.rs` holds the shapes, `mana.rs`
//! reads them in R65's order, `modifiers::add_modifier` assigns the id, pushes it and emits
//! `modifierChanged`, and §2.2's cleanup calls `modifiers::expire_modifiers`. Every part of that
//! exists; this file is only the verb in front of it, so a card file never writes player state
//! (CLAUDE.md rule 5).
//!
//! THE UNION IS NOT RE-DECLARED HERE. `PlayerModifier` in `state.rs` is the single definition of
//! what a modifier can be, and the expiry ladder of §2.2 (`thisTurn`, `nextTurnOf`, `used`, `never`)
//! is part of it. Narrowing or copying it here would mean two places to change when a set adds a
//! modifier and would let a card write a shape `mana.rs` cannot read, so the argument is exactly
//! `DistributiveOmit<PlayerModifier, "id">` — the whole union minus the field the engine assigns:
//! `PlayerModifierSpec` below is `PlayerModifier`'s `expiry` and `kind` and nothing else.
//!
//! Port of `packages/engine/src/effects/playerMods.ts`.

use serde::{Deserialize, Serialize};

use crate::modifiers::add_modifier;
use crate::script::Effect;
use crate::state::{ModifierExpiry, ModifierKind};

use super::targets::{PlayerSpec, player_of};

/// TS `DistributiveOmit<PlayerModifier, "id">`: a `PlayerModifier` without the id the engine assigns,
/// written as TS writes it (the kind's fields flattened beside `expiry`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerModifierSpec {
    pub expiry: ModifierExpiry,
    #[serde(flatten)]
    pub kind: ModifierKind,
}

/// `addPlayerModifier`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddPlayerModifierArgs {
    /// Whose player state it goes on, relative to the controller. Default "self".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(rename = "mod")]
    pub mod_: PlayerModifierSpec,
}

/// §6.3's cost and draw riders as one verb. The id is the engine's (`m<seq>`, so it is deterministic
/// under replay) and `modifierChanged` is emitted for it, which is what `view_for` animates.
///
/// There is no fizzle case: a modifier lands on a player, and a player is always there.
pub fn add_player_modifier(args: AddPlayerModifierArgs) -> Effect {
    Effect::new("addPlayerModifier", move |ctx| {
        // `EffectContext` derefs to `EngineSink` (`state`, `events`, `rng`), so the subsystem
        // function takes the context directly and no state write moves into this file.
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        add_modifier(ctx, player, args.mod_.expiry.clone(), args.mod_.kind.clone());
    })
}
