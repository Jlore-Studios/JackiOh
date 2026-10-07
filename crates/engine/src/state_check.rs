//! Deaths and the state check (SPEC §4.5). Run after each action, each whole effect or trigger, each
//! cast-on-draw cast and each combat, never between the hits of one effect (R59).
//! M2-T5 adds the full test set; the loop itself is here.
//!
//! §4.5 step 3 fires one Death hook per collected card, and a hook is an effect list like any other:
//! it can open a prompt. A prompt ends the action — "mid-action choices are state, not callbacks"
//! (§9.3) — so step 3 is an engine sequence that can span a pause, and `work.ts`'s header names it
//! as one. It is therefore resumable through `state.work` and through nothing else (R113, R117):
//!
//!   * the hook's own effect list pauses through `prompts.applyResumable`, which parks the effects
//!     after the one that asked;
//!   * the *pass* — which cards still owe a Death hook in R68's order, which ones reserved a zone for
//!     step 4's Reborn, and which ones step 5 has to report — is parked in the same work item, as
//!     plain JSON, so the paused board survives `JSON.parse(JSON.stringify(state))` and replays
//!     exactly. Holding the remaining units in a live array was the bug: the rest of an interrupted
//!     hook ran straight over the open prompt and the next dying card's prompt was silently dropped,
//!     since `openPrompt` refuses to overwrite one that is already open.
//!
//! R78 and R89 are why the parked item carries whole instances rather than ids. Leaving the field
//! resets an instance, so a Death hook reads the snapshot taken just before the move — and a
//! continuation cannot re-derive that snapshot from the board, because the board no longer has it.
//! The snapshot therefore travels in `resume.data` and is what `ctx.self` is on the way back too,
//! which is the whole of R89's "a Death hook still reads the whole snapshot".
//!
//! R117: the pass is owed at the moment it pauses and never in advance. While `runDeathPass` is on
//! the stack the cards it has not reached are its own, so a resolution loop running inside one of
//! them can neither take nor re-run the step it is standing in.
//!
//! Port of `packages/engine/src/stateCheck.ts` (part 3, SURFACE §4). TS walked live instances and
//! wrote through them; here a card is read as a copy and written back through the state by its id,
//! and a card taken off the field is moved as the copy it was read as (`zones::move_to_zone` and
//! `zones::place_on_field` take the instance they move, as TS's did). The work handler TS registered
//! for `DEATHS_WORK` (`registerWorkHandler`) is `run_owed_deaths`, which `work.rs`'s dispatcher calls
//! (SURFACE §6.6). `STATE_CHECK_PASS_CAP` lives in `config.rs` (CLAUDE.md rule 9).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{BACKROW_ZONES, STATE_CHECK_PASS_CAP, UNIT_ZONES};
use crate::prompts::{ListStatus, ResumePlan, SELF_KEY, run_resumable_list};
use crate::resolve::{HookOptions, make_context};
use crate::script::{EngineSink, HookArgs, Script};
use crate::state::{CardInstance, GameState, Resume, WorkItem, find_instance, find_instance_mut};
use crate::wire::{
    AttackHealth, GameEvent, GameOverReason, Keyword, KeywordKind, PLAYER_IDS, PlayerId, Position, Row, Winner,
    Zone, ZoneName, ZoneRef, has_keyword,
};
use crate::work::{PAUSE_KEY, PausedStep, WorkPlan, paused_of};
use crate::zones::{MoveResult, OffFieldZone, PlaceOnFieldOptions, ZoneSlot};

/// A row's lanes (§3), 1 up (TS `zones.slotsOf`'s count).
fn row_size(row: Row) -> i32 {
    match row {
        Row::Units => UNIT_ZONES,
        Row::Backrow => BACKROW_ZONES,
    }
}

/// TS `zones.slotOf(state, instance)`: the field zone a card stands in, or null off the field.
fn slot_of(card: &CardInstance) -> Option<ZoneRef> {
    match card.zone {
        Zone::Field { player, row, lane } => Some(ZoneRef { player, row, lane }),
        _ => None,
    }
}

/// TS `zones.isReserved(state, ref)`: R64's hold for a dying Reborn unit, or B3.1's home zone.
fn is_reserved(state: &GameState, at: &ZoneRef) -> bool {
    if state
        .reserved
        .iter()
        .any(|r| r.player == at.player && r.row == at.row && r.lane == at.lane)
    {
        return true;
    }
    state.homes.iter().flatten().any(|home| {
        home.zone.player == at.player && home.zone.row == at.row && home.zone.lane == at.lane
    })
}

/// TS `zones.reserveZone(state, ref)`.
fn reserve_zone(state: &mut GameState, at: &ZoneRef) {
    if !is_reserved(state, at) {
        state.reserved.push(*at);
    }
}

/// TS `zones.releaseZone(state, ref)`.
fn release_zone(state: &mut GameState, at: &ZoneRef) {
    state
        .reserved
        .retain(|r| !(r.player == at.player && r.row == at.row && r.lane == at.lane));
}

/// The card as it stands in the state now, by id; the copy itself when it is nowhere.
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id).cloned().unwrap_or_else(|| card.clone())
}

