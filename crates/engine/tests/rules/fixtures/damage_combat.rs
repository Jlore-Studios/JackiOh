//! Fixture cards for the damage and combat systems of patch v0.2.0 (docs/classic-sets.md B5 E5–E9,
//! E35, E37, and §4.5's backrow Death). The engine never imports `packages/cards` (CLAUDE.md), so each
//! system is proved through a card shaped like the one that will use it: a Final Gambit, a Voidwalker,
//! a Blood Moon, a Joro. Ids are `dc-…`, indexed from 4700, so they collide with no other test file.
//!
//! Port of `packages/engine/test/fixtures/damage-combat.ts`. Each exported def is a `pub static` under
//! TS's name snake_cased; TS's module counter (`nextIndex`, from 4700, one per `def` call in file
//! order, `unit`'s included) is each def's stated index. TS's module `let nonce` is a per-thread counter
//! (each Rust test runs on its own thread, as each TS test file ran in its own module), and an action
//! body is the TS literal as JSON (`act(&state, json!({ "type": "endTurn", "playerId": "p1" }))`) or
//! any `ActionInput`.

#![allow(non_upper_case_globals)]

use std::cell::Cell;
use std::sync::LazyLock;

use jackioh_engine::effects::{
    cancel_attack, damage, damage_all, damage_split, draw, forced_attack_own_hero, heal, set_health, steal,
};
use jackioh_engine::testkit::*;

use super::harness::{new_game, put, slot};

/// TS `def(name, type, extra = {})`: a Core Common at cost 0 whose faces print `name`, `extra` over it.
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("dc-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (damage and combat)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(target), Value::Object(extra)) = (card.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
    json_as(card)
}

/// TS `unit(name, attack, health, keywords = [], radiantKeywords = keywords)`: the Radiant face
/// doubles the stats. Pass `json!([])` for no keywords and the same list twice for TS's default.
fn unit(
    name: &str,
    index: i32,
    attack: i32,
    health: i32,
    keywords: Value,
    radiant_keywords: Value,
) -> CardDef {
    def(
        name,
        index,
        "Unit",
        json!({
            "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
            "radiant": { "attack": attack * 2, "health": health * 2, "keywords": radiant_keywords, "text": name },
        }),
    )
}

fn plain_unit(name: &str, index: i32, attack: i32, health: i32) -> CardDef {
    unit(name, index, attack, health, json!([]), json!([]))
}

// ---------------------------------------------------------------------------
// The note log: a Field Spell in p1's backrow lane 5 whose memory records what ran, in order.
// ---------------------------------------------------------------------------

pub const LOG_LANE: i32 = 5;
pub static log_card: LazyLock<CardDef> = LazyLock::new(|| def("log", 4701, "Field Spell", json!({})));

/// Everything the note log has recorded, in order.
pub fn notes(state: &GameState) -> Vec<String> {
    match state.players.p1.backrow.get((LOG_LANE - 1) as usize) {
        Some(Some(log)) => match log.memory.get("steps").and_then(Value::as_array) {
            Some(steps) => steps
                .iter()
                .filter_map(|step| step.as_str().map(str::to_string))
                .collect(),
            None => vec![],
        },
        _ => vec![],
    }
}

fn write(state: &mut GameState, entry: String) {
    let mut steps = notes(state);
    let Some(Some(log)) = state.players.p1.backrow.get_mut((LOG_LANE - 1) as usize) else {
        return;
    };
    steps.push(entry);
    log.memory.insert("steps".to_string(), json!(steps));
}

/// An effect that appends `entry` to the note log (nothing, while there is no log). TS's
/// `note(entry: string | ((ctx) => string))`: this is the string form; `note_with` the function form.
pub fn note(entry: impl Into<String>) -> Effect {
    let entry: String = entry.into();
    Effect::new("dc:note", move |ctx| write(ctx.state, entry.clone()))
}

/// A prompt for the running card's controller with one answer, so answering is trivial (§10.6).
pub fn ask_controller(step: impl Into<String>) -> Effect {
    let step: String = step.into();
    Effect::new("dc:ask", move |ctx| {
        let resume = resume_self(&*ctx, &step, Default::default());
        let player = ctx.controller;
        open_prompt(
            ctx,
            json_as(json!({
                "player": player,
                "kind": "target",
                "prompt": "the card asks its controller",
                "options": [{ "key": "none", "label": "nothing", "selection": { "pick": "none" } }],
                "resume": resume,
            })),
        );
    })
}

