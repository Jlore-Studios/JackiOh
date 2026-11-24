//! Fuzz-level invariants for zones, summoning sickness, exertion, game over, hidden information and owed
//! work (SPEC §3, §4.1, §4.2, §9.3, §10.3, §10.8, R11, R49, R53, R83, R113, R171, R636;
//! docs/polish/4-edge-cases.md "Fuzz invariants").
//! `cargo jackioh fuzz` runs one monitor per game (SURFACE §8, §12), `fuzz --duos` one per duel through
//! `DuosInvariantMonitor` (#1074; §13.4, §13.6), and the edge-case hunters reuse it.
//!
//! THE MONITOR IS AN ORACLE, NOT THE FIX RESTATED: it reads `summonedTurn` only to settle what its shadow can
//! bound. The shadow is built from the event stream and the Stack piles alone: `turn`; `entered` and `stint`
//! (an instance's latest entry turn and entry count; entries are plays, summons, control changes, `animated`
//! (R383, R1062), a `transformed` to a new instance and a card a Stack pile uncovers (R1829000, R212);
//! `fused` keeps the target's entry, R77); `lastAttack` (declared, not forced; R1202); `readied` (the
//! Transform results R424 lets attack, Classic+ #73.1 Classic Golem, R42); `kill_credits` (B5 E35
//! `mayAttackAgain`, Stinkyscape #94 Tmack Trav, R1869002); `lastSwitch` (each instance's latest chosen
//! switch, kept only when its `positionSwitched` confirms it: an effect's switch spends nothing, R20).
//! `chosenExertion` snapshots the chosen attacker's or switcher's identity and layered Deft before
//! reduction, since combat may take it off the field before its event.
//!
//! The twenty-three checks (a message leads with its id and cites the SPEC reference, so `signatureOf` groups
//! one bug):
//!   I1 no sick attack is offered (§4.1, §6.1, R83, R171): not in `attack_targets`, not as the chosen
//!      `attack`.
//!   I2 one declared attack per stint per turn, two with Windfury (R636); forced attacks are skipped (R53).
//!   I3 every card on the field arrived by an event (§10.3).
//!   I4 `summonedTurn` and a spent exertion match the shadow's stint (R171, R424, R1839002, R1869002).
//!   I5 nothing happens after the game is over (§2.5, R216).
//!   I6 no seat is sent a card it may not read (§9.1, §10.8, R97, R177, #348): hidden is read off the state's
//!      zones, never from `view_for`'s rules (R763, R764; a Glitch's reset or boards, R676, R678).
//!   I7 no orphaned work (§9.3, §10.3, R113): a live game with no prompt open (§10.6, R265) has `work` empty;
//!      a game over keeps its debt (I5; Classic #52 Final Gambit, #56 Spell Tyrant).
//!   I8 every attack, switch, activation, play, Trade, shop buy and shop sale `reduce` accepts is listed
//!      (§4.1, §4.2, §6.3, §9.1, §10.2, §10.5; R1838000, R1853003; #1192, #2978, #4143), as the server
//!      reduces without `legal_actions`. Not probed: R345, R923000, R79 and a well-formed `answer` (R90,
//!      #992); a malformed one is the fuzz's probe (#1574, #1694). Under a Mayor (R1200) a target is
//!      refused.
//!   I9 the wire tells the truth (§10.3, §10.10, #1719): `turnEnded` names the turn under way (§2.2), each
//!      `fatigue` is the next draw's `FATIGUE_DAMAGE(count)` (§2.4, R3, R315; #1902), the last `manaChanged`
//!      is `mana` (§2.3), and no 1v1 hero's health moves without its `damage`, `healthLost`, `healed`,
//!      `healthSet` or health `swapped` (R2201000).
//!   I10 no opponent-held prompt on its holder's off turn (§10.6, R1279000; #2568): a `pending` whose
//!      `player_id` is not `state.active` and whose `prompt_owner_of` is another player is a violation.
//!   I11 no definition-level Unit token sits in a graveyard, an exile pile or `held` (§3, §3.2, R11), read
//!      by `is_unit_token_def`, never the current face, so setting or transforming one cannot disguise it;
//!      Spell tokens are ordinary pile cards.
//!   I12 a chosen switch and a declared attack share no turn and stint without layered Deft (R49), and no
//!      Unit switches twice there. Only a chosen `switchPosition` its `positionSwitched` confirms counts
//!      (effects spend nothing, R20; forced attacks neither, R53); Windfury's first attack spends the
//!      switch exertion (R636); re-entry advances the stint, and a confirmed `mayAttackAgain` refresh
//!      clears the old switch beside I2's attack-count baseline.
//!   I13 every card a seat may play but `legal_actions` lists no play of says why (§10.8, R979000, R1571000,
//!      R2975000): while it may act, each hand card, and each graveyard card a permission admits, carries
//!      `playReason` exactly when no play of it is listed; read off I6's view and listing in `hidden`, so
//!      sampled with it; a seat barred to the tutorial's cards (R2533000) is not asked.
//!   I14 every action `legal_actions` lists, `reduce` accepts (§10.2; #3062), I8's converse: each seat's
//!      listing is reduced on a copy of the state, at most `I14_ACTION_CAP` of a long one spread over it,
//!      verifying that `reduce` accepts it without refusal or panic, and verifying the resulting state
//!      and events with the stateless invariants (I5, I7, I10, I11, I16 and I17; #4057), and I19, I21 and
//!      I23 against the state it was reduced from (#4454, #4508, #4609); invariants requiring shadow history (I2, I3, I4, I9's
//!      health tally, I12, I20's damage tally) remain on the played path. Run at setup (the mulligan's subsets) and every
//!      `I14_GATE_STRIDE`-th state; not in Duos.
//!   I15 every Activate ability a seat may see is held to the listing (§10.8, R384, R3061000): while it may
//!      act, each shown on a unit or backrow top of either side is `usable` exactly when `legal_actions`
//!      lists an `activate` of it and carries `reason` exactly when not; read off I6's view and listing in
//!      `hidden`, as I13 is; a carried Unit is not asked.
//!   I16 every card's own record agrees with the pile that holds it (§3.2, R78, R766): `zone` names the pile
//!      (a field slot's side, row and lane too), a field card's `controller` is its side, a hand, library,
//!      graveyard or exile card is owned and controlled by its side (R747, R2618000, R73, R12), a card off
//!      the field holds nothing `zones::reset_instance` (R78's reset) would clear, bar the keeps its pile
//!      lawfully allows (`field_state_held`), nor a graveyard or exile card R766's price.
//!   I17 the state check ran (§4.5): on I7's settled state no field unit at layered health 0 or less but
//!      an Indestructible one with max health above 0 (R69) or a Dormant one (R3539003), no unit in Defense
//!      Position while it is Indestructible, has Can't Defend or is Evasive, or while an aura bars Defense
//!      (§4.5 step 1: R2179000, R2657000, R3543002, R1849006), and no seat that `can_lose` (R2201000) at
//!      hero health 0 or less (a Duos seat's is its pool's, §13.4); on every state
//!      mana and card `damage` are 0 or more (§2.3), and every `damage` event deals more than 0 (R63,
//!      R1361, R1362).
//!   I18 every plain attack does what its seat's forecast said (R3100000, R4111000; #4111): `before` keeps
//!      the `combat_forecast` entry of a chosen `attack`, and `after` holds `PlayedAttack::read` of the
//!      step to it, a Cleave attack's splash and a Trample attack's overflow included (R4290000); what it
//!      skips is `foreseeable_attack`'s and `plain_combat`'s (a hook, a trap, a redirect, a damage chain,
//!      a reflection, a prompt or a turn end). 1v1 only.
//!   I19 nothing reaches a Dormant card (§6.1 Dormant, §3.2, R1839000; #4454): `before` notes each Dormant
//!      field card and its record (`SLUMBER_KEPT`); `after` fails an event that changes one and names it
//!      (`reached_ids`) before its `awakened`, `starshipLaunched` or own `activated` (R1839001, R1839002),
//!      and one whose record changed with no event. Lawful: a hull's `fused` then `starshipFused`
//!      (R1852000, R3539004), an exile in a `BOARD_SWEEPS` card's run (R3539002, R3554002, R3567000), a
//!      `controlChanged` or `bounced` after a board `swapped` or a `rotated` (R4454000), a board
//!      restore's after its `rolledBack` (R4454002), and a `radiantSet` under Tears of the Kwzki Tree's
//!      world (R4454001). A card Dormant mid-step is judged from the next; I14's branches and each Duos
//!      duel run it.
//!   I20 stored damage is what its events say (§4.4, §10.3, R78; #4478): each field card's `damage` is 0 on
//!      arrival (`summoned`, `cardPlayed`), plus each `damage` on it, less each `healed`, as I9 holds a hero's
//!      health; re-anchored by the lawful silent writers (`transformed`, `numberChanged` "health", a
//!      `summoned` via Reborn, `flickered`, `rolledBack`, a Glitch's reset or boards); dropped when the card
//!      leaves the field. Played path only (shadow history, so not on I14's copies); each Duos duel's
//!      monitor runs it.
//!   I21 no Spell changes a Unit Immune to Spells on the field (§6.1, R4453000; #4508): `before` notes each
//!      field Unit immune by its layered keywords; `after` fails an event that changes one and names it
//!      (`reached_ids`, I19's) while the card running is a Spell: its face's printed type in the catalog,
//!      from its `hookResolving` (or the prompt its answer continues) to the next marker, trap, AI turn or
//!      turn edge, a play nested in it being its own until its `cardResolved`, and a card set face-down
//!      running as a Trap (M #99's row, R3806000). Lawful: anything inside an attack (R4508002), a
//!      `controlChanged` or `bounced` after a board `swapped`, a `rotated` or a `rolledBack` and a
//!      restore's `radiantSet` (R4508001), an exile in a `LANE_TAKERS` card's run (R4508004) or by the
//!      Black Hole's pull at a turn's end (R4577008), a `radiantSet` under Tears of the Kwzki Tree's world
//!      (R4508003), the state check's switch out of a barred Defense and a Burn or Toxic tick (R4508005),
//!      and the state check's evolution of a Unit whose "Evolves when" holds after the step (R4623000). A Unit is judged only while still on the
//!      field and immune after the step, or gone from it with its own keywords immune. Reads the events
//!      and the catalog, never `restrictions::unaffected_by`; I14's branches and each Duos duel run it.
//!   I22 every `target` pick is one its source may aim at (§10.5 step 1, R450; #4540): no Dormant card,
//!      whatever the source or none (R1839000); no face-up Evasive card in play from a Spell or Field
//!      Spell, the source read as R781001 reads it (R4353002); no face-up card Immune to Spells from a
//!      Spell (R450). Read off the board (zones, `slumber`, face-down state, layered keywords), never
//!      through `restrictions`, `targeting` or the targeting point; a face-down card's Immune to Spells,
//!      which the engine also bars, is not read. `before` opens `seams`' record of the random `target`
//!      draws nobody is asked for (a random cast's, R452; a Mayor's, R1200) and `after` judges each on
//!      the state it was drawn on; `hidden` judges each `target` declaration of a listed play or
//!      activation (none while a Mayor acts, whose plays name none) and each option of an open `target`
//!      prompt. A random pick an effect makes is no targeting and is not recorded (R4353002: "a random
//!      enemy" may land on an Evasive card). Each Duos duel runs it; I14's branches do not.
//!   I23 nothing changes an Immutable card's text (§6.1 Immutable, §6.3, R23, R386, R4353000, R4609000;
//!      #4609): `before` notes each card Immutable by the keyword, or by Indestructible on a card whose
//!      running face prints the Unit type in the catalog, on the field face-up (its layered keywords) or
//!      in a hand or a library (its own, which a Nerf reads there), and its text (`TextRecord`); a Dormant
//!      or face-down card is not read. `after` fails a `transformed` of one (a Vanilla's, a copied or fused
//!      text's), a `degraded` or `upgraded` of it and a `fused` onto it, and a record whose text changed
//!      with no event (a Set stats' `statsOverride` among it, R1185002, which reports only `numberChanged`).
//!      Lawful: a Replace off the field and a `none` cue (R4609001), a Fuse onto a card the
//!      step made (R4353003), a card `flickered` or `summoned` anew and a step with a `rolledBack` or a
//!      Glitch's reset, for its record (R4609002), and a keyword kept for good (R4609003); Radiant is never
//!      read (R23). A card is judged only while Immutable after the step with its starting text put back,
//!      or, gone from the zones it reads, by its own keywords. Reads the events, the state and the catalog,
//!      never `layers::is_immutable` or `restrictions`; I14's branches and each Duos duel run it.

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::{IndexMap, IndexSet};
use serde_json::{Map, Value};

use crate::combat::{HERO_TARGET_PREFIX, hero_seat_of};
use crate::config::{
    BACKROW_ZONES, FATIGUE_DAMAGE, MIN_CHOSEN_X, MIN_PLAGUE_PAYMENT, UNIT_ZONES, WINDFURY_ATTACKS,
};
use crate::damage::DamageTarget;
use crate::duos::{DuosState, duos_view_for};
use crate::layers::AuraTable;
use crate::play_choices::PlayAction;
use crate::prompts::prompt_owner_of;
use crate::reduce::ReduceResult;
use crate::state::{CardInstance, GameState, PendingChoice, find_instance};
use crate::subsystems::activate::ActivateAction;
use crate::testkit::played_attack::PlayedAttack;
use crate::testkit::seams::{TargetDraw, TargetDrawBy};
use crate::wire::{
    Action, ActionBody, ActivationView, AttackHealth, BackrowView, CardType, CardView, DUOS_DUELS,
    DUOS_SEATS, DuosActionBody, DuosEvent, DuosSeat, DuosView, Enchantment, GameEvent, GameResult, HandView,
    Keyword, KeywordKind, PLAYER_IDS, PerPlayer, Phase, PlagueSpend, PlayerId, PlayerView, Position,
    PromptKind, RedirectWhat, RevealAt, Row, Selection, SwapWhat, Team, Tuning, TuningChange, Zone,
    ZoneChoice, ZoneName, has_keyword, opponent_of,
};

/// I6 under the 1,000-seed gate: every this-many-th state, setup's and the last always (the monitor
/// budget of docs/polish/4-edge-cases.md: within 25% of the time without it). The 100-seed
/// waves check every state.
pub const I6_GATE_STRIDE: usize = 5;

/// I8 under the 1,000-seed waves: every this-many-th state (#1192); the test sweep checks every state.
/// Measured on 2026-10-18 (2 cores), seeds 1–1000: `cargo jackioh fuzz` 103 s before I8, 104 s with
/// this stride (`--handicap` 130 s, 129 s); I8 on every state, 314 s and 417 s.
/// Measured on 2026-11-18 (4 cores), seeds 1–200, after #4143's Trade and Marketplace forges:
/// `cargo jackioh fuzz` 50 s before, 51 s after, at this stride.
pub const I8_GATE_STRIDE: usize = 100;

/// I14 under every wave and the test sweep: every this-many-th state, and setup's always (#3062).
/// Measured on 2026-11-05 (4 cores), seeds 1–200: `cargo jackioh fuzz` 39.0 s before I14, 44.0 s with
/// this stride and `I14_ACTION_CAP`; I14 on every state with no cap, 152 s (and no listed action refused).
pub const I14_GATE_STRIDE: usize = 20;

/// The most listed actions I14 reduces for one seat on one state: a mulligan lists up to
/// `MAX_MULLIGAN_SUBSETS`, a play up to `MAX_CHOICE_COMBINATIONS`.
pub const I14_ACTION_CAP: usize = 32;

/// The nonce of every action I14 reduces (the copy holds no history, as I8's).
pub const I14_NONCE: &str = "i14-probe";

/// The nonce of every action I8 reduces. The copy it reduces on holds no action history, so no
/// nonce could be taken for a replay (§9.3); this one names the probe in a debugger.
const PROBE_NONCE: &str = "i8-probe";

/// What one seat may not read: instance ids, and the definitions that name a card only it hides.
struct HiddenSet {
    /// Instance id -> where the card sits, for the message.
    ids: IndexMap<String, String>,
    /// Definition id -> where a hidden card of it sits. A definition any readable card carries is not here.
    defs: IndexMap<String, String>,
    /// Face-down traps' ids: R177 lets `legal_actions` offer one as a bare target.
    bare: IndexSet<String>,
    /// The bare ids the viewer's own prompt offers: R177 lets each be a bare option's `instanceId` only.
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

/// One mulligan return as the owed work item carries it, read defensively off the JSON.
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

/// Every card the state holds, in every pile (`find_instance`'s piles) and beneath a holder (R1850003).
fn cards_of(state: &GameState) -> Vec<&CardInstance> {
    let mut out: Vec<&CardInstance> = crate::state::every_instance(state).collect();
    // R1850003: a card held beneath another, face-up when it was taken (`effects::hold`).
    out.extend(state.held.iter().flatten().map(|entry| &entry.card));
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
    /// R316: an id a full library refused before it existed (`libraryOverflow`, `notCreated`) that copies
    /// nothing. It was never in any pile, so it reads openly to both seats.
    never_made: IndexSet<String>,
    /// Each id, and the definitions the engine's own events pair with it.
    ties: IndexMap<String, IndexSet<String>>,
    /// R11, R97: the definition each id last left a zone as (`LEAVING_EVENTS`). A definition can change
    /// and change back (R77's Fuse, then R419's Rollback restoring the token it was, fuzz seed 119), so
    /// its leaving names it better than every definition it ever wore.
    gone_as: IndexMap<String, String>,
    /// R1074000: in a duel, each id whose last place the log shows is a hand, and that hand's player. A
    /// pass (B20) takes a hand card out of its duel with no event, so one in no pile was passed.
    in_hand: IndexMap<String, PlayerId>,
    /// R1853003, R1861000: each id a `marketSold` names. A sale is public and the card ceases to exist
    /// as it goes, so it read to both seats then, a hand card's as much as a tray Resource's.
    sold: IndexSet<String>,
}

/// The card an event's id field names. R127: a Cry run out of a graveyard runs with no card, and the
/// `chaosRolled` it sends names none by an empty `instanceId` (R436, #1645), which is no card's id.
fn card_id(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).filter(|id| !id.is_empty())
}

fn lineage_of(state: &GameState, viewer: PlayerId) -> Lineage {
    let mut next: IndexMap<String, String> = IndexMap::new();
    let mut unread: IndexSet<String> = IndexSet::new();
    let mut never_made: IndexSet<String> = IndexSet::new();
    let mut ties: IndexMap<String, IndexSet<String>> = IndexMap::new();
    let mut gone_as: IndexMap<String, String> = IndexMap::new();
    let mut in_hand: IndexMap<String, PlayerId> = IndexMap::new();
    let mut ceased: IndexSet<String> = IndexSet::new();
    let mut sold: IndexSet<String> = IndexSet::new();
    let tie =
        |ties: &mut IndexMap<String, IndexSet<String>>, id: Option<&Value>, values: &[Option<&Value>]| {
            let Some(id) = card_id(id) else {
                return;
            };
            let tied = ties.entry(id.to_string()).or_default();
            for def in strings_of(values) {
                tied.insert(def);
            }
        };
    for event in state.applied.iter().flat_map(|entry| entry.events.iter()) {
        let value = serde_json::to_value(event).expect("a GameEvent serializes");
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
            // R2783000: a hook's marker names its card by `sourceId`, as the card stood when the hook began.
            if object.get("type").and_then(Value::as_str) == Some("hookResolving") && !unread_here {
                tie(&mut ties, object.get("sourceId"), &[object.get("defId")]);
            }
            let Some(id) = card_id(id) else {
                return;
            };
            if let (Some(kind), Some(def)) = (
                object.get("type").and_then(Value::as_str),
                object.get("defId").and_then(Value::as_str),
            ) && LEAVING_EVENTS.contains(&kind)
            {
                gone_as.insert(id.to_string(), def.to_string());
            }
            let former = object.get("formerId").and_then(Value::as_str);
            // R419: Classic+ #35 Rollback recreates a card transformed away under a fresh id whose
            // `formerId` is the old one, so the card exists again and R177's mark is spent: its old id
            // goes with it, a trap's that fired into exile included (fuzz seed 992: one #83 had transformed).
            if let Some(former) = former {
                next.insert(former.to_string(), id.to_string());
                unread.shift_remove(former);
            }
            if let Some(copy_of) = object.get("copyOf").and_then(Value::as_str) {
                next.insert(id.to_string(), copy_of.to_string());
            } else if object.get("type").and_then(Value::as_str) == Some("libraryOverflow")
                && object.get("outcome").and_then(Value::as_str) == Some("notCreated")
            {
                never_made.insert(id.to_string());
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
        match event {
            // R11: a unit token bounced or stolen off the field or out of the resolving zone ceased to exist
            // on its way to the hand, so a counter's steal's `addedToHand` after it puts it in none.
            GameEvent::Bounced {
                instance_id, def_id, ..
            }
            | GameEvent::Stolen {
                instance_id,
                def_id,
                zone: ZoneName::Field | ZoneName::Resolving,
                ..
            } if is_unit_token_def(state, def_id) => {
                in_hand.shift_remove(instance_id);
                ceased.insert(instance_id.clone());
            }
            GameEvent::Drawn {
                player, instance_id, ..
            }
            | GameEvent::AddedToHand {
                player, instance_id, ..
            }
            | GameEvent::Bounced {
                owner: player,
                instance_id,
                ..
            }
            | GameEvent::Stolen {
                to: player,
                instance_id,
                ..
            } => {
                if !ceased.contains(instance_id) {
                    in_hand.insert(instance_id.clone(), *player);
                }
            }
            // R227: a card set face-down from a hand left it under its old id, the `formerId`.
            GameEvent::CardPlayed {
                instance_id,
                former_id,
                ..
            }
            | GameEvent::Summoned {
                instance_id,
                former_id,
                ..
            }
            | GameEvent::ControlChanged {
                instance_id,
                former_id,
                ..
            } => {
                in_hand.shift_remove(instance_id);
                if let Some(former) = former_id {
                    in_hand.shift_remove(former);
                }
            }
            GameEvent::CardAnnounced { instance_id, .. }
            | GameEvent::Discarded { instance_id, .. }
            | GameEvent::Burned { instance_id, .. }
            | GameEvent::ShuffledIn { instance_id, .. }
            | GameEvent::Exiled { instance_id, .. }
            | GameEvent::EnteredGraveyard { instance_id, .. } => {
                in_hand.shift_remove(instance_id);
            }
            GameEvent::MarketSold { instance_id, .. } => {
                in_hand.shift_remove(instance_id);
                sold.insert(instance_id.clone());
            }
            GameEvent::Transformed {
                instance_id,
                new_instance_id,
                ..
            } => {
                if let Some(holder) = in_hand.shift_remove(instance_id) {
                    in_hand.insert(new_instance_id.clone(), holder);
                }
            }
            GameEvent::Fused {
                instance_ids,
                result_instance_id,
                ..
            } => {
                let mut holder = None;
                for id in instance_ids {
                    if let Some(h) = in_hand.shift_remove(id) {
                        holder = Some(h);
                    }
                }
                if let Some(holder) = holder {
                    in_hand.insert(result_instance_id.clone(), holder);
                }
            }
            _ => {}
        }
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
    // Only a Duos seat passes (B20): a 1v1 hand card never leaves by a pass.
    in_hand.retain(|id, holder| state.players[*holder].duos.is_some() && find_instance(state, id).is_none());
    Lineage {
        next,
        unread,
        never_made,
        ties,
        gone_as,
        in_hand,
        sold,
    }
}

/// R11: whether a definition is a unit token's, which ceases to exist when it leaves the field.
fn is_unit_token_def(state: &GameState, def_id: &str) -> bool {
    crate::catalog::find_def(Some(state), def_id).is_some_and(|def| def.token && def.type_ == CardType::Unit)
}

/// I11: definition-level Unit tokens cease to exist when they leave the field (§3, §3.2, R11),
/// so neither player's public piles nor the cards waiting in `held` may contain one. A face or layer
/// cannot disguise the token because `is_unit_token_def` reads its definition.
fn unit_token_pile_findings(state: &GameState) -> Vec<String> {
    let mut found = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        for (location, cards) in [
            ("graveyard", side.graveyard.as_slice()),
            ("exile", side.exile.as_slice()),
        ] {
            for card in cards {
                if is_unit_token_def(state, &card.def_id) {
                    found.push(format!(
                        "I11 unit token in pile: {} ({}) is in {player}'s {location} on turn {} \
                         (§3, §3.2, R11)",
                        card.id, card.def_id, state.turn
                    ));
                }
            }
        }
    }
    for held in state.held.iter().flatten() {
        let card = &held.card;
        if is_unit_token_def(state, &card.def_id) {
            found.push(format!(
                "I11 unit token in pile: {} ({}) is in held beneath {} on turn {} (§3, §3.2, R11)",
                card.id, card.def_id, held.holder_id, state.turn
            ));
        }
    }
    found
}

/// A card in a pile or a field slot (I16): the zone its own record must hold, and what holds it.
struct Placed<'a> {
    card: &'a CardInstance,
    zone: Zone,
    holder: &'static str,
}

impl Placed<'_> {
    /// Where it sits, for a finding: "p1's graveyard", "p2's unit zone 3".
    fn slot(&self) -> String {
        match self.zone {
            Zone::Field { player, lane, .. } => format!("{player}'s {} {lane}", self.holder),
            _ => format!("{}'s {}", self.zone.player(), self.holder),
        }
    }
}

/// Every card in a pile or a field slot, each with the zone it must record. A card `held` beneath another
/// is in no pile (`Zone::Gone`, R1850003) and is not walked.
fn placed_cards(state: &GameState) -> Vec<Placed<'_>> {
    let mut out = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        for (zone, holder, pile) in [
            (Zone::Hand { player }, "hand", &side.hand),
            (Zone::Library { player }, "library", &side.library),
            (Zone::Graveyard { player }, "graveyard", &side.graveyard),
            (Zone::Exile { player }, "exile", &side.exile),
            (Zone::Resolving { player }, "resolving", &side.resolving),
        ] {
            out.extend(pile.iter().map(|card| Placed {
                card,
                zone: zone.clone(),
                holder,
            }));
        }
        // Lane N is index N - 1 in each of the four field slots (§3.2).
        let on_field = |card, row, index: usize, holder| Placed {
            card,
            zone: Zone::Field {
                player,
                row,
                lane: index as i32 + 1,
            },
            holder,
        };
        for (index, pile) in side.units.iter().enumerate() {
            out.extend(
                pile.iter()
                    .flatten()
                    .map(|card| on_field(card, Row::Units, index, "unit zone")),
            );
        }
        for (index, top) in side.backrow.iter().enumerate() {
            out.extend(
                top.iter()
                    .map(|card| on_field(card, Row::Backrow, index, "backrow zone")),
            );
        }
        for (index, beneath) in side.backrow_piles.iter().flatten().enumerate() {
            out.extend(
                beneath
                    .iter()
                    .map(|card| on_field(card, Row::Backrow, index, "backrow pile")),
            );
        }
        for (index, carried) in side.carried.iter().flatten().enumerate() {
            out.extend(
                carried
                    .iter()
                    .map(|card| on_field(card, Row::Backrow, index, "carried unit")),
            );
        }
    }
    out
}

/// A zone as a finding spells it.
fn zone_text(zone: &Zone) -> String {
    match zone {
        Zone::Field { player, row, lane } => format!("{player}'s {row} {lane}"),
        other => format!("{}'s {}", other.player(), other.z()),
    }
}

