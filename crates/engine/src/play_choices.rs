//! The choices a play carries, built and validated (SPEC §10.5 step 1, §10.6, R81, R90): Zone, X,
//! embiggen, Tribute and the script's targets and modes travel in the `play` action, and this module is
//! the one place they become options and are checked, so `reduce`'s play path and `legal_actions` read
//! the same rules. The enumeration is bounded; `listed_play_choices` says when the bound cut it (#992).
//!
//! R90: declarations read the flat `targets` list in order, each taking its minimum and the last the
//! remainder; one may not name a card twice, two may name the same card; one the board cannot satisfy
//! fizzles on resolution unless the play needs it (`required`, R703), and then the play is neither
//! offered nor accepted, as while its play gate fails (R1851000). §9.1: a `hand` pick offers only the
//! chooser's own hand; R13: a unit pick only the top of a Stack pile. §6.2: a Stack card's legal zones
//! are the unit zones merely unlocked and unreserved (R64).
//!
//! A function that reads a card's script takes the state first (SURFACE §6.6); refusals are
//! `Result<(), EngineError>`, their text fixed by SURFACE §4.4.9.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{MAX_CHOICE_COMBINATIONS, MIN_CHOSEN_X};
use crate::graveyard_play::PlayPayment;
use crate::rng::Rng;
use crate::script::TargetCheckArgs;
use crate::state::{CardInstance, EngineError, GameState, ModifierKind};
use crate::wire::{
    ActionBody, CardCost, CardType, FilterOf, FilterSide, KeywordKind, ModeDecl, Phase, PlagueSpend,
    PlayerId, PromptKind, RevealAt, Row, Selection, Tag, TargetDecl, TargetFilter, Zone, ZoneChoice,
    opponent_of,
};
use crate::zones::ZoneSlot;

// §7's Sheep Token Tribute values live in `config.rs` (CLAUDE.md rule 9); re-exported for callers here.
pub use crate::config::{RADIANT_SHEEP_TRIBUTE_VALUE, SHEEP_TRIBUTE_VALUE};

/// The fields of `ActionBody::Play` as a struct, without the caller's `playerId` and `nonce`. It
/// serializes as that action body, `"type": "play"` included.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[serde(into = "ActionBody", try_from = "ActionBody")]
pub struct PlayAction {
    pub instance_id: String,
    pub zone: Option<ZoneChoice>,
    pub x: Option<i32>,
    pub embiggen: Option<bool>,
    /// R1086: play this Equip card onto the zone's host Unit (ME-EQUIP).
    pub equip: Option<bool>,
    /// Units sacrificed to pay a Tribute cost (§6.3).
    pub tributes: Option<Vec<String>>,
    pub targets: Option<Vec<Selection>>,
    pub modes: Option<Vec<String>>,
    /// B5 E11, E19: Plague Counters paying part of the price of a play from the graveyard.
    pub plague: Option<PlagueSpend>,
    /// R783004: play face-down as a Trap by the card's Cloaked (`cloaked`).
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
            equip: action.equip,
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
                equip,
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
                equip,
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

// R90's bound is `MAX_CHOICE_COMBINATIONS` (BUILD §2): a cut drops combinations, never a pick
// (`cross_product`, `interleaved`); Tribute sets are the play's price, listed whole (`legal_tribute_sets`).

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

fn name_of(state: &GameState, def_id: &str) -> String {
    crate::catalog::def_of(Some(state), def_id).name.clone()
}

fn at_most(n: i32, count: usize) -> usize {
    (n.max(0) as usize).min(count)
}

// What the card asked for

/// The `targets` a card's running face declares (§10.9, R81); the empty list when it declares none.
pub fn declared_targets(state: &GameState, card: &CardInstance) -> Vec<TargetDecl> {
    crate::scripts::script_of(state, card).targets.clone()
}

/// The `modes` a card's running face declares: "Choose one" and Silly Silas's direction (R81).
pub fn declared_modes(state: &GameState, card: &CardInstance) -> Vec<ModeDecl> {
    crate::scripts::script_of(state, card).modes.clone()
}

/// R831008: whether this player may choose this option of the card's own declared modes now
/// (`Script.mode_allowed`; Meditative #98 Showdown's empty lane).
pub fn mode_option_allowed(state: &GameState, player: PlayerId, card: &CardInstance, option: &str) -> bool {
    crate::scripts::script_of(state, card)
        .mode_allowed
        .as_ref()
        .is_none_or(|allowed| allowed(state, player, option))
}

/// R1851000: whether this player may play this card now (`Script.play_allowed`; Meditative #53
/// Prestige's Radiant face). `card` is the resolving face, so a copier answers with its copied face's
/// gate (R399, R1851002); `listed_play_choices` and `why_choices_refused` both read it, a cast never
/// does (R70, R1851001).
pub fn play_allowed(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    crate::scripts::script_of(state, card)
        .play_allowed
        .as_ref()
        .is_none_or(|allowed| allowed(state, player))
}

/// The target declarations a play with these modes answers (R90, §8 Conventions): one that belongs to
/// unchosen modes only (`forModes`, #24) takes no slot of the flat `targets` list, and one of a chosen
/// mode asks for its minimum, so a damage mode cannot name nobody while a target exists.
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

// The face a play resolves with (§10.5 step 3, R213, R214)

/// Every permanent that acts for this player: the tops of their unit piles and their backrow (§3.2),
/// a face-down card left out, whose text sleeps (R1487000).
fn permanents_of(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    let mut out: Vec<&CardInstance> = Vec::new();
    for row in ROWS {
        for slot in crate::zones::slots_of(player, row) {
            if let Some(held) = crate::zones::card_at(state, slot)
                && !crate::preview::lasting_text_off(state, held)
            {
                out.push(held);
            }
        }
    }
    out
}

/// §8 #64 Gifted Program, §10.5 step 3: whether a play this player makes now, paying `cost_paid` (R56's
/// cost actually paid, 0 for a cast, R70), is made Radiant. R213: "the first card costing 1 or less YOU
/// play each turn" counts the player's plays (`costsPaid`), not the Field Spell's history; only this
/// player's side counts (§8 Conventions), and never its own play: step 3 runs before step 4 (R119).
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

/// R214: the card whose declarations a play's targets and modes answer, as it will resolve. §10.5 step 3
/// can make it Radiant (#64) after step 1 reads the choices, and a Radiant face can declare others (#87,
/// #48), so step 1 checks, and `legal_actions` offers, the face step 5 will run: its cost is known.
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
    // the face that Spell was played on.
    crate::subsystems::copied_text::text_face_of(state, &face)
}

/// Classic+ #68 Organic Produce, R449: whether a permanent on this player's side makes every card
/// carrying one of its tags Radiant as the player plays it (`radiantPlaysTagged`), on every such play (a
/// cast included, R70). It never catches its own play: step 3 runs before step 4 (R119).
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

/// §10.5 step 3: whether this play is made Radiant as it is played: #64 Gifted Program (R213), a tag
/// rule (Classic+ #68), Meditative #40 Feng Shui (R983), #53 Prestige's next play (R831005), Meditative+
/// #22 Mountaintop Garden's dearer price (R1181002, read off the card stamped with its embiggen choice).
/// Step 1 reads it to know the face the choices answer (R214), and step 3 applies it.
pub fn play_made_radiant(state: &GameState, player: PlayerId, card: &CardInstance, cost_paid: i32) -> bool {
    gifted_makes_radiant(state, player, cost_paid)
        || tagged_play_radiant(state, player, card)
        || crate::subsystems::feng_shui::rewards(state, player, card)
        || radiant_next_play(state, player)
        || crate::mana::garden_dearer_price(state, card).is_some()
}

