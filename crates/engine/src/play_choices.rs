//! The choices a play carries, built and validated (SPEC §10.5 step 1, §10.6, R81, R90).
//!
//! R81: "Zone, X, embiggen, Tribute and the targets and modes a card's script declares travel in the
//! `play` action, which `legalActions` enumerates". This module is the one place those five kinds of
//! answer are turned into options and checked against what the card declared and what the board
//! allows, so `reduce`'s play path and `legal_actions` read the same rules:
//!
//!   * `legal_zones_for`, `legal_x_values`, `legal_embiggen_choices`, `legal_tribute_sets` — the play's
//!     own cost-and-placement choices;
//!   * `declared_targets` / `declared_modes` — what the card's script asked for;
//!   * `legal_selections_for` — every selection one `TargetDecl` filter admits, in a deterministic
//!     order (the chooser's side first, lane order within a side);
//!   * `play_choice_combinations` / `play_actions_for` — R90's enumeration, bounded;
//!   * `why_choices_refused` — the single refusal, returning `Ok` when every choice is legal.
//!
//! R90 is the governing ruling for the validation: several declarations read the flat `targets` list
//! in order, each taking its own minimum and the last one the remainder; one declaration may not
//! name the same card twice while two declarations may both name the same card; a declaration the
//! board cannot satisfy does not refuse the play, it fizzles on resolution — unless the play needs it
//! (`required`, R703), and then the play is neither offered nor accepted. §9.1 is why a `hand` pick
//! only ever offers the chooser's own hand, and R13 is why a unit pick only offers the top of a
//! Stack pile.
//!
//! §6.2's Stack row is why the zone question has two answers rather than one: "may be played onto an
//! occupied zone", so a Stack card's legal zones are the unit zones that are merely unlocked and
//! unreserved (R64), not the empty ones. `legal_zones_for` and `refuse_zone` read that off the same two
//! predicates on purpose — the client's greyed-out button and `reduce`'s refusal are the same rule
//! asked from opposite directions, and a Stack play that only one of them knew about would be a
//! board the client cannot reach or a refusal it cannot explain.
//!
//! Port of `packages/engine/src/playChoices.ts`. A function that reads a card's script takes the state
//! first (`declared_targets(state, card)`, `tribute_cost_of(state, card)` …), since a script is looked
//! up through the state (`scripts::script_of(state, def_id)`, SURFACE §6.6); a TS default parameter is
//! an `Option` (`None` is the default) or, for a list, an empty slice. Refusals are
//! `Result<(), EngineError>` with TS's text verbatim (SURFACE §4.4.9).

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{MAX_CHOICE_COMBINATIONS, MIN_CHOSEN_X};
use crate::graveyard_play::PlayPayment;
use crate::rng::Rng;
use crate::script::TargetCheckArgs;
use crate::state::{CardInstance, EngineError, GameState};
use crate::wire::{
    ActionBody, CardCost, CardType, FilterOf, FilterSide, KeywordKind, ModeDecl, PlagueSpend, PlayerId,
    PromptKind, RevealAt, Row, Selection, Tag, TargetDecl, TargetFilter, Zone, ZoneChoice, opponent_of,
};
use crate::zones::ZoneSlot;

// §7: a Radiant Sheep Token is "2/2, worth 3 Tributes" — the base one is worth 2. The numbers live in
// `config.rs` (CLAUDE.md rule 9); re-exported so TS's `playChoices.SHEEP_TRIBUTE_VALUE` path resolves.
pub use crate::config::{RADIANT_SHEEP_TRIBUTE_VALUE, SHEEP_TRIBUTE_VALUE};

/// The `play` member of the action union, without the `playerId` and `nonce` the caller adds (TS
/// `Extract<ActionBody, { type: "play" }>`): the fields of `ActionBody::Play`, as a struct so a reader
/// names them (`action.targets`). It serialises as that action body, `"type": "play"` included.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[serde(into = "ActionBody", try_from = "ActionBody")]
pub struct PlayAction {
    pub instance_id: String,
    pub zone: Option<ZoneChoice>,
    pub x: Option<i32>,
    pub embiggen: Option<bool>,
    /// R1086: play this Magnetic card onto the zone's host Unit (ME-MAGNETIC).
    pub magnetic: Option<bool>,
    /// Units sacrificed to pay a Tribute cost (§6.3).
    pub tributes: Option<Vec<String>>,
    pub targets: Option<Vec<Selection>>,
    pub modes: Option<Vec<String>>,
    /// B5 E11, E19: Plague Counters paying part of the price of a play from the graveyard.
    pub plague: Option<PlagueSpend>,
    /// ME-ALTPLAY, R1040, R1044: play face-down as a Trap revealing at this timing.
    pub face_down: Option<RevealAt>,
}

impl PlayAction {
    /// The `play` action body this is.
    pub fn into_body(self) -> ActionBody {
        self.into()
    }

    /// The play an action body is, or `None` for any other action.
    pub fn from_body(body: &ActionBody) -> Option<PlayAction> {
        PlayAction::try_from(body.clone()).ok()
    }
}

impl From<PlayAction> for ActionBody {
    fn from(action: PlayAction) -> ActionBody {
        ActionBody::Play {
            instance_id: action.instance_id,
            zone: action.zone,
            x: action.x,
            embiggen: action.embiggen,
            magnetic: action.magnetic,
            tributes: action.tributes,
            targets: action.targets,
            modes: action.modes,
            plague: action.plague,
            face_down: action.face_down,
        }
    }
}

impl TryFrom<ActionBody> for PlayAction {
    type Error = String;

    fn try_from(body: ActionBody) -> Result<PlayAction, String> {
        match body {
            ActionBody::Play {
                instance_id,
                zone,
                x,
                embiggen,
                magnetic,
                tributes,
                targets,
                modes,
                plague,
                face_down,
            } => Ok(PlayAction {
                instance_id,
                zone,
                x,
                embiggen,
                magnetic,
                tributes,
                targets,
                modes,
                plague,
                face_down,
            }),
            other => Err(format!("not a play action: {}", other.action_type())),
        }
    }
}

/// One answer to everything a card's script declared (R81): the `targets` and `modes` of a play.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayChoices {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<Selection>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modes: Option<Vec<String>>,
}

// R90's enumeration bound is `MAX_CHOICE_COMBINATIONS` in `config.rs` (BUILD §2): "Choose 2 or 3"
// on a full board is 165 combinations, and the client only needs enough of them to offer every
// picker, so the target and mode enumerations below stop at the cap rather than growing with the
// board — and a cut drops combinations, never a pick (`cross_product`, `interleaved`). A Tribute's
// paying sets are the exception: they are the play's price, listed whole (`legal_tribute_sets`).

/// §7: the Sheep Token, which is "worth 2 Tributes while on the field" (§3.2, §6.3).
pub const SHEEP_TOKEN_INDEX: &str = "T-sheep";

const ROWS: [Row; 2] = [Row::Units, Row::Backrow];

/// The pick kinds a `TargetFilter.of` may name, in the order selections are offered in.
const PICK_KIND_ORDER: [FilterOf; 6] = [
    FilterOf::Unit,
    FilterOf::Backrow,
    FilterOf::Hand,
    FilterOf::Graveyard,
    FilterOf::Zone,
    FilterOf::Hero,
];

/// The name a card's definition prints.
fn name_of(state: &GameState, def_id: &str) -> String {
    crate::catalog::def_of(Some(state), def_id).name.clone()
}

/// `min(n, count)` for a declaration's `min`/`max` read against a count of options (TS numbers).
fn at_most(n: i32, count: usize) -> usize {
    (n.max(0) as usize).min(count)
}

// ---------------------------------------------------------------------------
// What the card asked for
// ---------------------------------------------------------------------------

/// The `targets` a card's running face declares (§10.9, R81); the empty list when it declares none.
pub fn declared_targets(state: &GameState, card: &CardInstance) -> Vec<TargetDecl> {
    crate::scripts::script_of(state, card).targets.clone()
}

/// The `modes` a card's running face declares: "Choose one" and Silly Silas's direction (R81).
pub fn declared_modes(state: &GameState, card: &CardInstance) -> Vec<ModeDecl> {
    crate::scripts::script_of(state, card).modes.clone()
}

/// The target declarations a play with these modes answers (R90, §8 Conventions). A declaration that
/// belongs to some modes only (`forModes`, #24's damage and heal target) asks for nothing when none
/// of them was chosen, so it takes no slot of the flat `targets` list — and one that does belong to
/// the chosen mode asks for its minimum like any other, which is what stops a damage mode from
/// naming nobody while a target exists.
pub fn active_target_decls(decls: &[TargetDecl], modes: &[String]) -> Vec<TargetDecl> {
    decls
        .iter()
        .filter(|decl| match &decl.for_modes {
            None => true,
            Some(for_modes) => for_modes.iter().any(|mode| modes.contains(mode)),
        })
        .cloned()
        .collect()
}

/// Whether any declaration depends on the modes, so the modes are chosen before the targets.
pub fn targets_follow_modes(decls: &[TargetDecl]) -> bool {
    decls.iter().any(|decl| decl.for_modes.is_some())
}

// ---------------------------------------------------------------------------
// The face a play resolves with (§10.5 step 3, R213, R214)
// ---------------------------------------------------------------------------

/// Every permanent that acts for this player: the tops of their unit piles and their backrow (§3.2).
fn permanents_of(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    let mut out: Vec<&CardInstance> = Vec::new();
    for row in ROWS {
        for slot in crate::zones::slots_of(player, row) {
            if let Some(held) = crate::zones::card_at(state, slot) {
                out.push(held);
            }
        }
    }
    out
}

/// §8 #64 Gifted Program, §10.5 step 3: whether a play this player makes now, paying `cost_paid`
/// (R56's cost actually paid, 0 for a cast, R70), is made Radiant as it is played.
///
/// R213: "the first card costing 1 or less YOU play each turn" is counted over the player's plays,
/// which the turn log keeps (`costsPaid`), and not over the Field Spell's own history. So the count
/// stays with the player whatever happens to the card: a Gifted Program that fired for its owner and
/// was then stolen has not used up its thief's first cheap card, one that was bounced and played
/// again has not given its player a second, and a card costing 1 or less played before a Gifted
/// Program arrived was already the first — Hearthstone's Pint-Sized Summoner counts the same way.
/// Each Gifted Program asks with its own face's threshold (2 or less on the radiant one), and the
/// card's text is its controller's (§8 Conventions), so only the permanents on this player's side
/// count. A Vanilla one has no text (`flags_of`). It never catches its own play: step 3 runs before
/// step 4 puts it on the field (R119).
pub fn gifted_makes_radiant(state: &GameState, player: PlayerId, cost_paid: i32) -> bool {
    let earlier: &[i32] = state.players[player]
        .turn_log
        .costs_paid
        .as_deref()
        .unwrap_or(&[]);
    permanents_of(state, player).into_iter().any(|held| {
        let Some(printed) = crate::scripts::script_of(state, held).flags().gifted_program else {
            return false;
        };
        // R386: the threshold is the card's declared number `giftLimit` where it declares one.
        let threshold = crate::params::declared_or(state, held, "giftLimit", printed);
        if cost_paid > threshold {
            return false;
        }
        !earlier.iter().any(|paid| *paid <= threshold)
    })
}

