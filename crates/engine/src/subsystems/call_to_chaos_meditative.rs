//! Meditative #95 Call to Chaos (Meditative Edition) (SPEC §8.8 row 95, R28, R87, R423, R436, R1240–R1244,
//! R3135001):
//! its table of ten effects, which Core #95's subsystem (`call_to_chaos.rs`) rolls, announces and
//! resolves, so one rule serves every edition.
//!
//! Every entry is built when it resolves (`lazy_part`), as Core's are: a recursion changes what each
//! later entry reads (R87), and a pause parks the rest (R113). Entry 9 is the engine's own rest-of-game
//! effect (R458, R1241), stepped by `eternal_step`, so the table never leans on a card file.
//!
//! Issue #4154 cut entry 4's Armor and heal 5 → 4 (R4154305). The table it had through v0.6.6h,
//! `CHAOS_MED_EFFECTS_V0_6_6H`, is the one the card's kept script rolls for those replays (R767000).
//! Issue #4285 priced entries 1 to 3's cards (1), from (0), and cut entry 4 to 3 and 3 (R4285005); the
//! table through v0.6.7k is `CHAOS_MED_EFFECTS_V0_6_7K`.

use serde_json::{Value, json};

use crate::config::{
    CALL_TO_CHAOS_ETERNAL_CAP, CHAOS_MED_ACCLAIMED, CHAOS_MED_CN_CARDS, CHAOS_MED_COST,
    CHAOS_MED_COST_V0_6_7K, CHAOS_MED_FUSE_COPIES, CHAOS_MED_HEAL, CHAOS_MED_HEAL_V0_6_6H,
    CHAOS_MED_HEAL_V0_6_7K, CHAOS_MED_HERO_ARMOR, CHAOS_MED_HERO_ARMOR_V0_6_6H, CHAOS_MED_HERO_ARMOR_V0_6_7K,
    CHAOS_MED_NERFS, CHAOS_MED_PRIME_CARDS,
};
use crate::effects::add_to_hand::AddToHandArgs;
use crate::effects::delay::ForRestOfGameArgs;
use crate::effects::targets::{BoardScope, cards_in_scope};
use crate::effects::tune::{TuneDirection, tune_once};
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, EffectPart, Hook, hook};
use crate::state::{GameState, ModifierKind, find_instance, find_instance_mut};
use crate::subsystems::call_to_chaos::{ChaosEffectDef, cast_random_call_to_chaos};
use crate::subsystems::fuse::{FuseArgs, HandPrice};
use crate::wire::{PlayerId, SetName, ZoneName};

/// §7: the tokens this edition summons, by their indices in the Meditative set (B2.2).
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

/// Entries 2 and 3: independent picks of a pool (R60), each a fresh hand card at `cost` ((1) since R4285005,
/// (0) through v0.6.7k); a full hand burns (§2.4).
fn add_priced(name: &'static str, pool: Value, count: i32, cost: i32) -> Effect {
    entry(name, move |_ctx| {
        vec![crate::effects::add_to_hand::add_random_from_catalog(json_as(
            json!({
                "query": pool,
                "count": count,
                "costOverride": cost,
            }),
        ))]
    })
}

