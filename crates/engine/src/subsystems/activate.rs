//! Activate (docs/classic-sets.md B3.2, R384): a card's "Activate:", "Activate N:" and "Activate ♾️:"
//! abilities, used by the `activate` action while the card acts on its controller's side of the field.
//!
//! The rules, in B3.2's order, and where each one lives here:
//!   1. "Activate" is once per turn, "Activate N" N times, "Activate ♾️" any number of times, bounded
//!      by `ACTIVATE_UNLIMITED_CAP` (rule 7). Degrade and Upgrade move N by the tuning key "Activate"
//!      (`tuning::tuned_count`, rule 9), never ♾️. `uses_allowed`.
//!   2. Who and when: the card's controller, in their own main phase, with no prompt open and the game
//!      not over, while the card acts on the field — the top of its pile, or a face-up backrow card
//!      (`is_acting_on_field`). Summoning sickness and exertion do not apply: activating is not attacking.
//!   3. Uses are counted per card per turn on the instance (`memory.activations = { turn, count }`,
//!      SPEC §6.2), which R78's reset clears when the card leaves the field, so a card bounced and
//!      played again, or a copy, starts fresh. A card with several abilities (a fusion's) counts every
//!      use of any of them, and each ability allows as many as its own number says.
//!   4. A cost is paid as the ability is activated — mana, a random discard, a Tribute of the
//!      controller's units (the card itself allowed) or the card itself — and an ability whose cost
//!      cannot be paid cannot be activated (`why_cannot_activate_ability`).
//!   5. The targets and modes the ability declares travel in the action, as a play's do (R81), checked
//!      by `play_choices` against the ability's declarations (R90); the discards a declared target
//!      costs (B5 E5, R450, Classic #89) are random at pay time (R682) and travel nowhere. Choices
//!      made during resolution are ordinary prompts, which the card's `resume` table answers.
//!   6. Not a play: nothing that counts plays sees it (no turn log, no `counters.played`, no
//!      `cardPlayed`). What the effect plays or casts counts as usual (R70).
//!   8. `legal_actions` lists `activate` from the refusal below, which is also the list (§10.2's
//!      pattern), so a greyed-out control and a refused action give one reason.
//!  10. Heroic Power (Core #98) is on this module since the Heroic Power patch (R752): each of its
//!      powers is an ability with a mana price and, for Ping, a declared target, and
//!      `ActivationDecl.has` keeps the one power a copy rolled. `activatePower` stays the alias of
//!      `activate` that names it (`reduce.rs`).
//!
//! The sequence is resumable like every other that can ask (§9.3, R113): paying a Tribute runs the
//! tributed units' Death hooks, which can ask, and then the effect is owed on `state.work`
//! (`ACTIVATION_WORK`) as plain data; the effect's own list runs through `prompts::run_hook_resumable`
//! under the hook `activation:<id>`, so a tail its prompt parks comes back to the same ability
//! (`work::script_step_for`).
//!
//! Port of `packages/engine/src/subsystems/activate.ts`. TS registered `run_owed_activation` as the
//! `"@activate"` work handler; `work.rs`'s dispatcher calls it directly (SURFACE §6.6), so it is `pub`.
//! TS's `string | null` refusals are `Result<(), EngineError>` with TS's text (SURFACE §4.4.9). TS
//! wrote the use count through the live card; here it is written on the card in the state, by id.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::ACTIVATE_UNLIMITED_CAP;
use crate::effects::move_::discard_random_cards;
use crate::layers::unit_view;
use crate::mana::mana_event;
use crate::play_choices::{
    DeclaredChoices, in_declared_order, play_choice_combinations, targeting_discards_required,
    why_declared_choices_refused,
};
use crate::preview::is_face_down;
use crate::prompts::{HookInstance, HookResumableOptions, run_hook_resumable};
use crate::script::{
    ActivationDecl, ActivationUses, ConditionContext, ConditionZone, EffectContext, EngineSink, HookArgs,
    activation_decls, activation_hook,
};
use crate::state::{
    CardInstance, EngineError, GameState, Resume, WorkItem, find_instance, find_instance_mut,
};
use crate::state_check::{sacrifice_together, state_check};
use crate::stays::exit_mark;
use crate::targeting::why_targeting_discards_unpayable;
use crate::targeting_point::pay_targeting_discards;
use crate::tuning::tuned_count;
use crate::wire::{ActionBody, ActivationView, GameEvent, Phase, PlayerId, Row, Selection, ZoneName};
use crate::work::{paused, push_work};
use crate::zones::{active_units_of, acts_on_field, slot_of};