/// The names among `checks` whose test holds.
fn held_names(checks: &[(bool, &'static str)]) -> Vec<&'static str> {
    checks
        .iter()
        .filter(|(held, _)| *held)
        .map(|(_, name)| *name)
        .collect()
}

/// What R78's reset changes and a card off the field may hold in every pile (I16), by field name:
/// `controller` (I16's owner check holds it to the pile's side on its own), `memory` (R1841003's
/// `tributeLowered` rides a hand or a deck; a graveyard card keeps R429's `@returnPrice`, R766, and R178's
/// `@exileOnLanding`) and `brittle` (R385, R441: the count rides every zone; the reset drops a spent one).
const KEPT_IN_EVERY_PILE: [&str; 3] = ["controller", "memory", "brittle"];

/// What a hand or a library card may hold and a graveyard or an exile card may not: a Buff's and a grant's
/// state, which rides it onto the field (`effects/buff.rs`, B5 E38) and which R215 takes as it reaches a
/// graveyard or an exile pile (`buffs`, `grantedKeywords`, `grants`, `vanilla`, `counters`); a copy's
/// `statsOverride` and `armorOverride` (R57, R1242, R2198001; R175 reads them before R78's reset takes
/// them); and the `tipsy` stamp (R3548001) a card is given as it is drawn (`draw.rs`), which only a hand
/// or the field reads (`tipsy::blur_for`) and a hand card carries into a library.
const KEPT_IN_HAND_OR_LIBRARY: [&str; 8] = [
    "buffs",
    "grantedKeywords",
    "grants",
    "vanilla",
    "counters",
    "statsOverride",
    "armorOverride",
    "tipsy",
];

/// What a graveyard or an exile card that evolved there may hold: R3566004's Stinky Cheese keeps what its
/// Evolve gave it, for the Flashback play that brings it back (S+ #6.2). Leaving the field clears `evolved`
/// (R3536000), so a card that holds it off the field evolved there.
const KEPT_BY_AN_EVOLVE: [&str; 4] = ["evolved", "buffs", "grantedKeywords", "statsOverride"];

/// I19 (§6.1 Dormant, R1839000): what a Dormant card keeps while it sleeps, by field name: whatever an
/// effect would change by reaching it. Left out on purpose: `slumber`, the countdown, which its
/// controller's turns count down (R1839001); `summonedTurn` and `exertion`, the entry stamp a lawful
/// carry gives it (R171); `memory` and `starship`, the engine's bookkeeping (a hull's pieces, R1852000);
/// `zone`, which a board move along its own side changes with no event (R14); and `brittle`, its own
/// text's count, which waits while it sleeps (R4454004) and whose tick `counterChanged` reports.
const SLUMBER_KEPT: [&str; 20] = [
    "defId",
    "radiant",
    "controller",
    "position",
    "damage",
    "buffs",
    "grantedKeywords",
    "vanilla",
    "costMod",
    "costOverride",
    "counters",
    "statsOverride",
    "armorOverride",
    "markedDestroyed",
    "markedExiled",
    "tuning",
    "enchantments",
    "frozen",
    "grants",
    "grantedTags",
];

/// I19: the cards whose run is a board sweep, each with the ruling that lets it exile a Dormant card:
/// a sweep is no effect reaching for a card (R3539002, R3554002). Its `exiled` looks the same as The
/// Maestro's random pick did before #4322 (R4322000), so the card running is the only signature; a new
/// sweep joins the table with its ruling. A fusion holding one sweeps too (R43, R102). A kept version's
/// id (`<id>@v<version>`, R767000) sweeps only where its own script did.
const BOARD_SWEEPS: [(&str, &str); 5] = [
    // S+ #4 Profe Yates through v0.6.5b, a past version a fuzz game still plays: every card named with
    // M, O, U, S or E, Dormant cards included (`exile_named`).
    ("stinkyscapeplus-004@v0.6.5b", "R3567000"),
    // S+ #78 Slashed Dream Cries in Red: the slashed lane, Dormant cards included.
    ("stinkyscapeplus-078", "R3554002"),
    // S+ #79 A Glimpse into the Endless Sea: every permanent (R4154200's `exile_board`).
    ("stinkyscapeplus-079", "R3539002"),
    // S+ #80 The Last Voyage: its caster's board, the Radiant face's both.
    ("stinkyscapeplus-080", "R3539002"),
    // S+ #81 Doomsayer: its controller's board at the start of their turn.
    ("stinkyscapeplus-081", "R3539002"),
];

/// A card's fields as JSON, by name: each optional field is absent when unset.
fn card_fields(card: &CardInstance) -> Map<String, Value> {
    match serde_json::to_value(card).expect("a CardInstance serializes") {
        Value::Object(fields) => fields,
        other => panic!("a CardInstance serializes as an object, not {other}"),
    }
}

/// R78's reset of a card leaving the field, read on a card off it in `zone` (I16): the field-only state it
/// still holds, by field name. The fields are `zones::reset_instance`'s own, not a list kept here: the card
/// is cloned, reset and both serialized, and every field that differs is held, but for what `zone` lawfully
/// keeps (the constants above). A field a later set gives the reset is checked from the day it lands; one a
/// card may lawfully keep in a pile is named above, with its ruling, once a wave reports it.
fn field_state_held(card: &CardInstance, zone: &Zone) -> Vec<String> {
    let mut reset = card.clone();
    crate::zones::reset_instance(&mut reset);
    // Nearly every card off the field holds nothing the reset takes: no serializing for those.
    if reset == *card {
        return Vec::new();
    }
    let (held, clean) = (card_fields(card), card_fields(&reset));
    let in_hand_or_library = matches!(zone, Zone::Hand { .. } | Zone::Library { .. });
    let evolved_here =
        matches!(zone, Zone::Graveyard { .. } | Zone::Exile { .. }) && card.evolved == Some(true);
    let lawful = |field: &str| {
        KEPT_IN_EVERY_PILE.contains(&field)
            || (in_hand_or_library && KEPT_IN_HAND_OR_LIBRARY.contains(&field))
            || (evolved_here && KEPT_BY_AN_EVOLVE.contains(&field))
    };
    held.keys()
        .chain(clean.keys().filter(|field| !held.contains_key(*field)))
        .filter(|field| held.get(*field) != clean.get(*field) && !lawful(field))
        .cloned()
        .collect()
}

/// R766's price on a card in a graveyard or an exile pile (I16), by field name. `@returnPrice`, the climb
/// R766 keeps for #31's return, lives in `memory`, which no check here reads.
fn price_held(card: &CardInstance) -> Vec<&'static str> {
    held_names(&[
        (card.cost_mod != 0, "costMod"),
        (card.cost_override.is_some(), "costOverride"),
        (card.embiggen_cost_override.is_some(), "embiggenCostOverride"),
        (card.no_embiggen_premium.is_some(), "noEmbiggenPremium"),
        (card.fixed_x.is_some(), "fixedX"),
    ])
}

/// R636, R3556008: whether an aura gives Windfury: a unit gone by an action's end may have attacked
/// under one that woke inside it (S+ #63 Overdrive Dome, fuzz seeds 1596, 2217), and so may an animated
/// card back in its backrow by then (R383, M #66 Fiery Waraxe, fuzz seed 1013, #4079).
fn aura_gives_windfury(state: &GameState) -> bool {
    crate::layers::aura_entries(state).iter().any(|entry| {
        entry
            .mod_
            .keywords
            .as_deref()
            .is_some_and(|keywords| has_keyword(keywords, KeywordKind::Windfury))
    })
}

/// I16: each card's own `zone`, `owner` and `controller` agree with the pile or slot that holds it, and a
/// card off the field holds none of R78's field-only state or, in a graveyard or an exile pile, R766's
/// price (§3.2). `find_instance` reads the piles and never `zone`, so the two can drift unseen.
fn card_record_findings(state: &GameState) -> Vec<String> {
    let mut found = Vec::new();
    for placed in placed_cards(state) {
        let Placed { card, zone, .. } = &placed;
        let (named, slot, turn) = (
            format!("{} ({})", card.id, card.def_id),
            placed.slot(),
            state.turn,
        );
        if card.zone != *zone {
            found.push(format!(
                "I16 zone drift: {named} says {} but sits in {slot} on turn {turn} (§3.2)",
                zone_text(&card.zone)
            ));
        }
        match zone {
            // A card mid-play holds what its play set (`x`, a Buff) and may be a stolen Spell (R12): only
            // its zone is read.
            Zone::Resolving { .. } => {}
            Zone::Field { player, .. } => {
                if card.controller != *player {
                    found.push(format!(
                        "I16 controller drift: {named} is controlled by {} but sits in {slot} on turn {turn} \
                         (§3.2)",
                        card.controller
                    ));
                }
            }
            _ => {
                let side = zone.player();
                if card.owner != side || card.controller != side {
                    found.push(format!(
                        "I16 owner drift: {named} is owned by {} and controlled by {} but sits in {slot} on \
                         turn {turn} (§3.2, R747, R2618000)",
                        card.owner, card.controller
                    ));
                }
                let gone = matches!(zone, Zone::Graveyard { .. } | Zone::Exile { .. });
                let state_held = field_state_held(card, zone);
                if !state_held.is_empty() {
                    found.push(format!(
                        "I16 field state off the field: {named} in {slot} still holds {} on turn {turn} (R78)",
                        state_held.join(", ")
                    ));
                }
                let price = if gone { price_held(card) } else { Vec::new() };
                if !price.is_empty() {
                    found.push(format!(
                        "I16 price kept in a pile: {named} in {slot} still holds {} on turn {turn} (R766)",
                        price.join(", ")
                    ));
                }
            }
        }
    }
    found
}

/// I17: why §4.5 step 1 (`state_check::leave_defense`) would switch this unit out of Defense, or None.
fn forbidden_defense(auras: &AuraTable<'_>, unit: &CardInstance, barred: bool) -> Option<&'static str> {
    if auras.unit_has(unit, KeywordKind::Indestructible) {
        Some("it is Indestructible (R2179000)")
    } else if auras.unit_has(unit, KeywordKind::CantDefend) {
        Some("it has Can't Defend (R2657000)")
    } else if auras.unit_has(unit, KeywordKind::Evasive) {
        Some("it is Evasive (R3543002)")
    } else if barred {
        Some("an aura bars Defense Position (R1849006)")
    } else {
        None
    }
}

/// I17 (the header's): what §4.5's state check and §4.4's pipeline leave (§2.3, R63, R69, R2201000,
/// R3539003, R2179000, R2657000, R3543002, R1849006). A Duos seat's hero is its team's pool's (§13.4),
/// bounded by `DuosInvariantMonitor::after`: B15's `sync_pools` writes the pool into a spent hero in the
/// other duel, where no state check runs.
fn state_check_findings(events: &[GameEvent], state: &GameState) -> Vec<String> {
    let mut found = Vec::new();
    let turn = state.turn;
    for event in events {
        if let GameEvent::Damage {
            target_id, amount, ..
        } = event
            && *amount <= 0
        {
            found.push(format!(
                "I17 empty damage: a damage event of {amount} on {target_id} on turn {turn}; a hit of 0 \
                 emits none (§4.4, R63)"
            ));
        }
    }
    for player in PLAYER_IDS {
        let mana = state.players[player].mana;
        if mana.current < 0 || mana.max < 0 {
            found.push(format!(
                "I17 negative mana: {player} holds {} of {} mana on turn {turn} (§2.3)",
                mana.current, mana.max
            ));
        }
    }
    for placed in placed_cards(state) {
        let card = placed.card;
        if card.damage < 0 {
            found.push(format!(
                "I17 negative damage: {} ({}) in {} holds damage {} on turn {turn} (§4.4)",
                card.id,
                card.def_id,
                placed.slot(),
                card.damage
            ));
        }
    }
    if state.result.is_some() || state.pending.is_some() || state.mulligan.is_some() {
        return found;
    }
    let auras = AuraTable::new(state);
    let barred = crate::restrictions::defense_barred(state);
    for player in PLAYER_IDS {
        for unit in crate::zones::units_on_field_of(state, player) {
            // §4.5 step 1's switch out of Defense (`leave_defense`) passes a Dormant card by (R4454005).
            if unit.position == Some(Position::Def)
                && !crate::slumber::is_slumbering(unit)
                && let Some(rule) = forbidden_defense(&auras, unit, barred)
            {
                found.push(format!(
                    "I17 forbidden Defense: {} stands in Defense Position on turn {turn} with no prompt open, \
                     but {rule} (§4.1, §4.5 step 1)",
                    name_of(unit)
                ));
            }
            // R3539003: no health judges a Dormant card, so a 0/0 hull stands.
            if crate::slumber::is_slumbering(unit) {
                continue;
            }
            let view = auras.unit_view(unit);
            let stands = has_keyword(&view.keywords, KeywordKind::Indestructible) && view.max_health > 0;
            if view.health <= 0 && !stands {
                found.push(format!(
                    "I17 dead unit left standing: {} is at {} health of {} on turn {turn} with no prompt \
                     open (§4.5, R69)",
                    name_of(unit),
                    view.health,
                    view.max_health
                ));
            }
        }
        let side = &state.players[player];
        if side.duos.is_none() && crate::state::can_lose(state, player) && side.hero.health <= 0 {
            found.push(format!(
                "I17 live hero at 0: {player}'s hero has {} health on turn {turn} and the game goes on \
                 (§2.5, §4.5, R2201000)",
                side.hero.health
            ));
        }
    }
    found
}

/// I5: the check that ends the game is the last thing that happens (§2.5, R216).
fn game_over_findings(events: &[GameEvent], state: &GameState) -> Vec<String> {
    let mut found = Vec::new();
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
            state.turn
        ));
    }
    found
}

/// I7: a live game with no prompt open owes no paused sequence (§9.3, §10.3, R113).
fn orphaned_work_findings(state: &GameState) -> Vec<String> {
    let mut found = Vec::new();
    if state.result.is_none()
        && state.pending.is_none()
        && state.mulligan.is_none()
        && (!state.work.is_empty() || state.work_cursor != 0)
    {
        let owed: Vec<String> = state
            .work
            .iter()
            .map(|item| {
                format!(
                    "{} ({}, {}, {})",
                    item.id, item.resume.def_id, item.resume.hook, item.resume.step
                )
            })
            .collect();
        found.push(format!(
            "I7 orphaned work: {} item(s) owed [{}] with work cursor {} on turn {} and no prompt open \
             (§9.3, §10.3, R113)",
            state.work.len(),
            owed.join(", "),
            state.work_cursor,
            state.turn
        ));
    }
    found
}

/// I10: no prompt held by the opponent while its holder is not the active player (§10.6, R1279000).
fn opponent_prompt_findings(state: &GameState) -> Vec<String> {
    let mut found = Vec::new();
    if state.result.is_none()
        && let Some(pending) = &state.pending
        && pending.player_id != state.active
        && prompt_owner_of(pending) != pending.player_id
    {
        let asked_by = pending.resume.instance_id.as_deref();
        let asker = match asked_by.and_then(|id| find_instance(state, id)) {
            Some(card) => name_of(card),
            None => format!(
                "{} ({})",
                asked_by.unwrap_or("no instance"),
                pending.resume.def_id
            ),
        };
        found.push(format!(
            "I10 opponent prompt off turn: {asker} opened prompt {} for {}, held for {}, on turn {} while {} \
             is the active player (§10.6, R1279000)",
            pending.id,
            pending.player_id,
            prompt_owner_of(pending),
            state.turn,
            state.active
        ));
    }
    found
}

/// The stateless invariants gathered for one post-action state (§10.2, #4057): I5, I7, I10, I11, I16
/// and I17. Each reads only the state and the action's events, so it runs on both the played path and
/// every branch action I14 reduces.
pub fn branch_state_findings(events: &[GameEvent], state: &GameState) -> Vec<String> {
    let mut found = Vec::new();
    found.extend(game_over_findings(events, state));
    found.extend(orphaned_work_findings(state));
    found.extend(opponent_prompt_findings(state));
    found.extend(unit_token_pile_findings(state));
    found.extend(card_record_findings(state));
    found.extend(state_check_findings(events, state));
    found
}

/// I19: a field card Dormant as a step begins (§6.1 Dormant, R1839000): its name then, for the message,
/// and what it keeps while it sleeps (`SLUMBER_KEPT`).
#[derive(Clone, Debug)]
struct SlumberMark {
    label: String,
    /// The side whose field holds it as the step begins.
    side: PlayerId,
    kept: Map<String, Value>,
}

/// I19: a board move the step is inside, from its event to the next `hookResolving`, trap or turn edge:
/// a board swap or a rotation (`Swapped` of the board, `Rotated`; R73, R14), or a board restore
/// (`RolledBack`: C+ #35 Rollback, S+ #84 Bogos Binted; R419, R3550007).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoardMove {
    Carry,
    Restore,
}

/// I19: the Dormant field cards a step begins with, by instance id, and the run an answer step
/// continues (`state.pending`'s resume: its card's definition and instance), which marks no
/// `hookResolving` of its own (R2783000).
#[derive(Clone, Debug)]
struct SlumberStep {
    marks: IndexMap<String, SlumberMark>,
    pending_def: Option<String>,
    pending_instance: Option<String>,
    /// R4524002: the field cards that run a copied board sweep as the step begins, a copied Spell's
    /// (`copies_a_sweep`) or a copied Unit's (Stinkyscape #20 Tnack Trav, R1850000): read before the
    /// step, since a sweep of its own side exiles the copier with its copy (R78).
    sweepers: IndexSet<String>,
}

/// I19: a Dormant card's `SLUMBER_KEPT` fields.
fn kept_record(card: &CardInstance) -> Map<String, Value> {
    let mut fields = card_fields(card);
    fields.retain(|field, _| SLUMBER_KEPT.contains(&field.as_str()));
    fields
}

/// I19: the state a step begins on, as I19 reads it: every Dormant card in a field slot, a pile's, a
/// backrow pile's and a carried one's included (R1839003, R446). A card `held` beneath another is in no
/// slot (R1850003).
fn slumber_step(state: &GameState) -> SlumberStep {
    let marks = placed_cards(state)
        .into_iter()
        .filter(|placed| {
            matches!(placed.zone, Zone::Field { .. }) && crate::slumber::is_slumbering(placed.card)
        })
        .map(|placed| {
            let mark = SlumberMark {
                label: name_of(placed.card),
                side: placed.zone.player(),
                kept: kept_record(placed.card),
            };
            (placed.card.id.clone(), mark)
        })
        .collect();
    let sweepers = placed_cards(state)
        .into_iter()
        .filter(|placed| matches!(placed.zone, Zone::Field { .. }))
        .filter(|placed| runs_a_copied_sweep(state, placed.card))
        .map(|placed| placed.card.id.clone())
        .collect();
    let resume = state.pending.as_ref().map(|pending| &pending.resume);
    SlumberStep {
        marks,
        pending_def: resume.map(|resume| resume.def_id.clone()),
        pending_instance: resume.and_then(|resume| resume.instance_id.clone()),
        sweepers,
    }
}

/// I19: whether `def_id`'s run is a board sweep (`BOARD_SWEEPS`): a card the table names, or a fusion
/// holding one, which carries its text (R43, R102).
fn is_board_sweep(state: &GameState, def_id: &str) -> bool {
    BOARD_SWEEPS.iter().any(|(sweep, _)| *sweep == def_id)
        || crate::subsystems::fuse::fused_ingredients(state, def_id)
            .is_some_and(|parts| parts.iter().any(|part| is_board_sweep(state, part)))
}

/// I19 (R4524002): whether `card` runs a copied board sweep's text, a copied Unit's (Stinkyscape #20
/// Tnack Trav, R1850000) or a copied Spell's (`copies_a_sweep`), so that a hook it resolves is a sweep.
pub fn runs_a_copied_sweep(state: &GameState, card: &CardInstance) -> bool {
    crate::subsystems::unit_text::unit_text_of(card).is_some_and(|copy| is_board_sweep(state, &copy.def_id))
        || copies_a_sweep(state, &card.id)
}

/// I19: whether the card `source_id` runs a copied Spell's text that is a board sweep (C #57 Echo
/// copying S+ #80 The Last Voyage, R399, R546): the copy its play fixed, kept in its memory in every pile.
fn copies_a_sweep(state: &GameState, source_id: &str) -> bool {
    find_instance(state, source_id)
        .and_then(|card| crate::subsystems::copied_text::copied_text_of(state, card))
        .is_some_and(|copy| is_board_sweep(state, &copy.def_id))
}

/// I19: the cards an event changes and names (§10.3): the floor of what reaches a card, each id once.
fn reached_ids(event: &GameEvent) -> Vec<&str> {
    let mut ids: Vec<&str> = match event {
        GameEvent::Damage { target_id, .. }
        | GameEvent::DamageAbsorbed { target_id, .. }
        | GameEvent::Healed { target_id, .. } => vec![target_id],
        GameEvent::AttackDeclared {
            attacker_id,
            target_id,
            ..
        } => vec![attacker_id, target_id],
        GameEvent::DivineShieldLost { instance_id }
        | GameEvent::Buffed { instance_id, .. }
        | GameEvent::KeywordGranted { instance_id, .. }
        | GameEvent::CounterChanged { instance_id, .. }
        | GameEvent::CostChanged { instance_id, .. }
        | GameEvent::RadiantSet { instance_id, .. }
        | GameEvent::Deradianted { instance_id, .. }
        | GameEvent::Degraded { instance_id, .. }
        | GameEvent::Upgraded { instance_id, .. }
        | GameEvent::NumberChanged { instance_id, .. }
        | GameEvent::Evolved { instance_id, .. }
        | GameEvent::Transformed { instance_id, .. }
        | GameEvent::PositionSwitched { instance_id, .. }
        | GameEvent::Stolen { instance_id, .. }
        | GameEvent::Bounced { instance_id, .. }
        | GameEvent::Destroyed { instance_id, .. }
        | GameEvent::Crumbled { instance_id, .. }
        | GameEvent::Exiled { instance_id, .. }
        | GameEvent::Held { instance_id, .. }
        | GameEvent::Flickered { instance_id, .. }
        | GameEvent::Animated { instance_id, .. }
        | GameEvent::Deanimated { instance_id, .. }
        | GameEvent::Translated { instance_id } => vec![instance_id],
        // R437: a mark added is something aimed at the card; one taken off says the effect that waited
        // has gone (its own run is judged by its events), and a thaw's change shows in the record.
        GameEvent::Marked {
            instance_id,
            added: true,
            ..
        } => vec![instance_id],
        GameEvent::ControlChanged {
            instance_id,
            former_id,
            ..
        } => std::iter::once(instance_id.as_str())
            .chain(former_id.as_deref())
            .collect(),
        GameEvent::Fused {
            instance_ids,
            result_instance_id,
            ..
        } => instance_ids
            .iter()
            .chain(std::iter::once(result_instance_id))
            .map(String::as_str)
            .collect(),
        _ => Vec::new(),
    };
    let mut seen: IndexSet<&str> = IndexSet::new();
    ids.retain(|id| seen.insert(*id));
    ids
}

/// I19 (§6.1 Dormant, §3.2, R1839000): nothing reaches a card Dormant as the step began (`step`) until
/// it wakes. The step's events are walked in order: a card's `awakened`, `starshipLaunched` or own
/// `activated` (its waking Activate, the one ability it keeps, R1839001, R1839002, R3539004) frees it,
/// and before that every event that changes it and names it (`reached_ids`) is a finding, but for the
/// lawful ones:
/// - a hull's fuse, `fused` with `starshipFused` naming the card right after it (`starship::fuse_copy_into`,
///   R1852000, R3539004); an Equip's (R2260000) or any other fuse onto it is not;
/// - an `exiled` inside a `BOARD_SWEEPS` card's run, or a copier's running one's text (`copies_a_sweep`),
///   from its `hookResolving` (or the prompt its answer continues) to the next marker, trap or turn edge
///   (R3539002, R3554002, R3567000);
/// - a `controlChanged` or a `bounced` after a board `swapped` or a `rotated`, in the same window: the
///   board move carries the card whole (R4454000; the bounce is R73's and R14's at a Locked, reserved or
///   void zone, R3554002);
/// - a board restore's (`rolledBack`), in the same window: every card on a side it restores goes back as
///   the snapshot holds it, or to its hand with a `bounced`, a `controlChanged` for one that enters the
///   side and a `radiantSet` for Bogos Binted's Radiant face (R4454002);
/// - a `radiantSet` while Tears of the Kwzki Tree's world holds every card Radiant (`radiantWorld`,
///   R3569000, R4454001), which is no record change either.
///
/// A card still the same instance, still Dormant and still on the field after the step, neither freed
/// nor carried, holds the record it began with (`SLUMBER_KEPT`), which catches a change made with no
/// event. A card that turns Dormant inside the step is judged from the next. Draws nothing.
fn slumber_reach_findings(step: &SlumberStep, events: &[GameEvent], after: &GameState) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    if step.marks.is_empty() {
        return found;
    }
    // A Dormant card's own Activate answered by a prompt continues its run (R1839002).
    let mut released: IndexSet<&str> = step
        .pending_instance
        .as_deref()
        .filter(|id| step.marks.contains_key(*id))
        .into_iter()
        .collect();
    let mut carried: IndexSet<&str> = IndexSet::new();
    let mut running: Option<&str> = step.pending_def.as_deref();
    let mut sweeping = running.is_some_and(|def_id| is_board_sweep(after, def_id))
        || step
            .pending_instance
            .as_deref()
            .is_some_and(|id| copies_a_sweep(after, id) || step.sweepers.contains(id));
    let mut moving: Option<BoardMove> = None;
    // R4454001: under Tears of the Kwzki Tree every card is Radiant, the Dormant ones too (R3569000).
    let radiant_world = after.radiant_world == Some(true);
    for (at, event) in events.iter().enumerate() {
        match event {
            GameEvent::HookResolving {
                source_id, def_id, ..
            } => {
                running = Some(def_id);
                sweeping = is_board_sweep(after, def_id)
                    || copies_a_sweep(after, source_id)
                    || step.sweepers.contains(source_id.as_str());
                moving = None;
                continue;
            }
            GameEvent::TrapFired { def_id, .. } => {
                running = Some(def_id);
                sweeping = false;
                moving = None;
                continue;
            }
            GameEvent::TurnStarted { .. } | GameEvent::TurnEnded { .. } => {
                running = None;
                sweeping = false;
                moving = None;
                continue;
            }
            GameEvent::Swapped {
                what: SwapWhat::Board,
            }
            | GameEvent::Rotated { .. } => {
                moving = Some(BoardMove::Carry);
                continue;
            }
            GameEvent::RolledBack { sides, .. } => {
                moving = Some(BoardMove::Restore);
                // R4454002: what stood on a restored side goes back as the snapshot holds it, awake if
                // it was then, so for the rest of the step it is no longer the Dormant card the step
                // began with (R4524001); one the snapshot holds Dormant is marked again from the next.
                released.extend(
                    step.marks
                        .iter()
                        .filter(|(_, mark)| sides.contains(&mark.side))
                        .map(|(id, _)| id.as_str()),
                );
                continue;
            }
            GameEvent::Awakened { instance_id, .. }
            | GameEvent::StarshipLaunched { instance_id, .. }
            | GameEvent::Activated { instance_id, .. } => {
                released.insert(instance_id);
                continue;
            }
            _ => {}
        }
        for id in reached_ids(event) {
            let Some((id, mark)) = step.marks.get_key_value(id) else {
                continue;
            };
            let id = id.as_str();
            if released.contains(id) {
                continue;
            }
            match event {
                // R1852000, R3539004: a piece fusing into its hull.
                GameEvent::Fused { .. }
                    if matches!(
                        events.get(at + 1),
                        Some(GameEvent::StarshipFused { instance_id, .. }) if instance_id == id
                    ) =>
                {
                    carried.insert(id);
                }
                // R3539002, R3554002, R3567000: a board sweep.
                GameEvent::Exiled { .. } if sweeping => {
                    released.insert(id);
                }
                // R4454000, R4454002: a board move carries it whole, or bounces it on the way.
                GameEvent::ControlChanged { .. } if moving.is_some() => {
                    carried.insert(id);
                }
                GameEvent::Bounced { .. } if moving.is_some() => {
                    released.insert(id);
                }
                // R4454002: Bogos Binted's Radiant face rebuilds its side Radiant.
                GameEvent::RadiantSet { .. } if moving == Some(BoardMove::Restore) => {
                    carried.insert(id);
                }
                // R4454001: Tears of the Kwzki Tree's world.
                GameEvent::RadiantSet { .. } if radiant_world => {}
                _ => {
                    found.push(format!(
                        "I19 reached Dormant: {} names {} on turn {} while it is Dormant, in {}'s run (§6.1 \
                         Dormant, R1839000)",
                        event.event_type().as_str(),
                        mark.label,
                        after.turn,
                        running.unwrap_or("no card")
                    ));
                    released.insert(id);
                }
            }
        }
    }
    for (id, mark) in &step.marks {
        if released.contains(id.as_str()) || carried.contains(id.as_str()) {
            continue;
        }
        let Some(card) = find_instance(after, id) else {
            continue;
        };
        if !crate::slumber::is_slumbering(card) {
            continue;
        }
        let now = kept_record(card);
        let changed: Vec<&str> = SLUMBER_KEPT
            .into_iter()
            .filter(|field| !(radiant_world && *field == "radiant"))
            .filter(|field| now.get(*field) != mark.kept.get(*field))
            .collect();
        if !changed.is_empty() {
            found.push(format!(
                "I19 Dormant record changed: {} holds a different {} after the step on turn {}, with no event \
                 naming it (§6.1 Dormant, R1839000)",
                mark.label,
                changed.join(", "),
                after.turn
            ));
        }
    }
    found
}

