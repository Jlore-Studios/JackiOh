//! Firewall doctrine verbs and the board facts they read (IA #148, #152, #157, #168;
//! R4591000–R4591004): a set Attack this turn, a timed "can't attack or be attacked", a cap on how
//! many Units the opponent attacks with, and Units immune to damage.
//!
//! Each verb is an effect a card script returns (CLAUDE.md rule 5); each board fact is a read helper
//! beside it, so the layers, the restrictions, the combat and the damage pipeline all read the one
//! definition.

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, TargetSpec, instance_of, player_of};
use crate::modifiers::add_modifier;
use crate::script::{Effect, EffectContext};
use crate::state::{
    CardInstance, ContainedUntil, EngineError, GameState, ModifierKind, TurnAttack, find_instance_mut,
};
use crate::wire::PlayerId;
use crate::wire::opponent_of;
use crate::zones::{StandingAt, acting_on, active_units_of};

/// `setAttackThisTurn`'s argument (R4591000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SetAttackThisTurnArgs {
    pub target: TargetSpec,
    pub attack: i32,
}

/// R4591000 (IA #148 Quarantine): the Unit `target` names has `attack` Attack until this turn ends.
/// Read after every Attack layer (`layers::compute_layers_on`), so it beats buffs and aura sets, and
/// touches no Health. A Unit Immune to Spells is passed by (`instance_of`), as Freeze is passed by;
/// an Immutable one is not, since a status is not text.
pub fn set_attack_this_turn(args: SetAttackThisTurnArgs) -> Effect {
    Effect::new("setAttackThisTurn", move |ctx| {
        set_attack_this_turn_on(ctx, &args.target, args.attack);
    })
}

/// `set_attack_this_turn`'s body, shared with no other verb.
fn set_attack_this_turn_on(ctx: &mut EffectContext<'_>, target: &TargetSpec, attack: i32) {
    let Some(unit) = instance_of(ctx, target) else {
        return;
    };
    if !crate::zones::in_units_row_or_standing(ctx.state, &unit)
        && !crate::zones::is_carried(ctx.state, &unit)
    {
        return;
    }
    let turn = ctx.state.turn;
    if let Some(live) = find_instance_mut(ctx.state, &unit.id) {
        live.attack_this_turn = Some(TurnAttack { turn, attack });
    }
}

/// R4591000: the Attack a set Attack this turn reads — `Some` while the game's turn is the set's
/// turn, `None` once a later turn began (or never set), so it leaves with the turn and, by R78's
/// reset, with the Unit.
pub fn attack_set_this_turn(state: &GameState, unit: &CardInstance) -> Option<i32> {
    unit.attack_this_turn
        .filter(|set| set.turn == state.turn)
        .map(|set| set.attack)
}

/// `containUntilNextTurn`'s argument (R4591001).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ContainArgs {
    pub target: TargetSpec,
}

/// R4591001 (IA #157 Containment Field): the Unit `target` names can't attack or be attacked until
/// its caster's next turn begins (`is_contained`), declared and forced alike (E35). A Unit Immune
/// to Spells is passed by (`instance_of`); an Immutable one is not, since a status is not text.
pub fn contain_until_next_turn(args: ContainArgs) -> Effect {
    Effect::new("containUntilNextTurn", move |ctx| {
        let Some(unit) = instance_of(ctx, &args.target) else {
            return;
        };
        if !crate::zones::in_units_row_or_standing(ctx.state, &unit)
            && !crate::zones::is_carried(ctx.state, &unit)
        {
            return;
        }
        // The caster's next turn begins one turn on when cast on another's turn, two on their
        // own — stamped as the freeze's thaw is (R49), never worked out later.
        let cleanups = if ctx.state.active == ctx.controller {
            2
        } else {
            1
        };
        let until_turn = ctx.state.turn + cleanups;
        if let Some(live) = find_instance_mut(ctx.state, &unit.id) {
            live.contained = Some(ContainedUntil { until_turn });
        }
    })
}

/// R4591001: whether `unit` is Contained now — stamped, and the game's turn still before its
/// caster's next turn begins.
pub fn is_contained(state: &GameState, unit: &CardInstance) -> bool {
    unit.contained
        .is_some_and(|held| state.turn < held.until_turn)
}

