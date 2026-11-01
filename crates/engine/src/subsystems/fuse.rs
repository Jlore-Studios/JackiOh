//! Fuse and Craft a Card (SPEC §6.3 Fuse, R77): the transient definition two or three cards make,
//! and the instance the result lives on (#85 Unlicensed Experimentation, #99 Craft a Card).
//!
//! The result is a *definition*, not an instance: both faces fuse, so Make Radiant keeps working (§5.2),
//! written fresh into `state.transient_defs` as match state surviving a JSON round-trip (§10.1). Scripts
//! are code no JSON can hold, so Rust registers nothing (SURFACE §3, §6.6): they are composed from the
//! ingredients' scripts by `combine_objects`'s rules then and there, and R179's id names the
//! ingredients, so a replay of the same action list runs the same scripts (§9.3).
//!
//! R77 keeps an ingredient's *instance* already on the field, zone, damage and counters intact; only the
//! def id, summed buffs and united keywords change. R468–R470 (B5 E23): digest ids past `FUSED_ID_CAP`,
//! Radiant-face ingredients (`*`), a kept hand/library card (`into`) keeping its cost. Numbers live in
//! `crate::config` (CLAUDE.md rule 9).

use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::{
    CRAFTED_CARD_COST, FUSE_COST_CAP, FUSE_MIN_INGREDIENTS, FUSED_CARDS_CAP, FUSED_ID_CAP, TRIGGER_EXTRA_NONE,
};
use crate::script::{
    ActivationDecl, AttackMod, AttackModArgs, AttackModHook, AuraEntry, AuraHook, CardScripts,
    ConditionContext, ConditionHook, EchoXHook, Effect, EffectApply, EffectContext, EffectPart, EngineSink,
    FlagOrCount, Hook, HookArgs, ModeAllowedHook, PlagueMultiplierHook, PlayAllowedHook, PreviewHook,
    QuestBook, Script, SetStat, SetStatHook, SpellDamageAura, StatMod, StaticFlags, TargetCheck,
    TributeWhenHook, TriggerDef, TriggerExtraHook, TriggerRun, TriggerWhen, TuneMultiplier, WouldCounterHook,
    attack_mod_hook, aura_hook, condition_hook, hook, mode_allowed_hook, play_allowed_hook, read_hook,
    target_check, would_counter_hook,
};
use crate::state::{CardInstance, GameState, Grant, find_instance, find_instance_mut, new_instance};
use crate::wire::{
    AttackHealth, CardCost, CardDef, CardFace, CardType, FusedIngredient, GameEvent, Keyword, KeywordKind,
    PlayerId, Rarity, Selection, SetName, Tag, TargetDecl, Zone, ZoneName, keyword_key,
};

/// The `Script` key of the step table a continuation re-enters (`prompts::RESUME_HOOK`).
const RESUME_KEY: &str = "resume";

/// Keys whose reads return lists, not effects, so a fusion builds every ingredient's list at once:
/// auras, read off the field on every stat read (§10.4); `preview` (R280), the numbers `view_for`
/// shows; `drawLimit` (B5 E3, R457), read on every draw, so the lowest holds; `costAura` and
/// `graveyardPlay` (B5 E15, E11; R455, R454), `heroGuard` and `conditionalKeywords` (B5 E6, E35) —
/// each asked with the fused card's own context, as R196 asks `conditionMet`.
const EAGER_KEYS: &[&str] = &[
    "aura",
    "preview",
    "drawLimit",
    "costAura",
    "graveyardPlay",
    "heroGuard",
    "conditionalKeywords",
];

/// §8's rarity ladder, lowest first: a fusion reports the rarest ingredient's rarity.
const RARITY_ORDER: &[Rarity] = &[
    Rarity::Token,
    Rarity::Common,
    Rarity::Rare,
    Rarity::Epic,
    Rarity::Legendary,
    Rarity::Mythic,
];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FuseArgs {
    /// Every card going into the fusion, in the order the fusing card names them.
    pub ingredients: Vec<CardInstance>,
    /// R77: an ingredient already on the field that the result keeps as its instance (#85 fuses the
    /// permanent the opponent just played onto one of yours).
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
    /// R470: a card in a hand or a library that the result keeps as its instance (Classic+ #31's hand
    /// card, #73's deck cards, #78's Radiant-face pick), staying where it is; its type is the result's
    /// (R77). One not in a hand or library, or Immutable (R23), refuses; `target` wins over both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub into: Option<CardInstance>,
    /// R469: ingredients, by instance id, going in on their Radiant face, lending it and its text to
    /// both fused forms; every other ingredient puts in the face the form is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant_ingredients: Option<Vec<String>>,
    /// R470: "its cost doesn't change" — the kept card keeps its cost as it stood: its own override,
    /// or, for an X-cost or embiggen card with none, that printed cost form on the fused definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_cost: Option<bool>,
}

/// R352: the two prices a fused hand card can have.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HandPrice {
    Free,
    Fused,
}

// The fused definition (R77).

/// R77's unions: one keyword per distinct keyword — except Armor, which every ingredient keeps, its
/// number stacking from every source (§6.1, §10.4) the way the stats beside it do (R102).
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

/// R102's literal union of the ingredients' tags, except `Token` (R1177000): the fused card carries
/// it only when a token itself, so a token fused onto a real card makes a real card, neither labeled
/// Token nor counted as one (`CardDef::is_token`).
fn union_tags(defs: &[CardDef], token: bool) -> Vec<Tag> {
    let mut seen: IndexSet<Tag> = IndexSet::new();
    for def in defs {
        for tag in &def.tags {
            if *tag == Tag::Token && !token {
                continue;
            }
            seen.insert(*tag);
        }
    }
    seen.into_iter().collect()
}

/// R279, R102: the cards a fused text names are every ingredient's, since each face prints both texts
/// whole (`fused_face`), so every name still links. First appearance wins, in ingredient order; `None`
/// when no ingredient names any card, as a catalog card with no reference omits the key.
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
/// with no attack and no health rather than gaining a printed 0/0 (§5, `CardFace`). A static flag
/// that holds only a number (`echoGrant`, `luck`) is summed the same way.
fn sum_defined(values: impl IntoIterator<Item = Option<i32>>) -> Option<i32> {
    let defined: Vec<i32> = values.into_iter().flatten().collect();
    if defined.is_empty() {
        None
    } else {
        Some(defined.iter().sum())
    }
}

