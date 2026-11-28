//! Felinors vs Humans #100 Call to Chaos (Felinors vs. Humans Edition) (SPEC §8.14 row 100, FV2 #100, R28,
//! R87, R423, R436, R5262000): its table of ten effects, which Core #95's subsystem (`call_to_chaos.rs`)
//! rolls, announces and resolves as it does every other edition's, so one rule serves every edition. Every
//! entry is built when it resolves (`lazy_part`), as the other editions' are (R87, R113), and reads the
//! card's declared numbers with `param` as it resolves (CLAUDE.md rule 9).
//!
//! The pools are the non-token cards of every set, held ones included (`CATALOG_SETS`, as R3570000's), so a
//! held set's Felinors and Humans are drawn as a shipped set's are; each summon is its own draw (R60), fewer
//! with fewer open zones (R129), and none runs a Cry (R1).

use serde_json::{Value, json};

use crate::catalog::{HookFilter, def_id_by_index};
use crate::params::param;
use crate::prelude::json_as;
use crate::script::{Effect, EffectContext, EffectPart};
use crate::subsystems::call_to_chaos::{ChaosEffectDef, cast_random_other_call_to_chaos};
use crate::wire::{CATALOG_SETS, Keyword, KeywordKind, SetName, Tag};

/// §8.14: the set whose indices the table names (B2.2).
const SET: SetName = SetName::FelinorsVsHumans;
/// Entry 8: FvH #115 Call to Chaos Vending Machine.
const VENDING_MACHINE_INDEX: &str = "115";
/// Entry 9: FvH #100.1 Felinor Golem and #100.2 Human Golem.
const FELINOR_GOLEM_INDEX: &str = "100.1";
const HUMAN_GOLEM_INDEX: &str = "100.2";

/// The card's declared numbers (`params` in `catalog.json`): entry 1's Felinors, entry 2's Humans and
/// entry 4's Plagiarized casts.
const FELINORS_KEY: &str = "felinors";
const HUMANS_KEY: &str = "humans";
const PLAGIARIZED_KEY: &str = "plagiarized";