/// The `activate` member of the action union, without the `playerId` and `nonce` the caller adds (TS
/// `Extract<ActionBody, { type: "activate" }>`): `ActionBody::Activate`'s fields as a struct, whose JSON
/// is the action's own (`"type": "activate"` included).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(into = "ActionBody", try_from = "ActionBody")]
pub struct ActivateAction {
    pub instance_id: String,
    pub ability: Option<String>,
    pub targets: Option<Vec<Selection>>,
    pub modes: Option<Vec<String>>,
    pub tributes: Option<Vec<String>>,
}

impl ActivateAction {
    /// The action body this is (`{ type: "activate", … }`).
    pub fn into_body(self) -> ActionBody {
        ActionBody::Activate {
            instance_id: self.instance_id,
            ability: self.ability,
            targets: self.targets,
            modes: self.modes,
            tributes: self.tributes,
        }
    }

    /// The `activate` an action body is, or `None` for any other action.
    pub fn from_body(body: &ActionBody) -> Option<ActivateAction> {
        match body {
            ActionBody::Activate {
                instance_id,
                ability,
                targets,
                modes,
                tributes,
            } => Some(ActivateAction {
                instance_id: instance_id.clone(),
                ability: ability.clone(),
                targets: targets.clone(),
                modes: modes.clone(),
                tributes: tributes.clone(),
            }),
            _ => None,
        }
    }
}

impl From<ActivateAction> for ActionBody {
    fn from(action: ActivateAction) -> ActionBody {
        action.into_body()
    }
}

impl TryFrom<ActionBody> for ActivateAction {
    type Error = String;

    fn try_from(body: ActionBody) -> Result<ActivateAction, String> {
        ActivateAction::from_body(&body).ok_or_else(|| "not an activate action".to_string())
    }
}

/// B3.2 rule 9: the tuning key Degrade and Upgrade move an "Activate N" by (`tuning::tuned_count`).
pub const ACTIVATE_TUNING_KEY: &str = "Activate";

// ---------------------------------------------------------------------------
// The card and its abilities
// ---------------------------------------------------------------------------

/// B3.2 rule 2: the card acts on the field — it is the top of its pile (a card dormant under a Stack
/// pile is not on the field for effects, R13, and the same holds under a backrow pile), and a backrow
/// card is face-up: a face-down card has no text anyone can use.
pub fn is_acting_on_field(state: &GameState, card: &CardInstance) -> bool {
    // §3.2, R13, R446, R447: the top of a pile of either row, or a Unit a carrier holds — never a card
    // dormant beneath (`zones::acts_on_field`) — and a backrow card only while it is face-up.
    let Some(at) = slot_of(state, card) else {
        return false;
    };
    if !acts_on_field(state, card) {
        return false;
    }
    !(at.row == Row::Backrow && is_face_down(state, card))
}

/// The abilities the card has now: its running face's (`script_of`, so a Vanilla card has none, §6.3),
/// ids made unique for a fused card (`script::activation_decls`, R102), less any it does not have on this
/// instance (`ActivationDecl.has`).
pub fn abilities_of(state: &GameState, card: &CardInstance) -> Vec<ActivationDecl> {
    activation_decls(&crate::scripts::script_of(state, card))
        .into_iter()
        .filter(|decl| {
            decl.has.as_ref().is_none_or(|has| {
                has(HookArgs {
                    state,
                    self_: card,
                    radiant: card.radiant,
                })
            })
        })
        .collect()
}

/// The ability an action names, or the refusal: none named picks the card's only one.
fn find_ability(
    state: &GameState,
    card: &CardInstance,
    ability: Option<&str>,
) -> Result<ActivationDecl, EngineError> {
    let abilities = abilities_of(state, card);
    if abilities.is_empty() {
        return Err("that card has no Activate ability".into());
    }
    let Some(ability) = ability else {
        if abilities.len() > 1 {
            return Err("that card has several abilities: name the one to activate".into());
        }
        return match abilities.into_iter().next() {
            Some(only) => Ok(only),
            None => Err("that card has several abilities: name the one to activate".into()),
        };
    };
    abilities
        .into_iter()
        .find(|decl| decl.id == ability)
        .ok_or_else(|| EngineError::new(format!("that card has no ability \"{ability}\"")))
}

