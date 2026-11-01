//! The read-only board queries a card script asks its questions with (BUILD M3-T1, SPEC §10.9).
//!
//! §10.9 forbids a hook writing state, and BUILD M3-T1 bars card files from reading `state.players[…]`,
//! so `PlayerState` can change shape without editing cards. A verb a card needs lives in `effects`, a
//! fact here or among the root's other readers (`zones.rs`'s, `layers.rs`'s (§10.4), `def_of` and
//! `find_instance`). Nothing here can change the game: every function takes `(state, player, …)` and
//! returns numbers, booleans or a fresh copy, no reference the caller can write through.
//!
//! `catalog::query` (wrapped by `jackioh_cards::query`) asks §5.1's card pool, this module the board;
//! neither exports a bare `query`.

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{CHINESE_TAG, HAND_CAP, LIBRARY_CAP};
use crate::damage::DamageTarget;
use crate::layers::{AuraTable, face_of, unit_has};
use crate::script::{EffectContext, FlagOrCount, StaticFlags};
use crate::state::{CardInstance, FaceUpRecord, GameState, ModifierKind, PlayRecord, find_instance};
use crate::wire::{AttackHealth, CardType, GameEvent, KeywordKind, PlayerId, Row, Tag, Zone, opponent_of};
use crate::zones::{OffFieldZone, active_units_of, card_at, slot_of};

/// A hero's block as a card may see it (§10.1), copied so a script cannot write through it. Not the
/// view's `wire::HeroView`; `lib.rs` resolves the root name to this one.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct HeroView {
    pub health: i32,
    pub armor: i32,
}

/// A card, its instance id, or no card (`played_earlier`, `was_played_this_turn`).
#[derive(Clone, Copy, Debug)]
pub enum CardOrId<'a> {
    Card(&'a CardInstance),
    Id(&'a str),
    Nothing,
}

impl<'a> From<&'a CardInstance> for CardOrId<'a> {
    fn from(card: &'a CardInstance) -> CardOrId<'a> {
        CardOrId::Card(card)
    }
}

impl<'a> From<&'a str> for CardOrId<'a> {
    fn from(id: &'a str) -> CardOrId<'a> {
        CardOrId::Id(id)
    }
}

impl<'a> From<&'a String> for CardOrId<'a> {
    fn from(id: &'a String) -> CardOrId<'a> {
        CardOrId::Id(id)
    }
}

impl<'a> From<Option<&'a CardInstance>> for CardOrId<'a> {
    fn from(card: Option<&'a CardInstance>) -> CardOrId<'a> {
        match card {
            Some(card) => CardOrId::Card(card),
            None => CardOrId::Nothing,
        }
    }
}

/// §10.1's hero block for one player (`health` is what §2.6's loss check reads). `armor` is the field
/// §4.4 step 2 subtracts, not a computed total: Armor right now is the damage pipeline's (#73, #84).
pub fn hero_of(state: &GameState, player: PlayerId) -> HeroView {
    let hero = &state.players[player].hero;
    HeroView {
        health: hero.health,
        armor: hero.armor,
    }
}

/// R290, R180: the health this player's hero started the game with: its seat's handicap `heroHealth`,
/// else HERO_HEALTH (Stinkyscape #18.2 BALTHASAR, R1866002).
pub fn starting_health_of(state: &GameState, player: PlayerId) -> i32 {
    crate::state::starting_hero_health(state.players[player].handicap.as_ref())
}

/// The live pile, for this module only: `zones.rs` keeps `pile_for` private because a live pile hands
/// a card file a `push` into a zone, so every export below copies.
fn pile_of(state: &GameState, player: PlayerId, zone: OffFieldZone) -> &Vec<CardInstance> {
    let side = &state.players[player];
    match zone {
        OffFieldZone::Hand => &side.hand,
        OffFieldZone::Library => &side.library,
        OffFieldZone::Graveyard => &side.graveyard,
        _ => &side.exile,
    }
}

/// One of a player's off-field piles as a copy, in zone order: `library[0]` is the top (the next card
/// drawn), the others in arrival order (§3), safe to iterate while effects mutate the real pile (#83
/// replaces every card in three of them).
pub fn zone_cards(state: &GameState, player: PlayerId, zone: OffFieldZone) -> Vec<CardInstance> {
    pile_of(state, player, zone).clone()
}

pub fn zone_count(state: &GameState, player: PlayerId, zone: OffFieldZone) -> i32 {
    pile_of(state, player, zone).len() as i32
}

/// §2.4, R4, R1143: the most cards this player's hand holds before a card arriving is burned —
/// `HAND_CAP`, or the hand size an effect set for the rest of the game (Meditative #79). Whether the
/// hand has room is `room_in`'s question, not this one's. Public (§10.8).
pub fn hand_cap_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hand_cap.unwrap_or(HAND_CAP)
}

