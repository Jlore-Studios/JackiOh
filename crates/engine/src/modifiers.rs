//! Player modifiers and delayed effects, with their expiry (SPEC §2.2, §10.1, R62, R68).
//!
//! Port of `packages/engine/src/modifiers.ts` (part 3, SURFACE §4). TS's `addModifier` took the
//! modifier without its id (`DistributiveOmit<PlayerModifier, "id">`); a `PlayerModifier` is
//! `{ id, expiry, ...kind }` (`state.rs`), so the id-less half is its `expiry` and its `kind`, and
//! that is what `add_modifier` takes. The readers that TS answered with the live record
//! (`dueDelayed`, `dueStartOfTurnEffects`) answer copies, which a caller writes back by id.

use crate::config::SETUP_TURN;
use crate::script::EngineSink;
use crate::state::{
    DelayedAt, DelayedEffect, GameState, ModifierExpiry, ModifierKind, Phase, PlayerModifier, Resume,
    find_instance,
};
use crate::wire::{GameEvent, PLAYER_IDS, PlayerId, Row, ZoneName};

/// `"sourceId" in mod ? mod.sourceId : undefined`: the permanent a modifier belongs to (#79
/// Twinspell's rider is the one kind that carries one).
fn source_id_of(modifier: &PlayerModifier) -> Option<&str> {
    match &modifier.kind {
        ModifierKind::EchoNextSpell { source_id, .. } => source_id.as_deref(),
        _ => None,
    }
}

pub fn add_modifier(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    expiry: ModifierExpiry,
    kind: ModifierKind,
) -> PlayerModifier {
    let with_id = PlayerModifier {
        id: format!("m{}", sink.state.next_seq),
        expiry,
        kind,
    };
    sink.state.next_seq += 1;
    sink.state.players[player].mods.push(with_id.clone());
    sink.events.push(GameEvent::ModifierChanged {
        player,
        modifier_id: with_id.id.clone(),
        added: true,
    });
    with_id
}

pub fn remove_modifier(sink: &mut EngineSink<'_>, player: PlayerId, id: &str) {
    let side = &mut sink.state.players[player];
    let before = side.mods.len();
    side.mods.retain(|modifier| modifier.id != id);
    if side.mods.len() != before {
        sink.events.push(GameEvent::ModifierChanged {
            player,
            modifier_id: id.to_string(),
            added: false,
        });
    }
}

/// The modifiers a permanent installed and still owns (`sourceId`, #79 Twinspell) move with it when
/// its controller changes (§5.1, §8 conventions: "your" is the controller). Each keeps its id, and
/// the move is reported as the removal from one seat and the addition to the other, which is what
/// R169's badges on both seats read.
pub fn move_sourced_modifiers(sink: &mut EngineSink<'_>, source_id: &str, from: PlayerId, to: PlayerId) {
    if from == to {
        return;
    }
    let side = &mut sink.state.players[from];
    let moving: Vec<PlayerModifier> = side
        .mods
        .iter()
        .filter(|modifier| source_id_of(modifier) == Some(source_id))
        .cloned()
        .collect();
    if moving.is_empty() {
        return;
    }
    side.mods
        .retain(|modifier| source_id_of(modifier) != Some(source_id));
    for modifier in moving {
        sink.events.push(GameEvent::ModifierChanged {
            player: from,
            modifier_id: modifier.id.clone(),
            added: false,
        });
        let id = modifier.id.clone();
        sink.state.players[to].mods.push(modifier);
        sink.events.push(GameEvent::ModifierChanged {
            player: to,
            modifier_id: id,
            added: true,
        });
    }
}

/// R209: a modifier a permanent installed and still owns (`sourceId`, #79 Twinspell's "the next
/// Spell you play gains Echo") is that permanent's lasting effect (§5.1), so it lasts while the
/// permanent stays on the field and ends when it leaves — destroyed (#36, #88), bounced (#52's
/// radiant crossing, a Locked rotation or swap), exiled (#34, #100), eaten (#22), replaced (#83) or
/// fused away (#85). A card that later stands on the field under the same id is a new arrival that
/// installs its own (R174). The state check runs this, since it follows every action and every whole
/// effect (§4.5, R59), and each removal is reported like any other (R169).
pub fn end_orphaned_modifiers(sink: &mut EngineSink<'_>) {
    for player in PLAYER_IDS {
        let mods: Vec<PlayerModifier> = sink.state.players[player].mods.clone();
        for modifier in &mods {
            let Some(source_id) = source_id_of(modifier) else {
                continue;
            };
            let on_field =
                find_instance(sink.state, source_id).is_some_and(|source| source.zone.z() == ZoneName::Field);
            if on_field {
                continue;
            }
            remove_modifier(sink, player, &modifier.id);
        }
    }
}

