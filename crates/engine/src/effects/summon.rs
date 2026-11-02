//! Summon, Recruit and "fill your board": every way a card reaches the field without being played
//! (SPEC §6.3 Summon and Recruit, §3.2, §7, R64). A summon fires no Cry; `play` and `cast` do (R1).
//!
//! `summon_copy` (R57), `summon_random` (§5.1) and `summon`'s `random_keywords` (R21, #80) are each a
//! summon with one clause bolted on: placement, `summoned`, the face-down Trap and every fizzle stay in
//! `zone_for`/`summon_onto`. Off the board a card travels as an owned copy; placed, writes go to the state.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::animated::animate_on_entry;
use crate::catalog::{CatalogQueryArgs, def_of, excluding_def_id, pick_generated, query};
use crate::config::CRY_ON_PLAY_ONLY;
use crate::damage::DamageTarget;
use crate::effects::buff::{grant_keyword, grant_random_keywords};
use crate::effects::radiant::set_radiant;
use crate::effects::targets::{
    PlayerSpec, ScopeSide, TargetSpec, instance_of, player_of, resolve_target, sides_of,
};
use crate::faces::card_type_of;
use crate::mana::{effective_cost, is_x_cost};
use crate::prelude::json_as;
use crate::prompts::run_start_of_game;
use crate::resolve::lazy_part;
use crate::script::{Effect, EffectContext, EffectPart};
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut, new_instance};
use crate::stays::exit_mark;
use crate::wire::{
    AttackHealth, CardType, CostRange, GameEvent, Keyword, OneOrMany, PlayerId, Row, Tag, Zone, ZoneName,
};
use crate::zones::{
    PlaceOnFieldOptions, ZoneSlot, fill_board_zones, first_entry_zone, fresh_face_down_id, is_empty,
    is_reserved, is_unit_token, lands_face_down, open_zones, place_on_field, place_standing_on_field,
    remove_from_any_zone, row_size,
};

// R1: no verb placing a card without a play fires a Cry (Transform, Flicker, Reborn, `summon_this`).
const _: () = assert!(
    CRY_ON_PLAY_ONLY,
    "R1: no summon fires a Cry (effects/summon.rs and every verb that places a card without a play)"
);

/// A scratch instance to read a def's running face from before any card exists, off the state's ids.
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// R1184003: the X the card is summoned with, as if played for it (B2.7): a face that prints its
    /// stats as multiples of X (`xStats`) is those multiples of it, and whatever reads the card's X
    /// (`tuning::x_of`) reads it — Meditative+ #20.1 Lil' Kenny's X/X and its keywords by X.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    /// R1088: the card whose effect summoned it, for the client; no rule reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    /// ME-CN, R1185002: a fresh card summoned in Chinese (`CardInstance.chinese`, R1300), as an
    /// `addToHand`'s `chinese` rider makes one in a hand (M+ #21 Harmony Spirit's Rush Tokens).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chinese: Option<bool>,
    /// R2206000: what a fresh card's memory holds as it lands, so a later hook can tell a body this
    /// summon made from one played or made otherwise (TG #25 Trigor's chain, R2197004). Fresh
    /// summons only; leaving the field clears it as it clears all memory (R78).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>", optional))]
    pub memory: Option<IndexMap<String, Value>>,
}

/// `summon`'s arguments, the placement's keys beside the rest.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// R1184003: the X the card is summoned with (`SummonPlacement.x`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    /// Create a fresh card of this definition, a token included (§7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_id: Option<String>,
    /// MD-D30, R1126: once the fresh summon has landed, remember its id on the running card under
    /// this key, as `effects::memory::remember` writes — so a later trigger finds the card it made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remember_as: Option<String>,
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
    /// ST-S03-8 (R1860007's Felinor Containment Chamber, issue #1860): keywords the fresh summon
    /// gains as it lands ("It gains Divine Shield"). The grant belongs to the summon for the same
    /// reason as `random_keywords` above: no `TargetSpec` can name a just-created card (see that
    /// comment). Fresh summons only; a card moved by `instance` keeps what it has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<Keyword>>,
    /// R2206000: the fresh card's memory as it lands (`SummonPlacement.memory`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>", optional))]
    pub memory: Option<IndexMap<String, Value>>,
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
            x: self.x,
            source_id: None,
            chinese: None,
            memory: self.memory.clone(),
        }
    }
}