/// I21: a Unit Immune to Spells on the field as the step begins (§6.1).
#[derive(Clone, Debug)]
struct ImmuneMark {
    label: String,
    /// Whether its own keywords hold the immunity (`layers::card_keywords` and its conditional ones), so
    /// it is still judged once the step takes it off the field, where no aura reaches it.
    own: bool,
    /// R4577008: the side it stands on and the lanes it covers as the step begins, which the Black Hole's
    /// pull at a turn's end reads.
    side: PlayerId,
    lanes: Vec<i32>,
}

/// I21: the Units Immune to Spells a step begins with, by instance id; the cards then set face-down on
/// either field, whose runs are a Trap's; and the run an answer step continues (`state.pending`'s
/// resume), which marks no `hookResolving` of its own (R2783000).
#[derive(Clone, Debug)]
struct ImmuneStep {
    marks: IndexMap<String, ImmuneMark>,
    set: IndexSet<String>,
    /// R4577008: the Black Hole the step begins with, when it pulls at a turn's end (R4440050).
    hole: Option<crate::geometry::BlackHole>,
    pending_def: Option<String>,
    pending_instance: Option<String>,
}

/// I21: the state a step begins on, as I21 reads it: every Unit on either field (`zones::active_units_of`,
/// a carried and a standing one included) that has Immune to Spells through the layers (§10.4).
fn immune_step(state: &GameState) -> ImmuneStep {
    let table = AuraTable::new(state);
    let marks = PLAYER_IDS
        .into_iter()
        .flat_map(|player| crate::zones::active_units_of(state, player))
        .filter(|unit| table.unit_has(unit, KeywordKind::ImmuneToSpells))
        .map(|unit| {
            let own = has_keyword(
                &crate::layers::card_keywords(state, unit),
                KeywordKind::ImmuneToSpells,
            ) || has_keyword(
                &crate::layers::conditional_keywords_of(state, unit),
                KeywordKind::ImmuneToSpells,
            );
            let mark = ImmuneMark {
                label: name_of(unit),
                own,
                side: match unit.zone {
                    Zone::Field { player, .. } => player,
                    _ => unit.controller,
                },
                lanes: crate::giga::lanes_of(unit),
            };
            (unit.id.clone(), mark)
        })
        .collect();
    let set = placed_cards(state)
        .into_iter()
        .filter(|placed| matches!(placed.zone, Zone::Field { .. }) && placed.card.set_as.is_some())
        .map(|placed| placed.card.id.clone())
        .collect();
    let resume = state.pending.as_ref().map(|pending| &pending.resume);
    ImmuneStep {
        marks,
        set,
        hole: state.black_hole.filter(crate::geometry::BlackHole::pulls_at_end),
        pending_def: resume.map(|resume| resume.def_id.clone()),
        pending_instance: resume.and_then(|resume| resume.instance_id.clone()),
    }
}

/// I21: the Spells whose run takes a whole lane, an immune Unit with it, each with the ruling that lets
/// it: the board loses the lane with all it holds, which is no effect reaching for the card (R4508004). A
/// fusion holding one takes it too (R43, R102), and so does a copier running its text.
const LANE_TAKERS: [(&str, &str); 2] = [
    // S+ #77 Black Hole: the state check swallows what stands in the hole's lane (R3554000).
    ("stinkyscapeplus-077", "R4508004"),
    // S+ #78 Slashed Dream Cries in Red: the slashed lane is exiled and voided (R3554002).
    ("stinkyscapeplus-078", "R4508004"),
];

/// I21: whether `def_id`'s run takes a lane (`LANE_TAKERS`), or a fusion's holding one (R43, R102).
fn takes_a_lane(state: &GameState, def_id: &str) -> bool {
    LANE_TAKERS.iter().any(|(taker, _)| *taker == def_id)
        || crate::subsystems::fuse::fused_ingredients(state, def_id)
            .is_some_and(|parts| parts.iter().any(|part| takes_a_lane(state, part)))
}

/// I21: whether the card `source_id` runs a copied Spell's text that takes a lane (C #57 Echo, R399).
fn copies_a_lane_taker(state: &GameState, source_id: &str) -> bool {
    find_instance(state, source_id)
        .and_then(|card| crate::subsystems::copied_text::copied_text_of(state, card))
        .is_some_and(|copy| takes_a_lane(state, &copy.def_id))
}

/// I21: whether the run of `def_id`, the card `source_id` names, is a Spell's: the face it runs on (the
/// card's own, read where it is now, else the base face) printed as a Spell in the catalog, not a Field
/// Spell or a Trap. A card set face-down runs as a Trap, as it is set and as it reveals (M #99's row,
/// R3806000: Immune to Spells does not stop it): one set as the step began (`set`), one whose trap fired
/// in it (`fired`) or one set as it ends. Read off the catalog, never `restrictions::effect_is_from_spell`.
fn runs_a_spell(
    state: &GameState,
    def_id: &str,
    source_id: Option<&str>,
    set: &IndexSet<String>,
    fired: &IndexSet<&str>,
) -> bool {
    if def_id.is_empty() || crate::catalog::find_def(Some(state), def_id).is_none() {
        return false;
    }
    let source = source_id.and_then(|id| find_instance(state, id));
    if source_id.is_some_and(|id| set.contains(id) || fired.contains(id))
        || source.is_some_and(|card| card.set_as.is_some())
    {
        return false;
    }
    let radiant = source.is_some_and(|card| card.radiant);
    crate::faces::card_type_of_face(state, def_id, radiant) == CardType::Spell
}

/// I21: a window a Spell's run is inside, from its event to the next `hookResolving`, trap or turn edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImmuneWindow {
    /// A board swap or a rotation (`Swapped` of the board, `Rotated`; R73, R14).
    Carry,
    /// A board restore (`RolledBack`: C+ #35 Rollback, S+ #84 Bogos Binted; R419, R3550007).
    Restore,
    /// An attack the Spell forced or declared for its controller (`AttackDeclared`).
    Combat,
}

/// I21: the card running as the walk reads it: its definition, whether its run is a Spell's, whether
/// it takes a lane, and the window it is inside.
#[derive(Clone, Copy, Debug)]
struct ImmuneRun<'e> {
    running: Option<&'e str>,
    spell: bool,
    takes_lane: bool,
    window: Option<ImmuneWindow>,
}

impl ImmuneRun<'_> {
    const NONE: ImmuneRun<'static> = ImmuneRun {
        running: None,
        spell: false,
        takes_lane: false,
        window: None,
    };
}

/// I21: whether `event` is an affliction's tick on `id` (R4508005): a sourceless hit via Burn or Toxic
/// (`afflictions::tick_burn_at_turn_end`, `tick_at_turn_start`; R4154000, R3540001), or the Burn's
/// removal right after its hit.
fn ticks_on(event: &GameEvent, previous: Option<&GameEvent>, id: &str) -> bool {
    let tick = |event: &GameEvent| match event {
        GameEvent::Damage {
            source_id: None,
            target_id,
            via: Some(KeywordKind::Burn | KeywordKind::Toxic),
            ..
        }
        | GameEvent::DamageAbsorbed {
            source_id: None,
            target_id,
            via: Some(KeywordKind::Burn | KeywordKind::Toxic),
            ..
        } => target_id == id,
        _ => false,
    };
    tick(event)
        || matches!(
            event,
            GameEvent::CounterChanged {
                counter: crate::wire::CounterKind::Burn,
                value: 0,
                ..
            }
        ) && previous.is_some_and(tick)
}

/// I21, R4623000: whether `id` stands on the field after the step with its own "Evolves when" holding
/// (R3536001): the settled state check's evolution (`evolution::evolve_standing`), the game's rule reading
/// the card's own condition, whose `evolved` lands in the open run before its Evolve text's marker.
fn evolves_standing(after: &GameState, id: &str) -> bool {
    find_instance(after, id).is_some_and(|card| {
        matches!(card.zone, Zone::Field { .. })
            && crate::scripts::script_of(after, card)
                .evolves_when
                .as_ref()
                .is_some_and(|when| {
                    when(crate::script::HookArgs {
                        state: after,
                        self_: card,
                        radiant: card.radiant,
                    })
                })
    })
}

/// I21, R4577008: whether an `exiled` of `mark`'s Unit may be the Black Hole's pull at the end of `ended`'s
/// turn (R4440050, `geometry::pull_at_turn_end`): the step began with a hole that pulls at a turn's end and
/// pulls that player's permanents, the turn has ended, and the Unit stood on that player's side one lane
/// from the hole, where one step toward it reaches it (R3554001). The hole is the board's, not the Spell's.
fn pulled_into_the_hole(
    hole: Option<crate::geometry::BlackHole>,
    ended: Option<PlayerId>,
    mark: &ImmuneMark,
) -> bool {
    let (Some(hole), Some(player)) = (hole, ended) else {
        return false;
    };
    mark.side == player
        && hole.pulls.is_none_or(|pulls| pulls == player)
        && mark.lanes.iter().any(|lane| (lane - hole.lane).abs() == 1)
}

/// I21 (§6.1 Immune to Spells, R4453000): no Spell changes a Unit Immune to Spells on the field. The
/// step's events are walked in order, tracking the card running (`hookResolving`, a trap, a turn edge
/// and the AI's turn; a play nested in a run is its own from its `cardPlayed` to its `cardResolved`,
/// whose unmarked hooks are no Spell's): while it is a Spell (`runs_a_spell`), every event that changes
/// and names (`reached_ids`) a Unit immune as the step began (`step`) is a finding, once per Unit, but for
/// the lawful ones:
/// - anything inside an attack the Spell declared or forced, its combat, hooks and deaths (R4508002):
///   §6.1's immunity is no restriction on attacks, and the attacking Unit is what reaches its target;
/// - a `controlChanged` or a `bounced` after a board `swapped`, a `rotated` or a `rolledBack`, and a
///   board restore's `radiantSet` (R4508001; R4454000, R4454002): the board move carries it whole;
/// - an `exiled` in a `LANE_TAKERS` card's run (R4508004): the lane goes with all it holds; and one of
///   a Unit the Black Hole's pull swallows at a turn's end (R4577008, `pulled_into_the_hole`), a stage of
///   the turn's end with no marker of its own, which follows whatever Spell's trigger ran last (R4440050);
/// - a `radiantSet` while Tears of the Kwzki Tree's world holds every card Radiant (R4508003, R4454001);
/// - the game's own rules that run inside the Spell's run with no marker (R4508005): §4.5 step 1's
///   switch out of Defense of a Unit the board bars from it (`forbidden_defense`, I17's), and a Burn or
///   Toxic tick (`ticks_on`); and the settled state check's `evolved` of a Unit whose "Evolves when" holds
///   after the step (R4623000, `evolves_standing`).
///
/// A Unit is judged only when it is still on the field and still immune after the step (an aura that
/// dropped mid-step, or a step that first took its immunity away, makes no finding), or when it is gone
/// from the field and its own keywords held the immunity. A Spell's delayed effect, and its run after a
/// nested marker, carry no marker of the Spell's, so I21 cannot see them. Draws nothing.
fn immune_reach_findings(step: &ImmuneStep, events: &[GameEvent], after: &GameState) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    if step.marks.is_empty() {
        return found;
    }
    let table = AuraTable::new(after);
    let judged: IndexSet<&str> = step
        .marks
        .iter()
        .filter(|(id, mark)| match find_instance(after, id) {
            Some(card) if matches!(card.zone, Zone::Field { .. }) => {
                table.unit_has(card, KeywordKind::ImmuneToSpells)
            }
            _ => mark.own,
        })
        .map(|(id, _)| id.as_str())
        .collect();
    if judged.is_empty() {
        return found;
    }
    let barred = crate::restrictions::defense_barred(after);
    let mut released: IndexSet<&str> = IndexSet::new();
    // The ids a `trapFired` named so far: a card set face-down whose trap fires runs as a Trap.
    let mut fired: IndexSet<&str> = IndexSet::new();
    let pending = step.pending_instance.as_deref();
    let mut run = ImmuneRun {
        running: step.pending_def.as_deref(),
        spell: step
            .pending_def
            .as_deref()
            .is_some_and(|def_id| runs_a_spell(after, def_id, pending, &step.set, &fired)),
        takes_lane: step
            .pending_def
            .as_deref()
            .is_some_and(|def_id| takes_a_lane(after, def_id))
            || pending.is_some_and(|id| copies_a_lane_taker(after, id)),
        window: None,
    };
    // The runs a nested play interrupted, each with the card it plays, innermost last.
    let mut nested: Vec<(&str, ImmuneRun<'_>)> = Vec::new();
    // R4508003: under Tears of the Kwzki Tree every card is Radiant (R3569000).
    let radiant_world = after.radiant_world == Some(true);
    // R4577008: whose turn has ended in this step, so the Black Hole's pull may follow.
    let mut ended: Option<PlayerId> = None;
    for (at, event) in events.iter().enumerate() {
        match event {
            GameEvent::HookResolving {
                source_id, def_id, ..
            } => {
                run = ImmuneRun {
                    running: Some(def_id),
                    spell: runs_a_spell(after, def_id, Some(source_id), &step.set, &fired),
                    takes_lane: takes_a_lane(after, def_id) || copies_a_lane_taker(after, source_id),
                    window: None,
                };
                continue;
            }
            GameEvent::TrapFired {
                instance_id, def_id, ..
            } => {
                fired.insert(instance_id);
                run = ImmuneRun {
                    running: Some(def_id),
                    ..ImmuneRun::NONE
                };
                continue;
            }
            GameEvent::TurnEnded { player, .. } => {
                ended = Some(*player);
                run = ImmuneRun::NONE;
                continue;
            }
            GameEvent::TurnStarted { .. } | GameEvent::AutopilotEngaged { .. } => {
                ended = None;
                run = ImmuneRun::NONE;
                continue;
            }
            // A play begins its own run (its `onPlayHook`s and its arrival are no Spell's), and the run it
            // interrupted goes on once it resolves.
            GameEvent::CardPlayed { instance_id, .. } => {
                nested.push((instance_id, run));
                run = ImmuneRun::NONE;
                continue;
            }
            GameEvent::CardResolved { instance_id, .. } => {
                if let Some(at) = nested.iter().rposition(|(played, _)| played == instance_id) {
                    run = nested[at].1;
                    nested.truncate(at);
                }
                continue;
            }
            GameEvent::Swapped {
                what: SwapWhat::Board,
            }
            | GameEvent::Rotated { .. } => {
                run.window = Some(ImmuneWindow::Carry);
                continue;
            }
            GameEvent::RolledBack { .. } => {
                run.window = Some(ImmuneWindow::Restore);
                continue;
            }
            GameEvent::AttackDeclared { .. } => {
                run.window = Some(ImmuneWindow::Combat);
                continue;
            }
            _ => {}
        }
        if !run.spell {
            continue;
        }
        for id in reached_ids(event) {
            let Some((id, mark)) = step.marks.get_key_value(id) else {
                continue;
            };
            let id = id.as_str();
            if released.contains(id) || !judged.contains(id) {
                continue;
            }
            let lawful = match event {
                // R4508002: an attack, the Spell's forced one included.
                _ if run.window == Some(ImmuneWindow::Combat) => true,
                // R4508005: an affliction's tick.
                _ if ticks_on(event, at.checked_sub(1).and_then(|before| events.get(before)), id) => true,
                // R4508001: a board move carries it whole, or bounces it on the way.
                GameEvent::ControlChanged { .. } | GameEvent::Bounced { .. } => run.window.is_some(),
                // R4508001: Bogos Binted's Radiant face rebuilds its side Radiant (R4454002); R4508003:
                // Tears of the Kwzki Tree's world.
                GameEvent::RadiantSet { .. } => run.window == Some(ImmuneWindow::Restore) || radiant_world,
                // R4508004: a lane the board loses; R4577008: or the hole's pull at a turn's end.
                GameEvent::Exiled { .. } => run.takes_lane || pulled_into_the_hole(step.hole, ended, mark),
                // R4508005: §4.5 step 1 switches it out of a Defense Position the board bars it from.
                // R4623000: the state check evolves it by its own standing condition (R3536001).
                GameEvent::Evolved { .. } => evolves_standing(after, id),
                GameEvent::PositionSwitched {
                    position: Position::Atk,
                    ..
                } => find_instance(after, id).is_some_and(|card| {
                    matches!(card.zone, Zone::Field { .. })
                        && card.position == Some(Position::Atk)
                        && forbidden_defense(&table, card, barred).is_some()
                }),
                _ => false,
            };
            if lawful {
                continue;
            }
            found.push(format!(
                "I21 Spell reached Immune to Spells: {} names {} on turn {} in {}'s run (§6.1, R4453000)",
                event.event_type().as_str(),
                mark.label,
                after.turn,
                run.running.unwrap_or("no card")
            ));
            released.insert(id);
        }
    }
    found
}

/// I23: what a card's text is made of in its record (§6.1, §6.3): the definition it wears (a Fuse onto
/// a card keeps its id and gives it a new one, R77), a Vanilla's flag, the Nerfs and Buffs it keeps (`tuning`;
/// a cost change is `costMod`, which every cost effect shares, and its `degraded` names it), its hidden
/// Nerfs (R1845000), the stats a Set stats gave it (`statsOverride`, §6.3: Immutable stops it, R1185002;
/// a summon, Flicker or Reborn writes it anew, R78, R175) and the text it runs copied or fused in
/// (`unit_text::UNIT_TEXT_KEY`: R1850000, R4440006).
#[derive(Clone, Debug, PartialEq)]
struct TextRecord {
    def_id: String,
    vanilla: bool,
    tuning: Option<Tuning>,
    hidden_nerfs: Option<i32>,
    stats_override: Option<AttackHealth>,
    unit_text: Option<Value>,
}

impl TextRecord {
    fn of(card: &CardInstance) -> TextRecord {
        TextRecord {
            def_id: card.def_id.clone(),
            vanilla: card.vanilla,
            tuning: card.tuning.clone(),
            hidden_nerfs: card.hidden_nerfs,
            stats_override: card.stats_override,
            unit_text: card
                .memory
                .get(crate::subsystems::unit_text::UNIT_TEXT_KEY)
                .cloned(),
        }
    }

    /// `card` wearing this text again, the rest of its record as it is.
    fn put_on(&self, card: &CardInstance) -> CardInstance {
        let mut worn = CardInstance {
            def_id: self.def_id.clone(),
            vanilla: self.vanilla,
            tuning: self.tuning.clone(),
            hidden_nerfs: self.hidden_nerfs,
            stats_override: self.stats_override,
            ..card.clone()
        };
        let key = crate::subsystems::unit_text::UNIT_TEXT_KEY;
        match &self.unit_text {
            Some(text) => {
                worn.memory.insert(key.to_string(), text.clone());
            }
            None => {
                worn.memory.shift_remove(key);
            }
        }
        worn
    }

    /// The fields `now` holds differently, by their wire names; a tuning that only gained keywords kept
    /// for good (`kept`, R4609003) is no change.
    fn differing(&self, now: &TextRecord, kept: &[&Keyword]) -> Vec<&'static str> {
        held_names(&[
            (self.def_id != now.def_id, "defId"),
            (self.vanilla != now.vanilla, "vanilla"),
            (
                self.tuning != now.tuning
                    && !kept_grants_only(self.tuning.as_ref(), now.tuning.as_ref(), kept),
                "tuning",
            ),
            (self.hidden_nerfs != now.hidden_nerfs, "hiddenNerfs"),
            (self.stats_override != now.stats_override, "statsOverride"),
            (self.unit_text != now.unit_text, "unitText"),
        ])
    }
}

/// I23, R4609003: whether `now` is `was` with keywords appended to its `addKeywords` and nothing else,
/// each one the step's `keywordGranted` kept for good (`tune::gain_keyword_for_good`: R1870007, R4154202),
/// a grant Immutable never stopped and no Buff.
fn kept_grants_only(was: Option<&Tuning>, now: Option<&Tuning>, kept: &[&Keyword]) -> bool {
    let was = was.cloned().unwrap_or_default();
    let now = now.cloned().unwrap_or_default();
    let before = was.add_keywords.clone().unwrap_or_default();
    let after = now.add_keywords.clone().unwrap_or_default();
    after.starts_with(&before)
        && after[before.len()..]
            .iter()
            .all(|keyword| kept.contains(&keyword))
        && Tuning {
            add_keywords: was.add_keywords.clone(),
            ..now
        } == was
}

/// I23: a card Immutable as the step begins (§6.1, R4353000).
#[derive(Clone, Debug)]
struct ImmutableMark {
    label: String,
    /// The zone it stands in as the step begins: its record is held only while it is there again after.
    zone: ZoneName,
    /// Whether its own keywords (`layers::card_keywords` and its conditional ones) make it Immutable, so
    /// it is still judged once the step takes it out of the zones I23 reads, or out of every pile.
    own: bool,
    text: TextRecord,
}

/// I23: the Immutable cards a step begins with, by instance id, and the Glitch's reset count then (R676).
#[derive(Clone, Debug)]
struct ImmutableStep {
    marks: IndexMap<String, ImmutableMark>,
    resets: Option<i32>,
}

/// I23 (R4353000): whether `keywords`, the ones I23 reads `card` by, make it Immutable: the keyword, or
/// Indestructible on a card whose running face prints the Unit type in the catalog. Read here, never
/// through `layers::is_immutable`, `layers::immutable_by` or `restrictions`.
fn immutable_by_keywords(state: &GameState, card: &CardInstance, keywords: &[Keyword]) -> bool {
    has_keyword(keywords, KeywordKind::Immutable)
        || (has_keyword(keywords, KeywordKind::Indestructible)
            && crate::catalog::find_def(Some(state), &card.def_id).is_some()
            && crate::faces::card_type_of_face(state, &card.def_id, card.radiant) == CardType::Unit)
}

/// I23 (R4609000): the keywords I23 reads `card` by in `zone`, or `None` where its text is not judged: a
/// Dormant card (I19's, R1839000), a face-down Trap or Field Trap (`face_down_now`, as I22 leaves it) and
/// every zone but the field, a hand and a library, where nothing that Immutable stops reaches (§6.3). On
/// the field the layers' (§10.4); in a hand or a library the card's own (`layers::card_keywords`), as a
/// Nerf and a hidden Nerf read it there.
fn judged_keywords(
    state: &GameState,
    table: &AuraTable,
    card: &CardInstance,
    zone: ZoneName,
) -> Option<Vec<Keyword>> {
    if crate::slumber::is_slumbering(card) {
        return None;
    }
    match zone {
        ZoneName::Field => (!face_down_now(state, card)).then(|| table.unit_view(card).keywords),
        ZoneName::Hand | ZoneName::Library => Some(crate::layers::card_keywords(state, card)),
        _ => None,
    }
}

/// I23: the state a step begins on, as I23 reads it: every card in a field slot (a pile's and a carried
/// one's included), a hand or a library that is Immutable by `judged_keywords`.
fn immutable_step(state: &GameState) -> ImmutableStep {
    let table = AuraTable::new(state);
    let marks = placed_cards(state)
        .into_iter()
        .filter_map(|placed| {
            let zone = placed.zone.z();
            let keywords = judged_keywords(state, &table, placed.card, zone)?;
            if !immutable_by_keywords(state, placed.card, &keywords) {
                return None;
            }
            let mut own = crate::layers::card_keywords(state, placed.card);
            own.extend(crate::layers::conditional_keywords_of(state, placed.card));
            let mark = ImmutableMark {
                label: name_of(placed.card),
                zone,
                own: immutable_by_keywords(state, placed.card, &own),
                text: TextRecord::of(placed.card),
            };
            Some((placed.card.id.clone(), mark))
        })
        .collect();
    ImmutableStep {
        marks,
        resets: state.resets,
    }
}

/// I23 (§6.1 Immutable, R23, R386, R4353000; R4609000): nothing changes the text of a card Immutable as
/// the step began (`step`) and after it. The step's events are walked in order, and each that changes an
/// Immutable card's text and names it is a finding, once per card:
/// - a `transformed` of it, a Vanilla's (its own definition on both sides) and a copied or fused text's
///   included, but for a Replace off the field (R35, R921: a new instance, hidden from a player as a hand
///   or library card is, R177), which rewrites no card (R4609001);
/// - a `degraded` or an `upgraded` of it, but for one whose change is `none` (R440, R4609001);
/// - a `fused` whose kept card is it (Fuse-onto, R77): its ingredients are taken whole, and a card the
///   step made is not in `step` (R4353003, R4609002).
///
/// A card still the same instance and in the same zone after the step holds the text it began with
/// (`TextRecord`), which catches a change made with no event, but for a keyword kept for good, a grant
/// (R4609003), and for the lawful resets (R4609002): a card `flickered` or `summoned` anew (R78; Reborn
/// clears a Vanilla) from then on, and the whole step after a `rolledBack` (R419, R4454002) or a Glitch
/// (`glitched`, or the reset count moved: R676, R678). Radiant is the card's own text (R23) and is never
/// read. A card is judged when it is Immutable after the step with the text it began with put back on
/// (so a grant or an aura that lapses in the step lets it go, R4353000), or, gone from the field, a hand
/// and a library, when its own keywords held it. Draws nothing.
fn immutable_text_findings(step: &ImmutableStep, events: &[GameEvent], after: &GameState) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    if step.marks.is_empty() {
        return found;
    }
    let table = AuraTable::new(after);
    let judged: IndexSet<&str> = step
        .marks
        .iter()
        .filter(|(id, mark)| {
            let worn = find_instance(after, id).map(|card| mark.text.put_on(card));
            match worn.and_then(|card| Some((judged_keywords(after, &table, &card, card.zone.z())?, card))) {
                Some((keywords, card)) => immutable_by_keywords(after, &card, &keywords),
                None => mark.own,
            }
        })
        .map(|(id, _)| id.as_str())
        .collect();
    if judged.is_empty() {
        return found;
    }
    // The cards a finding named, whose record is not held again.
    let mut released: IndexSet<&str> = IndexSet::new();
    // R4609002: the cards reset in place, whose record is not held.
    let mut reset: IndexSet<&str> = IndexSet::new();
    for event in events {
        let (id, lawful) = match event {
            // R4609001: a Replace off the field.
            GameEvent::Transformed {
                instance_id,
                new_instance_id,
                hidden_from,
                ..
            } => (
                instance_id,
                new_instance_id != instance_id && hidden_from.is_some(),
            ),
            // R4609001, R440: an application on a hidden card that changed nothing.
            GameEvent::Degraded {
                instance_id, change, ..
            }
            | GameEvent::Upgraded {
                instance_id, change, ..
            } => (instance_id, *change == TuningChange::None),
            GameEvent::Fused {
                result_instance_id, ..
            } => (result_instance_id, false),
            // R4609002, R78: left the field and back, or back through Reborn.
            GameEvent::Flickered { instance_id, .. } | GameEvent::Summoned { instance_id, .. } => {
                reset.insert(instance_id);
                continue;
            }
            _ => continue,
        };
        let Some((id, mark)) = step.marks.get_key_value(id) else {
            continue;
        };
        let id = id.as_str();
        if lawful || released.contains(id) || !judged.contains(id) {
            continue;
        }
        found.push(format!(
            "I23 Immutable text changed: {} names {} on turn {} (§6.1, R23, R386, R4353000)",
            event.event_type().as_str(),
            mark.label,
            after.turn
        ));
        released.insert(id);
    }
    // R4609002: a board restore or a Glitch re-deals what it reaches.
    let restored = after.resets != step.resets
        || events
            .iter()
            .any(|event| matches!(event, GameEvent::RolledBack { .. } | GameEvent::Glitched { .. }));
    if restored {
        return found;
    }
    for (id, mark) in &step.marks {
        let id = id.as_str();
        if !judged.contains(id) || released.contains(id) || reset.contains(id) {
            continue;
        }
        let Some(card) = find_instance(after, id) else {
            continue;
        };
        if card.zone.z() != mark.zone {
            continue;
        }
        // R4609003: the keywords the step kept on it for good.
        let kept: Vec<&Keyword> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::KeywordGranted {
                    instance_id,
                    keyword,
                    lost: None,
                } if instance_id == id => Some(keyword),
                _ => None,
            })
            .collect();
        let changed = mark.text.differing(&TextRecord::of(card), &kept);
        if !changed.is_empty() {
            found.push(format!(
                "I23 Immutable record changed: {} holds a different {} after the step on turn {}, with no \
                 transformed, degraded, upgraded or fused naming it (§6.1, R23, R4353000)",
                mark.label,
                changed.join(", "),
                after.turn
            ));
        }
    }
    found
}

