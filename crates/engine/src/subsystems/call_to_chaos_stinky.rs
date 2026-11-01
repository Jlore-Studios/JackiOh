//! Stinkyscape #150 Call to Chaos (Stinky Edition) (SPEC §8.10 row 150, R28, R87, R423, R436,
//! R1882001–R1882004): its table of ten effects, which Core #95's subsystem (`call_to_chaos.rs`) rolls,
//! announces and resolves — `call_to_chaos({ radiant, table: CHAOS_STINKY_EFFECTS })` — as it does
//! Classic+ #73's and Meditative #95's: the base face rolls one entry, the Radiant three different ones
//! resolved in the list's order (R423), the recursion counts casts of every edition against
//! `CALL_TO_CHAOS_CHAIN_CAP` (R28), and both players are told what was rolled (`chaosRolled`, R436).
//!
//! Every entry is built when it resolves (`lazy_part`), as the other editions' are (R87, R113).
//!
//! Entry 8 is the engine's own rest-of-game rule (R1882004): a death watcher under
//! `CHAOS_DEATH_WATCH_HOOK`, whose step `work::script_step_for` answers with `death_watch_step` whatever
//! card made it, so the table never leans on a card file; a player holds at most
//! `CHAOS_STINKY_DEATH_WATCH_CAP`, and the watch casts at most `CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN`
//! for a player in a turn, its terminator (R2176001).

use serde_json::json;

use crate::catalog::{def_id_by_index, query, query_cost};
use crate::combat::{SwitchPositionOptions, switch_position};
use crate::config::{
    CHAOS_STINKY_DEATH_WATCH_CAP, CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN, CHAOS_STINKY_FIELD_SPELLS,
    CHAOS_STINKYSCAPE_COST,
};
use crate::effects::add_to_hand::AddToHandArgs;
use crate::effects::delay::delayed_resume;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, EffectPart, Hook, hook};
use crate::state::{CardInstance, GameState, ModifierKind};
use crate::subsystems::call_to_chaos::{ChaosEffectDef, cast_random_call_to_chaos};
use crate::wire::{CATALOG_SETS, PlayerId, Position, SetName, Tag, opponent_of};

/// §8.10: the set whose indices the table names (B2.2).
const SET: SetName = SetName::Stinkyscape;
/// Entry 1: S #159 ETHICS and S #95 Vanguard Coalition.
const ETHICS_INDEX: &str = "159";
const VANGUARD_INDEX: &str = "95";
/// Entry 2: S #113 A Very Dandy Lad and S #114 Arctic Citadel.
const DANDY_LAD_INDEX: &str = "113";
const ARCTIC_CITADEL_INDEX: &str = "114";
/// Entry 9: S #150.1 Stinkyscape Golem.
const GOLEM_INDEX: &str = "150.1";

/// R1882004: the hook of entry 8's death watcher, which the engine resolves itself.
pub const CHAOS_DEATH_WATCH_HOOK: &str = "@chaosDeathWatch";
/// Entry 8's step name, under `CHAOS_DEATH_WATCH_HOOK`.
const CHAOS_DEATH_WATCH_STEP: &str = "chaosDeathWatch";
/// R169: entry 8's badge, in the card's own words.
pub const CHAOS_DEATH_WATCH_LABEL: &str =
    "For the rest of the game, whenever an Acclaimed card dies, its controller casts a random Call to Chaos";

/// `callToChaosStinky:<name>`, the kind of an entry's part (an effect's kind is a static name).
fn entry_kind(name: &str) -> &'static str {
    match name {
        "ethics" => "callToChaosStinky:ethics",
        "dandy" => "callToChaosStinky:dandy",
        "quests" => "callToChaosStinky:quests",
        "acclaimed" => "callToChaosStinky:acclaimed",
        "freeze" => "callToChaosStinky:freeze",
        "humans" => "callToChaosStinky:humans",
        "allure" => "callToChaosStinky:allure",
        "deathWatch" => "callToChaosStinky:deathWatch",
        "golem" => "callToChaosStinky:golem",
        _ => "callToChaosStinky",
    }
}

/// One entry, built as it resolves (`lazy_part`), kept by name across a pause like Core's (R87).
fn entry(
    name: &'static str,
    build: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static,
) -> Effect {
    crate::resolve::lazy_part(entry_kind(name), move |ctx, _memo| EffectPart {
        effects: build(ctx),
        memo: None,
    })
}

/// The set's card at `index`, by its id, when the catalog holds it.
fn def_id_at(index: &str) -> Option<String> {
    def_id_by_index(SET, index)
}