/// `callToChaosFvh:<name>`, the kind of an entry's part (an effect's kind is a static name).
fn entry_kind(name: &str) -> &'static str {
    match name {
        "felinors" => "callToChaosFvh:felinors",
        "humans" => "callToChaosFvh:humans",
        "fragile" => "callToChaosFvh:fragile",
        "plagiarized" => "callToChaosFvh:plagiarized",
        "radiantFelinors" => "callToChaosFvh:radiantFelinors",
        "radiantHumans" => "callToChaosFvh:radiantHumans",
        "giga" => "callToChaosFvh:giga",
        "vendingMachine" => "callToChaosFvh:vendingMachine",
        "golems" => "callToChaosFvh:golems",
        "recast" => "callToChaosFvh:recast",
        _ => "callToChaosFvh",
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

/// R5262000: the pool of every non-token card carrying `tag`, held sets included, as S+ #142's is.
fn tag_pool(tag: Tag) -> Value {
    json!({ "tags": [tag], "set": CATALOG_SETS })
}

/// R5262000: the Units of `tag_pool(tag)`.
fn tribe_units(tag: Tag) -> Value {
    json!({ "type": "Unit", "tags": [tag], "set": CATALOG_SETS })
}

/// R5262000, R5239002: entry 1's pool, a Felinor Unit whose base face has a Death or prints Reborn.
fn fvh_chaos_felinors_query() -> Value {
    let mut query = tribe_units(Tag::Felinor);
    query["anyOf"] = json!([
        { "hooks": [HookFilter::Death] },
        { "keywords": [KeywordKind::Reborn] },
    ]);
    query
}

/// R5262000, R5239002: entry 2's pool, a Human Unit whose base face has an Aura.
fn fvh_chaos_humans_query() -> Value {
    let mut query = tribe_units(Tag::Human);
    query["hooks"] = json!([HookFilter::Aura]);
    query
}

/// `count` random summons of `query`, each its own draw (R60), placed per R64 with no Cry (R1).
fn summon_many(query: &Value, count: i32) -> Vec<Effect> {
    (0..count.max(0))
        .map(|_| crate::effects::summon_random(json_as(json!({ "query": query.clone() }))))
        .collect()
}

/// 1. R5262000: `felinors` random Felinor Units with a Death or Reborn.
fn build_felinors() -> Effect {
    entry("felinors", |ctx| {
        summon_many(&fvh_chaos_felinors_query(), param(&*ctx, FELINORS_KEY))
    })
}

/// 2. R5262000: `humans` random Human Units with an Aura.
fn build_humans() -> Effect {
    entry("humans", |ctx| {
        summon_many(&fvh_chaos_humans_query(), param(&*ctx, HUMANS_KEY))
    })
}

/// 3. R5262000, R5236010: every enemy Unit card on the field, in its owner's hand and in their deck gains
///    Fragile, carried onto the field from a hand or a deck as `buffCards`' stats are (R440); a Field Spell
///    gains none.
fn build_fragile() -> Effect {
    entry("fragile", |_ctx| {
        vec![crate::effects::grant_keyword_cards(json_as(json!({
            "scope": { "side": "enemy", "zones": ["field", "hand", "library"], "types": ["Unit"] },
            "keyword": Keyword::Fragile,
        })))]
    })
}

/// 4. R5262000, R5235002: `plagiarized` random casts of cards tagged Plagiarized (R452): a Spell resolves,
///    a permanent is played free into a zone (R70), every choice random (R656).
fn build_plagiarized() -> Effect {
    entry("plagiarized", |ctx| {
        vec![crate::effects::cast::cast_random(json_as(json!({
            "query": tag_pool(Tag::Plagiarized),
            "count": param(&*ctx, PLAGIARIZED_KEY).max(0),
        })))]
    })
}

/// 5 and 6. R5262000, R1080: each empty unit zone of its caster's filled with a Radiant Unit of `own`, then
/// each of the enemy's with a base Unit of `theirs`, Units only.
fn fill_both_boards(own: Tag, theirs: Tag) -> Vec<Effect> {
    vec![
        crate::effects::fill_board_random(json_as(json!({
            "query": tag_pool(own),
            "radiant": true,
        }))),
        crate::effects::fill_board_random(json_as(json!({
            "query": tag_pool(theirs),
            "player": "enemy",
        }))),
    ]
}

/// 5. Radiant Felinors for you, Humans for them.
fn build_radiant_felinors() -> Effect {
    entry("radiantFelinors", |_ctx| fill_both_boards(Tag::Felinor, Tag::Human))
}

/// 6. Radiant Humans for you, Felinors for them.
fn build_radiant_humans() -> Effect {
    entry("radiantHumans", |_ctx| fill_both_boards(Tag::Human, Tag::Felinor))
}

/// 7. R5262000, R3546002: every enemy Unit and Field Spell card in its owner's hand, deck and graveyard is
///    made GIGA, entering the field GIGA; the field's cards are not.
fn build_giga() -> Effect {
    entry("giga", |_ctx| {
        vec![crate::effects::grant_keyword_cards(json_as(json!({
            "scope": {
                "side": "enemy",
                "zones": ["hand", "library", "graveyard"],
                "types": ["Unit", "Field Spell"],
            },
            "keyword": Keyword::Giga,
        })))]
    })
}

/// 8. R5262000: a Radiant FvH #115 Call to Chaos Vending Machine, a Field Spell, into its caster's backrow
///    (R64); a full backrow takes none.
fn build_vending_machine() -> Effect {
    entry("vendingMachine", |_ctx| match def_id_by_index(SET, VENDING_MACHINE_INDEX) {
        None => vec![],
        Some(def_id) => vec![crate::effects::summon::summon(json_as(json!({
            "defId": def_id,
            "radiant": true,
        })))],
    })
}

/// 9. FvH #100.1 Felinor Golem, then #100.2 Human Golem, each placed per R64, on their base faces.
fn build_golems() -> Effect {
    entry("golems", |_ctx| {
        [FELINOR_GOLEM_INDEX, HUMAN_GOLEM_INDEX]
            .into_iter()
            .filter_map(|index| def_id_by_index(SET, index))
            .map(|def_id| crate::effects::summon::summon(json_as(json!({ "defId": def_id }))))
            .collect()
    })
}

/// 10. R5262000, R3570005: Core's recursion, the chain counting casts of every edition (R28, R87), that never
///     draws this edition's own card.
fn build_recast() -> Effect {
    cast_random_other_call_to_chaos()
}

/// §8.14 row 100's ten entries, in the order the card prints them. Each `label` is the clause as the card
/// prints it at its printed numbers, which `chaosRolled` names to both players (R436); a tuned `param` does
/// not change it (as R3570000's).
pub const CHAOS_FVH_EFFECTS: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "felinors",
        label: "Summon 5 random Felinors with Death or Reborn",
        build: build_felinors,
    },
    ChaosEffectDef {
        name: "humans",
        label: "Summon 5 random Humans with an Aura",
        build: build_humans,
    },
    ChaosEffectDef {
        name: "fragile",
        label: "Give enemy Units on the field, in their hand and in their deck Fragile",
        build: build_fragile,
    },
    ChaosEffectDef {
        name: "plagiarized",
        label: "Cast 5 random Plagiarized cards",
        build: build_plagiarized,
    },
    ChaosEffectDef {
        name: "radiantFelinors",
        label: "Fill your board with random Radiant Felinors and your opponent's with random Humans",
        build: build_radiant_felinors,
    },
    ChaosEffectDef {
        name: "radiantHumans",
        label: "Fill your board with random Radiant Humans and your opponent's with random Felinors",
        build: build_radiant_humans,
    },
    ChaosEffectDef {
        name: "giga",
        label: "Make enemy Units and Field Spells in their hand, deck and graveyard GIGA",
        build: build_giga,
    },
    ChaosEffectDef {
        name: "vendingMachine",
        label: "Summon a Radiant Call to Chaos Vending Machine",
        build: build_vending_machine,
    },
    ChaosEffectDef {
        name: "golems",
        label: "Summon a Felinor Golem and a Human Golem",
        build: build_golems,
    },
    ChaosEffectDef {
        name: "recast",
        label: "Cast a random Call to Chaos",
        build: build_recast,
    },
];