/// §5.1: the row a card type lives in, or `None` for a Spell, which is never summoned. B2.7: a card's
/// type is its running face's (`faces::card_type_of`), a pool's definition its base face's.
pub(crate) fn row_of(card_type: CardType) -> Option<Row> {
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
    zone_and_stance_for(ctx, player, row, at, false).map(|(slot, _)| slot)
}

/// `zone_for`, and whether the card stands in a backrow zone (`entry_zone`, R1081000, R2248000).
fn zone_and_stance_for(
    ctx: &EffectContext<'_>,
    player: PlayerId,
    row: Row,
    at: &SummonPlacement,
    flies: bool,
) -> Option<(ZoneSlot, bool)> {
    let Some(lane) = at.lane else {
        return entry_zone(ctx.state, player, row, flies);
    };
    if lane < 1 || lane > row_size(row) {
        return None;
    }
    let slot = ZoneSlot { player, row, lane };
    if can_place(ctx, &slot, at.stack == Some(true)) {
        Some((slot, false))
    } else {
        None
    }
}

/// R64, R688, R2248000: the zone a summon that names no lane enters, the leftmost entry zone of its
/// row, else, for a Unit that `flies`, the backrow's, where it stands (R1081000).
fn entry_zone(state: &GameState, player: PlayerId, row: Row, flies: bool) -> Option<(ZoneSlot, bool)> {
    if let Some(slot) = first_entry_zone(state, player, row) {
        return Some((slot, false));
    }
    if row == Row::Units && flies {
        return first_entry_zone(state, player, Row::Backrow).map(|slot| (slot, true));
    }
    None
}

/// R2248000: whether this Unit's face, as it stands where it is (§10.4), has Flying, so a summon into a
/// full unit row may stand it in the backrow.
fn summon_flies(state: &GameState, card: &CardInstance) -> bool {
    card_type_of(state, card) == CardType::Unit
        && crate::layers::unit_has(state, card, crate::wire::KeywordKind::Flying)
}

/// R2248000: whether a summon of `card` that names no lane finds a zone on `player`'s side, a Flying
/// Unit's in the backrow included, for a caller that must know before it draws or moves anything (R129).
pub(crate) fn summon_lands(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    row_of(card_type_of(state, card))
        .is_some_and(|row| entry_zone(state, player, row, summon_flies(state, card)).is_some())
}

