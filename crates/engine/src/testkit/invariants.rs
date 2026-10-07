//! Fuzz-level invariants for summoning sickness, exertion, game over and hidden information (SPEC §4.1,
//! §4.2, §10.3, §10.8, R53, R83, R171; docs/polish/4-edge-cases.md "Fuzz invariants"). `cargo jackioh
//! fuzz` runs one monitor per game (SURFACE §8, §12), and the edge-case hunters reuse it.
//!
//! THE MONITOR IS AN ORACLE, NOT THE FIX RESTATED. It never reads `summonedTurn` to decide who is
//! sick. It keeps a shadow built only from the event stream:
//!   - `turn`, from `turnStarted`;
//!   - `entered`, the turn of each instance's latest entry, and `stint`, how many entries it has had.
//!     An entry event is `cardPlayed`, `summoned`, `controlChanged`, `animated` (an Animated card
//!     stepping from its backrow zone into a unit zone enters it on that turn, R383), or a
//!     `transformed` whose new instance differs from the old. `fused` keeps the target's entry (R77), and a move along one
//!     side or a Stack card resuming emits nothing and changes nothing;
//!   - `lastAttack`, the turn and stint of each instance's latest declared (not forced) attack;
//!   - `readied`, the Transform results R424 lets attack on the turn they entered: an attacker that
//!     destroyed the Unit its declared attack targeted (`destroyed` naming it the killer, R42) and is
//!     then `transformed` (Classic+ #73.1 Classic Golem) passes "may attack again this turn" to the new
//!     instance, so that instance is not sick (I1) and carries no `summonedTurn` (I4a) for that stint.
//!
//! The six checks:
//!   I1 no sick attack is ever offered (§4.1, §6.1, R83, R171). In the main phase with no prompt
//!      open, a unit of the acting player that entered on this turn has no attack target unless it
//!      has Rush or Charge, and no hero target unless it has Charge; the chosen `attack`, if any, is
//!      checked the same way. It reads `attack_targets`, which is what `legal_actions` enumerates with.
//!   I2 one declared attack per stint per turn, two for a unit that had Windfury in that stint (R636).
//!      A unit that re-entered may attack again within its keywords (R83, R171); forced attacks are not
//!      declarations and are skipped (R53).
//!   I3 every card on the field arrived by an event (§10.3: every visible state change emits one).
//!   I4 the bookkeeping matches the shadow (white-box R171): (a) `summonedTurn` is the turn of the
//!      latest entry, or absent on a `readied` stint (R424); (b) a spent attack exertion belongs to
//!      the current stint, so an exertion left spent across an entry trips it.
//!   I5 nothing happens after the game is over (§2.5, R216): `gameOver` is the last event an action
//!      emits. Added in the hunt's fourth round, whose engine-invariants lens found the rest of an
//!      effect list, and a trap's consumption, resolving after the check that ended the game.
//!   I6 no seat is sent a card it may not read (§9.1, §10.8, R97, R177, issue #348). On every state the
//!      fuzz reaches, setup's included, for both seats: `view_for` and `legal_actions` do not panic, and
//!      neither names a hidden card. Hidden is read off the state's zones on their own terms, never
//!      from `view_for`'s rules: the other seat's hand while the game is live (R434), both libraries,
//!      the other seat's face-down traps (a dormant one under a backrow top too, B5 E21), a card the other seat is setting face-down (R448) and a
//!      mulligan return waiting for its shuffle (R224). The serialized view may not hold such a card's
//!      instance id, nor a former id it had (R227's `formerId`, a transform, a fuse, read off the log's
//!      raw events), nor its definition id unless a card the seat reads carries the same one: a card
//!      the state holds outside the hidden set, or one the raw events name that is readable where it
//!      is now, and the seat's own library only as `knownAs` records it (R311, R312). Never through the
//!      view being checked, which could pair a hidden definition with any id it likes; the
//!      seat's own prompt may offer its options (§10.8), and `legal_actions` may name a face-down trap
//!      by its bare id as a target (R177) and a hidden card by nothing else. The first run found one
//!      leak, R763 (`trapFired` named a fired trap to its controller after it was shuffled into a
//!      library), and five false positives, each fixed here and not in the engine:
//!        - R466's `stolen`: a card its viewer read where it was taken is named to them openly;
//!        - a `defs` body (R243): copied only for an id the rest of the view names, and it names its
//!          ingredients and its `refs` (R279), the text of cards the seat reads;
//!        - a definition a hidden card shares with a readable one (a token, a copy, a Book, a fused
//!          card) is no secret, so a definition counts only while nothing readable carries it;
//!        - Echo's `copies` (R399) names the definition of the card it copied (read off the copier);
//!        - `swapsBook.from` (R671) names the Book a swap took the text from (read off the card).
//!      A card in no pile that has no successor (no Transform or Fuse made it another, R177) reads only
//!      when it is a token (R11); a Glitch's reset or boards (R676, R678) takes the rest unseen, and
//!      the first oracle excused it, which was the engine's own R97 rule restated: R764 fixed it.
//!      Judging former ids by the log found a sixth, fuzz seed 992: R419's Rollback recreated a
//!      face-down trap #83 had transformed away, and the `formerId` it carries goes with the card
//!      (R227) once that card is public, R177's mark on the old id notwithstanding.
//!      `I6_GATE_STRIDE` is how often the 1,000-seed gate runs it.
//!
//! Every message leads with its id, names the instance, its def and the turn, and cites the SPEC
//! reference, so the fuzz report's `signatureOf` groups one bug into one entry.
//!
//! Port of `packages/cards/test/_invariants.ts` (SURFACE §8: `testkit::invariants::Monitor`). The
//! JSON a check walks is serde's, whose objects iterate in key order where TS's iterated in insertion
//! order, so where one needle sits at two paths the path a message names may be the other one; which
//! needles are found is the same.

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::{IndexMap, IndexSet};
use serde_json::{Map, Value};

