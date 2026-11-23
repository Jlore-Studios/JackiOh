//! Heroic Power (SPEC §8 #98, R43, R103, R352, R752–R761, R3551002): the fourteen powers, the roll
//! that picks one, and each power as the Activate ability it is. The kept v0.5.13g script plays the
//! thirteen (`PowerEdition::Thirteen`); the new face adds End Times and prices each of the thirteen
//! (1) less (`PowerEdition::EndTimes`).
//!
//! R43 puts everything about the card on its instance (`memory.power`), so two Heroic Powers in one
//! game never share a roll or a use and a serialized game resumes knowing both (§10.1). The roll goes
//! through the match rng, so a replay rolls the same power (§9.3).
//!
//! R3134002: since issue #3134 the card's Cry Discovers its power (`discover_power`): a Choose one of
//! `DISCOVER_OPTIONS` different powers, whose answer is the power (`POWER_PICK`). A copy that reaches
//! the field with no Cry to choose keeps R151's arrival roll (`roll_power_on_field`); one in a hand or
//! a library rolls nothing, as playing it asks. Through v0.5.11r every copy rolled in every zone
//! (`roll_power`, the kept script's).
//!
//! R752: the card costs (0); each power is a once-per-turn "Activate: Spend (X): …" ability (R384,
//! R81), of which the card has only the one it rolled (`ActivationDecl.has`). Three powers pause on a
//! Discover (§6.3) whose answer re-enters `hero_power`: a `PendingChoice`, never a callback (§10.6).
//! Numbers: `crate::config` (CLAUDE.md rule 9, SURFACE §6.4).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use indexmap::IndexMap;

use crate::config::{
    ARMOR_UP_ARMOR, BRAINSTORM_DISCOUNT, DIE_INSECT_DAMAGE, DIE_INSECT_LUCKY, DISCOVER_OPTIONS,
    END_TIMES_COST, END_TIMES_CUT, END_TIMES_RADIANT_CUT, HERO_POWER_DISCOUNT, LIFE_TAP_DAMAGE,
    LIFE_TAP_DRAW, PING_DAMAGE, PLUCK_COST, POWERS_THROUGH_V0_5_13G, STEADY_SHOT_RAISE,
    STITCHING_INGREDIENTS, STITCHING_MAX_COST, TANK_UP_ARMOR,
};
use crate::damage::DamageTarget;
use crate::effects::TargetSpec;
use crate::layers::unit_view;
use crate::prelude::json_as;
use crate::rng::Rng;
use crate::script::{
    ActivationCost, ActivationDecl, ActivationUses, Effect, EffectContext, EngineSink, hook, read_hook,
};
use crate::state::{CardInstance, GameState};
use crate::subsystems::activate::{
    ACTIVATIONS_MEMORY_KEY, abilities_of, ability_uses_this_turn, uses_allowed, uses_record_after_refund,
};
use crate::wire::{CardType, SetName, TargetDecl, ZoneName, opponent_of};

/// R43: where the rolled power lives on the instance.
pub const POWER_KEY: &str = "power";

/// The resume step a power that opened a Discover comes back to (§10.6), wired in #98's script:
///
/// ```ignore
/// resume: IndexMap::from([(POWER_RESUME, hook(subsystems::hero_power))]),
/// ```
pub const POWER_RESUME: &str = "heroPower";

/// The data key that names the power a paused activation is finishing.
pub const POWER_DATA_KEY: &str = "power";

/// R3134002: the `POWER_DATA_KEY` value of the Discover of the card's own power, which no stored power
/// name (R103) is: its answer is the power picked, where another power's answer is that power's pick.
pub const POWER_PICK: &str = "pick";

/// R3134002: the Discover of the card's power asks this.
pub const DISCOVER_POWER_PROMPT: &str = "Discover a power";

/// §7: the tokens #98 summons, by catalog index (§5.3), so the engine names no catalog id.
pub const RUSH_TOKEN_INDEX: &str = "T-rush";
pub const FELINOR_TOKEN_INDEX: &str = "T-felinor";
pub const GHOUL_TOKEN_INDEX: &str = "T-ghoul";
/// B2.2: the set whose indices those are, since an index is unique only within its set.
const TOKEN_SET: SetName = SetName::Core;

/// R754: Steady Shot's declared number ("{shot}"), its power's own (`power: "burn"`, R1430).
pub const STEADY_SHOT_PARAM: &str = "shot";

/// B3.4 rule 5 (#493): #98's other numbers, moved by the card's tuning (`power_number`). R1430: each
/// names its power (`Param.power`), so a Degrade, an Upgrade or KY's Constant reaches only the power the
/// card has now; another keeps its tuning for a reroll (R757). Life Tap's damage is base-only (R1431).
pub const LIFE_TAP_DRAW_PARAM: &str = "tapDraw";
pub const LIFE_TAP_DAMAGE_PARAM: &str = "tapDamage";
pub const PING_PARAM: &str = "ping";
pub const ARMOR_PARAM: &str = "armor";
pub const DIE_INSECT_PARAM: &str = "insect";
pub const BRAINSTORM_PARAM: &str = "discount";
pub const PLUCK_PARAM: &str = "fruitCost";
pub const STITCHING_PARAM: &str = "stitchCost";

/// A power's number as the card stands (#493): `printed` moved as far as the card's tuning moved its
/// declared `key` (`params::declared_or`); a card that declares no `key` reads `printed`.
fn power_number(ctx: &EffectContext<'_>, key: &str, printed: i32) -> i32 {
    match ctx.live_self() {
        Some(card) => crate::params::declared_or(&*ctx.sink.state, card, key, printed),
        None => printed,
    }
}
/// The data key a paused Stitching carries its Discover picks in (§10.6).
const STITCHING_PICKS_KEY: &str = "picks";

