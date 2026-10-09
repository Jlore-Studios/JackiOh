//! Fuse and Craft a Card (SPEC §6.3 Fuse, R77): the transient definition two or three cards make,
//! and the instance the result lives on. Used by #85 Unlicensed Experimentation and #99 Craft a Card.
//!
//! Three things make Fuse unlike every other verb.
//!
//! First, the result is a *definition*, not an instance: two cards' base forms make the fused base
//! form and their radiant forms make the fused radiant one, so the fused card still has both faces
//! and Make Radiant keeps working on it (§5.2). Definitions are shared by every copy of a card and
//! are never edited, so the fusion writes a fresh one into `state.transient_defs`, where `def_of`
//! finds it ahead of the catalog. It is match state and survives a JSON round-trip (§10.1).
//!
//! Second, the scripts are code, which no JSON state can hold. In TS the concatenated pair joined the
//! process's script registry under the new def id, and `syncFusedScripts` re-registered any of a
//! state's fused scripts the registry lacked wherever the engine was entered. Rust registers nothing
//! (SURFACE §3, §6.6): the fused scripts are built from the state — `scripts::script_of(state, def_id)`
//! finds the fused definition in `state.transient_defs` and answers the scripts `reduce` composed for it
//! into `state.fused_scripts` (`scripts::sync_fused_scripts`), or calls `compose_fused_scripts`, which
//! composes them from the ingredients' scripts by TS's `combineObjects` rules, then and there. R179's
//! id names the ingredients (`t-<n>:<a>+<b>`), and the fused scripts are a function of the
//! ingredients' ids and nothing else (`fused_script`), so one id means one pair of scripts in every
//! state that can mint it, and a replay of the same action list runs the same scripts (§9.3).
//!
//! Third, R77 keeps an ingredient's *instance* when one of them is a target already on the field:
//! the fused card is that card, with its zone, damage, exertion, counters and memory intact, and the
//! other ingredients cease to exist without dying. So the only instance-level work here is the def
//! id, the summed buffs and the united granted keywords; everything else is deliberately untouched.
//!
//! Patch v0.2.0 (docs/classic-sets.md B5 E23) adds three things, each a ruling of its own. R468: an
//! id that would spell out a list longer than `FUSED_ID_CAP` is a digest of that list instead, and
//! the definition keeps the list (`CardDef.ingredients`), which is what rebuilds a digest's scripts.
//! R469: an ingredient may go in on its Radiant face ("fuse a random Radiant card"), lending that
//! face and its text to both fused forms; the id marks it `*`. R470: the kept instance may be a card
//! in a hand or a library (`into`), which stays where it is, and "its cost doesn't change" keeps the
//! cost it had (`keep_cost`).
//!
//! Port of `packages/engine/src/subsystems/fuse.ts`, minus `syncFusedScripts`, `ensureFused` and the
//! writes to the process-wide registry and digest table (SURFACE §6.6). `FUSE_MIN_INGREDIENTS` (R77:
//! Craft a Card fuses "two or three cards", and #85 fuses two; fewer is not a fusion) and
//! `CRAFTED_CARD_COST` (§8 #99: "the result costs 0 and goes to your hand", as a `costOverride` per
//! R65) live in `crate::config` (CLAUDE.md rule 9).
//!
//! TS combined scripts generically, key by key over untyped objects (`combineValues`,
//! `combineObjects`, `isPlainObject`). A Rust `Script` is a struct, so `combine_objects` combines it
//! field by field, each field by the rule `combineValues` applied to that kind of value: a hook by
//! `combined_hook`, an eager read by `eager_read`/`eager_condition`, a list by concatenation, a nested
//! table (`resume`, `targetChecks`, `staticFlags`, `quests`) key by key by the same rules.

use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::{CRAFTED_CARD_COST, FUSE_COST_CAP, FUSE_MIN_INGREDIENTS, FUSED_ID_CAP};
use crate::script::{
    ActivationDecl, AuraEntry, AuraHook, CardScripts, ConditionContext, ConditionHook, Effect, EffectApply,
    EffectContext, EffectPart, EngineSink, FlagOrCount, Hook, HookArgs, PlagueMultiplierHook, QuestBook,
    Script, SetStat, SetStatHook, StatMod, StaticFlags, TargetCheck, TributeWhenHook, TriggerDef, TriggerRun,
    WouldCounterHook, aura_hook, condition_hook, hook, read_hook, target_check, would_counter_hook,
};
use crate::state::{CardInstance, GameState, Grant, find_instance, find_instance_mut, new_instance};
use crate::wire::{
    AttackHealth, CardCost, CardDef, CardFace, CardType, FusedIngredient, GameEvent, Keyword, KeywordKind,
    PlayerId, Rarity, Selection, SetName, Tag, TargetDecl, Zone, ZoneName, keyword_key,
};

/// The `Script` key of the step table a continuation re-enters (`prompts::RESUME_HOOK`).
const RESUME_KEY: &str = "resume";

/// The script keys whose functions return something other than effects, so they combine by building
/// every ingredient's list at once: an aura's entries are read off the field on every stat read
/// (§10.4), and there is no "when the list reaches it" for them. `preview` (R280) is the other: the
/// labelled numbers `view_for` shows, a pure read with nothing to resolve, so a fusion's list is its
/// ingredients' lists in ingredient order, each asked with the fused card's own context (the fused
/// instance as `self`, the face it runs, the zone it is asked about), as R196 asks `conditionMet` —
/// and each label still sits in the fused face's text, which prints every ingredient's text whole.
/// `drawLimit` (B5 E3, R457) is a third: the limits a card sets while it acts, read on every draw, so a
/// fusion sets every ingredient's limit and the lowest holds.
// B5 E6, E35: `heroGuard` and `conditionalKeywords` are pure reads returning lists too, so a fusion
// guards its hero with every ingredient's guard and has every ingredient's conditional keywords.
const EAGER_KEYS: &[&str] = &[
    "aura",
    "preview",
    // B5 E3 (R457): a draw limit is a pure read of the field.
    "drawLimit",
    // B5 E15, E11 (R455, R454): a price rule and a graveyard permission are pure reads of the field too.
    "costAura",
    "graveyardPlay",
    "heroGuard",
    "conditionalKeywords",
];

/// §8's rarity ladder, lowest first, so a fusion can report the rarest ingredient's rarity.
const RARITY_ORDER: &[Rarity] = &[
    Rarity::Token,
    Rarity::Common,
    Rarity::Rare,
    Rarity::Epic,
    Rarity::Legendary,
    Rarity::Mythic,
];

/// TS `FuseArgs`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FuseArgs {
    /// Every card going into the fusion, in the order the fusing card names them.
    pub ingredients: Vec<CardInstance>,
    /// R77: an ingredient already on the field that the result keeps as its instance. #85 fuses the
    /// permanent the opponent just played onto one of yours, and that one is the target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<CardInstance>,
    /// Craft a Card: no target on the field, so the result is a fresh card in this player's hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_hand: Option<PlayerId>,
    /// R352: what the hand card costs. `Free` is Craft a Card's "the result costs 0" (§8 #99), a
    /// `costOverride` of `CRAFTED_CARD_COST`; `Fused` leaves R77's fused cost, min(sum, 4), as
    /// Heroic Power's Stitching does. Absent is `Free`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_price: Option<HandPrice>,
    /// R352: the hand card is Radiant (radiant Stitching). Absent is R77's non-Radiant hand card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    // ---- v0.2.0, generation (E23, R468–R470) ----
    /// R470: a card in a hand or a library that the result keeps as its instance, as R77's `target`
    /// keeps one on the field — Classic+ #31 Fusion Lab's hand card, Classic+ #73's deck cards, the
    /// card Classic #78's Radiant face picks from a hand or a deck. It stays where it is; its type is
    /// the result's (R77). A card named here that is not in a hand or a library, or is Immutable (R23),
    /// refuses the fusion. `target` wins when both are named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub into: Option<CardInstance>,
    /// R469: the ingredients, by instance id, that go in on their Radiant face — "fuse a random Radiant
    /// card into it", "a Radiant copy of it is fused into this". Such an ingredient puts its Radiant face
    /// and text into both of the fused forms; every other ingredient puts in the face the form is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant_ingredients: Option<Vec<String>>,
    /// R470: "its cost doesn't change" — the kept card (`target` or `into`) keeps the cost it had:
    /// a `costOverride` of its own cost as it stood (its override if it had one), or, for an X-cost or
    /// embiggen card with no override, that printed cost form on the fused definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_cost: Option<bool>,
}

/// R352: the two prices a fused hand card can have.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum HandPrice {
    Free,
    Fused,
}

// ---------------------------------------------------------------------------------------------
// The fused definition (R77).
// ---------------------------------------------------------------------------------------------

/// R77's unions: one keyword per distinct keyword — except Armor, which every ingredient keeps. Armor
/// is the keyword whose number stacks from every source (§6.1, §10.4 sums it), so it adds up on the
/// fused face the way the stats beside it do: Armor 7 and Armor 3 print 10, and so do Armor 7 and
/// Armor 7 print 14 rather than collapsing into one because the numbers happen to match (R102).
fn union_keywords(keywords: &[Keyword]) -> Vec<Keyword> {
    let mut seen: IndexSet<String> = IndexSet::new();
    let mut out: Vec<Keyword> = Vec::new();
    for keyword in keywords {
        if keyword.kind() == KeywordKind::Armor {
            out.push(keyword.clone());
            continue;
        }
        let key = keyword_key(keyword);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        out.push(keyword.clone());
    }
    out
}

fn union_tags(defs: &[CardDef]) -> Vec<Tag> {
    let mut seen: IndexSet<Tag> = IndexSet::new();
    for def in defs {
        for tag in &def.tags {
            seen.insert(*tag);
        }
    }
    seen.into_iter().collect()
}

/// R279, R102: the cards a fused text names are every ingredient's — the fused face prints both texts
/// (`fused_face`), so each name in them still links. The union keeps the first appearance of each id,
/// in ingredient order, and is `None` when no ingredient names any card, so the def omits the key just
/// as a catalog card with no reference does.
fn union_refs(defs: &[CardDef]) -> Option<Vec<String>> {
    let mut seen: IndexSet<String> = IndexSet::new();
    for def in defs {
        for reference in def.refs.iter().flatten() {
            seen.insert(reference.clone());
        }
    }
    if seen.is_empty() {
        None
    } else {
        Some(seen.into_iter().collect())
    }
}

/// A stat the fused face has only if some ingredient had it: two fused Spells or Traps keep a face
/// with no attack and no health rather than gaining a printed 0/0 (§5, `CardFace`).
fn sum_defined(values: impl IntoIterator<Item = Option<i32>>) -> Option<i32> {
    let defined: Vec<i32> = values.into_iter().flatten().collect();
    if defined.is_empty() {
        None
    } else {
        Some(defined.iter().sum())
    }
}