/// One ingredient's face as its instance wears it. §7 and R175: a token summoned X/X carries its X as
/// `stats_override` (the Bread Token its "Armor X" as `armor_override`), since neither number can be
/// printed — they ARE its printed face, §10.4's layer 1. A Fuse sums that X/X, and the X replaces only
/// that token's own Armor (#85 fusing a 7/7 onto #18's Bread Token keeps the 7/7's Armor 7).
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
/// field (Field Trap if any ingredient is one)" — a Field Trap is not consumed when it fires and a
/// plain Trap is, so the Field Trap half wins. #85 only ever fuses two cards of the same type; with
/// no target and mixed types (#99 Discovers Units), the first ingredient's type is the fallback.
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

/// The id a transient def gets (R102, R179): `t-<n>`, n from how many transient defs the state holds,
/// so the same action list always produces the same id (§9.3) — then the ingredient ids, `t-<n>:<a>+<b>`.
/// A fused ingredient goes in parentheses, so the id reads back one way (R179).
///
/// The suffix keeps the scripts right: scripts are code built from the id's ingredients, and one server
/// process runs every match (§9.2) folding its log to rebuild it (§9.3), so an id carrying the ingredients
/// names the same scripts in every match that can mint it. R469: a Radiant-face ingredient takes `*`.
/// R468: past `FUSED_ID_CAP` the list becomes `t-<n>:#<digest>`, a pure hash of the spelled-out list
/// (Classic+ #74's nested fusions); the definition keeps the list (`CardDef.ingredients`) to rebuild from.
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

/// An ingredient as a fused id writes it: parentheses when itself fused (R179), `*` on its Radiant face (R469).
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
/// multiply-xorshift mixing over UTF-16 code units, as TS's `charCodeAt` with `Math.imul`/`>>>` as
/// `wrapping_mul`/a `u32` shift (CLAUDE.md rule 4: no crypto, no I/O; it names a list, not a secret).
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
/// minted. Split at `+` outside parentheses (R387's self-exclusion reads this too); a digest id's list
/// comes from its definition in the state (R468).
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

/// R874000: the cards a definition holds: one for a card that is no fusion, and for a fusion its
/// ingredients' (`CardDef.ingredients`, R468), each counted the same way. It stops once past
/// `FUSED_CARDS_CAP`, the most `fuse` makes, so a fusion of fusions is walked no further than that.
pub fn fused_card_count(state: &GameState, def_id: &str) -> usize {
    let Some(specs) = crate::catalog::find_def(Some(state), def_id).and_then(|def| def.ingredients.as_ref())
    else {
        return 1;
    };
    let mut count = 0;
    for spec in specs {
        count += fused_card_count(state, &spec.def_id);
        if count > FUSED_CARDS_CAP {
            break;
        }
    }
    count.max(1)
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
    // A fusion is a real card unless every ingredient was a token: fusing one onto a unit sheds the
    // Token tag (R1177000) and it no longer ceases to exist off the field (R11).
    let token = defs.iter().all(|def| def.token);
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
        tags: union_tags(defs, token),
        rarity: rarest_of(defs),
        printed_rarity: None,
        flavor: None,
        token,
        // R470: a kept X-cost or embiggen card that keeps its cost keeps that printed form.
        cost: kept_cost
            .and_then(|kept| kept.form)
            .unwrap_or_else(|| CardCost::Fixed(fused_cost(state, ingredients))),
        refs,
        params: None,
        deals_damage: None,
        uses_plague: None,
        resource: None,
        loc,
        radiant_fallback: None,
        // R179, R468: the list the id names, kept on the definition so the scripts can be rebuilt from
        // it even when the id is only a digest of it.
        ingredients: Some(specs),
        craft: None,
        snapshot_of: None,
        base: kept_piece_only(
            fused_face(ingredients, defs, false, forced),
            target_def.map(|def| &def.base),
        ),
        radiant: kept_piece_only(
            fused_face(ingredients, defs, true, forced),
            target_def.map(|def| &def.radiant),
        ),
    }
}

/// R1852001: a fusion that keeps a card prints Starship Piece only when the kept card's own face does;
/// keeping no card unites the keyword as every other (R77).
fn kept_piece_only(mut face: CardFace, kept: Option<&CardFace>) -> CardFace {
    if let Some(kept) = kept
        && !kept
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::StarshipPiece)
    {
        face.keywords
            .retain(|keyword| keyword.kind() != KeywordKind::StarshipPiece);
    }
    face
}

/// R470: what "its cost doesn't change" writes, read off the kept card before it becomes the fusion:
/// nothing when it already has a `cost_override`; for an X-cost or embiggen card with no cost hook,
/// that printed form, which the fused definition then prints; otherwise its own cost as it stands
/// (`override_`, R65's printed cost with a hook's computed one). `cost_mod` stays the instance's.
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

// What a fused card remembers of its ingredients: private copies, so this module reads them as it writes them.

/// How much of `PART_KEY`'s path the combined hooks above the running one have used (`PART_DEPTH_KEY`).
fn part_depth(data: &IndexMap<String, Value>) -> usize {
    data.get(crate::work::PART_DEPTH_KEY)
        .and_then(Value::as_u64)
        .and_then(|depth| usize::try_from(depth).ok())
        .unwrap_or(0)
}

// The concatenated scripts (R77).

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

/// An effect's kind is a static name, read by nothing but the debug output.
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

/// A second context reborrowed over the same sink, every fact of the run copied for the caller to change onward.
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

/// §10.4 layer 5: each ingredient's aura, reading "this" as the fused card at the price that ingredient
/// was played for (R102, `as_ingredient`): #46 Suppressive Aura played for 4 and fused onto a Mana Well
/// is still "paid 4: −5/−5".
///
/// An aura's entries may borrow the card read at an ingredient's price — a copy made here — so such an
/// entry carries that copy and asks its aura again for each unit applied to (a pure read: same answer).
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

/// The context ingredient `index`'s text builds and applies with (R102): its place in the fusion on the
/// path above it (`work::PART_KEY`); a re-entry names the whole path from the top, so a question asked
/// two fusions down comes back to its own ingredient (`work::PART_DEPTH_KEY`).
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

