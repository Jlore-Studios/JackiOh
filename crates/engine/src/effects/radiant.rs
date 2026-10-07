//! Make Radiant (SPEC §6.3, §5.2, R74): set the instance's `radiant` flag, and nothing else. The
//! flag is the whole model — never a separate card id — so in hand or library the stats and text
//! swap on the next read, and on the field the base-stat layer swaps at once while damage and buffs
//! stay and no Cry re-fires, because setting a flag is not an entry to the field (R22).
//!
//! Port of `packages/engine/src/effects/radiant.ts`.

use std::borrow::Borrow;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::catalog::def_of;
use crate::damage::DamageTarget;
use crate::effects::targets::{PlayerSpec, TargetSpec, instance_on_its_stay, player_of, resolve_target};
use crate::faces::card_type_of;
use crate::rng::Rng;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance, find_instance_mut};
use crate::wire::{CardType, GameEvent, Keyword, OneOrMany, PlayerId, Row, Zone, ZoneName};
use crate::zones::{card_at, slots_of};

/// An owned copy of a lookup's answer, whether the lookup lent the card or handed over a copy.
fn owned<C: Borrow<CardInstance>>(card: C) -> CardInstance {
    card.borrow().clone()
}

/// `playerOf(ctx, spec ?? "self")`: an absent spec is the running card's controller.
fn player_or_self(ctx: &EffectContext<'_>, spec: Option<PlayerSpec>) -> PlayerId {
    match spec {
        Some(spec) => player_of(ctx, spec),
        None => ctx.controller,
    }
}

/// Which card becomes Radiant: the pick the play or a prompt carried (#26 Glowy Jelly Bean's hand
/// pick, R81), `{ of: "self" }` for Radiant Saintess including itself (R22), or an instance id a
/// script captured earlier. All plain data, so a card file stays pure (CLAUDE.md rule 5).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RadiantTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

/// Where a random "becomes Radiant" looks: one zone, or the union of several (#28).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum RadiantZone {
    Hand,
    Library,
    Field,
}

fn instance_of(ctx: &EffectContext<'_>, args: &RadiantTarget) -> Option<CardInstance> {
    // R174: a card named by id is aimed at the stay it had when the run began (`instance_on_its_stay`).
    if let Some(instance_id) = &args.instance_id {
        return instance_on_its_stay(ctx, instance_id).map(owned);
    }
    let spec = args.target.clone().unwrap_or(TargetSpec::Chosen { index: None });
    match resolve_target(ctx, &spec) {
        Some(DamageTarget::Unit { instance }) => Some(owned(instance)),
        _ => None,
    }
}

/// R97, R177: whether some player may not read this card where it sits — a hand is its owner's alone
/// and a library nobody's (§9.1), and a face-down trap is read by its controller only (R33, §10.8).
fn hidden_from_someone(ctx: &EffectContext<'_>, card: &CardInstance) -> bool {
    match card.zone {
        Zone::Hand { .. } | Zone::Library { .. } => return true,
        Zone::Field { row: Row::Backrow, .. } => {}
        _ => return false,
    }
    if card.face_up == Some(true) {
        return false;
    }
    let card_type = card_type_of(ctx.state, card);
    card_type == CardType::Trap || card_type == CardType::FieldTrap
}