/// I22: what bars a field card as a `target` pick, read off the board (§6.1, §10.5 step 1).
#[derive(Clone, Copy, Debug)]
struct PickBars {
    /// Dormant: nothing targets it (R1839000).
    dormant: bool,
    /// Face-up and Evasive through the layers (R4353002).
    evasive: bool,
    /// Face-up and Immune to Spells through the layers (R450).
    immune: bool,
}

/// I22 (R1487000, R4353002): a Trap or Field Trap, a card set face-down among them, neither flipped nor
/// revealed. Its keywords sleep, so I22 reads neither Evasive nor Immune to Spells off it.
fn face_down_now(state: &GameState, card: &CardInstance) -> bool {
    matches!(
        crate::faces::card_type_of(state, card),
        CardType::Trap | CardType::FieldTrap
    ) && card.face_up != Some(true)
        && card.revealed != Some(true)
}

/// I22: the barred field cards among `picks` on `state`, by instance id, in pick order; each card read
/// once, through one aura table built only when a pick names a field card. A hero, a zone, a mode or a
/// card in a hand or a pile bars nothing.
fn barred_picks<'p>(
    state: &GameState,
    picks: impl IntoIterator<Item = &'p Selection>,
) -> IndexMap<String, PickBars> {
    let named: IndexSet<&str> = picks
        .into_iter()
        .filter_map(|pick| match pick {
            Selection::Instance { instance_id } => Some(instance_id.as_str()),
            _ => None,
        })
        .collect();
    let on_field: Vec<&CardInstance> = named
        .into_iter()
        .filter_map(|id| find_instance(state, id))
        .filter(|card| matches!(card.zone, Zone::Field { .. }))
        .collect();
    if on_field.is_empty() {
        return IndexMap::new();
    }
    let auras = AuraTable::new(state);
    on_field
        .into_iter()
        .filter_map(|card| {
            let face_up = !face_down_now(state, card);
            let bars = PickBars {
                dormant: crate::slumber::is_slumbering(card),
                evasive: face_up && auras.unit_has(card, KeywordKind::Evasive),
                immune: face_up && auras.unit_has(card, KeywordKind::ImmuneToSpells),
            };
            (bars.dormant || bars.evasive || bars.immune).then(|| (card.id.clone(), bars))
        })
        .collect()
}

/// I22: why `source`'s targeting may not aim at a card `bars` describes, with the ruling, or `None` when
/// it may. A Dormant card bars every source, and no card at all (R1839000). Evasive bars a Spell and a
/// Field Spell, the source read as R781001 reads it: a Unit while it stands in a unit zone, else its
/// face's printed type, so a Spell set face-down still aims as one (R4353002). Immune to Spells bars a
/// Spell read where it is (E35, R450): a Spell set face-down is a Trap there, and a Field Spell no Spell.
fn source_barred(
    state: &GameState,
    source: Option<&CardInstance>,
    bars: PickBars,
) -> Option<(&'static str, &'static str)> {
    if bars.dormant {
        return Some(("a Dormant card, which nothing targets", "R1839000"));
    }
    let source = source?;
    let aims_as = match source.zone {
        Zone::Field { row: Row::Units, .. } => CardType::Unit,
        _ => crate::faces::card_type_of_face(state, &source.def_id, source.radiant),
    };
    if bars.evasive && matches!(aims_as, CardType::Spell | CardType::FieldSpell) {
        return Some((
            "an Evasive card in play, which no Spell or Field Spell targets",
            "R4353002",
        ));
    }
    if bars.immune && crate::faces::card_type_of(state, source) == CardType::Spell {
        return Some(("a card Immune to Spells, which no Spell targets", "R450"));
    }
    None
}

/// I22 (#4540): every pick of one random `target` draw (`testkit::seams`) is one its source may aim at,
/// judged on the state it was drawn on: R452's random cast and R1200's Mayor draw only from what the
/// targeting point leaves.
pub fn target_draw_findings(draw: &TargetDraw) -> Vec<String> {
    let (under, drawn_by) = match draw.by {
        TargetDrawBy::RandomCast => ("a random cast", "R452"),
        TargetDrawBy::Mayor => ("a Mayor", "R1200"),
    };
    let source = draw.source.as_ref();
    barred_picks(&draw.state, &draw.picks)
        .into_iter()
        .filter_map(|(id, bars)| {
            let (what, rule) = source_barred(&draw.state, source, bars)?;
            Some(format!(
                "I22 unlawful random target: {} drew {} for {} on turn {} under {under}: {what} ({rule}, {drawn_by})",
                source.map_or_else(|| "no card".to_string(), name_of),
                card_label(&draw.state, &id),
                draw.seat,
                draw.state.turn,
            ))
        })
        .collect()
}

/// I22 (#4540): every `target` pick a listed play or activation of `viewer`'s declares is one its source
/// may aim at (§10.5 step 1, R450). I8 and I14 hold the listing and the reducer to each other, so a
/// filter both read passes both. A pick is matched to its declaration by the reducer's own reading
/// (`play_choices::targeting_decls_of`), and only a `target` declaration's is judged: a Spell may Tribute
/// its controller's own Evasive Unit. While a Mayor acts no listed action names a target (R1200), and
/// the record judges the draws instead. A face-down play declares nothing (R1044).
pub fn listed_target_findings(state: &GameState, viewer: PlayerId, legal: &[ActionBody]) -> Vec<String> {
    if crate::random_targets::targets_random(state) {
        return Vec::new();
    }
    let barred = barred_picks(
        state,
        legal.iter().flat_map(|body| match body {
            ActionBody::Play {
                targets: Some(targets),
                face_down: None,
                ..
            }
            | ActionBody::Activate {
                targets: Some(targets),
                ..
            } => targets.as_slice(),
            _ => &[],
        }),
    );
    if barred.is_empty() {
        return Vec::new();
    }
    let mut found: IndexSet<String> = IndexSet::new();
    let names_barred = |targets: &[Selection]| {
        targets.iter().any(
            |pick| matches!(pick, Selection::Instance { instance_id } if barred.contains_key(instance_id)),
        )
    };
    for body in legal {
        let (verb, source, targets, decls) = match body {
            ActionBody::Play {
                instance_id,
                targets: Some(targets),
                modes,
                face_down: None,
                ..
            } if names_barred(targets) => {
                let (Some(card), Some(play)) =
                    (find_instance(state, instance_id), PlayAction::from_body(body))
                else {
                    continue;
                };
                let face = crate::play_choices::face_for_action(state, viewer, card, &play);
                let modes = modes.as_deref().unwrap_or(&[]);
                let decls =
                    crate::play_choices::targeting_decls_of(state, viewer, &face, targets, modes, None);
                ("play", face, targets, decls)
            }
            ActionBody::Activate {
                instance_id,
                ability,
                targets: Some(targets),
                modes,
                ..
            } if names_barred(targets) => {
                let Some(card) = find_instance(state, instance_id) else {
                    continue;
                };
                let abilities = crate::subsystems::activate::abilities_of(state, card);
                let Some(decl) = (match ability {
                    Some(ability) => abilities.into_iter().find(|decl| decl.id == *ability),
                    None => abilities.into_iter().next(),
                }) else {
                    continue;
                };
                let modes = modes.as_deref().unwrap_or(&[]);
                let decls = crate::play_choices::targeting_decls_of(
                    state,
                    viewer,
                    card,
                    targets,
                    modes,
                    Some(decl.targets.as_slice()),
                );
                ("activation", card.clone(), targets, decls)
            }
            _ => continue,
        };
        for (pick, decl) in targets.iter().zip(decls) {
            let (Selection::Instance { instance_id }, Some(_)) = (pick, decl) else {
                continue;
            };
            let Some(bars) = barred.get(instance_id) else {
                continue;
            };
            if let Some((what, rule)) = source_barred(state, Some(&source), *bars) {
                found.insert(format!(
                    "I22 unlawful listed target: {viewer}'s {verb} of {} offers {} on turn {}: {what} ({rule})",
                    name_of(&source),
                    card_label(state, instance_id),
                    state.turn
                ));
            }
        }
    }
    found.into_iter().collect()
}

/// I22 (#4540): every option an open `target` prompt offers is one its source may aim at (R450): the
/// targeting point filters each such prompt as it opens (`prompts::open_prompt`). The source is the card
/// the prompt's resume names, as the targeting point reads it.
pub fn offered_target_findings(state: &GameState) -> Vec<String> {
    let Some(pending) = state
        .pending
        .as_ref()
        .filter(|pending| pending.kind == PromptKind::Target)
    else {
        return Vec::new();
    };
    let barred = barred_picks(state, pending.options.iter().map(|option| &option.selection));
    if barred.is_empty() {
        return Vec::new();
    }
    let source = pending
        .resume
        .instance_id
        .as_deref()
        .and_then(|id| find_instance(state, id));
    barred
        .into_iter()
        .filter_map(|(id, bars)| {
            let (what, rule) = source_barred(state, source, bars)?;
            Some(format!(
                "I22 unlawful target offered: prompt {} of {} offers {} to {} on turn {}: {what} ({rule})",
                pending.id,
                source.map_or_else(|| pending.resume.def_id.clone(), name_of),
                card_label(state, &id),
                pending.player_id,
                state.turn
            ))
        })
        .collect()
}

/// The events that take a card out of a zone and name the definition it left as (`Lineage::gone_as`).
const LEAVING_EVENTS: [&str; 5] = ["destroyed", "exiled", "burned", "discarded", "bounced"];

/// Where a card an id names stands for one viewer (`standing_of`'s answer).
struct Standing {
    reads: bool,
    where_: String,
}

/// Whether this viewer reads the card an id names, judged by where the card is now: a card the state
/// holds reads unless the hidden set has it, a vanished one by its successor, and one with none by the
/// rules below. An id neither the state nor the log knows vouches for nothing.
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
            // R316, #562: a card a full library refused before it existed, copying nothing, was never hidden.
            if lineage.never_made.contains(id) {
                return Some(Standing {
                    reads: true,
                    where_: "no pile (it was never created)".to_string(),
                });
            }
            // R1853003, R1861000: a card sold at the Marketplace (S #89 Spice) was named to both seats.
            if lineage.sold.contains(id) {
                return Some(Standing {
                    reads: true,
                    where_: "no pile (it was sold, publicly)".to_string(),
                });
            }
            let defs = lineage.ties.get(id)?;
            // R1074000: in a duel, a card the log last shows in a hand, token or not, was passed out of it.
            if let Some(&holder) = lineage.in_hand.get(id) {
                return Some(Standing {
                    reads: holder == viewer,
                    where_: format!("{holder}'s hand, which it was passed out of"),
                });
            }
            // R11: a token that ceased to exist (it left the field, was discarded or burned) was public when
            // it went, judged by what it left as. Any other card in no pile went unseen (a Glitch's reset
            // or boards, R676, R678), named by no leaving, and stays as hidden as every definition it wore.
            let token = match lineage.gone_as.get(id) {
                Some(def) => is_token_def(state, def, viewer),
                None => defs.iter().all(|def| is_token_def(state, def, viewer)),
            };
            return Some(Standing {
                reads: token,
                where_: "no pile (it ceased to exist)".to_string(),
            });
        };
        id = successor.as_str();
    }
    None
}

/// Whether `def_id` is a token's definition. A match-made one (R77's Fuse, a crafted card) the state
/// no longer holds, because a Glitch's reset (R676) dealt the game again without its `transient_defs`,
/// is rebuilt from its id alone as C+ #29 rebuilds one (R179), on a scratch copy; one that cannot be
/// rebuilt (a digest, R468) counts as no token, so it stays hidden.
fn is_token_def(state: &GameState, def_id: &str, viewer: PlayerId) -> bool {
    if let Some(def) = crate::catalog::find_def(Some(state), def_id) {
        return def.token;
    }
    let mut scratch = state.clone();
    crate::subsystems::fuse::rebuild_fused_def(&mut scratch, def_id, viewer).is_some_and(|def| def.token)
}

