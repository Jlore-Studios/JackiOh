//! Port of `packages/engine/test/rulings-c.test.ts`.
//!
//! The M3 gate's third rulings file (BUILD.md): one `it("R<n> …")` per SPEC §11 row from R91 to
//! R113 whose behaviour no other test names. R97 (event redaction) and R98 (a card that asks a
//! question while it resolves) are already proved by `viewFor.test.ts` and `prompts.test.ts`, and
//! R104 to R112 are server rulings, so neither kind is repeated here; `rulings.test.ts` is the index
//! that points at whichever file proves each row.
//!
//! Every fixture is this file's own — defs prefixed `rc-`, indexes from 1901, registered on top of
//! the shared fixture catalog by `game()` — so it cannot collide with another test file's (BUILD §0).
//!
//! Two rows have a clause that is easy to half-implement, so each gets a test of its own rather than
//! riding along in a bigger one:
//!   * R91's "no error" is a separate test from R91's "does nothing", because `switchPosition` has
//!     to notice that the position asked for is the one the unit already holds *before* it reads the
//!     unit's exertion — otherwise a unit that has already acted is refused where R91 says nothing
//!     at all happens.
//!   * R94 is one test per side of the exchange. Reading the attacker's attack once is the obvious
//!     half; reading the *defender's* once is the half a lazy `strikeBack` gets wrong, and it is the
//!     half R94 spells out ("a First Strike survivor is struck back with the attack the defender had
//!     before the hit landed"). Both tests assert that the fixture aura really does move the number
//!     between the two readings, so neither can pass by reading the same value twice.
//!
//! Porting notes (part 25.6): TS read and wrote its cards through the live objects `put`, `inHand`
//! and `newInstance` handed back; here those are copies, so a read after the engine has run looks the
//! card up again by id (`live`) and a write goes to the card in the state (`live_mut`). TS's
//! `sinkFor(state)` aliased the state; `SinkFor` keeps the sink's events and rng apart and lends them
//! to an `EngineSink` for each call, so one sink's rng and event list carry from call to call as TS's
//! did while the test reads the state in between.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::combat::{AttackTarget, ExertionKind, SwitchPositionOptions};
use jackioh_engine::damage::{DamageArgs, DamageTarget};
use jackioh_engine::draw::{AddToHandOutcome, DrawOutcome};
use jackioh_engine::prompts::{AnswerInput, OpenPromptArgs, RESUME_HOOK, ResumeOptions};
use jackioh_engine::resolve::{CastOptions, HookName, HookOptions};
use jackioh_engine::subsystems::activate::{
    ACTIVATIONS_MEMORY_KEY, ActivateAction, activate_ability, why_cannot_activate_ability,
};
use jackioh_engine::subsystems::combo_index::{grade_of, grade_rises, played_cards_this_turn};
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::subsystems::hero_power::{
    HERO_POWER_NAMES, HERO_POWERS, POWER_KEY, POWER_RESUME, RUSH_TOKEN_INDEX, STEADY_SHOT_PARAM, hero_power,
    power_abilities, power_ability_of, power_of, roll_power, used_this_turn,
};
use jackioh_engine::testkit::*;
use jackioh_engine::triggers::SettleOptions;
use jackioh_engine::zones::{MoveToZoneOptions, OffFieldZone, PlaceOnFieldOptions};

use super::fixtures::combat::stacker;
use super::fixtures::harness::{in_hand, new_game, put as put_with, slot};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

// ---------------------------------------------------------------------------
// Fixture definitions.
// ---------------------------------------------------------------------------

/// TS `{ ...target, ...extra }`: every key of `extra` written over `target`'s.
fn spread(target: &mut Value, extra: Value) {
    if let (Some(target), Value::Object(extra)) = (target.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
}

/// TS `def(name, type, extra)`. `index` is the number TS's module counter `nextIndex` (from 1900,
/// incremented before each def) gave this def in declaration order.
fn def(name: &str, type_: &str, index: u32, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("rc-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (rulings-c)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": format!("{name} base") },
        "radiant": { "keywords": [], "text": format!("{name} radiant") },
    });
    spread(&mut card, extra);
    json_as(card)
}

/// A unit fixture. The two faces are `Partial<CardFace>` rather than `CardFace` on purpose: a fixture
/// below names only the keywords it is about, and the helper supplies the rest of the face — §8's
/// `text` included, which none of these rows exercise. `Partial<CardDef>` would type `base` as a
/// whole `CardFace` and demand a `text` at every call site.
fn unit(name: &str, index: u32, attack: i32, health: i32, extra: Value) -> CardDef {
    let mut rest = extra;
    let base_extra = rest
        .as_object_mut()
        .and_then(|extra| extra.remove("base"))
        .unwrap_or(json!({}));
    let radiant_extra = rest
        .as_object_mut()
        .and_then(|extra| extra.remove("radiant"))
        .unwrap_or(json!({}));
    let mut base = json!({ "attack": attack, "health": health, "keywords": [], "text": name });
    spread(&mut base, base_extra);
    let mut radiant = json!({ "attack": attack, "health": health, "keywords": [], "text": name });
    spread(&mut radiant, radiant_extra);
    let mut faces = json!({ "base": base, "radiant": radiant });
    spread(&mut faces, rest);
    def(name, "Unit", index, faces)
}

/// A body big enough to survive every exchange below, so no death confuses a hit count.
const BODY: &str = "rc-body";
fn body() -> CardDef {
    unit("body", 1901, 4, 20, json!({}))
}
/// 2 attack and First Strike, so its step-1 strike is small enough to leave a survivor.
const QUICK: &str = "rc-quick";
fn quick() -> CardDef {
    unit(
        "quick",
        1902,
        2,
        20,
        json!({ "base": { "keywords": [{ "kind": "First Strike" }] }, "radiant": { "keywords": [{ "kind": "First Strike" }] } }),
    )
}
/// #20 Pointmaster's printed line: 7/2 with First Strike. It is the card R93 names, and the only
/// fixture here whose step-1 strike KILLS an ordinary attacker, which is the half of the ruling a
/// survivor can never show — a defender that lives through the exchange proves the order of the two
/// hits, not that the second one never happens.
const POINTMASTER: &str = "rc-pointmaster";
fn pointmaster() -> CardDef {
    unit(
        "pointmaster",
        1903,
        7,
        2,
        json!({ "base": { "keywords": [{ "kind": "First Strike" }] }, "radiant": { "keywords": [{ "kind": "First Strike" }] } }),
    )
}
/// A plain 3/3: the ordinary attacker #20 Pointmaster kills before its blow lands.
const DOOMED: &str = "rc-doomed";
fn doomed() -> CardDef {
    unit("doomed", 1904, 3, 3, json!({}))
}
/// Cleave on a body that survives, so R95 can count where the extra instances landed.
const CLEAVER: &str = "rc-cleaver";
fn cleaver() -> CardDef {
    unit(
        "cleaver",
        1905,
        3,
        20,
        json!({ "base": { "keywords": [{ "kind": "Cleave" }] }, "radiant": { "keywords": [{ "kind": "Cleave" }] } }),
    )
}

/// R94 needs a stat that changes *during* a combat. Layer 2 (`setStat`) is not wired into `unitView`
/// until M3-T4, so the fixture uses layer 5: a Field Spell whose aura reads `unit.damage`, which is
/// instance data an `applies` predicate may read (§10.4). A unit that has taken any damage loses 3
/// attack, so "the attack it had before the hit landed" and "the attack it has now" are different
/// numbers and the ruling is observable.
const WEAKENER: &str = "rc-weakener";
fn weakener() -> CardDef {
    def("weakener", "Field Spell", 1906, json!({}))
}
const WEAKEN_BY: i32 = 3;

/// #66 The Rock's shape: a Tribute cost on a body (§6.3).
const TRIBUTE_TWO: &str = "rc-tribute-two";
fn tribute_two() -> CardDef {
    unit("tribute-two", 1907, 8, 8, json!({ "cost": 3 }))
}
/// #55 Lava Golem's shape: a Tribute that may reach the opponent's units (R101).
const TRIBUTE_ENEMIES: &str = "rc-tribute-enemies";
fn tribute_enemies() -> CardDef {
    unit("tribute-enemies", 1908, 10, 5, json!({ "cost": 3 }))
}
/// §7's Sheep Token: the one unit worth 2 Tributes while on the field (§3.2).
const SHEEP: &str = "rc-sheep";
fn sheep() -> CardDef {
    unit(
        "sheep",
        1909,
        1,
        1,
        json!({ "index": play_choices::SHEEP_TOKEN_INDEX, "tags": ["Token"], "rarity": "Token", "token": true }),
    )
}

/// R99: a trap whose `when` admits only an event the opponent caused.
const PICKY_TRAP: &str = "rc-picky-trap";
fn picky_trap() -> CardDef {
    def("picky-trap", "Trap", 1910, json!({}))
}
/// R99 and R61: a trap whose condition is met and whose effect list is empty.
const EMPTY_TRAP: &str = "rc-empty-trap";
fn empty_trap() -> CardDef {
    def("empty-trap", "Trap", 1911, json!({}))
}
/// R99: a trap that declares no predicate, so it answers the event whichever side caused it.
const BARE_TRAP: &str = "rc-bare-trap";
fn bare_trap() -> CardDef {
    def("bare-trap", "Trap", 1912, json!({}))
}
/// R100: a Field Trap on `turnEnded`, which stays after firing so a second offer is observable.
const WINDOW_TRAP: &str = "rc-window-trap";
fn window_trap() -> CardDef {
    def("window-trap", "Field Trap", 1913, json!({}))
}

/// R102: two ingredients whose printed costs sum past the cap.
const FUSE_PRICEY: &str = "rc-fuse-pricey";
fn fuse_pricey() -> CardDef {
    unit("fuse-pricey", 1914, 1, 1, json!({ "cost": 4 }))
}
const FUSE_DEAR: &str = "rc-fuse-dear";
fn fuse_dear() -> CardDef {
    unit("fuse-dear", 1915, 1, 1, json!({ "cost": 3 }))
}
/// R102: an ingredient with a `cost` hook. Its printed cost is 3 but the hook answers 1, so the
/// fused def's cost is 1 + 1 = 2 while a surviving hook would still answer 1 — the two numbers
/// differ, which is what makes "any ingredient `cost` hook is dropped" observable.
const FUSE_HOOKED: &str = "rc-fuse-hooked";
fn fuse_hooked() -> CardDef {
    unit("fuse-hooked", 1916, 1, 1, json!({ "cost": 3 }))
}
const FUSE_CHEAP: &str = "rc-fuse-cheap";
fn fuse_cheap() -> CardDef {
    unit("fuse-cheap", 1917, 1, 1, json!({ "cost": 1 }))
}
/// R102: two ingredients that each ask for a Tribute, so the max and the sum differ.
const FUSE_TRIBUTE_TWO: &str = "rc-fuse-tribute-two";
fn fuse_tribute_two() -> CardDef {
    unit("fuse-tribute-two", 1918, 1, 1, json!({}))
}
const FUSE_TRIBUTE_THREE: &str = "rc-fuse-tribute-three";
fn fuse_tribute_three() -> CardDef {
    unit("fuse-tribute-three", 1919, 1, 1, json!({}))
}
/// R102: two traps that use the *same* trigger id on the same event, with different predicates.
const FUSE_TRAP_MINE: &str = "rc-fuse-trap-mine";
fn fuse_trap_mine() -> CardDef {
    def("fuse-trap-mine", "Trap", 1920, json!({}))
}
const FUSE_TRAP_THEIRS: &str = "rc-fuse-trap-theirs";
fn fuse_trap_theirs() -> CardDef {
    def("fuse-trap-theirs", "Trap", 1921, json!({}))
}
/// R102: an ingredient with a Death hook, so "no Death trigger" is observable.
const FUSE_DYING: &str = "rc-fuse-dying";
fn fuse_dying() -> CardDef {
    unit("fuse-dying", 1922, 2, 2, json!({}))
}
/// R102: one ingredient fused onto two different targets in turn (radiant #85). Its 7/3 is unlike
/// either host, so which fusion picked up its stats is readable off the fused def.
const FUSE_SHARED: &str = "rc-fuse-shared";
fn fuse_shared() -> CardDef {
    unit("fuse-shared", 1923, 7, 3, json!({}))
}
const FUSE_HOST_A: &str = "rc-fuse-host-a";
fn fuse_host_a() -> CardDef {
    unit("fuse-host-a", 1924, 1, 1, json!({}))
}
const FUSE_HOST_B: &str = "rc-fuse-host-b";
fn fuse_host_b() -> CardDef {
    unit("fuse-host-b", 1925, 2, 2, json!({}))
}

/// R103's stand-in for §8 #98: cost is the power's X, and a prompted power resumes into `heroPower`.
const HEROIC: &str = "rc-heroic";
fn heroic() -> CardDef {
    def(
        "heroic",
        "Field Spell",
        1926,
        json!({
            "cost": 0,
            "tags": ["Quickdraw"],
            "rarity": "Mythic",
            "params": [{ "key": STEADY_SHOT_PARAM, "base": 2, "radiant": 4, "better": "up", "step": 2, "min": 1 }],
        }),
    )
}

/// R114: a small body, so one hit can leave it at exactly 0 health with a Trample hit still to come.
const FRAIL: &str = "rc-frail";
fn frail() -> CardDef {
    unit("frail", 1927, 1, 3, json!({}))
}
/// R114: #32's shape with two more keywords bolted on, so one hit can be asked all three questions
/// at once — does the unit take damage, is it marked Poisonous, and does the source heal off it.
const TRAMPLER: &str = "rc-trampler";
fn trampler() -> CardDef {
    unit(
        "trampler",
        1928,
        5,
        5,
        json!({
            "base": { "keywords": [{ "kind": "Trample" }, { "kind": "Lifesteal" }, { "kind": "Poisonous" }] },
            "radiant": { "keywords": [{ "kind": "Trample" }, { "kind": "Lifesteal" }, { "kind": "Poisonous" }] },
        }),
    )
}

/// R115: #92's shape — a unit that both projects an aura and sets its own stats from the board.
const PROJECTOR: &str = "rc-projector";
fn projector() -> CardDef {
    unit("projector", 1929, 2, 2, json!({}))
}
/// R115 and R116: a plain body for the aura to land on and the set-stat hook to measure.
const MEASURED: &str = "rc-measured";
fn measured() -> CardDef {
    unit("measured", 1930, 3, 4, json!({}))
}
/// R116: a set-stat hook that returns a delta big enough that a total would read differently.
const SETTER: &str = "rc-setter";
fn setter() -> CardDef {
    unit("setter", 1931, 2, 6, json!({}))
}
/// R116: a second set-stat card, so "two on one board never read each other" is observable.
const OTHER_SETTER: &str = "rc-other-setter";
fn other_setter() -> CardDef {
    unit("other-setter", 1932, 4, 8, json!({}))
}

/// R117 and R118: the note log, parked in p1's backrow lane 5 so it survives `reduce`'s clone.
const LOG_CARD: &str = "rc-log";
fn log_card() -> CardDef {
    def("log", "Field Spell", 1933, json!({}))
}
/// R117 and R118: a Cry that notes it ran, so "exactly once" is countable.
const CRIER: &str = "rc-crier";
fn crier() -> CardDef {
    unit("crier", 1934, 2, 4, json!({ "cost": 0 }))
}
/// R117 and R118: a Trap answering the summon with a prompt for its own controller (§10.3).
const ASK_TRAP: &str = "rc-ask-trap";
fn ask_trap() -> CardDef {
    def("ask-trap", "Trap", 1935, json!({}))
}
/// R118: R17's carve-out — a Trap that takes the played card off the field before its Cry.
const EAT_TRAP: &str = "rc-eat-trap";
fn eat_trap() -> CardDef {
    def("eat-trap", "Trap", 1936, json!({}))
}

/// R122: a Spell whose Cry notes it ran, so "a Spell short of its graveyard" is observable.
const SPELL_CRIER: &str = "rc-spell-crier";
fn spell_crier() -> CardDef {
    def("spell-crier", "Spell", 1937, json!({ "cost": 0 }))
}
/// R122: a Trap that prompts on `cardPlayed`, so a Spell's play pauses the same way a summon's does.
const ASK_ON_PLAY: &str = "rc-ask-on-play";
fn ask_on_play() -> CardDef {
    def("ask-on-play", "Trap", 1938, json!({}))
}
/// R123: #22 Carnivorous Cube's shape — a `tribute` declaration with both a pick and an `amount`.
const CUBE: &str = "rc-cube";
fn cube() -> CardDef {
    unit("cube", 1939, 4, 6, json!({ "cost": 0 }))
}

/// R153: one card carrying the three hooks a hand and a graveyard must not answer.
const ZONE_HOOKS: &str = "rc-zone-hooks";
fn zone_hooks() -> CardDef {
    def("zone-hooks", "Field Spell", 1940, json!({}))
}

/// R155: #23's shape — a Spell whose resolving face declares an end-of-turn return.
const RETURN_SPELL: &str = "rc-return-spell";
fn return_spell() -> CardDef {
    def("return-spell", "Spell", 1941, json!({ "cost": 0 }))
}
/// R155: #13's shape — a Unit with an end-of-turn hook, which must never be flagged.
const DYING_UNIT: &str = "rc-dying-unit";
fn dying_unit() -> CardDef {
    unit("dying-unit", 1942, 1, 1, json!({ "cost": 0 }))
}

/// R124 and R125: #90's shape — an Anti-oneshot Armor card, whose *cap* is a ceiling, not a reduction.
const GUARD: &str = "rc-guard";
fn guard() -> CardDef {
    def("guard", "Field Spell", 1943, json!({}))
}

/// R126: a continuation under the `delayed` hook — the one shape `turn.runDelayed` re-enters today.
const DELAYED_HOOK_CARD: &str = "rc-delayed-hook";
fn delayed_hook_card() -> CardDef {
    unit("delayed-hook", 1944, 1, 1, json!({}))
}
/// R126: the same continuation as an entry in the card's `resume` step table (§10.6).
const DELAYED_STEP_CARD: &str = "rc-delayed-step";
fn delayed_step_card() -> CardDef {
    unit("delayed-step", 1945, 1, 1, json!({}))
}
/// R127: a card that schedules a continuation and is gone before it comes due (#50's shape, R76).
const GHOST_CARD: &str = "rc-ghost";
fn ghost_card() -> CardDef {
    def("ghost", "Spell", 1946, json!({ "cost": 0 }))
}
/// R131 and R132: #92's shape — a Felinor-tagged set-stat card that must not count itself.
const FIENDER: &str = "rc-fiender";
fn fiender() -> CardDef {
    unit("fiender", 1947, 5, 7, json!({ "tags": ["Felinor"] }))
}
/// R131 and R132: an ordinary Felinor for it to measure.
const FELINOR: &str = "rc-felinor";
fn felinor() -> CardDef {
    unit("felinor", 1948, 3, 10, json!({ "tags": ["Felinor"] }))
}

/// TS's `DEFS`, in its order (the catalog's insertion order).
fn defs() -> Vec<CardDef> {
    vec![
        body(),
        quick(),
        pointmaster(),
        doomed(),
        cleaver(),
        weakener(),
        tribute_two(),
        tribute_enemies(),
        sheep(),
        picky_trap(),
        empty_trap(),
        bare_trap(),
        window_trap(),
        fuse_pricey(),
        fuse_dear(),
        fuse_hooked(),
        fuse_cheap(),
        fuse_tribute_two(),
        fuse_tribute_three(),
        fuse_trap_mine(),
        fuse_trap_theirs(),
        fuse_dying(),
        fuse_shared(),
        fuse_host_a(),
        fuse_host_b(),
        heroic(),
        frail(),
        trampler(),
        projector(),
        measured(),
        setter(),
        other_setter(),
        log_card(),
        crier(),
        ask_trap(),
        eat_trap(),
        spell_crier(),
        ask_on_play(),
        cube(),
        guard(),
        zone_hooks(),
        return_spell(),
        dying_unit(),
        delayed_hook_card(),
        delayed_step_card(),
        ghost_card(),
        fiender(),
        felinor(),
    ]
}

// ---------------------------------------------------------------------------
// Fixture scripts.
// ---------------------------------------------------------------------------

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// Deal `amount` to the enemy hero: the one visible thing a fixture script needs to do.
fn hit(amount: i32) -> Hook {
    hook(move |_| {
        vec![effects::damage(json_as(
            json!({ "to": { "of": "enemyHero" }, "amount": amount }),
        ))]
    })
}

/// R115: what `projector`'s set-stat hook adds, chosen so a total and a delta read differently.
const PROJECTOR_SETS: i32 = 5;

/// R117 and R118: the note log lives in p1's backrow lane 5, so `reduce`'s state clone carries it.
const NOTE_LANE: i32 = 5;

fn log_of(state: &GameState) -> Option<&CardInstance> {
    state
        .players
        .p1
        .backrow
        .get((NOTE_LANE - 1) as usize)
        .and_then(|slot| slot.as_ref())
}

fn note(name: impl Into<String>) -> Effect {
    let name: String = name.into();
    Effect::new("rc:note", move |ctx| {
        let Some(log) = ctx
            .sink
            .state
            .players
            .p1
            .backrow
            .get_mut((NOTE_LANE - 1) as usize)
            .and_then(|slot| slot.as_mut())
        else {
            return;
        };
        let mut steps: Vec<Value> = log
            .memory
            .get("steps")
            .and_then(|steps| steps.as_array())
            .cloned()
            .unwrap_or_default();
        steps.push(json!(name));
        log.memory.insert("steps".to_string(), Value::Array(steps));
    })
}

