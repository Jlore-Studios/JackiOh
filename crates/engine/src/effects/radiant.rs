//! Make Radiant (SPEC §6.3, §5.2, R74): set the instance's `radiant` flag, never a separate card id,
//! so the face swaps on the next read; on the field damage and buffs stay and no Cry re-fires, as
//! setting a flag is not an entry to the field (R22).

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::catalog::def_of;
use crate::effects::targets::{PlayerSpec, TargetSpec, instance_by_id_or_spec, player_of};
use crate::faces::card_type_of;
use crate::rng::Rng;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance, find_instance_mut};
use crate::wire::{CardType, GameEvent, Keyword, OneOrMany, PlayerId, Row, Zone, ZoneName};
use crate::zones::{StandingAt, acting_on};

/// Which card becomes Radiant: the pick the play or a prompt carried (#26 Glowy Jelly Bean's hand
/// pick, R81), `{ of: "self" }` for Radiant Saintess including itself (R22), or an instance id a
/// script captured earlier. All plain data, so a card file stays pure (CLAUDE.md rule 5).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RadiantTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

/// Where a random "becomes Radiant" looks: one zone, or the union of several (#28).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum RadiantZone {
    Hand,
    Library,
    Field,
}

/// R97, R177: whether some player may not read this card where it sits — a hand is its owner's alone
/// and a library nobody's (§9.1), and a face-down trap is read by its controller only (R33, §10.8).
fn hidden_from_someone(ctx: &EffectContext<'_>, card: &CardInstance) -> bool {
    match card.zone {
        Zone::Hand { .. } | Zone::Library { .. } => return true,
        Zone::Field {
            row: Row::Backrow, ..
        } => {}
        _ => return false,
    }
    if card.face_up == Some(true) {
        return false;
    }
    let card_type = card_type_of(ctx.state, card);
    card_type == CardType::Trap || card_type == CardType::FieldTrap
}