/// R831005: whether a live `radiantNextPlay` modifier of the player makes their next play Radiant.
pub fn radiant_next_play(state: &GameState, player: PlayerId) -> bool {
    state.players[player].mods.iter().any(|modifier| {
        matches!(modifier.kind, ModifierKind::RadiantNextPlay)
            && crate::mana::modifier_is_live(state, modifier)
    })
}

/// What a play of this card with these prices would pay, read the way §10.5 step 1 reads it (R65) — as
/// a play wherever the card lies, so a play from a graveyard pays the player's prices (E11, R454).
fn cost_with(state: &GameState, card: &CardInstance, x: Option<i32>, embiggen: Option<bool>) -> i32 {
    crate::mana::play_cost(state, &stamped_for_play(state, card, x, embiggen))
}

/// The card stamped with a play's embiggen choice, as §10.5 step 1 stamps it before reading the price;
/// R214, R1181002: the face a play's choices answer is read off it (Mountaintop Garden's dearer price).
fn stamped_with_embiggen(state: &GameState, card: &CardInstance, embiggen: Option<bool>) -> CardInstance {
    if has_dearer_price(state, card) {
        CardInstance {
            embiggened: Some(embiggen == Some(true)),
            ..card.clone()
        }
    } else {
        card.clone()
    }
}

/// R1868000: the card as a play at these prices declares its choices, its embiggen and X stamped as
/// §10.5 step 1 stamps them, so a declaration's named check reads the play's X (Stinkyscape #99). A set
/// X (R1848003) is on the card already, and a card that chooses no X keeps the one it has.
fn stamped_for_play(
    state: &GameState,
    card: &CardInstance,
    x: Option<i32>,
    embiggen: Option<bool>,
) -> CardInstance {
    CardInstance {
        x: if chooses_x(state, card) {
            Some(x.unwrap_or(0))
        } else {
            card.x
        },
        ..stamped_with_embiggen(state, card, embiggen)
    }
}

/// #492, R81: what a play of this card at its embiggen price costs now (`cost_with`, as the pay step
/// charges it), or None for a card with none (R1181002: a Radiant card under Mountaintop Garden). The
/// view's `embiggenCost` (§10.8) is this number, so the client never prices a play (CLAUDE.md rule 7).
pub fn embiggen_play_cost(state: &GameState, card: &CardInstance) -> Option<i32> {
    legal_embiggen_choices(state, card)
        .contains(&true)
        .then(|| cost_with(state, card, None, Some(true)))
}

// Zone, X and embiggen

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

/// §6.2 Stack: "may be played onto an occupied zone" (§3.2), read off the card's keywords where it is
/// (§10.4, B5 E38). A backrow card with Stack tops an occupied backrow zone too (B5 E21, R447).
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

/// §3.2: the zone a Stack card may enter. Occupancy is the refusal Stack lifts, so what is left is a
/// Locked zone, a zone reserved for a dying Reborn unit (R64) and a backrow zone carrying a Unit (R446).
fn accepts_stack(state: &GameState, slot: &ZoneSlot) -> bool {
    crate::zones::accepts_stack_card(state, slot, Default::default())
}

/// R1081000: whether this card may also be played into a backrow zone: a Unit with Flying, played
/// face-up, read off its keywords where it is (§10.4).
fn flies(state: &GameState, card: &CardInstance) -> bool {
    if card.set_as.is_some() {
        return false;
    }
    if !needs_zone(state, card) {
        return false;
    }
    if row_for_card(state, card) != Row::Units {
        return false;
    }
    crate::layers::unit_has(state, card, KeywordKind::Flying)
}

/// R1081000: whether a Flying play stands in this backrow zone, the predicate `legal_zones_for` lists
/// by. Without Stack the zone must be open or freed by the play's own Tribute; with Stack occupancy is
/// waived, but a carrier takes a Unit only as its Unit (R446, `zones::why_cannot_carry`, R653).
fn flying_takes_backrow(
    state: &GameState,
    card: &CardInstance,
    slot: &ZoneSlot,
    tributes: &[String],
) -> bool {
    // R653, R23: an Immutable fuser takes no Unit in, standing on it or beneath it either.
    if immutable_fuser(state, slot) {
        return false;
    }
    if plays_on_stack(state, card) {
        accepts_stack(state, slot) && !holds_carrier(state, slot)
    } else {
        crate::zones::is_open(state, slot) || freed_by_tribute(state, slot, tributes)
    }
}

/// R446: whether the card acting in this zone is a carrier, which a Unit enters only as its Unit.
fn holds_carrier(state: &GameState, slot: &ZoneSlot) -> bool {
    crate::zones::card_at(state, slot).is_some_and(|top| crate::zones::is_carrier(state, top))
}

/// MD-F12, R1281: whether this Unit may be played onto a stack base.
fn onto_stack_base(state: &GameState, player: PlayerId, card: &CardInstance, slot: &ZoneSlot) -> bool {
    card.set_as.is_none()
        && row_for_card(state, card) == Row::Units
        && crate::zones::stack_base_at(state, player, slot)
}

/// R1086 (ME-EQUIP): whether this card may be played as Equip: a Unit with the Equip keyword, read off
/// its keywords where it is (§10.4).
pub fn plays_equip(state: &GameState, card: &CardInstance) -> bool {
    if !needs_zone(state, card) {
        return false;
    }
    if row_for_card(state, card) != Row::Units {
        return false;
    }
    crate::layers::unit_has(state, card, KeywordKind::Equip)
}