fn notes(state: &GameState) -> Vec<String> {
    log_of(state)
        .and_then(|log| log.memory.get("steps"))
        .and_then(|steps| steps.as_array())
        .map(|steps| {
            steps
                .iter()
                .filter_map(|step| step.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// §10.6: a prompt for the card's own controller, with one answer, so answering is trivial.
fn ask_controller() -> Effect {
    Effect::new("rc:ask", |ctx| {
        let player = ctx.controller;
        let resume = prompts::resume_self(ctx, "asked", IndexMap::new());
        prompts::open_prompt(
            ctx,
            OpenPromptArgs {
                player,
                kind: PromptKind::Target,
                aim: None,
                prompt: "the trap asks its owner".to_string(),
                options: vec![PromptOption {
                    key: "none".to_string(),
                    label: "nothing".to_string(),
                    selection: Selection::None,
                    cost: None,
                    radiant: None,
                }],
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume,
            },
        );
    })
}

fn heroic_script() -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(POWER_RESUME, hook(hero_power));
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        start_of_game: Some(hook(|_| vec![roll_power()])),
        activations: power_abilities(false),
        resume,
        ..Script::default()
    }
}

fn step_table(entries: Vec<(&'static str, Hook)>) -> IndexMap<&'static str, Hook> {
    entries.into_iter().collect()
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
    // §3.2, §7: the Sheep's worth is its face's text, the static flag its script declares.
    scripts.insert(
        SHEEP.to_string(),
        CardScripts {
            base: Script {
                static_flags: Some(StaticFlags {
                    tribute_worth: Some(config::SHEEP_TRIBUTE_VALUE),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
            radiant: Script {
                static_flags: Some(StaticFlags {
                    tribute_worth: Some(config::RADIANT_SHEEP_TRIBUTE_VALUE),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
        },
    );
    scripts.insert(
        WEAKENER.to_string(),
        both(Script {
            aura: Some(aura_hook(|_| {
                vec![AuraEntry {
                    applies: Box::new(|unit: &CardInstance| unit.damage > 0),
                    mod_: StatMod {
                        attack: Some(-WEAKEN_BY),
                        ..StatMod::default()
                    },
                }]
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        TRIBUTE_TWO.to_string(),
        both(Script {
            static_flags: Some(StaticFlags {
                tribute: Some(2),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    scripts.insert(
        TRIBUTE_ENEMIES.to_string(),
        both(Script {
            static_flags: Some(StaticFlags {
                tribute: Some(3),
                tribute_enemies: Some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    // R99: the predicate admits only an event the other player caused, and never spends the trap on
    // one of its controller's own.
    scripts.insert(
        PICKY_TRAP.to_string(),
        both(Script {
            triggers: vec![
                TriggerDef::new("theirs-only", &[GameEventType::CardPlayed], |_, _| {
                    vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
                })
                .with_when(|ctx, event| {
                    matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller)
                }),
            ],
            ..Script::default()
        }),
    );
    // R61 and R99: the condition is met, the effect list is empty, and the trap is spent anyway.
    scripts.insert(
        EMPTY_TRAP.to_string(),
        both(Script {
            triggers: vec![
                TriggerDef::new("always", &[GameEventType::CardPlayed], |_, _| vec![]).with_when(|_, _| true),
            ],
            ..Script::default()
        }),
    );
    scripts.insert(
        BARE_TRAP.to_string(),
        both(Script {
            triggers: vec![TriggerDef::new(
                "no-predicate",
                &[GameEventType::CardPlayed],
                |_, _| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
                    ))]
                },
            )],
            ..Script::default()
        }),
    );
    scripts.insert(
        WINDOW_TRAP.to_string(),
        both(Script {
            triggers: vec![TriggerDef::new(
                "turn-end",
                &[GameEventType::TurnEnded],
                |_, _| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                    ))]
                },
            )],
            ..Script::default()
        }),
    );
    // R102: the hook answers 1 where the printed cost is 3.
    scripts.insert(
        FUSE_HOOKED.to_string(),
        both(Script {
            cost: Some(cost_hook(|_| 1)),
            ..Script::default()
        }),
    );
    scripts.insert(
        FUSE_TRIBUTE_TWO.to_string(),
        both(Script {
            static_flags: Some(StaticFlags {
                tribute: Some(2),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    scripts.insert(
        FUSE_TRIBUTE_THREE.to_string(),
        both(Script {
            static_flags: Some(StaticFlags {
                tribute: Some(3),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    // R102: both traps call their trigger "fire", both watch `cardPlayed`, and their predicates are
    // opposites, so a fusion that lost the namespacing or merged the predicates reads differently.
    scripts.insert(
        FUSE_TRAP_MINE.to_string(),
        both(Script {
            triggers: vec![
                TriggerDef::new("fire", &[GameEventType::CardPlayed], |_, _| {
                    vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))]
                })
                .with_when(|ctx, event| {
                    matches!(event, GameEvent::CardPlayed { player, .. } if *player == ctx.controller)
                }),
            ],
            ..Script::default()
        }),
    );
    scripts.insert(
        FUSE_TRAP_THEIRS.to_string(),
        both(Script {
            triggers: vec![
                TriggerDef::new("fire", &[GameEventType::CardPlayed], |_, _| {
                    vec![effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 2 })))]
                })
                .with_when(|ctx, event| {
                    matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller)
                }),
            ],
            ..Script::default()
        }),
    );
    scripts.insert(
        FUSE_DYING.to_string(),
        both(Script {
            death: Some(hit(7)),
            ..Script::default()
        }),
    );
    scripts.insert(
        HEROIC.to_string(),
        CardScripts {
            base: heroic_script(),
            radiant: heroic_script(),
        },
    );
    // R115: one card doing both of the things Vanilla has to silence — projecting an aura at layer 5
    // and setting its own stats at layer 2 — plus the printed face Vanilla must leave alone.
    scripts.insert(
        PROJECTOR.to_string(),
        both(Script {
            aura: Some(aura_hook(|args| {
                let me = args.self_;
                vec![AuraEntry {
                    applies: Box::new(move |unit: &CardInstance| {
                        unit.controller == me.controller && unit.id != me.id
                    }),
                    mod_: StatMod {
                        attack: Some(2),
                        max_health: Some(2),
                        ..StatMod::default()
                    },
                }]
            })),
            set_stat: Some(read_hook(|_| SetStat {
                attack: Some(PROJECTOR_SETS),
                max_health: Some(PROJECTOR_SETS),
            })),
            ..Script::default()
        }),
    );
    // R116: a delta. It reads the other set-stat card on the board through `statsWithBuffs`, which is
    // layers 1 to 4 and therefore excludes layer 2 — so the two can never read each other.
    scripts.insert(
        SETTER.to_string(),
        both(Script {
            set_stat: Some(read_hook(|args| {
                let other = zones::active_units_of(args.state, args.self_.controller)
                    .into_iter()
                    .find(|unit| unit.id != args.self_.id && unit.def_id == OTHER_SETTER);
                let seen = other
                    .map(|other| layers::stats_with_buffs(args.state, other).attack)
                    .unwrap_or(0);
                SetStat {
                    attack: Some(seen),
                    max_health: Some(seen),
                }
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        OTHER_SETTER.to_string(),
        both(Script {
            set_stat: Some(read_hook(|args| {
                let other = zones::active_units_of(args.state, args.self_.controller)
                    .into_iter()
                    .find(|unit| unit.id != args.self_.id && unit.def_id == SETTER);
                let seen = other
                    .map(|other| layers::stats_with_buffs(args.state, other).attack)
                    .unwrap_or(0);
                SetStat {
                    attack: Some(seen),
                    max_health: Some(seen),
                }
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        CRIER.to_string(),
        both(Script {
            cry: Some(hook(|_| vec![note("cry")])),
            ..Script::default()
        }),
    );
    // R117 and R118: the trap's answer is a prompt for its own controller, which pauses the play that
    // emitted the `summoned` event partway through §10.5.
    scripts.insert(
        ASK_TRAP.to_string(),
        both(Script {
            triggers: vec![TriggerDef::new(
                "ask-summon",
                &[GameEventType::Summoned],
                |_, _| vec![ask_controller()],
            )],
            resume: step_table(vec![("asked", hook(|_| vec![note("answered")]))]),
            ..Script::default()
        }),
    );
    // R118 and R17: the trap takes the played permanent off the field, so there is no card left to
    // resolve and the Cry is genuinely lost rather than merely delayed.
    scripts.insert(
        EAT_TRAP.to_string(),
        both(Script {
            triggers: vec![TriggerDef::new(
                "eat-summon",
                &[GameEventType::Summoned],
                |_, event| {
                    let mut list = vec![note("eaten")];
                    if let GameEvent::Summoned { instance_id, .. } = event {
                        list.push(effects::exile(json_as(
                            json!({ "target": { "of": "instance", "instanceId": instance_id } }),
                        )));
                    }
                    list
                },
            )],
            ..Script::default()
        }),
    );
    scripts.insert(
        SPELL_CRIER.to_string(),
        both(Script {
            cry: Some(hook(|_| vec![note("spell")])),
            ..Script::default()
        }),
    );
    scripts.insert(
        ASK_ON_PLAY.to_string(),
        both(Script {
            triggers: vec![TriggerDef::new(
                "ask-play",
                &[GameEventType::CardPlayed],
                |_, _| vec![ask_controller()],
            )],
            resume: step_table(vec![("asked", hook(|_| vec![note("answered")]))]),
            ..Script::default()
        }),
    );
    // R123: one declaration carrying both halves — a pick the script reads out of `ctx.targets`, and
    // an `amount` that is §6.3's Tribute cost.
    scripts.insert(
        GUARD.to_string(),
        both(Script {
            static_flags: Some(StaticFlags {
                anti_oneshot: Some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }),
    );
    scripts.insert(
        RETURN_SPELL.to_string(),
        both(Script {
            end_of_turn: Some(hook(|_| vec![note("returned")])),
            ..Script::default()
        }),
    );
    scripts.insert(
        DYING_UNIT.to_string(),
        both(Script {
            end_of_turn: Some(hook(|_| vec![note("unit-end")])),
            ..Script::default()
        }),
    );
    // R153: a card with all three of the hooks whose zones the ruling narrows.
    scripts.insert(
        ZONE_HOOKS.to_string(),
        both(Script {
            start_of_turn: Some(hook(|_| vec![note("startOfTurn")])),
            end_of_turn: Some(hook(|_| vec![note("endOfTurn")])),
            on_play_hook: Some(hook(|_| vec![note("onPlayHook")])),
            ..Script::default()
        }),
    );
    // R126: the two shapes a delayed continuation may take. Both must be re-entered by one reader.
    scripts.insert(
        DELAYED_HOOK_CARD.to_string(),
        both(Script {
            delayed: Some(hook(|_| vec![note("delayed:hook")])),
            ..Script::default()
        }),
    );
    scripts.insert(
        DELAYED_STEP_CARD.to_string(),
        both(Script {
            resume: step_table(vec![("later", hook(|_| vec![note("delayed:step")]))]),
            ..Script::default()
        }),
    );
    // R127: the step reports whether it got a `self`, so "resolves with ctx.self === null" is visible.
    scripts.insert(
        GHOST_CARD.to_string(),
        both(Script {
            resume: step_table(vec![(
                "orphan",
                hook(|ctx| {
                    let carried = match ctx.data.get("carried") {
                        None | Some(Value::Null) => String::new(),
                        Some(Value::String(text)) => text.clone(),
                        Some(other) => other.to_string(),
                    };
                    let has_self = if ctx.self_.is_none() { "no-self" } else { "self" };
                    vec![note(format!("orphan:{has_self}:{carried}"))]
                }),
            )]),
            ..Script::default()
        }),
    );
    // R131: "all your Felinors" means every OTHER Felinor you control, matched by instance and not by
    // tag — so this counts itself out by id even though its own def carries the Felinor tag. R132: the
    // sums accumulate first and each stat is floored once, on its combined total.
    scripts.insert(
        FIENDER.to_string(),
        both(Script {
            set_stat: Some(read_hook(|args| {
                let mut attack = 0;
                let mut max_health = 0;
                for other in zones::active_units_of(args.state, args.self_.controller) {
                    if other.id == args.self_.id {
                        continue;
                    }
                    if !catalog::def_of(Some(args.state), &other.def_id)
                        .tags
                        .contains(&Tag::Felinor)
                    {
                        continue;
                    }
                    let stats = layers::stats_with_buffs(args.state, other);
                    attack += stats.attack;
                    max_health += stats.max_health;
                }
                SetStat {
                    attack: Some(attack.max(0)),
                    max_health: Some(max_health.max(0)),
                }
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        CUBE.to_string(),
        both(Script {
            targets: vec![TargetDecl {
                amount: Some(2),
                ..TargetDecl::tribute(1, 1, Value::Null)
            }],
            cry: Some(hook(|ctx| {
                let text = match ctx.targets.first() {
                    Some(Selection::Instance { instance_id }) => format!("ate:{instance_id}"),
                    _ => "ate:nothing".to_string(),
                };
                vec![note(text)]
            })),
            ..Script::default()
        }),
    );
    scripts
}

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

/// The harness's `put(state, defId, ref)` with TS's default `options = {}`.
fn put(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    put_with(state, def_id, at, json!({}))
}

/// The harness's `put(state, defId, ref, { radiant: true })`.
fn put_radiant(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    put_with(state, def_id, at, json!({ "radiant": true }))
}

/// The card as it stands now in `state`, by id (TS read the live object).
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id)
        .cloned()
        .unwrap_or_else(|| panic!("{} is in no zone", card.id))
}

/// The card in `state`, by id, to write through (TS wrote the live object).
fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    let id = card.id.clone();
    find_instance_mut(state, &id).unwrap_or_else(|| panic!("{id} is in no zone"))
}

/// `sinkFor(state, events)` (fixtures/harness.ts): a sink whose rng starts at the state's cursor, as
/// reduce does. Its events and rng are kept here and lent to an `EngineSink` over the state for each
/// call, so they carry across calls exactly as TS's one sink object did.
struct SinkFor {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl SinkFor {
    fn new(state: &GameState) -> SinkFor {
        SinkFor {
            events: Vec::new(),
            rng: Rng::new(&state.seed, state.rng_cursor),
        }
    }

    fn run<T>(&mut self, state: &mut GameState, call: impl FnOnce(&mut EngineSink<'_>) -> T) -> T {
        let mut sink = EngineSink::new(state, &mut self.events, &mut self.rng);
        call(&mut sink)
    }
}

/// `f(sinkFor(state), …)`: one call on a fresh sink.
fn with_sink<T>(state: &mut GameState, call: impl FnOnce(&mut EngineSink<'_>) -> T) -> T {
    let mut sink = SinkFor::new(state);
    sink.run(state, call)
}

/// `{ controller }`, the `HookOptions` most contexts below are made with.
fn as_controller(controller: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(controller),
        ..HookOptions::default()
    }
}

/// `applyEffects(effects, makeContext(sink, null, { controller }))`.
fn apply_as(sink: &mut SinkFor, state: &mut GameState, controller: PlayerId, list: Vec<Effect>) {
    sink.run(state, |engine| {
        let mut ctx = resolve::make_context(engine, None, as_controller(controller));
        resolve::apply_effects(&list, &mut ctx);
    });
}

/// TS's `string | null` refusal (or `{ error? }`): the message, or `None`.
fn refusal<T>(result: Result<T, EngineError>) -> Option<String> {
    result.err().map(|error| error.message)
}

/// `expect(() => …).toThrow(…)`: the panic's message.
fn panic_text<T>(run: impl FnOnce() -> T) -> String {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(_) => panic!("expected a panic"),
        Err(payload) => payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_default(),
    }
}

/// p1's main phase on turn 4, so nothing placed with `put` is summoning sick (§4.1).
fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = catalog::registered_catalog().clone();
    for entry in defs() {
        catalog.insert(entry.id.clone(), entry);
    }
    register_catalog(catalog);
    let mut all = scripts::registered_scripts();
    all.extend(scripts());
    register_scripts(all);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    state
}

thread_local! {
    /// TS's module-level `let nonce`.
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

fn act_result(state: &GameState, player_id: PlayerId, body: ActionBody) -> ReduceResult {
    let nonce = NONCE.with(|counter| {
        counter.set(counter.get() + 1);
        counter.get()
    });
    reduce::reduce(state, &Action::new(body, player_id, format!("rc{nonce}")))
}

fn act(state: &GameState, player_id: PlayerId, body: ActionBody) -> GameState {
    let result = act_result(state, player_id, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// A `play` action body from TS's object literal (its fields after `type: "play"`).
fn play_body(fields: Value) -> ActionBody {
    let mut body = json!({ "type": "play" });
    spread(&mut body, fields);
    json_as(body)
}

/// A `PlayAction` from TS's object literal (its fields after `type: "play"`).
fn play_action(fields: Value) -> PlayAction {
    let mut action = json!({ "type": "play" });
    spread(&mut action, fields);
    json_as(action)
}

/// `{ type: "activate", instanceId }`.
fn activate(instance_id: &str) -> ActivateAction {
    json_as(json!({ "type": "activate", "instanceId": instance_id }))
}

/// `{ ingredients, target }`.
fn fuse_args(ingredients: &[&CardInstance], target: &CardInstance) -> FuseArgs {
    json_as(json!({ "ingredients": ingredients, "target": target }))
}

/// Past the mulligans, in p1's main phase, with the note log in p1's backrow lane 5. R117 and R118
/// are about the play pipeline of §10.5, so they go through `reduce` rather than driving it.
fn playing(seed: &str) -> GameState {
    let mut state = reduce::begin_game(&game(seed)).state;
    let keep = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    state = act(&state, P1, ActionBody::Mulligan { keep });
    let keep = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
    state = act(&state, P2, ActionBody::Mulligan { keep });
    put(&mut state, LOG_CARD, slot(P1, Row::Backrow, NOTE_LANE));
    state
}

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("expected {what}"))
}

fn on_unit(instance: &CardInstance) -> AttackTarget {
    AttackTarget::Unit {
        instance: instance.clone(),
    }
}

fn on_hero(player: PlayerId) -> AttackTarget {
    AttackTarget::Hero { player }
}

/// One damage instance: (source, target, amount).
#[derive(Debug, PartialEq)]
struct Hit {
    from: String,
    to: String,
    amount: i32,
}

impl Hit {
    fn new(from: &str, to: &str, amount: i32) -> Hit {
        Hit {
            from: from.to_string(),
            to: to.to_string(),
            amount,
        }
    }
}

/// Every damage instance of a combat, as (source, target, amount) in the order they landed.
fn hits(events: &[GameEvent]) -> Vec<Hit> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                source_id,
                target_id,
                amount,
                ..
            } => Some(Hit {
                from: source_id.clone().unwrap_or_default(),
                to: target_id.clone(),
                amount: *amount,
            }),
            _ => None,
        })
        .collect()
}

/// `eventsOfType(events, type)`, counted.
fn count_of(events: &[GameEvent], kind: GameEventType) -> usize {
    events.iter().filter(|event| event.event_type() == kind).count()
}

/// The `instanceId` of every `trapFired` event.
fn trap_fired_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::TrapFired { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

/// The `attackerId` of every `attackDeclared` event.
fn attackers_declared(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::AttackDeclared { attacker_id, .. } => Some(attacker_id.clone()),
            _ => None,
        })
        .collect()
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// The ids of a player's active units.
fn unit_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    zones::active_units_of(state, player)
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

/// A `cardPlayed` event, the one every trap fixture below watches.
fn played(player: PlayerId) -> GameEvent {
    json_as(
        json!({ "type": "cardPlayed", "player": player, "instanceId": "c9001", "defId": BODY, "costPaid": 1 }),
    )
}

/// A wire value as JSON, so a name compares the same whatever enum or string carries it.
fn as_json<T: serde::Serialize + ?Sized>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

// ---------------------------------------------------------------------------
// R91, R92: positions (§4.1).
// ---------------------------------------------------------------------------

mod spec_11_r91_r96_positions_and_combat_m3_gate {
    use super::*;

    fn to(position: Position) -> SwitchPositionOptions {
        SwitchPositionOptions {
            to: Some(position),
            ..SwitchPositionOptions::default()
        }
    }

    #[test]
    fn r91_does_nothing_when_a_unit_is_switched_to_the_position_it_already_holds() {
        let mut state = game("r91-noop");
        let unit = put(&mut state, BODY, slot(P1, Row::Units, 1));
        assert_eq!(live(&state, &unit).position, Some(Position::Atk));

        // The player's action, naming the position the unit is already in: no event, no exertion.
        let mut attack_events = SinkFor::new(&state);
        let current = live(&state, &unit);
        assert_eq!(
            refusal(attack_events.run(&mut state, |sink| combat::switch_position(
                sink,
                &current,
                to(Position::Atk)
            ))),
            None
        );
        assert!(attack_events.events.is_empty());
        assert_eq!(live(&state, &unit).position, Some(Position::Atk));
        assert_eq!(
            live(&state, &unit).exertion,
            Exertion {
                attacked: false,
                switched: false,
                attacks: None
            }
        );
        assert!(combat::has_exertion(
            &state,
            &live(&state, &unit),
            ExertionKind::Switch
        ));

        // The same on the other face: a real flip spends the exertion and emits, a repeat does neither.
        let mut flip_events = SinkFor::new(&state);
        let current = live(&state, &unit);
        assert_eq!(
            refusal(flip_events.run(&mut state, |sink| combat::switch_position(
                sink,
                &current,
                to(Position::Def)
            ))),
            None
        );
        assert_eq!(count_of(&flip_events.events, GameEventType::PositionSwitched), 1);
        assert!(live(&state, &unit).exertion.switched);

        live_mut(&mut state, &unit).exertion = Exertion {
            attacked: false,
            switched: false,
            attacks: None,
        };
        let mut repeat_events = SinkFor::new(&state);
        let current = live(&state, &unit);
        assert_eq!(
            refusal(repeat_events.run(&mut state, |sink| combat::switch_position(
                sink,
                &current,
                to(Position::Def)
            ))),
            None
        );
        assert!(repeat_events.events.is_empty());
        assert_eq!(live(&state, &unit).position, Some(Position::Def));
        assert!(!live(&state, &unit).exertion.switched);

        // #48's "switch every unit" reaches units already in the position it would set (R20's effect
        // path spends no exertion either way, so the event list is what says nothing happened).
        let mut sweep_events = SinkFor::new(&state);
        let already = put(&mut state, BODY, slot(P1, Row::Units, 2));
        assert_eq!(
            refusal(sweep_events.run(&mut state, |sink| {
                combat::switch_position(
                    sink,
                    &already,
                    SwitchPositionOptions {
                        to: Some(Position::Atk),
                        spend_exertion: Some(false),
                    },
                )
            })),
            None
        );
        assert!(sweep_events.events.is_empty());
        assert_eq!(live(&state, &already).position, Some(Position::Atk));
    }

    #[test]
    fn r91_is_not_an_error_even_for_a_unit_that_has_already_acted_this_turn() {
        let mut state = game("r91-spent");
        let unit = put(&mut state, BODY, slot(P1, Row::Units, 1));
        live_mut(&mut state, &unit).exertion.attacked = true; // it attacked this turn, so it has no switch left (§4.1, R6)

        // R91: "Does nothing: no error, no event and no exertion spent." Nothing about the unit's
        // exertion changes that, because nothing is being spent — there is no switch to refuse.
        let mut events = SinkFor::new(&state);
        let current = live(&state, &unit);
        assert_eq!(
            refusal(events.run(&mut state, |sink| combat::switch_position(
                sink,
                &current,
                to(Position::Atk)
            ))),
            None
        );
        assert!(events.events.is_empty());
        assert_eq!(live(&state, &unit).position, Some(Position::Atk));
        assert_eq!(
            live(&state, &unit).exertion,
            Exertion {
                attacked: true,
                switched: false,
                attacks: None
            }
        );
    }

    #[test]
    fn r92_gives_a_position_only_to_a_card_on_the_field_so_a_dormant_or_absent_card_cannot_be_switched() {
        let mut state = game("r92-field-only");

        // In hand: no position at all, and the switch is refused rather than inventing one.
        let held = must(
            in_hand(&mut state, BODY, P1, 1).into_iter().next(),
            "a unit in hand",
        );
        assert_eq!(held.position, None);
        assert_eq!(
            refusal(with_sink(&mut state, |sink| {
                combat::switch_position(sink, &held, SwitchPositionOptions::default())
            })),
            Some("that unit is not on the field".to_string())
        );
        assert_eq!(live(&state, &held).position, None);

        // On the field it switches like any other unit.
        let under = put(&mut state, BODY, slot(P1, Row::Units, 1));
        assert_eq!(
            refusal(with_sink(&mut state, |sink| {
                combat::switch_position(sink, &under, SwitchPositionOptions::default())
            })),
            None
        );
        assert_eq!(live(&state, &under).position, Some(Position::Def));

        // Dormant under a Stack: in the lane, but not on the field for anything (§3.2, R13). The same
        // refusal R13 gives an attack, and the position it had is left exactly where it was.
        live_mut(&mut state, &under).exertion = Exertion {
            attacked: false,
            switched: false,
            attacks: None,
        }; // a fresh turn's worth, so a spend shows
        let mut top = new_instance(&mut state, &stacker.id, P1, Zone::Hand { player: P1 });
        assert!(zones::place_on_field(
            &mut state,
            &mut top,
            slot(P1, Row::Units, 1),
            PlaceOnFieldOptions { stack: Some(true) },
        ));
        let mut dormant_events = SinkFor::new(&state);
        let dormant = live(&state, &under);
        assert_eq!(
            refusal(dormant_events.run(&mut state, |sink| {
                combat::switch_position(sink, &dormant, SwitchPositionOptions::default())
            })),
            Some("that unit is not on the field".to_string())
        );
        assert!(dormant_events.events.is_empty());
        assert_eq!(live(&state, &under).position, Some(Position::Def));
        assert!(!live(&state, &under).exertion.switched);
        // Only the top of the pile has a position to switch.
        let top = live(&state, &top);
        assert_eq!(
            refusal(with_sink(&mut state, |sink| {
                combat::switch_position(sink, &top, SwitchPositionOptions::default())
            })),
            None
        );

        // Off the field entirely: the same refusal, even though the instance still remembers a position.
        let mut left = put(&mut state, BODY, slot(P1, Row::Units, 3));
        zones::remove_from_any_zone(&mut state, &mut left);
        left.zone = Zone::Graveyard { player: P1 };
        assert_eq!(
            refusal(with_sink(&mut state, |sink| {
                combat::switch_position(sink, &left, SwitchPositionOptions::default())
            })),
            Some("that unit is not on the field".to_string())
        );
    }

    // ---------------------------------------------------------------------------
    // R93, R94: how many times each unit strikes, and with what (§4.3).
    // ---------------------------------------------------------------------------

    #[test]
    fn r93_has_a_first_strike_unit_strike_once_on_whichever_side_it_is_step_1_is_when_its_strike_happens_not_an_extra_one()
     {
        // A First Strike attacker against a survivor: one hit each, not two for the attacker.
        let mut state = game("r93-attacker");
        let attacker = put(&mut state, QUICK, slot(P1, Row::Units, 1)); // 2/20 First Strike
        let defender = put(&mut state, BODY, slot(P2, Row::Units, 1)); // 4/20
        let mut events = SinkFor::new(&state);
        assert_eq!(
            refusal(events.run(&mut state, |sink| combat::declare_attack(
                sink,
                &attacker,
                &on_unit(&defender)
            ))),
            None
        );
        assert_eq!(
            hits(&events.events),
            vec![
                Hit::new(&attacker.id, &defender.id, 2),
                Hit::new(&defender.id, &attacker.id, 4)
            ]
        );
        assert_eq!(live(&state, &defender).damage, 2);
        assert_eq!(live(&state, &attacker).damage, 4);

        // A First Strike defender: likewise one hit each, the defender's first.
        let mut other = game("r93-defender");
        let plain_attacker = put(&mut other, BODY, slot(P1, Row::Units, 1));
        let quick_defender = put(&mut other, QUICK, slot(P2, Row::Units, 1));
        let mut other_events = SinkFor::new(&other);
        assert_eq!(
            refusal(other_events.run(&mut other, |sink| {
                combat::declare_attack(sink, &plain_attacker, &on_unit(&quick_defender))
            })),
            None
        );
        assert_eq!(
            hits(&other_events.events),
            vec![
                Hit::new(&quick_defender.id, &plain_attacker.id, 2),
                Hit::new(&plain_attacker.id, &quick_defender.id, 4)
            ]
        );

        // The half §4.3 used to leave unstated, and the reason R93 needed rewording: a First Strike
        // DEFENDER that kills its attacker in step 1 takes nothing back. #20 Pointmaster is a 7/2, so
        // a 3/3 attacking into it dies before its own blow lands and Pointmaster survives on 2 health
        // — which is exactly the case the old wording ("First Strike only moves the attacker's strike
        // earlier") got backwards while the engine had it right all along.
        let mut lethal = game("r93-defender-kills");
        let attacking = put(&mut lethal, DOOMED, slot(P1, Row::Units, 1));
        let point = put(&mut lethal, POINTMASTER, slot(P2, Row::Units, 1));
        let mut lethal_events = SinkFor::new(&lethal);
        assert_eq!(
            refusal(lethal_events.run(&mut lethal, |sink| combat::declare_attack(
                sink,
                &attacking,
                &on_unit(&point)
            ))),
            None
        );
        assert_eq!(
            hits(&lethal_events.events),
            vec![Hit::new(&point.id, &attacking.id, 7)]
        );
        assert_eq!(ids(&lethal.players.p1.graveyard), vec![attacking.id.clone()]);
        assert_eq!(live(&lethal, &point).damage, 0);

        // Two First Strikers trade in step 1: two hits in all, not four, and neither waits for step 2.
        let mut trade = game("r93-trade");
        let left = put(&mut trade, QUICK, slot(P1, Row::Units, 1));
        let right = put(&mut trade, QUICK, slot(P2, Row::Units, 1));
        let mut trade_events = SinkFor::new(&trade);
        assert_eq!(
            refusal(trade_events.run(&mut trade, |sink| combat::declare_attack(
                sink,
                &left,
                &on_unit(&right)
            ))),
            None
        );
        assert_eq!(
            hits(&trade_events.events),
            vec![Hit::new(&left.id, &right.id, 2), Hit::new(&right.id, &left.id, 2)]
        );
    }

    #[test]
    fn r94_reads_the_attackers_attack_once_per_combat_before_a_first_strike_defenders_hit_lands() {
        let mut state = game("r94-attacker");
        put(&mut state, WEAKENER, slot(P1, Row::Backrow, 1)); // a damaged unit loses 3 attack
        let attacker = put(&mut state, BODY, slot(P1, Row::Units, 1)); // 4/20, undamaged
        let defender = put(&mut state, QUICK, slot(P2, Row::Units, 1)); // 2/20 First Strike
        assert_eq!(layers::unit_view(&state, &live(&state, &attacker)).attack, 4);

        let mut events = SinkFor::new(&state);
        assert_eq!(
            refusal(events.run(&mut state, |sink| combat::declare_attack(
                sink,
                &attacker,
                &on_unit(&defender)
            ))),
            None
        );

        // The defender strikes in step 1, the attacker is damaged by it, and the aura drops its
        // *current* attack to 1 — so "read once, at the start" and "read now" are different numbers.
        assert_eq!(live(&state, &attacker).damage, 2);
        assert_eq!(
            layers::unit_view(&state, &live(&state, &attacker)).attack,
            4 - WEAKEN_BY
        );
        // R94: the attack was read at the start of the combat, so the attacker still struck for 4.
        assert_eq!(
            hits(&events.events),
            vec![
                Hit::new(&defender.id, &attacker.id, 2),
                Hit::new(&attacker.id, &defender.id, 4)
            ]
        );
    }

    #[test]
    fn r94_reads_the_defenders_attack_once_per_combat_so_a_first_strike_survivor_is_struck_back_with_the_attack_it_had_before_the_hit()
     {
        let mut state = game("r94-defender");
        put(&mut state, WEAKENER, slot(P1, Row::Backrow, 1)); // a damaged unit loses 3 attack
        let attacker = put(&mut state, QUICK, slot(P1, Row::Units, 1)); // 2/20 First Strike
        let defender = put(&mut state, BODY, slot(P2, Row::Units, 1)); // 4/20, undamaged
        assert_eq!(layers::unit_view(&state, &live(&state, &defender)).attack, 4);

        let mut events = SinkFor::new(&state);
        assert_eq!(
            refusal(events.run(&mut state, |sink| combat::declare_attack(
                sink,
                &attacker,
                &on_unit(&defender)
            ))),
            None
        );

        // The attacker's step-1 hit damages the defender, and the aura drops its *current* attack to 1,
        // so the two readings are different numbers rather than the same one twice.
        assert_eq!(live(&state, &defender).damage, 2);
        assert_eq!(
            layers::unit_view(&state, &live(&state, &defender)).attack,
            4 - WEAKEN_BY
        );
        // R94: "A First Strike survivor is struck back with the attack the defender had before the hit
        // landed" — 4, not the 1 the aura leaves it on.
        assert_eq!(
            hits(&events.events),
            vec![
                Hit::new(&attacker.id, &defender.id, 2),
                Hit::new(&defender.id, &attacker.id, 4)
            ]
        );
    }

    // ---------------------------------------------------------------------------
    // R95, R96: Cleave, and a forced attacker that is gone.
    // ---------------------------------------------------------------------------

    #[test]
    fn r95_lands_cleave_on_the_attackers_own_hit_never_on_the_strike_back_and_never_on_a_hero_target() {
        let mut state = game("r95-cleave");
        let attacker = put(&mut state, CLEAVER, slot(P1, Row::Units, 3)); // 3/20 Cleave
        // The attacker's own neighbours, which adjacency never crosses sides to reach (§3.1).
        let mine = [
            put(&mut state, BODY, slot(P1, Row::Units, 2)),
            put(&mut state, BODY, slot(P1, Row::Units, 4)),
        ];
        let left = put(&mut state, BODY, slot(P2, Row::Units, 1));
        let defender = put(&mut state, BODY, slot(P2, Row::Units, 2));
        let right = put(&mut state, BODY, slot(P2, Row::Units, 3));

        let mut events = SinkFor::new(&state);
        assert_eq!(
            refusal(events.run(&mut state, |sink| combat::declare_attack(
                sink,
                &attacker,
                &on_unit(&defender)
            ))),
            None
        );

        // The attacker's hit, then the two Cleave instances immediately after it, then the strike-back.
        assert_eq!(
            hits(&events.events),
            vec![
                Hit::new(&attacker.id, &defender.id, 3),
                Hit::new(&attacker.id, &left.id, 3),
                Hit::new(&attacker.id, &right.id, 3),
                Hit::new(&defender.id, &attacker.id, 4),
            ]
        );
        assert_eq!(
            mine.iter()
                .map(|unit| live(&state, unit).damage)
                .collect::<Vec<_>>(),
            vec![0, 0]
        );

        // A Cleave *defender* cleaves nothing: Cleave rides the attacker's hit, not the strike-back.
        let mut back = game("r95-strike-back");
        let plain_attacker = put(&mut back, BODY, slot(P1, Row::Units, 2));
        let my_neighbours = [
            put(&mut back, BODY, slot(P1, Row::Units, 1)),
            put(&mut back, BODY, slot(P1, Row::Units, 3)),
        ];
        let cleave_defender = put(&mut back, CLEAVER, slot(P2, Row::Units, 2));
        let mut back_events = SinkFor::new(&back);
        assert_eq!(
            refusal(back_events.run(&mut back, |sink| {
                combat::declare_attack(sink, &plain_attacker, &on_unit(&cleave_defender))
            })),
            None
        );
        assert_eq!(
            hits(&back_events.events),
            vec![
                Hit::new(&plain_attacker.id, &cleave_defender.id, 4),
                Hit::new(&cleave_defender.id, &plain_attacker.id, 3)
            ]
        );
        assert_eq!(
            my_neighbours
                .iter()
                .map(|unit| live(&back, unit).damage)
                .collect::<Vec<_>>(),
            vec![0, 0]
        );

        // A hero target cleaves nothing, since no unit is adjacent to a hero (§4.4 step 10, R63).
        let mut hero = game("r95-hero");
        let hero_cleaver = put(&mut hero, CLEAVER, slot(P1, Row::Units, 1));
        let bystanders = [
            put(&mut hero, BODY, slot(P2, Row::Units, 1)),
            put(&mut hero, BODY, slot(P2, Row::Units, 2)),
        ];
        let mut hero_events = SinkFor::new(&hero);
        assert_eq!(
            refusal(hero_events.run(&mut hero, |sink| combat::declare_attack(
                sink,
                &hero_cleaver,
                &on_hero(P2)
            ))),
            None
        );
        assert_eq!(
            hits(&hero_events.events),
            vec![Hit::new(&hero_cleaver.id, "hero-p2", 3)]
        );
        assert_eq!(
            bystanders
                .iter()
                .map(|unit| live(&hero, unit).damage)
                .collect::<Vec<_>>(),
            vec![0, 0]
        );
    }

    #[test]
    fn r96_skips_a_forced_attacker_that_is_already_gone_in_silence_and_stops_once_the_game_has_a_result() {
        let mut state = game("r96-gone");
        let mut gone = put(&mut state, BODY, slot(P1, Row::Units, 1));
        let present = put(&mut state, BODY, slot(P1, Row::Units, 2));
        let victim = put(&mut state, BODY, slot(P2, Row::Units, 1));

        // Whatever removed the first attacker — a trap, its own Death, a bounce — it is not there when
        // the sequence reaches it (R53 only says the sequence stops when the *target* is gone).
        zones::remove_from_any_zone(&mut state, &mut gone);
        gone.zone = Zone::Graveyard { player: P1 };

        let mut events = SinkFor::new(&state);
        events.run(&mut state, |sink| {
            combat::force_attacks_on(sink, &[gone.clone(), present.clone()], &on_unit(&victim), None);
        });

        // No `attackDeclared` for the missing attacker, and nothing else names it either.
        assert_eq!(attackers_declared(&events.events), vec![present.id.clone()]);
        assert!(
            !events
                .events
                .iter()
                .any(|event| serde_json::to_string(event).expect("an event").contains(&gone.id))
        );
        assert_eq!(
            hits(&events.events),
            vec![
                Hit::new(&present.id, &victim.id, 4),
                Hit::new(&victim.id, &present.id, 4)
            ]
        );

        // And the sequence stops once the game has a result, before the first attacker declares.
        let mut over = game("r96-over");
        let first = put(&mut over, BODY, slot(P1, Row::Units, 1));
        let second = put(&mut over, BODY, slot(P1, Row::Units, 2));
        let target = put(&mut over, BODY, slot(P2, Row::Units, 1));
        over.result = Some(GameResult {
            winner: Winner::Draw,
            reason: GameOverReason::TurnCap,
        });
        let mut over_events = SinkFor::new(&over);
        over_events.run(&mut over, |sink| {
            combat::force_attacks_on(sink, &[first.clone(), second.clone()], &on_unit(&target), None);
        });
        assert!(over_events.events.is_empty());
        assert_eq!(
            vec![
                live(&over, &first).damage,
                live(&over, &second).damage,
                live(&over, &target).damage
            ],
            vec![0, 0, 0]
        );
    }
}

// ---------------------------------------------------------------------------
// R99, R100: a trap's condition and the trap window's events (§5.1, §10.3, R62).
// ---------------------------------------------------------------------------

mod spec_11_r99_r101_traps_and_tribute_m3_gate {
    use super::*;

    #[test]
    fn r99_fires_a_trap_only_when_its_when_admits_the_event_spends_it_on_an_empty_effect_list_and_answers_either_side_without_a_predicate()
     {
        let mut state = game("r99-predicate");
        let trap = put(&mut state, PICKY_TRAP, slot(P1, Row::Backrow, 1));

        // The trigger's `on` matches an event p1 caused, but the predicate refuses it: the trap is not
        // fired, not consumed and not turned face up, because R61 leaves `run` no way to say so.
        assert_eq!(
            traps::traps_watching(&state, &played(P1))
                .iter()
                .map(|found| found.trap.id.clone())
                .collect::<Vec<_>>(),
            vec![trap.id.clone()]
        );
        let mut mine_sink = SinkFor::new(&state);
        assert!(
            mine_sink
                .run(&mut state, |sink| traps::fire_traps_for(sink, &played(P1)))
                .fired
                .is_empty()
        );
        assert!(trap_fired_ids(&mine_sink.events).is_empty());
        assert_eq!(
            zones::card_at(&state, slot(P1, Row::Backrow, 1)).map(|card| card.id.clone()),
            Some(trap.id.clone())
        );
        assert_ne!(live(&state, &trap).face_up, Some(true));
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);

        // The same event from the other side satisfies the predicate, so the trap fires and is spent.
        let mut theirs_sink = SinkFor::new(&state);
        assert_eq!(
            theirs_sink
                .run(&mut state, |sink| traps::fire_traps_for(sink, &played(P2)))
                .fired,
            vec![trap.id.clone()]
        );
        assert_eq!(trap_fired_ids(&theirs_sink.events), vec![trap.id.clone()]);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);
        assert!(zones::card_at(&state, slot(P1, Row::Backrow, 1)).is_none());
        assert_eq!(ids(&state.players.p1.graveyard), vec![trap.id.clone()]);

        // With the predicate satisfied, an empty effect list still spends the trap (R61).
        let mut empty = game("r99-empty");
        let spent = put(&mut empty, EMPTY_TRAP, slot(P1, Row::Backrow, 2));
        let mut empty_sink = SinkFor::new(&empty);
        assert_eq!(
            empty_sink
                .run(&mut empty, |sink| traps::fire_traps_for(sink, &played(P2)))
                .fired,
            vec![spent.id.clone()]
        );
        assert_eq!(trap_fired_ids(&empty_sink.events), vec![spent.id.clone()]);
        assert!(zones::card_at(&empty, slot(P1, Row::Backrow, 2)).is_none());
        assert_eq!(ids(&empty.players.p1.graveyard), vec![spent.id.clone()]);

        // A trap that declares no predicate answers every event it names, whichever side caused it.
        let mut bare = game("r99-bare");
        let unconditional = put(&mut bare, BARE_TRAP, slot(P1, Row::Backrow, 3));
        assert_eq!(
            with_sink(&mut bare, |sink| traps::fire_traps_for(sink, &played(P1))).fired,
            vec![unconditional.id.clone()]
        );
        assert_eq!(bare.players.p2.hero.health, HERO_HEALTH - 2);
    }

    #[test]
    fn r100_gives_the_end_of_turn_window_its_own_events_so_a_turnended_trap_fires_once_per_turn_end() {
        let mut state = game("r100-window");
        let trap = put(&mut state, WINDOW_TRAP, slot(P1, Row::Backrow, 1));
        let turn_ended = GameEvent::TurnEnded {
            player: P1,
            turn: state.turn,
            unspent_mana: 0,
        };

        assert!(traps::is_trap_window_event(&turn_ended));
        // The trap is watching, so this is the withholding and not a failure to match.
        assert_eq!(
            traps::traps_watching(&state, &turn_ended)
                .iter()
                .map(|found| found.trap.id.clone())
                .collect::<Vec<_>>(),
            vec![trap.id.clone()]
        );

        // The immediate check declines the event: it belongs to the scheduled window.
        let mut immediate = SinkFor::new(&state);
        let declined = immediate.run(&mut state, |sink| traps::fire_traps_for(sink, &turn_ended));
        assert!(declined.fired.is_empty());
        assert!(!declined.paused);
        assert!(immediate.events.is_empty());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);

        // The window delivers it, once.
        let mut window = SinkFor::new(&state);
        assert_eq!(
            window
                .run(&mut state, |sink| traps::run_trap_window(sink, &turn_ended))
                .fired,
            vec![trap.id.clone()]
        );
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);

        // It is a Field Trap, so it is still armed; offering the same event to the immediate check
        // again still fires nothing, which is what "once per turn end rather than twice" means.
        assert_eq!(
            zones::card_at(&state, slot(P1, Row::Backrow, 1)).map(|card| card.id.clone()),
            Some(trap.id.clone())
        );
        assert!(
            with_sink(&mut state, |sink| traps::fire_traps_for(sink, &turn_ended))
                .fired
                .is_empty()
        );
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);

        // Every other event still goes to the immediate check, so this is not a blanket withholding.
        let mut other = game("r100-immediate");
        let bare = put(&mut other, BARE_TRAP, slot(P1, Row::Backrow, 1));
        assert!(!traps::is_trap_window_event(&played(P2)));
        assert_eq!(
            with_sink(&mut other, |sink| traps::fire_traps_for(sink, &played(P2))).fired,
            vec![bare.id.clone()]
        );
    }

    // ---------------------------------------------------------------------------
    // R101: paying a Tribute (§6.3, §3.2).
    // ---------------------------------------------------------------------------

    #[test]
    fn r101_pays_a_tribute_with_a_minimal_set_counts_a_sheep_token_2_and_refuses_the_play_when_the_board_cannot_pay()
     {
        let mut state = game("r101-tribute");
        let card = must(
            in_hand(&mut state, TRIBUTE_TWO, P1, 1).into_iter().next(),
            "a Tribute 2 card in hand",
        );
        assert_eq!(play_choices::tribute_cost_of(&state, &card), 2);
        let play = |state: &GameState, tributes: Vec<String>| -> Option<String> {
            refusal(play_choices::why_choices_refused(
                state,
                P1,
                &card,
                &play_action(json!({
                    "instanceId": card.id,
                    "zone": { "row": "units", "lane": 3 },
                    "tributes": tributes,
                })),
            ))
        };

        // An unpayable Tribute makes the play illegal rather than fizzling on resolution (#66), and the
        // message names the card and the number: "<name> needs Tribute N".
        let needs = format!(
            "{} needs Tribute 2",
            catalog::def_of(Some(&state), &card.def_id).name
        );
        assert_eq!(play(&state, vec![]), Some(needs.clone()));
        let one = put(&mut state, BODY, slot(P1, Row::Units, 1));
        assert_eq!(play(&state, vec![one.id.clone()]), Some(needs.clone()));

        // Two ordinary bodies pay it exactly.
        let two = put(&mut state, BODY, slot(P1, Row::Units, 2));
        assert_eq!(play(&state, vec![one.id.clone(), two.id.clone()]), None);

        // Minimal: no unit could be dropped from the set and it still pay. A third body could be, so
        // the set of three is refused even though it pays.
        let three = put(&mut state, BODY, slot(P1, Row::Units, 4));
        assert!(
            must(
                play(&state, vec![one.id.clone(), two.id.clone(), three.id.clone()]),
                "a refusal"
            )
            .contains("tributes 2, no more")
        );

        // The Sheep Token counts 2, which is what makes overshooting unavoidable: it pays Tribute 2 on
        // its own, and is minimal doing it, while pairing it with a body is not.
        let woolly = put(&mut state, SHEEP, slot(P1, Row::Units, 5));
        assert_eq!(play_choices::tribute_value_of(&state, &woolly), 2);
        assert_eq!(play_choices::tribute_value_of(&state, &one), 1);
        assert_eq!(play(&state, vec![woolly.id.clone()]), None);
        assert!(
            must(play(&state, vec![woolly.id.clone(), one.id.clone()]), "a refusal")
                .contains("tributes 2, no more")
        );

        // The enumeration says the same: the Sheep alone, or two bodies, and never three of anything.
        let sets = play_choices::legal_tribute_sets(&state, P1, &card);
        assert!(sets.contains(&vec![woolly.id.clone()]));
        assert!(sets.contains(&vec![one.id.clone(), two.id.clone()]));
        assert!(sets.iter().all(|set| set.len() <= 2));
        assert!(!sets.contains(&vec![one.id.clone(), two.id.clone(), three.id.clone()]));
    }

    #[test]
    fn r101_lets_only_a_card_that_says_so_tribute_the_opponents_units() {
        let mut state = game("r101-enemies");
        // Two of p1's own, so its board can pay Tribute 2 on its own and the refusal below is about
        // *whose* units were named rather than about a board that cannot pay at all.
        let mine = vec![
            put(&mut state, BODY, slot(P1, Row::Units, 1)),
            put(&mut state, BODY, slot(P1, Row::Units, 2)),
        ];
        let theirs = vec![
            put(&mut state, BODY, slot(P2, Row::Units, 1)),
            put(&mut state, BODY, slot(P2, Row::Units, 2)),
            put(&mut state, SHEEP, slot(P2, Row::Units, 3)),
        ];

        // An ordinary Tribute reaches only its controller's units.
        let ordinary = must(
            in_hand(&mut state, TRIBUTE_TWO, P1, 1).into_iter().next(),
            "a Tribute 2 card",
        );
        assert!(!play_choices::may_tribute_enemy_units(&state, &ordinary));
        assert_eq!(
            play_choices::legal_tribute_units(&state, P1, &ordinary)
                .iter()
                .map(|unit| unit.id.clone())
                .collect::<Vec<_>>(),
            ids(&mine)
        );
        assert!(
            must(
                refusal(play_choices::why_choices_refused(
                    &state,
                    P1,
                    &ordinary,
                    &play_action(json!({
                        "instanceId": ordinary.id,
                        "zone": { "row": "units", "lane": 3 },
                        "tributes": [must(mine.first(), "an own unit").id, must(theirs.first(), "an enemy unit").id],
                    })),
                )),
                "a refusal"
            )
            .contains("cannot be tributed")
        );

        // #55 says so, so both sides are fodder — and an enemy Sheep is still worth 2 (§3.2 names no
        // side), which is how three of Tribute 3 can be paid by two enemy units.
        let golem = must(
            in_hand(&mut state, TRIBUTE_ENEMIES, P1, 1).into_iter().next(),
            "a Tribute 3 card that may take enemies",
        );
        assert!(play_choices::may_tribute_enemy_units(&state, &golem));
        assert_eq!(play_choices::tribute_cost_of(&state, &golem), 3);
        let mut reachable: Vec<String> = play_choices::legal_tribute_units(&state, P1, &golem)
            .iter()
            .map(|unit| unit.id.clone())
            .collect();
        reachable.sort();
        let mut everyone: Vec<String> = ids(&mine).into_iter().chain(ids(&theirs)).collect();
        everyone.sort();
        assert_eq!(reachable, everyone);
        let enemy_sheep = must(theirs.get(2), "the enemy Sheep");
        assert_eq!(play_choices::tribute_value_of(&state, enemy_sheep), 2);
        assert_eq!(
            refusal(play_choices::why_choices_refused(
                &state,
                P1,
                &golem,
                &play_action(json!({
                    "instanceId": golem.id,
                    "zone": { "row": "units", "lane": 3 },
                    "tributes": [enemy_sheep.id, must(theirs.first(), "an enemy unit").id],
                })),
            )),
            None
        );
    }
}

// ---------------------------------------------------------------------------
// R102: what a Fuse composes (§6.3, R77).
// ---------------------------------------------------------------------------

mod spec_11_r102_r103_fuse_and_the_heroic_power_surface_m3_gate {
    use super::*;

    #[test]
    fn r102_caps_the_fused_cost_and_drops_every_ingredients_cost_hook_so_the_cap_wins_over_a_cost_rewriting_hook()
     {
        // The cap: two printed costs that sum past it.
        let mut capped = game("r102-cost-cap");
        let target = put(&mut capped, FUSE_PRICEY, slot(P1, Row::Units, 1)); // cost 4
        let food = must(
            in_hand(&mut capped, FUSE_DEAR, P1, 1).into_iter().next(),
            "a second ingredient",
        ); // cost 3
        let result = must(
            with_sink(&mut capped, |sink| {
                fuse(sink, fuse_args(&[&target, &food], &target))
            }),
            "a fusion",
        );
        assert_eq!(
            catalog::def_of(Some(&capped), &result.def_id).cost,
            CardCost::Fixed(FUSE_COST_CAP)
        );

        // The hook: its printed cost is 3 and its hook answers 1, so the fused sum is 1 + 1 = 2. The
        // hook is dropped, so the result reports 2; a surviving hook would still report 1.
        let mut hooked = game("r102-cost-hook");
        let hook_target = put(&mut hooked, FUSE_HOOKED, slot(P1, Row::Units, 1));
        assert_eq!(mana::printed_cost(&hooked, &live(&hooked, &hook_target)), 1); // the hook, not the printed 3
        let cheap = must(
            in_hand(&mut hooked, FUSE_CHEAP, P1, 1).into_iter().next(),
            "a 1-cost ingredient",
        );
        let hook_result = must(
            with_sink(&mut hooked, |sink| {
                fuse(sink, fuse_args(&[&hook_target, &cheap], &hook_target))
            }),
            "a fusion",
        );

        assert!(
            scripts::scripts_for(&hooked, &hook_result.def_id)
                .base
                .cost
                .is_none()
        );
        assert!(
            scripts::scripts_for(&hooked, &hook_result.def_id)
                .radiant
                .cost
                .is_none()
        );
        assert_eq!(
            catalog::def_of(Some(&hooked), &hook_result.def_id).cost,
            CardCost::Fixed(2)
        );
        assert_eq!(mana::printed_cost(&hooked, &live(&hooked, &hook_result)), 2);
    }

    #[test]
    fn r102_takes_the_max_of_the_ingredients_staticflags_tribute_rather_than_the_sum() {
        let mut state = game("r102-tribute-max");
        let target = put(&mut state, FUSE_TRIBUTE_TWO, slot(P1, Row::Units, 1)); // Tribute 2
        let food = must(
            in_hand(&mut state, FUSE_TRIBUTE_THREE, P1, 1).into_iter().next(),
            "a Tribute 3 ingredient",
        );
        assert_eq!(play_choices::tribute_cost_of(&state, &target), 2);
        assert_eq!(play_choices::tribute_cost_of(&state, &food), 3);

        let result = must(
            with_sink(&mut state, |sink| {
                fuse(sink, fuse_args(&[&target, &food], &target))
            }),
            "a fusion",
        );
        // The stricter of the two requirements, not a doubled one: 3, never 5.
        assert_eq!(
            scripts::scripts_for(&state, &result.def_id)
                .base
                .static_flags
                .as_ref()
                .and_then(|flags| flags.tribute),
            Some(3)
        );
        assert_eq!(play_choices::tribute_cost_of(&state, &live(&state, &result)), 3);
    }

    #[test]
    fn r102_namespaces_trigger_ids_by_ingredient_so_two_ingredients_that_share_an_id_stay_two_conditions() {
        let mut state = game("r102-trigger-ids");
        let target = put(&mut state, FUSE_TRAP_MINE, slot(P1, Row::Backrow, 1));
        let food = must(
            in_hand(&mut state, FUSE_TRAP_THEIRS, P1, 1).into_iter().next(),
            "a second trap",
        );
        // Both ingredients call their trigger "fire" and both watch `cardPlayed`.
        assert_eq!(
            scripts::scripts_for(&state, FUSE_TRAP_MINE)
                .base
                .triggers
                .iter()
                .map(|trigger| trigger.id.clone())
                .collect::<Vec<_>>(),
            vec!["fire"]
        );
        assert_eq!(
            scripts::scripts_for(&state, FUSE_TRAP_THEIRS)
                .base
                .triggers
                .iter()
                .map(|trigger| trigger.id.clone())
                .collect::<Vec<_>>(),
            vec!["fire"]
        );

        let mut sink = SinkFor::new(&state);
        let result = must(
            sink.run(&mut state, |engine| {
                fuse(engine, fuse_args(&[&target, &food], &target))
            }),
            "a fused trap",
        );
        let triggers = scripts::scripts_for(&state, &result.def_id).base.triggers.clone();
        assert_eq!(
            triggers
                .iter()
                .map(|trigger| trigger.id.clone())
                .collect::<Vec<_>>(),
            vec![
                format!("{FUSE_TRAP_MINE}:fire"),
                format!("{FUSE_TRAP_THEIRS}:fire")
            ]
        );
        assert_eq!(
            triggers
                .iter()
                .map(|trigger| trigger.on.clone())
                .collect::<Vec<_>>(),
            vec![vec![GameEventType::CardPlayed], vec![GameEventType::CardPlayed]]
        );
        // Each kept its own R99 predicate, so only the one whose condition was met runs: p2's play
        // satisfies "theirs" (2) and not "mine" (1), so the hero takes 2 rather than 3.
        assert!(triggers.iter().all(|trigger| trigger.when.is_some()));
        sink.run(&mut state, |engine| {
            traps::fire_traps_for(engine, &played(P2));
        });
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
    }

    #[test]
    fn r102_resolves_a_multi_target_fuses_ingredients_once_and_reuses_them_since_a_consumed_one_cannot_be_re_found()
     {
        let mut state = game("r102-multi-target");
        let host_a = put(&mut state, FUSE_HOST_A, slot(P1, Row::Units, 1)); // 1/1
        let host_b = put(&mut state, FUSE_HOST_B, slot(P1, Row::Units, 2)); // 2/2
        let mut shared = put(&mut state, FUSE_SHARED, slot(P1, Row::Units, 3)); // 7/3, the played permanent
        let mut sink = SinkFor::new(&state);

        // The first fusion consumes the shared ingredient: 1/1 plus 7/3 is 8/4.
        let first = must(
            sink.run(&mut state, |engine| {
                fuse(engine, fuse_args(&[&host_a, &shared], &host_a))
            }),
            "the first fusion",
        );
        assert_eq!(first.id, host_a.id);
        let first_base = catalog::def_of(Some(&state), &first.def_id).base.clone();
        assert_eq!((first_base.attack, first_base.health), (Some(8), Some(4)));

        // The premise of the ruling: it is held by no pile now, so nothing could look it up by id — a
        // second fusion that tried to re-find it would have nothing to fuse. (TS also read the consumed
        // object's own zone, `{ z: "gone" }`; a Rust copy has no live object to read: spec gap.)
        assert!(find_instance(&state, &shared.id).is_none());
        assert!(zones::card_at(&state, slot(P1, Row::Units, 3)).is_none());

        // R102: the ingredients were resolved once, so the same resolved instance fuses again — which is
        // what lets radiant #85 fuse one played permanent onto every matching target, one at a time.
        // TS handed the second fusion the same object, which the first had marked gone (R86).
        shared.zone = Zone::Gone { player: P1 };
        let second = must(
            sink.run(&mut state, |engine| {
                fuse(engine, fuse_args(&[&host_b, &shared], &host_b))
            }),
            "the second fusion",
        );
        assert_eq!(second.id, host_b.id);
        // 2/2 plus the same 7/3 is 9/5: it contributed its definition a second time.
        let second_def = catalog::def_of(Some(&state), &second.def_id).clone();
        assert_eq!(
            (second_def.base.attack, second_def.base.health),
            (Some(9), Some(5))
        );
        assert!(second_def.name.contains("fuse-shared"));

        // Two fusions, two transient definitions, and the first host is untouched by the second.
        assert_ne!(second.def_id, first.def_id);
        let first_base = catalog::def_of(Some(&state), &first.def_id).base.clone();
        assert_eq!((first_base.attack, first_base.health), (Some(8), Some(4)));
    }

    #[test]
    fn r102_makes_every_consumed_ingredient_cease_to_exist_no_graveyard_no_death_trigger_no_destroyed_counter()
     {
        let mut state = game("r102-consumed");
        let target = put(&mut state, FUSE_TRIBUTE_TWO, slot(P1, Row::Units, 1));
        let eaten = put(&mut state, FUSE_DYING, slot(P1, Row::Units, 2)); // a Death hook worth 7 to the hero
        let before = state.counters.destroyed;

        let mut sink = SinkFor::new(&state);
        let result = must(
            sink.run(&mut state, |engine| {
                let result = fuse(engine, fuse_args(&[&target, &eaten], &target));
                triggers::settle(engine, SettleOptions::default());
                result
            }),
            "a fusion",
        );

        assert_eq!(result.id, target.id);
        // R86: gone, not moved — no pile holds it and the lane it stood in is empty. (TS also read the
        // consumed object's zone, `{ z: "gone" }`: spec gap, as above.)
        assert!(find_instance(&state, &eaten.id).is_none());
        assert!(zones::card_at(&state, slot(P1, Row::Units, 2)).is_none());
        assert!(!ids(&state.players.p1.graveyard).contains(&eaten.id));
        assert!(!ids(&state.players.p1.exile).contains(&eaten.id));
        // No Death trigger, and it does not count as destroyed.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        assert_eq!(state.counters.destroyed, before);
        assert_eq!(count_of(&sink.events, GameEventType::Destroyed), 0);
    }

    // ---------------------------------------------------------------------------
    // R103: the Heroic Power surface (§8 #98, R43).
    // ---------------------------------------------------------------------------

    #[test]
    fn r103_stores_the_thirteen_power_names_each_its_activate_abilitys_id_a_card_that_has_not_rolled_has_none()
     {
        // §11 R103 writes them out, and they are state, so they stay stable across versions: R352 added
        // `stitching` and R752 the five after it, each at the end, and the reworked powers kept theirs.
        let names = json!([
            "recruit",
            "draw",
            "ping",
            "burn",
            "rush",
            "felinor",
            "discover",
            "stitching",
            "armor",
            "insect",
            "brainstorm",
            "pluck",
            "tricks",
        ]);
        assert_eq!(as_json(&HERO_POWER_NAMES), names);
        assert_eq!(
            Value::Array(HERO_POWERS.iter().map(|power| as_json(&power.name)).collect()),
            names
        );
        // Ping reaches any unit or hero on either side, declared with the activation (R81).
        let ping = must(
            HERO_POWERS
                .iter()
                .find(|power| as_json(&power.name) == json!("ping")),
            "the ping power",
        );
        assert_eq!(ping.x, 1);
        let ping_targets = ping.targets.map(|targets| targets()).unwrap_or_default();
        assert_eq!(
            ping_targets.first().map(|decl| as_json(&decl.filter)),
            Some(json!({ "side": "any", "of": ["unit", "hero"] }))
        );

        let mut state = game("r103-unrolled");
        let card = put(&mut state, HEROIC, slot(P1, Row::Backrow, 1));
        assert!(!live(&state, &card).memory.contains_key(POWER_KEY));
        assert_eq!(mana::printed_cost(&state, &live(&state, &card)), 0);
        assert!(power_ability_of(&state, &live(&state, &card)).is_none());
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &card.id, None)),
            Some("that card has no Activate ability".to_string())
        );

        // Once it has rolled, the card has that power's ability, by its stored name.
        live_mut(&mut state, &card)
            .memory
            .insert(POWER_KEY.to_string(), json!("recruit"));
        let ability = power_ability_of(&state, &live(&state, &card));
        assert_eq!(
            ability.as_ref().map(|decl| decl.id.clone()),
            Some("recruit".to_string())
        );
        assert_eq!(
            ability
                .as_ref()
                .and_then(|decl| decl.cost)
                .and_then(|cost| cost.mana),
            Some(3)
        );
        assert_eq!(mana::printed_cost(&state, &live(&state, &card)), 0);
    }

    #[test]
    fn r103_refuses_as_activate_does_the_turn_and_the_phase_before_the_uses_then_the_price() {
        let mut state = game("r103-order");
        let card = put(&mut state, HEROIC, slot(P1, Row::Backrow, 1));
        live_mut(&mut state, &card)
            .memory
            .insert(POWER_KEY.to_string(), json!("recruit")); // X 3

        state.players.p1.mana.current = 0;
        state.active = P2;
        state.phase = Phase::End;
        let turn = state.turn;
        live_mut(&mut state, &card).memory.insert(
            ACTIVATIONS_MEMORY_KEY.to_string(),
            json!({ "turn": turn, "count": 1 }),
        );
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &card.id, None)),
            Some("it is not your turn".to_string())
        );
        state.active = P1;
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &card.id, None)),
            Some("an ability is activated in the main phase".to_string())
        );
        state.phase = Phase::Main;
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &card.id, None)),
            Some("that ability has already been used this turn".to_string())
        );
        live_mut(&mut state, &card)
            .memory
            .shift_remove(ACTIVATIONS_MEMORY_KEY);
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &card.id, None)),
            Some("that ability costs 3, more than your mana".to_string())
        );
        state.players.p1.mana.current = 3;
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &card.id, None)),
            None
        );
    }

    #[test]
    fn r103_counts_the_use_before_the_effects_run_and_fizzles_a_token_power_in_silence_when_the_token_is_absent()
     {
        // A power that pauses on a prompt has already spent the turn's activation.
        let mut state = game("r103-marked");
        let mut sink = SinkFor::new(&state);
        let card = put(&mut state, HEROIC, slot(P1, Row::Backrow, 1));
        live_mut(&mut state, &card)
            .memory
            .insert(POWER_KEY.to_string(), json!("discover"));
        assert_eq!(
            refusal(sink.run(&mut state, |engine| activate_ability(
                engine,
                P1,
                &activate(&card.id)
            ))),
            None
        );
        assert_eq!(
            must(state.pending.as_ref(), "the Discover").kind,
            PromptKind::Discover
        );
        assert!(used_this_turn(&state, &live(&state, &card)));
        state.pending = None;
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &card.id, None)),
            Some("that ability has already been used this turn".to_string())
        );

        // A power that summons a token resolves it by catalog index and fizzles in silence when the
        // catalog has no such token (§5.3, §7).
        let mut missing = game("r103-missing-token");
        let without_token: CardDefs = catalog::registered_catalog()
            .iter()
            .filter(|(_, entry)| entry.index != RUSH_TOKEN_INDEX)
            .map(|(id, entry)| (id.clone(), entry.clone()))
            .collect();
        register_catalog(without_token);
        assert!(catalog::def_by_index(SetName::Core, RUSH_TOKEN_INDEX).is_none());

        let mut token_sink = SinkFor::new(&missing);
        let rusher = put(&mut missing, HEROIC, slot(P1, Row::Backrow, 1));
        live_mut(&mut missing, &rusher)
            .memory
            .insert(POWER_KEY.to_string(), json!("rush"));
        assert_eq!(
            refusal(token_sink.run(&mut missing, |engine| activate_ability(
                engine,
                P1,
                &activate(&rusher.id)
            ))),
            None
        );
        assert_eq!(count_of(&token_sink.events, GameEventType::Summoned), 0);
        assert!(missing.pending.is_none());
        // The use is still spent: it was counted before the effects that found nothing to do.
        assert!(used_this_turn(&missing, &live(&missing, &rusher)));
    }
}

// ---------------------------------------------------------------------------
// R113: the order paused sequences resume in (§9.3, §10.5, §10.6).
// ---------------------------------------------------------------------------

mod spec_11_r113_the_work_cursor_m3_gate {
    use super::*;

    fn resume_of(step: &str) -> Resume {
        Resume {
            def_id: BODY.to_string(),
            hook: "rc:sequence".to_string(),
            step: step.to_string(),
            radiant: false,
            instance_id: None,
            data: IndexMap::new(),
        }
    }

    fn steps(state: &GameState) -> Vec<String> {
        state.work.iter().map(|item| item.resume.step.clone()).collect()
    }

    #[test]
    fn r113_parks_one_cascades_sequences_innermost_first_and_a_pause_during_a_resumption_ahead_of_everything_owed()
     {
        let mut state = game("r113-cursor");
        let mut sink = SinkFor::new(&state);
        assert!(state.work.is_empty());
        assert_eq!(state.work_cursor, 0);

        // One pause cascade, exactly R113's example: a Cry's effect list pauses inside play step 5, so
        // the Cry parks its own tail first and the pipeline then parks steps 6 to 8.
        let (cry_tail, play_tail) = sink.run(&mut state, |engine| {
            work::begin_work_cascade(engine);
            let cry_tail = work::push_work(engine, resume_of("cry-tail"), None);
            let play_tail = work::push_work(engine, resume_of("play-steps-6-8"), None);
            (cry_tail, play_tail)
        });

        // "The sequences parked by one pause land innermost-first": step 6 comes after step 5, not
        // inside it, so the Cry's tail is owed first. A stack would have buried it.
        assert_eq!(steps(&state), vec!["cry-tail", "play-steps-6-8"]);
        assert_eq!(state.work_cursor, 2);
        assert!(cry_tail.seq < play_tail.seq);

        // "Taking an item resets the cursor to 0, so a pause that happens during a resumption is
        // inserted ahead of everything still owed": the Cry's tail resumes and opens a second prompt.
        assert_eq!(
            work::take_work(&mut state).map(|item| item.id),
            Some(cry_tail.id.clone())
        );
        assert_eq!(state.work_cursor, 0);
        let inner_tail = sink.run(&mut state, |engine| {
            work::push_work(engine, resume_of("cry-tail-after-second-prompt"), None)
        });

        // Still inside step 5, so it precedes the play steps. A plain queue would run step 6 too early.
        assert_eq!(
            steps(&state),
            vec!["cry-tail-after-second-prompt", "play-steps-6-8"]
        );
        assert_eq!(
            work::take_work(&mut state).map(|item| item.id),
            Some(inner_tail.id.clone())
        );
        assert_eq!(
            work::take_work(&mut state).map(|item| item.id),
            Some(play_tail.id.clone())
        );
        assert!(work::take_work(&mut state).is_none());
    }

    #[test]
    fn r113_raises_rather_than_drop_a_work_item_nothing_knows_how_to_resume() {
        let mut state = game("r113-raise");
        let mut sink = SinkFor::new(&state);
        // No engine sequence registered this hook and `body`'s script has no step by that name, so
        // nothing can resume it: a lost sequence must never be dropped in silence.
        let orphan = sink.run(&mut state, |engine| {
            work::push_work(
                engine,
                Resume {
                    hook: "rc:no-such-sequence".to_string(),
                    ..resume_of("never-registered")
                },
                None,
            )
        });

        let raised = panic_text(|| sink.run(&mut state, |engine| work::run_work_item(engine, &orphan)));
        assert!(raised.contains("R113"));
        let raised = panic_text(|| sink.run(&mut state, |engine| work::run_work_item(engine, &orphan)));
        assert!(raised.contains("rc:no-such-sequence"));
        // It is still owed rather than quietly gone.
        assert_eq!(
            state.work.iter().map(|item| item.id.clone()).collect::<Vec<_>>(),
            vec![orphan.id.clone()]
        );
    }
}

// ---------------------------------------------------------------------------
// R114: Trample into a unit with no health left (§4.4 steps 5 to 9, R63).
// ---------------------------------------------------------------------------

mod spec_11_r114_r116_the_damage_pipeline_and_the_stat_layers_m3_gate {
    use super::*;

    #[test]
    fn r114_deals_nothing_to_a_unit_already_at_0_health_and_sends_the_whole_trample_amount_to_its_hero() {
        let mut state = game("r114-zero-health");
        let source = put(&mut state, TRAMPLER, slot(P1, Row::Units, 1)); // 5/5 Trample, Lifesteal, Poisonous
        let victim = put(&mut state, FRAIL, slot(P2, Row::Units, 1)); // 1/3

        // The fixture really does carry all three keywords, or the three negatives below are decoration.
        assert!(layers::unit_has(
            &state,
            &live(&state, &source),
            KeywordKind::Trample
        ));
        assert!(layers::unit_has(
            &state,
            &live(&state, &source),
            KeywordKind::Lifesteal
        ));
        assert!(layers::unit_has(
            &state,
            &live(&state, &source),
            KeywordKind::Poisonous
        ));

        // The control: the same source into a *healthy* unit deals, marks and heals. Without this the
        // negatives below could pass on an inert fixture.
        let mut control = game("r114-control");
        let control_source = put(&mut control, TRAMPLER, slot(P1, Row::Units, 1));
        let control_victim = put(&mut control, FRAIL, slot(P2, Row::Units, 1));
        control.players.p1.hero.health -= 20; // room for the Lifesteal to show
        let mut control_events = SinkFor::new(&control);
        let control_before = control.players.p1.hero.health;
        assert_eq!(
            control_events.run(&mut control, |sink| {
                damage::deal_damage(
                    sink,
                    DamageArgs {
                        source: Some(control_source.clone()),
                        target: DamageTarget::Unit {
                            instance: control_victim.clone(),
                        },
                        amount: 5,
                        flags: None,
                    },
                )
            }),
            3
        );
        assert_eq!(
            hits(&control_events.events),
            vec![
                Hit::new(&control_source.id, &control_victim.id, 3),
                Hit::new(&control_source.id, "hero-p2", 2)
            ]
        );
        assert_eq!(live(&control, &control_victim).marked_destroyed, Some(true));
        assert_eq!(
            live(&control, &control_victim).last_damaged_by,
            Some(control_source.id.clone())
        );
        // 3 off the unit plus 2 off the hero: the total healed is 5.
        assert_eq!(control.players.p1.hero.health - control_before, 5);

        // Now the ruling. One earlier instance takes the victim to exactly 0 health and the state check
        // has not collected it yet (R59: no state check between the hits of one effect), so it is at 0
        // and still on the field — which is the whole premise of the row.
        with_sink(&mut state, |sink| {
            damage::deal_damage(
                sink,
                DamageArgs {
                    source: None,
                    target: DamageTarget::Unit {
                        instance: victim.clone(),
                    },
                    amount: 3,
                    flags: None,
                },
            )
        });
        assert_eq!(live(&state, &victim).damage, 3);
        assert_eq!(layers::unit_view(&state, &live(&state, &victim)).health, 0);
        assert_eq!(
            zones::card_at(&state, slot(P2, Row::Units, 1)).map(|card| card.id.clone()),
            Some(victim.id.clone())
        );
        assert_ne!(live(&state, &victim).marked_destroyed, Some(true));
        assert!(live(&state, &victim).last_damaged_by.is_none());

        state.players.p1.hero.health -= 20; // room for the Lifesteal to show
        let before = state.players.p1.hero.health;
        let mut events = SinkFor::new(&state);
        let at_zero = live(&state, &victim);
        let dealt = events.run(&mut state, |sink| {
            damage::deal_damage(
                sink,
                DamageArgs {
                    source: Some(source.clone()),
                    target: DamageTarget::Unit {
                        instance: at_zero.clone(),
                    },
                    amount: 5,
                    flags: None,
                },
            )
        });

        // Nothing to the unit: no damage dealt, no `damage` event naming it, no `lastDamagedBy` for an
        // on-damage trigger to read, and no Poisonous mark.
        assert_eq!(dealt, 0);
        assert_eq!(live(&state, &victim).damage, 3);
        assert!(live(&state, &victim).last_damaged_by.is_none());
        assert_ne!(live(&state, &victim).marked_destroyed, Some(true));
        assert!(
            events
                .events
                .iter()
                .filter(|event| serde_json::to_string(event)
                    .expect("an event")
                    .contains(&victim.id))
                .collect::<Vec<_>>()
                .is_empty()
        );

        // The whole amount becomes the step 9 instance on the victim's controller's hero.
        assert_eq!(hits(&events.events), vec![Hit::new(&source.id, "hero-p2", 5)]);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 5);

        // Lifesteal heals it once, off the hero instance only, so the total healed is unchanged: 5 here
        // and 5 in the control above.
        let healed: Vec<(String, i32)> = events
            .events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Healed { target_id, amount } => Some((target_id.clone(), *amount)),
                _ => None,
            })
            .collect();
        assert_eq!(healed, vec![("hero-p1".to_string(), 5)]);
        assert_eq!(state.players.p1.hero.health - before, 5);
    }

    // ---------------------------------------------------------------------------
    // R115, R116: Vanilla's reach into the layers, and what a set-stat hook returns.
    // ---------------------------------------------------------------------------

    #[test]
    fn r115_stops_a_vanillad_permanent_projecting_its_aura_and_setting_its_own_stats_while_it_still_receives_other_cards_auras()
     {
        let mut state = game("r115-vanilla");
        let source = put(&mut state, PROJECTOR, slot(P1, Row::Units, 1)); // aura +2/+2 to allies, sets +5/+5
        let ally = put(&mut state, MEASURED, slot(P1, Row::Units, 2)); // 3/4
        let view = |state: &GameState, card: &CardInstance| layers::unit_view(state, &live(state, card));

        // Live: the aura reaches the ally and layer 2 sets the projector's own stats.
        assert_eq!(view(&state, &ally).attack, 3 + 2);
        assert_eq!(view(&state, &ally).max_health, 4 + 2);
        assert_eq!(view(&state, &source).attack, 2 + PROJECTOR_SETS);
        assert_eq!(view(&state, &source).max_health, 2 + PROJECTOR_SETS);

        // Vanilla clears its scripts, and an aura and a set-stat hook are both scripts (§6.3, §10.9).
        live_mut(&mut state, &source).vanilla = true;

        // It stops projecting at layer 5.
        assert_eq!(view(&state, &ally).attack, 3);
        assert_eq!(view(&state, &ally).max_health, 4);
        // And stops setting at layer 2, keeping its printed face.
        assert_eq!(view(&state, &source).attack, 2);
        assert_eq!(view(&state, &source).max_health, 2);

        // But it still *receives*: another card's aura is that card's text, not this one's.
        let giver = put(&mut state, PROJECTOR, slot(P2, Row::Units, 1));
        assert_eq!(view(&state, &giver).attack, 2 + PROJECTOR_SETS); // the second card is not Vanilla'd
        live_mut(&mut state, &source).controller = P2; // bring the Vanilla'd body inside the giver's "your units" clause
        assert_eq!(view(&state, &source).attack, 2 + 2);
        assert_eq!(view(&state, &source).max_health, 2 + 2);
        // Its own hook is still silent, so the +2 is the aura's and not a revived layer 2.
        assert_ne!(view(&state, &source).attack, 2 + PROJECTOR_SETS);
    }

    #[test]
    fn r116_adds_a_set_stat_hooks_return_to_the_printed_face_as_a_delta_floored_at_0_and_never_lets_two_of_them_read_each_other()
     {
        let mut state = game("r116-delta");
        let view = |state: &GameState, card: &CardInstance| layers::unit_view(state, &live(state, card));
        let buffed =
            |state: &GameState, card: &CardInstance| layers::stats_with_buffs(state, &live(state, card));
        let one = put(&mut state, SETTER, slot(P1, Row::Units, 1)); // printed 2/6
        assert_eq!(view(&state, &one).attack, 2); // nothing to measure yet
        assert_eq!(view(&state, &one).max_health, 6);

        // A delta: the hook returns the other card's layer-4 attack, which is *added* to 2/6. A total
        // would have read 4/4, so the two readings are distinguishable.
        let two = put(&mut state, OTHER_SETTER, slot(P1, Row::Units, 2)); // printed 4/8
        assert_eq!(buffed(&state, &two).attack, 4);
        assert_eq!(view(&state, &one).attack, 2 + 4);
        assert_eq!(view(&state, &one).max_health, 6 + 4);

        // Neither reads the other's layer 2, because `statsWithBuffs` is layers 1 to 4: `two` sees
        // `one`'s printed 2, not the 6 that layer 2 gives it, so the pair cannot recurse or escalate.
        assert_eq!(view(&state, &two).attack, 4 + 2);
        assert_eq!(view(&state, &two).max_health, 8 + 2);

        // Layer 4's buffs land on top of the delta, and the hook sees them on the card it measures.
        live_mut(&mut state, &two).buffs = AttackHealth { attack: 3, health: 0 };
        assert_eq!(buffed(&state, &two).attack, 7);
        assert_eq!(view(&state, &one).attack, 2 + 7);

        // Each component is floored at 0 on its own, so a negative return never eats the printed face.
        let mut floored = game("r116-floor");
        let lone = put(&mut floored, PROJECTOR, slot(P1, Row::Units, 1)); // sets +5/+5
        let mut all = scripts::registered_scripts();
        all.insert(
            PROJECTOR.to_string(),
            both(Script {
                set_stat: Some(read_hook(|_| SetStat {
                    attack: Some(-99),
                    max_health: Some(-99),
                })),
                ..Script::default()
            }),
        );
        register_scripts(all);
        assert_eq!(view(&floored, &lone).attack, 2);
        assert_eq!(view(&floored, &lone).max_health, 2);
    }
}

