//! Effects that ask the controller something: Choose one, a target, a card in hand, and Discover
//! (SPEC §6.3, §10.6). Each opens a prompt and hands the answer to a named resume step.
//!
//! Port of `packages/engine/src/effects/choose.ts`.

use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::targets::{PlayerSpec, player_of};
use crate::catalog::{CatalogQueryArgs, def_of, excluding_def_id, query};
use crate::faces::card_type_of;
use crate::mana::effective_cost;
use crate::prompts::{
    ANSWER_KEY, OpenPromptArgs, answer_key_of, cell_option_label, hero_option_label, open_prompt, resume_self,
};
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, GameState, PromptOption, Resume};
use crate::wire::{
    CardType, CostRange, FilterOf, FilterSide, OneOrMany, PLAYER_IDS, PlayerId, PromptKind, Row, Selection, Tag,
    ZoneRef, opponent_of,
};
use crate::zones::{active_units_of, card_at, is_unit_token, row_size, slots_of};

/// A resume step's captured data (TS `data?: Record<string, unknown>`).
type StepData = Option<IndexMap<String, Value>>;

/// `OpenPromptArgs` with every optional field absent; a verb sets the ones it asks for.
fn ask(player: PlayerId, kind: PromptKind, prompt: String, options: Vec<PromptOption>, resume: Resume) -> OpenPromptArgs {
    OpenPromptArgs {
        player,
        kind,
        aim: None,
        prompt,
        options,
        min: None,
        max: None,
        budget: None,
        owner: None,
        resume,
    }
}

/// TS `options.slice(0, end)`: the length a slice from the start keeps (a negative end counts back
/// from the end, as in JS).
fn slice_end(len: usize, end: i32) -> usize {
    if end < 0 {
        len.saturating_sub(end.unsigned_abs() as usize)
    } else {
        (end as usize).min(len)
    }
}

/// Which cards a `target` prompt may offer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TargetScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<FilterSide>,
    /// TS `("unit" | "hero" | "backrow")[]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<Vec<FilterOf>>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<OneOrMany<CardType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_self: Option<bool>,
}

fn sides_of(ctx: &EffectContext<'_>, side: Option<FilterSide>) -> Vec<PlayerId> {
    match side {
        Some(FilterSide::Ally) => vec![ctx.controller],
        Some(FilterSide::Enemy) => vec![opponent_of(ctx.controller)],
        _ => PLAYER_IDS.to_vec(),
    }
}

/// Every card and hero a scope allows, in a deterministic order (active side first, lane order).
pub fn targets_in_scope(ctx: &EffectContext<'_>, scope: Option<&TargetScope>) -> Vec<Selection> {
    let none = TargetScope::default();
    let scope = scope.unwrap_or(&none);
    let kinds: Vec<FilterOf> = scope.of.clone().unwrap_or_else(|| vec![FilterOf::Unit]);
    let self_id: Option<&str> = ctx.self_.as_ref().map(|card| card.id.as_str());
    let excludes_self = scope.exclude_self == Some(true);
    let mut out: Vec<Selection> = Vec::new();

    for player in sides_of(ctx, scope.side) {
        if kinds.contains(&FilterOf::Unit) {
            for unit in active_units_of(ctx.state, player) {
                if excludes_self && Some(unit.id.as_str()) == self_id {
                    continue;
                }
                out.push(Selection::Instance {
                    instance_id: unit.id.clone(),
                });
            }
        }
        if kinds.contains(&FilterOf::Backrow) {
            for slot in slots_of(player, Row::Backrow) {
                let Some(card) = card_at(ctx.state, slot) else {
                    continue;
                };
                if excludes_self && Some(card.id.as_str()) == self_id {
                    continue;
                }
                out.push(Selection::Instance {
                    instance_id: card.id.clone(),
                });
            }
        }
        if kinds.contains(&FilterOf::Hero) {
            out.push(Selection::Hero { player });
        }
    }

    out
}

/// §10.6: an answered prompt's selection arrives in `ctx.targets`, so a Discover's pick — a `mode`
/// option carrying a def id — is read from there. Play-time modes (R81) arrive in `ctx.modes`, and a
/// resume step may be reached either way, so both are offered in order.
pub fn chosen_options(ctx: &EffectContext<'_>) -> Vec<String> {
    let mut picked: Vec<String> = ctx
        .targets
        .iter()
        .filter_map(|selection| match selection {
            Selection::Mode { option } => Some(option.clone()),
            _ => None,
        })
        .collect();
    picked.extend(ctx.modes.iter().cloned());
    picked
}

fn label(ctx: &EffectContext<'_>, selection: &Selection) -> String {
    match selection {
        Selection::Hero { player } => hero_option_label(*player, ctx.controller),
        Selection::Instance { instance_id } => match find_on_board(ctx, instance_id) {
            None => instance_id.clone(),
            Some(card) => def_of(Some(&*ctx.state), &card.def_id).name.clone(),
        },
        Selection::Mode { option } => option.clone(),
        _ => "nothing".to_string(),
    }
}