/// R756: Ping reaches any unit or hero, either side, declared with the activation (R81).
fn ping_targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(
        1,
        1,
        json!({ "side": "any", "of": ["unit", "hero"] }),
    )]
}

/// R761: Terminus Tricks Discovers a Trap; a Field Trap is a Trap.
const TRAP_TYPES: &[CardType] = &[CardType::Trap, CardType::FieldTrap];

/// R103: the stored power names. They are state, so they stay stable and a new power is added at the end.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum HeroPowerName {
    Recruit,
    Draw,
    Ping,
    Burn,
    Rush,
    Felinor,
    Discover,
    Stitching,
    Armor,
    Insect,
    Brainstorm,
    Pluck,
    Tricks,
    EndTimes,
}

impl HeroPowerName {
    /// Every name, in R103's stored order.
    pub const ALL: &'static [HeroPowerName] = &[
        HeroPowerName::Recruit,
        HeroPowerName::Draw,
        HeroPowerName::Ping,
        HeroPowerName::Burn,
        HeroPowerName::Rush,
        HeroPowerName::Felinor,
        HeroPowerName::Discover,
        HeroPowerName::Stitching,
        HeroPowerName::Armor,
        HeroPowerName::Insect,
        HeroPowerName::Brainstorm,
        HeroPowerName::Pluck,
        HeroPowerName::Tricks,
        HeroPowerName::EndTimes,
    ];

    /// The stored name (the Activate ability's id).
    pub fn as_str(self) -> &'static str {
        match self {
            HeroPowerName::Recruit => "recruit",
            HeroPowerName::Draw => "draw",
            HeroPowerName::Ping => "ping",
            HeroPowerName::Burn => "burn",
            HeroPowerName::Rush => "rush",
            HeroPowerName::Felinor => "felinor",
            HeroPowerName::Discover => "discover",
            HeroPowerName::Stitching => "stitching",
            HeroPowerName::Armor => "armor",
            HeroPowerName::Insect => "insect",
            HeroPowerName::Brainstorm => "brainstorm",
            HeroPowerName::Pluck => "pluck",
            HeroPowerName::Tricks => "tricks",
            HeroPowerName::EndTimes => "endTimes",
        }
    }
}

impl std::fmt::Display for HeroPowerName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The effects one activation runs. `radiant` picks the Radiant face (§5.2).
pub type HeroPowerBuild = fn(&mut EffectContext<'_>, bool) -> Vec<Effect>;

#[derive(Clone, Copy)]
pub struct HeroPower {
    pub name: HeroPowerName,
    /// The power's name on the card (§8 #98), on each face: Armor Up is Tank Up on the Radiant one.
    pub title: &'static str,
    pub radiant_title: &'static str,
    /// "Activate: Spend (X)" (R752): the mana each use pays.
    pub x: i32,
    /// R3551002: the declared number the X is read off when the power's price moves (End Times'
    /// `endTimes`); `None` while the price is the table's `x` alone.
    pub x_param: Option<&'static str>,
    /// The §8 #98 words this entry implements, base face and Radiant face.
    pub label: &'static str,
    pub radiant_label: &'static str,
    /// What the activation declares (R81): Ping's target. A builder, so the table stays `const`.
    pub targets: Option<fn() -> Vec<TargetDecl>>,
    pub build: HeroPowerBuild,
}

impl std::fmt::Debug for HeroPower {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeroPower")
            .field("name", &self.name)
            .field("title", &self.title)
            .field("radiant_title", &self.radiant_title)
            .field("x", &self.x)
            .field("x_param", &self.x_param)
            .field("label", &self.label)
            .field("radiant_label", &self.radiant_label)
            .field("targets", &self.targets.is_some())
            .finish()
    }
}

fn token_def_id(index: &str) -> Option<String> {
    crate::catalog::def_id_by_index(TOKEN_SET, index)
}

/// R103: a token summon resolves the token by catalog index, and fizzles in silence without it.
fn summon_token(index: &str, radiant: bool) -> Vec<Effect> {
    match token_def_id(index) {
        None => vec![],
        Some(def_id) => {
            let args = if radiant {
                json!({ "defId": def_id, "radiant": true })
            } else {
                json!({ "defId": def_id })
            };
            vec![crate::effects::summon(json_as(args))]
        }
    }
}

/// Expedition Map. R43: "Recruit a permanent", which is §6.3's Recruit; the Radiant face makes it Radiant.
fn expedition_map(_ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    vec![crate::effects::recruit(json_as(json!({ "radiant": radiant })))]
}

/// Life Tap (R753): draw {tapDraw}, then {tapDamage} damage to your hero; Radiant: {tapDraw} from
/// your deck, then as many off the top of the opponent's.
fn life_tap(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let draws = power_number(ctx, LIFE_TAP_DRAW_PARAM, LIFE_TAP_DRAW);
    let own = crate::effects::draw(json_as(json!({ "count": draws })));
    if radiant {
        let mut effects = vec![own];
        for _ in 0..draws {
            effects.push(crate::effects::draw_from_opponent(json_as(
                json!({ "end": "top" }),
            )));
        }
        return effects;
    }
    let damage = power_number(ctx, LIFE_TAP_DAMAGE_PARAM, LIFE_TAP_DAMAGE);
    vec![
        own,
        crate::effects::damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": damage }))),
    ]
}

