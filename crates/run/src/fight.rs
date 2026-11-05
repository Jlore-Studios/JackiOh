//! DR14 part B: the run's content from the catalog, a fight's game setup, and its report. The run
//! still plays no game itself: the caller starts one from `run_fight_setup` and sends the
//! `RunAction::FightOver` that `run_fight_report` folds from the finished game.

use jackioh_engine::catalog::{deckable, registered_catalog};
use jackioh_engine::combat::attack_target_of;
use jackioh_engine::config::{AI_DIFFICULTY, HUMAN_HANDICAP, Handicap};
use jackioh_engine::damage::DamageTarget;
use jackioh_engine::reduce::{begin_game, reduce};
use jackioh_engine::replay::ReplayInput;
use jackioh_engine::state::{
    CreateGameOptions, GameState, SetupKind, create_recorded_game, starting_hero_health, validate_setup,
};
use jackioh_engine::wire::{CardDefs, DeckMark, GameEvent, PlayerId, Winner};
use serde::{Deserialize, Serialize};

use crate::config::{RUN_LOOMING_PRESENCE, RUN_OVERWHELMING_PRESENCE, RUN_STARTER_CARDS};
use crate::{
    RunAction, RunContent, RunEnemy, RunFightEffect, RunPassiveKind, RunPoolCard, RunState, passive_def,
    run_enemy, run_placeholder_content,
};

/// What the caller hands `createGame` for the fight being played: the human is P1, the enemy P2.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RunFightSetup {
    /// The game's seed, `"{run seed}-{row}-{col}"`: one run meets one deal at each node (DR2).
    pub seed: String,
    /// The run deck's ids, in the run's order, which `human_marks` index.
    pub human_deck: Vec<String>,
    /// `HUMAN_HANDICAP`, its `deck_size` the deck's length and its `hero_health` the run's HP (DR5),
    /// then each Combat artifact held, once per level (R2922001).
    pub human_handicap: Handicap,
    /// Each Radiant card of the run deck, and each buffed one with its buffs as upgrades.
    pub human_marks: Vec<DeckMark>,
    pub enemy_deck: Vec<String>,
    /// Easy's handicap, Medium's with Looming Presence, Hard's with Overwhelming Presence (DR5, R180),
    /// its `deck_size` the enemy deck's length and its `hero_health` the enemy's, then each curse taken
    /// (R2922001).
    pub enemy_handicap: Handicap,
    pub enemy: RunEnemy,
}

/// DR14: the content a run on the cards that ship deals from. The pool is every card a deck may hold
/// (`deckable`: no token, a set that ships, no card only a rule makes), with its rarity, set, type and
/// tags. The starting cards are `RUN_STARTER_CARDS`. The enemies, passives, events and curses are
/// `run_placeholder_content`'s until the designer's lists land, each enemy's own cards kept only where
/// the pool holds them, so no pseudo boss card is dealt while the Boss set has none.
pub fn run_catalog_content(catalog: &CardDefs) -> RunContent {
    let pool: Vec<RunPoolCard> = catalog
        .values()
        .filter(|def| deckable(def))
        .filter_map(|def| {
            Some(RunPoolCard {
                id: def.id.clone(),
                name: def.name.clone(),
                // A token's is the only rarity no card prints, and `deckable` has left out the tokens.
                rarity: def.rarity.as_str().parse().ok()?,
                set: def.set,
                card_type: def.type_,
                tags: def.tags.iter().map(|tag| tag.as_str().to_string()).collect(),
            })
        })
        .collect();
    let placeholder = run_placeholder_content();
    let mut enemies = placeholder.enemies;
    for enemy in &mut enemies {
        enemy.deck.retain(|id| pool.iter().any(|card| &card.id == id));
    }
    RunContent {
        pool,
        starter_access: RUN_STARTER_CARDS.iter().map(|id| (*id).to_string()).collect(),
        enemies,
        passives: placeholder.passives,
        events: placeholder.events,
        curses: placeholder.curses,
    }
}