/// Every unit on the field: the card that acts in each unit zone, the top of its pile (§3.2).
///
/// §4.5 step 1 collects "units with health 0 or less", and a card dormant under a Stack is not on
/// the field (§3.2, R13, R174) — so the check never reaches under a pile. The top shields what is
/// buried: an aura or a layer-2 Felinor that stops reaching a buried card cannot kill it there, and a
/// dormant card that could not survive is judged the moment it resumes on top, when the check reads
/// it with the board's auras again. It keeps its damage meanwhile (§3.2), and nothing can mark it
/// destroyed, since nothing can target it (R90). Collecting buried cards killed a damaged card the
/// moment a Stack card buried it and a positive aura stopped reaching it, and let a buried Reborn
/// card come back on top of the card acting in its zone (R175 returns onto a pile only a unit that
/// died on top of it).
///
/// TS read this through `zones.activeUnitsOf`; this is that walk, kept here: each unit zone's top in
/// lane order, then the Units the carriers hold, in backrow lane order (R446).
fn units_of(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    let side = &state.players[player];
    let mut units: Vec<CardInstance> = Vec::new();
    for lane in 1..=UNIT_ZONES {
        if let Some(top) = side
            .units
            .get((lane - 1) as usize)
            .and_then(|pile| pile.as_ref())
            .and_then(|pile| pile.first())
        {
            units.push(top.clone());
        }
    }
    for card in side.carried.iter().flatten().flatten() {
        units.push(card.clone());
    }
    units
}

/// §4.5 step 1: a unit dies at 0 or less health or when marked destroyed. Indestructible units are
/// not collected for damage or a destroy mark, but one whose max health has fallen to 0 or less is
/// (R69); one merely at 0 or less health with positive max health stays.
fn is_dying(sink: &EngineSink<'_>, unit: &CardInstance) -> bool {
    let view = crate::layers::unit_view(&*sink.state, unit);
    let indestructible = has_keyword(&view.keywords, KeywordKind::Indestructible);
    if indestructible {
        return view.max_health <= 0;
    }
    view.health <= 0 || unit.marked_destroyed == Some(true)
}

fn hero_check(sink: &mut EngineSink<'_>) -> bool {
    let dead: Vec<PlayerId> = PLAYER_IDS
        .into_iter()
        .filter(|player| sink.state.players[*player].hero.health <= 0)
        .collect();
    if dead.is_empty() {
        return false;
    }
    let winner = if dead.len() == 2 {
        Winner::Draw
    } else if dead[0] == PlayerId::P1 {
        Winner::P2
    } else {
        Winner::P1
    };
    let reason = if dead.len() == 2 {
        GameOverReason::BothHeroesDead
    } else {
        GameOverReason::HeroDeath
    };
    crate::game_over::end_game(sink, winner, reason);
    true
}

/// Every backrow card in play for this player, in lane order (§3).
fn backrow_of(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    let side = &state.players[player];
    (1..=BACKROW_ZONES)
        .filter_map(|lane| side.backrow.get((lane - 1) as usize).and_then(|card| card.clone()))
        .collect()
}

