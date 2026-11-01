//! Fuzz-level invariants for summoning sickness, exertion, game over, hidden information and owed
//! work (SPEC §4.1, §4.2, §9.3, §10.3, §10.8, R53, R83, R113, R171; docs/polish/4-edge-cases.md
//! "Fuzz invariants"). `cargo jackioh fuzz` runs one monitor per game (SURFACE §8, §12), `fuzz --duos`
//! one per duel through `DuosInvariantMonitor` (#1074, with the Duos view's own redaction and the pool
//! bounds, §13.4, §13.6), and the edge-case hunters reuse it.
//!
//! THE MONITOR IS AN ORACLE, NOT THE FIX RESTATED. It never reads `summonedTurn` to decide who is
//! sick, but to settle what its shadow can only bound (`readied`, an uncovering inside one action). It
//! keeps a shadow built only from the event stream and the Stack piles of the states it reads:
//!   - `turn` and the player whose turn it is, from `turnStarted`;
//!   - `entered`, the turn of each instance's latest entry, and `stint`, how many entries it has had.
//!     An entry event is `cardPlayed`, `summoned`, `controlChanged`, `animated` (an Animated card
//!     stepping from its backrow zone into a unit zone enters it on that turn, R383, except an
//!     "Animated on your turn" card animating at its controller's start of turn that has been on
//!     that side since the turn began, the keyword read as the animation had it (a later event in the
//!     action may change it): that enters nothing, R1062; one animating on the other
//!     player's turn, as Pareto Optimality's attack animates The Cane, R1126, enters, and so does one
//!     that fired as a trap this turn, animating as its firing's last step), or a
//!     `transformed` whose new instance differs from the old. `fused` keeps the target's entry (R77),
//!     and a move along one side emits nothing and changes nothing. A card a Stack pile uncovers has
//!     entered (R1829000) with no event (R212): one dormant in the last state read (`dormant`) that
//!     acts now, on a side no `rolledBack` restored and not the card a `fused` kept, entered inside the
//!     action, its stint kept, on the turn of the action its `summonedTurn` names (the last when it
//!     names none); one that may have been uncovered inside the action (dormant before, or in a zone
//!     an entry event or a `cardAnnounced` named) takes `summonedTurn`'s word when it names a turn of
//!     the action, as a `readied` stint does;
//!   - `lastAttack`, the turn and stint of each instance's latest declared (not forced) attack,
//!     pre-recorded with no attacks counted when its attack action is chosen, so a substitute
//!     barred from the target reads as this stint's spend (R1202);
//!   - `readied`, the Transform results R424 lets attack on the turn they entered: an attacker that
//!     destroyed the Unit its declared attack targeted (`destroyed` naming it the killer, R42) and is
//!     then `transformed` (Classic+ #73.1 Classic Golem) passes "may attack again this turn" to the new
//!     instance, so that instance is not sick (I1) and carries no `summonedTurn` (I4a) for that stint.
//!     A `redirected` attack targets the Unit it was re-aimed at, where its combat is fought (R1122;
//!     fuzz seed 9123 of the handicapped wave, #1645). A card `awakened` enters afresh: summoning sick
//!     for the rest of that turn, its `summonedTurn` the turn it awakened in (R2259000; before it, R1839002
//!     readied one awakened on the turn it entered, fuzz seed 2);
//!   - `kill_credits`, the fresh exertion a kill gives without a transform (B5 E35's `mayAttackAgain`,
//!     Stinkyscape #94 Tmack Trav, R1869002): an attacker that destroyed the Unit its attack targeted,
//!     declared or forced, and whose running text answers its attacks (`afterAttack`, the only text that
//!     can ready it) may declare its attacks of a turn again from there, in that stint that turn, and its
//!     stint is `readied`. The state settles the bound where it can: a kill whose attacker the action
//!     leaves on the field with its attack spent, its killing attack its last, readied nothing, and its
//!     credit is taken back.
//!
//! The ten checks:
//!   I1 no sick attack is ever offered (§4.1, §6.1, R83, R171). In the main phase with no prompt
//!      open, a unit of the acting player that entered on this turn has no attack target unless it
//!      has Rush or Charge, and no hero target unless it has Charge; the chosen `attack`, if any, is
//!      checked the same way. It reads `attack_targets`, which is what `legal_actions` enumerates with.
//!   I2 one declared attack per stint per turn, two for a unit that had Windfury in that stint (R636):
//!      seen on a state between actions, or gained inside the action, granted (`keywordGranted`) or
//!      Buffed (`upgraded`, R386), or carried onto the field by an entry inside the action, as the
//!      card was before the action or as the action left it (the autopilot's turn, R1183002).
//!      A unit that re-entered may attack again within its keywords (R83, R171), and a kill credit's fresh
//!      exertion counts its attacks again from the ones declared at the kill (R1869002); forced attacks
//!      are not declarations and are skipped (R53).
//!   I3 every card on the field arrived by an event (§10.3: every visible state change emits one).
//!   I4 the bookkeeping matches the shadow (white-box R171): (a) `summonedTurn` is the turn of the
//!      latest entry, or absent on a `readied` stint (R424, R1839002, R1869002); (b) a spent attack exertion
//!      belongs to the current stint, so an exertion left spent across an entry trips it.
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
//!      when it is a token (R11), judged by the definition its leaving named (a Fuse and then R419's
//!      Rollback can turn a token into a fused card and back, fuzz seed 119), or a card a full library refused before it existed that copies nothing
//!      (R316, fuzz seed 5967 of the handicapped wave, #562); a Glitch's reset or boards (R676, R678)
//!      takes the rest unseen, and the first oracle excused it, which was the engine's own R97 rule
//!      restated: R764 fixed it. In a duel, a card in no pile, a token included, that the log last
//!      shows in a hand was passed out of it (B20) and reads to that hand's player alone: the Duos
//!      fuzz's first run found `view_for` naming a passed card's draw to the passer's opponent (R1074000).
//!      Judging former ids by the log found a sixth, fuzz seed 992: R419's Rollback recreated a
//!      face-down trap #83 had transformed away, and the `formerId` it carries goes with the card
//!      (R227) once that card is public, R177's mark on the old id notwithstanding. An empty id names
//!      no card: a Cry run out of a graveyard runs with none (R127), so the `chaosRolled` its Call to
//!      Chaos sends carries `""` beside the definition (R436; fuzz seed 7232 of the handicapped wave, #1645).
//!      Stinkyscape's rows found another false positive, fuzz seed 4679 of the handicapped wave: a
//!      Starship's memory (R1852005) names the definitions of the pieces fused into it, each a card that
//!      died face-up (R1852000), so they read off a Starship the seat reads.
//!      A seat's own Created card names the card that generated it (`createdBy`, R1956001), which is
//!      read off that card as Echo's `copies` is.
//!      Stinkyscape's S04 found another, fuzz seed 8 and the Duos smoke wave's seed 1: a card sold from a
//!      hand at the Marketplace (S #89 Spice, R1861000) ceases to exist and is no token, yet its public
//!      `marketSold` named it to both seats as it went (R1853003), so a sold id reads, as R11's tokens do.
//!      S21's Marketplace table (R1881006) found another, fuzz seeds 4, 10 and 14: the offering is public
//!      (R1853001), so a lot's definition reads to every seat it is offered to, whatever hidden card shares it.
//!      `I6_GATE_STRIDE` is how often the 1,000-seed gate runs it.
//!   I7 no orphaned work (§9.3, §10.3, R113). After every action, setup's included, a live game with no
//!      prompt open (neither `pending` nor the mulligans, §10.6, R265) owes nothing: the resolution loop
//!      has drained every paused sequence, so `work` is empty and `work_cursor` is back at 0. A game that
//!      ended mid-sequence keeps what it owed (`settle` stops at the result, and nothing runs after it,
//!      I5), so a game over is not checked: the fuzz reaches such game overs (Classic #52 Final
//!      Gambit's `afterRedirect`, Classic #56 Spell Tyrant's `resolve`).
//!   I8 every attack, switch and activation `reduce` accepts is listed (§4.1, §4.2, §9.1, §10.2; #1192): the
//!      server reduces what a client sends without reading `legal_actions`. For the seat that may act, every
//!      `switchPosition` naming a card on either field (a pile's top, a card dormant under it, a Unit a
//!      carrier holds) and every `attack` from a card on its field on any of those or either hero is
//!      built from the board alone, and so is every `activate` of every ability of each of those cards,
//!      crossed with its declared choices (`play_choice_combinations`, R90), and the `activatePower` alias
//!      for the ability it names, judged against the `activate` it stands for (#1574); each the list
//!      lacks is reduced on a copy without the action history (as `scorer::dry_run_base` leaves it:
//!      nothing a refusal reads is in it), and each accepted is reported. Not probed: kinds never listed by design (R345, R923000, R79), and `play` and a
//!      well-formed `answer` (R90's bound lists a slice, #992); a malformed answer is the fuzz's
//!      `malformed-answer` probe (`malformed_answers`, #1574), and a listed play, activation or mulligan
//!      with one choice made impossible its `malformed-action` probe (`malformed_actions`, #1694). Under a Mayor (R1200) a named target is
//!      refused. Its first run, every state of seeds 1–1000 of both waves (2026-10-18), found nothing.
//!      `I8_GATE_STRIDE` is how often the 1,000-seed waves run it.
//!   I9 the wire tells the truth (§10.3: every visible state change emits an event; §10.10: every client
//!      renders from them; #1719). A payload wrong the same way every time folds to the same hash, so the
//!      replay check cannot see it. In the stream, a `turnEnded` or `turnAutoEnded` names the turn under
//!      way, the shadow's `turn` and player (§2.2), and each `fatigue` is its seat's next fatigue draw:
//!      its `count` one past the shadow's `fatigue` for that seat (the start state's count, then one per
//!      `fatigue` seen) and its `amount` `FATIGUE_DAMAGE(count)` (§2.4, R3, R315; #1902). On the state an
//!      action left, its last `turnStarted` names `state.turn` and `state.active` (§2.2), each `gameOver`
//!      is `state.result` (§2.5), each seat's last `manaChanged` is its `mana.current` and `mana.max`
//!      (§2.3): every mana write announces one (the refresh, a gain, a refresh of spent mana, a payment,
//!      a Duos pass, the sandbox's), and each seat's `fatigue_count` is the shadow's count, since a
//!      fatigue draw announces itself even when Armor takes its hit whole (R240, R1362). A Glitch's reset
//!      (R676) starts the game again once the action has settled, at setup, announcing neither a turn nor
//!      mana, so an action that reset the game (`state.resets` moved) has its `turnStarted` and
//!      `manaChanged` left unread: they named the game it took. Its `fatigue` events are still read,
//!      against the game they fell in, and the fatigue shadow starts again from the new game's counts.
//!   I10 no opponent-held prompt on its holder's off turn (§10.6, R1279000; #2568). A prompt the opponent
//!      holds (`by: "enemy"`) never opens while its holder is not the active player: each card defers it
//!      to the start of its holder's next turn by hand, and Pickle, Income Tax and In Too Deep opened
//!      theirs on the wrong turn through v0.3.16 with nothing to see it. After every action of a live
//!      game, a `pending` whose `player_id` is not `state.active`, and whose `prompt_owner_of` (the asking
//!      card's controller, which `open_prompt` records when the player asked is someone else) is another
//!      player, is a violation. A prompt held by the card's own controller opens on either turn (In Too
//!      Deep's reward choice), an engine prompt records no owner, and the mulligans are no `pending`.
//!      The message names the asking card and the turn.
//!
//! Every message leads with its id, names the instance, its def and the turn, and cites the SPEC
//! reference, so the fuzz report's `signatureOf` groups one bug into one entry.
//!
//! Port of `packages/cards/test/_invariants.ts` (SURFACE §8: `testkit::invariants::InvariantMonitor`). The
//! JSON a check walks is serde's, whose objects iterate in key order where TS's iterated in insertion
//! order, so where one needle sits at two paths the path a message names may be the other one; which
//! needles are found is the same.

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::{IndexMap, IndexSet};
use serde_json::{Map, Value};

