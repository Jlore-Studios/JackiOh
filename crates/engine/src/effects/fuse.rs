//! Fuse as a verb (SPEC §6.3 Fuse, R77, R102; §8.4 #85, §8.5 #99).
//!
//! `subsystems/fuse.ts` is R77 in full — the transient definition with both faces fused, the summed
//! stats, the united keywords and tags, the concatenated scripts, `min(sum, FUSE_COST_CAP)`, the
//! target instance kept with its zone, damage, exertion, counters and memory, the other ingredients
//! ceasing to exist with no death, and Craft a Card's fresh `costOverride` 0 hand card. NONE of that
//! is repeated here and none of it may be: it is eighteen kilobytes of rules with its own test file,
//! and a second implementation is a second set of rules. This file exists only because `fuse` takes
//! an `EngineSink` and mutates state, which a card script may not do (CLAUDE.md rule 5), and because
//! `FuseArgs.ingredients` is `readonly CardInstance[]` while a Discover hands over catalog ids.
//!
//! AN INGREDIENT THAT WAS NEVER A CARD (§8.5 #99). Craft a Card's ingredients are Discovered
//! DEFINITIONS: nobody ever saw them on a board, and R77 has them cease to exist the moment the
//! fusion is made. An ingredient only ever contributes its definition to the fusion, so the
//! cheapest faithful thing is an instance that sits in no pile at all: `{ z: "gone" }`, which
//! `@jackioh/shared`'s `Zone` really does offer and which `subsystems/fuse.ts` itself uses for
//! "ceased to exist" (R11, R86). No zone event fires for it, `findInstance` cannot reach it, and
//! nothing in `viewFor` lists it, so the ingredient is invisible exactly as #99 requires.
//!
//! The `fused` event, `state.transientDefs` and the script registration are all the subsystem's and
//! are deliberately untouched here.
//!
//! Port of `packages/engine/src/effects/fuse.ts`. The prompt answerer TS registered at module scope
//! (`registerPromptAnswerer(FUSE_ONTO_HOOK, …)`) is the plain `answer_fuse_onto`, which `prompts.rs`
//! calls by its hook (SURFACE §6.6).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{CatalogQueryArgs, def_of, excluding_def_id, query};
use crate::config::FUSE_MIN_INGREDIENTS;
use crate::effects::move_::{ExileArgs, exile};
use crate::effects::targets::{PlayerSpec, TargetSpec, instance_of, player_of};
use crate::faces::card_type_of;
use crate::layers::unit_has;
use crate::prompts::{
    AnswerInput, OpenPromptArgs, close_prompt, in_offered_order, open_prompt, why_answer_refused,
};
use crate::script::{Effect, EffectContext, EngineSink};
use crate::state::{CardInstance, EngineError, GameState, PromptOption, Resume, find_instance, new_instance};
use crate::subsystems::fuse::{FuseArgs, HandPrice, fuse};
use crate::wire::{CardDef, CardType, KeywordKind, PlayerId, PromptKind, Row, Selection, Zone, ZoneName};
use crate::work::{begin_work_cascade, drain_work};
use crate::zones::{card_at, is_buried, slots_of};

/// R77 and R86: an ingredient that is only a definition. It is created in `{ z: "gone" }` — in no
/// pile, so no zone event fires, nothing can target it and nothing can bring it back — which is the
/// same zone the subsystem moves a consumed ingredient to.
fn phantom_ingredient(ctx: &mut EffectContext<'_>, def_id: &str, owner: Option<PlayerSpec>) -> CardInstance {
    let player = match owner {
        None => ctx.controller,
        Some(spec) => player_of(ctx, spec),
    };
    new_instance(&mut *ctx.sink.state, def_id, player, Zone::Gone { player })
}

/// `fuseCards`' `pick`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum FuseCardsPick {
    Random,
    All,
}

/// `fuseCards`' argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
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
    /// (§9.3, §10.7). Default "all", which is also the only behaviour a single target can have.
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
    // ---- v0.2.0, generation (E23, R469, R470) ----
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