/// The body every summon shares: place the card (standing in a backrow zone if `stands`, R1081000,
/// R1879001, R2248000), apply the §7 stat override, leave a Trap face-down while a Field Spell is public
/// (§3.2, R33), and emit `summoned`. `card` is the caller's off-board copy, on success the card as placed.
fn summon_onto(
    ctx: &mut EffectContext<'_>,
    card: &mut CardInstance,
    slot: &ZoneSlot,
    at: &SummonPlacement,
    former_id: Option<String>,
    stands: bool,
) -> bool {
    let options = PlaceOnFieldOptions {
        stack: Some(at.stack == Some(true)),
    };
    let placed = if stands {
        place_standing_on_field(ctx.state, card, slot, options)
    } else {
        place_on_field(ctx.state, card, slot, options)
    };
    if !placed {
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
    let placed = crate::zones::live_or(ctx.state, card);
    if card_type_of(ctx.state, &placed) == CardType::FieldSpell
        && let Some(live) = find_instance_mut(ctx.state, &card.id)
    {
        live.face_up = Some(true);
    }
    let placed = find_instance(ctx.state, &card.id).cloned().unwrap_or(placed);
    // R2089000: the seat whose effect summoned it, for the game record, when it is not the side's.
    let creator = (ctx.controller != slot.player).then_some(ctx.controller);

    ctx.events.push(GameEvent::Summoned {
        player: slot.player,
        instance_id: placed.id.clone(),
        def_id: placed.def_id.clone(),
        row: slot.row,
        lane: slot.lane,
        former_id,
        source_id: at.source_id.clone(),
        arrived_during: None,
        exits_from: None,
        creator,
    });
    // R783010: a Crater Unit Locks the zone it was summoned into, before its Cry.
    crate::crater::lock_on_arrival(ctx, &placed.id);
    // B3.1 rule 4 (R383): an Animated Field Spell, or an "Animated on your turn" card on its controller's
    // turn, animates as it enters the field.
    animate_on_entry(ctx, &placed);
    // R43, R151: "one created later rolls when it is created", and the field is such an arrival (a #98
    // Heroic Power summoned by #22 or #95 reaches no hand or library); a rolled power takes no rng draw.
    // §9.3, R113: resumably, so a question in the clause pauses the rest of it.
    let arrived = find_instance(ctx.state, &card.id).cloned().unwrap_or(placed);
    run_start_of_game(ctx, &arrived, arrived.owner);
    *card = find_instance(ctx.state, &card.id).cloned().unwrap_or(arrived);
    true
}

/// A fresh card of `def_id`, created only once a zone is known so a fizzle creates nothing.
pub(crate) fn summon_fresh(
    ctx: &mut EffectContext<'_>,
    def_id: &str,
    player: PlayerId,
    at: &SummonPlacement,
) -> Option<CardInstance> {
    let probe = face_probe(def_id, player, at.radiant == Some(true));
    let row = row_of(card_type_of(ctx.state, &probe))?;
    let flies = summon_flies(ctx.state, &probe);
    let (slot, stands) = zone_and_stance_for(ctx, player, row, at, flies)?;
    summon_fresh_into(ctx, def_id, player, &slot, at, stands)
}

/// R1879001: a fresh Unit summoned standing in its controller's backrow zone `at.lane`, as a Flying
/// play stands there (R1081000): an open, unreserved zone, else nothing. The caller vets the card.
pub(crate) fn summon_fresh_standing(
    ctx: &mut EffectContext<'_>,
    def_id: &str,
    player: PlayerId,
    at: &SummonPlacement,
) -> Option<CardInstance> {
    let lane = at.lane?;
    if lane < 1 || lane > row_size(Row::Backrow) {
        return None;
    }
    let slot = ZoneSlot {
        player,
        row: Row::Backrow,
        lane,
    };
    if is_reserved(ctx.state, slot) || !is_empty(ctx.state, slot) {
        return None;
    }
    summon_fresh_into(ctx, def_id, player, &slot, at, true)
}

fn summon_fresh_into(
    ctx: &mut EffectContext<'_>,
    def_id: &str,
    player: PlayerId,
    slot: &ZoneSlot,
    at: &SummonPlacement,
    stands: bool,
) -> Option<CardInstance> {
    let mut card = new_instance(&mut *ctx.state, def_id, player, Zone::Resolving { player });
    if at.radiant == Some(true) {
        card.radiant = true;
    }
    if let Some(armor) = at.armor_override {
        card.armor_override = Some(armor);
    }
    if at.chinese == Some(true) {
        card.chinese = Some(true);
    }
    // R1184003: its X before it lands, so it arrives at its X stats and with its X-read keywords.
    if let Some(x) = at.x {
        card.x = Some(x);
    }
    if let Some(memory) = &at.memory {
        card.memory.extend(memory.clone());
    }
    crate::generators::record_generator(ctx, &card);
    if summon_onto(ctx, &mut card, slot, at, None, stands) {
        Some(card)
    } else {
        None
    }
}

/// ME-ATTACKSUMMON (R1202): summon a card that already exists — a Unit from its controller's
/// hand — with the default placement: the leftmost open zone of its row (R64), no Cry (R1).
pub fn summon_existing_card(
    ctx: &mut EffectContext<'_>,
    card: &CardInstance,
    player: PlayerId,
) -> Option<CardInstance> {
    summon_existing(ctx, card, player, &SummonPlacement::default())
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
    // R2533000: a card in the hand of a seat that plays only the tutorial's cards, and not one of them,
    // is put onto the field by no effect: a summon out of a hand is that hand's play.
    if let Zone::Hand { player: holder } = card.zone
        && crate::state::barred_from_play(ctx.state, holder, card)
    {
        return None;
    }
    let probe = face_probe(&card.def_id, player, card.radiant || at.radiant == Some(true));
    let row = row_of(card_type_of(ctx.state, &probe))?;
    // R2248000: the card's Flying as it stands where it is (a hand's, a graveyard's printed one).
    let flies = summon_flies(ctx.state, card);
    let (slot, stands) = zone_and_stance_for(ctx, player, row, at, flies)?;

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
    if let Some(x) = at.x {
        moving.x = Some(x);
    }
    if summon_onto(ctx, &mut moving, &slot, at, former_id, stands) {
        Some(moving)
    } else {
        None
    }
}

/// R21's roll on the card the summon created, once it has landed, through the engine's one keyword roll
/// (`effects::buff::grant_random_keywords`, so a token's keywords are distinct and new to it).
fn roll_random_keywords(ctx: &mut EffectContext<'_>, card: &CardInstance, count: Option<i32>) {
    let Some(count) = count.filter(|count| *count > 0) else {
        return;
    };
    // R174: the roll is aimed at the stay the card has just arrived on, so it is named from a mark
    // taken now — a unit the same list sent to the graveyard and has just summoned back is this one.
    let effect = grant_random_keywords(json_as(json!({
        "target": { "of": "instance", "instanceId": card.id },
        "count": count,
    })));
    let before = ctx.exits_from;
    ctx.exits_from = Some(exit_mark(ctx.state));
    (effect.apply)(ctx);
    ctx.exits_from = before;
}

/// The `keywords` rider: each keyword granted to the fresh card by its id through `grantKeyword`,
/// stay-marked as `roll_random_keywords` is (R174), so every grant is announced (`keywordGranted`).
fn grant_summon_keywords(ctx: &mut EffectContext<'_>, card: &CardInstance, keywords: Vec<Keyword>) {
    if keywords.is_empty() {
        return;
    }
    let before = ctx.exits_from;
    ctx.exits_from = Some(exit_mark(ctx.state));
    for keyword in keywords {
        let effect = grant_keyword(json_as(json!({
            "target": { "of": "instance", "instanceId": card.id },
            "keyword": keyword,
        })));
        (effect.apply)(ctx);
    }
    ctx.exits_from = before;
}

/// §6.3 Summon: put a card on the field from anywhere else, with no Cry. With no lane named it takes
/// the leftmost empty, unlocked, unreserved zone of its row (R64) and fails silently when the row
/// has none; a named lane fails the same way when it is occupied or Locked (R47).
pub fn summon(args: SummonArgs) -> Effect {
    Effect::new("summon", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let at = args.placement();

        if let Some(spec) = &args.instance {
            let Some(DamageTarget::Unit { instance }) = resolve_target(ctx, spec) else {
                return;
            };

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
            grant_summon_keywords(ctx, &made, args.keywords.clone().unwrap_or_default());
            // MD-D30, R1126: as `effects::memory::remember` writes (its own stay, its own part's key).
            if let Some(key) = &args.remember_as
                && let Some(this) = super::targets::self_on_its_stay(ctx)
                && let Some(card) = find_instance_mut(ctx.sink.state, &this.id)
            {
                crate::work::remember_on(&mut card.memory, &ctx.data, key, json!(made.id.clone()));
            }
        }
    })
}