/// Steady Shot (R754): {shot} to the enemy hero; on the Radiant face, then {shot} goes up by 2 for good.
fn steady_shot(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let shot = crate::params::param(ctx, STEADY_SHOT_PARAM);
    let hit = crate::effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": shot })));
    if !radiant {
        return vec![hit];
    }
    vec![
        hit,
        crate::effects::set_number(json_as(json!({
            "target": { "of": "self" },
            "which": format!("param:{STEADY_SHOT_PARAM}"),
            "value": shot + STEADY_SHOT_RAISE
        }))),
    ]
}

/// Ranching: a Rush Token, Radiant on the Radiant face (§7).
fn ranching(_ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    summon_token(RUSH_TOKEN_INDEX, radiant)
}

/// Cat Cafe (R755): a Felinor Token; the Radiant face summons a random non-token Felinor of every set.
fn cat_cafe(_ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    if radiant {
        return vec![crate::effects::summon_random(json_as(json!({
            "query": { "type": "Unit", "tags": ["Felinor"] },
            "player": "self"
        })))];
    }
    summon_token(FELINOR_TOKEN_INDEX, false)
}

/// Ping (R756): Pierce, {ping} damage to the unit or hero declared with the activation. The Radiant face
/// then asks, after the state check (R59), whether the unit left the field (`leftFieldAfter` against the
/// activation's own mark, which a pause keeps, R174; a Reborn body put back included), and if so
/// summons you a Ghoul Token with the unit's attack and undamaged health from before the hit.
fn ping(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let hit = crate::effects::damage(json_as(json!({
        "to": { "of": "chosen" },
        "amount": power_number(ctx, PING_PARAM, PING_DAMAGE),
        "ignoreArmor": true
    })));
    if !radiant {
        return vec![hit];
    }
    let target = crate::effects::resolve_target(ctx, &TargetSpec::Chosen { index: None });
    let ghoul = token_def_id(GHOUL_TOKEN_INDEX);
    let mark = ctx.exits_from;
    let (unit, ghoul, mark): (CardInstance, String, u32) = match (target, ghoul, mark) {
        (Some(DamageTarget::Unit { instance }), Some(ghoul), Some(mark)) => (instance.clone(), ghoul, mark),
        _ => return vec![hit],
    };
    let view = unit_view(&*ctx.sink.state, &unit);
    let attack = view.attack.max(0);
    let health = view.max_health.max(1);
    let unit_id = unit.id.clone();
    vec![
        hit,
        crate::effects::after_state_check(move |after| {
            if crate::stays::left_field_after(&*after.sink.state, mark, &unit_id) {
                vec![crate::effects::summon(json_as(json!({
                    "defId": ghoul,
                    "player": "self",
                    "statsOverride": { "attack": attack, "health": health }
                })))]
            } else {
                vec![]
            }
        }),
    ]
}

/// Witness Value: §6.3 Discover a Unit to hand, Radiant when the power is (R103).
fn witness_value(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    if let Some(picked) = crate::effects::chosen_options(ctx).first().cloned() {
        return vec![crate::effects::add_to_hand(json_as(json!({
            "defId": picked,
            "player": "self",
            "radiant": radiant
        })))];
    }
    vec![crate::effects::discover_from_catalog(json_as(json!({
        "step": POWER_RESUME,
        "query": { "type": "Unit" },
        "prompt": if radiant { "Discover a Radiant Unit" } else { "Discover a Unit" },
        "data": { POWER_DATA_KEY: "discover" }
    })))]
}

/// The picks a paused Stitching has made so far.
fn stitched_so_far(ctx: &EffectContext<'_>) -> Vec<String> {
    match ctx.data.get(STITCHING_PICKS_KEY).and_then(Value::as_array) {
        None => vec![],
        Some(stored) => stored
            .iter()
            .filter_map(|entry| entry.as_str().map(str::to_string))
            .collect(),
    }
}

/// R352: "Discover two (2) Cost or less Units. Fuse them." Two chained Discovers, each answer re-entering
/// `heroPower` with the picks so far in the prompt's data; the second fuses them per R77 into the hand at
/// R77's fused cost (Radiant on the Radiant face). Fewer than two picks fuse nothing.
fn stitching(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let max_cost = power_number(ctx, STITCHING_PARAM, STITCHING_MAX_COST);
    let answered = crate::effects::chosen_options(ctx).first().cloned();
    let mut picks = stitched_so_far(ctx);
    if let Some(answered) = answered {
        picks.push(answered);
    }
    if picks.len() as i32 >= STITCHING_INGREDIENTS {
        return vec![crate::effects::fuse_cards(json_as(json!({
            "defIds": picks,
            "toHand": "self",
            "handPrice": "fused",
            "radiant": radiant
        })))];
    }
    vec![crate::effects::discover_from_catalog(json_as(json!({
        "step": POWER_RESUME,
        "query": { "type": "Unit", "costRange": { "max": max_cost } },
        "prompt": if radiant {
            format!("Discover a Radiant Unit that costs ({max_cost}) or less")
        } else {
            format!("Discover a Unit that costs ({max_cost}) or less")
        },
        "data": { POWER_DATA_KEY: "stitching", STITCHING_PICKS_KEY: picks }
    })))]
}

/// Writes the card in the state and the context's own copy, so later effects of the context see it.
fn write_memory(ctx: &mut EffectContext<'_>, id: &str, key: &str, value: Value) {
    if let Some(card) = crate::state::find_instance_mut(&mut *ctx.sink.state, id) {
        card.memory.insert(key.to_string(), value.clone());
    }
    if let Some(card) = ctx.self_.as_mut().filter(|card| card.id == id) {
        card.memory.insert(key.to_string(), value);
    }
}

