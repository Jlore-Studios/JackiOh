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

/// ME-HANDCAP, R1143 (Meditative #79 Touched by KY): "for the rest of the game, your hand size is N".
/// A setting, not an increase: the latest one wins, a second of the same changes nothing, and it lasts
/// whatever becomes of the card that set it. Every rule that reads the hand cap reads it
/// (`query::hand_cap_of`). A hand already above a lowered size keeps its cards; only what arrives
/// next is burned. Public in both views (`SideView.handCap`), so it says so with no event.
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

/// MD-C21 (Meditative #48 Tranquility): this player's hero is immune to damage until their next
/// turn begins — a `HeroImmune` player modifier, read as a cap of 0 at §4.4 step 3.
///
/// Expiry is R757's Armor Up expiry: made on this player's turn it lasts through the opponent's
/// next turn (`nextTurnOf` the opponent), so the cleanup ending that turn takes it off as this
/// player's next turn begins. Made on the opponent's turn there is no turn of theirs to cover, so
/// it ends at that turn's cleanup.
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
/// instead of mana." A `HealthPrice` player modifier on the controller, until a Spell uses it (expiry
/// `used`): the next Spell they play from their hand, face-up, costs no mana and `multiple` times its
/// cost in health (`mana::health_price_rule`), lost with the play, and gains `echo` Echo. The newest
/// replaces any older one, so several never stack.
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
/// the next Spell the player plays face-up this turn, a cast included (R70), has `keyword` for its
/// resolution — a `keywordNextSpell` player modifier, expiry `thisTurn`, which that play spends at
/// §10.5 step 4 (`play_steps::give_next_spell_keywords`). The card's own "this has Poisonous" is the
/// keyword printed on its face, read off it while it resolves as a Spell's Pierce is (R346).
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

/// R1852006 (Stinkyscape #144 Prepare For Liftoff): "Your Starship launch costs (N)." A
/// `StarshipLaunchCost` player modifier on the controller, until a launch it prices spends it (expiry
/// `used`): their next launch by its Activate costs N (`starship::launch_cost`, the least of several,
/// never below 0). A Starship an effect awakens launches for nothing and spends none of it.
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

/// `giveUnitsKeyword`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GiveUnitsKeywordArgs {
    /// The keyword every Unit has: Trample, Deft, Rush or Flying for Stinkyscape's War Doctrines.
    pub keyword: Keyword,
}

/// R1859007 (Stinkyscape #137.1–#137.4, the War Doctrines): "For the rest of the game, your Units
/// have X." A `UnitsHaveKeyword` player modifier on the controller, expiry `never`: every Unit they
/// control on the field has the keyword, those that arrive later too (`layers::lent_by_player`).
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