/// `makeUnitsImmune`'s argument (R4591004).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MakeUnitsImmuneArgs {
    /// Whose Units it covers, relative to the controller. Default "self".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R4591004 (IA #168 Impenetrable): this player's Units are immune to damage until their next turn
/// begins, with R757's Armor Up expiry — made on their turn it lasts through the opponent's next;
/// made on the opponent's, it ends at that turn's cleanup.
pub fn make_units_immune(args: MakeUnitsImmuneArgs) -> Effect {
    Effect::new("makeUnitsImmune", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let expiry = super::player_mods::armor_up_expiry(ctx.state, player);
        add_modifier(ctx, player, expiry, ModifierKind::UnitsImmune);
    })
}

/// R4591004: whether `unit` takes no damage now — on the field, and its controller's `UnitsImmune`
/// holding. Covers every Unit the player controls while it holds, summoned later ones included.
pub fn units_immune_now(state: &GameState, unit: &CardInstance) -> bool {
    crate::zones::acts_on_field(state, unit)
        && state.players[unit.controller].mods.iter().any(|modifier| {
            matches!(modifier.kind, ModifierKind::UnitsImmune)
                && crate::mana::modifier_is_live(state, modifier)
        })
}

/// R4591002 (IA #152 Rate Limiter): the fewest Units `player`'s opponent may attack with this turn —
/// the minimum `opponent_attack_limit` acting on that opponent's side, or `None` when no limit
/// acts. A face-down card's text sleeps (R1487000), a Dormant card's is off (R1839000).
pub fn attack_limit_against(state: &GameState, player: PlayerId) -> Option<i32> {
    let enemy = opponent_of(player);
    acting_on(state, enemy, StandingAt::AmongUnits)
        .into_iter()
        .filter(|card| !crate::preview::lasting_text_off(state, card))
        .filter_map(|card| crate::scripts::flags_of(state, card).opponent_attack_limit)
        .min()
}

/// R4591002: the Units that attacked on this turn while a limit acted — the turn log's
/// `attackers`, plus the ids of `player`'s Units whose exertion says they attacked, with no
/// duplicates. The log keeps a Unit that attacked and then left the field.
pub fn attackers_this_turn(state: &GameState, player: PlayerId) -> Vec<String> {
    let mut attackers: Vec<String> = state.players[player]
        .turn_log
        .attackers
        .clone()
        .unwrap_or_default();
    for unit in active_units_of(state, player) {
        if unit.exertion.attacked && !attackers.iter().any(|id| *id == unit.id) {
            attackers.push(unit.id.clone());
        }
    }
    attackers
}

/// R4591002: why the attacklimit refuses `attacker` — the reason `legal_actions`, the reducer and
/// the view all give.
pub const ATTACK_LIMIT_REFUSAL: &str = "no more of your Units may attack this turn";

/// R4591002: whether `attacker` may still attack under the limit acting against its controller, or
/// the refusal. Returns `Ok` early when no limit applies, so the common path costs one walk. Only
/// declared attacks are refused; a Unit already counted may attack again (Windfury).
pub fn attack_limit_refusal(state: &GameState, attacker: &CardInstance) -> Result<(), EngineError> {
    let Some(limit) = attack_limit_against(state, attacker.controller) else {
        return Ok(());
    };
    let attackers = attackers_this_turn(state, attacker.controller);
    if attackers.iter().any(|id| *id == attacker.id) {
        return Ok(());
    }
    if attackers.len() as i32 >= limit {
        return Err(EngineError::new(ATTACK_LIMIT_REFUSAL));
    }
    Ok(())
}

/// R4591002: count `attacker`'s attack toward its side's limit — but only while a limit acts, so a
/// game with no limit never writes the log (D14) and serializes as before.
pub fn note_limited_attacker(state: &mut GameState, attacker: &CardInstance) {
    if attack_limit_against(state, attacker.controller).is_none() {
        return;
    }
    let log = &mut state.players[attacker.controller].turn_log;
    let attackers = log.attackers.get_or_insert_with(Vec::new);
    if !attackers.iter().any(|id| *id == attacker.id) {
        attackers.push(attacker.id.clone());
    }
}
