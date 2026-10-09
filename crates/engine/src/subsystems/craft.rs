//! ME-CRAFT: the card crafter of Meditative #17 True Craft a Card (`docs/meditative-set.md`
//! M6 #17, R880–R883). The block table over the engine's real pieces (type and stats; R21's
//! keywords plus Armor and Echo; the `cry`, `death`, `start_of_turn`, `end_of_turn`, Spell-resolution
//! and Trap-trigger hooks; and the effect verbs), each priced in points and in lines of code; the
//! validator; the compiler from a recipe to a transient definition (R882) and its scripts; the
//! seeded presets (R881); and the preview the editor shows (R880).
//!
//! Pure (CLAUDE.md rule 4): the compiler is a total function of the recipe, the presets read only
//! the match rng, and no `HashMap` orders anything — the id hashes JSON whose field order is
//! fixed, so every process mints the same id for one recipe.

use std::sync::Arc;

use serde_json::json;

use crate::catalog::{FUSED_DIGEST_MARK, def_of};
use crate::config::{
    CRAFT_ADJECTIVES, CRAFT_ECHO_POINTS_PER_N, CRAFT_HAT_PRICES, CRAFT_ID_PREFIX, CRAFT_KEYWORD_PRICES,
    CRAFT_LOC_BUDGET, CRAFT_LOC_SKELETON, CRAFT_MAX_COST, CRAFT_MAX_EFFECTS, CRAFT_MAX_N, CRAFT_NOUNS,
    CRAFT_RADIANT_MULTIPLIER, CRAFT_VERB_PRICES, craft_points_budget,
};
use indexmap::IndexMap;

use crate::effects::add_to_hand::add_to_hand;
use crate::effects::{
    ForEachCardArgs, GrantKeywordArgs, TargetSpec, add_random_from_catalog, bounce, buff,
    buff_all_units, chosen_options, damage, damage_all, damage_split, degrade, destroy,
    discover_from_catalog, discard_random, draw, for_each_card, gain_hero_armor, gain_mana,
    grant_keyword, heal, lock_random_zone, summon, upgrade,
};
use crate::params::param;
use crate::prelude::json_as;
use crate::rng::Rng;
use crate::script::{CardScripts, Effect, EffectContext, Hook, Script, StaticFlags, TriggerDef, hook};
use crate::state::find_instance;
use crate::subsystems::fuse::fused_digest;
use crate::wire::{
    CardCost, CardDef, CardFace, CardType, CraftEffect, CraftHat, CraftHatKind, CraftKeyword,
    CraftKeywordPrice, CraftPreview, CraftRecipe, CraftVerb, CraftVerbPrice, GameEvent, GameEventType,
    Keyword, KeywordKind, Param, ParamBetter, Rarity, SetName, TargetDecl, opponent_of,
};
use crate::zones::active_units_of;

// ---------------------------------------------------------------------------
// The id (R882)
// ---------------------------------------------------------------------------

/// A crafted definition's id: `craft:` plus the digest of the recipe's canonical JSON (R179,
/// R468's form), so the id names the recipe and the def keeps the recipe itself (`CardDef.craft`),
/// from which the scripts rebuild in any process, as a Fuse's do.
pub fn craft_id(recipe: &CraftRecipe) -> String {
    let json = serde_json::to_string(recipe).unwrap_or_default();
    format!("{CRAFT_ID_PREFIX}{FUSED_DIGEST_MARK}{}", fused_digest(&json))
}

// ---------------------------------------------------------------------------
// Prices (R883)
// ---------------------------------------------------------------------------

fn verb_price(verb: CraftVerb) -> &'static CraftVerbPrice {
    CRAFT_VERB_PRICES
        .iter()
        .find(|row| row.verb == verb)
        .expect("every CraftVerb has a price row")
}

fn keyword_price(kind: KeywordKind) -> Option<&'static CraftKeywordPrice> {
    CRAFT_KEYWORD_PRICES.iter().find(|row| row.kind == kind)
}

fn hat_price(hat: CraftHatKind) -> &'static crate::wire::CraftHatPrice {
    CRAFT_HAT_PRICES
        .iter()
        .find(|row| row.hat == hat)
        .expect("every CraftHatKind has a price row")
}

/// One verb's points: `points + per_n × N`, halved (rounded up) when the row says so. A
/// `grantKeyword` costs what its keyword costs.
fn effect_points(effect: &CraftEffect) -> i32 {
    if effect.verb == CraftVerb::GrantKeyword {
        return effect
            .keyword
            .and_then(keyword_price)
            .map(|row| row.points + row.per_n * effect.n.unwrap_or(0))
            .unwrap_or(0);
    }
    let row = verb_price(effect.verb);
    let price = row.points + row.per_n * effect.n.unwrap_or(0);
    if row.halve { (price + 1) / 2 } else { price }
}

/// A recipe's points: a Unit's `attack + health − 1`, the keyword prices, `echo × 4`, and each
/// effect's price times its hat's multiplier.
pub fn recipe_points(recipe: &CraftRecipe) -> i32 {
    let mut points = 0;
    if recipe.type_ == CardType::Unit {
        points += recipe.attack + recipe.health - 1;
    }
    for keyword in &recipe.keywords {
        points += keyword_price(keyword.kind)
            .map(|row| row.points + row.per_n * keyword.n.unwrap_or(0))
            .unwrap_or(0);
    }
    points += recipe.echo * CRAFT_ECHO_POINTS_PER_N;
    for hat in &recipe.hats {
        let multiplier = hat_price(hat.hat).multiplier;
        points += multiplier * hat.effects.iter().map(effect_points).sum::<i32>();
    }
    points
}

