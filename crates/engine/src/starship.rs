//! The Starship (SPEC §6.1 Starship Piece and Starship, §6.2 When launched, §6.3 Launch;
//! R1852000–R1852007): Stinkyscape's ship built from the pieces that die, ahead of the set's cards.
//!
//! - **The piece.** Starship Piece is a keyword with no rule of its own until the card dies (§4.5: the
//!   state check's collection or a Sacrifice, never a card a replacement sends elsewhere). Then, once the
//!   death pass has returned its Reborn bodies and reported its graveyard (`state_check`'s last step), a
//!   copy of it — the card as it lies in its graveyard: its printed face, on its Radiant face if it was
//!   Radiant, with its tuning and a token's X/X (R57's library copy) — fuses into its owner's Dormant
//!   Starship (R77, `subsystems::fuse`), summoning one first if they have none (R1852000, R1852002). A
//!   discard, a mill or any other way into a graveyard fuses nothing.
//! - **The hull.** The Dormant Starship (`DORMANT_STARSHIP_DEF_ID`) is a 0/0 Unit that prints Dormant
//!   (`slumber.rs`) and the hull (`StaticFlags.starship_hull`); entering the field it starts a memory
//!   (`CardInstance.starship`), the pieces it holds (R1852005). It is summoned into its owner's leftmost
//!   open unit zone, else stacked on top of their lane-3 unit zone, Locked or not, and the copy fuses into
//!   it at once, so no state check ever reads it at 0/0. A Fuse never makes it a piece (R1852001).
//! - **The launch.** While it is Dormant its one ability is the engine's launch, Activate: (3)
//!   (`launch_decl`, `STARSHIP_LAUNCH_COST`, moved by "your Starship launch costs (N)", R1852006), which
//!   awakens it; an awakening of a hull is its launch, whatever awakened it (`slumber::awaken_card`
//!   calls `launch`). It takes the Starship keyword and a random name (R1852004), `starshipLaunched`
//!   goes out, its fused Cries fire as a triggered Cry does (R467: asked as prompts for its controller),
//!   and then its fused "When launched" triggers answer the event (R1852003).
//! - **The exile.** A Stack card played onto a Dormant Starship lands and is exiled before it resolves
//!   (R1852007), the one exception to R1839003.
//!
//! Every field is optional and absent on every card that is not a Starship, so a game without one
//! serializes, hashes and replays exactly as before (D14).

use crate::config::{
    DORMANT_STARSHIP_DEF_ID, STARSHIP_FULL_ROW_LANE, STARSHIP_LAUNCH_COST, STARSHIP_LAUNCH_USES,
    STARSHIP_NAMES,
};
use crate::script::{ActivationCost, ActivationDecl, ActivationUses, EngineSink, TriggerDef, hook};
use crate::state::{
    CardInstance, GameState, ModifierKind, StarshipMemory, StarshipPieceRecord, find_instance,
    find_instance_mut, new_instance,
};
use crate::subsystems::fuse::{FuseArgs, fuse};
use crate::wire::{
    GameEvent, GameEventType, Keyword, KeywordKind, PlayerId, Row, Tag, Zone, ZoneName, has_keyword,
};
use crate::zones::{PlaceOnFieldOptions, ZoneSlot, first_entry_zone, place_on_field};

/// R1852003: the id of a Dormant Starship's launch, the one ability it has while it is Dormant.
pub const STARSHIP_LAUNCH_ABILITY: &str = "launch";

// ---------------------------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------------------------

/// R1852000: whether a card the death pass collects dies as a Starship Piece — the keyword is among
/// its keywords as it stood (printed, granted or lent, read through the layers before it moved) and it
/// was face-up, a face-down card having no text in force (R1487000).
pub fn dies_as_piece(state: &GameState, card: &CardInstance) -> bool {
    !crate::preview::is_face_down(state, card)
        && has_keyword(
            &crate::layers::unit_view(state, card).keywords,
            KeywordKind::StarshipPiece,
        )
}

/// R1852000: whether a card's running text is a Starship's hull (`StaticFlags.starship_hull`).
pub fn is_hull(state: &GameState, card: &CardInstance) -> bool {
    crate::scripts::flags_of(state, card).starship_hull == Some(true)
}

/// R1852000: called where every field arrival passes (`zones::place_on_field`), beside the Dormant
/// start: a hull arriving from off the field as a Unit, face-up, starts a Starship's memory, empty. A
/// move from one field zone to another is no arrival (R638), so it keeps the memory it has.
pub fn start_starship_on_field(
    state: &GameState,
    card: &mut CardInstance,
    from_off_field: bool,
    as_unit: bool,
) {
    if !from_off_field || !as_unit || card.set_as.is_some() || card.starship.is_some() {
        return;
    }
    if is_hull(state, card) {
        card.starship = Some(StarshipMemory::default());
    }
}