fn declared_of(decl: &ActivationDecl) -> DeclaredChoices {
    DeclaredChoices {
        targets: decl.targets.clone(),
        modes: decl.modes.clone(),
    }
}

/// B5 E5, R450: the discards the ability's declared targets cost (Classic #89).
fn discards_owed(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    decl: &ActivationDecl,
    targets: &[Selection],
    modes: &[String],
) -> i32 {
    targeting_discards_required(state, player, card, targets, modes, Some(decl.targets.as_slice()))
}

/// R450: the hand cards the activation's own picks use, which cannot pay its targeting cost.
fn hand_picks(targets: &[Selection]) -> Vec<String> {
    targets
        .iter()
        .filter_map(|selection| match selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Uses (B3.2 rules 1, 3, 7, 9)
// ---------------------------------------------------------------------------

/// R384, SPEC §6.2: where an instance counts its uses — `{ turn, count }` for the turn it names.
pub const ACTIVATIONS_MEMORY_KEY: &str = "activations";

/// `{ turn, count }`, as numbers came back through JSON (TS's `typeof … === "number"`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct UsesRecord {
    turn: f64,
    count: f64,
}

fn uses_record(card: &CardInstance) -> Option<UsesRecord> {
    let raw = card.memory.get(ACTIVATIONS_MEMORY_KEY)?;
    if !raw.is_object() && !raw.is_array() {
        return None;
    }
    let turn = raw.get("turn").and_then(Value::as_f64)?;
    let count = raw.get("count").and_then(Value::as_f64)?;
    Some(UsesRecord { turn, count })
}

/// How many times the card's abilities have been used this turn.
pub fn uses_this_turn(state: &GameState, card: &CardInstance) -> i32 {
    match uses_record(card) {
        Some(record) if record.turn == f64::from(state.turn) => record.count as i32,
        _ => 0,
    }
}

/// B3.2 rules 1, 7, 9: how many uses the ability allows each turn. "Activate N" is N moved by Degrade
/// and Upgrade (never below 1, R386's floor); "Activate ♾️" is `ACTIVATE_UNLIMITED_CAP`, which no
/// tuning moves.
pub fn uses_allowed(card: &CardInstance, decl: &ActivationDecl) -> i32 {
    match decl.uses {
        ActivationUses::Unlimited => ACTIVATE_UNLIMITED_CAP,
        ActivationUses::Count(n) => tuned_count(card, ACTIVATE_TUNING_KEY, n.max(1)),
    }
}

fn mark_use(state: &mut GameState, card_id: &str) {
    let Some(card) = find_instance(state, card_id) else {
        return;
    };
    let record = json!({ "turn": state.turn, "count": uses_this_turn(state, card) + 1 });
    if let Some(card) = find_instance_mut(state, card_id) {
        card.memory.insert(ACTIVATIONS_MEMORY_KEY.to_string(), record);
    }
}

// ---------------------------------------------------------------------------
// The refusal, which is also the list (B3.2 rules 2, 4, 8)
// ---------------------------------------------------------------------------

/// B3.2 rule 4: the units a Tribute cost may take — the controller's acting units, the card too.
fn tribute_units_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    decl: &ActivationDecl,
) -> Vec<CardInstance> {
    let units: Vec<CardInstance> = active_units_of(state, player).into_iter().cloned().collect();
    // "Tribute this" pays with the card itself, so it is not also one of the units a Tribute counts.
    // R683: a cost that excludes itself (Classic #21) never lists the card either.
    // R1220: no Tribute cost may take an Untributable card.
    let cost = decl.cost.unwrap_or_default();
    if cost.tribute_self == Some(true) || cost.tribute_excludes_self == Some(true) {
        return units
            .into_iter()
            .filter(|unit| unit.id != card.id && !crate::query::is_untributable(state, unit))
            .collect();
    }
    units
        .into_iter()
        .filter(|unit| !crate::query::is_untributable(state, unit))
        .collect()
}

fn plural(count: i32, one: &str) -> String {
    if count == 1 {
        format!("a {one}")
    } else {
        format!("{count} {one}s")
    }
}

