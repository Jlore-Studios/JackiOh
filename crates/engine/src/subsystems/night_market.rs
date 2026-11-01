//! Meditative #42 CN Flea Market's night market (ME-MARKET, docs/meditative-set.md M6 #42, SPEC §8.8
//! row 42, R1000–R1002): the stall, the yuan prices, the `market` prompt and its deals, each step a
//! plain-data continuation, so a market paused at its prompt survives JSON and replays (§9.3).
//!
//! The stall is rolled once from the match rng as the Spell resolves (R1000), and nothing else in a
//! market draws from it, so every simulation (R185) and replay sees the same lots. Each answer is one
//! deal (R113): a lot bought on either face (R831002) arrives in the hand, hidden per zone (R97); on the
//! Radiant face a barter exiles a hand card, in public, for its price (R1001); Leave loses the yuan left.
//! Only lots the yuan covers are offered, none while the hand is full, so neither `legal_actions` nor a
//! random answer (R452) names one the reducer refuses. The stall and yuan ride in the resume data,
//! which no view sends (§10.8). A timeout leaves (R1002).

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::catalog::{CatalogQueryArgs, excluding_def_id, find_def, query, query_cost};
use crate::config::{YUAN_PER_COST, YUAN_PER_RARITY, YUAN_RADIANT_MULTIPLIER, YUAN_RARITY_RANKS};
use crate::params::param;
use crate::prelude::json_as;
use crate::prompts::{OpenPromptArgs, open_prompt, resume_self};
use crate::script::{Effect, Hook, hook};
use crate::state::{GameState, PromptOption};
use crate::wire::{ActionBody, CardDef, PlayerId, PromptKind, Rarity, Selection};

/// Resume-data key: the stall's ids still for sale, a copy per lot, in the order they were rolled.
pub const MARKET_STALL_KEY: &str = "marketStall";
/// Resume-data key: the yuan left.
pub const MARKET_YUAN_KEY: &str = "marketYuan";
/// Resume-data key: whether this market barters (the Radiant face, R1001).
pub const MARKET_BARTER_KEY: &str = "marketBarter";
/// The step every answer re-enters.
pub const MARKET_DEAL_STEP: &str = "deal";
/// The card's declared number: the yuan it opens with (50, Radiant 80; R386 tunes it).
pub const MARKET_YUAN_PARAM: &str = "yuan";
/// Leave's option key, a `none` pick as `choose_cell`'s Done is.
pub const MARKET_LEAVE_KEY: &str = "none";
/// R831002: the suffix a lot's option carries when it sells the lot's Radiant face.
pub const MARKET_RADIANT_SUFFIX: &str = ":radiant";

/// One shelf of the stall: a random draw from a catalog pool, or copies of one named card.
#[derive(Clone, Debug)]
pub enum MarketShelf {
    /// `count` different cards drawn from `query` (R60), the running card left out (R387).
    Pool {
        query: Box<CatalogQueryArgs>,
        count: i32,
    },
    /// `count` copies of `def_id` (a named card: no pool, no R387).
    Named { def_id: String, count: i32 },
}

/// `night_market_script`'s answer: the card's Cry and the step its prompt re-enters.
#[derive(Clone)]
pub struct NightMarketScript {
    pub cry: Hook,
    pub resume: IndexMap<&'static str, Hook>,
}

/// R1000: a card's price in yuan, by its printed cost out of play (R65's `query_cost`: X counts 0) and
/// its rarity's rank (a token's printed rarity, else Common), times `YUAN_RADIANT_MULTIPLIER` for a
/// Radiant card (R1001's barters read the same rule).
pub fn yuan_price(def: &CardDef, radiant: bool) -> i32 {
    let rarity = if def.rarity == Rarity::Token {
        def.printed_rarity
            .and_then(|printed| printed.as_str().parse::<Rarity>().ok())
            .unwrap_or(Rarity::Common)
    } else {
        def.rarity
    };
    let rank = YUAN_RARITY_RANKS
        .iter()
        .find(|(listed, _)| *listed == rarity)
        .map_or(1, |(_, rank)| *rank);
    let price = YUAN_PER_COST * query_cost(def) + YUAN_PER_RARITY * rank;
    if radiant {
        price * YUAN_RADIANT_MULTIPLIER
    } else {
        price
    }
}

