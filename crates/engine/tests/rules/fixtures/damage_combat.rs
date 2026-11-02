//! Fixture cards for the damage and combat systems (docs/classic-sets.md B5 E5–E9, E35, E37, and
//! §4.5's backrow Death), each proved through a card shaped like the one that will use it, since the
//! engine never imports the cards crate. Ids are `dc-…`, indexed from 4700, so they collide with no other
//! test file. The action nonce is a per-thread counter (each test runs on its own thread).

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects::{
    cancel_attack, damage, damage_all, damage_split, draw, forced_attack_own_hero, heal, set_health, steal,
    transform,
};
use jackioh_engine::testkit::*;

use super::harness::{act, act_result, both, new_game, next_action, put, slot};

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

// The note log: a Field Spell in p1's backrow lane 5 whose memory records what ran, in order.

pub const LOG_LANE: i32 = 5;
pub static log_card: LazyLock<CardDef> = LazyLock::new(|| def("log", 4701, "Field Spell", json!({})));

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

/// An effect that appends `entry` to the note log (nothing, while there is no log).
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

fn both_of(base: Script, radiant: Script) -> CardScripts {
    CardScripts { base, radiant }
}

// E5 replacements

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
/// The same on a Field Trap, which no firing spends, as a fused Unit holding Classic #14's text is not.
pub static echo_shadowstep: LazyLock<CardDef> =
    LazyLock::new(|| def("echo-shadowstep", 4749, "Field Trap", json!({})));
/// A 1/0 Unit: it dies at every state check, a flicker bringing it back dying again.
pub static husk: LazyLock<CardDef> = LazyLock::new(|| plain_unit("husk", 4750, 1, 0));
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

// E6, E7, E8 pipeline pieces

/// Classic #75's shape: the hero's hits are halved (Radiant: quartered), rounded up, after Armor.
pub static argus: LazyLock<CardDef> = LazyLock::new(|| def("argus", 4711, "Field Spell", json!({})));
/// A Trap guarding its hero like Argusland: it guards nothing while it is face-down (R463).
pub static hidden_argus: LazyLock<CardDef> = LazyLock::new(|| def("hidden-argus", 4712, "Trap", json!({})));
/// Classic+ #11's shape: the hero takes at most 1 per hit.
pub static anime_armor: LazyLock<CardDef> = LazyLock::new(|| plain_unit("anime-armor", 4713, 4, 4));
/// Classic+ #11's shape (#2179): its hero and this Unit take at most 1 per hit (R2179005).
pub static self_guard: LazyLock<CardDef> = LazyLock::new(|| plain_unit("self-guard", 4790, 4, 4));
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

// E35 restrictions and statuses

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

// E37 and backrow Death

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
/// R1026: "After this is attacked": notes the attacker, whether the attack was forced, whether
/// the attacker survived, and the zone this reads itself from (Meditative #49.3's shape).
pub static defender: LazyLock<CardDef> = LazyLock::new(|| plain_unit("defender", 4744, 1, 6));
/// R1026: the same, whose hook asks its controller before it finishes.
pub static defender_asker: LazyLock<CardDef> = LazyLock::new(|| plain_unit("defender-asker", 4745, 1, 6));
/// R1704001: Classic #52's shape (#1704): cancel a lethal hit on its hero, then heal 10 and draw 3.
pub static cancel_gambit: LazyLock<CardDef> = LazyLock::new(|| def("cancel-gambit", 4746, "Trap", json!({})));
/// R2199000: "After one of your Units attacks": notes the attacker, whether the attack was forced,
/// whether the attacker survived, and the player it acts for (Training Grounds #31 Level Up!'s shape).
pub static ally_noter: LazyLock<CardDef> =
    LazyLock::new(|| def("ally-noter", 4747, "Field Spell", json!({})));
/// R2199000: the same, whose hook asks its controller before it finishes.
pub static ally_asker: LazyLock<CardDef> =
    LazyLock::new(|| def("ally-asker", 4748, "Field Spell", json!({})));