/// One ingredient's face as its instance wears it. §7 and R175: a token summoned X/X carries its X as
/// `stats_override`, and the Bread Token its "Armor X" as `armor_override`, because neither number can
/// be printed — they ARE its printed face, §10.4's layer 1. So a Fuse sums that X/X, not the printed
/// 0/0 it stands in for, and the X replaces only that token's own Armor, never an Armor another
/// ingredient prints (#85 fusing a 7/7 onto #18's Bread Token keeps the 7/7's Armor 7).
fn worn_face(def: &CardDef, card: Option<&CardInstance>, radiant: bool) -> CardFace {
    let face = if radiant { &def.radiant } else { &def.base };
    // R349: a token with no Radiant form of its own (the Ghoul Token) doubles its X/X on the Radiant
    // face, so the fused Radiant face sums the doubled X.
    let stats = card.and_then(|card| {
        let worn = CardInstance {
            radiant,
            ..card.clone()
        };
        crate::layers::worn_stats_override(def, &worn)
    });
    let armor = card.and_then(|card| card.armor_override);
    let keywords = match armor {
        None => face.keywords.clone(),
        Some(n) => face
            .keywords
            .iter()
            .map(|keyword| {
                if keyword.kind() == KeywordKind::Armor {
                    Keyword::Armor { n }
                } else {
                    keyword.clone()
                }
            })
            .collect(),
    };
    CardFace {
        attack: stats.map_or(face.attack, |stats| Some(stats.attack)),
        health: stats.map_or(face.health, |stats| Some(stats.health)),
        keywords,
        ..face.clone()
    }
}

/// R77: one face of the fusion — summed stats, united keywords, both texts. R469: an ingredient that
/// went in on its Radiant face (`forced`) puts that face into the base form too.
fn fused_face(ingredients: &[CardInstance], defs: &[CardDef], radiant: bool, forced: &[bool]) -> CardFace {
    let faces: Vec<CardFace> = defs
        .iter()
        .enumerate()
        .map(|(at, def)| worn_face(def, ingredients.get(at), radiant || forced.get(at) == Some(&true)))
        .collect();
    let attack = sum_defined(faces.iter().map(|face| face.attack));
    let health = sum_defined(faces.iter().map(|face| face.health));
    let keywords: Vec<Keyword> = faces
        .iter()
        .flat_map(|face| face.keywords.iter().cloned())
        .collect();
    CardFace {
        type_: None,
        attack,
        health,
        x_stats: None,
        keywords: union_keywords(&keywords),
        text: faces
            .iter()
            .map(|face| face.text.as_str())
            .collect::<Vec<&str>>()
            .join("\n"),
        // ME-GRANT: a fused face grants nothing itself; the fused script's grant map (R102) is what
        // a grant it carries resolves through.
        grants: None,
    }
}

/// R77: "Its type is the target's, or the ingredients' shared type when there is no target on the
/// field (Field Trap if any ingredient is one)". The parenthetical settles a trap fusion, because a
/// Field Trap is not consumed when it fires and a plain Trap is, so the Field Trap half wins; it
/// cannot turn a unit fusion into a trap, and #85 only ever fuses two cards of the same type.
///
/// Ingredients of different types with no target is a shape no Core card makes (#99 Discovers Units);
/// the first ingredient's type is the fallback rather than a fizzle.
fn fused_type(defs: &[CardDef], target_def: Option<&CardDef>) -> CardType {
    let types: Vec<CardType> = defs.iter().map(|def| def.type_).collect();
    let first = types.first().copied().unwrap_or(CardType::Unit);
    let shared = if types.iter().all(|t| *t == first) {
        Some(first)
    } else {
        None
    };
    let base = target_def.map(|def| def.type_).or(shared).unwrap_or(first);
    if (base == CardType::Trap || base == CardType::FieldTrap) && types.contains(&CardType::FieldTrap) {
        return CardType::FieldTrap;
    }
    base
}

/// R77: "min(sum of the printed costs per R65, 4)". `printed_cost` is R65's reading of one card —
/// Ceaseless Void's computed cost, the X chosen on the instance, the embiggen price it was played
/// for — so the sum needs no second cost rule here.
fn fused_cost(state: &GameState, ingredients: &[CardInstance]) -> i32 {
    let sum: i32 = ingredients
        .iter()
        .map(|card| crate::mana::printed_cost(state, card))
        .sum();
    sum.min(FUSE_COST_CAP)
}

fn rarest_of(defs: &[CardDef]) -> Rarity {
    let mut best = Rarity::Common;
    let mut best_rank: i64 = -1;
    for def in defs {
        let rank = RARITY_ORDER
            .iter()
            .position(|rarity| *rarity == def.rarity)
            .map_or(-1, |at| at as i64);
        if rank > best_rank {
            best_rank = rank;
            best = def.rarity;
        }
    }
    best
}

/// The id a transient def gets (R102, R179): `t-<n>`, where n depends only on how many transient defs
/// the state already holds, so the same action list always produces the same id (§9.3) — followed by
/// the ids of the ingredients it was fused from, `t-<n>:<a>+<b>`. An ingredient that is itself a
/// fused card is written in parentheses, `t-2:(t-1:<a>+<b>)+<c>`, so the id reads back one way:
/// without them `t-2:t-1:a+b+c+d` could be `t-1:a+b` fused with c and d, or `t-1:a+b+c` with d.
///
/// The suffix is what keeps the scripts right. A def is match state, but its scripts are code, built
/// from the id's ingredients (see this file's header), and one server process runs every match (§9.2)
/// and folds a match's log to rebuild it (§9.3). The fused scripts are a function of the ingredients'
/// ids and nothing else (`fused_script`), so an id that carries them names the same scripts in every
/// match that can mint it. R469: an ingredient that went in on its Radiant face is written with a
/// trailing `*`, since its scripts are its Radiant ones on both forms.
///
/// R468: the list is spelled out only while it fits `FUSED_ID_CAP`. A card fused onto again and
/// again (Classic+ #74) nests every earlier id inside the next, so the spelled-out id grows with every
/// fusion; past the cap the id is `t-<n>:#<digest>`, the digest a pure hash of the very list the id
/// would have spelled out (`fused_digest`). It still names one list and so one pair of scripts in every
/// match (R179), and the definition keeps the list itself (`CardDef.ingredients`), which is where the
/// scripts are rebuilt from (`compose_fused_scripts`).
fn next_transient_id(state: &GameState, specs: &[FusedIngredient]) -> String {
    let taken = |n: usize| {
        let bare = format!("t-{n}");
        let headed = format!("t-{n}:");
        state
            .transient_defs
            .keys()
            .any(|id| *id == bare || id.starts_with(&headed))
    };
    let mut n = state.transient_defs.len() + 1;
    while taken(n) {
        n += 1;
    }
    let body = specs
        .iter()
        .map(ingredient_name)
        .collect::<Vec<String>>()
        .join("+");
    if body.encode_utf16().count() <= FUSED_ID_CAP {
        format!("t-{n}:{body}")
    } else {
        format!(
            "t-{n}:{}{}",
            crate::catalog::FUSED_DIGEST_MARK,
            fused_digest(&body)
        )
    }
}

/// An ingredient as a fused id writes it: in parentheses when it is itself a fused card (R179), and
/// followed by `*` when it went in on its Radiant face (R469).
fn ingredient_name(spec: &FusedIngredient) -> String {
    let name = if crate::catalog::fused_head_len(&spec.def_id).is_some() {
        format!("({})", spec.def_id)
    } else {
        spec.def_id.clone()
    };
    if spec.radiant == Some(true) {
        format!("{name}{}", crate::catalog::RADIANT_INGREDIENT_MARK)
    } else {
        name
    }
}

/// R468: a pure, deterministic 64-bit digest of a string, as 16 hex digits — two 32-bit lanes of
/// multiply-xorshift mixing (the cyrb53 construction, widened to both lanes), so no `crypto` and no
/// I/O (CLAUDE.md rule 4). It names an ingredient list, not a secret: all it must do is give two
/// different lists two different ids in any process that could hold both. Over UTF-16 code units, as
/// TS's `charCodeAt`, with TS's `Math.imul` as `wrapping_mul` and `>>>` as a `u32` shift.
pub fn fused_digest(text: &str) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let length = units.len() as u32;
    let mut h1: u32 = 0xdead_beef ^ length;
    let mut h2: u32 = 0x41c6_ce57 ^ length;
    for &code in &units {
        h1 = (h1 ^ u32::from(code)).wrapping_mul(2_654_435_761);
        h2 = (h2 ^ u32::from(code)).wrapping_mul(1_597_334_677);
    }
    h1 = (h1 ^ (h1 >> 16)).wrapping_mul(2_246_822_507) ^ (h2 ^ (h2 >> 13)).wrapping_mul(3_266_489_909);
    h2 = (h2 ^ (h2 >> 16)).wrapping_mul(2_246_822_507) ^ (h1 ^ (h1 >> 13)).wrapping_mul(3_266_489_909);
    format!("{h2:08x}{h1:08x}")
}

/// R179: the ingredient ids a fused def's id names, in ingredient order, or `None` for an id no Fuse
/// minted (a catalog card's, or a bare `t-<n>`). The inverse of `next_transient_id`: the list is split
/// at the `+` signs outside parentheses, and a parenthesised ingredient loses its parentheses
/// (`fused_id_specs_in`, which R387's self-exclusion reads too). R468: a digest id's list comes from
/// its definition in the state.
pub fn fused_ingredients(state: &GameState, def_id: &str) -> Option<Vec<String>> {
    let parts: Vec<String> = crate::catalog::fused_id_specs(Some(state), def_id)?
        .into_iter()
        .map(|spec| spec.def_id)
        .collect();
    if parts.len() >= FUSE_MIN_INGREDIENTS {
        Some(parts)
    } else {
        None
    }
}

/// R179, R468, R469: `fused_ingredients` with each ingredient's Radiant mark.
pub fn fused_ingredient_specs(state: &GameState, def_id: &str) -> Option<Vec<FusedIngredient>> {
    let specs = crate::catalog::fused_id_specs(Some(state), def_id)?;
    if specs.len() >= FUSE_MIN_INGREDIENTS {
        Some(specs)
    } else {
        None
    }
}

/// E36: a fused definition's lines of code are its ingredients' sum, absent when none has any.
fn summed_loc(defs: &[CardDef]) -> Option<i32> {
    let counted: Vec<i32> = defs.iter().filter_map(|def| def.loc).collect();
    if counted.is_empty() {
        None
    } else {
        Some(counted.iter().sum())
    }
}

