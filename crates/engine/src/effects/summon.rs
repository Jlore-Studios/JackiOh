//! Summon, Recruit and "fill your board": every way a card reaches the field without being played
//! (SPEC §6.3 Summon and Recruit, §3.2, §7, R64). A summon fires no Cry; `play` and `cast` do (R1).
//!
//! Three of the five verbs are a summon with one extra clause, so each is written as the same
//! placement path with that clause bolted on rather than as a second placement: `summon_copy` is
//! §10.7's copy semantics (R57), `summon_random` is §10.7's `catalog::query` pool (§5.1), and
//! `random_keywords` on `summon` is R21's roll (#80). Placement, the `summoned` event, the face-down
//! Trap and every fizzle stay in `zone_for`/`summon_onto` for all of them.
//!
//! Port of `packages/engine/src/effects/summon.ts`. TS handed the live instance from function to
//! function and wrote through it; here a card travels as an owned copy while it is off the board,
//! and once it is placed every write goes to the card in the state, found by its id.

use std::borrow::Borrow;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::animated::animate_on_entry;
use crate::catalog::{CatalogQueryArgs, def_of, excluding_def_id, pick_generated, query};
use crate::damage::DamageTarget;
use crate::effects::buff::grant_random_keywords;
use crate::effects::targets::{PlayerSpec, TargetSpec, instance_of, player_of, resolve_target};
use crate::faces::card_type_of;
use crate::mana::{effective_cost, is_x_cost};
use crate::prelude::json_as;
use crate::prompts::run_start_of_game;
use crate::resolve::lazy_part;
use crate::script::{Effect, EffectContext, EffectPart};
use crate::state::{CardInstance, find_instance, find_instance_mut, new_instance};
use crate::stays::exit_mark;
use crate::wire::{
    AttackHealth, CardType, CostRange, Enchantment, GameEvent, OneOrMany, PlayerId, Row, Tag, Zone, ZoneName,
};
use crate::zones::{
    PlaceOptions, ZoneSlot, fill_board_zones, first_entry_zone, fresh_face_down_id, is_empty, is_reserved,
    is_unit_token, lands_face_down, place_on_field, remove_from_any_zone, row_size,
};

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

/// TS `{ defId, radiant }`, the partial instance `faces.cardTypeOf` reads a definition's running face
/// from before any card exists: a scratch instance numbered from a counter of its own, so the
/// state's ids are untouched, with no zone on the field.
fn face_probe(def_id: &str, player: PlayerId, radiant: bool) -> CardInstance {
    let mut scratch: u32 = 0;
    let mut probe = new_instance(&mut scratch, def_id, player, Zone::Resolving { player });
    probe.radiant = radiant;
    probe
}

/// §7: the stats a token is summoned with instead of its printed ones (Bread Token, Call to Chaos).
pub type StatsOverride = AttackHealth;

/// Where and how a summoned card lands.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SummonPlacement {
    /// Who controls the summoned card; its owner too when the card is created here (R12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    /// A named lane ("this lane", Reborn's zone); it fails when that zone is occupied or Locked (R47).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<i32>,
    /// §3.2: a Stack card may enter an occupied unit zone and becomes the top of the pile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    /// §7: the Bread Token's "Armor X", carried beside `stats_override` for the same reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats_override: Option<StatsOverride>,
}

/// `summon`'s arguments (TS `SummonPlacement & { defId?; instance?; randomKeywords? }`, the
/// placement's keys beside the rest, as TS writes the literal).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SummonArgs {
    /// Who controls the summoned card; its owner too when the card is created here (R12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    /// A named lane ("this lane", Reborn's zone); it fails when that zone is occupied or Locked (R47).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<i32>,
    /// §3.2: a Stack card may enter an occupied unit zone and becomes the top of the pile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    /// §7: the Bread Token's "Armor X", carried beside `stats_override` for the same reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats_override: Option<StatsOverride>,
    /// Create a fresh card of this definition, a token included (§7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_id: Option<String>,
    /// Or move a card that already exists onto the field (from a hand, library, GY or exile).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<TargetSpec>,
    /// R21: "each with N random keywords" (#80 Zao Gao). The roll belongs to the summon rather than to
    /// a second effect because nothing else can name a card that was just created: `summon` returns
    /// an `Effect`, not an instance, and `TargetSpec` has no "last summoned" case. Rolling it here
    /// also keeps the rng draws adjacent to the summon they belong to, which is what a replay folds
    /// (§9.3), and removes the fizzle hazard of a follow-up effect aimed at a card that never landed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random_keywords: Option<i32>,
}