/// R1852000: whether the card is a Dormant Starship — a Starship on the field that has not launched.
pub fn is_dormant_starship(card: &CardInstance) -> bool {
    card.starship.is_some() && crate::slumber::is_slumbering(card)
}

/// R1852003: whether the card is a Starship that has launched (on the field: R78's reset takes the
/// memory off as it leaves).
pub fn is_launched_starship(card: &CardInstance) -> bool {
    card.starship
        .as_ref()
        .is_some_and(|memory| memory.launched.is_some())
}

/// R1852000, R1852005: `player`'s Dormant Starship — one a time, the first in lane order, beneath a
/// Stack pile or not (a card buried there is still theirs and still Dormant, R1839003).
pub fn dormant_starship_of(state: &GameState, player: PlayerId) -> Option<&CardInstance> {
    state.players[player]
        .units
        .iter()
        .flatten()
        .flatten()
        .find(|card| is_dormant_starship(card))
}

/// R1852005 (#137 Firststrike Capability's "If you are building a ship"): whether `player` has a
/// Dormant Starship.
pub fn building_a_ship(state: &GameState, player: PlayerId) -> bool {
    dormant_starship_of(state, player).is_some()
}

/// R1852005: the pieces a Starship holds, oldest first; none for a card that is not one.
pub fn starship_pieces_of(card: &CardInstance) -> Vec<StarshipPieceRecord> {
    card.starship
        .as_ref()
        .map(|memory| memory.pieces.clone())
        .unwrap_or_default()
}

/// R1852005 (#137.4 Space Doctrine's "the launched Starship"): `player`'s launched Starship on the
/// field — the one launched last, when they have several.
pub fn launched_starship_of(state: &GameState, player: PlayerId) -> Option<&CardInstance> {
    crate::zones::units_on_field_of(state, player)
        .into_iter()
        .filter(|card| card.controller == player)
        .filter_map(|card| {
            card.starship
                .as_ref()
                .and_then(|memory| memory.launched)
                .map(|at| (at, card))
        })
        .max_by_key(|(at, _)| *at)
        .map(|(_, card)| card)
}

// ---------------------------------------------------------------------------------------------
// A piece dies (R1852000–R1852002)
// ---------------------------------------------------------------------------------------------

/// R1852000: the copy of a dead piece that fuses: a fresh card of its definition in no pile, owned by
/// its owner, carrying what R57's library copy carries — its Radiant face, its tuning, a token's X/X
/// (`statsOverride`, its printed face, R175) and its language (R1300) — read off `snapshot`, the card
/// as it stood before it died (R89). Its id is fresh, so it can be named by no other card.
fn piece_copy(state: &mut GameState, snapshot: &CardInstance) -> CardInstance {
    let owner = snapshot.owner;
    let mut copy = new_instance(state, &snapshot.def_id, owner, Zone::Gone { player: owner });
    copy.radiant = snapshot.radiant;
    copy.tuning = crate::tuning::copy_tuning(snapshot.tuning.as_ref());
    copy.stats_override = snapshot.stats_override;
    copy.armor_override = snapshot.armor_override;
    copy.chinese = snapshot.chinese;
    copy
}

/// R1852000: summon `player` a Dormant Starship — into their leftmost open unit zone (a Locked one
/// takes a summon, R688), else stacked on top of their lane-3 unit zone, Locked or not — and report it.
/// `None`, with nothing made, when no catalog holds the Dormant Starship (until the set ships) or the
/// zone cannot take it (held for a Reborn return, R64).
fn summon_dormant_starship(sink: &mut EngineSink<'_>, player: PlayerId) -> Option<String> {
    crate::catalog::find_def(Some(&*sink.state), DORMANT_STARSHIP_DEF_ID)?;
    let (slot, stack) = match first_entry_zone(sink.state, player, Row::Units) {
        Some(slot) => (slot, false),
        None => (
            ZoneSlot {
                player,
                row: Row::Units,
                lane: STARSHIP_FULL_ROW_LANE,
            },
            true,
        ),
    };
    if !crate::zones::is_reserved(sink.state, slot) {
        let mut card = new_instance(
            sink.state,
            DORMANT_STARSHIP_DEF_ID,
            player,
            Zone::Resolving { player },
        );
        if place_on_field(
            sink.state,
            &mut card,
            slot,
            PlaceOnFieldOptions { stack: Some(stack) },
        ) {
            let turn = sink.state.turn;
            let id = card.id.clone();
            if let Some(live) = find_instance_mut(sink.state, &id) {
                live.summoned_turn = Some(turn);
            }
            sink.events.push(GameEvent::Summoned {
                player,
                instance_id: id.clone(),
                def_id: DORMANT_STARSHIP_DEF_ID.to_string(),
                row: slot.row,
                lane: slot.lane,
                former_id: None,
                source_id: None,
                arrived_during: None,
                exits_from: None,
            });
            return Some(id);
        }
    }
    None
}

