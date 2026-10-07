//! Permanent stat buffs and keyword grants. A buff is layer 4 of §10.4, under the auras of layer 5,
//! so it is stored on the instance and survives the aura that came and went; §10.4 computes the
//! totals, so nothing here ever writes a stat. R78 drops both when the card leaves the field.
//!
//! B5 E38 (patch v0.2.0): a buff or a granted keyword may land on a card in a hand or a deck as well —
//! a named card anywhere, or `buffCards` / `grantKeywordCards` over a card scope — and rides it onto
//! the field: a draw, a play and a Recruit move a card without R78's reset, so what it gained in its
//! hand or deck is still on it there (R243 shows it to the hand's owner). It goes when the card leaves
//! the field (R78), or reaches a graveyard or an exile pile from a hand or a deck (R215).
//!
//! Port of `packages/engine/src/effects/buff.ts`. TS wrote through the live instance a target
//! resolved to; Rust writes through `state::find_instance_mut` by the instance's id (the resolvers
//! hand back copies), so the change lands on the card as it stands in the state.

use serde::{Deserialize, Serialize};

use crate::config::RANDOM_KEYWORD_POOL;
use crate::damage::DamageTarget;
use crate::effects::card_scope::{CardScope, Readers, cards_in_card_scope};
use crate::effects::targets::{PlayerSpec, TargetSpec, player_of, resolve_target};
use crate::layers::unit_view;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance, find_instance_mut};
use crate::wire::{GameEvent, Keyword, KeywordKind, PlayerId, has_keyword};
use crate::zones::active_units_of;

/// A stat change in attack, max health, or both (#4 Gary, #43 Friend of Felinors, #63).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuffAmount {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<i32>,
}

/// R21's pool is written as text in config; "Armor 1" is its one numbered entry (§6.1).
const POOL_KEYWORDS: &[(&str, Keyword)] = &[
    ("Taunt", Keyword::Taunt),
    ("Armor 1", Keyword::Armor { n: 1 }),
    ("Rush", Keyword::Rush),
    ("Charge", Keyword::Charge),
    ("First Strike", Keyword::FirstStrike),
    ("Poisonous", Keyword::Poisonous),
    ("Lifesteal", Keyword::Lifesteal),
    ("Reborn", Keyword::Reborn),
    ("Divine Shield", Keyword::DivineShield),
    ("Trample", Keyword::Trample),
    ("Cleave", Keyword::Cleave),
    ("Pierce", Keyword::Pierce),
    ("Windfury", Keyword::Windfury),
    ("Deft", Keyword::Deft),
];

/// R21's pool as keywords, in the pool's order: what a random keyword is drawn from (B3.4's Upgrade too).
pub fn random_pool_keywords() -> Vec<Keyword> {
    RANDOM_KEYWORD_POOL
        .iter()
        .map(|entry| {
            match POOL_KEYWORDS.iter().find(|(key, _)| key == entry) {
                Some((_, keyword)) => keyword.clone(),
                None => panic!("RANDOM_KEYWORD_POOL entry \"{entry}\" has no keyword (R21)"),
            }
        })
        .collect()
}

/// The amount a `{ attack?, health? }` argument carries.
fn amount_of(attack: Option<i32>, health: Option<i32>) -> BuffAmount {
    BuffAmount { attack, health }
}

/// A layer-4 buff on one card. `report` false leaves out the `buffed` event: R440's silent change to a
/// card of a hidden pile a scope reached (`buffCards`).
fn apply_buff(ctx: &mut EffectContext<'_>, unit: &CardInstance, amount: &BuffAmount, report: bool) {
    let attack = amount.attack.unwrap_or(0);
    let health = amount.health.unwrap_or(0);
    if attack == 0 && health == 0 {
        return;
    }
    if let Some(live) = find_instance_mut(ctx.sink.state, &unit.id) {
        live.buffs.attack += attack;
        live.buffs.health += health;
    }
    // The event carries the change this buff made; a unit's totals are read through `unitView`.
    if report {
        ctx.sink.events.push(GameEvent::Buffed {
            instance_id: unit.id.clone(),
            attack,
            health,
        });
    }
}