use crate::config::{FATIGUE_DAMAGE, MIN_PLAGUE_PAYMENT, WINDFURY_ATTACKS};
use crate::damage::DamageTarget;
use crate::duos::{DuosState, duos_view_for};
use crate::play_choices::PlayAction;
use crate::prompts::prompt_owner_of;
use crate::state::{CardInstance, GameState, PendingChoice, find_instance};
use crate::subsystems::activate::ActivateAction;
use crate::wire::{
    Action, ActionBody, CardType, DUOS_DUELS, DUOS_SEATS, DuosActionBody, DuosEvent, DuosSeat, DuosView,
    Enchantment, GameEvent, GameResult, KeywordKind, PLAYER_IDS, PerPlayer, Phase, PlagueSpend, PlayerId,
    PlayerView, PromptKind, RedirectWhat, RevealAt, Row, Selection, Team, TuningChange, Zone, ZoneChoice,
    ZoneName, has_keyword, opponent_of,
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

/// I8 under the 1,000-seed waves: every this-many-th state (#1192); the test sweep checks every state.
///
/// Measured on 2026-10-18 (2 cores, idle), seeds 1–1000: `cargo jackioh fuzz` 103 s before I8,
/// 104 s with this stride (+1%); `--handicap` 130 s, 129 s. The planner (2026-10-18, four
/// cores): 36 s with no monitor, 52 s with I1–I7 (×1.45, so the 25% budget is spent before I8; the
/// figures above are the TS gate's); I8 on every state, reducing on the full state, 314 s and 417 s.
pub const I8_GATE_STRIDE: usize = 100;

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

/// Every card the state holds, in every pile (`find_instance`'s piles) and beneath a holder (R1850003).
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
    /// R11, R97: the definition each id last left a zone as (`destroyed`, `exiled`, `burned`,
    /// `discarded`, `bounced`), which is what a card that ceased to exist was as it went. A card's
    /// definition can change and change back (R77's Fuse, then R419's Rollback restoring the token
    /// it was, fuzz seed 119), so its leaving names it better than every definition it ever wore.
    gone_as: IndexMap<String, String>,
    /// R1074000: in a duel, each id whose last place the log shows is a hand, and that hand's player. A
    /// pass (B20) takes a hand card out of its duel with no event, so one in no pile was passed. A unit
    /// token bounced or stolen off the field or out of the resolving zone reached no hand (R11).
    in_hand: IndexMap<String, PlayerId>,
    /// R1853003, R1861000: each id a `marketSold` names. A sale is public and the card ceases to exist
    /// as it goes, so it read to both seats then, a hand card's as much as a tray Resource's.
    sold: IndexSet<String>,
}

