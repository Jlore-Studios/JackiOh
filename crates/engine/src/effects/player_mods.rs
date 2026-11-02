//! Player modifiers as a verb (SPEC §2.2's cleanup, §2.3, §6.3 Cost, §10.1, R30, R48, R65).
//!
//! A modifier hangs off a PLAYER rather than a card: "your next Spell costs 1 less" (#35 Lunar
//! Eclipse; #77, #78, #79 Twinspell). `state.rs` holds the shapes, `mana.rs` reads them in R65's
//! order, `modifiers` assigns ids and expires them at §2.2's cleanup; this file is the verb, so a card
//! file never writes player state (CLAUDE.md rule 5).
//!
//! THE UNION IS NOT RE-DECLARED HERE: `PlayerModifier` in `state.rs` is the one definition, its §2.2
//! expiry ladder included, so a card cannot write a shape `mana.rs` cannot read. `PlayerModifierSpec`
//! is its `expiry` and `kind`, without the id the engine assigns.

use serde::{Deserialize, Serialize};

use crate::config::HAND_CAP_MAX;
use crate::modifiers::add_modifier;
use crate::script::Effect;
use crate::state::{ModifierExpiry, ModifierKind};
use crate::wire::{Keyword, opponent_of};

use super::targets::{PlayerSpec, player_of};

/// TS `DistributiveOmit<PlayerModifier, "id">`: a `PlayerModifier` without the id the engine assigns,
/// written as TS writes it (the kind's fields flattened beside `expiry`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PlayerModifierSpec {
    pub expiry: ModifierExpiry,
    #[serde(flatten)]
    pub kind: ModifierKind,
}

/// `addPlayerModifier`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AddPlayerModifierArgs {
    /// Whose player state it goes on, relative to the controller. Default "self".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(rename = "mod")]
    pub mod_: PlayerModifierSpec,
}

/// §6.3's cost and draw riders as one verb. The id is the engine's (`m<seq>`, deterministic under
/// replay) and its `modifierChanged` is what `view_for` animates. It never fizzles: a player is always there.
pub fn add_player_modifier(args: AddPlayerModifierArgs) -> Effect {
    Effect::new("addPlayerModifier", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        add_modifier(ctx, player, args.mod_.expiry.clone(), args.mod_.kind.clone());
    })
}

/// `setHandCap`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SetHandCapArgs {
    /// The hand size, held to 0..=`HAND_CAP_MAX`.
    pub cap: i32,
    /// Whose hand, relative to the controller. Default "self".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// ME-HANDCAP, R1143 (Meditative #79 Touched by KY): "for the rest of the game, your hand size is N" —
/// a setting (the latest wins) read by `query::hand_cap_of`; a hand above a lowered size keeps its
/// cards, only what arrives next burning. Public (`SideView.handCap`), so no event.
pub fn set_hand_cap(args: SetHandCapArgs) -> Effect {
    Effect::new("setHandCap", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        ctx.state.players[player].hand_cap = Some(args.cap.clamp(0, HAND_CAP_MAX));
    })
}

/// `makeHeroImmune`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MakeHeroImmuneArgs {
    /// Whose hero it covers, relative to the controller. Default "self".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// MD-C21 (Meditative #48 Tranquility): this player's hero is immune to damage until their next turn
/// begins (a cap of 0 at §4.4 step 3), with R757's Armor Up expiry: made on their turn it lasts through
/// the opponent's next; made on the opponent's, it ends at that turn's cleanup.
pub fn make_hero_immune(args: MakeHeroImmuneArgs) -> Effect {
    Effect::new("makeHeroImmune", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let expiry = if ctx.sink.state.active == player {
            ModifierExpiry::NextTurnOf {
                player: opponent_of(player),
                from_turn: ctx.sink.state.turn,
            }
        } else {
            ModifierExpiry::ThisTurn {
                turn: ctx.sink.state.turn,
            }
        };
        add_modifier(ctx, player, expiry, ModifierKind::HeroImmune);
    })
}

/// `priceNextSpellInHealth`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HealthPriceArgs {
    /// How many times the Spell's cost it costs in health (Bloodbloom's declared `multiple`).
    pub multiple: i32,
    /// The Echo the priced Spell gains (the Radiant Bloodbloom's 1). Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echo: Option<i32>,
}