/// §2.4, R4, R80, R1143: how many more cards this player's `pile` takes before the next one is burned
/// (a hand, at its `hand_cap_of`) or turned away (a library, at `LIBRARY_CAP`). It is the one test
/// of a full pile: the two arrivals that enforce the caps (`draw::add_to_hand`, the library's
/// shuffle-in) and every rule that asks first — "fill your hand", "your hand is full", a buy, a pass,
/// a Trade — read it here. Never below 0: a hand already above a lowered cap keeps its cards and has
/// no room (R1143). A graveyard or an exile pile has no cap, so it always has room (`i32::MAX`).
/// Public (§10.8).
pub fn room_in(state: &GameState, player: PlayerId, pile: OffFieldZone) -> i32 {
    let cap = match pile {
        OffFieldZone::Hand => hand_cap_of(state, player),
        OffFieldZone::Library => LIBRARY_CAP,
        OffFieldZone::Graveyard | OffFieldZone::Exile => return i32::MAX,
    };
    (cap - zone_count(state, player, pile)).max(0)
}

/// R2250000: whether this player's hand holds a card of one of these definitions ("if you have a
/// Narcor", Stinkyscape #39's Activate).
pub fn hand_holds_any(state: &GameState, player: PlayerId, def_ids: &[&str]) -> bool {
    state.players[player]
        .hand
        .iter()
        .any(|card| def_ids.contains(&card.def_id.as_str()))
}

/// R2250000: copies of the Resources in this player's hand, in hand order. Its owner reads them, the
/// other player the hand's count (§10.8), so a hook asks it of its own controller.
pub fn resources_in_hand_of(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    state.players[player]
        .hand
        .iter()
        .filter(|card| crate::resources::is_resource_card(state, card))
        .cloned()
        .collect()
}

/// R1881007: whether a card of one of these definitions is mid-resolution (§10.5's `resolving`) for this
/// player, as S #138 Pocket Peptides asks of its controller's CN Peptides cast.
pub fn is_resolving(state: &GameState, player: PlayerId, def_ids: &[&str]) -> bool {
    state.players[player]
        .resolving
        .iter()
        .any(|card| def_ids.contains(&card.def_id.as_str()))
}

/// R1861001: "if Brittle destroyed this" (Stinkyscape #39 The Narcor Project): the card's Brittle count
/// in force is spent, at 0. A Death hook reads it off its snapshot, taken before R78's reset drops the
/// spent count (`state_check.rs`'s collect, R441), so only a crumble's death reads true.
pub fn brittle_spent(card: &CardInstance) -> bool {
    crate::brittle_count::active_brittle_count(card) == Some(0)
}

/// R1840005: this player's Credits, 0 until first gained. Public (§10.8).
pub fn market_credits_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].market_credits.unwrap_or(0)
}

/// The mana this player holds unspent now, which `turnEnded.unspentMana` records as the turn ends (R62):
/// #18 Bread and Butter's X and its preview (R280) read the same number. Mana is public (§10.8).
pub fn unspent_mana_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].mana.current
}