use crate::config::WINDFURY_ATTACKS;
use crate::damage::DamageTarget;
use crate::state::{CardInstance, GameState, PendingChoice, find_instance};
use crate::wire::{
    ActionBody, CardType, Enchantment, GameEvent, KeywordKind, PLAYER_IDS, Phase, PlayerId, PlayerView,
    Selection, ZoneName, has_keyword, opponent_of,
};

/// I6 under the 1,000-seed gate: every this-many-th state, setup's and the last always (the monitor
/// budget of docs/polish/4-edge-cases.md: within 25% of the time without it). The 100-seed
/// waves check every state.
///
/// Measured on 2026-10-06 (four cores, idle). `pnpm fuzz`, seeds 1–1000, both files, before I6 and
/// with no monitor in fuzz-handicap: 342 s. With I1–I6 in both files and every state checked: 502 s
/// (×1.47, over the budget, and 0 violations). With this stride: 386 s (×1.13). Once definitions
/// were judged through the state and the log rather than the view: 411 s (×1.20), and with a card in no pile
/// judged hidden unless it is a token: 419 s (×1.23).
/// `pnpm test --project cards fuzz`, seeds 1–100, every state: 38 s before, 55 s after (×1.45).
pub const I6_GATE_STRIDE: usize = 5;

/// What one seat may not read: instance ids, and the definitions that name a card only it hides.
struct HiddenSet {
    /// Instance id -> where the card sits, for the message.
    ids: IndexMap<String, String>,
    /// Definition id -> where a hidden card of it sits. A definition any readable card carries is not here.
    defs: IndexMap<String, String>,
    /// Face-down traps' ids: R177 lets `legal_actions` offer one as a bare target.
    bare: IndexSet<String>,
    /// The bare ids the viewer's own prompt offers: R177 lets the view carry each as a bare option's `instanceId`, nowhere else.
    offered_bare: IndexSet<String>,
}

/// R33, R686: another seat's armed Trap or Field Trap, not flipped (`faceUp`) and not revealed (R638).
fn face_down_to(state: &GameState, card: &CardInstance, viewer: PlayerId) -> bool {
    if card.controller == viewer || card.face_up == Some(true) || card.revealed == Some(true) {
        return false;
    }
    let type_ = crate::faces::card_type_of(state, card);
    type_ == CardType::Trap || type_ == CardType::FieldTrap
}

/// R448: a card the other seat is setting face-down waits in `resolving` for its announce window.
fn set_face_down_by(state: &GameState, card: &CardInstance, viewer: PlayerId) -> bool {
    match crate::announce::announce_of(state, &card.id) {
        Some(record) => record.face_down == Some(true) && record.player != viewer,
        None => false,
    }
}

/// One mulligan return as the owed work item carries it (TS read it as a `CardInstance`): its id,
/// definition and owner, read defensively off the JSON.
struct Returned {
    id: String,
    def_id: String,
    owner: Option<PlayerId>,
}

/// R224: the cards a mulligan returned that wait in a work item for their shuffle-back: in no pile.
fn awaiting_shuffle(state: &GameState) -> Vec<Returned> {
    let waiting: IndexSet<String> = crate::setup::returned_awaiting_shuffle(state)
        .into_iter()
        .collect();
    let mut out: Vec<Returned> = Vec::new();
    for item in &state.work {
        let returned = item
            .resume
            .data
            .get("owed")
            .and_then(Value::as_object)
            .and_then(|owed| owed.get("returned"))
            .and_then(Value::as_array);
        for card in returned.into_iter().flatten() {
            let Some(id) = card.get("id").and_then(Value::as_str) else {
                continue;
            };
            if !waiting.contains(id) {
                continue;
            }
            out.push(Returned {
                id: id.to_string(),
                def_id: card
                    .get("defId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                owner: card
                    .get("owner")
                    .and_then(Value::as_str)
                    .and_then(|owner| PLAYER_IDS.into_iter().find(|player| player.as_str() == owner)),
            });
        }
    }
    out
}

/// Every card the state holds, in every pile (`find_instance`'s piles).
fn cards_of(state: &GameState) -> Vec<&CardInstance> {
    let mut out: Vec<&CardInstance> = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        out.extend(side.hand.iter());
        out.extend(side.library.iter());
        out.extend(side.graveyard.iter());
        out.extend(side.exile.iter());
        out.extend(side.units.iter().flatten().flatten());
        out.extend(side.backrow.iter().flatten());
        out.extend(side.backrow_piles.iter().flatten().flatten());
        out.extend(side.carried.iter().flatten().flatten());
        out.extend(side.resolving.iter());
    }
    out
}

