//! Stinkyscape+ #142 Call to Chaos (Mindustrial-Swarm Edition) (SPEC §8.12 row 142, R28, R87, R423, R436,
//! R3570000–R3570005): its table of ten effects, which Core #95's subsystem (`call_to_chaos.rs`) rolls,
//! announces and resolves, so one rule serves every edition. Every entry is built when it resolves
//! (`lazy_part`), as the other editions' are (R87, R113).
//!
//! Entry 8 is the engine's own rest-of-game rule (R3570004), as S #150's death watcher is (R1882004): a
//! launch watcher under `CHAOS_MS_LAUNCH_WATCH_HOOK`, which `work::script_step_for` answers with
//! `launch_watch_step` whatever card made it, so the table never leans on a card file; its per-turn cap
//! terminates it.

use serde_json::{Value, json};

use crate::catalog::def_id_by_index;
use crate::config::{
    CHAOS_MS_CARD_COST, CHAOS_MS_GIGA_UNITS, CHAOS_MS_LAUNCH_WATCH_ANSWERS_PER_TURN,
    CHAOS_MS_LAUNCH_WATCH_CAP, CHAOS_MS_PAPERCLIP_REPLENISH, CHAOS_MS_TERMINUS_REPLENISH,
    CHAOS_MS_WEAPON_COST,
};
use crate::effects::delay::delayed_resume;
use crate::params::param;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, EffectPart, Hook, hook};
use crate::state::{DrawCount, GameState, ModifierKind};
use crate::subsystems::call_to_chaos::{
    ChaosEffectDef, cast_random_call_to_chaos, cast_random_other_call_to_chaos,
};
use crate::wire::{CATALOG_SETS, PlayerId, SetName, Tag};

/// §8.12: the set whose indices the table names (B2.2).
const SET: SetName = SetName::StinkyscapePlus;
/// Entry 6: S+ #121 Toxopid.
const TOXOPID_INDEX: &str = "121";
/// Entry 9: S+ #142.1 Mindustrial Golem and #142.2 Swarm Golem.
const MINDUSTRIAL_GOLEM_INDEX: &str = "142.1";
const SWARM_GOLEM_INDEX: &str = "142.2";

/// The card's declared numbers (`params` in `catalog.json`), read with `param` as the entry resolves
/// (CLAUDE.md rule 9): the Paperclip, Evolved Swarm, Jlockeed and Terminus counts, and Toxic and Burn.
const PAPERCLIP_COUNT_KEY: &str = "card";
const SWARM_COUNT_KEY: &str = "card2";
const JLOCKEED_COUNT_KEY: &str = "card3";
const TERMINUS_COUNT_KEY: &str = "card4";
const TOXIC_KEY: &str = "toxic";
const BURN_KEY: &str = "burn";

/// R3570004: the hook of entry 8's rest-of-game effect, the engine's own: `work::script_step_for` answers it
/// with `launch_watch_step` whatever card made it.
pub const CHAOS_MS_LAUNCH_WATCH_HOOK: &str = "@chaosMsLaunchWatch";
/// Entry 8's step name, under `CHAOS_MS_LAUNCH_WATCH_HOOK`.
const CHAOS_MS_LAUNCH_WATCH_STEP: &str = "chaosMsLaunchWatch";
/// R169, R3570000: entry 8's clause and its badge, as the card prints it.
pub const CHAOS_MS_LAUNCH_WATCH_LABEL: &str = "For the rest of the game, when you launch a Starship, cast a random Call to Chaos and summon a random (2) Cost Weapon";

/// `callToChaosMindustrialSwarm:<name>`, the kind of an entry's part (an effect's kind is a static name).
fn entry_kind(name: &str) -> &'static str {
    match name {
        "paperclips" => "callToChaosMindustrialSwarm:paperclips",
        "swarm" => "callToChaosMindustrialSwarm:swarm",
        "giga" => "callToChaosMindustrialSwarm:giga",
        "jlockeed" => "callToChaosMindustrialSwarm:jlockeed",
        "terminus" => "callToChaosMindustrialSwarm:terminus",
        "toxopid" => "callToChaosMindustrialSwarm:toxopid",
        "afflict" => "callToChaosMindustrialSwarm:afflict",
        "launchWatch" => "callToChaosMindustrialSwarm:launchWatch",
        "golems" => "callToChaosMindustrialSwarm:golems",
        "recast" => "callToChaosMindustrialSwarm:recast",
        _ => "callToChaosMindustrialSwarm",
    }
}

/// One entry, built as it resolves (`lazy_part`), kept by name across a pause like Core's (R87, R113).
fn entry(
    name: &'static str,
    build: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static,
) -> Effect {
    crate::resolve::lazy_part(entry_kind(name), move |ctx, _memo| EffectPart {
        effects: build(ctx),
        memo: None,
    })
}

