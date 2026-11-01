//! Deaths and the state check (SPEC §4.5, SURFACE §4). Run after each action, each whole effect or
//! trigger, each cast-on-draw cast and each combat, never between the hits of one effect (R59).
//!
//! A Death hook (§4.5 step 3) can open a prompt, which ends the action (§9.3), so the pass is resumable
//! through `state.work` and nothing else (R113): the hook's list parks its own tail, and the pass (the
//! hooks still owed in R68's order, step 4's reserved zones, step 5's report) rides in the same work
//! item as plain JSON, so a paused board round-trips and replays exactly and the rest of an interrupted
//! hook cannot run over the open prompt. It carries whole instances, not ids (R78, R89): leaving the
//! field resets an instance, so the snapshot taken before the move is `ctx.self` on the way back too.
//! R117: the pass is owed the moment it pauses, never in advance, so a resolution loop inside one of
//! its hooks can neither take nor re-run the step it stands in.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{BACKROW_ZONES, STATE_CHECK_PASS_CAP, UNIT_ZONES};
use crate::layers::AuraTable;
use crate::prompts::{ListStatus, ResumePlan, SELF_KEY, run_resumable_list};
use crate::resolve::{HookOptions, make_context};
use crate::script::{EngineSink, HookArgs};
use crate::state::{
    CardInstance, GameState, Resume, WorkItem, active_first, find_instance, find_instance_mut,
};
use crate::wire::{
    AttackHealth, GameEvent, GameOverReason, Keyword, KeywordKind, PLAYER_IDS, PlayerId, Position, Row,
    Winner, Zone, ZoneName, ZoneRef, has_keyword,
};
use crate::work::{PAUSE_KEY, PausedStep, WorkPlan, paused_of};
use crate::zones::{MoveResult, OffFieldZone, PlaceOnFieldOptions, ZoneSlot, live_or};

/// Every unit on the field: the top of each unit zone's pile in lane order, then the Units the carriers
/// hold in backrow lane order (R446), then the Units standing in the backrow zones (R1081000), which the
/// death pass takes from here and not from `backrow_of`.
///
/// A card dormant under a Stack is not on the field (§3.2, R13, R174), so §4.5 step 1 never reaches
/// under a pile: an aura that stops reaching a buried card cannot kill it there; it keeps its damage,
/// cannot be marked (R90) and is judged once it resumes on top. Collecting buried cards also let a
/// buried Reborn card come back over the card acting in its zone (R175).
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
    units.extend(
        crate::zones::standing_units_of(state, player)
            .into_iter()
            .cloned(),
    );
    units
}

/// §4.5 step 1: a unit dies at 0 or less health or when marked destroyed; an Indestructible one only
/// when its max health has fallen to 0 or less (R69).
fn is_dying(auras: &AuraTable<'_>, unit: &CardInstance) -> bool {
    let view = auras.unit_view(unit);
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
        // R2201000: a floored seat never dies (its hero never reads below the floor anyway).
        .filter(|player| crate::state::can_lose(sink.state, *player))
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
        .filter_map(|lane| {
            side.backrow
                .get((lane - 1) as usize)
                .and_then(|card| card.clone())
        })
        .collect()
}

/// R46: a marked Indestructible unit switches to Attack Position and loses Taunt for the turn; a
/// backrow card has no position, so an Indestructible Field Spell only has its mark dropped. §10.3:
/// both are reported, the switch as `positionSwitched` only when there is one (R91), the Taunt as a
/// `keywordGranted` with `lost` set (no event says a keyword went), so the Taunt that decides what may
/// be attacked (§4.2 step 3) never goes silently.
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
            // R91: the event is §10.10's 90° turn, so a unit already in Attack Position reports none.
            if view.position != Position::Atk {
                if let Some(card) = find_instance_mut(sink.state, &unit.id) {
                    card.position = Some(Position::Atk);
                }
                sink.events.push(GameEvent::PositionSwitched {
                    instance_id: unit.id.clone(),
                    position: Position::Atk,
                });
            }
            // R347 keeps Taunt off an Indestructible unit, so there is none to report (nor on a second
            // knock-down); the stamp still matters if it stops being Indestructible this turn (a Vanilla).
            let now = live_or(sink.state, &unit);
            let had_taunt = has_keyword(
                &crate::layers::unit_view(sink.state, &now).keywords,
                KeywordKind::Taunt,
            );
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
            if !has_keyword(
                &crate::layers::unit_view(sink.state, &card).keywords,
                KeywordKind::Indestructible,
            ) {
                continue;
            }
            if let Some(live_card) = find_instance_mut(sink.state, &card.id) {
                live_card.marked_destroyed = Some(false);
            }
        }
    }
}