/// `summonPile`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SummonPileArgs {
    /// The pile's cards, bottom first; the last is its top, the one that acts (§3.2).
    pub def_ids: Vec<String>,
    /// Who controls the pile; its owner too, as its cards are created here (R12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    /// A named lane; nothing is summoned when that zone is occupied or reserved (R47).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<i32>,
    /// Every card of the pile is Radiant (the Radiant Drone Battleship's layers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// R1842000: `def_ids` summoned into one unit zone as a §3.2 Stack pile, first at the bottom (the Drone
/// Battleship's layers, so the Shield is fought first), into `zone_for`'s zone (R64, R688) or nothing,
/// each its own summon (no Cry, R1), a non-Unit passed over. The zone is the part's memo, so a question a
/// card asks as it arrives (R151) pauses the rest, which resumes onto the same zone (R113).
pub fn summon_pile(args: SummonPileArgs) -> Effect {
    lazy_part("summonPile", move |ctx, memo| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let remembered = memo
            .as_ref()
            .and_then(Value::as_i64)
            .and_then(|lane| i32::try_from(lane).ok());
        let lane = match remembered {
            Some(lane) => lane,
            None => {
                let at = SummonPlacement {
                    lane: args.lane,
                    ..SummonPlacement::default()
                };
                let Some(slot) = zone_for(ctx, player, Row::Units, &at) else {
                    return EffectPart::default();
                };
                slot.lane
            }
        };
        let source_id = ctx.self_.as_ref().map(|me| me.id.clone());
        let effects: Vec<Effect> = args
            .def_ids
            .iter()
            .enumerate()
            .map(|(index, def_id)| {
                let def_id = def_id.clone();
                let placement = SummonPlacement {
                    lane: Some(lane),
                    stack: (index > 0).then_some(true),
                    radiant: args.radiant,
                    source_id: source_id.clone(),
                    ..SummonPlacement::default()
                };
                Effect::new("summonPileCard", move |at| {
                    let probe = face_probe(&def_id, player, placement.radiant == Some(true));
                    if card_type_of(at.state, &probe) != CardType::Unit {
                        return;
                    }
                    summon_fresh(at, &def_id, player, &placement);
                })
            })
            .collect();
        EffectPart {
            effects,
            memo: Some(json!(lane)),
        }
    })
}