/// A deck's length as a handicap's `deck_size`.
fn deck_size(deck: &[String]) -> i32 {
    i32::try_from(deck.len()).unwrap_or(i32::MAX)
}

/// R2922001: one fight effect, `times` over, on the two seats' handicaps. A negative amount counts as
/// none, so every setup stays one the engine takes.
fn apply_fight_effect(effect: RunFightEffect, times: i32, human: &mut Handicap, enemy: &mut Handicap) {
    let grow = |n: i32| n.max(0).saturating_mul(times.max(0));
    match effect {
        RunFightEffect::EnemyHealth { n } => {
            enemy.hero_health = Some(starting_hero_health(Some(enemy)).saturating_add(grow(n)));
        }
        RunFightEffect::EnemyOpeningCards { n } => {
            enemy.extra_opening_cards = enemy.extra_opening_cards.saturating_add(grow(n));
        }
        RunFightEffect::EnemyMana { n } => {
            enemy.mana_bonus = enemy.mana_bonus.saturating_add(grow(n));
            enemy.mana_cap = enemy.mana_cap.saturating_add(grow(n));
        }
        RunFightEffect::OwnOpeningCards { n } => {
            human.extra_opening_cards = human.extra_opening_cards.saturating_add(grow(n));
        }
        RunFightEffect::OwnArmor { n } => {
            let armor = human.hero_armor.unwrap_or(0).saturating_add(grow(n));
            human.hero_armor = (armor > 0).then_some(armor);
        }
    }
}

/// DR5, DR14: the setup of the fight being played, `None` outside `RunPhase::Fight`. R2922001: after
/// Presence sets the enemy's tier, each curse taken strengthens the enemy once and each Combat artifact
/// held helps the human once per level.
pub fn run_fight_setup(state: &RunState, content: &RunContent) -> Option<RunFightSetup> {
    let enemy = run_enemy(state)?;
    let row = state.row?;
    let human_deck: Vec<String> = state.deck.iter().map(|card| card.id.clone()).collect();
    let human_marks = state
        .deck
        .iter()
        .enumerate()
        .filter(|(_, card)| card.radiant || card.buffs > 0)
        .map(|(index, card)| DeckMark {
            index,
            radiant: card.radiant,
            upgrades: card.buffs,
        })
        .collect();
    let carries = |id: &str| enemy.artifacts.iter().any(|a| a == id);
    let tier = if carries(RUN_OVERWHELMING_PRESENCE) {
        AI_DIFFICULTY.hard
    } else if carries(RUN_LOOMING_PRESENCE) {
        AI_DIFFICULTY.medium
    } else {
        AI_DIFFICULTY.easy
    };
    let mut human_handicap = Handicap {
        deck_size: deck_size(&human_deck),
        hero_health: Some(state.hp),
        ..HUMAN_HANDICAP
    };
    let mut enemy_handicap = Handicap {
        deck_size: deck_size(&enemy.deck),
        hero_health: Some(enemy.hero_health),
        ..tier
    };
    // R2922001: after Presence, each curse taken once and each Combat artifact held once per level.
    let curses = state
        .curses
        .iter()
        .filter_map(|id| content.curses.iter().find(|c| &c.id == id))
        .filter_map(|curse| curse.fight_effect.map(|effect| (effect, 1)));
    let artifacts = state.artifacts.iter().filter_map(|held| {
        passive_def(content, &held.id)
            .filter(|def| def.kind == RunPassiveKind::Combat)
            .and_then(|def| def.fight_effect)
            .map(|effect| (effect, held.level))
    });
    for (effect, times) in curses.chain(artifacts) {
        apply_fight_effect(effect, times, &mut human_handicap, &mut enemy_handicap);
    }
    Some(RunFightSetup {
        seed: format!("{}-{row}-{}", state.seed, state.col),
        human_handicap,
        human_deck,
        human_marks,
        enemy_deck: enemy.deck.clone(),
        enemy_handicap,
        enemy: enemy.clone(),
    })
}