/// A fresh hand card of the set's card at `index`; a full hand burns it (§2.4).
fn add_card(index: &str, radiant: bool, cost_override: Option<i32>) -> Option<Effect> {
    let def_id = def_id_at(index)?;
    Some(crate::effects::add_to_hand::add_to_hand(AddToHandArgs {
        def_id: Some(def_id),
        radiant: radiant.then_some(true),
        cost_override,
        ..AddToHandArgs::default()
    }))
}

/// 1. A Radiant Faction: ETHICS, then a Radiant Faction: Vanguard Coalition, each a fresh hand card (§5.2).
fn build_ethics() -> Effect {
    entry("ethics", |_ctx| {
        [ETHICS_INDEX, VANGUARD_INDEX]
            .into_iter()
            .filter_map(|index| add_card(index, true, None))
            .collect()
    })
}

/// 2. A Very Dandy Lad, then an Arctic Citadel, each costing (0) (`costOverride`, R65).
fn build_dandy() -> Effect {
    entry("dandy", |_ctx| {
        [DANDY_LAD_INDEX, ARCTIC_CITADEL_INDEX]
            .into_iter()
            .filter_map(|index| add_card(index, false, Some(CHAOS_STINKYSCAPE_COST)))
            .collect()
    })
}

/// 3. R1882001: three independent picks (R60) of the Field Spells tagged Quest or Wincon (R1422), from
///    every set the catalog orders, so the held set's own reach it (R1420), each summoned into its
///    caster's backrow per R64; a full backrow takes no draw for a summon that cannot land (R129).
fn build_quests() -> Effect {
    entry("quests", |_ctx| {
        (0..CHAOS_STINKY_FIELD_SPELLS)
            .map(|_| {
                crate::effects::summon_random(json_as(json!({
                    "query": {
                        "type": "Field Spell",
                        "anyTags": [Tag::Quest, Tag::Wincon],
                        "set": CATALOG_SETS
                    }
                })))
            })
            .collect()
    })
}

/// 4. R1882002: one pick (R60) of the set's non-token Acclaimed Units, added to its caster's hand; once
///    it is there, a Refresh by its printed cost (§6.3, R364, never past max mana). A card a full hand
///    burns refreshes nothing.
fn build_acclaimed() -> Effect {
    entry("acclaimed", |_ctx| {
        vec![Effect::new("callToChaosStinky:addAcclaimed", |ctx| {
            let pool = query(&json_as(json!({
                "type": "Unit",
                "tags": [Tag::Acclaimed],
                "set": SET
            })));
            let Some(def) = ctx.sink.rng.pick(&pool).copied() else {
                return;
            };
            let controller = ctx.controller;
            let before = ctx.state.players[controller].hand.len();
            let add = crate::effects::add_to_hand::add_to_hand(AddToHandArgs {
                def_id: Some(def.id.clone()),
                ..AddToHandArgs::default()
            });
            (add.apply)(ctx);
            if ctx.state.players[controller].hand.len() <= before {
                return;
            }
            let refresh = crate::effects::refresh_mana(json_as(json!({ "amount": query_cost(def) })));
            (refresh.apply)(ctx);
        })]
    })
}

/// 5. R1882003: every enemy Unit active on the field switched to Attack Position (an effect's switch,
///    R20: no exertion, nothing for one already there or one that may not be), then every enemy Unit
///    Frozen (§6.3, R1174000); a hero is never touched.
fn build_freeze() -> Effect {
    entry("freeze", |_ctx| {
        vec![
            Effect::new("callToChaosStinky:toAttack", |ctx| {
                let enemy = opponent_of(ctx.controller);
                let units: Vec<CardInstance> = crate::zones::active_units_of(ctx.state, enemy)
                    .into_iter()
                    .cloned()
                    .collect();
                for unit in units {
                    let _ = switch_position(
                        ctx,
                        &unit,
                        SwitchPositionOptions {
                            spend_exertion: Some(false),
                            to: Some(Position::Atk),
                        },
                    );
                }
            }),
            crate::effects::freeze_all(json_as(json!({ "side": "enemy" }))),
        ]
    })
}

/// 6. R1080: every empty unit zone of its caster's filled with its own pick of the set's Human Units.
fn build_humans() -> Effect {
    entry("humans", |_ctx| {
        vec![crate::effects::fill_board_random(json_as(json!({
            "query": { "tags": [Tag::Human], "set": SET }
        })))]
    })
}

/// 7. R963: every enemy Unit Allured.
fn build_allure() -> Effect {
    entry("allure", |_ctx| vec![crate::effects::allure_enemy_units()])
}

/// R1882004: how many of entry 8's death watchers `player` holds.
pub fn death_watches_held(state: &GameState, player: PlayerId) -> i32 {
    state.players[player]
        .mods
        .iter()
        .filter(|held| {
            matches!(
                &held.kind,
                ModifierKind::DeathWatch { resume, .. } if resume.hook == CHAOS_DEATH_WATCH_HOOK
            )
        })
        .count() as i32
}