/// The turn, the uses, the text's own condition and the costs: why the ability cannot be used now.
fn why_ability_unusable(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    decl: &ActivationDecl,
) -> Result<(), EngineError> {
    if state.result.is_some() {
        return Err("the game is over".into());
    }
    if state.pending.is_some() {
        return Err("answer the open prompt first".into());
    }
    if state.active != player {
        return Err("it is not your turn".into());
    }
    if state.phase != Phase::Main {
        return Err("an ability is activated in the main phase".into());
    }

    let allowed = uses_allowed(card, decl);
    if uses_this_turn(state, card) >= allowed {
        return Err(if allowed == 1 {
            EngineError::new("that ability has already been used this turn")
        } else {
            EngineError::new(format!("that ability has been used {allowed} times this turn"))
        });
    }
    if let Some(can_activate) = &decl.can_activate
        && !can_activate(ConditionContext {
            state,
            self_: card,
            controller: player,
            radiant: card.radiant,
            zone: ConditionZone::Field,
            your_turn: true,
        })
    {
        return Err("that ability can't be activated now".into());
    }

    let side = &state.players[player];
    let cost = decl.cost.unwrap_or_default();
    let mana = cost.mana.unwrap_or(0).max(0);
    // R1223: an Activate's mana cost may be borrowed too.
    if mana > crate::credit::spendable_mana(state, player) {
        return Err(EngineError::new(format!(
            "that ability costs {mana}, more than your mana"
        )));
    }
    let discards = cost.discard_random.unwrap_or(0).max(0);
    if discards > side.hand.len() as i32 {
        return Err(EngineError::new(format!(
            "that ability needs {} in your hand to discard",
            plural(discards, "card")
        )));
    }
    // R1220: an Untributable card's own "Tribute this" can never be paid.
    if cost.tribute_self == Some(true) && crate::query::is_untributable(state, card) {
        return Err("that card can't be Tributed".into());
    }
    let tributes = cost.tribute.unwrap_or(0).max(0);
    if tributes > tribute_units_for(state, player, card, decl).len() as i32 {
        return Err(EngineError::new(format!(
            "that ability needs {} to Tribute",
            plural(tributes, "Unit")
        )));
    }
    Ok(())
}

/// R384: why `player` cannot activate that ability of that card right now, or `Ok` when they can.
/// `ability` names one of several (B3.2 rule 5); none named is the card's only one. `legal_actions`
/// lists an `activate` for exactly the abilities this answers `Ok` for, so the two agree (§10.2).
pub fn why_cannot_activate_ability(
    state: &GameState,
    player: PlayerId,
    instance_id: &str,
    ability: Option<&str>,
) -> Result<(), EngineError> {
    let Some(card) = find_instance(state, instance_id) else {
        return Err(EngineError::new(format!("no card {instance_id}")));
    };
    if card.controller != player {
        return Err("that card is not yours".into());
    }
    if card.zone.z() != ZoneName::Field {
        return Err("that card is not on the field".into());
    }
    if !is_acting_on_field(state, card) {
        return Err(if is_face_down(state, card) {
            "a face-down card has no ability to use".into()
        } else {
            "that card is under a pile and does not act".into()
        });
    }
    let decl = find_ability(state, card, ability)?;
    why_ability_unusable(state, player, card, &decl)
}

/// B3.2 rule 4: the units an action tributes, checked: exactly the number the cost names, each once.
fn refuse_tributes(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    decl: &ActivationDecl,
    picked: &[String],
) -> Result<(), EngineError> {
    let need = decl.cost.and_then(|cost| cost.tribute).unwrap_or(0).max(0);
    if need == 0 {
        return if picked.is_empty() {
            Ok(())
        } else {
            Err("that ability needs no Tribute".into())
        };
    }
    let legal: Vec<String> = tribute_units_for(state, player, card, decl)
        .into_iter()
        .map(|unit| unit.id)
        .collect();
    let mut seen: Vec<&str> = Vec::new();
    for id in picked {
        if !legal.contains(id) {
            return Err(EngineError::new(format!(
                "{id} cannot be tributed for that ability"
            )));
        }
        if seen.contains(&id.as_str()) {
            return Err("that ability cannot tribute the same Unit twice".into());
        }
        seen.push(id);
    }
    if picked.len() as i32 == need {
        Ok(())
    } else {
        Err(EngineError::new(format!(
            "that ability tributes {}",
            plural(need, "Unit")
        )))
    }
}