/// R757: the power rerolls among the other twelve (the match rng) and gives back the use this
/// activation spent, so the new power may be activated this turn too, paying its own X.
/// R3551002: the card's edition picks the roll (Tank Up's refresh never lands on End Times).
pub fn refresh_power() -> Effect {
    Effect::new("refreshPower", |ctx| {
        let Some(mut card) = ctx.live_self().cloned() else {
            return;
        };
        let current = power_of(&card).map(|power| power.name);
        let edition = edition_of(&*ctx.sink.state, &card);
        let others: Vec<&'static HeroPower> = edition
            .powers()
            .iter()
            .filter(|power| Some(power.name) != current)
            .collect();
        let Some(next) = ctx.sink.rng.pick(&others).copied() else {
            return;
        };
        let id = card.id.clone();
        let spent = power_ability_of(&*ctx.sink.state, &card).map(|decl| decl.id);
        let refunded = uses_record_after_refund(&*ctx.sink.state, &card, spent.as_deref());
        card.memory
            .insert(POWER_KEY.to_string(), json!(next.name.as_str()));
        write_memory(ctx, &id, POWER_KEY, json!(next.name.as_str()));
        if let Some(record) = refunded {
            write_memory(ctx, &id, ACTIVATIONS_MEMORY_KEY, record);
        }
    })
}

/// Armor Up (R757): hero Armor as a player modifier through the opponent's next turn, gone when yours
/// begins. Tank Up, its Radiant face: Armor your hero keeps, then the power refreshes.
fn armor_up(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    if radiant {
        let armor = power_number(ctx, ARMOR_PARAM, TANK_UP_ARMOR);
        return vec![
            crate::effects::gain_hero_armor(json_as(json!({ "amount": armor }))),
            refresh_power(),
        ];
    }
    let armor = power_number(ctx, ARMOR_PARAM, ARMOR_UP_ARMOR);
    let opponent = opponent_of(ctx.controller);
    vec![crate::effects::add_player_modifier(json_as(json!({
        "mod": {
            "kind": "heroArmor",
            "amount": armor,
            "expiry": { "until": "nextTurnOf", "player": opponent, "fromTurn": ctx.sink.state.turn }
        }
    })))]
}

/// R758: what Die Insect may hit — an enemy Unit acting on the field, or the enemy hero (`None`).
type InsectPick = Option<CardInstance>;

/// R758, R414: a Unit beats the hero; between Units, higher attack plus current health, then cost, then the lower lane.
fn better_insect_pick(state: &GameState) -> impl Fn(InsectPick, InsectPick) -> InsectPick + '_ {
    let worth = move |card: &CardInstance| -> i32 {
        let view = unit_view(state, card);
        view.attack + view.health
    };
    // A card with no slot sorts past every lane.
    let lane = move |card: &CardInstance| -> i64 {
        crate::zones::slot_of(state, card).map_or(i64::MAX, |at| i64::from(at.lane))
    };
    move |a, b| {
        let (a, b) = match (a, b) {
            (None, b) => return b,
            (a, None) => return a,
            (Some(a), Some(b)) => (a, b),
        };
        if worth(&a) != worth(&b) {
            return if worth(&a) > worth(&b) { Some(a) } else { Some(b) };
        }
        let cost_a = crate::mana::cost_now(state, &a);
        let cost_b = crate::mana::cost_now(state, &b);
        if cost_a != cost_b {
            return if cost_a > cost_b { Some(a) } else { Some(b) };
        }
        if lane(&b) < lane(&a) { Some(b) } else { Some(a) }
    }
}

/// Die Insect (R758): {insect} damage to a random enemy Unit acting on the field or the enemy hero,
/// each as likely. The Radiant face is Lucky 1: two picks, the better kept.
fn die_insect(radiant: bool, amount: i32) -> Effect {
    Effect::new("dieInsect", move |ctx| {
        let mut pool: Vec<InsectPick> =
            crate::effects::cards_in_scope(ctx, &json_as(json!({ "side": "enemy" })))
                .iter()
                .map(|card| Some(CardInstance::clone(card)))
                .collect();
        pool.push(None);
        let roll = |rng: &mut Rng| -> InsectPick { rng.pick(&pool).cloned().flatten() };
        // R987: the controller's Luck rolls extra times beside the Radiant face's own Lucky.
        let luck = crate::query::luck_of(&*ctx.sink.state, ctx.controller);
        let lucky = if radiant { DIE_INSECT_LUCKY } else { 0 } + luck;
        let pick = if lucky > 0 {
            ctx.sink
                .rng
                .lucky(lucky, roll, better_insect_pick(&*ctx.sink.state))
        } else {
            roll(&mut *ctx.sink.rng)
        };
        let to = match pick {
            None => json!({ "of": "enemyHero" }),
            Some(card) => json!({ "of": "instance", "instanceId": card.id }),
        };
        let hit = crate::effects::damage(json_as(json!({ "to": to, "amount": amount })));
        (hit.apply)(ctx);
    })
}

/// A named builder, so the table stays `const`.
fn die_insect_power(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    vec![die_insect(
        radiant,
        power_number(ctx, DIE_INSECT_PARAM, DIE_INSECT_DAMAGE),
    )]
}

/// R759: the Spells (the Spell type, not Field Spells) in the activating player's hand, read once.
fn spells_in_hand(ctx: &EffectContext<'_>) -> Vec<String> {
    let state: &GameState = &*ctx.sink.state;
    state.players[ctx.controller]
        .hand
        .iter()
        .filter(|card| crate::faces::card_type_of(state, card) == CardType::Spell)
        .map(|card| card.id.clone())
        .collect()
}

