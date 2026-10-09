//! Meditative #95 Call to Chaos (Meditative Edition) (SPEC §8.8 row 95, R28, R87, R423, R436, R1240–R1244):
//! its table of ten effects, which Core #95's subsystem (`call_to_chaos.rs`) rolls, announces and resolves —
//! `call_to_chaos({ radiant, table: CHAOS_MED_EFFECTS })` — so one rule serves every edition: the base face
//! rolls one entry, the Radiant three different ones resolved in the list's order (R423), the recursion
//! counts casts of every edition against `CALL_TO_CHAOS_CHAIN_CAP` (R28) and at the cap resolves into
//! nothing (R87), and both players are told what was rolled (`chaosRolled`, R436).
//!
//! Every entry is built when it resolves, not when the Cry returns (a part of the list, `lazy_part`), as
//! Core's and Classic+'s are: a recursion rolled before it resolves its whole chain first and changes the
//! hand, the deck and the board each later entry reads (R87), and a pause inside one parks the rest of it
//! (R113).
//!
//! Entry 9 is the engine's own rest-of-game effect (R458, R1241): a start-of-turn effect under
//! `CHAOS_ETERNAL_HOOK`, whose step `work::script_step_for` answers with `eternal_step` whatever card
//! made it, so the table never leans on a card file; a player holds at most `CALL_TO_CHAOS_ETERNAL_CAP`.

use serde_json::{Value, json};

use crate::config::{
    CALL_TO_CHAOS_ETERNAL_CAP, CHAOS_MED_ACCLAIMED, CHAOS_MED_CN_CARDS, CHAOS_MED_COST,
    CHAOS_MED_FUSE_COPIES, CHAOS_MED_HEAL, CHAOS_MED_HERO_ARMOR, CHAOS_MED_NERFS, CHAOS_MED_PRIME_CARDS,
};
use crate::effects::add_to_hand::AddToHandArgs;
use crate::effects::delay::ForRestOfGameArgs;
use crate::effects::targets::{BoardScope, cards_in_scope};
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, EffectPart, Hook, hook};
use crate::state::{GameState, ModifierKind, find_instance, find_instance_mut};
use crate::subsystems::call_to_chaos::{ChaosEffectDef, cast_random_call_to_chaos};
use crate::subsystems::fuse::{FuseArgs, HandPrice};
use crate::wire::{KeywordKind, PlayerId, SetName, ZoneName};

/// §7: the tokens this edition summons, by their indices in the Meditative set (B2.2): #39.5 Jade
/// Beauty and #95.1 CN Golem.
const TOKEN_SET: SetName = SetName::Meditative;
const JADE_BEAUTY_INDEX: &str = "39.5";
const GOLEM_INDEX: &str = "95.1";

/// R1241: the hook of entry 9's rest-of-game effect, which the engine resolves itself.
pub const CHAOS_ETERNAL_HOOK: &str = "@chaosEternal";
/// Entry 9's step name, under `CHAOS_ETERNAL_HOOK`.
const CHAOS_ETERNAL_STEP: &str = "chaosEternal";
/// R169: entry 9's badge, in the card's own words.
pub const CHAOS_ETERNAL_LABEL: &str = "At the start of each of your turns, cast a random Call to Chaos";