/// §9.3 "reduce refuses illegal actions itself", for everything an `activate` carries: the card and
/// the ability (`why_cannot_activate_ability`), the targets and modes it declares (R81, R90, through the
/// same `play_choices` rules a play is read by), and the units a Tribute cost takes.
pub fn why_activate_refused(
    state: &GameState,
    player: PlayerId,
    action: &ActivateAction,
) -> Result<(), EngineError> {
    why_cannot_activate_ability(state, player, &action.instance_id, action.ability.as_deref())?;
    let Some(card) = find_instance(state, &action.instance_id) else {
        return Err(EngineError::new(format!("no card {}", action.instance_id)));
    };
    let decl = find_ability(state, card, action.ability.as_deref())?;
    let targets: &[Selection] = action.targets.as_deref().unwrap_or(&[]);
    let modes: &[String] = action.modes.as_deref().unwrap_or(&[]);
    why_declared_choices_refused(state, player, card, &declared_of(&decl), targets, modes)?;
    refuse_tributes(
        state,
        player,
        card,
        &decl,
        action.tributes.as_deref().unwrap_or(&[]),
    )?;
    // B5 E5, R450, R682: a declared target that costs discards needs that many other cards held —
    // the discards are random at pay time, so the action carries none.
    why_targeting_discards_unpayable(
        state,
        player,
        discards_owed(state, player, card, &decl, targets, modes),
        &hand_picks(targets),
    )
}

/// Every `size`-unit set of `units`, in board order (§6.3's Tribute: the price, listed whole, R90).
fn unit_sets(units: &[CardInstance], size: i32) -> Vec<Vec<String>> {
    if size <= 0 {
        return vec![Vec::new()];
    }
    fn walk(
        units: &[CardInstance],
        size: usize,
        from: usize,
        chosen: &mut Vec<String>,
        out: &mut Vec<Vec<String>>,
    ) {
        if chosen.len() == size {
            out.push(chosen.clone());
            return;
        }
        for (at, unit) in units.iter().enumerate().skip(from) {
            chosen.push(unit.id.clone());
            walk(units, size, at + 1, chosen, out);
            chosen.pop();
        }
    }
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut chosen: Vec<String> = Vec::new();
    walk(units, size as usize, 0, &mut chosen, &mut out);
    out
}