/// R214: the card whose declarations a play's targets and modes answer — the card as it will
/// resolve. §10.5 step 3 can make it Radiant as it is played (#64), after step 1 has read the choices
/// and before step 5 resolves them, and a Radiant face can declare other choices than the face in
/// hand: #87's "you may skip adding it", #48's "all enemy units, or all units", a crafted card whose
/// radiant Bigot names no target. So step 1 checks, and `legal_actions` offers, the choices of the
/// face step 5 will run, which is known at step 1: the cost it pays is.
pub fn resolving_face(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    cost_paid: i32,
) -> CardInstance {
    let face = if card.radiant || !play_made_radiant(state, player, card, cost_paid) {
        card.clone()
    } else {
        CardInstance {
            radiant: true,
            ..card.clone()
        }
    };
    // B5 E14, R399: a copier (Classic #57 Echo) declares the choices of the Spell whose text it has, on
    // the face that Spell was played on — its own face only adds its Echo (`subsystems/copied_text.rs`).
    crate::subsystems::copied_text::text_face_of(state, &face)
}

/// Classic+ #68 Organic Produce, R449: whether a permanent on this player's side makes every card carrying
/// one of its tags Radiant as the player plays it (`radiantPlaysTagged`) — R213's rule by tag, on every
/// such play (a cast included, R70) rather than the first cheap one. The card's text is its
/// controller's (§8 Conventions), a Vanilla one has none (`flags_of`), and it never catches its own
/// play: step 3 runs before step 4 puts it on the field (R119).
pub fn tagged_play_radiant(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    // MD-B15, R923: a granted tag counts for the play's tags too.
    let tags: Vec<Tag> = crate::query::tags_of(state, card);
    if tags.is_empty() {
        return false;
    }
    permanents_of(state, player).into_iter().any(|held| {
        match crate::scripts::script_of(state, held)
            .flags()
            .radiant_plays_tagged
        {
            None => false,
            Some(wanted) => wanted.iter().any(|tag| tags.contains(tag)),
        }
    })
}

/// §10.5 step 3: whether this play is made Radiant as it is played — #64 Gifted Program's first cheap
/// card (R213), a tag rule's (Classic+ #68), or Meditative #40 Feng Shui's reward for an auspicious
/// play (R983). Step 1 reads it to know the face the play's choices answer (R214), and step 3
/// applies it.
pub fn play_made_radiant(state: &GameState, player: PlayerId, card: &CardInstance, cost_paid: i32) -> bool {
    gifted_makes_radiant(state, player, cost_paid)
        || tagged_play_radiant(state, player, card)
        || crate::subsystems::feng_shui::rewards(state, player, card)
}

/// What a play of this card with these prices would pay, read the way §10.5 step 1 reads it (R65) — as
/// a play wherever the card lies, so a play from a graveyard pays the player's prices (E11, R454).
fn cost_with(state: &GameState, card: &CardInstance, x: Option<i32>, embiggen: Option<bool>) -> i32 {
    let probe = CardInstance {
        x: if chooses_x(state, card) {
            Some(x.unwrap_or(0))
        } else {
            card.x
        },
        embiggened: if has_embiggen_price(state, card) {
            Some(embiggen == Some(true))
        } else {
            card.embiggened
        },
        ..card.clone()
    };
    crate::mana::play_cost(state, &probe)
}

/// #492, R81: what a play of this card at its embiggen price costs now, or None for a card with no
/// embiggen price. It is `cost_with`, the price §10.5 step 1 reads for a play with `embiggen: true`,
/// and so `mana::play_cost` on the card stamped embiggened, as the pay step charges it: the view's
/// `embiggenCost` (§10.8) is this number, so the client never prices a play itself (CLAUDE.md rule 7).
pub fn embiggen_play_cost(state: &GameState, card: &CardInstance) -> Option<i32> {
    has_embiggen_price(state, card).then(|| cost_with(state, card, None, Some(true)))
}

// ---------------------------------------------------------------------------
// Zone, X and embiggen
// ---------------------------------------------------------------------------

/// §5.1: a Unit goes in the unit row, every other permanent in the backrow.
pub fn row_for_card(state: &GameState, card: &CardInstance) -> Row {
    if crate::faces::card_type_of(state, card) == CardType::Unit {
        Row::Units
    } else {
        Row::Backrow
    }
}

/// §10.5 step 4: permanents take a zone, a Spell resolves without one.
pub fn needs_zone(state: &GameState, card: &CardInstance) -> bool {
    crate::faces::card_type_of(state, card) != CardType::Spell
}

/// §6.2 Stack: "may be played onto an occupied zone" (§3.2). Read off the card's keywords as they stand
/// where it is (§10.4, `layers::unit_has`): its face's, the ones granted to it in hand (B5 E38) and the
/// ones an aura gives the cards in a hand (a "Your cards have Stack" aura). A backrow
/// card with Stack tops an occupied backrow zone as a Unit tops a unit zone (B5 E21, R447).
pub fn plays_on_stack(state: &GameState, card: &CardInstance) -> bool {
    // ME-ALTPLAY: a face-down play goes into an open backrow zone, never onto a Stack.
    if card.set_as.is_some() {
        return false;
    }
    if !needs_zone(state, card) {
        return false;
    }
    crate::layers::unit_has(state, card, KeywordKind::Stack)
}

/// §3.2: the zone a Stack card may enter. Occupancy is exactly the refusal Stack lifts, so what is
/// left is what occupancy never covered: a Locked zone "accepts no summons until the game ends", a
/// zone reserved for a dying Reborn unit "counts as occupied for every other card that would enter it"
/// (R64) — and a backrow zone carrying a Unit takes nothing more (R446). `zones::accepts_stack_card`.
fn accepts_stack(state: &GameState, slot: &ZoneSlot) -> bool {
    crate::zones::accepts_stack_card(state, slot, Default::default())
}

/// MD-F12, R1281: whether this Unit may be played onto a stack base.
fn onto_stack_base(state: &GameState, player: PlayerId, card: &CardInstance, slot: &ZoneSlot) -> bool {
    card.set_as.is_none()
        && row_for_card(state, card) == Row::Units
        && crate::zones::stack_base_at(state, player, slot)
}

/// R1086 (ME-MAGNETIC): whether this card may be played as Magnetic — a Unit with the Magnetic
/// keyword that needs a zone. Read off the card's keywords as they stand where it is (§10.4), as
/// `plays_on_stack` is.
pub fn plays_magnetic(state: &GameState, card: &CardInstance) -> bool {
    if !needs_zone(state, card) {
        return false;
    }
    if row_for_card(state, card) != Row::Units {
        return false;
    }
    crate::layers::unit_has(state, card, KeywordKind::Magnetic)
}

/// R1086: the host a Magnetic play onto `slot` fuses into — the top card of the zone when it is a
/// Unit its controller acts with that is not Immutable (R23), and not one this same play tributes.
/// Both the legal-action listing and the refusal read this one function, so the two cannot disagree.
pub fn magnetic_host_at(
    state: &GameState,
    player: PlayerId,
    slot: &ZoneSlot,
    tributes: &[String],
) -> Option<String> {
    if !crate::zones::accepts_stack_card(state, slot, Default::default()) {
        return None;
    }
    let top = crate::zones::card_at(state, slot)?;
    if crate::faces::card_type_of(state, top) != CardType::Unit {
        return None;
    }
    if crate::layers::unit_has(state, top, KeywordKind::Immutable) {
        return None;
    }
    if top.controller != player {
        return None;
    }
    if tributes.contains(&top.id) {
        return None;
    }
    Some(top.id.clone())
}

/// R1086: the unit-row zones that hold a host for a Magnetic play, in lane order. Empty unless the
/// card plays Magnetic at all.
pub fn legal_magnetic_zones_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    tributes: &[String],
) -> Vec<ZoneChoice> {
    if !plays_magnetic(state, card) {
        return Vec::new();
    }
    crate::zones::slots_of(player, Row::Units)
        .iter()
        .filter(|slot| magnetic_host_at(state, player, slot, tributes).is_some())
        .map(|slot| ZoneChoice {
            row: slot.row,
            lane: slot.lane,
        })
        .collect()
}

/// §3.2: "the player picks the zone" — every empty, unlocked, unreserved zone of the right row, plus
/// the occupied zones of that row for a Stack card (§6.2, B5 E21), plus, for a Unit, the backrow zones
/// of a carrier that holds none yet (R446, `zones::carrier_zones_for`). `refuse_zone` below reads the
/// same rules off the same predicates, so the client's greyed-out button and `reduce`'s refusal agree.
///
/// R391 (B4.5): with the Tribute a play pays (`tributes`, empty for none), also every zone that Tribute
/// empties (`freed_by_tribute`), in lane order with the open ones — the zone is judged once the Tribute
/// is paid, since §10.5 pays at step 2 and places at step 4.
pub fn legal_zones_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    tributes: &[String],
) -> Vec<ZoneChoice> {
    if !needs_zone(state, card) {
        return Vec::new();
    }
    let row = row_for_card(state, card);
    let stack = plays_on_stack(state, card);
    let mut slots: Vec<ZoneSlot> = crate::zones::slots_of(player, row)
        .into_iter()
        .filter(|slot| {
            if stack {
                accepts_stack(state, slot)
            } else {
                crate::zones::is_open(state, slot)
                    || freed_by_tribute(state, slot, tributes)
                    || onto_stack_base(state, player, card, slot)
            }
        })
        .collect();
    if row == Row::Units {
        slots.extend(
            crate::zones::carrier_zones_for(state, player)
                .into_iter()
                .filter(|slot| !immutable_fuser(state, slot)),
        );
    }
    slots
        .into_iter()
        .map(|slot| ZoneChoice {
            row: slot.row,
            lane: slot.lane,
        })
        .collect()
}

/// R653, R23: a carrier that fuses its Unit in (Classic+ #33 Ivory Tower) takes no Unit while it is
/// Immutable, since its text could not change to take the Unit in. Read here and in `refuse_zone`, beside
/// `zones::why_cannot_carry`, because Immutable is a keyword the layers compute (§10.4).
fn immutable_fuser(state: &GameState, slot: &ZoneSlot) -> bool {
    match crate::zones::card_at(state, slot) {
        None => false,
        Some(top) => {
            crate::scripts::script_of(state, top).flags().fuses_carried == Some(true)
                && crate::layers::unit_has(state, top, KeywordKind::Immutable)
        }
    }
}

/// R391 (B4.5): whether the Tribute a play pays empties this zone for it — a zone whose pile is exactly
/// one card, and that card one the Tribute takes (a Stack pile's next card would resume, R13, so
/// tributing its top frees nothing), without Reborn (its zone would be reserved for the return, R64),
/// in a zone neither Locked nor reserved for anything else. A zone that is open anyway is not "freed".
/// The rule is the row's, not the Unit row's alone: a backrow card with a Tribute cost reads it the
/// same way, though a Tribute pays with units and so empties a backrow zone only when a unit sits in
/// one. An Activate cost needs no zone and never comes here (B3.2).
pub fn freed_by_tribute(state: &GameState, slot: &ZoneSlot, tributes: &[String]) -> bool {
    if tributes.is_empty() {
        return false;
    }
    if crate::zones::is_locked(state, slot) || crate::zones::is_reserved(state, slot) {
        return false;
    }
    let Some(held) = crate::zones::card_at(state, slot) else {
        return false;
    };
    if !tributes.contains(&held.id) {
        return false;
    }
    if slot.row == Row::Units && crate::zones::pile_at(state, slot).map_or(0, |pile| pile.len()) != 1 {
        return false;
    }
    !crate::layers::unit_has(state, held, KeywordKind::Reborn)
}

