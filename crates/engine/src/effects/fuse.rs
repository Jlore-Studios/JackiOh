//! Fuse as a verb (SPEC §6.3 Fuse, R77, R102; §8.4 #85, §8.5 #99).
//!
//! The subsystem (`subsystems::fuse`) is R77 in full, and none of it may be repeated here: a second
//! implementation is a second set of rules. This file exists only because `fuse` takes an `EngineSink`
//! and mutates state, which a card script may not do (CLAUDE.md rule 5), and because
//! `FuseArgs.ingredients` takes instances while a Discover hands over catalog ids. The `fused` event,
//! `state.transientDefs` and the script registration stay the subsystem's.
//!
//! An ingredient that was never a card (§8.5 #99) only contributes its definition, so it is an instance
//! in no pile, `{ z: "gone" }`, the subsystem's own "ceased to exist" (R11, R86): no zone event fires,
//! `find_instance` cannot reach it and no view lists it, so it is invisible as #99 requires.
//! `prompts.rs` calls `answer_fuse_onto` by its hook (SURFACE §6.6).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{CatalogQueryArgs, def_of, excluding_def_id, query};
use crate::config::FUSE_MIN_INGREDIENTS;
use crate::effects::choose::{DiscoverFromCatalogArgs, chosen_options, discover_from_catalog};
use crate::effects::move_::{ExileArgs, exile};
use crate::effects::targets::{PlayerSpec, TargetSpec, instance_of, player_of, self_on_its_stay};
use crate::faces::card_type_of;
use crate::layers::unit_has;
use crate::prompts::{
    AnswerInput, OpenPromptArgs, close_prompt, in_offered_order, open_prompt, why_answer_refused,
};
use crate::script::{Effect, EffectContext, EngineSink, Hook, hook};
use crate::state::{CardInstance, EngineError, GameState, PromptOption, Resume, find_instance, new_instance};
use crate::subsystems::fuse::{FuseArgs, HandPrice, fuse};
use crate::wire::{CardDef, CardType, KeywordKind, PlayerId, PromptKind, Selection, Zone, ZoneName};
use crate::work::{begin_work_cascade, drain_work};
use crate::zones::{StandingAt, acting_on, is_buried};

/// R77 and R86: an ingredient that is only a definition, made in `{ z: "gone" }` (where the subsystem
/// moves a consumed one): in no pile, so no zone event fires and nothing can target or return it.
fn phantom_ingredient(ctx: &mut EffectContext<'_>, def_id: &str, owner: Option<PlayerSpec>) -> CardInstance {
    let player = match owner {
        None => ctx.controller,
        Some(spec) => player_of(ctx, spec),
    };
    new_instance(&mut *ctx.sink.state, def_id, player, Zone::Gone { player })
}

/// `fuseCards`' `pick`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum FuseCardsPick {
    Random,
    All,
}

/// `fuseCards`' argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FuseCardsArgs {
    /// Ingredients that are definitions only: #99's Discover picks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_ids: Option<Vec<String>>,
    /// Ingredients that already exist as cards: #85's played permanent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_ids: Option<Vec<String>>,
    /// R77's kept instance: the on-field ingredient the result becomes. #99 never passes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_instance_id: Option<String>,
    /// Several kept instances, one fusion each in the order given (#85 radiant's "onto every such
    /// permanent"). `targetInstanceId` is the one-target spelling of the same thing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_instance_ids: Option<Vec<String>>,
    /// #85 base is "a random permanent of yours of that type", so the draw is one `ctx.rng.pick` over
    /// the candidates INSIDE apply — never at construction time, or the draw escapes the reducer
    /// (§9.3, §10.7). Default "all", which is also the only behavior a single target can have.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pick: Option<FuseCardsPick>,
    /// R77's Craft a Card path: whose hand the fresh `costOverride` 0 result goes to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_hand: Option<PlayerSpec>,
    /// R352: the hand card's price, `"free"` (#99, the default) or R77's `"fused"` cost (#98's Stitching).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_price: Option<HandPrice>,
    /// R352: the hand card is made Radiant (#98's radiant Stitching).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    /// R470: the kept instance is this card in a hand or a library, which stays where it is (Classic+
    /// #31 Fusion Lab's pick, Classic #78 Radiant's). `targetInstanceId` is the field's spelling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub into_instance_id: Option<String>,
    /// R469: every ingredient but the kept card goes in on its Radiant face — "a Radiant copy of it is
    /// fused into this" (Classic+ #74 Radiant), "fuse 3 random Radiant AI generated cards" (#43 Radiant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant_ingredients: Option<bool>,
    /// R470: the kept card keeps the cost it had ("its cost doesn't change").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_cost: Option<bool>,
    /// ME-FUSE-RANDOM (Meditative #47 饕餮, MD-C20): instead of named ingredients, fuse one card
    /// drawn uniformly from this scope into the kept target — an enemy permanent on the field (tops
    /// of piles, both rows, face-down cards included, never Immutable) or a card of the opponent's
    /// library. A library ingredient goes in as a fresh phantom (its definition only), and the
    /// library card ceases to exist, so its id never reaches a view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random_ingredient: Option<crate::effects::card_scope::CardScope>,
}