/// R1849006: while an aura says Units cannot be in Defense Position (Stinkyscape #23 Warp Disruptor),
/// every Unit on either side in Defense Position switches to Attack Position, active player's first
/// (R68) — the one that arrived in it, or stood in it as the aura came. An effect's switch (R20), so no
/// exertion is spent; `positionSwitched` reports it. R2179000: so does an Indestructible Unit in
/// Defense Position, aura or not — one that arrived in it, or became Indestructible standing in it —
/// and R2657000 a Unit with Can't Defend, the same way.
fn leave_defense(sink: &mut EngineSink<'_>) {
    let barred = crate::restrictions::defense_barred(sink.state);
    for player in active_first(sink.state) {
        for unit in units_of(sink.state, player) {
            if unit.position != Some(Position::Def) {
                continue;
            }
            if !barred
                && !crate::layers::unit_has(sink.state, &unit, KeywordKind::Indestructible)
                && !crate::layers::unit_has(sink.state, &unit, KeywordKind::CantDefend)
            {
                continue;
            }
            if let Some(card) = find_instance_mut(sink.state, &unit.id) {
                card.position = Some(Position::Atk);
            }
            sink.events.push(GameEvent::PositionSwitched {
                instance_id: unit.id.clone(),
                position: Position::Atk,
            });
        }
    }
}

// One pass of §4.5 steps 3 to 5, and the remainder it owes (R113, R117)

/// R113: the `resume.hook` of the one work item this module parks, the rest of a state-check pass.
/// An engine sequence's, so no `Script` can hold the name; `work.rs` dispatches it to
/// `run_owed_deaths`, since a pass that cannot be resumed is the lost sequence R113 prevents.
pub const DEATHS_WORK: &str = "@deaths";

/// The pass has one step, named so a reader of `state.work` can see what is owed.
const DEATHS_STEP: &str = "hooks";

/// Where the pass sits inside `resume.data`, so the pause block (`PAUSE_KEY`) keeps its own key.
const PASS_KEY: &str = "pass";

/// What one pass of the check still owes once its collected cards have moved (§4.5 steps 3 to 5):
///
///  - `owed`: step 3's Death hooks, in R68's order, each the card's snapshot from just before it left
///    the field, the only `ctx.self` once R78 has reset the instance (R89, R127).
///  - `reborn`: step 4's returns, each unit by id with the zone it reserved (R64), so one a Death hook
///    removed in between is not resurrected from a stale reference.
///  - `collected`: step 5's `enteredGraveyard` report, of what Reborn left in a graveyard (R47).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeathPass {
    pub owed: Vec<CardInstance>,
    pub reborn: Vec<RebornEntry>,
    pub collected: Vec<CollectedEntry>,
    /// A Sacrifice's pass (§6.3) rather than the check's own: one effect of the list that made it, so
    /// finishing it after a Death hook's question does not run the check, owed after the whole list
    /// (§4.5, R59); only the check's own pass goes round again (§4.5 step 5). Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sacrificed: Option<bool>,
    /// R1852000: the Starship Pieces among the collected cards, each its snapshot as it died (R89), in
    /// R68's order, which fuse into their owners' Dormant Starships at the pass's end. Absent when none
    /// died, so a pass without one serializes as before (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pieces: Option<Vec<CardInstance>>,
}

/// One of step 4's returns (`DeathPass.reborn`). `token` is the card as it left, for a unit token, which
/// ceased to exist (R11) and cannot be found by id; `face` is the X/X a card was summoned with (§7), its
/// printed face (§10.4 layer 1), which comes back with it (R175).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RebornEntry {
    pub id: String,
    pub at: ZoneRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<CardInstance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<RebornFace>,
    /// R1081000: the unit stood in a backrow zone itself, so it returns standing there. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing: Option<bool>,
}