// ---------------------------------------------------------------------------
// R117, R118: who owns a paused sequence's steps, and the Cry that survives a trap.
// ---------------------------------------------------------------------------

mod spec_11_r117_r118_pausing_the_play_pipeline_m3_gate {
    use super::*;

    fn answer(state: &GameState) -> ActionBody {
        ActionBody::Answer {
            choice_id: state
                .pending
                .as_ref()
                .map(|pending| pending.id.clone())
                .unwrap_or_default(),
            selection: vec![Selection::None],
        }
    }

    #[test]
    fn r117_owes_a_plays_remaining_steps_only_at_the_moment_it_pauses_never_in_advance() {
        // The control first: a play that never pauses owes nothing at all, and its Cry fires once. If
        // the pipeline parked itself in advance this queue would not be empty.
        let mut clean = playing("r117-no-pause");
        let clean_card = must(
            in_hand(&mut clean, CRIER, P1, 1).into_iter().next(),
            "a Cry unit in hand",
        );
        let straight = act_result(
            &clean,
            P1,
            play_body(json!({ "instanceId": clean_card.id, "zone": { "row": "units", "lane": 1 } })),
        );
        assert!(straight.error.is_none());
        assert!(straight.state.work.is_empty());
        assert_eq!(notes(&straight.state), vec!["cry"]);

        // Now a trap prompts partway through the same play. Only now is the remainder owed, and only
        // once: one item for one play, belonging to the player whose pipeline it is.
        let mut state = playing("r117-pause");
        put(&mut state, ASK_TRAP, slot(P2, Row::Backrow, 1));
        let card = must(
            in_hand(&mut state, CRIER, P1, 1).into_iter().next(),
            "a Cry unit in hand",
        );
        let paused = act_result(
            &state,
            P1,
            play_body(json!({ "instanceId": card.id, "zone": { "row": "units", "lane": 1 } })),
        );
        assert!(paused.error.is_none());
        assert_eq!(
            paused.state.pending.as_ref().map(|pending| pending.player_id),
            Some(P2)
        );

        let owed: Vec<&WorkItem> = paused
            .state
            .work
            .iter()
            .filter(|item| item.resume.hook == play_steps::PLAY_WORK_KIND)
            .collect();
        assert_eq!(owed.len(), 1);
        assert_eq!(owed.iter().map(|item| item.owner).collect::<Vec<_>>(), vec![P1]);
        // Ahead of it, the trap's own end (§10.3: it resolves to completion before the play goes on,
        // R113), owed at the same pause and no sooner.
        assert_eq!(
            paused
                .state
                .work
                .iter()
                .map(|item| item.resume.hook.clone())
                .collect::<Vec<_>>(),
            vec![traps::TRAP_FIRING_WORK, play_steps::PLAY_WORK_KIND]
        );

        // While the driver was on the stack the steps were the driver's alone: step 4's nested `settle`
        // drains `state.work` before it pops a trigger, and it neither took nor ran the steps it was
        // standing in — so the Cry has not fired yet.
        assert!(notes(&paused.state).is_empty());

        // The answer runs the trap's continuation and then the owed remainder, once each.
        let answered = act(&paused.state, P2, answer(&paused.state));
        assert_eq!(notes(&answered), vec!["answered", "cry"]);
        assert_eq!(
            notes(&answered)
                .iter()
                .filter(|step| step.as_str() == "cry")
                .count(),
            1
        );
        assert!(answered.work.is_empty());
        assert!(answered.pending.is_none());
    }