impl SummonArgs {
    /// The `SummonPlacement` half of these arguments.
    pub fn placement(&self) -> SummonPlacement {
        SummonPlacement {
            player: self.player,
            lane: self.lane,
            stack: self.stack,
            radiant: self.radiant,
            armor_override: self.armor_override,
            stats_override: self.stats_override,
        }
    }
}

/// §5.1: the row a card type lives in, or `None` for a Spell, which is never summoned. B2.7: a card's
/// type is its running face's (`faces::card_type_of`), a pool's definition its base face's.
fn row_of(card_type: CardType) -> Option<Row> {
    match card_type {
        CardType::Unit => Some(Row::Units),
        CardType::Spell => None,
        _ => Some(Row::Backrow),
    }
}

/// §6.3 Recruit: Unit, Field Spell, Trap and Field Trap are the permanents.
fn is_permanent_type(card_type: CardType) -> bool {
    card_type != CardType::Spell
}

/// What `place_on_field` will accept, checked before the card leaves the zone it is in (§3.2). R688: a
/// named summon enters a Locked zone — only plays refuse one — so this checks the reservation alone.
fn can_place(ctx: &EffectContext<'_>, slot: &ZoneSlot, stack: bool) -> bool {
    if is_reserved(ctx.state, slot) {
        return false;
    }
    if is_empty(ctx.state, slot) {
        return true;
    }
    slot.row == Row::Units && stack
}

/// R64 with no lane named, the named lane otherwise; `None` when the summon fizzles (§3.2).
fn zone_for(ctx: &EffectContext<'_>, player: PlayerId, row: Row, at: &SummonPlacement) -> Option<ZoneSlot> {
    let Some(lane) = at.lane else {
        return first_entry_zone(ctx.state, player, row);
    };
    if lane < 1 || lane > row_size(row) {
        return None;
    }
    let slot = ZoneSlot { player, row, lane };
    if can_place(ctx, &slot, at.stack == Some(true)) {
        Some(slot)
    } else {
        None
    }
}

/// The body every summon shares: put the card in the zone, apply the §7 stat override, leave a Trap
/// face-down while a Field Spell is public (§3.2, R33), and emit `summoned`.
///
/// `card` is the caller's copy, off the board; on success it is the card as it stands placed.
fn summon_onto(
    ctx: &mut EffectContext<'_>,
    card: &mut CardInstance,
    slot: &ZoneSlot,
    at: &SummonPlacement,
    former_id: Option<String>,
) -> bool {
    if !place_on_field(
        ctx.state,
        card,
        slot,
        PlaceOptions {
            stack: Some(at.stack == Some(true)),
        },
    ) {
        return false;
    }

    let turn = ctx.state.turn;
    let stats_override = at.stats_override;
    if let Some(live) = find_instance_mut(ctx.state, &card.id) {
        live.summoned_turn = Some(turn);
        if let Some(stats) = stats_override {
            live.stats_override = Some(AttackHealth {
                attack: stats.attack,
                health: stats.health,
            });
        }
    }
    let placed = find_instance(ctx.state, &card.id).cloned().unwrap_or_else(|| card.clone());
    if card_type_of(ctx.state, &placed) == CardType::FieldSpell
        && let Some(live) = find_instance_mut(ctx.state, &card.id)
    {
        live.face_up = Some(true);
    }
    let placed = find_instance(ctx.state, &card.id).cloned().unwrap_or(placed);

    ctx.events.push(GameEvent::Summoned {
        player: slot.player,
        instance_id: placed.id.clone(),
        def_id: placed.def_id.clone(),
        row: slot.row,
        lane: slot.lane,
        former_id,
        arrived_during: None,
        exits_from: None,
    });
    // B3.1 rule 4 (R383): an Animated Field Spell, or an "Animated on your turn" card on its controller's
    // turn, animates as it enters the field.
    animate_on_entry(ctx, &placed);
    // R43, R151: "one created later rolls when it is created", as it arrives anywhere a card can be
    // looked at, and the field is such a place. A #98 Heroic Power that #22's Death summons as a copy
    // or #95 summons into the backrow reaches neither a hand nor a library, the two arrivals
    // `draw.rs` rolls on, and would otherwise hold no power and never be offered `activatePower`. The
    // hook keeps a power the card already rolled (`hero_power::ensure_power`), so a card that arrives
    // with its answer takes no rng draw.
    // §9.3, R113: resumably, so a question in the clause pauses the rest of it (`run_start_of_game`).
    let arrived = find_instance(ctx.state, &card.id).cloned().unwrap_or(placed);
    run_start_of_game(ctx, &arrived, arrived.owner);
    *card = find_instance(ctx.state, &card.id).cloned().unwrap_or(arrived);
    true
}

