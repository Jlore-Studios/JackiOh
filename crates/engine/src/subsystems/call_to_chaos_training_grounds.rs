//! Training Grounds #39 Call to Chaos (Training-Grounds Edition) (SPEC §8.11 row 39, R28, R87, R423, R436,
//! R2200000–R2200004): its table of ten effects, which Core #95's subsystem (`call_to_chaos.rs`) rolls,
//! announces and resolves, so one rule serves every edition. Every entry is built when it resolves
//! (`lazy_part`), as the other editions' are (R87, R113).

use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::config::{
    CHAOS_TG_ALTERNATE_CAP, CHAOS_TG_ARMY_TOKENS, CHAOS_TG_CARDS, CHAOS_TG_COST, CHAOS_TG_DRAW,
    CHAOS_TG_EVERY, CHAOS_TG_MANA, CHAOS_TG_PERMANENTS, CHAOS_TG_RADIANT_CARDS, CHAOS_TG_REFRESH,
    CHAOS_TG_RUSH_TOKENS,
};
use crate::effects::add_to_hand::AddToHandArgs;
use crate::effects::delay::ForRestOfGameArgs;
use crate::prelude::json_as;
use crate::query::room_in;
use crate::script::{Effect, EffectContext, EffectPart, Hook, hook};
use crate::state::{GameState, ModifierKind};
use crate::subsystems::call_to_chaos::{CHAOS_TAG, ChaosEffectDef, cast_random_call_to_chaos};
use crate::wire::{PlayerId, SetName};
use crate::zones::OffFieldZone;

/// R2200000: the pool "Training Grounds cards" and "Training Grounds permanents" name, the set's
/// non-token cards, held or shipped (R2194001).
const TG_SET: &str = "Training Grounds";
/// §7: the cards this edition makes, by index.
const RUSH_TOKEN_INDEX: &str = "T-rush";
const DESIGN_INDEX: &str = "40";
const GOLEM_INDEX: &str = "39.1";

/// R2200002, R458: the hook of entry 9's rest-of-game effect, the engine's own: `work::script_step_for`
/// answers it with `alternate_step` whatever card made it.
pub const CHAOS_TG_ALTERNATE_HOOK: &str = "@chaosTgAlternate";
/// Entry 9's step name, under `CHAOS_TG_ALTERNATE_HOOK`.
const CHAOS_TG_ALTERNATE_STEP: &str = "chaosTgAlternate";
/// Entry 9's data: the caster's `turns_started` as the effect was made.
const FROM_KEY: &str = "fromTurn";
/// R169: entry 9's badge, in the card's own words.
pub const CHAOS_TG_ALTERNATE_LABEL: &str = "Add a random Call to Chaos to your hand, which costs (0), at the start of every second turn of yours for the rest of the game";