/// What §10.7's copy semantics need on top of a placement: R57's two #61 deviations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// The `SummonPlacement` half of these arguments (its `player`, `lane`, `stack`).
    fn placement(&self) -> SummonPlacement {
        SummonPlacement {
            player: self.player,
            lane: self.lane,
            stack: self.stack,
            ..SummonPlacement::default()
        }
    }
}

/// §10.7's `cloneInstance` and R57: the copy keeps the radiant flag, the buffs (none for a Vanilla copy,
/// R781003), the granted keywords, the Vanilla state and `statsOverride`; damage, exertion, counters and
/// `summonedTurn` reset (summoning sick, §4.1), the rest at `new_instance`'s default (no cost history or
/// memory, an unspent Divine Shield and Reborn). Auras (§10.4 layer 5) apply afresh on read (§8.3 #61).
pub fn clone_of(
    ctx: &mut EffectContext<'_>,
    source: &CardInstance,
    player: PlayerId,
    args: &SummonCopyArgs,
) -> CardInstance {
    let mut copy = new_instance(
        &mut *ctx.state,
        &source.def_id,
        player,
        Zone::Resolving { player },
    );
    copy.radiant = source.radiant;
    // ME-CN, R1300: a copy of a Chinese card is Chinese, as a copy of a Radiant card is Radiant.
    copy.chinese = source.chinese;
    // MD-B15, R923: a copy keeps the granted tags of the card copied.
    copy.granted_tags = source.granted_tags.clone();
    // ME-GRANT (MD-D13): a copy keeps the granted Death abilities of the card copied (R57).
    copy.grants = source.grants.clone();
    copy.buffs = AttackHealth {
        attack: source.buffs.attack,
        health: source.buffs.health,
    };
    copy.vanilla = args.vanilla == Some(true) || source.vanilla;
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
    // B3.4 rule 4, R443: a copy keeps the source's tuning and enchantments, never its Brittle count (R57).
    if let Some(tuning) = source.tuning.clone() {
        copy.tuning = Some(tuning);
    }
    if let Some(enchantments) = crate::enchantments::united_enchantments([source]) {
        copy.enchantments = Some(enchantments);
    }
    // R781003: a copy made Vanilla is a Vanilla'd Unit like any other, at its printed stats (#61).
    if args.vanilla == Some(true) {
        super::transform::to_printed_stats(&mut copy);
    }
    crate::generators::record_generator(ctx, &copy);
    copy
}