/// A fresh card of `def_id`, created only once a zone is known so a fizzle creates nothing.
fn summon_fresh(ctx: &mut EffectContext<'_>, def_id: &str, player: PlayerId, at: &SummonPlacement) -> Option<CardInstance> {
    let probe = face_probe(def_id, player, at.radiant == Some(true));
    let row = row_of(card_type_of(ctx.state, &probe))?;
    let slot = zone_for(ctx, player, row, at)?;

    let mut card = new_instance(&mut *ctx.state, def_id, player, Zone::Resolving { player });
    if at.radiant == Some(true) {
        card.radiant = true;
    }
    if let Some(armor) = at.armor_override {
        card.armor_override = Some(armor);
    }
    if summon_onto(ctx, &mut card, &slot, at, None) {
        Some(card)
    } else {
        None
    }
}

/// A card that already exists, moved onto the field "from anywhere else" (§6.3 Summon).
fn summon_existing(
    ctx: &mut EffectContext<'_>,
    card: &CardInstance,
    player: PlayerId,
    at: &SummonPlacement,
) -> Option<CardInstance> {
    if card.zone.z() == ZoneName::Field {
        return None;
    }
    let probe = face_probe(&card.def_id, player, card.radiant || at.radiant == Some(true));
    let row = row_of(card_type_of(ctx.state, &probe))?;
    let slot = zone_for(ctx, player, row, at)?;

    let mut moving = card.clone();
    remove_from_any_zone(ctx.state, &mut moving);
    // R227: a Trap a Recruit sets face-down takes a fresh id, as a Trap played face-down does, so an
    // id seen while the card was public never names it in its zone.
    let former_id = if lands_face_down(ctx.state, &moving, slot.row) {
        Some(fresh_face_down_id(ctx.state, &mut moving))
    } else {
        None
    };
    if at.radiant == Some(true) {
        moving.radiant = true;
    }
    if let Some(armor) = at.armor_override {
        moving.armor_override = Some(armor);
    }
    if summon_onto(ctx, &mut moving, &slot, at, former_id) {
        Some(moving)
    } else {
        None
    }
}

/// R21's roll, applied to the card the summon actually created and only once it has landed. There is
/// one keyword-roll implementation in the engine and it lives in `effects::buff`: `grant_random_keywords`
/// already recomputes R21's pool per draw (so the two keywords of one token are distinct and never
/// one the token already has, which is why a Rush Token draws from the other ten), and it is reached
/// here through the `{ of: "instance" }` `TargetSpec` rather than by copying the pool into this file.
fn roll_random_keywords(ctx: &mut EffectContext<'_>, card: &CardInstance, count: Option<i32>) {
    let Some(count) = count.filter(|count| *count > 0) else {
        return;
    };
    // R174: the roll is aimed at the stay the card has just arrived on, so it is named from a mark
    // taken now — a unit the same list sent to the graveyard and has just summoned back is this one.
    // (TS ran it on a copy of the context with that mark; here the mark is set for the one effect and
    // put back.)
    let effect = grant_random_keywords(json_as(json!({
        "target": { "of": "instance", "instanceId": card.id },
        "count": count,
    })));
    let before = ctx.exits_from;
    ctx.exits_from = Some(exit_mark(ctx.state));
    (effect.apply)(ctx);
    ctx.exits_from = before;
}

