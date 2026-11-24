//! "This turn" lends to Units (R4594002): `giveUnitsAttack` and `giveUnitsKeyword` with
//! `ThisTurn` expiry, for Infinite Assault #220 Last Lap. The modifiers are the same
//! player-level kinds the lasting verbs write (`UnitsAttack`, `UnitsHaveKeyword`), so Units
//! arriving later read them through the layers as they land; they expire at §2.2's cleanup.
//! A Spell's lend passes a Unit Immune to Spells by (`from_spell`, R4453000).

use crate::modifiers::add_modifier;
use crate::restrictions::effect_is_from_spell;
use crate::script::Effect;
use crate::state::{ModifierExpiry, ModifierKind};

use super::player_mods::{GiveUnitsAttackArgs, GiveUnitsKeywordArgs};
use super::targets::{PlayerSpec, player_of};

/// R4594002 (Infinite Assault #220 Last Lap, "your Units have +N Attack this turn"): `amount`
/// more Attack on every Unit the affected player controls, those arriving later too, until §2.2's
/// cleanup. A Spell's lend passes a Unit Immune to Spells by.
pub fn give_units_attack_this_turn(args: GiveUnitsAttackArgs) -> Effect {
    Effect::new("giveUnitsAttackThisTurn", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let turn = ctx.sink.state.turn;
        let from_spell = effect_is_from_spell(ctx).then_some(true);
        add_modifier(
            ctx,
            player,
            ModifierExpiry::ThisTurn { turn },
            ModifierKind::UnitsAttack { amount: args.amount, from_spell },
        );
    })
}

/// R4594002 (Infinite Assault #220 Last Lap, "your Units have Charge this turn"): `keyword` on
/// every Unit the controller controls, those arriving later too, until §2.2's cleanup. A Spell's
/// lend passes a Unit Immune to Spells by.
pub fn give_units_keyword_this_turn(args: GiveUnitsKeywordArgs) -> Effect {
    Effect::new("giveUnitsKeywordThisTurn", move |ctx| {
        let player = ctx.controller;
        let turn = ctx.sink.state.turn;
        let from_spell = effect_is_from_spell(ctx).then_some(true);
        add_modifier(
            ctx,
            player,
            ModifierExpiry::ThisTurn { turn },
            ModifierKind::UnitsHaveKeyword { keyword: args.keyword.clone(), from_spell },
        );
    })
}
