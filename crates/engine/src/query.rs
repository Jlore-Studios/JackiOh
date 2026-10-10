//! The read-only board queries a card script asks its questions with (BUILD M3-T1, SPEC §10.9).
//!
//! §10.9 lets a hook READ state to compute an effect's arguments and forbids it writing; BUILD M3-T1
//! asks, more strictly, that `grep -r "state.players[" packages/cards` come back empty. A raw
//! `ctx.state.players[…]` read satisfies the spec and fails the acceptance grep, and the grep is
//! right for a reason the spec does not state: fifteen card files spelling out the shape of
//! `PlayerState` means `PlayerState` cannot change shape without editing fifteen card files. This
//! module is the fix — the read half of the card-facing surface, next to `effects/mod.rs`,
//! which is the write half.
//!
//! So: a verb a card needs lives in `effects`; a FACT a card needs lives here. The existing
//! readers a card file already imports off the root stay where they are and are part of the
//! same surface — `active_units_of`, `dormant_units_of`, `card_at`, `slots_of`, `slot_of` (`zones.rs`, which
//! owns the field because it owns the lanes), `face_of`, `stats_with_buffs` (`layers.rs`, §10.4),
//! `def_of` (`catalog.rs`), `find_instance` (`state.rs`). What had no home was everything ABOUT a
//! player rather than about a card or a lane: the hero block, the off-field piles and the turn log.
//!
//! Why a module of its own rather than more of `zones.rs`: `zones.rs` is a mutator module — it owns
//! `place_on_field`, `move_to_zone`, `lock_zone`, `reset_instance` — and its readers are there because
//! they read the thing it writes. The hero block and the turn log are not zones at all, and putting
//! a read-only surface among the writers hides the one property that makes it a surface: nothing
//! here can change the game. Every function below takes `(state, player, …)`, returns numbers,
//! booleans or a fresh copy, and holds no reference the caller can write through.
//!
//! NAMING: `catalog.rs` owns the CARD-POOL query of §5.1 (`catalog::query`, wrapped for cards by
//! `jackioh_cards::query`). This module queries the BOARD. Two different questions; the
//! names below say which is which, and neither exports a bare `query`.
//!
//! Port of `packages/engine/src/query.ts`.

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::HAND_CAP;
use crate::damage::DamageTarget;
use crate::layers::{face_of, unit_has};
use crate::script::{EffectContext, FlagOrCount, StaticFlags};
use crate::state::{CardInstance, FaceUpRecord, GameState, ModifierKind, PlayRecord, find_instance};
use crate::wire::{AttackHealth, CardType, GameEvent, KeywordKind, PlayerId, Row, Tag, Zone, opponent_of};
use crate::zones::{OffFieldZone, active_units_of, card_at, slot_of, slots_of};

/// A hero's block as a card may see it: §10.1's `{ health, armor }`, copied, so a script cannot
/// write a hero's health by assigning through the result (which the acceptance grep could not catch
/// and `state_check` would then disagree with). Not the view's `wire::HeroView`; `lib.rs` resolves the
/// root name to this one.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct HeroView {
    pub health: i32,
    pub armor: i32,
}

/// TS `CardInstance | string | null`: a card, its instance id, or no card (`played_earlier`,
/// `was_played_this_turn`). `From` lets a caller hand over whichever it holds.
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

/// §10.1's hero block for one player. `health` is the current total that §4.4 damages and §2.6 reads
/// for the loss check; `armor` is the stored field §4.4 step 2 subtracts, not a computed total, so a
/// card that wants "the Armor this hero has right now" wants the damage pipeline's own reader and
/// not this (#73, #84).
pub fn hero_of(state: &GameState, player: PlayerId) -> HeroView {
    let hero = &state.players[player].hero;
    HeroView {
        health: hero.health,
        armor: hero.armor,
    }
}