/// `buff`'s argument: `{ target } & BuffAmount`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BuffArgs {
    pub target: TargetSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<i32>,
}

/// "+X/+Y" on one unit: a permanent layer-4 buff (§10.4).
pub fn buff(args: BuffArgs) -> Effect {
    Effect::new("buff", move |ctx| {
        let Some(DamageTarget::Unit { instance }) = resolve_target(ctx, &args.target) else {
            return;
        };
        apply_buff(ctx, &instance, &amount_of(args.attack, args.health), true);
    })
}

/// `buffAllUnits`' side: a `PlayerSpec`, or "both".
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum BuffAllSide {
    One(PlayerSpec),
    Both(BuffAllBoth),
}

/// The literal "both" of `buffAllUnits`' side.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuffAllBoth {
    #[serde(rename = "both")]
    Both,
}

/// `buffAllUnits`' argument: `{ side?: PlayerSpec | "both" } & BuffAmount`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuffAllUnitsArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<BuffAllSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<i32>,
}

/// "+X/+Y to every unit you control" (#43), in lane order, one `buffed` event each.
pub fn buff_all_units(args: BuffAllUnitsArgs) -> Effect {
    Effect::new("buffAllUnits", move |ctx| {
        let players: Vec<PlayerId> = match args.side {
            Some(BuffAllSide::Both(_)) => {
                vec![player_of(ctx, PlayerSpec::SelfSide), player_of(ctx, PlayerSpec::Enemy)]
            }
            Some(BuffAllSide::One(side)) => vec![player_of(ctx, side)],
            None => vec![player_of(ctx, PlayerSpec::SelfSide)],
        };
        let amount = amount_of(args.attack, args.health);
        for player in players {
            let units: Vec<CardInstance> = active_units_of(ctx.sink.state, player)
                .into_iter().cloned()
                .collect();
            for unit in &units {
                apply_buff(ctx, unit, &amount, true);
            }
        }
    })
}

/// R21, §10.4: the grant itself, on one instance — the set rule, the stacking numbers, the shield and
/// the Reborn coming back.
fn grant_onto(unit: &mut CardInstance, keyword: &Keyword) {
    if keyword.kind() == KeywordKind::DivineShield {
        unit.divine_shield_spent = None;
    }
    if keyword.kind() == KeywordKind::Reborn {
        unit.reborn_spent = None;
    }

    let stacks = matches!(keyword.kind(), KeywordKind::Armor | KeywordKind::Lucky);
    let already = unit.granted_keywords.iter().any(|k| k.kind() == keyword.kind());
    if stacks || !already {
        unit.granted_keywords.push(keyword.clone());
    }
}

/// Add a keyword to `grantedKeywords` (§10.4). Keywords are a set, so a kind the unit was already
/// granted is not stored twice, while Armor and Lucky carry a number and sum across sources (§6.1).
/// A spent Divine Shield and a used Reborn come back when the keyword is granted again (§10.4).
///
/// `unit` is the caller's copy of the card: the grant lands on the card as it stands in the state
/// when it is there (TS's live object), and the copy is brought up to date from it, so a caller that
/// grants again reads what this grant left; a card in no pile is granted on the copy alone, as TS
/// granted on the detached object.
fn grant_to(ctx: &mut EffectContext<'_>, unit: &mut CardInstance, keyword: &Keyword, report: bool) {
    match find_instance_mut(ctx.sink.state, &unit.id) {
        Some(live) => {
            grant_onto(live, keyword);
            *unit = live.clone();
        }
        None => grant_onto(unit, keyword),
    }

    if report {
        ctx.sink.events.push(GameEvent::KeywordGranted {
            instance_id: unit.id.clone(),
            keyword: keyword.clone(),
            lost: None,
        });
    }
}

/// `grantKeyword`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GrantKeywordArgs {
    pub target: TargetSpec,
    pub keyword: Keyword,
}

