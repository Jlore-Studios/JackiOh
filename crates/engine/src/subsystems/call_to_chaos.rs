//! Call to Chaos (SPEC §8 #95, R28, R87, R423, R436): the ten effects, the roll that picks them, and the
//! capped recursion the tenth one drives.
//!
//! The card is a Spell whose base text is "one random effect" and whose Radiant text, since patch
//! v0.2.0, is "three different random effects, resolved in the order listed" (R423, the shape Classic+
//! #73 has: one rule serves both editions). The recursion is one entry of the list like any other: the
//! Radiant face rolls it only when it falls among its three, and it resolves where the list puts it,
//! last. Every effect here is built from the effects library, so #95's own file is a one-line hook that
//! returns `[callToChaos()]` and stays a list of effects, like every other card (CLAUDE.md rule 5).
//!
//! Three things need care. First, each effect reads the board when it *resolves*, not when the hook
//! builds it: a nested cast can draw cards, summon units and change costs in between, so "your hand
//! becomes Radiant" and "draw your whole library" must see the hand and library as they are at that
//! moment (R58's "the library size when the effect starts"). Every effect is therefore one lazy
//! wrapper that builds its sub-effects inside `apply`. Second, the chain length is game state, not a
//! module variable: it lives on the cast instance's `memory` (§10.1), so a paused, serialized game
//! resumes with the same cap left and two independent Calls in one turn never share a counter. Third,
//! what was rolled is told to both players before any of it resolves (`chaosRolled`, R436), once: a
//! roll a pause interrupted is rebuilt from the part's memo and is not announced a second time.
//!
//! Port of `packages/engine/src/subsystems/callToChaos.ts`. The numbers (`CHAOS_UNIT_COUNT` …
//! `CHAOS_BACKROW_CARDS`) live in `crate::config` (CLAUDE.md rule 9, SURFACE §6.4) and
//! `subsystems/mod.rs` re-exports them under TS's `subsystems.X` path. The effect tables are
//! `const` slices of plain data with `fn` builders, so an edition's list (Classic+ #73's
//! `CHAOS_PLUS_EFFECTS`) is a `&'static [ChaosEffectDef]` like Core's.

use serde_json::{Value, json};

use crate::config::{
    CALL_TO_CHAOS_CHAIN_CAP, CALL_TO_CHAOS_RADIANT_EFFECTS, CHAOS_ADDED_CARDS, CHAOS_BACKROW_CARDS,
    CHAOS_COST_DISCOUNT, CHAOS_HEAL, CHAOS_MANA, CHAOS_RUSH_TOKENS, CHAOS_UNIT_COST, CHAOS_UNIT_COUNT,
};
use crate::prelude::json_as;
use crate::rng::Rng;
use crate::script::{Effect, EffectContext, EffectPart, EngineSink, Memo};
use crate::state::{CardInstance, new_instance};
use crate::wire::{GameEvent, Selection, SetName, Tag, Zone};

/// §5, §8 #95: the tag the Call to Chaos family carries, and so the recursion's pool (§10.7).
pub const CHAOS_TAG: Tag = Tag::CallToChaos;
/// §7: the tokens #95 summons, by catalog index.
const RUSH_TOKEN_INDEX: &str = "T-rush";
const CHAOS_GOLEM_INDEX: &str = "95.1";
/// B2.2: the set whose indices those are, since an index is unique only within its set.
const TOKEN_SET: SetName = SetName::Core;

/// §5.1: the backrow half of the catalog — "Field Spells or Traps (Field Traps included)" (§8).
fn chaos_backrow_query() -> Value {
    json!({ "type": ["Field Spell", "Trap", "Field Trap"] })
}

/// R28: how many casts of the chain this instance is. The played #95 has no entry and so is 0; the
/// card it casts is 1, the card that one casts is 2, and the chain stops once a cast would be the
/// 21st. It lives in `memory` because §10.1 puts everything a card must remember on the instance,
/// which keeps the counter serializable and per-chain.
pub const CHAOS_CHAIN_KEY: &str = "chaosChain";