/// The live pile, for this module's own use only. The zone -> pile mapping is `zones.rs`'s
/// (`pile_for`), which is private there because handing a card file a live pile hands it a `push`
/// into a zone; every export below copies before it returns.
fn pile_of(state: &GameState, player: PlayerId, zone: OffFieldZone) -> &Vec<CardInstance> {
    let side = &state.players[player];
    match zone {
        OffFieldZone::Hand => &side.hand,
        OffFieldZone::Library => &side.library,
        OffFieldZone::Graveyard => &side.graveyard,
        _ => &side.exile,
    }
}

/// One of a player's off-field piles, in zone order, as a copy: `library[0]` is the top (the next
/// card drawn, `draw.rs`), a hand and a graveyard are in arrival order, and an exile pile is in
/// exile order (§3). The field is not a pile — `active_units_of`, `dormant_units_of` and `card_at` read
/// the lanes, because R13 makes the top of a Stack the only card that acts.
///
/// The list is fresh, so iterating it is safe while the effects it builds resolve and mutate the
/// real pile (#83 replaces every card in three of them).
pub fn zone_cards(state: &GameState, player: PlayerId, zone: OffFieldZone) -> Vec<CardInstance> {
    pile_of(state, player, zone).clone()
}

/// How many cards are in one of a player's off-field piles (§3).
pub fn zone_count(state: &GameState, player: PlayerId, zone: OffFieldZone) -> i32 {
    pile_of(state, player, zone).len() as i32
}

/// §2.4, R4, R1143: the most cards this player's hand holds before a card arriving is burned —
/// `HAND_CAP`, or the hand size an effect set for the rest of the game (Meditative #79). Every rule
/// that reads the hand cap reads it here: draws, adds, steals, "fill your hand". Public (§10.8).
pub fn hand_cap_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hand_cap.unwrap_or(HAND_CAP)
}

/// The mana this player holds now and has not spent: `turnEnded.unspentMana` is this number as the
/// turn ends (R62), which #18 Bread and Butter's X reads, so its preview (R280) reads the same number
/// off the same function for the turn as it stands. Mana is public (§10.8).
pub fn unspent_mana_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].mana.current
}

/// §10.5 step 4's count of the cards this player has played this turn, cleared by `start_turn`.
/// The card being played is already counted when its own script and any `cardPlayed` trigger run —
/// §6.2's Combo X reads "played EARLIER this turn", so a card counting the plays before itself
/// subtracts one (#38).
pub fn cards_played_this_turn(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].turn_log.cards_played
}

/// The instance ids this player has played this turn, in play order, as a copy. The ids are enough
/// to name the cards: `find_instance` turns one into its instance wherever it has landed since (R98).
pub fn played_ids_this_turn(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].turn_log.played_ids.clone()
}

/// §6.2 Combo X: how many cards this player played earlier this turn than this card's play — its
/// place in the turn's log, which §10.5 step 4 wrote as it played the card ("`turnLog.cardsPlayed`
/// checked at play time", §8 #10). A card the play itself goes on to cast (a cast-on-draw card that
/// /fullsend's Combo draw takes at step 5, R70) is played after it, never earlier, so the count does
/// not move while the play resolves. A card played twice this turn is counted from its latest play.
/// A card still in a hand, or one the log does not hold, has not been played: every play this turn
/// is earlier than the one it would be. No card (`CardOrId::Nothing`) is a script running with no
/// instance (`ctx.self` of a card that has ceased to exist, R127): its play is taken as the latest one
/// in the log, so every play but that one is earlier.
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

/// Whether this player played this card this turn — the one-shot gate §5.1's "End of turn: add this
/// back to your hand" spells need (#23, #24, #31, R68): the card returns on the turn it was played
/// and not at every end of turn thereafter, and a copy that reached the graveyard by being discarded
/// or milled was never played and stays there. `player` is explicit because a card's owner and its
/// controller can differ and each card says which log it means.
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