/// What the log says of the cards it names, for one viewer: the state keeps no zone for a card that
/// took a new id or ceased to exist, so the log's raw (unredacted) events are the only witness of it.
struct Lineage {
    /// Each vanished id's successor: R227's `formerId` (a card set face-down took a fresh id), §6.3's
    /// `transformed`, R77's `fused`, and R316's `copyOf` (a copy refused before it existed is that card's).
    next: IndexMap<String, String>,
    /// R177: an id that ceased to exist where this viewer could not read it (`transformed.hiddenFrom`).
    unread: IndexSet<String>,
    /// Each id, and the definitions the engine's own events pair with it.
    ties: IndexMap<String, IndexSet<String>>,
}

fn lineage_of(state: &GameState, viewer: PlayerId) -> Lineage {
    let mut next: IndexMap<String, String> = IndexMap::new();
    let mut unread: IndexSet<String> = IndexSet::new();
    let mut ties: IndexMap<String, IndexSet<String>> = IndexMap::new();
    let tie =
        |ties: &mut IndexMap<String, IndexSet<String>>, id: Option<&Value>, values: &[Option<&Value>]| {
            let Some(id) = id.and_then(Value::as_str) else {
                return;
            };
            let tied = ties.entry(id.to_string()).or_default();
            for def in strings_of(values) {
                tied.insert(def);
            }
        };
    for event in state.applied.iter().flat_map(|entry| entry.events.iter()) {
        let value = serde_json::to_value(event).expect("a GameEvent serialises");
        walk_objects(&value, &mut |object| {
            let id = object.get("instanceId");
            let hidden_to = object.get("hiddenFrom").and_then(Value::as_array);
            let unread_here = hidden_to.is_some_and(|players| {
                players
                    .iter()
                    .any(|player| player.as_str() == Some(viewer.as_str()))
            });
            // A card transformed where the viewer could not read it was never this viewer's to read (R177).
            tie(
                &mut ties,
                id,
                &[
                    object.get("defId"),
                    if unread_here {
                        None
                    } else {
                        object.get("fromDefId")
                    },
                ],
            );
            tie(&mut ties, object.get("newInstanceId"), &[object.get("toDefId")]);
            tie(&mut ties, object.get("resultInstanceId"), &[object.get("defId")]);
            let Some(id) = id.and_then(Value::as_str) else {
                return;
            };
            let former = object.get("formerId").and_then(Value::as_str);
            // R419: Classic+ #35 Rollback recreates a card transformed away under a fresh id whose
            // `formerId` is the old one, so the card exists again and R177's mark is spent: its old id
            // goes with it, a trap's that fired into exile included (fuzz seed 992).
            if let Some(former) = former {
                next.insert(former.to_string(), id.to_string());
                unread.shift_remove(former);
            }
            if let Some(copy_of) = object.get("copyOf").and_then(Value::as_str) {
                next.insert(id.to_string(), copy_of.to_string());
            }
            let fresh = object.get("newInstanceId").and_then(Value::as_str);
            if object.get("type").and_then(Value::as_str) == Some("transformed")
                && let Some(fresh) = fresh
                && fresh != id
            {
                next.insert(id.to_string(), fresh.to_string());
                if unread_here {
                    unread.insert(id.to_string());
                }
            }
        });
        if let GameEvent::Fused {
            instance_ids,
            result_instance_id,
            ..
        } = event
        {
            for id in instance_ids {
                if id != result_instance_id {
                    next.insert(id.clone(), result_instance_id.clone());
                }
            }
        }
    }
    Lineage { next, unread, ties }
}

/// Where a card an id names stands for one viewer (`standing_of`'s answer).
struct Standing {
    reads: bool,
    where_: String,
}

/// Whether this viewer reads the card an id names, judged by where the card is now: a card the state
/// holds reads unless the hidden set has it, and a vanished one by its successor. A vanished card with
/// none reads only when every definition the log pairs with it is a token's: R11's tokens go public
/// (leaving the field, discarded, burned), where any other card in no pile went unseen, out of a
/// Glitch's reset or boards (R676, R678), and stays as hidden as it was. An id neither the state nor
/// the log knows vouches for nothing.
fn standing_of(
    state: &GameState,
    held: &IndexSet<String>,
    viewer: PlayerId,
    ids: &IndexMap<String, String>,
    lineage: &Lineage,
    start: &str,
) -> Option<Standing> {
    let mut id: &str = start;
    for _hops in 0..=lineage.next.len() {
        if let Some(where_) = ids.get(id) {
            return Some(Standing {
                reads: false,
                where_: where_.clone(),
            });
        }
        if held.contains(id) {
            return Some(Standing {
                reads: true,
                where_: "a pile it reads".to_string(),
            });
        }
        if lineage.unread.contains(id) {
            return Some(Standing {
                reads: false,
                where_: format!("a pile {viewer} could not read when it was transformed"),
            });
        }
        let Some(successor) = lineage.next.get(id) else {
            let defs = lineage.ties.get(id)?;
            // R11: a token that ceased to exist (it left the field, was discarded or burned) was public when
            // it went. Any other card in no pile went unseen (a Glitch's reset or boards, R676, R678) and
            // stays as hidden as it was.
            let token = defs
                .iter()
                .all(|def| crate::catalog::def_of(Some(&*state), def).token);
            return Some(Standing {
                reads: token,
                where_: "no pile (it ceased to exist)".to_string(),
            });
        };
        id = successor.as_str();
    }
    None
}