/// R46: a marked Indestructible unit switches to Attack Position and loses Taunt for the turn.
/// A backrow card has no position, so an Indestructible Field Spell simply keeps its zone and the
/// mark is dropped with no event.
///
/// §10.3: both halves are reported. The switch is `positionSwitched`, and only when there is one
/// (R91); the Taunt the knock-down takes — one the unit still has in Attack Position, printed,
/// granted or from an aura — is a `keywordGranted` with `lost` set, since no event type says a
/// keyword went and one is not added for this (R46). A unit already in Attack Position has nothing
/// to switch, so without it the change both views show, and the one that decides what may be
/// attacked (§4.2 step 3), would go out with no event at all.
fn resolve_indestructible_marks(sink: &mut EngineSink<'_>) {
    for player in PLAYER_IDS {
        for unit in units_of(sink.state, player) {
            if unit.marked_destroyed != Some(true) {
                continue;
            }
            let view = crate::layers::unit_view(sink.state, &unit);
            if !has_keyword(&view.keywords, KeywordKind::Indestructible) || view.max_health <= 0 {
                continue;
            }
            if let Some(card) = find_instance_mut(sink.state, &unit.id) {
                card.marked_destroyed = Some(false);
            }
            // R91: a unit already in Attack Position has nothing to switch, so no switch is reported — the
            // event is §10.10's 90° turn, and a unit that did not move must not be seen to.
            if view.position != Position::Atk {
                if let Some(card) = find_instance_mut(sink.state, &unit.id) {
                    card.position = Some(Position::Atk);
                }
                sink.events.push(GameEvent::PositionSwitched {
                    instance_id: unit.id.clone(),
                    position: Position::Atk,
                });
            }
            // The Taunt it has in Attack Position, which the suppression takes (a second knock-down the
            // same turn finds none left to take). R347 keeps Taunt off a unit while it is Indestructible,
            // so there is none here to report; the stamp still matters if it stops being Indestructible
            // before the turn ends (a Vanilla).
            let now = live(sink.state, &unit);
            let had_taunt = has_keyword(&crate::layers::unit_view(sink.state, &now).keywords, KeywordKind::Taunt);
            let turn = sink.state.turn;
            if let Some(card) = find_instance_mut(sink.state, &unit.id) {
                card.taunt_suppressed_turn = Some(turn);
            }
            if had_taunt {
                sink.events.push(GameEvent::KeywordGranted {
                    instance_id: unit.id.clone(),
                    keyword: Keyword::Taunt,
                    lost: Some(true),
                });
            }
        }
        for card in backrow_of(sink.state, player) {
            if card.marked_destroyed != Some(true) {
                continue;
            }
            if !has_keyword(&crate::layers::unit_view(sink.state, &card).keywords, KeywordKind::Indestructible) {
                continue;
            }
            if let Some(live_card) = find_instance_mut(sink.state, &card.id) {
                live_card.marked_destroyed = Some(false);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// One pass of §4.5 steps 3 to 5, and the remainder it owes (R113, R117)
// ---------------------------------------------------------------------------

/// R113: the `resume.hook` of the one work item this module parks — the rest of a state-check pass.
/// It is an engine sequence and not a card's, so the name is one no `Script` can hold, and
/// `runOwedDeaths` below is registered for it at module scope, in the module that owns the sequence
/// and never from a test: `work.runWorkItem` raises on a hook nothing knows, and a pass that cannot
/// be resumed is exactly the lost sequence R113 exists to prevent.
pub const DEATHS_WORK: &str = "@deaths";

/// The pass has one step, named so a reader of `state.work` can see what is owed.
const DEATHS_STEP: &str = "hooks";

/// Where the pass sits inside `resume.data`, so `work.ts`'s pause block keeps its own key.
const PASS_KEY: &str = "pass";

/// What one pass of the check still owes once its collected cards have moved (§4.5 steps 3 to 5),
/// as plain JSON:
///
///  - `owed`   — step 3's Death hooks still to fire, in R68's order, each as the snapshot of the card
///               taken just before it left the field. R78 has already reset the instance on the
///               board, so the snapshot is the only place the hook's `ctx.self` can come from (R89),
///               and a continuation cannot re-derive it — hence it travels here (R127).
///  - `reborn` — step 4's returns: which collected unit had Reborn and which zone it reserved (R64).
///               The instance is found again by id when the step runs, so a Death hook that removed
///               it in between cannot be resurrected by a stale reference.
///  - `collected` — step 5's report: `enteredGraveyard` for everything that is still in a graveyard
///               once Reborn has taken its own back out (R47).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeathPass {
    pub owed: Vec<CardInstance>,
    pub reborn: Vec<RebornEntry>,
    pub collected: Vec<CollectedEntry>,
    /// A Sacrifice's pass (§6.3, `sacrificeNow`, `sacrificeTogether`) rather than the state check's own:
    /// one effect inside the list that made it, so finishing it after a Death hook's question does not
    /// run the check — that is owed after the whole list (§4.5, R59), as it is when nothing asks. Only
    /// the check's own pass goes round again (§4.5 step 5). Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sacrificed: Option<bool>,
}

/// One of step 4's returns (`DeathPass.reborn`).
///
/// `token` is set for a unit token, which ceased to exist as it left the field (R11) and so cannot
/// be found again by id: it is the card as it left, which step 4 brings back instead (R175).
/// `face` is the X/X a token was summoned with (§7's `statsOverride`, and the Bread Token's
/// `armorOverride`), which is its printed face (§10.4 layer 1) and so comes back with it (R175).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RebornEntry {
    pub id: String,
    pub at: ZoneRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<CardInstance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<RebornFace>,
}

/// One of step 5's reports (`DeathPass.collected`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct CollectedEntry {
    pub id: String,
    pub def_id: String,
    pub owner: PlayerId,
}

/// R175: the face a Reborn body comes back with when the card was summoned X/X. §10.4 layer 1 reads a
/// token's printed stats off `statsOverride` ("printed 0/0; always summoned as X/X", §7), so it is the
/// card's face and not a change to it: R78's reset takes the override off a card that leaves the field
/// for a pile, but a body that returns at 1 health returns as the card it was printed as, and a Bread
/// Token that came back 0/0 at 0 health would only die again at the next pass.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct RebornFace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats_override: Option<AttackHealth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_override: Option<i32>,
}

fn reborn_face_of(unit: &CardInstance) -> Option<RebornFace> {
    if unit.stats_override.is_none() && unit.armor_override.is_none() {
        return None;
    }
    Some(RebornFace {
        stats_override: unit.stats_override,
        armor_override: unit.armor_override,
    })
}

/// Plain JSON, never a live array: what is parked must survive a round trip (§9.3, §10.1).
fn pass_json(pass: &DeathPass) -> Value {
    serde_json::to_value(pass).expect("a death pass is plain JSON (§9.3)")
}

fn instances_of(raw: Option<&Value>) -> Vec<CardInstance> {
    let Some(Value::Array(items)) = raw else {
        return Vec::new();
    };
    items
        .iter()
        .filter(|card| card.as_object().is_some_and(|card| card.get("id").is_some_and(Value::is_string)))
        .filter_map(|card| serde_json::from_value::<CardInstance>(card.clone()).ok())
        .collect()
}

/// The entries of a parked list, read back one by one (TS kept whatever the array held).
fn entries_of<T: serde::de::DeserializeOwned>(raw: Option<&Value>) -> Vec<T> {
    let Some(Value::Array(items)) = raw else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| serde_json::from_value::<T>(item.clone()).ok())
        .collect()
}