/// R3570000: the pool of every card carrying `tag`, held sets included (R1420), as S #150's quest pool is.
fn tag_pool(tag: Tag) -> Value {
    json!({ "tags": [tag], "set": CATALOG_SETS })
}

/// 1. R3570000: `card` random Paperclip cards, each costing `CHAOS_MS_CARD_COST`; a full hand burns the rest
///    (§2.4). Then Refresh 2 (§6.3, R364), whatever the hand took.
fn build_paperclips() -> Effect {
    entry("paperclips", |ctx| {
        vec![
            crate::effects::add_to_hand::add_random_from_catalog(json_as(json!({
                "query": tag_pool(Tag::Paperclip),
                "count": param(&*ctx, PAPERCLIP_COUNT_KEY),
                "costOverride": CHAOS_MS_CARD_COST,
            }))),
            crate::effects::mana::refresh_mana(json_as(json!({ "amount": CHAOS_MS_PAPERCLIP_REPLENISH }))),
        ]
    })
}

/// 2. R3570000, R3570001: `card2` random Swarm cards, each costing `CHAOS_MS_CARD_COST` and marked to enter
///    the field evolved where their text prints an Evolve hook (R3536002).
fn build_swarm() -> Effect {
    entry("swarm", |ctx| {
        vec![crate::effects::add_to_hand::add_random_from_catalog(json_as(
            json!({
                "query": tag_pool(Tag::Swarm),
                "count": param(&*ctx, SWARM_COUNT_KEY),
                "costOverride": CHAOS_MS_CARD_COST,
                "entersEvolved": true,
            }),
        ))]
    })
}

/// 3. R3570000, R3570002: `CHAOS_MS_GIGA_UNITS` random Radiant Units (a pool naming no set, R380), each
///    summoned GIGA, drawn only when four open zones fit one (R129, R3546000).
fn build_giga() -> Effect {
    entry("giga", |_ctx| {
        (0..CHAOS_MS_GIGA_UNITS)
            .map(|_| {
                crate::effects::summon_random_with(json_as(json!({
                    "query": { "type": "Unit" },
                    "radiant": true,
                    "giga": true,
                })))
            })
            .collect()
    })
}

/// 4. R3570003: `card3` random Jlockeed cards, each Spell cast and each permanent summoned.
fn build_jlockeed() -> Effect {
    entry("jlockeed", |ctx| {
        (0..param(&*ctx, JLOCKEED_COUNT_KEY).max(0))
            .map(|_| crate::effects::cast::summon_or_cast_one_random(json_as(tag_pool(Tag::Jlockeed))))
            .collect()
    })
}

/// 5. R3570000: `card4` random Terminus cards, tokens among them (S+ #142's designer text names no
///    printed set: SP5), each at its printed price (the card prints no `(0)`), then Refresh 4.
fn build_terminus() -> Effect {
    entry("terminus", |ctx| {
        let mut query = tag_pool(Tag::Terminus);
        query["withTokens"] = json!(true);
        vec![
            crate::effects::add_to_hand::add_random_from_catalog(json_as(json!({
                "query": query,
                "count": param(&*ctx, TERMINUS_COUNT_KEY),
            }))),
            crate::effects::mana::refresh_mana(json_as(json!({ "amount": CHAOS_MS_TERMINUS_REPLENISH }))),
        ]
    })
}

/// 6. R3570000: S+ #121 Toxopid, a fresh hand card costing `CHAOS_MS_CARD_COST`; a full hand burns it (§2.4).
fn build_toxopid() -> Effect {
    entry("toxopid", |_ctx| match def_id_by_index(SET, TOXOPID_INDEX) {
        None => vec![],
        Some(def_id) => vec![crate::effects::add_to_hand::add_to_hand(json_as(json!({
            "defId": def_id,
            "costOverride": CHAOS_MS_CARD_COST,
        })))],
    })
}

/// 7. R3570000: Toxic `toxic`, then Burn `burn`, on every enemy Unit and their hero (R3540000).
fn build_afflict() -> Effect {
    entry("afflict", |ctx| {
        vec![
            crate::effects::toxic_all(json_as(json!({
                "amount": param(&*ctx, TOXIC_KEY),
                "side": "enemy",
                "heroes": true,
            }))),
            crate::effects::burn_all(json_as(json!({
                "amount": param(&*ctx, BURN_KEY),
                "side": "enemy",
                "heroes": true,
            }))),
        ]
    })
}

/// R3570004: how many of entry 8's launch watchers `player` holds.
pub fn launch_watches_held(state: &GameState, player: PlayerId) -> i32 {
    state.players[player]
        .mods
        .iter()
        .filter(|held| {
            matches!(
                &held.kind,
                ModifierKind::LaunchWatch { resume, .. } if resume.hook == CHAOS_MS_LAUNCH_WATCH_HOOK
            )
        })
        .count() as i32
}