/// §10.5 step 4's count of this player's plays this turn (cleared by `start_turn`), the card being
/// played included, so a Combo X (§6.2, "played EARLIER") counting plays before it subtracts one (#38).
pub fn cards_played_this_turn(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].turn_log.cards_played
}

/// The ids this player has played this turn, in order, as a copy (`find_instance` finds each, R98).
pub fn played_ids_this_turn(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].turn_log.played_ids.clone()
}

/// §6.2 Combo X: how many cards this player played this turn before this card's play, its place in
/// the log §10.5 step 4 wrote (§8 #10). A card the play itself casts (R70) comes after it; a card
/// played twice counts from its latest play; for a card in a hand or not in the log every play is
/// earlier; no card (`ctx.self` gone, R127) is the log's latest play.
pub fn played_earlier<'a>(state: &GameState, player: PlayerId, card: impl Into<CardOrId<'a>>) -> i32 {
    let log = &state.players[player].turn_log;
    let (id, instance): (&str, Option<&CardInstance>) = match card.into() {
        CardOrId::Nothing => return (log.cards_played - 1).max(0),
        CardOrId::Card(card) => (card.id.as_str(), Some(card)),
        CardOrId::Id(id) => (id, find_instance(state, id)),
    };
    if instance.is_some_and(|instance| matches!(instance.zone, Zone::Hand { .. })) {
        return log.cards_played;
    }
    match log.played_ids.iter().rposition(|played| played == id) {
        Some(at) => at as i32,
        None => log.cards_played,
    }
}

/// Whether this player played this card this turn: the one-shot gate of §5.1's "End of turn: add this
/// back to your hand" spells (#23, #24, #31, R68); a copy discarded or milled into the graveyard was
/// never played. `player` is explicit, as a card's owner and controller can differ.
pub fn was_played_this_turn<'a>(state: &GameState, player: PlayerId, card: impl Into<CardOrId<'a>>) -> bool {
    let id = match card.into() {
        CardOrId::Card(card) => card.id.as_str(),
        CardOrId::Id(id) => id,
        CardOrId::Nothing => return false,
    };
    state.players[player]
        .turn_log
        .played_ids
        .iter()
        .any(|played| played == id)
}

/// R429, R766 (#557, #572): the climb a Spell whose return keeps its price (`StaticFlags.returnKeepsPrice`,
/// #31 KY's Math Equation) has had from its own returns alone, in hand, in play and in its graveyard
/// awaiting its end-of-turn return (§10.5 step 7, R155); else 0 (`resolve::RETURN_PRICE_KEY`).
pub fn return_price_of(card: &CardInstance) -> i32 {
    card.memory
        .get(crate::resolve::RETURN_PRICE_KEY)
        .and_then(Value::as_i64)
        .map_or(0, |price| price as i32)
}

/// R427, R174: whether the card a play's `cardResolved` names has left the field since it resolved,
/// taken by something answering the play (an earlier trap of the same dispatch), not in its own
/// resolution, which #41 Sheepish answers (`traps::standing_event`, §10.3). Any other event: false.
pub fn left_field_since_resolved(state: &GameState, event: &GameEvent) -> bool {
    match event {
        GameEvent::CardResolved {
            exits_from: Some(mark),
            instance_id,
            ..
        } => crate::stays::left_field_after(state, *mark, instance_id),
        _ => false,
    }
}

/// R2179002: whether a `cardPlayed` or `cardResolved` reports a cast on draw, which is no play, so a card
/// answering a play passes it by. Any other event: false.
pub fn cast_on_draw_event(event: &GameEvent) -> bool {
    matches!(
        event,
        GameEvent::CardPlayed {
            on_draw: Some(true),
            ..
        } | GameEvent::CardResolved {
            on_draw: Some(true),
            ..
        }
    )
}