/// What a `DEATHS_WORK` item owes, or null when it is not one: the reader for its payload, read back
/// defensively because the item came through JSON (§10.1).
pub fn owed_deaths_of(resume: &Resume) -> Option<DeathPass> {
    if resume.hook != DEATHS_WORK {
        return None;
    }
    let raw = resume.data.get(PASS_KEY)?;
    if !(raw.is_object() || raw.is_array()) {
        return None;
    }
    Some(DeathPass {
        owed: instances_of(raw.get("owed")),
        reborn: entries_of(raw.get("reborn")),
        collected: entries_of(raw.get("collected")),
        sacrificed: if raw.get("sacrificed") == Some(&Value::Bool(true)) {
            Some(true)
        } else {
            None
        },
    })
}

/// The continuation of the pass as it stands now. `prompts.applyResumable` is handed this as its
/// plan, so the tail of a Death hook that asks is parked as *this* item — the hook's own remaining
/// effects and the rest of the pass in one record, which is what keeps R113's order right without
/// two items racing each other.
fn plan_for(pass: &DeathPass, owner: PlayerId) -> ResumePlan {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(PASS_KEY.to_string(), pass_json(pass));
    WorkPlan::new(
        Resume {
            def_id: String::new(),
            hook: DEATHS_WORK.to_string(),
            step: DEATHS_STEP.to_string(),
            radiant: false,
            instance_id: None,
            data,
        },
        owner,
    )
}

/// Park the rest of the pass (R113). `work.ts` owns `state.work`, so this only ever calls `owe`: the
/// item lands at `state.workCursor`, which puts it behind anything the pausing hook's own effects
/// parked inside it and — when a resumption parks again — in front of everything else still owed.
///
/// R117: every caller calls this at the moment it actually pauses and never in advance.
///
/// TS's `owe(sink, resume)` parks a `Resume` as a new item through `pushWork` with the default owner
/// (the active player); that is the call made here.
fn owe_deaths(sink: &mut EngineSink<'_>, pass: &DeathPass, step: Option<&PausedStep>) {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(PASS_KEY.to_string(), pass_json(pass));
    if let Some(step) = step {
        data.insert(
            PAUSE_KEY.to_string(),
            serde_json::to_value(step).expect("a paused step is plain JSON (§9.3)"),
        );
    }
    let resume = Resume {
        def_id: String::new(),
        hook: DEATHS_WORK.to_string(),
        step: DEATHS_STEP.to_string(),
        radiant: false,
        instance_id: None,
        data,
    };
    crate::work::push_work(sink, resume, None);
}

/// What §4.5 step 4 and R175 do to a body on its way back: no Reborn, no Vanilla, and an X/X token's
/// X/X kept as its printed face.
fn prepare_body(card: &mut CardInstance, entry: &RebornEntry) {
    card.granted_keywords.retain(|keyword| keyword.kind() != KeywordKind::Reborn);
    card.vanilla = false;
    // R175: an X/X token's X/X is its printed face, so the body keeps it through the reset.
    if let Some(face) = &entry.face {
        if let Some(stats) = face.stats_override {
            card.stats_override = Some(stats);
        }
        if let Some(armor) = face.armor_override {
            card.armor_override = Some(armor);
        }
    }
}