/// Whether `player`'s hand has room for one more card (§2.4, R1143's hand size).
fn hand_has_room(state: &GameState, player: PlayerId) -> bool {
    crate::query::room_in(state, player, crate::zones::OffFieldZone::Hand) > 0
}

/// R1000, R1001, R831002: the market's options for `player`: each lot's base then Radiant face
/// (distinct ids in rolled order), then on a barter market one per hand card (its price negative, as it
/// adds to the yuan), then Leave. With nothing to do, no option at all.
fn market_options(
    state: &GameState,
    player: PlayerId,
    stall: &[String],
    yuan: i32,
    barter: bool,
) -> Vec<PromptOption> {
    let mut options: Vec<PromptOption> = Vec::new();
    if hand_has_room(state, player) {
        let mut seen: Vec<&str> = Vec::new();
        for id in stall {
            if seen.contains(&id.as_str()) {
                continue;
            }
            seen.push(id);
            let Some(def) = find_def(Some(state), id) else {
                continue;
            };
            let copies = stall.iter().filter(|other| *other == id).count();
            let label = if copies > 1 {
                format!("{} ×{copies}", def.name)
            } else {
                def.name.clone()
            };
            for radiant in [false, true] {
                let price = yuan_price(def, radiant);
                if price > yuan {
                    continue;
                }
                let option = if radiant {
                    format!("{id}{MARKET_RADIANT_SUFFIX}")
                } else {
                    id.clone()
                };
                options.push(PromptOption {
                    key: format!("mode:{option}"),
                    label: label.clone(),
                    selection: Selection::Mode { option },
                    cost: Some(price),
                    radiant: radiant.then_some(true),
                });
            }
        }
    }
    if barter {
        for card in &state.players[player].hand {
            let Some(def) = find_def(Some(state), &card.def_id) else {
                continue;
            };
            options.push(PromptOption {
                key: format!("instance:{}", card.id),
                label: def.name.clone(),
                selection: Selection::Instance {
                    instance_id: card.id.clone(),
                },
                cost: Some(-yuan_price(def, card.radiant)),
                radiant: card.radiant.then_some(true),
            });
        }
    }
    if options.is_empty() {
        return options;
    }
    options.push(PromptOption {
        key: MARKET_LEAVE_KEY.to_string(),
        label: "Leave".to_string(),
        selection: Selection::None,
        cost: None,
        radiant: None,
    });
    options
}

/// R1000, R113: open (or reopen) the market for the running card's controller, its stall, yuan and
/// barter flag in the resume data; with nothing to do, it closes instead.
fn open_market(stall: Vec<String>, yuan: i32, barter: bool) -> Effect {
    Effect::new("nightMarket", move |ctx| {
        let player = ctx.controller;
        let options = market_options(ctx.state, player, &stall, yuan, barter);
        if options.is_empty() {
            return;
        }
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert(MARKET_STALL_KEY.to_string(), json!(stall));
        data.insert(MARKET_YUAN_KEY.to_string(), json!(yuan));
        data.insert(MARKET_BARTER_KEY.to_string(), json!(barter));
        let resume = resume_self(ctx, MARKET_DEAL_STEP, data);
        open_prompt(
            ctx,
            OpenPromptArgs {
                player,
                kind: PromptKind::Market,
                aim: None,
                prompt: format!("Night market: {yuan} yuan"),
                options,
                min: Some(1),
                max: Some(1),
                budget: Some(yuan),
                owner: None,
                resume,
            },
        );
    })
}