pub fn chaos_chain_of(instance: Option<&CardInstance>) -> i32 {
    // TS: `typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.trunc(value) : 0`
    // (SURFACE §4.4.10: the bag is re-read defensively).
    let value = instance.and_then(|card| card.memory.get(CHAOS_CHAIN_KEY)).and_then(Value::as_f64);
    match value {
        Some(n) if n.is_finite() && n > 0.0 => n.trunc() as i32,
        _ => 0,
    }
}

/// R28: the hard stop. At the cap the recursion effect resolves and does nothing at all.
pub fn chaos_chain_cap_reached(depth: i32) -> bool {
    depth >= CALL_TO_CHAOS_CHAIN_CAP
}

fn sink_of<'c>(ctx: &'c mut EffectContext<'_>) -> EngineSink<'c> {
    ctx.sink.reborrow()
}

/// §7: a token summon needs the token's def id, which the catalog holds under its index in Core.
fn token_def_id(index: &str) -> Option<String> {
    crate::catalog::def_by_index(TOKEN_SET, index).map(|def| def.id.clone())
}

/// The kind of one entry's lazy part, TS's `` `callToChaos:${name}` ``: an effect's kind is a
/// `&'static str`, so the ten are spelled out.
fn chaos_kind(name: ChaosEffectName) -> &'static str {
    match name {
        ChaosEffectName::Units => "callToChaos:units",
        ChaosEffectName::Heal => "callToChaos:heal",
        ChaosEffectName::Draw => "callToChaos:draw",
        ChaosEffectName::Add => "callToChaos:add",
        ChaosEffectName::Radiant => "callToChaos:radiant",
        ChaosEffectName::Tokens => "callToChaos:tokens",
        ChaosEffectName::Discount => "callToChaos:discount",
        ChaosEffectName::Golem => "callToChaos:golem",
        ChaosEffectName::Backrow => "callToChaos:backrow",
        ChaosEffectName::Recast => "callToChaos:recast",
    }
}

/// One of the ten effects, built when it resolves rather than when the hook returns it, so every
/// state read happens after the effects before it have landed. It is a part of the list that holds it
/// (`resolve.lazyPart`), so an effect inside it that asks — a draw whose cast asks, in "draw your
/// whole library and gain 4 mana" — pauses the rest of it until the answer (R113): the mana waits
/// for the draw, as the partner waits for the recursion (R87).
fn chaos_effect(
    name: ChaosEffectName,
    build: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static,
) -> Effect {
    crate::resolve::lazy_part(chaos_kind(name), move |ctx, _memo| EffectPart {
        effects: build(ctx),
        memo: None,
    })
}

/// Point an effect from the library at one card by id. The library names a target through the
/// selections a play carried (R81), so a pick a script made itself travels the same way instead of
/// opening a second targeting path — and `setCostMod` stays the only place a cost change is written.
fn on_instance(effect: Effect, instance_id: String) -> Effect {
    let kind = effect.kind;
    Effect::new(kind, move |ctx| {
        // TS `effect.apply({ ...ctx, targets: [{ pick: "instance", instanceId }] })`: the inner effect
        // sees only this one selection, and the context's own targets are as they were afterwards.
        let selection = Selection::Instance {
            instance_id: instance_id.clone(),
        };
        let saved = std::mem::replace(&mut ctx.targets, vec![selection]);
        (effect.apply)(ctx);
        ctx.targets = saved;
    })
}

// ---------------------------------------------------------------------------
// The ten effects, in the order §8 #95 lists them.
// ---------------------------------------------------------------------------

/// 1. "Summon 3 random 3-cost Units": three independent picks (R60), placed per R64. Each pick is its
///    own `summonRandom`, which draws only when its unit has a zone to go to (R129), so a full row takes
///    no draw for a summon that cannot land.
pub fn summon_random_three_cost_units() -> Effect {
    chaos_effect(ChaosEffectName::Units, |_ctx| {
        (0..CHAOS_UNIT_COUNT)
            .map(|_| {
                crate::effects::summon_random(json_as(json!({
                    "query": { "type": "Unit", "cost": CHAOS_UNIT_COST }
                })))
            })
            .collect()
    })
}