/// §4.5 step 4: each collected unit that had Reborn returns to its reserved zone (Locked since or
/// not, R688: the return is no play) at 1 health without Reborn, as a reset instance (R78); its Cry
/// does not fire, and because it has entered the field again it is summoning sick for the rest of
/// that turn (R83).
fn reborn_step(sink: &mut EngineSink<'_>, pass: &DeathPass) {
    // §4.5 step 4 returns every collected Reborn unit in one step, at 1 health: the bodies are all put
    // back first, and each one's 1 health is read once they all stand, so a body whose layers read the
    // others — a Felinor Fiender's layer 2 summing the Felinors that came back with it (R116) — is at 1
    // whichever lane comes first (R89's "read before any of them moves", from the other side).
    let mut back: Vec<(String, ZoneRef)> = Vec::new();
    for entry in &pass.reborn {
        // R563: a C+ #35 Rollback that let the zone go has given it to the snapshot's card; no return.
        if !is_reserved(sink.state, &entry.at) {
            continue;
        }
        release_zone(sink.state, &entry.at);
        // R127's shape at the level of a unit: the pass names it by id, so a Death hook that exiled or
        // unmade it in between leaves nothing to bring back rather than a stale object to resurrect.
        // A unit token is the exception R175 makes: it ceased to exist as it left (R11), so no pile
        // holds it and the pass carries the card as it left instead, reset the way R78 resets any
        // Reborn body on its way out.
        let mut copy = match &entry.token {
            None => match find_instance(sink.state, &entry.id) {
                Some(card) => card.clone(),
                None => continue,
            },
            Some(token) => reborn_token(token),
        };
        // §4.5 step 4 returns the card from the graveyard step 1 moved it to. A Death hook of the same
        // pass can have moved it on — exiled with the graveyard, returned to a hand — and it returns from
        // nowhere else: taken off the field from there, it would stay in that pile as well, one card in
        // two zones (§10.1).
        if entry.token.is_none() && copy.zone.z() != ZoneName::Graveyard {
            continue;
        }
        prepare_body(&mut copy, entry);
        // TS changed the graveyard's own object, so a body that finds no room keeps these changes there.
        if entry.token.is_none()
            && let Some(card) = find_instance_mut(sink.state, &entry.id)
        {
            prepare_body(card, entry);
        }
        // R175: a unit that died on top of a Stack pile left the card beneath to resume in its zone
        // (§3.2), and that card did not enter anything, so the zone is still the one R64 reserved. The
        // body returns on top of the pile, and the card beneath goes dormant again. With no pile the
        // zone is empty, which `stack` never changes: every other card was kept out by the reservation.
        let slot = ZoneSlot {
            player: entry.at.player,
            row: entry.at.row,
            lane: entry.at.lane,
        };
        if !crate::zones::place_on_field(sink.state, &mut copy, &slot, PlaceOnFieldOptions { stack: Some(true) }) {
            continue;
        }
        let owner = copy.owner;
        let id = copy.id.clone();
        sink.state.players[owner].graveyard.retain(|card| card.id != id);
        let turn = sink.state.turn;
        if let Some(body) = find_instance_mut(sink.state, &id) {
            body.reborn_spent = Some(true);
            // R83: it enters the field again now, so it is summoning sick like any fresh summon.
            body.summoned_turn = Some(turn);
        }
        back.push((id, entry.at));
    }
    for (id, _) in &back {
        let Some(body) = find_instance(sink.state, id).cloned() else {
            continue;
        };
        let max_health = crate::layers::unit_view(sink.state, &body).max_health;
        if let Some(card) = find_instance_mut(sink.state, id) {
            card.damage = (max_health - 1).max(0);
        }
    }
    for (id, at) in &back {
        let Some(body) = find_instance(sink.state, id) else {
            continue;
        };
        let event = GameEvent::Summoned {
            player: body.controller,
            instance_id: body.id.clone(),
            def_id: body.def_id.clone(),
            row: at.row,
            lane: at.lane,
            former_id: None,
            arrived_during: None,
            exits_from: None,
        };
        sink.events.push(event);
    }
}

/// R175: the body a unit token with Reborn comes back as. The snapshot is the token as it left the
/// field, so R78's reset is applied here, where `moveToZone` would have applied it had the token
/// reached a pile: damage, buffs, granted keywords, counters, memory, exertion and controller go,
/// and the zone is set by `placeOnField`.
fn reborn_token(snapshot: &CardInstance) -> CardInstance {
    let mut body = snapshot.clone();
    crate::zones::reset_instance(&mut body);
    body
}

/// A card that came straight back through Reborn never stayed in the graveyard, so only the ones
/// still there are reported (R688: a Locked zone takes the return, so no Reborn body stays for it).
fn graveyard_step(sink: &mut EngineSink<'_>, pass: &DeathPass) {
    for entry in &pass.collected {
        let stayed = sink.state.players[entry.owner]
            .graveyard
            .iter()
            .any(|card| card.id == entry.id);
        if !stayed {
            continue;
        }
        sink.events.push(GameEvent::EnteredGraveyard {
            instance_id: entry.id.clone(),
            def_id: entry.def_id.clone(),
            owner: entry.owner,
        });
    }
}

/// §4.5 steps 3 to 5 for one pass, resumably. Returns true when the pass finished, false when a
/// prompt (or the end of the game) stopped it — in which case what is left is on `state.work`.
///
/// `at` is the control block a resumption brings back: the first card in `owed` is then part-way
/// through its own effect list and continues at that index, with the selections the pause captured.
fn run_death_pass(sink: &mut EngineSink<'_>, pass: &mut DeathPass, at: Option<PausedStep>) -> bool {
    let mut resume_at = at;

    loop {
        // The game ending stops the pass for good: there is nothing left to resume into.
        if sink.state.result.is_some() {
            return false;
        }

        let Some(snapshot) = pass.owed.first().cloned() else {
            break;
        };

        // §9.3: a prompt is state, so the cards from here on wait for the answer action — and they have
        // not had their Death hook, which is precisely what is owed (R113). A prompt already open when
        // the pass begins means it has fired nothing at all, so the whole of step 3 is owed.
        if sink.state.pending.is_some() {
            owe_deaths(sink, pass, resume_at.as_ref());
            return false;
        }

        // §5.2: the face the card was wearing as it died, which is the snapshot's own.
        let Some(hook) = crate::scripts::script_of(sink.state, &snapshot).death else {
            pass.owed.remove(0);
            resume_at = None;
            continue;
        };

        let paused = resume_at.take();
        let plan = plan_for(pass, snapshot.controller);
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert(
            SELF_KEY.to_string(),
            serde_json::to_value(&snapshot).expect("an instance is plain JSON (§10.1)"),
        );

        // A fused Death runs every ingredient's list (R77, R102), and a pause inside one continues in it.
        // `applyResumable` parks the whole rest of the hook as this pass's item (`planFor`), so the item
        // owes the cards after this one and steps 4 and 5 too. When the hook's very last effect asked,
        // there was nothing of the hook to park, and the pass still owes the rest: the card is done, so
        // the pass parks itself without it. Counting the pass's items before and after to tell the two
        // apart was fooled by a nested pass — a sacrifice in the hook whose own Death asked — which
        // parks an item of its own (R156).
        let status = {
            let mut ctx = make_context(
                sink,
                Some(&snapshot),
                HookOptions {
                    controller: Some(snapshot.controller),
                    targets: Some(paused.as_ref().map(|step| step.targets.clone()).unwrap_or_default()),
                    modes: Some(paused.as_ref().map(|step| step.modes.clone()).unwrap_or_default()),
                    // R89: a prompt this hook opens is answered in a later action, when the instance on the board
                    // is R78's reset one; the step it re-enters reads this snapshot instead (`prompts.runResume`).
                    data: Some(data),
                },
            );
            // R174: a hook the pause split keeps the mark its list began with.
            if let Some(exits_from) = paused.as_ref().and_then(|step| step.exits_from) {
                ctx.exits_from = Some(exits_from);
            }
            // R136: and the units its head summoned before the pause.
            if let Some(summoned) = paused.as_ref().and_then(|step| step.summoned.clone()) {
                ctx.summoned = Some(summoned);
            }
            let effects = hook(&mut ctx);
            run_resumable_list(&mut ctx, &plan, effects, paused)
        };
        if status == ListStatus::Done {
            pass.owed.remove(0);
            continue;
        }
        if status == ListStatus::Over || sink.state.result.is_some() {
            return false;
        }
        if status == ListStatus::Asked {
            pass.owed.remove(0);
            owe_deaths(sink, pass, None);
        }
        return false;
    }

    // Neither of these can ask anything, so they finish the pass once step 3 is done.
    reborn_step(sink, pass);
    graveyard_step(sink, pass);
    true
}