/// The hidden set, from the state's zones on their own terms, never from `view_for`'s rules: the other seat's
/// hand and Resource tray while the game is live (R434, R1840000), both libraries (§9.1), its face-down traps
/// and cards being set, mulligan returns awaiting their shuffle, and their former ids (R227); less every
/// definition the viewer reads through a card, never through the view being checked.
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
    if state.result.is_none() && !crate::revealed_hand::hand_revealed_to(state, rival, viewer) {
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
            Selection::Craft { recipe } => {
                // R880: a `craft` option's key names the recipe's definition, which the holder reads
                // with the offered recipe (§10.8), even when the rival's hand holds one crafted from it.
                defs.shift_remove(&crate::subsystems::craft::craft_id(recipe));
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

    // R1853001, R1881006: the Marketplace's offering is public, so a definition on a lot the viewer is
    // offered (a rolled lot, or one an effect added for them) is no secret, whatever hidden card shares it.
    for offer in state.marketplace.iter().flat_map(|market| market.offers.iter()) {
        if offer.owner.is_none_or(|owner| owner == viewer) {
            defs.shift_remove(&offer.def_id);
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
        for enchantment in card.enchantments.iter().flatten() {
            if let Enchantment::SwapsBook { from } = enchantment {
                defs.shift_remove(from);
            }
        }
        // R1852005: a Starship's memory names the pieces fused into it, each a card that died face-up
        // (R1852000), read off the Starship.
        for piece in crate::starship::starship_pieces_of(card) {
            defs.shift_remove(&piece.def_id);
        }
        // R3291000, R468: a fused card's abilities name the ingredient each is printed on
        // (`ActivationView.printedOn`), read off the card: its definition keeps its `ingredients`.
        for source in &crate::scripts::script_of(state, card).activation_sources {
            defs.shift_remove(&source.def_id);
        }
        // R1956001: a Created card's owner is told the card that generated it (`CardView.createdBy`),
        // read off the card, as an Echo's copied Spell is.
        if card.owner == viewer
            && let Some(generator) = state.generators.get(&card.id)
        {
            defs.shift_remove(generator);
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

/// R1852003, R1852005: a launch names the pieces its Starship held, the public memory of a public
/// launch, whatever has become of each piece's card since (a piece shuffled back into a library, a
/// Starship that died and lost its memory, R78). The definitions the engine's own event named are
/// taken out of the view's copy of it, so only what `view_for` may have added is scanned.
fn drop_launched_pieces(shown: &mut Value, launched: &[String]) {
    if let Some(Value::Array(pieces)) = shown.get_mut("pieces") {
        pieces.retain(|piece| {
            !piece
                .as_str()
                .is_some_and(|def| launched.iter().any(|named| named == def))
        });
    }
}

/// R1852000, R1852005: a fusion names its dead piece with the definition it died as, face up, the public
/// memory of a public death whatever has become of the piece's card or its Starship since (a Classic+ #35
/// Rollback that took the Starship back to its owner's hand while a copy of the piece waits in that
/// owner's library: fuzz seed 5912, issue #3800). The definition the engine's own event named is taken out
/// of the view's copy of it, as a launch's pieces are; the piece's id is still scanned.
fn drop_fused_piece(shown: &mut Value, piece_def: &str) {
    if let Some(object) = shown.as_object_mut()
        && object.get("defId").and_then(Value::as_str) == Some(piece_def)
    {
        object.remove("defId");
    }
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

/// Whether `at` is `view.pending.options[<n>].instanceId`, by hand (no regex crate, SURFACE §8).
fn is_pending_option_instance_id(at: &str) -> bool {
    let Some(rest) = at.strip_prefix("view.pending.options[") else {
        return false;
    };
    let Some((index, tail)) = rest.split_once(']') else {
        return false;
    };
    !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()) && tail == ".instanceId"
}

/// Whether `at` is `view.hiddenTargets[<n>].instanceId` (R3777001), by hand as the one above.
fn is_hidden_target_instance_id(at: &str) -> bool {
    let Some(rest) = at.strip_prefix("view.hiddenTargets[") else {
        return false;
    };
    let Some((index, tail)) = rest.split_once(']') else {
        return false;
    };
    !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()) && tail == ".instanceId"
}

/// I6 for one seat on one state: what `view_for` and `legal_actions` sent it names no card it may not
/// read (§10.8); `legal` may offer a face-down trap's bare id as a target (R177) and nothing else, and
/// the view may name that bare id in `hiddenTargets` (R3777001) only while `legal` offers it.
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
    // R2652000: less the face-down rolls the view drops for this viewer, so the two still line up.
    let withheld = crate::view_for::withheld_rolls(state, viewer);
    let raw: Vec<&GameEvent> = state
        .applied
        .iter()
        .flat_map(|entry| entry.events.iter())
        .enumerate()
        .filter(|(at, _)| !withheld.contains(at))
        .map(|(_, event)| event)
        .collect();
    let offset = raw.len() as isize - view.events.len() as isize;
    let mut events: Vec<Value> = Vec::new();
    for (at, event) in view.events.iter().enumerate() {
        let source = usize::try_from(offset + at as isize)
            .ok()
            .and_then(|index| raw.get(index).copied());
        if source.is_some_and(|source| readable_where_taken(source, viewer)) {
            continue;
        }
        let mut shown = serde_json::to_value(event).expect("a GameEvent serializes");
        if let Some(GameEvent::StarshipLaunched { pieces, .. }) = source {
            drop_launched_pieces(&mut shown, pieces);
        }
        if let Some(GameEvent::StarshipFused { def_id, .. }) = source {
            drop_fused_piece(&mut shown, def_id);
        }
        events.push(shown);
    }
    // `defs` (R243) is left out: a definition is copied only for an id the rest of the view names, and
    // its body names its ingredients and its `refs` (R279), the text of cards the viewer reads.
    let mut scanned = serde_json::to_value(view).expect("a PlayerView serializes");
    if let Some(object) = scanned.as_object_mut() {
        object.remove("defs");
        object.insert("events".to_string(), Value::Array(events));
    }

    let mut offered: IndexMap<String, String> = IndexMap::new();
    let legal_json = serde_json::to_value(legal).expect("ActionBodies serialize");
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
    let mut named: IndexMap<String, String> = IndexMap::new();
    walk(
        &scanned,
        "view",
        &mut |text, at| {
            // R177: the viewer's own prompt names a face-down trap by its id, as a bare option.
            if offered_bare.contains(text) && is_pending_option_instance_id(at) {
                return;
            }
            // R3777001: the view names the zone of a face-down card `legal` offers as a target, by its bare id.
            if bare.contains(text) && offered.contains_key(text) && is_hidden_target_instance_id(at) {
                return;
            }
            if !named.contains_key(text) {
                named.insert(text.to_string(), at.to_string());
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
    // R860: the opponent's unrevealed choices reach neither seat's view of the other side, so a
    // `choice: Some` on its entry while its record is unrevealed leaks through `secret_views`.
    if let Some(records) = &state.secrets {
        let hidden: Vec<&str> = records
            .iter()
            .filter(|secret| secret.owner != viewer && secret.revealed != Some(true))
            .map(|secret| secret.id.as_str())
            .collect();
        for entry in view.opponent.secrets.iter().flatten() {
            if entry.choice.is_some() && hidden.contains(&entry.id.as_str()) {
                found.push(format!(
                    "I6 hidden choice in the view: {viewer} is sent a choice for secret \"{}\" on turn {} (§9.1, §10.8, R860)",
                    entry.id,
                    state.turn,
                ));
            }
        }
    }
    found
}

/// I13 for one seat on one state (§10.8, R979000, R1571000, R2975000): while it may act (R979000's
/// window), each card in its hand, and each in its graveyard a permission admits, carries `playReason`
/// exactly when `legal` lists no `play` of it; no other graveyard card carries one. A seat that plays
/// only the tutorial's cards (R2533000) lists no play of any other and says nothing of it: not asked.
pub fn play_reason_violations(
    state: &GameState,
    viewer: PlayerId,
    view: &PlayerView,
    legal: &[ActionBody],
) -> Vec<String> {
    if state.result.is_some()
        || state.phase != Phase::Main
        || state.pending.is_some()
        || state.active != viewer
    {
        return Vec::new();
    }
    let listed: IndexSet<&str> = legal
        .iter()
        .filter_map(|action| match action {
            ActionBody::Play { instance_id, .. } => Some(instance_id.as_str()),
            _ => None,
        })
        .collect();
    let hand: &[CardView] = match &view.you.hand {
        HandView::Cards(cards) => cards,
        HandView::Count { .. } => &[],
    };
    let side = &state.players[viewer];
    let piles = [
        ("hand", &side.hand, hand, "R979000"),
        (
            "graveyard",
            &side.graveyard,
            view.you.graveyard.as_slice(),
            "R1571000",
        ),
    ];
    let mut found: Vec<String> = Vec::new();
    for (pile, cards, sent, ruling) in piles {
        for card in cards {
            if crate::state::barred_from_play(state, viewer, card) {
                continue;
            }
            let reason = sent
                .iter()
                .find(|shown| shown.instance_id == card.id)
                .and_then(|shown| shown.play_reason.as_deref());
            let admitted = pile == "hand" || crate::graveyard_play::playable_from_graveyard(state, card);
            let is_listed = listed.contains(card.id.as_str());
            match (admitted && !is_listed, reason) {
                (true, None) => found.push(format!(
                    "I13 silent card: {viewer}'s {pile} card {} ({}) has no play listed and no playReason on turn {} \
                     (§10.8, {ruling})",
                    card.id, card.def_id, state.turn
                )),
                (false, Some(reason)) => found.push(format!(
                    "I13 stray reason: {viewer}'s {pile} card {} ({}) carries playReason \"{reason}\" though {} on turn {} \
                     (§10.8, {ruling})",
                    card.id,
                    card.def_id,
                    if is_listed {
                        "a play of it is listed"
                    } else {
                        "no permission admits it"
                    },
                    state.turn
                )),
                _ => {}
            }
        }
    }
    found
}

/// I15 for one seat on one state (§10.8, R384, R3061000): while it may act (I13's window), each Activate
/// ability its view shows on a unit or a public backrow top of either side is `usable` exactly when
/// `legal` lists an `activate` of it, and carries `reason` exactly when it is not. A carried Unit (R446)
/// is not asked: the listing never walks one, and I8 holds what `reduce` accepts of it.
pub fn activation_reason_violations(
    state: &GameState,
    viewer: PlayerId,
    view: &PlayerView,
    legal: &[ActionBody],
) -> Vec<String> {
    if state.result.is_some()
        || state.phase != Phase::Main
        || state.pending.is_some()
        || state.active != viewer
    {
        return Vec::new();
    }
    let listed: IndexSet<(&str, &str)> = legal
        .iter()
        .filter_map(|action| match action {
            ActionBody::Activate {
                instance_id,
                ability: Some(ability),
                ..
            } => Some((instance_id.as_str(), ability.as_str())),
            _ => None,
        })
        .collect();
    let mut shown: Vec<(&str, &str, &ActivationView)> = Vec::new();
    for side in [&view.you, &view.opponent] {
        for unit in side.units.iter().flatten() {
            for activation in unit.activations.iter().flatten() {
                shown.push((unit.instance_id.as_str(), unit.def_id.as_str(), activation));
            }
        }
        for top in side.backrow.iter().flatten() {
            if let BackrowView::Public(card) = top {
                for activation in card.activations.iter().flatten() {
                    shown.push((card.instance_id.as_str(), card.def_id.as_str(), activation));
                }
            }
        }
    }
    let mut found: Vec<String> = Vec::new();
    for (instance_id, def_id, activation) in shown {
        let ability = activation.ability.as_str();
        let is_listed = listed.contains(&(instance_id, ability));
        let what = format!("{viewer}'s view of {instance_id} ({def_id}) ability \"{ability}\"");
        let turn = state.turn;
        match (activation.usable, is_listed, activation.reason.as_deref()) {
            (true, false, _) => found.push(format!(
                "I15 usable but unlisted: {what} is usable though no activate of it is listed on turn {turn} \
                 (§10.8, R384, R3061000)"
            )),
            (false, true, _) => found.push(format!(
                "I15 listed but unusable: {what} is not usable though an activate of it is listed on turn \
                 {turn} (§10.8, R384, R3061000)"
            )),
            (false, false, None) => found.push(format!(
                "I15 silent ability: {what} is not usable and has no reason on turn {turn} \
                 (§10.8, R384, R3061000)"
            )),
            (true, true, Some(reason)) => found.push(format!(
                "I15 stray reason: {what} carries reason \"{reason}\" though an activate of it is listed on \
                 turn {turn} (§10.8, R384, R3061000)"
            )),
            _ => {}
        }
    }
    found
}

/// I8's cards on one side: every card on that side's field, each pile's top and the cards dormant
/// under it in both rows, and a Unit a carrier holds (§3.2, R13, R446).
fn field_cards_of(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    let side = &state.players[player];
    let mut out: Vec<&CardInstance> = Vec::new();
    out.extend(side.units.iter().flatten().flatten());
    out.extend(side.backrow.iter().flatten());
    out.extend(side.backrow_piles.iter().flatten().flatten());
    out.extend(side.carried.iter().flatten().flatten());
    out
}

/// I8's forged activations of one card for `player` (#1574), each with the `activate` the list would hold
/// for it: every ability crossed with the combinations it declares (`play_choice_combinations`, R90),
/// spelled as `legal_actions` spells one (R1200), and each with no mode and no Tribute as the
/// `activatePower` alias too when that names the ability (`reduce::activate_card`; §10.2, R43, R384).
fn forged_activations(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
) -> Vec<(ActionBody, ActionBody)> {
    let abilities = crate::subsystems::activate::abilities_of(state, card);
    let aliased: Option<String> = crate::subsystems::hero_power::power_ability_of(state, card)
        .map(|decl| decl.id)
        .or_else(|| match abilities.as_slice() {
            [only] => Some(only.id.clone()),
            _ => None,
        });
    let units: Vec<String> = crate::zones::active_units_of(state, player)
        .into_iter()
        .filter(|unit| unit.id != card.id)
        .map(|unit| unit.id.clone())
        .collect();
    let random = crate::random_targets::targets_random(state);
    let mut out: Vec<(ActionBody, ActionBody)> = Vec::new();
    for decl in &abilities {
        let need = usize::try_from(decl.cost.and_then(|cost| cost.tribute).unwrap_or(0)).unwrap_or(0);
        let tributes = (need > 0).then(|| units.iter().take(need).cloned().collect::<Vec<String>>());
        let declared = crate::play_choices::DeclaredChoices {
            targets: decl.targets.clone(),
            modes: decl.modes.clone(),
        };
        for choice in crate::play_choices::play_choice_combinations(state, player, card, Some(&declared)) {
            let activate = ActionBody::Activate {
                instance_id: card.id.clone(),
                ability: Some(decl.id.clone()),
                targets: choice.targets.clone(),
                modes: choice.modes.clone(),
                tributes: tributes.clone(),
            };
            // R1200: while a Mayor acts the list spells an activation left with no pick as no picks at
            // all (`play_choices::without_target_picks`), and the reducer reads the two alike.
            let listed = if random && choice.targets.as_ref().is_some_and(Vec::is_empty) {
                ActionBody::Activate {
                    instance_id: card.id.clone(),
                    ability: Some(decl.id.clone()),
                    targets: None,
                    modes: choice.modes.clone(),
                    tributes: tributes.clone(),
                }
            } else {
                activate.clone()
            };
            if aliased.as_deref() == Some(decl.id.as_str()) && choice.modes.is_none() && tributes.is_none() {
                let alias = ActionBody::ActivatePower {
                    instance_id: card.id.clone(),
                    targets: choice.targets.clone(),
                };
                out.push((alias, listed.clone()));
            }
            out.push((activate, listed));
        }
    }
    out
}

/// One zone group's identity in `unlisted_accepted`: a listed play's card, X, embiggen, Equip,
/// Tributes and face-down — everything a re-sent play keeps but its zone.
type PlayZoneGroup = (
    String,
    Option<i32>,
    Option<bool>,
    Option<bool>,
    Option<Vec<String>>,
    Option<RevealAt>,
);

/// I8 for one seat: every `switchPosition`, `attack`, `activate`, `activatePower`, `trade`,
/// `marketBuy`, `marketSell` and `play` that `legal` does not hold and `reduce` accepts (§4.1, §4.2,
/// §6.3, §9.1, §10.2, §10.5; #2978, #4143). Switches, attacks and activations are built from the board
/// alone; a `trade` and a `marketSell` are forged for every hand card and a `marketBuy` for every offer
/// index in the Marketplace (#4143); plays are forged at the card level (a playable hand or graveyard
/// card no listed play names) and at the zone level (a real lane no listed play of an otherwise listed
/// card names). Empty when clean.
pub fn unlisted_accepted(state: &GameState, player: PlayerId, legal: &[ActionBody]) -> Vec<String> {
    let mut probed = state.clone();
    probed.applied = Vec::new();
    let accepts_as = |body: ActionBody, listed: &ActionBody| {
        !legal.contains(listed)
            && crate::reduce::reduce(&probed, &Action::new(body, player, PROBE_NONCE))
                .error
                .is_none()
    };
    let accepts = |body: ActionBody| {
        let listed = body.clone();
        accepts_as(body, &listed)
    };
    let field: Vec<&CardInstance> = PLAYER_IDS
        .into_iter()
        .flat_map(|side| field_cards_of(state, side))
        .collect();
    let mut found: Vec<String> = Vec::new();
    for card in &field {
        if accepts(ActionBody::SwitchPosition {
            instance_id: card.id.clone(),
        }) {
            found.push(format!(
                "I8 unlisted switch accepted: {player} may switch {} on turn {}, though legalActions does not \
                 list it (§4.1, §9.1)",
                name_of(card),
                state.turn
            ));
        }
    }
    let mut target_ids: Vec<String> = field.iter().map(|card| card.id.clone()).collect();
    target_ids.extend(
        PLAYER_IDS
            .into_iter()
            .map(|side| crate::reduce::attack_target_id(&DamageTarget::Hero { player: side })),
    );
    for attacker in field_cards_of(state, player) {
        for target_id in &target_ids {
            if accepts(ActionBody::Attack {
                attacker_id: attacker.id.clone(),
                target_id: target_id.clone(),
            }) {
                found.push(format!(
                    "I8 unlisted attack accepted: {player} may attack {target_id} with {} on turn {}, though \
                     legalActions does not list it (§4.2, §9.1)",
                    name_of(attacker),
                    state.turn
                ));
            }
        }
    }
    for card in &field {
        for (forged, listed) in forged_activations(state, player, card) {
            if accepts_as(forged.clone(), &listed) {
                found.push(format!(
                    "I8 unlisted activation accepted: {player} may send {} for {} on turn {}, though \
                     legalActions does not list it (§10.2, R384)",
                    serde_json::to_string(&forged).unwrap_or_default(),
                    name_of(card),
                    state.turn
                ));
            }
        }
    }
    // I8's forged deals (§10.2, §6.3; #4143): a `trade` and a `marketSell` for every hand card, whatever it
    // is, and a `marketBuy` for every offer index the Marketplace holds, the other seat's lots among them.
    // `why_trade_refused` and `why_market_buy_refused` are the reducer's own refusals (the limit, the
    // Credits, the stock, the phase) and the listing's, so these fire only where the reducer's acceptance
    // and the listing part ways; a `marketSell` is always refused (R4410000), so it stays silent.
    for card in &state.players[player].hand {
        if accepts(ActionBody::Trade {
            instance_id: card.id.clone(),
        }) {
            found.push(format!(
                "I8 unlisted trade accepted: {player} may trade {} on turn {}, though legalActions does not \
                 list it (§10.2, R1838000)",
                name_of(card),
                state.turn
            ));
        }
        if accepts(ActionBody::MarketSell {
            instance_id: card.id.clone(),
        }) {
            found.push(format!(
                "I8 unlisted marketSell accepted: {player} may sell {} on turn {}, though legalActions does \
                 not list it (§10.2, R1853003, R1861000)",
                name_of(card),
                state.turn
            ));
        }
    }
    let offers = state.marketplace.as_ref().map_or(0, |market| market.offers.len());
    for offer in 0..offers as i32 {
        if accepts(ActionBody::MarketBuy { offer }) {
            let lot = state
                .marketplace
                .as_ref()
                .and_then(|market| crate::marketplace::offered_to(market, player).nth(offer as usize))
                .map_or("no such offer", |(_, lot)| lot.def_id.as_str());
            found.push(format!(
                "I8 unlisted marketBuy accepted: {player} may buy offer {offer} ({lot}) on turn {}, though \
                 legalActions does not list it (§10.2, R1853003, R2250002)",
                state.turn
            ));
        }
    }
    // I8's forged plays (§10.2, §10.5; #2978): a `play` `reduce` accepts that `legal` never lists.
    // R90's bound drops choice combinations only, never a zone or a card, so forging at the card and
    // the zone level is exact. No play is acceptable with a prompt or a mulligan open or the game
    // over, so forging there would only burn reduces.
    if state.pending.is_none() && state.mulligan.is_none() && state.result.is_none() {
        let listed_plays: Vec<PlayAction> = legal.iter().filter_map(PlayAction::from_body).collect();
        let listed_ids: IndexSet<&str> = listed_plays
            .iter()
            .map(|play| play.instance_id.as_str())
            .collect();
        // Card level: each hand card, and each graveyard card a permission admits, that no listed
        // play names, sent with the default zone, the smallest X and no other choices. A card whose
        // combinations R90's bound cut is skipped: its absence from the list proves nothing.
        let side = &state.players[player];
        let mut playable: Vec<&CardInstance> = side.hand.iter().collect();
        playable.extend(
            side.graveyard
                .iter()
                .filter(|card| crate::graveyard_play::playable_from_graveyard(state, card)),
        );
        for card in playable {
            if listed_ids.contains(card.id.as_str()) {
                continue;
            }
            if crate::play_choices::hand_play_choices_truncated(state, player, card) {
                continue;
            }
            let forged = PlayAction {
                instance_id: card.id.clone(),
                zone: None,
                x: crate::play_choices::chooses_x(state, card).then_some(MIN_CHOSEN_X),
                embiggen: None,
                equip: None,
                tributes: None,
                targets: None,
                modes: None,
                plague: None,
                face_down: None,
            };
            let forged = forged.into_body();
            if accepts(forged.clone()) {
                found.push(format!(
                    "I8 unlisted play accepted: {player} may play {} for {} on turn {}, though \
                     legalActions does not list it (§10.2, §10.5)",
                    serde_json::to_string(&forged).unwrap_or_default(),
                    name_of(card),
                    state.turn
                ));
            }
        }
        // Zone level: one listed play per group sharing its card, X, embiggen, Equip, Tributes and
        // face-down, re-sent into each unit or backrow lane the group never lists. Refusals (taken,
        // Locked, the wrong row, Equip-only, face-down-only) are silent; only an acceptance the list
        // left out is a finding. A group that takes no zone is skipped, and zone `None` is never
        // forged: where the card has listed plays the reducer's default-zone pick is listed behavior.
        let mut groups: IndexMap<PlayZoneGroup, (PlayAction, Vec<ZoneChoice>)> = IndexMap::new();
        for play in &listed_plays {
            let entry = groups
                .entry((
                    play.instance_id.clone(),
                    play.x,
                    play.embiggen,
                    play.equip,
                    play.tributes.clone(),
                    play.face_down,
                ))
                .or_insert_with(|| (play.clone(), Vec::new()));
            if let Some(zone) = play.zone
                && !entry.1.contains(&zone)
            {
                entry.1.push(zone);
            }
        }
        for (template, zones) in groups.values() {
            if template.zone.is_none() {
                continue;
            }
            let card = find_instance(state, &template.instance_id);
            let lanes = (1..=UNIT_ZONES)
                .map(|lane| ZoneChoice {
                    row: Row::Units,
                    lane,
                })
                .chain((1..=BACKROW_ZONES).map(|lane| ZoneChoice {
                    row: Row::Backrow,
                    lane,
                }));
            for lane in lanes {
                if zones.contains(&lane) {
                    continue;
                }
                let forged = PlayAction {
                    zone: Some(lane),
                    ..template.clone()
                };
                let forged = forged.into_body();
                if accepts(forged.clone()) {
                    let name = card.map(name_of).unwrap_or_else(|| template.instance_id.clone());
                    found.push(format!(
                        "I8 unlisted play accepted: {player} may play {} for {} on turn {}, though \
                         legalActions does not list it (§10.2, §10.5)",
                        serde_json::to_string(&forged).unwrap_or_default(),
                        name,
                        state.turn
                    ));
                }
            }
        }
    }
    found
}

/// The listed actions I14 sends: all up to `I14_ACTION_CAP`, else that many spread evenly over the list from
/// a start that moves with the state's `rng_cursor` (read, never drawn from).
fn sampled(legal: &[ActionBody], rotate: u32) -> Vec<&ActionBody> {
    let len = legal.len();
    if len <= I14_ACTION_CAP {
        return legal.iter().collect();
    }
    let start = rotate as usize % len;
    (0..I14_ACTION_CAP)
        .map(|k| &legal[(start + k * len / I14_ACTION_CAP) % len])
        .collect()
}

/// I14 for one seat (§10.2; #3062, #4057): each action of `legal` (at most `I14_ACTION_CAP`, spread over the
/// list) sent to `reducer` on a copy of `state`; each it refuses, panics on, or leaves with a stateless
/// invariant violation on the resulting board, or with I19's, I21's or I23's against the state it was
/// reduced from (#4454, #4508, #4609), is a finding. Empty when clean.
pub fn listed_refused(
    state: &GameState,
    player: PlayerId,
    legal: &[ActionBody],
    reducer: impl Fn(&GameState, &Action) -> ReduceResult,
) -> Vec<String> {
    let mut probed = state.clone();
    probed.applied = Vec::new();
    // I19: the Dormant cards every branch begins with (#4454); I21: its Units Immune to Spells (#4508);
    // I23: its Immutable cards (#4609).
    let slumber = slumber_step(&probed);
    let immune = immune_step(&probed);
    let immutable = immutable_step(&probed);
    let mut found: Vec<String> = Vec::new();
    for body in sampled(legal, state.rng_cursor) {
        let action = Action::new(body.clone(), player, I14_NONCE);
        let sent = serde_json::to_string(body).unwrap_or_default();
        match catch_unwind(AssertUnwindSafe(|| reducer(&probed, &action))) {
            Ok(result) => {
                if let Some(error) = result.error {
                    found.push(format!(
                        "I14 listed action refused: {player} sent {sent} on turn {} (seed {}), though legalActions \
                         lists it, and reduce refused it: {error} (§10.2)",
                        state.turn, state.seed
                    ));
                } else {
                    let branch = branch_state_findings(&result.events, &result.state)
                        .into_iter()
                        .chain(slumber_reach_findings(&slumber, &result.events, &result.state))
                        .chain(immune_reach_findings(&immune, &result.events, &result.state))
                        .chain(immutable_text_findings(&immutable, &result.events, &result.state));
                    for finding in branch {
                        found.push(format!(
                            "I14 branch: after {player} sent {sent} (listed, not played) on turn {} (seed {}): {finding}",
                            state.turn, state.seed
                        ));
                    }
                }
            }
            Err(payload) => found.push(format!(
                "I14 listed action panicked: {player} sent {sent} on turn {} (seed {}), though legalActions \
                 lists it, and reduce threw: {} (§10.2)",
                state.turn,
                state.seed,
                panic_text(&*payload)
            )),
        }
    }
    found
}

/// The malformed-answer probe's variants (#1574), each named in its failure.
pub const STALE_CHOICE_ANSWER: &str = "stale-choice";
pub const OTHER_SEAT_ANSWER: &str = "other-seat";
pub const DROPPED_PICK_ANSWER: &str = "dropped-pick";
pub const DOUBLED_PICK_ANSWER: &str = "doubled-pick";
pub const UNOFFERED_PICK_ANSWER: &str = "unoffered-pick";
/// What `stale-choice` appends to the open prompt's id: a prompt that is not open.
const STALE_CHOICE_SUFFIX: &str = "-stale";
/// The pick `unoffered-pick` makes: no instance has this id, so no prompt offers it.
const UNOFFERED_INSTANCE_ID: &str = "i8-unoffered";

/// #1574: `answer`, a listed answer to the open prompt, made malformed every way
/// `prompts::why_answer_refused` refuses (§10.6, R221), each with the seat that sends it: malformed by
/// construction, never merely unlisted, since R90's bound lists only a slice of the answers. Empty with
/// no prompt open (a mulligan has its own action) or when `answer` is not an answer to it.
pub fn malformed_answers(
    state: &GameState,
    answer: &ActionBody,
) -> Vec<(&'static str, PlayerId, ActionBody)> {
    let (Some(pending), ActionBody::Answer { choice_id, selection }) = (&state.pending, answer) else {
        return Vec::new();
    };
    if pending.kind == PromptKind::Mulligan || *choice_id != pending.id {
        return Vec::new();
    }
    let holder = pending.player_id;
    let answer_with = |selection: Vec<Selection>| ActionBody::Answer {
        choice_id: choice_id.clone(),
        selection,
    };
    let mut out: Vec<(&'static str, PlayerId, ActionBody)> = vec![
        (
            STALE_CHOICE_ANSWER,
            holder,
            ActionBody::Answer {
                choice_id: format!("{choice_id}{STALE_CHOICE_SUFFIX}"),
                selection: selection.clone(),
            },
        ),
        (OTHER_SEAT_ANSWER, opponent_of(holder), answer.clone()),
    ];
    if let Ok(keep) = usize::try_from(pending.min - 1) {
        let mut dropped = selection.clone();
        dropped.truncate(keep);
        out.push((DROPPED_PICK_ANSWER, holder, answer_with(dropped)));
    }
    if let Some(first) = selection.first() {
        let mut doubled = selection.clone();
        match doubled.len() {
            1 => doubled.push(first.clone()),
            len => doubled[len - 1] = first.clone(),
        }
        out.push((DOUBLED_PICK_ANSWER, holder, answer_with(doubled)));
    }
    let unoffered = Selection::Instance {
        instance_id: UNOFFERED_INSTANCE_ID.to_string(),
    };
    let mut swapped = selection.clone();
    match swapped.first_mut() {
        Some(first) => *first = unoffered,
        None => swapped.push(unoffered),
    }
    out.push((UNOFFERED_PICK_ANSWER, holder, answer_with(swapped)));
    out
}

/// The malformed-action probe's variants (#1694), each named in its failure.
pub const UNOFFERED_TARGET_ACTION: &str = "unoffered-target";
pub const UNOFFERED_MODE_ACTION: &str = "unoffered-mode";
pub const UNOFFERED_ZONE_ACTION: &str = "unoffered-zone";
pub const NEGATIVE_X_ACTION: &str = "negative-x";
pub const UNOFFERED_TRIBUTE_ACTION: &str = "unoffered-tribute";
pub const UNOFFERED_PLAGUE_ACTION: &str = "unoffered-plague";
pub const UNOFFERED_REVEAL_ACTION: &str = "unoffered-reveal";
pub const UNOFFERED_ABILITY_ACTION: &str = "unoffered-ability";
pub const UNOFFERED_POWER_TARGET_ACTION: &str = "unoffered-power-target";
pub const UNDEALT_KEEP_ACTION: &str = "undealt-keep";
pub const ANSWERED_MULLIGAN_ACTION: &str = "answered-mulligan";
/// The mode `unoffered-mode` names: no card declares it.
const UNOFFERED_MODE: &str = "i8-unoffered-mode";
/// The ability `unoffered-ability` names: no card has it.
const UNOFFERED_ABILITY: &str = "i8-unoffered-ability";
/// The lane `unoffered-zone` names: a row's zones are numbered from 1 (§3.2), so no row has it.
const UNOFFERED_LANE: i32 = 0;
/// The X `negative-x` chooses: an X is never negative (§2.3, R348, `play_choices::why_x_refused`).
const NEGATIVE_X: i32 = -1;
/// The timing `unoffered-reveal` asks for: a card is set face-down by its Cloaked alone (R783004,
/// R1044), so `alt_play::face_down_options` offers no card this one.
const UNOFFERED_REVEAL: RevealAt = RevealAt::EndOfThisTurn;

/// `picks` with its first replaced by `unoffered`, or `unoffered` alone when there are none.
fn with_unoffered<T: Clone>(picks: Option<&[T]>, unoffered: T) -> Vec<T> {
    let mut out: Vec<T> = picks.unwrap_or_default().to_vec();
    match out.first_mut() {
        Some(first) => *first = unoffered,
        None => out.push(unoffered),
    }
    out
}

/// #1694: `chosen`, an action `legal_actions` lists `player`, with one declared choice made impossible
/// (never merely unlisted, as with `malformed_answers`), each with the seat that sends it, as
/// `play_choices::why_choices_refused` (§10.5 step 1, R81, R90, R454), `activate::why_activate_refused`
/// (R384) and `setup::why_mulligan_refused` (R265) refuse it. Empty for any other action.
pub fn malformed_actions(
    state: &GameState,
    player: PlayerId,
    chosen: &ActionBody,
) -> Vec<(&'static str, PlayerId, ActionBody)> {
    let unoffered_pick = || Selection::Instance {
        instance_id: UNOFFERED_INSTANCE_ID.to_string(),
    };
    let unoffered_id = || UNOFFERED_INSTANCE_ID.to_string();
    let mut out: Vec<(&'static str, PlayerId, ActionBody)> = Vec::new();
    if let Some(play) = PlayAction::from_body(chosen) {
        let mut push = |name: &'static str, forged: PlayAction| out.push((name, player, forged.into_body()));
        push(
            UNOFFERED_TARGET_ACTION,
            PlayAction {
                targets: Some(with_unoffered(play.targets.as_deref(), unoffered_pick())),
                ..play.clone()
            },
        );
        push(
            UNOFFERED_MODE_ACTION,
            PlayAction {
                modes: Some(with_unoffered(play.modes.as_deref(), UNOFFERED_MODE.to_string())),
                ..play.clone()
            },
        );
        push(
            UNOFFERED_ZONE_ACTION,
            PlayAction {
                zone: Some(ZoneChoice {
                    row: play.zone.map_or(Row::Units, |zone| zone.row),
                    lane: UNOFFERED_LANE,
                }),
                ..play.clone()
            },
        );
        // R43: an X a cost hook fixes is ignored whatever the play names, so only a chosen X is forged.
        if play.x.is_some() {
            push(
                NEGATIVE_X_ACTION,
                PlayAction {
                    x: Some(NEGATIVE_X),
                    ..play.clone()
                },
            );
        }
        push(
            UNOFFERED_TRIBUTE_ACTION,
            PlayAction {
                tributes: Some(with_unoffered(play.tributes.as_deref(), unoffered_id())),
                ..play.clone()
            },
        );
        push(
            UNOFFERED_PLAGUE_ACTION,
            PlayAction {
                plague: Some(PlagueSpend {
                    from: unoffered_id(),
                    tokens: MIN_PLAGUE_PAYMENT,
                }),
                ..play.clone()
            },
        );
        push(
            UNOFFERED_REVEAL_ACTION,
            PlayAction {
                face_down: Some(UNOFFERED_REVEAL),
                ..play
            },
        );
    }
    if let Some(activate) = ActivateAction::from_body(chosen) {
        let mut push =
            |name: &'static str, forged: ActivateAction| out.push((name, player, forged.into_body()));
        push(
            UNOFFERED_ABILITY_ACTION,
            ActivateAction {
                ability: Some(UNOFFERED_ABILITY.to_string()),
                ..activate.clone()
            },
        );
        push(
            UNOFFERED_TARGET_ACTION,
            ActivateAction {
                targets: Some(with_unoffered(activate.targets.as_deref(), unoffered_pick())),
                ..activate.clone()
            },
        );
        push(
            UNOFFERED_MODE_ACTION,
            ActivateAction {
                modes: Some(with_unoffered(
                    activate.modes.as_deref(),
                    UNOFFERED_MODE.to_string(),
                )),
                ..activate.clone()
            },
        );
        push(
            UNOFFERED_TRIBUTE_ACTION,
            ActivateAction {
                tributes: Some(with_unoffered(activate.tributes.as_deref(), unoffered_id())),
                ..activate.clone()
            },
        );
        out.push((
            UNOFFERED_POWER_TARGET_ACTION,
            player,
            ActionBody::ActivatePower {
                instance_id: activate.instance_id.clone(),
                targets: Some(with_unoffered(activate.targets.as_deref(), unoffered_pick())),
            },
        ));
    }
    if let ActionBody::Mulligan { keep } = chosen {
        out.push((
            UNDEALT_KEEP_ACTION,
            player,
            ActionBody::Mulligan {
                keep: with_unoffered(Some(keep.as_slice()), unoffered_id()),
            },
        ));
        let other = opponent_of(player);
        if state.mulligan.is_some() && !crate::setup::mulligan_owed(state).contains(&other) {
            out.push((
                ANSWERED_MULLIGAN_ACTION,
                other,
                ActionBody::Mulligan { keep: Vec::new() },
            ));
        }
    }
    out
}

/// The prompt-blocked probe's variants (#2829), each named in its failure.
pub const PROMPT_OPEN_PLAY_ACTION: &str = "prompt-open-play";
pub const PROMPT_OPEN_END_TURN_ACTION: &str = "prompt-open-end-turn";

/// #2829: the seat's non-answer action while a non-mulligan prompt is open, with the seat that
/// sends it, as `reduce` refuses it (§9.3, §10.6, R79): a `Play` naming the seat's first held card
/// when it holds one, else `EndTurn`. `None` with no prompt open or a mulligan one (R265-R266,
/// whose own path `malformed_actions` covers).
pub fn prompt_blocked_action(
    state: &GameState,
    player: PlayerId,
) -> Option<(&'static str, PlayerId, ActionBody)> {
    let pending = state.pending.as_ref()?;
    if pending.kind == PromptKind::Mulligan {
        return None;
    }
    match state.players[player].hand.first() {
        Some(held) => Some((
            PROMPT_OPEN_PLAY_ACTION,
            player,
            ActionBody::Play {
                instance_id: held.id.clone(),
                zone: None,
                x: None,
                embiggen: None,
                equip: None,
                tributes: None,
                targets: None,
                modes: None,
                plague: None,
                face_down: None,
            },
        )),
        None => Some((PROMPT_OPEN_END_TURN_ACTION, player, ActionBody::EndTurn)),
    }
}

/// The turn and stint of an instance's latest declared attack, and how many it has declared in them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AttackMark {
    turn: i32,
    stint: i32,
    count: i32,
}

/// The turn and stint of an instance's latest chosen position switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SwitchMark {
    turn: i32,
    stint: i32,
}

/// I12's pre-reduction snapshot: an attack can remove its Unit before `attackDeclared`, while Deft
/// must be read through the layers as the chosen action saw it (R49).
#[derive(Clone, Debug, PartialEq, Eq)]
struct ChosenExertion {
    instance_id: String,
    def_id: String,
    kind: crate::combat::ExertionKind,
    deft: bool,
}

/// B5 E35, R1869002: the fresh exertion a kill gave its attacker in one stint on one turn: `at` is how
/// many attacks it had declared there then (none once a settling took it back), `before` the credit it
/// held there before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct KillCredit {
    turn: i32,
    stint: i32,
    at: Option<i32>,
    before: Option<i32>,
}

/// R424: the latest attack, and whether its attacker destroyed its target (R42). A forced one (R53) readies
/// no transform, but its kill may earn a kill credit (R1869002). `answered`: R3139004's credit for an
/// attack on a Unit that killed nothing has been given.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ShadowAttack {
    attacker_id: String,
    target_id: String,
    killed: bool,
    forced: bool,
    answered: bool,
}

/// I18 (R3100000, #4111): the forecast entry of the attack `before` saw chosen, with what it was told
/// of the step: the turn and both cards' labels are read before the attack, since it may remove the
/// attacker, and `foreseeable` is `foreseeable_attack` on the board it was chosen on.
#[derive(Clone, Debug)]
struct KeptForecast {
    entry: crate::query::AttackForecast,
    turn: i32,
    attacker: String,
    target: String,
    foreseeable: bool,
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

/// I20: every card `field_cards_of` finds on either side, with the damage it stores.
fn field_damage(state: &GameState) -> IndexMap<String, i32> {
    PLAYER_IDS
        .into_iter()
        .flat_map(|player| field_cards_of(state, player))
        .map(|card| (card.id.clone(), card.damage))
        .collect()
}

/// R1829000: the cards dormant beneath the top of a unit pile or a backrow pile (§3.2, R13).
fn dormant_cards(state: &GameState) -> IndexSet<String> {
    let mut out = IndexSet::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        out.extend(
            side.units
                .iter()
                .flatten()
                .flat_map(|pile| pile.iter().skip(1))
                .map(|card| card.id.clone()),
        );
        out.extend(
            side.backrow_piles
                .iter()
                .flatten()
                .flatten()
                .map(|card| card.id.clone()),
        );
    }
    out
}

/// A caught panic's message.
fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "a panic with no message".to_string()
    }
}

fn name_of(card: &CardInstance) -> String {
    format!("{} ({}, {}'s)", card.id, card.def_id, card.controller)
}

/// I18: a card as its message names it, `<id> (<def id> "<name>")`, or a hero's `hero-<player>` id.
/// The name is the catalog's, `?` for a definition no catalog lists (`def_of` would panic).
fn card_label(state: &GameState, id: &str) -> String {
    match find_instance(state, id) {
        Some(card) => {
            let name =
                crate::catalog::find_def(Some(state), &card.def_id).map_or("?", |def| def.name.as_str());
            format!("{id} ({} \"{name}\")", card.def_id)
        }
        None => id.to_string(),
    }
}

/// I18 (#4111): whether the board `attacker_id`'s attack on `target_id` is chosen on lets its forecast
/// foretell all of it. Each `false` is a source R3100000's forecast leaves out, since the board does
/// not show what it will do:
/// - no attacker card;
/// - the attacker, a Unit target or a unit its Cleave strikes (R4290000) in a split or shared damage
///   chain (§4.4 steps 0b and 8b, R3554007), which R4032000 leaves unprojected;
/// - the attacker's "When this attacks" (R2285000) or "After this attacks" (R2285001), or the Unit
///   target's "After this is attacked" (MD-C26);
/// - a card acting for the attacker's side with "After one of your Units attacks" (R2199000), or for
///   the target's side with "When one of your Units is attacked" (R1849008). `hook_marks.rs` marks none
///   of these attack hooks with a `hookResolving`, so `plain_combat` cannot see them run;
/// - for a Unit target, a card acting for its side that reflects an attacker's strike back onto it
///   (Stinkyscape #104 Reflective Plating's Radiant face, §4.4 step 8a, R1849003): the forecast's
///   `taken` is the strike back alone (fuzz seed 1298), until the forecast projects it;
/// - for a Unit target, a card acting for its side that reflects what it takes onto the attacker's hero
///   (Reflective Plating's base face, R1849003) while the attacker's side has a damage pool to soak it
///   (Stinkyscape+ #42 Force Projector, R3550004): the reflection spends the pool before the strike back
///   lands, which the forecast reads against the pool as the board shows it (R4221004; fuzz seed 9305);
/// - the attacker, a Unit target or a unit its Cleave strikes (R4290000) with cards dormant beneath it
///   in its pile (R1829000; its count is shown, R13) or holding one (Stinkyscape #135.1 Sack's Unit,
///   R1850003): its leaving the field brings them in at the state check after the combat, where an aura
///   among them may exile a combatant (Stinkyscape #115 Scornboi), and the forecast foretells nothing
///   past the combat (R4221002; fuzz seeds 4897 and 6161);
/// - a card acting for either side with a "would die" replacement (Classic #14 Shadowstep's Radiant
///   face, Stinkyscape #138 Pocket Peptides; §4.5 step 1, R4221000): it answers the death check after
///   the combat, a flicker or its own `then`, with no `trapFired` or `hookResolving` for `plain_combat`
///   to see, and the forecast projects no replacement (fuzz seed 9561);
/// - a Unit the state check after the combat takes off, as the forecast settles it (R4379000; a Cleave
///   neighbour, §4.4 step 10, a sweep's other hit, R3554008, or one an aura no longer holds up), with a
///   card dormant beneath it or held by it (R4221002's reason, R4318000): its leaving brings that card
///   in, which the forecast is not shown;
/// - for a Unit target, an attacker that reflects what it takes onto the enemy hero (Stinkyscape #104
///   Reflective Plating, either face, R1849003): its strike back is dealt again to the target's hero
///   after any Trample excess and may end the game, and the forecast projects no reflection (R4555001;
///   fuzz seed 4338).
fn foreseeable_attack(state: &GameState, attacker_id: &str, target_id: &str) -> bool {
    let Some(attacker) = find_instance(state, attacker_id) else {
        return false;
    };
    let (target, target_side) = match hero_seat_of(target_id) {
        Some(seat) => (None, seat),
        None => match find_instance(state, target_id) {
            Some(unit) => (Some(unit), unit.controller),
            None => return false,
        },
    };
    // R4290000: a Cleave attack's other units are judged too, so they are skipped as its target is.
    let struck: Vec<CardInstance> = target.map_or_else(Vec::new, |unit| {
        crate::query::cleave_struck(state, attacker, unit)
    });
    let chained = |card: &CardInstance| card.split_chain.is_some() || card.shared_chain.is_some();
    if chained(attacker) || struck.iter().any(chained) {
        return false;
    }
    let script = crate::scripts::script_of(state, attacker);
    if script.when_attacks.is_some() || script.after_attack.is_some() {
        return false;
    }
    if target.is_some_and(|unit| crate::scripts::script_of(state, unit).after_attacked.is_some()) {
        return false;
    }
    let attacking_side = crate::damage::acting_texts_of(state, attacker.controller);
    if attacking_side
        .iter()
        .any(|card| crate::scripts::script_of(state, card).ally_attack.is_some())
    {
        return false;
    }
    let defending_side = crate::damage::acting_texts_of(state, target_side);
    if defending_side
        .iter()
        .any(|card| crate::scripts::script_of(state, card).ally_attacked.is_some())
    {
        return false;
    }
    let takes_deaths = |card: &CardInstance| {
        crate::scripts::script_of(state, card)
            .replacements
            .iter()
            .any(|def| def.on == crate::script::ReplacementMoment::WouldDie)
    };
    if attacking_side.iter().chain(&defending_side).any(takes_deaths) {
        return false;
    }
    let brings_in = |card: &CardInstance| {
        crate::zones::slot_of(state, card)
            .is_some_and(|slot| !crate::zones::beneath_at(state, slot).is_empty())
            || !crate::query::held_beneath(state, card).is_empty()
    };
    if brings_in(attacker) || struck.iter().any(brings_in) {
        return false;
    }
    let Some(aimed) = crate::combat::attack_target_of(state, target_id) else {
        return false;
    };
    let settled = crate::query::attack_deaths(state, attacker, &aimed);
    if settled
        .gone()
        .filter_map(|id| find_instance(state, id))
        .any(brings_in)
    {
        return false;
    }
    // R4555001: an attacker that reflects what it takes onto the enemy hero (Reflective Plating, R1849003).
    if target.is_some() && crate::scripts::flags_of(state, attacker).reflects_damage == Some(true) {
        return false;
    }
    let pool_soaks_reflection = crate::query::damage_pool_of(state, attacker.controller, false) > 0;
    !(target.is_some()
        && defending_side.iter().any(|card| {
            let flags = crate::scripts::flags_of(state, card);
            flags.reflects_to_attacker == Some(true)
                || (pool_soaks_reflection && flags.reflects_damage == Some(true))
        }))
}

/// I18 (#4111): whether one step's `events`, and the `state` it left, were `entry`'s attack and nothing
/// the forecast does not foretell. All of these hold:
/// - no prompt is open after it (R3477000: an answer is still to come);
/// - exactly one `attackDeclared`, with the entry's ids, not forced (R53) and no substitute's (R1202);
/// - no `hookResolving` (R2783000: a hook ran), `trapFired` (R3477000), `attackCancelled` or
///   `redirected` (of the attack, R1122, or of a hit, §4.4 steps 0a and 4a, which R4032000 leaves
///   unprojected);
/// - no `turnEnded`, `turnAutoEnded` or `turnCutShort`: the next turn's hits would read as the
///   attack's (R82).
fn plain_combat(events: &[GameEvent], state: &GameState, entry: &crate::query::AttackForecast) -> bool {
    if state.pending.is_some() {
        return false;
    }
    let mut declared = 0;
    for event in events {
        match event {
            GameEvent::AttackDeclared {
                attacker_id,
                target_id,
                forced,
                instead_of,
            } => {
                if *attacker_id != entry.attacker_id
                    || *target_id != entry.target_id
                    || *forced
                    || instead_of.is_some()
                {
                    return false;
                }
                declared += 1;
            }
            GameEvent::HookResolving { .. }
            | GameEvent::TrapFired { .. }
            | GameEvent::AttackCancelled { .. }
            | GameEvent::Redirected { .. }
            | GameEvent::TurnEnded { .. }
            | GameEvent::TurnAutoEnded { .. }
            | GameEvent::TurnCutShort { .. } => return false,
            _ => {}
        }
    }
    declared == 1
}

/// The monitor, made by `create_invariant_monitor`: the shadow of this file's header and the checks
/// run against it. `before` is I1, I3, I12's chosen-action snapshot, I18's kept forecast, I19's
/// Dormant cards, I21's immune Units and I22's open record, `after` feeds one action's events into the
/// shadow and runs I2, I4, I5, I7, I9, I10, I11, I12, I16, I17, I18, I19, I20, I21 and I22's recorded
/// draws, `hidden` is I6, I13, I15 and I22's listed targets for both seats and I22's open prompt,
/// `unlisted` is I8 and `listed` is I14 (I19 and I21 on each branch).
#[derive(Clone, Debug)]
pub struct InvariantMonitor {
    turn: i32,
    /// The player whose turn it is, from `turnStarted`: R1062's start-of-turn animation is theirs.
    active: PlayerId,
    entered: IndexMap<String, i32>,
    stint: IndexMap<String, i32>,
    last_attack: IndexMap<String, AttackMark>,
    last_switch: IndexMap<String, SwitchMark>,
    chosen_exertion: Option<ChosenExertion>,
    /// R636: the stint in which each instance was last seen with Windfury. Read from the states between
    /// actions and after each one, so a unit that dies on its second attack was seen with it before —
    /// and from `keywordGranted` events and a Buff's keyword row (`upgraded`, R386) inside the action, so
    /// a unit given Windfury mid-action is seen before its declarations: R44's AI turn can summon a
    /// Conjure token and fight with it twice inside one outer action, where no between-action state ever
    /// holds it, or Buff an attacker My Pawn stopped into Windfury and attack with it again (R636).
    windfury: IndexMap<String, i32>,
    /// R636: the cards off the field that carry Windfury as the action begins (`layers::card_keywords`:
    /// the face, its tuning, a grant), read by `before`. One that enters with it has it in the stint it
    /// enters, so it may declare twice inside one action, as the autopilot's whole turn does (R1183002).
    carried_windfury: IndexSet<String>,
    /// R636, R781004: the units `note_drawn_windfury` read as maybe Windfury, whose drawn keywords a
    /// change of control keeps (the card stays on the field).
    drawn_windfury: IndexSet<String>,
    /// R3543003: the stint in which each field unit was seen with Frenzy as the action began, read by
    /// `before`. A kill inside the action is answered by the Frenzy its attacker had then, though the
    /// state the action leaves no longer shows it: the autopilot's whole turn kills, attacks again and
    /// loses the attacker to a trap's exile or a counter-hit, which resets the keywords a Drone Corvette
    /// drew (R78, R781004; fuzz seeds 8022 and 7429, issue #3800).
    frenzy: IndexMap<String, i32>,
    /// The cards that fired as traps this turn: one animates as its firing's last step, not at the
    /// start of the turn, and that is an entry even on its controller's turn (R383, R1062).
    fired: IndexSet<String>,
    readied: IndexSet<String>,
    /// B5 E35, R1869002: each instance's latest kill credit.
    kill_credits: IndexMap<String, KillCredit>,
    /// R1869002: the attackers credited since the state last settled whose killing attack is still their
    /// latest, for the state an action leaves with nothing owed to settle.
    unsettled_credits: IndexSet<String>,
    /// R424: the latest attack, and whether its attacker destroyed its target (R42).
    attack: Option<ShadowAttack>,
    /// R1203: the Primes' attacks held behind their joiners, outermost first: each joiner declares a
    /// forced attack on the Prime's target, and the Prime's own combat follows with no declaration, so its
    /// kill is its own and not the latest joiner's (fuzz seed 867).
    behind_joiners: Vec<ShadowAttack>,
    /// R424, R1869002: whether the last action left a prompt or work owed, so a combat it declared may
    /// still be resolving (an ambush's Discovers, S #22) and its attack is kept for the next action.
    attack_open: bool,
    /// I9: `state.resets` as the last state `after` read left it; a move is a Glitch's reset (R676).
    resets: Option<i32>,
    /// I9: each seat's fatigue draws as the stream counted them (§2.4, R3, R315): the start state's
    /// `fatigue_count`, one more per `fatigue` event, and the new game's count after a Glitch's reset (R676).
    fatigue: PerPlayer<i32>,
    /// I9: each seat's hero health as the stream's events moved it (§10.3): the start state's, less each
    /// `damage` on the hero and `healthLost`, plus each `healed`, set by `healthSet`, swapped by `swapped`
    /// (health), held to the seat's hero floor as `duos_life` holds it (R2201000); a Glitch's reset (R676)
    /// starts it again from the state it left.
    health: PerPlayer<i32>,
    /// I20: each field card's stored damage as the stream's `damage` and `healed` events moved it (§4.4,
    /// §10.3): 0 on arrival, plus each `damage` on it, less each `healed`; `counted_damage` re-anchors it
    /// where a lawful writer moves the field with no such event and drops a card that left the field.
    unit_damage: IndexMap<String, i32>,
    /// R1829000: the cards dormant beneath a pile's top, either row, in the last state read.
    dormant: IndexSet<String>,
    /// I18: whether this monitor holds attacks to their forecast; `new` sets it, and the Duos monitor
    /// clears it on its duels (#4111).
    judge_forecast: bool,
    /// I18: the forecast of the attack `before` saw chosen, for `after` to judge.
    forecast_kept: Option<KeptForecast>,
    /// I18: how many attacks `after` held to their forecast, so a filter that judges nothing shows.
    forecast_judged: usize,
    /// I19: the Dormant cards the step began with, noted by `before` (or the Duos monitor's for the duel
    /// not acting) and judged by `after`, which takes them: an `after` with no `before` judges none.
    slumber_step: Option<SlumberStep>,
    /// I21: the Units Immune to Spells the step began with, noted and taken as `slumber_step` is.
    immune_step: Option<ImmuneStep>,
    /// I23: the Immutable cards the step began with, noted and taken as `slumber_step` is.
    immutable_step: Option<ImmutableStep>,
}

/// R781004: whether `later`, the rest of an action's events, summons `id` again (back through Reborn): an
/// arrival whose fresh draw is the one the state the action leaves holds.
fn summoned_later(id: &str, later: &[GameEvent]) -> bool {
    later
        .iter()
        .any(|event| matches!(event, GameEvent::Summoned { instance_id, .. } if instance_id == id))
}

pub fn create_invariant_monitor(start: &GameState) -> InvariantMonitor {
    InvariantMonitor::new(start)
}

impl InvariantMonitor {
    /// A shadow that starts on `start`'s turn with nothing entered.
    pub fn new(start: &GameState) -> InvariantMonitor {
        InvariantMonitor {
            turn: start.turn,
            active: start.active,
            entered: IndexMap::new(),
            stint: IndexMap::new(),
            last_attack: IndexMap::new(),
            last_switch: IndexMap::new(),
            chosen_exertion: None,
            windfury: IndexMap::new(),
            carried_windfury: IndexSet::new(),
            frenzy: IndexMap::new(),
            drawn_windfury: IndexSet::new(),
            fired: IndexSet::new(),
            readied: IndexSet::new(),
            kill_credits: IndexMap::new(),
            unsettled_credits: IndexSet::new(),
            attack: None,
            behind_joiners: Vec::new(),
            attack_open: false,
            resets: start.resets,
            fatigue: start.players.map(|_, side| side.fatigue_count),
            health: start.players.map(|_, side| side.hero.health),
            unit_damage: field_damage(start),
            dormant: dormant_cards(start),
            judge_forecast: true,
            forecast_kept: None,
            forecast_judged: 0,
            slumber_step: None,
            immune_step: None,
            immutable_step: None,
        }
    }

    /// I19: the Dormant cards `state` holds as a step begins on it, for `after` to judge.
    fn note_slumber(&mut self, state: &GameState) {
        self.slumber_step = Some(slumber_step(state));
    }

    /// I21: the Units Immune to Spells `state` holds as a step begins on it, for `after` to judge.
    fn note_immune(&mut self, state: &GameState) {
        self.immune_step = Some(immune_step(state));
    }

    /// I23: the Immutable cards `state` holds as a step begins on it, for `after` to judge.
    fn note_immutable(&mut self, state: &GameState) {
        self.immutable_step = Some(immutable_step(state));
    }

    /// I18: how many attacks this monitor held to their forecast (#4111).
    pub fn forecast_judged(&self) -> usize {
        self.forecast_judged
    }

    fn stint_of(&self, id: &str) -> i32 {
        self.stint.get(id).copied().unwrap_or(0)
    }

    fn note_windfury(&mut self, state: &GameState) {
        for card in field_cards(state) {
            if crate::layers::unit_has(state, card, KeywordKind::Windfury) {
                let stint = self.stint_of(&card.id);
                self.windfury.insert(card.id.clone(), stint);
            }
        }
    }

    /// R3543003: the field units with Frenzy as the action begins, each in its stint (`frenzy`).
    fn note_frenzy(&mut self, state: &GameState) {
        self.frenzy.clear();
        for card in field_cards(state) {
            if crate::layers::unit_has(state, card, KeywordKind::Frenzy) {
                let stint = self.stint_of(&card.id);
                self.frenzy.insert(card.id.clone(), stint);
            }
        }
    }

    /// R636, R781004: a unit whose text draws its keywords as it arrives or fuses (M #65 Keymaster
    /// Keenus' Aura, the Stinkyscape Drones') keeps them in its memory, which leaving the field resets
    /// (R78). One gone from the field by the action's end leaves no state to say whether it drew
    /// Windfury, so its attacks in this stint count as Windfury's (Keenus onto C+ #26: Duos seed 679).
    /// So does one the field holds at the end in a later stint (`back_later`: it is summoned again
    /// later in the action's events, Reborn's return among them), whose new draw says nothing of the
    /// stint before it (a Drone Corvette under Stinkyscape #47's autopilot: handicapped fuzz seed 7089).
    fn note_drawn_windfury(&mut self, id: &str, def_id: &str, state: &GameState, back_later: bool) {
        let draws = match find_instance(state, id) {
            // On the field at the end, its keywords are read there (`note_entry_windfury`), unless that
            // is a later stint's body.
            Some(card) if card.zone.z() == ZoneName::Field => {
                back_later
                    && crate::scripts::script_of(state, card)
                        .conditional_keywords
                        .is_some()
            }
            Some(card) => crate::scripts::script_of(state, card)
                .conditional_keywords
                .is_some(),
            None => crate::scripts::script_of(state, def_id)
                .base
                .conditional_keywords
                .is_some(),
        };
        if draws {
            let stint = self.stint_of(id);
            self.windfury.insert(id.to_string(), stint);
            self.drawn_windfury.insert(id.to_string());
        }
    }

    /// R636, R3778000: a change that raises such a unit's number on the field draws the rest of its list
    /// as it resolves (`effects::buff::follow_combat_keywords`), which no `keywordGranted` reports. On
    /// the field at the end in the same stint, its list, never shortened in a stay, says whether it drew
    /// Windfury; gone, or back in a later stint, it reads as `note_drawn_windfury` reads it.
    fn note_topped_up_windfury(&mut self, id: &str, def_id: &str, state: &GameState, back_later: bool) {
        match find_instance(state, id) {
            Some(card) if card.zone.z() == ZoneName::Field && !back_later => {
                let drawn = crate::effects::drawn_combat_keywords(card, None);
                if has_keyword(&drawn, KeywordKind::Windfury) {
                    let stint = self.stint_of(id);
                    self.windfury.insert(id.to_string(), stint);
                }
            }
            _ => self.note_drawn_windfury(id, def_id, state, back_later),
        }
    }

    /// R636: a unit that just entered and carries Windfury, as it was before the action
    /// (`carried_windfury`) or as the action left it, has it in this stint. One gone from the field by
    /// the action's end is also read standing in its controller's unit row, so an aura still acting
    /// there (S+ #62 Overdrive Projector's "All Units gain Windfury") gives it the Windfury it had while
    /// it stood: played, swinging twice and destroyed in one autopilot turn (fuzz seed 160).
    fn note_entry_windfury(&mut self, id: &str, state: &GameState) {
        let left_with = find_instance(state, id).is_some_and(|card| {
            let keywords = if card.zone.z() == ZoneName::Field {
                crate::layers::keywords_of(state, card)
            } else {
                let mut standing = card.clone();
                standing.zone = Zone::Field {
                    player: card.controller,
                    row: Row::Units,
                    lane: 1,
                };
                let mut keywords = crate::layers::card_keywords(state, card);
                // R89: as a dead card's snapshot is read, no pile's top holds the auras off it.
                keywords.extend(crate::layers::unit_view_as_it_died(state, &standing).keywords);
                keywords
            };
            has_keyword(&keywords, KeywordKind::Windfury)
        });
        if left_with || self.carried_windfury.contains(id) {
            let stint = self.stint_of(id);
            self.windfury.insert(id.to_string(), stint);
        }
    }

    /// R424: a readied stint whose body carries no `summonedTurn`, as the readying transform leaves it;
    /// R1869002: or `mayAttackAgain` does, for a stint holding a kill credit no state has settled yet.
    fn is_readied(&self, card: &CardInstance) -> bool {
        // R1869002: a kill credit still unsettled (a prompt left open, a steal before its `afterAttack`)
        // may have readied it; `settle_kill_credits` takes it back if its attack stayed spent.
        (self.readied.contains(&card.id)
            || self.credited_at(&card.id).is_some()
            || self.unsettled_credits.contains(&card.id))
            && card.summoned_turn.is_none()
    }

    /// B5 E35, R1869002: the attacks the instance had declared in its current stint this turn when its
    /// latest kill credit there freshened its exertion, if it holds one.
    fn credited_at(&self, id: &str) -> Option<i32> {
        self.kill_credits
            .get(id)
            .filter(|credit| credit.turn == self.turn && credit.stint == self.stint_of(id))
            .and_then(|credit| credit.at)
    }

    /// R1869002: a kill credit for the instance at the attacks it has declared in its current stint this
    /// turn, the one it held there kept for a settling that takes the new one back.
    fn credit_kill(&mut self, id: &str) {
        let stint = self.stint_of(id);
        let declared = self
            .last_attack
            .get(id)
            .filter(|mark| mark.turn == self.turn && mark.stint == stint)
            .map_or(0, |mark| mark.count);
        let credit = KillCredit {
            turn: self.turn,
            stint,
            at: Some(declared),
            before: self.credited_at(id),
        };
        self.kill_credits.insert(id.to_string(), credit);
        self.unsettled_credits.insert(id.to_string());
    }

    /// R1869002: a credited attacker the action leaves on the field, with every exertion half the
    /// shadow saw spent still spent, was readied by nothing: its credit is taken back. This matters for
    /// a forced attack (R53), which spends no attack exertion itself: only a reset of an earlier attack
    /// or switch proves `mayAttackAgain` ran. One it readied is `readied` for the rest of its stint, as
    /// `mayAttackAgain` leaves it with no `summonedTurn`. Settled once nothing is owed, since an
    /// `afterAttack` a prompt paused runs in a later action (R113).
    fn settle_kill_credits(&mut self, state: &GameState) {
        if state.pending.is_some() || !state.work.is_empty() {
            return;
        }
        for id in std::mem::take(&mut self.unsettled_credits) {
            let Some(credit) = self.kill_credits.get(&id).copied() else {
                continue;
            };
            let attack_was_spent = credit.at.unwrap_or(0) > credit.before.unwrap_or(0);
            let switch_was_spent = self
                .last_switch
                .get(&id)
                .is_some_and(|mark| mark.turn == credit.turn && mark.stint == credit.stint);
            let refreshed = find_instance(state, &id).is_some_and(|card| {
                card.zone.z() == ZoneName::Field
                    && ((attack_was_spent && !card.exertion.attacked)
                        || (switch_was_spent && !card.exertion.switched))
            });
            if refreshed {
                self.readied.insert(id.clone());
                self.last_switch.shift_remove(&id);
            } else if let Some(credit) = self.kill_credits.get_mut(&id) {
                credit.at = credit.before;
            }
        }
    }

    pub fn enter(&mut self, id: &str) {
        self.entered.insert(id.to_string(), self.turn);
        let stint = self.stint_of(id) + 1;
        self.stint.insert(id.to_string(), stint);
        self.readied.shift_remove(id);
    }

    /// R1062: an "Animated on your turn" card whose latest entry was on an earlier turn enters nothing as
    /// it animates at its controller's start of turn; on the other player's `side` (R1126's Cane) it is
    /// R383's entry. Read off the shadow and the keyword as the animation had it, never `summonedTurn`:
    /// the first change in `later`, the action's events after it, says what it was (fuzz seed 2443).
    fn on_its_side_before_this_turn(
        &self,
        id: &str,
        side: PlayerId,
        state: &GameState,
        later: &[GameEvent],
    ) -> bool {
        if side != self.active {
            return false;
        }
        let (Some(at), Some(card)) = (self.entered.get(id).copied(), find_instance(state, id)) else {
            return false;
        };
        let changed = later.iter().find_map(|event| match event {
            GameEvent::Degraded {
                instance_id,
                change: TuningChange::Keyword { keyword, added, also },
                ..
            }
            | GameEvent::Upgraded {
                instance_id,
                change: TuningChange::Keyword { keyword, added, also },
                ..
            } if instance_id == id
                && std::iter::once(keyword)
                    .chain(also.iter().flatten())
                    .any(|keyword| keyword.kind() == KeywordKind::AnimatedOnYourTurn) =>
            {
                Some(!*added)
            }
            GameEvent::KeywordGranted {
                instance_id,
                keyword,
                lost,
            } if instance_id == id && keyword.kind() == KeywordKind::AnimatedOnYourTurn => {
                Some(lost.is_some())
            }
            _ => None,
        });
        let had =
            changed.unwrap_or_else(|| crate::layers::unit_has(state, card, KeywordKind::AnimatedOnYourTurn));
        at < self.turn && had
    }

    /// R1829000: the entries one action made by uncovering cards in their Stack piles, which no event reports
    /// (R212): a card dormant in the last state read that acts now (not on a side a `rolledBack` restored,
    /// R566, nor a fusion's kept card, R77, R1086), on the turn `summonedTurn` names from `began_on`, else
    /// the last. One uncovered unseen (dormant, or in a zone R14 names) takes `summonedTurn`'s word only for
    /// a turn of the action. So does a card the action's own `summoned` or `controlChanged` named, wherever it
    /// stands now: the hole's pull at a turn's start moves a pile one lane and no event says so (R3554001),
    /// so a Drone Battleship's layer summoned beneath its Shield, uncovered on a later turn of the one action
    /// and pulled on, stands in no zone an event named (fuzz handicap seed 6708, issue #3800).
    fn note_uncoverings(&mut self, events: &[GameEvent], state: &GameState, began_on: i32) {
        let mut restored: Vec<PlayerId> = Vec::new();
        let mut landed: Vec<(PlayerId, Row, i32)> = Vec::new();
        let mut arrived: Vec<&str> = Vec::new();
        let mut kept: Vec<&str> = Vec::new();
        // §8 #52: the rotating player is the one whose card played (`RotationArgs.perspective`).
        let mut playing: Option<PlayerId> = None;
        for event in events {
            match event {
                GameEvent::CardPlayed { player, .. } => playing = Some(*player),
                // R14: the rings turn with every pile on them, so a zone a play covered is one step on.
                GameEvent::Rotated { direction } => {
                    if let Some(perspective) = playing {
                        for (player, row, lane) in landed.iter_mut() {
                            let from = crate::zones::ZoneSlot {
                                player: *player,
                                row: *row,
                                lane: *lane,
                            };
                            let to = crate::zones::ring_neighbor(from, *direction, perspective);
                            (*player, *row, *lane) = (to.player, to.row, to.lane);
                        }
                    }
                }
                GameEvent::RolledBack { sides, .. } => restored.extend(sides.iter().copied()),
                GameEvent::Summoned {
                    player,
                    row,
                    lane,
                    instance_id,
                    ..
                } => {
                    landed.push((*player, *row, *lane));
                    arrived.push(instance_id);
                }
                GameEvent::ControlChanged {
                    controller,
                    row,
                    lane,
                    instance_id,
                    ..
                } => {
                    landed.push((*controller, *row, *lane));
                    arrived.push(instance_id);
                }
                GameEvent::Animated {
                    player, unit_lane, ..
                } => landed.push((*player, Row::Units, *unit_lane)),
                GameEvent::Deanimated {
                    player, backrow_lane, ..
                } => landed.push((*player, Row::Backrow, *backrow_lane)),
                GameEvent::CardAnnounced {
                    player,
                    row: Some(row),
                    lane: Some(lane),
                    ..
                } => landed.push((*player, *row, *lane)),
                GameEvent::Fused {
                    result_instance_id, ..
                } => kept.push(result_instance_id),
                _ => {}
            }
        }
        let now = self.turn;
        for card in field_cards(state) {
            let Zone::Field { player, row, lane } = card.zone else {
                continue;
            };
            // I3 reports a card with no entry, which only an event gives it.
            if !self.entered.contains_key(&card.id) {
                continue;
            }
            let was_dormant = self.dormant.contains(&card.id);
            let uncovered = was_dormant
                && !restored.contains(&player)
                && !kept.contains(&card.id.as_str())
                && crate::zones::acts_on_field(state, card);
            let may_have_been =
                was_dormant || landed.contains(&(player, row, lane)) || arrived.contains(&card.id.as_str());
            let during = card.summoned_turn.filter(|turn| (began_on..=now).contains(turn));
            let at = match during {
                Some(turn) if may_have_been => Some(turn),
                None if uncovered => Some(now),
                _ => None,
            };
            if let Some(at) = at {
                self.entered.insert(card.id.clone(), at);
            }
        }
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
        let keywords = crate::layers::keywords_of(state, unit);
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

    /// I9 on the state one action left: its last `turnStarted` names the state's turn and player, each
    /// `gameOver` is its result, and each seat's last `manaChanged` is that seat's mana. `reset` (a
    /// Glitch's, R676) leaves the turn and the mana unread: the action announced them in the game it took.
    fn announced_state(events: &[GameEvent], state: &GameState, reset: bool) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for event in events {
            let GameEvent::GameOver { winner, reason } = event else {
                continue;
            };
            let announced = GameResult {
                winner: *winner,
                reason: *reason,
            };
            if state.result != Some(announced) {
                let held = match state.result {
                    Some(result) => format!("{} by {}", result.winner, result.reason),
                    None => "none".to_string(),
                };
                found.push(format!(
                    "I9 wrong game over: gameOver names {winner} by {reason} on turn {}, but the result is \
                     {held} (§2.5, §10.3)",
                    state.turn
                ));
            }
        }
        if reset {
            return found;
        }
        let started = events.iter().rev().find_map(|event| match event {
            GameEvent::TurnStarted { player, turn, .. } => Some((*player, *turn)),
            _ => None,
        });
        if let Some((player, turn)) = started
            && (player, turn) != (state.active, state.turn)
        {
            found.push(format!(
                "I9 wrong turn start: turnStarted names {player}'s turn {turn}, but the state is at {}'s \
                 turn {} (§2.2, §10.3)",
                state.active, state.turn
            ));
        }
        for seat in PLAYER_IDS {
            let announced = events.iter().rev().find_map(|event| match event {
                GameEvent::ManaChanged { player, current, max } if *player == seat => Some((*current, *max)),
                _ => None,
            });
            let mana = &state.players[seat].mana;
            if let Some((current, max)) = announced
                && (current, max) != (mana.current, mana.max)
            {
                found.push(format!(
                    "I9 wrong mana: {seat}'s last manaChanged says {current}/{max} on turn {}, but {seat} \
                     holds {}/{} (§2.3, §10.3)",
                    state.turn, mana.current, mana.max
                ));
            }
        }
        found
    }

    /// I9 on the state one action left: each seat's `fatigue_count` is the fatigue draws the stream
    /// announced for it (§2.4, §10.3, R315), one whose hit Armor took whole included (R240, R1362). A
    /// Glitch's reset (R676) starts the game again, so the count starts again from the state it left.
    fn counted_fatigue(&mut self, state: &GameState, reset: bool) -> Vec<String> {
        if reset {
            self.fatigue = state.players.map(|_, side| side.fatigue_count);
            return Vec::new();
        }
        let mut found: Vec<String> = Vec::new();
        for seat in PLAYER_IDS {
            let (counted, held) = (self.fatigue[seat], state.players[seat].fatigue_count);
            if counted != held {
                found.push(format!(
                    "I9 silent fatigue: {seat} holds fatigue count {held} on turn {}, but the stream's count for \
                     {seat} is {counted} (§2.4, §10.3, R315)",
                    state.turn
                ));
            }
        }
        found
    }

    /// I9 on the state one action left: each 1v1 seat's hero health is what the stream's `damage`,
    /// `healthLost`, `healed`, `healthSet` and health `swapped` events left it (§10.3, R2201000). A
    /// Duos seat is skipped, as I17's hero bound is: `duos::reduce` copies the team pool into a spent
    /// hero with no event (`duos/reduce.rs`). A Glitch's reset (R676) starts the tally again from the
    /// state it left. A silent move is reported once: the tally then follows the state.
    fn counted_health(&mut self, state: &GameState, reset: bool) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for seat in PLAYER_IDS {
            let side = &state.players[seat];
            let held = side.hero.health;
            if reset || side.duos.is_some() {
                self.health[seat] = held;
                continue;
            }
            let counted = self.health[seat];
            if counted != held {
                found.push(format!(
                    "I9 silent hero health: {seat} holds hero health {held} on turn {}, but the stream's tally for \
                     {seat} is {counted} (§10.3, R2201000)",
                    state.turn
                ));
                self.health[seat] = held;
            }
        }
        found
    }

    /// I20: feeds one event into the stored-damage tally (§4.4, §10.3, R78). `anchored` collects the cards
    /// whose damage a lawful writer moved with no `damage` or `healed` (the tally then takes the state's),
    /// and `anchor_all` is a board put back or dealt again. A blind spot, accepted: `numberChanged`
    /// "health" is also a health tuning that leaves damage as it was, so that card is not held in that step.
    fn feed_damage(&mut self, event: &GameEvent, anchored: &mut IndexSet<String>, anchor_all: &mut bool) {
        match event {
            // §4.4 step 5: the amount is what got through; the pipeline adds exactly it to `damage`.
            GameEvent::Damage {
                target_id, amount, ..
            } => {
                if let Some(held) = self.unit_damage.get_mut(target_id) {
                    *held += *amount;
                }
            }
            // `heal_unit_by` removes what it healed and emits only when that is above 0.
            GameEvent::Healed {
                target_id, amount, ..
            } => {
                if let Some(held) = self.unit_damage.get_mut(target_id) {
                    *held -= *amount;
                }
            }
            // R78: a card arrives with its damage reset.
            GameEvent::CardPlayed { instance_id, .. } => {
                self.unit_damage.insert(instance_id.clone(), 0);
            }
            GameEvent::Summoned { instance_id, via, .. } => {
                self.unit_damage.insert(instance_id.clone(), 0);
                // `reborn_step` returns a card with `max_health - 1` damage, announced by this `summoned`.
                if matches!(via, Some(KeywordKind::Reborn)) {
                    anchored.insert(instance_id.clone());
                }
            }
            // Vanilla's "stays alive at 1 health" writes stored damage on the card it makes; the
            // transform redefines the card, so both ids take the state's.
            GameEvent::Transformed {
                instance_id,
                new_instance_id,
                ..
            } => {
                anchored.insert(instance_id.clone());
                anchored.insert(new_instance_id.clone());
            }
            // `set_stats_on` clears `damage` with no `healed`, announced only by this.
            GameEvent::NumberChanged { instance_id, key, .. } if key == "health" => {
                anchored.insert(instance_id.clone());
            }
            // R78: a card that left the field and came back at once is reset, with no `healed`.
            GameEvent::Flickered { instance_id, .. } => {
                anchored.insert(instance_id.clone());
            }
            // `restore_board` puts records back and a Glitch's boards deal both fields again, with no
            // event per card.
            GameEvent::RolledBack { .. }
            | GameEvent::Glitched {
                outcome: crate::wire::GlitchOutcome::Boards,
                ..
            } => *anchor_all = true,
            _ => {}
        }
    }

    /// I20 on the state one action left: each field card's stored `damage` is what the stream's `damage`
    /// and `healed` events left it (§4.4, §10.3, R78), as `counted_health` holds a hero's health. A card
    /// the tally has not seen (a Stack pile's uncovering, a Glitch's deal) and one `anchored` or under
    /// `reset` (R676) takes the state's; a card that left the field is dropped (R78 zeroed it). A silent
    /// move is reported once: the tally then follows the state.
    ///
    /// No stride: the tally is fed from every event and read in one pass over the field's cards with no
    /// clone, so a stride would only delay a report. Measured on 2026-11-22 (4 cores, seeds 1–200, the
    /// release `cargo jackioh fuzz`): 52 s before I20 and 54–57 s with it for the 1v1 wave (an
    /// unloaded repeat, 54 s), 146 s before and 151–152 s with it for the Duos wave, checked on every
    /// state; the handicapped wave 51 s with it. Each is under the 10% that would call for a stride.
    fn counted_damage(&mut self, state: &GameState, reset: bool, anchored: &IndexSet<String>) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        let held_now = field_damage(state);
        self.unit_damage.retain(|id, _| held_now.contains_key(id));
        for (id, held) in held_now {
            let counted = match self.unit_damage.get(&id) {
                Some(counted) if !reset && !anchored.contains(&id) => *counted,
                _ => held,
            };
            if counted != held {
                let name = find_instance(state, &id).map_or("a card", |card| card.def_id.as_str());
                found.push(format!(
                    "I20 silent card damage: {name} ({id}) holds damage {held} on turn {}, but the stream's tally \
                     for it is {counted} (§4.4, §10.3, R78)",
                    state.turn
                ));
            }
            self.unit_damage.insert(id, held);
        }
        found
    }

    /// R2201000: a floored hero's tally reads its floor after any move that would take it lower.
    fn hold_hero_floor(&mut self, seat: PlayerId, state: &GameState) {
        if let Some(floor) = crate::state::hero_floor_of(state, seat)
            && self.health[seat] < floor
        {
            self.health[seat] = floor;
        }
    }

    fn hero_loses(&mut self, seat: PlayerId, amount: i32, state: &GameState) {
        self.health[seat] -= amount;
        self.hold_hero_floor(seat, state);
    }

    /// I1 and I3 on the state the next action is chosen in, plus I12's chosen exertion snapshot, the
    /// forecast I18 holds a chosen attack to, the Dormant cards I19 holds untouched, the Units Immune to
    /// Spells I21 does and the Immutable cards whose text I23 does; opens I22's record of random `target`
    /// draws (`testkit::seams`). Empty when clean.
    pub fn before(&mut self, state: &GameState, player: PlayerId, action: &ActionBody) -> Vec<String> {
        // I22: the random target draws from here to `after` are the step's (#4540).
        crate::testkit::seams::start_target_draws();
        // I18: the forecast the seat was shown for the attack it chose, kept for `after` (R3100000).
        self.forecast_kept = None;
        self.note_slumber(state);
        self.note_immune(state);
        self.note_immutable(state);
        if self.judge_forecast
            && let ActionBody::Attack {
                attacker_id,
                target_id,
            } = action
        {
            self.forecast_kept = crate::query::combat_forecast(state, player)
                .attacks
                .into_iter()
                .find(|entry| entry.attacker_id == *attacker_id && entry.target_id == *target_id)
                .map(|entry| KeptForecast {
                    turn: state.turn,
                    attacker: card_label(state, attacker_id),
                    target: card_label(state, target_id),
                    foreseeable: foreseeable_attack(state, attacker_id, target_id),
                    entry,
                });
        }
        let mut found: Vec<String> = Vec::new();
        self.chosen_exertion = match action {
            ActionBody::Attack { attacker_id, .. } => {
                Some((attacker_id, crate::combat::ExertionKind::Attack))
            }
            ActionBody::SwitchPosition { instance_id } => {
                Some((instance_id, crate::combat::ExertionKind::Switch))
            }
            _ => None,
        }
        .and_then(|(instance_id, kind)| {
            find_instance(state, instance_id).map(|card| ChosenExertion {
                instance_id: instance_id.clone(),
                def_id: card.def_id.clone(),
                kind,
                deft: crate::layers::unit_has(state, card, KeywordKind::Deft),
            })
        });
        self.note_windfury(state);
        self.note_frenzy(state);
        self.carried_windfury = PLAYER_IDS
            .into_iter()
            .flat_map(|player| {
                let side = &state.players[player];
                side.hand
                    .iter()
                    .chain(side.library.iter())
                    .chain(side.graveyard.iter())
                    .chain(side.exile.iter())
            })
            .filter(|card| has_keyword(&crate::layers::card_keywords(state, card), KeywordKind::Windfury))
            .map(|card| card.id.clone())
            .collect();

        // R1202: declaring spends the declarer's exertion before its substitute is even summoned, so a
        // substitute barred from the target spends with no declaration following (fuzz seed 2550). The
        // declaration is pre-recorded with no attacks counted, so I4 reads the spend as this stint's and
        // I2 still counts only the declarations the events carry.
        if let ActionBody::Attack { attacker_id, .. } = action {
            let current = self.stint_of(attacker_id);
            let fresh = match self.last_attack.get(attacker_id) {
                Some(previous) => previous.turn != state.turn || previous.stint != current,
                None => true,
            };
            if fresh {
                self.last_attack.insert(
                    attacker_id.clone(),
                    AttackMark {
                        turn: state.turn,
                        stint: current,
                        count: 0,
                    },
                );
            }
        }

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
                .map(crate::reduce::attack_target_id)
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

    /// Feeds one action's events into the shadow, then I2, I4, I5, I7, I9–I12, I16–I21 and I23 against the
    /// resulting state, and I22 on each random `target` draw the step made, on the state it was drawn on.
    pub fn after(&mut self, events: &[GameEvent], state: &GameState) -> Vec<String> {
        // I18: a kept forecast is this step's alone, and so are I19's Dormant cards, I21's immune Units,
        // I22's draws and I23's Immutable cards.
        let kept = self.forecast_kept.take();
        let slumber = self.slumber_step.take();
        let immune = self.immune_step.take();
        let immutable = self.immutable_step.take();
        let draws = crate::testkit::seams::take_target_draws();
        let mut found: Vec<String> = Vec::new();
        // R1829000: the turn the action began on; an uncovering inside it happened on a turn from here on.
        let began_on = self.turn;
        // R424: the Golem's transform follows its combat inside the one attack action, so an attack from
        // an earlier action readies nothing, unless that action left a prompt or work owed: the attack is
        // still resolving then (R1869002: a kill an ambush's Discovers delayed credits on landing).
        if !self.attack_open {
            self.attack = None;
            self.behind_joiners.clear();
        }

        let mut anchored: IndexSet<String> = IndexSet::new();
        let mut anchor_all = false;
        for (index, event) in events.iter().enumerate() {
            self.feed_damage(event, &mut anchored, &mut anchor_all);
            match event {
                GameEvent::TurnStarted { player, turn, .. } => {
                    self.turn = *turn;
                    self.active = *player;
                    self.attack = None;
                    self.behind_joiners.clear();
                    self.fired.clear();
                }
                GameEvent::TurnEnded { player, turn, .. } | GameEvent::TurnAutoEnded { player, turn } => {
                    // I9: a turn ends as the turn under way, the shadow's (§2.2).
                    if (*player, *turn) != (self.active, self.turn) {
                        found.push(format!(
                            "I9 wrong turn end: {} names {player}'s turn {turn}, but the turn under way is {}'s \
                             turn {} (§2.2, §10.3)",
                            event.event_type().as_str(),
                            self.active,
                            self.turn
                        ));
                    }
                }
                GameEvent::Fatigue {
                    player,
                    count,
                    amount,
                } => {
                    // I9: a fatigue draw is its seat's next, and deals FATIGUE_DAMAGE of its count (§2.4, R3,
                    // R315). The shadow counts the events, so one skipped count is one finding.
                    let expected = self.fatigue[*player] + 1;
                    self.fatigue[*player] = expected;
                    if *count != expected {
                        found.push(format!(
                            "I9 wrong fatigue count: fatigue names {player}'s fatigue draw {count} on turn {}, but it \
                             is their fatigue draw {expected} (§2.4, R3, R315)",
                            self.turn
                        ));
                    }
                    let dealt = FATIGUE_DAMAGE(*count);
                    if *amount != dealt {
                        found.push(format!(
                            "I9 wrong fatigue damage: fatigue says {player}'s fatigue draw {count} deals {amount} on \
                             turn {}, but FATIGUE_DAMAGE({count}) is {dealt} (§2.4, R3, R315)",
                            self.turn
                        ));
                    }
                }
                GameEvent::Damage {
                    target_id, amount, ..
                } => {
                    // I9: the amount is what got through Armor (§4.4 step 5); a unit's hit moves no hero.
                    if let Some(seat) = hero_seat_of(target_id) {
                        self.hero_loses(seat, *amount, state);
                    }
                }
                GameEvent::HealthLost { player, amount } => self.hero_loses(*player, *amount, state),
                GameEvent::Healed {
                    target_id, amount, ..
                } => {
                    if let Some(seat) = hero_seat_of(target_id) {
                        self.health[seat] += *amount;
                    }
                }
                GameEvent::HealthSet { player, health, .. } => self.health[*player] = *health,
                GameEvent::Swapped {
                    what: SwapWhat::Health,
                } => {
                    let (one, other) = (self.health[PlayerId::P1], self.health[PlayerId::P2]);
                    self.health[PlayerId::P1] = other;
                    self.health[PlayerId::P2] = one;
                    for seat in PLAYER_IDS {
                        self.hold_hero_floor(seat, state);
                    }
                }
                GameEvent::CardPlayed { instance_id, .. }
                | GameEvent::Summoned { instance_id, .. }
                | GameEvent::ControlChanged { instance_id, .. } => {
                    self.enter(instance_id);
                    self.note_entry_windfury(instance_id, state);
                    if let GameEvent::Summoned { def_id, .. } = event {
                        let back_later = summoned_later(instance_id, &events[index + 1..]);
                        self.note_drawn_windfury(instance_id, def_id, state, back_later);
                    }
                    // R12, R781004: a steal keeps the card on the field, its drawn keywords with it.
                    if matches!(event, GameEvent::ControlChanged { .. })
                        && self.drawn_windfury.contains(instance_id)
                    {
                        let stint = self.stint_of(instance_id);
                        self.windfury.insert(instance_id.clone(), stint);
                    }
                }
                GameEvent::TrapFired { instance_id, .. } => {
                    self.fired.insert(instance_id.clone());
                }
                // R678: a Glitch's boards outcome deals both fields again, its cards placed with no
                // `summoned` (`glitch::place_glitch_boards`), so each card the field holds once the
                // action is over entered on this turn.
                GameEvent::Glitched {
                    outcome: crate::wire::GlitchOutcome::Boards,
                    ..
                } => {
                    for card in field_cards(state) {
                        self.enter(&card.id);
                    }
                }
                GameEvent::Awakened { instance_id, .. } => {
                    // R2259000: a card that awakens is summoning sick for the rest of the turn, as one
                    // that entered the field now is, and `awaken_card` stamps its `summonedTurn` so.
                    self.enter(instance_id);
                }
                GameEvent::Animated {
                    player,
                    instance_id,
                    carried,
                    ..
                } => {
                    // R383: moving into the unit row is entering it, but not for a carried Unit leaving its
                    // carrier (C+ #33) or an "Animated on your turn" card at its controller's start of turn
                    // (R1062, R1126) that did not fire as a trap (#1216). A set Unit's reveal enters, as any
                    // Unit that comes into a unit zone does (R4237000).
                    if *carried != Some(true)
                        && (self.fired.contains(instance_id)
                            || !self.on_its_side_before_this_turn(
                                instance_id,
                                *player,
                                state,
                                &events[index + 1..],
                            ))
                    {
                        self.enter(instance_id);
                        self.note_entry_windfury(instance_id, state);
                    }
                }
                GameEvent::Transformed {
                    instance_id,
                    new_instance_id,
                    ..
                } => {
                    if new_instance_id != instance_id {
                        self.enter(new_instance_id);
                        self.note_entry_windfury(new_instance_id, state);
                    }
                    // R2285002: an attack whose target its attacker's "when this attacks" transformed is
                    // fought at the Unit that took its place, so that Unit is the one its kill names.
                    if let Some(attack) = self.attack.as_mut()
                        && attack.target_id == *instance_id
                        && !attack.killed
                    {
                        attack.target_id = new_instance_id.clone();
                    }
                    if self.attack.as_ref().is_some_and(|attack| {
                        attack.killed && !attack.forced && attack.attacker_id == *instance_id
                    }) {
                        self.readied.insert(new_instance_id.clone());
                    }
                }
                GameEvent::Destroyed {
                    instance_id,
                    killer_id,
                    ..
                } => {
                    // R1203: the latest attack's, or a Prime's held behind its joiners, whose combat is last.
                    let killed_by = self
                        .attack
                        .iter_mut()
                        .chain(self.behind_joiners.iter_mut().rev())
                        .find(|attack| {
                            attack.target_id == *instance_id
                                && killer_id.as_deref() == Some(attack.attacker_id.as_str())
                                && !attack.killed
                        })
                        .map(|attack| {
                            attack.killed = true;
                            attack.attacker_id.clone()
                        });
                    if let Some(attacker_id) = killed_by {
                        // R1869002: a kill readies its attacker only through its own `afterAttack` text;
                        // R3543003: or its Frenzy, which answers the same kill.
                        // R3543003: the Frenzy it began the action with, in the stint it still attacks in.
                        let began_frenzied =
                            self.frenzy.get(&attacker_id) == Some(&self.stint_of(&attacker_id));
                        let answers = began_frenzied
                            || find_instance(state, &attacker_id).is_some_and(|attacker| {
                                crate::scripts::script_of(state, attacker).after_attack.is_some()
                                    || crate::layers::unit_has(state, attacker, KeywordKind::Frenzy)
                            })
                            // R3543003, R781004: a unit that drew its keywords (`note_drawn_windfury`) and is
                            // gone by the end may have drawn Frenzy (a Drone Corvette, Duos seed 1778, #4079).
                            || (self.drawn_windfury.contains(&attacker_id)
                                && find_instance(state, &attacker_id)
                                    .is_none_or(|card| card.zone.z() != ZoneName::Field));
                        if answers {
                            self.credit_kill(&attacker_id);
                        }
                    }
                }
                GameEvent::Redirected {
                    what: RedirectWhat::Attack,
                    from_id,
                    to_id,
                    ..
                } => {
                    // R1122: a redirected attack is fought at the Unit it was re-aimed at, so that Unit is
                    // the defender a Classic Golem must kill to ready its transform (R424, R42).
                    if let Some(attack) = self.attack.as_mut()
                        && attack.target_id == *from_id
                    {
                        attack.target_id = to_id.clone();
                    }
                }
                // R636, R1086: a Fuse (an Equip among them) brings its ingredients' text and a Windfury an
                // aura of theirs gives, which no `keywordGranted` reports (M #65 Keymaster Keenus onto S
                // #113); one gone by the action's end may have drawn it (`note_drawn_windfury`).
                GameEvent::Fused {
                    result_instance_id,
                    def_id,
                    ..
                } => {
                    self.note_entry_windfury(result_instance_id, state);
                    let back_later = summoned_later(result_instance_id, &events[index + 1..]);
                    self.note_drawn_windfury(result_instance_id, def_id, state, back_later);
                }
                GameEvent::KeywordGranted {
                    instance_id,
                    keyword,
                    lost,
                } => {
                    // A grant the stream saw is held from this stint on; `lost` (R46) takes a keyword away.
                    if keyword.kind() == KeywordKind::Windfury && *lost != Some(true) {
                        let stint = self.stint_of(instance_id);
                        self.windfury.insert(instance_id.clone(), stint);
                    }
                }
                GameEvent::Upgraded {
                    instance_id,
                    change:
                        TuningChange::Keyword {
                            keyword,
                            added: true,
                            also,
                        },
                    ..
                } => {
                    // A Buff's keyword row (R386) is the other way a unit gains a keyword, R1160's
                    // multiplied row adding `also` beside it.
                    let mut gained = std::iter::once(keyword).chain(also.iter().flatten());
                    if gained.any(|keyword| keyword.kind() == KeywordKind::Windfury) {
                        let stint = self.stint_of(instance_id);
                        self.windfury.insert(instance_id.clone(), stint);
                    }
                }
                // R3778000: a raised number tops up a drawn-keywords Aura's list with no `keywordGranted`.
                GameEvent::Upgraded {
                    instance_id,
                    def_id,
                    change: TuningChange::Number { .. },
                    ..
                }
                | GameEvent::NumberChanged {
                    instance_id, def_id, ..
                }
                | GameEvent::RadiantSet {
                    instance_id, def_id, ..
                } => {
                    let back_later = summoned_later(instance_id, &events[index + 1..]);
                    self.note_topped_up_windfury(instance_id, def_id, state, back_later);
                }
                GameEvent::PositionSwitched { instance_id, .. } => {
                    // R20: an effect's switch emits the same event but spends nothing. I12 counts only
                    // the event that confirms the chosen `switchPosition` action.
                    let chosen = self
                        .chosen_exertion
                        .as_ref()
                        .filter(|chosen| {
                            chosen.kind == crate::combat::ExertionKind::Switch
                                && chosen.instance_id == *instance_id
                        })
                        .cloned();
                    if let Some(chosen) = chosen {
                        // The chosen action owns only its first matching event. A later switch of the
                        // same Unit in this reduction came from an effect and spends nothing (R20).
                        self.chosen_exertion = None;
                        let current = self.stint_of(instance_id);
                        let switched_here = self
                            .last_switch
                            .get(instance_id)
                            .is_some_and(|mark| mark.turn == self.turn && mark.stint == current);
                        if switched_here {
                            found.push(format!(
                                "I12 second switch: {} ({}) switched position twice on turn {} in the same \
                                 field stint (§4.1 one switch exertion per turn; Deft keeps only one, R49)",
                                chosen.instance_id, chosen.def_id, self.turn
                            ));
                        }
                        let credited = self.credited_at(instance_id).unwrap_or(0);
                        let attacked_since_refresh = self.last_attack.get(instance_id).is_some_and(|mark| {
                            mark.turn == self.turn && mark.stint == current && mark.count > credited
                        });
                        if attacked_since_refresh && !chosen.deft {
                            found.push(format!(
                                "I12 switch after attack: {} ({}) switched position after declaring an attack \
                                 on turn {} in the same field stint without Deft (§4.1, R49; Windfury's first \
                                 attack spends the switch exertion, R636)",
                                chosen.instance_id, chosen.def_id, self.turn
                            ));
                        }
                        self.last_switch.insert(
                            instance_id.clone(),
                            SwitchMark {
                                turn: self.turn,
                                stint: current,
                            },
                        );
                    }
                }
                GameEvent::AttackDeclared {
                    attacker_id,
                    target_id,
                    forced,
                    instead_of,
                } => {
                    // R3139004: the attack before this one inside the action, on a Unit it did not kill, readied an attacker
                    // whose own `afterAttack` answers it (Core #89 Corpse Eater), as the action's end credits it below; read
                    // here, since its second attack is counted before the end (daily fuzz seed 11162).
                    if let Some(attack) = self.attack.as_mut().filter(|attack| {
                        !attack.killed
                            && !attack.answered
                            && !attack.target_id.starts_with(HERO_TARGET_PREFIX)
                    }) {
                        attack.answered = true;
                        let previous = attack.attacker_id.clone();
                        if find_instance(state, &previous).is_some_and(|attacker| {
                            crate::scripts::script_of(state, attacker).after_attack.is_some()
                        }) {
                            self.credit_kill(&previous);
                        }
                    }
                    // R53: a forced attack is not a declaration and spends nothing (nor readies a
                    // transform, R424); its kill may still earn a kill credit (R1869002).
                    let previous = self.attack.take();
                    // R1203: a Prime's joiners attack its target first, each declaring a forced attack, and
                    // its own combat follows with no declaration, so its attack is held behind them.
                    if *forced {
                        self.behind_joiners.retain(|prime| prime.target_id == *target_id);
                        if let Some(prime) = previous.filter(|prime| {
                            !prime.killed
                                && prime.target_id == *target_id
                                && find_instance(state, &prime.attacker_id).is_some_and(|card| {
                                    crate::scripts::flags_of(state, card).attack_joiners == Some(true)
                                })
                        }) {
                            self.behind_joiners.push(prime);
                        }
                    } else {
                        self.behind_joiners.clear();
                    }
                    if *forced {
                        self.attack = Some(ShadowAttack {
                            attacker_id: attacker_id.clone(),
                            target_id: target_id.clone(),
                            killed: false,
                            forced: true,
                            answered: false,
                        });
                        continue;
                    }
                    // R1202: a substitute's declaration spends its declarer's attack, not its own —
                    // Windfast's exertion, Windfast's Windfury count — while the strike itself is the
                    // substitute's to answer for.
                    let declarer = instead_of.as_deref().unwrap_or(attacker_id);
                    let current = self.stint_of(declarer);
                    // R1869002: a credited declarer attacking again inside the action consumes I2's
                    // speculative attack baseline before the settled state can. The declaration alone
                    // does not prove that I12's shared switch exertion was refreshed: a no-op
                    // `afterAttack` can be followed by the very illegal declaration this oracle must
                    // catch. Clear a spent switch only when the resulting field state independently
                    // shows that half reset. R1202 makes `declarer`, not `attacker_id`, own both facts.
                    let switch_spent = self
                        .last_switch
                        .get(declarer)
                        .is_some_and(|mark| mark.turn == self.turn && mark.stint == current);
                    let switch_refreshed = switch_spent
                        && find_instance(state, declarer)
                            .is_some_and(|card| card.zone.z() == ZoneName::Field && !card.exertion.switched);
                    if (!switch_spent || switch_refreshed) && self.unsettled_credits.shift_remove(declarer) {
                        self.readied.insert(declarer.to_string());
                        if switch_refreshed {
                            self.last_switch.shift_remove(declarer);
                        }
                    }
                    let chosen = self.chosen_exertion.as_ref().filter(|chosen| {
                        chosen.kind == crate::combat::ExertionKind::Attack && chosen.instance_id == declarer
                    });
                    let deft = chosen.map_or_else(
                        || {
                            find_instance(state, declarer)
                                .is_some_and(|card| crate::layers::unit_has(state, card, KeywordKind::Deft))
                        },
                        |chosen| chosen.deft,
                    );
                    let switched_here = self
                        .last_switch
                        .get(declarer)
                        .is_some_and(|mark| mark.turn == self.turn && mark.stint == current);
                    if switched_here && !deft {
                        let who = chosen.map_or_else(
                            || find_instance(state, declarer).map_or_else(|| declarer.to_string(), name_of),
                            |chosen| format!("{} ({})", chosen.instance_id, chosen.def_id),
                        );
                        found.push(format!(
                            "I12 attack after switch: {who} declared an attack after switching position on turn \
                             {} in the same field stint without Deft (§4.1, R49; Windfury does not restore the \
                             switch exertion, R636)",
                            self.turn
                        ));
                    }
                    let previous = self.last_attack.get(declarer).copied();
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
                    // R3550001: a Unit that "can attack N times per turn" declares N attacks.
                    // R636, R3556008: one gone by the end may have had an aura's Windfury, read where the action left it.
                    // R383: an animated card back in its backrow by the action's end is read as a gone unit (fuzz
                    // seed 1013, #4079).
                    let gone = find_instance(state, declarer).is_none_or(|card| {
                        card.zone.z() != ZoneName::Field
                            || (!matches!(card.zone, Zone::Field { row: Row::Units, .. })
                                && crate::animated::face_type_of(state, card) != CardType::Unit)
                    });
                    let per_turn = if self.windfury.get(declarer) == Some(&current)
                        || (gone && aura_gives_windfury(state))
                    {
                        WINDFURY_ATTACKS
                    } else {
                        1
                    };
                    let per_turn = per_turn.max(
                        find_instance(state, declarer)
                            .map_or(1, |card| crate::combat::attacks_per_turn(state, card)),
                    );
                    // R1869002: a kill credit's fresh exertion counts the attacks of a turn again from there.
                    let allowed = per_turn + self.credited_at(declarer).unwrap_or(0);
                    if mark.count > allowed {
                        let who = match find_instance(state, declarer) {
                            None => declarer.to_string(),
                            Some(unit) => name_of(unit),
                        };
                        found.push(format!(
                            "I2 extra attack: {who} declared attack number {} on turn {} without re-entering the \
                             field (§4.1 one exertion per turn, R171; Windfury two, R636)",
                            mark.count, self.turn
                        ));
                    }
                    self.last_attack.insert(declarer.to_string(), mark);
                    self.attack = Some(ShadowAttack {
                        attacker_id: attacker_id.clone(),
                        target_id: target_id.clone(),
                        killed: false,
                        forced: false,
                        answered: false,
                    });
                }
                _ => {}
            }
        }

        self.note_windfury(state);
        // R3139004: an attacker whose text answers an attack on a Unit, survivors only (Core #89 Corpse Eater:
        // "After this attacks a Unit, it may attack again"), is readied by it kill or no kill, so it earns
        // the credit a kill earns; `settle_kill_credits` takes it back if the attack stayed spent.
        let unanswered = self.attack.as_mut().filter(|attack| {
            !attack.killed && !attack.answered && !attack.target_id.starts_with(HERO_TARGET_PREFIX)
        });
        if let Some(attack) = unanswered {
            attack.answered = true;
            let attacker_id = attack.attacker_id.clone();
            let answers = find_instance(state, &attacker_id).is_some_and(|attacker| {
                attacker.zone.z() == ZoneName::Field
                    && crate::scripts::script_of(state, attacker).after_attack.is_some()
            });
            if answers {
                self.credit_kill(&attacker_id);
            }
        }
        self.settle_kill_credits(state);

        // I9: what the action announced is the state it left (§10.3); a Glitch's reset moves `resets` (R676).
        let reset = state.resets != self.resets;
        self.resets = state.resets;
        found.extend(InvariantMonitor::announced_state(events, state, reset));
        found.extend(self.counted_fatigue(state, reset));
        found.extend(self.counted_health(state, reset));
        found.extend(self.counted_damage(state, reset || anchor_all, &anchored));

        // R1829000: a card a Stack pile uncovers has entered, though no event says so (R212).
        if !reset {
            self.note_uncoverings(events, state, began_on);
        }
        self.dormant = dormant_cards(state);
        self.attack_open = state.pending.is_some() || !state.work.is_empty();

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

        found.extend(branch_state_findings(events, state));
        // I19: nothing reached a card Dormant as the step began (§6.1 Dormant, R1839000).
        if let Some(step) = &slumber {
            found.extend(slumber_reach_findings(step, events, state));
        }
        // I21: no Spell reached a Unit Immune to Spells as the step began (§6.1, R4453000).
        if let Some(step) = &immune {
            found.extend(immune_reach_findings(step, events, state));
        }
        // I23: nothing changed the text of a card Immutable as the step began (§6.1, R23, R4353000).
        if let Some(step) = &immutable {
            found.extend(immutable_text_findings(step, events, state));
        }
        // I22: every random target draw aimed where its source may (R452, R1200).
        found.extend(draws.iter().flat_map(target_draw_findings));
        // I18: a plain attack did what its seat's forecast said (R3100000, R4111000).
        if let Some(kept) = kept
            && kept.foreseeable
            && plain_combat(events, state, &kept.entry)
        {
            self.forecast_judged += 1;
            let forecast = PlayedAttack::forecast(&kept.entry);
            let splashed: Vec<String> = kept
                .entry
                .splash
                .iter()
                .map(|hit| hit.instance_id.clone())
                .collect();
            let played = PlayedAttack::read(
                events,
                state,
                &kept.entry.attacker_id,
                &kept.entry.target_id,
                &splashed,
            );
            if played != forecast {
                found.push(format!(
                    "I18 forecast drift: {} attacking {} on turn {} was forecast {forecast} but did {played} \
                     (R3100000)",
                    kept.attacker, kept.target, kept.turn
                ));
            }
        }
        // A chosen action can confirm at most this one reduction. In Duos, `after` also visits the
        // untouched duel, so no marker may survive to match another duel's later event.
        self.chosen_exertion = None;
        found
    }

    /// I6, I13, I15 and I22's listed declarations for both seats on a state the fuzz reached, setup's
    /// included, and I22's open prompt. Empty when clean. A panic in `view_for` or `legal_actions` is
    /// reported, not raised.
    pub fn hidden(&self, state: &GameState) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for viewer in PLAYER_IDS {
            let checked = catch_unwind(AssertUnwindSafe(|| {
                let view = crate::view_for::view_for(state, viewer);
                let legal = crate::reduce::legal_actions(state, viewer);
                // I13 and I15 read the same view and listing (R2975000, R3061000).
                let mut violations = hidden_information_violations(state, viewer, &view, &legal);
                violations.extend(play_reason_violations(state, viewer, &view, &legal));
                violations.extend(activation_reason_violations(state, viewer, &view, &legal));
                // I22: and every target the listing declares (R450, #4540).
                violations.extend(listed_target_findings(state, viewer, &legal));
                violations
            }));
            match checked {
                Ok(violations) => found.extend(violations),
                Err(payload) => {
                    found.push(format!(
                        "I6 view threw: viewFor or legalActions for {viewer} on turn {}: {} (§10.8)",
                        state.turn,
                        panic_text(&*payload)
                    ));
                }
            }
        }
        // I22: every option an open target prompt offers (R450, #4540).
        found.extend(offered_target_findings(state));
        found
    }

    /// I8 on a state the fuzz reached, for the seat that may act: every unlisted attack, switch,
    /// activation, play, Trade, buy and sale `reduce` accepts. Empty when clean.
    pub fn unlisted(&self, state: &GameState) -> Vec<String> {
        let Some(player) = crate::reduce::seat_to_act(state) else {
            return Vec::new();
        };
        let legal = crate::reduce::legal_actions(state, player);
        unlisted_accepted(state, player, &legal)
    }

    /// I14 on a state the fuzz reached, for both seats: the listed actions `reducer` refuses. Empty when clean.
    pub fn listed(
        &self,
        state: &GameState,
        reducer: impl Fn(&GameState, &Action) -> ReduceResult,
    ) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for seat in PLAYER_IDS {
            let legal = crate::reduce::legal_actions(state, seat);
            found.extend(listed_refused(state, seat, &legal, &reducer));
        }
        found
    }
}