/// §10.6, §10.8: an option's key is what the client sends back, so one key names one option — two
/// Duplicating Felinors in reach are two options, and a key built from the label (the card's name)
/// gave both the same one. The key is built from the selection itself, which is unique among the
/// options by construction, and never from a name (R177 keeps names off what a view may not read).
fn key_of(selection: &Selection) -> String {
    match selection {
        Selection::Instance { instance_id } => format!("instance:{instance_id}"),
        Selection::Hero { player } => format!("hero:{player}"),
        Selection::Mode { option } => format!("mode:{option}"),
        Selection::Zone { player, row, lane } => format!("zone:{player}:{row}:{lane}"),
        Selection::None => "none".to_string(),
    }
}

fn find_on_board<'c>(ctx: &'c EffectContext<'_>, instance_id: &str) -> Option<&'c CardInstance> {
    let state: &'c GameState = &*ctx.state;
    for player in PLAYER_IDS {
        let side = &state.players[player];
        let found = side
            .hand
            .iter()
            .chain(side.graveyard.iter())
            .chain(side.units.iter().flatten().flatten())
            .chain(side.backrow.iter().flatten())
            // R446: a Unit a carrier holds stands in the backrow row.
            .chain(side.carried.iter().flatten().flatten())
            .find(|card| card.id == instance_id);
        if found.is_some() {
            return found;
        }
    }
    None
}

/// `choose_mode`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChooseModeArgs {
    pub options: Vec<String>,
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
    /// B5 E18: who answers. Default the card's controller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<PlayerSpec>,
    /// B5 E18: the caption of each option, by the option; absent is the option itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<IndexMap<String, String>>,
}

/// §6.3 Choose one: a mode prompt whose answer resumes the script at `step`.
///
/// B5 E18: `by: "enemy"` hands the prompt to the other player (Classic #8 Pickle: "your opponent
/// chooses"). The other player sees the options and answers; the answered step still runs as this
/// card's controller (`prompts::PROMPT_OWNER_KEY`), so "you draw" is yours. `labels` gives an option a
/// caption other than its own string (a quest reward's text, a difficulty's rolled reward).
pub fn choose_mode(args: ChooseModeArgs) -> Effect {
    Effect::new("chooseMode", move |ctx| {
        let player = player_of(ctx, args.by.unwrap_or(PlayerSpec::SelfSide));
        let owner = ctx.controller;
        let options: Vec<PromptOption> = args
            .options
            .iter()
            .map(|option| {
                let caption = args
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get(option))
                    .map(String::as_str)
                    .unwrap_or(option);
                mode_option(option, caption)
            })
            .collect();
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        let mut asked = ask(
            player,
            PromptKind::Mode,
            args.prompt.clone().unwrap_or_else(|| "Choose one".to_string()),
            options,
            resume,
        );
        asked.owner = Some(owner);
        open_prompt(ctx, asked);
    })
}

/// One fixed option: the string is what the answer carries back (`chosen_options`).
fn mode_option(option: &str, label: &str) -> PromptOption {
    PromptOption {
        key: format!("mode:{option}"),
        label: label.to_string(),
        selection: Selection::Mode {
            option: option.to_string(),
        },
        cost: None,
        radiant: None,
    }
}

/// `choose_target`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChooseTargetArgs {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<TargetScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
}

/// A target prompt (§10.6). With no legal target the effect fizzles and the card still resolves.
pub fn choose_target(args: ChooseTargetArgs) -> Effect {
    Effect::new("chooseTarget", move |ctx| {
        let selections = targets_in_scope(ctx, args.scope.as_ref());
        if selections.is_empty() {
            return;
        }
        let reader: &EffectContext<'_> = &*ctx;
        let options: Vec<PromptOption> = selections
            .into_iter()
            .map(|selection| PromptOption {
                key: key_of(&selection),
                label: label(reader, &selection),
                selection,
                cost: None,
                radiant: None,
            })
            .collect();
        let player = ctx.controller;
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        open_prompt(
            ctx,
            ask(
                player,
                PromptKind::Target,
                args.prompt.clone().unwrap_or_else(|| "Choose a target".to_string()),
                options,
                resume,
            ),
        );
    })
}

/// `choose_from_hand`'s `where`: only the cards this admits are offered.
pub type HandFilter = Arc<dyn Fn(&EffectContext<'_>, &CardInstance) -> bool + Send + Sync>;

/// `choose_from_hand`'s arguments (TS's inline object). `where_` is a callback, so serde skips it and a
/// card sets it in Rust.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChooseFromHandArgs {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
    /// Whose hand. Default the card's controller's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<PlayerSpec>,
    /// Who picks. Default the card's controller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<PlayerSpec>,
    /// Only the cards this admits are offered (C+ #31 Fusion Lab: no Immutable card, R23).
    #[serde(skip)]
    pub where_: Option<HandFilter>,
}