/// `work.ts`'s handler for a parked pass: the same pass, continued where it stopped (R113, R122).
/// Once a pass of the check is done the check goes round again, because §4.5 step 5 repeats until
/// nothing changes and the pause did not excuse the pass from its repeat. A Sacrifice's pass is one
/// effect of the list that made it, and the check waits for the whole list (§4.5, R59): the rest of
/// that list is owed behind this item (R113), so a question in the sacrificed unit's Death changes
/// nothing about when the check runs — a heal later in the same Cry still lands first, and a Tribute
/// paid at §10.5 step 2 still leaves the check to step 4's loop, which holds it (R118).
///
/// TS registered this for `DEATHS_WORK` at module scope (`registerWorkHandler`); `work.rs`'s
/// dispatcher calls it for that hook (SURFACE §6.6).
pub fn run_owed_deaths(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(mut pass) = owed_deaths_of(&item.resume) else {
        return;
    };
    if !run_death_pass(sink, &mut pass, paused_of(&item.resume.data)) {
        return;
    }
    if pass.sacrificed != Some(true) {
        state_check(sink);
    }
}

// ---------------------------------------------------------------------------
// The check itself (§4.5)
// ---------------------------------------------------------------------------

// §4.5 loops until nothing changes; `STATE_CHECK_PASS_CAP` (config.rs) bounds a pathological loop
// loudly (R69, R89).

/// How a collected card left: the state check's own collection, or a Sacrifice (§6.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeathCause {
    Collected,
    Sacrificed,
}

/// One collected card as it was read before any of them moved.
struct ReadCard {
    unit: CardInstance,
    reborn: bool,
    attack: i32,
    max_health: i32,
    at: Option<ZoneRef>,
    token: bool,
    snapshot: CardInstance,
}