/// R64, R391: the zone a play that names none takes — the leftmost open zone of its row, else, on a
/// full row, the leftmost zone its own Tribute empties. `None` when there is neither.
pub fn default_zone_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    tributes: &[String],
) -> Option<ZoneSlot> {
    let row = row_for_card(state, card);
    crate::zones::first_free_zone(state, player, row).or_else(|| {
        crate::zones::slots_of(player, row)
            .into_iter()
            .find(|slot| freed_by_tribute(state, slot, tributes))
    })
}

/// §2.3: whether the player chooses X when playing this card — an X-cost card whose X is not fixed
/// by a `cost` hook. #98 Heroic Power prints X but "its X is fixed by its power" (§2.3, R43, R65):
/// its cost hook answers the X, so there is nothing to choose and no X travels in its play.
pub fn chooses_x(state: &GameState, instance: &CardInstance) -> bool {
    // B5 E14, R545: a copier pays its own price and chooses the X of an X-cost text it copies.
    if crate::subsystems::copied_text::copies_text(instance) {
        return crate::subsystems::copied_text::copied_chooses_x(state, instance);
    }
    crate::mana::is_x_cost(state, instance) && crate::scripts::script_of(state, instance).cost.is_none()
}

/// §2.3, R348: why this X is not one the player may choose for this card, or `Ok` when it is —
/// "X is chosen at play time, 1 ≤ X ≤ current mana" (`MIN_CHOSEN_X`). `legal_x_values` offers exactly
/// the values this passes and `refuse_x` refuses exactly the ones it names, so the picker and the
/// reducer's refusal cannot disagree. `most` defaults to the player's current mana.
pub fn why_x_refused(
    state: &GameState,
    player: PlayerId,
    value: i32,
    most: Option<i32>,
) -> Result<(), EngineError> {
    // R1223: an X may be chosen out of borrowed mana too.
    let mana = crate::credit::spendable_mana(state, player);
    let most = most.unwrap_or(mana);
    // TS also refused an X that is not a whole number ("X must be a whole number"); an `i32` always is.
    if value < 0 {
        return Err(EngineError::new("X cannot be negative"));
    }
    if value < MIN_CHOSEN_X {
        return Err(EngineError::new(format!("X must be at least {MIN_CHOSEN_X}")));
    }
    if value > most {
        return Err(EngineError::new(if most < mana {
            "X is above your mana after paying"
        } else {
            "X is above your current mana"
        }));
    }
    Ok(())
}

/// The most X a play of this card may choose: the player's current mana for an X-cost card, and for a
/// copier with an X-cost text the mana left once its own price is paid (B5 E14, R545).
fn most_x(state: &GameState, player: PlayerId, card: &CardInstance) -> i32 {
    // R1223: an X may be chosen out of borrowed mana too.
    let mana = crate::credit::spendable_mana(state, player);
    if !crate::subsystems::copied_text::copies_text(card) {
        return mana;
    }
    mana - crate::mana::play_cost(state, card)
}