/// R2285000: "When this attacks": notes the attack's target, whether it was forced, and the enemy
/// hero's health as the hook runs (Classic #45 Nature Titan's shape).
pub static striker: LazyLock<CardDef> = LazyLock::new(|| plain_unit("striker", 4749, 2, 2));
/// R2285000: the same, whose hook asks its controller before it finishes.
pub static striker_asker: LazyLock<CardDef> = LazyLock::new(|| plain_unit("striker-asker", 4750, 2, 2));
/// R2285000: "When this attacks, destroy this": an attack whose attacker is gone does not fight.
pub static kamikaze: LazyLock<CardDef> = LazyLock::new(|| plain_unit("kamikaze", 4751, 2, 2));
/// R2285002: "When this attacks a Unit, transform it into a wall" (Classic+ #73.1 Classic Golem's shape).
pub static shifter: LazyLock<CardDef> = LazyLock::new(|| plain_unit("shifter", 4752, 3, 3));
/// R2396000: "Whenever this attacks": the veteran's notes, `even:`, with the player it acts for, run
/// though it died in the combat (Core #32 Prem Panther's shape).
pub static even_veteran: LazyLock<CardDef> = LazyLock::new(|| plain_unit("even-veteran", 4753, 2, 2));
/// R2396000: the same with Reborn, 2/2, so a combat can kill it and bring a new body back.
pub static reborn_even_veteran: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "reborn-even-veteran",
        4754,
        2,
        2,
        json!([{ "kind": "Reborn" }]),
        json!([{ "kind": "Reborn" }]),
    )
});
/// R2396000: the veteran asker's hook as a "Whenever this attacks".
pub static even_asker: LazyLock<CardDef> = LazyLock::new(|| plain_unit("even-asker", 4755, 2, 2));
/// R2396000: "Whenever this destroys a Unit": notes the Unit it killed and the player it acts for,
/// though it died in that combat (Meditative #95.1 CN Golem's shape).
pub static gone_killer: LazyLock<CardDef> = LazyLock::new(|| plain_unit("gone-killer", 4756, 2, 2));
/// R2396000: the same with Reborn.
pub static reborn_gone_killer: LazyLock<CardDef> = LazyLock::new(|| {
    unit(
        "reborn-gone-killer",
        4757,
        2,
        2,
        json!([{ "kind": "Reborn" }]),
        json!([{ "kind": "Reborn" }]),
    )
});

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
        echo_shadowstep.clone(),
        husk.clone(),
        blood_moon.clone(),
        joro.clone(),
        argus.clone(),
        hidden_argus.clone(),
        anime_armor.clone(),
        self_guard.clone(),
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
        defender.clone(),
        defender_asker.clone(),
        cancel_gambit.clone(),
        ally_noter.clone(),
        ally_asker.clone(),
        striker.clone(),
        striker_asker.clone(),
        kamikaze.clone(),
        shifter.clone(),
        even_veteran.clone(),
        reborn_even_veteran.clone(),
        even_asker.clone(),
        gone_killer.clone(),
        reborn_gone_killer.clone(),
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

fn cancel_gambit_script(amount: i32) -> Script {
    Script {
        replacements: vec![ReplacementDef {
            then: Some("after".to_string()),
            ..replacement("cancel", ReplacementMoment::LethalHit, json!({ "cancel": true }))
        }],
        resume: IndexMap::from([(
            "after",
            hook(move |ctx| {
                let redirected = replacement_of(&*ctx)
                    .and_then(|record| record.redirected_to)
                    .map_or("none".to_string(), |player| player.to_string());
                vec![
                    note(format!("cancel:after:{redirected}")),
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

/// R2396000: a "Whenever this attacks" noting the veteran's facts, `even:`, and the player it acts for.
fn even_noter() -> Script {
    Script {
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
                "even:{target}:{destroyed}:{survived}:{forced}:{zone}:{}",
                ctx.controller
            ))]
        })),
        after_attack_even_if_gone: true,
        ..Script::default()
    }
}

/// R2396000: a "Whenever this destroys a Unit" noting the Unit it killed and the player it acts for.
fn gone_kill_noter() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("gone-kill", &[GameEventType::Destroyed], |ctx, event| {
                match (event, ctx.self_.as_ref()) {
                    (
                        GameEvent::Destroyed {
                            instance_id,
                            killer_id: Some(killer),
                            ..
                        },
                        Some(me),
                    ) if *killer == me.id => vec![note(format!("kill:{instance_id}:{}", ctx.controller))],
                    _ => vec![],
                }
            })
            .even_if_gone(),
        ],
        ..Script::default()
    }
}

fn shadowstep_script() -> CardScripts {
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
    })
}