/// TS `String(x)` for an optional boolean: `"true"`, `"false"` or `"undefined"`.
fn js_bool(value: Option<bool>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "undefined".to_string(),
    }
}

/// R426: the units the attack destroyed, whether its attacker survived it, and the player the hook acts for.
fn attack_facts(ctx: &EffectContext<'_>) -> String {
    let facts = after_attack_of(ctx);
    let destroyed = facts
        .as_ref()
        .map_or(String::new(), |facts| facts.destroyed_ids.join("+"));
    let survived = js_bool(facts.as_ref().map(|facts| facts.survived));
    format!("after:{destroyed}:{survived}:{}", ctx.controller)
}

/// TS `both(script, radiant = script)`.
fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn both_of(base: Script, radiant: Script) -> CardScripts {
    CardScripts { base, radiant }
}

// ---------------------------------------------------------------------------
// E5 replacements
// ---------------------------------------------------------------------------

/// Classic #52's shape: redirect a lethal hit on its hero to the enemy hero, then heal 10 and draw 3.
pub static gambit: LazyLock<CardDef> = LazyLock::new(|| def("gambit", 4702, "Trap", json!({})));
/// A Field Trap that redirects every lethal hit on its hero and stays: two of them could ping-pong for ever.
pub static echo_gambit: LazyLock<CardDef> =
    LazyLock::new(|| def("echo-gambit", 4703, "Field Trap", json!({})));
/// The same, whose follow-up asks its controller before it heals: a pause inside the follow-up.
pub static gambit_asker: LazyLock<CardDef> = LazyLock::new(|| def("gambit-asker", 4704, "Trap", json!({})));
/// Classic #50's shape: a Unit whose aura exiles every card that would go to a graveyard (Radiant: the enemy's).
pub static voidwalker: LazyLock<CardDef> = LazyLock::new(|| plain_unit("voidwalker", 4705, 6, 3));
/// Classic #28's shape: a Field Spell exiling its controller's own cards on their way to a graveyard.
pub static second_wind: LazyLock<CardDef> =
    LazyLock::new(|| def("second-wind", 4706, "Field Spell", json!({})));
/// Classic #60's shape: a Spell that goes to the bottom of its owner's library instead of its graveyard.
pub static pile_on: LazyLock<CardDef> = LazyLock::new(|| def("pile-on", 4707, "Spell", json!({})));
/// Classic #14's Radiant shape: its controller's dying units flicker instead; the follow-up notes them.
pub static shadowstep: LazyLock<CardDef> = LazyLock::new(|| def("shadowstep", 4708, "Trap", json!({})));
/// Classic+ #22's shape: an enemy's heal becomes Pierce damage, for the rest of the turn.
pub static blood_moon: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "blood-moon",
        4709,
        "Trap",
        json!({
            // B2.7: the Radiant face is a Field Trap that converts from then on, while it stays on the field.
            "radiant": { "type": "Field Trap", "keywords": [], "text": "blood moon radiant" },
        }),
    )
});
/// Classic #33's shape: from the hand, interposes when the opponent targets a friendly unit.
pub static joro: LazyLock<CardDef> = LazyLock::new(|| plain_unit("joro", 4710, 1, 1));

// ---------------------------------------------------------------------------
// E6, E7, E8 pipeline pieces
// ---------------------------------------------------------------------------