/// §6.3 Summon: put a card on the field from anywhere else, with no Cry. With no lane named it takes
/// the leftmost empty, unlocked, unreserved zone of its row (R64) and fails silently when the row
/// has none; a named lane fails the same way when it is occupied or Locked (R47).
pub fn summon(args: SummonArgs) -> Effect {
    Effect::new("summon", move |ctx| {
        let player = player_or_self(ctx, args.player);
        let at = args.placement();

        if let Some(spec) = &args.instance {
            let Some(DamageTarget::Unit { instance }) = resolve_target(ctx, spec) else {
                return;
            };
            let instance = owned(instance);
            if let Some(moved) = summon_existing(ctx, &instance, player, &at) {
                roll_random_keywords(ctx, &moved, args.random_keywords);
            }
            return;
        }
        let Some(def_id) = &args.def_id else {
            return;
        };
        if let Some(made) = summon_fresh(ctx, def_id, player, &at) {
            roll_random_keywords(ctx, &made, args.random_keywords);
        }
    })
}

/// What §10.7's copy semantics need on top of a placement: R57's two #61 deviations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SummonCopyArgs {
    /// The card to copy. #12 copies `{ of: "self" }`; #61 copies the unit the play chose.
    pub of: TargetSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<bool>,
    /// #61: the copy comes out textless, which R23 allows even from an Immutable source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vanilla: Option<bool>,
    /// #61: `false` drops the source's granted keywords, which R57 would otherwise carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted_keywords: Option<bool>,
}

impl SummonCopyArgs {
    /// TS passed these arguments where a `SummonPlacement` was read (its `player`, `lane`, `stack`).
    fn placement(&self) -> SummonPlacement {
        SummonPlacement {
            player: self.player,
            lane: self.lane,
            stack: self.stack,
            ..SummonPlacement::default()
        }
    }
}