/// §6.3 Fuse per R77, wrapped. Cards that already exist (`instanceIds`, #85) go in before definitions
/// that never were cards (`defIds`, #99), a convention since no Core card passes both; each group keeps
/// the caller's order, the order the fused name, text and scripts are concatenated in.
///
/// One fusion per target, looped in here (R77, §8.4 #85 radiant): after the first fusion the played
/// card is in no pile, so one `fuseCards` per target would leave every later one below
/// `FUSE_MIN_INGREDIENTS`. So the ingredients are resolved ONCE and reused, as an ingredient only ever
/// contributes its definition. R86's dropping of gone ids is for pools, not for ids named outright.
///
/// It fizzles silently and the card still resolves (§6.3): an unresolved `instanceIds` entry drops out,
/// and the subsystem changes nothing for fewer than `FUSE_MIN_INGREDIENTS`, a target off the field or
/// Immutable (R23), or a call that names neither a target nor a hand.
pub fn fuse_cards(args: FuseCardsArgs) -> Effect {
    Effect::new("fuseCards", move |ctx| {
        // ME-FUSE-RANDOM alone: its ingredient is drawn inside `apply` from the match rng.
        if let Some(scope) = &args.random_ingredient {
            fuse_random_ingredient(ctx, &args, scope);
            return;
        }
        // Resolved ONCE: every fusion below reuses these, consumed or not.
        let mut ingredients: Vec<CardInstance> = Vec::new();
        for instance_id in args.instance_ids.iter().flatten() {
            if let Some(found) = find_instance(ctx.sink.state, instance_id) {
                ingredients.push(found.clone());
            }
        }
        for def_id in args.def_ids.iter().flatten() {
            let phantom = phantom_ingredient(ctx, def_id, args.to_hand);
            ingredients.push(phantom);
        }

        let target_ids: Vec<String> = match (&args.target_instance_ids, &args.target_instance_id) {
            (Some(ids), _) => ids.clone(),
            (None, Some(id)) => vec![id.clone()],
            (None, None) => Vec::new(),
        };
        let mut targets: Vec<CardInstance> = Vec::new();
        for instance_id in &target_ids {
            if let Some(found) = find_instance(ctx.sink.state, instance_id) {
                targets.push(found.clone());
            }
        }

        let to_hand: Option<PlayerId> = args.to_hand.map(|spec| player_of(ctx, spec));
        let (hand_price, radiant) = if to_hand.is_some() {
            (
                args.hand_price,
                if args.radiant == Some(true) {
                    Some(true)
                } else {
                    None
                },
            )
        } else {
            (None, None)
        };
        // R469, R470: which ingredients go in Radiant, and whether the kept card keeps its cost.
        let radiant_ingredients: Option<Vec<String>> = if args.radiant_ingredients == Some(true) {
            Some(ingredients.iter().map(|card| card.id.clone()).collect())
        } else {
            None
        };
        let keep_cost = if args.keep_cost == Some(true) {
            Some(true)
        } else {
            None
        };

        // R470: a kept card in a hand or a library, which stays where it is.
        if let Some(into_id) = &args.into_instance_id {
            let Some(into) = find_instance(ctx.sink.state, into_id).cloned() else {
                return;
            };
            fuse(
                ctx,
                FuseArgs {
                    ingredients,
                    into: Some(into),
                    radiant_ingredients,
                    keep_cost,
                    ..FuseArgs::default()
                },
            );
            return;
        }

        // #99's path: no kept instance at all, so one fusion into a hand.
        if target_ids.is_empty() {
            fuse(
                ctx,
                FuseArgs {
                    ingredients,
                    to_hand,
                    hand_price,
                    radiant,
                    radiant_ingredients,
                    keep_cost,
                    ..FuseArgs::default()
                },
            );
            return;
        }

        // Nothing left to fuse onto: the trap fired and did nothing (R61), taking no rng draw.
        if targets.is_empty() {
            return;
        }

        let chosen: Vec<CardInstance> = if args.pick == Some(FuseCardsPick::Random) {
            ctx.sink.rng.pick(&targets).cloned().into_iter().collect()
        } else {
            targets
        };

        // R77: each target is its own fusion, in order, each with its own transient definition.
        for target in chosen {
            fuse(
                ctx,
                FuseArgs {
                    ingredients: ingredients.clone(),
                    target: Some(target),
                    to_hand,
                    hand_price,
                    radiant,
                    radiant_ingredients: radiant_ingredients.clone(),
                    keep_cost,
                    ..FuseArgs::default()
                },
            );
        }
    })
}