/// 2. "Heal your hero 30": §6.3 gives a hero heal no cap, so this may pass 30 health.
pub fn heal_hero_thirty() -> Effect {
    chaos_effect(ChaosEffectName::Heal, |_ctx| {
        vec![crate::effects::heal(json_as(json!({
            "target": { "of": "selfHero" },
            "amount": CHAOS_HEAL
        })))]
    })
}

/// 3. "Draw your whole library and gain 4 mana": R58 fixes the count at the library size when the
///    effect starts, so a cast-on-draw card drawn along the way cannot lengthen the draw, and an empty
///    library draws nothing at all rather than taking a fatigue hit (§2.4, R3).
pub fn draw_library_and_gain_mana() -> Effect {
    chaos_effect(ChaosEffectName::Draw, |ctx| {
        let count = ctx.sink.state.players[ctx.controller].library.len();
        vec![
            crate::effects::draw(json_as(json!({ "count": count }))),
            crate::effects::gain_mana(json_as(json!({ "amount": CHAOS_MANA }))),
        ]
    })
}

/// 4. "Add 3 random cards to hand costing 0": three independent picks (R60) from the whole catalog,
///    which §5.1 keeps free of tokens and of the generating card — "never include the generating card's
///    own definition, unless the card names the pool itself", and only the recursion names its pool —
///    so no Call to Chaos is added. The 0 is a `costOverride` the card takes on reaching the hand (R65);
///    a full hand burns what it cannot take (§2.4, R4), without the price.
pub fn add_random_zero_cost_cards() -> Effect {
    chaos_effect(ChaosEffectName::Add, |_ctx| {
        vec![crate::effects::add_random_from_catalog(json_as(json!({
            "count": CHAOS_ADDED_CARDS,
            "costOverride": 0
        })))]
    })
}

/// 5. "Your hand becomes Radiant": every card in hand right now (§5.2). A card that is already
///    Radiant is untouched, since the flag is never unset (§6.3 Make Radiant).
pub fn make_hand_radiant() -> Effect {
    chaos_effect(ChaosEffectName::Radiant, |ctx| {
        ctx.sink.state.players[ctx.controller]
            .hand
            .iter()
            .map(|card| crate::effects::set_radiant(json_as(json!({ "instanceId": card.id }))))
            .collect()
    })
}

/// 6. "Summon five Radiant Rush Tokens": the §7 Rush Token's own Radiant face (6/6), five separate
///    summons, so a board with fewer free zones simply takes fewer (R64) instead of failing as a whole.
pub fn summon_rush_tokens() -> Effect {
    chaos_effect(ChaosEffectName::Tokens, |_ctx| {
        let Some(def_id) = token_def_id(RUSH_TOKEN_INDEX) else {
            return vec![];
        };
        (0..CHAOS_RUSH_TOKENS)
            .map(|_| crate::effects::summon(json_as(json!({ "defId": def_id, "radiant": true }))))
            .collect()
    })
}

/// 7. "Every card in your hand and library costs 2 less": the cards that are there when the effect
///    resolves, each getting a permanent `costMod` that travels with it between zones (R78). It changes
///    those cards, not the player, so a card drawn afterwards still pays full price.
pub fn discount_hand_and_library() -> Effect {
    chaos_effect(ChaosEffectName::Discount, |ctx| {
        let side = &ctx.sink.state.players[ctx.controller];
        let discount = crate::effects::set_cost_mod(json_as(json!({
            "target": { "of": "chosen" },
            "amount": -CHAOS_COST_DISCOUNT
        })));
        side.hand
            .iter()
            .chain(side.library.iter())
            .map(|card| on_instance(discount.clone(), card.id.clone()))
            .collect()
    })
}