impl std::fmt::Debug for ChooseFromHandArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChooseFromHandArgs")
            .field("step", &self.step)
            .field("count", &self.count)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("prompt", &self.prompt)
            .field("data", &self.data)
            .field("of", &self.of)
            .field("by", &self.by)
            .field("where", &self.where_.is_some())
            .finish()
    }
}

/// A pick from a hand: your own by default (#26, #80).
///
/// B5 E16, E17, E18: `of` names whose hand and `by` who picks. `of: "enemy", by: "enemy"` is the
/// other player's own hand pick — Classic #8's "they discard", Classic #9's "they keep one card of
/// their choice" — answered by them, and continued as this card's controller. `of: "enemy"` with the
/// default `by` is Classic #11 Mind Melt's "look at your opponent's hand": their hand cards are the
/// options, which only the chooser is shown (§10.8, R81); the hand's owner sees that a prompt is open
/// and nothing more. `min` and `max` default to `count` picks, fewer when the hand is shorter.
pub fn choose_from_hand(args: ChooseFromHandArgs) -> Effect {
    Effect::new("chooseFromHand", move |ctx| {
        let whose = player_of(ctx, args.of.unwrap_or(PlayerSpec::SelfSide));
        let reader: &EffectContext<'_> = &*ctx;
        let hand: Vec<CardInstance> = reader.state.players[whose]
            .hand
            .iter()
            .filter(|card| args.where_.as_ref().is_none_or(|admits| admits(reader, card)))
            .cloned()
            .collect();
        if hand.is_empty() {
            return;
        }
        let len = hand.len() as i32;
        let count = args.count.unwrap_or(1).min(len);
        let options: Vec<PromptOption> = hand
            .iter()
            .map(|card| PromptOption {
                key: format!("instance:{}", card.id),
                label: def_of(Some(&*reader.state), &card.def_id).name.clone(),
                selection: Selection::Instance {
                    instance_id: card.id.clone(),
                },
                cost: None,
                radiant: if card.radiant { Some(true) } else { None },
            })
            .collect();
        let player = player_of(ctx, args.by.unwrap_or(PlayerSpec::SelfSide));
        let owner = ctx.controller;
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        let mut asked = ask(
            player,
            PromptKind::Hand,
            args.prompt
                .clone()
                .unwrap_or_else(|| "Choose a card in your hand".to_string()),
            options,
            resume,
        );
        asked.owner = Some(owner);
        asked.min = Some(args.min.unwrap_or(count).min(len));
        asked.max = Some(args.max.unwrap_or(count).min(len));
        open_prompt(ctx, asked);
    })
}

// ---------------------------------------------------------------------------
// B5 E17, E18: the new prompt kinds (docs/classic-sets.md B5)
// ---------------------------------------------------------------------------

/// `choose_cost_in_hand`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChooseCostInHandArgs {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
}

/// B5 E17: the costs present in a hand, each an option of a `mode` prompt (Classic #11 Mind Melt's
/// Radiant: "choose a cost; exile every card of that cost from it"). A cost is the one the card would
/// be played for now (R65, `effective_cost`), which is also what `exile_matching`'s `cost` reads, so
/// the group chosen is the group exiled. Each option's caption names the cards of that cost, which
/// only the chooser is shown; `chosen_number` reads the answer. An empty hand asks nothing.
pub fn choose_cost_in_hand(args: ChooseCostInHandArgs) -> Effect {
    Effect::new("chooseCostInHand", move |ctx| {
        let whose = player_of(ctx, args.of.unwrap_or(PlayerSpec::Enemy));
        let state: &GameState = &*ctx.state;
        let mut groups: IndexMap<i32, Vec<String>> = IndexMap::new();
        for card in &state.players[whose].hand {
            let cost = effective_cost(state, card, Default::default());
            groups
                .entry(cost)
                .or_default()
                .push(def_of(Some(state), &card.def_id).name.clone());
        }
        let mut costs: Vec<i32> = groups.keys().copied().collect();
        costs.sort();
        let options: Vec<PromptOption> = costs
            .iter()
            .map(|cost| {
                let names = groups.get(cost).map(|names| names.join(", ")).unwrap_or_default();
                mode_option(&cost.to_string(), &format!("({cost}) {names}"))
            })
            .collect();
        let player = player_of(ctx, args.by.unwrap_or(PlayerSpec::SelfSide));
        let owner = ctx.controller;
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        let mut asked = ask(
            player,
            PromptKind::Mode,
            args.prompt.clone().unwrap_or_else(|| "Choose a cost".to_string()),
            options,
            resume,
        );
        asked.owner = Some(owner);
        open_prompt(ctx, asked);
    })
}

/// `choose_number`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChooseNumberArgs {
    pub step: String,
    pub from: i32,
    pub to: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
}