/// KY Brainstorm (R759): a random KY card of every set (R380) to your hand — Radiant on the Radiant
/// face — then every Spell in your hand, that card included, costs ({discount}) less there.
fn ky_brainstorm(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let discount = power_number(ctx, BRAINSTORM_PARAM, BRAINSTORM_DISCOUNT);
    let add = if radiant {
        json!({ "query": { "tags": ["KY"] }, "radiant": true })
    } else {
        json!({ "query": { "tags": ["KY"] } })
    };
    vec![
        crate::effects::add_random_from_catalog(json_as(add)),
        crate::effects::each::for_each_card(crate::effects::each::ForEachCardArgs {
            cards: std::sync::Arc::new(|ctx: &mut EffectContext<'_>| spells_in_hand(ctx)),
            each: std::sync::Arc::new(move |instance_id: &str| {
                crate::effects::set_cost_mod(json_as(json!({
                    "target": { "of": "instance", "instanceId": instance_id },
                    "amount": -discount,
                    "inHandOnly": true
                })))
            }),
        }),
    ]
}

/// Pluck (R760): a random Fruit (R382's pool, a Grape rolled again) to your hand, costing ({fruitCost}),
/// (0) as printed; Radiant on the Radiant face.
fn pluck(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let cost = power_number(ctx, PLUCK_PARAM, PLUCK_COST);
    let args = if radiant {
        json!({ "query": { "tags": ["Fruit"] }, "costOverride": cost, "radiant": true })
    } else {
        json!({ "query": { "tags": ["Fruit"] }, "costOverride": cost })
    };
    vec![crate::effects::add_random_from_catalog(json_as(args))]
}

/// Terminus Tricks (R761): Discover a Trap and summon it face-down into your leftmost free backrow zone
/// (§3.2, R64), fizzling with none (R47); Radiant on the Radiant face.
fn terminus_tricks(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    if let Some(picked) = crate::effects::chosen_options(ctx).first().cloned() {
        let args = if radiant {
            json!({ "defId": picked, "player": "self", "radiant": true })
        } else {
            json!({ "defId": picked, "player": "self" })
        };
        return vec![crate::effects::summon(json_as(args))];
    }
    vec![crate::effects::discover_from_catalog(json_as(json!({
        "step": POWER_RESUME,
        "query": { "type": TRAP_TYPES },
        "prompt": if radiant { "Discover a Radiant Trap to summon" } else { "Discover a Trap to summon" },
        "data": { POWER_DATA_KEY: "tricks" }
    })))]
}

/// R3551002: End Times (S01's fourteenth power): "Discover a Terminus card." Its answer is the
/// named `defId`, never Radiant on either face; with no answer (no such card) nothing happens.
/// `end_times_reset` runs first, so a use takes back what the ends of turn took off.
fn end_times(ctx: &mut EffectContext<'_>, _radiant: bool) -> Vec<Effect> {
    if let Some(picked) = crate::effects::chosen_options(ctx).first().cloned() {
        return vec![crate::effects::add_to_hand(json_as(json!({
            "defId": picked,
            "player": "self"
        })))];
    }
    vec![
        end_times_reset(),
        crate::effects::discover_from_catalog(json_as(json!({
            "step": POWER_RESUME,
            "query": { "tags": ["Terminus"], "withTokens": true },
            "prompt": END_TIMES_PROMPT,
            "data": { POWER_DATA_KEY: "endTimes" }
        }))),
    ]
}

/// R3551002: End Times' End of Turn, on its controller's end of turn alone (the hook's): it takes
/// the face's cut (1, Radiant 2) off the next use's price, never below 0, and banks what it took in
/// `END_TIMES_TAKEN_KEY` so the use takes it back. With any other power, nothing.
pub fn end_times_tick() -> Effect {
    Effect::new("endTimesTick", |ctx| {
        let Some(card) = ctx.live_self().cloned() else {
            return;
        };
        if power_of(&card).map(|power| power.name) != Some(HeroPowerName::EndTimes) {
            return;
        }
        let price =
            crate::params::declared_or(&*ctx.sink.state, &card, END_TIMES_PRICE_PARAM, END_TIMES_COST);
        let printed_cut = if ctx.radiant {
            END_TIMES_RADIANT_CUT
        } else {
            END_TIMES_CUT
        };
        let cut = power_number(ctx, END_TIMES_CUT_PARAM, printed_cut);
        let step = cut.min(price).max(0);
        if step <= 0 {
            return;
        }
        let lower = crate::effects::set_number(json_as(json!({
            "target": { "of": "self" },
            "which": format!("param:{END_TIMES_PRICE_PARAM}"),
            "value": price - step
        })));
        (lower.apply)(ctx);
        let taken = card
            .memory
            .get(END_TIMES_TAKEN_KEY)
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32;
        write_memory(ctx, &card.id, END_TIMES_TAKEN_KEY, json!(taken + step));
    })
}

/// R3551002: give back what the ends of turn took off, so a used End Times costs (10) again: the
/// price moves back up by the banked `END_TIMES_TAKEN_KEY`, banked to 0.
fn end_times_reset() -> Effect {
    Effect::new("endTimesReset", |ctx| {
        let Some(card) = ctx.live_self().cloned() else {
            return;
        };
        let taken = card
            .memory
            .get(END_TIMES_TAKEN_KEY)
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32;
        if taken <= 0 {
            return;
        }
        let price =
            crate::params::declared_or(&*ctx.sink.state, &card, END_TIMES_PRICE_PARAM, END_TIMES_COST);
        let back = crate::effects::set_number(json_as(json!({
            "target": { "of": "self" },
            "which": format!("param:{END_TIMES_PRICE_PARAM}"),
            "value": price + taken
        })));
        (back.apply)(ctx);
        write_memory(ctx, &card.id, END_TIMES_TAKEN_KEY, json!(0));
    })
}

/// The thirteen powers of §8 #98 (R752), in R103's stored order.
pub const HERO_POWERS: &[HeroPower] = &[
    HeroPower {
        name: HeroPowerName::Recruit,
        title: "Expedition Map",
        radiant_title: "Expedition Map",
        x: 2,
        x_param: None,
        label: "Recruit a permanent.",
        radiant_label: "Recruit a permanent. Make it Radiant.",
        targets: None,
        build: expedition_map,
    },
    HeroPower {
        name: HeroPowerName::Draw,
        title: "Life Tap",
        radiant_title: "Life Tap",
        x: 0,
        x_param: None,
        label: "Draw {tapDraw}. Take {tapDamage} damage.",
        radiant_label: "Draw {tapDraw} from each player's deck.",
        targets: None,
        build: life_tap,
    },
    HeroPower {
        name: HeroPowerName::Ping,
        title: "Ping",
        radiant_title: "Ping",
        x: 0,
        x_param: None,
        label: "Pierce. Deal {ping} damage.",
        radiant_label: "Pierce. Deal {ping} damage. If this kills a Unit, summon a Ghoul Token with its stats.",
        targets: Some(ping_targets),
        build: ping,
    },
    HeroPower {
        name: HeroPowerName::Burn,
        title: "Steady Shot",
        radiant_title: "Steady Shot",
        x: 0,
        x_param: None,
        label: "Deal {shot} damage to the enemy hero.",
        radiant_label: "Deal {shot} damage to the enemy hero. This permanently deals 2 more damage.",
        targets: None,
        build: steady_shot,
    },
    HeroPower {
        name: HeroPowerName::Rush,
        title: "Ranching",
        radiant_title: "Ranching",
        x: 1,
        x_param: None,
        label: "Summon a Rush Token.",
        radiant_label: "Summon a Radiant Rush Token.",
        targets: None,
        build: ranching,
    },
    HeroPower {
        name: HeroPowerName::Felinor,
        title: "Cat Cafe",
        radiant_title: "Cat Cafe",
        x: 0,
        x_param: None,
        label: "Summon a Felinor Token.",
        radiant_label: "Summon a random Felinor.",
        targets: None,
        build: cat_cafe,
    },
    HeroPower {
        name: HeroPowerName::Discover,
        title: "Witness Value",
        radiant_title: "Witness Value",
        x: 1,
        x_param: None,
        label: "Discover a Unit.",
        radiant_label: "Discover a Radiant Unit.",
        targets: None,
        build: witness_value,
    },
    HeroPower {
        name: HeroPowerName::Stitching,
        title: "Stitching",
        radiant_title: "Stitching",
        x: 1,
        x_param: None,
        label: "Discover two ({stitchCost}) Cost or less Units. Fuse them.",
        radiant_label: "Discover two Radiant ({stitchCost}) Cost or less Units. Fuse them.",
        targets: None,
        build: stitching,
    },
    HeroPower {
        name: HeroPowerName::Armor,
        title: "Armor Up",
        radiant_title: "Tank Up",
        x: 0,
        x_param: None,
        label: "Your hero gains {armor} Armor until your next turn.",
        radiant_label: "Your hero gains {armor} Armor, then this power refreshes.",
        targets: None,
        build: armor_up,
    },
    HeroPower {
        name: HeroPowerName::Insect,
        title: "Die Insect",
        radiant_title: "Die Insect",
        x: 1,
        x_param: None,
        label: "Deal {insect} damage to a random enemy.",
        radiant_label: "Lucky 1. Deal {insect} damage to a random enemy.",
        targets: None,
        build: die_insect_power,
    },
    HeroPower {
        name: HeroPowerName::Brainstorm,
        title: "KY Brainstorm",
        radiant_title: "KY Brainstorm",
        x: 1,
        x_param: None,
        label: "Add a random KY card to your hand. Reduce the cost of all Spells in your hand by ({discount}).",
        radiant_label: "Add a Radiant KY card to your hand. Reduce the cost of all Spells in your hand by ({discount}).",
        targets: None,
        build: ky_brainstorm,
    },
    HeroPower {
        name: HeroPowerName::Pluck,
        title: "Pluck",
        radiant_title: "Pluck",
        x: 1,
        x_param: None,
        label: "Add a random Fruit to your hand. It costs ({fruitCost}).",
        radiant_label: "Add a random Radiant Fruit to your hand. It costs ({fruitCost}).",
        targets: None,
        build: pluck,
    },
    HeroPower {
        name: HeroPowerName::Tricks,
        title: "Terminus Tricks",
        radiant_title: "Terminus Tricks",
        x: 2,
        x_param: None,
        label: "Discover a Trap to summon.",
        radiant_label: "Discover a Radiant Trap to summon.",
        targets: None,
        build: terminus_tricks,
    },
    HeroPower {
        name: HeroPowerName::EndTimes,
        title: "End Times",
        radiant_title: "End Times",
        x: END_TIMES_COST,
        x_param: Some(END_TIMES_PRICE_PARAM),
        label: "Discover a Terminus card. Each end of turn, reduce the cost of the next use of this by ({endTimesCut}).",
        radiant_label: "Discover a Terminus card. Each end of turn, reduce the cost of the next use of this by ({endTimesCut}).",
        targets: None,
        build: end_times,
    },
];

/// R3551002: the declared number End Times' price is read off (`power: "endTimes"`, R1430), and
/// the number its End of Turn takes off, (1) on the base face and (2) on the Radiant one.
pub const END_TIMES_PRICE_PARAM: &str = "endTimes";
pub const END_TIMES_CUT_PARAM: &str = "endTimesCut";

/// R3551002: where the cuts taken so far wait while End Times sits unused, so its activation takes
/// them back.
pub const END_TIMES_TAKEN_KEY: &str = "endTimesTaken";

/// R3551002: what End Times Discovers.
pub const END_TIMES_PROMPT: &str = "Discover a Terminus card";

/// R3551002: the thirteen powers v0.5.13g and the versions before it offered, without End Times;
/// `THIRTEEN_POWER_NAMES` the same over the stored names.
pub const THIRTEEN_POWERS: &[HeroPower] = HERO_POWERS.split_at(POWERS_THROUGH_V0_5_13G).0;
pub const THIRTEEN_POWER_NAMES: &[HeroPowerName] = HeroPowerName::ALL.split_at(POWERS_THROUGH_V0_5_13G).0;

/// R3551002: which Heroic Power the card plays (S01): `EndTimes` for the new face, `Thirteen` for
/// the kept v0.5.13g script's. The script's declarations pick it, never a new `ActivationCost` field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerEdition {
    Thirteen,
    EndTimes,
}