    #[test]
    fn r118_lets_a_traps_prompt_interrupt_a_play_without_eating_its_cry_which_still_fires_exactly_once() {
        let mut state = playing("r118-cry-survives");
        put(&mut state, ASK_TRAP, slot(P2, Row::Backrow, 1));
        let card = must(
            in_hand(&mut state, CRIER, P1, 1).into_iter().next(),
            "a Cry unit in hand",
        );

        let paused = act_result(
            &state,
            P1,
            play_body(json!({ "instanceId": card.id, "zone": { "row": "units", "lane": 1 } })),
        );
        assert!(paused.error.is_none());
        // The trap holds the play between step 4's `summoned` and step 5's Cry (§10.3, R17).
        assert_eq!(
            paused.state.pending.as_ref().map(|pending| pending.player_id),
            Some(P2)
        );
        assert!(notes(&paused.state).is_empty());

        let answered = act(&paused.state, P2, answer(&paused.state));
        // The trap resolved to completion first, then the play resumed at the step after the one that
        // paused: the Cry fires, after the trap, exactly once (R1).
        assert_eq!(notes(&answered), vec!["answered", "cry"]);
        assert!(answered.work.is_empty());
        // And the card is where the play put it, so nothing about the pause unwound the play.
        assert_eq!(
            zones::card_at(&answered, slot(P1, Row::Units, 1)).map(|placed| placed.def_id.clone()),
            Some(CRIER.to_string())
        );
        assert!(act_result(&answered, P1, ActionBody::EndTurn).error.is_none());

        // R17's "the Cry is lost" is the other case, and only that case: a trap that takes the card off
        // the field leaves nothing to resolve, so the Cry never fires.
        let mut eaten = playing("r118-cry-lost");
        put(&mut eaten, EAT_TRAP, slot(P2, Row::Backrow, 1));
        let doomed = must(
            in_hand(&mut eaten, CRIER, P1, 1).into_iter().next(),
            "a Cry unit in hand",
        );
        let after = act_result(
            &eaten,
            P1,
            play_body(json!({ "instanceId": doomed.id, "zone": { "row": "units", "lane": 1 } })),
        );
        assert!(after.error.is_none());
        assert!(after.state.pending.is_none());
        assert_eq!(notes(&after.state), vec!["eaten"]);
        assert!(zones::card_at(&after.state, slot(P1, Row::Units, 1)).is_none());
        assert!(after.state.work.is_empty());
    }
}