/// The context an ingredient's text runs in: its place in the fusion, and the price its card was played
/// for as `embiggened` (R102, `ingredient_paid`) — #59's trigger reads it by that place's path.
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

/// A combined hook: each ingredient's list in turn, as parts (R102, R113). A continuation one text left —
/// the step its prompt re-enters, the delayed effect it scheduled — names that ingredient (`work::PART_KEY`),
/// and comes back to its list alone: one Masochism Mask's "choose one" is that Mask's pick, not every
/// ingredient's that names its step the same.
fn combined_hook(fns: Vec<Option<Hook>>, step: bool) -> Hook {
    let fns = Arc::new(fns);
    hook(move |ctx| {
        let depth = part_depth(&ctx.data);
        // A `resume` step is one continuation of one text, so one naming no part — left for the card as
        // a whole, like the prompt of the power R43 activates once (`hero_power::activate_power`) — comes
        // back to the first ingredient having the step, once, not every one naming it the same (R43, R102).
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

/// One named target predicate across the ingredients defining it (§10.6, R102). Not a hook: a declaration's
/// filter asks it with the candidate for a boolean, in a context with no instance data to route a part by
/// (wrapping it as a fused Cry threw on the first ask). One predicate stays as is; ingredients naming the
/// same one must each admit the candidate — the stricter requirement flags and numbers take to combine.
fn combined_checks(checks: Vec<TargetCheck>) -> TargetCheck {
    if checks.len() == 1
        && let Some(only) = checks.first()
    {
        return only.clone();
    }
    target_check(move |args| checks.iter().all(|check| check(args)))
}

/// A hook-valued key (a Cry, Death, a start/end-of-turn or delayed hook, a resume
/// step): one hook running each in turn, so "both Cry and Death lists run" (R77) — each built only when the
/// one before resolved (`lazy_part`, R102), so a later ingredient reads the board the earlier ones left
/// (#68's "8 if your hero is below 10" after Reno set it to 30, #22's meal after #100 exiled it). An
/// `EAGER_KEYS` key runs each at once; a continuation left comes back to that one alone (`combined_hook`).
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

/// An `EAGER_KEYS` read of `{ state, self, radiant }`: one ingredient's read
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

/// R1878001: a fused card's texts lend no Starship Piece. R1852001 keeps the keyword only from the card a
/// Fuse keeps, whose own lend became its grant as fused (`keep_instance`); an ingredient's would otherwise
/// reach the card fused onto, read at that card's price (Embiggened S #129 onto its own-priced host).
fn without_lent_piece(lend: Option<ReadListHook<Keyword>>) -> Option<ReadListHook<Keyword>> {
    lend.map(|lend| {
        read_hook(move |args| {
            lend(args)
                .into_iter()
                .filter(|keyword| keyword.kind() != KeywordKind::StarshipPiece)
                .collect()
        })
    })
}

/// R1045: a number read of `{ state, self, radiant }` that several ingredients give (`revealEcho`, and
/// R1864004's `tradesPerTurn`): the largest of their answers, as several Radiant Paranoias add one Echo,
/// not their sum.
fn largest_read(values: Vec<Option<crate::script::RevealEchoHook>>) -> Option<crate::script::RevealEchoHook> {
    let defined: Vec<crate::script::RevealEchoHook> = values.into_iter().flatten().collect();
    match defined.len() {
        0 => None,
        1 => defined.into_iter().next(),
        _ => Some(Arc::new(move |args| {
            defined.iter().map(|f| f(args)).max().unwrap_or(0)
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

/// A combined static flag: true when any ingredient set it.
fn any_flag(values: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    let defined: Vec<bool> = values.into_iter().flatten().collect();
    if defined.is_empty() {
        None
    } else {
        Some(defined.iter().any(|flag| *flag))
    }
}

/// R2396000: an ingredient's `afterAttack` as the fused card runs it: a "Whenever this attacks" part
/// whatever became of the attacker, an "After this attacks" part only for a survivor (R2285001).
fn after_attack_part(record: &Script) -> Option<Hook> {
    let run = record.after_attack.clone()?;
    if record.after_attack_even_if_gone {
        return Some(run);
    }
    Some(hook(move |ctx| {
        if crate::combat::after_attack_of(ctx).is_some_and(|facts| !facts.survived) {
            Vec::new()
        } else {
            run(ctx)
        }
    }))
}

/// A number of the combined static flags: the larger, which is the one stricter requirement rather
/// than a doubled one (`staticFlags.tribute`).
fn max_number(values: impl IntoIterator<Item = Option<i32>>) -> Option<i32> {
    values.into_iter().flatten().max()
}

/// R1181002: a number of the combined static flags where the least is the one that acts — two
/// Mountaintop Gardens' premiums, of which the cheaper applies.
fn least_number(values: impl IntoIterator<Item = Option<i32>>) -> Option<i32> {
    values.into_iter().flatten().min()
}

/// The static flags that are an amount of what the text does, not a quality the card has: #79's Echo +1,
/// #38's Combo, #84's hero Armor. R102: a card fused from two such texts carries both,
/// so amounts add — a Twinspell onto a Twinspell grants Echo +2, as two standing apart do (R124) — where
/// a quality (`castOnDraw`, `immutable`) is had once, a requirement (`tribute`) takes the stricter,
/// and a `true` is one.
fn summed_count(values: impl IntoIterator<Item = Option<FlagOrCount>>) -> Option<FlagOrCount> {
    let defined: Vec<FlagOrCount> = values.into_iter().flatten().collect();
    if defined.is_empty() {
        None
    } else {
        Some(FlagOrCount::Count(defined.iter().map(|flag| flag.count()).sum()))
    }
}

/// R1180000 (Meditative+ #9): a fused card's Spell Damage auras add up, as two standing apart do, and
/// reach the opponent too when either ingredient's does.
fn summed_spell_damage_aura(
    values: impl IntoIterator<Item = Option<SpellDamageAura>>,
) -> Option<SpellDamageAura> {
    let defined: Vec<SpellDamageAura> = values.into_iter().flatten().collect();
    if defined.is_empty() {
        return None;
    }
    Some(SpellDamageAura {
        amount: defined.iter().map(|aura| aura.amount).sum(),
        both: defined.iter().any(|aura| aura.both == Some(true)).then_some(true),
    })
}

/// R1160 (Meditative #84 Volatility, ME-TUNEMULT): a fused card's tune multipliers multiply — each
/// direction the product of the ingredients naming it, a direction none names none.
fn multiplied_tune(values: impl IntoIterator<Item = Option<TuneMultiplier>>) -> Option<TuneMultiplier> {
    let defined: Vec<TuneMultiplier> = values.into_iter().flatten().collect();
    if defined.is_empty() {
        return None;
    }
    let product = |read: fn(&TuneMultiplier) -> Option<i32>| {
        let factors: Vec<i32> = defined.iter().filter_map(read).collect();
        if factors.is_empty() {
            None
        } else {
            Some(factors.iter().product())
        }
    };
    Some(TuneMultiplier {
        upgrade: product(|flags| flags.upgrade),
        degrade: product(|flags| flags.degrade),
    })
}

/// `staticFlags`, a nested object, combined key by key by the same rules: a flag is true when any set it,
/// a number takes the larger, an amount adds up, a list is the lists in order. One ingredient's stay as is.
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
                hand_sale: flags(|f| f.hand_sale),
                infinite_reserves: flags(|f| f.infinite_reserves),
                never_defense: flags(|f| f.never_defense),
                echo: numbers(|f| f.echo),
                echo_grant: sum_defined(defined.iter().map(|f| f.echo_grant)),
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
                judges_plays: flags(|f| f.judges_plays),
                hears_emotes: flags(|f| f.hears_emotes),
                exiles_on_damage: flags(|f| f.exiles_on_damage),
                tune_multiplier: multiplied_tune(defined.iter().map(|f| f.tune_multiplier)),
                cant_be_attacked: flags(|f| f.cant_be_attacked),
                attacked_only_from_lane: flags(|f| f.attacked_only_from_lane),
                cant_attack_or_be_attacked: flags(|f| f.cant_attack_or_be_attacked),
                immune_to_tribal_hate: flags(|f| f.immune_to_tribal_hate),
                never_berserk: flags(|f| f.never_berserk),
                heal_to_damage: flags(|f| f.heal_to_damage),
                random_targets: flags(|f| f.random_targets),
                attack_from_hand: flags(|f| f.attack_from_hand),
                bounce_attacker: flags(|f| f.bounce_attacker),
                attack_joiners: flags(|f| f.attack_joiners),
                stack_base: flags(|f| f.stack_base),
                stack_base_buffs: numbers(|f| f.stack_base_buffs),
                tribute_cheap: numbers(|f| f.tribute_cheap),
                lane_multiplier: numbers(|f| f.lane_multiplier),
                credit_line: numbers(|f| f.credit_line),
                credit_instalments: numbers(|f| f.credit_instalments),
                credit_lapses: flags(|f| f.credit_lapses),
                no_natural_mana: flags(|f| f.no_natural_mana),
                feng_shui: flags(|f| f.feng_shui),
                luck: sum_defined(defined.iter().map(|f| f.luck)),
                spell_damage_aura: summed_spell_damage_aura(defined.iter().map(|f| f.spell_damage_aura)),
                doubles_healing: flags(|f| f.doubles_healing),
                answers_targeted_spells: flags(|f| f.answers_targeted_spells),
                embiggen_garden: least_number(defined.iter().map(|f| f.embiggen_garden)),
                // R1839000: a card fused from a Dormant ingredient prints the first ingredient's Dormant.
                slumber: defined.iter().find_map(|f| f.slumber.clone()),
                // R1852000: a fusion the Dormant Starship is in is still its hull.
                starship_hull: any_flag(defined.iter().map(|f| f.starship_hull)),
                immune_to_spell_damage: flags(|f| f.immune_to_spell_damage),
                destroyed_exiled: flags(|f| f.destroyed_exiled),
                enemy_destroyed_exiled: flags(|f| f.enemy_destroyed_exiled),
                reflects_damage: flags(|f| f.reflects_damage),
                reflects_to_attacker: flags(|f| f.reflects_to_attacker),
                // R1849004: the largest cap of the ingredients' redirects, one stricter text.
                own_turn_redirect: defined
                    .iter()
                    .filter_map(|f| f.own_turn_redirect)
                    .max_by_key(|redirect| redirect.cap),
                gauntlet: flags(|f| f.gauntlet),
                units_never_defense: flags(|f| f.units_never_defense),
                reveal_chance: numbers(|f| f.reveal_chance),
                pilots_actions: numbers(|f| f.pilots_actions),
                pilots_turn: flags(|f| f.pilots_turn),
                hints_plays: flags(|f| f.hints_plays),
                lends_equip: flags(|f| f.lends_equip),
            })
        }
    }
}

/// `quests`, a nested object: one ingredient's book stays as is; several combine key by key — quests and
/// rewards in order, and `first`, a string, is the last ingredient's (nothing mixes kinds under one key).
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

/// The ingredients' scripts combined, `records` aligned with the ingredients: every kind
/// of value combines by one rule, which is what keeps this working as `Script` grows new hooks —
/// a hook becomes one hook running each in turn (`combine_hooks`), an aura running each at once; a list
/// becomes the lists in ingredient order; a nested object combines key by key; a flag is true when any
/// set it, a number takes the larger, an amount of what the text does adds up (`summed_count`).
///
/// `cost`, `setStat`, `conditionMet`, `tributeWhen`, `plagueMultiplier`, `wouldCounter`, `attackMods` and
/// `echoX` left each record (`script_record`) to combine on their own, as `cry` and `aura` did for
/// `fused_cry` and `fused_aura`. `targetingDiscards` and `recordsPlayAs` ride no fusion: their readers
/// take a fused card's ingredients one by one, so a fusion carries neither.
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
        fused_into: hooks(|s| s.fused_into.clone(), "fusedInto"),
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
        attack_mods: None,
        triggers: lists_of_triggers(|s| s.triggers.clone()),
        on_play_hook: hooks(|s| s.on_play_hook.clone(), "onPlayHook"),
        hand_triggers: lists_of_triggers(|s| s.hand_triggers.clone()),
        static_flags: combine_static_flags(records),
        targets: concat(records.iter().map(|s| s.targets.clone())),
        modes: concat(records.iter().map(|s| s.modes.clone())),
        mode_allowed: fused_mode_allowed(records),
        play_allowed: fused_play_allowed(records),
        // R865: a fusion keeps its ingredients' declared modes secret when any of them did.
        secret_modes: records.iter().any(|s| s.secret_modes),
        condition_met: None,
        preview: eager_condition(records.iter().map(|s| s.preview.clone()).collect()),
        // R2321000: a fusion keeps every ingredient's counts, in ingredient order, as it previews.
        tallies: eager_condition(records.iter().map(|s| s.tallies.clone()).collect()),
        activations: concat(records.iter().map(|s| s.activations.clone())),
        target_checks: combine_target_checks(records),
        cost_aura: eager_read(records.iter().map(|s| s.cost_aura.clone()).collect()),
        graveyard_play: eager_read(records.iter().map(|s| s.graveyard_play.clone()).collect()),
        reveal_echo: largest_read(records.iter().map(|s| s.reveal_echo.clone()).collect()),
        targeting_discards: None,
        records_play_as: None,
        draw_limit: eager_read(records.iter().map(|s| s.draw_limit.clone()).collect()),
        discard_guard: eager_read(records.iter().map(|s| s.discard_guard.clone()).collect()),
        replacements: concat(records.iter().map(|s| s.replacements.clone())),
        hero_guard: eager_read(records.iter().map(|s| s.hero_guard.clone()).collect()),
        conditional_keywords: without_lent_piece(eager_read(
            records.iter().map(|s| s.conditional_keywords.clone()).collect(),
        )),
        when_attacks: hooks(|s| s.when_attacks.clone(), "whenAttacks"),
        after_attack: hooks(after_attack_part, "afterAttack"),
        // R2396000: a fusion runs its `afterAttack` for a gone attacker when any ingredient's is a
        // "Whenever this attacks"; `after_attack_part` keeps the others to a survivor.
        after_attack_even_if_gone: records.iter().any(|s| s.after_attack_even_if_gone),
        after_attacked: hooks(|s| s.after_attacked.clone(), "afterAttacked"),
        evolve: hooks(|s| s.evolve.clone(), "evolve"),
        ally_attacked: hooks(|s| s.ally_attacked.clone(), "allyAttacked"),
        ally_attack: hooks(|s| s.ally_attack.clone(), "allyAttack"),
        plague_multiplier: None,
        echo_x: None,
        deck_triggers: lists_of_triggers(|s| s.deck_triggers.clone()),
        graveyard_triggers: lists_of_triggers(|s| s.graveyard_triggers.clone()),
        quests: combine_quests(records),
        // R1854000: a list, the ingredients' stages in order.
        stages: concat(records.iter().map(|s| s.stages.clone())),
        tribute_when: None,
        would_counter: None,
        start_of_opponent_turn: hooks(|s| s.start_of_opponent_turn.clone(), "startOfOpponentTurn"),
        turn_hook_extra: None,
        cry_death_extra: None,
        // R1487000, R767000: a fusion's text works face-down when any ingredient's did.
        live_face_down: records.iter().any(|s| s.live_face_down),
        // R1850002, R1850005: every ingredient's exile auras, and the first keyword count printed.
        exile_auras: concat(records.iter().map(|s| s.exile_auras.clone())),
        radiant_at_keywords: records.iter().find_map(|s| s.radiant_at_keywords),
        // R1864004: the most Trades a turn any ingredient gives.
        trades_per_turn: largest_read(records.iter().map(|s| s.trades_per_turn.clone()).collect()),
    }
}

/// R831008: a fusion allows a mode option only when every ingredient's check allows it; a check
/// allows every option it does not know (Meditative #98 Showdown's reads only its `lane-N`).
fn fused_mode_allowed(records: &[Script]) -> Option<ModeAllowedHook> {
    let checks: Vec<ModeAllowedHook> = records
        .iter()
        .filter_map(|record| record.mode_allowed.clone())
        .collect();
    if checks.is_empty() {
        return None;
    }
    Some(mode_allowed_hook(move |state, player, option| {
        checks.iter().all(|check| check(state, player, option))
    }))
}

/// R1851002: a fusion may be played only while every ingredient's play gate holds, as it needs every
/// ingredient's needed pick (R703, whose declarations it carries in order).
fn fused_play_allowed(records: &[Script]) -> Option<PlayAllowedHook> {
    let gates: Vec<PlayAllowedHook> = records
        .iter()
        .filter_map(|record| record.play_allowed.clone())
        .collect();
    if gates.is_empty() {
        return None;
    }
    Some(play_allowed_hook(move |state, player| {
        gates.iter().all(|gate| gate(state, player))
    }))
}

/// MD-D4, R1120: a fusion carries every ingredient's combat-only attack modifiers — attacks added,
/// Poisonous or-ed — each read with its own ingredient as the source, as `fused_aura` reads.
fn fused_attack_mods(faces: &[Face]) -> Option<AttackModHook> {
    let hooks: Vec<Option<AttackModHook>> =
        faces.iter().map(|face| face.script.attack_mods.clone()).collect();
    if hooks.iter().all(Option::is_none) {
        return None;
    }
    Some(attack_mod_hook(move |args| {
        let mut out = AttackMod::default();
        for (index, hook) in hooks.iter().enumerate() {
            let Some(hook) = hook else {
                continue;
            };
            let recorded =
                crate::scripts::ingredients_of(args.self_).is_some_and(|records| records.len() > index);
            if !recorded {
                let got = hook(args);
                out.attack += got.attack;
                out.poisonous = out.poisonous || got.poisonous;
                continue;
            }
            let card = crate::scripts::as_ingredient(args.self_, index);
            let got = hook(AttackModArgs {
                state: args.state,
                self_: &card,
                radiant: args.radiant,
                attacker: args.attacker,
                defender: args.defender,
            });
            out.attack += got.attack;
            out.poisonous = out.poisonous || got.poisonous;
        }
        out
    }))
}

/// One ingredient's script, ready to combine: without its `cost` hook (R77 fixes the fused cost at
/// min(sum, 4); a surviving Ceaseless Void hook would overrule it — R65), and without the members that
/// combine on their own: `setStat`, `conditionMet` (R195's glow answers a boolean, so or-ed — R196),
/// `tributeWhen` and `wouldCounter` (R403, R667), `plagueMultiplier` (R471), `attackMods` (R1120, summed),
/// `echoX` (R802, the larger). Trigger ids are namespaced, so two "turn-end"s stay distinct, each running
/// in its ingredient's place (R102), as do its Activate abilities (R102, R384).
fn script_record(script: &Script, def_id: &str, index: usize) -> Script {
    let namespaced = |list: &[TriggerDef]| -> Vec<TriggerDef> {
        list.iter()
            .map(|trigger| TriggerDef {
                id: format!("{def_id}:{}", trigger.id),
                run: in_trigger_ingredient(trigger, index),
                when: trigger.when.as_ref().map(|when| in_when_ingredient(when, index)),
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
        echo_x: None,
        would_counter: None,
        turn_hook_extra: None,
        cry_death_extra: None,
        triggers: namespaced(&script.triggers),
        hand_triggers: namespaced(&script.hand_triggers),
        deck_triggers: namespaced(&script.deck_triggers),
        graveyard_triggers: namespaced(&script.graveyard_triggers),
        activations: script
            .activations
            .iter()
            .map(|decl| in_activation_ingredient(decl, index))
            .collect(),
        preview: script
            .preview
            .clone()
            .map(|preview| in_preview_ingredient(preview, index)),
        ..script.clone()
    }
}

/// R280, R2476000: an ingredient's `preview` asked as that ingredient's text reads the card
/// (`as_ingredient_text`), so a number it shows from what its text remembered is its own (R102): two
/// Carnivorous Cubes crafted into one card show two meals, each after its own "remember it".
fn in_preview_ingredient(preview: PreviewHook, index: usize) -> PreviewHook {
    condition_hook(move |ctx| {
        let card = as_ingredient_text(ctx.self_, index);
        preview(ConditionContext {
            state: ctx.state,
            self_: &card,
            controller: ctx.controller,
            radiant: ctx.radiant,
            zone: ctx.zone,
            your_turn: ctx.your_turn,
        })
    })
}

/// R102, R384: an ingredient's Activate ability in its place in the fusion. Its list builds and applies
/// there, so it reads its own numbers and what its text remembered; `canActivate` and `has` read the card
/// as that ingredient's text does (`as_ingredient_text`), carrying no context to name a place.
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

/// R102: the card as ingredient `index`'s text reads it, with no context to name its place: at that
/// ingredient's price, with what that text remembered under its own keys (`memory_of_part`).
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

/// R99, R102, R2271000: a trigger's condition, read in its ingredient's place, as its list is built
/// there: the traps' dispatch reads it on a fused trap, `run_queued_trigger` on any other fusion.
fn in_when_ingredient(when: &TriggerWhen, index: usize) -> TriggerWhen {
    let when = when.clone();
    Arc::new(move |ctx, event| {
        let patch = part_data(&ctx.data, index);
        let mut inner = in_place(ctx, &patch);
        when(&mut inner, event)
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

/// R802: a computed Echo X on a fused card takes the larger of the ingredients', each asked about the
/// fused card at its own ingredient's price (`as_ingredient`, R102).
fn fused_echo_x(scripts: &[&Script]) -> Option<EchoXHook> {
    let hooks: Vec<Option<EchoXHook>> = scripts.iter().map(|script| script.echo_x.clone()).collect();
    if hooks.iter().all(Option::is_none) {
        return None;
    }
    Some(read_hook(move |args| {
        hooks
            .iter()
            .enumerate()
            .fold(0, |largest, (index, echo_x)| match echo_x {
                None => largest,
                Some(echo_x) => {
                    let card = crate::scripts::as_ingredient(args.self_, index);
                    largest.max(echo_x(HookArgs {
                        state: args.state,
                        self_: &card,
                        radiant: args.radiant,
                    }))
                }
            })
    }))
}

/// R471: "Plague Counters placed on this are doubled" (Classic #27). Each ingredient's text multiplies
/// what is placed on the fused card — a Pestilent Slime onto a Pestilent Slime quadruples, as back-to-back
/// doublings do — each asked at its own ingredient's price (`as_ingredient`, R102).
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

/// R820: a trigger multiplier (Meditative #9, #10). Multipliers never stack, so a fused card carrying two
/// takes the higher extra, as two on the field do — each asked at its own ingredient's price (R102).
fn fused_highest_extra(
    scripts: &[&Script],
    read: fn(&Script) -> Option<TriggerExtraHook>,
) -> Option<TriggerExtraHook> {
    let hooks: Vec<Option<TriggerExtraHook>> = scripts.iter().map(|script| read(script)).collect();
    if hooks.iter().all(Option::is_none) {
        return None;
    }
    Some(read_hook(move |args| {
        hooks
            .iter()
            .enumerate()
            .fold(TRIGGER_EXTRA_NONE, |highest, (index, extra)| match extra {
                None => highest,
                Some(extra) => {
                    let card = crate::scripts::as_ingredient(args.self_, index);
                    highest.max(extra(HookArgs {
                        state: args.state,
                        self_: &card,
                        radiant: args.radiant,
                    }))
                }
            })
    }))
}

/// One ingredient's face script: a registered card's borrowed from the registry, a fused one's composed.
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

/// R102 concatenates the ingredients' declared targets and modes in ingredient order, and R90 reads that
/// flat list declaration by declaration — so each ingredient's Cry resolves with its OWN slice of the play's
/// choices, not the whole list: handed the whole list, every ingredient read its first slot (a crafted Bigot
/// and Twisted Sorcerer aimed the Sorcerer's 4 at the unit Bigot destroyed). Modes split by mode-declaration
/// counts; targets split by R90's rule over the active declarations (`forModes`) — with the lengths §10.5
/// step 1 read the play with, handed over in `data` (`DECLARATION_SLICES_KEY`), since the board at resolution
/// is not the checked one. Only a Cry with no play behind it measures against the board as it stands.
///
/// Every effect an ingredient's Cry returns is bound to that slice (`ctx.targets` reads at apply time, in
/// the fused card's context), and each Cry is its own part (`lazy_part`), reading the board the earlier
/// ingredients left (R102); a pause inside resumes into the rest of that part, then the parts after (R113).
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

        // Each ingredient's Cry builds as the list reaches it (`lazy_part`), reading the board left by the
        // earlier ingredients (R102); its slice of the choices is fixed now, as step 1 checked them.
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

/// R196: a fusion's yellow glow. Every ingredient's list still runs, so each printed condition still picks
/// its own branch, and the fused card glows when any holds — each asked with the fused card's own context,
/// where only exactly `true` counts, as `conditionActive` counts it.
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
    combined.attack_mods = fused_attack_mods(&faces);
    combined.cry = fused_cry(&faces);
    combined.plague_multiplier = fused_plague_multiplier(&scripts);
    combined.echo_x = fused_echo_x(&scripts);
    combined.turn_hook_extra = fused_highest_extra(&scripts, |script| script.turn_hook_extra.clone());
    combined.cry_death_extra = fused_highest_extra(&scripts, |script| script.cry_death_extra.clone());
    combined
}

// Building a fused card's scripts on lookup (SURFACE §6.6).

/// R179, R468: the ingredients a fused id names — its definition's list when the state holds it, else the
/// id read back — with R77's two-ingredient minimum; `None` for any id no Fuse minted.
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

/// The script an ingredient runs on one form: a fused ingredient's composed here on that form, its fused
/// ingredients first; any other id's from the registry. `seen` stops a malformed id naming itself (no script).
///
/// Only the asked-for form is composed: composing both forms of every ingredient on every lookup doubles
/// the work per link, and a fourteen-deep Fuse of Fuses (golden seed 68) is then 2^14 per `script_of`.
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

/// SURFACE §6.6: a fused (or crafted) definition's two scripts, composed from its ingredients' scripts
/// from the id alone (R179).
/// R468: a digest id's list is read off its definition, so a digest naming another digest finds both.
/// A definition naming no fusion has no script.
pub fn compose_fused_scripts(state: &GameState, def: &CardDef) -> CardScripts {
    match fused_def_specs(state, def) {
        Some(specs) => compose_specs(state, &def.id, &specs, &IndexSet::new()),
        None => CardScripts::default(),
    }
}

/// One face of `compose_fused_scripts`, composed alone: what a running-face lookup needs, at half the work.
pub fn compose_fused_face(state: &GameState, def: &CardDef, radiant: bool) -> Script {
    let Some(specs) = fused_def_specs(state, def) else {
        return Script::default();
    };
    let mut inside: IndexSet<String> = IndexSet::new();
    inside.insert(def.id.clone());
    fused_script(state, &specs, radiant, &inside)
}

/// R2226000: whether `trigger_id` of fused def `def_id` is a Trap/Field Trap ingredient's text, or one
/// fused inside an ingredient (`script_record` namespaces each trigger `<def id>:<own id>`). `false` for
/// a definition naming no fusion.
pub fn is_trap_ingredient_trigger(state: &GameState, def_id: &str, trigger_id: &str) -> bool {
    let Some(specs) = fused_specs(state, def_id) else {
        return false;
    };
    specs.iter().any(|spec| {
        let Some(inner) = trigger_id
            .strip_prefix(spec.def_id.as_str())
            .and_then(|rest| rest.strip_prefix(':'))
        else {
            return false;
        };
        let is_trap = crate::catalog::find_def(Some(state), &spec.def_id)
            .is_some_and(|def| matches!(def.type_, CardType::Trap | CardType::FieldTrap));
        is_trap || is_trap_ingredient_trigger(state, &spec.def_id, inner)
    })
}

/// The ingredients `compose_fused_scripts` composes from: the def's own list (R468) with R77's minimum,
/// else the id's; `None` when it names no fusion.
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

/// R179, R417, R564: the definition `def_id` names in this state — a catalog card's, one already held, or
/// a fused one rebuilt from the id alone (ingredients first), scripts composed on lookup. Ingredients are
/// worn as printed and priced as R65 reads them out of play, targetless (R77's shared type). `None` for a
/// digest (R468), a bare `t-<n>`, an unknown ingredient. C+ #29 brings fused cards back this way.
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

// The result.

/// R77's keep-the-instance path. The fused card *is* the target: only its def id, summed buffs and united
/// granted keywords change; zone, position, damage, exertion, summoned_turn, counters, memory, the radiant
/// and Vanilla flags stay exactly as they were. A token's `stats_override`/`armor_override` were its printed
/// face (§7, R175), already summed into the fused definition by `worn_face`, so they leave with it — kept,
/// §10.4's layer 1 would read them in place of the fused face. And the memory gains the ingredients'
/// prices when not all the kept card's, so each text reads its own (R102).
fn keep_instance(
    state: &mut GameState,
    def: &CardDef,
    ingredients: &[CardInstance],
    kept: &CardInstance,
) -> CardInstance {
    let attack: i32 = ingredients.iter().map(|card| card.buffs.attack).sum();
    let health: i32 = ingredients.iter().map(|card| card.buffs.health).sum();
    let mut all_granted: Vec<Keyword> = ingredients
        .iter()
        .flat_map(|card| card.granted_keywords.iter().cloned())
        .collect();
    // R1878001: a Starship Piece the kept card's own text lent it becomes its grant (fused texts lend none).
    let lent_itself = crate::layers::conditional_keywords_of(state, kept)
        .iter()
        .any(|keyword| keyword.kind() == KeywordKind::StarshipPiece);
    if lent_itself {
        all_granted.push(Keyword::StarshipPiece);
    }
    let mut granted = union_keywords(&all_granted);
    // R1852001: and keeps a granted Starship Piece only when the kept card had it granted itself.
    if !lent_itself
        && !kept
            .granted_keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::StarshipPiece)
    {
        granted.retain(|keyword| keyword.kind() != KeywordKind::StarshipPiece);
    }
    let before = crate::catalog::def_of(Some(&*state), &kept.def_id).clone();

    // R102: the price each ingredient's text reads as its own, recorded before any of them ceases to
    // exist and only when they are not all the kept card's (`scripts::INGREDIENTS_KEY`).
    let record = crate::scripts::ingredient_record(kept, ingredients);
    let at = ingredients.iter().position(|card| card.id == kept.id);
    if let Some(live) = find_instance_mut(state, &kept.id) {
        // R1850000: a copier fused onto runs the fused text — its own printed text among it — and copies no more.
        live.memory
            .shift_remove(crate::subsystems::unit_text::UNIT_TEXT_KEY);
        // R77, R102: the kept card's texts become ingredient `index` of the fusion, and what they remembered
        // moves with them to the path they now run at (`reroot_remembered`).
        if let Some(at) = at {
            crate::work::reroot_remembered(&mut live.memory, at);
        }
        gain_printed_keywords(live, &before, def);
        live.def_id = def.id.clone();
        live.buffs = AttackHealth { attack, health };
        live.granted_keywords = granted;
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
        // R77: the transient definition is the card's new printed face. A token face saved before Set
        // Stats described an ingredient and must not replace the fusion after Flicker or Reborn.
        crate::zones::forget_summoned_token_face(live);
        live.stats_override = None;
        live.armor_override = None;
    }

    // R1086, R1829000: an Equip card standing on its host uncovers it as it ceases to exist, but the
    // host is the kept card and never left: it keeps the turn it arrived.
    let arrived = find_instance(state, &kept.id).and_then(|card| card.summoned_turn);
    // R77 and R86: an ingredient that is not kept ceases to exist — no graveyard, no exile pile, no
    // Death trigger and nothing destroyed — and one that stood on the field has left it (R174).
    for card in ingredients {
        if card.id == kept.id {
            continue;
        }
        crate::zones::cease_to_exist(state, &mut card.clone());
    }
    if let Some(live) = find_instance_mut(state, &kept.id) {
        live.summoned_turn = arrived;
    }
    find_instance(state, &kept.id)
        .cloned()
        .unwrap_or_else(|| kept.clone())
}

/// §5.2: "newly gained keywords apply at once". The fused face can print a keyword the kept card's face did
/// not — Jilliax's Divine Shield onto a K-Pop Fanatic, a Radiant Saintess's Reborn onto a unit back through
/// a granted one — and the card gains it with the new text, so a spent shield or Reborn is up again (as
/// radiant #50's), while one the kept face already printed stays spent. A Vanilla unit prints nothing (§6.3).
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

/// R102, B3.4 rule 4, R443: what a fusion's card carries of its ingredients — tuning summed, enchantments
/// and granted tags united (MD-B15, R923). A Brittle count is a counter, kept only on the kept card with
/// its other counters (R77).
fn carry_instance_data(card: &mut CardInstance, ingredients: &[CardInstance]) {
    let tunings: Vec<_> = ingredients
        .iter()
        .map(|ingredient| ingredient.tuning.clone())
        .collect();
    card.tuning = crate::tuning::sum_tunings(&tunings);
    card.enchantments = crate::enchantments::united_enchantments(ingredients);
    // MD-B15, R923: every ingredient's granted tags united in order — none when no ingredient has any.
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
    // ME-GRANT (MD-D13): every ingredient's granted Death abilities united in order, none when lacking any.
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

/// The terms a crafted hand card is made on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CraftTerms {
    hand_price: HandPrice,
    radiant: bool,
}

/// R77's Craft a Card path: "a fresh, non-Radiant hand card with `costOverride` 0". The ingredients cease
/// to exist here too (#99's are Discovered definitions, never board cards). A full hand burns the result
/// like any other (§2.4, R4) — the 0 prices the card in that hand — so a burned result is an ordinary
/// graveyard card, though R78 would otherwise carry the price into every later zone.
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
    // ME-CN, R1300: a newly made fused card is Chinese when any ingredient was (a kept instance keeps its own).
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
    if let Some(landed) = find_instance_mut(sink.state, &card.id)
        && landed.zone.z() == ZoneName::Hand
        && terms.hand_price == HandPrice::Free
    {
        landed.cost_override = Some(CRAFTED_CARD_COST);
    }
    find_instance(sink.state, &card.id).cloned().unwrap_or(card)
}

/// An argument's card as it stands now: the state's copy, or — for a card the state no longer holds, an
/// ingredient an earlier fusion consumed or a definition Discovered into no pile — the caller's (R86).
fn as_it_stands(state: &GameState, card: &CardInstance) -> CardInstance {
    match find_instance(state, &card.id) {
        Some(live) => live.clone(),
        None => CardInstance {
            zone: Zone::Gone { player: card.owner },
            ..card.clone()
        },
    }
}

/// §6.3 Fuse per R77: the fused card — the kept target, the kept hand/library card (R470), or the
/// crafted hand card — or `None` when the fusion cannot happen, nothing changed.
///
/// Fewer than two ingredients, a target off the field, an `into` outside hand and library, an Immutable
/// kept card (R23; R61: the trap fires and does nothing), or neither a kept card nor a hand named. A target
/// not listed as an ingredient is one anyway, so #85 may name its two cards separately.
///
/// An ingredient gives only its definition, so one an earlier fusion already consumed still fuses — letting
/// radiant #85 fuse the played permanent "onto each matching permanent separately" (R77), each fusion its
/// own transient definition.
pub fn fuse(sink: &mut EngineSink<'_>, args: FuseArgs) -> Option<CardInstance> {
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
    // R2280001: whether the kept card stood ready to animate before the fusion, so a fusion that first
    // gives it both stats and Animated owes it its animation.
    let was = target
        .as_ref()
        .is_some_and(|target| crate::animated::ready_to_animate(state, &target.id));
    // R874000: a fusion past FUSED_CARDS_CAP cards is not made, before any of it is built.
    let held: usize = ingredients
        .iter()
        .map(|card| fused_card_count(state, &card.def_id))
        .sum();
    if held > FUSED_CARDS_CAP {
        return None;
    }
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

    // The def is match state; its id names the scripts (R179, SURFACE §6.6); a digest id's list rides on
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
        // R43, R151: the kept card now carries every ingredient's text, a #98 Heroic Power's included — and
        // "one created later rolls when it is created": the rolled power ceased to exist with its ingredient,
        // and the kept memory is the target's (R77), so without the roll the card would carry no power, only
        // "Once per turn, spend X". A card with its power keeps it (`hero_power::ensure_power`); a crafted
        // card rolls on reaching the hand.
        let controller = result.controller;
        let _ = crate::prompts::run_start_of_game(sink, &result, controller);
        // R781004: and a text that draws for its own card (Keymaster Keenus's Aura) draws for this one.
        if let Some(live) = find_instance(sink.state, &result.id).cloned() {
            let _ = crate::prompts::run_fused_into(sink, &live, controller);
        }
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

    if target.is_some() {
        crate::animated::owe_animation(sink.state, &result.id, was);
    }
    sink.events.push(GameEvent::Fused {
        instance_ids: ingredients.iter().map(|card| card.id.clone()).collect(),
        result_instance_id: result.id.clone(),
        def_id: def.id.clone(),
    });
    Some(result)
}