/// 8. "Summon a Chaos Golem": the 10/10 token of §7 (index 95.1), placed per R64.
pub fn summon_chaos_golem() -> Effect {
    chaos_effect(ChaosEffectName::Golem, |_ctx| match token_def_id(CHAOS_GOLEM_INDEX) {
        None => vec![],
        Some(def_id) => vec![crate::effects::summon(json_as(json!({ "defId": def_id })))],
    })
}

/// 9. "Summon 5 random Field Spells or Traps (Field Traps included, traps face-down) into your
///    backrow": five independent picks (R60). `summonRandom` sends every one of those types to the
///    backrow, leaves a Trap or Field Trap face-down while a Field Spell is public (§3.2, R33), and draws
///    only for a summon that has a zone to go to (R129).
pub fn summon_random_backrow() -> Effect {
    chaos_effect(ChaosEffectName::Backrow, |_ctx| {
        (0..CHAOS_BACKROW_CARDS)
            .map(|_| crate::effects::summon_random(json_as(json!({ "query": chaos_backrow_query() }))))
            .collect()
    })
}

/// 10. "Cast a random Call to Chaos": a Cast per R70 — free, counted as a play, running the card's
///     own script. The pool is every card tagged Call to Chaos in every set (R380: a pool that names no set
///     reaches every set), so it holds both editions, and the card cast is the *base* form even when a
///     Radiant Call cast it (R28); the new card is Radiant only if something later makes it so.
///
/// R28 caps the chain at CALL_TO_CHAOS_CHAIN_CAP casts of either edition. The cap is a hard stop: at
/// the cap this effect resolves into nothing, and no re-roll replaces it (R87, R423).
///
/// The cast card is a real generated card, like the ones "add 3 random cards to hand" makes (R60), so
/// §10.5 step 7 sends it to the caster's graveyard when it has resolved (R87), which is what feeds
/// Gravedigger and Reminisce down a long chain.
pub fn cast_random_call_to_chaos() -> Effect {
    Effect::new("callToChaos:recast", |ctx| {
        let depth = chaos_chain_of(ctx.live_self());
        if chaos_chain_cap_reached(depth) {
            return;
        }

        let pool = crate::catalog::query(&json_as(json!({ "tags": [CHAOS_TAG] })));
        let Some(def_id) = crate::catalog::pick_generated(&mut *ctx.sink.rng, &pool, None).map(|def| def.id.clone())
        else {
            return;
        };

        let controller = ctx.controller;
        let mut card = new_instance(&mut *ctx.sink.state, &def_id, controller, Zone::Resolving { player: controller });
        card.memory.insert(CHAOS_CHAIN_KEY.to_string(), json!(depth + 1));
        crate::resolve::cast_card(&mut sink_of(ctx), &card, crate::resolve::CastOptions::default());
    })
}

// ---------------------------------------------------------------------------
// The roll (§8 #95, R28, R423).
// ---------------------------------------------------------------------------

/// Core #95's ten entries by name. Another edition's list (Classic+ #73) names its own.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum ChaosEffectName {
    Units,
    Heal,
    Draw,
    Add,
    Radiant,
    Tokens,
    Discount,
    Golem,
    Backrow,
    Recast,
}

impl ChaosEffectName {
    /// Every name, in the order §8 #95 lists them.
    pub const ALL: &'static [ChaosEffectName] = &[
        ChaosEffectName::Units,
        ChaosEffectName::Heal,
        ChaosEffectName::Draw,
        ChaosEffectName::Add,
        ChaosEffectName::Radiant,
        ChaosEffectName::Tokens,
        ChaosEffectName::Discount,
        ChaosEffectName::Golem,
        ChaosEffectName::Backrow,
        ChaosEffectName::Recast,
    ];

    /// The literal TS writes.
    pub fn as_str(self) -> &'static str {
        match self {
            ChaosEffectName::Units => "units",
            ChaosEffectName::Heal => "heal",
            ChaosEffectName::Draw => "draw",
            ChaosEffectName::Add => "add",
            ChaosEffectName::Radiant => "radiant",
            ChaosEffectName::Tokens => "tokens",
            ChaosEffectName::Discount => "discount",
            ChaosEffectName::Golem => "golem",
            ChaosEffectName::Backrow => "backrow",
            ChaosEffectName::Recast => "recast",
        }
    }
}