/// A recipe's lines of code: the frame plus the hat lines plus the verb lines (a targeted verb's
/// row counts its `TargetDecl` line).
pub fn recipe_loc(recipe: &CraftRecipe) -> i32 {
    let mut loc = CRAFT_LOC_SKELETON;
    for hat in &recipe.hats {
        loc += hat_price(hat.hat).lines;
        for effect in &hat.effects {
            loc += verb_price(effect.verb).lines;
        }
    }
    loc
}

// ---------------------------------------------------------------------------
// The validator (R880)
// ---------------------------------------------------------------------------

fn is_reveal_hat(hat: CraftHatKind) -> bool {
    matches!(
        hat,
        CraftHatKind::OpponentPlaysUnit | CraftHatKind::OpponentPlaysSpell | CraftHatKind::OpponentAttacks
    )
}

/// One sentence for each problem `validate_recipe` finds; empty is valid. `cost` is the cost the
/// `number` prompt chose — the recipe's own cost must match it, and both sit in 0 to
/// `CRAFT_MAX_COST`.
pub fn validate_recipe(recipe: &CraftRecipe, cost: i32) -> Result<(), Vec<String>> {
    let mut reasons: Vec<String> = Vec::new();
    if cost < 0 || cost > CRAFT_MAX_COST {
        reasons.push(format!("the cost is {cost}, outside 0 to {CRAFT_MAX_COST}"));
    }
    if cost != recipe.cost {
        reasons.push(format!(
            "the recipe's cost is {}, but the chosen cost is {cost}",
            recipe.cost
        ));
    }
    if !matches!(
        recipe.type_,
        CardType::Unit | CardType::Spell | CardType::FieldSpell | CardType::Trap
    ) {
        reasons.push(format!("a {} cannot be crafted", recipe.type_.as_str()));
    }
    if !CRAFT_ADJECTIVES.contains(&recipe.adjective.as_str()) {
        reasons.push(format!("\"{}\" is not one of the name words", recipe.adjective));
    }
    if !CRAFT_NOUNS.contains(&recipe.noun.as_str()) {
        reasons.push(format!("\"{}\" is not one of the name words", recipe.noun));
    }
    if recipe.type_ == CardType::Unit {
        if recipe.attack < 0 {
            reasons.push(format!("attack is {}, below 0", recipe.attack));
        }
        if recipe.health < 1 {
            reasons.push(format!("health is {}, below 1", recipe.health));
        }
    } else if recipe.attack != 0 || recipe.health != 1 {
        reasons.push("only a Unit has stats".to_string());
    }
    let mut seen_keywords: Vec<KeywordKind> = Vec::new();
    for keyword in &recipe.keywords {
        if seen_keywords.contains(&keyword.kind) {
            reasons.push(format!("{} is listed twice", keyword.kind.as_str()));
        } else {
            seen_keywords.push(keyword.kind);
        }
        if keyword_price(keyword.kind).is_none() {
            reasons.push(format!("{} is not a craft block", keyword.kind.as_str()));
        }
        let spell = keyword_price(keyword.kind).is_some_and(|row| row.spell);
        if recipe.type_ == CardType::Spell {
            if !spell {
                reasons.push(format!(
                    "a Spell takes only Lifesteal and Pierce, not {}",
                    keyword.kind.as_str()
                ));
            }
        } else if recipe.type_ != CardType::Unit {
            reasons.push(format!(
                "a {} takes no keywords, not {}",
                recipe.type_.as_str(),
                keyword.kind.as_str()
            ));
        }
        if keyword.kind == KeywordKind::Armor {
            if keyword.n.is_none_or(|n| n < 1 || n > CRAFT_MAX_N) {
                reasons.push(format!(
                    "Armor is {}, outside 1 to {CRAFT_MAX_N}",
                    keyword.n.unwrap_or(0)
                ));
            }
        } else if keyword.n.is_some() {
            reasons.push(format!("{} takes no number", keyword.kind.as_str()));
        }
    }
    if recipe.echo != 0 && recipe.type_ != CardType::Spell {
        reasons.push("only a Spell echoes".to_string());
    }
    if recipe.echo < 0 || recipe.echo > CRAFT_MAX_N {
        reasons.push(format!("Echo is {}, outside 0 to {CRAFT_MAX_N}", recipe.echo));
    }
    let mut seen_hats: Vec<CraftHatKind> = Vec::new();
    for hat in &recipe.hats {
        if seen_hats.contains(&hat.hat) {
            reasons.push(format!("{} is listed twice", hat_label(hat.hat)));
        } else {
            seen_hats.push(hat.hat);
        }
        if !hat_price(hat.hat).types.contains(&recipe.type_) {
            reasons.push(format!(
                "a {} takes no {}",
                recipe.type_.as_str(),
                hat_label(hat.hat)
            ));
        }
        if hat.effects.is_empty() {
            reasons.push(format!("{} has no effect under it", hat_label(hat.hat)));
        }
    }
    let when_casts = recipe
        .hats
        .iter()
        .filter(|hat| hat.hat == CraftHatKind::WhenCast)
        .count();
    if recipe.type_ == CardType::Spell && when_casts != 1 {
        reasons.push("a Spell has exactly one When cast".to_string());
    }
    if recipe.type_ == CardType::Trap && recipe.hats.iter().filter(|hat| is_reveal_hat(hat.hat)).count() != 1
    {
        reasons.push("a Trap has exactly one reveal hat".to_string());
    }
    if recipe.type_ == CardType::FieldSpell && recipe.hats.is_empty() {
        reasons.push("a Field Spell has at least one hat".to_string());
    }
    let effects: i32 = recipe.hats.iter().map(|hat| hat.effects.len() as i32).sum();
    if effects > CRAFT_MAX_EFFECTS as i32 {
        reasons.push(format!("it has {effects} effects, more than {CRAFT_MAX_EFFECTS}"));
    }
    for hat in &recipe.hats {
        let targeted_ok = hat.hat == CraftHatKind::Cry || hat.hat == CraftHatKind::WhenCast;
        for effect in &hat.effects {
            let row = verb_price(effect.verb);
            if row.takes_n {
                if effect.n.is_none_or(|n| n < 1 || n > CRAFT_MAX_N) {
                    reasons.push(format!(
                        "{} needs a number from 1 to {CRAFT_MAX_N}",
                        verb_name(effect.verb)
                    ));
                }
            } else if effect.n.is_some() {
                reasons.push(format!("{} takes no number", verb_name(effect.verb)));
            }
            if effect.verb == CraftVerb::GrantKeyword {
                match effect.keyword {
                    None => reasons.push("granting a keyword needs one".to_string()),
                    Some(KeywordKind::Armor) => {
                        reasons.push("Armor is a face number, not a granted keyword".to_string())
                    }
                    Some(_) => {}
                }
            } else if effect.keyword.is_some() {
                reasons.push(format!(
                    "only granting a keyword names one, not {}",
                    verb_name(effect.verb)
                ));
            }
            if effect.verb != CraftVerb::AddRandom
                && effect.verb != CraftVerb::Discover
                && effect.card_type.is_some()
            {
                reasons.push(format!(
                    "only adding or discovering a card names a type, not {}",
                    verb_name(effect.verb)
                ));
            }
            if row.targeted && !targeted_ok {
                reasons.push(format!(
                    "{} aims only under Cry or When cast",
                    verb_name(effect.verb)
                ));
            }
        }
    }
    let budget = craft_points_budget(cost.max(0).min(CRAFT_MAX_COST));
    let points = recipe_points(recipe);
    if points > budget {
        reasons.push(format!("it costs {points} points, over the budget of {budget}"));
    }
    let loc = recipe_loc(recipe);
    if loc > CRAFT_LOC_BUDGET {
        reasons.push(format!(
            "it is {loc} lines, over the budget of {CRAFT_LOC_BUDGET}"
        ));
    }
    if reasons.is_empty() { Ok(()) } else { Err(reasons) }
}