/// R174, R2199007: the field's departures so far, a mark to ask `left_field_since_mark` with later
/// (Training Grounds #31 Level Up!: a Unit that left and came back is a new arrival).
pub fn field_exit_mark(state: &GameState) -> u32 {
    crate::stays::exit_mark(state)
}

/// R174: whether `instance_id` has left the field since `mark` (`field_exit_mark`), a Reborn body back
/// under the same id included.
pub fn left_field_since_mark(state: &GameState, mark: u32, instance_id: &str) -> bool {
    crate::stays::left_field_after(state, mark, instance_id)
}

/// What the card running a script remembers under `key` (§10.1: #22 Carnivorous Cube's meal), as stored
/// JSON: a fused card's ingredients remember apart (R102), and a card a Fuse kept reads it where the Fuse
/// moved it (`work::reroot_remembered`, R77).
pub fn recalled(ctx: &EffectContext<'_>, key: &str) -> Option<Value> {
    // The live `ctx.self`, so a memory an earlier effect of the list wrote is read here.
    let card = ctx.live_self()?;
    card.memory
        .get(&crate::work::part_memory_key(&ctx.data, key))
        .cloned()
}

/// R42, R361: the Unit that destroyed a card, asked by its Death hook (#86 "Miss" Mrow), from the dying
/// card's last-known state (R78), which keeps R42's credit for the hit that doomed it: that hit's dealer
/// while it is a Unit atop its pile on the field (R13), else nobody (no hit doomed it, a Spell's damage
/// did, or the killer has died or left). A card file reads it and never writes it (CLAUDE.md rule 5).
pub fn killer_of<'a>(state: &'a GameState, card: Option<&CardInstance>) -> Option<&'a CardInstance> {
    let id = card?.last_damaged_by.as_ref()?;
    let killer = find_instance(state, id)?;
    // R1081000: a killer standing in the backrow is a Unit acting on the field.
    if !crate::zones::in_units_row_or_standing(state, killer) {
        return None;
    }
    if crate::faces::card_type_of(state, killer) != CardType::Unit {
        return None;
    }
    let at = slot_of(state, killer)?;
    if card_at(state, at).map(|top| top.id.clone()) != Some(killer.id.clone()) {
        return None;
    }
    Some(killer)
}

// Board facts by workstream: generation (B5 E19, R471)
// Plague Counters, read here, ruled by `plague.rs` (Classic #27, #39, #43, #59, #69, #87; C+ #3);
// `permanents_on_field` is every permanent on the field in R68's order, face-down included.
pub use crate::plague::{permanents_on_field, plague_multiplier_of, plague_on, plague_on_field};

// Readers: play pipeline A (B5 E1 announces, E4 play counters, R448, R451)

/// B5 E4: how many cards this player has played this turn of these types, each as the type it was
/// played as (B2.7), casts included (R70), countered plays never (R448), on both players' turns and
/// cleared every start of turn (Classic+ #37 Wardrum; a Field Trap is a Trap, §5.1).
pub fn played_this_turn_of_type(state: &GameState, player: PlayerId, types: &[CardType]) -> i32 {
    let counts = state.players[player].turn_log.played_by_type.as_ref();
    let wanted: IndexSet<CardType> = types.iter().copied().collect();
    wanted
        .iter()
        .map(|card_type| {
            counts
                .and_then(|counts| counts.get(card_type))
                .copied()
                .unwrap_or(0)
        })
        .sum()
}

/// MD-B15, R923, R1176000: every tag a card instance carries: its definition's, then each granted tag
/// (Meditative #35) it lacks, then CN when it is in Chinese (R1300). Every instance-level tag read goes
/// through this; catalog pools read definitions and never see a granted tag or a translation.
pub fn tags_of(state: &GameState, card: &CardInstance) -> Vec<Tag> {
    let mut tags: Vec<Tag> = crate::catalog::def_of(Some(state), &card.def_id).tags.clone();
    if let Some(granted) = card.granted_tags.as_deref() {
        for tag in granted {
            if !tags.contains(tag) {
                tags.push(*tag);
            }
        }
    }
    if card.chinese == Some(true) && !tags.contains(&CHINESE_TAG) {
        tags.push(CHINESE_TAG);
    }
    tags
}