/// B5 E18: a number from a fixed range, `from` to `to` inclusive (Classic #18 Glitch in the System
/// declares its 0 to 10 with the play instead, as a `ModeDecl` of kind `number`, R81). The options are
/// the numbers, so they reveal nothing. `chosen_number` reads the answer.
pub fn choose_number(args: ChooseNumberArgs) -> Effect {
    Effect::new("chooseNumber", move |ctx| {
        let low = args.from.min(args.to);
        let high = args.from.max(args.to);
        let options: Vec<PromptOption> = (low..=high)
            .map(|number| {
                let number = number.to_string();
                mode_option(&number, &number)
            })
            .collect();
        let player = player_of(ctx, args.by.unwrap_or(PlayerSpec::SelfSide));
        let owner = ctx.controller;
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        let mut asked = ask(
            player,
            PromptKind::Number,
            args.prompt.clone().unwrap_or_else(|| "Choose a number".to_string()),
            options,
            resume,
        );
        asked.owner = Some(owner);
        open_prompt(ctx, asked);
    })
}

/// B5 E18: the number an answered `number` prompt (or a play's `number` mode) carries, if any.
///
/// TS `Number(option)` on a non-blank option, kept when it is an integer: the option, trimmed, read
/// as a decimal number.
pub fn chosen_number(ctx: &EffectContext<'_>) -> Option<i32> {
    for option in chosen_options(ctx) {
        let trimmed = option.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = trimmed.parse::<f64>()
            && value.is_finite()
            && value.fract() == 0.0
        {
            return Some(value as i32);
        }
    }
    None
}

/// R465: the ids the options of an `answer` prompt carry, in the order they are shown.
pub const ANSWER_OPTION_IDS: &[&str] = &["A", "B", "C", "D", "E", "F", "G", "H"];

/// `choose_answer`'s arguments (TS's inline object). `correct` is an index into `options`, so the
/// half of TS's guard that refused a negative one cannot arise.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChooseAnswerArgs {
    pub step: String,
    pub statement: String,
    pub options: Vec<String>,
    pub correct: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shuffle: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
}

/// B5 E18, R465: a multiple-choice problem (Classic+ #42 KY's Test). `statement` is the prompt's text
/// and `options` the answers, `correct` the index of the right one among them. The options are shown
/// in an order the match rng shuffles (unless `shuffle` is false), each under a letter
/// (`ANSWER_OPTION_IDS`) that says nothing about which is right, and the right letter goes into the
/// prompt's resume data under `prompts::ANSWER_KEY` — which `view_for` never sends and the AI's
/// redaction strips (R185) — so the key never leaves the engine. The answered step reads the verdict
/// with `answered_correctly`. At most `ANSWER_OPTION_IDS.len()` options; fewer than two asks nothing.
pub fn choose_answer(args: ChooseAnswerArgs) -> Effect {
    Effect::new("chooseAnswer", move |ctx| {
        // `(text, index)`: TS `{ text, index }`.
        let given: Vec<(String, usize)> = args
            .options
            .iter()
            .take(ANSWER_OPTION_IDS.len())
            .enumerate()
            .map(|(index, text)| (text.clone(), index))
            .collect();
        if given.len() < 2 || args.correct >= given.len() {
            return;
        }
        let shown = if args.shuffle == Some(false) {
            given
        } else {
            ctx.rng.shuffle(&given)
        };
        let at = shown.iter().position(|(_, index)| *index == args.correct);
        let key = at
            .and_then(|at| ANSWER_OPTION_IDS.get(at))
            .copied()
            .unwrap_or("")
            .to_string();
        let options: Vec<PromptOption> = shown
            .iter()
            .enumerate()
            .map(|(index, (text, _))| {
                let id = ANSWER_OPTION_IDS
                    .get(index)
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| index.to_string());
                mode_option(&id, text)
            })
            .collect();
        let mut data = args.data.clone().unwrap_or_default();
        data.insert(ANSWER_KEY.to_string(), Value::String(key));
        let player = player_of(ctx, args.by.unwrap_or(PlayerSpec::SelfSide));
        let owner = ctx.controller;
        let resume = resume_self(ctx, &args.step, data);
        let mut asked = ask(player, PromptKind::Answer, args.statement.clone(), options, resume);
        asked.owner = Some(owner);
        open_prompt(ctx, asked);
    })
}

/// R465: whether the answer to the step's `answer` prompt was the right one. False with no key in the
/// data: a state whose key was stripped (the AI's redacted copy) judges no answer right, so what the
/// AI simulates never tells one option from another and it answers from what the prompt shows.
pub fn answered_correctly(ctx: &EffectContext<'_>) -> bool {
    let Some(key) = answer_key_of(&ctx.data) else {
        return false;
    };
    let picked = ctx
        .targets
        .iter()
        .find(|selection| matches!(selection, Selection::Mode { .. }));
    matches!(picked, Some(Selection::Mode { option }) if *option == key)
}

/// `CellScope.side`: sides, relative to the card's controller.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum CellSide {
    Any,
    #[serde(rename = "self")]
    SelfSide,
    Enemy,
}

/// Which board cells a `cell` prompt offers (B5 E18).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CellScope {
    /// Sides, relative to the card's controller. Default both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<CellSide>,
    /// Rows. Default both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Row>>,
    /// Lanes to leave out (Classic+ #62's lanes already used).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub except_lanes: Option<Vec<i32>>,
}