/// One of step 5's reports (`DeathPass.collected`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct CollectedEntry {
    pub id: String,
    pub def_id: String,
    pub owner: PlayerId,
}

/// R175: the face a Reborn body summoned X/X comes back with. §10.4 layer 1 reads a token's printed
/// stats off `statsOverride`, so it is the card's face and outlasts R78's reset on a returning body: a
/// Bread Token back at 0/0 would only die again at the next pass.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct RebornFace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats_override: Option<AttackHealth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_override: Option<i32>,
}

fn reborn_face_of(state: &GameState, unit: &CardInstance) -> Option<RebornFace> {
    crate::zones::summoned_token_face(state, unit).map(|face| RebornFace {
        stats_override: face.stats_override,
        armor_override: face.armor_override,
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
        .filter(|card| {
            card.as_object()
                .is_some_and(|card| card.get("id").is_some_and(Value::is_string))
        })
        .filter_map(|card| serde_json::from_value::<CardInstance>(card.clone()).ok())
        .collect()
}

fn entries_of<T: serde::de::DeserializeOwned>(raw: Option<&Value>) -> Vec<T> {
    let Some(Value::Array(items)) = raw else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| serde_json::from_value::<T>(item.clone()).ok())
        .collect()
}

/// What a `DEATHS_WORK` item owes, or `None` for another item, read back defensively because the item
/// came through JSON (§10.1).
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
        pieces: raw.get("pieces").map(|pieces| instances_of(Some(pieces))),
    })
}

/// The continuation of the pass as it stands now, `run_resumable_list`'s plan, so a Death hook that
/// asks parks its own remaining effects and the rest of the pass as one item, which keeps R113's order
/// without two items racing.
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

/// Park the rest of the pass (R113) with the default owner (the active player): the item lands at the
/// work cursor, behind anything the pausing hook's own effects parked and, when a resumption parks
/// again, in front of everything else still owed. R117: every caller calls this the moment it pauses.
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
    card.granted_keywords
        .retain(|keyword| keyword.kind() != KeywordKind::Reborn);
    card.vanilla = false;
    if let Some(face) = &entry.face {
        if let Some(stats) = face.stats_override {
            card.stats_override = Some(stats);
        }
        if let Some(armor) = face.armor_override {
            card.armor_override = Some(armor);
        }
    }
}

/// §4.5 step 4: each collected unit that had Reborn returns to its reserved zone (Locked or not, R688:
/// the return is no play) at 1 health without Reborn, as a reset instance (R78), its Cry unfired and
/// summoning sick for the turn (R83).
fn reborn_step(sink: &mut EngineSink<'_>, pass: &DeathPass) {
    // Every body goes back before any one's 1 health is set, so one whose layers read the others (a
    // Felinor Fiender's layer 2, R116) is at 1 whichever lane comes first (R89).
    let mut back: Vec<(String, ZoneRef)> = Vec::new();
    for entry in &pass.reborn {
        // R563: a C+ #35 Rollback that let the zone go has given it to the snapshot's card; no return.
        if !crate::zones::is_reserved(sink.state, entry.at) {
            continue;
        }
        crate::zones::release_zone(sink.state, entry.at);
        // R127: found by id, so a unit a Death hook exiled or unmade in between is not resurrected; a
        // unit token is in no pile (R11, R175), so the pass carries it, reset as R78 resets any body.
        let mut copy = match &entry.token {
            None => match find_instance(sink.state, &entry.id) {
                Some(card) => card.clone(),
                None => continue,
            },
            Some(token) => reborn_token(token),
        };
        // §4.5 step 4 returns it only from the graveyard: one a Death hook moved on (exiled, to a hand)
        // would otherwise be in two zones at once (§10.1).
        if entry.token.is_none() && copy.zone.z() != ZoneName::Graveyard {
            continue;
        }
        prepare_body(&mut copy, entry);
        // A body that finds no room keeps these changes in the graveyard.
        if entry.token.is_none()
            && let Some(card) = find_instance_mut(sink.state, &entry.id)
        {
            prepare_body(card, entry);
        }
        // R175: a unit that died atop a Stack pile returns on top of it, and the card beneath, which never
        // left the zone R64 reserved (§3.2, R1829000), goes dormant again; with no pile the reservation
        // kept the zone empty, so `stack` changes nothing.
        let slot = ZoneSlot {
            player: entry.at.player,
            row: entry.at.row,
            lane: entry.at.lane,
        };
        let options = PlaceOnFieldOptions { stack: Some(true) };
        let landed = if entry.standing == Some(true) {
            crate::zones::place_standing_on_field(sink.state, &mut copy, slot, options)
        } else {
            crate::zones::place_on_field(sink.state, &mut copy, slot, options)
        };
        if !landed {
            continue;
        }
        let owner = copy.owner;
        let id = copy.id.clone();
        sink.state.players[owner].graveyard.retain(|card| card.id != id);
        let turn = sink.state.turn;
        if let Some(body) = find_instance_mut(sink.state, &id) {
            body.reborn_spent = Some(true);
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
            source_id: None,
            arrived_during: None,
            exits_from: None,
            creator: None,
        };
        sink.events.push(event);
        // R783010: a Crater body Locks the zone it came back into.
        crate::crater::lock_on_arrival(sink, id);
    }
}