/// The verb's words for a refusal sentence.
fn verb_name(verb: CraftVerb) -> &'static str {
    match verb {
        CraftVerb::DamageTarget => "dealing damage to a target",
        CraftVerb::DamageEnemyHero => "dealing damage to the enemy hero",
        CraftVerb::DamageRandomEnemy => "dealing damage to a random enemy",
        CraftVerb::DamageEachEnemy => "dealing damage to each enemy",
        CraftVerb::HealTarget => "healing a target",
        CraftVerb::HealYourHero => "healing your hero",
        CraftVerb::HealYourSide => "healing your side",
        CraftVerb::Draw => "drawing cards",
        CraftVerb::GainMana => "gaining mana",
        CraftVerb::GainArmor => "gaining Armor",
        CraftVerb::SummonRush => "summoning a Rush Token",
        CraftVerb::SummonFelinor => "summoning a Felinor Token",
        CraftVerb::SummonSheep => "summoning a Sheep Token",
        CraftVerb::BuffTarget => "giving a unit +N/+N",
        CraftVerb::BuffYourUnits => "giving your units +N/+N",
        CraftVerb::GrantKeyword => "granting a keyword",
        CraftVerb::Destroy => "destroying a unit",
        CraftVerb::Bounce => "bouncing a unit",
        CraftVerb::AddRandom => "adding a random card",
        CraftVerb::Discover => "discovering a card",
        CraftVerb::OpponentDiscards => "discarding from the opponent",
        CraftVerb::BuffRandomCard => "Buffing a random card",
        CraftVerb::NerfRandomEnemyCard => "Nerfing a random enemy card",
        CraftVerb::LockRandomZone => "locking a random enemy zone",
    }
}

/// The token a summon block names (`CardDef.refs`, R279).
fn summon_token(verb: CraftVerb) -> Option<&'static str> {
    match verb {
        CraftVerb::SummonRush => Some("core-t-rush"),
        CraftVerb::SummonFelinor => Some("core-t-felinor"),
        CraftVerb::SummonSheep => Some("core-t-sheep"),
        _ => None,
    }
}

/// The hat's words for a refusal sentence.
fn hat_label(hat: CraftHatKind) -> &'static str {
    match hat {
        CraftHatKind::Cry => "Cry",
        CraftHatKind::Death => "Death",
        CraftHatKind::StartOfTurn => "Start of turn",
        CraftHatKind::EndOfTurn => "End of turn",
        CraftHatKind::WhenCast => "When cast",
        CraftHatKind::OpponentPlaysUnit => "Reveals when your opponent plays a Unit",
        CraftHatKind::OpponentPlaysSpell => "Reveals when your opponent plays a Spell",
        CraftHatKind::OpponentAttacks => "Reveals when your opponent attacks",
    }
}

// ---------------------------------------------------------------------------
// The definition compiler (R882)
// ---------------------------------------------------------------------------