// The Fuse variants (docs/classic-sets.md B5 E23; R77, R102, R468–R470).

/// B4.1, R387: a random pool never offers the card generating from it — nor, for a fused card, any of
/// its ingredients (`catalog.selfDefIds`).
fn pool_for(ctx: &EffectContext<'_>, asked: Option<&CatalogQueryArgs>) -> Vec<&'static CardDef> {
    let own = ctx
        .self_
        .as_ref()
        .map(|card| card.def_id.clone())
        .or_else(|| ctx.def_id.clone());
    let asked = asked.cloned().unwrap_or_default();
    query(&excluding_def_id(Some(&*ctx.sink.state), &asked, own.as_deref()))
}

/// ME-FUSE-RANDOM (Meditative #47, MD-C20): fuse one card drawn uniformly from `scope` into the
/// live `targetInstanceId`. The pool leaves out the kept card and every Immutable card (being eaten
/// changes its text, R23); an Immutable kept card or an empty pool fuses and draws nothing. A library
/// ingredient is a phantom, as `randomIngredient` says.
fn fuse_random_ingredient(
    ctx: &mut EffectContext<'_>,
    args: &FuseCardsArgs,
    scope: &crate::effects::card_scope::CardScope,
) {
    let Some(kept_id) = args.target_instance_id.clone() else {
        return;
    };
    let Some(kept) = find_instance(ctx.sink.state, &kept_id).cloned() else {
        return;
    };
    if !keepable(ctx.sink.state, &kept) {
        return;
    }
    // A card the scope reaches that has left meanwhile is no ingredient.
    let pool: Vec<CardInstance> = crate::effects::card_scope::cards_in_card_scope(ctx, scope, None)
        .into_iter()
        .map(|entry| entry.card)
        .filter(|card| card.id != kept.id)
        .filter(|card| find_instance(ctx.sink.state, &card.id).is_some())
        .filter(|card| {
            find_instance(ctx.sink.state, &card.id)
                .is_none_or(|live| !unit_has(ctx.sink.state, live, KeywordKind::Immutable))
        })
        .collect();
    if pool.is_empty() {
        return;
    }
    let Some(picked) = ctx.sink.rng.pick(&pool).cloned() else {
        return;
    };
    // A library ingredient's phantom carries its radiant flag, tuning and enchantments; a field
    // ingredient goes in as the card standing there.
    let ingredient = match picked.zone {
        Zone::Library { player } => {
            let mut phantom = new_instance(
                &mut *ctx.sink.state,
                &picked.def_id,
                player,
                Zone::Gone { player },
            );
            phantom.radiant = picked.radiant;
            phantom.tuning = picked.tuning.clone();
            phantom.enchantments = picked.enchantments.clone();
            if let Some(live) = find_instance(ctx.sink.state, &picked.id).cloned() {
                let mut gone = live;
                crate::zones::cease_to_exist(&mut *ctx.sink.state, &mut gone);
            }
            phantom
        }
        _ => {
            let Some(live) = find_instance(ctx.sink.state, &picked.id).cloned() else {
                return;
            };
            live
        }
    };
    fuse(
        ctx,
        FuseArgs {
            ingredients: vec![ingredient],
            target: Some(kept),
            ..FuseArgs::default()
        },
    );
}

/// R23, R470: a card a Fuse may keep — on the field and acting (R77's target), or in a hand or a
/// library (`into`) — and not Immutable.
fn keepable(state: &GameState, card: &CardInstance) -> bool {
    let zone = card.zone.z();
    let refused = if zone == ZoneName::Field {
        is_buried(state, card)
    } else {
        zone != ZoneName::Hand && zone != ZoneName::Library
    };
    if refused {
        return false;
    }
    !unit_has(state, card, KeywordKind::Immutable)
}

/// `FuseInto`'s pile: a hand or a library.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum FuseIntoPile {
    Hand,
    Library,
}