/// Redaction checks on a `DuosView` (§13.6, B22, B23):
/// - enemy `ended` must remain hidden (all `false`)
/// - enemy `passes_left` must remain hidden (all `0`)
/// - `concede_offered_by` must belong to the viewer's own team
pub fn check_duos_view_redaction(view: &DuosView) -> Vec<String> {
    let mut violations = Vec::new();
    if view.enemy.ended.iter().any(|&e| e) {
        violations.push(format!(
            "I6 duos leak: enemy ended leaked {:?} to {} (§13.6, B22)",
            view.enemy.ended, view.seat
        ));
    }
    if view.enemy.passes_left.iter().any(|&p| p != 0) {
        violations.push(format!(
            "I6 duos leak: enemy passes_left leaked {:?} to {} (§13.6, B22)",
            view.enemy.passes_left, view.seat
        ));
    }
    if let Some(offer) = view.concede_offered_by
        && offer.team() != view.seat.team()
    {
        violations.push(format!(
            "I6 duos leak: enemy concede offer from {offer} leaked to {} (§13.6)",
            view.seat
        ));
    }
    violations
}

/// A tamper callback for testing seat redaction on a generated `DuosView`.
pub type DuosViewTamper = std::sync::Arc<dyn Fn(&mut DuosView) + Send + Sync>;