/// The effects that declare numbers, in hat-then-effect order, with their `bK` keys: the one
/// walk the definition and the scripts number params through, so `param` never misses a key.
fn numbered(recipe: &CraftRecipe) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    for (hi, hat) in recipe.hats.iter().enumerate() {
        for (ei, effect) in hat.effects.iter().enumerate() {
            if verb_price(effect.verb).takes_n {
                out.push((hi, ei, format!("b{}", out.len() + 1)));
            }
        }
    }
    out
}

/// The param key of one effect, if it declares a number.
fn key_of(numbered: &[(usize, usize, String)], hi: usize, ei: usize) -> Option<String> {
    numbered
        .iter()
        .find(|(nhi, nei, _)| *nhi == hi && *nei == ei)
        .map(|(_, _, key)| key.clone())
}

/// One keyword block as the face prints it. Armor and Echo are face numbers, not params: Nerf
/// and Buff never move them (R883).
fn keyword_of(block: &CraftKeyword, doubled: bool) -> Keyword {
    match block.kind {
        KeywordKind::Armor => Keyword::Armor {
            n: block.n.unwrap_or(1) * if doubled { CRAFT_RADIANT_MULTIPLIER } else { 1 },
        },
        KeywordKind::Taunt => Keyword::Taunt,
        KeywordKind::Rush => Keyword::Rush,
        KeywordKind::Charge => Keyword::Charge,
        KeywordKind::FirstStrike => Keyword::FirstStrike,
        KeywordKind::Poisonous => Keyword::Poisonous,
        KeywordKind::Lifesteal => Keyword::Lifesteal,
        KeywordKind::Reborn => Keyword::Reborn,
        KeywordKind::DivineShield => Keyword::DivineShield,
        KeywordKind::Trample => Keyword::Trample,
        KeywordKind::Cleave => Keyword::Cleave,
        KeywordKind::Pierce => Keyword::Pierce,
        KeywordKind::Deft => Keyword::Deft,
        KeywordKind::Windfury => Keyword::Windfury,
        _ => Keyword::Deft,
    }
}

/// A keyword's words on the face ("Armor 2", "First Strike").
fn keyword_text(block: &CraftKeyword, doubled: bool) -> String {
    match block.kind {
        KeywordKind::Armor => format!(
            "Armor {}",
            block.n.unwrap_or(1) * if doubled { CRAFT_RADIANT_MULTIPLIER } else { 1 }
        ),
        kind => kind.as_str().to_string(),
    }
}

/// A face's keywords. A crafted Unit with no number beyond its stats gains the cheapest table
/// keyword it lacks on the Radiant face (R275's keyword-only rule, R882).
fn face_keywords(recipe: &CraftRecipe, radiant: bool) -> Vec<Keyword> {
    let mut keywords: Vec<Keyword> = recipe
        .keywords
        .iter()
        .map(|block| keyword_of(block, radiant))
        .collect();
    if radiant
        && recipe.type_ == CardType::Unit
        && !recipe
            .keywords
            .iter()
            .any(|block| block.kind == KeywordKind::Armor)
        && !recipe.hats.iter().any(|hat| {
            hat.effects
                .iter()
                .any(|effect| verb_price(effect.verb).takes_n && effect.n.is_some())
        })
    {
        let has = |kind: KeywordKind| recipe.keywords.iter().any(|block| block.kind == kind);
        let gained = CRAFT_KEYWORD_PRICES
            .iter()
            .map(|row| row.kind)
            .find(|kind| *kind != KeywordKind::Armor && !has(*kind))
            .unwrap_or(KeywordKind::Armor);
        keywords.push(keyword_of(
            &CraftKeyword {
                kind: gained,
                n: Some(1),
            },
            false,
        ));
    }
    keywords
}

/// One effect's sentence, in the house style of Core #5, #6 and #41. `{key}` (or `{key|one|many}`)
/// is the declared number both faces print; fixed blocks print their words.
fn effect_sentence(effect: &CraftEffect, key: Option<&str>) -> String {
    let key = key.unwrap_or("b0");
    let typed = effect.card_type.map(|type_| type_.as_str().to_string());
    let of = typed.as_deref().unwrap_or("card");
    match effect.verb {
        CraftVerb::DamageTarget => format!("Deal {{{key}}} damage to a target."),
        CraftVerb::DamageEnemyHero => format!("Deal {{{key}}} damage to the enemy hero."),
        CraftVerb::DamageRandomEnemy => format!("Deal {{{key}}} damage to a random enemy."),
        CraftVerb::DamageEachEnemy => format!("Deal {{{key}}} damage to each enemy."),
        CraftVerb::HealTarget => format!("Heal a target {{{key}}}."),
        CraftVerb::HealYourHero => format!("Heal your hero {{{key}}}."),
        CraftVerb::HealYourSide => format!("Heal your hero and your units {{{key}}}."),
        CraftVerb::Draw => format!("Draw {{{key}|card|cards}}."),
        CraftVerb::GainMana => format!("Gain {{{key}}} mana."),
        CraftVerb::GainArmor => format!("Your hero gains {{{key}}} Armor."),
        CraftVerb::SummonRush => "Summon a Rush Token.".to_string(),
        CraftVerb::SummonFelinor => "Summon a Felinor Token.".to_string(),
        CraftVerb::SummonSheep => "Summon a Sheep Token.".to_string(),
        CraftVerb::BuffTarget => format!("Give a unit +{{{key}}}/+{{{key}}}."),
        CraftVerb::BuffYourUnits => format!("Give your units +{{{key}}}/+{{{key}}}."),
        CraftVerb::GrantKeyword => format!(
            "Give a unit {}.",
            effect
                .keyword
                .map(|kind| kind.as_str().to_string())
                .unwrap_or_default()
        ),
        CraftVerb::Destroy => "Destroy a unit.".to_string(),
        CraftVerb::Bounce => "Bounce a unit.".to_string(),
        CraftVerb::AddRandom => format!("Add a random {of} card to your hand."),
        CraftVerb::Discover => format!("Discover a {of} card."),
        CraftVerb::OpponentDiscards => format!("Your opponent discards {{{key}|card|cards}}."),
        CraftVerb::BuffRandomCard => "Buff a random card of yours.".to_string(),
        CraftVerb::NerfRandomEnemyCard => "Nerf a random enemy card.".to_string(),
        CraftVerb::LockRandomZone => "Lock a random enemy zone.".to_string(),
    }
}