fn build_def(
    state: &GameState,
    ingredients: &[CardInstance],
    defs: &[CardDef],
    target_def: Option<&CardDef>,
    forced: &[bool],
    kept_cost: Option<&KeptCost>,
    fixed_id: Option<&str>,
) -> CardDef {
    let specs: Vec<FusedIngredient> = defs
        .iter()
        .enumerate()
        .map(|(at, def)| FusedIngredient {
            def_id: def.id.clone(),
            radiant: if forced.get(at) == Some(&true) {
                Some(true)
            } else {
                None
            },
        })
        .collect();
    let id = match fixed_id {
        Some(id) => id.to_string(),
        None => next_transient_id(state, &specs),
    };
    let refs = union_refs(defs);
    let loc = summed_loc(defs);
    CardDef {
        // Transient defs are not catalog cards, so no random pool or Discover can reach one (§5.1);
        // the index is the id itself, which keeps `def_by_index` unambiguous.
        index: id.clone(),
        id,
        name: defs
            .iter()
            .map(|def| def.name.as_str())
            .collect::<Vec<&str>>()
            .join(" + "),
        set: target_def
            .map(|def| def.set)
            .or_else(|| defs.first().map(|def| def.set))
            .unwrap_or(SetName::Core),
        type_: fused_type(defs, target_def),
        tags: union_tags(defs),
        rarity: rarest_of(defs),
        printed_rarity: None,
        // A fusion is a real card unless every ingredient was a token, so fusing a token onto a unit
        // gives a result that no longer ceases to exist off the field (R11).
        token: defs.iter().all(|def| def.token),
        // R470: a kept X-cost or embiggen card that keeps its cost keeps that printed form.
        cost: kept_cost
            .and_then(|kept| kept.form)
            .unwrap_or_else(|| CardCost::Fixed(fused_cost(state, ingredients))),
        refs,
        params: None,
        loc,
        radiant_fallback: None,
        // R179, R468: the list the id names, kept on the definition so the scripts can be rebuilt from
        // it even when the id is only a digest of it.
        ingredients: Some(specs),
        base: fused_face(ingredients, defs, false, forced),
        radiant: fused_face(ingredients, defs, true, forced),
    }
}

/// R470: what "its cost doesn't change" writes, read off the kept card before it becomes the fusion:
/// nothing when it already has a `cost_override` (which the kept instance keeps); for an X-cost or
/// embiggen card with no cost hook, that printed form (`form`), which the fused definition then
/// prints; otherwise its own cost as it stands (`override_`, R65's printed cost with a hook's computed
/// one), which becomes its `cost_override`. `cost_mod` is the instance's and stays, so the card's cost
/// after the fusion is the one it had before it.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct KeptCost {
    form: Option<CardCost>,
    override_: Option<i32>,
}

fn kept_cost_of(state: &GameState, kept: &CardInstance) -> KeptCost {
    if kept.cost_override.is_some() {
        return KeptCost::default();
    }
    let printed = crate::catalog::def_of(Some(state), &kept.def_id).cost;
    if !matches!(printed, CardCost::Fixed(_)) && crate::scripts::script_of(state, kept).cost.is_none() {
        return KeptCost {
            form: Some(printed),
            override_: None,
        };
    }
    KeptCost {
        form: None,
        override_: Some(crate::mana::printed_cost(state, kept)),
    }
}

// ---------------------------------------------------------------------------------------------
// What a fused card remembers of its ingredients (TS `scripts.ts`'s `INGREDIENTS_KEY` readers and
// `work.ts`'s part paths), private copies so this module reads them exactly as it writes them.
// ---------------------------------------------------------------------------------------------

/// How much of `PART_KEY`'s path the combined hooks above the running one have used (`PART_DEPTH_KEY`).
fn part_depth(data: &IndexMap<String, Value>) -> usize {
    data.get(crate::work::PART_DEPTH_KEY)
        .and_then(Value::as_u64)
        .and_then(|depth| usize::try_from(depth).ok())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------------------------
// The concatenated scripts (R77).
// ---------------------------------------------------------------------------------------------

/// Builds an `EffectApply` from a closure, so its signature is the higher-ranked one.
fn effect_apply(f: impl Fn(&mut EffectContext<'_>) + Send + Sync + 'static) -> EffectApply {
    Arc::new(f)
}

/// Builds a `TriggerRun` from a closure, so its signature is the higher-ranked one.
fn trigger_run(
    f: impl Fn(&mut EffectContext<'_>, &GameEvent) -> Vec<Effect> + Send + Sync + 'static,
) -> TriggerRun {
    Arc::new(f)
}

/// TS `fused:part${index}`: an effect's kind is a static name, read by nothing but the debug output.
fn part_kind(index: usize) -> &'static str {
    const KINDS: [&str; 8] = [
        "fused:part0",
        "fused:part1",
        "fused:part2",
        "fused:part3",
        "fused:part4",
        "fused:part5",
        "fused:part6",
        "fused:part7",
    ];
    KINDS.get(index).copied().unwrap_or("fused:part")
}

/// TS `{ ...ctx }`: a second context over the same sink (state, events, rng and the per-call flags,
/// borrowed again) with every fact of the run copied, for a caller to change before it hands it on.
fn derive_context<'b>(ctx: &'b mut EffectContext<'_>) -> EffectContext<'b> {
    let events_from = ctx.events_from;
    let exits_from = ctx.exits_from;
    let chosen_from = ctx.chosen_from;
    let event_stay = ctx.event_stay.clone();
    let summoned = ctx.summoned.clone();
    let self_resolving = ctx.self_resolving;
    let controller = ctx.controller;
    let self_ = ctx.self_.clone();
    let def_id = ctx.def_id.clone();
    let radiant = ctx.radiant;
    let targets = ctx.targets.clone();
    let modes = ctx.modes.clone();
    let x = ctx.x;
    let embiggened = ctx.embiggened;
    let data = ctx.data.clone();
    let mana_before_play = ctx.mana_before_play;
    EffectContext {
        sink: ctx.sink.reborrow(),
        events_from,
        exits_from,
        chosen_from,
        event_stay,
        summoned,
        self_resolving,
        controller,
        self_,
        def_id,
        radiant,
        targets,
        modes,
        x,
        embiggened,
        data,
        mana_before_play,
    }
}

/// §10.4 layer 5: each ingredient's aura, reading "this" as the fused card at the price that
/// ingredient was played for (R102, `as_ingredient`): #46 Suppressive Aura played for 4 and
/// fused onto a Mana Well is still "paid 4: −5/−5", though the kept instance's own price is the Mana
/// Well's.
///
/// An aura's entries may borrow the card it was asked about, and a card read at an ingredient's price
/// is a copy made here; so an entry of such a reading carries that copy and asks its aura again for
/// each unit it is applied to (a pure read: the same answer as the first asking).
fn fused_aura(faces: &[Face]) -> Option<AuraHook> {
    let hooks: Vec<Option<AuraHook>> = faces.iter().map(|face| face.script.aura.clone()).collect();
    if hooks.iter().all(Option::is_none) {
        return None;
    }
    Some(aura_hook(move |args| {
        let mut out = Vec::new();
        for (index, aura) in hooks.iter().enumerate() {
            let Some(aura) = aura else {
                continue;
            };
            let recorded =
                crate::scripts::ingredients_of(args.self_).is_some_and(|records| records.len() > index);
            if !recorded {
                out.extend(aura(args));
                continue;
            }
            let card = Arc::new(crate::scripts::as_ingredient(args.self_, index));
            let mods: Vec<StatMod> = aura(HookArgs {
                state: args.state,
                self_: &card,
                radiant: args.radiant,
            })
            .into_iter()
            .map(|entry| entry.mod_)
            .collect();
            for (at, mod_) in mods.into_iter().enumerate() {
                let aura = aura.clone();
                let card = card.clone();
                let state = args.state;
                let radiant = args.radiant;
                out.push(AuraEntry {
                    applies: Box::new(move |unit: &CardInstance| {
                        aura(HookArgs {
                            state,
                            self_: &card,
                            radiant,
                        })
                        .get(at)
                        .is_some_and(|entry| (entry.applies)(unit))
                    }),
                    mod_,
                });
            }
        }
        out
    }))
}

/// The context ingredient `index`'s text builds and applies with (R102): its place in the fusion
/// appended to the path the combined hooks above it have used (`work::PART_KEY`). A re-entry names the
/// whole path from the top (`work::PART_DEPTH_KEY`), so a path that already names this ingredient at
/// this level keeps the rest of it, for the levels below to route by: a question asked two fusions
/// down comes back to its own ingredient, not to the first one there that names its step the same.
fn part_data(data: &IndexMap<String, Value>, index: usize) -> IndexMap<String, Value> {
    let depth = part_depth(data);
    let named = crate::work::part_path_of(data).unwrap_or_default();
    let path: Vec<usize> = if named.get(depth) == Some(&index) {
        named
    } else {
        let mut path: Vec<usize> = named.into_iter().take(depth).collect();
        path.push(index);
        path
    };
    let mut patch = IndexMap::new();
    patch.insert(crate::work::PART_KEY.to_string(), json!(path));
    patch.insert(crate::work::PART_DEPTH_KEY.to_string(), json!(depth + 1));
    patch
}

/// The context an ingredient's text runs in: its place in the fusion (`part_data`'s patch), and the
/// price its card was played for as `embiggened` (R102, `ingredient_paid`), found by that
/// place's path — #59's trigger reads it.
fn in_place<'b>(ctx: &'b mut EffectContext<'_>, patch: &IndexMap<String, Value>) -> EffectContext<'b> {
    let path = crate::work::part_path_of(patch).unwrap_or_default();
    let embiggened = match ctx.live_self() {
        None => ctx.embiggened,
        Some(card) => crate::scripts::ingredient_paid(card, &path),
    };
    let mut data = ctx.data.clone();
    for (key, value) in patch {
        data.insert(key.clone(), value.clone());
    }
    let mut inner = derive_context(ctx);
    inner.embiggened = embiggened;
    inner.data = data;
    inner
}

/// An effect that applies, and builds any part of its own, with its ingredient's place (R102).
fn in_ingredient(effect: Effect, patch: IndexMap<String, Value>) -> Effect {
    let patch = Arc::new(patch);
    let apply = effect.apply.clone();
    let apply_patch = patch.clone();
    let run = move |ctx: &mut EffectContext<'_>| {
        let mut inner = in_place(ctx, &apply_patch);
        apply(&mut inner);
    };
    match effect.expand.clone() {
        None => Effect::new(effect.kind, run),
        Some(expand) => Effect::with_expand(effect.kind, run, move |ctx, memo| {
            let built = {
                let mut inner = in_place(ctx, &patch);
                expand(&mut inner, memo)
            };
            EffectPart {
                effects: built
                    .effects
                    .into_iter()
                    .map(|inner| in_ingredient(inner, (*patch).clone()))
                    .collect(),
                memo: built.memo,
            }
        }),
    }
}

/// One ingredient's list as a part of the combined list (`lazy_part`, R102): built when the
/// list reaches it, with the ingredient's place in its context, and every effect of it applied there.
fn ingredient_part(
    index: usize,
    build: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static,
) -> Effect {
    crate::resolve::lazy_part(part_kind(index), move |at, _memo| {
        let patch = part_data(&at.data, index);
        let effects = {
            let mut inner = in_place(at, &patch);
            build(&mut inner)
        };
        // The part needs nothing handed to its rebuild; `null` says so in a form JSON keeps, where an
        // absent memo would sit in a paused list's `memo` array as `undefined` and come back as `null`,
        // so the paused state and its JSON round trip would differ (§9.3).
        EffectPart {
            effects: effects
                .into_iter()
                .map(|effect| in_ingredient(effect, patch.clone()))
                .collect(),
            memo: Some(Value::Null),
        }
    })
}

/// A combined hook: each ingredient's list in turn, as parts (R102, R113). A continuation one
/// ingredient's text left — the step its prompt re-enters, the delayed effect it scheduled — names
/// that ingredient (`work::PART_KEY`, which `prompts::resume_self` carries in the card's data), and comes
/// back to its list alone: the answer to one Masochism Mask's "choose one" is that Mask's pick, not a
/// pick for every ingredient that names its step the same.
fn combined_hook(fns: Vec<Option<Hook>>, step: bool) -> Hook {
    let fns = Arc::new(fns);
    hook(move |ctx| {
        let depth = part_depth(&ctx.data);
        // A step of the `resume` table is one continuation of one text, so one that names no part — the
        // engine left it for the card as a whole, not one of its texts: the prompt of the power R43
        // activates once (`hero_power::activate_power`) — comes back to the first ingredient that has the
        // step, once, rather than to every ingredient that names its step the same (R43, R102).
        let first = fns.iter().position(Option::is_some);
        let routed: Option<usize> = crate::work::part_path_of(&ctx.data)
            .and_then(|path| path.get(depth).copied())
            .or(if step { first } else { None });
        let indices: Vec<usize> = fns
            .iter()
            .enumerate()
            .filter(|(index, f)| f.is_some() && routed.is_none_or(|routed| routed == *index))
            .map(|(index, _)| index)
            .collect();
        indices
            .into_iter()
            .filter_map(|index| {
                let f = fns.get(index).cloned().flatten()?;
                Some(ingredient_part(index, move |built| f(built)))
            })
            .collect()
    })
}

/// One named target predicate across the ingredients that define it (§10.6, R102). It is not a hook:
/// a declaration's filter asks it with the candidate and gets a boolean back, in a context that
/// carries no instance data to route a part by, so wrapping it as a fused Cry threw on the first ask
/// and would have answered with a list of effects had it not. One ingredient's predicate stays as it
/// is; ingredients that name the same predicate must each admit the candidate, the one stricter
/// requirement that flags and numbers also take when they combine.
fn combined_checks(checks: Vec<TargetCheck>) -> TargetCheck {
    if checks.len() == 1
        && let Some(only) = checks.first()
    {
        return only.clone();
    }
    target_check(move |args| checks.iter().all(|check| check(args)))
}

/// TS `combineValues` for a hook-valued key (a Cry, Death, a start/end-of-turn hook, a resume step, a
/// delayed hook): one hook that runs each of them in turn, so "both Cry and Death lists run" (R77) —
/// each ingredient's list built only when the one before it has resolved (`lazy_part`, R102), so a
/// later ingredient reads the board the earlier ones left: #68's "8 if your hero is below 10" after
/// Reno has set the hero to 30, #22's meal after #100 has exiled it — and a continuation one of them
/// left comes back to that one alone (`combined_hook`). A key among `EAGER_KEYS` runs each at once.
fn combine_hooks(values: Vec<Option<Hook>>, key: &str, parent: &str) -> Option<Hook> {
    if values.iter().all(Option::is_none) {
        return None;
    }
    if EAGER_KEYS.contains(&key) {
        let defined: Vec<Hook> = values.into_iter().flatten().collect();
        if defined.len() == 1 {
            return defined.into_iter().next();
        }
        return Some(hook(move |ctx| {
            let mut out = Vec::new();
            for f in &defined {
                out.extend(f(ctx));
            }
            out
        }));
    }
    Some(combined_hook(values, parent == RESUME_KEY))
}

type ReadListHook<R> = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<R> + Send + Sync>;
type ConditionListHook<R> = Arc<dyn for<'a> Fn(ConditionContext<'a>) -> Vec<R> + Send + Sync>;