/// Where a random ingredient is fused: one named card, or every card of a hand or a library.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(untagged)]
pub enum FuseInto {
    Target {
        target: TargetSpec,
    },
    Pile {
        pile: FuseIntoPile,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        player: Option<PlayerSpec>,
    },
}

/// The kept cards `into` names, read once as the effect applies, in pile order (a library top down).
fn kept_cards(ctx: &EffectContext<'_>, into: &FuseInto) -> Vec<CardInstance> {
    match into {
        FuseInto::Target { target } => instance_of(ctx, target).into_iter().collect(),
        FuseInto::Pile { pile, player } => {
            let side = &ctx.sink.state.players[player_of(ctx, player.unwrap_or(PlayerSpec::SelfSide))];
            match pile {
                FuseIntoPile::Hand => side.hand.clone(),
                FuseIntoPile::Library => side.library.clone(),
            }
        }
    }
}

/// `fuseRandomInto`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FuseRandomIntoArgs {
    pub into: FuseInto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<CatalogQueryArgs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_cost: Option<bool>,
}

/// E23 (Classic+ #31 Fusion Lab, #73): for each kept card, the one `into.target` names or every card
/// of `into.pile` read once (a library top down), one random card of the pool (§5.1, never the running
/// card or its ingredients, B4.1) is fused into it per R77, each its own fusion and draw (R102). The
/// kept card keeps its zone, its type and by default its cost (R470); a Radiant pick goes in on its
/// Radiant face (R469); no kept card or an empty pool draws nothing (R129). A hand or library card's
/// `fused` event is hidden as the card is (§10.8).
pub fn fuse_random_into(args: FuseRandomIntoArgs) -> Effect {
    Effect::new("fuseRandomInto", move |ctx| {
        let kept = kept_cards(ctx, &args.into);
        if kept.is_empty() {
            return;
        }
        let pool = pool_for(ctx, args.query.as_ref());
        if pool.is_empty() {
            return;
        }
        for card in kept {
            // A card an earlier fusion of this list has taken off its pile is no longer one to fuse into,
            // and one the Fuse would refuse (R23, R470) draws nothing for it (R129).
            let Some(card) = find_instance(ctx.sink.state, &card.id).cloned() else {
                continue;
            };
            if !keepable(ctx.sink.state, &card) {
                continue;
            }
            let Some(picked) = ctx.sink.rng.pick(&pool) else {
                return;
            };
            let picked_id = picked.id.clone();
            let ingredient = new_instance(
                &mut *ctx.sink.state,
                &picked_id,
                card.owner,
                Zone::Gone { player: card.owner },
            );
            let on_field = card.zone.z() == ZoneName::Field;
            let radiant_ingredients = if args.radiant == Some(true) {
                Some(vec![ingredient.id.clone()])
            } else {
                None
            };
            let (target, into) = if on_field {
                (Some(card), None)
            } else {
                (None, Some(card))
            };
            fuse(
                ctx,
                FuseArgs {
                    ingredients: vec![ingredient],
                    target,
                    into,
                    radiant_ingredients,
                    keep_cost: if args.keep_cost == Some(false) {
                        None
                    } else {
                        Some(true)
                    },
                    ..FuseArgs::default()
                },
            );
        }
    })
}

// The weapon kit: Equip from a pick (#1841, R1841002; SPEC §6.1 Equip).

/// `equipPicks`' `pick`: how each card is picked.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum EquipPick {
    /// A Discover of `options` cards of the pool ("choose between 2" is 2).
    #[default]
    Discover,
    /// One random card of the pool, with no prompt.
    Random,
}

/// `equipPicks`' argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct EquipPicksArgs {
    /// The pool (§5.1 through `catalog.query`, never the running card, B4.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<CatalogQueryArgs>,
    /// Default `discover`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pick: Option<EquipPick>,
    /// A Discover's offer, `DISCOVER_OPTIONS` by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<i32>,
    /// How many picks, each its own fusion. Default 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub times: Option<i32>,
    /// Every pick goes in on its Radiant face (R469).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    /// A Discover's prompt text, `EQUIP_PICK_PROMPT` by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// R1841002: the step an Equip pick's Discover resumes at, the engine's own whatever card asked
