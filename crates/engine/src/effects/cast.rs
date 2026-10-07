//! Cast, from anywhere (SPEC §6.3's Cast row, docs/classic-sets.md B5 E12; R70, R452, R453), and the
//! price rules a card installs on a player (B5 E15, E39; R455).
//!
//! A cast is §10.5's pipeline entered at step 3 (`resolve.castCard`): free, counted as a play (R70),
//! announced like one (E1), its Cry or spell script fired, its choices its caster's — or, for a random
//! cast, the rng's (R452). These verbs only choose the card and how it is cast:
//!
//!  - `cast` — a card that exists, in a hand, a library, a graveyard or an exile pile (Classic #56 Spell
//!    Tyrant's Spells, cast out of the graveyard and exiled after, R453);
//!  - `castEach` — every card of a set read once as the list reaches it (#56 Radiant's "every Spell in
//!    your graveyard, oldest first"), so a pause inside one cast resumes over the same set (R113);
//!  - `castNew` — a new card of a named definition (Classic #47 casts #34 Ancient Acquisition; #7
//!    InfiniScepter's copy of the Spell it remembers; Classic+ #37 Wardrum's copies), which the caster
//!    owns and which lands in the caster's graveyard after it resolves (R87);
//!  - `castRandom` — random cards from a pool, every choice random (Classic+ #47 Jogg's Box, #38.1
//!    Solarius Prime), never the casting card's own definition (B4.1, R387).
//!
//! A cast verb that would begin a cast past RANDOM_CAST_CHAIN_CAP inside a random cast's chain resolves
//! into nothing (R452), as R28's Call to Chaos cap does.
//!
//! Port of `packages/engine/src/effects/cast.ts`. An argument TS typed as a value or a function of the
//! context (`castNew`'s `def`, `castRandom`'s `query` and `count`) is an enum here with one variant per
//! form; an argument struct that holds a function derives `Clone` only (a function is not data).

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::catalog::{CatalogQueryArgs, excluding_def_id, pick_generated, query};
use crate::effects::each::{ForEachCardArgs, for_each_card};
use crate::effects::targets::{PlayerSpec, TargetSpec, instance_of, player_of};
use crate::modifiers::add_modifier;
use crate::random_cast::may_cast_now;
use crate::resolve::{CastAfterward, CastOptions, cast_card, lazy_part};
use crate::script::{Effect, EffectContext, EffectPart, EngineSink};
use crate::state::{CostRule, ModifierExpiry, ModifierKind, find_instance_mut, new_instance};
use crate::subsystems::call_to_chaos::{CHAOS_CHAIN_KEY, CHAOS_TAG, chaos_chain_of};
use crate::wire::{Enchantment, PlayerId, Zone, ZoneName};

/// How a cast verb casts (`resolve.CastOptions`, without the choices R70 leaves to the caster).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CastHow {
    /// R452: every choice at random.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random: Option<bool>,
    /// R452, R656: target picks aim by declaration ("Each aims at enemies when it harms and at your side when it helps").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_enemies: Option<bool>,
    /// R453: a resolved Spell goes to exile instead of its graveyard ("Cast them, then exile them").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub afterward: Option<CastAfterward>,
}

/// The piles a card that exists may be cast from: never the field, where it already is.
const CASTABLE_ZONES: &[ZoneName] = &[
    ZoneName::Hand,
    ZoneName::Library,
    ZoneName::Graveyard,
    ZoneName::Exile,
];

fn sink_of<'b>(ctx: &'b mut EffectContext<'_>) -> EngineSink<'b> {
    ctx.sink.reborrow()
}

fn options_of(how: &CastHow) -> CastOptions {
    CastOptions {
        random: if how.random == Some(true) {
            Some(true)
        } else {
            None
        },
        target_enemies: if how.target_enemies == Some(true) {
            Some(true)
        } else {
            None
        },
        afterward: how.afterward,
        ..CastOptions::default()
    }
}

/// `cast`'s argument: `{ target? } & CastHow`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CastArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(flatten)]
    pub how: CastHow,
}