pub static DC_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    IndexMap::from([
        (gambit.id.clone(), both_of(gambit_script(10), gambit_script(20))),
        (
            cancel_gambit.id.clone(),
            both_of(cancel_gambit_script(10), cancel_gambit_script(20)),
        ),
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
        (shadowstep.id.clone(), shadowstep_script()),
        (echo_shadowstep.id.clone(), shadowstep_script()),
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
                    guards_self: None,
                }),
                hero_guard(HeroGuard {
                    cap: None,
                    divisor: Some(4),
                    guards_self: None,
                }),
            ),
        ),
        (
            hidden_argus.id.clone(),
            both(hero_guard(HeroGuard {
                cap: None,
                divisor: Some(2),
                guards_self: None,
            })),
        ),
        (
            anime_armor.id.clone(),
            both(hero_guard(HeroGuard {
                cap: Some(1),
                divisor: None,
                guards_self: None,
            })),
        ),
        (
            self_guard.id.clone(),
            both(hero_guard(HeroGuard {
                cap: Some(1),
                divisor: None,
                guards_self: Some(true),
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
        (even_veteran.id.clone(), both(even_noter())),
        (reborn_even_veteran.id.clone(), both(even_noter())),
        (
            even_asker.id.clone(),
            both(Script {
                after_attack: Some(hook(|_ctx| {
                    vec![note("even:before"), ask_controller("answered"), note("even:tail")]
                })),
                resume: IndexMap::from([(
                    "answered",
                    hook(|ctx| {
                        let survived = js_bool(after_attack_of(&*ctx).map(|facts| facts.survived));
                        vec![note(format!("even:answered:{survived}"))]
                    }),
                )]),
                after_attack_even_if_gone: true,
                ..Script::default()
            }),
        ),
        (gone_killer.id.clone(), both(gone_kill_noter())),
        (reborn_gone_killer.id.clone(), both(gone_kill_noter())),
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
            defender.id.clone(),
            both(Script {
                after_attacked: Some(hook(|ctx| {
                    let facts = after_attacked_of(&*ctx);
                    let attacker = facts
                        .as_ref()
                        .map_or("?".to_string(), |facts| facts.attacker_id.clone());
                    let forced = js_bool(facts.as_ref().map(|facts| facts.forced));
                    let survived = js_bool(facts.as_ref().map(|facts| facts.attacker_survived));
                    let zone = ctx.self_.as_ref().map_or("none", |card| card.zone.z().as_str());
                    vec![note(format!("attacked:{attacker}:{forced}:{survived}:{zone}"))]
                })),
                ..Script::default()
            }),
        ),
        (
            ally_noter.id.clone(),
            both(Script {
                ally_attack: Some(hook(|ctx| {
                    let facts = ally_attack_of(&*ctx);
                    let attacker = facts
                        .as_ref()
                        .map_or("?".to_string(), |facts| facts.attacker_id.clone());
                    let forced = js_bool(facts.as_ref().map(|facts| facts.forced));
                    let survived = js_bool(facts.as_ref().map(|facts| facts.survived));
                    vec![note(format!(
                        "ally:{attacker}:{forced}:{survived}:{}",
                        ctx.controller
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            ally_asker.id.clone(),
            both(Script {
                ally_attack: Some(hook(|_ctx| {
                    vec![note("ally:before"), ask_controller("answered"), note("ally:tail")]
                })),
                resume: IndexMap::from([(
                    "answered",
                    hook(|ctx| {
                        let survived = js_bool(ally_attack_of(&*ctx).map(|facts| facts.survived));
                        vec![note(format!("ally:answered:{survived}"))]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            defender_asker.id.clone(),
            both(Script {
                after_attacked: Some(hook(|_ctx| {
                    vec![
                        note("attacked:before"),
                        ask_controller("answered"),
                        note("attacked:tail"),
                    ]
                })),
                resume: IndexMap::from([(
                    "answered",
                    hook(|ctx| {
                        let survived = js_bool(after_attacked_of(&*ctx).map(|facts| facts.attacker_survived));
                        vec![note(format!("attacked:answered:{survived}"))]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            striker.id.clone(),
            both(Script {
                when_attacks: Some(hook(|ctx| {
                    let facts = when_attacks_of(&*ctx);
                    let target = facts
                        .as_ref()
                        .map_or("?".to_string(), |facts| facts.target_id.clone());
                    let forced = js_bool(facts.as_ref().map(|facts| facts.forced));
                    let enemy = ctx.state.players[opponent_of(ctx.controller)].hero.health;
                    vec![note(format!("when:{target}:{forced}:{enemy}"))]
                })),
                ..Script::default()
            }),
        ),
        (
            striker_asker.id.clone(),
            both(Script {
                when_attacks: Some(hook(|_ctx| {
                    vec![note("when:before"), ask_controller("answered"), note("when:tail")]
                })),
                resume: IndexMap::from([(
                    "answered",
                    hook(|ctx| {
                        let forced = js_bool(when_attacks_of(&*ctx).map(|facts| facts.forced));
                        vec![note(format!("when:answered:{forced}"))]
                    }),
                )]),
                ..Script::default()
            }),
        ),
        (
            kamikaze.id.clone(),
            both(Script {
                when_attacks: Some(hook(|_ctx| {
                    vec![damage(json_as(json!({ "to": { "of": "self" }, "amount": 99 })))]
                })),
                ..Script::default()
            }),
        ),
        (
            shifter.id.clone(),
            both(Script {
                when_attacks: Some(hook(|ctx| match when_attacks_of(&*ctx) {
                    Some(facts) if !facts.target_id.starts_with("hero-") => vec![transform(json_as(
                        json!({ "instanceId": facts.target_id, "defId": wall.id }),
                    ))],
                    _ => vec![],
                })),
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

// Harness

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

pub fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).expect("a state is JSON"))
        .expect("a state's JSON is a state")
}

/// §9.3: the live game and its replay agree.
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

/// A recorder: every action it plays goes into its log, so a test can replay what it played.
#[derive(Clone, Debug)]
pub struct Recorder {
    current: GameState,
    pub log: Vec<Action>,
    pub start: GameState,
}

impl Recorder {
    pub fn state(&self) -> &GameState {
        &self.current
    }

    pub fn play(&mut self, body: impl serde::Serialize) -> ReduceResult {
        let action = next_action(body);
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

pub fn event_types(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| event.event_type().as_str().to_string())
        .collect()
}

/// Whether the card is in one of the player's piles, `pile` a `ZoneName` or its string.
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