/// R1852000–R1852002: a Starship Piece died (`snapshot` is the card as it stood, R89): a copy of it
/// fuses into its owner's Dormant Starship — the player whose graveyard it entered, whoever controlled
/// it (R1852002) — summoning one first if they have none. The fusion is R77's with the Starship kept
/// (its zone, its Dormant status, its memory), the copy on the face it was on (R469), and the Starship
/// is never made a piece by it (R1852001). Nothing happens when the Starship cannot be had, or when the
/// fusion would pass `FUSED_CARDS_CAP` (R874000).
pub fn fuse_dead_piece(sink: &mut EngineSink<'_>, snapshot: &CardInstance) {
    let owner = snapshot.owner;
    let ship_id = match dormant_starship_of(sink.state, owner) {
        Some(ship) => ship.id.clone(),
        None => match summon_dormant_starship(sink, owner) {
            Some(id) => id,
            None => return,
        },
    };
    let Some(ship) = find_instance(sink.state, &ship_id).cloned() else {
        return;
    };
    let copy = piece_copy(sink.state, snapshot);
    let radiant_ingredients = if copy.radiant {
        Some(vec![copy.id.clone()])
    } else {
        None
    };
    let record = StarshipPieceRecord {
        def_id: copy.def_id.clone(),
        radiant: if copy.radiant { Some(true) } else { None },
    };
    let fused = fuse(
        sink,
        FuseArgs {
            ingredients: vec![ship.clone(), copy],
            target: Some(ship),
            radiant_ingredients,
            ..FuseArgs::default()
        },
    );
    if fused.is_none() {
        return;
    }
    let Some(live) = find_instance_mut(sink.state, &ship_id) else {
        return;
    };
    let memory = live.starship.get_or_insert_with(StarshipMemory::default);
    memory.pieces.push(record);
    let pieces = memory.pieces.len() as u32;
    sink.events.push(GameEvent::StarshipFused {
        player: owner,
        instance_id: ship_id,
        piece_id: snapshot.id.clone(),
        def_id: snapshot.def_id.clone(),
        pieces,
    });
}

// ---------------------------------------------------------------------------------------------
// The launch (R1852003–R1852006)
// ---------------------------------------------------------------------------------------------

/// R1852006: what `player`'s launch costs now — the least "your Starship launch costs (N)" they hold,
/// else `STARSHIP_LAUNCH_COST`. Never below 0.
pub fn launch_cost(state: &GameState, player: PlayerId) -> i32 {
    state.players[player]
        .mods
        .iter()
        .filter(|modifier| crate::mana::modifier_is_live(state, modifier))
        .filter_map(|modifier| match modifier.kind {
            ModifierKind::StarshipLaunchCost { cost } => Some(cost),
            _ => None,
        })
        .min()
        .unwrap_or(STARSHIP_LAUNCH_COST)
        .max(0)
}

/// R1852003: a Dormant Starship's one ability, the engine's: "Activate: pay (3) to launch", priced
/// for its controller now (R1852006), its label naming the price, which is what the view shows. Its
/// effect is the launch itself (`subsystems::activate` runs `pay_launch`), so its own list is empty.
pub fn launch_decl(state: &GameState, card: &CardInstance) -> ActivationDecl {
    let cost = launch_cost(state, card.controller);
    ActivationDecl {
        id: STARSHIP_LAUNCH_ABILITY.to_string(),
        label: format!("Launch ({cost})"),
        uses: ActivationUses::Count(STARSHIP_LAUNCH_USES),
        cost: Some(ActivationCost {
            mana: Some(cost),
            ..ActivationCost::default()
        }),
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|_ctx| Vec::new()),
    }
}

/// R1852003: whether an activation is a Dormant Starship's launch.
pub fn is_launch(card: &CardInstance, ability: &str) -> bool {
    ability == STARSHIP_LAUNCH_ABILITY && is_dormant_starship(card)
}