/// `choose_cell`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChooseCellArgs {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cells: Option<CellScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
}

/// B5 E18: a board cell — a zone of either side and either row, empty or not — as a `zone` selection
/// (Classic+ #62 KY's Papaya's points, asked one at a time). The options run the chooser's own side
/// first, backrow then units, lane 1 upward, then the other side's units then backrow: the rows in the
/// order they lie on the board from the chooser's hero outward. `done` adds an option that picks no
/// cell (`{ pick: "none" }`), for a question a player may stop answering. `chosen_cells` reads it.
pub fn choose_cell(args: ChooseCellArgs) -> Effect {
    Effect::new("chooseCell", move |ctx| {
        let chooser = player_of(ctx, args.by.unwrap_or(PlayerSpec::SelfSide));
        let scope = args.cells.clone().unwrap_or_default();
        let rows: Vec<Row> = scope.rows.clone().unwrap_or_else(|| vec![Row::Units, Row::Backrow]);
        let except: IndexSet<i32> = scope.except_lanes.clone().unwrap_or_default().into_iter().collect();
        let mut sides: Vec<(PlayerId, [Row; 2])> = Vec::new();
        if scope.side != Some(CellSide::Enemy) {
            sides.push((ctx.controller, [Row::Backrow, Row::Units]));
        }
        if scope.side != Some(CellSide::SelfSide) {
            sides.push((opponent_of(ctx.controller), [Row::Units, Row::Backrow]));
        }
        let mut options: Vec<PromptOption> = Vec::new();
        for (player, order) in sides {
            for row in order {
                if !rows.contains(&row) {
                    continue;
                }
                for lane in 1..=row_size(row) {
                    if except.contains(&lane) {
                        continue;
                    }
                    let selection = Selection::Zone { player, row, lane };
                    options.push(PromptOption {
                        key: key_of(&selection),
                        label: cell_option_label(player, row, lane, chooser),
                        selection,
                        cost: None,
                        radiant: None,
                    });
                }
            }
        }
        if options.is_empty() {
            return;
        }
        if args.done == Some(true) {
            options.push(PromptOption {
                key: "none".to_string(),
                label: "Done".to_string(),
                selection: Selection::None,
                cost: None,
                radiant: None,
            });
        }
        let owner = ctx.controller;
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        let mut asked = ask(
            chooser,
            PromptKind::Cell,
            args.prompt.clone().unwrap_or_else(|| "Choose a cell".to_string()),
            options,
            resume,
        );
        asked.owner = Some(owner);
        open_prompt(ctx, asked);
    })
}

/// B5 E18: the cells an answered `cell` prompt picked, in offered order; empty for "done".
pub fn chosen_cells(ctx: &EffectContext<'_>) -> Vec<ZoneRef> {
    ctx.targets
        .iter()
        .filter_map(|selection| match selection {
            Selection::Zone { player, row, lane } => Some(ZoneRef {
                player: *player,
                row: *row,
                lane: *lane,
            }),
            _ => None,
        })
        .collect()
}

/// One reward `choose_reward` offers: the id the answer carries back, under its caption.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RewardOption {
    pub id: String,
    pub label: String,
}

/// `choose_reward`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChooseRewardArgs {
    pub step: String,
    pub rewards: Vec<RewardOption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
}

/// B5 E18: a completed quest's reward (Classic #90 In Too Deep): one of `rewards`, each an id the
/// answer carries back (`chosen_options`) under its caption. Asked of the card's controller whoever's
/// turn it is — a non-active player's prompt on the opponent's turn (R79's prompt clock).
pub fn choose_reward(args: ChooseRewardArgs) -> Effect {
    Effect::new("chooseReward", move |ctx| {
        let options: Vec<PromptOption> = args
            .rewards
            .iter()
            .map(|reward| mode_option(&reward.id, &reward.label))
            .collect();
        let player = ctx.controller;
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        open_prompt(
            ctx,
            ask(
                player,
                PromptKind::Reward,
                args.prompt.clone().unwrap_or_else(|| "Choose a reward".to_string()),
                options,
                resume,
            ),
        );
    })
}

/// `PileSpec.zone`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum PileZone {
    Graveyard,
    Exile,
    Hand,
    Library,
    Field,
}

/// A pile a `pick` prompt draws its options from (B5 E18): a graveyard, an exile pile, a hand (the other
/// player's too — Classic #11's "look at your opponent's hand"), a library, or the field (the tops of
/// the unit piles and the backrow cards of that side — Classic #78's Radiant "on your field, in your
/// hand or in your deck").
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PileSpec {
    pub zone: PileZone,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// JS `Number.MAX_SAFE_INTEGER`.
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// `c17` → 17: an instance's place in creation order (`state::new_instance` numbers ids from `nextId`).
fn creation_number(id: &str) -> f64 {
    match id.strip_prefix('c') {
        Some(digits) if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) => {
            digits.parse::<f64>().unwrap_or(MAX_SAFE_INTEGER)
        }
        _ => MAX_SAFE_INTEGER,
    }
}