/// R429, R766 (issues #557, #572): the price a Spell whose return keeps it (`StaticFlags.returnKeepsPrice`,
/// #31 KY's Math Equation) has had from its own returns — its climb, and no other change to its price —
/// as it carries it in a hand and into its play, and as §10.5 step 7 notes it again while it lies in its
/// graveyard flagged for its end-of-turn return (R155). 0 for any other card, and for one that has not
/// climbed (`resolve::RETURN_PRICE_KEY`).
pub fn return_price_of(card: &CardInstance) -> i32 {
    card.memory
        .get(crate::resolve::RETURN_PRICE_KEY)
        .and_then(Value::as_i64)
        .map_or(0, |price| price as i32)
}

/// R427, R174: whether the card a play's `cardResolved` names has left the field since the play
/// resolved — taken off it by something answering the play, an earlier trap of the same dispatch —
/// rather than during its own resolution, before the event (its `permanent` was already false then).
/// #41 Sheepish answers a Unit that left in its own resolution, and is not offered one a trap before
/// it already took (the stays `traps::standing_event` judges, §10.3). Any other event: false.
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

/// What the card running a script remembers under `key` (§10.1: #22 Carnivorous Cube's meal), read
/// the way `effects/memory::remember` wrote it. On a fused card each ingredient remembers apart (R102),
/// so an ingredient reads its own and nothing else — two Cubes crafted into one card copy two meals —
/// and a card a Fuse kept reads what it remembered before at the path its text now runs at, where the
/// Fuse moved it (`work::reroot_remembered`, R77), while the texts fused onto it read nothing of it. The
/// value is handed back as stored, JSON, for the card to read defensively.
pub fn recalled(ctx: &EffectContext<'_>, key: &str) -> Option<Value> {
    // TS read the live `ctx.self`, so a memory an earlier effect of the list wrote is read here.
    let card = ctx.live_self()?;
    card.memory
        .get(&crate::work::part_memory_key(&ctx.data, key))
        .cloned()
}