/// One entry of an edition's list (TS `ChaosEffectDef`): plain data and a builder, so a list is a
/// `const` slice.
#[derive(Clone, Copy)]
pub struct ChaosEffectDef {
    /// Unique within its list: what a roll a pause interrupted is kept by (the part's memo).
    pub name: &'static str,
    /// The clause as the card prints it: what `chaosRolled` names to both players (R436).
    pub label: &'static str,
    pub build: fn() -> Effect,
}

impl std::fmt::Debug for ChaosEffectDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChaosEffectDef")
            .field("name", &self.name)
            .field("label", &self.label)
            .finish()
    }
}

/// R28: the effect that drives the chain, "cast a random Call to Chaos" (R423: rolled like any other).
pub const CHAOS_RECURSION: ChaosEffectName = ChaosEffectName::Recast;

/// The ten effects of §8 #95, in the order the card lists them. Each `label` is the clause as the
/// card prints it (R432's cost words, R373's "deck"), which is what `chaosRolled` names to both
/// players (R436).
pub const CHAOS_EFFECTS: &[ChaosEffectDef] = &[
    ChaosEffectDef {
        name: "units",
        label: "Summon 3 random (3) Cost Units",
        build: summon_random_three_cost_units,
    },
    ChaosEffectDef {
        name: "heal",
        label: "Heal your hero 30",
        build: heal_hero_thirty,
    },
    ChaosEffectDef {
        name: "draw",
        label: "Draw your whole deck and gain 4 mana",
        build: draw_library_and_gain_mana,
    },
    ChaosEffectDef {
        name: "add",
        label: "Add 3 random cards to your hand, which cost (0)",
        build: add_random_zero_cost_cards,
    },
    ChaosEffectDef {
        name: "radiant",
        label: "Make your hand Radiant",
        build: make_hand_radiant,
    },
    ChaosEffectDef {
        name: "tokens",
        label: "Summon 5 Radiant Rush Tokens",
        build: summon_rush_tokens,
    },
    ChaosEffectDef {
        name: "discount",
        label: "Cards in your hand and deck cost (2) less",
        build: discount_hand_and_library,
    },
    ChaosEffectDef {
        name: "golem",
        label: "Summon a Chaos Golem",
        build: summon_chaos_golem,
    },
    ChaosEffectDef {
        name: "backrow",
        label: "Summon 5 random Field Spells or Traps into your backrow, Traps face-down",
        build: summon_random_backrow,
    },
    ChaosEffectDef {
        name: "recast",
        label: "Cast a random Call to Chaos",
        build: cast_random_call_to_chaos,
    },
];

pub fn chaos_effect_by_name(name: &str) -> Option<&'static ChaosEffectDef> {
    CHAOS_EFFECTS.iter().find(|effect| effect.name == name)
}

/// R28, R423: the base form rolls one entry of its list; the Radiant form rolls
/// `CALL_TO_CHAOS_RADIANT_EFFECTS` *different* entries — drawn one at a time without replacement, so no
/// effect comes up twice — and resolves them in the order the list writes them, whatever order they
/// were drawn in. The recursion is an entry like any other: it is rolled only when it falls among the
/// three, and resolves where the list puts it (R87's "the recursion where it falls"). `table` is the
/// edition's list (Core #95's by default; Classic+ #73 brings its own), so one roll serves both.
pub fn roll_chaos_effects(rng: &mut Rng, radiant: bool, table: Option<&[ChaosEffectDef]>) -> Vec<ChaosEffectDef> {
    let table = table.unwrap_or(CHAOS_EFFECTS);
    let wanted = if radiant { CALL_TO_CHAOS_RADIANT_EFFECTS } else { 1 };
    let count = (wanted.max(0) as usize).min(table.len());
    let mut left: Vec<ChaosEffectDef> = table.to_vec();
    let mut drawn: Vec<&'static str> = Vec::new();
    for _ in 0..count {
        let at = rng.int(left.len() as i32) as usize;
        if at < left.len() {
            let one = left.remove(at);
            drawn.push(one.name);
        }
    }
    // Names are unique within a list, so "is this entry drawn" is a name match (TS: identity).
    table.iter().filter(|effect| drawn.contains(&effect.name)).copied().collect()
}