/// The seat whose side a damaged or healed target (a hero or a unit) is on: the unit's controller as
/// the step began, or, for a unit the step brought in, as it ended.
fn side_of(target_id: &str, before: &GameState, after: &GameState) -> Option<PlayerId> {
    match attack_target_of(before, target_id).or_else(|| attack_target_of(after, target_id))? {
        DamageTarget::Hero { player } => Some(player),
        DamageTarget::Unit { instance } => Some(instance.controller),
    }
}

/// What a fight's report sums over its events.
#[derive(Default)]
struct Tally {
    damage_dealt: i32,
    healing_done: i32,
    cards_drawn: i32,
}

impl Tally {
    /// One step's events, `before` and `after` the states around them.
    fn read(&mut self, human: PlayerId, before: &GameState, events: &[GameEvent], after: &GameState) {
        for event in events {
            match event {
                GameEvent::Damage {
                    target_id, amount, ..
                } if side_of(target_id, before, after) == Some(human.opponent()) => {
                    self.damage_dealt += amount;
                }
                GameEvent::Healed {
                    target_id, amount, ..
                } if side_of(target_id, before, after) == Some(human) => {
                    self.healing_done += amount;
                }
                // A turn's draw or a card's: the opening deal has no `turnDraw`, and a fatigue draw
                // draws no card.
                GameEvent::Drawn {
                    player,
                    turn_draw: Some(_),
                    ..
                } if *player == human => self.cards_drawn += 1,
                _ => {}
            }
        }
    }
}

/// DR14: the `RunAction::FightOver` a finished fight sends, folded from its record as `summarize_game`
/// folds one (an action the engine refuses is skipped). `won` is `human`'s win; `hp` its hero's health
/// at the end; `damage_dealt` every `damage` event's amount on a target the enemy controls, its hero
/// or a unit; `healing_done` every `healed` event's on `human`'s side; `cards_drawn` `human`'s draws
/// after the opening deal; `turns` the game's. A setup the engine refuses, or a game with no result,
/// is refused, and so is a game whose decks name a custom card: a run plays none.
pub fn run_fight_report(game: &ReplayInput, human: PlayerId) -> Result<RunAction, String> {
    // The fold's JSON is `createGame`'s plus its `log` (as the browser's `fold` reads it), so every
    // setup field the record carries, the deck marks among them, reaches the game unnamed here.
    let options: CreateGameOptions = serde_json::to_value(game)
        .and_then(serde_json::from_value)
        .map_err(|e| e.to_string())?;
    let catalog: &CardDefs = match &options.catalog {
        Some(catalog) => catalog,
        None => registered_catalog(),
    };
    // `create_recorded_game` panics on a setup it refuses: refuse it here instead. A run gives the
    // game no custom card, so a deck that names one is refused as a card the game was not given.
    validate_setup(&options, catalog, &CardDefs::new(), SetupKind::Recorded).map_err(|e| e.to_string())?;

    let mut tally = Tally::default();
    let start = create_recorded_game(&options);
    let begun = begin_game(&start);
    tally.read(human, &start, &begun.events, &begun.state);
    let mut state = begun.state;
    for action in &game.log {
        let result = reduce(&state, action);
        if result.error.is_some() {
            continue;
        }
        tally.read(human, &state, &result.events, &result.state);
        state = result.state;
    }

    let outcome = state
        .result
        .ok_or("the game is not over: only a finished fight reports")?;
    Ok(RunAction::FightOver {
        won: outcome.winner == Winner::from(human),
        hp: state.players[human].hero.health,
        damage_dealt: tally.damage_dealt,
        healing_done: tally.healing_done,
        cards_drawn: tally.cards_drawn,
        turns: state.turn,
    })
}