/// R443: what a copy carries — every enchantment of the cards it is made from, once each, in their
/// order; `None` when none carries any, so the card stores nothing. A private copy of
/// `enchantments::united_enchantments` (TS compared their JSON, SURFACE §4.4.3).
fn united_enchantments(instances: &[&CardInstance]) -> Option<Vec<Enchantment>> {
    let mut out: Vec<Enchantment> = Vec::new();
    for instance in instances {
        for entry in instance.enchantments.iter().flatten() {
            if !out.iter().any(|held| held == entry) {
                out.push(entry.clone());
            }
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

/// §10.7's `cloneInstance(inst)` and R57: the copy keeps the radiant flag, the buffs, the granted
/// keywords, the Vanilla state and `statsOverride`, and resets damage, exertion, counters and
/// `summonedTurn` — the last of which `summon_onto` sets to the current turn, so the copy is
/// summoning sick like any other new body (§4.1).
///
/// §10.7 names `cloneInstance` as the copy primitive but nothing in the engine provides it, so the
/// clone is written here, once, behind `summon_copy`. Everything R57 does not list is left at
/// `new_instance`'s default rather than carried: the copy is a new card, so it owes no cost history
/// (`costMod`, `costOverride`), has an unspent Divine Shield and an unused Reborn, and remembers
/// nothing of its own (`memory`). Auras are layer 5 of §10.4 and computed on read, so they apply to
/// the copy afresh with no work here (§8.3 #61).
pub fn clone_of(
    ctx: &mut EffectContext<'_>,
    source: &CardInstance,
    player: PlayerId,
    args: &SummonCopyArgs,
) -> CardInstance {
    let mut copy = new_instance(&mut *ctx.state, &source.def_id, player, Zone::Resolving { player });
    copy.radiant = source.radiant;
    copy.buffs = AttackHealth {
        attack: source.buffs.attack,
        health: source.buffs.health,
    };
    copy.vanilla = args.vanilla == Some(true) || source.vanilla;
    // A `Keyword` is never mutated in place — `effects::buff` pushes pool constants that several units
    // already share — so the list is copied and its entries are not.
    if args.granted_keywords != Some(false) {
        copy.granted_keywords = source.granted_keywords.clone();
    }
    if let Some(stats) = source.stats_override {
        copy.stats_override = Some(AttackHealth {
            attack: stats.attack,
            health: stats.health,
        });
    }
    // §7: a Bread Token's "Armor X" is carried beside its X/X, so a copy keeps both halves (R57).
    if let Some(armor) = source.armor_override {
        copy.armor_override = Some(armor);
    }
    // B3.4 rule 4, R443: a copy keeps the source's tuning and enchantments too — never its Brittle
    // count, which a copy never inherits (R57's counters). (`copyTuning` is a deep copy.)
    if let Some(tuning) = source.tuning.clone() {
        copy.tuning = Some(tuning);
    }
    if let Some(enchantments) = united_enchantments(&[source]) {
        copy.enchantments = Some(enchantments);
    }
    copy
}

/// §6.3 Summon plus §10.7's copy semantics: "summon a copy of this unit" (#12) and "summon a Vanilla
/// copy" (#61). The copy is placed per R64 through the same path as `summon`, so it emits `summoned`,
/// fizzles silently on a full, occupied or Locked zone, and fires no Cry (§6.2, R1) — which is the
/// whole reason #12 does not fill the board for 2 mana.
pub fn summon_copy(args: SummonCopyArgs) -> Effect {
    Effect::new("summonCopy", move |ctx| {
        // R57 copies "a unit on the field" (#12 itself, #61's chosen Human). One that has left it —
        // sacrificed by a fused card's other half (#22) earlier in the same list — is gone for this
        // effect (R174), and a copy is never made of a card in a graveyard.
        let Some(source) = instance_of(ctx, &args.of).map(owned) else {
            return;
        };
        if source.zone.z() != ZoneName::Field {
            return;
        }
        let player = player_or_self(ctx, args.player);
        let Some(row) = row_of(card_type_of(ctx.state, &source)) else {
            return;
        };

        // The zone is found before the clone exists, so a fizzle creates nothing and takes no id.
        let at = args.placement();
        let Some(slot) = zone_for(ctx, player, row, &at) else {
            return;
        };
        let mut copy = clone_of(ctx, &source, player, &args);
        summon_onto(ctx, &mut copy, &slot, &at, None);
    })
}

/// `summonRandom`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SummonRandomArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<CatalogQueryArgs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<bool>,
}

impl SummonRandomArgs {
    /// TS passed these arguments where a `SummonPlacement` was read.
    fn placement(&self) -> SummonPlacement {
        SummonPlacement {
            player: self.player,
            lane: self.lane,
            stack: self.stack,
            radiant: self.radiant,
            ..SummonPlacement::default()
        }
    }
}

/// §6.3 Summon from a random pool: "summon a random 1-cost Trap face-down into your backrow zone in
/// this lane" (#67 Zoomerbin Oomen), "summon 3 random 3-cost Units" and "summon 5 random Field Spells
/// or Traps" (#95 Call to Chaos, one of these per card). §10.7 makes `catalog::query` the single source
/// of random pools and §5.1 keeps the requesting def out of one, so the draw is one `rng.pick`
/// over `query({ …, excludeDefId })` (R387), exactly as `discover_from_catalog` builds its offer.
///
/// The draw happens inside `apply`, never when the effect is built: a draw taken at
/// factory-construction time would escape the reducer and desync every later replay (§9.3, R60).
/// It precedes the placement because the row follows from the def drawn (§5.1) — a Trap goes to
/// the backrow and a Unit to the units row — and placement is then `summon`'s own code, so R64's
/// leftmost-free fallback, R47's occupied-or-Locked fizzle, the `summoned` event and the face-down
/// Trap of §3.2/R33 all stay in one place.
///
/// But R129 comes first: "an effect that finds nothing to do draws no random numbers". So the draw is
/// taken only when some card of the pool has a zone to go to — #67's lane already holding a backrow
/// card, or a full unit row under #95's third summon, draws nothing at all.
pub fn summon_random(args: SummonRandomArgs) -> Effect {
    Effect::new("summonRandom", move |ctx| {
        // §5.1: a random pool never offers the card that generated it.
        let own = ctx
            .self_
            .as_ref()
            .map(|me| me.def_id.clone())
            .or_else(|| ctx.def_id.clone());
        let asked: CatalogQueryArgs = args.query.clone().unwrap_or_default();
        let pool = query(&excluding_def_id(&asked, own.as_deref()));
        let player = player_or_self(ctx, args.player);
        let at = args.placement();
        let rows: IndexSet<Row> = pool.iter().filter_map(|def| row_of(def.type_)).collect();
        if !rows.iter().any(|row| zone_for(ctx, player, *row, &at).is_some()) {
            return;
        }

        let Some(def_id) = pick_generated(ctx.sink.rng, &pool, None).map(|def| def.id.clone()) else {
            return;
        };
        summon_fresh(ctx, &def_id, player, &at);
    })
}

/// Which library cards a Recruit will consider, on top of "must be a permanent" (§6.3).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecruitFilter {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<OneOrMany<CardType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_range: Option<CostRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_id: Option<OneOrMany<String>>,
}

/// TS `asList`: `undefined` is none, one value is a list of one.
fn as_list<T: Clone + PartialEq>(value: Option<&OneOrMany<T>>) -> Vec<T> {
    match value {
        None => Vec::new(),
        Some(values) => values.as_slice().to_vec(),
    }
}

/// Whether a library card passes a Recruit's filter. The cost is R65's one calculation for that
/// instance (`effective_cost`), which R65 applies outside play ("library, hand, GY, pools, filters,
/// comparisons") and #30 Archivist and #94 Genn's Greed already read library cards by (R24, R66): a
/// card never played has no X (an X-cost card reads 0) and no embiggen price (its base), and its
/// `costMod` and `costOverride` travel with it into every zone (R78), so a printed-3 Unit #95 made
/// "cost 2 less" is a Unit costing 1 for #69 Call to Arms. The definition's printed cost would miss it.
fn matches_filter(ctx: &EffectContext<'_>, card: &CardInstance, filter: &RecruitFilter) -> bool {
    let def = def_of(ctx.state, &card.def_id);
    let types = as_list(filter.type_.as_ref());
    if !types.is_empty() && !types.contains(&card_type_of(ctx.state, card)) {
        return false;
    }
    let def_ids = as_list(filter.def_id.as_ref());
    if !def_ids.is_empty() && !def_ids.contains(&def.id) {
        return false;
    }
    if let Some(tags) = &filter.tags
        && !tags.iter().all(|tag| def.tags.contains(tag))
    {
        return false;
    }
    if let Some(not_tags) = &filter.not_tags
        && not_tags.iter().any(|tag| def.tags.contains(tag))
    {
        return false;
    }

    let cost = effective_cost(ctx.state, card, Default::default());
    if let Some(wanted) = filter.cost
        && cost != wanted
    {
        return false;
    }
    if let Some(min) = filter.cost_range.and_then(|range| range.min)
        && cost < min
    {
        return false;
    }
    if let Some(max) = filter.cost_range.and_then(|range| range.max)
        && cost > max
    {
        return false;
    }
    true
}

/// Where a Recruit scans (B5 E25): a player's library, top down (§6.3), or a player's exile, newest
/// first — exile is chronological (§3), so its "top" is the card exiled last (Classic #1 Radiant).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum RecruitSource {
    Library,
    Exile,
}