// ---------------------------------------------------------------------------
// R122, R123: who finishes an interrupted sequence, and how a Tribute travels.
// ---------------------------------------------------------------------------

mod spec_11_r122_r123_answering_a_prompt_and_a_declared_tribute_m3_gate {
    use super::*;

    #[test]
    fn r122_has_the_answering_action_finish_what_the_prompt_interrupted_even_driven_without_the_reducer() {
        let mut state = playing("r122-answer-drains");
        put(&mut state, ASK_ON_PLAY, slot(P2, Row::Backrow, 1));
        let spell = must(
            in_hand(&mut state, SPELL_CRIER, P1, 1).into_iter().next(),
            "a Spell in hand",
        );

        // p1 plays a Spell; p2's trap answers the `cardPlayed` and prompts, so the play stops between
        // step 4 and step 5 with its remainder owed (R117).
        let paused = act_result(&state, P1, play_body(json!({ "instanceId": spell.id })));
        assert!(paused.error.is_none());
        let mut held = paused.state;
        assert_eq!(held.pending.as_ref().map(|pending| pending.player_id), Some(P2));
        assert!(notes(&held).is_empty());
        // The premise of the row: there is something owed, and the Spell is mid-resolution.
        assert!(!held.work.is_empty());
        assert_eq!(ids(&held.players.p1.resolving), vec![spell.id.clone()]);
        assert!(!ids(&held.players.p1.graveyard).contains(&spell.id));

        // Answer it through `answerPrompt` directly rather than through `reduce`: R122 puts the drain in
        // the answering action itself, so a caller driving the engine does not lose the remainder.
        let choice_id = held
            .pending
            .as_ref()
            .map(|pending| pending.id.clone())
            .unwrap_or_default();
        assert_eq!(
            refusal(with_sink(&mut held, |sink| {
                prompts::answer_prompt(
                    sink,
                    &AnswerInput {
                        player_id: P2,
                        choice_id,
                        selection: vec![Selection::None],
                    },
                )
            })),
            None
        );

        // The trap's continuation ran, then the owed play steps, in R113's order — so the Cry fired and
        // the Spell reached its graveyard rather than stopping short of it.
        assert_eq!(notes(&held), vec!["answered", "spell"]);
        assert!(held.pending.is_none());
        assert!(held.work.is_empty());
        assert!(held.players.p1.resolving.is_empty());
        assert!(ids(&held.players.p1.graveyard).contains(&spell.id));
    }