/// The hidden set, worked out from the state's zones on their own terms and never from `view_for`'s
/// rules (the oracle must not be the fix restated): the other seat's hand while the game is live
/// (R434), both libraries (§9.1), the other seat's face-down traps (dormant ones under a backrow top
/// included, B5 E21), a card the other seat is setting face-down (R448), mulligan returns waiting for
/// their shuffle (R224), and every former id whose card is now one of these (R227, R177). Then it
/// takes out every definition the viewer reads through a card: one the state holds and the viewer
/// reads, one the log names that the viewer reads where it is now, its own library's cards as it was
/// shown them going in (R311, R312), and the options of its own prompt (§10.8, R177). Never through
/// the view being checked: a view may name a definition beside any id it likes.
fn hidden_from(state: &GameState, viewer: PlayerId) -> HiddenSet {
    let mut ids: IndexMap<String, String> = IndexMap::new();
    let mut defs: IndexMap<String, String> = IndexMap::new();
    let mut bare: IndexSet<String> = IndexSet::new();
    let mut hide =
        |ids: &mut IndexMap<String, String>, id: &str, def_id: &str, where_: String, with_def: bool| {
            ids.insert(id.to_string(), where_.clone());
            if with_def {
                defs.insert(def_id.to_string(), where_);
            }
        };
    let rival = opponent_of(viewer);
    if state.result.is_none() {
        for card in &state.players[rival].hand {
            hide(&mut ids, &card.id, &card.def_id, format!("{rival}'s hand"), true);
        }
    }
    for player in PLAYER_IDS {
        let side = &state.players[player];
        // R312: the owner reads a library card's definition only as `knownAs` records it, below.
        for card in &side.library {
            hide(
                &mut ids,
                &card.id,
                &card.def_id,
                format!("{player}'s library"),
                true,
            );
        }
        for card in side.backrow.iter().flatten() {
            if !face_down_to(state, card, viewer) {
                continue;
            }
            hide(
                &mut ids,
                &card.id,
                &card.def_id,
                format!("{player}'s face-down trap"),
                true,
            );
            bare.insert(card.id.clone());
        }
        // B5 E21: the dormant cards under a backrow top keep their backrow zone, so a Trap among them is
        // face-down as a top one is. They are not on the field for effects, so none is a bare target.
        for card in side.backrow_piles.iter().flatten().flatten() {
            if face_down_to(state, card, viewer) {
                hide(
                    &mut ids,
                    &card.id,
                    &card.def_id,
                    format!("{player}'s dormant face-down trap"),
                    true,
                );
            }
        }
        for card in &side.resolving {
            if set_face_down_by(state, card, viewer) {
                hide(
                    &mut ids,
                    &card.id,
                    &card.def_id,
                    format!("{player}'s card being set face-down"),
                    true,
                );
            }
        }
    }
    // The viewer's own mulligan returns are its opening hand, which it read (R224).
    for card in awaiting_shuffle(state) {
        hide(
            &mut ids,
            &card.id,
            &card.def_id,
            "a mulligan return awaiting its shuffle".to_string(),
            card.owner != Some(viewer),
        );
    }

    // §10.8: the viewer's own prompt offers what it may choose among, a revealed library card included.
    let mut offered_bare: IndexSet<String> = IndexSet::new();
    let prompt: Option<&PendingChoice> = match &state.pending {
        Some(pending) if pending.player_id == viewer => Some(pending),
        _ => crate::setup::mulligan_prompt_for(state, viewer),
    };
    for option in prompt.map(|prompt| prompt.options.as_slice()).unwrap_or(&[]) {
        match &option.selection {
            Selection::Mode { option: mode } => {
                defs.shift_remove(mode);
            }
            Selection::Instance { instance_id } => {
                // R177: a face-down trap is offered by its id alone: its id stays hidden everywhere but that
                // option's `instanceId`, and its definition stays hidden everywhere.
                if bare.contains(instance_id) {
                    offered_bare.insert(instance_id.clone());
                    continue;
                }
                ids.shift_remove(instance_id);
                if let Some(card) = find_instance(state, instance_id) {
                    defs.shift_remove(&card.def_id);
                }
            }
            _ => {}
        }
    }

    // R227, R177: an id a card had before is that card, so it is hidden wherever the card is.
    let lineage = lineage_of(state, viewer);
    // The end of a chain counts too (a card that ceased to exist where the viewer could not read it).
    let held: IndexSet<String> = cards_of(state).into_iter().map(|card| card.id.clone()).collect();
    let chained: IndexSet<String> = lineage
        .next
        .keys()
        .cloned()
        .chain(lineage.unread.iter().cloned())
        .chain(lineage.ties.keys().cloned())
        .collect();
    for id in chained {
        if held.contains(&id) {
            continue;
        }
        let standing = if ids.contains_key(&id) {
            None
        } else {
            standing_of(state, &held, viewer, &ids, &lineage, &id)
        };
        if let Some(standing) = standing
            && !standing.reads
        {
            ids.insert(id, format!("{} (a former id)", standing.where_));
        }
    }

    // A definition a card the viewer reads carries is no secret, whatever else carries it (a token, a
    // copy, a Book, a fused card): every card the state holds outside the hidden set, with the Spell
    // an Echo copies (R399) and the Book a swap took its text from (R671).
    for card in cards_of(state) {
        if ids.contains_key(&card.id) {
            continue;
        }
        defs.shift_remove(&card.def_id);
        if let Some(copy) = crate::subsystems::copied_text::copied_text_of(state, card) {
            defs.shift_remove(&copy.def_id);
        }
        // `enchantmentsOfKind(card, "swapsBook")`.
        for enchantment in card.enchantments.iter().flatten() {
            if let Enchantment::SwapsBook { from } = enchantment {
                defs.shift_remove(from);
            }
        }
    }
    // R311, R312: the viewer's own library reads as it was shown going in, and a card never shown not at all.
    for card in &state.players[viewer].library {
        if let Some(known_as) = &card.known_as {
            defs.shift_remove(&known_as.def_id);
        }
    }
    // Every card the log names that the viewer reads where it is now, as the engine's events named it.
    for (id, tied) in &lineage.ties {
        if standing_of(state, &held, viewer, &ids, &lineage, id).is_some_and(|standing| standing.reads) {
            for def in tied {
                defs.shift_remove(def);
            }
        }
    }
    HiddenSet {
        ids,
        defs,
        bare,
        offered_bare,
    }
}