/// R42, R361: the Unit that destroyed a card, asked by the card's own Death hook (#86 "Miss" Mrow's
/// "take control of the Unit that destroyed this"). `card` is the dying card as the hook reads it —
/// its last-known state (R78), which still carries R42's credit for the hit that doomed it. The
/// answer is the card that dealt that hit only while it is a Unit acting on the field: the top of its
/// pile, never a card dormant under a Stack (R13). So a card no hit doomed — destroyed by an effect,
/// tributed, starved by an aura — names nobody (R42), and so does one a Spell's damage killed, or
/// whose killer has since died or left the field (a mutual combat death, R78). The card is handed
/// back for its id; a card file reads it and never writes it (CLAUDE.md rule 5).
pub fn killer_of<'a>(state: &'a GameState, card: Option<&CardInstance>) -> Option<&'a CardInstance> {
    let id = card?.last_damaged_by.as_ref()?;
    let killer = find_instance(state, id)?;
    if !matches!(killer.zone, Zone::Field { row: Row::Units, .. }) {
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

// ---- v0.2.0 board facts, by workstream: generation (B5 E19, R471) ----
// Plague Counters are read here like every other board fact; the counter's rules are `plague.rs`'s.
//   `plague_on(card)`             the tokens on one permanent (Classic #39, #43, #69, #87; C+ #3)
//   `plague_on_field(state, p?)`  every token on the field, or one side's (Classic #59)
//   `permanents_on_field(state)`  every permanent on the field in R68's order, face-down included
//   `plague_multiplier_of(s, c)`  what a placement onto that card is multiplied by (Classic #27)
pub use crate::plague::{permanents_on_field, plague_multiplier_of, plague_on, plague_on_field};

// ---------------------------------------------------------------------------
// v0.2.0 readers: play pipeline A (B5 E1 announces, E4 play counters, R448, R451)
// ---------------------------------------------------------------------------

/// B5 E4: how many cards this player has played this turn of the given types — the type each was
/// played as (B2.7) — casts included (R70), countered plays never (R448). Counted on both players'
/// turns and cleared with the rest of "this turn" at every start of turn (Classic+ #37 Wardrum's
/// Spells, Field Spells and Traps, a Field Trap being a Trap, §5.1). (TS took one type or several:
/// one is `&[t]`.)
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

/// MD-B15, R923: every tag a card instance carries — its definition's tags, then each granted tag
/// (`CardInstance.granted_tags`, Meditative #35) it lacks, in order. Every instance-level tag read
/// goes through this; catalog pools read definitions and never see granted tags.
pub fn tags_of(state: &GameState, card: &CardInstance) -> Vec<Tag> {
    let mut tags: Vec<Tag> = crate::catalog::def_of(Some(state), &card.def_id).tags.clone();
    if let Some(granted) = card.granted_tags.as_deref() {
        for tag in granted {
            if !tags.contains(tag) {
                tags.push(*tag);
            }
        }
    }
    tags
}

/// §6.1, R1438: the Lucky X a card instance has — the Lucky its running face prints, as a Degrade or
/// an Upgrade has moved it (B3.4), plus every Lucky it was given (a granted keyword, B5 E38) — or 0.
/// Every luck-based roll a card makes reads its Lucky here, so given Lucky counts wherever printed
/// Lucky does. The auras of the field are the unit's (`unit_view`), not the card's.
pub fn lucky_on(state: &GameState, card: &CardInstance) -> i32 {
    crate::tuning::numbered_sum(&crate::layers::card_keywords(state, card), KeywordKind::Lucky).unwrap_or(0)
}

/// MD-B6, R943: whether the card is Created — minted after the decks were built.
pub fn is_created(card: &CardInstance) -> bool {
    card.created == Some(true)
}

/// MD-B6, R943: the ids of the Created cards in the player's library, in order (index 0 is the
/// top, the next card drawn).
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

/// B5 E4, R451: the last Spell either player played (Classic #57 Echo) — as its play recorded it, so a
/// played Echo is the Spell it copied — or `None` before any. A copy, so a script cannot write it.
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

/// B5 E1, R448: whether the play a `cardAnnounced` names can still be countered — its window is open
/// and no Counter has cancelled it yet. A response that asks before it counters (Classic #4 Palantir)
/// reads this; the engine already offers a cancelled announce to no trap and runs no trigger on it.
pub fn play_still_announced(state: &GameState, instance_id: &str) -> bool {
    crate::announce::is_announce_live(state, instance_id)
}

// ---------------------------------------------------------------------------
// v0.2.0 readers: the Classic #1–#45 cards (card-specific, the cards-classic-a workstream)
// ---------------------------------------------------------------------------

/// §2.3's max mana, which the refresh fills current mana to: Classic #36 Burn's Radiant face, "if your
/// max mana is 4 or more" (SPEC §8.6 row 36). Mana is public (§10.8), so a `conditionMet` may read it.
pub fn max_mana_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].mana.max
}

// ---------------------------------------------------------------------------
// The yellow glow's facts for the cards R195 left out (R662)
// ---------------------------------------------------------------------------

/// Every permanent that acts for this player: the tops of their unit piles and their backrow (§3.2).
fn permanents_held_by(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    [Row::Units, Row::Backrow]
        .into_iter()
        .flat_map(|row| slots_of(player, row))
        .filter_map(|slot| card_at(state, slot).cloned())
        .collect()
}