/// The cards of one pile, in the order a `pick` prompt offers them. A library's are in the order the
/// instances were created, never library order, which the options would otherwise show the chooser
/// (§9.1: a library's order is hidden from both players). The field's are the side's unit piles' tops,
/// lane 1 upward, then its backrow (R13: a dormant card is not on the field).
fn pile_cards(state: &GameState, player: PlayerId, zone: PileZone) -> Vec<&CardInstance> {
    let side = &state.players[player];
    let pile = match zone {
        PileZone::Field => {
            let mut out: Vec<&CardInstance> = active_units_of(state, player);
            for slot in slots_of(player, Row::Backrow) {
                if let Some(card) = card_at(state, slot) {
                    out.push(card);
                }
            }
            return out;
        }
        PileZone::Graveyard => &side.graveyard,
        PileZone::Exile => &side.exile,
        PileZone::Hand => &side.hand,
        PileZone::Library => &side.library,
    };
    let mut cards: Vec<&CardInstance> = pile.iter().collect();
    if zone == PileZone::Library {
        cards.sort_by(|a, b| creation_number(&a.id).total_cmp(&creation_number(&b.id)));
    }
    cards
}

/// Which cards of those piles a `pick` prompt offers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PickFilter {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<OneOrMany<CardType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_range: Option<CostRange>,
}

fn pickable(state: &GameState, card: &CardInstance, filter: &PickFilter) -> bool {
    if let Some(types) = &filter.type_
        && !types.includes(&card_type_of(state, card))
    {
        return false;
    }
    if let Some(tags) = &filter.tags {
        let printed = &def_of(Some(state), &card.def_id).tags;
        if !tags.iter().all(|tag| printed.contains(tag)) {
            return false;
        }
    }
    let cost = effective_cost(state, card, Default::default());
    let range = filter.cost_range.unwrap_or_default();
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
    true
}

/// `choose_pick`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChoosePickArgs {
    pub step: String,
    pub from: Vec<PileSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<PickFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    pub max: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
}

/// B5 E18: a pick of one card or several from a pile, or from cards across zones, budgeted by count
/// (`max`) or by cost (`budget`) — Classic #11 Mind Melt's "a card of your opponent's hand" (one
/// card), #44 Back from the GY's "Units with a total cost of (5) or less", #56's "up to 3 Spells",
/// #78's Radiant "a card of yours of its type on your field, in your hand or in your deck". Every
/// matching card of the piles is an option — no Discover limit of three — carrying its cost as R65
/// reads it where it lies (`effective_cost`: a hand card at its hand cost, any other at its own), and
/// with `budget` the picks may cost no more than it together (`prompts::why_answer_refused`;
/// `prompt_answers` lists only sets that fit). `min` and `max` are picks, clamped to what the piles
/// hold; `min` defaults to 0 ("up to"). The picks arrive in `ctx.targets` in offered order. The options
/// go to the chooser alone (R81, §10.8), which is what makes the other player's hand or a library
/// safe to offer: the other seat sees that a prompt is open, and a library's order never shows
/// (`pile_cards`). A face-down card of the other player's is offered as its zone only (R177, `view_for`).
/// No matching card asks nothing.
pub fn choose_pick(args: ChoosePickArgs) -> Effect {
    Effect::new("choosePick", move |ctx| {
        let filter = args.filter.clone().unwrap_or_default();
        let mut seen: IndexSet<String> = IndexSet::new();
        let mut options: Vec<PromptOption> = Vec::new();
        let reader: &EffectContext<'_> = &*ctx;
        let state: &GameState = &*reader.state;
        for pile in &args.from {
            let player = player_of(reader, pile.player.unwrap_or(PlayerSpec::SelfSide));
            for card in pile_cards(state, player, pile.zone) {
                if seen.contains(&card.id) || !pickable(state, card, &filter) {
                    continue;
                }
                seen.insert(card.id.clone());
                options.push(PromptOption {
                    key: format!("instance:{}", card.id),
                    label: def_of(Some(state), &card.def_id).name.clone(),
                    selection: Selection::Instance {
                        instance_id: card.id.clone(),
                    },
                    cost: Some(effective_cost(state, card, Default::default())),
                    radiant: if card.radiant { Some(true) } else { None },
                });
            }
        }
        if options.is_empty() {
            return;
        }
        let max = args.max.min(options.len() as i32).max(0);
        let min = args.min.unwrap_or(0).max(0).min(max);
        let player = ctx.controller;
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        let mut asked = ask(
            player,
            PromptKind::Pick,
            args.prompt.clone().unwrap_or_else(|| "Choose cards".to_string()),
            options,
            resume,
        );
        asked.min = Some(min);
        asked.max = Some(max);
        asked.budget = args.budget;
        open_prompt(ctx, asked);
    })
}