/// §5.2: the flag is never unset, so a card that is already Radiant is untouched (§6.3). The cue is
/// another matter on a card someone may not read: R97 keeps a hidden card's event in the other seat's
/// stream, redacted but present, so a cue only for the cards that changed would count, for the
/// opponent, which of #29's hand or #26's chosen card were Radiant already — the face R177 hides. So a
/// named Make Radiant on a hidden card is always reported, changed or not; the random picks only ever
/// pick non-Radiant cards (R60), and a public card's face is public either way.
///
/// `card` is a snapshot; the card is read again by its id, as it stands now, and written there.
fn make_radiant(ctx: &mut EffectContext<'_>, card: &CardInstance) -> bool {
    let card = find_instance(ctx.state, &card.id).cloned().unwrap_or_else(|| card.clone());
    if card.radiant {
        if hidden_from_someone(ctx, &card) {
            ctx.events.push(GameEvent::RadiantSet {
                instance_id: card.id.clone(),
                def_id: card.def_id.clone(),
                zone: card.zone.clone(),
            });
        }
        return false;
    }
    // R311: a library card's `knownAs` is left as it was, so its owner's list keeps showing the face
    // it went in with — the change was made where nobody reads it.
    if let Some(live) = find_instance_mut(ctx.state, &card.id) {
        live.radiant = true;
    }
    gain_printed_shield(ctx, &card);
    ctx.events.push(GameEvent::RadiantSet {
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        zone: card.zone.clone(),
    });
    true
}

/// §5.2: on the field "newly gained keywords apply at once". A radiant face that prints Divine Shield
/// where the base face does not (#20, #50, #89) gives the unit a shield it did not have, so one an
/// earlier, granted Divine Shield spent is up again — the same as a shield granted again (§10.4,
/// `buff::grant_to`). A shield printed on both faces is not newly gained, and stays spent.
fn gain_printed_shield(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    // A Vanilla unit has no printed text on either face (§6.3), so its face prints no shield.
    if card.zone.z() != ZoneName::Field || card.divine_shield_spent != Some(true) || card.vanilla {
        return;
    }
    let def = def_of(ctx.state, &card.def_id);
    let prints = |keywords: &[Keyword]| keywords.iter().any(|keyword| matches!(keyword, Keyword::DivineShield));
    let newly = prints(def.radiant.keywords.as_slice()) && !prints(def.base.keywords.as_slice());
    if newly && let Some(live) = find_instance_mut(ctx.state, &card.id) {
        live.divine_shield_spent = None;
    }
}

/// §3.2 and R13: only the top of a Stack pile is on the field, so only it can be picked.
fn field_cards_of(ctx: &EffectContext<'_>, player: PlayerId) -> Vec<CardInstance> {
    let mut out = Vec::new();
    for row in [Row::Units, Row::Backrow] {
        for slot in slots_of(player, row) {
            if let Some(card) = card_at(ctx.state, &slot) {
                out.push(owned(card));
            }
        }
    }
    out
}

/// Every card of the named zones, Radiant ones included, in hand order, library top down and lane
/// order, so a draw over them depends only on (seed, cursor). A random pick narrows it to the
/// non-Radiant cards group by group (R60, R242); a per-card roll (#42) takes every one
/// (`radiant_chance`).
fn pool_of(ctx: &EffectContext<'_>, player: PlayerId, zones: &[RadiantZone]) -> Vec<CardInstance> {
    let side = &ctx.state.players[player];
    let mut seen: IndexSet<String> = IndexSet::new();
    let mut pool: Vec<CardInstance> = Vec::new();
    for zone in zones {
        let cards: Vec<CardInstance> = match zone {
            RadiantZone::Hand => side.hand.clone(),
            RadiantZone::Library => side.library.clone(),
            RadiantZone::Field => field_cards_of(ctx, player),
        };
        for card in cards {
            if seen.contains(&card.id) {
                continue;
            }
            seen.insert(card.id.clone());
            pool.push(card);
        }
    }
    pool
}

/// `setRadiant`'s argument is a `RadiantTarget` (default the first chosen selection).
pub type SetRadiantArgs = RadiantTarget;

/// §6.3 Make Radiant: one named card, with no effect when it is already Radiant.
pub fn set_radiant(args: RadiantTarget) -> Effect {
    Effect::new("setRadiant", move |ctx| {
        let Some(card) = instance_of(ctx, &args) else {
            return;
        };
        make_radiant(ctx, &card);
    })
}