/// The Duos invariant monitor (#1074): one `InvariantMonitor` per duel fed from `DuosEvent::Duel`
/// event streams, hidden information checks across both duels, `duos_view_for` seat redaction
/// checks, and life pool bounds (§13.4).
pub struct DuosInvariantMonitor {
    monitors: [InvariantMonitor; DUOS_DUELS],
    pub view_tamper: Option<DuosViewTamper>,
}

impl Clone for DuosInvariantMonitor {
    fn clone(&self) -> Self {
        DuosInvariantMonitor {
            monitors: self.monitors.clone(),
            view_tamper: self.view_tamper.clone(),
        }
    }
}

impl std::fmt::Debug for DuosInvariantMonitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DuosInvariantMonitor")
            .field("monitors", &self.monitors)
            .field("view_tamper", &self.view_tamper.as_ref().map(|_| "<tamper fn>"))
            .finish()
    }
}

pub fn create_duos_invariant_monitor(start: &DuosState) -> DuosInvariantMonitor {
    DuosInvariantMonitor::new(start)
}

impl DuosInvariantMonitor {
    pub fn new(start: &DuosState) -> DuosInvariantMonitor {
        // I18 judges no Duos attack: Duos is out of #4111's scope, and a Duos seat is shown a subset of
        // its duel's forecast entries.
        let duel = |index: usize| InvariantMonitor {
            judge_forecast: false,
            ..InvariantMonitor::new(&start.duels[index])
        };
        DuosInvariantMonitor {
            monitors: [duel(0), duel(1)],
            view_tamper: None,
        }
    }