/// §5.2, §6.3: a card already Radiant is untouched, but a hidden one is cued anyway: R97 keeps its
/// redacted event in the other seat's stream, so cueing only changed cards would tell which of #29's
/// hand or #26's pick was Radiant already, the face R177 hides (random picks take non-Radiant cards
/// only, R60). `card` is a snapshot; the card is read again by its id and written there.
pub(crate) fn make_radiant(ctx: &mut EffectContext<'_>, card: &CardInstance) -> bool {
    let card = find_instance(ctx.state, &card.id)
        .cloned()
        .unwrap_or_else(|| card.clone());
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
    // R311: a library card's `knownAs` is left as it was: the change was made where nobody reads it.
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

/// §5.2: on the field "newly gained keywords apply at once", so a radiant face printing Divine Shield
/// the base face lacks (#20, #50, #89) puts a spent shield up again, as a shield granted again does
/// (§10.4, `buff::grant_to`). One printed on both faces is not newly gained, and stays spent.
fn gain_printed_shield(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    // A Vanilla unit has no printed text on either face (§6.3), so its face prints no shield.
    if card.zone.z() != ZoneName::Field || card.divine_shield_spent != Some(true) || card.vanilla {
        return;
    }
    let def = def_of(Some(&*ctx.state), &card.def_id);
    let prints = |keywords: &[Keyword]| {
        keywords
            .iter()
            .any(|keyword| matches!(keyword, Keyword::DivineShield))
    };
    let newly = prints(def.radiant.keywords.as_slice()) && !prints(def.base.keywords.as_slice());
    if newly && let Some(live) = find_instance_mut(ctx.state, &card.id) {
        live.divine_shield_spent = None;
    }
}

/// §3.2 and R13: only the top of a Stack pile, or a Unit a carrier holds (R446, R1824000), is on the
/// field, so only it can be picked.
fn field_cards_of(ctx: &EffectContext<'_>, player: PlayerId) -> Vec<CardInstance> {
    acting_on(ctx.state, player, StandingAt::ItsLane)
        .into_iter()
        // R1839000: nor is a Dormant card, which no effect reaches.
        .filter(|card| !crate::slumber::is_slumbering(card))
        .cloned()
        .collect()
}

/// Every card of the named zones, Radiant ones included, in hand order, library top down and lane
/// order, so a draw over them depends only on (seed, cursor). A random pick narrows it (R60, R242).
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

/// §6.3 Make Radiant: one named card, with no effect when it is already Radiant.
pub fn set_radiant(args: RadiantTarget) -> Effect {
    Effect::new("setRadiant", move |ctx| {
        let Some(card) = instance_by_id_or_spec(ctx, args.instance_id.as_deref(), args.target.as_ref())
        else {
            return;
        };
        make_radiant(ctx, &card);
    })
}

/// MD-D6 (§5.2's exception, Meditative #53 Prestige): De-Radiant is Make Radiant reversed, reported by
/// `deradianted`. The layers read the flag, so the face swaps back while damage, buffs and tuning stay,
/// keywords only the Radiant face printed are lost, triggers read the base text and no Cry fires. A
/// fused card returns to its fused base form (R77).
pub fn clear_radiant(args: RadiantTarget) -> Effect {
    Effect::new("clearRadiant", move |ctx| {
        let Some(card) = instance_by_id_or_spec(ctx, args.instance_id.as_deref(), args.target.as_ref())
        else {
            return;
        };
        let card = find_instance(ctx.state, &card.id)
            .cloned()
            .unwrap_or_else(|| card.clone());
        if !card.radiant {
            return;
        }
        // R311: as Make Radiant leaves it, a library card's `knownAs` is left as it was.
        if let Some(live) = find_instance_mut(ctx.state, &card.id) {
            live.radiant = false;
        }
        ctx.events.push(GameEvent::Deradianted {
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            zone: card.zone.clone(),
        });
    })
}

/// `setRadiantRandom`'s arguments: one zone or several, `count` (default 1), `player` (default "self").
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
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
        // R242: a public group's slots are its non-Radiant cards; a hidden card is one whatever its face.
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
        // R242: the events go out group by group (`READERS`), not in the zones' order, where a pick's
        // place beside a public pick would tell which zone, so which hidden card, it was.
        for cards in &groups {
            for card in cards {
                if chosen.contains(&card.id) {
                    make_radiant(ctx, card);
                }
            }
        }
    })
}

/// R242: who may read a pool card where it sits — everyone, only its side's player (their hand, their
/// face-down trap: §9.1, R33), or nobody (a library, §3) — in the order the groups' events go out.
const READERS: [Readers; 3] = [Readers::Everyone, Readers::Owner, Readers::Nobody];

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

/// R242: how many of `count` picks each reader group takes — a uniform draw of `count` slots among all
/// of them, a hidden card being a slot whatever its face, so the split hangs on the groups' sizes alone,
/// never on a face a player may not read (§9.1). With every slot taken, or one group only, nothing is
/// drawn (R129), which leaves a single group's pick exactly R60's.
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
/// chance to become Radiant"): one independent roll per card in `pool_of`'s order (§10.7).
///
/// EVERY card is rolled, a Radiant one included (§8 #42 is not one of R60's picks): skipping them would
/// make the number of draws hang on how many of a library nobody may read were Radiant, and a success
/// is cued either way (R177, §9.1). The pool is read as this effect applies, so an earlier exile in the
/// list is never rolled (§4.5, R59).
pub fn radiant_chance(args: RadiantChanceArgs) -> Effect {
    Effect::new("radiantChance", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let zones: Vec<RadiantZone> = args.zone.as_slice().to_vec();
        // R987: the controller's Luck rolls extra times beside the card's own Lucky.
        let luck = crate::query::luck_of(&*ctx.sink.state, ctx.controller);
        let lucky = (args.lucky.unwrap_or(0) + luck).max(0);
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