/// `setRadiantRandom`'s arguments: one zone or several, `count` (default 1), `player` (default "self").
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetRadiantRandomArgs {
    pub zones: OneOrMany<RadiantZone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// A random "becomes Radiant" (#23, #27, #28, #93 grade B): `count` different cards drawn uniformly
/// from the non-Radiant cards of the named zones, all of them when fewer exist, and nothing at all
/// when none are left (R60) — split by who may read each card when the zones mix them (R242).
pub fn set_radiant_random(args: SetRadiantRandomArgs) -> Effect {
    Effect::new("setRadiantRandom", move |ctx| {
        let player = player_or_self(ctx, args.player);
        let zones: Vec<RadiantZone> = args.zones.as_slice().to_vec();
        let count = args.count.unwrap_or(1).max(0) as usize;
        // The zones' cards in their own order, read before anything changes, split by who reads them.
        let every_card = pool_of(ctx, player, &zones);
        let mut groups: Vec<Vec<CardInstance>> = Vec::new();
        for reader in READERS {
            let mut group = Vec::new();
            for card in &every_card {
                if readers_of(ctx, card) == reader {
                    group.push(card.clone());
                }
            }
            groups.push(group);
        }
        // R242: a public card's face is public, so only its non-Radiant cards are slots; a hidden
        // card is a slot whatever its face, since whether it is Radiant is what the reader may not see.
        let sizes: Vec<usize> = groups
            .iter()
            .enumerate()
            .map(|(at, cards)| {
                if READERS[at] == Readers::Everyone {
                    cards.iter().filter(|card| !card.radiant).count()
                } else {
                    cards.len()
                }
            })
            .collect();
        let quotas = split_picks(ctx.sink.rng, &sizes, count);

        let mut chosen: IndexSet<String> = IndexSet::new();
        for (at, cards) in groups.iter().enumerate() {
            let quota = quotas.get(at).copied().unwrap_or(0);
            if quota == 0 {
                continue;
            }
            // R60 within the group: its non-Radiant cards, uniformly; R129: nothing drawn when none is left.
            let fresh: Vec<CardInstance> = cards.iter().filter(|card| !card.radiant).cloned().collect();
            let picks: Vec<CardInstance> = if fresh.is_empty() {
                Vec::new()
            } else {
                ctx.sink.rng.shuffle(&fresh).into_iter().take(quota).collect()
            };
            for card in &picks {
                chosen.insert(card.id.clone());
            }
            // R177: the picks the group could not make are cued on its Radiant cards, in its own order,
            // so a hidden group's cues always number its share of the pick.
            for card in cards.iter().filter(|held| held.radiant).take(quota - picks.len()) {
                chosen.insert(card.id.clone());
            }
        }
        // R242: the events go out group by group — the public cards', the owner's hidden cards', then
        // the library's — each in the zones' own order (hand order, lane order, the library top down).
        // In the zones' order a pick's place beside a public pick would say which zone it was in, and
        // so which hidden card was base-face: the hand's comes before a unit's, a face-down trap's after.
        for cards in &groups {
            for card in cards {
                if chosen.contains(&card.id) {
                    make_radiant(ctx, card);
                }
            }
        }
    })
}

/// R242: who may read a card of a random pick's pool where it sits — everyone (a unit, a face-up
/// backrow card), only the player whose side it is on (their hand, their face-down trap: §9.1, R33),
/// or nobody (a library, §3). The groups are listed in the order their events go out.
const READERS: [Readers; 3] = [Readers::Everyone, Readers::Owner, Readers::Nobody];

/// `(typeof READERS)[number]`: "everyone" | "owner" | "nobody". Private, as TS's was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Readers {
    Everyone,
    Owner,
    Nobody,
}

fn readers_of(ctx: &EffectContext<'_>, card: &CardInstance) -> Readers {
    if card.zone.z() == ZoneName::Library {
        return Readers::Nobody;
    }
    if hidden_from_someone(ctx, card) {
        Readers::Owner
    } else {
        Readers::Everyone
    }
}