/// R175: the body a unit token with Reborn comes back as. The snapshot is the token as it left the
/// field, so R78's reset is applied here, where `moveToZone` would have applied it had the token
/// reached a pile: damage, buffs, granted keywords, counters, memory, exertion and controller go,
/// and the zone is set by `placeOnField`. It comes back as the card of the player it died under, as a
/// card the move took to that player's graveyard does (R2618000).
fn reborn_token(snapshot: &CardInstance) -> CardInstance {
    let mut body = snapshot.clone();
    body.owner = body.controller;
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

/// §4.5 steps 3 to 5 for one pass, resumably: true when it finished, false when a prompt or the game's
/// end stopped it, the rest then on `state.work`. `at` is a resumption's control block: the first card
/// in `owed` continues part-way through its list, with the selections the pause captured.
fn run_death_pass(sink: &mut EngineSink<'_>, pass: &mut DeathPass, at: Option<PausedStep>) -> bool {
    let mut resume_at = at;

    loop {
        // A game that ended leaves nothing to resume into.
        if sink.state.result.is_some() {
            return false;
        }

        let Some(snapshot) = pass.owed.first().cloned() else {
            break;
        };

        // §9.3: a prompt is state, so the cards from here on, their Death hooks unfired, wait for the
        // answer (R113); one open as the pass begins owes the whole of step 3.
        if sink.state.pending.is_some() {
            owe_deaths(sink, pass, resume_at.as_ref());
            return false;
        }

        // §5.2: the face the card was wearing as it died, which is the snapshot's own.
        // ME-GRANT (MD-D13): the card's own Death, then each granted Death in the order granted.
        let Some(hook) = crate::grants::death_hook_of(sink.state, &snapshot) else {
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

        // A fused Death runs every ingredient's list (R77, R102), and a pause continues in it. A pause
        // parks the rest of the hook with the pass (`plan_for`); when the hook's last effect asked there
        // is none to park, so the pass parks itself without the card. The status tells the two apart,
        // not a count of items: a nested pass parks an item of its own (R156).
        let status = {
            let mut ctx = make_context(
                sink,
                Some(&snapshot),
                HookOptions {
                    controller: Some(snapshot.controller),
                    targets: Some(
                        paused
                            .as_ref()
                            .map(|step| step.targets.clone())
                            .unwrap_or_default(),
                    ),
                    modes: Some(paused.as_ref().map(|step| step.modes.clone()).unwrap_or_default()),
                    // R89: a prompt this hook opens is answered once R78 has reset the instance; the step
                    // it re-enters reads this snapshot instead.
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
    starship_step(sink, pass);
    true
}

/// R1852000: the pass's last step, each Starship Piece that died in it fusing a copy into its owner's
/// Dormant Starship in R68's order, after the Reborn bodies are back, so the Starship takes the leftmost
/// zone open after them. It asks nothing.
fn starship_step(sink: &mut EngineSink<'_>, pass: &DeathPass) {
    for snapshot in pass.pieces.iter().flatten() {
        if sink.state.result.is_some() {
            return;
        }
        crate::starship::fuse_dead_piece(sink, snapshot);
    }
}

/// `work.rs`'s handler for `DEATHS_WORK` (SURFACE §6.6): the parked pass, continued (R113, R122), then
/// the check again, as §4.5 step 5 repeats until nothing changes; not after a Sacrifice's pass, whose
/// list's rest is owed behind this item and the check waits for it (§4.5, R59), so a heal later in the
/// same Cry still lands first and a Tribute paid at §10.5 step 2 leaves the check to step 4's loop (R118).
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

// The check itself (§4.5)

// §4.5 loops until nothing changes; `STATE_CHECK_PASS_CAP` (config.rs, CLAUDE.md rule 9) bounds a
// pathological loop loudly (R69, R89).

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
    /// R1852000: it dies as a Starship Piece.
    piece: bool,
    attack: i32,
    max_health: i32,
    at: Option<ZoneRef>,
    token: bool,
    standing: bool,
    snapshot: CardInstance,
}

/// §4.5 step 1: move the collected cards (units by health or a destroy mark, backrow cards by a mark)
/// all at once, in R68's order, leaving the pass steps 3 to 5 owe. Every card is read (layers, Reborn,
/// snapshot) before any moves, since a card's layers depend on the others (#65.1 Spikey Pillow's aura,
/// a #92 Felinor Fiender's count), so what it "was as it died" (R89) does not hang on lane order. A
/// Sacrifice (§6.3) names no killer (R42) and bypasses Indestructible, so its caller decides it dies.
fn collect(sink: &mut EngineSink<'_>, dying: &[CardInstance], cause: DeathCause) -> DeathPass {
    let mut pass = DeathPass::default();

    let read: Vec<ReadCard> = dying
        .iter()
        .map(|card| {
            let unit = live_or(sink.state, card);
            let view = crate::layers::unit_view(sink.state, &unit);
            ReadCard {
                reborn: has_keyword(&view.keywords, KeywordKind::Reborn),
                piece: crate::starship::dies_as_piece(sink.state, &unit),
                attack: view.attack,
                max_health: view.max_health,
                at: crate::zones::slot_of(sink.state, &unit),
                token: crate::zones::is_unit_token(sink.state, &unit),
                standing: crate::zones::stands_in_backrow(sink.state, &unit),
                // R78 resets an instance as it leaves, so the Death hook of step 3 reads this snapshot (R89).
                snapshot: unit.clone(),
                unit,
            }
        })
        .collect();

    // R2396000: the killers that answer "Whenever this destroys a Unit" even once gone, each as it stands
    // now (one dying in this pass as its snapshot), kept for the dispatch of the kill it made.
    let gone_killers: IndexMap<String, CardInstance> = if cause == DeathCause::Sacrificed {
        IndexMap::new()
    } else {
        read.iter()
            .filter_map(|card| card.snapshot.last_damaged_by.as_deref())
            .filter_map(|killer_id| {
                read.iter()
                    .find(|other| other.unit.id == killer_id)
                    .map(|other| other.snapshot.clone())
                    .or_else(|| {
                        find_instance(sink.state, killer_id)
                            .filter(|killer| matches!(killer.zone, Zone::Field { .. }))
                            .cloned()
                    })
            })
            .filter(|killer| crate::triggers::answers_kills_when_gone(sink.state, killer))
            .map(|killer| (killer.id.clone(), killer))
            .collect()
    };

    // R463: the cards collected together leave the field together — every one of them is off the field
    // before any lands — so a "would go to a graveyard" replacement (B5 E5) is read with all of them
    // gone: a Voidwalker dying in this pass has taken its aura with it for the cards beside it too.
    for card in &read {
        crate::zones::remove_from_field(sink.state, &card.unit, Default::default());
    }
    // R823 (Meditative #10 Double Counting): a Death runs again for each extra run its controller's
    // multiplier gives, on the same snapshot (R89), the multiplier read with the collected cards gone
    // (R463), so one dying in this pass doubles nothing.
    let mut extra: IndexMap<PlayerId, usize> = IndexMap::new();
    for player in PLAYER_IDS {
        let runs =
            crate::multipliers::extra_runs(sink.state, player, crate::multipliers::Multiplied::CryAndDeath);
        extra.insert(player, runs.max(0) as usize);
    }

    for mut card in read {
        let landed = crate::zones::move_to_zone(
            sink.state,
            &mut card.unit,
            OffFieldZone::Graveyard,
            Default::default(),
        );
        if matches!(landed, MoveResult::Replaced) {
            // R461: a card exiled (or sent to its library) instead of reaching a graveyard has not died: no
            // Death hook, no Reborn, no `destroyed` for "destroys a Unit" (R42) or the destroyed count (R55).
            crate::zones::report_graveyard_landing(sink, &card.unit, landed);
            continue;
        }
        pass.owed.push(card.snapshot.clone());
        // R1852000: a piece that died, which the pass's last step fuses into its owner's Starship.
        if card.piece {
            pass.pieces
                .get_or_insert_with(Vec::new)
                .push(card.snapshot.clone());
        }
        let runs = extra.get(&card.snapshot.controller).copied().unwrap_or(0);
        if runs > 0
            && crate::scripts::script_of(sink.state, &card.snapshot)
                .death
                .is_some()
        {
            for _ in 0..runs {
                pass.owed.push(card.snapshot.clone());
            }
        }
        pass.collected.push(CollectedEntry {
            id: card.unit.id.clone(),
            def_id: card.unit.def_id.clone(),
            owner: card.unit.owner,
        });
        if card.reborn
            && let Some(at) = card.at
        {
            crate::zones::reserve_zone(sink.state, at);
            // R175: a unit token is in no pile, so the pass carries its return with the X/X it was
            // summoned as, read off the snapshot since the move above reset the instance (R78).
            let face = reborn_face_of(sink.state, &card.snapshot);
            pass.reborn.push(RebornEntry {
                id: card.unit.id.clone(),
                at,
                token: if card.token {
                    let mut body = card.snapshot.clone();
                    crate::zones::forget_set_stats(&mut body);
                    Some(body)
                } else {
                    None
                },
                face,
                standing: if card.standing { Some(true) } else { None },
            });
        }
        sink.state.counters.destroyed += 1;
        // R42, R89: the unit whose damage instance was lethal. `damage.ts` credits a hit only as it
        // takes the unit from above 0 health to 0 or less (or Poisonous marks it), and a destroy
        // effect clears the credit as it marks (`effects/destroy.ts`), so a unit a spell destroyed or
        // an aura starved after some unit damaged it has no killer. A sacrifice has none either.
        let killer_id = if cause == DeathCause::Sacrificed {
            None
        } else {
            card.snapshot.last_damaged_by.clone()
        };
        // R2396000: a killer that answers this kill even once gone waits for its dispatch.
        if let Some(killer) = killer_id.as_ref().and_then(|id| gone_killers.get(id)) {
            sink.state
                .kill_snapshots
                .get_or_insert_with(IndexMap::new)
                .insert(card.unit.id.clone(), killer.clone());
        }
        // R89: the event carries what the card was, since R78 resets the instance as it leaves.
        sink.events.push(GameEvent::Destroyed {
            instance_id: card.unit.id.clone(),
            def_id: card.unit.def_id.clone(),
            owner: card.unit.owner,
            // Read off the snapshot: the move above has reset the instance's controller to its owner (R78).
            controller: card.snapshot.controller,
            attack: card.attack,
            max_health: card.max_health,
            killer_id,
            // R89: its face as it died, which a unit token that has ceased to exist can no longer tell
            // (C+ #12.8 Frostspatula remembers what it killed by definition and face, R409).
            radiant: if card.snapshot.radiant { Some(true) } else { None },
        });
    }

    pass
}

/// §6.3 Sacrifice: the card goes from the field to its controller's graveyard at once (R2618000), bypassing
/// Indestructible, and "counts as a death" — so it is §4.5's death in full rather than a move with a
/// Death hook bolted on: the destroyed counter (R55) and the `destroyed` event (R89), its Death hook
/// off the snapshot (R78), and §6.1's Reborn, which returns a sacrificed Reborn unit to the zone it
/// reserved at 1 health (R64, R83), exactly as the state check returns a unit that died there. A
/// Death hook that asks pauses the rest of the pass on `state.work` like any other (R113). Step 2's
/// hero check is left to the state check that follows the whole effect (R59): a sacrifice is one
/// effect among the list that made it, and nothing about it touches a hero.
pub fn sacrifice_now(sink: &mut EngineSink<'_>, card: &CardInstance) {
    // R1220: a Sacrifice of an Untributable card does nothing.
    if crate::query::is_untributable(sink.state, card) {
        return;
    }
    let mut pass = collect(sink, std::slice::from_ref(card), DeathCause::Sacrificed);
    pass.sacrificed = Some(true);
    run_death_pass(sink, &mut pass, None);
}

/// §6.3 Tribute: "sacrifice X of your units" is one payment, R101's tributed set, so it dies together
/// as §4.5 step 1's collection does, its Deaths in R68's order and not the order a play listed them,
/// which would let the client order the Deaths (#81's Death radiating a unit #86's was to steal).
pub fn sacrifice_together(sink: &mut EngineSink<'_>, cards: &[CardInstance]) {
    // R1220: Untributable cards are dropped from a Sacrifice first; when none is left, nothing happens.
    let cards: Vec<CardInstance> = cards
        .iter()
        .filter(|card| !crate::query::is_untributable(sink.state, card))
        .cloned()
        .collect();
    if cards.is_empty() {
        return;
    }
    // R68's walk: the active player's side, then the opponent's; units, then the backrow; lane 1 up.
    let order = active_first(sink.state);
    let mut ordered: Vec<CardInstance> = Vec::new();
    for player in order {
        for row in [Row::Units, Row::Backrow] {
            for lane in 1..=crate::zones::row_size(row) {
                for card in &cards {
                    if let Some(at) = crate::zones::slot_of(sink.state, card)
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

/// MD-D31, R1124: §4.5 step 1 exiles the units damage marked ahead of deaths, in R68's order, by §6.3
/// Exile's move (`zones::exile_to_pile`, R11, R55): no Death, no Reborn, no `destroyed` (R461), even
/// when the hit was lethal.
fn exile_marked(sink: &mut EngineSink<'_>) {
    let order = active_first(sink.state);
    let mut marked: Vec<CardInstance> = Vec::new();
    for player in order {
        marked.extend(
            units_of(sink.state, player)
                .into_iter()
                .filter(|unit| unit.marked_exiled == Some(true)),
        );
    }
    for unit in marked {
        let Some(live) = find_instance(sink.state, &unit.id).cloned() else {
            continue;
        };
        if live.marked_exiled != Some(true) {
            continue;
        }
        crate::zones::exile_to_pile(sink, live);
    }
}

/// R42: `last_damaged_by` names the hit that took a unit to 0 or less health, so a unit the check leaves
/// standing above 0 drops it, and a death the layers cause later names no killer.
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

/// The cards acting on the field, the active player's side first (units, then the backrow, lane 1 up).
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
    // R460, R1875006: the cards whose would-die replacement flickered or resolved its follow-up in this
    // check, which answer no later pass of it, as each card replaces an event at most once.
    let mut resolved_first: Vec<String> = Vec::new();
    for _pass in 0..STATE_CHECK_PASS_CAP {
        if sink.state.result.is_some() {
            return;
        }
        // R446: a Unit whose carrier is gone steps down, or is marked destroyed for want of a unit zone,
        // before anything is collected.
        crate::carriers::settle_carried(sink);
        resolve_indestructible_marks(sink);
        leave_defense(sink);
        crate::modifiers::end_orphaned_modifiers(sink);
        crate::modifiers::install_lasting_modifiers(sink);
        // R403: a permanent whose "When …, Tribute this" (`tribute_when`) holds now, face-down ones
        // included, is sacrificed before anything is collected; a Vanilla one has no text.
        let tributes: Vec<CardInstance> = {
            let state = &*sink.state;
            // #892: one aura table for every permanent's Untributable.
            let auras = AuraTable::new(state);
            permanents_in_play(state)
                .into_iter()
                .filter(|card| {
                    // R1220: an Untributable card's `tributeWhen` never fires — without this, a card
                    // whose condition holds would loop the state check up to its pass cap.
                    !crate::query::is_untributable_in(&auras, card)
                        && crate::scripts::script_of(state, card)
                            .tribute_when
                            .as_ref()
                            .is_some_and(|when| {
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

        // MD-D31, R1124: the marked units leave for exile before anything is collected for death.
        // Exiling asks nothing, so a prompt already open (R156) does not stop the collection below.
        exile_marked(sink);
        if sink.state.result.is_some() {
            return;
        }
        // R1850003, R1850002, R1850005: the card-on-card verbs' standing rules, each a whole step that
        // the check starts over after when it changed something; then R2280001's owed animations.
        if crate::card_on_card::release_held(sink)
            || crate::card_on_card::standing_text(sink)
            || crate::animated::animate_owed(sink)
        {
            continue;
        }

        let order = active_first(sink.state);
        let units: Vec<CardInstance> = order
            .iter()
            .flat_map(|player| units_of(sink.state, *player))
            .collect();
        let dying_ids: Vec<String> = {
            // #892: one aura table for every unit's death check.
            let auras = AuraTable::new(&*sink.state);
            units
                .iter()
                .filter(|unit| is_dying(&auras, unit))
                .map(|unit| unit.id.clone())
                .collect()
        };
        let survivors: Vec<CardInstance> = units
            .iter()
            .filter(|unit| !dying_ids.contains(&unit.id))
            .cloned()
            .collect();
        forget_spent_killers(sink, &survivors);
        // R68's order, which §4.5 step 3's Deaths keep: the active player's side first, each side's units
        // by lane, then its backrow, so a backrow Death (Classic+ #61, #12.8) fires in its side's place.
        let mut dying: Vec<CardInstance> = Vec::new();
        for player in order {
            dying.extend(
                units_of(sink.state, player)
                    .into_iter()
                    .filter(|unit| dying_ids.contains(&unit.id)),
            );
            // R1081000: a Unit standing in the backrow is among `units_of`'s already, collected once.
            dying.extend(
                backrow_of(sink.state, player)
                    .into_iter()
                    .filter(|card| card.marked_destroyed == Some(true))
                    .filter(|card| !crate::zones::is_standing_unit(sink.state, card)),
            );
        }

        if dying.is_empty() {
            // R850: the settled game-end point — a hero at 0 loses first, then a held win wins.
            if hero_check(sink) || crate::win_conditions::win_check(sink) {
                return;
            }
            // B5 E33, R404: the board has settled, so a quest completed by what happened is noticed now.
            crate::subsystems::quests::notice_quests(sink);
            return;
        }

        // B5 E5: before any card moves, the units that would die meet the "would die" replacements
        // (Classic #14's Radiant flicker), which take some of them out of the collection (R462).
        let remaining: Vec<CardInstance> =
            match crate::replacements::would_die_window(sink, &dying, &mut resolved_first) {
                crate::replacements::WouldDie::Die(remaining) => remaining,
                // R1875006: a replacement resolved before anything moved: judge the board it left again.
                crate::replacements::WouldDie::StartOver => {
                    if sink.state.result.is_some() || sink.state.pending.is_some() {
                        return;
                    }
                    continue;
                }
            };
        // R1849002: what a destroy-to-exile aura reaches, judged for every card before any moves, is
        // exiled first in R68's order, not collected: no Death, Reborn or `destroyed` (R461, R1124).
        let (exiled, remaining): (Vec<CardInstance>, Vec<CardInstance>) = remaining
            .into_iter()
            .partition(|card| crate::replacements::destroy_exiles(sink.state, &live_or(sink.state, card)));
        for card in &exiled {
            let card = live_or(sink.state, card);
            crate::zones::exile_to_pile(sink, card);
        }
        let mut collected = collect(sink, &remaining, DeathCause::Collected);

        // Step 2: heroes.
        if hero_check(sink) {
            return;
        }

        // Steps 3 to 5, which a Death hook's prompt can pause: what is left is then owed in state and
        // `run_owed_deaths` brings the check back, repeat included.
        if !run_death_pass(sink, &mut collected, None) {
            return;
        }
    }

    // §4.5 repeats "until nothing changes"; a board that never settles is a bug, not a draw.
    panic!("the board did not settle in {STATE_CHECK_PASS_CAP} passes");
}