/// (`work::script_step_for`).
pub const EQUIP_PICK_STEP: &str = "@equipPick";
/// Where the Discover's resume data carries the `EquipPicksArgs` the answer goes on with.
const EQUIP_PICKS_KEY: &str = "equipPicks";
/// The Discover's prompt when the card names none.
const EQUIP_PICK_PROMPT: &str = "Discover a card to Equip";

/// R1841002 (SPEC §6.1 Equip): Equip straight into this Unit from a pick, a Discover or a random card
/// of the pool, `times` times, never through a hand. Each pick is a fresh copy fused in as an Equip play
/// fuses its card (R77, R1086), at R77's cost; it is no play, so its Cry never runs and nothing counting
/// plays sees it. Each pick is its own fusion (R102); nothing is offered, drawn or fused when the unit
/// can take no pick (`equip_pool`) or the pool is empty (R129).
pub fn equip_picks(args: EquipPicksArgs) -> Effect {
    Effect::new("equipPicks", move |ctx| {
        let times = args.times.unwrap_or(1);
        match args.pick.unwrap_or_default() {
            EquipPick::Random => {
                for _ in 0..times {
                    let Some(query) = equip_pool(ctx, &args) else {
                        return;
                    };
                    let random = fuse_random_into(FuseRandomIntoArgs {
                        into: FuseInto::Target {
                            target: TargetSpec::SelfCard,
                        },
                        query: Some(query),
                        radiant: args.radiant,
                        // R77: the fused cost, as an Equip play's is.
                        keep_cost: Some(false),
                    });
                    (random.apply)(ctx);
                }
            }
            EquipPick::Discover => {
                if times < 1 {
                    return;
                }
                let Some(query) = equip_pool(ctx, &args) else {
                    return;
                };
                let ask = discover_from_catalog(DiscoverFromCatalogArgs {
                    step: EQUIP_PICK_STEP.to_string(),
                    query: Some(query),
                    count: args.options,
                    prompt: Some(
                        args.prompt
                            .clone()
                            .unwrap_or_else(|| EQUIP_PICK_PROMPT.to_string()),
                    ),
                    data: Some(IndexMap::from([(
                        EQUIP_PICKS_KEY.to_string(),
                        serde_json::to_value(&args).unwrap_or_default(),
                    )])),
                    ..DiscoverFromCatalogArgs::default()
                });
                (ask.apply)(ctx);
            }
        }
    })
}

/// R1841002: the pool of the next pick, or `None` when the unit can take none — it is off the field
/// or not on the stay the run began on (R174), buried, or Immutable (R23). B4.1, R387: the pool leaves
/// out the unit as it stands now, its own card and every card an earlier pick fused into it, so the
/// picks are different cards.
fn equip_pool(ctx: &EffectContext<'_>, args: &EquipPicksArgs) -> Option<CatalogQueryArgs> {
    let host = self_on_its_stay(ctx)?;
    if host.zone.z() != ZoneName::Field || !keepable(ctx.sink.state, &host) {
        return None;
    }
    Some(excluding_def_id(
        Some(&*ctx.sink.state),
        &args.query.clone().unwrap_or_default(),
        Some(&host.def_id),
    ))
}

/// R1841002: the answer to an Equip pick's Discover. The picked card is fused into the unit that
/// asked, on its Radiant face when the pick was Radiant, and with picks left the next Discover opens.
pub fn equip_pick_step() -> Hook {
    hook(|ctx| {
        let Some(pick) = chosen_options(ctx).into_iter().next() else {
            return Vec::new();
        };
        let Some(host) = self_on_its_stay(ctx) else {
            return Vec::new();
        };
        let Some(args) = ctx
            .data
            .get(EQUIP_PICKS_KEY)
            .and_then(|raw| serde_json::from_value::<EquipPicksArgs>(raw.clone()).ok())
        else {
            return Vec::new();
        };
        let mut out = vec![fuse_cards(FuseCardsArgs {
            def_ids: Some(vec![pick]),
            target_instance_id: Some(host.id),
            radiant_ingredients: args.radiant.filter(|radiant| *radiant),
            ..FuseCardsArgs::default()
        })];
        let left = args.times.unwrap_or(1) - 1;
        if left > 0 {
            out.push(equip_picks(EquipPicksArgs {
                times: Some(left),
                ..args
            }));
        }
        out
    })
}