/// R1086: the host an Equip play onto `slot` fuses into: the zone's top card when it is a Unit its
/// controller acts with, not Immutable (R23), not Dormant (R2260000, R1839000) and not one this play
/// tributes; R1869000: a host that lends Equip hosts in its backrow zone too, a Unit or not. The
/// legal-action listing and the refusal both read it, so they cannot disagree.
pub fn equip_host_at(
    state: &GameState,
    player: PlayerId,
    slot: &ZoneSlot,
    tributes: &[String],
) -> Option<String> {
    if !crate::zones::accepts_stack_card(state, slot, Default::default()) {
        return None;
    }
    let top = crate::zones::card_at(state, slot)?;
    if !lends_equip_at(state, slot) {
        // R1081000: a Unit standing in a backrow zone hosts as one in a unit zone does; no other
        // backrow card is a Unit (a carrier's Unit stands on the carrier, never as the zone's card).
        if slot.row == Row::Backrow && !crate::zones::is_standing_unit(state, top) {
            return None;
        }
        if crate::faces::card_type_of(state, top) != CardType::Unit {
            return None;
        }
    }
    if crate::layers::unit_has(state, top, KeywordKind::Immutable) {
        return None;
    }
    if crate::slumber::is_slumbering(top) {
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

/// R1869000: whether this card is a Unit card that needs a zone, the cards a host that lends Equip
/// takes as an Equip play though they have no Equip.
fn plays_as_unit(state: &GameState, card: &CardInstance) -> bool {
    needs_zone(state, card) && row_for_card(state, card) == Row::Units
}

/// R1869000: whether the card on top of `slot` lends Equip (`staticFlags.lendsEquip`, Stinkyscape #16
/// Rod of Ages) — while its definition is no fusion.
fn lends_equip_at(state: &GameState, slot: &ZoneSlot) -> bool {
    crate::zones::card_at(state, slot).is_some_and(|top| {
        crate::scripts::flags_of(state, top).lends_equip == Some(true)
            && crate::subsystems::fuse::fused_ingredients(state, &top.def_id).is_none()
    })
}

/// `equip_onto_at` for a Unit card whose Equip is already read. The lending is read first, so a plain
/// Unit costs no layer read where nothing lends.
fn equip_host_for(
    state: &GameState,
    player: PlayerId,
    equip: bool,
    slot: &ZoneSlot,
    tributes: &[String],
) -> Option<String> {
    if !equip && !lends_equip_at(state, slot) {
        return None;
    }
    equip_host_at(state, player, slot, tributes)
}

/// R1086, R1869000: the host `card` fuses into when played onto `slot` as an Equip play: an Equip card
/// onto any host, any other Unit card onto one that lends Equip. The listing and the refusal read it.
pub fn equip_onto_at(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    slot: &ZoneSlot,
    tributes: &[String],
) -> Option<String> {
    if !plays_as_unit(state, card) {
        return None;
    }
    equip_host_for(state, player, plays_equip(state, card), slot, tributes)
}

/// R1086: the zones that hold a host for an Equip play, in lane order: the unit row's, then the backrow
/// zones a Unit stands in (R1081000). Empty unless the card plays Equip or a host lends it (R1869000).
pub fn legal_equip_zones_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    tributes: &[String],
) -> Vec<ZoneChoice> {
    if !plays_as_unit(state, card) {
        return Vec::new();
    }
    let equip = plays_equip(state, card);
    ROWS.iter()
        .flat_map(|row| crate::zones::slots_of(player, *row))
        .filter(|slot| equip_host_for(state, player, equip, slot, tributes).is_some())
        .map(|slot| ZoneChoice {
            row: slot.row,
            lane: slot.lane,
        })
        .collect()
}

/// §3.2: "the player picks the zone": every empty, unlocked, unreserved zone of the right row, the
/// occupied ones for a Stack card (§6.2, B5 E21) and, for a Unit, a carrier's zone (R446). `refuse_zone`
/// reads the same predicates, so the client's greyed button and `reduce`'s refusal agree. R391 (B4.5):
/// also every zone the play's Tribute empties, since §10.5 pays at step 2 and places at step 4.
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
    // R1081000: a Flying Unit is also offered the backrow zones it stands in, after its unit-row zones.
    if flies(state, card) {
        for slot in crate::zones::slots_of(player, Row::Backrow) {
            if flying_takes_backrow(state, card, &slot, tributes) && !slots.contains(&slot) {
                slots.push(slot);
            }
        }
    }
    slots
        .into_iter()
        .map(|slot| ZoneChoice {
            row: slot.row,
            lane: slot.lane,
        })
        .collect()
}

/// R653, R23: a carrier that fuses its Unit in (Classic+ #33 Ivory Tower) takes no Unit while Immutable.
/// Read here and in `refuse_zone`, beside `zones::why_cannot_carry`, as the layers compute it (§10.4).
fn immutable_fuser(state: &GameState, slot: &ZoneSlot) -> bool {
    match crate::zones::card_at(state, slot) {
        None => false,
        Some(top) => {
            crate::scripts::script_of(state, top).flags().fuses_carried == Some(true)
                && crate::layers::unit_has(state, top, KeywordKind::Immutable)
        }
    }
}

/// R391 (B4.5): whether the Tribute a play pays empties this zone for it: a pile of exactly one card,
/// that card tributed (a Stack pile's next card would resume, R13) and without Reborn (its zone would be
/// reserved, R64), in a zone neither Locked nor reserved. The rule is the row's, the backrow's included;
/// an Activate cost needs no zone and never comes here (B3.2).
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
    // R1081000: the same reading in the backrow, where a Unit stands on its own — a card beneath it
    // would resume (R13), and a Unit a carrier holds goes with the carrier's zone, not the Tribute's.
    if slot.row == Row::Backrow
        && (!crate::zones::beneath_at(state, *slot).is_empty()
            || crate::zones::carried_at(state, *slot).is_some())
    {
        return false;
    }
    !crate::layers::unit_has(state, held, KeywordKind::Reborn)
}

/// R64, R391: the zone a play that names none takes: the leftmost open zone of its row, else the
/// leftmost its own Tribute empties; R1081000: a Flying Unit then tries the backrow the same way. One
/// function for `refuse_zone`'s no-zone arm and §10.5 step 4's assignment, so the two cannot disagree.
pub fn default_zone_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    tributes: &[String],
) -> Option<ZoneSlot> {
    let row = row_for_card(state, card);
    let own = crate::zones::first_free_zone(state, player, row).or_else(|| {
        crate::zones::slots_of(player, row)
            .into_iter()
            .find(|slot| freed_by_tribute(state, slot, tributes))
    });
    if own.is_some() || !flies(state, card) {
        return own;
    }
    crate::zones::first_free_zone(state, player, Row::Backrow).or_else(|| {
        crate::zones::slots_of(player, Row::Backrow)
            .into_iter()
            .find(|slot| freed_by_tribute(state, slot, tributes))
    })
}

/// §2.3: whether the player chooses X when playing this card: an X-cost card whose X no `cost` hook
/// fixes (#98 Heroic Power's "is fixed by its power", R43, R65, so no X travels in its play).
pub fn chooses_x(state: &GameState, instance: &CardInstance) -> bool {
    // B5 E14, R545: a copier pays its own price and chooses the X of an X-cost text it copies.
    if crate::subsystems::copied_text::copies_text(instance) {
        return crate::subsystems::copied_text::copied_chooses_x(state, instance);
    }
    // R1848003: nor one whose X is set (`fixed_x`): it is played for that X.
    crate::mana::is_x_cost(state, instance)
        && instance.fixed_x.is_none()
        && crate::scripts::script_of(state, instance).cost.is_none()
}