/// §6.3 Fuse per R77, wrapped. The ingredients are named in two ways, in this order: cards that
/// already exist (`instanceIds`, #85 Unlicensed Experimentation's played permanent) and then
/// definitions that never were cards (`defIds`, #99 Craft a Card's Discover picks). No Core card
/// passes both, so the order between the two groups is a convention rather than a rule; within a
/// group the caller's order is kept, because it is the order the fused name, text and scripts are
/// concatenated in.
///
/// ONE FUSION AT A TIME, AND WHY THE LOOP IS IN HERE (R77, §8.4 #85 radiant). Radiant #85 fuses its
/// played permanent "onto each matching permanent separately", each fusion its own transient
/// definition. That CANNOT be a card emitting one `fuseCards` per target: the subsystem's
/// `ceaseToExist` calls `removeFromAnyZone`, so after the first fusion the played card is in
/// `{ z: "gone" }` and held by no pile — `findInstance` cannot reach it and its id cannot recover
/// it, so every later call would see one ingredient, fall below `FUSE_MIN_INGREDIENTS` and answer
/// `null`. The subsystem anticipates exactly this ("an ingredient that already ceased to exist in an
/// earlier fusion still fuses", because an ingredient only ever contributes its DEFINITION), and the
/// way to honour it is to resolve the ingredients ONCE and reuse those same objects across the
/// fusions — which only something holding them can do. Hence `targetInstanceIds` and the loop here.
///
/// R86 is about POOLS, not about this: it drops ids whose cards are gone when the card said "a
/// random card you played this turn", so a pool degrades instead of fizzling at random. An
/// `instanceIds` entry is an instruction the caller named outright, so it is resolved once and then
/// kept — dropping it mid-loop would turn a deliberate second fusion into a silent no-op.
///
/// It fizzles silently and the card still resolves (§6.3): an `instanceIds` entry that never
/// resolved at all drops out, an empty target list after filtering fuses nothing, and the subsystem
/// itself answers `null` — changing nothing — for fewer than `FUSE_MIN_INGREDIENTS`, for a target
/// that is off the field or Immutable (R23), and for a call that names neither a target nor a hand.
pub fn fuse_cards(args: FuseCardsArgs) -> Effect {
    Effect::new("fuseCards", move |ctx| {
        // ME-FUSE-RANDOM first: the ingredient is drawn inside `apply` from the match rng, and the
        // kept card is the target named beside it. Nothing else of this call runs then.
        if let Some(scope) = &args.random_ingredient {
            fuse_random_ingredient(ctx, &args, scope);
            return;
        }
        // Resolved ONCE, before any fusion: these objects are reused for every fusion below, so an
        // ingredient the first fusion consumed still contributes its definition to the second.
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
            // `EffectContext` derefs to `EngineSink`, so the subsystem takes the context as it stands.
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

// ---------------------------------------------------------------------------------------------
// Patch v0.2.0: the Fuse variants (docs/classic-sets.md B5 E23; R77, R102, R468–R470).
// ---------------------------------------------------------------------------------------------

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
/// kept target named beside it. The kept card is `targetInstanceId`'s live instance; an Immutable
/// kept card eats nothing and draws nothing (R23). The pool is the scope's cards minus the kept
/// card and minus every Immutable card (being eaten changes its text, R23); with none, nothing is
/// fused and nothing is drawn. A library ingredient goes in as a fresh phantom of its definition —
/// radiant flag, tuning and enchantments carried — and the library card ceases to exist, so its id
/// never reaches a view.
fn fuse_random_ingredient(
    ctx: &mut EffectContext<'_>,
    args: &FuseCardsArgs,
    scope: &crate::effects::card_scope::CardScope,
) {
    // The kept card is the target's live instance; without one there is no fusion.
    let Some(kept_id) = args.target_instance_id.clone() else {
        return;
    };
    let Some(kept) = find_instance(ctx.sink.state, &kept_id).cloned() else {
        return;
    };
    // An Immutable kept card eats nothing and draws nothing (R23).
    if !keepable(ctx.sink.state, &kept) {
        return;
    }
    // The scope's cards minus the kept card and minus every Immutable one; a card the scope
    // reaches that has left meanwhile is no ingredient.
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
    // A library ingredient goes in as a fresh phantom of its definition — radiant flag, tuning
    // and enchantments carried — and the library card ceases to exist, so its id never reaches a
    // view. A field ingredient goes in as the card standing there.
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
#[serde(rename_all = "camelCase")]
pub enum FuseIntoPile {
    Hand,
    Library,
}

/// Where a random ingredient is fused: one named card, or every card of a hand or a library.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

/// E23: "fuse a random card into a card in your hand; its cost doesn't change" (Classic+ #31 Fusion
/// Lab, whose Radiant face fuses "a random Radiant card") and "fuse all cards in your deck with a
/// random card, they maintain their original cost" (Classic+ #73). For each kept card — the one
/// `into.target` names, or every card of `into.pile`, read once, a library top down — one random card
/// of the pool (§5.1 through `catalog.query`, never the running card or its ingredients, B4.1) is fused
/// into it per R77: the kept card is the instance, keeps its zone and its type, and keeps the cost it
/// had (R470, `keepCost`, default on); a Radiant pick goes in on its Radiant face (R469). Each fusion is
/// its own transient definition (R102) and draws its own pick; with no kept card or an empty pool the
/// effect does nothing and draws nothing (R129). A kept card on the field is R77's `target`; a hand or
/// library card stays where it is (R470) and its `fused` event is hidden as the card is (§10.8).
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

/// `fuseGenerated`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
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
}

/// E23: "fuse 3 random AI generated cards and add the result to your hand; it costs (0)" (Classic+
/// #43 AI Slop; Radiant: "3 random Radiant AI generated cards"). `count` independent picks from the
/// pool (generated cards may repeat, R60; never the running card or its ingredients, B4.1), fused per
/// R77 with no target: the shared type, else the first pick's (R102); a Token when every pick is one;
/// the result a fresh card in the hand, at `handPrice` (default `"free"`, a `costOverride` of 0, R352)
/// and burned by a full hand (R4). A Radiant pick goes in on its Radiant face (R469). Fewer than two
/// picks is no fusion, so it draws nothing (R77, R129), and neither does an empty pool.
pub fn fuse_generated(args: FuseGeneratedArgs) -> Effect {
    Effect::new("fuseGenerated", move |ctx| {
        // MD-D14: with named pools, one `pool_for` pick from each pool, in order — and if any pool
        // is empty, fuse nothing (R142).
        let picked_ids: Vec<String> = if let Some(pools) = &args.pools {
            let mut picked_ids: Vec<String> = Vec::with_capacity(pools.len());
            for pool in pools {
                let pool = pool_for(ctx, Some(pool));
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
        fuse(
            ctx,
            FuseArgs {
                ingredients,
                to_hand: Some(player),
                hand_price: Some(args.hand_price.unwrap_or(HandPrice::Free)),
                radiant_ingredients,
                ..FuseArgs::default()
            },
        );
    })
}

/// The piles `fuseOntoYourCard` may offer the controller's cards from.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
/// (units, then backrow; the top of a pile only, R13), the hand in hand order, and the library in an
/// order its cards alone decide — name, definition, face, id — never its own, which is hidden from
/// everyone (§9.1, R310). An Immutable card is never a Fuse target (R23), and the ingredient itself is
/// not one of its own candidates.
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
    };
    let mut out: Vec<CardInstance> = Vec::new();
    if piles.contains(&FuseOntoPile::Field) {
        for row in [Row::Units, Row::Backrow] {
            for slot in slots_of(player, row) {
                if let Some(card) = card_at(state, slot) {
                    let card: &CardInstance = card;
                    if fits(card) {
                        out.push(card.clone());
                    }
                }
            }
        }
    }
    let side = &state.players[player];
    if piles.contains(&FuseOntoPile::Hand) {
        out.extend(side.hand.iter().filter(|&card| fits(card)).cloned());
    }
    if piles.contains(&FuseOntoPile::Library) {
        let mut library: Vec<CardInstance> =
            side.library.iter().filter(|&card| fits(card)).cloned().collect();
        // Stable, as `Array.prototype.sort` is (SURFACE §4.4.1).
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
#[serde(rename_all = "camelCase")]
pub enum FuseOntoOtherwise {
    Exile,
    Nothing,
}

/// `fuseOntoYourCard`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FuseOntoYourCardArgs {
    pub target: TargetSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Vec<FuseOntoPile>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub otherwise: Option<FuseOntoOtherwise>,
}

/// E23, Classic #78 Mutate Spell's Radiant face: "fuse it onto a card of yours of its type on your
/// field, in your hand or in your deck, or exile it if you have none". The card `target` names — on
/// the field, on the stay the run aimed at (R174) — is the ingredient; the controller picks the card
/// it is fused onto from their own cards of its type (`fuseCandidates`) in a `target` prompt of their
/// own, which the other player sees only as open (§10.6, R81): the deck's cards are shown to the
/// chooser and nobody else (§10.8, as KY's Private Tutor shows library cards). The answer fuses per
/// R77 onto the pick — kept on the field as R77's target, or in the hand or the deck where it is
/// (R470) — and the ingredient ceases to exist (R77, R86). With no candidate the ingredient is exiled
/// (`otherwise: "exile"`, the default) or left alone (`"nothing"`), with no prompt. A prompt splits
/// the list the effect stands in: the rest is parked and resumes after the answer (R113).
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
        // TS `resumeAt({ … })`: the hook and the face as given, and the card's own data, which here is
        // the ingredient alone (no control key for `work.cardData` to take out).
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
/// that has moved to a pile it may not be kept in, fuses nothing. (TS registered it at module scope
/// under `FUSE_ONTO_HOOK`; `prompts.rs` calls it by that hook.)
pub fn answer_fuse_onto(sink: &mut EngineSink<'_>, answer: &AnswerInput) -> Result<(), EngineError> {
    let Some(pending) = sink.state.pending.clone() else {
        return Err(EngineError::new("no prompt is open"));
    };
    why_answer_refused(&pending, answer)?;
    // TS `resumeOf(pending).data`: a typed `Resume` has every field already, so its data is the data.
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