/// The pile a Recruit scans, in scan order: a library top down, an exile newest first.
fn recruit_pile(ctx: &EffectContext<'_>, from: RecruitSource, whose: PlayerId) -> Vec<CardInstance> {
    let side = &ctx.state.players[whose];
    match from {
        RecruitSource::Exile => side.exile.iter().rev().cloned().collect(),
        RecruitSource::Library => side.library.clone(),
    }
}

/// A card a Recruit may take: a permanent (§6.3, R43's "recruits a permanent") that matches the
/// filter — and never a unit-token card, which leaves a library only by being drawn (R11, R218).
fn recruitable(ctx: &EffectContext<'_>, card: &CardInstance, filter: &RecruitFilter) -> bool {
    is_permanent_type(card_type_of(ctx.state, card))
        && !is_unit_token(ctx.state, card)
        && matches_filter(ctx, card, filter)
}

/// R690: of the valid targets a scan may take, an (X)-cost card comes last — recruited, never played,
/// it would arrive with no X behind it, so a scan takes the first valid target that is not (X)-cost
/// and only when nothing else is valid takes the first (X)-cost one.
fn skip_x_cost(ctx: &EffectContext<'_>, cards: Vec<CardInstance>) -> Vec<CardInstance> {
    if cards.is_empty() {
        return cards;
    }
    let solid: Vec<CardInstance> = cards.iter().filter(|card| !is_x_cost(ctx.state, card)).cloned().collect();
    if solid.is_empty() { cards } else { solid }
}