    #[test]
    fn r123_carries_a_declared_tributes_picks_in_targets_and_its_amount_as_the_plays_tribute_cost() {
        let mut state = game("r123-tribute-travels");
        let card = must(
            in_hand(&mut state, CUBE, P1, 1).into_iter().next(),
            "a card with a tribute declaration",
        );

        // The `amount` on the declaration is the Tribute cost, not a `staticFlags.tribute`.
        assert!(
            scripts::scripts_for(&state, CUBE)
                .base
                .static_flags
                .as_ref()
                .and_then(|flags| flags.tribute)
                .is_none()
        );
        assert_eq!(
            play_choices::declared_targets(&state, &card)
                .iter()
                .map(|decl| (decl.kind, decl.amount))
                .collect::<Vec<_>>(),
            vec![(PromptKind::Tribute, Some(2))]
        );
        assert_eq!(play_choices::tribute_cost_of(&state, &card), 2);

        // An unpayable board refuses the play outright rather than fizzling on resolution (R101).
        let play = |state: &GameState, extra: Value| -> Option<String> {
            let mut fields = json!({ "instanceId": card.id, "zone": { "row": "units", "lane": 5 } });
            spread(&mut fields, extra);
            refusal(play_choices::why_choices_refused(
                state,
                P1,
                &card,
                &play_action(fields),
            ))
        };
        let needs = format!(
            "{} needs Tribute 2",
            catalog::def_of(Some(&state), &card.def_id).name
        );
        assert_eq!(play(&state, json!({})), Some(needs.clone()));

        let one = put(&mut state, BODY, slot(P1, Row::Units, 1));
        let two = put(&mut state, BODY, slot(P1, Row::Units, 2));

        // Both halves travel: the units in `tributes` as the cost, and the pick in `targets` as the
        // declared choice the script reads.
        assert_eq!(
            play(
                &state,
                json!({ "tributes": [one.id, two.id], "targets": [{ "pick": "instance", "instanceId": one.id }] })
            ),
            None
        );
        // The cost half alone is not the declaration: the pick is still required (R90).
        assert!(play(&state, json!({ "tributes": [one.id, two.id] })).is_some());
        // And the pick alone does not pay the cost.
        assert_eq!(
            play(
                &state,
                json!({ "targets": [{ "pick": "instance", "instanceId": one.id }] })
            ),
            Some(needs.clone())
        );

        // `legalActions` enumerates them that way: every offered play carries both lists.
        let offered: Vec<Value> = play_choices::play_actions_for(&state, P1, &card)
            .iter()
            .map(as_json)
            .collect();
        assert!(!offered.is_empty());
        for action in &offered {
            assert_ne!(action.get("tributes").cloned().unwrap_or(json!([])), json!([]));
            assert_ne!(action.get("targets").cloned().unwrap_or(json!([])), json!([]));
        }
        // The declared pick is offered over the same permanents the cost may take (§6.3's "your units").
        let offered_picks: BTreeSet<String> = offered
            .iter()
            .flat_map(|action| {
                action
                    .get("targets")
                    .and_then(|targets| targets.as_array())
                    .cloned()
                    .unwrap_or_default()
            })
            .map(|pick| pick.to_string())
            .collect();
        let expected_picks: BTreeSet<String> = [&one, &two]
            .iter()
            .map(|unit| json!({ "pick": "instance", "instanceId": unit.id }).to_string())
            .collect();
        assert_eq!(offered_picks, expected_picks);

        // A Sheep Token counts 2 (§3.2), so one Sheep pays the cost while the pick stays a single unit.
        let woolly = put(&mut state, SHEEP, slot(P1, Row::Units, 3));
        assert_eq!(play_choices::tribute_value_of(&state, &woolly), 2);
        assert!(play_choices::legal_tribute_sets(&state, P1, &card).contains(&vec![woolly.id.clone()]));
        assert_eq!(
            play(
                &state,
                json!({ "tributes": [woolly.id], "targets": [{ "pick": "instance", "instanceId": woolly.id }] })
            ),
            None
        );

        // And the script reads its permanent out of `ctx.targets[0]`, which is what #22 does.
        let mut running = game("r123-script-reads");
        put(&mut running, LOG_CARD, slot(P1, Row::Backrow, NOTE_LANE));
        let eater = put(&mut running, CUBE, slot(P1, Row::Units, 1));
        let eaten = put(&mut running, BODY, slot(P1, Row::Units, 2));
        let cry = scripts::scripts_for(&running, CUBE).base.cry.clone();
        with_sink(&mut running, |sink| {
            let mut ctx = resolve::make_context(
                sink,
                Some(&eater),
                HookOptions {
                    controller: Some(P1),
                    targets: Some(vec![Selection::Instance {
                        instance_id: eaten.id.clone(),
                    }]),
                    ..HookOptions::default()
                },
            );
            let list = cry.as_ref().map(|cry| cry(&mut ctx)).unwrap_or_default();
            resolve::apply_effects(&list, &mut ctx);
        });
        assert_eq!(notes(&running), vec![format!("ate:{}", eaten.id)]);
    }
}

// ---------------------------------------------------------------------------
// R124, R125: hero Armor from several sources, and Armor against fatigue.
// ---------------------------------------------------------------------------

mod spec_11_r124_r125_hero_armor_m3_gate {
    use super::*;

    /// One hit on p2's hero, and what it actually dealt.
    fn hit_hero(state: &mut GameState, amount: i32) -> (i32, Vec<GameEvent>) {
        let mut sink = SinkFor::new(state);
        let dealt = sink.run(state, |engine| {
            damage::deal_damage(
                engine,
                DamageArgs {
                    source: None,
                    target: DamageTarget::Hero { player: P2 },
                    amount,
                    flags: None,
                },
            )
        });
        (dealt, sink.events)
    }

    #[test]
    fn r124_adds_hero_armor_up_across_its_sources_where_the_anti_oneshot_cap_instead_takes_the_smallest() {
        let mut state = game("r124-armor-adds");

        // Bare: step 2 has nothing to subtract, so the whole hit lands.
        assert_eq!(state.players.p2.hero.armor, 0);
        assert_eq!(hit_hero(&mut state, 6).0, 6);

        // One Going Long paid 2.
        state.players.p2.hero.armor = 2;
        assert_eq!(hit_hero(&mut state, 6).0, 4);

        // A second granting card contributes like any other layer: two Going Longs paid 2 give Armor 4,
        // not 2 — so the reduction is the sum and never the largest single source.
        state.players.p2.hero.armor = 4;
        assert_eq!(hit_hero(&mut state, 6).0, 2);
        // Plus any Armor written on the hero itself, on top of both.
        state.players.p2.hero.armor = 4 + 1;
        assert_eq!(hit_hero(&mut state, 6).0, 1);
        // And enough of it zeroes the hit, which R63 makes no damage instance at all.
        state.players.p2.hero.armor = 6;
        let stopped = hit_hero(&mut state, 6);
        assert_eq!(stopped.0, 0);
        assert!(stopped.1.is_empty());

        // The opposite case, in the same pipeline: step 3's Anti-oneshot cap is a *ceiling*, so two of
        // those cards give the smallest cap either provides, never their sum.
        let mut capped = game("r124-cap-mins");
        capped.players.p2.hero.armor = 0;
        put(&mut capped, GUARD, slot(P2, Row::Backrow, 1)); // base: cap 5
        assert_eq!(damage::hero_damage_cap(&capped, P2), Some(ANTI_ONESHOT_CAP.base));
        put_radiant(&mut capped, GUARD, slot(P2, Row::Backrow, 2)); // radiant: cap 3
        assert_eq!(
            damage::hero_damage_cap(&capped, P2),
            Some(ANTI_ONESHOT_CAP.radiant)
        );
        const { assert!(ANTI_ONESHOT_CAP.radiant < ANTI_ONESHOT_CAP.base) };
        // Two cards, and a 10 is clamped to the smaller of the two — not to 8, and not to 5.
        assert_eq!(hit_hero(&mut capped, 10).0, ANTI_ONESHOT_CAP.radiant);
    }

    #[test]
    fn r125_sends_fatigue_through_the_whole_damage_pipeline_so_armor_absorbs_the_early_draws() {
        // The control first: with no Armor, the Nth empty draw takes the full N (§2.4, R3). Without this
        // the assertions below could pass on a fatigue that never fired.
        let mut bare = game("r125-no-armor");
        bare.players.p1.library = vec![];
        let mut bare_sink = SinkFor::new(&bare);
        for n in [1, 2, 3, 4] {
            let before = bare.players.p1.hero.health;
            assert!(matches!(
                bare_sink.run(&mut bare, |sink| draw::draw_one(sink, P1, None)),
                DrawOutcome::Fatigue
            ));
            assert_eq!(before - bare.players.p1.hero.health, n);
        }
        assert_eq!(bare.players.p1.fatigue_count, 4);

        // Now behind Going Long's Armor 3. Fatigue is an ordinary damage instance on its own hero, so
        // step 2 applies: draws 1 to 3 are absorbed entirely, and R63's zero rule makes each no damage
        // instance. Each is still a draw that happened, so it is reported by a hit of 0 (R240).
        let mut armoured = game("r125-armour");
        armoured.players.p1.library = vec![];
        armoured.players.p1.hero.armor = 3;
        let mut sink = SinkFor::new(&armoured);
        let full = HERO_HEALTH;

        for n in [1, 2, 3] {
            assert!(matches!(
                sink.run(&mut armoured, |engine| draw::draw_one(engine, P1, None)),
                DrawOutcome::Fatigue
            ));
            assert_eq!(armoured.players.p1.fatigue_count, n);
            assert_eq!(armoured.players.p1.hero.health, full);
        }
        assert_eq!(
            hits(&sink.events),
            [0, 0, 0]
                .iter()
                .map(|amount| Hit::new("", "hero-p1", *amount))
                .collect::<Vec<_>>()
        );

        // The escalating Nth-draw damage is what eventually beats the Armor: the 4th draw is 4, so 1
        // gets through, and the 5th lets 2 through.
        assert!(matches!(
            sink.run(&mut armoured, |engine| draw::draw_one(engine, P1, None)),
            DrawOutcome::Fatigue
        ));
        assert_eq!(armoured.players.p1.hero.health, full - 1);
        assert!(matches!(
            sink.run(&mut armoured, |engine| draw::draw_one(engine, P1, None)),
            DrawOutcome::Fatigue
        ));
        assert_eq!(armoured.players.p1.hero.health, full - 1 - 2);
        assert_eq!(
            hits(&sink.events),
            [0, 0, 0, 1, 2]
                .iter()
                .map(|amount| Hit::new("", "hero-p1", *amount))
                .collect::<Vec<_>>()
        );

        // Step 3's cap rides along too, since it is the same pipeline: a fatigue above the cap is
        // clamped to it.
        let mut capped = game("r125-capped");
        capped.players.p1.library = vec![];
        put_radiant(&mut capped, GUARD, slot(P1, Row::Backrow, 1)); // cap 3
        let mut capped_sink = SinkFor::new(&capped);
        capped.players.p1.fatigue_count = 8; // the next empty draw is the 9th
        let before_cap = capped.players.p1.hero.health;
        assert!(matches!(
            capped_sink.run(&mut capped, |engine| draw::draw_one(engine, P1, None)),
            DrawOutcome::Fatigue
        ));
        assert_eq!(FATIGUE_DAMAGE(9), 9);
        assert_eq!(
            before_cap - capped.players.p1.hero.health,
            ANTI_ONESHOT_CAP.radiant
        );

        // Only "lose health" escapes the pipeline (R18), which is what makes the contrast above a rule
        // about fatigue rather than about heroes.
        let mut losing = game("r125-lose-health");
        losing.players.p1.hero.armor = 3;
        let mut losing_sink = SinkFor::new(&losing);
        assert_eq!(
            losing_sink.run(&mut losing, |engine| damage::lose_health(engine, P1, 2)),
            2
        );
        assert_eq!(losing.players.p1.hero.health, HERO_HEALTH - 2);
        assert_eq!(count_of(&losing_sink.events, GameEventType::Damage), 0);
    }
}

// ---------------------------------------------------------------------------
// R126, R127: how a delayed continuation is re-entered (§10.6, R113).
// ---------------------------------------------------------------------------

mod spec_11_r126_r127_delayed_continuations_m3_gate {
    use super::*;

    /// A game with the note log in place and a delayed effect due at p1's next turn start.
    fn scheduled(seed: &str, resume: Resume) -> (GameState, SinkFor) {
        let mut state = game(seed);
        put(&mut state, LOG_CARD, slot(P1, Row::Backrow, NOTE_LANE));
        let mut sink = SinkFor::new(&state);
        sink.run(&mut state, |engine| {
            modifiers::schedule_delayed(
                engine,
                P1,
                DelayedAt {
                    phase: Phase::Start,
                    player: P1,
                },
                resume,
                None,
                None,
            );
        });
        (state, sink)
    }

    #[test]
    fn r126_re_enters_a_delayed_continuation_under_the_delayed_hook() {
        let card = Resume {
            def_id: DELAYED_HOOK_CARD.to_string(),
            hook: effects::delay::DELAYED_HOOK.to_string(),
            step: String::new(),
            radiant: false,
            instance_id: None,
            data: IndexMap::new(),
        };
        let (mut state, mut sink) = scheduled("r126-hook", card.clone());
        let holder = put(&mut state, DELAYED_HOOK_CARD, slot(P1, Row::Units, 1));
        let delay = must(state.delayed.first().cloned(), "the delay");
        state.delayed[0] = DelayedEffect {
            resume: Resume {
                instance_id: Some(holder.id.clone()),
                ..card.clone()
            },
            ..delay
        };

        sink.run(&mut state, |engine| turn::start_turn(engine, P1));
        assert_eq!(notes(&state), vec!["delayed:hook"]);
        assert!(state.delayed.is_empty());
    }

    #[test]
    fn r126_re_enters_a_delayed_continuation_that_lives_in_the_cards_resume_step_table() {
        // Same continuation, spelled the other way §10.6 allows: `resume.hook` names the step table and
        // `resume.step` picks the entry. One reader must resolve both shapes, so this must run too — and
        // a card must never have to register its continuation under two keys to be re-entered.
        let card = Resume {
            def_id: DELAYED_STEP_CARD.to_string(),
            hook: RESUME_HOOK.to_string(),
            step: "later".to_string(),
            radiant: false,
            instance_id: None,
            data: IndexMap::new(),
        };
        let (mut state, mut sink) = scheduled("r126-step", card.clone());
        let holder = put(&mut state, DELAYED_STEP_CARD, slot(P1, Row::Units, 1));
        let delay = must(state.delayed.first().cloned(), "the delay");
        state.delayed[0] = DelayedEffect {
            resume: Resume {
                instance_id: Some(holder.id.clone()),
                ..card.clone()
            },
            ..delay
        };

        // The step table really is where the continuation lives, so this is not a missing fixture.
        assert!(
            scripts::scripts_for(&state, DELAYED_STEP_CARD)
                .base
                .resume
                .contains_key("later")
        );

        // `expect(() => startTurn(sink, "p1")).not.toThrow()`: a panic fails the test.
        sink.run(&mut state, |engine| turn::start_turn(engine, P1));
        assert_eq!(notes(&state), vec!["delayed:step"]);
    }

    #[test]
    fn r127_resolves_a_delayed_continuation_whose_instance_is_gone_with_ctx_self_null_and_its_data() {
        // #50 K-Pop Fanatic's ordinary case (R76): the card that scheduled the delay has left play. The
        // continuation names its script by stored def id, so it still re-enters — dropping it would
        // silently lose a sequence, which R113 forbids.
        let mut data = IndexMap::new();
        data.insert("carried".to_string(), json!("payload"));
        let resume = Resume {
            def_id: GHOST_CARD.to_string(),
            hook: RESUME_HOOK.to_string(),
            step: "orphan".to_string(),
            radiant: false,
            instance_id: None,
            data,
        };
        let (mut state, mut sink) = scheduled("r127-no-instance", resume.clone());
        assert_eq!(state.delayed.len(), 1);
        assert!(state.delayed[0].resume.instance_id.is_none());

        // `expect(() => startTurn(sink, "p1")).not.toThrow()`: a panic fails the test.
        sink.run(&mut state, |engine| turn::start_turn(engine, P1));
        assert_eq!(notes(&state), vec!["orphan:no-self:payload"]);

        // The same continuation is resolved this way on the work path, which is the shape R127 asks the
        // delayed path to match (§9.3's resumable-work queue).
        let mut other = game("r127-work-path");
        put(&mut other, LOG_CARD, slot(P1, Row::Backrow, NOTE_LANE));
        let mut work_sink = SinkFor::new(&other);
        assert!(work_sink.run(&mut other, |engine| {
            prompts::run_resume(
                engine,
                &resume,
                ResumeOptions {
                    controller: Some(P1),
                    ..ResumeOptions::default()
                },
            )
        }));
        assert_eq!(notes(&other), vec!["orphan:no-self:payload"]);
    }