/// The card an event's id field names. R127: a Cry run out of a graveyard runs with no card, and the
/// `chaosRolled` it sends names none by an empty `instanceId` (R436), which is no card's id.
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
            // goes with it, a trap's that fired into exile included (fuzz seed 992).
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

/// The events that take a card out of a zone and name the definition it left as (`Lineage::gone_as`).
const LEAVING_EVENTS: [&str; 5] = ["destroyed", "exiled", "burned", "discarded", "bounced"];

/// Where a card an id names stands for one viewer (`standing_of`'s answer).
struct Standing {
    reads: bool,
    where_: String,
}

/// Whether this viewer reads the card an id names, judged by where the card is now: a card the state
/// holds reads unless the hidden set has it, and a vanished one by its successor. A vanished card with
/// none reads when a full library refused it before it existed and it copies nothing (R316) or when a
/// public sale took it (R1853003, R1861000), and otherwise only when every definition the log pairs
/// with it is a token's: R11's tokens go public
/// (leaving the field, discarded, burned), where any other card in no pile went unseen, out of a
/// Glitch's reset or boards (R676, R678), and stays as hidden as it was. In a duel, one the log last
/// shows in a hand, a token's included, was passed out of it (B20) and reads to that hand's player
/// alone (R1074000). An id neither the state nor the log knows vouches for nothing.
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
            // R316: a card a full library refused before it existed, copying nothing, was never hidden.
            if lineage.never_made.contains(id) {
                return Some(Standing {
                    reads: true,
                    where_: "no pile (it was never created)".to_string(),
                });
            }
            // R1853003, R1861000: a card sold at the Marketplace was named to both seats as it went.
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
/// no longer holds, because a Glitch's reset (R676) dealt the game again without its `transient_defs`
/// (fuzz seed 8183, #1216), is rebuilt from its id alone as C+ #29 rebuilds one (R179), on a scratch
/// copy; one that cannot be rebuilt (a digest, R468) counts as no token, so it stays hidden.
fn is_token_def(state: &GameState, def_id: &str, viewer: PlayerId) -> bool {
    if let Some(def) = crate::catalog::find_def(Some(state), def_id) {
        return def.token;
    }
    let mut scratch = state.clone();
    crate::subsystems::fuse::rebuild_fused_def(&mut scratch, def_id, viewer).is_some_and(|def| def.token)
}