/// TS `combineValues` for an `EAGER_KEYS` read of `{ state, self, radiant }`: one ingredient's read
/// stays as it is, several become their lists in ingredient order, each asked with the same arguments.
fn eager_read<R: 'static>(values: Vec<Option<ReadListHook<R>>>) -> Option<ReadListHook<R>> {
    let defined: Vec<ReadListHook<R>> = values.into_iter().flatten().collect();
    match defined.len() {
        0 => None,
        1 => defined.into_iter().next(),
        _ => Some(read_hook(move |args| {
            defined.iter().flat_map(|f| f(args)).collect::<Vec<R>>()
        })),
    }
}

/// `eager_read` for a read of the condition context (`preview`, R280).
fn eager_condition<R: 'static>(values: Vec<Option<ConditionListHook<R>>>) -> Option<ConditionListHook<R>> {
    let defined: Vec<ConditionListHook<R>> = values.into_iter().flatten().collect();
    match defined.len() {
        0 => None,
        1 => defined.into_iter().next(),
        _ => Some(condition_hook(move |ctx| {
            defined.iter().flat_map(|f| f(ctx)).collect::<Vec<R>>()
        })),
    }
}

/// A list key (triggers, declared targets and modes, abilities, replacements): the lists in
/// ingredient order, so a fused trap carries every ingredient's trigger condition.
fn concat<T: Clone>(lists: impl IntoIterator<Item = Vec<T>>) -> Vec<T> {
    lists.into_iter().flatten().collect()
}

/// `resume`, the table of steps: combined even when one ingredient holds it, so every step of it is
/// wrapped at this fusion's level and a part path (`work::PART_KEY`) counts each level of a nesting,
/// as `param` walks it (R102). Each step is a hook of the table (`combine_hooks` with `RESUME_KEY` as
/// its parent); the keys keep their first-seen order across the ingredients.
fn combine_resume(records: &[Script]) -> IndexMap<&'static str, Hook> {
    let mut keys: IndexSet<&'static str> = IndexSet::new();
    for record in records {
        for key in record.resume.keys() {
            keys.insert(*key);
        }
    }
    let mut out = IndexMap::new();
    for key in keys {
        let values = records
            .iter()
            .map(|record| record.resume.get(key).cloned())
            .collect();
        if let Some(combined) = combine_hooks(values, key, RESUME_KEY) {
            out.insert(key, combined);
        }
    }
    out
}

/// `targetChecks`, the named predicates: one ingredient's table stays as it is, several are combined
/// name by name (`combined_checks`).
fn combine_target_checks(records: &[Script]) -> IndexMap<&'static str, TargetCheck> {
    let defined: Vec<&IndexMap<&'static str, TargetCheck>> = records
        .iter()
        .map(|record| &record.target_checks)
        .filter(|table| !table.is_empty())
        .collect();
    match defined.len() {
        0 => IndexMap::new(),
        1 => defined[0].clone(),
        _ => {
            let mut keys: IndexSet<&'static str> = IndexSet::new();
            for table in &defined {
                for key in table.keys() {
                    keys.insert(*key);
                }
            }
            let mut out = IndexMap::new();
            for key in keys {
                let checks: Vec<TargetCheck> = defined
                    .iter()
                    .filter_map(|table| table.get(key).cloned())
                    .collect();
                if !checks.is_empty() {
                    out.insert(key, combined_checks(checks));
                }
            }
            out
        }
    }
}

/// A flag of the combined static flags: true when any ingredient set it (TS `combineValues` on booleans).
fn any_flag(values: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    let defined: Vec<bool> = values.into_iter().flatten().collect();
    if defined.is_empty() {
        None
    } else {
        Some(defined.iter().any(|flag| *flag))
    }
}

/// A number of the combined static flags: the larger, which is the one stricter requirement rather
/// than a doubled one (`staticFlags.tribute`).
fn max_number(values: impl IntoIterator<Item = Option<i32>>) -> Option<i32> {
    values.into_iter().flatten().max()
}

/// The static flags that are an amount of what the text does, not a quality the card has: #79's
/// "the next Spell you play gains Echo +1", #38's granted Combo and #84's hero Armor (TS
/// `SUMMED_FLAGS`: `echoGrant`, `quickstriker`, `heroArmor`). R102: a card fused from two such texts
/// carries both, so its amount is theirs added — a Twinspell fused onto a Twinspell grants Echo +2,
/// and a Going Long onto a Going Long gives Armor twice, as two standing apart do (R124) — where a
/// quality (`castOnDraw`, `immutable`) is had once and a requirement (`tribute`) takes the stricter.
/// A `true` is one.
fn summed_count(values: impl IntoIterator<Item = Option<FlagOrCount>>) -> Option<FlagOrCount> {
    let defined: Vec<FlagOrCount> = values.into_iter().flatten().collect();
    if defined.is_empty() {
        None
    } else {
        Some(FlagOrCount::Count(defined.iter().map(|flag| flag.count()).sum()))
    }
}

/// `summed_count` for a flag that holds only a number (`echoGrant`).
fn summed_number(values: impl IntoIterator<Item = Option<i32>>) -> Option<i32> {
    let defined: Vec<i32> = values.into_iter().flatten().collect();
    if defined.is_empty() {
        None
    } else {
        Some(defined.iter().sum())
    }
}