/// Classic #75's shape: the hero's hits are halved (Radiant: quartered), rounded up, after Armor.
pub static argus: LazyLock<CardDef> = LazyLock::new(|| def("argus", 4711, "Field Spell", json!({})));
/// A Trap guarding its hero like Argusland: it guards nothing while it is face-down (R463).
pub static hidden_argus: LazyLock<CardDef> = LazyLock::new(|| def("hidden-argus", 4712, "Trap", json!({})));
/// Classic+ #11's shape: the hero takes at most 1 per hit.
pub static anime_armor: LazyLock<CardDef> = LazyLock::new(|| plain_unit("anime-armor", 4713, 4, 4));
/// Classic+ #38's shape: Spell Damage +2 (Radiant +5).
pub static solar: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "solar",
        4714,
        3,
        2,
        json!([{ "kind": "Spell Damage", "n": 2 }]),
        json!([{ "kind": "Spell Damage", "n": 5 }]),
    )
});
/// A Field Spell printing Spell Damage +1, for summing and for the face-down rule.
pub static lens: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "lens",
        4715,
        "Field Spell",
        json!({
            "base": { "keywords": [{ "kind": "Spell Damage", "n": 1 }], "text": "lens" },
            "radiant": { "keywords": [{ "kind": "Spell Damage", "n": 1 }], "text": "lens" },
        }),
    )
});
/// A Trap printing Spell Damage +3, which counts for nothing while it is face-down.
pub static hidden_lens: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "hidden-lens",
        4716,
        "Trap",
        json!({
            "base": { "keywords": [{ "kind": "Spell Damage", "n": 3 }], "text": "hidden lens" },
            "radiant": { "keywords": [{ "kind": "Spell Damage", "n": 3 }], "text": "hidden lens" },
        }),
    )
});
/// A Spell: deal 3 damage to a declared target.
pub static bolt: LazyLock<CardDef> = LazyLock::new(|| def("bolt", 4717, "Spell", json!({})));
/// Classic #83's shape: a Spell with printed Trample, "deal 11 damage to a Unit".
pub static lance: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        "lance",
        4718,
        "Spell",
        json!({
            "base": { "keywords": [{ "kind": "Trample" }], "text": "lance" },
            "radiant": { "keywords": [{ "kind": "Trample" }], "text": "lance" },
        }),
    )
});
/// Classic #29's shape: set a declared hero's health to 13.
pub static vital_kill: LazyLock<CardDef> = LazyLock::new(|| def("vital-kill", 4719, "Spell", json!({})));
/// A Spell: heal a declared target 5.
pub static mend: LazyLock<CardDef> = LazyLock::new(|| def("mend", 4720, "Spell", json!({})));
/// A Field Spell: its controller's start of turn, deal 4 to every unit (a board sweep that is no Spell).
pub static sweep: LazyLock<CardDef> = LazyLock::new(|| def("sweep", 4721, "Field Spell", json!({})));
/// A Spell: deal 2 damage to every unit (a Spell's sweep).
pub static storm: LazyLock<CardDef> = LazyLock::new(|| def("storm", 4722, "Spell", json!({})));

// ---------------------------------------------------------------------------
// E35 restrictions and statuses
// ---------------------------------------------------------------------------

/// Classic+ #51's shape: can't be attacked.
pub static fighter: LazyLock<CardDef> = LazyLock::new(|| plain_unit("fighter", 4723, 7, 2));
/// Classic+ #19.1's shape: only units in its lane may attack it; Radiant adds Immune to Spells.
pub static top_loser: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "top-loser",
        4724,
        5,
        5,
        json!([{ "kind": "Taunt" }]),
        json!([{ "kind": "Taunt" }, { "kind": "Immune to Spells" }]),
    )
});
/// Neither attacks nor is attacked.
pub static statue: LazyLock<CardDef> = LazyLock::new(|| plain_unit("statue", 4725, 3, 3));
/// Classic #69's shape: First Strike only while it carries a Plague Counter.
pub static charger: LazyLock<CardDef> = LazyLock::new(|| plain_unit("charger", 4726, 4, 2));
/// Classic+ #19.5's shape: while Berserk, attacks its own hero at its controller's start of turn; Radiant can't go Berserk.
pub static bot_loser: LazyLock<CardDef> = LazyLock::new(|| plain_unit("bot-loser", 4727, 5, 5));
/// An immune unit: Immune to Spells printed.
pub static warded: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "warded",
        4728,
        2,
        4,
        json!([{ "kind": "Immune to Spells" }]),
        json!([{ "kind": "Immune to Spells" }]),
    )
});

// ---------------------------------------------------------------------------
// E37 and backrow Death
// ---------------------------------------------------------------------------