/// The hidden set, worked out from the state's zones on their own terms and never from `view_for`'s
/// rules (the oracle must not be the fix restated): the other seat's hand and Resource tray while the
/// game is live (R434, R1840000), both libraries (§9.1), the other seat's face-down traps (dormant
/// ones under a backrow top included, B5 E21), a card the other seat is setting face-down (R448), mulligan returns waiting for
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
            Selection::Craft { recipe } => {
                // R880: a `craft` prompt's option key names the recipe's definition, which the
                // holder reads with the offered recipe (§10.8) — even when the rival's hand
                // holds a card crafted from the same recipe (fuzz seed 1708, #1216).
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
        // `enchantmentsOfKind(card, "swapsBook")`.
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
        events.push(shown);
    }
    // `defs` (R243) is left out: a definition is copied only for an id the rest of the view names, and
    // its body names its ingredients and its `refs` (R279), the text of cards the viewer reads.
    let mut scanned = serde_json::to_value(view).expect("a PlayerView serializes");
    if let Some(object) = scanned.as_object_mut() {
        object.remove("defs");
        object.insert("events".to_string(), Value::Array(events));
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
    // R860: the opponent's unrevealed choices reach neither seat's view of the other side. A
    // `choice: Some` on the opponent's entry of the view, while its record is not revealed, is
    // the choice leaking through `secret_views`.
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
/// for it: every ability the card has (`abilities_of`) crossed with the target and mode combinations it
/// declares (`play_choice_combinations`, the list's own bound, R90), spelled as `legal_actions` spells
/// one (the ability named, a Tribute paid with the first of `player`'s units other than the card, an
/// empty pick list while a Mayor acts as none, R1200); and for the ability an `activatePower` names (the
/// card's power, or its only ability, `reduce::activate_card`), each combination with no mode and no
/// Tribute as that alias too (§10.2, R43, R384).
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

/// I8 for one seat: every `switchPosition`, `attack`, `activate` and `activatePower` built from the board
/// alone that `legal` does not hold and `reduce` accepts (§4.1, §4.2, §9.1, §10.2). Empty when clean.
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

/// #1574: `answer`, a listed answer to the open prompt, made malformed every way `prompts::why_answer_refused`
/// refuses, each with the seat that sends it: another prompt's id, the holder's opponent, fewer picks
/// than `min`, a pick twice, a pick no option offers (§10.6, R221). Each is malformed by construction,
/// never merely unlisted, since R90's bound lists only a slice of the answers. Empty with no prompt open
/// (a mulligan has its own action) or when `answer` is not an answer to it.
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
/// The timing `unoffered-reveal` asks for: since patch v0.3.7 a card is set face-down by its Cloaked
/// alone (R783004, R1044), so `alt_play::face_down_options` offers no card this one.
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

/// #1694: `chosen`, an action `legal_actions` lists `player`, with one declared choice made impossible,
/// each with the seat that sends it: a `play` naming a target, a mode, a zone, a Tribute, a Plague source
/// or a face-down timing nothing offers, or a negative X (§10.5 step 1, R81, R90,
/// `play_choices::why_choices_refused`, R454); an `activate` naming an ability the card lacks, a target,
/// a mode or a Tribute nothing offers, and its `activatePower` alias aimed at nothing (R384,
/// `activate::why_activate_refused`); a `mulligan` keeping a card the seat was not dealt, and a second
/// answer from the other seat once it has answered (R265, `setup::why_mulligan_refused`). Each is
/// impossible by construction, never merely unlisted, since R90's bound lists only a slice of the legal
/// combinations. Empty for any other action (an answer is `malformed_answers`').
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

/// The turn and stint of an instance's latest declared attack, and how many it has declared in them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AttackMark {
    turn: i32,
    stint: i32,
    count: i32,
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
/// no transform, but its kill may earn a kill credit (R1869002).
#[derive(Clone, Debug, PartialEq, Eq)]
struct ShadowAttack {
    attacker_id: String,
    target_id: String,
    killed: bool,
    forced: bool,
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

fn name_of(card: &CardInstance) -> String {
    format!("{} ({}, {}'s)", card.id, card.def_id, card.controller)
}

/// The keywords a unit has now, through the layers (`layers::unit_view`).
fn keywords_now(state: &GameState, card: &CardInstance) -> Vec<crate::wire::Keyword> {
    crate::layers::unit_view(state, card).keywords.clone()
}

/// TS `InvariantMonitor`, made by `create_invariant_monitor`: the shadow of this file's header and
/// the checks run against it. `before` is I1 and I3, `after` feeds one action's events into the
/// shadow and runs I2, I4, I5, I7, I9 and I10, `hidden` is I6 for both seats and `unlisted` is I8.
#[derive(Clone, Debug)]
pub struct InvariantMonitor {
    turn: i32,
    /// The player whose turn it is, from `turnStarted`: R1062's start-of-turn animation is theirs.
    active: PlayerId,
    entered: IndexMap<String, i32>,
    stint: IndexMap<String, i32>,
    last_attack: IndexMap<String, AttackMark>,
    /// R636: the stint in which each instance was last seen with Windfury. Read from the states between
    /// actions and after each one, so a unit that dies on its second attack was seen with it before —
    /// and from `keywordGranted` events and a Buff's keyword row (`upgraded`, R386) inside the action, so
    /// a unit given Windfury mid-action is seen before its declarations: R44's AI turn can summon a
    /// Conjure token and fight with it twice inside one outer action, where no between-action state ever
    /// holds it, or Buff an attacker My Pawn stopped into Windfury and attack with it again (R636).
    windfury: IndexMap<String, i32>,
    /// R636: the cards off the field that carry Windfury as the action begins, in what they are made
    /// of (`layers::card_keywords`: the face, its tuning, a grant), read by `before`. A unit that
    /// enters with it is seen with it in the stint it enters, so one that enters and declares twice
    /// inside one action, which no state between actions holds on the field, is within its keywords:
    /// the autopilot plays a whole turn in one action (R1183002; fuzz seeds 5325, 6431, 9388, and 6732
    /// and 7295 of the handicapped wave, #1216).
    carried_windfury: IndexSet<String>,
    /// R636, R781004: the units `note_drawn_windfury` read as maybe Windfury, whose drawn keywords a
    /// change of control keeps (the card stays on the field).
    drawn_windfury: IndexSet<String>,
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
    /// R424, R1869002: whether the last action left a prompt or work owed, so a combat it declared may
    /// still be resolving (an ambush's Discovers, S #22) and its attack is kept for the next action.
    attack_open: bool,
    /// I9: `state.resets` as the last state `after` read left it; a move is a Glitch's reset (R676).
    resets: Option<i32>,
    /// I9: each seat's fatigue draws as the stream counted them (§2.4, R3, R315): the start state's
    /// `fatigue_count`, one more per `fatigue` event, and the new game's count after a Glitch's reset (R676).
    fatigue: PerPlayer<i32>,
    /// R1829000: the cards dormant beneath a pile's top, either row, in the last state read.
    dormant: IndexSet<String>,
}

pub fn create_invariant_monitor(start: &GameState) -> InvariantMonitor {
    InvariantMonitor::new(start)
}

impl InvariantMonitor {
    /// `createInvariantMonitor(start)`: a shadow that starts on `start`'s turn with nothing entered.
    pub fn new(start: &GameState) -> InvariantMonitor {
        InvariantMonitor {
            turn: start.turn,
            active: start.active,
            entered: IndexMap::new(),
            stint: IndexMap::new(),
            last_attack: IndexMap::new(),
            windfury: IndexMap::new(),
            carried_windfury: IndexSet::new(),
            drawn_windfury: IndexSet::new(),
            fired: IndexSet::new(),
            readied: IndexSet::new(),
            kill_credits: IndexMap::new(),
            unsettled_credits: IndexSet::new(),
            attack: None,
            attack_open: false,
            resets: start.resets,
            fatigue: start.players.map(|_, side| side.fatigue_count),
            dormant: dormant_cards(start),
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

    /// R636, R781004: a unit whose text draws its keywords as it arrives or fuses (M #65 Keymaster
    /// Keenus' Aura, the Stinkyscape Drones', a `conditionalKeywords` text) keeps them in its memory,
    /// which leaving the field resets (R78) and ceasing to exist takes with it. One gone from the field
    /// by the action's end leaves no state to say whether it drew Windfury, so its attacks in this stint
    /// count as Windfury's (Keenus equipped onto C+ #26 and exiled: Duos fuzz seed 679; a Drone
    /// Corvette summoned and killed inside one autopilot action: fuzz seed 8567).
    fn note_drawn_windfury(&mut self, id: &str, def_id: &str, state: &GameState) {
        let draws = match find_instance(state, id) {
            Some(card) if card.zone.z() == ZoneName::Field => false,
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

    /// R636: a unit that just entered and carries Windfury, as it was before the action
    /// (`carried_windfury`) or as the action left it, has it in this stint.
    fn note_entry_windfury(&mut self, id: &str, state: &GameState) {
        let left_with = find_instance(state, id).is_some_and(|card| {
            let keywords = if card.zone.z() == ZoneName::Field {
                keywords_now(state, card)
            } else {
                crate::layers::card_keywords(state, card)
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
        // R1869002: a kill credit still unsettled (a prompt the action left open, a steal that gave the
        // attacker a new stint before its `afterAttack` freed it: handicapped fuzz seed 4737) may have
        // readied it; `settle_kill_credits` takes it back once nothing is owed if its attack stayed spent.
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

    /// R1869002: a credited attacker the action leaves on the field, its killing attack its last, with
    /// its attack exertion spent was readied by nothing: its credit is taken back. One it readied is
    /// `readied` for the rest of its stint, as `mayAttackAgain` leaves it with no `summonedTurn`. Settled
    /// once nothing is owed, since an `afterAttack` a prompt paused runs in a later action (R113).
    fn settle_kill_credits(&mut self, state: &GameState) {
        if state.pending.is_some() || !state.work.is_empty() {
            return;
        }
        for id in std::mem::take(&mut self.unsettled_credits) {
            let spent = find_instance(state, &id)
                .is_some_and(|card| card.zone.z() == ZoneName::Field && card.exertion.attacked);
            if !spent {
                self.readied.insert(id);
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

    /// R1062: an "Animated on your turn" card whose latest entry was on an earlier turn enters nothing as it
    /// animates at its controller's start of turn. Read off the shadow and the card's keywords, never `summonedTurn`.
    /// `side` is the side it animated on: on the other player's turn (R1126's Cane) it is an entry, R383's.
    /// The keyword is read as the animation had it: `later`, the action's events after it, may change it
    /// before the state is read (a Knowledge Breaker the autopilot plays on that turn Nerfs it off the
    /// Cane, fuzz seed 2443), and the first such change says what it was.
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
        let had = changed
            .unwrap_or_else(|| has_keyword(&keywords_now(state, card), KeywordKind::AnimatedOnYourTurn));
        at < self.turn && had
    }

    /// R1829000: the entries one action made by uncovering cards in their Stack piles, which no event
    /// reports (R212). A card dormant in the last state read that acts now entered inside the action,
    /// on a turn from `began_on` to the action's last: `summonedTurn` says which, and when it names none
    /// of them the entry is the last, so I4 reports it. Not on a side a `rolledBack` restored (R566),
    /// nor for the card a fusion kept (R77, R1086). A card that may have been uncovered inside the action
    /// without that showing on the piles (dormant before, or in a zone an entry event named or a play
    /// was announced into, which a Stack, Equip or stack-base play covers and the rest of the action may
    /// uncover, followed one step through a rotation the action made, R14) takes `summonedTurn`'s word
    /// only when it names a turn of the action.
    fn note_uncoverings(&mut self, events: &[GameEvent], state: &GameState, began_on: i32) {
        let mut restored: Vec<PlayerId> = Vec::new();
        let mut landed: Vec<(PlayerId, Row, i32)> = Vec::new();
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
                    player, row, lane, ..
                } => landed.push((*player, *row, *lane)),
                GameEvent::ControlChanged {
                    controller,
                    row,
                    lane,
                    ..
                } => landed.push((*controller, *row, *lane)),
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
            let may_have_been = was_dormant || landed.contains(&(player, row, lane));
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

    /// I1 and I3 on the state the next action is chosen in. Empty when clean.
    pub fn before(&mut self, state: &GameState, player: PlayerId, action: &ActionBody) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        self.note_windfury(state);
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

        // R1202: declaring spends the declarer's exertion before its substitute is even summoned,
        // so a substitute barred from the target spends with no declaration following (fuzz seed
        // 2550, #1216). Pre-record the declaration with no attacks counted yet, so I4 reads the
        // spend as this stint's; I2 still counts only the declarations the events carry, since a
        // first real declaration counts one from zero as from none, and a stint that already
        // declared keeps its count.
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

    /// Feeds one action's events into the shadow, then I2, I4, I5, I7, I9 and I10 against the resulting state.
    pub fn after(&mut self, events: &[GameEvent], state: &GameState) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        // R1829000: the turn the action began on; an uncovering inside it happened on a turn from here on.
        let began_on = self.turn;
        // R424: the Golem's transform follows its combat inside the one attack action, so an attack the
        // record holds from an earlier action readies nothing (a later Transmogulate of that unit is not it).
        // An action that left a prompt or work owed is the attack's own still resolving (R1869002: a kill
        // an ambush's Discovers delayed credits its attacker when it lands), so the attack is kept then.
        if !self.attack_open {
            self.attack = None;
        }

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

        for (index, event) in events.iter().enumerate() {
            match event {
                GameEvent::TurnStarted { player, turn, .. } => {
                    self.turn = *turn;
                    self.active = *player;
                    self.attack = None;
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
                GameEvent::CardPlayed { instance_id, .. }
                | GameEvent::Summoned { instance_id, .. }
                | GameEvent::ControlChanged { instance_id, .. } => {
                    self.enter(instance_id);
                    self.note_entry_windfury(instance_id, state);
                    if let GameEvent::Summoned { def_id, .. } = event {
                        self.note_drawn_windfury(instance_id, def_id, state);
                    }
                    // R12, R781004: a steal keeps the card on the field, its drawn keywords with it
                    // (a Drone Corvette summoned, stolen and bounced inside one AI turn: handicapped
                    // fuzz seed 1358).
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
                // action is over entered on this turn (a Glitch played from the graveyard mid-game:
                // handicapped fuzz seed 7613 of the Stinkyscape release).
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
                    // R383: moving into the unit row is entering it on that turn; a carried Unit stepping
                    // down off its carrier (C+ #33, carriers.rs) was a Unit on the field all along and enters nothing.
                    // ME-ALTPLAY, R1041: a set Unit keeps its set turn as `summonedTurn`, so its reveal
                    // animation enters nothing either. R1062: nor does an "Animated on your turn" card
                    // animating at its controller's start of turn that has been on that side since the
                    // turn began; on the other player's turn (R1126) it enters, as R383 has it. One that
                    // fired as a trap this turn animates as its firing's last step, not at the start of
                    // the turn, and enters even then (fuzz seed 5644, #1216: a face-down Field Trap
                    // fused with Fiery Waraxe, set on an earlier turn, fired on its controller's turn).
                    if *carried != Some(true)
                        && (self.fired.contains(instance_id)
                            || !self.on_its_side_before_this_turn(
                                instance_id,
                                *player,
                                state,
                                &events[index + 1..],
                            ))
                    {
                        let is_unit_def = find_instance(state, instance_id).is_some_and(|card| {
                            crate::catalog::def_of(Some(state), &card.def_id).type_
                                == crate::wire::CardType::Unit
                        });
                        if !is_unit_def {
                            self.enter(instance_id);
                            self.note_entry_windfury(instance_id, state);
                        }
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
                    if let Some(attack) = self.attack.as_mut()
                        && attack.target_id == *instance_id
                        && killer_id.as_deref() == Some(attack.attacker_id.as_str())
                        && !attack.killed
                    {
                        attack.killed = true;
                        // R1869002: a kill readies its attacker only through its own `afterAttack` text.
                        let attacker_id = attack.attacker_id.clone();
                        let answers = find_instance(state, &attacker_id).is_some_and(|attacker| {
                            crate::scripts::script_of(state, attacker).after_attack.is_some()
                        });
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
                // R636, R1086: a Fuse (an Equip among them) brings its ingredients' text, and with it a
                // Windfury an aura of theirs gives, which no `keywordGranted` reports; a unit that holds
                // it as the action leaves it has it from the fuse on (M #65 Keymaster Keenus' random
                // keywords, equipped onto S #113: handicapped fuzz seed 7566), and one gone by the action's
                // end may have drawn it (`note_drawn_windfury`).
                GameEvent::Fused {
                    result_instance_id,
                    def_id,
                    ..
                } => {
                    self.note_entry_windfury(result_instance_id, state);
                    self.note_drawn_windfury(result_instance_id, def_id, state);
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
                    // multiplied row adding `also` beside it. Fuzz seed 7758 of the handicapped wave: My
                    // Pawn canceled an attack, R44's random rest of the turn Buffed the attacker into
                    // Windfury, and it attacked once more inside the one action, as R636 allows.
                    let mut gained = std::iter::once(keyword).chain(also.iter().flatten());
                    if gained.any(|keyword| keyword.kind() == KeywordKind::Windfury) {
                        let stint = self.stint_of(instance_id);
                        self.windfury.insert(instance_id.clone(), stint);
                    }
                }
                GameEvent::AttackDeclared {
                    attacker_id,
                    target_id,
                    forced,
                    instead_of,
                } => {
                    // R53: a forced attack is not a declaration and spends nothing (nor readies a
                    // transform, R424); its kill may still earn a kill credit (R1869002).
                    self.attack = None;
                    // R1869002: a credited attacker declaring again inside the action was readied.
                    if self.unsettled_credits.shift_remove(attacker_id) {
                        self.readied.insert(attacker_id.clone());
                    }
                    if *forced {
                        self.attack = Some(ShadowAttack {
                            attacker_id: attacker_id.clone(),
                            target_id: target_id.clone(),
                            killed: false,
                            forced: true,
                        });
                        continue;
                    }
                    // R1202: a substitute's declaration spends its declarer's attack, not its own —
                    // Windfast's exertion, Windfast's Windfury count — while the strike itself is the
                    // substitute's to answer for.
                    let declarer = instead_of.as_deref().unwrap_or(attacker_id);
                    let current = self.stint_of(declarer);
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
                    let per_turn = if self.windfury.get(declarer) == Some(&current) {
                        WINDFURY_ATTACKS
                    } else {
                        1
                    };
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
                    });
                }
                _ => {}
            }
        }

        self.note_windfury(state);
        self.settle_kill_credits(state);

        // I9: what the action announced is the state it left (§10.3); a Glitch's reset moves `resets` (R676).
        let reset = state.resets != self.resets;
        self.resets = state.resets;
        found.extend(InvariantMonitor::announced_state(events, state, reset));
        found.extend(self.counted_fatigue(state, reset));

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

        // I7: a live game with no prompt open owes no paused sequence (§9.3, §10.3, R113).
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

        // I10: no prompt held by the opponent while its holder is not the active player (§10.6, R1279000).
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

    /// I8 on a state the fuzz reached, for the seat that may act: every unlisted attack, switch and
    /// activation `reduce` accepts. Empty when clean.
    pub fn unlisted(&self, state: &GameState) -> Vec<String> {
        let Some(player) = crate::reduce::seat_to_act(state) else {
            return Vec::new();
        };
        let legal = crate::reduce::legal_actions(state, player);
        unlisted_accepted(state, player, &legal)
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
        DuosInvariantMonitor {
            monitors: [
                InvariantMonitor::new(&start.duels[0]),
                InvariantMonitor::new(&start.duels[1]),
            ],
            view_tamper: None,
        }
    }

    pub fn duel_monitor_mut(&mut self, duel: usize) -> &mut InvariantMonitor {
        &mut self.monitors[duel]
    }

    /// I1 and I3 on the state the next Duos action is chosen in.
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
        violations
            .into_iter()
            .map(|v| format!("duel {duel}: {v}"))
            .collect()
    }

    /// Feeds one action's events into the monitors, asserts I2, I4, I5, I7, I9 and I10 against both duels,
    /// and checks pool bounds (§13.4) for both teams.
    pub fn after(&mut self, events: &[DuosEvent], state: &DuosState) -> Vec<String> {
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

    /// I6 for all seats: both duels' 1v1 hidden checks, plus `duos_view_for` redaction for each seat.
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

        // Initially clean
        assert_eq!(check_duos_view_redaction(&view), Vec::<String>::new());

        // Enemy ended leak
        view.enemy.ended = vec![true, false];
        let violations = check_duos_view_redaction(&view);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("enemy ended leaked"));

        // Enemy passes_left leak
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

        // Clean initially
        assert_eq!(monitor.after(&[], &state), Vec::<String>::new());

        // Overflow team A pool
        state.teams.a.pool = 25;
        let findings = monitor.after(&[], &state);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("pool bound: team a pool 25 > cap 20"))
        );

        // Reset pool and test live game with pool <= 0
        state.teams.a.pool = 0;
        let findings = monitor.after(&[], &state);
        assert!(
            findings
                .iter()
                .any(|f| f.contains("pool bound: live game has team a pool 0 <= 0"))
        );
    }
}