    pub fn duel_monitor_mut(&mut self, duel: usize) -> &mut InvariantMonitor {
        &mut self.monitors[duel]
    }

    /// I1 and I3 on the state the next Duos action is chosen in, plus I12's chosen exertion snapshot, and
    /// I19's Dormant cards, I21's immune Units and I23's Immutable cards in every duel, since an action in
    /// one may reach into the other (#4454, #4508, #4609).
    pub fn before(&mut self, state: &DuosState, seat: DuosSeat, action: &DuosActionBody) -> Vec<String> {
        let duel = seat.duel();
        let violations = match action {
            DuosActionBody::Duel { body } => {
                self.monitors[duel].before(&state.duels[duel], seat.player(), body)
            }
            DuosActionBody::Pass { .. } | DuosActionBody::Concede => {
                self.monitors[duel].before(&state.duels[duel], seat.player(), &ActionBody::EndTurn)
            }
        };
        for (other, monitor) in self.monitors.iter_mut().enumerate() {
            if other != duel
                && let Some(other_state) = state.duels.get(other)
            {
                monitor.note_slumber(other_state);
                monitor.note_immune(other_state);
                monitor.note_immutable(other_state);
            }
        }
        violations
            .into_iter()
            .map(|v| format!("duel {duel}: {v}"))
            .collect()
    }

    /// Feeds one action's events into the monitors, asserts I2, I4, I5, I7, I9–I12, I16, I17, I19–I21 and
    /// I23 against both duels and I22 on each random `target` draw under the duel it was drawn in, and checks
    /// pool bounds (§13.4) for both teams.
    pub fn after(&mut self, events: &[DuosEvent], state: &DuosState) -> Vec<String> {
        // I22: the action's draws, taken before the duels' own `after`s, which would file them all under
        // the first. A draw's state carries its duel's seed (`duos_duel_seed`), which names the duel.
        let draws = crate::testkit::seams::take_target_draws();
        let mut found = Vec::new();
        let mut duel_events: [Vec<GameEvent>; DUOS_DUELS] = [Vec::new(), Vec::new()];
        for event in events {
            if let DuosEvent::Duel { duel, event } = event {
                let idx = *duel as usize;
                if idx < DUOS_DUELS {
                    duel_events[idx].push(event.clone());
                }
            }
        }
        for (duel, duel_events_list) in duel_events.iter().enumerate().take(DUOS_DUELS) {
            if duel < state.duels.len() {
                let violations = self.monitors[duel].after(duel_events_list, &state.duels[duel]);
                found.extend(violations.into_iter().map(|v| format!("duel {duel}: {v}")));
            }
        }
        for draw in &draws {
            let duel = state.duels.iter().position(|duel| duel.seed == draw.state.seed);
            found.extend(target_draw_findings(draw).into_iter().map(|v| match duel {
                Some(duel) => format!("duel {duel}: {v}"),
                None => v,
            }));
        }
        for &team in Team::ALL {
            let pool = state.teams[team].pool;
            let cap = state.teams[team].rules.pool_health;
            if pool > cap {
                found.push(format!("pool bound: team {team} pool {pool} > cap {cap} (§13.4)"));
            }
            if state.result.is_none() && pool <= 0 {
                found.push(format!(
                    "pool bound: live game has team {team} pool {pool} <= 0 (§13.4)"
                ));
            }
        }
        found
    }

    /// I6, I13, I15 and I22's listed and offered targets for all seats: both duels' 1v1 checks, plus
    /// `duos_view_for` redaction for each seat.
    pub fn hidden(&self, state: &DuosState) -> Vec<String> {
        let mut found = Vec::new();
        for (duel, monitor) in self.monitors.iter().enumerate() {
            if duel < state.duels.len() {
                let violations = monitor.hidden(&state.duels[duel]);
                found.extend(violations.into_iter().map(|v| format!("duel {duel}: {v}")));
            }
        }
        for seat in DUOS_SEATS {
            let checked = catch_unwind(AssertUnwindSafe(|| {
                let mut view = duos_view_for(state, seat);
                if let Some(tamper) = &self.view_tamper {
                    tamper(&mut view);
                }
                check_duos_view_redaction(&view)
            }));
            match checked {
                Ok(violations) => {
                    found.extend(
                        violations
                            .into_iter()
                            .map(|v| format!("duel {} (seat {seat}): {v}", seat.duel())),
                    );
                }
                Err(payload) => {
                    let message = if let Some(text) = payload.downcast_ref::<&str>() {
                        (*text).to_string()
                    } else if let Some(text) = payload.downcast_ref::<String>() {
                        text.clone()
                    } else {
                        "a panic with no message".to_string()
                    };
                    found.push(format!(
                        "duel {} (seat {seat}): I6 duos view threw: duos_view_for: {message} (§13.6)",
                        seat.duel()
                    ));
                }
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DECK_SIZE;
    use crate::testkit::scenario::{catalog_override, register_catalog};
    use crate::wire::{CardDefs, DuosDecks};

    fn test_decks() -> DuosDecks {
        if catalog_override().is_none() {
            let defs: CardDefs = serde_json::from_str(include_str!("../../../cards/catalog.json"))
                .expect("crates/cards/catalog.json parses as CardDefs");
            register_catalog(defs);
        }
        let defs = catalog_override().unwrap();
        let size = usize::try_from(DECK_SIZE).unwrap();
        let ids: Vec<String> = defs
            .iter()
            .filter(|(_, def)| !def.is_token())
            .map(|(id, _)| id.clone())
            .take(size * 4)
            .collect();
        DuosDecks {
            a1: ids[0..size].to_vec(),
            a2: ids[size..size * 2].to_vec(),
            b1: ids[size * 2..size * 3].to_vec(),
            b2: ids[size * 3..size * 4].to_vec(),
        }
    }

    /// I18 (#4111): a Duos seat is shown a subset of its duel's forecast entries, so neither duel's
    /// monitor judges an attack by the duel's own forecast; a 1v1 monitor does.
    #[test]
    fn i18_a_duos_duel_monitor_judges_no_forecast() {
        let state = crate::testkit::duos_scenario::duos_scenario("i18-duos", test_decks(), None);
        let monitor = create_duos_invariant_monitor(&state);
        assert!(monitor.monitors.iter().all(|duel| !duel.judge_forecast));
        assert!(create_invariant_monitor(&state.duels[0]).judge_forecast);
    }

    #[test]
    fn duos_monitor_clean_on_fresh_game() {
        let decks = test_decks();
        let state = crate::testkit::duos_scenario::duos_scenario("clean-duos-monitor", decks, None);
        let mut monitor = create_duos_invariant_monitor(&state);
        assert_eq!(monitor.after(&[], &state), Vec::<String>::new());
        assert_eq!(monitor.hidden(&state), Vec::<String>::new());
    }

    #[test]
    fn check_duos_view_redaction_flags_enemy_ended_and_passes_leak() {
        let decks = test_decks();
        let state = crate::testkit::duos_scenario::duos_scenario("redaction-test", decks, None);
        let mut view = duos_view_for(&state, DuosSeat::A1);

        assert_eq!(check_duos_view_redaction(&view), Vec::<String>::new());

        view.enemy.ended = vec![true, false];
        let violations = check_duos_view_redaction(&view);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("enemy ended leaked"));

        view.enemy.ended = vec![false, false];
        view.enemy.passes_left = vec![1, 0];
        let violations = check_duos_view_redaction(&view);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("enemy passes_left leaked"));

        // Enemy concede offer leak (offer from B1 leaked to A1)
        view.enemy.passes_left = vec![0, 0];
        view.concede_offered_by = Some(DuosSeat::B1);
        let violations = check_duos_view_redaction(&view);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("enemy concede offer from b1 leaked to a1"));
    }

    #[test]
    fn duos_monitor_flags_pool_bound_overflow() {
        let decks = test_decks();
        let mut state = crate::testkit::duos_scenario::duos_scenario("pool-overflow-test", decks, None);
        let mut monitor = create_duos_invariant_monitor(&state);

        assert_eq!(monitor.after(&[], &state), Vec::<String>::new());

        state.teams.a.pool = 25;
        let findings = monitor.after(&[], &state);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("pool bound: team a pool 25 > cap 20"))
        );

        state.teams.a.pool = 0;
        let findings = monitor.after(&[], &state);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("pool bound: live game has team a pool 0 <= 0"))
        );
    }

    #[test]
    fn duos_monitor_leaves_a_duos_seat_s_hero_to_the_pool_bound_but_holds_its_mana_i17() {
        let mut state = crate::testkit::duos_scenario::duos_scenario("i17-duos", test_decks(), None);
        let monitor = create_duos_invariant_monitor(&state);
        // The premise: duel 0 is settled and its p1 is a Duos seat, so only the `duos` guard stops I17.
        let duel = &state.duels[0];
        assert!(duel.result.is_none() && duel.pending.is_none() && duel.mulligan.is_none());
        assert!(duel.players[PlayerId::P1].duos.is_some());
        let i17 = |found: Vec<String>| -> Vec<String> {
            found.into_iter().filter(|f| f.contains(": I17 ")).collect()
        };

        // B15's `sync_pools` writes the pool into a spent hero where no state check runs (§13.4).
        state.duels[0].players[PlayerId::P1].hero.health = 0;
        assert_eq!(i17(monitor.clone().after(&[], &state)), Vec::<String>::new());

        // The contrast: the same hero on a 1v1 seat is a live hero at 0.
        let mut one_v_one = state.clone();
        one_v_one.duels[0].players[PlayerId::P1].duos = None;
        let found = i17(monitor.clone().after(&[], &one_v_one));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("duel 0: I17 live hero at 0"), "{}", found[0]);

        state.duels[0].players[PlayerId::P1].mana.current = -1;
        let found = i17(monitor.clone().after(&[], &state));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("duel 0: I17 negative mana"), "{}", found[0]);
    }
    #[test]
    fn duos_monitor_skips_a_duos_seat_s_hero_health_i9() {
        let mut state = crate::testkit::duos_scenario::duos_scenario("i9-duos-health", test_decks(), None);
        let monitor = create_duos_invariant_monitor(&state);
        assert!(state.duels[0].players[PlayerId::P1].duos.is_some());
        let silent = |found: Vec<String>| -> Vec<String> {
            found
                .into_iter()
                .filter(|f| f.contains(": I9 silent hero health"))
                .collect()
        };

        // `duos::reduce` copies the team pool into a spent hero with no event (§13.4).
        state.duels[0].players[PlayerId::P1].hero.health = 7;
        assert_eq!(silent(monitor.clone().after(&[], &state)), Vec::<String>::new());

        // The contrast: the same silent move on a 1v1 seat is reported.
        let mut one_v_one = state.clone();
        one_v_one.duels[0].players[PlayerId::P1].duos = None;
        let found = silent(monitor.clone().after(&[], &one_v_one));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("duel 0: I9 silent hero health: p1"),
            "{}",
            found[0]
        );
    }

    /// I19 (#4454): each Duos duel's monitor judges the Dormant cards its duel began the step with,
    /// whichever duel the acting seat sits in, and an `after` with no `before` judges none.
    #[test]
    fn i19_each_duos_duel_judges_its_dormant_cards() {
        let mut state = crate::testkit::duos_scenario::duos_scenario("i19-duos", test_decks(), None);
        let duel = &mut state.duels[1];
        let def_id = duel.players[PlayerId::P1].hand[0].def_id.clone();
        let at = Zone::Field {
            player: PlayerId::P1,
            row: Row::Units,
            lane: 1,
        };
        let mut card = crate::state::new_instance(duel, &def_id, PlayerId::P1, at);
        card.slumber = Some(crate::state::Slumber { turns: Some(2) });
        let id = card.id.clone();
        duel.players[PlayerId::P1].units[0] = Some(vec![card]);
        let hit: GameEvent = serde_json::from_value(serde_json::json!({
            "type": "damage",
            "sourceId": null,
            "targetId": id,
            "amount": 1,
            "combat": false,
        }))
        .expect("a damage event");
        let events = [DuosEvent::Duel { duel: 1, event: hit }];
        let i19 = |found: Vec<String>| -> Vec<String> {
            found.into_iter().filter(|f| f.contains(": I19 ")).collect()
        };
        let end_turn = DuosActionBody::Duel {
            body: ActionBody::EndTurn,
        };
        // A seat of duel 0 acts, and duel 1 judges its own Dormant card; a seat of duel 1 likewise.
        for seat in [DuosSeat::A1, DuosSeat::A2] {
            let mut monitor = create_duos_invariant_monitor(&state);
            monitor.before(&state, seat, &end_turn);
            let found = i19(monitor.after(&events, &state));
            assert_eq!(found.len(), 1, "{seat}: {found:?}");
            assert!(
                found[0].starts_with("duel 1: I19 reached Dormant"),
                "{}",
                found[0]
            );
            assert!(found[0].contains(&id), "{}", found[0]);
        }
        let mut unprimed = create_duos_invariant_monitor(&state);
        assert_eq!(i19(unprimed.after(&events, &state)), Vec::<String>::new());
    }

    /// I21 (#4508): each Duos duel's monitor judges the Units Immune to Spells its duel began the step
    /// with, whichever duel the acting seat sits in, and an `after` with no `before` judges none.
    #[test]
    fn i21_each_duos_duel_judges_its_immune_units() {
        let mut state = crate::testkit::duos_scenario::duos_scenario("i21-duos", test_decks(), None);
        let duel = &mut state.duels[1];
        let at = Zone::Field {
            player: PlayerId::P1,
            row: Row::Units,
            lane: 1,
        };
        let card = crate::state::new_instance(duel, "stand-in (1) 4/4 Immune to Spells", PlayerId::P1, at);
        let id = card.id.clone();
        duel.players[PlayerId::P1].units[0] = Some(vec![card]);
        let event = |value: serde_json::Value| -> DuosEvent {
            DuosEvent::Duel {
                duel: 1,
                event: serde_json::from_value(value).expect("an event"),
            }
        };
        let events = [
            event(serde_json::json!({
                "type": "hookResolving",
                "sourceId": "i21-source",
                "defId": "stand-in (0) Spell",
                "controller": "p2",
                "hook": "cry",
            })),
            event(serde_json::json!({ "type": "buffed", "instanceId": id, "attack": 1, "health": 1 })),
        ];
        let i21 = |found: Vec<String>| -> Vec<String> {
            found.into_iter().filter(|f| f.contains(": I21 ")).collect()
        };
        let end_turn = DuosActionBody::Duel {
            body: ActionBody::EndTurn,
        };
        // A seat of duel 0 acts, and duel 1 judges its own immune Unit; a seat of duel 1 likewise.
        for seat in [DuosSeat::A1, DuosSeat::A2] {
            let mut monitor = create_duos_invariant_monitor(&state);
            monitor.before(&state, seat, &end_turn);
            let found = i21(monitor.after(&events, &state));
            assert_eq!(found.len(), 1, "{seat}: {found:?}");
            assert!(
                found[0].starts_with("duel 1: I21 Spell reached Immune to Spells"),
                "{}",
                found[0]
            );
            assert!(found[0].contains(&id), "{}", found[0]);
        }
        let mut unprimed = create_duos_invariant_monitor(&state);
        assert_eq!(i21(unprimed.after(&events, &state)), Vec::<String>::new());
    }

    /// I23 (#4609): each Duos duel judges its own Immutable cards, whichever duel's seat acts.
    #[test]
    fn i23_each_duos_duel_judges_its_immutable_cards() {
        let mut state = crate::testkit::duos_scenario::duos_scenario("i23-duos", test_decks(), None);
        let duel = &mut state.duels[1];
        let at = Zone::Field {
            player: PlayerId::P1,
            row: Row::Units,
            lane: 1,
        };
        let card = crate::state::new_instance(duel, "stand-in (1) 4/4 Indestructible", PlayerId::P1, at);
        let id = card.id.clone();
        duel.players[PlayerId::P1].units[0] = Some(vec![card]);
        let events = [DuosEvent::Duel {
            duel: 1,
            event: serde_json::from_value(serde_json::json!({
                "type": "degraded",
                "instanceId": id,
                "defId": "stand-in (1) 4/4 Indestructible",
                "change": { "kind": "stats", "attack": -1, "health": -3 },
            }))
            .expect("an event"),
        }];
        let i23 = |found: Vec<String>| -> Vec<String> {
            found.into_iter().filter(|f| f.contains(": I23 ")).collect()
        };
        let end_turn = DuosActionBody::Duel {
            body: ActionBody::EndTurn,
        };
        for seat in [DuosSeat::A1, DuosSeat::A2] {
            let mut monitor = create_duos_invariant_monitor(&state);
            monitor.before(&state, seat, &end_turn);
            let found = i23(monitor.after(&events, &state));
            assert_eq!(found.len(), 1, "{seat}: {found:?}");
            assert!(
                found[0].starts_with("duel 1: I23 Immutable text changed: degraded"),
                "{}",
                found[0]
            );
            assert!(found[0].contains(&id), "{}", found[0]);
        }
        let mut unprimed = create_duos_invariant_monitor(&state);
        assert_eq!(i23(unprimed.after(&events, &state)), Vec::<String>::new());
    }

    /// I20 (#4478): each Duos duel's monitor holds its own cards' stored damage to its own events.
    #[test]
    fn i20_each_duos_duel_s_monitor_holds_its_stored_damage() {
        let mut state = crate::testkit::duos_scenario::duos_scenario("i20-duos", test_decks(), None);
        let duel = &mut state.duels[1];
        let def_id = duel.players[PlayerId::P1].hand[0].def_id.clone();
        let at = Zone::Field {
            player: PlayerId::P1,
            row: Row::Units,
            lane: 1,
        };
        let card = crate::state::new_instance(duel, &def_id, PlayerId::P1, at);
        let id = card.id.clone();
        duel.players[PlayerId::P1].units[0] = Some(vec![card]);
        let monitor = create_duos_invariant_monitor(&state);
        let i20 = |found: Vec<String>| -> Vec<String> {
            found.into_iter().filter(|f| f.contains(": I20 ")).collect()
        };
        let hit: GameEvent = serde_json::from_value(serde_json::json!({
            "type": "damage",
            "sourceId": null,
            "targetId": id,
            "amount": 1,
            "combat": false,
        }))
        .expect("a damage event");
        let events = [DuosEvent::Duel { duel: 1, event: hit }];

        // A bump with no event is reported by the duel that holds the card.
        let bump = |state: &mut DuosState| {
            let card = state.duels[1].players[PlayerId::P1].units[0]
                .as_mut()
                .and_then(|pile| pile.first_mut())
                .expect("the unit");
            card.damage += 1;
        };
        bump(&mut state);
        let found = i20(monitor.clone().after(&[], &state));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("duel 1: I20 silent card damage"),
            "{}",
            found[0]
        );
        // The same bump with its `damage` event is clean.
        assert_eq!(i20(monitor.clone().after(&events, &state)), Vec::<String>::new());
    }

    /// I22 (#4540): a random target draw made while a Duos action runs is judged under the duel it was
    /// drawn in, whichever duel the acting seat sits in, and once.
    #[test]
    fn i22_a_duos_draw_is_judged_under_the_duel_it_was_drawn_in() {
        let mut state = crate::testkit::duos_scenario::duos_scenario("i22-duos", test_decks(), None);
        let duel = &mut state.duels[1];
        let def_id = duel.players[PlayerId::P1].hand[0].def_id.clone();
        let at = Zone::Field {
            player: PlayerId::P1,
            row: Row::Units,
            lane: 1,
        };
        let mut card = crate::state::new_instance(duel, &def_id, PlayerId::P1, at);
        card.slumber = Some(crate::state::Slumber { turns: Some(2) });
        let pick = Selection::Instance {
            instance_id: card.id.clone(),
        };
        let id = card.id.clone();
        duel.players[PlayerId::P1].units[0] = Some(vec![card]);
        let i22 = |found: Vec<String>| -> Vec<String> {
            found.into_iter().filter(|f| f.contains("I22 ")).collect()
        };
        let end_turn = DuosActionBody::Duel {
            body: ActionBody::EndTurn,
        };
        let mut monitor = create_duos_invariant_monitor(&state);
        monitor.before(&state, DuosSeat::A1, &end_turn);
        crate::testkit::seams::record_target_draw(
            &state.duels[1],
            None,
            PlayerId::P1,
            TargetDrawBy::Mayor,
            std::slice::from_ref(&pick),
        );
        let found = i22(monitor.after(&[], &state));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("duel 1: I22 unlawful random target"),
            "{}",
            found[0]
        );
        assert!(
            found[0].contains(&id) && found[0].contains("R1839000"),
            "{}",
            found[0]
        );
        assert_eq!(i22(monitor.after(&[], &state)), Vec::<String>::new());
    }
}