/// R209, R169: a permanent's lasting effect is installed by the permanent standing on the field, not
/// by its Cry — #79 Twinspell's "the next Spell you play gains Echo +1" prints no "Cry:" (§8), and a
/// Cry fires only for a card played from hand (§6.2). So every permanent that stands on a side of the
/// field with a `staticFlags.echoGrant` has one `echoNextSpell` rider on that side's player, owned by
/// it (`sourceId`), however it got there: played, summoned (#22's copies, #95's backrow, #98's
/// recruit), or holding Twinspell's text through a Fuse onto another permanent (#85, R77), in which
/// case "you" is that permanent's controller (§8 Conventions). The state check runs this beside
/// `endOrphanedModifiers`, which ends the rider when the permanent leaves, and `echo.grantedEcho`
/// runs it once more before a Spell takes the grant, so a permanent that arrived since the last check
/// is not missed. A rider that already exists is left alone, so a card that stays on the field keeps
/// the id its badge was given (R169); one that changed sides has had its rider moved with it
/// (`moveSourcedModifiers`), and the one it finds on its new controller's side is that same rider.
pub fn install_lasting_modifiers(sink: &mut EngineSink<'_>) {
    for player in PLAYER_IDS {
        for row in [Row::Units, Row::Backrow] {
            for lane in 1..=crate::zones::row_size(row) {
                let Some(card) =
                    crate::zones::card_at(sink.state, crate::wire::ZoneRef { player, row, lane }).cloned()
                else {
                    continue;
                };
                let amount = crate::scripts::flags_of(sink.state, &card)
                    .echo_grant
                    .unwrap_or(0)
                    .max(0);
                if amount <= 0 {
                    continue;
                }
                let owned = sink.state.players[player].mods.iter().any(|modifier| {
                    matches!(&modifier.kind, ModifierKind::EchoNextSpell { source_id: Some(source), .. } if *source == card.id)
                });
                if owned {
                    continue;
                }
                add_modifier(
                    sink,
                    player,
                    // §2.2: not turn-scoped — it survives cleanup and waits for a Spell (R30).
                    ModifierExpiry::Used,
                    ModifierKind::EchoNextSpell {
                        amount,
                        source_id: Some(card.id.clone()),
                    },
                );
            }
        }
    }
}

/// An "until used" modifier is consumed the moment it applies (Lunar Eclipse's discount).
pub fn consume_modifier(sink: &mut EngineSink<'_>, player: PlayerId, id: &str) {
    remove_modifier(sink, player, id);
}

/// Cleanup at the end of `player`'s turn: "this turn" modifiers go, and so do the ones that were
/// scheduled to last through this player's turn (Professor Curvature, R48).
pub fn expire_modifiers(sink: &mut EngineSink<'_>, player: PlayerId) {
    let turn = sink.state.turn;
    for side in PLAYER_IDS {
        let keeps: Vec<bool> = sink.state.players[side]
            .mods
            .iter()
            .map(|modifier| match &modifier.expiry {
                // §2.2: a "this turn" effect lasts to the end of its turn. One made after its turn's cleanup
                // had run — a trigger answering cleanup's own events, which resolve inside that turn (R62) — is
                // over by the next cleanup, whoever's it is, and dead from the next turn on (`modifierIsLive`).
                ModifierExpiry::ThisTurn { turn: made } => *made > turn,
                // R48: it covers that player's *next* turn, so it survives the turn it was created on.
                ModifierExpiry::NextTurnOf {
                    player: of,
                    from_turn,
                } => !(*of == player && turn > *from_turn),
                _ => true,
            })
            .collect();
        let mods = std::mem::take(&mut sink.state.players[side].mods);
        let mut kept: Vec<PlayerModifier> = Vec::with_capacity(mods.len());
        for (modifier, keep) in mods.into_iter().zip(keeps) {
            if keep {
                kept.push(modifier);
            } else {
                sink.events.push(GameEvent::ModifierChanged {
                    player: side,
                    modifier_id: modifier.id.clone(),
                    added: false,
                });
            }
        }
        sink.state.players[side].mods = kept;
    }
}

pub fn schedule_delayed(
    sink: &mut EngineSink<'_>,
    owner: PlayerId,
    at: DelayedAt,
    resume: Resume,
    watch: Option<String>,
    not_before: Option<i32>,
) -> DelayedEffect {
    let effect = DelayedEffect {
        id: format!("d{}", sink.state.next_seq),
        seq: sink.state.next_seq,
        owner,
        at,
        not_before,
        resume,
        watch,
    };
    sink.state.next_seq += 1;
    sink.state.delayed.push(effect.clone());
    effect
}