impl PowerEdition {
    /// The powers the edition offers, and the price each use pays (`HERO_POWER_DISCOUNT` off the
    /// thirteen for the new face; EndTimes' own X moves as its ends of turn take off it).
    pub fn powers(self) -> &'static [HeroPower] {
        match self {
            PowerEdition::Thirteen => THIRTEEN_POWERS,
            PowerEdition::EndTimes => HERO_POWERS,
        }
    }

    /// The stored names the edition may roll or offer.
    pub fn names(self) -> &'static [HeroPowerName] {
        match self {
            PowerEdition::Thirteen => THIRTEEN_POWER_NAMES,
            PowerEdition::EndTimes => HeroPowerName::ALL,
        }
    }

    /// The mana one use of `power` pays under this edition.
    pub fn price(self, power: &HeroPower) -> i32 {
        match self {
            PowerEdition::Thirteen => power.x + HERO_POWER_DISCOUNT,
            PowerEdition::EndTimes => power.x,
        }
    }
}

/// R3551002: the edition the card plays: End Times if and only if its running script declares the
/// `endTimes` ability. `abilities_of` is not read: its `has` filter hides that id.
pub fn edition_of(state: &GameState, card: &CardInstance) -> PowerEdition {
    let script = crate::scripts::script_of(state, card);
    let end_times = script
        .activations
        .iter()
        .any(|decl| decl.id.split('#').next() == Some(HeroPowerName::EndTimes.as_str()));
    if end_times {
        PowerEdition::EndTimes
    } else {
        PowerEdition::Thirteen
    }
}