/// `callToChaosMeditative:<name>`, the kind of an entry's part (an effect's kind is a static name).
fn entry_kind(name: &str) -> &'static str {
    match name {
        "fuse" => "callToChaosMeditative:fuse",
        "cn" => "callToChaosMeditative:cn",
        "prime" => "callToChaosMeditative:prime",
        "armor" => "callToChaosMeditative:armor",
        "jade" => "callToChaosMeditative:jade",
        "acclaimed" => "callToChaosMeditative:acclaimed",
        "bounce" => "callToChaosMeditative:bounce",
        "golem" => "callToChaosMeditative:golem",
        "eternal" => "callToChaosMeditative:eternal",
        _ => "callToChaosMeditative",
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

/// Entries 2 and 3: N independent picks of a pool (R60), each a fresh hand card that costs (0); a full
/// hand burns it (§2.4).
fn add_free(name: &'static str, pool: Value, count: i32) -> Effect {
    entry(name, move |_ctx| {
        vec![crate::effects::add_to_hand::add_random_from_catalog(json_as(
            json!({
                "query": pool,
                "count": count,
                "costOverride": CHAOS_MED_COST,
            }),
        ))]
    })
}

/// Entries 5 and 8: a Meditative token by its index, placed per R64; nothing on a full row.
fn summon_token(name: &'static str, index: &'static str) -> Effect {
    entry(name, move |_ctx| {
        let def_id = crate::catalog::def_by_index(TOKEN_SET, index).map(|def| def.id.clone());
        match def_id {
            None => vec![],
            Some(def_id) => vec![crate::effects::summon::summon(json_as(
                json!({ "defId": def_id }),
            ))],
        }
    })
}

/// R1241: how many of entry 9's rest-of-game effects `player` holds.
pub fn eternal_effects_held(state: &GameState, player: PlayerId) -> i32 {
    state.players[player]
        .mods
        .iter()
        .filter(|held| {
            matches!(
                &held.kind,
                ModifierKind::StartOfTurnEffect { resume, .. } if resume.hook == CHAOS_ETERNAL_HOOK
            )
        })
        .count() as i32
}

/// R1241: the step entry 9's effect re-enters at the start of each of its player's turns: a random
/// Call to Chaos, cast with no `self` and so as a new chain (cast depth 1, R28's cap per chain), the
/// cast card a base Call (R28).
pub fn eternal_step() -> Hook {
    hook(|_ctx| vec![cast_random_call_to_chaos()])
}

/// 1. R1242: every card in your hand as the entry resolves, in hand order, but the Immutable ones
///    (R23), fused into one fresh hand card (R77, R102, R469), plus two copies of it (R57's riders),
///    all three costing (0). One fusable card is not fused: it takes the (0) and the copies. None, and
///    the entry does nothing. The fusion is a hand card's, so its `fused` event is hidden as the hand
///    is (R470).
fn build_fuse() -> Effect {
    entry("fuse", |_ctx| {
        vec![Effect::new("callToChaosMeditative:fuseHand", |ctx| {
            let controller = ctx.controller;
            let state: &GameState = ctx.state;
            let fusable: Vec<_> = state.players[controller]
                .hand
                .iter()
                .filter(|card| !crate::layers::unit_has(state, card, KeywordKind::Immutable))
                .cloned()
                .collect();
            let kept_id = match fusable.len() {
                0 => return,
                1 => fusable[0].id.clone(),
                _ => {
                    let fused = crate::subsystems::fuse::fuse(
                        ctx,
                        FuseArgs {
                            ingredients: fusable,
                            to_hand: Some(controller),
                            hand_price: Some(HandPrice::Free),
                            ..FuseArgs::default()
                        },
                    );
                    match fused {
                        Some(card) => card.id,
                        None => return,
                    }
                }
            };
            let Some(kept) = find_instance(ctx.state, &kept_id) else {
                return;
            };
            if kept.zone.z() != ZoneName::Hand {
                return;
            }
            if let Some(live) = find_instance_mut(ctx.state, &kept_id) {
                live.cost_override = Some(CHAOS_MED_COST);
            }
            for _ in 0..CHAOS_MED_FUSE_COPIES {
                let copy = crate::effects::add_to_hand::add_to_hand(AddToHandArgs {
                    copy_of: Some(kept_id.clone()),
                    cost_override: Some(CHAOS_MED_COST),
                    ..AddToHandArgs::default()
                });
                (copy.apply)(ctx);
            }
        })]
    })
}

/// 2. Non-token CN cards of every set that ships (R380, R1420).
fn build_cn() -> Effect {
    add_free("cn", json!({ "tags": ["CN"] }), CHAOS_MED_CN_CARDS)
}

/// 3. The Prime pool, which holds the Prime tokens (R1421).
fn build_prime() -> Effect {
    add_free("prime", json!({ "tags": ["Prime"] }), CHAOS_MED_PRIME_CARDS)
}

/// 4. §6.1's Armor on the hero, kept for the rest of the game (C+ #46's, R124), then a heal of the hero
///    with no cap (§6.3).
fn build_armor() -> Effect {
    entry("armor", |_ctx| {
        vec![
            crate::effects::perks::gain_hero_armor(json_as(json!({ "amount": CHAOS_MED_HERO_ARMOR }))),
            crate::effects::heal(json_as(json!({
                "target": { "of": "selfHero" },
                "amount": CHAOS_MED_HEAL
            }))),
        ]
    })
}

/// 5. The Jade Beauty token's base face (Meditative #39.5); the Jade Counter is not touched.
fn build_jade() -> Effect {
    summon_token("jade", JADE_BEAUTY_INDEX)
}

/// 6. R1243: three independent picks (R60) of the non-token Acclaimed permanents, each summoned per R64
///    with no Cry and no Tribute (R1); the Acclaimed tokens are not in the pool (§5.1).
fn build_acclaimed() -> Effect {
    entry("acclaimed", |_ctx| {
        (0..CHAOS_MED_ACCLAIMED)
            .map(|_| {
                crate::effects::summon_random(json_as(json!({
                    "query": {
                        "tags": ["Acclaimed"],
                        "type": ["Unit", "Field Spell", "Trap", "Field Trap"]
                    }
                })))
            })
            .collect()
    })
}

/// 7. R1244: every enemy permanent acting on the field — pile tops and face-down cards — bounced
///    together to its controller's hand (R747; tokens cease to exist, R11; a full hand burns, R4), then
///    each bounced card that reached a hand Nerfed once (one Degrade, R386), hidden there (R177, R440).
fn build_bounce() -> Effect {
    entry("bounce", |ctx| {
        let scope: BoardScope = json_as(json!({ "side": "enemy", "rows": ["units", "backrow"] }));
        let bounced: Vec<String> = cards_in_scope(ctx, &scope)
            .iter()
            .map(|card| card.id.clone())
            .collect();
        vec![
            crate::effects::bounce_all(scope),
            Effect::new("callToChaosMeditative:nerfBounced", move |ctx| {
                for id in &bounced {
                    let in_hand =
                        find_instance(ctx.state, id).is_some_and(|card| card.zone.z() == ZoneName::Hand);
                    if !in_hand {
                        continue;
                    }
                    let nerf = crate::effects::tune::degrade(json_as(json!({
                        "instanceId": id,
                        "times": CHAOS_MED_NERFS
                    })));
                    (nerf.apply)(ctx);
                }
            }),
        ]
    })
}

/// 8. The CN Golem token (Meditative #95.1), placed per R64.
fn build_golem() -> Effect {
    summon_token("golem", GOLEM_INDEX)
}

/// 9. R458, R1241: "for the rest of the game, at the start of each of your turns, cast a random Call to
///    Chaos", the caster's own start-of-turn effect from their next turn on, stacking — up to
///    `CALL_TO_CHAOS_ETERNAL_CAP`. A roll of it for a player at the cap resolves into nothing, with no
///    re-roll, as R87's recursion at the cap does.
fn build_eternal() -> Effect {
    entry("eternal", |ctx| {
        if eternal_effects_held(ctx.state, ctx.controller) >= CALL_TO_CHAOS_ETERNAL_CAP {
            return vec![];
        }
        vec![crate::effects::delay::for_rest_of_game(ForRestOfGameArgs {
            step: CHAOS_ETERNAL_STEP.into(),
            label: CHAOS_ETERNAL_LABEL.into(),
            hook: Some(CHAOS_ETERNAL_HOOK.into()),
            data: None,
            player: None,
        })]
    })
}

/// §8.8 row 95's ten entries, in the order the card prints them. Each `label` is the clause as the card
/// prints it (R432's cost words, R373's "deck"), which `chaosRolled` names to both players (R436).
pub const CHAOS_MED_EFFECTS: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "fuse",
        label: "Fuse your hand into one card and add 2 copies of it to your hand, all three of which cost (0)",
        build: build_fuse,
    },
    ChaosEffectDef {
        name: "cn",
        label: "Add 3 random CN cards to your hand, which cost (0)",
        build: build_cn,
    },
    ChaosEffectDef {
        name: "prime",
        label: "Add 2 random Prime cards to your hand, which cost (0)",
        build: build_prime,
    },
    ChaosEffectDef {
        name: "armor",
        label: "Your hero gains 8 Armor and you heal it 8",
        build: build_armor,
    },
    ChaosEffectDef {
        name: "jade",
        label: "Summon a Jade Beauty",
        build: build_jade,
    },
    ChaosEffectDef {
        name: "acclaimed",
        label: "Summon 3 random Acclaimed cards",
        build: build_acclaimed,
    },
    ChaosEffectDef {
        name: "bounce",
        label: "Bounce every enemy permanent, then Nerf each card bounced",
        build: build_bounce,
    },
    ChaosEffectDef {
        name: "golem",
        label: "Summon a CN Golem",
        build: build_golem,
    },
    ChaosEffectDef {
        name: "eternal",
        label: "For the rest of the game, at the start of each of your turns, cast a random Call to Chaos",
        build: build_eternal,
    },
    // 10. Core's recursion, the chain counting casts of every edition (R28, R87, R1240).
    ChaosEffectDef {
        name: "recast",
        label: "Cast a random Call to Chaos",
        build: cast_random_call_to_chaos,
    },
];