/// `staticFlags`, a nested object, combined key by key by the same rules: a flag is true when any
/// ingredient set it, a number takes the larger, an amount adds up (`summed_count`), a list is the
/// lists in order. One ingredient's flags stay exactly as they are.
fn combine_static_flags(records: &[Script]) -> Option<StaticFlags> {
    let defined: Vec<&StaticFlags> = records
        .iter()
        .filter_map(|record| record.static_flags.as_ref())
        .collect();
    match defined.len() {
        0 => None,
        1 => Some(defined[0].clone()),
        _ => {
            let flags = |read: fn(&StaticFlags) -> Option<bool>| any_flag(defined.iter().map(|f| read(f)));
            let numbers = |read: fn(&StaticFlags) -> Option<i32>| max_number(defined.iter().map(|f| read(f)));
            let tagged: Vec<Vec<Tag>> = defined
                .iter()
                .filter_map(|f| f.radiant_plays_tagged.clone())
                .collect();
            Some(StaticFlags {
                cast_on_draw: flags(|f| f.cast_on_draw),
                quickdraw: flags(|f| f.quickdraw),
                infinite_reserves: flags(|f| f.infinite_reserves),
                never_defense: flags(|f| f.never_defense),
                echo: numbers(|f| f.echo),
                echo_grant: summed_number(defined.iter().map(|f| f.echo_grant)),
                quickstriker: summed_count(defined.iter().map(|f| f.quickstriker)),
                gifted_program: numbers(|f| f.gifted_program),
                tribute: numbers(|f| f.tribute),
                tribute_worth: numbers(|f| f.tribute_worth),
                tribute_enemies: flags(|f| f.tribute_enemies),
                enemy_tribute_hands_over: flags(|f| f.enemy_tribute_hands_over),
                anti_oneshot: flags(|f| f.anti_oneshot),
                hero_armor: summed_count(defined.iter().map(|f| f.hero_armor)),
                counts_plays: flags(|f| f.counts_plays),
                return_keeps_price: flags(|f| f.return_keeps_price),
                carrier: flags(|f| f.carrier),
                fuses_carried: flags(|f| f.fuses_carried),
                radiant_plays_tagged: match tagged.len() {
                    0 => None,
                    _ => Some(concat(tagged)),
                },
                copies_last_spell: flags(|f| f.copies_last_spell),
                cant_be_attacked: flags(|f| f.cant_be_attacked),
                attacked_only_from_lane: flags(|f| f.attacked_only_from_lane),
                cant_attack_or_be_attacked: flags(|f| f.cant_attack_or_be_attacked),
                never_berserk: flags(|f| f.never_berserk),
                heal_to_damage: flags(|f| f.heal_to_damage),
            })
        }
    }
}

/// `quests`, a nested object: one ingredient's book stays as it is; several combine key by key — the
/// quests and the rewards are the lists in order, and `first`, a string, is the last ingredient's
/// (TS: "nothing in `Script` mixes kinds under one key; the last ingredient wins").
fn combine_quests(records: &[Script]) -> Option<QuestBook> {
    let defined: Vec<&QuestBook> = records
        .iter()
        .filter_map(|record| record.quests.as_ref())
        .collect();
    match defined.len() {
        0 => None,
        1 => Some(defined[0].clone()),
        _ => Some(QuestBook {
            first: defined[defined.len() - 1].first.clone(),
            quests: concat(defined.iter().map(|book| book.quests.clone())),
            rewards: concat(defined.iter().map(|book| book.rewards.clone())),
        }),
    }
}

/// TS `combineObjects` over the ingredients' scripts: combine every member of several scripts,
/// `records` aligned with the ingredients. The rule is the same for every kind of value a script
/// holds, which is what keeps this working as `Script` grows new hooks:
///   - a hook becomes one hook that runs each of them in turn (`combine_hooks`); an aura, which
///     returns no effects, runs each at once (`EAGER_KEYS`);
///   - a list (triggers, declared targets and modes) becomes the lists in ingredient order, so a
///     fused trap carries every ingredient's trigger condition;
///   - a nested object (static flags, the resume table) is combined key by key by the same rules;
///   - a flag is true when any ingredient set it, and a number takes the larger, which is the one
///     stricter requirement rather than a doubled one (`staticFlags.tribute`) — except an amount of
///     what the text does, which adds up (`summed_count`).
///
/// `cost`, `setStat`, `conditionMet`, `tributeWhen`, `plagueMultiplier` and `wouldCounter` were taken
/// out of each record (`script_record`) and are combined on their own (`fused_script`), as are `cry`
/// and `aura`, which `fused_cry` and `fused_aura` replace. `targetingDiscards` and `recordsPlayAs`
/// return no list either; TS's combination of them is never asked, since their readers take a fused
/// card's ingredients one by one (`targeting.ts`'s `fusedCost`, `playCounts.playRecordOf`), so a
/// fusion carries neither.
fn combine_objects(records: &[Script]) -> Script {
    let hooks = |read: fn(&Script) -> Option<Hook>, key: &str| {
        combine_hooks(records.iter().map(read).collect(), key, "")
    };
    let lists_of_triggers = |read: fn(&Script) -> Vec<TriggerDef>| concat(records.iter().map(read));
    Script {
        cost: None,
        cry: None,
        death: hooks(|s| s.death.clone(), "death"),
        start_of_game: hooks(|s| s.start_of_game.clone(), "startOfGame"),
        enters_hand: hooks(|s| s.enters_hand.clone(), "entersHand"),
        resume: combine_resume(records),
        delayed: hooks(|s| s.delayed.clone(), "delayed"),
        set_stat: None,
        start_of_turn: hooks(|s| s.start_of_turn.clone(), "startOfTurn"),
        end_of_turn: hooks(|s| s.end_of_turn.clone(), "endOfTurn"),
        // ME-GRANT (MD-D13): a fused card merges its ingredients' grant maps (R102); the keys name
        // their definition (`<defId>#<key>`), so they never collide.
        grants: {
            let mut grants: IndexMap<&'static str, Hook> = IndexMap::new();
            for record in records {
                for (key, hook) in &record.grants {
                    grants.insert(*key, hook.clone());
                }
            }
            grants
        },
        aura: None,
        triggers: lists_of_triggers(|s| s.triggers.clone()),
        on_play_hook: hooks(|s| s.on_play_hook.clone(), "onPlayHook"),
        hand_triggers: lists_of_triggers(|s| s.hand_triggers.clone()),
        static_flags: combine_static_flags(records),
        targets: concat(records.iter().map(|s| s.targets.clone())),
        modes: concat(records.iter().map(|s| s.modes.clone())),
        condition_met: None,
        preview: eager_condition(records.iter().map(|s| s.preview.clone()).collect()),
        activations: concat(records.iter().map(|s| s.activations.clone())),
        target_checks: combine_target_checks(records),
        cost_aura: eager_read(records.iter().map(|s| s.cost_aura.clone()).collect()),
        graveyard_play: eager_read(records.iter().map(|s| s.graveyard_play.clone()).collect()),
        targeting_discards: None,
        records_play_as: None,
        draw_limit: eager_read(records.iter().map(|s| s.draw_limit.clone()).collect()),
        replacements: concat(records.iter().map(|s| s.replacements.clone())),
        hero_guard: eager_read(records.iter().map(|s| s.hero_guard.clone()).collect()),
        conditional_keywords: eager_read(records.iter().map(|s| s.conditional_keywords.clone()).collect()),
        after_attack: hooks(|s| s.after_attack.clone(), "afterAttack"),
        plague_multiplier: None,
        deck_triggers: lists_of_triggers(|s| s.deck_triggers.clone()),
        graveyard_triggers: lists_of_triggers(|s| s.graveyard_triggers.clone()),
        quests: combine_quests(records),
        tribute_when: None,
        would_counter: None,
        start_of_opponent_turn: hooks(|s| s.start_of_opponent_turn.clone(), "startOfOpponentTurn"),
    }
}

/// One ingredient's script, ready to be combined: without its `cost` hook, because R77 fixes the
/// fused cost at min(sum, 4) and a surviving Ceaseless Void hook would overrule it (R65); without the
/// members that return no list (`setStat`, summed like every other stat R77 sums; `conditionMet`,
/// R195's yellow glow, which answers a boolean, so the ingredients' hooks are or-ed (R196);
/// `tributeWhen` and `wouldCounter` (R403, R667), or-ed the same way; `plagueMultiplier` (R471), which
/// multiplies), each combined on its own; and with its trigger ids namespaced, so two ingredients that
/// both call a trigger "turn-end" stay two distinct conditions on the fused card — each running in its
/// ingredient's place (R102), so a question it asks comes back to its own step — and its Activate
/// abilities run in its place (R102, R384).
fn script_record(script: &Script, def_id: &str, index: usize) -> Script {
    let namespaced = |list: &[TriggerDef]| -> Vec<TriggerDef> {
        list.iter()
            .map(|trigger| TriggerDef {
                id: format!("{def_id}:{}", trigger.id),
                run: in_trigger_ingredient(trigger, index),
                ..trigger.clone()
            })
            .collect()
    };
    Script {
        cost: None,
        set_stat: None,
        condition_met: None,
        tribute_when: None,
        plague_multiplier: None,
        would_counter: None,
        triggers: namespaced(&script.triggers),
        hand_triggers: namespaced(&script.hand_triggers),
        deck_triggers: namespaced(&script.deck_triggers),
        graveyard_triggers: namespaced(&script.graveyard_triggers),
        activations: script
            .activations
            .iter()
            .map(|decl| in_activation_ingredient(decl, index))
            .collect(),
        ..script.clone()
    }
}

/// R102, R384: an ingredient's Activate ability in its place in the fusion. Its effect list builds
/// and applies there, as a trigger's does (`in_trigger_ingredient`), so it reads its own numbers
/// (`param`) and what its own text remembered (`recalled`); its `canActivate` and `has`, which carry
/// no context to name a place, read the card as that ingredient's text does (`as_ingredient_text`).
fn in_activation_ingredient(decl: &ActivationDecl, index: usize) -> ActivationDecl {
    let run = decl.run.clone();
    let can_activate: Option<ConditionHook> = decl.can_activate.clone().map(|can_activate| {
        condition_hook(move |ctx| {
            let card = as_ingredient_text(ctx.self_, index);
            can_activate(ConditionContext {
                state: ctx.state,
                self_: &card,
                controller: ctx.controller,
                radiant: ctx.radiant,
                zone: ctx.zone,
                your_turn: ctx.your_turn,
            })
        })
    });
    let has = decl.has.clone().map(|has| {
        read_hook(move |args| {
            let card = as_ingredient_text(args.self_, index);
            has(HookArgs {
                state: args.state,
                self_: &card,
                radiant: args.radiant,
            })
        })
    });
    ActivationDecl {
        run: hook(move |ctx| {
            let patch = part_data(&ctx.data, index);
            let effects = {
                let mut inner = in_place(ctx, &patch);
                run(&mut inner)
            };
            effects
                .into_iter()
                .map(|effect| in_ingredient(effect, patch.clone()))
                .collect()
        }),
        can_activate,
        has,
        ..decl.clone()
    }
}