/// R662: whether a card this player plays now from their hand would take a Combo branch a permanent
/// or a modifier grants it — #38 Quickstriker's "Combo X: deal X damage" (its flag on a permanent of
/// theirs, or a `quickstrikerDamage` rider), #78 /fullsend's Radiant "Combo: draw 1" (a `comboDraw`
/// rider). §10.5 step 5 resolves both only when the play has a card played earlier this turn before it
/// (`play_steps::quickstriker_combo`, `combo_draw_step`), and a card still in a hand has every play this
/// turn before it (`played_earlier`), so this is "one of them is in place, and a card has been played".
/// Plays and modifiers are public (§10.8).
pub fn granted_combo_live(state: &GameState, player: PlayerId) -> bool {
    if cards_played_this_turn(state, player) < 1 {
        return false;
    }
    let granted = permanents_held_by(state, player).iter().any(|held| {
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
/// they played it now — the question §10.5 step 3 asks (`gifted_makes_radiant`), with the cost a play
/// of it pays now (R56). A card that is Radiant already gains nothing, so it is never one.
pub fn gifted_would_make_radiant(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    if card.radiant {
        return false;
    }
    crate::play_choices::gifted_makes_radiant(state, player, crate::mana::play_cost(state, card))
}

/// R662, R44: the units acting on the other side whose attack on this player's hero would be lethal
/// now, by #96 My Pawn's own projection (`subsystems::is_lethal`: after Armor and the Anti-oneshot cap,
/// net of a Lifesteal strike back). It asks whether the blow is on the board, not whether it can be
/// declared this moment. Stats, keywords and health are public (§10.8).
pub fn lethal_attackers_of(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    let target = DamageTarget::Hero { player };
    active_units_of(state, opponent_of(player))
        .into_iter()
        .filter(|&unit| crate::subsystems::lethal::is_lethal(state, unit, &target))
        .cloned()
        .collect()
}

/// R662: the permanents of this player's that #85 Unlicensed Experimentation could fuse a played
/// permanent onto — every one acting for them but `except` (the trap itself) that is not Immutable
/// (R23). The types are the trap's to match when a permanent is played.
pub fn fusable_permanents_of(state: &GameState, player: PlayerId, except: Option<&str>) -> Vec<CardInstance> {
    permanents_held_by(state, player)
        .into_iter()
        .filter(|held| Some(held.id.as_str()) != except && !unit_has(state, held, KeywordKind::Immutable))
        .collect()
}

/// Whether any card acting for this player carries the picked static flag — the active units and
/// the face-up backrow tops, as `damage.rs`'s `acting_texts_of` walks them (a face-down Trap's text
/// is in nobody's use until it fires, R33). What the Pareto judge and the emote gate ask (MD-D28,
/// MD-D29, R1125, R1127).
pub fn acting_with_flag(state: &GameState, player: PlayerId, pick: fn(&StaticFlags) -> Option<bool>) -> bool {
    let mut acting: Vec<CardInstance> = active_units_of(state, player).into_iter().cloned().collect();
    for slot in slots_of(player, Row::Backrow) {
        let Some(card) = card_at(state, slot) else {
            continue;
        };
        let card_type = crate::faces::card_type_of(state, card);
        let face_down =
            (card_type == CardType::Trap || card_type == CardType::FieldTrap) && card.face_up != Some(true);
        if !face_down {
            acting.push(card.clone());
        }
    }
    acting
        .iter()
        .any(|card| pick(&crate::scripts::flags_of(state, card)) == Some(true))
}

/// MD-D29, R1127: whether an acting card of the opponent hears emotes, so an `Emote` action is legal.
pub fn emotes_heard(state: &GameState, player: PlayerId) -> bool {
    acting_with_flag(state, opponent_of(player), |flags| flags.hears_emotes)
}

/// MD-D19, R1122: the acting Units adjacent to the attacker on its own side that it may attack, in
/// lane order — the neighbours a redirect may re-aim the attack at. Taunt is ignored: the target
/// must pass §4.2 step 2's restrictions, never step 3's wall.
pub fn redirect_neighbours(state: &GameState, attacker: &CardInstance) -> Vec<CardInstance> {
    let Some(at) = slot_of(state, attacker) else {
        return Vec::new();
    };
    crate::zones::adjacent(at)
        .into_iter()
        .filter_map(|slot| card_at(state, slot).cloned())
        .filter(|unit| {
            crate::zones::acts_on_field(state, unit)
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

/// R901 (Meditative #30 Fickle E-Kitten's "a more expensive permanent than you"): the highest cost
/// among the permanents acting for this player, the tops of their unit piles and their backrow (R13),
/// each read at R396's `cost_now`: an X card at the X it was played for, 0 without one, any other at
/// its cost as it stands. A face-down card is never an X card, so it counts at `effective_cost`, the
/// cost R351's view shows both players. `None` when the side holds no permanent.
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