/// §4.5 step 1: collect, in R68's order, and move them all at once — units by health or a destroy
/// mark, backrow cards by a destroy mark, since they have no health of their own — leaving the pass
/// that steps 3 to 5 still owe.
///
/// "At once" is two loops, not one. Every collected card is read — its layers for R89's `destroyed`
/// event, its Reborn for step 4 and its snapshot for step 3 — before any of them moves, because a
/// card's layers depend on the others: an aura source (#65.1 Spikey Pillow's −2 attack) or a Felinor
/// a #92 Felinor Fiender counts at layer 2 that was moved first would leave the next card read
/// without it, so what it "was as it died" would hang on nothing but lane order.
///
/// A Sacrifice (§6.3) "counts as a death" and reaches the same pass through `sacrificeNow`: it is no
/// damage instance, so it names no killer (R42), and it bypasses Indestructible, which is why the
/// caller rather than `isDying` decides it dies.
fn collect(sink: &mut EngineSink<'_>, dying: &[CardInstance], cause: DeathCause) -> DeathPass {
    let mut pass = DeathPass::default();

    let read: Vec<ReadCard> = dying
        .iter()
        .map(|card| {
            let unit = live(sink.state, card);
            let view = crate::layers::unit_view(sink.state, &unit);
            ReadCard {
                reborn: has_keyword(&view.keywords, KeywordKind::Reborn),
                attack: view.attack,
                max_health: view.max_health,
                at: slot_of(&unit),
                token: crate::zones::is_unit_token(sink.state, &unit),
                // R78 resets an instance as it leaves, so the Death hook of step 3 reads this snapshot (R89).
                snapshot: unit.clone(),
                unit,
            }
        })
        .collect();

    // R463: the cards collected together leave the field together — every one of them is off the field
    // before any lands — so a "would go to a graveyard" replacement (B5 E5) is read with all of them
    // gone: a Voidwalker dying in this pass has taken its aura with it for the cards beside it too.
    for card in &read {
        crate::zones::remove_from_field(sink.state, &card.unit, Default::default());
    }

    for mut card in read {
        let landed = crate::zones::move_to_zone(sink.state, &mut card.unit, OffFieldZone::Graveyard, Default::default());
        if matches!(landed, MoveResult::Replaced) {
            // R461: a card exiled (or sent to its library) instead of reaching a graveyard has not died: no
            // Death hook, no Reborn, no `destroyed` for "destroys a Unit" (R42) or the destroyed count (R55).
            crate::zones::report_graveyard_landing(sink, &card.unit, landed);
            continue;
        }
        pass.owed.push(card.snapshot.clone());
        pass.collected.push(CollectedEntry {
            id: card.unit.id.clone(),
            def_id: card.unit.def_id.clone(),
            owner: card.unit.owner,
        });
        if card.reborn
            && let Some(at) = card.at
        {
            reserve_zone(sink.state, &at);
            // R175: a unit token ceases to exist below and no pile will hold it, so its return is
            // carried by the pass itself, with the X/X it was summoned as.
            // Read off the snapshot: the move above has already reset the instance (R78).
            let face = reborn_face_of(&card.snapshot);
            pass.reborn.push(RebornEntry {
                id: card.unit.id.clone(),
                at,
                token: if card.token { Some(card.snapshot.clone()) } else { None },
                face,
            });
        }
        sink.state.counters.destroyed += 1;
        // R89: the event carries what the card was, since R78 resets the instance as it leaves.
        sink.events.push(GameEvent::Destroyed {
            instance_id: card.unit.id.clone(),
            def_id: card.unit.def_id.clone(),
            owner: card.unit.owner,
            // Read off the snapshot: the move above has reset the instance's controller to its owner (R78).
            controller: card.snapshot.controller,
            attack: card.attack,
            max_health: card.max_health,
            // R42, R89: the unit whose damage instance was lethal. `damage.ts` credits a hit only as it
            // takes the unit from above 0 health to 0 or less (or Poisonous marks it), and a destroy
            // effect clears the credit as it marks (`effects/destroy.ts`), so a unit a spell destroyed or
            // an aura starved after some unit damaged it has no killer. A sacrifice has none either.
            killer_id: if cause == DeathCause::Sacrificed {
                None
            } else {
                card.snapshot.last_damaged_by.clone()
            },
            // R89: its face as it died, which a unit token that has ceased to exist can no longer tell
            // (C+ #12.8 Frostspatula remembers what it killed by definition and face, R409).
            radiant: if card.snapshot.radiant { Some(true) } else { None },
        });
    }

    pass
}

/// §6.3 Sacrifice: the card goes from the field to its owner's graveyard at once, bypassing
/// Indestructible, and "counts as a death" — so it is §4.5's death in full rather than a move with a
/// Death hook bolted on: the destroyed counter (R55) and the `destroyed` event (R89), its Death hook
/// off the snapshot (R78), and §6.1's Reborn, which returns a sacrificed Reborn unit to the zone it
/// reserved at 1 health (R64, R83), exactly as the state check returns a unit that died there. A
/// Death hook that asks pauses the rest of the pass on `state.work` like any other (R113). Step 2's
/// hero check is left to the state check that follows the whole effect (R59): a sacrifice is one
/// effect among the list that made it, and nothing about it touches a hero.
pub fn sacrifice_now(sink: &mut EngineSink<'_>, card: &CardInstance) {
    let mut pass = collect(sink, std::slice::from_ref(card), DeathCause::Sacrificed);
    pass.sacrificed = Some(true);
    run_death_pass(sink, &mut pass, None);
}

/// §6.3 Tribute: "sacrifice X of your units" is one payment, the tributed set of R101, so the set
/// dies together, the way §4.5 step 1 moves everything it collects at once: every unit is read before
/// any of them moves, then all move, and their Death hooks fire in R68's order — "Death triggers use
/// the same side and lane order (§4.5)" — not in the order a play happened to list them. Listed one
/// by one, the first sacrifice's Death ran before the others had died, so the client chose the order
/// the Deaths resolved in (#81's Death radiating a unit #86's Death was about to steal, or not).
pub fn sacrifice_together(sink: &mut EngineSink<'_>, cards: &[CardInstance]) {
    if cards.is_empty() {
        return;
    }
    // R68's walk: the active player's side, then the opponent's; units, then the backrow; lane 1 up.
    let order: [PlayerId; 2] = if sink.state.active == PlayerId::P1 {
        [PlayerId::P1, PlayerId::P2]
    } else {
        [PlayerId::P2, PlayerId::P1]
    };
    let mut ordered: Vec<CardInstance> = Vec::new();
    for player in order {
        for row in [Row::Units, Row::Backrow] {
            for lane in 1..=row_size(row) {
                for card in cards {
                    if let Some(at) = slot_of(card)
                        && at.player == player
                        && at.row == row
                        && at.lane == lane
                    {
                        ordered.push(card.clone());
                    }
                }
            }
        }
    }
    let mut pass = collect(sink, &ordered, DeathCause::Sacrificed);
    pass.sacrificed = Some(true);
    run_death_pass(sink, &mut pass, None);
}