/// `fuseGenerated`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FuseGeneratedArgs {
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<CatalogQueryArgs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_hand: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_price: Option<HandPrice>,
    /// MD-D14: one pick from each named pool, in order (Meditative #59 fuses a random Book and a
    /// random AI card). Set: `count` and `query` are ignored; an empty pool fuses nothing (R142).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pools: Option<Vec<CatalogQueryArgs>>,
    /// R1183001 (Meditative+ #2 NA Tech): every pick leaves out the definitions picked before it, so
    /// the ingredients are different cards. A pool that only an earlier pick filled is then empty,
    /// and nothing is fused (R142).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distinct: Option<bool>,
    /// R1183001: the fused hand card costs this (a `costOverride`, NA Tech's "It costs (2)"), in
    /// place of `handPrice`. A card a full hand burns keeps no price, as R77's free one does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
}

/// E23 (Classic+ #43 AI Slop): `count` independent picks from the pool (they may repeat, R60; never
/// the running card or its ingredients, B4.1), fused per R77 with no target: the shared type, else the
/// first pick's (R102), a Token when every pick is one; the result a fresh hand card at `handPrice`
/// (default `"free"`, R352) that a full hand burns (R4). A Radiant pick goes in on its Radiant face
/// (R469). Fewer than two picks or an empty pool draws nothing (R77, R129).
pub fn fuse_generated(args: FuseGeneratedArgs) -> Effect {
    Effect::new("fuseGenerated", move |ctx| {
        // MD-D14: with named pools, one `pool_for` pick from each pool, in order — and if any pool
        // is empty, fuse nothing (R142).
        let picked_ids: Vec<String> = if let Some(pools) = &args.pools {
            let mut picked_ids: Vec<String> = Vec::with_capacity(pools.len());
            for pool in pools {
                let mut pool = pool_for(ctx, Some(pool));
                // R1183001: a distinct pick passes over the definitions already picked.
                if args.distinct == Some(true) {
                    pool.retain(|def| !picked_ids.contains(&def.id));
                }
                let Some(picked) = ctx.sink.rng.pick(&pool) else {
                    return;
                };
                picked_ids.push(picked.id.clone());
            }
            picked_ids
        } else {
            let count = args.count;
            if count < FUSE_MIN_INGREDIENTS as i32 {
                return;
            }
            let pool = pool_for(ctx, args.query.as_ref());
            if pool.is_empty() {
                return;
            }
            let mut picked_ids: Vec<String> = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let Some(picked) = ctx.sink.rng.pick(&pool) else {
                    return;
                };
                picked_ids.push(picked.id.clone());
            }
            picked_ids
        };
        let player = player_of(ctx, args.to_hand.unwrap_or(PlayerSpec::SelfSide));
        let mut ingredients: Vec<CardInstance> = Vec::new();
        for picked_id in &picked_ids {
            ingredients.push(new_instance(
                &mut *ctx.sink.state,
                picked_id,
                player,
                Zone::Gone { player },
            ));
        }
        let radiant_ingredients = if args.radiant == Some(true) {
            Some(ingredients.iter().map(|card| card.id.clone()).collect())
        } else {
            None
        };
        // R1183001: a set price replaces `handPrice`, laid on the card once it stands in the hand.
        let hand_price = if args.cost_override.is_some() {
            HandPrice::Fused
        } else {
            args.hand_price.unwrap_or(HandPrice::Free)
        };
        let made = fuse(
            ctx,
            FuseArgs {
                ingredients,
                to_hand: Some(player),
                hand_price: Some(hand_price),
                radiant_ingredients,
                ..FuseArgs::default()
            },
        );
        if let (Some(cost), Some(made)) = (args.cost_override, made)
            && let Some(card) = crate::state::find_instance_mut(ctx.sink.state, &made.id)
            && card.zone.z() == ZoneName::Hand
        {
            card.cost_override = Some(cost);
        }
    })
}

/// The piles `fuseOntoYourCard` may offer the controller's cards from.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum FuseOntoPile {
    Field,
    Hand,
    Library,
}