/// The whole market: the Cry that rolls the stall from `shelves` and opens it, and the deal step.
pub fn night_market_script(shelves: &[MarketShelf]) -> NightMarketScript {
    let shelves: Arc<Vec<MarketShelf>> = Arc::new(shelves.to_vec());

    let roll = Effect::new("nightMarketRoll", move |ctx| {
        let own: Option<String> = ctx
            .self_
            .as_ref()
            .map(|card| card.def_id.clone())
            .or_else(|| ctx.def_id.clone());
        let mut stall: Vec<String> = Vec::new();
        for shelf in shelves.iter() {
            match shelf {
                MarketShelf::Pool { query: args, count } => {
                    let pool: Vec<String> = query(&excluding_def_id(Some(&*ctx.state), args, own.as_deref()))
                        .into_iter()
                        .map(|def| def.id.clone())
                        .collect();
                    // R129: an empty pool draws nothing.
                    if pool.is_empty() {
                        continue;
                    }
                    let drawn = ctx.sink.rng.shuffle(&pool);
                    stall.extend(drawn.into_iter().take((*count).max(0) as usize));
                }
                MarketShelf::Named { def_id, count } => {
                    if find_def(Some(&*ctx.state), def_id).is_some() {
                        stall.extend((0..*count).map(|_| def_id.clone()));
                    }
                }
            }
        }
        let yuan = param(&*ctx, MARKET_YUAN_PARAM);
        let barter = ctx.radiant;
        (open_market(stall, yuan, barter).apply)(ctx);
    });

    let deal: Hook = hook(|ctx| {
        let stall: Vec<String> = ctx
            .data
            .get(MARKET_STALL_KEY)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default();
        let yuan = ctx
            .data
            .get(MARKET_YUAN_KEY)
            .and_then(Value::as_i64)
            .map_or(0, |yuan| yuan as i32);
        let barter = ctx
            .data
            .get(MARKET_BARTER_KEY)
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let player = ctx.controller;
        match ctx.targets.first() {
            Some(Selection::Mode { option }) => {
                // R831002: the lot's Radiant face, or its base face.
                let (lot, radiant) = match option.strip_suffix(MARKET_RADIANT_SUFFIX) {
                    Some(lot) => (lot, true),
                    None => (option.as_str(), false),
                };
                let Some(at) = stall.iter().position(|id| id == lot) else {
                    return vec![];
                };
                let Some(price) = find_def(Some(&*ctx.state), lot).map(|def| yuan_price(def, radiant)) else {
                    return vec![];
                };
                if price > yuan || !hand_has_room(ctx.state, player) {
                    return vec![];
                }
                let mut rest = stall.clone();
                rest.remove(at);
                vec![
                    crate::effects::add_to_hand(json_as(json!({ "defId": lot, "radiant": radiant }))),
                    open_market(rest, yuan - price, barter),
                ]
            }
            Some(Selection::Instance { instance_id }) if barter => {
                let Some(card) = ctx.state.players[player]
                    .hand
                    .iter()
                    .find(|card| card.id == *instance_id)
                else {
                    return vec![];
                };
                let price =
                    find_def(Some(&*ctx.state), &card.def_id).map_or(0, |def| yuan_price(def, card.radiant));
                vec![
                    crate::effects::exile(json_as(json!({
                        "target": { "of": "instance", "instanceId": instance_id },
                    }))),
                    open_market(stall, yuan + price, barter),
                ]
            }
            // Leave, or anything the market no longer holds: the market closes.
            _ => vec![],
        }
    });

    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(MARKET_DEAL_STEP, deal);
    NightMarketScript {
        cry: hook(move |_ctx| vec![roll.clone()]),
        resume,
    }
}

/// R1002: a timeout's answer to an open night market, Leave; `None` for any other prompt.
pub fn leave_answer(state: &GameState) -> Option<ActionBody> {
    let pending = state.pending.as_ref()?;
    if pending.kind != PromptKind::Market {
        return None;
    }
    pending
        .options
        .iter()
        .any(|option| option.key == MARKET_LEAVE_KEY)
        .then(|| ActionBody::Answer {
            choice_id: pending.id.clone(),
            selection: vec![Selection::None],
        })
}
