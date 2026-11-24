//! "Your Units have +N Attack this turn" and "your Units have <keyword> this turn" (SPEC §6.3,
//! R4588000; Infinite Assault #83 Swarm Logic): `UnitsHaveKeyword`'s lend with expiry `thisTurn`,
//! reaching every Unit their controller controls this turn, later arrivals included
//! (`layers::lent_by_player`, `layers::lent_attack_by_player`), gated as the rest-of-game lend is
//! (R1859007, R4508000).

use serde::{Deserialize, Serialize};

use crate::modifiers::add_modifier;
use crate::script::Effect;
use crate::state::{ModifierExpiry, ModifierKind};

use super::player_mods::GiveUnitsKeywordArgs;

/// `giveUnitsAttackThisTurn`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GiveUnitsAttackArgs {
    /// The Attack every Unit has this turn.
    pub amount: i32,
}

/// R4588000 (Infinite Assault #83 Swarm Logic): "your Units have +N Attack this turn" — a `thisTurn`
/// modifier lending `amount` Attack to every Unit they control, later ones too
/// (`layers::lent_attack_by_player`), an aura that sets Attack still winning. R4508000: one a Spell
/// gives passes a Unit Immune to Spells by.
pub fn give_units_attack_this_turn(args: GiveUnitsAttackArgs) -> Effect {
    Effect::new("giveUnitsAttackThisTurn", move |ctx| {
        let player = ctx.controller;
        let turn = ctx.sink.state.turn;
        let from_spell = crate::restrictions::effect_is_from_spell(ctx).then_some(true);
        add_modifier(
            ctx,
            player,
            ModifierExpiry::ThisTurn { turn },
            ModifierKind::UnitsHaveAttack {
                amount: args.amount,
                from_spell,
            },
        );
    })
}

/// R4588000 (Infinite Assault #83 Swarm Logic): "your Units have <keyword> this turn" — R1859007's
/// lend with expiry `thisTurn`, so it lends `keyword` on this turn only (`mana::modifier_is_live`); a
/// Unit landing during it has the keyword as it lands. R4508000: one a Spell gives passes a Unit
/// Immune to Spells by.
pub fn give_units_keyword_this_turn(args: GiveUnitsKeywordArgs) -> Effect {
    Effect::new("giveUnitsKeywordThisTurn", move |ctx| {
        let player = ctx.controller;
        let turn = ctx.sink.state.turn;
        let from_spell = crate::restrictions::effect_is_from_spell(ctx).then_some(true);
        add_modifier(
            ctx,
            player,
            ModifierExpiry::ThisTurn { turn },
            ModifierKind::UnitsHaveKeyword {
                keyword: args.keyword.clone(),
                from_spell,
            },
        );
    })
}