/// R77, R35, R61: "of its type" — a Field Trap counts as a Trap, and a Trap as a Field Trap.
fn same_fuse_type(a: CardType, b: CardType) -> bool {
    let trap = |kind: CardType| kind == CardType::Trap || kind == CardType::FieldTrap;
    a == b || (trap(a) && trap(b))
}

/// Plain code-unit order: the same in every runtime, unlike `localeCompare` (JS's `<` on strings
/// compares UTF-16 code units, which a card name outside ASCII can tell from UTF-8 byte order).
fn compare_text(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// The controller's cards of the ingredient's type, in the piles named: the field in lane order
/// (`zones::acting_on`, the top of a pile only, R13, R446, R1824000), the hand in hand order, and the
/// library in an order its cards alone decide, never its own, which is hidden (§9.1, R310). Never an
/// Immutable card (R23), nor the ingredient itself.
fn fuse_candidates(
    state: &GameState,
    player: PlayerId,
    ingredient: &CardInstance,
    piles: &[FuseOntoPile],
) -> Vec<CardInstance> {
    let kind = card_type_of(state, ingredient);
    let fits = |card: &CardInstance| -> bool {
        card.id != ingredient.id
            && same_fuse_type(card_type_of(state, card), kind)
            && !unit_has(state, card, KeywordKind::Immutable)
            // R1839000: nothing fuses onto a Dormant card.
            && !crate::slumber::is_slumbering(card)
    };
    let mut out: Vec<CardInstance> = Vec::new();
    if piles.contains(&FuseOntoPile::Field) {
        out.extend(
            acting_on(state, player, StandingAt::ItsLane)
                .into_iter()
                .filter(|card| fits(card))
                .cloned(),
        );
    }
    let side = &state.players[player];
    if piles.contains(&FuseOntoPile::Hand) {
        out.extend(side.hand.iter().filter(|&card| fits(card)).cloned());
    }
    if piles.contains(&FuseOntoPile::Library) {
        let mut library: Vec<CardInstance> =
            side.library.iter().filter(|&card| fits(card)).cloned().collect();
        // A stable sort (SURFACE §4.4.1).
        library.sort_by(|a, b| {
            let left = def_of(Some(state), &a.def_id);
            let right = def_of(Some(state), &b.def_id);
            compare_text(&left.name, &right.name)
                .then_with(|| compare_text(&a.def_id, &b.def_id))
                .then_with(|| i32::from(a.radiant).cmp(&i32::from(b.radiant)))
                .then_with(|| compare_text(&a.id, &b.id))
        });
        out.extend(library);
    }
    out
}

/// The prompt `fuseOntoYourCard` opens, answered by `answerFuseOnto` below (R122).
pub const FUSE_ONTO_HOOK: &str = "fuse:onto";

/// What the prompt carries to its answer: the ingredient to fuse onto the pick.
struct FuseOntoData {
    ingredient: String,
}

fn fuse_onto_data(data: &IndexMap<String, Value>) -> Option<FuseOntoData> {
    match data.get("ingredient") {
        Some(Value::String(ingredient)) => Some(FuseOntoData {
            ingredient: ingredient.clone(),
        }),
        _ => None,
    }
}

/// `fuseOntoYourCard`'s `otherwise`: exile the ingredient, or leave it alone.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum FuseOntoOtherwise {
    Exile,
    Nothing,
}

/// `fuseOntoYourCard`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FuseOntoYourCardArgs {
    pub target: TargetSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Vec<FuseOntoPile>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub otherwise: Option<FuseOntoOtherwise>,
}