/// `callToChaosTrainingGrounds:<name>`, the kind of an entry's part (an effect's kind is a static name).
fn entry_kind(name: &str) -> &'static str {
    match name {
        "fortify" => "callToChaosTrainingGrounds:fortify",
        "designs" => "callToChaosTrainingGrounds:designs",
        "cards" => "callToChaosTrainingGrounds:cards",
        "mana" => "callToChaosTrainingGrounds:mana",
        "permanents" => "callToChaosTrainingGrounds:permanents",
        "radiant" => "callToChaosTrainingGrounds:radiant",
        "army" => "callToChaosTrainingGrounds:army",
        "golem" => "callToChaosTrainingGrounds:golem",
        "alternate" => "callToChaosTrainingGrounds:alternate",
        _ => "callToChaosTrainingGrounds",
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

fn def_id_of(set: SetName, index: &str) -> Option<String> {
    crate::catalog::def_by_index(set, index).map(|def| def.id.clone())
}

/// `count` summons of a card by index, placed per R64; a full row takes fewer.
fn summon_many(set: SetName, index: &str, count: i32) -> Vec<Effect> {
    let Some(def_id) = def_id_of(set, index) else {
        return vec![];
    };
    (0..count)
        .map(|_| crate::effects::summon::summon(json_as(json!({ "defId": def_id }))))
        .collect()
}

/// Entries 3 and 6: N independent picks (R60) of the set's non-token cards, never this one (R387), each
/// costing (0); a full hand burns the rest (§2.4).
fn add_free(name: &'static str, count: i32, radiant: bool) -> Effect {
    entry(name, move |_ctx| {
        vec![crate::effects::add_to_hand::add_random_from_catalog(json_as(
            json!({
                "query": { "set": TG_SET },
                "count": count,
                "radiant": radiant,
                "costOverride": CHAOS_TG_COST,
            }),
        ))]
    })
}

/// R2200002, R2200004: one random Call to Chaos of the sets that ship (R380, R1420), this edition among
/// them once its set ships (R387), added at (0); a full hand burns it (§2.4).
fn add_random_call() -> Effect {
    Effect::new("callToChaosTrainingGrounds:addCall", |ctx| {
        let pool = crate::catalog::query(&json_as(json!({ "tags": [CHAOS_TAG] })));
        let Some(def_id) =
            crate::catalog::pick_generated(&mut *ctx.sink.rng, &pool, None).map(|def| def.id.clone())
        else {
            return;
        };
        let add = crate::effects::add_to_hand::add_to_hand(AddToHandArgs {
            def_id: Some(def_id),
            cost_override: Some(CHAOS_TG_COST),
            ..AddToHandArgs::default()
        });
        (add.apply)(ctx);
    })
}

/// R2200002: how many of entry 9's rest-of-game effects `player` holds.
pub fn alternate_effects_held(state: &GameState, player: PlayerId) -> i32 {
    state.players[player]
        .mods
        .iter()
        .filter(|held| {
            matches!(
                &held.kind,
                ModifierKind::StartOfTurnEffect { resume, .. } if resume.hook == CHAOS_TG_ALTERNATE_HOOK
            )
        })
        .count() as i32
}

/// R2200002: entry 9's step at the start of each of its player's turns: it adds only when the turns
/// begun since the effect was made are a positive multiple of `CHAOS_TG_EVERY` (extra turns, R846).
pub fn alternate_step() -> Hook {
    hook(|ctx| {
        let now = ctx.state.players[ctx.controller].turns_started;
        let from = ctx
            .data
            .get(FROM_KEY)
            .and_then(Value::as_i64)
            .map_or(now, |from| from as i32);
        let since = now - from;
        if since > 0 && since % CHAOS_TG_EVERY == 0 {
            vec![add_random_call()]
        } else {
            vec![]
        }
    })
}

/// 1. R2200001: three base Rush Tokens (§7), then Taunt and Divine Shield for the caster's Unit cards
///    on the field, in hand and in the deck (C+ #40's scope); a hidden card changes silently (R440).
fn build_fortify() -> Effect {
    entry("fortify", |_ctx| {
        let scope = json!({
            "side": "self",
            "zones": ["field", "hand", "library"],
            "rows": ["units"],
            "types": ["Unit"],
        });
        let mut effects = summon_many(SetName::Core, RUSH_TOKEN_INDEX, CHAOS_TG_RUSH_TOKENS);
        for kind in ["Taunt", "Divine Shield"] {
            effects.push(crate::effects::grant_keyword_cards(json_as(
                json!({ "scope": scope, "keyword": { "kind": kind } }),
            )));
        }
        effects
    })
}

/// 2. R2200001: Goetzmann Game Designs (#40) until the hand is full (S #88 Siva 2's fill), then §6.3
///    Refresh 4 (R364).
fn build_designs() -> Effect {
    entry("designs", |ctx| {
        let room = room_in(ctx.state, ctx.controller, OffFieldZone::Hand);
        let mut effects: Vec<Effect> = match def_id_of(SetName::TrainingGrounds, DESIGN_INDEX) {
            None => vec![],
            Some(def_id) => (0..room)
                .map(|_| crate::effects::add_to_hand::add_to_hand(json_as(json!({ "defId": def_id }))))
                .collect(),
        };
        effects.push(crate::effects::mana::refresh_mana(json_as(
            json!({ "amount": CHAOS_TG_REFRESH }),
        )));
        effects
    })
}

/// 3. R2200000: four random Training Grounds cards at (0).
fn build_cards() -> Effect {
    add_free("cards", CHAOS_TG_CARDS, false)
}

/// 4. Temporary mana that may pass max (§2.3), then two draws.
fn build_mana() -> Effect {
    entry("mana", |_ctx| {
        vec![
            crate::effects::gain_mana(json_as(json!({ "amount": CHAOS_TG_MANA }))),
            crate::effects::draw(json_as(json!({ "count": CHAOS_TG_DRAW }))),
        ]
    })
}

/// 5. R2200000: six independent picks (R60) of the set's non-token permanents, each summoned by its type
///    (a Trap or Field Trap face-down, §3.2) with no Cry and no Tribute (R1), drawn only when its zone
///    is open (R129).
fn build_permanents() -> Effect {
    entry("permanents", |_ctx| {
        (0..CHAOS_TG_PERMANENTS)
            .map(|_| {
                crate::effects::summon_random(json_as(json!({
                    "query": {
                        "set": TG_SET,
                        "type": ["Unit", "Field Spell", "Trap", "Field Trap"]
                    }
                })))
            })
            .collect()
    })
}

/// 6. R2200000: two random Radiant Training Grounds cards at (0).
fn build_radiant() -> Effect {
    add_free("radiant", CHAOS_TG_RADIANT_CARDS, true)
}

/// 7. R2200003: five base Rush Tokens, then three `unitsHaveKeyword` modifiers on the caster (R1859007),
///    so Units that arrive later have them too, kept when the card is gone.
fn build_army() -> Effect {
    entry("army", |_ctx| {
        let mut effects = summon_many(SetName::Core, RUSH_TOKEN_INDEX, CHAOS_TG_ARMY_TOKENS);
        for kind in ["Rush", "Trample", "Pierce"] {
            effects.push(crate::effects::player_mods::give_units_keyword(json_as(
                json!({ "keyword": { "kind": kind } }),
            )));
        }
        effects
    })
}

/// 8. The Training-Grounds Golem token (#39.1), placed per R64.
fn build_golem() -> Effect {
    entry("golem", |_ctx| {
        summon_many(SetName::TrainingGrounds, GOLEM_INDEX, 1)
    })
}

/// 9. R2200002: a random Call to Chaos at (0) now, then the rest-of-game effect, stacking up to
///    `CHAOS_TG_ALTERNATE_CAP`; a roll of it at the cap resolves into nothing, the "now" included, with
///    no re-roll (R87's shape, as R1241).
fn build_alternate() -> Effect {
    entry("alternate", |ctx| {
        if alternate_effects_held(ctx.state, ctx.controller) >= CHAOS_TG_ALTERNATE_CAP {
            return vec![];
        }
        let mut data = IndexMap::new();
        data.insert(
            FROM_KEY.to_string(),
            json!(ctx.state.players[ctx.controller].turns_started),
        );
        vec![
            add_random_call(),
            crate::effects::delay::for_rest_of_game(ForRestOfGameArgs {
                step: CHAOS_TG_ALTERNATE_STEP.into(),
                label: CHAOS_TG_ALTERNATE_LABEL.into(),
                hook: Some(CHAOS_TG_ALTERNATE_HOOK.into()),
                data: Some(data),
                player: None,
                at_end: None,
            }),
        ]
    })
}

/// §8.11 row 39's ten entries, in the order the card prints them. Each `label` is the clause as the card
/// prints it, which `chaosRolled` names to both players (R436).
pub const CHAOS_TG_EFFECTS: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "fortify",
        label: "Summon 3 Rush Tokens, then give your Units on the field, in your hand and in your deck Taunt and Divine Shield",
        build: build_fortify,
    },
    ChaosEffectDef {
        name: "designs",
        label: "Fill your hand with Goetzmann Game Designs and refresh 4 mana",
        build: build_designs,
    },
    ChaosEffectDef {
        name: "cards",
        label: "Add 4 random Training Grounds cards to your hand, which cost (0)",
        build: build_cards,
    },
    ChaosEffectDef {
        name: "mana",
        label: "Gain 8 mana and draw 2 cards",
        build: build_mana,
    },
    ChaosEffectDef {
        name: "permanents",
        label: "Summon 6 random Training Grounds permanents",
        build: build_permanents,
    },
    ChaosEffectDef {
        name: "radiant",
        label: "Add 2 random Radiant Training Grounds cards to your hand, which cost (0)",
        build: build_radiant,
    },
    ChaosEffectDef {
        name: "army",
        label: "Summon 5 Rush Tokens, and for the rest of the game your Units have Rush, Trample and Pierce",
        build: build_army,
    },
    ChaosEffectDef {
        name: "golem",
        label: "Summon a Training-Grounds Golem",
        build: build_golem,
    },
    ChaosEffectDef {
        name: "alternate",
        label: "Add a random Call to Chaos to your hand, which costs (0), now and at the start of every second turn of yours for the rest of the game",
        build: build_alternate,
    },
    // 10. Core's recursion, the chain counting casts of every edition (R28, R87).
    ChaosEffectDef {
        name: "recast",
        label: "Cast a random Call to Chaos",
        build: cast_random_call_to_chaos,
    },
];