/// R3570004: how many launches the launch watch has answered for `player` this turn.
pub fn launch_watch_answers_this_turn(state: &GameState, player: PlayerId) -> i32 {
    match state.players[player].launch_watch_casts {
        Some(casts) if casts.turn == state.turn => casts.count,
        _ => 0,
    }
}

/// R3570004: entry 8's step after each of its holder's Starship launches, as the holder: a base Call to
/// Chaos cast with no `self`, so a new chain (R28's cap per chain), then a random Weapon of
/// `CHAOS_MS_WEAPON_COST` summoned (R129); capped per turn (R2176001's shape).
pub fn launch_watch_step() -> Hook {
    hook(|ctx| {
        let player = ctx.controller;
        let count = launch_watch_answers_this_turn(&*ctx.state, player);
        if count >= CHAOS_MS_LAUNCH_WATCH_ANSWERS_PER_TURN {
            return vec![];
        }
        // CLAUDE.md rule 5 binds card files and this step is no card's, so it writes the count as it
        // reads it: the cast below and every launch it causes read a count holding this answer.
        let turn = ctx.state.turn;
        ctx.state.players[player].launch_watch_casts = Some(DrawCount {
            turn,
            count: count + 1,
        });
        vec![
            cast_random_call_to_chaos(),
            crate::effects::summon_random(json_as(json!({
                "query": { "tags": [Tag::Weapon], "cost": CHAOS_MS_WEAPON_COST, "set": CATALOG_SETS }
            }))),
        ]
    })
}

/// 8. R3570004: a watcher on its caster answering their own Starship launches, stacking up to
///    `CHAOS_MS_LAUNCH_WATCH_CAP`; at the cap a roll of it resolves into nothing.
fn build_launch_watch() -> Effect {
    entry("launchWatch", |ctx| {
        if launch_watches_held(ctx.state, ctx.controller) >= CHAOS_MS_LAUNCH_WATCH_CAP {
            return vec![];
        }
        vec![Effect::new("callToChaosMindustrialSwarm:watchLaunches", |ctx| {
            let mut resume = delayed_resume(
                ctx,
                CHAOS_MS_LAUNCH_WATCH_STEP,
                Default::default(),
                CHAOS_MS_LAUNCH_WATCH_HOOK,
            );
            resume.instance_id = None;
            let player = ctx.controller;
            crate::modifiers::add_launch_watch(ctx, player, resume, CHAOS_MS_LAUNCH_WATCH_LABEL);
        })]
    })
}

/// 9. S+ #142.1 Mindustrial Golem, then #142.2 Swarm Golem, each placed per R64.
fn build_golems() -> Effect {
    entry("golems", |_ctx| {
        [MINDUSTRIAL_GOLEM_INDEX, SWARM_GOLEM_INDEX]
            .into_iter()
            .filter_map(|index| def_id_by_index(SET, index))
            .map(|def_id| crate::effects::summon::summon(json_as(json!({ "defId": def_id }))))
            .collect()
    })
}

/// 10. R3570005: Core's recursion, the chain counting casts of every edition (R28, R87), that never draws
///     this edition's own card.
fn build_recast() -> Effect {
    cast_random_other_call_to_chaos()
}

/// §8.12 row 142's ten entries, in the order the card prints them. Each `label` is the clause as the card
/// prints it, which `chaosRolled` names to both players (R436); a tuned `param` does not change it
/// (R3570000).
pub const CHAOS_MS_EFFECTS: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "paperclips",
        label: "Add 3 random Paperclip cards to your hand, they cost (0), replenish 2 mana",
        build: build_paperclips,
    },
    ChaosEffectDef {
        name: "swarm",
        label: "Add 3 random Evolved Swarm cards to your hand, they cost (0)",
        build: build_swarm,
    },
    ChaosEffectDef {
        name: "giga",
        label: "Summon two random Radiant Units, make them GIGA",
        build: build_giga,
    },
    ChaosEffectDef {
        name: "jlockeed",
        label: "Summon or cast 5 random Jlockeed cards",
        build: build_jlockeed,
    },
    ChaosEffectDef {
        name: "terminus",
        label: "Add 3 random Terminus cards to your hand, replenish 4 mana",
        build: build_terminus,
    },
    ChaosEffectDef {
        name: "toxopid",
        label: "Add the Toxopid to your hand, it costs (0)",
        build: build_toxopid,
    },
    ChaosEffectDef {
        name: "afflict",
        label: "Toxic 3 and Burn 3 all enemies",
        build: build_afflict,
    },
    ChaosEffectDef {
        name: "launchWatch",
        label: CHAOS_MS_LAUNCH_WATCH_LABEL,
        build: build_launch_watch,
    },
    ChaosEffectDef {
        name: "golems",
        label: "Summon a Mindustrial Golem and a Swarm Golem",
        build: build_golems,
    },
    ChaosEffectDef {
        name: "recast",
        label: "Cast a random Call to Chaos",
        build: build_recast,
    },
];