/// R436: tell both players what was rolled, before any of it resolves — the rolled clauses by their
/// printed labels, in the order they resolve. The card itself follows R97 (`viewFor.redactEvent`): a
/// #95 read by nobody is the sentinel, while what it rolled is public, as the resolution is.
fn announce_roll(rolled: &[ChaosEffectDef]) -> Effect {
    let labels: Vec<String> = rolled.iter().map(|effect| effect.label.to_string()).collect();
    Effect::new("callToChaos:announce", move |ctx| {
        let (instance_id, def_id) = match ctx.live_self() {
            Some(card) => (card.id.clone(), card.def_id.clone()),
            None => (String::new(), ctx.def_id.clone().unwrap_or_default()),
        };
        let player = ctx.controller;
        ctx.sink.events.push(GameEvent::ChaosRolled {
            player,
            instance_id,
            def_id,
            effects: labels.clone(),
        });
    })
}

/// `callToChaos(args)`'s argument: `radiant` defaults to the context's face, `table` to Core #95's.
#[derive(Clone, Copy, Debug, Default)]
pub struct CallToChaosArgs {
    pub radiant: Option<bool>,
    pub table: Option<&'static [ChaosEffectDef]>,
}

/// The whole card, as one effect: #95's script is `cry: () => [callToChaos()]` for both forms.
///
/// The roll happens when the effect resolves, so the rng cursor moves with the resolution and a
/// replay that stops on a prompt in between still lines up (§10.7). `radiant` defaults to the
/// instance's own flag, which is what `makeContext` put in the context (§5.2). `table` is the
/// edition's list (R423: Classic+ #73 rolls its own through the same rule).
pub fn call_to_chaos(args: CallToChaosArgs) -> Effect {
    let table: &'static [ChaosEffectDef] = args.table.unwrap_or(CHAOS_EFFECTS);
    crate::resolve::lazy_part("callToChaos", move |ctx, memo| {
        // R87, R423: the rolled effects resolve in list order, the recursion's whole chain where it falls,
        // and a cast in that chain can ask — so the roll is a part of the Cry's list, and a pause inside
        // it waits with the rest of it owed. What was rolled is the part's memo: resuming builds the same
        // effects again, and rolls nothing a second time (§10.7). The announcement heads the part, so a
        // resumed part, which goes on after what it had already run, never announces it again (R436).
        let kept = rolled_names(memo);
        let rolled: Vec<ChaosEffectDef> = match kept {
            None => {
                let radiant = args.radiant.unwrap_or(ctx.radiant);
                roll_chaos_effects(&mut *ctx.sink.rng, radiant, Some(table))
            }
            Some(names) => names
                .iter()
                .filter_map(|name| table.iter().find(|effect| effect.name == name.as_str()).copied())
                .collect(),
        };
        let mut effects = vec![announce_roll(&rolled)];
        effects.extend(rolled.iter().map(|chosen| (chosen.build)()));
        let names: Vec<&str> = rolled.iter().map(|chosen| chosen.name).collect();
        EffectPart {
            effects,
            memo: Some(json!(names)),
        }
    })
}

/// A roll kept across a pause (`EffectPart.memo`), read back defensively: it came through JSON.
fn rolled_names(memo: &Memo) -> Option<Vec<String>> {
    let list = memo.as_ref()?.as_array()?;
    Some(list.iter().filter_map(|name| name.as_str().map(str::to_string)).collect())
}