/// One hat's line: the label (none for When cast) and its effects' sentences.
fn hat_sentence(recipe: &CraftRecipe, hi: usize, numbered: &[(usize, usize, String)]) -> String {
    let hat = &recipe.hats[hi];
    let effects: Vec<String> = hat
        .effects
        .iter()
        .enumerate()
        .map(|(ei, effect)| effect_sentence(effect, key_of(numbered, hi, ei).as_deref()))
        .collect();
    let label = match hat.hat {
        CraftHatKind::Cry => "Cry: ",
        CraftHatKind::Death => "Death: ",
        CraftHatKind::StartOfTurn => "Start of turn: ",
        CraftHatKind::EndOfTurn => "End of turn: ",
        CraftHatKind::WhenCast => "",
        CraftHatKind::OpponentPlaysUnit => "Reveals when your opponent plays a Unit: ",
        CraftHatKind::OpponentPlaysSpell => "Reveals when your opponent plays a Spell: ",
        CraftHatKind::OpponentAttacks => "Reveals when your opponent attacks: ",
    };
    format!("{label}{}", effects.join(" "))
}

/// A face's text: the keyword line first, then one line per hat — the house style of Core #5,
/// #6 and #41.
fn face_text(recipe: &CraftRecipe, radiant: bool) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut keywords: Vec<String> = recipe
        .keywords
        .iter()
        .map(|block| keyword_text(block, radiant))
        .collect();
    if recipe.type_ == CardType::Spell && recipe.echo > 0 {
        keywords.push(format!(
            "Echo {}",
            recipe.echo * if radiant { CRAFT_RADIANT_MULTIPLIER } else { 1 }
        ));
    }
    if !keywords.is_empty() {
        lines.push(keywords.join(", "));
    }
    let numbered = numbered(recipe);
    for hi in 0..recipe.hats.len() {
        lines.push(hat_sentence(recipe, hi, &numbered));
    }
    lines.join(" ")
}

/// The transient definition a recipe compiles to (R882). Total: it never panics, so the preview
/// and the reducer agree on every recipe, valid or not.
pub fn compile_crafted_def(recipe: &CraftRecipe) -> CardDef {
    let id = craft_id(recipe);
    let numbered = numbered(recipe);
    let mut refs: Vec<String> = Vec::new();
    for hat in &recipe.hats {
        for effect in &hat.effects {
            if let Some(token) = summon_token(effect.verb)
                && !refs.contains(&token.to_string())
            {
                refs.push(token.to_string());
            }
        }
    }
    let params: Vec<Param> = numbered
        .iter()
        .map(|(hi, ei, key)| {
            let n = recipe.hats[*hi].effects[*ei].n.unwrap_or(1).max(1);
            Param {
                key: key.clone(),
                base: n,
                radiant: n * CRAFT_RADIANT_MULTIPLIER,
                better: ParamBetter::Up,
                step: None,
                min: Some(1),
                max: None,
                tuned_on: None,
                power: None,
            }
        })
        .collect();
    let face = |radiant: bool| {
        let (attack, health) = if recipe.type_ == CardType::Unit {
            let times = if radiant { CRAFT_RADIANT_MULTIPLIER } else { 1 };
            (Some(recipe.attack * times), Some(recipe.health * times))
        } else {
            (None, None)
        };
        CardFace {
            type_: None,
            attack,
            health,
            x_stats: None,
            keywords: face_keywords(recipe, radiant),
            text: face_text(recipe, radiant),
        }
    };
    CardDef {
        id: id.clone(),
        index: id,
        name: recipe.name(),
        set: SetName::Meditative,
        type_: recipe.type_,
        tags: Vec::new(),
        rarity: Rarity::Mythic,
        printed_rarity: None,
        token: false,
        cost: CardCost::Fixed(recipe.cost),
        refs: if refs.is_empty() { None } else { Some(refs) },
        params: if params.is_empty() { None } else { Some(params) },
        loc: Some(recipe_loc(recipe)),
        radiant_fallback: None,
        ingredients: None,
        craft: Some(recipe.clone()),
        base: face(false),
        radiant: face(true),
    }
}

// ---------------------------------------------------------------------------
// The script compiler (R882)
// ---------------------------------------------------------------------------

/// One effect ready to compile: the block, its param key and its play-time target declaration.
#[derive(Clone)]
struct CompiledEffect {
    effect: CraftEffect,
    key: Option<String>,
    target: Option<usize>,
}