/// E12, R453: cast a card that exists (default: the first chosen one). It is cast by the running card's
/// controller, who makes its choices (R70), from whatever pile it lies in; its owner does not change,
/// so it lands in its owner's piles afterwards. A card on the field, or one that has ceased to exist,
/// is not cast.
pub fn cast(args: CastArgs) -> Effect {
    Effect::new("cast", move |ctx| {
        let spec = args.target.clone().unwrap_or(TargetSpec::Chosen { index: None });
        let Some(mut card) = instance_of(ctx, &spec) else {
            return;
        };
        if !CASTABLE_ZONES.contains(&card.zone.z()) {
            return;
        }
        if !may_cast_now(ctx.sink.state, ctx.controller) {
            return;
        }
        let controller = ctx.controller;
        if let Some(live) = find_instance_mut(ctx.sink.state, &card.id) {
            live.controller = controller;
        }
        card.controller = controller;
        let options = options_of(&args.how);
        cast_card(&mut sink_of(ctx), &card, options);
    })
}

/// What `castEach` reads its set from: the ids of the cards, read once as the list reaches the clause
/// (TS `(ctx) => readonly (CardInstance | string)[]`; a card file maps its instances to their ids).
pub type CastEachCards = Arc<dyn Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync>;

/// `castEach`'s argument: `{ cards } & CastHow`.
#[derive(Clone)]
pub struct CastEachArgs {
    pub cards: CastEachCards,
    pub how: CastHow,
}

/// E12, R453: `cast` each card of a set read once, as the list reaches it (`forEachCard`, R113), one
/// after another, each with its caster's choices — Classic #56 Radiant's "Cast every Spell in your
/// graveyard, oldest first": a Spell a cast puts in the graveyard is not one of them. A card that has
/// left its pile by its turn (an earlier cast exiled it) is passed over.
pub fn cast_each(args: CastEachArgs) -> Effect {
    let how = args.how.clone();
    for_each_card(ForEachCardArgs {
        cards: args.cards.clone(),
        each: Arc::new(move |instance_id: &str| {
            cast(CastArgs {
                target: Some(TargetSpec::Instance {
                    instance_id: instance_id.to_string(),
                }),
                how: how.clone(),
            })
        }),
    })
}

/// What `castNew` casts: a definition, and the face to cast it with.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CastDef {
    pub def_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// `castNew`'s `def`: a definition id, a `CastDef`, or a read of the context as the effect applies
/// (#7's remembered Spell), where `None` casts nothing (TS `string | CastDef | ((ctx) => string |
/// CastDef | null)`). A card's `json!` literal reads as `Id` or `Def`; `Read` is set in Rust.
#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum CastNewDef {
    Id(String),
    Def(CastDef),
    #[serde(skip)]
    Read(CastDefReader),
}

/// `CastNewDef::Read`'s function: the definition to cast, read when the effect resolves.
pub type CastDefReader = Arc<dyn Fn(&mut EffectContext<'_>) -> Option<CastDef> + Send + Sync>;

impl From<&str> for CastNewDef {
    fn from(def_id: &str) -> CastNewDef {
        CastNewDef::Id(def_id.to_string())
    }
}

impl From<String> for CastNewDef {
    fn from(def_id: String) -> CastNewDef {
        CastNewDef::Id(def_id)
    }
}

impl From<CastDef> for CastNewDef {
    fn from(def: CastDef) -> CastNewDef {
        CastNewDef::Def(def)
    }
}

/// `castNew`'s argument: `{ def; radiant? } & CastHow`.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastNewArgs {
    pub def: CastNewDef,
    #[serde(default)]
    pub radiant: Option<bool>,
    #[serde(flatten)]
    pub how: CastHow,
}