/// §6.1, R1438: a card's Lucky X: what its running face prints, as a Degrade or Upgrade moved it (B3.4),
/// plus every Lucky it was given (B5 E38), or 0. Every luck roll reads it here, so given Lucky counts
/// wherever printed Lucky does; the field's auras are the unit's (`unit_view`), not the card's.
pub fn lucky_on(state: &GameState, card: &CardInstance) -> i32 {
    crate::tuning::numbered_sum(&crate::layers::card_keywords(state, card), KeywordKind::Lucky).unwrap_or(0)
}

/// §6.1, R1438: only the Lucky a card instance was given (B5 E38), never its printed Lucky; #23
/// Reoccurring Dream and #42 Eugenics add it to their fixed roll, so tuning never reaches it (D14).
pub fn given_lucky_on(card: &CardInstance) -> i32 {
    crate::tuning::numbered_sum(&card.granted_keywords, KeywordKind::Lucky).unwrap_or(0)
}

/// MD-B6, R943: whether the card is Created — minted after the decks were built.
pub fn is_created(card: &CardInstance) -> bool {
    card.created == Some(true)
}

/// MD-B6, R943: the ids of the Created cards in the player's library, top (next drawn) first.
pub fn created_in_library(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .library
        .iter()
        .filter(|card| is_created(card))
        .map(|card| card.id.clone())
        .collect()
}

/// B5 E4: how many cards carrying `tag` this player has played this game, casts included (R70),
/// countered plays never (R448); never reset (Classic+ #64's Fruit, AI Scaling Law's AI).
pub fn played_this_game_with_tag(state: &GameState, player: PlayerId, tag: Tag) -> i32 {
    state.players[player]
        .game_log
        .as_ref()
        .and_then(|log| log.played_by_tag.get(&tag))
        .copied()
        .unwrap_or(0)
}

/// B5 E4, R451: the last Spell either player played (Classic #57 Echo) as its play recorded it (a
/// played Echo is the Spell it copied), or `None`. A copy.
pub fn last_spell_played(state: &GameState) -> Option<PlayRecord> {
    state.last_spell.clone()
}

/// B5 E4, R451: the last face-up card this player played (AI Autocomplete), with the type it was played
/// as, or `None` before any. Traps and Field Traps are set face-down and never count, and the AI
/// generated cards are passed over (`LAST_FACE_UP_SKIPPED_TAGS`). A copy.
pub fn last_face_up_played(state: &GameState, player: PlayerId) -> Option<FaceUpRecord> {
    state.players[player]
        .game_log
        .as_ref()
        .and_then(|log| log.last_face_up_play.clone())
}

/// R1850003, R1865000, R2476000: the cards held beneath `holder` on its stay now (`effects::hold`), in
/// the order taken: Stinkyscape #9 Ticketed's permanent, #135.1 Sack's Unit, #112 Job Application's
/// resume. A card held on an earlier stay of the holder's is not its own (`stays`). Each was taken in
/// public, so its preview may name it. Copies.
pub fn held_beneath(state: &GameState, holder: &CardInstance) -> Vec<CardInstance> {
    state
        .held
        .iter()
        .flatten()
        .filter(|held| {
            held.holder_id == holder.id
                && !crate::stays::left_field_after(state, held.holder_mark, &holder.id)
        })
        .map(|held| held.card.clone())
        .collect()
}

/// R1855003, R2476000: how many of `player`'s Anomaly scans have missed this game, which the Scanning
/// Probes' chances grow by (`effects::anomaly_chances`). A miss is silent, so only the player may read
/// it: a preview of it stays in their own hand.
pub fn anomaly_misses_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].anomaly_misses.unwrap_or(0)
}