/// The `TargetDecl` filter a targeted block declares: damage and heal aim at any unit or hero,
/// the rest at another unit (R883).
fn target_filter(verb: CraftVerb) -> serde_json::Value {
    match verb {
        CraftVerb::DamageTarget | CraftVerb::HealTarget => {
            json!({ "side": "any", "of": ["unit", "hero"] })
        }
        _ => json!({ "side": "any", "of": ["unit"], "excludeSelf": true }),
    }
}

/// One compiled effect, with the running face's numbers read now (`param`), as every card script
/// reads its declared numbers when its hook runs.
fn effect_of(ctx: &mut EffectContext<'_>, compiled: &CompiledEffect) -> Vec<Effect> {
    let effect = &compiled.effect;
    let n = compiled.key.as_deref().map(|key| param(&*ctx, key)).unwrap_or(1);
    let chosen = || json!({ "of": "chosen", "index": compiled.target.unwrap_or(0) });
    match effect.verb {
        CraftVerb::DamageTarget => vec![damage(json_as(json!({ "to": chosen(), "amount": n })))],
        CraftVerb::DamageEnemyHero => {
            vec![damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": n }),
            ))]
        }
        CraftVerb::DamageRandomEnemy => {
            vec![damage_split(json_as(
                json!({ "amount": n, "among": "enemies", "perHit": n }),
            ))]
        }
        CraftVerb::DamageEachEnemy => {
            vec![damage_all(json_as(
                json!({ "amount": n, "heroes": true, "side": "enemy" }),
            ))]
        }
        CraftVerb::HealTarget => vec![heal(json_as(json!({ "target": chosen(), "amount": n })))],
        CraftVerb::HealYourHero => {
            vec![heal(json_as(
                json!({ "target": { "of": "selfHero" }, "amount": n }),
            ))]
        }
        CraftVerb::HealYourSide => {
            let controller = ctx.controller;
            let ids: Vec<String> = active_units_of(ctx.state, controller)
                .iter()
                .map(|card| card.id.clone())
                .collect();
            let mut out = vec![heal(json_as(
                json!({ "target": { "of": "selfHero" }, "amount": n }),
            ))];
            out.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(move |_| ids.clone()),
                each: Arc::new(move |id: &str| {
                    heal(json_as(json!({
                        "target": { "of": "instance", "instanceId": id },
                        "amount": n,
                    })))
                }),
            }));
            out
        }
        CraftVerb::Draw => vec![draw(json_as(json!({ "count": n })))],
        CraftVerb::GainMana => vec![gain_mana(json_as(json!({ "amount": n })))],
        CraftVerb::GainArmor => vec![gain_hero_armor(json_as(json!({ "amount": n })))],
        CraftVerb::SummonRush => {
            vec![summon(json_as(json!({ "defId": "core-t-rush" })))]
        }
        CraftVerb::SummonFelinor => {
            vec![summon(json_as(json!({ "defId": "core-t-felinor" })))]
        }
        CraftVerb::SummonSheep => {
            vec![summon(json_as(json!({ "defId": "core-t-sheep" })))]
        }
        CraftVerb::BuffTarget => vec![buff(json_as(
            json!({ "target": chosen(), "attack": n, "health": n }),
        ))],
        CraftVerb::BuffYourUnits => vec![buff_all_units(json_as(
            json!({ "side": "self", "attack": n, "health": n }),
        ))],
        CraftVerb::GrantKeyword => vec![grant_keyword(GrantKeywordArgs {
            target: TargetSpec::Chosen {
                index: compiled.target,
            },
            // Armor never reaches here (the validator refuses it); every other kind grants at 1.
            keyword: Keyword::of_kind(effect.keyword.unwrap_or(KeywordKind::Taunt), 1),
        })],
        CraftVerb::Destroy => vec![destroy(json_as(json!({ "target": chosen() })))],
        CraftVerb::Bounce => vec![bounce(json_as(json!({ "target": chosen() })))],
        CraftVerb::AddRandom => vec![add_random_from_catalog(json_as(json!({
            "query": { "type": effect.card_type },
            "count": 1,
        })))],
        CraftVerb::Discover => vec![discover_from_catalog(json_as(json!({
            "step": DISCOVER_STEP,
            "query": { "type": effect.card_type },
            "prompt": "Discover a card",
        })))],
        CraftVerb::OpponentDiscards => {
            vec![discard_random(json_as(json!({ "count": n, "player": "enemy" })))]
        }
        CraftVerb::BuffRandomCard => vec![upgrade(json_as(
            json!({ "scope": { "zones": ["hand", "field"] }, "random": 1 }),
        ))],
        CraftVerb::NerfRandomEnemyCard => vec![degrade(json_as(json!({
            "scope": { "side": "enemy", "zones": ["hand", "field"] },
            "random": 1,
        })))],
        CraftVerb::LockRandomZone => {
            vec![lock_random_zone(json_as(json!({ "side": "enemy" })))]
        }
    }
}

/// The resume step a crafted Discover answers: it adds the pick, as Core #7's does.
const DISCOVER_STEP: &str = "discover";