/// §6.3 Summon plus §10.7's copy semantics: "summon a copy of this unit" (#12), "summon a Vanilla copy"
/// (#61), placed per R64 by `summon`'s path: it fizzles on a full, occupied or Locked zone and fires no
/// Cry (§6.2, R1), which is why #12 does not fill the board for 2 mana.
pub fn summon_copy(args: SummonCopyArgs) -> Effect {
    Effect::new("summonCopy", move |ctx| {
        // R57 copies "a unit on the field": one that left it earlier in the same list (#22's sacrifice)
        // is gone for this effect (R174), and a card in a graveyard is never copied.
        let Some(source) = instance_of(ctx, &args.of) else {
            return;
        };
        if source.zone.z() != ZoneName::Field {
            return;
        }
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let Some(row) = row_of(card_type_of(ctx.state, &source)) else {
            return;
        };

        // The zone is found before the clone exists, so a fizzle creates nothing and takes no id.
        // R2248000: a Flying copy stands in the backrow when the unit row is full.
        let at = args.placement();
        let flies = summon_flies(ctx.state, &source);
        let Some((slot, stands)) = zone_and_stance_for(ctx, player, row, &at, flies) else {
            return;
        };
        let mut copy = clone_of(ctx, &source, player, &args);
        summon_onto(ctx, &mut copy, &slot, &at, None, stands);
    })
}

/// `summonRandom`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// The `SummonPlacement` half of these arguments.
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