/// The stored names in the table's order.
pub const HERO_POWER_NAMES: &[HeroPowerName] = HeroPowerName::ALL;

pub fn power_by_name(name: &str) -> Option<&'static HeroPower> {
    HERO_POWERS.iter().find(|power| power.name.as_str() == name)
}

// The power on the instance (R43).

/// The power this card rolled, or null for a card that has not rolled one yet.
pub fn power_of(instance: &CardInstance) -> Option<&'static HeroPower> {
    instance
        .memory
        .get(POWER_KEY)
        .and_then(Value::as_str)
        .and_then(power_by_name)
}

/// The power's name on the card's face (R752): Armor Up reads Tank Up on a Radiant card.
pub fn power_title_of(power: &HeroPower, radiant: bool) -> &'static str {
    if radiant { power.radiant_title } else { power.title }
}

/// R43's roll, idempotent: #98 calls it from `startOfGame` for every copy in a hand or library (§6.2)
/// and again whenever a copy arrives without a power (a bounced or reset instance, R78, R151).
/// R3551002: the card's edition picks the roll (End Times joins the new face's roll alone).
pub fn ensure_power(sink: &mut EngineSink<'_>, instance: &mut CardInstance) -> Option<&'static HeroPower> {
    if let Some(existing) = power_of(instance) {
        return Some(existing);
    }
    let edition = edition_of(&*sink.state, instance);
    let rolled = sink.rng.pick(edition.powers())?;
    instance
        .memory
        .insert(POWER_KEY.to_string(), json!(rolled.name.as_str()));
    Some(rolled)
}

/// R43's roll as an effect, so #98's `startOfGame` hook is a list of effects like every other card's
/// (CLAUDE.md rule 5).
pub fn roll_power() -> Effect {
    Effect::new("rollPower", |ctx| {
        let Some(mut card) = ctx.live_self().cloned() else {
            return;
        };
        ensure_power(&mut ctx.sink, &mut card);
        if let Some(power) = card.memory.get(POWER_KEY).cloned() {
            let id = card.id.clone();
            write_memory(ctx, &id, POWER_KEY, power);
        }
    })
}

/// R3134002: the card's power set to `name`, replacing any it held (the Discover's answer). The uses
/// record is left as it is: a card that is played is a fresh instance (R78), with none.
pub fn set_power(name: HeroPowerName) -> Effect {
    Effect::new("setPower", move |ctx| {
        let Some(card) = ctx.live_self() else {
            return;
        };
        let id = card.id.clone();
        write_memory(ctx, &id, POWER_KEY, json!(name.as_str()));
    })
}

/// R3134002: "Cry: Discover one of 13 powers." A Choose one of `DISCOVER_OPTIONS` different powers, drawn
/// without replacement from the thirteen through the match rng (R60), each named by its title on the
/// card's face (Tank Up for Armor Up on the Radiant one, R757). Its answer re-enters `hero_power` at
/// `POWER_RESUME` and becomes the card's power. Only its controller reads the offer (§10.6).
/// R3551002: the card's edition picks the offer (End Times joins the new face's Cry alone).
pub fn discover_power() -> Effect {
    Effect::new("discoverPower", |ctx| {
        let radiant = ctx.radiant;
        let edition = ctx
            .live_self()
            .map(|card| edition_of(&*ctx.sink.state, card))
            .unwrap_or(PowerEdition::Thirteen);
        let offered: Vec<&'static HeroPower> = ctx
            .sink
            .rng
            .shuffle(edition.names())
            .into_iter()
            .take(DISCOVER_OPTIONS.max(0) as usize)
            .filter_map(|name| power_by_name(name.as_str()))
            .collect();
        let options: Vec<String> = offered
            .iter()
            .map(|power| power.name.as_str().to_string())
            .collect();
        let labels: IndexMap<String, String> = offered
            .iter()
            .map(|power| {
                (
                    power.name.as_str().to_string(),
                    power_title_of(power, radiant).to_string(),
                )
            })
            .collect();
        let discover = crate::effects::choose_mode(crate::effects::ChooseModeArgs {
            options,
            step: POWER_RESUME.to_string(),
            prompt: Some(DISCOVER_POWER_PROMPT.to_string()),
            data: Some(IndexMap::from([(POWER_DATA_KEY.to_string(), json!(POWER_PICK))])),
            by: None,
            labels: Some(labels),
        });
        (discover.apply)(ctx);
    })
}

/// R3134002: R151's arrival roll for a copy that reaches the field with no Cry to choose: Summoned,
/// Recruited, fused onto a card on the field, Reborn or Flickered back (`summon_onto`'s and the fuse's
/// arrival clause), so it is never dead. A copy in a hand or a library rolls nothing: playing it runs
/// the Cry, which asks.
pub fn roll_power_on_field() -> Effect {
    Effect::new("rollPowerOnField", |ctx| {
        if ctx
            .live_self()
            .is_some_and(|card| card.zone.z() == ZoneName::Field)
        {
            (roll_power().apply)(ctx);
        }
    })
}

/// R752: the edition's Activate abilities on the face `radiant` names, each paying its edition
/// price (R3551002), declaring its targets (R81) and the card's only while it rolled that power
/// (`has`). The id is the stored name (R103), so a fused card's ids (R102), a resumed tail and the
/// alias name the same power.
pub fn power_abilities(radiant: bool, edition: PowerEdition) -> Vec<ActivationDecl> {
    edition
        .powers()
        .iter()
        .map(|power: &'static HeroPower| {
            let name = power.name.as_str();
            ActivationDecl {
                id: name.to_string(),
                label: format!(
                    "{}: {}",
                    power_title_of(power, radiant),
                    if radiant { power.radiant_label } else { power.label }
                ),
                uses: ActivationUses::Count(1),
                cost: Some(ActivationCost {
                    mana: Some(edition.price(power)),
                    mana_param: power.x_param,
                    ..ActivationCost::default()
                }),
                targets: power.targets.map(|targets| targets()).unwrap_or_default(),
                modes: vec![],
                can_activate: None,
                has: Some(read_hook(move |args| {
                    args.self_.memory.get(POWER_KEY).and_then(Value::as_str) == Some(name)
                })),
                run: hook(move |ctx| (power.build)(ctx, radiant)),
            }
        })
        .collect()
}