/// R2176001: how many Calls to Chaos the death watch has cast for `player` this turn.
pub fn death_watch_casts_this_turn(state: &GameState, player: PlayerId) -> i32 {
    match state.players[player].death_watch_casts {
        Some(casts) if casts.turn == state.turn => casts.count,
        _ => 0,
    }
}

/// R1882004: the step entry 8's watcher re-enters after each Acclaimed death, as the dead card's
/// controller: a random Call to Chaos, cast with no `self` and so as a new chain (R28's cap per chain),
/// the cast card a base Call. R2176001: once the watch has cast `CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN`
/// for that player this turn, it casts nothing more until the next.
pub fn death_watch_step() -> Hook {
    hook(|ctx| {
        let player = ctx.controller;
        let count = death_watch_casts_this_turn(&*ctx.state, player);
        if count >= CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN {
            return vec![];
        }
        // The count is written here, in the engine's own step, rather than through an Effect: CLAUDE.md
        // rule 5 binds card files, and this step is no card's (`work::script_step_for` answers the hook
        // whatever card made the watcher). It is written as it is read, so the record and the decision
        // it gates are one step: the cast returned below, and every death it causes, read the count
        // that already holds this cast (R2176001), as `draws` is counted as each draw is made.
        let turn = ctx.state.turn;
        ctx.state.players[player].death_watch_casts = Some(crate::state::DrawCount {
            turn,
            count: count + 1,
        });
        vec![cast_random_call_to_chaos()]
    })
}

/// 8. R1882004: "for the rest of the game, whenever an Acclaimed card dies, its controller casts a random
///    Call to Chaos", a watcher on its caster answering either player's Acclaimed deaths, stacking up to
///    `CHAOS_STINKY_DEATH_WATCH_CAP`; a roll of it for a player at the cap resolves into nothing.
fn build_death_watch() -> Effect {
    entry("deathWatch", |ctx| {
        if death_watches_held(ctx.state, ctx.controller) >= CHAOS_STINKY_DEATH_WATCH_CAP {
            return vec![];
        }
        vec![Effect::new("callToChaosStinky:watchDeaths", |ctx| {
            let mut resume = delayed_resume(
                ctx,
                CHAOS_DEATH_WATCH_STEP,
                Default::default(),
                CHAOS_DEATH_WATCH_HOOK,
            );
            resume.instance_id = None;
            let player = ctx.controller;
            crate::modifiers::add_death_watch(ctx, player, Tag::Acclaimed, resume, CHAOS_DEATH_WATCH_LABEL);
        })]
    })
}

/// 9. A Stinkyscape Golem (S #150.1), placed per R64.
fn build_golem() -> Effect {
    entry("golem", |_ctx| match def_id_at(GOLEM_INDEX) {
        None => vec![],
        Some(def_id) => vec![crate::effects::summon::summon(json_as(
            json!({ "defId": def_id }),
        ))],
    })
}

/// §8.10 row 150's ten entries, in the order the card prints them. Each `label` is the clause as the
/// card prints it, which `chaosRolled` names to both players (R436).
pub const CHAOS_STINKY_EFFECTS: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "ethics",
        label: "Add a Radiant Faction: ETHICS and a Radiant Faction: Vanguard Coalition to your hand",
        build: build_ethics,
    },
    ChaosEffectDef {
        name: "dandy",
        label: "Add A Very Dandy Lad and an Arctic Citadel to your hand, which cost (0)",
        build: build_dandy,
    },
    ChaosEffectDef {
        name: "quests",
        label: "Summon 3 random Quest or Wincon Field Spells into your backrow",
        build: build_quests,
    },
    ChaosEffectDef {
        name: "acclaimed",
        label: "Add a random Acclaimed Stinkyscape Unit to your hand and refresh as much mana as it costs",
        build: build_acclaimed,
    },
    ChaosEffectDef {
        name: "freeze",
        label: "Switch every enemy Unit to Attack Position and Freeze them",
        build: build_freeze,
    },
    ChaosEffectDef {
        name: "humans",
        label: "Fill your board with random Human Stinkyscape Units",
        build: build_humans,
    },
    ChaosEffectDef {
        name: "allure",
        label: "Allure every enemy Unit",
        build: build_allure,
    },
    ChaosEffectDef {
        name: "deathWatch",
        label: CHAOS_DEATH_WATCH_LABEL,
        build: build_death_watch,
    },
    ChaosEffectDef {
        name: "golem",
        label: "Summon a Stinkyscape Golem",
        build: build_golem,
    },
    // 10. Core's recursion, the chain counting casts of every edition (R28, R87, R452).
    ChaosEffectDef {
        name: "recast",
        label: "Cast a random Call to Chaos",
        build: cast_random_call_to_chaos,
    },
];