/// R466: a `stolen` event reads openly to a viewer who could read the card in the zone it was taken
/// from. Any other event is not one (false).
fn readable_where_taken(event: &GameEvent, viewer: PlayerId) -> bool {
    let GameEvent::Stolen {
        from,
        zone,
        readable_from,
        ..
    } = event
    else {
        return false;
    };
    if let Some(readable_from) = readable_from {
        return readable_from.contains(&viewer);
    }
    if *zone == ZoneName::Graveyard || *zone == ZoneName::Exile || *zone == ZoneName::Resolving {
        return true;
    }
    *zone == ZoneName::Hand && *from == viewer
}

/// Every string and object inside `value`, with the path it was found at. Values only, never keys.
fn walk(
    value: &Value,
    path: &str,
    on_string: &mut dyn FnMut(&str, &str),
    on_object: &mut dyn FnMut(&Map<String, Value>),
) {
    match value {
        Value::String(text) => on_string(text, path),
        Value::Array(items) => {
            for (at, item) in items.iter().enumerate() {
                walk(item, &format!("{path}[{at}]"), on_string, on_object);
            }
        }
        Value::Object(object) => {
            on_object(object);
            for (key, item) in object {
                walk(item, &format!("{path}.{key}"), on_string, on_object);
            }
        }
        _ => {}
    }
}

/// `walk` with no string callback: every object inside `value`, parents first. Builds no paths.
fn walk_objects(value: &Value, on_object: &mut dyn FnMut(&Map<String, Value>)) {
    match value {
        Value::Array(items) => {
            for item in items {
                walk_objects(item, on_object);
            }
        }
        Value::Object(object) => {
            on_object(object);
            for item in object.values() {
                walk_objects(item, on_object);
            }
        }
        _ => {}
    }
}

fn strings_of(values: &[Option<&Value>]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| value.and_then(Value::as_str).map(str::to_string))
        .collect()
}

/// `/^view\.pending\.options\[\d+\]\.instanceId$/.test(at)`, by hand (no regex crate, SURFACE §8).
fn is_pending_option_instance_id(at: &str) -> bool {
    let Some(rest) = at.strip_prefix("view.pending.options[") else {
        return false;
    };
    let Some((index, tail)) = rest.split_once(']') else {
        return false;
    };
    !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()) && tail == ".instanceId"
}