    #[test]
    fn r127_resolves_a_delayed_continuation_whose_instance_has_since_ceased_to_exist() {
        let mut state = game("r127-instance-gone");
        put(&mut state, LOG_CARD, slot(P1, Row::Backrow, NOTE_LANE));
        let mut sink = SinkFor::new(&state);
        let mut scheduler = put(&mut state, GHOST_CARD, slot(P1, Row::Backrow, 1));
        let mut data = IndexMap::new();
        data.insert("carried".to_string(), json!("payload"));
        let resume = Resume {
            def_id: GHOST_CARD.to_string(),
            hook: RESUME_HOOK.to_string(),
            step: "orphan".to_string(),
            radiant: false,
            instance_id: Some(scheduler.id.clone()),
            data,
        };
        sink.run(&mut state, |engine| {
            modifiers::schedule_delayed(
                engine,
                P1,
                DelayedAt {
                    phase: Phase::Start,
                    player: P1,
                },
                resume,
                None,
                None,
            );
        });
        // It ceases to exist before the delay comes due, which is the premise of the row.
        zones::remove_from_any_zone(&mut state, &mut scheduler);
        scheduler.zone = Zone::Gone { player: P1 };
        assert!(find_instance(&state, &scheduler.id).is_none());

        // `expect(() => startTurn(sink, "p1")).not.toThrow()`: a panic fails the test.
        sink.run(&mut state, |engine| turn::start_turn(engine, P1));
        assert_eq!(notes(&state), vec!["orphan:no-self:payload"]);
        assert!(state.delayed.is_empty());
    }
}

// ---------------------------------------------------------------------------
// R128, R129, R130: sweeps, fizzles and Lucky (§4.4, §10.7).
// ---------------------------------------------------------------------------

mod spec_11_r128_r130_sweeps_fizzles_and_lucky_m3_gate {
    use super::*;

    fn targets_hit(events: &[GameEvent]) -> Vec<String> {
        hits(events).into_iter().map(|landed| landed.to).collect()
    }

    #[test]
    fn r128_resolves_a_two_sided_sweep_as_units_in_r68_order_then_one_instance_per_scoped_hero_in_that_order()
    {
        let mut state = game("r128-sweep");
        let mine = vec![
            put(&mut state, BODY, slot(P1, Row::Units, 1)),
            put(&mut state, BODY, slot(P1, Row::Units, 3)),
        ];
        let theirs = vec![
            put(&mut state, BODY, slot(P2, Row::Units, 2)),
            put(&mut state, BODY, slot(P2, Row::Units, 4)),
        ];

        let mut sink = SinkFor::new(&state);
        apply_as(
            &mut sink,
            &mut state,
            P1,
            vec![effects::damage_all(json_as(
                json!({ "amount": 2, "heroes": true, "side": "any" }),
            ))],
        );

        // p1 is active: its units in lane order, then p2's, then the two heroes in the same side order.
        let mut expected = ids(&mine);
        expected.extend(ids(&theirs));
        expected.extend(["hero-p1".to_string(), "hero-p2".to_string()]);
        assert_eq!(targets_hit(&sink.events), expected);

        // Not a hardcoded p1-first: with p2 active the whole order reverses (R68).
        let mut flipped = game("r128-sweep-flipped");
        flipped.active = P2;
        let flipped_mine = vec![put(&mut flipped, BODY, slot(P1, Row::Units, 1))];
        let flipped_theirs = vec![put(&mut flipped, BODY, slot(P2, Row::Units, 2))];
        let mut flipped_sink = SinkFor::new(&flipped);
        apply_as(
            &mut flipped_sink,
            &mut flipped,
            P1,
            vec![effects::damage_all(json_as(
                json!({ "amount": 2, "heroes": true, "side": "any" }),
            ))],
        );
        let mut expected = ids(&flipped_theirs);
        expected.extend(ids(&flipped_mine));
        expected.extend(["hero-p2".to_string(), "hero-p1".to_string()]);
        assert_eq!(targets_hit(&flipped_sink.events), expected);

        // The target list is fixed when the effect begins: a unit the sweep has already taken to 0 or
        // less health still gets its own instance, and the units after it are still reached, because the
        // state check waits for the whole effect (R59).
        let mut lethal = game("r128-sweep-lethal");
        let doomed = put(&mut lethal, FRAIL, slot(P1, Row::Units, 1)); // 1/3
        let after = put(&mut lethal, BODY, slot(P1, Row::Units, 2));
        let mut lethal_sink = SinkFor::new(&lethal);
        apply_as(
            &mut lethal_sink,
            &mut lethal,
            P1,
            vec![effects::damage_all(json_as(
                json!({ "amount": 5, "side": "self" }),
            ))],
        );
        assert_eq!(
            targets_hit(&lethal_sink.events),
            vec![doomed.id.clone(), after.id.clone()]
        );
        assert!(layers::unit_view(&lethal, &live(&lethal, &doomed)).health <= 0);
        assert_eq!(
            zones::card_at(&lethal, slot(P1, Row::Units, 1)).map(|card| card.id.clone()),
            Some(doomed.id.clone())
        ); // still there: no state check yet
    }

    #[test]
    fn r129_has_a_fizzling_effect_draw_no_randomness_which_is_why_a_whole_hand_discard_is_its_own_verb() {
        // The hazard, made concrete: a random discard repeated by hand size moves the cursor once per
        // card, so `rngCursor` would depend on the board at that moment (§10.7).
        let mut random = game("r129-random-discard");
        in_hand(&mut random, BODY, P1, 5);
        let mut random_sink = SinkFor::new(&random);
        let started_at = random_sink.rng.cursor();
        apply_as(
            &mut random_sink,
            &mut random,
            P1,
            vec![effects::discard_random(json_as(json!({ "count": 5 })))],
        );
        assert!(random.players.p1.hand.is_empty());
        assert_eq!(random_sink.rng.cursor() - started_at, 5);

        // R129: the whole-hand verb reaches the same end state and draws nothing at all.
        let mut whole = game("r129-discard-hand");
        in_hand(&mut whole, BODY, P1, 5);
        let mut whole_sink = SinkFor::new(&whole);
        let before = whole_sink.rng.cursor();
        apply_as(
            &mut whole_sink,
            &mut whole,
            P1,
            vec![effects::discard_hand(json_as(json!({})))],
        );
        assert!(whole.players.p1.hand.is_empty());
        assert_eq!(whole.players.p1.graveyard.len(), 5);
        assert_eq!(whole_sink.rng.cursor() - before, 0);

        // And an effect that finds nothing to do takes no draw either, so the cursor does not depend on
        // whether the hand happened to be empty.
        let mut empty = game("r129-fizzle");
        let mut empty_sink = SinkFor::new(&empty);
        let empty_before = empty_sink.rng.cursor();
        apply_as(
            &mut empty_sink,
            &mut empty,
            P1,
            vec![effects::discard_random(json_as(json!({ "count": 3 })))],
        );
        assert_eq!(empty_sink.rng.cursor() - empty_before, 0);
    }

    #[test]
    fn r130_leaves_a_roll_with_no_better_outcome_alone_so_lucky_costs_it_no_draw() {
        // The hazard, made concrete: applying Lucky where "better" cannot distinguish the outcomes still
        // burns the extra draws, which moves `rngCursor` and desynchronises a replay (R129).
        let mut one = Rng::new("r130", 0);
        one.coin();
        let mut lucky_rng = Rng::new("r130", 0);
        lucky_rng.lucky(1, |rng| rng.coin(), |a, _| a); // an identity comparator: no better outcome
        assert_eq!(lucky_rng.cursor(), one.cursor() + 1);

        // R130 and R32: a coin-stat effect pays out on both faces, so it has no better outcome and Lucky
        // does nothing to it — the effect takes exactly one draw per coin, and the cursor lands where a
        // game with no Lucky anywhere would leave it.
        let mut state = game("r130-coins");
        let target = put(&mut state, BODY, slot(P1, Row::Units, 1));
        let mut sink = SinkFor::new(&state);
        let before = sink.rng.cursor();
        apply_as(
            &mut sink,
            &mut state,
            P1,
            vec![effects::flip_coins(json_as(json!({
                "target": { "of": "instance", "instanceId": target.id },
                "coins": 5,
                "perHeads": { "attack": 1 },
                "perTails": { "health": 1 },
            })))],
        );
        assert_eq!(sink.rng.cursor() - before, 5);
        // The two gains total the coins, so every flip was counted once and none was re-rolled.
        let buffs = live(&state, &target).buffs;
        assert_eq!(buffs.attack + buffs.health, 5);
    }
}

// ---------------------------------------------------------------------------
// R131 to R136: the set-stat layer, play pools, grades, Genn's Greed and the event window.
// ---------------------------------------------------------------------------

mod spec_11_r131_r136_layer_2_pools_grades_and_event_windows_m3_gate {
    use super::*;

    fn view(state: &GameState, card: &CardInstance) -> UnitView {
        layers::unit_view(state, &live(state, card))
    }

    fn buffed_attack(state: &GameState, card: &CardInstance) -> i32 {
        layers::stats_with_buffs(state, &live(state, card)).attack
    }

    #[test]
    fn r131_never_counts_felinor_fiender_itself_even_when_it_carries_the_felinor_tag() {
        let mut state = game("r131-self");
        let me = put(&mut state, FIENDER, slot(P1, Row::Units, 1)); // 5/7, tagged Felinor

        // The premise: its own def really does carry the tag, so a tag-only match would count it.
        assert!(
            catalog::def_of(Some(&state), &me.def_id)
                .tags
                .contains(&Tag::Felinor)
        );
        // Alone it is exactly its printed face: "all your Felinors" is every OTHER one (R39's floor).
        assert_eq!(view(&state, &me).attack, 5);
        assert_eq!(view(&state, &me).max_health, 7);

        // A second Fiender is a Felinor to the first, and contributes only its printed and buffed stats
        // — never its own layer-2 total, so the layer cannot recurse (R116).
        let second = put(&mut state, FIENDER, slot(P1, Row::Units, 2)); // 5/7, also tagged Felinor
        assert_eq!(buffed_attack(&state, &second), 5);
        assert_eq!(view(&state, &me).attack, 5 + 5);
        assert_eq!(view(&state, &second).attack, 5 + 5);
        // If either read the other's layer-2 total the pair would escalate past 10.
        assert_ne!(view(&state, &me).attack, 5 + 10);

        // An enemy Felinor is nobody's: "your Felinors" is the controller's side.
        put(&mut state, FELINOR, slot(P2, Row::Units, 1));
        assert_eq!(view(&state, &me).attack, 5 + 5);
    }

    #[test]
    fn r132_applies_r39s_floor_to_each_stats_combined_total_not_per_felinor_and_not_across_the_two_stats() {
        let mut state = game("r132-floor");
        let me = put(&mut state, FIENDER, slot(P1, Row::Units, 1)); // 5/7
        let plus = put(&mut state, FELINOR, slot(P1, Row::Units, 2)); // 3/10
        let minus = put(&mut state, FELINOR, slot(P1, Row::Units, 3)); // 3/10
        live_mut(&mut state, &plus).buffs = AttackHealth { attack: 4, health: 0 };
        live_mut(&mut state, &minus).buffs = AttackHealth {
            attack: -5,
            health: 0,
        };

        // The control: the helper really does read permanent buffs, so a positive one lands in full.
        // Without this the negative case below could fail on buffs that were never applied.
        assert_eq!(buffed_attack(&state, &plus), 3 + 4);

        // R132 and R116: each Felinor is measured at layers 1 to 4 — printed plus permanent buffs — and
        // the floor belongs to the combined total, so a Felinor carrying a negative buff contributes a
        // negative attack and pulls the sum toward 0.
        assert_eq!(buffed_attack(&state, &minus), 3 - 5);

        // The sum is 7 + (-2) = 5, so the total is 5 + 5 = 10. A floor applied per Felinor instead would
        // read 7 + 0 = 7 and give 12.
        assert_eq!(view(&state, &me).attack, 10);

        // Each stat is floored on its own: the health sum is untouched by whatever the attack sum did.
        assert_eq!(view(&state, &me).max_health, 7 + 10 + 10);

        // And neither sum goes below 0: enough negative attack floors the attack contribution at 0
        // without dragging the health contribution down with it.
        live_mut(&mut state, &minus).buffs = AttackHealth {
            attack: -99,
            health: 0,
        };
        assert_eq!(view(&state, &me).attack, 5);
        assert_eq!(view(&state, &me).max_health, 7 + 10 + 10);
    }

    #[test]
    fn r133_weights_a_card_played_twice_in_one_turn_once_because_the_pool_is_the_set_of_cards_played() {
        let mut state = game("r133-pool");
        let card = put(&mut state, BODY, slot(P1, Row::Units, 1));
        let mut other = put(&mut state, FRAIL, slot(P1, Row::Units, 2));

        // The premise the row states: `playedIds` records one entry per play, so a card played, bounced
        // and replayed appears twice in the log.
        state.players.p1.turn_log.played_ids = vec![card.id.clone(), other.id.clone(), card.id.clone()];
        state.players.p1.turn_log.cards_played = 3;
        assert_eq!(
            query::played_ids_this_turn(&state, P1).to_vec(),
            vec![card.id.clone(), other.id.clone(), card.id.clone()]
        );

        // R133: the pool a random pick draws from is the SET of cards played, so the replayed card is
        // one candidate and not two.
        let pool = played_cards_this_turn(&state, P1);
        assert_eq!(
            pool.iter().map(|entry| entry.id.clone()).collect::<Vec<_>>(),
            vec![card.id.clone(), other.id.clone()]
        );

        // R86's half of the same line still holds: an id whose card has ceased to exist drops out.
        zones::remove_from_any_zone(&mut state, &mut other);
        other.zone = Zone::Gone { player: P1 };
        assert_eq!(
            played_cards_this_turn(&state, P1)
                .iter()
                .map(|entry| entry.id.clone())
                .collect::<Vec<_>>(),
            vec![card.id.clone()]
        );
    }

    #[test]
    fn r134_keeps_a_grade_counter_on_the_instance_through_a_change_of_control_and_reads_the_new_controllers_turn_log()
     {
        let mut state = game("r134-grade");
        let index = put(&mut state, LOG_CARD, slot(P1, Row::Backrow, 1));
        live_mut(&mut state, &index).counters.grade = Some(2); // grade D: it needs 2 plays to rise

        assert_eq!(grade_of(&live(&state, &index)), 2);
        // p1 has played twice, p2 not at all, so it would rise for p1 and not for p2.
        state.players.p1.turn_log.cards_played = 2;
        state.players.p2.turn_log.cards_played = 0;
        assert!(grade_rises(&state, &live(&state, &index)));

        // Control changes. The counter is the card's, so it travels with the instance.
        let mut sink = SinkFor::new(&state);
        apply_as(
            &mut sink,
            &mut state,
            P2,
            vec![effects::steal(json_as(json!({ "instanceId": index.id })))],
        );
        assert_eq!(live(&state, &index).controller, P2);
        assert_eq!(grade_of(&live(&state, &index)), 2);

        // The threshold is the controller's: it now reads p2's turn log, which is short of the grade.
        assert!(!grade_rises(&state, &live(&state, &index)));
        state.players.p2.turn_log.cards_played = 2;
        assert!(grade_rises(&state, &live(&state, &index)));
        // And p1's log no longer decides it, which is what "the counter is the card's, the threshold is
        // the controller's" means.
        state.players.p1.turn_log.cards_played = 0;
        assert!(grade_rises(&state, &live(&state, &index)));
    }

    #[test]
    fn r135_exiles_each_card_on_its_own_and_needs_a_verb_that_walks_library_then_hand_then_graveyard() {
        // The per-card half, on a verb that ships: R55's counter moves once per card and anything
        // watching sees one `exiled` event per card rather than a batch.
        let mut state = game("r135-per-card");
        let hand = in_hand(&mut state, BODY, P1, 3);
        let mut sink = SinkFor::new(&state);
        let before = state.counters.exiled;
        apply_as(
            &mut sink,
            &mut state,
            P1,
            vec![effects::exile_hand(json_as(json!({})))],
        );

        assert!(state.players.p1.hand.is_empty());
        assert_eq!(state.counters.exiled - before, 3);
        let exiled: Vec<String> = sink
            .events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Exiled { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(exiled, ids(&hand));

        // The ordering half needs a verb that can walk the three zones by cost in §8's order. #94's own
        // clauses are blocked on it, so the order has no home yet: library, then hand, then graveyard,
        // with the draw clause running first so a drawn card is never exiled by the same play.
        // (TS: `Object.keys(effects)` contains both verbs; in Rust, naming them is the check.)
        let _exile_matching = effects::exile_matching;
        let _draw_from_library = effects::draw_from_library;
    }

    #[test]
    fn r136_gives_a_script_its_own_event_window_so_an_earlier_event_in_the_same_action_is_not_its_own() {
        // The control: with no `summoned` event on the list at all, the filter compels nobody — so the
        // failure below is caused by the event, not by the filter being ignored.
        let mut quiet = game("r136-control");
        put(&mut quiet, BODY, slot(P1, Row::Units, 1));
        let quiet_victim = put(&mut quiet, BODY, slot(P2, Row::Units, 1));
        let mut quiet_sink = SinkFor::new(&quiet);
        apply_as(
            &mut quiet_sink,
            &mut quiet,
            P1,
            vec![effects::forced_attacks(json_as(json!({
                "attackers": { "side": "self", "summonedThisScript": true },
                "target": { "instanceId": quiet_victim.id },
            })))],
        );
        assert!(attackers_declared(&quiet_sink.events).is_empty());

        let mut state = game("r136-window");
        let earlier = put(&mut state, BODY, slot(P1, Row::Units, 1));
        let victim = put(&mut state, BODY, slot(P2, Row::Units, 1));
        let mut sink = SinkFor::new(&state);

        // Something earlier in the same action summoned a unit, so its `summoned` event is already on
        // the sink's list — a second copy of a card, or a trap firing mid-action, does exactly this.
        sink.events.push(GameEvent::Summoned {
            player: P1,
            instance_id: earlier.id.clone(),
            def_id: earlier.def_id.clone(),
            row: Row::Units,
            lane: 1,
            former_id: None,
            arrived_during: None,
            exits_from: None,
        });

        // This script summons nothing, so "the ones I just made" is empty and nobody is compelled.
        apply_as(
            &mut sink,
            &mut state,
            P1,
            vec![effects::forced_attacks(json_as(json!({
                "attackers": { "side": "self", "summonedThisScript": true },
                "target": { "instanceId": victim.id },
            })))],
        );

        assert!(attackers_declared(&sink.events).is_empty());
        assert_eq!(live(&state, &victim).damage, 0);
    }
}

// ---------------------------------------------------------------------------
// R150, R154: where a floor lives in a summing read, and what `trapFired` carries.
// ---------------------------------------------------------------------------

mod spec_11_r150_and_r154_summing_reads_and_the_trapfired_payload_m3_gate {
    use super::*;