/// The step a Discover comes back to (§10.6): the power's own builder finishes the activation, and the
/// Discover of the card's own power (R3134002, `POWER_PICK`) writes the power it picked.
pub fn hero_power(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let tag = ctx.data.get(POWER_DATA_KEY).and_then(Value::as_str);
    if tag == Some(POWER_PICK) {
        return match crate::effects::chosen_options(ctx)
            .first()
            .and_then(|name| power_by_name(name))
        {
            Some(picked) => vec![set_power(picked.name)],
            None => vec![],
        };
    }
    let named = tag.and_then(power_by_name);
    let power = named.or_else(|| ctx.live_self().and_then(power_of));
    let Some(power) = power else {
        return vec![];
    };
    let radiant = ctx.radiant;
    (power.build)(ctx, radiant)
}

/// The ability of the power this card rolled: its id is the power's name, `<name>#n` on a fusion (R102).
pub fn power_ability_of(state: &GameState, card: &CardInstance) -> Option<ActivationDecl> {
    let power = power_of(card)?;
    abilities_of(state, card)
        .into_iter()
        .find(|decl| decl.id.split('#').next() == Some(power.name.as_str()))
}

/// R3551002: the mana the card's power costs to use now: the declared `endTimes` moved by its ends
/// of turn, or the edition's price of any other power.
pub fn power_price(state: &GameState, card: &CardInstance) -> Option<i32> {
    let decl = power_ability_of(state, card)?;
    decl.cost
        .map(|cost| super::activate::mana_price(state, card, cost))
}

/// R752: whether the card's power has spent this turn's uses (Activate's count, R384).
pub fn used_this_turn(state: &GameState, card: &CardInstance) -> bool {
    match power_ability_of(state, card) {
        None => false,
        Some(decl) => ability_uses_this_turn(state, card, &decl.id) >= uses_allowed(card, &decl),
    }
}