/// I6 for one seat on one state: what `view_for` and `legal_actions` sent it names no card it may not
/// read. A hidden card's instance id may not appear anywhere in the view, nor its definition id unless
/// a card the viewer reads carries the same one (§10.8); `legal` may offer a face-down trap's bare id
/// as a target (R177) and nothing else of a hidden card.
pub fn hidden_information_violations(
    state: &GameState,
    viewer: PlayerId,
    view: &PlayerView,
    legal: &[ActionBody],
) -> Vec<String> {
    let HiddenSet {
        ids,
        defs,
        bare,
        offered_bare,
    } = hidden_from(state, viewer);

    // The view's events are the tail of the log's, in order: line them up to find a `stolen` among them.
    let raw: Vec<&GameEvent> = state
        .applied
        .iter()
        .flat_map(|entry| entry.events.iter())
        .collect();
    let offset = raw.len() as isize - view.events.len() as isize;
    let events: Vec<&GameEvent> = view
        .events
        .iter()
        .enumerate()
        .filter(|(at, _)| {
            let source = usize::try_from(offset + *at as isize)
                .ok()
                .and_then(|index| raw.get(index).copied());
            !source.is_some_and(|source| readable_where_taken(source, viewer))
        })
        .map(|(_, event)| event)
        .collect();
    // `defs` (R243) is left out: a definition is copied only for an id the rest of the view names, and
    // its body names its ingredients and its `refs` (R279), the text of cards the viewer reads.
    let mut scanned = serde_json::to_value(view).expect("a PlayerView serialises");
    if let Some(object) = scanned.as_object_mut() {
        object.remove("defs");
        object.insert(
            "events".to_string(),
            serde_json::to_value(&events).expect("GameEvents serialise"),
        );
    }

    let mut named: IndexMap<String, String> = IndexMap::new();
    walk(
        &scanned,
        "view",
        &mut |text, at| {
            // R177: the viewer's own prompt names a face-down trap by its id, as a bare option.
            if offered_bare.contains(text) && is_pending_option_instance_id(at) {
                return;
            }
            if !named.contains_key(text) {
                named.insert(text.to_string(), at.to_string());
            }
        },
        &mut |_| {},
    );
    let mut offered: IndexMap<String, String> = IndexMap::new();
    let legal_json = serde_json::to_value(legal).expect("ActionBodies serialise");
    walk(
        &legal_json,
        "legal",
        &mut |text, at| {
            if !offered.contains_key(text) {
                offered.insert(text.to_string(), at.to_string());
            }
        },
        &mut |_| {},
    );

    let mut found: Vec<String> = Vec::new();
    let mut report = |kind: &str, into: &str, needle: &str, path: &str, where_: &str| {
        found.push(format!(
            "I6 hidden {kind} in {into}: {viewer} is sent \"{needle}\" at {path}, a card in {where_}, on turn {} \
             (§9.1, §10.8, R97{})",
            state.turn,
            if into == "legalActions" { ", R177" } else { "" }
        ));
    };
    for (id, where_) in &ids {
        if let Some(at) = named.get(id) {
            report("card", "the view", id, at, where_);
        }
        if let Some(offer) = offered.get(id)
            && !bare.contains(id)
        {
            report("card", "legalActions", id, offer, where_);
        }
    }
    for (def, where_) in &defs {
        if let Some(at) = named.get(def) {
            report("definition", "the view", def, at, where_);
        }
        if let Some(offer) = offered.get(def) {
            report("definition", "legalActions", def, offer, where_);
        }
    }
    found
}

/// The turn and stint of an instance's latest declared attack, and how many it has declared in them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AttackMark {
    turn: i32,
    stint: i32,
    count: i32,
}

/// R424: the latest declared attack, and whether its attacker destroyed its target (R42).
#[derive(Clone, Debug, PartialEq, Eq)]
struct ShadowAttack {
    attacker_id: String,
    target_id: String,
    killed: bool,
}

/// Every card on the field, a card dormant under a Stack pile and the backrow included (§3.2).
fn field_cards(state: &GameState) -> Vec<&CardInstance> {
    let mut out: Vec<&CardInstance> = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        out.extend(side.units.iter().flatten().flatten());
        out.extend(side.backrow.iter().flatten());
    }
    out
}

fn name_of(card: &CardInstance) -> String {
    format!("{} ({}, {}'s)", card.id, card.def_id, card.controller)
}

/// The keywords a unit has now, through the layers (`layers::unit_view`).
fn keywords_now(state: &GameState, card: &CardInstance) -> Vec<crate::wire::Keyword> {
    crate::layers::unit_view(state, card).keywords.clone()
}

/// TS `InvariantMonitor`, made by `create_invariant_monitor`: the shadow of this file's header and
/// the checks run against it. `before` is I1 and I3, `after` feeds one action's events into the
/// shadow and runs I2, I4 and I5, and `hidden` is I6 for both seats.
#[derive(Clone, Debug)]
pub struct InvariantMonitor {
    turn: i32,
    entered: IndexMap<String, i32>,
    stint: IndexMap<String, i32>,
    last_attack: IndexMap<String, AttackMark>,
    /// R636: the stint in which each instance was last seen with Windfury. Read from the states between
    /// actions and after each one, so a unit that dies on its second attack was seen with it before —
    /// and from `keywordGranted` events inside the action, so a unit granted Windfury mid-action is seen
    /// before its declarations: R44's AI turn can summon a Conjure token and fight with it twice inside
    /// one outer action, where no between-action state ever holds it.
    windfury: IndexMap<String, i32>,
    readied: IndexSet<String>,
    /// R424: the latest declared attack, and whether its attacker destroyed its target (R42).
    attack: Option<ShadowAttack>,
}

/// SURFACE §8's name for the monitor (`testkit::invariants::Monitor`).
pub type Monitor = InvariantMonitor;

pub fn create_invariant_monitor(start: &GameState) -> InvariantMonitor {
    InvariantMonitor::new(start)
}

impl InvariantMonitor {
    /// `createInvariantMonitor(start)`: a shadow that starts on `start`'s turn with nothing entered.
    pub fn new(start: &GameState) -> InvariantMonitor {
        InvariantMonitor {
            turn: start.turn,
            entered: IndexMap::new(),
            stint: IndexMap::new(),
            last_attack: IndexMap::new(),
            windfury: IndexMap::new(),
            readied: IndexSet::new(),
            attack: None,
        }
    }