/// E12, R453: cast a new card of a named definition — the running card's controller makes it, owns it
/// and casts it; it lands in their graveyard after it resolves (R87). `def` may be read off the context
/// as the effect applies (#7's remembered Spell); null casts nothing. `radiant` is the face when `def`
/// does not say (Classic+ #37's copy "by definition and face").
pub fn cast_new(args: CastNewArgs) -> Effect {
    Effect::new("castNew", move |ctx| {
        let named: Option<CastDef> = match &args.def {
            CastNewDef::Id(def_id) => Some(CastDef {
                def_id: def_id.clone(),
                radiant: None,
            }),
            CastNewDef::Def(def) => Some(def.clone()),
            CastNewDef::Read(read) => read(ctx),
        };
        let Some(def) = named else {
            return;
        };
        if !may_cast_now(ctx.sink.state, ctx.controller) {
            return;
        }
        let controller = ctx.controller;
        let mut card = new_instance(
            &mut *ctx.sink.state,
            &def.def_id,
            controller,
            Zone::Resolving { player: controller },
        );
        card.radiant = def.radiant.or(args.radiant).unwrap_or(false);
        let options = options_of(&args.how);
        cast_card(&mut sink_of(ctx), &card, options);
    })
}

/// One random cast of `castRandom`: its own card, drawn as it applies.
fn cast_one_random(asked: CatalogQueryArgs, radiant: bool, how: CastHow) -> Effect {
    Effect::new("castRandom:one", move |ctx| {
        if !may_cast_now(ctx.sink.state, ctx.controller) {
            return;
        }
        // B4.1, R387: never the casting card's own definition, named by its id.
        let own = ctx
            .self_
            .as_ref()
            .map(|card| card.def_id.clone())
            .or_else(|| ctx.def_id.clone());
        let pool = query(&excluding_def_id(Some(&*ctx.sink.state), &asked, own.as_deref()));
        let Some(def) = pick_generated(&mut *ctx.sink.rng, &pool, None) else {
            return;
        };
        let def_id = def.id.clone();
        let chaos = def.tags.contains(&CHAOS_TAG);
        let controller = ctx.controller;
        let mut card = new_instance(
            &mut *ctx.sink.state,
            &def_id,
            controller,
            Zone::Resolving { player: controller },
        );
        card.radiant = radiant;
        // R28: a Call to Chaos a random cast makes is a cast of the chain (Classic+ #47 Jogg's Box), one
        // deeper than the card casting it, so it counts against CALL_TO_CHAOS_CHAIN_CAP.
        if chaos {
            let depth = chaos_chain_of(ctx.self_.as_ref()) + 1;
            card.memory.insert(CHAOS_CHAIN_KEY.to_string(), json!(depth));
        }
        let options = CastOptions {
            random: Some(true),
            ..options_of(&how)
        };
        cast_card(&mut sink_of(ctx), &card, options);
    })
}