/// R1849004, R2476000: how much more `card`'s own-turn redirect (Stinkyscape #5 Masochism Unmasked)
/// may take off its controller's hero this turn: its declared `cap` (R386) less what it has taken this
/// turn, at least 0. `None` for a card with no such redirect.
pub fn own_turn_redirect_left(state: &GameState, card: &CardInstance) -> Option<i32> {
    let redirect = crate::scripts::flags_of(state, card).own_turn_redirect?;
    let cap = crate::params::declared_or(state, card, "cap", redirect.cap).max(0);
    Some((cap - crate::damage::own_turn_redirect_moved(state, card)).max(0))
}

// Readers for the Classic #1–#45 cards (card-specific)

/// §2.3's max mana, which the refresh fills current mana to: Classic #36 Burn's Radiant face, "if your
/// max mana is 4 or more" (SPEC §8.6 row 36). Public (§10.8).
pub fn max_mana_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].mana.max
}

/// R1064: every point of Armor on the field: each acting Unit's on both sides through the §10.4 layers
/// (printed, Defense Position's +1, auras) and both heroes' as §4.4 step 2 reads it (`hero_armor_of`,
/// R124). Public (§10.8).
pub fn armor_on_field(state: &GameState) -> i32 {
    crate::wire::PLAYER_IDS
        .iter()
        .map(|&player| {
            crate::damage::hero_armor_of(state, player)
                + active_units_of(state, player)
                    .into_iter()
                    .map(|unit| crate::layers::unit_view(state, unit).armor)
                    .sum::<i32>()
        })
        .sum()
}

// The yellow glow's facts for the cards R195 left out (R662)

/// Every permanent that acts for this player: the tops of their unit piles, the Units their carriers
/// hold (R446, R1824000) and their backrow (§3.2), in `zones::acting_on`'s order.
fn permanents_held_by(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    crate::zones::acting_on(state, player, crate::zones::StandingAt::ItsLane)
        .into_iter()
        .cloned()
        .collect()
}

/// R662: whether a card played now from this player's hand would take a granted Combo branch, #38
/// Quickstriker's (a permanent's flag or a `quickstrikerDamage` rider) or #78 /fullsend's Radiant draw
/// (a `comboDraw` rider). §10.5 step 5 resolves both after any earlier play this turn, and a card in
/// hand has every play before it, so: one is in place and a card has been played.
pub fn granted_combo_live(state: &GameState, player: PlayerId) -> bool {
    if cards_played_this_turn(state, player) < 1 {
        return false;
    }
    let granted = permanents_held_by(state, player).iter().any(|held| {
        // R1487000: a face-down card's text sleeps until it is revealed or face-up.
        if crate::preview::lasting_text_off(state, held) {
            return false;
        }
        match crate::scripts::flags_of(state, held).quickstriker {
            Some(FlagOrCount::Flag(flag)) => flag,
            Some(FlagOrCount::Count(count)) => count > 0,
            None => false,
        }
    });
    if granted {
        return true;
    }
    state.players[player].mods.iter().any(|modifier| {
        crate::mana::modifier_is_live(state, modifier)
            && match &modifier.kind {
                ModifierKind::QuickstrikerDamage => true,
                ModifierKind::ComboDraw { amount } => *amount > 0,
                _ => false,
            }
    })
}

/// R662, R213: whether #64 Gifted Program would make this card from this player's hand Radiant if
/// played now (§10.5 step 3's `gifted_makes_radiant`, at the cost it pays now, R56).
pub fn gifted_would_make_radiant(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    if card.radiant {
        return false;
    }
    crate::play_choices::gifted_makes_radiant(state, player, crate::mana::play_cost(state, card))
}