/// §2.3, R348: every X `why_x_refused` passes, lowest first. Not an X-cost card, no X values.
pub fn legal_x_values(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<i32> {
    if !chooses_x(state, card) {
        return Vec::new();
    }
    let most = most_x(state, player, card);
    (0..=most.max(0))
        .filter(|x| why_x_refused(state, player, *x, Some(most)).is_ok())
        .collect()
}

/// §2.3: an "A embiggen B" card offers two prices; every other card offers none.
pub fn has_embiggen_price(state: &GameState, card: &CardInstance) -> bool {
    matches!(
        crate::catalog::def_of(Some(state), &card.def_id).cost,
        CardCost::Embiggen { .. }
    )
}

pub fn legal_embiggen_choices(state: &GameState, card: &CardInstance) -> Vec<bool> {
    if has_embiggen_price(state, card) {
        vec![false, true]
    } else {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Tribute (§6.3, §3.2)
// ---------------------------------------------------------------------------

// #55 Lava Golem "may tribute enemy units", which no other Tribute card may (R101). TS kept a local
// alias `TributeFlags = StaticFlags & { tributeEnemies?: boolean }` for readability; `StaticFlags`
// declares `tribute_enemies` itself, so the flags are read directly. The flag defaults to false,
// which is every other card.

/// §6.3 "Tribute X": the number of units playing this card sacrifices, 0 when it asks for none.
pub fn tribute_cost_of(state: &GameState, card: &CardInstance) -> i32 {
    let declared = declared_targets(state, card)
        .into_iter()
        .find(|decl| decl.kind == PromptKind::Tribute)
        .and_then(|decl| decl.amount);
    let flagged = crate::scripts::script_of(state, card).flags();
    declared.or(flagged.tribute).unwrap_or(0).max(0)
}

pub fn may_tribute_enemy_units(state: &GameState, card: &CardInstance) -> bool {
    crate::scripts::script_of(state, card).flags().tribute_enemies == Some(true)
}

/// §3.2: "Sheep Tokens are worth 2 Tributes while on the field"; every other unit is worth 1.
///
/// §7 gives the Sheep a Radiant face worth 3, so the value is read off the face that is up — the
/// Sheep's script declares it as `staticFlags.tributeWorth`, and `flags_of` reads the flags of the
/// face the instance wears, which is exactly what a Radiant form is.
///
/// "Worth 2 Tributes" is the Sheep's text (§7), and §6.3's Vanilla removes a unit's text, so a
/// Vanilla Sheep — radiant #61's copy of one — is worth 1 like any other unit (R115: `flags_of` reads
/// nothing off a Vanilla instance). And a fused card's text is both texts joined, with nothing about
/// an ingredient dropped (R102), so a Sheep #85 fused a unit onto is still worth 2 — the fused flags
/// take the larger worth, as they take the larger Tribute.
pub fn tribute_value_of(state: &GameState, unit: &CardInstance) -> i32 {
    let flag = crate::scripts::script_of(state, unit).flags().tribute_worth;
    // B3.4 rule 5, R386: a card that declares its worth as a number (C #82 Sheeople's `worth`) is worth
    // what Degrade, Upgrade and KY's Constant have left it, read off the instance like any declared number.
    let worth = if flag.is_some() && crate::params::param_decl_of(state, &unit.def_id, "worth").is_some() {
        Some(crate::params::param_value(
            state,
            Some(unit),
            "worth",
            Default::default(),
        ))
    } else {
        flag
    };
    match worth {
        Some(worth) if worth > 1 => worth,
        _ => 1,
    }
}

/// §6.3, R1282: your own units, plus the enemy's for a card that says so (#55), plus cheap permanents
/// of either row and side for a wide Tribute (#97.4, R1282: up to 20 permanents). Dormant cards never.
pub fn legal_tribute_units<'a>(
    state: &'a GameState,
    player: PlayerId,
    card: &CardInstance,
) -> Vec<&'a CardInstance> {
    let sides: Vec<PlayerId> = if may_tribute_enemy_units(state, card) {
        vec![player, opponent_of(player)]
    } else {
        vec![player]
    };
    let mut out: Vec<&'a CardInstance> = Vec::new();
    for side in sides {
        for unit in crate::zones::active_units_of(state, side) {
            // R1220: no Tribute cost may take an Untributable card.
            if unit.id != card.id && !crate::query::is_untributable(state, unit) {
                out.push(unit);
            }
        }
    }
    // R1282's wide Tribute; R1220: never an Untributable card here either.
    if let Some(printed) = crate::scripts::script_of(state, card).flags().tribute_cheap {
        let cheap = crate::params::declared_or(state, card, "cheap", printed);
        for side in [player, opponent_of(player)] {
            if side == opponent_of(player) {
                for unit in crate::zones::active_units_of(state, side) {
                    if unit.id != card.id
                        && !out.iter().any(|c| c.id == unit.id)
                        && !crate::query::is_untributable(state, unit)
                        && crate::mana::cost_now(state, unit) <= cheap
                    {
                        out.push(unit);
                    }
                }
            }
            for slot in crate::zones::slots_of(side, Row::Backrow) {
                if let Some(c) = crate::zones::card_at(state, slot)
                    && c.id != card.id
                    && !out.iter().any(|existing| existing.id == c.id)
                    && !crate::query::is_untributable(state, c)
                    && crate::mana::cost_now(state, c) <= cheap
                {
                    out.push(c);
                }
            }
        }
    }
    out
}

fn tribute_total(state: &GameState, units: &[&CardInstance]) -> i32 {
    units.iter().map(|unit| tribute_value_of(state, unit)).sum()
}

/// Every set of units that pays the Tribute exactly: enough to meet the cost, and minimal, so no unit
/// in the set could be dropped and still pay it. The Sheep Token's 2 is why a set may overshoot.
///
/// R90, R1282: every one of them, with no cut. The Tribute is what the play costs, and a set left off the
/// list is a price the player can never pay: the client builds a play only out of the plays
/// `legal_actions` lists (CLAUDE.md rule 7), and the §10.7 policy draws from the same list. Cut at
/// `MAX_CHOICE_COMBINATIONS` with the player's own units first, a Lava Golem beside four of its own
/// units and five enemy ones was never offered #55's defining play, three enemy units (R101). The
/// count is the board's to bound: at most ten units stand, so Core's largest Tribute, #55's 3, has at
/// most 120 minimal sets; a wide Tribute (R1282) on a full board can have C(20,5) = 15,504 sets.
pub fn legal_tribute_sets(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<Vec<String>> {
    let need = tribute_cost_of(state, card);
    if need == 0 {
        return vec![Vec::new()];
    }

    let units = legal_tribute_units(state, player, card);
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut chosen: Vec<&CardInstance> = Vec::new();

    fn walk<'a>(
        state: &GameState,
        need: i32,
        units: &[&'a CardInstance],
        from: usize,
        chosen: &mut Vec<&'a CardInstance>,
        out: &mut Vec<Vec<String>>,
    ) {
        let paid = tribute_total(state, chosen);
        if paid >= need {
            if is_minimal_tribute(state, chosen, need) {
                out.push(chosen.iter().map(|unit| unit.id.clone()).collect());
            }
            return;
        }
        for at in from..units.len() {
            chosen.push(units[at]);
            walk(state, need, units, at + 1, chosen, out);
            chosen.pop();
        }
    }

    walk(state, need, &units, 0, &mut chosen, &mut out);
    out
}

fn is_minimal_tribute(state: &GameState, units: &[&CardInstance], need: i32) -> bool {
    let paid = tribute_total(state, units);
    units
        .iter()
        .all(|unit| paid - tribute_value_of(state, unit) < need)
}

// ---------------------------------------------------------------------------
// Declared targets: the options one declaration admits
// ---------------------------------------------------------------------------

fn selection_key(selection: &Selection) -> String {
    match selection {
        Selection::Instance { instance_id } => format!("instance:{instance_id}"),
        Selection::Hero { player } => format!("hero:{player}"),
        Selection::Zone { player, row, lane } => format!("zone:{player}:{row}:{lane}"),
        Selection::Mode { option } => format!("mode:{option}"),
        // ME-CRAFT (R880): a recipe names its definition, as its prompt's option key does.
        Selection::Craft { recipe } => crate::subsystems::craft::craft_id(recipe),
        Selection::None => "none".to_string(),
    }
}

/// The sides a declaration reaches, the chooser's own first so the offered order is stable.
///
/// A Tribute defaults to the chooser's side — §6.3 reads "sacrifice X of *your* units" — and every
/// other kind defaults to both, which is what §10.6's bare `target` means. A `hand` declaration is
/// forced to the chooser whatever it says (§9.1: the opponent's hand is hidden), which the hand case of
/// `legal_selections_for` enforces rather than this.
fn sides_for(player: PlayerId, decl: &TargetDecl) -> Vec<PlayerId> {
    let side =
        decl.filter
            .as_ref()
            .and_then(|filter| filter.side)
            .unwrap_or(if decl.kind == PromptKind::Tribute {
                FilterSide::Ally
            } else {
                FilterSide::Any
            });
    match side {
        FilterSide::Ally => vec![player],
        FilterSide::Enemy => vec![opponent_of(player)],
        FilterSide::Any => vec![player, opponent_of(player)],
    }
}

/// §10.6: what a declaration picks. `of` names it; otherwise the declaration's own kind does.
fn pick_kinds_for(decl: &TargetDecl) -> Vec<FilterOf> {
    if let Some(named) = decl.filter.as_ref().and_then(|filter| filter.of.as_ref())
        && !named.is_empty()
    {
        return PICK_KIND_ORDER
            .into_iter()
            .filter(|kind| named.contains(kind))
            .collect();
    }
    if decl.kind == PromptKind::Hand {
        return vec![FilterOf::Hand];
    }
    if decl.kind == PromptKind::Zone {
        return vec![FilterOf::Zone];
    }
    // R90: "a card that declared nothing takes nothing"; a bare `target` or `tribute` means a unit.
    vec![FilterOf::Unit]
}

fn type_allows(filter: Option<&TargetFilter>, card_type: CardType) -> bool {
    match filter.and_then(|filter| filter.type_.as_ref()) {
        None => true,
        Some(named) => named.includes(&card_type),
    }
}

/// §10.6: `tags` wants every tag it names, `notTags` none of them — the same reading as §5.1's query.
fn tags_allow(filter: Option<&TargetFilter>, tags: &[Tag]) -> bool {
    if let Some(wanted) = filter.and_then(|filter| filter.tags.as_ref())
        && !wanted.iter().all(|tag| tags.contains(tag))
    {
        return false;
    }
    if let Some(barred) = filter.and_then(|filter| filter.not_tags.as_ref())
        && barred.iter().any(|tag| tags.contains(tag))
    {
        return false;
    }
    true
}

fn card_allowed(
    state: &GameState,
    filter: Option<&TargetFilter>,
    held: &CardInstance,
    self_: &CardInstance,
    player: PlayerId,
) -> bool {
    if filter.and_then(|filter| filter.exclude_self) == Some(true) && held.id == self_.id {
        return false;
    }
    // MD-B15, R923: a granted tag is part of the card for a declaration too. MD-B16, R924: a
    // face-down backrow card its chooser cannot read is a legal pick whatever its tags, so the
    // picks reveal nothing — no shipped declaration filters the backrow by tag, so only the tag
    // check is skipped for one.
    let tags: Vec<Tag> = crate::query::tags_of(state, held);
    let unreadable_backrow = matches!(
        held.zone,
        Zone::Field {
            row: Row::Backrow,
            ..
        }
    ) && crate::preview::is_face_down(state, held)
        && held.controller != player;
    if !type_allows(filter, crate::faces::card_type_of(state, held))
        || (!unreadable_backrow && !tags_allow(filter, &tags))
    {
        return false;
    }
    // §10.6, B5: the v0.2.0 filter fields. The cost is R65's where the card is now (a hand card at its
    // hand cost; an X card on the field at the X it was played for, which it keeps there).
    if let Some(range) = filter.and_then(|filter| filter.cost_range) {
        let cost = crate::mana::effective_cost(state, held, Default::default());
        if let Some(min) = range.min
            && cost < min
        {
            return false;
        }
        if let Some(max) = range.max
            && cost > max
        {
            return false;
        }
    }
    if filter.and_then(|filter| filter.damaged) == Some(true) && held.damage <= 0 {
        return false;
    }
    if filter.and_then(|filter| filter.plague) == Some(true) && held.counters.plague.unwrap_or(0) <= 0 {
        return false;
    }
    check_allows(
        state,
        filter,
        self_,
        player,
        Some(held),
        &Selection::Instance {
            instance_id: held.id.clone(),
        },
    )
}

/// §10.6: a filter's named predicate (`TargetFilter.check`), the declaring card's own `targetChecks`
/// entry, asked with the candidate — `None` for a hero or a zone. A name the script does not hold
/// admits nothing, so a misspelt check never widens a declaration.
fn check_allows(
    state: &GameState,
    filter: Option<&TargetFilter>,
    self_: &CardInstance,
    player: PlayerId,
    candidate: Option<&CardInstance>,
    selection: &Selection,
) -> bool {
    let Some(name) = filter.and_then(|filter| filter.check.as_ref()) else {
        return true;
    };
    let script = crate::scripts::script_of(state, self_);
    let Some(check) = script.target_checks.get(name.as_str()) else {
        return false;
    };
    check(TargetCheckArgs {
        state,
        self_,
        player,
        radiant: self_.radiant,
        candidate,
        selection,
    })
}

/// A hero has no card type, no tags, no cost, no damage count and no Plague Counters, so a filter that
/// names any of them cannot reach one; a named predicate is asked.
fn hero_allowed(
    state: &GameState,
    filter: Option<&TargetFilter>,
    self_: &CardInstance,
    player: PlayerId,
    selection: &Selection,
) -> bool {
    if let Some(filter) = filter {
        if filter.type_.is_some() || filter.tags.is_some() {
            return false;
        }
        if filter.cost_range.is_some() || filter.damaged == Some(true) || filter.plague == Some(true) {
            return false;
        }
    }
    check_allows(state, filter, self_, player, None, selection)
}

/// B5 E5, E35, R450: whether a declaration may pick a card acting on the field on top of its filter —
/// a Spell's declarations never offer a card Immune to Spells, and a `target` declaration never offers
/// a card whose targeting cost (Classic #89) its chooser cannot pay from the rest of their hand.
fn reachable(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    decl: &TargetDecl,
    candidate: &CardInstance,
) -> bool {
    if decl.kind == PromptKind::Tribute {
        // R1220: a declared Tribute never offers an Untributable card.
        return !crate::query::is_untributable(state, candidate);
    }
    if crate::restrictions::spell_cannot_reach(state, Some(card), candidate) {
        return false;
    }
    // MD-B1, R940: a harmful declaration whose filter names a tribal tag never offers an immune card.
    if crate::targeting::target_aim(decl) == crate::wire::TargetAim::Harm {
        let (tags, not_tags) = match decl.filter.as_ref() {
            Some(filter) => (filter.tags.as_deref(), filter.not_tags.as_deref()),
            None => (None, None),
        };
        if crate::restrictions::tribal_hate_cannot_reach(state, candidate, tags, not_tags) {
            return false;
        }
    }
    decl.kind != PromptKind::Target
        || crate::targeting::can_pay_to_target(state, player, candidate, Some(card.id.as_str()))
}

/// Every selection this declaration admits right now, in offer order: the chooser's side first, and
/// within a side units by lane, then the backrow by lane, then the hand, then zones, then the hero.
///
/// R90: a unit pick offers the top of a Stack pile and never a dormant card (R13), a hand pick offers
/// only the chooser's own hand (§9.1), and the card being played is never offered out of the hand it
/// is leaving. An empty result is not an error — the play stays legal and the effect fizzles.
pub fn legal_selections_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    decl: &TargetDecl,
) -> Vec<Selection> {
    let filter = decl.filter.as_ref();
    let kinds = pick_kinds_for(decl);
    let mut out: Vec<Selection> = Vec::new();
    let mut seen: IndexSet<String> = IndexSet::new();

    let mut offer = |selection: Selection| {
        let key = selection_key(&selection);
        if seen.contains(&key) {
            return;
        }
        seen.insert(key);
        out.push(selection);
    };

    // B5 E5, E35, R450: a card on the field is offered only where the declaring card may reach it.
    let on_field = |held: &CardInstance| -> bool {
        card_allowed(state, filter, held, card, player) && reachable(state, player, card, decl, held)
    };

    for side in sides_for(player, decl) {
        for kind in &kinds {
            match kind {
                FilterOf::Unit => {
                    for unit in crate::zones::active_units_of(state, side) {
                        if on_field(unit) {
                            offer(Selection::Instance {
                                instance_id: unit.id.clone(),
                            });
                        }
                    }
                }
                FilterOf::Backrow => {
                    for slot in crate::zones::slots_of(side, Row::Backrow) {
                        let Some(held) = crate::zones::card_at(state, slot) else {
                            continue;
                        };
                        if on_field(held) {
                            offer(Selection::Instance {
                                instance_id: held.id.clone(),
                            });
                        }
                    }
                }
                FilterOf::Hand => {
                    // §9.1: only ever the chooser's own hand, whatever side the filter names.
                    if side != player {
                        continue;
                    }
                    for held in &state.players[player].hand {
                        if held.id == card.id {
                            continue;
                        }
                        if card_allowed(state, filter, held, card, player) {
                            offer(Selection::Instance {
                                instance_id: held.id.clone(),
                            });
                        }
                    }
                }
                FilterOf::Graveyard => {
                    // B5 (Classic #54's "on the field or in your graveyard"): a graveyard is public (§3), so
                    // either side's may be named; the card being played is never in one.
                    for held in &state.players[side].graveyard {
                        if held.id == card.id {
                            continue;
                        }
                        if card_allowed(state, filter, held, card, player) {
                            offer(Selection::Instance {
                                instance_id: held.id.clone(),
                            });
                        }
                    }
                }
                FilterOf::Zone => {
                    for row in ROWS {
                        for slot in crate::zones::slots_of(side, row) {
                            if !crate::zones::is_open(state, slot) {
                                continue;
                            }
                            let selection = Selection::Zone {
                                player: slot.player,
                                row: slot.row,
                                lane: slot.lane,
                            };
                            if check_allows(state, filter, card, player, None, &selection) {
                                offer(selection);
                            }
                        }
                    }
                }
                FilterOf::Hero => {
                    let selection = Selection::Hero { player: side };
                    if hero_allowed(state, filter, card, player, &selection) {
                        offer(selection);
                    }
                }
            }
        }
    }

    out
}

/// B5 E5, R450: which of a play's flat `targets` are targetings — each pick's declaration when it is a
/// `target` declaration, `None` for a Tribute, hand or zone pick (R90's reading of the list). The
/// targeting point reads it: a cost is owed, and an interception answers, only for these. `declared`
/// is the card's own declarations by default (`None`); an activation passes its ability's (B3.2, R384).
pub fn targeting_decls_of(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    selections: &[Selection],
    modes: &[String],
    declared: Option<&[TargetDecl]>,
) -> Vec<Option<TargetDecl>> {
    let own;
    let declared: &[TargetDecl] = match declared {
        Some(declared) => declared,
        None => {
            own = declared_targets(state, card);
            &own
        }
    };
    let decls = active_target_decls(declared, modes);
    if decls.is_empty() {
        return selections.iter().map(|_| None).collect();
    }
    let offered: Vec<Vec<Selection>> = decls
        .iter()
        .map(|decl| legal_selections_for(state, player, card, decl))
        .collect();
    let slices = split_selections(&decls, &offered, selections);
    slices
        .iter()
        .enumerate()
        .flat_map(|(index, slice)| {
            let decl = decls.get(index);
            slice.iter().map(move |_| match decl {
                Some(decl) if decl.kind == PromptKind::Target => Some(decl.clone()),
                _ => None,
            })
        })
        .collect()
}

/// B5 E5, R450: the discards a play's declared targets cost it (Classic #89), read against the face
/// the play resolves (R214) — or an activation's, against its ability's declarations. 0 for choices
/// that target nothing costly.
pub fn targeting_discards_required(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    selections: &[Selection],
    modes: &[String],
    declared: Option<&[TargetDecl]>,
) -> i32 {
    let decls = targeting_decls_of(state, player, card, selections, modes, declared);
    let mut total = 0;
    for (index, selection) in selections.iter().enumerate() {
        let Some(Some(_)) = decls.get(index) else {
            continue;
        };
        let Selection::Instance { instance_id } = selection else {
            continue;
        };
        if let Some(candidate) = crate::state::find_instance(state, instance_id) {
            total += crate::targeting::targeting_discards_of(state, candidate);
        }
    }
    total
}

/// Classic #33 Joro, R450: whether `interceptor`, summoned into its controller's leftmost open unit
/// zone, would be a legal pick of `decl` — a declared pick moves to it only then (Hearthstone's
/// Spellbender). Read with the card as it would stand there: its cost on the field, undamaged.
pub fn interceptor_fits_decl(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    decl: &TargetDecl,
    interceptor: &CardInstance,
) -> bool {
    let defender = interceptor.controller;
    if !sides_for(player, decl).contains(&defender) || !pick_kinds_for(decl).contains(&FilterOf::Unit) {
        return false;
    }
    let Some(zone) = crate::zones::first_entry_zone(state, defender, Row::Units) else {
        return false;
    };
    let probe = CardInstance {
        zone: Zone::Field {
            player: defender,
            row: Row::Units,
            lane: zone.lane,
        },
        ..interceptor.clone()
    };
    card_allowed(state, decl.filter.as_ref(), &probe, card, player)
        && !crate::restrictions::spell_cannot_reach(state, Some(card), &probe)
        // MD-B1, R940: a harmful declaration's tribal filter never diverts to an immune interceptor.
        && !(crate::targeting::target_aim(decl) == crate::wire::TargetAim::Harm
            && {
                let (tags, not_tags) = match decl.filter.as_ref() {
                    Some(filter) => (filter.tags.as_deref(), filter.not_tags.as_deref()),
                    None => (None, None),
                };
                crate::restrictions::tribal_hate_cannot_reach(state, &probe, tags, not_tags)
            })
}

// ---------------------------------------------------------------------------
// R90: reading the flat list, and enumerating it
// ---------------------------------------------------------------------------

/// R90: how many selections a declaration takes off the flat list. Every declaration but the last
/// takes a fixed number — its own minimum, or everything the board can offer when that is less, so a
/// declaration the board cannot satisfy takes none and the next one still reads its own slot. The
/// last declaration takes the remainder.
fn take_for(decl: &TargetDecl, offered: usize) -> usize {
    at_most(decl.min, offered)
}

/// R703: a declaration the play needs (`required`) that the board cannot satisfy. The play is then
/// neither offered (`subsets_for`) nor accepted (`refuse_targets`), instead of fizzling as R90 has it.
pub(crate) fn unmet_requirement(decl: &TargetDecl, offered: usize) -> bool {
    decl.required == Some(true) && (offered as i64) < i64::from(decl.min)
}

/// R90's reading of a flat `targets` list: one slice per declaration, in declaration order, measured
/// against what the board offers each declaration now. A fused card reads its ingredients' slices
/// this way (R102), each ingredient's script getting only the declarations it made.
pub fn selections_per_declaration(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    decls: &[TargetDecl],
    selections: &[Selection],
) -> Vec<Vec<Selection>> {
    let offered: Vec<Vec<Selection>> = decls
        .iter()
        .map(|decl| legal_selections_for(state, player, card, decl))
        .collect();
    split_selections(decls, &offered, selections)
}

/// R90, R102: where the play's choices were split. A fused card's Cry resolves each ingredient with
/// its own slice of the flat `targets` list, and the slices are the ones §10.5 step 1 read the play
/// with — the board as it stood when the play was checked and `legal_actions` offered it — not the
/// board at step 5, where the played card itself may now be one of the options (a crafted Postdoc +
/// Sorcerer is a Human, R102). The pipeline carries the lengths in the Cry's `data` under this key.
pub const DECLARATION_SLICES_KEY: &str = "__declarationSlices";

/// How many selections each active declaration took off the flat list, read now (R90).
pub fn declaration_slices(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    selections: &[Selection],
    modes: &[String],
) -> Vec<usize> {
    let decls = active_target_decls(&declared_targets(state, card), modes);
    let offered: Vec<Vec<Selection>> = decls
        .iter()
        .map(|decl| legal_selections_for(state, player, card, decl))
        .collect();
    split_selections(&decls, &offered, selections)
        .iter()
        .map(Vec::len)
        .collect()
}

/// The lengths a pipeline stored under `DECLARATION_SLICES_KEY`, or `None` when there are none. It
/// came back through JSON (SURFACE §4.4.10): every entry must be a number.
pub fn stored_declaration_slices(data: &IndexMap<String, Value>) -> Option<Vec<usize>> {
    let raw = data.get(DECLARATION_SLICES_KEY)?.as_array()?;
    raw.iter()
        .map(|value| value.as_f64().map(|n| n.max(0.0) as usize))
        .collect()
}

/// R221, R90: a play's picks for each declaration, in the order the declaration offers its options.
/// `legal_actions` offers each set of picks once, in that order (`subsets_for`), and step 1 accepts
/// any listing of an offered set, so step 5 must never resolve a listing no offered play means: one
/// declaration's picks are a set, as an answer's are (`prompts::in_offered_order`). The slices are the
/// ones step 1 read the play with, and a pick no option names keeps its place after the others.
/// `target_decls` is the card's own declarations by default (`None`).
pub fn in_declared_order(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    selections: &[Selection],
    modes: &[String],
    target_decls: Option<&[TargetDecl]>,
) -> Vec<Selection> {
    let own;
    let target_decls: &[TargetDecl] = match target_decls {
        Some(target_decls) => target_decls,
        None => {
            own = declared_targets(state, card);
            &own
        }
    };
    let decls = active_target_decls(target_decls, modes);
    if decls.is_empty() {
        return selections.to_vec();
    }
    let offered: Vec<Vec<Selection>> = decls
        .iter()
        .map(|decl| legal_selections_for(state, player, card, decl))
        .collect();
    let mut out: Vec<Selection> = Vec::new();
    for (index, slice) in split_selections(&decls, &offered, selections)
        .into_iter()
        .enumerate()
    {
        let order: Vec<String> = offered
            .get(index)
            .map(|options| options.iter().map(selection_key).collect())
            .unwrap_or_default();
        let rank = |selection: &Selection| -> usize {
            let key = selection_key(selection);
            order
                .iter()
                .position(|offered| *offered == key)
                .unwrap_or(usize::MAX)
        };
        let mut entries: Vec<(Selection, usize, usize)> = slice
            .into_iter()
            .enumerate()
            .map(|(at, selection)| {
                let ranked = rank(&selection);
                (selection, at, ranked)
            })
            .collect();
        // A stable sort, rank first and then the listing's own order (SURFACE §4.4.1).
        entries.sort_by(|a, b| a.2.cmp(&b.2).then(a.1.cmp(&b.1)));
        out.extend(entries.into_iter().map(|(selection, _, _)| selection));
    }
    out
}

/// R123: a `tribute` declaration with an `amount` is one Tribute written in two lists — the units the
/// play pays with (`tributes`, sacrificed at §10.5 step 2) and the picks its script reads as units it
/// tributed (`targets`) — so every pick is one of the units the play tributes. A pick may name fewer
/// units than the cost takes (one pick of a Tribute 2, or the one Sheep Token that pays it, §3.2), but
/// never a unit the play keeps. True when the picks agree, or when the card declares no such Tribute.
fn tribute_picks_agree(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    selections: &[Selection],
    modes: &[String],
    tributes: &[String],
) -> bool {
    let decls = active_target_decls(&declared_targets(state, card), modes);
    let Some(at) = decls
        .iter()
        .position(|decl| decl.kind == PromptKind::Tribute && decl.amount.is_some())
    else {
        return true;
    };
    let offered: Vec<Vec<Selection>> = decls
        .iter()
        .map(|decl| legal_selections_for(state, player, card, decl))
        .collect();
    let slice = split_selections(&decls, &offered, selections)
        .into_iter()
        .nth(at)
        .unwrap_or_default();
    let paid: IndexSet<&String> = tributes.iter().collect();
    slice.iter().all(|selection| match selection {
        Selection::Instance { instance_id } => paid.contains(instance_id),
        _ => false,
    })
}

/// Whether the card declares a Tribute that travels in both lists (R123), which binds them.
fn declares_bound_tribute(state: &GameState, card: &CardInstance) -> bool {
    declared_targets(state, card)
        .iter()
        .any(|decl| decl.kind == PromptKind::Tribute && decl.amount.is_some())
}

fn split_selections(
    decls: &[TargetDecl],
    offered: &[Vec<Selection>],
    selections: &[Selection],
) -> Vec<Vec<Selection>> {
    let mut out: Vec<Vec<Selection>> = Vec::new();
    let mut at = 0usize;
    for (index, decl) in decls.iter().enumerate() {
        if index == decls.len() - 1 {
            out.push(selections[at.min(selections.len())..].to_vec());
            at = selections.len();
            continue;
        }
        let available = offered.get(index).map_or(0, Vec::len);
        let take = take_for(decl, available).min(selections.len().saturating_sub(at));
        out.push(selections[at..at + take].to_vec());
        at += take;
    }
    out
}

/// Every subset of `options` a declaration may answer with, size-ascending then index order; none at
/// all for a needed pick the board cannot satisfy, so no play is offered (R703).
fn subsets_for(options: &[Selection], decl: &TargetDecl, is_last: bool) -> Vec<Vec<Selection>> {
    if unmet_requirement(decl, options.len()) {
        return Vec::new();
    }
    let low = take_for(decl, options.len());
    let high = if is_last {
        at_most(decl.max, options.len())
    } else {
        low
    };
    let mut out: Vec<Vec<Selection>> = Vec::new();

    fn walk(
        options: &[Selection],
        size: usize,
        from: usize,
        picked: &mut Vec<Selection>,
        out: &mut Vec<Vec<Selection>>,
    ) {
        if out.len() >= MAX_CHOICE_COMBINATIONS {
            return;
        }
        if picked.len() == size {
            out.push(picked.clone());
            return;
        }
        for at in from..options.len() {
            picked.push(options[at].clone());
            walk(options, size, at + 1, picked, out);
            picked.pop();
            if out.len() >= MAX_CHOICE_COMBINATIONS {
                return;
            }
        }
    }

    let mut size = low;
    while size <= high {
        let mut picked: Vec<Selection> = Vec::new();
        walk(options, size, 0, &mut picked, &mut out);
        if out.len() >= MAX_CHOICE_COMBINATIONS {
            break;
        }
        size += 1;
    }

    out
}

/// The cross of several lists, the first varying slowest, bounded by `cap` (R90). When the whole cross
/// fits under the bound it is all of it, in that order. When it does not, the bound drops
/// combinations and never a pick: the combinations kept first offer every item of every list — one
/// per place in the longest list, each list taking its items in turn — and the rest follow in order
/// up to the bound. Cut in order alone, the first list's later items were in no combination at all:
/// a crafted Twisted Sorcerer + K-Pop Fanatic crossed over eleven and eight picks never offered the
/// Sorcerer's 4 damage at the enemy hero (R81, R102), and the client, which builds a play only out of
/// the plays `legal_actions` lists (CLAUDE.md rule 7), could not make it.
fn cross_product<T: Clone>(lists: &[Vec<T>], cap: usize) -> Vec<Vec<T>> {
    if lists.iter().any(Vec::is_empty) {
        return Vec::new();
    }
    let total = lists
        .iter()
        .fold(1usize, |product, list| product.saturating_mul(list.len()));
    let mut out: Vec<Vec<T>> = Vec::new();
    let mut kept: IndexSet<String> = IndexSet::new();
    let mut keep = |indices: &[usize], out: &mut Vec<Vec<T>>| {
        let key = indices
            .iter()
            .map(usize::to_string)
            .collect::<Vec<String>>()
            .join(",");
        if kept.contains(&key) || out.len() >= cap {
            return;
        }
        kept.insert(key);
        out.push(
            indices
                .iter()
                .enumerate()
                .map(|(at, index)| lists[at][*index].clone())
                .collect(),
        );
    };
    if total > cap {
        let width = lists.iter().map(Vec::len).max().unwrap_or(0);
        for place in 0..width {
            let indices: Vec<usize> = lists.iter().map(|list| place % list.len()).collect();
            keep(&indices, &mut out);
        }
    }
    // The rest in order: an odometer whose last wheel turns fastest.
    let mut wheel: Vec<usize> = vec![0; lists.len()];
    loop {
        if out.len() >= cap {
            break;
        }
        keep(&wheel, &mut out);
        let mut at: isize = wheel.len() as isize - 1;
        while at >= 0 {
            let index = at as usize;
            let length = lists[index].len();
            wheel[index] = (wheel[index] + 1) % length;
            if wheel[index] != 0 {
                break;
            }
            at -= 1;
        }
        if at < 0 {
            break;
        }
    }
    out
}

/// R90: several groups of answers kept under one bound — the target combinations of each mode choice
/// a `forModes` card offers — all of them when they fit, and otherwise taken a round at a time, one
/// from each group in turn, so every mode choice keeps answers of its own.
fn interleaved<T: Clone>(groups: &[Vec<T>], cap: usize) -> Vec<T> {
    let total: usize = groups.iter().map(Vec::len).sum();
    if total <= cap {
        return groups.iter().flatten().cloned().collect();
    }
    let mut out: Vec<T> = Vec::new();
    let mut round = 0usize;
    while out.len() < cap {
        let mut any = false;
        for group in groups {
            let Some(item) = group.get(round) else {
                continue;
            };
            any = true;
            out.push(item.clone());
            if out.len() >= cap {
                break;
            }
        }
        if !any {
            break;
        }
        round += 1;
    }
    out
}

/// R90: "`legalActions` enumerates every legal combination, bounded by `MAX_CHOICE_COMBINATIONS`".
/// Declarations are read in order, the first varying slowest, and the target and mode lists that come
/// back are exactly what a `play` action carries. A card that declares nothing yields one empty
/// answer, so it is still offered once. `declared` is the card's own declarations by default (`None`).
pub fn play_choice_combinations(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    declared: Option<&DeclaredChoices>,
) -> Vec<PlayChoices> {
    let target_decls: Vec<TargetDecl> = match declared {
        Some(declared) => declared.targets.clone(),
        None => declared_targets(state, card),
    };
    let mode_decls: Vec<ModeDecl> = match declared {
        Some(declared) => declared.modes.clone(),
        None => declared_modes(state, card),
    };
    if target_decls.is_empty() && mode_decls.is_empty() {
        return vec![PlayChoices::default()];
    }

    let target_combos_for = |decls: &[TargetDecl]| -> Vec<Vec<Selection>> {
        let per_decl: Vec<Vec<Vec<Selection>>> = decls
            .iter()
            .enumerate()
            .map(|(index, decl)| {
                subsets_for(
                    &legal_selections_for(state, player, card, decl),
                    decl,
                    index == decls.len() - 1,
                )
            })
            .collect();
        cross_product(&per_decl, MAX_CHOICE_COMBINATIONS)
            .into_iter()
            .map(|slices| slices.into_iter().flatten().collect())
            .collect()
    };
    let mode_option_lists: Vec<Vec<String>> = mode_decls.iter().map(|decl| decl.options.clone()).collect();
    let mode_combos: Vec<Vec<String>> = cross_product(&mode_option_lists, MAX_CHOICE_COMBINATIONS);

    if targets_follow_modes(&target_decls) {
        // The target declarations a play answers depend on its modes (`forModes`), so each mode choice
        // is enumerated with the targets it asks for — and each keeps answers under the bound (R90).
        let mode_choices: Vec<Vec<String>> = if mode_decls.is_empty() {
            vec![Vec::new()]
        } else {
            mode_combos
        };
        let groups: Vec<Vec<PlayChoices>> = mode_choices
            .into_iter()
            .map(|modes| {
                let active = active_target_decls(&target_decls, &modes);
                let answers: Vec<Option<Vec<Selection>>> = if active.is_empty() {
                    vec![None]
                } else {
                    target_combos_for(&active).into_iter().map(Some).collect()
                };
                answers
                    .into_iter()
                    .map(|targets| PlayChoices {
                        targets,
                        modes: if mode_decls.is_empty() {
                            None
                        } else {
                            Some(modes.clone())
                        },
                    })
                    .collect()
            })
            .collect();
        return interleaved(&groups, MAX_CHOICE_COMBINATIONS);
    }

    let target_answers: Vec<Option<Vec<Selection>>> = if target_decls.is_empty() {
        vec![None]
    } else {
        target_combos_for(&target_decls).into_iter().map(Some).collect()
    };
    let mode_answers: Vec<Option<Vec<String>>> = if mode_decls.is_empty() {
        vec![None]
    } else {
        mode_combos.into_iter().map(Some).collect()
    };

    // TS crosses the two answer lists themselves; the cross picks by index alone, so it is crossed over
    // their indices here and the answers read back.
    let index_lists: Vec<Vec<usize>> = vec![
        (0..target_answers.len()).collect(),
        (0..mode_answers.len()).collect(),
    ];
    cross_product(&index_lists, MAX_CHOICE_COMBINATIONS)
        .into_iter()
        .map(|pair| PlayChoices {
            targets: target_answers[pair[0]].clone(),
            modes: mode_answers[pair[1]].clone(),
        })
        .collect()
}

/// Every `play` action this card could legally produce: R81's five choice kinds crossed, skipping the
/// prices the player cannot pay. This is what `legal_actions` lists for a card in hand.
pub fn play_actions_for(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<PlayAction> {
    // R1223: a card affordable on borrowed mana is listed.
    let mana = crate::credit::spendable_mana(state, player);
    let mut out = priced_play_actions(state, player, card, |cost| {
        if cost <= mana {
            vec![PlayPayment::default()]
        } else {
            Vec::new()
        }
    });
    out.extend(face_down_play_actions(state, player, card, false, |cost| {
        if cost <= mana {
            vec![PlayPayment::default()]
        } else {
            Vec::new()
        }
    }));
    out
}

/// ME-ALTPLAY, R1040, R1044: every face-down `play` this card could legally produce — one per timing,
/// price, payment, tribute set and open backrow zone. No targets or modes are declared when set.
fn face_down_play_actions(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    from_graveyard: bool,
    payments: impl Fn(i32) -> Vec<PlayPayment>,
) -> Vec<PlayAction> {
    let Some(kind) = crate::alt_play::face_down_kind(state, player, card, from_graveyard) else {
        return Vec::new();
    };
    let mut out: Vec<PlayAction> = Vec::new();
    for reveal in crate::alt_play::timings(kind) {
        let probe = CardInstance {
            set_as: Some(crate::state::SetAs {
                reveal,
                set_turn: state.turn,
                echo: None,
                revealing: None,
            }),
            ..card.clone()
        };
        for price in play_prices(state, player, &probe) {
            let paid = payments(price.cost);
            if paid.is_empty() {
                continue;
            }
            let tribute_sets: Vec<Vec<String>> = match kind {
                crate::alt_play::SetKind::Unit => legal_tribute_sets(state, player, &probe),
                crate::alt_play::SetKind::Spell => vec![Vec::new()],
            };
            for tributes in &tribute_sets {
                for zone in crate::alt_play::open_backrow_zones(state, player) {
                    for payment in &paid {
                        out.push(PlayAction {
                            instance_id: card.id.clone(),
                            zone: Some(zone),
                            x: price.x,
                            embiggen: price.embiggen,
                            magnetic: None,
                            tributes: if tributes.is_empty() {
                                None
                            } else {
                                Some(tributes.clone())
                            },
                            targets: None,
                            modes: None,
                            plague: payment.plague.clone(),
                            face_down: Some(reveal),
                        });
                    }
                }
            }
        }
    }
    out
}

/// E11, R454: every `play` action `legal_actions` lists for a card in the player's graveyard — R81's
/// choices crossed exactly as for a hand card, each with the ways a permission lets it be paid
/// (`graveyard_play::graveyard_payments_for`). Nothing without a permission that admits it.
pub fn graveyard_play_actions_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
) -> Vec<PlayAction> {
    if !crate::graveyard_play::playable_from_graveyard(state, card) || card.zone.player() != player {
        return Vec::new();
    }
    let mut out = priced_play_actions(state, player, card, |price| {
        crate::graveyard_play::graveyard_payments_for(state, player, card, price)
    });
    out.extend(face_down_play_actions(state, player, card, true, |price| {
        crate::graveyard_play::graveyard_payments_for(state, player, card, price)
    }));
    out
}

/// One price a card's X and embiggen choices come to now (R65): the choices, the probe stamped with
/// them, its cost.
struct PlayPrice {
    x: Option<i32>,
    embiggen: Option<bool>,
    #[allow(dead_code)]
    probe: CardInstance,
    cost: i32,
}

/// R65, R455: every price the card could be played at now, one per X and embiggen choice — the prices
/// `priced_play_actions` lists plays at, with the ones a ban forbids left out, before any is paid for.
fn play_prices(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<PlayPrice> {
    let x_values: Vec<Option<i32>> = if chooses_x(state, card) {
        legal_x_values(state, player, card)
            .into_iter()
            .map(Some)
            .collect()
    } else {
        vec![None]
    };
    let embiggens: Vec<Option<bool>> = if has_embiggen_price(state, card) {
        legal_embiggen_choices(state, card)
            .into_iter()
            .map(Some)
            .collect()
    } else {
        vec![None]
    };
    let mut out: Vec<PlayPrice> = Vec::new();
    for x in &x_values {
        for embiggen in &embiggens {
            let probe = CardInstance {
                x: x.or(card.x),
                embiggened: embiggen.or(card.embiggened),
                ..card.clone()
            };
            let cost = crate::mana::play_cost(state, &probe);
            if crate::cost_rules::why_play_banned(state, player, &probe, cost).is_err() {
                continue;
            }
            out.push(PlayPrice {
                x: *x,
                embiggen: *embiggen,
                probe,
                cost,
            });
        }
    }
    out
}

/// R667: the costs a play of this card from its player's hand could pay now, whatever mana is left.
pub fn offered_play_costs(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<i32> {
    play_prices(state, player, card)
        .into_iter()
        .map(|price| price.cost)
        .collect::<IndexSet<i32>>()
        .into_iter()
        .collect()
}

/// R1200 (ME-RANDOMTARGETS): the face §10.5 step 1 reads a play's choices against — the one
/// `why_choices_refused` computes (`resolving_face` with the action's own prices).
pub fn face_for_action(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    action: &PlayAction,
) -> CardInstance {
    resolving_face(
        state,
        player,
        card,
        cost_with(state, card, action.x, action.embiggen),
    )
}

/// R1200 (ME-RANDOMTARGETS): a play's picks with every `target` declaration's slice left out —
/// what `legal_actions` offers while a Mayor acts, so the client asks for no target. The slices
/// are `split_selections`' own, so the draw below inverts them exactly. An empty remainder is
/// `None`, the same as no picks at all.
pub fn without_target_picks(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    decls: &[TargetDecl],
    targets: &[Selection],
    modes: &[String],
) -> Option<Vec<Selection>> {
    let active = active_target_decls(decls, modes);
    let offered: Vec<Vec<Selection>> = active
        .iter()
        .map(|decl| legal_selections_for(state, player, card, decl))
        .collect();
    let kept: Vec<Selection> = split_selections(&active, &offered, targets)
        .into_iter()
        .enumerate()
        .filter(|(index, _)| active[*index].kind != PromptKind::Target)
        .flat_map(|(_, slice)| slice)
        .collect();
    if kept.is_empty() { None } else { Some(kept) }
}

/// R1200 (ME-RANDOMTARGETS): rebuild a stripped play's full target list, drawing each `target`
/// declaration's picks at random (`random_targets::draw_picks`) with `subsets_for`'s own bounds —
/// `take_for` picks off the list for every declaration but the last, which takes the rest — so the
/// full list splits again exactly as `legal_actions` split it. Any other declaration takes its
/// slice of the picks the action carried. Picks left over refuse the play: a named target is no
/// play while a Mayor acts.
pub fn with_drawn_target_picks(
    state: &GameState,
    rng: &mut Rng,
    player: PlayerId,
    card: &CardInstance,
    decls: &[TargetDecl],
    given: &[Selection],
    modes: &[String],
) -> Result<Vec<Selection>, EngineError> {
    let active = active_target_decls(decls, modes);
    let offered: Vec<Vec<Selection>> = active
        .iter()
        .map(|decl| legal_selections_for(state, player, card, decl))
        .collect();
    let mut out: Vec<Selection> = Vec::new();
    let mut at = 0usize;
    for (index, decl) in active.iter().enumerate() {
        let is_last = index + 1 == active.len();
        if decl.kind == PromptKind::Target {
            let options = &offered[index];
            let low = take_for(decl, options.len()) as i32;
            let high = if is_last {
                at_most(decl.max, options.len()) as i32
            } else {
                low
            };
            out.extend(crate::random_targets::draw_picks(
                state,
                rng,
                player,
                options,
                low,
                high,
                crate::targeting::target_aim(decl),
            ));
            continue;
        }
        let rest = &given[at.min(given.len())..];
        if is_last {
            out.extend_from_slice(rest);
            at = given.len();
            continue;
        }
        let take = take_for(decl, offered[index].len()).min(rest.len());
        out.extend_from_slice(&rest[..take]);
        at += take;
    }
    if at != given.len() {
        return Err(EngineError::new("targets are drawn at random while a Mayor acts"));
    }
    Ok(out)
}

/// R81, R90's enumeration with the payment left to the caller: for each price the card's X and embiggen
/// choices come to, `payments` answers the ways a play may pay it — none, and that price is not offered;
/// `{}` for a price paid in mana alone. A hand card pays in mana (`play_actions_for`); a card a permission
/// lets its player play from the graveyard may also pay with Plague Counters (`graveyard_play.rs`).
///
/// R455: a price a ban forbids is never offered (`cost_rules::why_play_banned`). R391: each Tribute set is
/// paired with the zones it leaves open, the open ones and the ones it empties itself, and never with a
/// zone another set empties.
pub fn priced_play_actions(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    payments: impl Fn(i32) -> Vec<PlayPayment>,
) -> Vec<PlayAction> {
    let mut out: Vec<PlayAction> = Vec::new();
    let tribute_sets = legal_tribute_sets(state, player, card);

    for PlayPrice {
        x, embiggen, cost, ..
    } in play_prices(state, player, card)
    {
        let paid = payments(cost);
        if paid.is_empty() {
            continue;
        }
        // R214: the choices of the face step 5 will resolve, which this price decides (#64).
        let face = resolving_face(state, player, card, cost);
        let bound = declares_bound_tribute(state, &face);
        for tributes in &tribute_sets {
            // R1086: the plain zones first, then the Magnetic host zones after them (D14: a game that
            // never plays Magnetic lists what it listed before).
            let zones: Vec<(Option<ZoneChoice>, Option<bool>)> = if needs_zone(state, card) {
                let mut paired: Vec<(Option<ZoneChoice>, Option<bool>)> =
                    legal_zones_for(state, player, card, tributes)
                        .into_iter()
                        .map(|zone| (Some(zone), None))
                        .collect();
                paired.extend(
                    legal_magnetic_zones_for(state, player, card, tributes)
                        .into_iter()
                        .map(|zone| (Some(zone), Some(true))),
                );
                paired
            } else {
                vec![(None, None)]
            };
            for (zone, magnetic) in &zones {
                for choices in play_choice_combinations(state, player, &face, None) {
                    let targets: &[Selection] = choices.targets.as_deref().unwrap_or(&[]);
                    let modes: &[String] = choices.modes.as_deref().unwrap_or(&[]);
                    // R123: the declared Tribute's pick names the units this play tributes, and no others.
                    if bound && !tribute_picks_agree(state, player, &face, targets, modes, tributes) {
                        continue;
                    }
                    // B5 E5, R450, R682: the targets' discard cost is random at pay time, so it lists no
                    // paying sets — one action, offered only when the cost can be paid at all.
                    let owed = targeting_discards_required(state, player, &face, targets, modes, None);
                    if crate::targeting::why_targeting_discards_unpayable(
                        state,
                        player,
                        owed,
                        &play_uses(card, targets),
                    )
                    .is_err()
                    {
                        continue;
                    }
                    for payment in &paid {
                        let mut pushed = PlayAction {
                            instance_id: card.id.clone(),
                            zone: *zone,
                            x,
                            embiggen,
                            magnetic: *magnetic,
                            tributes: if tributes.is_empty() {
                                None
                            } else {
                                Some(tributes.clone())
                            },
                            targets: choices.targets.clone(),
                            modes: choices.modes.clone(),
                            plague: payment.plague.clone(),
                            face_down: None,
                        };
                        // R1200: while a Mayor acts the play carries no declared targets — the
                        // reducer draws them — so collapsed choices list one action, not one per
                        // target set.
                        if crate::random_targets::targets_random(state) {
                            pushed.targets = without_target_picks(
                                state,
                                player,
                                &face,
                                &declared_targets(state, &face),
                                targets,
                                modes,
                            );
                            if out.contains(&pushed) {
                                continue;
                            }
                        }
                        out.push(pushed);
                    }
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// §10.5 step 1: the refusal
// ---------------------------------------------------------------------------

fn plural(count: i32, one: &str) -> String {
    if count == 1 {
        format!("{count} {one}")
    } else {
        format!("{count} {one}s")
    }
}

fn refuse(message: String) -> Result<(), EngineError> {
    Err(EngineError::new(message))
}

/// R1086: the refusal for a Magnetic play. A play with `magnetic: false` is a plain play. The
/// card must play Magnetic at all, the zone must name a unit-row lane in range, and that zone must
/// hold a host — read off `magnetic_host_at`, the same function the listing uses.
fn refuse_magnetic(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    zone: Option<&ZoneChoice>,
    tributes: &[String],
) -> Result<(), EngineError> {
    let name = name_of(state, &card.def_id);
    if !plays_magnetic(state, card) {
        return refuse(format!("{name} is not Magnetic"));
    }
    let Some(zone) = zone else {
        return refuse(format!("{name} played as Magnetic takes a zone"));
    };
    if zone.row != Row::Units {
        return refuse(format!("{name} played as Magnetic goes in the units row"));
    }
    if zone.lane < 1 || zone.lane > crate::zones::row_size(zone.row) {
        return refuse(format!("there is no {} zone {}", zone.row, zone.lane));
    }
    let slot = crate::zones::ZoneSlot {
        player,
        row: zone.row,
        lane: zone.lane,
    };
    if magnetic_host_at(state, player, &slot, tributes).is_none() {
        return refuse(format!("that units zone holds no Unit for {name}"));
    }
    Ok(())
}

fn refuse_zone(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    zone: Option<&ZoneChoice>,
    tributes: &[String],
) -> Result<(), EngineError> {
    let name = name_of(state, &card.def_id);
    let needs = needs_zone(state, card);

    let Some(zone) = zone else {
        if !needs {
            return Ok(());
        }
        let row = row_for_card(state, card);
        // §3.2: playing a permanent from hand requires an open zone in the right row — or, R391, one the
        // play's own Tribute empties.
        if default_zone_for(state, player, card, tributes).is_none() {
            return refuse(format!("no free {row} zone"));
        }
        return Ok(());
    };
    if !needs {
        return refuse(format!("{name} takes no zone"));
    }

    let row = row_for_card(state, card);
    // R446: a Unit may name a backrow zone whose card carries one (Classic+ #33 Ivory Tower, R653).
    if row == Row::Units && zone.row == Row::Backrow {
        // TS also refused a lane that is not a whole number; an `i32` always is.
        if zone.lane < 1 || zone.lane > crate::zones::row_size(zone.row) {
            return refuse(format!("there is no {} zone {}", zone.row, zone.lane));
        }
        let slot = ZoneSlot {
            player,
            row: zone.row,
            lane: zone.lane,
        };
        crate::zones::why_cannot_carry(state, slot)?;
        if immutable_fuser(state, &slot) {
            return refuse("an Immutable card takes no Unit in".to_string());
        }
        return Ok(());
    }
    if zone.row != row {
        return refuse(format!("{name} goes in the {row} row"));
    }
    // §3.2: a zone is one of the row's lanes, numbered 1 up — never a place between two of them.
    if zone.lane < 1 || zone.lane > crate::zones::row_size(row) {
        return refuse(format!("there is no {row} zone {}", zone.lane));
    }

    // §6.2 Stack: an occupied unit zone is a legal zone for a Stack card, and only occupancy is
    // waived — `accepts_stack` still refuses a Locked or Reborn-reserved zone (R64).
    let slot = ZoneSlot {
        player,
        row: zone.row,
        lane: zone.lane,
    };
    // R391: a zone the play's own Tribute empties is open for it once the Tribute is paid.
    let takes_it = if plays_on_stack(state, card) {
        accepts_stack(state, &slot)
    } else {
        crate::zones::is_open(state, slot)
            || freed_by_tribute(state, &slot, tributes)
            || onto_stack_base(state, player, card, &slot)
    };
    if !takes_it {
        return refuse(format!("that {row} zone is not open"));
    }
    Ok(())
}

fn refuse_x(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    x: Option<i32>,
) -> Result<(), EngineError> {
    let name = name_of(state, &card.def_id);
    // B5 E14, R545: a copier's X is its copied text's, up to the mana left once its own price is paid.
    if crate::subsystems::copied_text::copies_text(card) {
        if !chooses_x(state, card) {
            return match x {
                None => Ok(()),
                Some(_) => refuse(format!("{name} has no X to choose now")),
            };
        }
        return why_x_refused(state, player, x.unwrap_or(0), Some(most_x(state, player, card)));
    }
    if !crate::mana::is_x_cost(state, card) {
        return match x {
            None => Ok(()),
            Some(_) => refuse(format!("{name} does not cost X")),
        };
    }
    // R43: an X the card's own cost hook fixes (#98) is not the player's to choose, so whatever the
    // action names is ignored — never recorded on the card, never read by the cost (`play_steps`).
    if !chooses_x(state, card) {
        return Ok(());
    }
    // R348: an X the play leaves out is the old default 0, which is no longer one to choose.
    why_x_refused(state, player, x.unwrap_or(0), None)
}

fn refuse_embiggen(
    state: &GameState,
    card: &CardInstance,
    embiggen: Option<bool>,
) -> Result<(), EngineError> {
    if has_embiggen_price(state, card) {
        return Ok(());
    }
    if embiggen == Some(true) {
        return refuse(format!("{} has no embiggen price", name_of(state, &card.def_id)));
    }
    Ok(())
}

fn refuse_tributes(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    tributes: Option<&[String]>,
) -> Result<(), EngineError> {
    let name = name_of(state, &card.def_id);
    let need = tribute_cost_of(state, card);
    let picked: &[String] = tributes.unwrap_or(&[]);
    if need == 0 {
        if picked.is_empty() {
            return Ok(());
        }
        return refuse(format!("{name} needs no Tribute"));
    }

    let legal: IndexMap<String, &CardInstance> = legal_tribute_units(state, player, card)
        .into_iter()
        .map(|unit| (unit.id.clone(), unit))
        .collect();
    // §6.3: the Tribute is an additional *cost*, so a board that cannot pay it refuses the play (#66).
    let every: Vec<&CardInstance> = legal.values().copied().collect();
    if tribute_total(state, &every) < need {
        return refuse(format!("{name} needs Tribute {need}"));
    }

    let mut chosen: Vec<&CardInstance> = Vec::new();
    let mut seen: IndexSet<&str> = IndexSet::new();
    for id in picked {
        let Some(unit) = legal.get(id) else {
            return refuse(format!("{id} cannot be tributed to play {name}"));
        };
        if seen.contains(id.as_str()) {
            return refuse(format!("{name} cannot tribute the same unit twice"));
        }
        seen.insert(id.as_str());
        chosen.push(*unit);
    }

    if tribute_total(state, &chosen) < need {
        return refuse(format!("{name} needs Tribute {need}"));
    }
    if !is_minimal_tribute(state, &chosen, need) {
        return refuse(format!("{name} tributes {need}, no more"));
    }
    Ok(())
}

fn refuse_targets(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    selections: &[Selection],
    modes: &[String],
    declared: Option<&[TargetDecl]>,
) -> Result<(), EngineError> {
    let own;
    let declared: &[TargetDecl] = match declared {
        Some(declared) => declared,
        None => {
            own = declared_targets(state, card);
            &own
        }
    };
    let name = name_of(state, &card.def_id);
    // R90: a declaration that belongs to modes the play did not choose asks for nothing.
    let decls = active_target_decls(declared, modes);
    if decls.is_empty() && !declared.is_empty() {
        if selections.is_empty() {
            return Ok(());
        }
        return refuse(format!("{name} takes no targets for that choice"));
    }
    // R90: "a card that declared nothing takes nothing".
    if decls.is_empty() {
        if selections.is_empty() {
            return Ok(());
        }
        return refuse(format!("{name} takes no targets"));
    }

    let offered: Vec<Vec<Selection>> = decls
        .iter()
        .map(|decl| legal_selections_for(state, player, card, decl))
        .collect();
    let slices = split_selections(&decls, &offered, selections);

    for (index, decl) in decls.iter().enumerate() {
        let got: &[Selection] = slices.get(index).map(Vec::as_slice).unwrap_or(&[]);
        let options: &[Selection] = offered.get(index).map(Vec::as_slice).unwrap_or(&[]);

        // R703: unless the play needs it, in which case the play is refused.
        if unmet_requirement(decl, options.len()) {
            return refuse(format!(
                "{name} cannot be played without {}",
                plural(decl.min, "legal target")
            ));
        }
        // R90: a declaration the board cannot satisfy asks for what the board has, not for its minimum.
        let required = take_for(decl, options.len());
        if got.len() < required {
            return refuse(format!(
                "{name} needs {} for that choice",
                plural(required as i32, "target")
            ));
        }
        if (got.len() as i64) > i64::from(decl.max) {
            return refuse(format!(
                "{name} takes at most {} for that choice",
                plural(decl.max, "target")
            ));
        }

        let keys: IndexSet<String> = options.iter().map(selection_key).collect();
        let mut used: IndexSet<String> = IndexSet::new();
        for selection in got {
            let key = selection_key(selection);
            if !keys.contains(&key) {
                return refuse(format!("that is not a legal target for {name}"));
            }
            // R90: one declaration may not pick the same card twice; two declarations may.
            if used.contains(&key) {
                return refuse(format!("{name} cannot name the same target twice in one choice"));
            }
            used.insert(key);
        }
    }
    Ok(())
}

fn refuse_modes(
    state: &GameState,
    card: &CardInstance,
    modes: &[String],
    decls: Option<&[ModeDecl]>,
) -> Result<(), EngineError> {
    let own;
    let decls: &[ModeDecl] = match decls {
        Some(decls) => decls,
        None => {
            own = declared_modes(state, card);
            &own
        }
    };
    let name = name_of(state, &card.def_id);
    if decls.is_empty() {
        if modes.is_empty() {
            return Ok(());
        }
        return refuse(format!("{name} takes no mode choices"));
    }
    if modes.len() > decls.len() {
        return refuse(format!(
            "{name} takes {}, not {}",
            plural(decls.len() as i32, "mode choice"),
            modes.len()
        ));
    }

    for (index, decl) in decls.iter().enumerate() {
        let Some(option) = modes.get(index) else {
            return refuse(format!(
                "{name} needs a mode choice for each of its {}",
                plural(decls.len() as i32, "mode declaration")
            ));
        };
        if !decl.options.contains(option) {
            return refuse(format!("\"{option}\" is not a mode of {name}"));
        }
    }
    Ok(())
}

/// ME-ALTPLAY, R1040, R1044: refusal for a face-down play — no kind, a timing outside its
/// timings, a named zone outside the open backrow zones, or any target or mode. X, embiggen and
/// tributes are still checked.
fn why_face_down_refused(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    action: &PlayAction,
) -> Result<(), EngineError> {
    let Some(reveal) = action.face_down else {
        return refuse("no face-down timing".to_string());
    };
    let from_graveyard = matches!(card.zone, crate::wire::Zone::Graveyard { .. });
    let Some(kind) = crate::alt_play::face_down_kind(state, player, card, from_graveyard) else {
        return refuse(format!(
            "{} cannot be played face-down",
            name_of(state, &card.def_id)
        ));
    };
    if !crate::alt_play::timings(kind).contains(&reveal) {
        return refuse(format!(
            "{} cannot reveal at that time",
            name_of(state, &card.def_id)
        ));
    }
    if let Some(zone) = action.zone.as_ref() {
        let open = crate::alt_play::open_backrow_zones(state, player);
        if !open.contains(zone) {
            return refuse("no free backrow zone".to_string());
        }
    }
    if action.targets.is_some() || action.modes.is_some() {
        return refuse(format!(
            "{} declares no targets or modes face-down",
            name_of(state, &card.def_id)
        ));
    }
    // R1086 beside R1044: a face-down play goes into an open backrow zone, never onto a host.
    if action.magnetic == Some(true) {
        return refuse(format!(
            "{} is not played Magnetic face-down",
            name_of(state, &card.def_id)
        ));
    }
    refuse_x(state, player, card, action.x)?;
    refuse_embiggen(state, card, action.embiggen)?;
    refuse_tributes(state, player, card, action.tributes.as_deref())?;
    // B5 E5 discards still apply to a face-down play.
    let face = resolving_face(
        state,
        player,
        card,
        cost_with(state, card, action.x, action.embiggen),
    );
    let required = targeting_discards_required(state, player, &face, &[], &[], None);
    crate::targeting::why_targeting_discards_unpayable(state, player, required, &play_uses(card, &[]))
}

/// §10.5 step 1, R90: every choice the play carried, checked against what the card declared and what
/// the board allows. Returns `Ok` when the play's choices are legal, and the refusal otherwise —
/// `reduce` returns that message and leaves the state untouched (§9.3).
///
/// Mana affordability is not a choice and stays with `reduce`'s pay step (§10.5 step 2).
pub fn why_choices_refused(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    action: &PlayAction,
) -> Result<(), EngineError> {
    if action.face_down.is_some() {
        return why_face_down_refused(state, player, card, action);
    }
    let tributes: &[String] = action.tributes.as_deref().unwrap_or(&[]);
    if action.magnetic == Some(true) {
        refuse_magnetic(state, player, card, action.zone.as_ref(), tributes)?;
    } else {
        refuse_zone(state, player, card, action.zone.as_ref(), tributes)?;
    }
    refuse_x(state, player, card, action.x)?;
    refuse_embiggen(state, card, action.embiggen)?;
    refuse_tributes(state, player, card, action.tributes.as_deref())?;
    // R214: the targets and modes are the resolving face's, so they are read against it. The Tribute
    // above is paid at step 2, before step 3 can change the face, so it is the face in hand's.
    let face = resolving_face(
        state,
        player,
        card,
        cost_with(state, card, action.x, action.embiggen),
    );
    let targets: &[Selection] = action.targets.as_deref().unwrap_or(&[]);
    let modes: &[String] = action.modes.as_deref().unwrap_or(&[]);
    refuse_targets(state, player, &face, targets, modes, None)?;
    refuse_modes(state, &face, modes, None)?;
    // R123: a Tribute declared with an amount names the same units in `targets` and `tributes`.
    if !tribute_picks_agree(state, player, &face, targets, modes, tributes) {
        return refuse(format!(
            "{}'s Tribute pick must be a unit it tributes",
            name_of(state, &face.def_id)
        ));
    }
    // B5 E5, R450, R682: a declared target that costs discards needs that many other cards held —
    // the discards are random at pay time, so the action carries none. Never the card being played.
    let required = targeting_discards_required(state, player, &face, targets, modes, None);
    crate::targeting::why_targeting_discards_unpayable(state, player, required, &play_uses(card, targets))
}

/// R450: the hand cards a play itself uses — the card played and any hand card it picks — which pay no cost.
pub fn play_uses(card: &CardInstance, targets: &[Selection]) -> Vec<String> {
    let mut out: Vec<String> = vec![card.id.clone()];
    for selection in targets {
        if let Selection::Instance { instance_id } = selection {
            out.push(instance_id.clone());
        }
    }
    out
}

// ---------------------------------------------------------------------------
// ---- v0.2.0: activate and turn (B3.2 activation choices, R384) ----
// ---------------------------------------------------------------------------

/// The declarations a set of choices answers when they are not the card's own play-time ones: an
/// Activate ability's `targets` and `modes` (B3.2 rule 5, R384), which travel in the `activate` action
/// as a play's travel in `play` (R81) and are read by the same rules (R90).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DeclaredChoices {
    pub targets: Vec<TargetDecl>,
    pub modes: Vec<ModeDecl>,
}

/// R384, R90: the targets and modes an `activate` action carried, checked against the ability's
/// declarations and the board exactly as a play's are (`refuse_targets`, `refuse_modes`), so the two
/// refusals cannot drift apart. `card` is the card whose ability it is: `excludeSelf` reads it.
pub fn why_declared_choices_refused(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    declared: &DeclaredChoices,
    targets: &[Selection],
    modes: &[String],
) -> Result<(), EngineError> {
    refuse_targets(state, player, card, targets, modes, Some(&declared.targets))?;
    refuse_modes(state, card, modes, Some(&declared.modes))
}