/// The valid targets of one scan, in scan order (before R690's ordering).
fn valid_in_pile(
    ctx: &EffectContext<'_>,
    from: RecruitSource,
    whose: PlayerId,
    filter: &RecruitFilter,
) -> Vec<CardInstance> {
    let mut valid = Vec::new();
    for card in recruit_pile(ctx, from, whose) {
        if recruitable(ctx, &card, filter) {
            valid.push(card);
        }
    }
    valid
}

/// #98 radiant, "Recruit and make it Radiant": the second half is §6.3's Make Radiant, and every
/// visible change is announced (§10.3) — `radiantSet` is what §10.10 animates the glow from. The flag
/// went on as the card left the pile, so it lands on its Radiant face; the cue follows the summon, and
/// goes out whether or not the card was Radiant already, as R177 has a Make Radiant on a card someone
/// may not read (a face-down Trap) do.
fn announce_radiant(ctx: &mut EffectContext<'_>, recruited: Option<&CardInstance>, radiant: Option<bool>) {
    if let Some(recruited) = recruited
        && radiant == Some(true)
    {
        ctx.events.push(GameEvent::RadiantSet {
            instance_id: recruited.id.clone(),
            def_id: recruited.def_id.clone(),
            zone: recruited.zone.clone(),
        });
    }
}

/// `recruit`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecruitArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<RecruitFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    // ---- v0.2.0, generation (E25) ----
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<RecruitSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whose: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
}

/// §6.3 Recruit: scan the library top down for the first permanent that matches, summon it per R64
/// (a Trap face-down), and leave the rest of the library in its order.
///
/// B5 E25 extends where and how often: `from: "exile"` scans an exile newest first, and `whose` names
/// whose pile it is (default the recruiting side's) — "Recruit a card from their exile" (Classic #1
/// Radiant) summons the opponent's card on the recruiting side, under its control, its owner unchanged,
/// so it goes back to its owner's piles when it leaves the field (§3.2, R12). `count` is "Recruit N"
/// (Classic #31 Radiant, #65 Radiant): N scans, one after another, each the whole of a single Recruit,
/// so a scan whose card finds no zone fizzles and the next scan finds that card again (Core #69's
/// "three top-down scans; stops when the board is full"). Every scan skips (X)-cost cards unless they
/// are the only valid targets (R690).
pub fn recruit(args: RecruitArgs) -> Effect {
    Effect::new("recruit", move |ctx| {
        let player = player_or_self(ctx, args.player);
        let whose = match args.whose {
            None => player,
            Some(spec) => player_of(ctx, spec),
        };
        let filter = args.filter.clone().unwrap_or_default();
        let from = args.from.unwrap_or(RecruitSource::Library);
        let scans = args.count.unwrap_or(1).max(1);
        for _ in 0..scans {
            let valid = valid_in_pile(ctx, from, whose, &filter);
            let Some(found) = skip_x_cost(ctx, valid).into_iter().next() else {
                return;
            };

            let at = SummonPlacement {
                lane: args.lane,
                radiant: args.radiant,
                ..SummonPlacement::default()
            };
            let recruited = summon_existing(ctx, &found, player, &at);
            announce_radiant(ctx, recruited.as_ref(), args.radiant);
        }
    })
}