    fn stint_of(&self, id: &str) -> i32 {
        self.stint.get(id).copied().unwrap_or(0)
    }

    fn note_windfury(&mut self, state: &GameState) {
        for card in field_cards(state) {
            if has_keyword(&keywords_now(state, card), KeywordKind::Windfury) {
                let stint = self.stint_of(&card.id);
                self.windfury.insert(card.id.clone(), stint);
            }
        }
    }

    /// R424: a readied stint whose body carries no `summonedTurn`, as the readying transform leaves it.
    fn is_readied(&self, card: &CardInstance) -> bool {
        self.readied.contains(&card.id) && card.summoned_turn.is_none()
    }

    fn enter(&mut self, id: &str) {
        self.entered.insert(id.to_string(), self.turn);
        let stint = self.stint_of(id) + 1;
        self.stint.insert(id.to_string(), stint);
        self.readied.shift_remove(id);
    }

    /// I1 for one unit and one would-be target set.
    fn sick_attack(
        state: &GameState,
        unit: &CardInstance,
        target_ids: &[String],
        what: &str,
    ) -> Option<String> {
        if target_ids.is_empty() {
            return None;
        }
        let keywords = keywords_now(state, unit);
        if has_keyword(&keywords, KeywordKind::Charge) {
            return None;
        }
        if !has_keyword(&keywords, KeywordKind::Rush) {
            return Some(format!(
                "I1 sick attack {what}: {} entered on turn {} and has neither Rush nor Charge, yet may attack {} \
                 (§4.1, R171)",
                name_of(unit),
                state.turn,
                target_ids.join(", ")
            ));
        }
        let heroes: Vec<&str> = target_ids
            .iter()
            .map(String::as_str)
            .filter(|id| id.starts_with("hero-"))
            .collect();
        if heroes.is_empty() {
            return None;
        }
        Some(format!(
            "I1 sick hero attack {what}: {} entered on turn {} with Rush and no Charge, yet may attack {} \
             (§6.1, R171)",
            name_of(unit),
            state.turn,
            heroes.join(", ")
        ))
    }

    /// I1 and I3 on the state the next action is chosen in. Empty when clean.
    pub fn before(&mut self, state: &GameState, player: PlayerId, action: &ActionBody) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        self.note_windfury(state);

        // I3: nothing is on the field that no event put there.
        for card in field_cards(state) {
            if !self.entered.contains_key(&card.id) {
                found.push(format!(
                    "I3 silent arrival: {} is on the field on turn {} with no entry event (§10.3)",
                    name_of(card),
                    state.turn
                ));
            }
        }