/// Entries 5 and 8: a Meditative token by its index, placed per R64; nothing on a full row.
fn summon_token(name: &'static str, index: &'static str) -> Effect {
    entry(name, move |_ctx| {
        let def_id = crate::catalog::def_id_by_index(TOKEN_SET, index);
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

/// R1241: entry 9's step, a random base Call to Chaos cast with no `self`, so a new chain (R28's cap).
pub fn eternal_step() -> Hook {
    hook(|_ctx| vec![cast_random_call_to_chaos()])
}

/// 1. R1242, R3135001: every non-Immutable card in your hand (R23), in hand order, fused into one fresh hand
///    card (R77, R102, R469) plus `CHAOS_MED_FUSE_COPIES` copy (R57's riders), both costing `cost`; a lone
///    card is not fused but takes the price and the copy. Its `fused` event is hidden as the hand is (R470).
fn fuse_entry(cost: i32) -> Effect {
    entry("fuse", move |_ctx| {
        vec![Effect::new("callToChaosMeditative:fuseHand", move |ctx| {
            let controller = ctx.controller;
            let state: &GameState = ctx.state;
            let fusable: Vec<_> = state.players[controller]
                .hand
                .iter()
                .filter(|card| !crate::layers::is_immutable(state, card))
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
                live.cost_override = Some(cost);
            }
            for _ in 0..CHAOS_MED_FUSE_COPIES {
                let copy = crate::effects::add_to_hand::add_to_hand(AddToHandArgs {
                    copy_of: Some(kept_id.clone()),
                    cost_override: Some(cost),
                    ..AddToHandArgs::default()
                });
                (copy.apply)(ctx);
            }
        })]
    })
}

/// 1. Both cards at (1) since R4285005.
fn build_fuse() -> Effect {
    fuse_entry(CHAOS_MED_COST)
}

/// R4285005, R767000: entry 1 through v0.6.7k, both cards at (0).
fn build_fuse_v0_6_7k() -> Effect {
    fuse_entry(CHAOS_MED_COST_V0_6_7K)
}

/// 2. Non-token CN cards of every set that ships (R380, R1420), at (1) since R4285005.
fn build_cn() -> Effect {
    add_priced(
        "cn",
        json!({ "tags": ["CN"] }),
        CHAOS_MED_CN_CARDS,
        CHAOS_MED_COST,
    )
}

/// R4285005, R767000: entry 2 through v0.6.7k, at (0).
fn build_cn_v0_6_7k() -> Effect {
    add_priced(
        "cn",
        json!({ "tags": ["CN"] }),
        CHAOS_MED_CN_CARDS,
        CHAOS_MED_COST_V0_6_7K,
    )
}

/// 3. The Prime pool, which holds the Prime tokens (R1421), at (1) since R4285005.
fn build_prime() -> Effect {
    add_priced(
        "prime",
        json!({ "tags": ["Prime"] }),
        CHAOS_MED_PRIME_CARDS,
        CHAOS_MED_COST,
    )
}

/// R4285005, R767000: entry 3 through v0.6.7k, at (0).
fn build_prime_v0_6_7k() -> Effect {
    add_priced(
        "prime",
        json!({ "tags": ["Prime"] }),
        CHAOS_MED_PRIME_CARDS,
        CHAOS_MED_COST_V0_6_7K,
    )
}

/// 4. §6.1's Armor kept for the rest of the game (C+ #46's, R124), then an uncapped hero heal (§6.3), both
///    3 since R4285005 (4 from R4154305 through v0.6.7k, 5 from R3135001 through v0.6.6h).
fn armor_entry(armor: i32, heal: i32) -> Effect {
    entry("armor", move |_ctx| {
        vec![
            crate::effects::perks::gain_hero_armor(json_as(json!({ "amount": armor }))),
            crate::effects::heal(json_as(json!({
                "target": { "of": "selfHero" },
                "amount": heal
            }))),
        ]
    })
}

fn build_armor() -> Effect {
    armor_entry(CHAOS_MED_HERO_ARMOR, CHAOS_MED_HEAL)
}

/// R4285005, R767000: entry 4 through v0.6.7k, 4 Armor and a heal of 4.
fn build_armor_v0_6_7k() -> Effect {
    armor_entry(CHAOS_MED_HERO_ARMOR_V0_6_7K, CHAOS_MED_HEAL_V0_6_7K)
}

/// R4154305, R767000: entry 4 through v0.6.6h, 5 Armor and a heal of 5.
fn build_armor_v0_6_6h() -> Effect {
    armor_entry(CHAOS_MED_HERO_ARMOR_V0_6_6H, CHAOS_MED_HEAL_V0_6_6H)
}

/// 5. The Jade Beauty token's base face (Meditative #39.5); the Jade Counter is not touched.
fn build_jade() -> Effect {
    summon_token("jade", JADE_BEAUTY_INDEX)
}

/// 6. R1243, R3135001: two independent picks (R60) of the non-token Acclaimed permanents, each summoned per R64
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

/// 7. R1244: every enemy permanent acting on the field bounced together (R747, R11, R4), then each one
///    that reached a hand Nerfed (a Degrade, R386), hidden there (R177, R440).
fn build_bounce() -> Effect {
    entry("bounce", |ctx| {
        let scope: BoardScope = json_as(json!({ "side": "enemy", "rows": ["units", "backrow"] }));
        let bounced: Vec<String> = cards_in_scope(ctx, &scope)
            .iter()
            .map(|card| card.id.clone())
            .collect();
        vec![
            crate::effects::bounce_all(scope),
            // Aimed at in the hand, not the stay it left (R174), so each Nerf is a `tune_once` (B3.4 rule 1).
            Effect::new("callToChaosMeditative:nerfBounced", move |ctx| {
                for id in &bounced {
                    let Some(card) = find_instance(ctx.state, id)
                        .filter(|card| card.zone.z() == ZoneName::Hand)
                        .cloned()
                    else {
                        continue;
                    };
                    for _ in 0..CHAOS_MED_NERFS {
                        tune_once(ctx, &card, TuneDirection::Degrade, true);
                    }
                }
            }),
        ]
    })
}

/// 8. The CN Golem token (Meditative #95.1), placed per R64.
fn build_golem() -> Effect {
    summon_token("golem", GOLEM_INDEX)
}

/// 9. R458, R1241: the rest-of-game effect, stacking up to `CALL_TO_CHAOS_ETERNAL_CAP`; at the cap it
///    resolves into nothing, with no re-roll, as R87's recursion does.
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
            at_end: None,
        })]
    })
}