/// R102: the card as ingredient `index`'s text reads it from a hook with no context to name its
/// place: at that ingredient's price (`as_ingredient`), with what that text remembered under
/// its own keys (`memory_of_part`), so `recalled` with no part named reads it back.
fn as_ingredient_text(instance: &CardInstance, index: usize) -> CardInstance {
    let mut card = crate::scripts::as_ingredient(instance, index);
    card.memory = crate::work::memory_of_part(&card.memory, index);
    card
}

/// A trigger's list, built and applied in its ingredient's place (R102).
fn in_trigger_ingredient(trigger: &TriggerDef, index: usize) -> TriggerRun {
    let run = trigger.run.clone();
    trigger_run(move |ctx, event| {
        let patch = part_data(&ctx.data, index);
        let effects = {
            let mut inner = in_place(ctx, &patch);
            run(&mut inner, event)
        };
        effects
            .into_iter()
            .map(|effect| in_ingredient(effect, patch.clone()))
            .collect()
    })
}

/// §10.4 layer 2 (#92 Felinor Fiender): a card that sets its own stats from the board. R77 sums
/// printed stats, so a fusion of two set-stat cards sums what they set, and a stat only one of them
/// sets is that one's.
fn fused_set_stat(scripts: &[&Script]) -> Option<SetStatHook> {
    let fns: Vec<SetStatHook> = scripts
        .iter()
        .filter_map(|script| script.set_stat.clone())
        .collect();
    match fns.len() {
        0 => None,
        1 => fns.into_iter().next(),
        _ => Some(read_hook(move |args| {
            let mut attack: Option<i32> = None;
            let mut max_health: Option<i32> = None;
            for f in &fns {
                let set = f(args);
                if let Some(value) = set.attack {
                    attack = Some(attack.unwrap_or(0) + value);
                }
                if let Some(value) = set.max_health {
                    max_health = Some(max_health.unwrap_or(0) + value);
                }
            }
            SetStat { attack, max_health }
        })),
    }
}

/// R471: "Plague Counters placed on this are doubled" (Classic #27). Each ingredient's text multiplies
/// what is placed on the fused card, so two such texts multiply: a Pestilent Slime fused onto a
/// Pestilent Slime quadruples, as two doublings in a row do. Each hook is asked about the fused card
/// at its own ingredient's price (`as_ingredient`, R102).
fn fused_plague_multiplier(scripts: &[&Script]) -> Option<PlagueMultiplierHook> {
    let hooks: Vec<Option<PlagueMultiplierHook>> = scripts
        .iter()
        .map(|script| script.plague_multiplier.clone())
        .collect();
    if hooks.iter().all(Option::is_none) {
        return None;
    }
    Some(read_hook(move |args| {
        hooks
            .iter()
            .enumerate()
            .fold(1, |product, (index, multiplier)| match multiplier {
                None => product,
                Some(multiplier) => {
                    let card = crate::scripts::as_ingredient(args.self_, index);
                    product
                        * multiplier(HookArgs {
                            state: args.state,
                            self_: &card,
                            radiant: args.radiant,
                        })
                }
            })
    }))
}

/// One ingredient's face script (TS `type Face = { defId; script }`): a registered card's borrowed
/// from the registry, a fused ingredient's composed.
#[derive(Clone)]
struct Face {
    def_id: String,
    script: crate::scripts::ScriptRef,
}

/// An effect that resolves with one ingredient's own play choices, whatever context applies it.
fn with_choices(effect: Effect, targets: Vec<Selection>, modes: Vec<String>) -> Effect {
    let apply = effect.apply.clone();
    Effect {
        kind: effect.kind,
        apply: effect_apply(move |ctx| {
            let mut inner = derive_context(ctx);
            inner.targets = targets.clone();
            inner.modes = modes.clone();
            apply(&mut inner);
        }),
        expand: effect.expand,
    }
}

/// R102 concatenates the ingredients' declared targets and modes in ingredient order, and R90 reads
/// that flat list declaration by declaration — so each ingredient's Cry must resolve with its OWN
/// slice of the play's choices, not the whole list. Handed the whole list, every ingredient read its
/// first slot: a crafted Bigot + Twisted Sorcerer aimed the Sorcerer's 4 damage at the unit Bigot
/// destroyed, and an Archivist + Silly Silas rotated by Archivist's "highest". Modes split by each
/// ingredient's count of mode declarations; targets split by R90's rule over the declarations that
/// the ingredient's own modes make active (`forModes`) — with the lengths §10.5 step 1 read the play
/// with, which the pipeline hands over in `data` (`DECLARATION_SLICES_KEY`), because the board at
/// resolution is not the one the play was checked against: by step 5 a crafted Postdoc + Sorcerer
/// stands on the field and is itself a Human the Postdoc's declaration could take. Only a Cry run
/// with no play behind it measures against the board as it stands.
///
/// Every effect an ingredient's Cry returns is bound to that slice, because an effect reads
/// `ctx.targets` when it applies, and the context applying it is the fused card's. And each
/// ingredient's Cry is its own part of the list (`lazy_part`), built when the list reaches it,
/// so it reads the board the ingredients before it left (R102), and a pause inside it resumes into
/// the rest of that part and then every part after it (`prompts::apply_resumable`, R113).
fn fused_cry(faces: &[Face]) -> Option<Hook> {
    if !faces.iter().any(|face| face.script.cry.is_some()) {
        return None;
    }
    let faces: Arc<Vec<Face>> = Arc::new(faces.to_vec());
    Some(hook(move |ctx| {
        let mut modes_of: Vec<Vec<String>> = Vec::new();
        let mut mode_at = 0;
        for face in faces.iter() {
            let count = face.script.modes.len();
            modes_of.push(ctx.modes.iter().skip(mode_at).take(count).cloned().collect());
            mode_at += count;
        }
        let decls_of: Vec<Vec<TargetDecl>> = faces
            .iter()
            .enumerate()
            .map(|(index, face)| {
                crate::play_choices::active_target_decls(&face.script.targets, &modes_of[index])
            })
            .collect();
        let decls: Vec<TargetDecl> = decls_of.iter().flatten().cloned().collect();
        let stored = crate::play_choices::stored_declaration_slices(&ctx.data);
        let slices: Vec<Vec<Selection>> = match stored {
            Some(stored) if stored.len() == decls.len() => cut_slices(&ctx.targets, &stored),
            _ => match ctx.live_self().cloned() {
                None => decls.iter().map(|_| Vec::new()).collect(),
                Some(card) => crate::play_choices::selections_per_declaration(
                    ctx.sink.state,
                    ctx.controller,
                    &card,
                    &decls,
                    &ctx.targets,
                ),
            },
        };

        // Each ingredient's Cry is built as the list reaches it (`lazy_part`), so it reads the board the
        // ingredients before it left (R102); its slice of the play's choices is fixed now, as step 1
        // checked them.
        let mut decl_at = 0;
        faces
            .iter()
            .enumerate()
            .map(|(index, face)| {
                let count = decls_of.get(index).map_or(0, Vec::len);
                let targets: Vec<Selection> = slices
                    .iter()
                    .skip(decl_at)
                    .take(count)
                    .flatten()
                    .cloned()
                    .collect();
                decl_at += count;
                let cry = face.script.cry.clone();
                let modes = modes_of.get(index).cloned().unwrap_or_default();
                ingredient_part(index, move |at| match &cry {
                    None => Vec::new(),
                    Some(cry) => {
                        let built = {
                            let mut inner = derive_context(at);
                            inner.targets = targets.clone();
                            inner.modes = modes.clone();
                            cry(&mut inner)
                        };
                        built
                            .into_iter()
                            .map(|effect| with_choices(effect, targets.clone(), modes.clone()))
                            .collect()
                    }
                })
            })
            .collect()
    }))
}

/// The flat list cut into consecutive slices of these lengths; the last takes the remainder (R90).
fn cut_slices(selections: &[Selection], lengths: &[usize]) -> Vec<Vec<Selection>> {
    let mut at = 0;
    lengths
        .iter()
        .enumerate()
        .map(|(index, length)| {
            let slice: Vec<Selection> = if index == lengths.len() - 1 {
                selections.iter().skip(at).cloned().collect()
            } else {
                selections.iter().skip(at).take(*length).cloned().collect()
            };
            at += slice.len();
            slice
        })
        .collect()
}

/// R196: a fusion's yellow glow. Its Cry, Death and triggers run every ingredient's list, so each
/// ingredient's printed condition still picks its own branch when the fused card resolves, and the
/// fused card glows when any of them holds. Each hook is asked with the fused card's own context
/// (the fused instance as `self`, the face it runs), and only an answer of exactly `true` counts,
/// as `conditionActive` counts it. One hooked ingredient's hook is the fusion's unchanged.
fn fused_condition_met(scripts: &[&Script]) -> Option<ConditionHook> {
    let hooks: Vec<ConditionHook> = scripts
        .iter()
        .filter_map(|script| script.condition_met.clone())
        .collect();
    match hooks.len() {
        0 => None,
        1 => hooks.into_iter().next(),
        _ => Some(condition_hook(move |ctx| hooks.iter().any(|hook| hook(ctx)))),
    }
}

/// R403, R102: a fusion carries every ingredient's "When …, Tribute this", so any one that holds takes it.
fn fused_tribute_when(scripts: &[&Script]) -> Option<TributeWhenHook> {
    let hooks: Vec<TributeWhenHook> = scripts
        .iter()
        .filter_map(|script| script.tribute_when.clone())
        .collect();
    if hooks.len() <= 1 {
        return hooks.into_iter().next();
    }
    Some(read_hook(move |args| hooks.iter().any(|hook| hook(args))))
}

/// R667, R102: a fusion carries every ingredient's counter trigger, so it would counter what any of them would.
fn fused_would_counter(scripts: &[&Script]) -> Option<WouldCounterHook> {
    let hooks: Vec<WouldCounterHook> = scripts
        .iter()
        .filter_map(|script| script.would_counter.clone())
        .collect();
    if hooks.len() <= 1 {
        return hooks.into_iter().next();
    }
    Some(would_counter_hook(move |args| {
        hooks.iter().any(|hook| hook(args))
    }))
}