    /// The redacted identity of the view's `trapFired` event: (instanceId, defId).
    fn seen_by(view: &PlayerView) -> (String, String) {
        view.events
            .iter()
            .find_map(|entry| match entry {
                GameEvent::TrapFired {
                    instance_id, def_id, ..
                } => Some((instance_id.clone(), def_id.clone())),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no trapFired in view"))
    }

    #[test]
    fn r150_keeps_a_stat_floor_off_each_contributor_of_a_summing_read_leaving_it_on_the_combined_total() {
        let mut state = game("r150-contributor-floor");
        let summer = put(&mut state, FIENDER, slot(P1, Row::Units, 1)); // 5/7, sums the others
        let donor = put(&mut state, FELINOR, slot(P1, Row::Units, 2)); // printed 3/10
        let view = |state: &GameState, card: &CardInstance| layers::unit_view(state, &live(state, card));

        // The control: the layer-4 reading really does see permanent buffs, so the negative case below
        // is about the floor and not about a buff that never applied.
        live_mut(&mut state, &donor).buffs = AttackHealth { attack: 4, health: 0 };
        assert_eq!(
            layers::stats_with_buffs(&state, &live(&state, &donor)).attack,
            3 + 4
        );
        assert_eq!(view(&state, &summer).attack, 5 + 7);

        // R150: the layer-4 reading has no per-unit floor, so a contributor with a negative buff pulls
        // the total down rather than contributing nothing.
        live_mut(&mut state, &donor).buffs = AttackHealth {
            attack: -5,
            health: 0,
        };
        assert_eq!(
            layers::stats_with_buffs(&state, &live(&state, &donor)).attack,
            3 - 5
        );

        // The floor belongs where the value is finally used — on the combined total (R132), per stat.
        assert_eq!(view(&state, &summer).attack, 5);
        assert_eq!(view(&state, &summer).max_health, 7 + 10);

        // And on a card's own stats, at the point of display.
        live_mut(&mut state, &donor).buffs = AttackHealth {
            attack: -99,
            health: 0,
        };
        assert_eq!(view(&state, &donor).attack, 0);
        assert_eq!(view(&state, &summer).attack, 5);
    }

    #[test]
    fn r154_carries_the_traps_row_and_lane_on_trapfired_with_its_identity_redacted_for_the_other_player() {
        let mut state = game("r154-trap-lane");
        let trap = put(&mut state, BARE_TRAP, slot(P1, Row::Backrow, 3));
        let mut sink = SinkFor::new(&state);
        assert_eq!(
            sink.run(&mut state, |engine| traps::fire_traps_for(engine, &played(P2)))
                .fired,
            vec![trap.id.clone()]
        );

        let fired = must(
            sink.events
                .iter()
                .find(|event| matches!(event, GameEvent::TrapFired { .. }))
                .cloned(),
            "a trapFired event",
        );
        match &fired {
            GameEvent::TrapFired { controller, .. } => assert_eq!(*controller, P1),
            other => panic!("not a trapFired event: {other:?}"),
        }

        // R154: the zone that flipped, so a client can animate the lane without being told which card
        // it was — which is the whole point, since a face-down trap is given no instance id (R97).
        let keys = as_json(&fired);
        assert!(keys.as_object().is_some_and(|object| object.contains_key("row")));
        assert!(keys.as_object().is_some_and(|object| object.contains_key("lane")));

        // `viewFor` reads its event stream off `state.applied`, which only a completed `reduce` writes
        // (`rememberNonce`). This fixture drives the sink directly, so the action has to be recorded
        // the way `reduce` would before any view can see it.
        state.applied = vec![AppliedAction {
            nonce: "r154".to_string(),
            events: sink.events.clone(),
        }];

        // Its identity follows §10.8's redaction: the controller reads it, the other player reads the
        // sentinel — keyed to the controller and NOT to the card's current zone, because firing the
        // trap moves it to a public graveyard (R97's exception, stated in R154).
        let owner = view_for::view_for(&state, P1);
        let other = view_for::view_for(&state, P2);
        assert_eq!(seen_by(&owner), (trap.id.clone(), trap.def_id.clone()));
        assert_eq!(
            seen_by(&other),
            (view_for::HIDDEN_ID.to_string(), view_for::HIDDEN_ID.to_string())
        );
    }

    #[test]
    fn r763_hides_a_fired_trap_from_its_controller_once_it_is_shuffled_into_a_library() {
        let mut state = game("r763-trap-to-library");
        let trap = put(&mut state, BARE_TRAP, slot(P1, Row::Backrow, 3));
        let mut sink = SinkFor::new(&state);
        assert_eq!(
            sink.run(&mut state, |engine| traps::fire_traps_for(engine, &played(P2)))
                .fired,
            vec![trap.id.clone()]
        );
        state.applied = vec![AppliedAction {
            nonce: "r763".to_string(),
            events: sink.events.clone(),
        }];

        // The control: the fired trap sits in a public pile, where R154 lets its controller read it.
        assert_eq!(
            seen_by(&view_for::view_for(&state, P1)),
            (trap.id.clone(), trap.def_id.clone())
        );

        // C #17 Counterspell Trap shuffled back into its owner's deck: §10.8 and R310 never send the id
        // of a library card, so the event that announced the flip reads as the sentinel to both seats.
        let mut fired_trap = must(find_instance(&state, &trap.id).cloned(), "the fired trap");
        zones::move_to_zone(
            &mut state,
            &mut fired_trap,
            OffFieldZone::Library,
            MoveToZoneOptions::default(),
        );
        let hidden = (view_for::HIDDEN_ID.to_string(), view_for::HIDDEN_ID.to_string());
        assert_eq!(seen_by(&view_for::view_for(&state, P1)), hidden);
        assert_eq!(seen_by(&view_for::view_for(&state, P2)), hidden);
    }
}

// ---------------------------------------------------------------------------
// R138, R139, R140, R152: casts with no zone, lapsed limits, Stack placement, AI turns.
// ---------------------------------------------------------------------------

mod spec_11_r138_r140_and_r152_plays_limits_and_lockouts_m3_gate {
    use super::*;

    #[test]
    fn r138_counts_a_cast_permanent_with_no_zone_as_played_resolves_it_and_sends_it_to_the_graveyard() {
        // The control first: with a free zone the same cast lands on the field, so the fixture really
        // does cast a permanent and the row below is about the full row rather than a broken cast.
        let mut roomy = game("r138-control");
        put(&mut roomy, LOG_CARD, slot(P1, Row::Backrow, NOTE_LANE));
        let mut roomy_sink = SinkFor::new(&roomy);
        // Cast it straight out of the hand: `castCard` takes it out of whatever pile holds it.
        let lands = must(
            in_hand(&mut roomy, CRIER, P1, 1).into_iter().next(),
            "a permanent to cast",
        );
        roomy_sink.run(&mut roomy, |sink| {
            resolve::cast_card(sink, &lands, CastOptions::default())
        });
        assert!(unit_ids(&roomy, P1).contains(&lands.id));
        assert_eq!(notes(&roomy), vec!["cry"]);

        // Now the row: every unit zone taken, so there is nowhere for it to go.
        let mut state = game("r138-no-zone");
        put(&mut state, LOG_CARD, slot(P1, Row::Backrow, NOTE_LANE));
        for lane in [1, 2, 3, 4, 5] {
            put(&mut state, BODY, slot(P1, Row::Units, lane));
        }
        assert_eq!(zones::active_units_of(&state, P1).len(), 5);

        let mut sink = SinkFor::new(&state);
        let cast = must(
            in_hand(&mut state, CRIER, P1, 1).into_iter().next(),
            "a permanent to cast",
        );
        let played_before = state.players.p1.turn_log.cards_played;
        sink.run(&mut state, |engine| {
            resolve::cast_card(engine, &cast, CastOptions::default())
        });

        // A cast cannot be refused the way a play can (R70): it still counts as played...
        assert_eq!(state.players.p1.turn_log.cards_played, played_before + 1);
        assert!(state.players.p1.turn_log.played_ids.contains(&cast.id));
        // ...and still resolves its script...
        assert_eq!(notes(&state), vec!["cry"]);
        // ...and is in no zone, so §10.5 step 7 sends it to its owner's graveyard.
        assert!(!unit_ids(&state, P1).contains(&cast.id));
        assert!(ids(&state.players.p1.graveyard).contains(&cast.id));
        assert!(!ids(&state.players.p1.resolving).contains(&cast.id));
    }

    #[test]
    fn r139_lapses_a_once_per_turn_limit_at_the_turn_boundary_so_a_later_turn_is_told_whose_turn_it_is() {
        let mut state = game("r139-lapse");
        let card = put(&mut state, HEROIC, slot(P1, Row::Backrow, 1));
        live_mut(&mut state, &card)
            .memory
            .insert(POWER_KEY.to_string(), json!("burn")); // X 1
        state.players.p1.mana.current = 0; // unaffordable, so the order of the checks is observable

        // Used this turn: the uses are counted against the turn they were made on (R384).
        let used_on = state.turn;
        live_mut(&mut state, &card).memory.insert(
            ACTIVATIONS_MEMORY_KEY.to_string(),
            json!({ "turn": used_on, "count": 1 }),
        );
        assert!(used_this_turn(&state, &live(&state, &card)));
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &card.id, None)),
            Some("that ability has already been used this turn".to_string())
        );

        // R139: the count is stored with the turn it was made on, so it is spent only while the game is
        // still on that turn. The stored value does not move — the turn does.
        state.turn += 1;
        state.active = P2;
        assert_eq!(
            live(&state, &card).memory.get(ACTIVATIONS_MEMORY_KEY),
            Some(&json!({ "turn": used_on, "count": 1 }))
        );
        assert!(!used_this_turn(&state, &live(&state, &card)));

        // So the player is told whose turn it is, not that the ability is spent.
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &card.id, None)),
            Some("it is not your turn".to_string())
        );
        // And on their own turn it is the mana, the check after the uses.
        state.active = P1;
        assert_eq!(
            refusal(why_cannot_activate_ability(&state, P1, &card.id, None)),
            Some("that ability costs 1, more than your mana".to_string())
        );
    }

    #[test]
    fn r140_gives_a_zone_less_stack_play_the_leftmost_empty_zone_and_lifts_occupancy_only_when_named() {
        let mut state = game("r140-stack-zone");
        let card = must(
            in_hand(&mut state, &stacker.id, P1, 1).into_iter().next(),
            "a Stack card in hand",
        );
        let play = |state: &GameState, lane: Option<i32>| -> Option<String> {
            let fields = match lane {
                None => json!({ "instanceId": card.id }),
                Some(lane) => json!({ "instanceId": card.id, "zone": { "row": "units", "lane": lane } }),
            };
            refusal(play_choices::why_choices_refused(
                state,
                P1,
                &card,
                &play_action(fields),
            ))
        };

        // The control: on an empty row the convenience path is fine, and a named zone is too.
        assert_eq!(play(&state, None), None);
        assert_eq!(play(&state, Some(3)), None);

        // Fill every unit zone. A named occupied zone is still legal — that is exactly the refusal
        // Stack lifts (§3.2, R64).
        let occupants: Vec<CardInstance> = [1, 2, 3, 4, 5]
            .into_iter()
            .map(|lane| put(&mut state, BODY, slot(P1, Row::Units, lane)))
            .collect();
        assert_eq!(occupants.len(), 5);
        assert_eq!(play(&state, Some(2)), None);
        assert_eq!(
            play_choices::legal_zones_for(&state, P1, &card, &[])
                .iter()
                .map(|zone| zone.lane)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );

        // R140: the zone-less path still wants an EMPTY zone, so a full row refuses it even though
        // every lane would accept a named one. The asymmetry is confined to the convenience path.
        assert!(must(play(&state, None), "a refusal").contains("no free units zone"));

        // And with one lane freed it takes the leftmost empty one, not the first that would accept a
        // Stack — lane 4 here, with 1, 2, 3 and 5 still occupied.
        let mut freed = must(occupants.get(3).cloned(), "the lane-4 occupant");
        zones::remove_from_any_zone(&mut state, &mut freed);
        freed.zone = Zone::Graveyard { player: P1 };
        assert_eq!(play(&state, None), None);
        // "The leftmost empty, unlocked zone" is lane 4, not lane 1: lane 1 would accept a *named*
        // Stack play, and the convenience path deliberately does not take it.
        assert_eq!(
            zones::first_free_zone(&state, P1, Row::Units),
            Some(slot(P1, Row::Units, 4))
        );
        assert_eq!(
            zones::card_at(&state, slot(P1, Row::Units, 1)).map(|card| card.def_id.clone()),
            Some(BODY.to_string())
        );
    }

    #[test]
    fn r152_clears_the_ai_lockout_at_the_end_of_the_turn_it_was_set_for() {
        // `aiPlaysOutTurn` sets the lockout and then plays the turn out, so the flag is only observable
        // mid-turn; R152 is about where it is *cleared*, which is what this sets up directly.
        let mut state = game("r152-ai-turn");
        let mut sink = SinkFor::new(&state);
        assert!(!state.players.p1.ai_turn);
        state.players.p1.ai_turn = true; // the lockout `aiPlaysOutTurn` sets on the active player
        assert_eq!(state.active, P1);
        let locked_on = state.turn;

        // §8's "until end of turn": the end of THIS turn clears it, not that player's next turn start —
        // otherwise a player stays locked out of a turn that is no longer the one the effect took.
        sink.run(&mut state, turn::end_turn);
        assert!(!state.players.p1.ai_turn);

        // And it was cleared by the end of the turn it was set for, not by p1 reaching another turn:
        // it is p2's turn now, and p1 has not started one since.
        assert_eq!(state.active, P2);
        assert!(state.turn > locked_on);
    }
}

// ---------------------------------------------------------------------------
// R151, R153: when a power rolls, and which hooks a zone answers.
// ---------------------------------------------------------------------------

mod spec_11_r151_and_r153_arrivals_and_zone_gated_hooks_m3_gate {
    use super::*;

    #[test]
    fn r151_rolls_a_heroic_powers_power_as_it_arrives_in_a_hand_not_only_at_the_start_of_the_game() {
        let mut state = game("r151-arrival");
        let mut sink = SinkFor::new(&state);

        // A copy that reached a hand some other way than the opening draw: returned from a graveyard.
        // The premise, and the bug R151 closes — it carries no power, so it would cost 0 for ever.
        let card = new_instance(&mut state, HEROIC, P1, Zone::Graveyard { player: P1 });
        state.players.p1.graveyard.push(card.clone());
        assert!(!card.memory.contains_key(POWER_KEY));
        assert!(power_of(&card).is_none());

        // It arrives somewhere a card can be looked at.
        let mut arriving = card.clone();
        assert!(matches!(
            sink.run(&mut state, |engine| draw::add_to_hand(engine, &mut arriving)),
            AddToHandOutcome::Hand
        ));

        // R151: it rolls on arrival, so it has a power again (R43, R78).
        let rolled = must(
            live(&state, &card).memory.get(POWER_KEY).cloned(),
            "a rolled power",
        );
        assert!(rolled.is_string());
        assert!(
            as_json(&HERO_POWER_NAMES)
                .as_array()
                .is_some_and(|names| names.contains(&rolled))
        );
        assert!(must(power_of(&live(&state, &card)), "the rolled power").x > 0);

        // And the roll is idempotent: a card that already has one keeps it when it arrives again.
        let kept = must(power_of(&live(&state, &card)), "the rolled power");
        let mut again = live(&state, &card);
        state.players.p1.hand.retain(|entry| entry.id != card.id);
        assert!(matches!(
            sink.run(&mut state, |engine| draw::add_to_hand(engine, &mut again)),
            AddToHandOutcome::Hand
        ));
        let now = must(power_of(&live(&state, &card)), "the kept power");
        assert_eq!(as_json(&now.name), as_json(&kept.name));
        assert_eq!(now.x, kept.x);
    }

    #[test]
    fn r153_registers_only_the_hooks_a_cards_zone_allows_so_a_hand_or_a_graveyard_answers_no_start_or_end_of_turn_hook()
     {
        let mut state = game("r153-zone-hooks");

        // The control: on the field the card IS a holder for all three hooks, so the enumerator is live
        // and the negatives below are about the zone rather than about a hook that never registered.
        let on_field = put(&mut state, ZONE_HOOKS, slot(P1, Row::Backrow, 1));
        let holders = |state: &GameState, hook: HookName| -> Vec<String> {
            triggers::trigger_holders_with_hook(state, hook, None)
                .iter()
                .map(|holder| holder.card.id.clone())
                .collect()
        };
        assert_eq!(holders(&state, HookName::StartOfTurn), vec![on_field.id.clone()]);
        assert_eq!(holders(&state, HookName::EndOfTurn), vec![on_field.id.clone()]);
        assert_eq!(holders(&state, HookName::OnPlayHook), vec![on_field.id.clone()]);

        // In a hand a card answers only its `handTriggers` (#89 Corpse Eater). A Field Spell held in
        // hand must not summon its token every turn, and a Gifted Program in hand must not make a play
        // Radiant — which is what an unfiltered enumeration lets both of them do.
        let held = must(
            in_hand(&mut state, ZONE_HOOKS, P1, 1).into_iter().next(),
            "a copy in hand",
        );
        assert!(!holders(&state, HookName::StartOfTurn).contains(&held.id));
        assert!(!holders(&state, HookName::EndOfTurn).contains(&held.id));
        assert!(!holders(&state, HookName::OnPlayHook).contains(&held.id));

        // In a graveyard only the end-of-turn return of a spell that flagged itself when it was played
        // (#23, #24, #31) — so an unflagged card there answers nothing at all.
        let buried = new_instance(&mut state, ZONE_HOOKS, P1, Zone::Graveyard { player: P1 });
        state.players.p1.graveyard.push(buried.clone());
        assert_ne!(buried.return_to_hand_at_end_of_turn, Some(true));
        assert!(!holders(&state, HookName::StartOfTurn).contains(&buried.id));
        assert!(!holders(&state, HookName::OnPlayHook).contains(&buried.id));
        assert!(!holders(&state, HookName::EndOfTurn).contains(&buried.id));

        // The field copy is still the one holder for each, so nothing above was achieved by emptying
        // the enumeration.
        assert_eq!(holders(&state, HookName::StartOfTurn), vec![on_field.id.clone()]);
        assert_eq!(holders(&state, HookName::OnPlayHook), vec![on_field.id.clone()]);
    }
}

// ---------------------------------------------------------------------------
// R155: when the return-to-hand flag is set (§5.1, §10.5 step 7).
// ---------------------------------------------------------------------------

mod spec_11_r155_the_return_to_hand_flag_m3_gate {
    use super::*;

    /// Every copy of `id` in p1's hand, graveyard and exile: TS pushed one object into a second pile
    /// without taking it out of the first, so a flag written to it showed in both.
    fn copies(state: &GameState, id: &str) -> Vec<CardInstance> {
        let side = &state.players.p1;
        side.hand
            .iter()
            .chain(side.graveyard.iter())
            .chain(side.exile.iter())
            .filter(|card| card.id == id)
            .cloned()
            .collect()
    }

    #[test]
    fn r155_flags_a_spell_that_asks_to_return_as_step_7_lands_it_in_the_graveyard_and_clears_it_that_turn() {
        let mut state = game("r155-flag");
        put(&mut state, LOG_CARD, slot(P1, Row::Backrow, NOTE_LANE));
        let mut sink = SinkFor::new(&state);

        // The real call site: a cast goes through the same landing a play does (R70), so this is §10.5
        // step 7 doing the flagging rather than the setter being poked directly.
        let spell = must(
            in_hand(&mut state, RETURN_SPELL, P1, 1).into_iter().next(),
            "a returning Spell",
        );
        sink.run(&mut state, |engine| {
            resolve::cast_card(engine, &spell, CastOptions::default())
        });
        assert!(ids(&state.players.p1.graveyard).contains(&spell.id));
        assert_eq!(live(&state, &spell).return_to_hand_at_end_of_turn, Some(true));

        // Each of step 7's three conditions, checked by a card that fails exactly one of them. None of
        // these may be flagged, or the flag would mean no more than "in the graveyard this turn".
        //
        // Not a Spell: #13's shape, a Unit with an end-of-turn hook that died the turn it was played.
        // It is in the play log AND in the graveyard, which is why the log cannot stand in for this.
        let mut unit = must(
            in_hand(&mut state, DYING_UNIT, P1, 1).into_iter().next(),
            "a Unit with an end-of-turn hook",
        );
        unit.zone = Zone::Graveyard { player: P1 };
        state.players.p1.hand.retain(|card| card.id != unit.id);
        state.players.p1.graveyard.push(unit.clone());
        state.players.p1.turn_log.played_ids.push(unit.id.clone());
        assert!(
            scripts::scripts_for(&state, &unit.def_id)
                .base
                .end_of_turn
                .is_some()
        );
        resolve::flag_return_to_hand_at_end_of_turn(&mut state, &unit.id);
        assert_ne!(live(&state, &unit).return_to_hand_at_end_of_turn, Some(true));

        // A Spell whose face declares no end-of-turn return.
        let plain = must(
            in_hand(&mut state, SPELL_CRIER, P1, 1).into_iter().next(),
            "a Spell with no end-of-turn hook",
        );
        live_mut(&mut state, &plain).zone = Zone::Graveyard { player: P1 };
        state.players.p1.graveyard.push(live(&state, &plain));
        resolve::flag_return_to_hand_at_end_of_turn(&mut state, &plain.id);
        assert!(
            copies(&state, &plain.id)
                .iter()
                .all(|copy| copy.return_to_hand_at_end_of_turn != Some(true))
        );

        // And #39's shape: a Spell that exiled itself is not in the graveyard when step 7 runs.
        let exiled = must(
            in_hand(&mut state, RETURN_SPELL, P1, 1).into_iter().next(),
            "a second returning Spell",
        );
        live_mut(&mut state, &exiled).zone = Zone::Exile { player: P1 };
        state.players.p1.exile.push(live(&state, &exiled));
        resolve::flag_return_to_hand_at_end_of_turn(&mut state, &exiled.id);
        assert!(
            copies(&state, &exiled.id)
                .iter()
                .all(|copy| copy.return_to_hand_at_end_of_turn != Some(true))
        );

        // The flag means "this turn": cleanup clears it at the end of the turn that set it.
        assert_eq!(state.active, P1);
        sink.run(&mut state, turn::end_turn);
        assert_ne!(
            find_instance(&state, &spell.id).and_then(|card| card.return_to_hand_at_end_of_turn),
            Some(true)
        );
    }

    #[test]
    fn r155_makes_the_flag_alone_the_graveyards_gate_so_this_turns_play_log_is_not_enough() {
        let mut state = game("r155-gate");
        let end_of_turn_holders = |state: &GameState| -> Vec<String> {
            triggers::trigger_holders_with_hook(state, HookName::EndOfTurn, None)
                .iter()
                .map(|holder| holder.card.id.clone())
                .collect()
        };

        // A flagged Spell in the graveyard answers its end-of-turn return (R153's one graveyard hook).
        let mut flagged = must(
            in_hand(&mut state, RETURN_SPELL, P1, 1).into_iter().next(),
            "a returning Spell",
        );
        flagged.zone = Zone::Graveyard { player: P1 };
        state.players.p1.hand = vec![];
        state.players.p1.graveyard.push(flagged.clone());
        live_mut(&mut state, &flagged).return_to_hand_at_end_of_turn = Some(true);
        assert_eq!(end_of_turn_holders(&state), vec![flagged.id.clone()]);

        // An identical Spell in the same graveyard, played this very turn but never flagged — because
        // it never landed there through step 7 — answers nothing. Being in the log is not the gate.
        let unflagged = new_instance(&mut state, RETURN_SPELL, P1, Zone::Graveyard { player: P1 });
        state.players.p1.graveyard.push(unflagged.clone());
        state.players.p1.turn_log.played_ids.push(unflagged.id.clone());
        state.players.p1.turn_log.cards_played += 1;
        assert!(unflagged.return_to_hand_at_end_of_turn.is_none());
        assert_eq!(end_of_turn_holders(&state), vec![flagged.id.clone()]);

        // Flagging it is what admits it, so the gate is the flag and nothing else.
        live_mut(&mut state, &unflagged).return_to_hand_at_end_of_turn = Some(true);
        assert_eq!(
            end_of_turn_holders(&state),
            vec![flagged.id.clone(), unflagged.id.clone()]
        );
    }
}