/// The trigger a reveal hat becomes, in Core #41's shape: it fires on the opponent's play of the
/// named type, or on their attack (R883).
fn reveal_trigger(hat: CraftHatKind, compiled: Vec<CompiledEffect>) -> TriggerDef {
    let (id, on) = match hat {
        CraftHatKind::OpponentPlaysUnit => ("crafted-reveal-unit", vec![GameEventType::CardPlayed]),
        CraftHatKind::OpponentPlaysSpell => ("crafted-reveal-spell", vec![GameEventType::CardPlayed]),
        _ => ("crafted-reveal-attack", vec![GameEventType::AttackDeclared]),
    };
    TriggerDef::new(id, &on, move |ctx, _event| {
        let mut out = Vec::new();
        for one in &compiled {
            out.extend(effect_of(ctx, one));
        }
        out
    })
    .with_when(move |ctx, event| match event {
        GameEvent::CardPlayed { player, def_id, .. } => {
            if *player == ctx.controller {
                return false;
            }
            let played = def_of(Some(&*ctx.state), def_id).type_;
            (hat == CraftHatKind::OpponentPlaysUnit && played == CardType::Unit)
                || (hat == CraftHatKind::OpponentPlaysSpell && played == CardType::Spell)
        }
        GameEvent::AttackDeclared { attacker_id, .. } => find_instance(ctx.state, attacker_id)
            .is_some_and(|attacker| attacker.controller == opponent_of(ctx.controller)),
        _ => false,
    })
}

/// One face's scripts. Both faces share the compiled hooks — the numbers read `param`, which
/// answers the running face — except Echo, a face number on the static flags (R883).
fn compile_face(recipe: &CraftRecipe, numbered: &[(usize, usize, String)], radiant: bool) -> Script {
    let mut targets: Vec<TargetDecl> = Vec::new();
    let mut compiled_hats: Vec<Vec<CompiledEffect>> = Vec::new();
    for (hi, hat) in recipe.hats.iter().enumerate() {
        let mut compiled: Vec<CompiledEffect> = Vec::new();
        for (ei, effect) in hat.effects.iter().enumerate() {
            let target = if verb_price(effect.verb).targeted {
                let index = targets.len();
                targets.push(TargetDecl::target(1, 1, target_filter(effect.verb)));
                Some(index)
            } else {
                None
            };
            compiled.push(CompiledEffect {
                effect: effect.clone(),
                key: key_of(numbered, hi, ei),
                target,
            });
        }
        compiled_hats.push(compiled);
    }
    let mut cry: Option<Hook> = None;
    let mut death: Option<Hook> = None;
    let mut start_of_turn: Option<Hook> = None;
    let mut end_of_turn: Option<Hook> = None;
    let mut triggers: Vec<TriggerDef> = Vec::new();
    for (hat, compiled) in recipe.hats.iter().map(|hat| hat.hat).zip(compiled_hats) {
        if is_reveal_hat(hat) {
            triggers.push(reveal_trigger(hat, compiled));
            continue;
        }
        let run = move |ctx: &mut EffectContext<'_>| {
            let mut out = Vec::new();
            for one in &compiled {
                out.extend(effect_of(ctx, one));
            }
            out
        };
        match hat {
            CraftHatKind::Cry | CraftHatKind::WhenCast => cry = Some(hook(run)),
            CraftHatKind::Death => death = Some(hook(run)),
            CraftHatKind::StartOfTurn => start_of_turn = Some(hook(run)),
            CraftHatKind::EndOfTurn => end_of_turn = Some(hook(run)),
            _ => {}
        }
    }
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    if recipe.hats.iter().any(|hat| {
        hat.effects
            .iter()
            .any(|effect| effect.verb == CraftVerb::Discover)
    }) {
        resume.insert(
            DISCOVER_STEP,
            hook(|ctx| match chosen_options(ctx).into_iter().next() {
                Some(def_id) => vec![add_to_hand(json_as(json!({ "defId": def_id })))],
                None => vec![],
            }),
        );
    }
    let echo = if recipe.echo > 0 {
        Some(recipe.echo * if radiant { CRAFT_RADIANT_MULTIPLIER } else { 1 })
    } else {
        None
    };
    Script {
        cry,
        death,
        start_of_turn,
        end_of_turn,
        triggers,
        resume,
        static_flags: echo.map(|echo| StaticFlags {
            echo: Some(echo),
            ..StaticFlags::default()
        }),
        targets,
        ..Script::default()
    }
}

/// The two scripts of a recipe, rebuilt from the definition in any process (R882).
pub fn compile_crafted_scripts(recipe: &CraftRecipe) -> CardScripts {
    let numbered = numbered(recipe);
    CardScripts {
        base: compile_face(recipe, &numbered, false),
        radiant: compile_face(recipe, &numbered, true),
    }
}

// ---------------------------------------------------------------------------
// The presets (R881)
// ---------------------------------------------------------------------------

/// A 2-point Unit keyword for the Unit preset, drawn from the palette.
const UNIT_PRESET_KEYWORDS: &[KeywordKind] = &[
    KeywordKind::Taunt,
    KeywordKind::Rush,
    KeywordKind::FirstStrike,
    KeywordKind::Cleave,
    KeywordKind::Trample,
    KeywordKind::Pierce,
    KeywordKind::Lifesteal,
];

/// One verb's price at N, for the presets' affordability draw.
fn price_at(verb: CraftVerb, n: i32) -> i32 {
    effect_points(&CraftEffect {
        verb,
        n: Some(n),
        keyword: None,
        card_type: None,
    })
}

/// The largest N from 1 to `CRAFT_MAX_N` that fits the budget under the hat's multiplier.
fn fitting_n(verb: CraftVerb, multiplier: i32, budget: i32) -> Option<i32> {
    (1..=CRAFT_MAX_N)
        .rev()
        .find(|n| multiplier * price_at(verb, *n) <= budget)
}

/// A recipe's name, drawn from the two word lists.
fn preset_name(rng: &mut Rng) -> (String, String) {
    let adjective = rng.pick(CRAFT_ADJECTIVES).copied().unwrap_or("Pure").to_string();
    let noun = rng.pick(CRAFT_NOUNS).copied().unwrap_or("Closure").to_string();
    (adjective, noun)
}