/// R68: delayed effects due now, in the order they were created. R458: one made for "your *next*
/// turn" waits until that turn (`notBefore`), so the boundary of the turn it was made on passes it by.
///
/// `phase` is `Phase::Start` or `Phase::End` (TS `"start" | "end"`).
pub fn due_delayed(state: &GameState, phase: Phase, player: PlayerId) -> Vec<DelayedEffect> {
    let mut due: Vec<DelayedEffect> = state
        .delayed
        .iter()
        .filter(|effect| {
            effect.at.phase == phase
                && effect.at.player == player
                && effect
                    .not_before
                    .is_none_or(|not_before| state.turn >= not_before)
        })
        .cloned()
        .collect();
    due.sort_by_key(|a| a.seq);
    due
}

pub fn drop_delayed(state: &mut GameState, id: &str) {
    state.delayed.retain(|effect| effect.id != id);
}

// ---------------------------------------------------------------------------
// ---- v0.2.0: activate and turn (B5 E10, E28) ----
// ---------------------------------------------------------------------------

/// TS `Extract<PlayerModifier, { kind: "turnEnds" }>`: a `PlayerModifier` whose kind is `TurnEnds`.
pub type TurnEndsModifier = PlayerModifier;

/// TS `Extract<PlayerModifier, { kind: "startOfTurnEffect" }>`: a `PlayerModifier` whose kind is
/// `StartOfTurnEffect`.
pub type StartOfTurnEffectModifier = PlayerModifier;

/// B5 E10, R456: the "your turn ends" rider on `player` for the turn running now, or null.
pub fn turn_ends_of(state: &GameState, player: PlayerId) -> Option<&TurnEndsModifier> {
    for modifier in &state.players[player].mods {
        if !matches!(modifier.kind, ModifierKind::TurnEnds { .. }) {
            continue;
        }
        if let ModifierExpiry::ThisTurn { turn } = modifier.expiry
            && turn != state.turn
        {
            continue;
        }
        return Some(modifier);
    }
    None
}

/// B5 E10, R456: `player`'s turn ends once `actionsLeft` more of their main-phase actions have
/// resolved (0: once what is resolving now has resolved). Only on that player's own turn and before
/// it has begun to end — on the other player's turn there is no turn of theirs to end. A second
/// rider on the same turn keeps the sooner end, and names the card that set it.
pub fn cut_turn_short(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    actions_left: i32,
    by_instance_id: Option<String>,
) {
    let state = &*sink.state;
    if state.turn == SETUP_TURN
        || state.active != player
        || state.phase == Phase::End
        || state.result.is_some()
    {
        return;
    }
    let left = actions_left.max(0);
    let turn = state.turn;
    if let Some(existing) = turn_ends_of(state, player).map(|modifier| modifier.id.clone()) {
        let found = sink.state.players[player]
            .mods
            .iter_mut()
            .find(|modifier| modifier.id == existing);
        if let Some(PlayerModifier {
            kind:
                ModifierKind::TurnEnds {
                    actions_left: current,
                    by_instance_id: by,
                },
            ..
        }) = found
            && left < *current
        {
            *current = left;
            *by = by_instance_id;
        }
        return;
    }
    add_modifier(
        sink,
        player,
        ModifierExpiry::ThisTurn { turn },
        ModifierKind::TurnEnds {
            actions_left: left,
            by_instance_id,
        },
    );
}

/// B5 E28, R458: "For the rest of the game: at the start of your turn, …" — a `never` modifier on
/// `player` that re-enters `resume` at each start of their turn (`turn.ts`'s delayed stage). Its `seq`
/// is the one its id is numbered from (`addModifier`), R68's creation order among the delayed effects.
pub fn add_start_of_turn_effect(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    resume: Resume,
    label: &str,
) -> PlayerModifier {
    let seq = sink.state.next_seq;
    add_modifier(
        sink,
        player,
        ModifierExpiry::Never,
        ModifierKind::StartOfTurnEffect {
            seq,
            resume,
            label: label.to_string(),
            ran_turn: None,
        },
    )
}

/// B5 E28, R458: `player`'s rest-of-game effects that have not run this turn, in creation order.
pub fn due_start_of_turn_effects(state: &GameState, player: PlayerId) -> Vec<StartOfTurnEffectModifier> {
    let mut due: Vec<(u32, StartOfTurnEffectModifier)> = state.players[player]
        .mods
        .iter()
        .filter_map(|modifier| match &modifier.kind {
            ModifierKind::StartOfTurnEffect { seq, ran_turn, .. } if *ran_turn != Some(state.turn) => {
                Some((*seq, modifier.clone()))
            }
            _ => None,
        })
        .collect();
    due.sort_by_key(|a| a.0);
    due.into_iter().map(|(_, modifier)| modifier).collect()
}