/// E23, Classic #78 Mutate Spell's Radiant face: the card `target` names (on the field, on the stay the
/// run aimed at, R174) is fused onto a card of the controller's of its type (`fuse_candidates`), picked
/// in a `target` prompt the other player sees only as open (§10.6, R81), deck cards shown to the
/// chooser alone (§10.8). The pick stays where it is (R470) and the ingredient ceases to exist (R86).
/// With no candidate the ingredient is exiled (`otherwise`'s default) or left alone, with no prompt. A
/// prompt parks the rest of the list until the answer (R113).
pub fn fuse_onto_your_card(args: FuseOntoYourCardArgs) -> Effect {
    Effect::new("fuseOntoYourCard", move |ctx| {
        let Some(ingredient) = instance_of(ctx, &args.target) else {
            return;
        };
        if ingredient.zone.z() != ZoneName::Field || is_buried(ctx.sink.state, &ingredient) {
            return;
        }
        let piles: Vec<FuseOntoPile> = args
            .from
            .clone()
            .unwrap_or_else(|| vec![FuseOntoPile::Field, FuseOntoPile::Hand, FuseOntoPile::Library]);
        let candidates = fuse_candidates(ctx.sink.state, ctx.controller, &ingredient, &piles);
        if candidates.is_empty() {
            if args.otherwise != Some(FuseOntoOtherwise::Nothing) {
                let banish = exile(ExileArgs {
                    target: TargetSpec::Instance {
                        instance_id: ingredient.id.clone(),
                    },
                });
                (banish.apply)(ctx);
            }
            return;
        }
        // R1200: while a Mayor acts the fuse-onto pick is drawn at random, and nobody is asked.
        if crate::random_targets::targets_random(ctx.sink.state) {
            let at = ctx.sink.rng.int(candidates.len() as i32);
            let picked = usize::try_from(at)
                .ok()
                .and_then(|at| candidates.get(at))
                .map(|card| Selection::Instance {
                    instance_id: card.id.clone(),
                });
            let ingredient_id = ingredient.id.clone();
            fuse_onto_picked(ctx, &ingredient_id, picked);
            return;
        }
        let options: Vec<PromptOption> = candidates
            .iter()
            .map(|card| PromptOption {
                key: format!("instance:{}", card.id),
                label: def_of(Some(&*ctx.sink.state), &card.def_id).name.clone(),
                selection: Selection::Instance {
                    instance_id: card.id.clone(),
                },
                cost: None,
                radiant: None,
            })
            .collect();
        // The card's own data is the ingredient alone (no control key for `work.cardData` to take out).
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert("ingredient".to_string(), Value::String(ingredient.id.clone()));
        let resume = Resume {
            def_id: ctx
                .self_
                .as_ref()
                .map(|card| card.def_id.clone())
                .or_else(|| ctx.def_id.clone())
                .unwrap_or_default(),
            hook: FUSE_ONTO_HOOK.to_string(),
            step: "onto".to_string(),
            radiant: ctx.radiant,
            instance_id: ctx.self_.as_ref().map(|card| card.id.clone()),
            data,
        };
        let player = ctx.controller;
        open_prompt(
            ctx,
            OpenPromptArgs {
                player,
                kind: PromptKind::Target,
                aim: None,
                prompt: "Choose a card of yours to fuse it onto".to_string(),
                options,
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume,
            },
        );
    })
}

/// The fusion onto the pick — the answered pick's, or the random one drawn under a Mayor
/// (R1200) — shared by the prompt's answer and the effect's random branch.
fn fuse_onto_picked(sink: &mut EngineSink<'_>, ingredient_id: &str, picked: Option<Selection>) {
    let ingredient = find_instance(sink.state, ingredient_id).cloned();
    let onto = match &picked {
        Some(Selection::Instance { instance_id }) => find_instance(sink.state, instance_id).cloned(),
        _ => None,
    };
    if let (Some(ingredient), Some(onto)) = (ingredient, onto)
        && ingredient.zone.z() == ZoneName::Field
    {
        let on_field = onto.zone.z() == ZoneName::Field;
        let (target, into) = if on_field {
            (Some(onto), None)
        } else {
            (None, Some(onto))
        };
        fuse(
            sink,
            FuseArgs {
                ingredients: vec![ingredient],
                target,
                into,
                ..FuseArgs::default()
            },
        );
    }
}

/// R122: the answer to `fuseOntoYourCard`'s prompt — validated as any prompt's, then the fusion onto
/// the pick, then what the prompt interrupted (R113). An ingredient no longer on the field, or a pick
/// that has moved to a pile it may not be kept in, fuses nothing.
pub fn answer_fuse_onto(sink: &mut EngineSink<'_>, answer: &AnswerInput) -> Result<(), EngineError> {
    let Some(pending) = sink.state.pending.clone() else {
        return Err(EngineError::new("no prompt is open"));
    };
    why_answer_refused(&pending, answer)?;
    let Some(data) = fuse_onto_data(&pending.resume.data) else {
        return Err(EngineError::new("that prompt carries no card to fuse"));
    };
    let picked: Option<Selection> = in_offered_order(&pending, &answer.selection).into_iter().next();

    close_prompt(sink);
    begin_work_cascade(sink);
    fuse_onto_picked(sink, &data.ingredient, picked);
    drain_work(sink);
    Ok(())
}