/// One form's script of a fusion: each ingredient's script on that form — or on its Radiant form
/// whichever form this is, for an ingredient that went in on it (R469) — combined member by member.
fn fused_script(
    state: &GameState,
    specs: &[FusedIngredient],
    radiant: bool,
    seen: &IndexSet<String>,
) -> Script {
    let faces: Vec<Face> = specs
        .iter()
        .map(|spec| Face {
            def_id: spec.def_id.clone(),
            script: face_for(state, &spec.def_id, radiant || spec.radiant == Some(true), seen),
        })
        .collect();
    let scripts: Vec<&Script> = faces.iter().map(|face| &*face.script).collect();
    let records: Vec<Script> = faces
        .iter()
        .enumerate()
        .map(|(index, face)| script_record(&face.script, &face.def_id, index))
        .collect();
    let mut combined = combine_objects(&records);
    combined.tribute_when = fused_tribute_when(&scripts);
    combined.would_counter = fused_would_counter(&scripts);
    combined.set_stat = fused_set_stat(&scripts);
    combined.condition_met = fused_condition_met(&scripts);
    combined.aura = fused_aura(&faces);
    combined.cry = fused_cry(&faces);
    combined.plague_multiplier = fused_plague_multiplier(&scripts);
    combined
}

// ---------------------------------------------------------------------------------------------
// Building a fused card's scripts on lookup (SURFACE §6.6, in place of TS's registry sync).
// ---------------------------------------------------------------------------------------------

/// R179, R468: the ingredients a fused id names — its definition's list when the state holds the
/// definition, else the id read back (`fused_ingredient_specs`) — with R77's two-ingredient minimum;
/// `None` for any id no Fuse minted.
fn fused_specs(state: &GameState, def_id: &str) -> Option<Vec<FusedIngredient>> {
    let from_def = state
        .transient_defs
        .get(def_id)
        .and_then(|def| def.ingredients.as_ref())
        .map(|list| {
            list.iter()
                .map(crate::catalog::copy_spec)
                .collect::<Vec<FusedIngredient>>()
        });
    let specs = match from_def {
        Some(specs) => specs,
        None => crate::catalog::fused_id_specs(Some(state), def_id)?,
    };
    if specs.len() >= FUSE_MIN_INGREDIENTS {
        Some(specs)
    } else {
        None
    }
}

/// The script an ingredient runs on one form: a fused ingredient's composed here on that form, its
/// fused ingredients first (TS `ensureFused`); any other id's from the registry (`scripts::script_of`).
/// `seen` stops a malformed id that names itself: TS left such an id unregistered, so it ran no script.
///
/// Only the form asked for is composed. TS composed each fused id once, into its registry, so a chain
/// of fusions cost one composition per link; composing both forms of every ingredient on every lookup
/// doubles the work at each link, and a Fuse of a Fuse of a Fuse … fourteen deep (golden seed 68) is
/// then 2^14 compositions on every `script_of`.
fn face_for(
    state: &GameState,
    def_id: &str,
    radiant: bool,
    seen: &IndexSet<String>,
) -> crate::scripts::ScriptRef {
    match fused_specs(state, def_id) {
        None => crate::scripts::face_ref(state, def_id, radiant),
        Some(_) if seen.contains(def_id) => crate::scripts::ScriptRef::Composed(Arc::new(Script::default())),
        Some(specs) => {
            let mut inside = seen.clone();
            inside.insert(def_id.to_string());
            crate::scripts::ScriptRef::Composed(Arc::new(fused_script(state, &specs, radiant, &inside)))
        }
    }
}

/// TS `{ base: fusedScript(from, false), radiant: fusedScript(from, true) }`, with `def_id` marked seen.
fn compose_specs(
    state: &GameState,
    def_id: &str,
    specs: &[FusedIngredient],
    seen: &IndexSet<String>,
) -> CardScripts {
    let mut inside = seen.clone();
    inside.insert(def_id.to_string());
    CardScripts {
        base: fused_script(state, specs, false, &inside),
        radiant: fused_script(state, specs, true, &inside),
    }
}

/// SURFACE §6.6: a fused (or crafted) definition's two scripts, composed from its ingredients'
/// scripts by TS's `combineObjects` rules — what TS registered under the def's id as it minted it,
/// and what `syncFusedScripts` rebuilt from the id alone (R179). `scripts::script_of` calls it for an
/// id it finds in `state.transient_defs`. R468: a digest id's list is read off its definition
/// (`CardDef.ingredients`), so a digest that names another digest finds both. A definition that names
/// no fusion has no script, as TS's registry held none for it.
pub fn compose_fused_scripts(state: &GameState, def: &CardDef) -> CardScripts {
    match fused_def_specs(state, def) {
        Some(specs) => compose_specs(state, &def.id, &specs, &IndexSet::new()),
        None => CardScripts::default(),
    }
}

/// One face of `compose_fused_scripts(state, def)` (`radiant`: its radiant script, else its base),
/// composed alone: what a lookup of a fused instance's running face needs, at half the work.
pub fn compose_fused_face(state: &GameState, def: &CardDef, radiant: bool) -> Script {
    let Some(specs) = fused_def_specs(state, def) else {
        return Script::default();
    };
    let mut inside: IndexSet<String> = IndexSet::new();
    inside.insert(def.id.clone());
    fused_script(state, &specs, radiant, &inside)
}

/// The ingredients `compose_fused_scripts` composes a definition from: its own list (R468) with R77's
/// minimum, else the ones its id names; `None` when it names no fusion.
fn fused_def_specs(state: &GameState, def: &CardDef) -> Option<Vec<FusedIngredient>> {
    let own = def
        .ingredients
        .as_ref()
        .map(|list| {
            list.iter()
                .map(crate::catalog::copy_spec)
                .collect::<Vec<FusedIngredient>>()
        })
        .filter(|specs| specs.len() >= FUSE_MIN_INGREDIENTS);
    match own {
        Some(specs) => Some(specs),
        None => fused_specs(state, &def.id),
    }
}

/// R179, R417, R564: the definition `def_id` names in this state — a catalog card's, one the state
/// already holds, or a fused one rebuilt from the id alone into `transient_defs` (its fused
/// ingredients first); its scripts are composed on lookup like any fused card's. No instance stands
/// behind it, so each ingredient is worn as printed and priced as R65 reads it out of play, with no
/// target on the field (R77's shared type). `None` for an id that cannot be rebuilt: a digest
/// (R468), a bare `t-<n>`, an ingredient the catalog lacks. C+ #29 brings fused cards back this way.
pub fn rebuild_fused_def(state: &mut GameState, def_id: &str, owner: PlayerId) -> Option<CardDef> {
    if let Some(known) = crate::catalog::find_def(Some(&*state), def_id).cloned() {
        return Some(known);
    }
    let specs = if crate::catalog::is_digest_id(def_id) {
        None
    } else {
        fused_ingredient_specs(state, def_id)
    }?;
    let mut defs: Vec<CardDef> = Vec::new();
    for spec in &specs {
        defs.push(rebuild_fused_def(state, &spec.def_id, owner)?);
    }
    // Instances in no pile, numbered off a scratch counter so the state's ids are untouched.
    let mut scratch: u32 = 0;
    let ingredients: Vec<CardInstance> = defs
        .iter()
        .map(|def| new_instance(&mut scratch, &def.id, owner, Zone::Gone { player: owner }))
        .collect();
    let forced: Vec<bool> = specs.iter().map(|spec| spec.radiant == Some(true)).collect();
    let def = build_def(state, &ingredients, &defs, None, &forced, None, Some(def_id));
    state.transient_defs.insert(def_id.to_string(), def.clone());
    crate::scripts::sync_fused_scripts(state);
    Some(def)
}

// ---------------------------------------------------------------------------------------------
// The result.
// ---------------------------------------------------------------------------------------------

/// R77's keep-the-instance path. The fused card *is* the target: only its def id, its buffs (the sum
/// of every ingredient's) and its granted keywords (their union) change, and the rest of the
/// instance — zone, position, damage, exertion, summoned_turn, counters, memory, the radiant flag and
/// the Vanilla flag — is left exactly as it was. A token's `stats_override` and `armor_override` are one
/// exception: they were its printed face (§7, R175), which `worn_face` has already summed into the
/// fused definition, so they leave the instance with it — kept, §10.4's layer 1 would read them in
/// place of the fused face and a 3/3 Bread Token fused with a 7/7 would still be a 3/3. The other is
/// R77's own: the memory gains the ingredients' prices (`scripts::INGREDIENTS_KEY`) when they were not
/// all the kept card's, so each ingredient's text reads its own (R102).
fn keep_instance(
    state: &mut GameState,
    def: &CardDef,
    ingredients: &[CardInstance],
    kept: &CardInstance,
) -> CardInstance {
    let attack: i32 = ingredients.iter().map(|card| card.buffs.attack).sum();
    let health: i32 = ingredients.iter().map(|card| card.buffs.health).sum();
    let all_granted: Vec<Keyword> = ingredients
        .iter()
        .flat_map(|card| card.granted_keywords.iter().cloned())
        .collect();
    let granted = union_keywords(&all_granted);
    let before = crate::catalog::def_of(Some(&*state), &kept.def_id).clone();

    // R102: the price each ingredient's text reads as its own, recorded before any of them ceases to
    // exist and only when they are not all the kept card's (`scripts::INGREDIENTS_KEY`).
    let record = crate::scripts::ingredient_record(kept, ingredients);
    let at = ingredients.iter().position(|card| card.id == kept.id);
    if let Some(live) = find_instance_mut(state, &kept.id) {
        // R77, R102: the kept card's texts become ingredient `index` of the fusion, and what they
        // remembered moves with them to the path they now run at (`reroot_remembered`). (TS's
        // `findIndex` of a kept card that is no ingredient was -1; `work`'s index is a `usize`.)
        if let Some(at) = at {
            crate::work::reroot_remembered(&mut live.memory, at);
        }
        gain_printed_keywords(live, &before, def);
        live.def_id = def.id.clone();
        live.buffs = AttackHealth { attack, health };
        live.granted_keywords = granted;
        // R102, B3.4 rule 4, R443: the fused card sums its ingredients' tuning and carries all their
        // enchantments.
        carry_instance_data(live, ingredients);
        match record {
            None => {
                live.memory.shift_remove(crate::scripts::INGREDIENTS_KEY);
            }
            Some(record) => {
                live.memory.insert(
                    crate::scripts::INGREDIENTS_KEY.to_string(),
                    serde_json::to_value(record).unwrap_or(Value::Null),
                );
            }
        }
        live.stats_override = None;
        live.armor_override = None;
    }

    // R77 and R86: an ingredient that is not kept ceases to exist — no graveyard, no exile pile, no
    // Death trigger and nothing destroyed — and one that stood on the field has left it (R174).
    for card in ingredients {
        if card.id == kept.id {
            continue;
        }
        crate::zones::cease_to_exist(state, &mut card.clone());
    }
    find_instance(state, &kept.id)
        .cloned()
        .unwrap_or_else(|| kept.clone())
}