/// Classic+ #3's shape: Death — deal 1 damage to a random enemy for each Plague Counter on it.
pub static snake: LazyLock<CardDef> = LazyLock::new(|| plain_unit("snake", 4729, 1, 6));
/// Classic+ #61's shape: a Field Spell with a Death hook.
pub static bauble: LazyLock<CardDef> = LazyLock::new(|| def("bauble", 4730, "Field Spell", json!({})));
/// A unit with a Death hook that notes itself, for R68's order beside a backrow Death.
pub static rattle: LazyLock<CardDef> = LazyLock::new(|| plain_unit("rattle", 4731, 1, 1));
/// A Unit with Reborn, to show R461: an exiled unit comes back from nothing.
pub static phoenix: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "phoenix",
        4732,
        2,
        2,
        json!([{ "kind": "Reborn" }]),
        json!([{ "kind": "Reborn" }]),
    )
});
/// A Unit with Lifesteal, for E8's conversion of Lifesteal.
pub static leech: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "leech",
        4733,
        3,
        5,
        json!([{ "kind": "Lifesteal" }]),
        json!([{ "kind": "Lifesteal" }]),
    )
});
/// A plain 2/2 and a plain 1/8, bodies to attack with and at.
pub static grunt: LazyLock<CardDef> = LazyLock::new(|| plain_unit("grunt", 4734, 2, 2));
pub static wall: LazyLock<CardDef> = LazyLock::new(|| plain_unit("wall", 4735, 1, 8));
/// A Field Trap that notes every declared attack and every summon it is offered (it fires again and again).
pub static watcher: LazyLock<CardDef> = LazyLock::new(|| def("watcher", 4736, "Field Trap", json!({})));
/// "After this attacks": notes the combat's facts (Classic #13's shape).
pub static veteran: LazyLock<CardDef> = LazyLock::new(|| plain_unit("veteran", 4737, 2, 2));
/// R426: "After this attacks" with Cleave: notes the units destroyed, whether it survived, and for whom it acts.
pub static cleave_veteran: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "cleave-veteran",
        4738,
        3,
        6,
        json!([{ "kind": "Cleave" }]),
        json!([{ "kind": "Cleave" }]),
    )
});
/// R426: the same with Reborn, 2/2, so a combat can kill it and bring a new body back.
pub static reborn_veteran: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "reborn-veteran",
        4739,
        2,
        2,
        json!([{ "kind": "Reborn" }]),
        json!([{ "kind": "Reborn" }]),
    )
});
/// #86 Mrow's shape: "Death: Take control of the Unit that destroyed this."
pub static turncoat: LazyLock<CardDef> = LazyLock::new(|| plain_unit("turncoat", 4740, 1, 1));
/// The same, whose hook asks its controller before it finishes.
pub static veteran_asker: LazyLock<CardDef> = LazyLock::new(|| plain_unit("veteran-asker", 4741, 2, 2));
/// A 1/1 whose Death asks its controller something (a pause inside §4.5 step 3).
pub static death_asker: LazyLock<CardDef> = LazyLock::new(|| plain_unit("death-asker", 4742, 1, 1));
/// #96 My Pawn's shape: cancels every declared attack (R44).
pub static pawn: LazyLock<CardDef> = LazyLock::new(|| def("pawn", 4743, "Trap", json!({})));

pub static DC_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        log_card.clone(),
        gambit.clone(),
        echo_gambit.clone(),
        gambit_asker.clone(),
        voidwalker.clone(),
        second_wind.clone(),
        pile_on.clone(),
        shadowstep.clone(),
        blood_moon.clone(),
        joro.clone(),
        argus.clone(),
        hidden_argus.clone(),
        anime_armor.clone(),
        solar.clone(),
        lens.clone(),
        hidden_lens.clone(),
        bolt.clone(),
        lance.clone(),
        vital_kill.clone(),
        mend.clone(),
        sweep.clone(),
        storm.clone(),
        fighter.clone(),
        top_loser.clone(),
        statue.clone(),
        charger.clone(),
        bot_loser.clone(),
        warded.clone(),
        snake.clone(),
        bauble.clone(),
        rattle.clone(),
        phoenix.clone(),
        leech.clone(),
        cleave_veteran.clone(),
        reborn_veteran.clone(),
        turncoat.clone(),
        grunt.clone(),
        wall.clone(),
        watcher.clone(),
        veteran.clone(),
        veteran_asker.clone(),
        death_asker.clone(),
        pawn.clone(),
    ]
});

fn any_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))]
}

fn unit_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "of": ["unit"] }))]
}

fn hero_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "of": ["hero"] }))]
}