/// B3.2 rule 8, R384: every `activate` action this card's abilities offer `player` now — each usable
/// ability crossed with the Tribute sets its cost may take and the target and mode combinations it
/// declares, bounded as a play's are (`play_choices::play_choice_combinations`, R90).
pub fn activate_actions_for(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<ActivateAction> {
    let mut out: Vec<ActivateAction> = Vec::new();
    for decl in abilities_of(state, card) {
        if why_cannot_activate_ability(state, player, &card.id, Some(&decl.id)).is_err() {
            continue;
        }
        let tribute_sets = unit_sets(
            &tribute_units_for(state, player, card, &decl),
            decl.cost.and_then(|cost| cost.tribute).unwrap_or(0).max(0),
        );
        let choices = play_choice_combinations(state, player, card, Some(&declared_of(&decl)));
        for tributes in &tribute_sets {
            for choice in &choices {
                // B5 E5, R450, R682: the targets' discard cost is random at pay time, so it lists no
                // paying sets — one action, offered only when the cost can be paid at all.
                let targets: Vec<Selection> = choice.targets.clone().unwrap_or_default();
                let modes: Vec<String> = choice.modes.clone().unwrap_or_default();
                let owed = discards_owed(state, player, card, &decl, &targets, &modes);
                if why_targeting_discards_unpayable(state, player, owed, &hand_picks(&targets)).is_err() {
                    continue;
                }
                let mut pushed = ActivateAction {
                    instance_id: card.id.clone(),
                    ability: Some(decl.id.clone()),
                    tributes: if tributes.is_empty() {
                        None
                    } else {
                        Some(tributes.clone())
                    },
                    targets: choice.targets.clone(),
                    modes: choice.modes.clone(),
                };
                // R1200: while a Mayor acts the activation carries no declared targets — the
                // reducer draws them — so collapsed choices list one action, not one per target set.
                if crate::random_targets::targets_random(state) {
                    pushed.targets = crate::play_choices::without_target_picks(
                        state,
                        player,
                        card,
                        &declared_of(&decl).targets,
                        &targets,
                        &modes,
                    );
                    if out.contains(&pushed) {
                        continue;
                    }
                }
                out.push(pushed);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Activating (B3.2 rules 3, 4, 6)
// ---------------------------------------------------------------------------

/// R384, R78: a unit a Tribute cost took, as it stood before it died — Classic #21 Turtinator deals
/// damage equal to its Attack "as it stood" (last-known information, as a Death hook reads, R89).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TributedUnit {
    pub instance_id: String,
    pub def_id: String,
    pub radiant: bool,
    pub attack: i32,
    pub health: i32,
    pub max_health: i32,
}

/// Where an ability's effect list finds what its activation paid (`activation_paid`).
pub const ACTIVATION_DATA_KEY: &str = "__activation";

/// What an ability's effect list reads of the activation that runs it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActivationPaid {
    pub ability: String,
    pub tributed: Vec<TributedUnit>,
}

/// The units a JSON list holds, read back defensively (TS kept whatever the array held).
fn tributed_from(raw: Option<&Value>) -> Vec<TributedUnit> {
    match raw {
        Some(list @ Value::Array(_)) => {
            serde_json::from_value::<Vec<TributedUnit>>(list.clone()).unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

/// R384: what the activation running this effect paid — the ability's id and the units its Tribute
/// cost took, as they stood (`TributedUnit`). Carried in the run's data, so a step a prompt re-enters
/// reads it too. Empty outside an activation.
pub fn activation_paid(ctx: &EffectContext<'_>) -> ActivationPaid {
    let Some(raw) = ctx.data.get(ACTIVATION_DATA_KEY) else {
        return ActivationPaid::default();
    };
    if !raw.is_object() && !raw.is_array() {
        return ActivationPaid::default();
    }
    ActivationPaid {
        ability: raw
            .get("ability")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_default(),
        tributed: tributed_from(raw.get("tributed")),
    }
}

/// One activation between its costs and its effect: all JSON, so a pause can owe it (R113).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ActivationRun {
    player: PlayerId,
    instance_id: String,
    def_id: String,
    radiant: bool,
    ability: String,
    targets: Vec<Selection>,
    modes: Vec<String>,
    tributed: Vec<TributedUnit>,
    /// R174: the field's departures when the choices were checked, so a target the costs took is gone.
    exits_from: u32,
}

/// R113: the `resume.hook` of an activation whose cost paused before its effect ran.
pub const ACTIVATION_WORK: &str = "@activate";

const RUN_DATA_KEY: &str = "run";

fn snapshot_of(state: &GameState, unit: &CardInstance) -> TributedUnit {
    let view = unit_view(state, unit);
    TributedUnit {
        instance_id: unit.id.clone(),
        def_id: unit.def_id.clone(),
        radiant: unit.radiant,
        attack: view.attack,
        health: view.health,
        max_health: view.max_health,
    }
}

/// B3.2 rule 4: the costs, in the order the ability names them — mana, then the discards its declared
/// targets cost (B5 E5, R450), then a random discard, then the Tribute (the tributed units and "Tribute
/// this" die together as one payment, §6.3, R101). A Tribute is a Sacrifice, so it is a death in full:
/// Death hooks, Reborn, the destroyed counter (§4.5).
fn pay_costs(
    sink: &mut EngineSink<'_>,
    run: &mut ActivationRun,
    card: &CardInstance,
    decl: &ActivationDecl,
    tributes: &[String],
) {
    let cost = decl.cost.unwrap_or_default();

    let mana = cost.mana.unwrap_or(0).max(0);
    if mana > 0 {
        // R1223: an Activate's mana cost may be borrowed past current mana.
        crate::credit::pay_mana(sink.state, run.player, mana);
        let event = mana_event(run.player, &sink.state.players[run.player]);
        sink.events.push(event);
    }

    // B5 E5, R450, R682: a targeting cost is part of the price, paid with it (Classic #89) — random
    // cards from the hand, drawn at pay time. Never a hand card the activation picks.
    let owed = discards_owed(sink.state, run.player, card, decl, &run.targets, &run.modes);
    if owed > 0 {
        pay_targeting_discards(sink, run.player, owed, &hand_picks(&run.targets));
    }

    // R800: a random discard paid as the price is no forced discard, so a discard guard never stops it.
    let random = cost.discard_random.unwrap_or(0).max(0);
    if random > 0 {
        discard_random_cards(sink, run.player, random);
    }

    let units: Vec<CardInstance> = tributes
        .iter()
        .filter_map(|id| {
            find_instance(sink.state, id)
                .filter(|unit| unit.zone.z() == ZoneName::Field)
                .cloned()
        })
        .collect();
    run.tributed = units.iter().map(|unit| snapshot_of(sink.state, unit)).collect();
    let mut paying = units;
    if cost.tribute_self == Some(true)
        && let Some(live) =
            find_instance(sink.state, &card.id).filter(|live| live.zone.z() == ZoneName::Field)
    {
        paying.push(live.clone());
    }
    if !paying.is_empty() {
        sacrifice_together(sink, &paying);
    }
}

/// The ability's effect, run as the card's own list under `activation:<id>` so a prompt inside it
/// pauses the rest (R113) and its answer comes back to the same ability. The card is its `self`
/// wherever it is now (a card that tributed itself resolves from its graveyard, as a Death hook does),
/// with the face it was activated with. R59: the state check follows the whole ability; one whose list
/// is still asking is not whole yet, and the loop checks once it is.
fn run_effect(sink: &mut EngineSink<'_>, run: &ActivationRun) {
    let paid = ActivationPaid {
        ability: run.ability.clone(),
        tributed: run.tributed.clone(),
    };
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        ACTIVATION_DATA_KEY.to_string(),
        serde_json::to_value(&paid).unwrap_or(Value::Null),
    );
    run_hook_resumable(
        sink,
        HookInstance {
            id: run.instance_id.clone(),
            def_id: run.def_id.clone(),
            controller: run.player,
            radiant: run.radiant,
        },
        &activation_hook(&run.ability),
        HookResumableOptions {
            controller: Some(run.player),
            targets: Some(run.targets.clone()),
            modes: Some(run.modes.clone()),
            data: Some(data),
            exits_from: Some(run.exits_from),
        },
    );
    if !paused(sink) {
        state_check(sink);
    }
}

/// R113, R117: owe the effect of an activation whose cost paused, at the moment it paused.
fn owe_effect(sink: &mut EngineSink<'_>, run: &ActivationRun) {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        RUN_DATA_KEY.to_string(),
        serde_json::to_value(run).unwrap_or(Value::Null),
    );
    let resume = Resume {
        def_id: run.def_id.clone(),
        hook: ACTIVATION_WORK.to_string(),
        step: run.ability.clone(),
        radiant: run.radiant,
        instance_id: Some(run.instance_id.clone()),
        data,
    };
    push_work(sink, resume, Some(run.player));
}

/// The run an owed item carries, read back defensively: it came through JSON (§10.1).
fn run_from(data: &IndexMap<String, Value>) -> Option<ActivationRun> {
    let raw = data.get(RUN_DATA_KEY)?;
    if !raw.is_object() && !raw.is_array() {
        return None;
    }
    let player = match raw.get("player").and_then(Value::as_str) {
        Some("p1") => PlayerId::P1,
        Some("p2") => PlayerId::P2,
        _ => return None,
    };
    let instance_id = raw.get("instanceId").and_then(Value::as_str)?.to_string();
    let def_id = raw.get("defId").and_then(Value::as_str)?.to_string();
    let ability = raw.get("ability").and_then(Value::as_str)?.to_string();
    let exits_from = raw.get("exitsFrom").and_then(Value::as_f64)? as u32;
    let targets: Vec<Selection> = match raw.get("targets") {
        Some(list @ Value::Array(_)) => serde_json::from_value(list.clone()).unwrap_or_default(),
        _ => Vec::new(),
    };
    let modes: Vec<String> = match raw.get("modes") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|mode| mode.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    };
    Some(ActivationRun {
        player,
        instance_id,
        def_id,
        radiant: raw.get("radiant") == Some(&Value::Bool(true)),
        ability,
        targets,
        modes,
        tributed: tributed_from(raw.get("tributed")),
        exits_from,
    })
}

/// R113: the `"@activate"` work item — an activation whose cost paused — runs its effect now. TS
/// registered this as the hook's work handler; `work.rs`'s dispatcher calls it (SURFACE §6.6).
pub fn run_owed_activation(sink: &mut EngineSink<'_>, item: &WorkItem) {
    if let Some(run) = run_from(&item.resume.data) {
        run_effect(sink, &run);
    }
}

/// The `activate` action (B3.2, R384): validate, count the use, announce it (`activated`), pay the
/// costs, run the effect. The use is counted before anything can pause, so an ability that asks has
/// spent its use and the answer cannot buy another (as R43's power does). Not a play (rule 6): no turn
/// log, no play counter, no `cardPlayed`.
pub fn activate_ability(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    action: &ActivateAction,
) -> Result<(), EngineError> {
    // R1200: while a Mayor acts the action carries no declared targets — `legal_actions` offered
    // it stripped — so draw each one at random before anything is checked or paid. A card or an
    // ability this does not find is the validation's refusal to give, not this draw's.
    let mut owned = action.clone();
    if crate::random_targets::targets_random(sink.state)
        && let Some(card) = find_instance(sink.state, &owned.instance_id).cloned()
        && let Ok(decl) = find_ability(sink.state, &card, owned.ability.as_deref())
    {
        let given = owned.targets.clone().unwrap_or_default();
        let modes = owned.modes.clone().unwrap_or_default();
        let declared = declared_of(&decl);
        let drawn = crate::play_choices::with_drawn_target_picks(
            sink.state,
            &mut *sink.rng,
            player,
            &card,
            &declared.targets,
            &given,
            &modes,
        )?;
        owned.targets = Some(drawn);
    }
    let action: &ActivateAction = &owned;
    why_activate_refused(sink.state, player, action)?;
    let Some(card) = find_instance(sink.state, &action.instance_id).cloned() else {
        return Err(EngineError::new(format!("no card {}", action.instance_id)));
    };
    let decl = find_ability(sink.state, &card, action.ability.as_deref())?;

    let modes: Vec<String> = action.modes.clone().unwrap_or_default();
    let mut run = ActivationRun {
        player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        radiant: card.radiant,
        ability: decl.id.clone(),
        // R221, R90: each declaration's picks are a set, taken in the order it offers them.
        targets: in_declared_order(
            sink.state,
            player,
            &card,
            action.targets.as_deref().unwrap_or(&[]),
            &modes,
            Some(decl.targets.as_slice()),
        ),
        modes,
        tributed: Vec::new(),
        exits_from: exit_mark(sink.state),
    };

    mark_use(sink.state, &card.id);
    sink.events.push(GameEvent::Activated {
        player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        ability: decl.id.clone(),
    });
    pay_costs(
        sink,
        &mut run,
        &card,
        &decl,
        action.tributes.as_deref().unwrap_or(&[]),
    );
    if paused(sink) {
        if sink.state.result.is_none() {
            owe_effect(sink, &run);
        }
        return Ok(());
    }
    run_effect(sink, &run);
    Ok(())
}

// ---------------------------------------------------------------------------
// The view (§10.8)
// ---------------------------------------------------------------------------

/// R384: the card's abilities as its controller's client needs them — on the controller's own view
/// of a card acting on the field, and nowhere else (the other player reads the card's text; whether it
/// could be used is its controller's business). `usable` is exactly whether `legal_actions` lists it.
pub fn activation_views_for(
    state: &GameState,
    viewer: PlayerId,
    card: &CardInstance,
) -> Option<Vec<ActivationView>> {
    if card.controller != viewer || !is_acting_on_field(state, card) {
        return None;
    }
    let abilities = abilities_of(state, card);
    if abilities.is_empty() {
        return None;
    }
    Some(
        abilities
            .iter()
            .map(|decl| {
                let reason = why_cannot_activate_ability(state, viewer, &card.id, Some(&decl.id))
                    .err()
                    .map(|error| error.message);
                let uses_left = match decl.uses {
                    ActivationUses::Unlimited => None,
                    ActivationUses::Count(_) => {
                        Some((uses_allowed(card, decl) - uses_this_turn(state, card)).max(0))
                    }
                };
                ActivationView {
                    ability: decl.id.clone(),
                    label: decl.label.clone(),
                    uses_left,
                    usable: reason.is_none(),
                    reason,
                }
            })
            .collect(),
    )
}