/// R662, R44: the units acting on the other side whose attack on this player's hero would be lethal now
/// by #96 My Pawn's projection (`subsystems::is_lethal`: after Armor and the Anti-oneshot cap, net of a
/// Lifesteal strike back): whether the blow is on the board, not whether it can be declared now.
pub fn lethal_attackers_of(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    let target = DamageTarget::Hero { player };
    active_units_of(state, opponent_of(player))
        .into_iter()
        .filter(|&unit| crate::subsystems::lethal::is_lethal(state, unit, &target))
        .cloned()
        .collect()
}

/// R987, Meditative #40 Feng Shui: the player's Luck, the sum of the "You have Luck X" auras acting on
/// their side (§3.2), each at the card's declared `luck` where it declares one (R386, as
/// `damage::hero_armor_of` reads `armor`). 0 with no Feng Shui, so every roll draws as before (D14).
pub fn luck_of(state: &GameState, player: PlayerId) -> i32 {
    let mut sum = 0;
    for card in crate::damage::acting_texts_of(state, player) {
        for text in crate::scripts::texts_of(state, &card) {
            let Some(printed) = text.flags.luck else {
                continue;
            };
            sum += crate::params::declared_or(state, &card, "luck", printed);
        }
    }
    sum
}

/// R1180003 (Meditative+ #7 Life Mage's Radiant face): the factor of a heal of this player's hero or
/// Units: `HEAL_MULTIPLIER` per "healing is twice as effective" text acting on their side (§3.2),
/// multiplied, so two make it 4; 1 with none (D9). The opponent's texts never reach these heals.
pub fn heal_multiplier_of(state: &GameState, player: PlayerId) -> i32 {
    let mut multiple: i32 = 1;
    for card in crate::damage::acting_texts_of(state, player) {
        for text in crate::scripts::texts_of(state, &card) {
            if text.flags.doubles_healing == Some(true) {
                multiple = multiple.saturating_mul(crate::config::HEAL_MULTIPLIER);
            }
        }
    }
    multiple
}

/// R662: the permanents #85 Unlicensed Experimentation could fuse a played permanent onto: every one
/// acting for this player but `except` (the trap) that is not Immutable (R23); the trap matches types.
pub fn fusable_permanents_of(state: &GameState, player: PlayerId, except: Option<&str>) -> Vec<CardInstance> {
    permanents_held_by(state, player)
        .into_iter()
        .filter(|held| Some(held.id.as_str()) != except && !unit_has(state, held, KeywordKind::Immutable))
        .collect()
}

/// Whether any card acting for this player (`damage::acting_texts_of`: active units, face-up backrow
/// tops; a face-down Trap's text sleeps until it fires, R33, or is revealed, R1487000) carries the picked
/// static flag, as the Pareto judge and the emote gate ask (MD-D28, MD-D29, R1125, R1127).
pub fn acting_with_flag(state: &GameState, player: PlayerId, pick: fn(&StaticFlags) -> Option<bool>) -> bool {
    crate::damage::acting_texts_of(state, player)
        .iter()
        .any(|card| pick(&crate::scripts::flags_of(state, card)) == Some(true))
}

/// MD-D29, R1127: whether an acting card of the opponent hears emotes, so an `Emote` action is legal.
pub fn emotes_heard(state: &GameState, player: PlayerId) -> bool {
    acting_with_flag(state, opponent_of(player), |flags| flags.hears_emotes)
}

/// MD-D19, R1122: the acting Units beside the attacker on its own side that a redirect may re-aim it at,
/// in lane order: §4.2 step 2's restrictions apply, never step 3's Taunt wall.
pub fn redirect_neighbours(state: &GameState, attacker: &CardInstance) -> Vec<CardInstance> {
    let Some(at) = slot_of(state, attacker) else {
        return Vec::new();
    };
    crate::zones::adjacent(at)
        .into_iter()
        .filter_map(|slot| card_at(state, slot).cloned())
        .filter(|unit| {
            // R1081000: beside a Unit standing in the backrow, only a Unit standing there is adjacent.
            crate::zones::is_adjacent_card(state, attacker, unit)
                && crate::zones::acts_on_field(state, unit)
                && !crate::zones::is_carried(state, unit)
                && crate::restrictions::attack_restriction(
                    state,
                    attacker,
                    &DamageTarget::Unit {
                        instance: unit.clone(),
                    },
                )
                .is_ok()
        })
        .collect()
}