/// R42: `lastDamagedBy` names the hit that took a unit to 0 or less health. A unit the check leaves
/// standing above 0 — healed, buffed, or given back its health by an aura leaving — was killed by no
/// hit, so an older credit must not name the killer of a death the layers cause later.
fn forget_spent_killers(sink: &mut EngineSink<'_>, survivors: &[CardInstance]) {
    for unit in survivors {
        if unit.last_damaged_by.is_none() {
            continue;
        }
        if crate::layers::unit_view(sink.state, unit).health > 0
            && let Some(card) = find_instance_mut(sink.state, &unit.id)
        {
            card.last_damaged_by = None;
        }
    }
}

/// TS `plague.permanentsOnField(state)`: the cards acting on the field, the active player's side first
/// (units, then the backrow, lane 1 up). A private copy.
fn permanents_in_play(state: &GameState) -> Vec<CardInstance> {
    let first = state.active;
    let mut out: Vec<CardInstance> = Vec::new();
    for player in [first, first.opponent()] {
        for lane in 1..=UNIT_ZONES {
            if let Some(top) = state.players[player]
                .units
                .get((lane - 1) as usize)
                .and_then(|pile| pile.as_ref())
                .and_then(|pile| pile.first())
            {
                out.push(top.clone());
            }
        }
        out.extend(backrow_of(state, player));
    }
    out
}

pub fn state_check(sink: &mut EngineSink<'_>) {
    for _pass in 0..STATE_CHECK_PASS_CAP {
        if sink.state.result.is_some() {
            return;
        }
        // R446: a Unit whose carrier is gone steps down, or is marked destroyed for want of a unit zone,
        // before anything is collected.
        crate::carriers::settle_carried(sink);
        resolve_indestructible_marks(sink);
        crate::modifiers::end_orphaned_modifiers(sink);
        crate::modifiers::install_lasting_modifiers(sink);
        // R403: a permanent whose "When …, Tribute this" (`Script.tributeWhen`) holds now — face-down ones
        // included — is sacrificed before anything is collected; a Vanilla one has no text (`scriptOf`).
        let tributes: Vec<CardInstance> = {
            let state = &*sink.state;
            permanents_in_play(state)
                .into_iter()
                .filter(|card| {
                    crate::scripts::script_of(state, card).tribute_when.as_ref().is_some_and(|when| {
                        when(HookArgs {
                            state,
                            self_: card,
                            radiant: card.radiant,
                        })
                    })
                })
                .collect()
        };
        if !tributes.is_empty() {
            sacrifice_together(sink, &tributes);
            if sink.state.result.is_some() || sink.state.pending.is_some() {
                return;
            }
            continue;
        }

        let order: [PlayerId; 2] = if sink.state.active == PlayerId::P1 {
            [PlayerId::P1, PlayerId::P2]
        } else {
            [PlayerId::P2, PlayerId::P1]
        };
        let units: Vec<CardInstance> = order.iter().flat_map(|player| units_of(sink.state, *player)).collect();
        let dying_ids: Vec<String> = units
            .iter()
            .filter(|unit| is_dying(sink, unit))
            .map(|unit| unit.id.clone())
            .collect();
        let survivors: Vec<CardInstance> = units
            .iter()
            .filter(|unit| !dying_ids.contains(&unit.id))
            .cloned()
            .collect();
        forget_spent_killers(sink, &survivors);
        // R68's order, which §4.5 step 3's Death hooks keep: side by side, the active player's first, and
        // within a side the units by lane and then the backrow by lane — a backrow card with a Death
        // (Classic+ #61, #12.8) fires in its side's place, not after every unit of both sides.
        let mut dying: Vec<CardInstance> = Vec::new();
        for player in order {
            dying.extend(
                units_of(sink.state, player)
                    .into_iter()
                    .filter(|unit| dying_ids.contains(&unit.id)),
            );
            dying.extend(
                backrow_of(sink.state, player)
                    .into_iter()
                    .filter(|card| card.marked_destroyed == Some(true)),
            );
        }

        if dying.is_empty() {
            if hero_check(sink) {
                return;
            }
            // B5 E33, R404: the board has settled, so a quest completed by what happened is noticed now.
            crate::subsystems::quests::notice_quests(sink);
            return;
        }

        // B5 E5: before any card moves, the units that would die meet the "would die" replacements
        // (Classic #14's Radiant flicker), which take some of them out of the collection (R462).
        let remaining: Vec<CardInstance> = crate::replacements::would_die_window(sink, &dying);
        let mut collected = collect(sink, &remaining, DeathCause::Collected);

        // Step 2: heroes.
        if hero_check(sink) {
            return;
        }

        // Steps 3 to 5, which a Death hook's prompt can pause: what is left is then owed in state and
        // `runOwedDeaths` brings the check back, repeat included.
        if !run_death_pass(sink, &mut collected, None) {
            return;
        }
    }

    // §4.5 repeats "until nothing changes"; a board that never settles is a bug, not a draw.
    panic!("the board did not settle in {STATE_CHECK_PASS_CAP} passes");
}