/// R1180004 (Meditative+ #6 Bloodbloom): "Your next Spell costs health equal to N times its cost
/// instead of mana." A `HealthPrice` modifier, expiry `used`: the controller's next Spell played
/// face-up from hand costs `multiple` times its cost in health (`mana::health_price_rule`) and gains
/// `echo` Echo. The newest replaces any older one, so several never stack.
pub fn price_next_spell_in_health(args: HealthPriceArgs) -> Effect {
    Effect::new("priceNextSpellInHealth", move |ctx| {
        let player = ctx.controller;
        let older: Vec<String> = ctx.sink.state.players[player]
            .mods
            .iter()
            .filter(|modifier| matches!(modifier.kind, ModifierKind::HealthPrice { .. }))
            .map(|modifier| modifier.id.clone())
            .collect();
        for id in &older {
            crate::modifiers::remove_modifier(ctx, player, id);
        }
        add_modifier(
            ctx,
            player,
            ModifierExpiry::Used,
            ModifierKind::HealthPrice {
                multiple: args.multiple.max(0),
                echo: args.echo.filter(|echo| *echo > 0),
            },
        );
    })
}

/// `giveNextSpellKeyword`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GiveNextSpellKeywordArgs {
    /// The keyword the Spell has: Poisonous for Stinkyscape #13 Asphyxiants Cloud.
    pub keyword: Keyword,
    /// Whose next Spell, relative to the controller. Default "self".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R1848002 (Stinkyscape #13 Asphyxiants Cloud, "This and your next Spell this turn have Poisonous"):
/// the player's next face-up Spell this turn, a cast included (R70), has `keyword` as it resolves — a
/// `thisTurn` modifier that play spends at §10.5 step 4 (`play_steps::give_next_spell_keywords`). The
/// card's own "this has" is its printed keyword, read as a Spell's Pierce is (R346).
pub fn give_next_spell_keyword(args: GiveNextSpellKeywordArgs) -> Effect {
    Effect::new("giveNextSpellKeyword", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let turn = ctx.sink.state.turn;
        add_modifier(
            ctx,
            player,
            ModifierExpiry::ThisTurn { turn },
            ModifierKind::KeywordNextSpell {
                keyword: args.keyword.clone(),
            },
        );
    })
}

/// `starshipLaunchCosts`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StarshipLaunchCostArgs {
    /// What the launch costs (#144 Prepare For Liftoff's 0).
    pub cost: i32,
}

/// R1852006 (Stinkyscape #144 Prepare For Liftoff): "Your Starship launch costs (N)." A `used`
/// modifier: the controller's next launch by its Activate costs N (`starship::launch_cost`, the least
/// of several, never below 0). A Starship an effect awakens launches free and spends none of it.
pub fn starship_launch_costs(args: StarshipLaunchCostArgs) -> Effect {
    Effect::new("starshipLaunchCosts", move |ctx| {
        let player = ctx.controller;
        add_modifier(
            ctx,
            player,
            ModifierExpiry::Used,
            ModifierKind::StarshipLaunchCost {
                cost: args.cost.max(0),
            },
        );
    })
}

/// `giveUnitsKeyword`'s and `giveUnitsKeywordNextTurn`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GiveUnitsKeywordArgs {
    /// The keyword every Unit has: Trample, Deft, Rush or Flying for Stinkyscape's War Doctrines,
    /// Charge for #4.4 Last Word.
    pub keyword: Keyword,
}

/// R1859007 (Stinkyscape #137.1–#137.4, the War Doctrines): "For the rest of the game, your Units have
/// X" — a `never` modifier lending it to every Unit they control, later ones too (`layers::lent_by_player`).
pub fn give_units_keyword(args: GiveUnitsKeywordArgs) -> Effect {
    Effect::new("giveUnitsKeyword", move |ctx| {
        let player = ctx.controller;
        add_modifier(
            ctx,
            player,
            ModifierExpiry::Never,
            ModifierKind::UnitsHaveKeyword {
                keyword: args.keyword.clone(),
            },
        );
    })
}

/// R1870004 (Stinkyscape #4.4 Last Word, "the next turn is yours: your Units have Charge during it"):
/// R1859007's modifier with expiry `nextTurnOf` from this turn, so it lends `keyword` on their next
/// turn only (`mana::modifier_is_live`); a Unit landing during it has the keyword as it lands.
pub fn give_units_keyword_next_turn(args: GiveUnitsKeywordArgs) -> Effect {
    Effect::new("giveUnitsKeywordNextTurn", move |ctx| {
        let player = ctx.controller;
        let turn = ctx.sink.state.turn;
        add_modifier(
            ctx,
            player,
            ModifierExpiry::NextTurnOf {
                player,
                from_turn: turn,
            },
            ModifierKind::UnitsHaveKeyword {
                keyword: args.keyword.clone(),
            },
        );
    })
}