/// R1852003, R1852006: the launch's effect once its price is paid: the price rules it was priced by
/// are spent (every "your Starship launch costs (N)" `player` holds), and the Starship awakens, which
/// launches it (`launch`).
pub fn pay_launch(sink: &mut EngineSink<'_>, player: PlayerId, instance_id: &str) {
    let spent: Vec<String> = sink.state.players[player]
        .mods
        .iter()
        .filter(|modifier| matches!(modifier.kind, ModifierKind::StarshipLaunchCost { .. }))
        .map(|modifier| modifier.id.clone())
        .collect();
    for id in &spent {
        crate::modifiers::remove_modifier(sink, player, id);
    }
    crate::slumber::awaken_card(sink, instance_id);
}

/// R1852004: the names a Starship may take — those whose tag it holds when there are any (a tagged
/// name is preferred, and appears only on a Starship with its tag), else those that suit any ship.
pub fn launch_names(tags: &[Tag]) -> Vec<&'static str> {
    let tagged: Vec<&'static str> = STARSHIP_NAMES
        .iter()
        .filter(|(_, tag)| tag.is_some_and(|tag| tags.contains(&tag)))
        .map(|(name, _)| *name)
        .collect();
    if !tagged.is_empty() {
        return tagged;
    }
    STARSHIP_NAMES
        .iter()
        .filter(|(_, tag)| tag.is_none())
        .map(|(name, _)| *name)
        .collect()
}

/// R1852003: a hull awakened (`slumber::awaken_card`, already reported as `awakened`): it launches. It
/// takes the Starship keyword and a name drawn from the match rng (R1852004), its memory records the
/// launch (R1852005), `starshipLaunched` goes out, and its fused Cries fire as a triggered Cry does, for
/// its controller, who makes their choices (`cry_trigger::trigger_cry_of`, R467). Its fused "When
/// launched" triggers answer the event once the Cries are done (R113: work drains before the trigger
/// queue moves). Nothing happens to a card that is not a Starship.
pub fn launch(sink: &mut EngineSink<'_>, instance_id: &str) {
    let Some(card) = find_instance(sink.state, instance_id).cloned() else {
        return;
    };
    if card.starship.is_none() || card.zone.z() != ZoneName::Field {
        return;
    }
    let tags = crate::query::tags_of(sink.state, &card);
    let names = launch_names(&tags);
    let name = sink
        .rng
        .pick(&names)
        .map(|name| (*name).to_string())
        .unwrap_or_default();
    let seq = sink.state.next_seq;
    sink.state.next_seq += 1;
    let prints_it = has_keyword(
        &crate::layers::unit_view(sink.state, &card).keywords,
        KeywordKind::Starship,
    );
    let Some(live) = find_instance_mut(sink.state, instance_id) else {
        return;
    };
    if !prints_it {
        live.granted_keywords.push(Keyword::Starship);
    }
    let memory = live.starship.get_or_insert_with(StarshipMemory::default);
    memory.name = Some(name.clone());
    memory.launched = Some(seq);
    let pieces: Vec<String> = memory.pieces.iter().map(|piece| piece.def_id.clone()).collect();
    let controller = live.controller;
    let event = GameEvent::StarshipLaunched {
        player: controller,
        instance_id: live.id.clone(),
        def_id: live.def_id.clone(),
        name,
        pieces,
    };
    sink.events.push(event);
    if let Some(launched) = find_instance(sink.state, instance_id).cloned() {
        crate::cry_trigger::trigger_cry_of(sink, &launched, controller);
    }
}

/// §6.2 "When launched" (R1852003): the trigger a Starship Piece prints, answering the
/// `starshipLaunched` of the Starship it is fused into and never another card's — so a piece's own
/// "When launched" runs only as part of a Starship, after its fused Cries.
pub fn when_launched(
    id: impl Into<String>,
    run: impl Fn(&mut crate::script::EffectContext<'_>, &GameEvent) -> Vec<crate::script::Effect>
    + Send
    + Sync
    + 'static,
) -> TriggerDef {
    TriggerDef::new(id, &[GameEventType::StarshipLaunched], run).with_when(|ctx, event| {
        matches!(
            event,
            GameEvent::StarshipLaunched { instance_id, .. }
                if ctx.self_.as_ref().is_some_and(|this| &this.id == instance_id)
        )
    })
}

// ---------------------------------------------------------------------------------------------
// The Stack onto it (R1852007)
// ---------------------------------------------------------------------------------------------

/// R1852007: whether a Stack card played onto `base` is exiled before it resolves — `base` is a
/// Dormant Starship.
pub fn exiles_stacked_play(base: &CardInstance) -> bool {
    is_dormant_starship(base)
}