/// A replacement as TS's literal `{ id, on, instead }`, with no `where`, `when`, `then` or `by`.
fn replacement(id: &str, on: ReplacementMoment, instead: Value) -> ReplacementDef {
    ReplacementDef {
        id: id.to_string(),
        on,
        where_: None,
        when: None,
        instead: json_as(instead),
        then: None,
        by: None,
    }
}

fn gambit_script(amount: i32) -> Script {
    Script {
        replacements: vec![ReplacementDef {
            then: Some("after".to_string()),
            ..replacement(
                "gambit",
                ReplacementMoment::LethalHit,
                json!({ "redirect": "enemyHero" }),
            )
        }],
        resume: IndexMap::from([(
            "after",
            hook(move |ctx| {
                let redirected = replacement_of(&*ctx)
                    .and_then(|record| record.redirected_to)
                    .map_or("?".to_string(), |player| player.to_string());
                vec![
                    note(format!("gambit:after:{redirected}")),
                    heal(json_as(
                        json!({ "target": { "of": "selfHero" }, "amount": amount }),
                    )),
                    draw(json_as(json!({ "count": 3 }))),
                ]
            }),
        )]),
        ..Script::default()
    }
}

fn hero_guard(guard: HeroGuard) -> Script {
    Script {
        hero_guard: Some(read_hook(move |_a| vec![guard])),
        ..Script::default()
    }
}

fn after_attack_noter() -> Script {
    Script {
        after_attack: Some(hook(|ctx| vec![note(attack_facts(&*ctx))])),
        ..Script::default()
    }
}