/// R242: how many of `count` picks each reader group takes — a uniform draw of `count` different
/// slots among all of them, which is how a uniform pick of `count` cards over the whole pool falls,
/// except that a hidden card is a slot whatever its face. So the chance that a public card is picked,
/// and how many picks land among each player's unread cards, hang on the groups' sizes alone, which
/// both players can count: never on a face a player may not read (§9.1). When every slot is taken, or
/// the pool is one group only, there is nothing random to decide and nothing is drawn (R129) — which
/// leaves a single group's pick exactly R60's.
fn split_picks(rng: &mut Rng, sizes: &[usize], count: usize) -> Vec<usize> {
    let total: usize = sizes.iter().sum();
    let groups = sizes.iter().filter(|size| **size > 0).count();
    if total <= count || groups <= 1 {
        return sizes.iter().map(|size| (*size).min(count)).collect();
    }
    let slots: Vec<usize> = sizes
        .iter()
        .enumerate()
        .flat_map(|(at, size)| std::iter::repeat_n(at, *size))
        .collect();
    let mut quotas: Vec<usize> = vec![0; sizes.len()];
    for at in rng.shuffle(&slots).into_iter().take(count) {
        quotas[at] += 1;
    }
    quotas
}

/// §6.1's Lucky X keeps "the best"; for a chance roll that is a success beating a failure (R32).
fn keep_success(a: bool, b: bool) -> bool {
    a || b
}

/// `radiantChance`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RadiantChanceArgs {
    /// Which zones are rolled. Singular to match the card files; `set_radiant_random` above spells the
    /// same argument `zones`, and the inconsistency is reported rather than papered over by accepting
    /// both names here.
    pub zone: OneOrMany<RadiantZone>,
    /// The per-card probability, as `rng.chance` reads it: 0.3 for "30%".
    pub chance: f64,
    /// §6.1 Lucky X: this many extra rolls per card, keeping the success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lucky: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// A per-card chance rather than a pick of N (#42 Eugenics: "each remaining library card has a 30%
/// chance to become Radiant", radiant "Lucky 1 at 40%"). One INDEPENDENT `rng.chance` roll per
/// card in the named zones, in `pool_of`'s order — hand order, library top down, lane order — so the
/// draws depend only on (seed, cursor) and nothing else (§10.7).
///
/// EVERY card is rolled, a Radiant one included. §8 #42 rolls "each remaining library card", and it
/// is not one of R60's random picks, which choose among the non-Radiant cards: skipping the Radiant
/// ones made the number of draws, and so every later draw, hang on how many of a library nobody may
/// read were Radiant already, and a success on one is cued like any other (R177), so the cues cannot
/// count them either (§9.1). A success on a card that is already Radiant changes nothing (§6.3).
/// `lucky: n` is §6.1's Lucky X — n extra rolls per card, keeping the success — so the draw count is
/// (n + 1) per card, which is what makes "Lucky 1 at 40%" two rolls a card.
///
/// ORDER WITHIN AN EFFECT LIST. #42 exiles 8 cards first and then rolls over what is LEFT. The pool
/// is read here, when this effect applies, straight off the live library, so an exile earlier in the
/// same list has already taken its cards out and they are never rolled — §4.5 and R59 put the state
/// check after the whole list, never between two of its effects.
pub fn radiant_chance(args: RadiantChanceArgs) -> Effect {
    Effect::new("radiantChance", move |ctx| {
        let player = player_or_self(ctx, args.player);
        let zones: Vec<RadiantZone> = args.zone.as_slice().to_vec();
        let lucky = args.lucky.unwrap_or(0).max(0);
        let chance = args.chance;

        // The pool is a snapshot taken before any roll, so every card gets exactly its own rolls.
        for card in pool_of(ctx, player, &zones) {
            let hit = if lucky == 0 {
                ctx.sink.rng.chance(chance)
            } else {
                ctx.sink.rng.lucky(lucky, |rng| rng.chance(chance), keep_success)
            };
            if hit {
                make_radiant(ctx, &card);
            }
        }
    })
}