/// `castRandom`'s `query`: a fixed query, or one read off the context as the part is built (set in
/// Rust; a `json!` literal reads as `Fixed`).
#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum CastRandomQuery {
    Fixed(CatalogQueryArgs),
    #[serde(skip)]
    Read(Arc<dyn Fn(&mut EffectContext<'_>) -> CatalogQueryArgs + Send + Sync>),
}

impl From<CatalogQueryArgs> for CastRandomQuery {
    fn from(query: CatalogQueryArgs) -> CastRandomQuery {
        CastRandomQuery::Fixed(query)
    }
}

/// `castRandom`'s `count`: a number, or one read off the context as the part is built (set in Rust;
/// a `json!` number reads as `Fixed`).
#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum CastRandomCount {
    Fixed(i32),
    #[serde(skip)]
    Read(Arc<dyn Fn(&mut EffectContext<'_>) -> i32 + Send + Sync>),
}

impl From<i32> for CastRandomCount {
    fn from(count: i32) -> CastRandomCount {
        CastRandomCount::Fixed(count)
    }
}

/// `castRandom`'s argument: `{ query; count?; radiant? } & Omit<CastHow, "random">`.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastRandomArgs {
    pub query: CastRandomQuery,
    #[serde(default)]
    pub count: Option<CastRandomCount>,
    #[serde(default)]
    pub radiant: Option<bool>,
    #[serde(default)]
    pub target_enemies: Option<bool>,
    #[serde(default)]
    pub afterward: Option<CastAfterward>,
}

/// E12, R452: cast `count` random cards from a pool (§5.1's `query`: no tokens unless it asks, never the
/// casting card's own definition), one after another, each a random cast — every choice random, its X
/// the caster's current mana (R453) — made and owned by the caster, landing in their graveyard (R87).
/// Classic+ #47 Jogg's Box: `{ query: { type: "Spell" }, count: 10 }`; #38.1 Solarius Prime: five, with
/// `targetEnemies` and, on its Radiant face, `radiant`. The casts are parts of the list read once as it
/// reaches them, so one the other player's prompt pauses is followed by the rest when it is answered.
pub fn cast_random(args: CastRandomArgs) -> Effect {
    lazy_part("castRandom", move |ctx, memo| {
        let count = match memo.as_ref().and_then(Value::as_i64) {
            Some(remembered) => remembered as i32,
            None => {
                let asked = match &args.count {
                    Some(CastRandomCount::Read(read)) => read(ctx),
                    Some(CastRandomCount::Fixed(count)) => *count,
                    None => 1,
                };
                asked.max(0)
            }
        };
        let asked = match &args.query {
            CastRandomQuery::Read(read) => read(ctx),
            CastRandomQuery::Fixed(query) => query.clone(),
        };
        let how = CastHow {
            random: None,
            target_enemies: args.target_enemies,
            afterward: args.afterward,
        };
        let radiant = args.radiant == Some(true);
        EffectPart {
            effects: (0..count)
                .map(|_| cast_one_random(asked.clone(), radiant, how.clone()))
                .collect(),
            memo: Some(json!(count)),
        }
    })
}

// ---------------------------------------------------------------------------
// Price rules on a player (E15, E39, R455)
// ---------------------------------------------------------------------------

/// How long a price rule lasts: until the play it prices is made ("your next …"), this turn, through the
/// named player's next turn (R48: "during their next turn", AI Alignment Tax), or for the rest of the
/// game.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum CostRuleSpan {
    Used,
    ThisTurn,
    TheirNextTurn,
    Never,
}

fn expiry_of(ctx: &EffectContext<'_>, span: CostRuleSpan, player: PlayerId) -> ModifierExpiry {
    match span {
        CostRuleSpan::Used => ModifierExpiry::Used,
        CostRuleSpan::ThisTurn => ModifierExpiry::ThisTurn {
            turn: ctx.sink.state.turn,
        },
        CostRuleSpan::TheirNextTurn => ModifierExpiry::NextTurnOf {
            player,
            from_turn: ctx.sink.state.turn,
        },
        CostRuleSpan::Never => ModifierExpiry::Never,
    }
}

/// `addCostRule`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddCostRuleArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    pub rule: CostRule,
    pub lasts: CostRuleSpan,
}

/// E15, R455: a price rule on a player's cards (`costRules.ts`) — Classic #2's "Your next Trap or Field
/// Spell costs (2) less" (`{ types: ["Trap", "Field Trap", "Field Spell"], amount: -2 }` until used, or
/// `setTo: 0` on its Radiant face), AI Alignment Tax's "Your opponent's cards cost (1) more during
/// their next turn" (`player: "enemy"`, `{ amount: 1 }`, "theirNextTurn"). A rule until used is spent by
/// the play whose price it changed (never by a cast, R70).
pub fn add_cost_rule(args: AddCostRuleArgs) -> Effect {
    Effect::new("addCostRule", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let expiry = expiry_of(ctx, args.lasts, player);
        add_modifier(
            ctx,
            player,
            expiry,
            ModifierKind::CostRule {
                rule: args.rule.clone(),
            },
        );
    })
}

/// `enchantNextSpell`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnchantNextSpellArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    pub enchantment: Enchantment,
}

/// E39, R455 (Classic+ #14 Forever&): "The next Spell you play gains …" — the enchantment is given to
/// that Spell as it is played (§10.5 step 4), and the rider is spent then. It waits across turns.
pub fn enchant_next_spell(args: EnchantNextSpellArgs) -> Effect {
    Effect::new("enchantNextSpell", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        add_modifier(
            ctx,
            player,
            ModifierExpiry::Used,
            ModifierKind::EnchantNextSpell {
                enchantment: args.enchantment.clone(),
            },
        );
    })
}