/// §2.3, R348, R1223: why this X is not one the player may choose for this card, or `Ok` when it is —
/// "X is chosen at play time, 1 ≤ X ≤ the mana the player can spend" (`MIN_CHOSEN_X`): their current
/// mana plus what a credit line still lends them (`credit::spendable_mana`). `legal_x_values` offers
/// exactly the values this passes and `refuse_x` refuses exactly the ones it names, so the picker and
/// the reducer's refusal cannot disagree. `most` defaults to the mana the player can spend.
pub fn why_x_refused(
    state: &GameState,
    player: PlayerId,
    value: i32,
    most: Option<i32>,
) -> Result<(), EngineError> {
    // R1223: an X may be chosen out of borrowed mana too.
    let mana = crate::credit::spendable_mana(state, player);
    let most = most.unwrap_or(mana);
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

/// The most X a play of this card may choose: the mana the player can spend for an X-cost card (R1223),
/// and for a copier with an X-cost text what is left of it once its own price is paid (B5 E14, R545).
fn most_x(state: &GameState, player: PlayerId, card: &CardInstance) -> i32 {
    // R1223: an X may be chosen out of borrowed mana too.
    let mana = crate::credit::spendable_mana(state, player);
    if !crate::subsystems::copied_text::copies_text(card) {
        return mana;
    }
    mana - crate::mana::play_cost(state, card)
}

/// §2.3, R348: every X `why_x_refused` passes, lowest first.
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

/// R1563004: a card offers a dearer price when it is an embiggen card or while Mountaintop Garden
/// offers it its Radiant price (`mana::garden_offers_radiant`); the play names it with `embiggen: true`.
pub fn has_dearer_price(state: &GameState, card: &CardInstance) -> bool {
    has_embiggen_price(state, card) || crate::mana::garden_offers_radiant(state, card)
}

/// R1181002: while Mountaintop Garden reaches a Radiant card, it has its normal price only. R1563004: a
/// card of one price that is not Radiant has the Garden's dearer price beside it.
pub fn legal_embiggen_choices(state: &GameState, card: &CardInstance) -> Vec<bool> {
    if !has_dearer_price(state, card) {
        Vec::new()
    } else if card.radiant && crate::mana::garden_reaches(state, card) {
        vec![false]
    } else {
        vec![false, true]
    }
}

// Tribute (§6.3, §3.2)

// #55 Lava Golem "may tribute enemy units", which no other Tribute card may (R101).

/// §6.3 "Tribute X": the number of units playing this card sacrifices, 0 when it asks for none, as a
/// Nerf or a Buff in a hand or a deck left it (R386), less what its owner's Floats took off (R1841003).
pub fn tribute_cost_of(state: &GameState, card: &CardInstance) -> i32 {
    let declared = declared_targets(state, card)
        .into_iter()
        .find(|decl| decl.kind == PromptKind::Tribute)
        .and_then(|decl| decl.amount);
    let flagged = crate::scripts::script_of(state, card).flags();
    let printed = declared.or(flagged.tribute).unwrap_or(0).max(0);
    // R386: a Nerf or a Buff in a hand or a deck moved it (`tuning.x`), never below 1.
    let tuned = crate::tuning::tuned_count(card, crate::numbers::NumberedKey::Tribute.as_str(), printed);
    // R1841003: each of its owner's Floats took one off, never below 0.
    (tuned - crate::float::tribute_lowered(card)).max(0)
}

pub fn may_tribute_enemy_units(state: &GameState, card: &CardInstance) -> bool {
    crate::scripts::script_of(state, card).flags().tribute_enemies == Some(true)
}

/// §3.2: "Sheep Tokens are worth 2 Tributes while on the field"; every other unit is worth 1. It is
/// read off the face that is up (§7's Radiant Sheep is worth 3, `staticFlags.tributeWorth`). A Vanilla
/// Sheep (radiant #61's copy) is worth 1, its text removed (§6.3, R115); a fusion keeps the larger worth
/// (#85, R102); a face-down backrow card taken by a wide Tribute is worth 1, its text asleep (R1487000).
pub fn tribute_value_of(state: &GameState, unit: &CardInstance) -> i32 {
    let flag = if crate::preview::lasting_text_dormant(state, unit) {
        None
    } else {
        crate::scripts::script_of(state, unit).flags().tribute_worth
    };
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

/// Every set of units that pays the Tribute exactly: enough, and minimal, so no unit could be dropped
/// (the Sheep Token's 2 is why a set may overshoot). R90, R1282: every one, with no cut: a set left off
/// is a price the player can never pay, since the client and the §10.7 policy play only what
/// `legal_actions` lists (CLAUDE.md rule 7). The board bounds the count (a wide Tribute: C(20,5) sets).
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

// Declared targets: the options one declaration admits

/// The sides a declaration reaches, the chooser's own first so the offered order is stable. A Tribute
/// defaults to the chooser's side (§6.3 "*your* units"), every other kind to both (§10.6); a `hand`
/// declaration is forced to the chooser by `legal_selections_for` (§9.1).
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
    // MD-B15, R923: a granted tag is part of the card. MD-B16, R924: a face-down backrow card its
    // chooser cannot read skips the tag check, so the picks reveal nothing.
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
    // §10.6, B5: the cost is R65's where the card is now (an X card on the field at its played X).
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

/// §10.6: a filter's named predicate (`TargetFilter.check`), the card's `targetChecks` entry, asked with
/// the candidate (`None` for a hero or a zone); a misspelt name admits nothing, never widening a pick.
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

/// A hero has no type, tags, cost, damage or Plague Counters, so a filter naming any cannot reach one.
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

/// B5 E5, E35, R450: whether a declaration may pick a card on the field beyond its filter: a Spell's
/// never offers one Immune to Spells, a `target` one never one whose targeting cost (Classic #89) the
/// chooser cannot pay from the rest of their hand.
fn reachable(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    decl: &TargetDecl,
    candidate: &CardInstance,
) -> bool {
    // R1839000: a Dormant card is never picked, for a target, a Tribute or anything else.
    if crate::slumber::is_slumbering(candidate) {
        return false;
    }
    if decl.kind == PromptKind::Tribute {
        // R1220: a declared Tribute never offers an Untributable card.
        return !crate::query::is_untributable(state, candidate);
    }
    if crate::restrictions::spell_cannot_reach(state, Some(card), candidate) {
        return false;
    }
    // R781001: a Spell's, Field Spell's or Field Trap's `target` never offers an Immutable card in play.
    if decl.kind == PromptKind::Target && crate::restrictions::immutable_bars(state, Some(card), candidate) {
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
/// R90: a unit pick offers the top of a Stack pile and never a dormant card (R13), and the card being
/// played is never offered. An empty result is not an error: the play stays legal and the effect fizzles.
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
        let key = crate::prompts::option_key(&selection);
        if seen.contains(&key) {
            return;
        }
        seen.insert(key);
        out.push(selection);
    };

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
                        // R1081000: a Unit standing there is no backrow card (§6.1 Animated).
                        if on_field(held) && !crate::zones::is_standing_unit(state, held) {
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

/// B5 E5, R450: each flat `targets` pick's declaration when it is a `target` one, `None` for a Tribute,
/// hand or zone pick (R90): a cost is owed, and an interception answers, only for these. `declared`
/// defaults to the card's own (`None`); an activation passes its ability's (B3.2, R384).
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
/// the play resolves (R214), or an activation's, against its ability's declarations.
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
/// zone and read as it would stand there, would be a legal pick of `decl`; only then does a pick move.
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
        // R781001: nor to an Immutable one, when the source is one Immutable bars.
        && !(decl.kind == PromptKind::Target && crate::restrictions::immutable_bars(state, Some(card), &probe))
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

// R90: reading the flat list, and enumerating it

/// R90: how many selections a declaration takes off the flat list: its own minimum, or all the board
/// offers when that is less, so one the board cannot satisfy takes none and the next still reads its own
/// slot. The last declaration takes the remainder.
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

/// R90, R102: the key the pipeline stores where the play's choices were split under, in the Cry's
/// `data`. A fused card's Cry resolves each ingredient with its slice as §10.5 step 1 read it, not the
/// board at step 5, where the played card may be an option itself (a crafted Postdoc + Sorcerer).
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
/// Step 1 accepts any listing of a set `legal_actions` offers once (`subsets_for`), so step 5 resolves
/// one declaration's picks as a set (`prompts::in_offered_order`); a pick no option names keeps its
/// place after the others. `target_decls` defaults to the card's own (`None`).
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
            .map(|options| options.iter().map(crate::prompts::option_key).collect())
            .unwrap_or_default();
        let rank = |selection: &Selection| -> usize {
            let key = crate::prompts::option_key(selection);
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

/// R123: a `tribute` declaration with an `amount` is one Tribute written in two lists, the units paid
/// (`tributes`, sacrificed at §10.5 step 2) and the picks its script reads (`targets`), so every pick is
/// a unit the play tributes, though it may name fewer (§3.2's Sheep). True too with no such Tribute.
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

/// Every subset of `options` a declaration may answer with, size-ascending then index order, up to
/// R90's bound; none at all for a needed pick the board cannot satisfy, so no play is offered (R703).
/// Also whether the bound dropped any (#992).
fn subsets_for(options: &[Selection], decl: &TargetDecl, is_last: bool) -> (Vec<Vec<Selection>>, bool) {
    // #992: one past R90's bound, so a subset the bound drops is seen.
    const SEEN: usize = MAX_CHOICE_COMBINATIONS + 1;
    if unmet_requirement(decl, options.len()) {
        return (Vec::new(), false);
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
        if out.len() >= SEEN {
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
            if out.len() >= SEEN {
                return;
            }
        }
    }

    let mut size = low;
    while size <= high {
        let mut picked: Vec<Selection> = Vec::new();
        walk(options, size, 0, &mut picked, &mut out);
        if out.len() >= SEEN {
            break;
        }
        size += 1;
    }

    let truncated = out.len() > MAX_CHOICE_COMBINATIONS;
    out.truncate(MAX_CHOICE_COMBINATIONS);
    (out, truncated)
}

/// The cross of several lists, the first varying slowest, bounded by `cap` (R90), and whether the bound
/// dropped any (#992). Over the bound it drops combinations, never a pick: the first kept offer every
/// item of every list, one per place in the longest, then the rest in order, so the client, which plays
/// only what `legal_actions` lists (CLAUDE.md rule 7), can make every pick (a fusion's, R81, R102).
fn cross_product<T: Clone>(lists: &[Vec<T>], cap: usize) -> (Vec<Vec<T>>, bool) {
    if lists.iter().any(Vec::is_empty) {
        return (Vec::new(), false);
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
    (out, total > cap)
}

/// R90: several groups of answers under one bound (the target combinations of each mode choice of a
/// `forModes` card): all when they fit, else a round at a time, one from each group, so every mode
/// choice keeps answers of its own. Also whether the bound dropped any (#992).
fn interleaved<T: Clone>(groups: &[Vec<T>], cap: usize) -> (Vec<T>, bool) {
    let total: usize = groups.iter().map(Vec::len).sum();
    if total <= cap {
        return (groups.iter().flatten().cloned().collect(), false);
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
    (out, true)
}

/// #992: a card's choice combinations as `play_choice_combinations` lists them, and whether R90's
/// bound dropped any.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ListedChoices {
    /// Exactly `play_choice_combinations`' list, in its order.
    pub combos: Vec<PlayChoices>,
    /// The bound dropped at least one legal combination, so `combos` is a slice whose first ones offer
    /// every pick (R90). Never set when `combos` is empty: no combination exists then.
    pub truncated: bool,
}

/// R90: "`legalActions` enumerates every legal combination, bounded by `MAX_CHOICE_COMBINATIONS`".
/// Declarations are read in order, the first varying slowest; a card that declares nothing yields one
/// empty answer, so it is still offered once. `declared` defaults to the card's own (`None`).
pub fn play_choice_combinations(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    declared: Option<&DeclaredChoices>,
) -> Vec<PlayChoices> {
    listed_play_choices(state, player, card, declared).combos
}

/// #992: `play_choice_combinations` and whether R90's bound cut it, seen where the cut is made
/// (`subsets_for`, `cross_product`, `interleaved`), never by counting the combinations a second way.
pub fn listed_play_choices(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    declared: Option<&DeclaredChoices>,
) -> ListedChoices {
    let target_decls: Vec<TargetDecl> = match declared {
        Some(declared) => declared.targets.clone(),
        None => declared_targets(state, card),
    };
    let mode_decls: Vec<ModeDecl> = match declared {
        Some(declared) => declared.modes.clone(),
        None => declared_modes(state, card),
    };
    // R1851000: a play gate that does not hold leaves no combination, so the card is not offered.
    if declared.is_none() && !play_allowed(state, player, card) {
        return ListedChoices {
            combos: Vec::new(),
            truncated: false,
        };
    }
    if target_decls.is_empty() && mode_decls.is_empty() {
        return ListedChoices {
            combos: vec![PlayChoices::default()],
            truncated: false,
        };
    }

    let target_combos_for = |decls: &[TargetDecl]| -> (Vec<Vec<Selection>>, bool) {
        let mut subsets_cut = false;
        let per_decl: Vec<Vec<Vec<Selection>>> = decls
            .iter()
            .enumerate()
            .map(|(index, decl)| {
                let (subsets, cut) = subsets_for(
                    &legal_selections_for(state, player, card, decl),
                    decl,
                    index == decls.len() - 1,
                );
                subsets_cut |= cut;
                subsets
            })
            .collect();
        let (crossed, cross_cut) = cross_product(&per_decl, MAX_CHOICE_COMBINATIONS);
        // No combination at all leaves nothing to cut.
        let truncated = !crossed.is_empty() && (subsets_cut || cross_cut);
        (
            crossed
                .into_iter()
                .map(|slices| slices.into_iter().flatten().collect())
                .collect(),
            truncated,
        )
    };
    // R831008: the card's own modes offer only the options its check allows now; a declaration left
    // with none leaves no combination, so the card is not offered.
    let mode_option_lists: Vec<Vec<String>> = mode_decls
        .iter()
        .map(|decl| {
            decl.options
                .iter()
                .filter(|option| declared.is_some() || mode_option_allowed(state, player, card, option))
                .cloned()
                .collect()
        })
        .collect();
    let (mode_combos, modes_cut) = cross_product(&mode_option_lists, MAX_CHOICE_COMBINATIONS);

    if targets_follow_modes(&target_decls) {
        // `forModes`: each mode choice is enumerated with its own targets, each under the bound (R90).
        let mode_choices: Vec<Vec<String>> = if mode_decls.is_empty() {
            vec![Vec::new()]
        } else {
            mode_combos
        };
        let mut targets_cut = false;
        let groups: Vec<Vec<PlayChoices>> = mode_choices
            .into_iter()
            .map(|modes| {
                let active = active_target_decls(&target_decls, &modes);
                let answers: Vec<Option<Vec<Selection>>> = if active.is_empty() {
                    vec![None]
                } else {
                    let (combos, cut) = target_combos_for(&active);
                    targets_cut |= cut;
                    combos.into_iter().map(Some).collect()
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
        let (combos, interleave_cut) = interleaved(&groups, MAX_CHOICE_COMBINATIONS);
        let truncated = !combos.is_empty() && (modes_cut || targets_cut || interleave_cut);
        return ListedChoices { combos, truncated };
    }

    let (target_answers, targets_cut) = if target_decls.is_empty() {
        (vec![None], false)
    } else {
        let (combos, cut) = target_combos_for(&target_decls);
        (
            combos
                .into_iter()
                .map(Some)
                .collect::<Vec<Option<Vec<Selection>>>>(),
            cut,
        )
    };
    let mode_answers: Vec<Option<Vec<String>>> = if mode_decls.is_empty() {
        vec![None]
    } else {
        mode_combos.into_iter().map(Some).collect()
    };

    // The cross picks by index alone, so it crosses the indices and the answers are read back.
    let index_lists: Vec<Vec<usize>> = vec![
        (0..target_answers.len()).collect(),
        (0..mode_answers.len()).collect(),
    ];
    let (pairs, pairs_cut) = cross_product(&index_lists, MAX_CHOICE_COMBINATIONS);
    let truncated = !pairs.is_empty() && (targets_cut || modes_cut || pairs_cut);
    ListedChoices {
        combos: pairs
            .into_iter()
            .map(|pair| PlayChoices {
                targets: target_answers[pair[0]].clone(),
                modes: mode_answers[pair[1]].clone(),
            })
            .collect(),
        truncated,
    }
}

/// Every `play` action this card could legally produce: R81's five choice kinds crossed, skipping the
/// prices the player cannot pay. This is what `legal_actions` lists for a card in hand.
pub fn play_actions_for(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<PlayAction> {
    // R2533000: a seat that plays only the tutorial's cards lists no play of any other card.
    if crate::state::barred_from_play(state, player, card) {
        return Vec::new();
    }
    // R1223: a card affordable on borrowed mana is listed.
    let mana = crate::credit::spendable_mana(state, player);
    // R1180004: a Spell its player's health price reaches takes no mana, so every price of it is
    // listed, the ones only the health affords included. A face-down play is a Trap's, never priced so.
    let health = crate::mana::health_priced(state, card);
    let mut out = priced_play_actions(state, player, card, |cost| {
        if health || cost <= mana {
            vec![PlayPayment::default()]
        } else {
            Vec::new()
        }
    });
    out.extend(face_down_play_actions(state, player, card, |cost| {
        if cost <= mana {
            vec![PlayPayment::default()]
        } else {
            Vec::new()
        }
    }));
    out
}

/// #992, R90, §10.8: whether `legal_actions` lists plays of this card in hand whose choice combinations
/// R90's bound cut (the view's `choicesTruncated`): only where it lists plays at all (§10.2, R211, R265),
/// at a payable price whose face's combinations were cut and at which a face-up play is listed.
pub fn hand_play_choices_truncated(state: &GameState, player: PlayerId, card: &CardInstance) -> bool {
    if state.result.is_some()
        || state.pending.is_some()
        || state.mulligan.is_some()
        || state.active != player
        || state.phase != Phase::Main
    {
        return false;
    }
    let mana = crate::credit::spendable_mana(state, player);
    // R1180004: a price the health price pays is one the player can pay.
    let health = crate::mana::health_priced(state, card);
    let cut_prices: Vec<PlayPrice> = play_prices(state, player, card)
        .into_iter()
        .filter(|price| {
            (health || price.cost <= mana)
                && listed_play_choices(
                    state,
                    player,
                    &resolving_face(
                        state,
                        player,
                        &stamped_for_play(state, card, price.x, price.embiggen),
                        price.cost,
                    ),
                    None,
                )
                .truncated
        })
        .collect();
    // A face-down play carries no choices (R783004); a price with no zone or Tribute set to play at
    // lists no play, so what it cut is not offered.
    !cut_prices.is_empty()
        && play_actions_for(state, player, card).iter().any(|play| {
            play.face_down.is_none()
                && cut_prices
                    .iter()
                    .any(|price| price.x == play.x && price.embiggen == play.embiggen)
        })
}

/// R783004: every face-down `play` a Cloaked card in hand could legally produce — one per price,
/// payment, tribute set and open backrow zone. No targets or modes are declared when set.
fn face_down_play_actions(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    payments: impl Fn(i32) -> Vec<PlayPayment>,
) -> Vec<PlayAction> {
    let mut out: Vec<PlayAction> = Vec::new();
    for (_, reveal) in crate::alt_play::face_down_options(state, card, false) {
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
            // R783004: a Cloaked card pays its own price, a Tribute included.
            for tributes in &legal_tribute_sets(state, player, &probe) {
                for zone in crate::alt_play::open_backrow_zones(state, player) {
                    for payment in &paid {
                        out.push(PlayAction {
                            instance_id: card.id.clone(),
                            zone: Some(zone),
                            x: price.x,
                            embiggen: price.embiggen,
                            equip: None,
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

/// E11, R454: every `play` action `legal_actions` lists for a card in the player's graveyard, crossed as
/// for a hand card, with the ways a permission lets it be paid (`graveyard_play::graveyard_payments_for`).
pub fn graveyard_play_actions_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
) -> Vec<PlayAction> {
    if !crate::graveyard_play::playable_from_graveyard(state, card)
        || card.zone.player() != player
        // R2533000: nor from the graveyard.
        || crate::state::barred_from_play(state, player, card)
    {
        return Vec::new();
    }
    priced_play_actions(state, player, card, |price| {
        crate::graveyard_play::graveyard_payments_for(state, player, card, price)
    })
}

/// One price a card's X and embiggen choices come to now (R65).
struct PlayPrice {
    x: Option<i32>,
    embiggen: Option<bool>,
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
    let embiggens: Vec<Option<bool>> = if has_dearer_price(state, card) {
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

/// R979000, §10.8: why a card in `player`'s hand cannot be played now, the first refusal in a fixed
/// order, as `UnitView.attack_reason` (R849000) is for an attack. Each branch reads what `legal_actions`
/// and §10.5 step 1 call, so the reason and the listing cannot disagree (`reduce.rs`'s header); `None`
/// when a play is listed or no refusal fires. The order is ban (R455), price, zone, Tribute, choices: a
/// card refused at every price is refused whatever the board holds, then step 1's own order.
pub fn why_hand_card_unplayable(state: &GameState, player: PlayerId, card: &CardInstance) -> Option<String> {
    // R1223: borrowed mana pays; R1180004: a Spell its player's health price reaches is refused by no mana.
    let mana = crate::credit::spendable_mana(state, player);
    let health = crate::mana::health_priced(state, card);
    why_unplayable(
        state,
        player,
        card,
        |cost| health || cost <= mana,
        |cost| {
            crate::credit::why_unaffordable(state, card, cost)
                .err()
                .map(|error| error.message)
        },
    )
}

/// R979000, R1571000: `why_hand_card_unplayable`'s order for any play source: `payable` says whether
/// a payment meets a price, as the source's listing asks it; `refuse_price` refuses a price none meets.
fn why_unplayable(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    payable: impl Fn(i32) -> bool,
    refuse_price: impl Fn(i32) -> Option<String>,
) -> Option<String> {
    // (a) Ban: every X and embiggen combination `play_prices` enumerates, stamped alike, is banned (R455).
    let x_values: Vec<Option<i32>> = if chooses_x(state, card) {
        legal_x_values(state, player, card)
            .into_iter()
            .map(Some)
            .collect()
    } else {
        vec![None]
    };
    let embiggens: Vec<Option<bool>> = if has_dearer_price(state, card) {
        legal_embiggen_choices(state, card)
            .into_iter()
            .map(Some)
            .collect()
    } else {
        vec![None]
    };
    let mut banned = 0;
    let mut combos = 0;
    let mut ban_message: Option<String> = None;
    for x in &x_values {
        for embiggen in &embiggens {
            combos += 1;
            let probe = CardInstance {
                x: x.or(card.x),
                embiggened: embiggen.or(card.embiggened),
                ..card.clone()
            };
            let cost = crate::mana::play_cost(state, &probe);
            if let Err(error) = crate::cost_rules::why_play_banned(state, player, &probe, cost) {
                banned += 1;
                ban_message.get_or_insert(error.message);
            }
        }
    }
    if combos > 0 && banned == combos {
        return ban_message;
    }
    // (b) Price: `play_prices` left nothing unbanned. An X card with no legal X names its smallest
    // refusal (R348); otherwise, when no price can be paid, the cheapest price's refusal (`refuse_price`).
    let prices = play_prices(state, player, card);
    if prices.is_empty() {
        if chooses_x(state, card) && legal_x_values(state, player, card).is_empty() {
            return why_x_refused(state, player, MIN_CHOSEN_X, None)
                .err()
                .map(|error| error.message);
        }
        return None;
    }
    let Some(cheapest) = prices
        .iter()
        .filter(|price| payable(price.cost))
        .min_by_key(|price| price.cost)
    else {
        let cheapest = prices.iter().min_by_key(|price| price.cost)?;
        return refuse_price(cheapest.cost);
    };
    let tribute_sets = legal_tribute_sets(state, player, card);
    // (c) Zone: no set the Tribute could pay leaves a zone: an open one, one the set empties (R391) or a
    // host an Equip play would fuse into (R1086, read off this price's probe, as the listing reads it).
    if needs_zone(state, card) {
        let sets: Vec<&[String]> = if tribute_sets.is_empty() {
            vec![&[]]
        } else {
            tribute_sets.iter().map(Vec::as_slice).collect()
        };
        let nowhere = sets.iter().all(|tributes| {
            default_zone_for(state, player, card, tributes).is_none()
                && legal_zones_for(state, player, card, tributes).is_empty()
                && legal_equip_zones_for(state, player, &cheapest.probe, tributes).is_empty()
        });
        if nowhere {
            return refuse_zone(state, player, card, None, &[])
                .err()
                .map(|error| error.message);
        }
    }
    // (d) Tribute: the board cannot pay what the play costs (§6.3).
    if tribute_cost_of(state, card) > 0 && tribute_sets.is_empty() {
        return refuse_tributes(state, player, card, None)
            .err()
            .map(|error| error.message);
    }
    // (e) Choices: the resolving face at the cheapest price declares what the board cannot answer (R703,
    // R831008, R1851000). The probe is the listing's default (this price, its zone, its first Tribute
    // set, no picks), so the refusal is the one §10.5 step 1 gives that play.
    let face = resolving_face(
        state,
        player,
        &stamped_for_play(state, card, cheapest.x, cheapest.embiggen),
        cheapest.cost,
    );
    if play_choice_combinations(state, player, &face, None).is_empty() {
        let paid: &[String] = tribute_sets.first().map(Vec::as_slice).unwrap_or(&[]);
        let probe = PlayAction {
            instance_id: card.id.clone(),
            zone: if needs_zone(state, card) {
                default_zone_for(state, player, card, paid).map(|slot| ZoneChoice {
                    row: slot.row,
                    lane: slot.lane,
                })
            } else {
                None
            },
            x: cheapest.x,
            embiggen: cheapest.embiggen,
            equip: None,
            tributes: if paid.is_empty() {
                None
            } else {
                Some(paid.to_vec())
            },
            targets: None,
            modes: None,
            plague: None,
            face_down: None,
        };
        return why_choices_refused(state, player, card, &probe)
            .err()
            .map(|error| error.message);
    }
    None
}

/// R1571000, §10.8: R979000's order and words for a card in the player's own graveyard that a
/// permission admits, the price asked as `graveyard_play_actions_for` asks it; `None` for any other.
pub fn why_graveyard_card_unplayable(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
) -> Option<String> {
    if !crate::graveyard_play::playable_from_graveyard(state, card) || card.zone.player() != player {
        return None;
    }
    why_unplayable(
        state,
        player,
        card,
        |cost| !crate::graveyard_play::graveyard_payments_for(state, player, card, cost).is_empty(),
        |cost| crate::graveyard_play::why_graveyard_price_unpaid(state, player, card, cost),
    )
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
        &stamped_for_play(state, card, action.x, action.embiggen),
        cost_with(state, card, action.x, action.embiggen),
    )
}

/// R1200 (ME-RANDOMTARGETS): a play's picks with every `target` declaration's slice left out, what
/// `legal_actions` offers while a Mayor acts. The slices are `split_selections`' own, so the draw below
/// inverts them exactly; an empty remainder is `None`.
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
/// declaration's picks at random (`random_targets::draw_picks`) with `subsets_for`'s own bounds, so the
/// list splits again exactly as `legal_actions` split it; any other declaration takes its slice of the
/// given picks. Picks left over refuse the play: a named target is no play while a Mayor acts.
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

/// R81, R90's enumeration with the payment left to the caller: for each price, `payments` answers the
/// ways a play may pay it (none: not offered; `{}`: mana alone), a hand card in mana, a graveyard one
/// perhaps with Plague Counters (`graveyard_play.rs`). R455: a banned price is never offered. R391: each
/// Tribute set is paired with the zones it leaves open, never with a zone another set empties.
pub fn priced_play_actions(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    payments: impl Fn(i32) -> Vec<PlayPayment>,
) -> Vec<PlayAction> {
    let mut out: Vec<PlayAction> = Vec::new();
    let tribute_sets = legal_tribute_sets(state, player, card);
    // R1200: read once, at the first action listed: no Tribute set, zone or choice changes it.
    let mut random: Option<bool> = None;

    for PlayPrice {
        x,
        embiggen,
        probe,
        cost,
    } in play_prices(state, player, card)
    {
        let paid = payments(cost);
        if paid.is_empty() {
            continue;
        }
        // R214: the choices of the face step 5 will resolve, which this price decides (#64, R1181002).
        let face = resolving_face(state, player, &stamped_for_play(state, card, x, embiggen), cost);
        let bound = declares_bound_tribute(state, &face);
        // #1889: the price's payable choices, listed once the price offers a zone: neither reads the
        // Tribute set or the zone, so they are listed once, not once per pairing.
        let mut payable: Option<Vec<PlayChoices>> = None;
        for tributes in &tribute_sets {
            // R1086: the plain zones first, then the Equip host zones after them (D14: a game that
            // never plays Equip lists what it listed before).
            let zones: Vec<(Option<ZoneChoice>, Option<bool>)> = if needs_zone(state, card) {
                let mut paired: Vec<(Option<ZoneChoice>, Option<bool>)> =
                    legal_zones_for(state, player, card, tributes)
                        .into_iter()
                        .map(|zone| (Some(zone), None))
                        .collect();
                // R831013: the probe carries this price's embiggen, which an Equip may hang on
                // (Core #56 Jilliax: "Paid (3): Equip.").
                paired.extend(
                    legal_equip_zones_for(state, player, &probe, tributes)
                        .into_iter()
                        .map(|zone| (Some(zone), Some(true))),
                );
                paired
            } else {
                vec![(None, None)]
            };
            for (zone, equip) in &zones {
                let listed = payable.get_or_insert_with(|| payable_play_choices(state, player, card, &face));
                for choices in listed.iter() {
                    let targets: &[Selection] = choices.targets.as_deref().unwrap_or(&[]);
                    let modes: &[String] = choices.modes.as_deref().unwrap_or(&[]);
                    // R123: the declared Tribute's pick names the units this play tributes, and no others.
                    if bound && !tribute_picks_agree(state, player, &face, targets, modes, tributes) {
                        continue;
                    }
                    for payment in &paid {
                        let mut pushed = PlayAction {
                            instance_id: card.id.clone(),
                            zone: *zone,
                            x,
                            embiggen,
                            equip: *equip,
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
                        // R1200: while a Mayor acts the reducer draws the targets, so collapsed
                        // choices list one action, not one per target set.
                        if *random.get_or_insert_with(|| crate::random_targets::targets_random(state)) {
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

/// B5 E5, R450, R682: `face`'s choice combinations (R90) whose targets' discard cost can be paid; the
/// discards are random at pay time, so no paying sets are listed. `card`, the card played, pays none.
fn payable_play_choices(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    face: &CardInstance,
) -> Vec<PlayChoices> {
    play_choice_combinations(state, player, face, None)
        .into_iter()
        .filter(|choices| {
            let targets: &[Selection] = choices.targets.as_deref().unwrap_or(&[]);
            let modes: &[String] = choices.modes.as_deref().unwrap_or(&[]);
            let owed = targeting_discards_required(state, player, face, targets, modes, None);
            crate::targeting::why_targeting_discards_unpayable(state, player, owed, &play_uses(card, targets))
                .is_ok()
        })
        .collect()
}

// §10.5 step 1: the refusal

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

/// R1086: the refusal for an Equip play (`equip: false` is a plain play): the card must play Equip or
/// be a Unit card a host may lend it to (R1869000), and the zone must hold a host it may go onto, read
/// off `equip_onto_at` as the listing reads it.
fn refuse_equip(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    zone: Option<&ZoneChoice>,
    tributes: &[String],
) -> Result<(), EngineError> {
    let name = name_of(state, &card.def_id);
    if !plays_as_unit(state, card) {
        return refuse(format!("{name} is not Equip"));
    }
    let Some(zone) = zone else {
        return refuse(format!("{name} played as Equip takes a zone"));
    };
    if zone.lane < 1 || zone.lane > crate::zones::row_size(zone.row) {
        return refuse(format!("there is no {} zone {}", zone.row, zone.lane));
    }
    let slot = crate::zones::ZoneSlot {
        player,
        row: zone.row,
        lane: zone.lane,
    };
    if equip_host_at(state, player, &slot, tributes).is_none() {
        return refuse(format!("that {} zone holds no Unit for {name}", zone.row));
    }
    if equip_onto_at(state, player, card, &slot, tributes).is_none() {
        return refuse(format!("{name} is not Equip"));
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
        if zone.lane < 1 || zone.lane > crate::zones::row_size(zone.row) {
            return refuse(format!("there is no {} zone {}", zone.row, zone.lane));
        }
        let slot = ZoneSlot {
            player,
            row: zone.row,
            lane: zone.lane,
        };
        // R1081000: a Flying Unit may stand in a backrow zone it takes, as `legal_zones_for` lists
        // them; a carrier takes it only as its Unit, on the R446 path below (R653).
        if flies(state, card) {
            if flying_takes_backrow(state, card, &slot, tributes) {
                return Ok(());
            }
            if !holds_carrier(state, &slot) {
                return refuse(format!("that {} zone is not open", zone.row));
            }
        }
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

    // §6.2 Stack: only occupancy is waived; `accepts_stack` still refuses a Locked or reserved zone (R64).
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
    // R1848003: a set X is the play's; the action may name it or leave it out, and names no other.
    if let Some(fixed) = card.fixed_x {
        return match x {
            Some(other) if other != fixed => refuse(format!("{name}'s X is set at {fixed}")),
            _ => Ok(()),
        };
    }
    // R43: an X the card's own cost hook fixes (#98) is not the player's to choose, so whatever the
    // action names is ignored — never recorded on the card, never read by the cost (`play_steps`).
    if !chooses_x(state, card) {
        return Ok(());
    }
    // R348: an X the play leaves out reads as 0, which is not one to choose.
    why_x_refused(state, player, x.unwrap_or(0), None)
}

fn refuse_embiggen(
    state: &GameState,
    card: &CardInstance,
    embiggen: Option<bool>,
) -> Result<(), EngineError> {
    if embiggen != Some(true) || legal_embiggen_choices(state, card).contains(&true) {
        return Ok(());
    }
    if has_embiggen_price(state, card) {
        // R1181002: a Radiant card under Mountaintop Garden.
        return refuse(format!(
            "{} has only its normal price now",
            name_of(state, &card.def_id)
        ));
    }
    refuse(format!("{} has no embiggen price", name_of(state, &card.def_id)))
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

        let keys: IndexSet<String> = options.iter().map(crate::prompts::option_key).collect();
        let mut used: IndexSet<String> = IndexSet::new();
        for selection in got {
            let key = crate::prompts::option_key(selection);
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
    player: PlayerId,
    card: &CardInstance,
    modes: &[String],
    decls: Option<&[ModeDecl]>,
) -> Result<(), EngineError> {
    let own_modes = decls.is_none();
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
        // R831008: the same check `legalActions` offers by.
        if own_modes && !mode_option_allowed(state, player, card, option) {
            return refuse(format!("{name} cannot choose \"{option}\" now"));
        }
    }
    Ok(())
}

/// R783004: refusal for a face-down play: a card with no Cloaked (or in a graveyard), a timing other
/// than Cloaked's (not R1044's three), a named zone outside the open backrow zones, or any target or
/// mode. X, embiggen and tributes are still checked.
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
    let options = crate::alt_play::face_down_options(state, card, from_graveyard);
    if options.is_empty() {
        return refuse(format!(
            "{} cannot be played face-down",
            name_of(state, &card.def_id)
        ));
    }
    if !options.iter().any(|(_, timing)| *timing == reveal) {
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
    if action.equip == Some(true) {
        return refuse(format!(
            "{} is not played Equip face-down",
            name_of(state, &card.def_id)
        ));
    }
    refuse_x(state, player, card, action.x)?;
    // R1181002: a card set face-down is no play Mountaintop Garden reaches, so its prices are read off
    // the card stamped set, as the listing reads them (`face_down_play_actions`).
    let set = CardInstance {
        set_as: Some(crate::state::SetAs {
            reveal,
            set_turn: state.turn,
            echo: None,
            revealing: None,
        }),
        ..card.clone()
    };
    refuse_embiggen(state, &set, action.embiggen)?;
    refuse_tributes(state, player, card, action.tributes.as_deref())?;
    // B5 E5 discards still apply to a face-down play.
    let face = resolving_face(
        state,
        player,
        card,
        cost_with(state, &set, action.x, action.embiggen),
    );
    let required = targeting_discards_required(state, player, &face, &[], &[], None);
    crate::targeting::why_targeting_discards_unpayable(state, player, required, &play_uses(card, &[]))
}

/// §10.5 step 1, R90: every choice the play carried, checked against what the card declared and what
/// the board allows; `reduce` returns the refusal and leaves the state untouched (§9.3). Mana
/// affordability is not a choice and stays with `reduce`'s pay step (§10.5 step 2).
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
    // R831013: an Equip a price grants is read off the card stamped with the play's embiggen, as the
    // listing reads it off that price's probe (Core #56 Jilliax); R1181002: so is the face.
    let stamped = stamped_with_embiggen(state, card, action.embiggen);
    if action.equip == Some(true) {
        refuse_equip(state, player, &stamped, action.zone.as_ref(), tributes)?;
    } else {
        refuse_zone(state, player, card, action.zone.as_ref(), tributes)?;
    }
    refuse_x(state, player, card, action.x)?;
    refuse_embiggen(state, card, action.embiggen)?;
    refuse_tributes(state, player, card, action.tributes.as_deref())?;
    // R214: the targets and modes are the resolving face's, so they are read against it. The Tribute
    // above is paid at step 2, before step 3 can change the face, so it is the face in hand's.
    // R1868000: the face carries the play's X, which a declaration's named check may read.
    let face = resolving_face(
        state,
        player,
        &stamped_for_play(state, card, action.x, action.embiggen),
        cost_with(state, card, action.x, action.embiggen),
    );
    // R1851000: a play gate that does not hold refuses the play whatever it carried.
    if !play_allowed(state, player, &face) {
        return refuse(format!(
            "{} cannot be played without something to act on",
            name_of(state, &face.def_id)
        ));
    }
    let targets: &[Selection] = action.targets.as_deref().unwrap_or(&[]);
    let modes: &[String] = action.modes.as_deref().unwrap_or(&[]);
    refuse_targets(state, player, &face, targets, modes, None)?;
    refuse_modes(state, player, &face, modes, None)?;
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

// Activate and turn (B3.2 activation choices, R384)

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
    refuse_modes(state, player, card, modes, Some(&declared.modes))
}