/// `recruitAll`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecruitAllArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<RecruitFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<RecruitSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whose: Option<PlayerSpec>,
}

/// B5 E25, Classic #60 Pile On: "Recruit every permanent in your deck". The pile is read once, top
/// down (a library) or newest first (an exile), as the effect reaches it, and each matching permanent
/// is summoned in turn while its row has an open zone — a Unit to the unit row, the rest to the backrow,
/// a Trap face-down (§3.2) — and passed over, staying where it is, once that row is full; a Spell or a
/// unit-token card stays (§6.3, R218). (X)-cost cards stay too unless they are the only valid targets
/// (R690). Each summon is its own step of a part (`resolve::lazy_part`) whose
/// card list is kept as its memo, so a question a summoned card asks as it arrives (R151) pauses the
/// rest, which resumes over the same cards in the same order (R113).
pub fn recruit_all(args: RecruitAllArgs) -> Effect {
    lazy_part("recruitAll", move |ctx, memo| {
        let player = player_or_self(ctx, args.player);
        let whose = match args.whose {
            None => player,
            Some(spec) => player_of(ctx, spec),
        };
        let filter = args.filter.clone().unwrap_or_default();
        let pile = args.from.unwrap_or(RecruitSource::Library);
        let ids: Vec<String> = match memo.as_ref().and_then(Value::as_array) {
            Some(listed) => listed.iter().filter_map(|id| id.as_str().map(str::to_string)).collect(),
            None => skip_x_cost(ctx, valid_in_pile(ctx, pile, whose, &filter))
                .into_iter()
                .map(|card| card.id)
                .collect(),
        };
        let effects: Vec<Effect> = ids
            .iter()
            .map(|id| {
                let id = id.clone();
                Effect::new("recruitOne", move |at| {
                    let Some(card) = recruit_pile(at, pile, whose).into_iter().find(|candidate| candidate.id == id) else {
                        return;
                    };
                    let Some(row) = row_of(card_type_of(at.state, &card)) else {
                        return;
                    };
                    if first_entry_zone(at.state, player, row).is_none() {
                        return;
                    }
                    summon_existing(at, &card, player, &SummonPlacement::default());
                })
            })
            .collect();
        EffectPart {
            effects,
            memo: Some(json!(ids)),
        }
    })
}

/// `fillBoard`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FillBoardArgs {
    pub def_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats_override: Option<StatsOverride>,
    /// §7: the Bread Token's "Armor X", carried beside `stats_override` (#22 radiant's copies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_override: Option<i32>,
}

/// §7 and R64: "fill your board" summons the named token into every empty, unlocked unit zone,
/// left to right.
pub fn fill_board(args: FillBoardArgs) -> Effect {
    Effect::new("fillBoard", move |ctx| {
        let player = player_or_self(ctx, args.player);
        let probe = face_probe(&args.def_id, player, args.radiant == Some(true));
        if row_of(card_type_of(ctx.state, &probe)) != Some(Row::Units) {
            return;
        }

        for slot in fill_board_zones(ctx.state, player) {
            let at = SummonPlacement {
                lane: Some(slot.lane),
                radiant: args.radiant,
                stats_override: args.stats_override,
                armor_override: args.armor_override,
                ..SummonPlacement::default()
            };
            summon_fresh(ctx, &args.def_id, player, &at);
        }
    })
}