        // I1: only where attacks can be offered at all.
        if state.result.is_some()
            || state.pending.is_some()
            || state.phase != Phase::Main
            || player != state.active
        {
            return found;
        }
        for unit in crate::zones::active_units_of(state, player) {
            let unit: &CardInstance = unit;
            if self.entered.get(&unit.id) != Some(&state.turn) || self.is_readied(unit) {
                continue;
            }
            let targets: Vec<String> = crate::combat::attack_targets(state, unit)
                .iter()
                .map(|target| match target {
                    DamageTarget::Hero { player } => format!("hero-{player}"),
                    DamageTarget::Unit { instance } => instance.id.clone(),
                })
                .collect();
            if let Some(violation) = InvariantMonitor::sick_attack(state, unit, &targets, "offered") {
                found.push(violation);
            }
        }
        if let ActionBody::Attack {
            attacker_id,
            target_id,
            ..
        } = action
            && let Some(attacker) = find_instance(state, attacker_id)
            && self.entered.get(&attacker.id) == Some(&state.turn)
            && !self.is_readied(attacker)
            && let Some(violation) =
                InvariantMonitor::sick_attack(state, attacker, std::slice::from_ref(target_id), "chosen")
        {
            found.push(violation);
        }
        found
    }

    /// Feeds one action's events into the shadow, then I2, I4 and I5 against the resulting state.
    pub fn after(&mut self, events: &[GameEvent], state: &GameState) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        // R424: the Golem's transform follows its combat inside the one attack action, so an attack the
        // record holds from an earlier action readies nothing (a later Transmogulate of that unit is not it).
        self.attack = None;

        // I5: the check that ends the game is the last thing that happens (§2.5, R216).
        if let Some(over) = events
            .iter()
            .position(|event| matches!(event, GameEvent::GameOver { .. }))
            && over + 1 < events.len()
        {
            let later: Vec<&str> = events[over + 1..]
                .iter()
                .map(|event| event.event_type().as_str())
                .collect();
            found.push(format!(
                "I5 event after game over: {} followed gameOver on turn {} (§2.5, R216)",
                later.join(", "),
                self.turn
            ));
        }

        for event in events {
            match event {
                GameEvent::TurnStarted { turn, .. } => {
                    self.turn = *turn;
                    self.attack = None;
                }
                GameEvent::CardPlayed { instance_id, .. }
                | GameEvent::Summoned { instance_id, .. }
                | GameEvent::ControlChanged { instance_id, .. } => {
                    self.enter(instance_id);
                }
                GameEvent::Animated {
                    instance_id, carried, ..
                } => {
                    // R383: moving into the unit row is entering it on that turn; a carried Unit stepping
                    // down off its carrier (C+ #33, carriers.rs) was a Unit on the field all along and enters nothing.
                    if *carried != Some(true) {
                        self.enter(instance_id);
                    }
                }
                GameEvent::Transformed {
                    instance_id,
                    new_instance_id,
                    ..
                } => {
                    if new_instance_id != instance_id {
                        self.enter(new_instance_id);
                    }
                    if self
                        .attack
                        .as_ref()
                        .is_some_and(|attack| attack.killed && attack.attacker_id == *instance_id)
                    {
                        self.readied.insert(new_instance_id.clone());
                    }
                }
                GameEvent::Destroyed {
                    instance_id,
                    killer_id,
                    ..
                } => {
                    if let Some(attack) = self.attack.as_mut()
                        && attack.target_id == *instance_id
                        && killer_id.as_deref() == Some(attack.attacker_id.as_str())
                    {
                        attack.killed = true;
                    }
                }
                GameEvent::KeywordGranted {
                    instance_id,
                    keyword,
                    lost,
                } => {
                    // A grant the stream saw is a grant the unit holds from this stint on: without this, a
                    // Windfury granted mid-action reads as a second attack without Windfury (see above).
                    // `lost` (R46) takes a keyword away instead of giving it, so it is never recorded.
                    if keyword.kind() == KeywordKind::Windfury && *lost != Some(true) {
                        let stint = self.stint_of(instance_id);
                        self.windfury.insert(instance_id.clone(), stint);
                    }
                }
                GameEvent::AttackDeclared {
                    attacker_id,
                    target_id,
                    forced,
                } => {
                    // R53: a forced attack is not a declaration and spends nothing (nor readies, R424).
                    self.attack = None;
                    if *forced {
                        continue;
                    }
                    let current = self.stint_of(attacker_id);
                    let previous = self.last_attack.get(attacker_id).copied();
                    let repeat = previous
                        .is_some_and(|previous| previous.turn == self.turn && previous.stint == current);
                    let mark = AttackMark {
                        turn: self.turn,
                        stint: current,
                        count: match previous {
                            Some(previous) if repeat => previous.count + 1,
                            _ => 1,
                        },
                    };
                    let allowed = if self.windfury.get(attacker_id) == Some(&current) {
                        WINDFURY_ATTACKS
                    } else {
                        1
                    };
                    if mark.count > allowed {
                        let who = match find_instance(state, attacker_id) {
                            None => attacker_id.clone(),
                            Some(unit) => name_of(unit),
                        };
                        found.push(format!(
                            "I2 extra attack: {who} declared attack number {} on turn {} without re-entering the \
                             field (§4.1 one exertion per turn, R171; Windfury two, R636)",
                            mark.count, self.turn
                        ));
                    }
                    self.last_attack.insert(attacker_id.clone(), mark);
                    self.attack = Some(ShadowAttack {
                        attacker_id: attacker_id.clone(),
                        target_id: target_id.clone(),
                        killed: false,
                    });
                }
                _ => {}
            }
        }

        self.note_windfury(state);

        // I4: the engine's bookkeeping agrees with the shadow.
        for card in field_cards(state) {
            // I3 reports a card with no entry before the next action.
            let Some(at) = self.entered.get(&card.id).copied() else {
                continue;
            };
            if card.summoned_turn != Some(at) && !self.is_readied(card) {
                let summoned = match card.summoned_turn {
                    Some(turn) => turn.to_string(),
                    None => "undefined".to_string(),
                };
                found.push(format!(
                    "I4 entry mismatch: {} has summonedTurn {summoned} on turn {}, but its latest entry event was \
                     on turn {at} (§4.1, R83, R171)",
                    name_of(card),
                    state.turn
                ));
            }
            if card.exertion.attacked {
                let mark = self.last_attack.get(&card.id);
                if mark.is_none_or(|mark| mark.stint != self.stint_of(&card.id)) {
                    found.push(format!(
                        "I4 stale exertion: {} has a spent attack on turn {} that it did not declare since its \
                         latest entry (§4.1, R171)",
                        name_of(card),
                        state.turn
                    ));
                }
            }
        }

        found
    }

    /// I6 for both seats on a state the fuzz reached, setup's included. Empty when clean. A panic in
    /// `view_for` or `legal_actions` (TS: a throw) is reported, not raised.
    pub fn hidden(&self, state: &GameState) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for viewer in PLAYER_IDS {
            let checked = catch_unwind(AssertUnwindSafe(|| {
                let view = crate::view_for::view_for(state, viewer);
                let legal = crate::reduce::legal_actions(state, viewer);
                hidden_information_violations(state, viewer, &view, &legal)
            }));
            match checked {
                Ok(violations) => found.extend(violations),
                Err(payload) => {
                    let message = if let Some(text) = payload.downcast_ref::<&str>() {
                        (*text).to_string()
                    } else if let Some(text) = payload.downcast_ref::<String>() {
                        text.clone()
                    } else {
                        "a panic with no message".to_string()
                    };
                    found.push(format!(
                        "I6 view threw: viewFor or legalActions for {viewer} on turn {}: {message} (§10.8)",
                        state.turn
                    ));
                }
            }
        }
        found
    }
}