/// What a Discover's options are (R247). `card`, the default, offers the definitions themselves: each
/// option is a catalog id, labelled with the card's name, and the view names the card behind it
/// (§10.8). `index` offers the definitions' §5 indices instead — #82 KY's Trial's "Discover among 3
/// distinct random numbers" — so each option is the number, labelled with it and naming no
/// definition, and the resume step turns the number it gets back into its card (`def_by_index`, with
/// the set its pool named, since a number is unique only within its set).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum DiscoverOffer {
    Card,
    Index,
}

/// `discover_from_catalog`'s pool as a function of the context (TS's `query` when it is a function).
pub type CatalogQueryFn = Arc<dyn Fn(&mut EffectContext<'_>) -> CatalogQueryArgs + Send + Sync>;

/// `discover_from_catalog`'s arguments (TS's inline object). TS's `query` is either a query or a
/// function of the context; here the function is `query_fn`, a callback serde skips, which wins over
/// `query` when it is set.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverFromCatalogArgs {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<CatalogQueryArgs>,
    #[serde(skip)]
    pub query_fn: Option<CatalogQueryFn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
    /// R247: what each option is, the card or its number. Default `card`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offer: Option<DiscoverOffer>,
}

impl std::fmt::Debug for DiscoverFromCatalogArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoverFromCatalogArgs")
            .field("step", &self.step)
            .field("query", &self.query)
            .field("query_fn", &self.query_fn.is_some())
            .field("count", &self.count)
            .field("prompt", &self.prompt)
            .field("data", &self.data)
            .field("offer", &self.offer)
            .finish()
    }
}

/// §6.3 Discover: choose 1 of 3, drawn without replacement from the stated pool and shown only to
/// the chooser. The options are definitions, so the resume step decides what to do with the pick —
/// or, with `offer: "index"`, their numbers (R247).
///
/// `query` may be a function of the context, read when the effect applies rather than when the hook
/// builds its list: a hook is rebuilt each time a paused list resumes (`prompts::run_resume`), and a
/// pool that costs something to build — #97 Zephyrs' scorer plays every candidate (§10.7) — is then
/// built once, for the Discover that uses it, and not again for the effects after it.
pub fn discover_from_catalog(args: DiscoverFromCatalogArgs) -> Effect {
    Effect::new("discoverFromCatalog", move |ctx| {
        let asked: CatalogQueryArgs = match &args.query_fn {
            Some(of_context) => of_context(ctx),
            None => args.query.clone().unwrap_or_default(),
        };
        // §5.1: a random pool never offers the card that generated it.
        let generating = ctx
            .self_
            .as_ref()
            .map(|card| card.def_id.clone())
            .or_else(|| ctx.def_id.clone());
        let pool = query(&excluding_def_id(Some(&*ctx.state), &asked, generating.as_deref()));
        if pool.is_empty() {
            return;
        }

        let shuffled = ctx.rng.shuffle(&pool);
        let end = slice_end(shuffled.len(), args.count.unwrap_or(3));
        let by_index = args.offer == Some(DiscoverOffer::Index);
        let options: Vec<PromptOption> = shuffled[..end]
            .iter()
            .map(|def| {
                // R247: a number is offered as itself, so nothing in the option names the card it stands for.
                let option = if by_index { def.index.clone() } else { def.id.clone() };
                PromptOption {
                    key: format!("mode:{option}"),
                    label: if by_index { def.index.clone() } else { def.name.clone() },
                    selection: Selection::Mode { option },
                    cost: None,
                    radiant: None,
                }
            })
            .collect();
        let player = ctx.controller;
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        open_prompt(
            ctx,
            ask(
                player,
                PromptKind::Discover,
                args.prompt.clone().unwrap_or_else(|| "Discover a card".to_string()),
                options,
                resume,
            ),
        );
    })
}

/// Which library cards a `discover_from_library` may reveal. It is not a `CatalogQueryArgs`: the pool
/// is a pile of instances rather than the catalog, so only the filters #51 KY's Private Tutor names
/// are here, and a card that needs tags or rarity out of a library should widen this rather than be
/// routed through `catalog::query`, which would offer cards the library does not hold.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFilter {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<OneOrMany<CardType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_range: Option<CostRange>,
}

/// #51's engine cell: "Field Trap counts as Trap". A `type` filter matches the field exactly, so
/// asking for "Trap" has to name both fields or #18 Bread and Butter and #71 Intern Stimmy silently
/// vanish from the pool. The reverse does not hold: asking for "Field Trap" means Field Traps only.
const TRAP_TYPES: &[CardType] = &[CardType::Trap, CardType::FieldTrap];

fn filter_types(filter: &LibraryFilter) -> Option<Vec<CardType>> {
    let asked: &[CardType] = match &filter.type_ {
        None => &[],
        Some(types) => types.as_slice(),
    };
    if asked.is_empty() {
        return None;
    }
    Some(
        asked
            .iter()
            .flat_map(|card_type| {
                if *card_type == CardType::Trap {
                    TRAP_TYPES.to_vec()
                } else {
                    vec![*card_type]
                }
            })
            .collect(),
    )
}