pub static DC_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (gambit.id.clone(), both_of(gambit_script(10), gambit_script(20))),
        (
            echo_gambit.id.clone(),
            both(Script {
                replacements: vec![replacement(
                    "echo",
                    ReplacementMoment::LethalHit,
                    json!({ "redirect": "enemyHero" }),
                )],
                ..Script::default()
            }),
        ),
        (
            gambit_asker.id.clone(),
            both(Script {
                replacements: vec![ReplacementDef {
                    then: Some("after".to_string()),
                    ..replacement(
                        "gambit",
                        ReplacementMoment::LethalHit,
                        json!({ "redirect": "enemyHero" }),
                    )
                }],
                resume: IndexMap::from([
                    (
                        "after",
                        hook(|_ctx| {
                            vec![
                                note("asker:before"),
                                ask_controller("answered"),
                                note("asker:tail"),
                            ]
                        }),
                    ),
                    (
                        "answered",
                        hook(|_ctx| {
                            vec![
                                note("asker:answered"),
                                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": 10 }))),
                            ]
                        }),
                    ),
                ]),
                ..Script::default()
            }),
        ),
        (
            voidwalker.id.clone(),
            both_of(
                Script {
                    replacements: vec![replacement(
                        "void",
                        ReplacementMoment::ToGraveyard,
                        json!({ "to": "exile" }),
                    )],
                    ..Script::default()
                },
                Script {
                    replacements: vec![ReplacementDef {
                        when: Some(replacement_when(
                            |c| matches!(c.event, ReplacedEvent::ToGraveyard { owner, .. } if *owner != c.controller),
                        )),
                        ..replacement("void", ReplacementMoment::ToGraveyard, json!({ "to": "exile" }))
                    }],
                    ..Script::default()
                },
            ),
        ),
        (
            second_wind.id.clone(),
            both(Script {
                replacements: vec![ReplacementDef {
                    when: Some(replacement_when(
                        |c| matches!(c.event, ReplacedEvent::ToGraveyard { owner, .. } if *owner == c.controller),
                    )),
                    ..replacement("wind", ReplacementMoment::ToGraveyard, json!({ "to": "exile" }))
                }],
                ..Script::default()
            }),
        ),
        (
            pile_on.id.clone(),
            both_of(
                Script {
                    cry: Some(hook(|_ctx| vec![note("pile-on:cry")])),
                    replacements: vec![ReplacementDef {
                        where_: Some(ReplacementWhere::SelfCard),
                        ..replacement(
                            "pile",
                            ReplacementMoment::ToGraveyard,
                            json!({ "to": "bottomOfLibrary" }),
                        )
                    }],
                    ..Script::default()
                },
                Script {
                    cry: Some(hook(|_ctx| vec![note("pile-on:cry")])),
                    ..Script::default()
                },
            ),
        ),
        (
            shadowstep.id.clone(),
            both(Script {
                replacements: vec![ReplacementDef {
                    then: Some("copies".to_string()),
                    ..replacement("step", ReplacementMoment::WouldDie, json!({ "flicker": "yours" }))
                }],
                resume: IndexMap::from([(
                    "copies",
                    hook(|ctx| {
                        let flickered: Vec<String> = replacement_of(&*ctx)
                            .and_then(|record| record.flickered.clone())
                            .unwrap_or_default()
                            .iter()
                            .map(|card| card.instance_id.clone())
                            .collect();
                        vec![note(format!("shadowstep:{}", flickered.join(",")))]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            blood_moon.id.clone(),
            both_of(
                Script {
                    replacements: vec![replacement(
                        "moon",
                        ReplacementMoment::Healed,
                        json!({ "damage": "pierce", "lasting": "thisTurn" }),
                    )],
                    ..Script::default()
                },
                Script {
                    static_flags: Some(StaticFlags {
                        heal_to_damage: Some(true),
                        ..StaticFlags::default()
                    }),
                    replacements: vec![replacement(
                        "moon",
                        ReplacementMoment::Healed,
                        json!({ "damage": "pierce" }),
                    )],
                    ..Script::default()
                },
            ),
        ),
        (
            joro.id.clone(),
            both(Script {
                replacements: vec![ReplacementDef {
                    where_: Some(ReplacementWhere::Hand),
                    ..replacement("joro", ReplacementMoment::Targeted, json!({ "interpose": true }))
                }],
                ..Script::default()
            }),
        ),
        (
            argus.id.clone(),
            both_of(
                hero_guard(HeroGuard {
                    cap: None,
                    divisor: Some(2),
                }),
                hero_guard(HeroGuard {
                    cap: None,
                    divisor: Some(4),
                }),
            ),
        ),
        (
            hidden_argus.id.clone(),
            both(hero_guard(HeroGuard {
                cap: None,
                divisor: Some(2),
            })),
        ),
        (
            anime_armor.id.clone(),
            both(hero_guard(HeroGuard {
                cap: Some(1),
                divisor: None,
            })),
        ),
        (
            bolt.id.clone(),
            both(Script {
                targets: any_target(),
                cry: Some(hook(|_ctx| {
                    vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 3 })))]
                })),
                ..Script::default()
            }),
        ),
        (
            lance.id.clone(),
            both(Script {
                targets: unit_target(),
                cry: Some(hook(|_ctx| {
                    vec![damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 11, "trample": true }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            vital_kill.id.clone(),
            both(Script {
                targets: hero_target(),
                cry: Some(hook(|_ctx| {
                    vec![set_health(json_as(
                        json!({ "to": { "of": "chosen" }, "value": 13 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            mend.id.clone(),
            both(Script {
                targets: any_target(),
                cry: Some(hook(|_ctx| {
                    vec![heal(json_as(
                        json!({ "target": { "of": "chosen" }, "amount": 5 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            sweep.id.clone(),
            both(Script {
                start_of_turn: Some(hook(|_ctx| {
                    vec![damage_all(json_as(json!({ "amount": 4, "side": "any" })))]
                })),
                ..Script::default()
            }),
        ),
        (
            storm.id.clone(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![damage_all(json_as(json!({ "amount": 2, "side": "any" })))]
                })),
                ..Script::default()
            }),
        ),
        (
            fighter.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    cant_be_attacked: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (
            top_loser.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    attacked_only_from_lane: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (
            statue.id.clone(),
            both(Script {
                static_flags: Some(StaticFlags {
                    cant_attack_or_be_attacked: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }),
        ),
        (
            charger.id.clone(),
            both(Script {
                conditional_keywords: Some(read_hook(|a| {
                    if a.self_.counters.plague.unwrap_or(0) > 0 {
                        vec![Keyword::FirstStrike]
                    } else {
                        vec![]
                    }
                })),
                ..Script::default()
            }),
        ),
        (
            bot_loser.id.clone(),
            both_of(
                Script {
                    start_of_turn: Some(hook(|ctx| match ctx.self_.as_ref() {
                        Some(me) if is_berserk(me) => {
                            vec![forced_attack_own_hero(json_as(
                                json!({ "attacker": { "of": "self" } }),
                            ))]
                        }
                        _ => vec![],
                    })),
                    ..Script::default()
                },
                Script {
                    static_flags: Some(StaticFlags {
                        never_berserk: Some(true),
                        ..StaticFlags::default()
                    }),
                    ..Script::default()
                },
            ),
        ),
        (
            snake.id.clone(),
            both(Script {
                // A Death hook reads its card as it died (R89).
                death: Some(hook(|ctx| {
                    let amount = ctx
                        .self_
                        .as_ref()
                        .and_then(|card| card.counters.plague)
                        .unwrap_or(0);
                    vec![damage_split(json_as(
                        json!({ "amount": amount, "among": "enemies" }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            bauble.id.clone(),
            both(Script {
                death: Some(hook(|_ctx| vec![note("bauble:death")])),
                ..Script::default()
            }),
        ),
        (
            rattle.id.clone(),
            both(Script {
                death: Some(hook(|ctx| {
                    let controller = ctx
                        .self_
                        .as_ref()
                        .map_or("?".to_string(), |card| card.controller.to_string());
                    vec![note(format!("rattle:death:{controller}"))]
                })),
                ..Script::default()
            }),
        ),
        (
            veteran.id.clone(),
            both(Script {
                after_attack: Some(hook(|ctx| {
                    let facts = after_attack_of(&*ctx);
                    let target = facts
                        .as_ref()
                        .map_or("?".to_string(), |facts| facts.target_id.clone());
                    let destroyed = facts
                        .as_ref()
                        .map_or(String::new(), |facts| facts.destroyed_ids.join("+"));
                    let survived = js_bool(facts.as_ref().map(|facts| facts.survived));
                    let forced = js_bool(facts.as_ref().map(|facts| facts.forced));
                    let zone = ctx.self_.as_ref().map_or("none", |card| card.zone.z().as_str());
                    vec![note(format!(
                        "after:{target}:{destroyed}:{survived}:{forced}:{zone}"
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (cleave_veteran.id.clone(), both(after_attack_noter())),
        (reborn_veteran.id.clone(), both(after_attack_noter())),
        (
            turncoat.id.clone(),
            both(Script {
                death: Some(hook(|ctx| match killer_of(ctx.state, ctx.self_.as_ref()) {
                    None => vec![],
                    Some(killer) => vec![steal(json_as(json!({ "instanceId": killer.id })))],
                })),
                ..Script::default()
            }),
        ),
        (
            veteran_asker.id.clone(),
            both(Script {
                after_attack: Some(hook(|_ctx| {
                    vec![
                        note("after:before"),
                        ask_controller("answered"),
                        note("after:tail"),
                    ]
                })),
                resume: IndexMap::from([(
                    "answered",
                    hook(|ctx| {
                        let survived = js_bool(after_attack_of(&*ctx).map(|facts| facts.survived));
                        vec![note(format!("after:answered:{survived}"))]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            death_asker.id.clone(),
            both(Script {
                death: Some(hook(|_ctx| vec![note("death:ask"), ask_controller("answered")])),
                resume: IndexMap::from([("answered", hook(|_ctx| vec![note("death:answered")]))]),
                ..Script::default()
            }),
        ),
        (
            pawn.id.clone(),
            both(Script {
                triggers: vec![
                    TriggerDef::new("pawn", &[GameEventType::AttackDeclared], |_ctx, _event| {
                        vec![cancel_attack(Default::default())]
                    })
                    .with_when(|_ctx, event| {
                        matches!(event, GameEvent::AttackDeclared { forced: false, .. })
                    }),
                ],
                ..Script::default()
            }),
        ),
        (
            watcher.id.clone(),
            both(Script {
                triggers: vec![TriggerDef::new(
                    "watch",
                    &[GameEventType::AttackDeclared, GameEventType::Summoned],
                    |_ctx, event| {
                        let entry = match event {
                            GameEvent::AttackDeclared { target_id, .. } => {
                                format!("watch:attack:{target_id}")
                            }
                            GameEvent::Summoned { instance_id, .. } => format!("watch:summon:{instance_id}"),
                            _ => "watch".to_string(),
                        };
                        vec![note(entry)]
                    },
                )],
                ..Script::default()
            }),
        ),
    ])
});

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

pub fn register() {
    let mut defs: CardDefs = registered_catalog().clone();
    for card in DC_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(DC_SCRIPTS.clone());
    register_scripts(merged);
}

thread_local! {
    /// TS's module `let nonce = 0`, shared by `act_result` and every `recorder`.
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

fn next_nonce() -> u32 {
    NONCE.with(|nonce| {
        let next = nonce.get() + 1;
        nonce.set(next);
        next
    })
}

fn action_of(body: impl serde::Serialize, nonce: String) -> Action {
    let input: ActionInput = json_as(serde_json::to_value(&body).expect("an action body is JSON"));
    input.with_nonce(nonce)
}

/// `reduce` with the next `dc<n>` nonce. `body` is an `ActionInput` or its JSON.
pub fn act_result(state: &GameState, body: impl serde::Serialize) -> ReduceResult {
    let n = next_nonce();
    reduce(state, &action_of(body, format!("dc{n}")))
}

/// `act_result`'s state; panics with the refusal's text.
pub fn act(state: &GameState, body: impl serde::Serialize) -> GameState {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// A game past both mulligans, in p1's first main phase, with the note log in p1's backrow lane 5.
pub fn playing(seed: &str) -> GameState {
    let created = new_game(seed, None);
    register();
    let mut state = begin_game(&created).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }),
    );
    let keep: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }),
    );
    put(
        &mut state,
        &log_card.id,
        slot(PlayerId::P1, Row::Backrow, LOG_LANE),
        json!({}),
    );
    state
}

/// Answer the one open prompt, whoever it belongs to.
pub fn answer(state: &GameState) -> ReduceResult {
    let Some(pending) = state.pending.as_ref() else {
        panic!("expected a prompt to be open");
    };
    let result = act_result(
        state,
        json!({
            "type": "answer",
            "choiceId": pending.id,
            "selection": [{ "pick": "none" }],
            "playerId": pending.player_id,
        }),
    );
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// TS `JSON.parse(JSON.stringify(state))`.
pub fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).expect("a state is JSON"))
        .expect("a state's JSON is a state")
}

/// §9.3: the live game and its replay agree. `start` is the state the log was played from; the log is
/// folded again from a clone of it, action by action through `reduce`, and the two end states hash
/// the same (`replay.hashState`).
pub fn replays_to(start: &GameState, log: &[Action], live: &GameState) -> bool {
    let mut state = clone_state(start);
    for action in log {
        let result = reduce(&state, action);
        if let Some(error) = &result.error {
            panic!("replay rejected {}: {error}", action.action_type());
        }
        state = result.state;
    }
    hash_state(&state) == hash_state(live)
}

/// A recorder: every action it plays goes into its log, so a test can replay what it played. TS's
/// `{ state: () => GameState, play(body), log, start }`.
#[derive(Clone, Debug)]
pub struct Recorder {
    current: GameState,
    pub log: Vec<Action>,
    pub start: GameState,
}

impl Recorder {
    /// The state after every action played so far.
    pub fn state(&self) -> &GameState {
        &self.current
    }

    /// Plays one action with the next `dcr<n>` nonce and records it; panics with a refusal's text.
    pub fn play(&mut self, body: impl serde::Serialize) -> ReduceResult {
        let n = next_nonce();
        let action = action_of(body, format!("dcr{n}"));
        let result = reduce(&self.current, &action);
        if let Some(error) = &result.error {
            panic!("{error}");
        }
        self.log.push(action);
        self.current = result.state.clone();
        result
    }
}

pub fn recorder(start: &GameState) -> Recorder {
    Recorder {
        current: start.clone(),
        log: Vec::new(),
        start: clone_state(start),
    }
}

/// Each event's `type`, in order.
pub fn event_types(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| event.event_type().as_str().to_string())
        .collect()
}

/// Whether the card is in one of the player's piles. TS `pile: "hand" | "library" | "graveyard" |
/// "exile"`: a `ZoneName` or its string.
pub fn in_pile(state: &GameState, player: PlayerId, pile: impl ToString, id: &str) -> bool {
    let side = &state.players[player];
    let cards = match pile.to_string().as_str() {
        "hand" => &side.hand,
        "library" => &side.library,
        "graveyard" => &side.graveyard,
        "exile" => &side.exile,
        other => panic!("in_pile: {other} is not a pile"),
    };
    cards.iter().any(|card| card.id == id)
}