/// R901 (Meditative #30 Fickle E-Kitten's "a more expensive permanent than you"): the highest R396
/// `cost_now` among the permanents acting for this player (R13, R1824000): an X card's X (0 without
/// one), a face-down card's `effective_cost` (R351's view), any other's cost as it stands; else `None`.
pub fn highest_permanent_cost(state: &GameState, player: PlayerId) -> Option<i32> {
    permanents_held_by(state, player)
        .iter()
        .map(|held| crate::mana::cost_now(state, held))
        .max()
}

/// R1181 (Meditative #94's "base stats"): §10.4 layer 1, the running face's `statsOverride` as that face
/// wears it (R349), else its X or printed stats. No buffs, tuning (layer 4), auras or damage: it is
/// `face_of` with the tuning taken back off.
pub fn base_stats_of(state: &GameState, card: &CardInstance) -> AttackHealth {
    let face = face_of(state, card);
    let tuning = card.tuning.as_ref();
    AttackHealth {
        attack: face.attack - tuning.and_then(|tuning| tuning.attack).unwrap_or(0),
        health: face.health - tuning.and_then(|tuning| tuning.health).unwrap_or(0),
    }
}

/// R1220: whether this card carries Untributable — no Tribute cost may take it, and every Sacrifice
/// of it does nothing. Read through the layers so auras apply, as the sacrifice sites do.
pub fn is_untributable(state: &GameState, card: &CardInstance) -> bool {
    is_untributable_in(&AuraTable::per_read(state), card)
}

/// `is_untributable` read through a sweep's `AuraTable` (#892).
pub fn is_untributable_in(auras: &AuraTable<'_>, card: &CardInstance) -> bool {
    auras.unit_has(card, KeywordKind::Untributable)
}

/// R1225: whether this player borrowed mana this turn (a spend went past their current mana), as
/// Jlarna's base face asks at its controller's end of turn.
pub fn credit_used_this_turn(state: &GameState, player: PlayerId) -> bool {
    state.players[player].turn_log.mana_borrowed.unwrap_or(0) > 0
}

/// R831008 (Meditative #98 Showdown's "an empty lane"): whether both players' unit and backrow zones of
/// this lane are empty. A face-down card occupies its zone: the opponent sees it taken, never by what.
pub fn lane_is_empty(state: &GameState, lane: i32) -> bool {
    [PlayerId::P1, PlayerId::P2].into_iter().all(|player| {
        [Row::Units, Row::Backrow]
            .into_iter()
            .all(|row| card_at(state, crate::zones::ZoneSlot { player, row, lane }).is_none())
    })
}

/// R1850004 (Stinkyscape #139 Wormhole, "a beneficial Spell on themselves or a permanent they control"):
/// the id of the Spell `event` announces when its player chose targets, all their own hero or cards they
/// control on the field (a face-down set announces no Spell). `None` for any other event.
pub fn beneficial_on_own_side(state: &GameState, event: &GameEvent) -> Option<String> {
    let GameEvent::CardAnnounced {
        player,
        instance_id,
        card_type,
        targets,
        face_down,
        ..
    } = event
    else {
        return None;
    };
    if *card_type != CardType::Spell || *face_down == Some(true) || targets.is_empty() {
        return None;
    }
    let own_hero = format!("hero-{player}");
    let own_side = |id: &String| {
        *id == own_hero
            || find_instance(state, id)
                .is_some_and(|card| matches!(card.zone, Zone::Field { .. }) && card.controller == *player)
    };
    if targets.iter().all(own_side) {
        Some(instance_id.clone())
    } else {
        None
    }
}