/// R65: a library card's cost is R65's one calculation for that instance (`effective_cost`), which is
/// what #30 Archivist and #94 Genn's Greed read (R24, R66): a card never played has no X (so an X-cost
/// card reads 0) and no embiggen price (its base), and its `costMod` and `costOverride` travel with
/// it into every zone (R78), so #95's "every card in your library costs 2 less" moves its bracket.
pub fn matches_library_filter(state: &GameState, card: &CardInstance, filter: &LibraryFilter) -> bool {
    // R218: a unit-token card leaves a library only by being drawn or played (R11), so a reveal that
    // puts the pick in a hand passes over it, as a Recruit does.
    if is_unit_token(state, card) {
        return false;
    }
    if let Some(types) = filter_types(filter)
        && !types.contains(&card_type_of(state, card))
    {
        return false;
    }

    let cost = effective_cost(state, card, Default::default());
    let range = filter.cost_range.unwrap_or_default();
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
    true
}

/// `discover_from_library`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverFromLibraryArgs {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<LibraryFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
}

/// §6.3's Discover row is explicit that this is the same primitive with the library as the pool:
/// "'Reveal N matching cards, then choose one' (KY's Private Tutor) is this same primitive with the
/// library as the pool: the revealed cards are that prompt's options, so only the chooser ever sees
/// them (§10.8)". So this is `discover_from_graveyard` over a filtered library: `count` options drawn
/// without replacement with `ctx.rng.shuffle`, so the revealed cards are always different (R60).
///
/// §10.8 is what makes revealing safe: "a card revealed out of a library is revealed only as an
/// option of the prompt that reveals it: the chooser sees it in full, the opponent sees only that a
/// prompt is open, and the rest of the library stays hidden from both." Nothing is copied out of
/// `state.pending.options`, so `view_for` has one place to hide. The prompt therefore goes to
/// `ctx.controller` — the chooser — even when `player` names the other side's library as the pool.
///
/// The options are real library INSTANCES, not definitions, which is the whole difference from
/// `discover_from_catalog`: the resume step moves the pick with `add_to_hand({ instance: { of: "chosen" }
/// })` rather than creating a copy and leaving the revealed card in the library.
///
/// No match at all opens no prompt: the effect fizzles and the card still resolves (§6.3), which is
/// the branch #51 answers with its Empty Notebook.
pub fn discover_from_library(args: DiscoverFromLibraryArgs) -> Effect {
    Effect::new("discoverFromLibrary", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let filter = args.filter.clone().unwrap_or_default();
        let state: &GameState = &*ctx.state;
        let pool: Vec<CardInstance> = state.players[player]
            .library
            .iter()
            .filter(|card| matches_library_filter(state, card, &filter))
            .cloned()
            .collect();
        if pool.is_empty() {
            return;
        }

        let shuffled = ctx.rng.shuffle(&pool);
        let end = slice_end(shuffled.len(), args.count.unwrap_or(3));
        let options: Vec<PromptOption> = shuffled[..end]
            .iter()
            .map(|card| PromptOption {
                key: format!("instance:{}", card.id),
                label: def_of(Some(&*ctx.state), &card.def_id).name.clone(),
                selection: Selection::Instance {
                    instance_id: card.id.clone(),
                },
                cost: None,
                radiant: None,
            })
            .collect();
        let chooser = ctx.controller;
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        open_prompt(
            ctx,
            ask(
                chooser,
                PromptKind::Discover,
                args.prompt
                    .clone()
                    .unwrap_or_else(|| "Choose one of the revealed cards".to_string()),
                options,
                resume,
            ),
        );
    })
}

/// `discover_from_graveyard`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverFromGraveyardArgs {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: StepData,
}

/// R50: Discover from the actual graveyard, so spell tokens there are eligible.
pub fn discover_from_graveyard(args: DiscoverFromGraveyardArgs) -> Effect {
    Effect::new("discoverFromGraveyard", move |ctx| {
        let controller = ctx.controller;
        let graveyard: Vec<CardInstance> = ctx.state.players[controller].graveyard.clone();
        if graveyard.is_empty() {
            return;
        }

        let shuffled = ctx.rng.shuffle(&graveyard);
        let end = slice_end(shuffled.len(), args.count.unwrap_or(3));
        let options: Vec<PromptOption> = shuffled[..end]
            .iter()
            .map(|card| PromptOption {
                key: format!("instance:{}", card.id),
                label: def_of(Some(&*ctx.state), &card.def_id).name.clone(),
                selection: Selection::Instance {
                    instance_id: card.id.clone(),
                },
                cost: None,
                radiant: None,
            })
            .collect();
        let resume = resume_self(ctx, &args.step, args.data.clone().unwrap_or_default());
        open_prompt(
            ctx,
            ask(
                controller,
                PromptKind::Discover,
                args.prompt
                    .clone()
                    .unwrap_or_else(|| "Discover a card from your graveyard".to_string()),
                options,
                resume,
            ),
        );
    })
}