pub fn grant_keyword(args: GrantKeywordArgs) -> Effect {
    Effect::new("grantKeyword", move |ctx| {
        let Some(DamageTarget::Unit { mut instance }) = resolve_target(ctx, &args.target) else {
            return;
        };
        grant_to(ctx, &mut instance, &args.keyword, true);
    })
}

/// Every pool keyword this unit does not have yet, read through the layers (§10.4, R21). A unit
/// that already has Armor from any source is not offered "Armor 1", as R21 counts by keyword.
fn pool_candidates(ctx: &EffectContext<'_>, unit: &CardInstance) -> Vec<Keyword> {
    let held = unit_view(ctx.sink.state, unit).keywords;
    random_pool_keywords()
        .into_iter()
        .filter(|keyword| !has_keyword(&held, keyword.kind()))
        .collect()
}

/// `grantRandomKeywords`' argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GrantRandomKeywordsArgs {
    pub target: TargetSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
}

/// R21: #63 Plastic Surgery's and #80 Zao Gao's random keywords. Each draw comes from `ctx.rng`,
/// never picks a keyword the unit already has, and never repeats inside one grant; a unit that
/// already holds the whole pool gets nothing.
pub fn grant_random_keywords(args: GrantRandomKeywordsArgs) -> Effect {
    Effect::new("grantRandomKeywords", move |ctx| {
        let Some(DamageTarget::Unit { instance }) = resolve_target(ctx, &args.target) else {
            return;
        };
        // The unit as it stands now (TS read the live object through every draw).
        let mut unit = find_instance(ctx.sink.state, &instance.id)
            .cloned()
            .unwrap_or(instance);
        let count = args.count.unwrap_or(1).max(0);

        for _ in 0..count {
            // Recomputed each draw, so the keyword just granted is out of the pool for the next one.
            let candidates = pool_candidates(ctx, &unit);
            if candidates.is_empty() {
                return;
            }
            let Some(keyword) = ctx.sink.rng.pick(&candidates).cloned() else {
                return;
            };
            grant_to(ctx, &mut unit, &keyword, true);
        }
    })
}

/// `buffCards`' argument: `{ scope: CardScope } & BuffAmount`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BuffCardsArgs {
    pub scope: CardScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<i32>,
}

/// B5 E38: every card a card scope reaches, on the field and in hands and decks, gets "+X/+Y" (Classic+
/// #40 Appropriations' Military and Healthcare: "your Units on the field, in your hand and in your deck
/// get +2X Attack", `{ scope: { zones: ["field", "hand", "library"], types: ["Unit"] }, attack }`). A
/// card on the field acts at once; one in a hand or a deck carries it onto the field. R440: a card of a
/// pile someone may not read (a hand, a deck, a face-down trap) changes silently — a `buffed` there
/// would count the cards the scope's filters let through — and its owner reads its stats off its view
/// (R243); a public card's buff is reported as `buff`'s is.
pub fn buff_cards(args: BuffCardsArgs) -> Effect {
    Effect::new("buffCards", move |ctx| {
        let amount = amount_of(args.attack, args.health);
        let scoped = cards_in_card_scope(ctx, &args.scope, None);
        for entry in &scoped {
            apply_buff(ctx, &entry.card, &amount, entry.readers == Readers::Everyone);
        }
    })
}

/// `grantKeywordCards`' argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GrantKeywordCardsArgs {
    pub scope: CardScope,
    pub keyword: Keyword,
}

/// B5 E38: every card a card scope reaches gains `keyword` (Military's Rush, Healthcare's Armor X,
/// Classic+ #77 Anti-Softlock's Stack and Pierce on every card on the field and in hands and decks),
/// carried onto the field as `buffCards`' stats are and reported the same way (R440).
pub fn grant_keyword_cards(args: GrantKeywordCardsArgs) -> Effect {
    Effect::new("grantKeywordCards", move |ctx| {
        let scoped = cards_in_card_scope(ctx, &args.scope, None);
        for entry in scoped {
            let mut card = entry.card.clone();
            grant_to(ctx, &mut card, &args.keyword, entry.readers == Readers::Everyone);
        }
    })
}