/// §5.2: "newly gained keywords apply at once". The fused face can print a keyword the kept card's
/// own face did not — Jilliax's Divine Shield fused onto a K-Pop Fanatic, a Radiant Saintess's Reborn
/// onto a unit that came back through a granted one — and the card gains it with the new text, so a
/// shield or a Reborn the card had spent is up again, as radiant #50's printed shield is after a
/// granted one was spent (`effects::radiant`). A keyword the kept face already printed is not newly
/// gained, and stays spent. A Vanilla unit prints nothing on either face (§6.3).
fn gain_printed_keywords(kept: &mut CardInstance, before: &CardDef, after: &CardDef) {
    if kept.vanilla {
        return;
    }
    let radiant = kept.radiant;
    let prints = |def: &CardDef, kind: KeywordKind| -> bool {
        let face = if radiant { &def.radiant } else { &def.base };
        face.keywords.iter().any(|keyword| keyword.kind() == kind)
    };
    if prints(after, KeywordKind::DivineShield) && !prints(before, KeywordKind::DivineShield) {
        kept.divine_shield_spent = None;
    }
    if prints(after, KeywordKind::Reborn) && !prints(before, KeywordKind::Reborn) {
        kept.reborn_spent = None;
    }
}

/// R102, B3.4 rule 4, R443: what a fusion's card carries of its ingredients' instance data — their
/// tuning summed (`tuning::sum_tunings`) and their enchantments united — and their granted tags
/// united (MD-B15, R923). A Brittle count is a counter, which a Fuse keeps only on the kept card as
/// it keeps its other counters (R77).
fn carry_instance_data(card: &mut CardInstance, ingredients: &[CardInstance]) {
    let tunings: Vec<_> = ingredients
        .iter()
        .map(|ingredient| ingredient.tuning.clone())
        .collect();
    card.tuning = crate::tuning::sum_tunings(&tunings);
    card.enchantments = crate::enchantments::united_enchantments(ingredients);
    // MD-B15, R923: a Fuse unites the granted tags of every ingredient, the kept card itself
    // included, in order — and carries none when no ingredient has any.
    let mut seen: IndexSet<Tag> = IndexSet::new();
    for tags in std::iter::once(card.granted_tags.clone().unwrap_or_default()).chain(
        ingredients
            .iter()
            .map(|ingredient| ingredient.granted_tags.clone().unwrap_or_default()),
    ) {
        for tag in tags {
            seen.insert(tag);
        }
    }
    card.granted_tags = if seen.is_empty() {
        None
    } else {
        Some(seen.into_iter().collect())
    };
    // ME-GRANT (MD-D13): a Fuse unites the granted Death abilities of every ingredient, the kept
    // card itself included, in order — and carries none when no ingredient has any.
    let mut united: Vec<Grant> = card.grants.clone().unwrap_or_default();
    for ingredient in ingredients {
        for grant in ingredient.grants.clone().unwrap_or_default() {
            if !united.contains(&grant) {
                united.push(grant);
            }
        }
    }
    card.grants = if united.is_empty() { None } else { Some(united) };
}

/// The terms a crafted hand card is made on (TS `craftInHand`'s `terms`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CraftTerms {
    hand_price: HandPrice,
    radiant: bool,
}

/// R77's Craft a Card path: "a fresh, non-Radiant hand card with `costOverride` 0". The ingredients
/// went into it, so they cease to exist here too — #99's are Discovered definitions that were never
/// cards on a board, and for anything else a consumed ingredient is what a fusion means.
/// A full hand burns the result like any other card reaching it (§2.4, R4), and the 0 goes with the
/// hand: §8 #99's "the result costs 0 and goes to your hand" is a price for the card in that hand,
/// the reading `add_to_hand`'s cost riders have (R4), so a burned result is an ordinary graveyard card
/// that R78 would otherwise carry the price for into every later zone (a Reminisce, a Gravedigger).
fn craft_in_hand(
    sink: &mut EngineSink<'_>,
    def: &CardDef,
    player: PlayerId,
    ingredients: &[CardInstance],
    terms: CraftTerms,
) -> CardInstance {
    let mut card = new_instance(sink.state, &def.id, player, Zone::Hand { player });
    // R352: radiant Stitching's result is Radiant as it is made, so it reaches the hand on that face.
    if terms.radiant {
        card.radiant = true;
    }
    // R102, B3.4 rule 4, R443: as `keep_instance`'s.
    carry_instance_data(&mut card, ingredients);
    // ME-CN, R1300: a newly made fused card is Chinese when any ingredient was (a kept instance keeps
    // its own flag, as it keeps its id).
    if ingredients
        .iter()
        .any(|ingredient| ingredient.chinese == Some(true))
    {
        card.chinese = Some(true);
    }
    for ingredient in ingredients {
        crate::zones::cease_to_exist(sink.state, &mut ingredient.clone());
    }
    let _ = crate::draw::add_to_hand(sink, &mut card);
    // TS: `addToHand(sink, card) === "hand"`, which is where the card now stands.
    if let Some(landed) = find_instance_mut(sink.state, &card.id)
        && landed.zone.z() == ZoneName::Hand
        && terms.hand_price == HandPrice::Free
    {
        landed.cost_override = Some(CRAFTED_CARD_COST);
    }
    find_instance(sink.state, &card.id).cloned().unwrap_or(card)
}

/// An argument's card as it stands now (TS held the live object): the state's copy, or — for a card
/// the state no longer holds, an ingredient an earlier fusion consumed or a definition Discovered
/// into no pile — the caller's copy, in no zone, as a card that ceased to exist is (R86).
fn as_it_stands(state: &GameState, card: &CardInstance) -> CardInstance {
    match find_instance(state, &card.id) {
        Some(live) => live.clone(),
        None => CardInstance {
            zone: Zone::Gone { player: card.owner },
            ..card.clone()
        },
    }
}

/// §6.3 Fuse per R77. Returns the fused card — the kept target instance, the kept hand or library
/// card (R470), or the crafted hand card — or `None` when the fusion cannot happen, in which case
/// nothing has changed.
///
/// It does not happen when there are fewer than two ingredients, when a named target is not on the
/// field or a card named `into` is not in a hand or a library, when the kept card is Immutable (R23:
/// an Immutable permanent is never chosen as a Fuse target, and R61 has the trap fire and do
/// nothing), or when the call names neither a kept card nor a hand to craft into. A target the caller did not also list as an ingredient is one anyway, so #85
/// may name the played card and its victim separately.
///
/// An ingredient only ever contributes its definition, so an ingredient that already ceased to exist
/// in an earlier fusion still fuses: that is what lets radiant #85 fuse the played permanent "onto
/// each matching permanent separately, one fusion at a time" (R77), each fusion its own transient
/// definition, without the card having to rebuild the permanent it consumed.
pub fn fuse(sink: &mut EngineSink<'_>, args: FuseArgs) -> Option<CardInstance> {
    // R470: the kept instance — R77's target on the field, or a hand or library card (`into`).
    let target: Option<CardInstance> = args
        .target
        .as_ref()
        .or(args.into.as_ref())
        .map(|card| as_it_stands(sink.state, card));
    let to_hand = args.to_hand;

    let mut ingredients: Vec<CardInstance> = Vec::new();
    for card in args.ingredients.iter().chain(target.iter()) {
        if !ingredients.iter().any(|seen| seen.id == card.id) {
            ingredients.push(as_it_stands(sink.state, card));
        }
    }
    if ingredients.len() < FUSE_MIN_INGREDIENTS {
        return None;
    }

    if let Some(target) = &target {
        let zone = target.zone.z();
        let allowed = if args.target.is_some() {
            zone == ZoneName::Field
        } else {
            zone == ZoneName::Hand || zone == ZoneName::Library
        };
        if !allowed {
            return None;
        }
        if crate::layers::unit_has(sink.state, target, KeywordKind::Immutable) {
            return None;
        }
    } else if to_hand.is_none() {
        return None;
    }

    let state: &GameState = sink.state;
    let defs: Vec<CardDef> = ingredients
        .iter()
        .map(|card| crate::catalog::def_of(Some(state), &card.def_id).clone())
        .collect();
    let target_def: Option<CardDef> = target
        .as_ref()
        .map(|target| crate::catalog::def_of(Some(state), &target.def_id).clone());
    let radiant_ids: IndexSet<String> = args
        .radiant_ingredients
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect();
    let forced: Vec<bool> = ingredients
        .iter()
        .map(|card| radiant_ids.contains(&card.id))
        .collect();
    // R470: read before the kept card becomes the fusion, whose own cost is R77's.
    let kept_cost = match &target {
        Some(target) if args.keep_cost == Some(true) => Some(kept_cost_of(state, target)),
        _ => None,
    };
    let def = build_def(
        state,
        &ingredients,
        &defs,
        target_def.as_ref(),
        &forced,
        kept_cost.as_ref(),
        None,
    );

    // The def is match state; its id names the scripts (R179, SURFACE §6.6), composed now into
    // `state.fused_scripts` as TS registered them as it minted them, and a digest id's list rides on
    // the def (R468).
    sink.state.transient_defs.insert(def.id.clone(), def.clone());
    crate::scripts::sync_fused_scripts(sink.state);

    let result: CardInstance = if let Some(target) = &target {
        let mut result = keep_instance(sink.state, &def, &ingredients, target);
        if let Some(kept_override) = kept_cost.and_then(|kept| kept.override_)
            && let Some(live) = find_instance_mut(sink.state, &result.id)
        {
            live.cost_override = Some(kept_override);
            result = live.clone();
        }
        // R43, R151: the kept card now carries every ingredient's text, a #98 Heroic Power's included —
        // and "one created later rolls when it is created". The ingredient's rolled power ceased to exist
        // with it, and the kept instance's memory is the target's (R77), so without the roll the card
        // would carry "Once per turn, spend X" and no power for as long as it stood. A card that already
        // has its power keeps it (`hero_power::ensure_power`). A crafted card rolls as it reaches the hand.
        let controller = result.controller;
        let _ = crate::prompts::run_start_of_game(sink, &result, controller);
        find_instance(sink.state, &result.id).cloned().unwrap_or(result)
    } else {
        let player = to_hand?;
        craft_in_hand(
            sink,
            &def,
            player,
            &ingredients,
            CraftTerms {
                hand_price: args.hand_price.unwrap_or(HandPrice::Free),
                // R469: with no kept card and every ingredient Radiant, the result is Radiant (Classic+ #43).
                radiant: args.radiant == Some(true) || forced.iter().all(|is_radiant| *is_radiant),
            },
        )
    };

    sink.events.push(GameEvent::Fused {
        instance_ids: ingredients.iter().map(|card| card.id.clone()).collect(),
        result_instance_id: result.id.clone(),
        def_id: def.id.clone(),
    });
    Some(result)
}
