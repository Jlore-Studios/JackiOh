//! Who may be targeted, and what targeting costs (docs/classic-sets.md B5 E5, E35; R450): the pure
//! reads the declarations (`play_choices::legal_selections_for`), the target prompts
//! (`targeting_point.rs`) and the attack's interception (§4.2 step 2) all ask, so a picker's greyed-out
//! option and the reducer's refusal are one rule (§10.2).
//!
//! "Targeting" is choosing (R450): a play's, a cast's or an activation's declared target of kind
//! `target`, and a `target` prompt's answer. Random picks and "all" effects target nothing, and neither
//! does a Tribute, a hand pick or a zone pick. What it reaches is a card acting on the field — the
//! text that makes a card harder to target (Classic #89) or answers its targeting (Classic #33, from a
//! hand) is read from there.
//!
//! A card's script is looked up through the state (`scripts::script_of` composes a fused card's on
//! lookup, SURFACE §6.6), so `interposes_from_hand` and the source check (R651) take the state too
//! (SURFACE §4).

use crate::script::{HookArgs, ReplacementMoment, ReplacementWhere, Script, TargetedReplacement};
use crate::state::{CardInstance, EngineError, GameState, find_instance};
use crate::wire::{CardType, PlayerId, Row, Selection, TargetAim, TargetDecl, Zone, opponent_of};

/// One face's own targeting cost, read with the card as "this" (a pure read, §10.9).
fn face_cost(script: &Script, state: &GameState, self_: &CardInstance) -> i32 {
    let Some(hook) = &script.targeting_discards else {
        return 0;
    };
    let count = hook(HookArgs {
        state,
        self_,
        radiant: self_.radiant,
    });
    count.max(0)
}

/// R102: a fused card carries every ingredient's text, so the stricter of their costs holds.
fn fused_cost(state: &GameState, self_: &CardInstance, def_id: &str) -> i32 {
    let Some(parts) = crate::catalog::fused_id_parts(Some(state), def_id) else {
        let entry = crate::scripts::scripts_ref(state, def_id);
        return face_cost(
            if self_.radiant {
                &entry.radiant
            } else {
                &entry.base
            },
            state,
            self_,
        );
    };
    parts
        .iter()
        .map(|part| fused_cost(state, self_, part))
        .fold(0, i32::max)
}

/// B5 E5, Classic #89: how many cards a player must also discard to target this card now — its
/// `targetingDiscards`, while it acts on the field. A Vanilla card has no text (§6.3, R115).
pub fn targeting_discards_of(state: &GameState, card: &CardInstance) -> i32 {
    if card.vanilla || !crate::zones::acts_on_field(state, card) {
        return 0;
    }
    if crate::catalog::fused_id_parts(Some(state), &card.def_id).is_some() {
        return fused_cost(state, card, &card.def_id);
    }
    face_cost(&crate::scripts::script_of(state, card), state, card)
}

/// R450: whether `chooser` could pay to target `card`: at least its cost in OTHER cards in their hand —
/// `leaving` is the card a play is taking out of that hand, which cannot pay for its own target. It
/// binds both players, the card's own controller included.
pub fn can_pay_to_target(
    state: &GameState,
    chooser: PlayerId,
    card: &CardInstance,
    leaving: Option<&str>,
) -> bool {
    let cost = targeting_discards_of(state, card);
    if cost == 0 {
        return true;
    }
    let others = state.players[chooser]
        .hand
        .iter()
        .filter(|held| Some(held.id.as_str()) != leaving)
        .count() as i32;
    others >= cost
}

/// R450, E35: whether `source`'s targeting may pick `candidate` at all — a Spell never picks a card
/// Immune to Spells, and nobody picks a card whose targeting cost they cannot pay.
pub fn may_target(
    state: &GameState,
    chooser: PlayerId,
    source: Option<&CardInstance>,
    candidate: &CardInstance,
    leaving: Option<&str>,
) -> bool {
    if crate::restrictions::spell_cannot_reach(state, source, candidate) {
        return false;
    }
    can_pay_to_target(state, chooser, candidate, leaving)
}