/// §8.8 row 95's ten entries in printed order, each `label` the printed clause (R432's cost words,
/// R373's "deck") that `chaosRolled` names to both players (R436). Issue #4154 cut entry 4 (R4154305); issue #4285
/// priced entries 1 to 3's cards (1) and cut entry 4 again (R4285005).
pub const CHAOS_MED_EFFECTS: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "fuse",
        label: "Fuse your hand into one card and add 1 copy of it to your hand, both of which cost (1)",
        build: build_fuse,
    },
    ChaosEffectDef {
        name: "cn",
        label: "Add 2 random CN cards to your hand, which cost (1)",
        build: build_cn,
    },
    ChaosEffectDef {
        name: "prime",
        label: "Add 2 random Prime cards to your hand, which cost (1)",
        build: build_prime,
    },
    ChaosEffectDef {
        name: "armor",
        label: "Your hero gains 3 Armor and you heal it 3",
        build: build_armor,
    },
    ChaosEffectDef {
        name: "jade",
        label: "Summon a Jade Beauty",
        build: build_jade,
    },
    ChaosEffectDef {
        name: "acclaimed",
        label: "Summon 2 random Acclaimed cards",
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

/// R4285005, R767000: §8.8 row 95's ten entries as they were through v0.6.7k, before issue #4285 priced entries
/// 1 to 3's cards (1) and cut entry 4 to 3 and 3: the card's kept script (`script_v0_6_7k`) rolls it for those
/// replays.
pub const CHAOS_MED_EFFECTS_V0_6_7K: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "fuse",
        label: "Fuse your hand into one card and add 1 copy of it to your hand, both of which cost (0)",
        build: build_fuse_v0_6_7k,
    },
    ChaosEffectDef {
        name: "cn",
        label: "Add 2 random CN cards to your hand, which cost (0)",
        build: build_cn_v0_6_7k,
    },
    ChaosEffectDef {
        name: "prime",
        label: "Add 2 random Prime cards to your hand, which cost (0)",
        build: build_prime_v0_6_7k,
    },
    ChaosEffectDef {
        name: "armor",
        label: "Your hero gains 4 Armor and you heal it 4",
        build: build_armor_v0_6_7k,
    },
    ChaosEffectDef {
        name: "jade",
        label: "Summon a Jade Beauty",
        build: build_jade,
    },
    ChaosEffectDef {
        name: "acclaimed",
        label: "Summon 2 random Acclaimed cards",
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

/// R4154305, R767000: §8.8 row 95's ten entries as they were through v0.6.6h, before issue #4154 cut entry 4's
/// Armor and heal: the card's kept script (`script_v0_6_6h`) rolls it for those replays.
pub const CHAOS_MED_EFFECTS_V0_6_6H: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "fuse",
        label: "Fuse your hand into one card and add 1 copy of it to your hand, both of which cost (0)",
        build: build_fuse_v0_6_7k,
    },
    ChaosEffectDef {
        name: "cn",
        label: "Add 2 random CN cards to your hand, which cost (0)",
        build: build_cn_v0_6_7k,
    },
    ChaosEffectDef {
        name: "prime",
        label: "Add 2 random Prime cards to your hand, which cost (0)",
        build: build_prime_v0_6_7k,
    },
    ChaosEffectDef {
        name: "armor",
        label: "Your hero gains 5 Armor and you heal it 5",
        build: build_armor_v0_6_6h,
    },
    ChaosEffectDef {
        name: "jade",
        label: "Summon a Jade Beauty",
        build: build_jade,
    },
    ChaosEffectDef {
        name: "acclaimed",
        label: "Summon 2 random Acclaimed cards",
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