fn preset_recipe(
    rng: &mut Rng,
    cost: i32,
    type_: CardType,
    keywords: Vec<CraftKeyword>,
    hats: Vec<CraftHat>,
) -> CraftRecipe {
    let (adjective, noun) = preset_name(rng);
    CraftRecipe {
        cost,
        type_,
        adjective,
        noun,
        attack: 0,
        health: 1,
        keywords,
        echo: 0,
        hats,
    }
}

/// The Unit preset: a 2-point keyword when the budget holds one, and all the rest in stats.
fn preset_unit(rng: &mut Rng, cost: i32, budget: i32) -> CraftRecipe {
    let keyword = if budget >= 4 {
        rng.pick(UNIT_PRESET_KEYWORDS)
            .copied()
            .map(|kind| CraftKeyword { kind, n: None })
    } else {
        None
    };
    let spent = keyword
        .as_ref()
        .map(|block| keyword_price(block.kind).map(|row| row.points).unwrap_or(0))
        .unwrap_or(0);
    let rest = (budget - spent).max(0);
    let attack = rest / 2;
    let mut recipe = preset_recipe(
        rng,
        cost,
        CardType::Unit,
        keyword.into_iter().collect(),
        Vec::new(),
    );
    recipe.attack = attack;
    recipe.health = 1 + rest - attack;
    recipe
}

/// A one-hat, one-effect preset around the given verbs, with the largest N that fits.
fn preset_effect(
    rng: &mut Rng,
    cost: i32,
    type_: CardType,
    hat: CraftHatKind,
    verbs: &[CraftVerb],
    budget: i32,
) -> CraftRecipe {
    let multiplier = hat_price(hat).multiplier;
    let verb = verbs
        .iter()
        .copied()
        .filter(|verb| fitting_n(*verb, multiplier, budget).is_some())
        .collect::<Vec<_>>();
    let verb = rng.pick(&verb).copied().unwrap_or(CraftVerb::DamageEnemyHero);
    let n = fitting_n(verb, multiplier, budget);
    preset_recipe(
        rng,
        cost,
        type_,
        Vec::new(),
        vec![CraftHat {
            hat,
            effects: vec![CraftEffect {
                verb,
                n,
                keyword: None,
                card_type: None,
            }],
        }],
    )
}

/// The four seeded presets, in Unit, Spell, Field Spell and Trap order: complete recipes within
/// the budget, drawn from the match rng (R881). Every word, hat and verb is an rng draw, so one
/// seed names one set of four.
pub fn craft_presets(rng: &mut Rng, cost: i32) -> Vec<CraftRecipe> {
    let cost = cost.max(0).min(CRAFT_MAX_COST);
    let budget = craft_points_budget(cost);
    let unit = preset_unit(rng, cost, budget);
    let spell = preset_effect(
        rng,
        cost,
        CardType::Spell,
        CraftHatKind::WhenCast,
        &[
            CraftVerb::DamageEnemyHero,
            CraftVerb::DamageRandomEnemy,
            CraftVerb::HealYourHero,
            CraftVerb::GainArmor,
            CraftVerb::Draw,
            CraftVerb::DamageEachEnemy,
            CraftVerb::BuffYourUnits,
        ],
        budget,
    );
    let field_hat = rng
        .pick(&[CraftHatKind::StartOfTurn, CraftHatKind::EndOfTurn])
        .copied()
        .unwrap_or(CraftHatKind::StartOfTurn);
    let field = preset_effect(
        rng,
        cost,
        CardType::FieldSpell,
        field_hat,
        &[
            CraftVerb::DamageEnemyHero,
            CraftVerb::DamageRandomEnemy,
            CraftVerb::HealYourHero,
            CraftVerb::GainArmor,
        ],
        budget,
    );
    let reveal_hat = rng
        .pick(&[
            CraftHatKind::OpponentPlaysUnit,
            CraftHatKind::OpponentPlaysSpell,
            CraftHatKind::OpponentAttacks,
        ])
        .copied()
        .unwrap_or(CraftHatKind::OpponentPlaysUnit);
    let trap = preset_effect(
        rng,
        cost,
        CardType::Trap,
        reveal_hat,
        &[
            CraftVerb::DamageEnemyHero,
            CraftVerb::DamageRandomEnemy,
            CraftVerb::GainArmor,
            CraftVerb::SummonFelinor,
            CraftVerb::SummonSheep,
            CraftVerb::Draw,
        ],
        budget,
    );
    vec![unit, spell, field, trap]
}

// ---------------------------------------------------------------------------
// The preview (R880)
// ---------------------------------------------------------------------------

/// The engine's own verdict on a recipe at the chosen cost: validity, one sentence per problem,
/// the points and lines used against their budgets, and the card it would make. The editor shows
/// this, so the client decides nothing (CLAUDE.md rule 7).
pub fn craft_preview(recipe: &CraftRecipe, cost: i32) -> CraftPreview {
    CraftPreview {
        valid: validate_recipe(recipe, cost).is_ok(),
        reasons: validate_recipe(recipe, cost).err().unwrap_or_default(),
        points: recipe_points(recipe),
        points_budget: craft_points_budget(cost.max(0).min(CRAFT_MAX_COST)),
        loc: recipe_loc(recipe),
        loc_budget: CRAFT_LOC_BUDGET,
        def: compile_crafted_def(recipe),
    }
}