/// §6.3 Summon from a random pool (#67 Zoomerbin Oomen, #95 Call to Chaos): one `rng.pick` over
/// `query({ …, excludeDefId })` (§10.7, §5.1, R387), drawn inside `apply`, never as the effect is built,
/// which would desync every later replay (§9.3, R60), and before placement, as the def drawn picks the
/// row (R64, R47, §3.2/R33). R129: it draws only when some card of the pool has a zone to go to.
pub fn summon_random(args: SummonRandomArgs) -> Effect {
    Effect::new("summonRandom", move |ctx| {
        // §5.1: a random pool never offers the card that generated it.
        let own = ctx
            .self_
            .as_ref()
            .map(|me| me.def_id.clone())
            .or_else(|| ctx.def_id.clone());
        let asked: CatalogQueryArgs = args.query.clone().unwrap_or_default();
        let pool = query(&excluding_def_id(Some(&*ctx.state), &asked, own.as_deref()));
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let at = args.placement();
        let rows: IndexSet<Row> = pool.iter().filter_map(|def| row_of(def.type_)).collect();
        // R2248000: a Flying Unit of the pool may stand in the backrow when the unit row is full.
        let a_flier_lands = || {
            at.lane.is_none()
                && pool.iter().any(|def| {
                    def.type_ == CardType::Unit
                        && def
                            .base
                            .keywords
                            .iter()
                            .any(|keyword| keyword.kind() == crate::wire::KeywordKind::Flying)
                })
                && first_entry_zone(ctx.state, player, Row::Backrow).is_some()
        };
        if !rows.iter().any(|row| zone_for(ctx, player, *row, &at).is_some()) && !a_flier_lands() {
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// No value is an empty list, one value a list of one.
fn as_list<T: Clone + PartialEq>(value: Option<&OneOrMany<T>>) -> Vec<T> {
    match value {
        None => Vec::new(),
        Some(values) => values.as_slice().to_vec(),
    }
}

/// Whether a library card passes a Recruit's filter, at R65's one cost (`effective_cost`), as #30
/// Archivist and #94 Genn's Greed read library cards (R24, R66): no X (an X-cost card reads 0), no
/// embiggen price, and the `costMod` and `costOverride` it carries (R78, R766).
fn matches_filter(ctx: &EffectContext<'_>, card: &CardInstance, filter: &RecruitFilter) -> bool {
    let def = def_of(Some(&*ctx.state), &card.def_id);
    let types = as_list(filter.type_.as_ref());
    if !types.is_empty() && !types.contains(&card_type_of(ctx.state, card)) {
        return false;
    }
    let def_ids = as_list(filter.def_id.as_ref());
    if !def_ids.is_empty() && !def_ids.contains(&def.id) {
        return false;
    }
    // MD-B15, R923: a granted tag is part of the card for a Recruit filter too.
    let tags = crate::query::tags_of(ctx.state, card);
    if let Some(wanted) = &filter.tags
        && !wanted.iter().all(|tag| tags.contains(tag))
    {
        return false;
    }
    if let Some(not_tags) = &filter.not_tags
        && not_tags.iter().any(|tag| tags.contains(tag))
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// R690: a scan takes the first valid target that is not (X)-cost (recruited, never played, it would
/// arrive with no X), and the first (X)-cost one only when nothing else is valid.
fn skip_x_cost(ctx: &EffectContext<'_>, cards: Vec<CardInstance>) -> Vec<CardInstance> {
    if cards.is_empty() {
        return cards;
    }
    let solid: Vec<CardInstance> = cards
        .iter()
        .filter(|card| !is_x_cost(ctx.state, card))
        .cloned()
        .collect();
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

/// #98 radiant, "Recruit and make it Radiant": §6.3's Make Radiant, announced (§10.3) as `radiantSet`
/// for §10.10's glow. The flag went on as the card left the pile; the cue follows the summon even on a
/// card already Radiant, as R177 has a Make Radiant on a card someone may not read (a face-down Trap) do.
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<RecruitSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whose: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
}

/// §6.3 Recruit: summon the library's first matching permanent, top down, per R64 (a Trap face-down).
/// B5 E25: `from: "exile"` scans newest first; `whose` names whose pile (Classic #1 Radiant takes the
/// opponent's, its owner unchanged, §3.2, R12, but into a graveyard it lands as its controller's,
/// R2618000); `count` is N whole scans, so a card with no zone fizzles and the next finds it again
/// (Classic #31 and #65 Radiant, Core #69). (X)-cost cards come last (R690).
pub fn recruit(args: RecruitArgs) -> Effect {
    Effect::new("recruit", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// B5 E25, Classic #60 Pile On: "Recruit every permanent in your deck". The pile is read once and each
/// match summoned in turn where its row has an open zone (for a Flying Unit, the backrow's, R2248000),
/// else passed over; a Spell, a unit-token card (§6.3, R218) and an (X)-cost one unless the only one
/// (R690) stay. The memo is the card list, so a question a card asks (R151) pauses the rest (R113).
pub fn recruit_all(args: RecruitAllArgs) -> Effect {
    lazy_part("recruitAll", move |ctx, memo| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let whose = match args.whose {
            None => player,
            Some(spec) => player_of(ctx, spec),
        };
        let filter = args.filter.clone().unwrap_or_default();
        let pile = args.from.unwrap_or(RecruitSource::Library);
        let ids: Vec<String> = match memo.as_ref().and_then(Value::as_array) {
            Some(listed) => listed
                .iter()
                .filter_map(|id| id.as_str().map(str::to_string))
                .collect(),
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
                    let Some(card) = recruit_pile(at, pile, whose)
                        .into_iter()
                        .find(|candidate| candidate.id == id)
                    else {
                        return;
                    };
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// MD-B3, R942: the row to fill. Default units; backrow fills the backrow (Meditative #26).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<Row>,
    /// MD-B3, R942: the sides to fill. Default the one `player`; `any` fills both sides, the
    /// active player's side first (R68), each zone for the player whose zone it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<ScopeSide>,
    /// MD-B3, R942: the side whose summons are Radiant (Meditative #26's Radiant face).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant_for: Option<PlayerSpec>,
    /// ME-CN, R1185002: every card the fill summons is in Chinese (M+ #21 Harmony Spirit's Radiant
    /// face, "Rush Tokens in Chinese").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chinese: Option<bool>,
}

/// §7 and R64: "fill your board" summons the named token into every empty, unreserved unit zone, left to
/// right, a Locked one included (`fill_board_zones`, R1136000), then, for a Flying token, every such
/// backrow zone, standing (R2248000). `row: backrow` fills only unlocked backrow zones (MD-B3, R942).
pub fn fill_board(args: FillBoardArgs) -> Effect {
    Effect::new("fillBoard", move |ctx| {
        let row = args.row.unwrap_or(Row::Units);
        let players: Vec<PlayerId> = match args.side {
            Some(side) => sides_of(ctx, Some(side)),
            None => vec![player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide))],
        };
        for player in players {
            let probe = face_probe(&args.def_id, player, args.radiant == Some(true));
            if row_of(card_type_of(ctx.state, &probe)) != Some(row) {
                continue;
            }
            let radiant = args.radiant == Some(true)
                || args.radiant_for.map(|spec| player_of(ctx, spec)) == Some(player);
            let slots = match row {
                Row::Units => fill_board_zones(ctx.state, player),
                Row::Backrow => open_zones(ctx.state, player, row),
            };
            let at = |lane: i32| SummonPlacement {
                lane: Some(lane),
                radiant: if radiant { Some(true) } else { None },
                stats_override: args.stats_override,
                armor_override: args.armor_override,
                chinese: args.chinese,
                ..SummonPlacement::default()
            };
            for slot in slots {
                summon_fresh(ctx, &args.def_id, player, &at(slot.lane));
            }
            if row == Row::Units && summon_flies(ctx.state, &probe) {
                for lane in 1..=row_size(Row::Backrow) {
                    summon_fresh_standing(ctx, &args.def_id, player, &at(lane));
                }
            }
        }
    })
}

/// `fillBoardRandom`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FillBoardRandomArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<CatalogQueryArgs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// R1080 (M #51): "fill your board" with a pick from the match rng (R60) for each unit zone `fill_board`
/// fills (R64, R1136000), repeats allowed, from `summon_random`'s pool (every set, R380, but the card
/// that generated it, R387) kept to Units. With no zone to fill nothing is drawn (R129).
pub fn fill_board_random(args: FillBoardRandomArgs) -> Effect {
    Effect::new("fillBoardRandom", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let slots = fill_board_zones(ctx.state, player);
        if slots.is_empty() {
            return;
        }
        let own = ctx
            .self_
            .as_ref()
            .map(|me| me.def_id.clone())
            .or_else(|| ctx.def_id.clone());
        let asked: CatalogQueryArgs = args.query.clone().unwrap_or_default();
        let pool: Vec<_> = query(&excluding_def_id(Some(&*ctx.state), &asked, own.as_deref()))
            .into_iter()
            .filter(|def| def.type_ == CardType::Unit)
            .collect();
        if pool.is_empty() {
            return;
        }
        for slot in slots {
            let Some(def_id) = pick_generated(ctx.sink.rng, &pool, None).map(|def| def.id.clone()) else {
                return;
            };
            let at = SummonPlacement {
                lane: Some(slot.lane),
                radiant: args.radiant,
                ..SummonPlacement::default()
            };
            summon_fresh(ctx, &def_id, player, &at);
        }
    })
}

/// `summonRandomFromHand`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SummonRandomFromHandArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// R1083 (M #63): summon one random Unit card from your hand, summoning sick, where `summon` places it
/// (R64, R2248000). The pick is drawn only when a zone is open and a Unit is in hand (R129), the opponent
/// sees nothing until it lands (R177), and on the Radiant face the Unit is made Radiant once there.
pub fn summon_random_from_hand(args: SummonRandomFromHandArgs) -> Effect {
    Effect::new("summonRandomFromHand", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let units: Vec<_> = ctx.state.players[player]
            .hand
            .iter()
            .filter(|card| card_type_of(ctx.state, card) == CardType::Unit)
            // R2533000: the pick is among the Units the hand may play.
            .filter(|card| !crate::state::barred_from_play(ctx.state, player, card))
            .cloned()
            .collect();
        // R129: no open zone draws nothing — not even the pick; R2248000: with a Flying Unit in hand an
        // open backrow zone is one, since that Unit may land there.
        let a_flier_lands = || {
            first_entry_zone(ctx.state, player, Row::Backrow).is_some()
                && units.iter().any(|card| summon_flies(ctx.state, card))
        };
        if open_zones(ctx.state, player, Row::Units).is_empty() && !a_flier_lands() {
            return;
        }
        let Some(picked) = ctx.sink.rng.pick(&units).cloned() else {
            return;
        };
        let source_id = ctx.self_.as_ref().map(|me| me.id.clone());
        let at = SummonPlacement {
            source_id,
            ..SummonPlacement::default()
        };
        let Some(moved) = summon_existing(ctx, &picked, player, &at) else {
            return;
        };
        if args.radiant == Some(true) {
            // R174: aimed at the stay the card has just arrived on, as `roll_random_keywords` aims.
            let effect = set_radiant(json_as(json!({ "instanceId": moved.id })));
            let before = ctx.exits_from;
            ctx.exits_from = Some(exit_mark(ctx.state));
            (effect.apply)(ctx);
            ctx.exits_from = before;
        }
    })
}