/// The card's "targeted" replacements that answer from a hand (`{ on: "targeted", where: "hand" }`).
fn hand_interpositions(state: &GameState, card: &CardInstance) -> Vec<TargetedReplacement> {
    crate::scripts::script_of(state, card)
        .replacements
        .iter()
        .filter(|entry| {
            entry.on == ReplacementMoment::Targeted && entry.where_ == Some(ReplacementWhere::Hand)
        })
        .cloned()
        .collect()
}

/// Classic #33 Joro, R450: whether a card answers the targeting of a friendly unit from its owner's
/// hand — its script declares the replacement `{ on: "targeted", where: "hand" }` (`Script.replacements`),
/// the one declaration the attack half (§4.2 step 2) reads too. A Vanilla card has no text.
pub fn interposes_from_hand(state: &GameState, card: &CardInstance) -> bool {
    !hand_interpositions(state, card).is_empty()
}

/// R651: whether the card's "targeted" replacement answers a targeting from this source — a `by:
/// "spell"` replacement (Classic #33 Joro) answers only a Spell's targeting.
fn answers_source(state: &GameState, card: &CardInstance, source: Option<CardType>) -> bool {
    hand_interpositions(state, card)
        .iter()
        .any(|def| def.by.is_none() || source == Some(CardType::Spell))
}

/// Classic #33 Joro, R450: the card that answers `chooser` targeting `targeted` — the first card in
/// the targeted unit's controller's hand that `interposes_from_hand` and answers this source (R651),
/// when the targeted card is a unit of the chooser's opponent acting on the field and that player
/// has an open unit zone to summon it into (with none, nothing happens). Null when nothing answers.
pub fn interceptor_for<'a>(
    state: &'a GameState,
    chooser: PlayerId,
    targeted: &CardInstance,
    source: Option<CardType>,
) -> Option<&'a CardInstance> {
    if !matches!(targeted.zone, Zone::Field { row: Row::Units, .. })
        || crate::zones::is_buried(state, targeted)
    {
        return None;
    }
    let defender = targeted.controller;
    if defender != opponent_of(chooser) {
        return None;
    }
    crate::zones::first_entry_zone(state, defender, Row::Units)?;
    state.players[defender]
        .hand
        .iter()
        .find(|card| interposes_from_hand(state, card) && answers_source(state, card, source))
}

/// R450: how many cards `picks` cost their chooser to target — each targeting pick naming a card with a
/// targeting cost adds that card's count (a card named twice is targeted twice).
pub fn targeting_discards_for(
    state: &GameState,
    picks: &[Selection],
    targeting: Option<&dyn Fn(usize) -> bool>,
) -> i32 {
    let mut total = 0;
    for (index, pick) in picks.iter().enumerate() {
        if let Some(targeting) = targeting
            && !targeting(index)
        {
            continue;
        }
        let Selection::Instance { instance_id } = pick else {
            continue;
        };
        if let Some(card) = find_instance(state, instance_id) {
            total += targeting_discards_of(state, card);
        }
    }
    total
}

/// R450, R682: whether the player can pay `required` targeting discards: that many cards held outside
/// `keep` (the card a play is taking out of that hand, §10.5 step 1, and any hand card the same play
/// picks); `Err` is the refusal text (SURFACE §4.4.9). The discards are random at pay time (R682: a discard is its
/// player's choice only when the card says "of your choice"), so no paying sets are enumerated.
pub fn why_targeting_discards_unpayable(
    state: &GameState,
    player: PlayerId,
    required: i32,
    keep: &[String],
) -> Result<(), EngineError> {
    if required <= 0 {
        return Ok(());
    }
    let held = state.players[player]
        .hand
        .iter()
        .filter(|card| !keep.contains(&card.id))
        .count() as i32;
    if held >= required {
        return Ok(());
    }
    Err(EngineError::new(format!(
        "targeting that costs {required} discard{}, and you hold {held} card{}",
        if required == 1 { "" } else { "s" },
        if held == 1 { "" } else { "s" },
    )))
}

/// R656: whether a target declaration aims to help ("help") or harm ("harm").
/// Defaults to "harm".
pub fn target_aim(decl: &TargetDecl) -> TargetAim {
    decl.aim.unwrap_or(TargetAim::Harm)
}
