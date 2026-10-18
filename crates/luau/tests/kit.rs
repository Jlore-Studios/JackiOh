//! The card kit's tests (#913, SPEC §12.3 to §12.5, R915002–R915004): official cards rewritten in the
//! kit as custom twins, each played beside its Rust card on the same board and compared; the kit's
//! questions and their re-runs; and a script that fails.
//!
//! A twin is the official card's frame (its name, type, cost, stats, keywords, text and numbers) under
//! a custom id, with the kit fixture as its script, so the only difference between the two games is
//! the script. The real catalog and its Rust scripts are registered (`jackioh-cards`, a
//! dev-dependency), and every game runs under `KitHost`, as the browser's Luau module lends it.

use std::rc::Rc;

use jackioh_engine::custom::{custom_def, validate_custom_card, with_custom_scripts};
use jackioh_engine::testkit::{CardDef, PlayerId, Scenario, scenario};
use jackioh_engine::{
    CardCost, CustomCard, CustomFrame, CustomParam, GameEvent, PrintedRarity, Rarity, find_def, fold,
    hash_state,
};
use jackioh_luau::config::CUSTOM_HOOK_INTERRUPTS;
use jackioh_luau::custom::{KitHost, check_custom_source, kit_findings};
use serde_json::{Value, json};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// A card that keeps a hand from auto-ending the turn (§2.5): #5 Quick Study, a cheap Spell.
const FILLER: &str = "core-005";
/// Units to aim at: #8 Mr. Vanilla, a 4/4 with no text, and #19 Menace, a 9/9 Taunt.
const TARGET: &str = "core-008";
const SHIELDED: &str = "core-019";
/// #16 Hit Job: "Destroy a Unit." The kit's twin of it is a fixture too.
const HIT_JOB: &str = "core-016";

/// The real catalog and scripts, then `run` with the kit lent to the engine.
fn kit<R>(run: impl FnOnce() -> R) -> R {
    jackioh_cards::register_all();
    with_custom_scripts(Rc::new(KitHost), run)
}

fn printed(rarity: Rarity) -> PrintedRarity {
    match rarity {
        Rarity::Rare => PrintedRarity::Rare,
        Rarity::Epic => PrintedRarity::Epic,
        Rarity::Legendary => PrintedRarity::Legendary,
        Rarity::Mythic => PrintedRarity::Mythic,
        Rarity::Common | Rarity::Token => PrintedRarity::Common,
    }
}

/// The official card's frame, as a custom card's.
fn frame_of(def: &CardDef) -> CustomFrame {
    CustomFrame {
        name: def.name.clone(),
        type_: def.type_,
        cost: match def.cost {
            CardCost::Fixed(cost) => cost,
            CardCost::Embiggen { base, .. } => base,
            CardCost::X => 0,
        },
        rarity: printed(def.rarity),
        attack: def.base.attack,
        health: def.base.health,
        radiant_attack: def.radiant.attack,
        radiant_health: def.radiant.health,
        keywords: def.base.keywords.iter().map(|keyword| keyword.kind()).collect(),
        text: def.base.text.clone(),
        params: def
            .params
            .iter()
            .flatten()
            .map(|param| CustomParam {
                key: param.key.clone(),
                base: param.base,
                radiant: param.radiant,
            })
            .collect(),
    }
}

/// The official card under a custom id, with `source` as its script.
fn twin(official: &str, source: &str) -> CustomCard {
    let Some(def) = find_def(None, official) else {
        panic!("no {official} in the catalog");
    };
    CustomCard {
        id: format!("custom-{official}"),
        version: 1,
        frame: frame_of(def),
        source: source.to_string(),
        builder: None,
    }
}

/// What a board is, ids aside: each side's hero, mana, the size of each pile, the hand's cards and
/// each unit's lane, stats and keywords, the custom id read as the official one.
fn summary(s: &Scenario, custom: &str, official: &str) -> Value {
    let state = s.state();
    let mut sides = Vec::new();
    for player in [P1, P2] {
        let side = &state.players[player];
        let units: Vec<Value> = (1..=5)
            .filter_map(|lane| s.unit(player, lane).map(|unit| (lane, unit)))
            .map(|(lane, unit)| {
                let view = s.stats(unit.id.as_str());
                let mut keywords: Vec<String> = view
                    .keywords
                    .iter()
                    .map(|keyword| keyword.kind().as_str().to_string())
                    .collect();
                keywords.sort();
                json!({
                    "lane": lane,
                    "def": unit.def_id.replace(custom, official),
                    "attack": view.attack,
                    "health": view.health,
                    "maxHealth": view.max_health,
                    "keywords": keywords,
                })
            })
            .collect();
        let hand: Vec<String> = side
            .hand
            .iter()
            .map(|card| card.def_id.replace(custom, official))
            .collect();
        sides.push(json!({
            "health": side.hero.health,
            "armor": side.hero.armor,
            "mana": [side.mana.current, side.mana.max],
            "hand": hand,
            "library": side.library.len(),
            "graveyard": side.graveyard.len(),
            "exile": side.exile.len(),
            "backrow": (1..=5).filter(|lane| s.backrow(player, *lane).is_some()).count(),
            "units": units,
        }));
    }
    json!({ "sides": sides, "turn": state.turn, "active": state.active.as_str() })
}

fn failures(s: &Scenario) -> Vec<String> {
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::ScriptFailed { message, .. } => Some(message.clone()),
            _ => None,
        })
        .collect()
}

/// R915002: play `official` and its twin on the same board, `setup(id)` naming the card under test,
/// `act(s, id, kit)` playing it (`kit` true for the twin, which asks its questions as prompts), and
/// compare the boards.
fn parity(
    official: &str,
    source: &str,
    setup: impl Fn(&str) -> Value,
    act: impl Fn(&mut Scenario, &str, bool),
) {
    kit(|| {
        let card = twin(official, source);
        assert_eq!(
            check_custom_source(&card),
            Vec::new(),
            "{official}'s kit twin loads"
        );
        let mut real = scenario(setup(official));
        act(&mut real, official, false);
        let mut opts = setup(&card.id);
        opts["custom"] = json!([card]);
        let mut kit = scenario(opts);
        act(&mut kit, &card.id, true);
        assert_eq!(
            failures(&kit),
            Vec::<String>::new(),
            "{official}'s twin ran cleanly"
        );
        assert_eq!(
            summary(&kit, &card.id, official),
            summary(&real, &card.id, official),
            "{official} and its kit twin leave the same board"
        );
    });
}

/// A board with a unit to aim at on each side and fillers in both hands.
fn board(id: &str) -> Value {
    json!({
        "p1": { "hand": [id, FILLER], "field": [TARGET], "library": ["core-011", "core-017", "core-019"], "health": 20 },
        "p2": { "hand": [FILLER, "core-010", "core-012"], "field": [SHIELDED, TARGET], "library": ["core-013"] },
    })
}

fn hero(player: PlayerId) -> Value {
    json!([{ "pick": "hero", "player": player.as_str() }])
}

fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> Value {
    let Some(unit) = s.unit(player, lane) else {
        panic!("no unit in {} lane {lane}", player.as_str());
    };
    json!([{ "pick": "instance", "instanceId": unit.id }])
}

// ---------------------------------------------------------------------------------------------
// R915002: official cards, rewritten in the kit, play as their Rust cards do
// ---------------------------------------------------------------------------------------------

mod r915002_parity {
    use super::*;

    #[test]
    fn r915002_core_005_draw_then_heal_your_hero() {
        parity(
            "core-005",
            "return Card {\n  cry = function(c, n)\n    c:draw(n.draw)\n    c:heal(\"your hero\", n.heal)\n  end,\n}\n",
            |id| json!({ "p1": { "hand": [id, "core-010"], "library": ["core-011", "core-017", "core-019"], "health": 20 }, "p2": { "hand": [FILLER] } }),
            |s, id, _| {
                s.play(id, json!({}));
            },
        );
    }

    #[test]
    fn r915002_core_015_cry_summons_a_rush_token() {
        parity(
            "core-015",
            "return Card {\n  cry = function(c, n)\n    c:summon(\"Rush Token\", n.tokens)\n  end,\n}\n",
            board,
            |s, id, _| {
                s.play(id, json!({ "zone": 3 }));
            },
        );
    }

    #[test]
    fn r915002_core_016_a_declared_target_is_destroyed() {
        parity(
            HIT_JOB,
            "return Card {\n  target = \"a unit\",\n  cry = function(c, n)\n    c:destroy(c:target())\n  end,\n}\n",
            board,
            |s, id, _| {
                let targets = unit_at(s, P2, 2);
                s.play(id, json!({ "targets": targets }));
            },
        );
    }

    #[test]
    fn r915002_classic_003_heals_the_hero_it_targets() {
        parity(
            "classic-003",
            "return Card {\n  target = \"a unit or hero\",\n  cry = function(c, n)\n    c:heal(c:target(), n.heal)\n  end,\n}\n",
            board,
            |s, id, _| {
                s.play(id, json!({ "targets": hero(P1) }));
            },
        );
    }

    #[test]
    fn r915002_classic_016_damages_the_unit_or_hero_it_targets() {
        let source = "return Card {\n  target = \"a unit or hero\",\n  cry = function(c, n)\n    c:damage(c:target(), n.damage)\n  end,\n}\n";
        parity("classic-016", source, board, |s, id, _| {
            s.play(id, json!({ "targets": hero(P2) }));
        });
        parity("classic-016", source, board, |s, id, _| {
            let targets = unit_at(s, P2, 1);
            s.play(id, json!({ "targets": targets }));
        });
    }

    #[test]
    fn r915002_classic_024_draws() {
        parity(
            "classic-024",
            "return Card {\n  cry = function(c, n)\n    c:draw(n.draw)\n  end,\n}\n",
            board,
            |s, id, _| {
                s.play(id, json!({}));
            },
        );
    }

    #[test]
    fn r915002_classicplus_057_buffs_the_unit_it_targets() {
        parity(
            "classicplus-057",
            "return Card {\n  target = \"a unit\",\n  cry = function(c, n)\n    c:buff(c:target(), n.buff, n.buff)\n  end,\n}\n",
            board,
            |s, id, _| {
                let targets = unit_at(s, P1, 1);
                s.play(id, json!({ "targets": targets }));
            },
        );
    }

    #[test]
    fn r915002_classicplus_056_the_enemy_discards_at_random() {
        parity(
            "classicplus-056",
            "return Card {\n  cry = function(c, n)\n    c:discard(n.discards, \"the enemy\")\n  end,\n}\n",
            board,
            |s, id, _| {
                s.play(id, json!({}));
            },
        );
    }

    #[test]
    fn r915002_core_068_reads_the_board_for_its_number() {
        let source = "return Card {\n  target = \"a unit or hero\",\n  cry = function(c, n)\n    local amount = n.damage\n    if c:health(\"your hero\") < n.threshold then\n      amount = n.lowDamage\n    end\n    c:damage(c:target(), amount)\n  end,\n}\n";
        for health in [30, 5] {
            parity(
                "core-068",
                source,
                |id| {
                    let mut setup = board(id);
                    setup["p1"]["health"] = json!(health);
                    setup
                },
                |s, id, _| {
                    s.play(id, json!({ "zone": 3, "targets": hero(P2) }));
                },
            );
        }
    }

    #[test]
    fn r915002_classicplus_028_death_heals_your_hero() {
        parity(
            "classicplus-028",
            "return Card {\n  death = function(c, n)\n    c:heal(\"your hero\", n.heal)\n  end,\n}\n",
            |id| {
                json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [id], "health": 20 },
                    "p2": { "hand": [FILLER] },
                })
            },
            |s, _, _| {
                let targets = unit_at(s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
            },
        );
    }

    #[test]
    fn r915002_classic_082_death_draws() {
        parity(
            "classic-082",
            "return Card {\n  death = function(c, n)\n    c:draw(n.draw)\n  end,\n}\n",
            |id| {
                json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "field": [id], "library": ["core-011", "core-017", "core-019"] },
                    "p2": { "hand": [FILLER] },
                })
            },
            |s, _, _| {
                let targets = unit_at(s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
            },
        );
    }

    #[test]
    fn r915002_core_013_end_of_turn_damages_each_enemy() {
        parity(
            "core-013",
            "return Card {\n  endOfTurn = function(c, n)\n    c:damage(\"all enemies\", n.damage)\n  end,\n}\n",
            |id| {
                json!({
                    "p1": { "hand": [FILLER], "field": [id], "library": ["core-011", "core-017"] },
                    "p2": { "hand": [FILLER, "core-010"], "field": [TARGET, SHIELDED], "library": ["core-013", "core-019"] },
                })
            },
            |s, _, _| {
                s.end_turn();
            },
        );
    }

    #[test]
    fn r915002_core_006_start_of_turn_gains_mana() {
        parity(
            "core-006",
            "return Card {\n  startOfTurn = function(c, n)\n    c:mana(n.mana)\n  end,\n}\n",
            |id| {
                json!({
                    "p1": { "hand": [FILLER], "backrow": [id], "library": ["core-011", "core-017"] },
                    "p2": { "hand": [FILLER, "core-010"], "library": ["core-013", "core-019"] },
                })
            },
            |s, _, _| {
                s.end_turn();
                s.end_turn();
            },
        );
    }

    #[test]
    fn r915002_core_058_start_of_turn_summons_a_rush_token() {
        parity(
            "core-058",
            "return Card {\n  startOfTurn = function(c, n)\n    c:summon(\"Rush Token\")\n  end,\n}\n",
            |id| {
                json!({
                    "p1": { "hand": [FILLER], "backrow": [id], "library": ["core-011", "core-017"] },
                    "p2": { "hand": [FILLER, "core-010"], "library": ["core-013", "core-019"] },
                })
            },
            |s, _, _| {
                s.end_turn();
                s.end_turn();
            },
        );
    }

    #[test]
    fn r915002_core_014_an_aura_gives_your_units_attack_and_keywords() {
        parity(
            "core-014",
            "return Card {\n  aura = { attack = \"attack\", give = { \"Rush\", \"First Strike\" }, to = \"your units\" },\n}\n",
            |id| {
                json!({
                    "p1": { "hand": [FILLER], "field": [TARGET], "backrow": [id] },
                    "p2": { "hand": [FILLER], "field": [TARGET] },
                })
            },
            |_, _, _| {},
        );
    }

    /// The third mode reduces the Cost of a card in your hand since patch v0.3.10 (#831; it gained mana
    /// before), which the kit has no verb for, so the twin names it and only the two it can write are
    /// compared.
    #[test]
    fn r915002_classic_081_an_activate_asks_which_of_its_three() {
        let source = "return Card {\n  activate = {\n    label = \"Choose one\",\n    run = function(c, n)\n      local pick = c:choose({ \"heal\", \"draw\", \"reduce\" })\n      if pick == \"heal\" then\n        c:heal(\"your hero\", n.heal)\n      elseif pick == \"draw\" then\n        c:draw(n.draw)\n      end\n    end,\n  },\n}\n";
        for mode in ["heal", "draw"] {
            parity(
                "classic-081",
                source,
                |id| {
                    json!({
                        "p1": { "hand": [FILLER], "backrow": [id], "library": ["core-011", "core-017"], "health": 20 },
                        "p2": { "hand": [FILLER] },
                    })
                },
                |s, id, kit| {
                    if kit {
                        s.activate(id, json!({}));
                        s.answer(json!(mode));
                    } else {
                        s.activate(id, json!({ "modes": [mode] }));
                    }
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// R915003: questions mid-script
// ---------------------------------------------------------------------------------------------

/// A custom 1-mana 1/1 Unit with `source`, under `id`.
fn custom_unit(id: &str, source: &str) -> CustomCard {
    let card = CustomCard {
        id: id.to_string(),
        version: 1,
        frame: CustomFrame {
            name: "Kit Fixture".to_string(),
            type_: jackioh_engine::CardType::Unit,
            cost: 1,
            rarity: PrintedRarity::Common,
            attack: Some(1),
            health: Some(1),
            radiant_attack: None,
            radiant_health: None,
            keywords: Vec::new(),
            text: String::new(),
            params: vec![CustomParam {
                key: "dmg".to_string(),
                base: 2,
                radiant: 4,
            }],
        },
        source: source.to_string(),
        builder: None,
    };
    assert_eq!(validate_custom_card(&card), Vec::new());
    card
}

#[test]
fn r915003_a_target_asked_mid_script_is_a_prompt_and_the_run_goes_on_with_its_answer() {
    kit(|| {
        let card = custom_unit(
            "custom-ask",
            "return Card {\n  cry = function(c, n)\n    c:draw(1)\n    local t = c:target(\"an enemy unit\")\n    c:damage(t, n.dmg)\n    c:armor(3)\n  end,\n}\n",
        );
        let mut s = scenario(json!({
            "custom": [card],
            "p1": { "hand": ["custom-ask", FILLER], "library": ["core-011", "core-017"] },
            "p2": { "hand": [FILLER], "field": [TARGET] },
        }));
        s.play("custom-ask", json!({ "zone": 2 }));
        assert_eq!(failures(&s), Vec::<String>::new());
        // The draw before the question resolved, then the question opened.
        assert_eq!(
            s.state().players[P1].hand.len(),
            2,
            "the filler and the drawn card"
        );
        let pending = s.state().pending.clone().expect("the question is a prompt");
        assert_eq!(pending.options.len(), 1);
        let enemy = s.unit(P2, 1).expect("the enemy unit");
        s.answer(json!(enemy.id));
        s.expect_stats(enemy.id.as_str(), json!({ "health": 2 }));
        assert_eq!(s.state().players[P1].hero.armor, 3);
        assert_eq!(s.state().players[P1].hand.len(), 2, "the draw was not made again");
        assert_eq!(failures(&s), Vec::<String>::new());
    });
}

#[test]
fn r915003_a_reading_before_the_question_answers_the_same_on_the_run_after_it() {
    kit(|| {
        // Two 1-health enemies: the count reads 2, the damage before the question kills both, and
        // the run after the answer still reads 2 and draws 2.
        let card = custom_unit(
            "custom-reads",
            "return Card {\n  cry = function(c, n)\n    local count = c:count(\"enemy units\")\n    c:damage(\"each enemy unit\", 1)\n    local t = c:target(\"a friendly unit\")\n    c:buff(t, 1, 1)\n    c:draw(count)\n  end,\n}\n",
        );
        let mut s = scenario(json!({
            "custom": [card],
            "p1": { "hand": ["custom-reads", FILLER], "field": [TARGET], "library": ["core-011", "core-017", "core-019"] },
            "p2": { "hand": [FILLER], "field": [{ "def": TARGET, "damage": 3 }, { "def": TARGET, "damage": 3 }] },
        }));
        s.play("custom-reads", json!({ "zone": 2 }));
        assert_eq!(
            s.unit(P2, 1).map(|unit| unit.damage),
            Some(4),
            "the damage resolved before the question; the deaths wait for the hook's end (§4.5)"
        );
        let friend = s.unit(P1, 1).expect("the friendly unit");
        s.answer(json!(friend.id));
        s.expect_stats(friend.id.as_str(), json!({ "attack": 5, "health": 5 }));
        assert_eq!(
            s.state().players[P1].hand.len(),
            3,
            "the filler and the 2 the logged count drew"
        );
        assert_eq!(
            s.state().players[P2].graveyard.len(),
            2,
            "the damage was not dealt again"
        );
        assert_eq!(failures(&s), Vec::<String>::new());
    });
}

#[test]
fn r915003_choose_answers_its_option_and_two_questions_chain() {
    kit(|| {
        let card = custom_unit(
            "custom-choose",
            "return Card {\n  cry = function(c, n)\n    if c:choose({ \"Hit\", \"Guard\" }) == \"Hit\" then\n      c:damage(c:target(\"an enemy\"), n.dmg)\n    else\n      c:armor(n.dmg)\n    end\n  end,\n}\n",
        );
        let setup = json!({
            "custom": [card],
            "p1": { "hand": ["custom-choose", FILLER] },
            "p2": { "hand": [FILLER], "field": [TARGET] },
        });
        let mut hit = scenario(setup.clone());
        hit.play("custom-choose", json!({ "zone": 2 }));
        hit.answer(json!("Hit"));
        hit.answer(json!([{ "pick": "hero", "player": "p2" }]));
        assert_eq!(failures(&hit), Vec::<String>::new());
        assert_eq!(hit.state().players[P2].hero.health, 28);

        let mut guard = scenario(setup);
        guard.play("custom-choose", json!({ "zone": 2 }));
        guard.answer(json!("Guard"));
        assert_eq!(guard.state().players[P1].hero.armor, 2);
        assert!(guard.state().pending.is_none());
    });
}

#[test]
fn r915003_a_target_with_nothing_to_choose_asks_nothing_and_its_verbs_do_nothing() {
    kit(|| {
        let card = custom_unit(
            "custom-none",
            "return Card {\n  cry = function(c, n)\n    local t = c:target(\"an enemy unit\")\n    c:destroy(t)\n    c:draw(1)\n  end,\n}\n",
        );
        let mut s = scenario(json!({
            "custom": [card],
            "p1": { "hand": ["custom-none", FILLER], "library": ["core-011"] },
            "p2": { "hand": [FILLER] },
        }));
        s.play("custom-none", json!({ "zone": 2 }));
        assert!(s.state().pending.is_none(), "no prompt with nothing to choose");
        assert_eq!(s.state().players[P1].hand.len(), 2, "the draw still happened");
    });
}

// ---------------------------------------------------------------------------------------------
// R1174000: c:freeze, the kit's Freeze
// ---------------------------------------------------------------------------------------------

#[test]
fn r1174000_c_freeze_freezes_every_enemy_unit_its_words_name_and_no_hero() {
    kit(|| {
        let card = custom_unit(
            "custom-freeze",
            "return Card {\n  cry = function(c, n)\n    c:freeze(\"all enemies\")\n  end,\n}\n",
        );
        assert_eq!(check_custom_source(&card), Vec::new(), "the freezer loads");
        let mut s = scenario(json!({
            "custom": [card],
            "p1": { "hand": ["custom-freeze", FILLER], "field": [TARGET] },
            "p2": { "hand": [FILLER], "field": [TARGET, SHIELDED] },
        }));
        s.play("custom-freeze", json!({ "zone": 2 }));
        assert_eq!(failures(&s), Vec::<String>::new());
        let turn = s.state().turn;
        for lane in [1, 2] {
            let unit = s.unit(P2, lane).expect("an enemy unit");
            assert_eq!(unit.frozen, Some(turn + 1), "the enemy unit in lane {lane} is Frozen");
        }
        let friendly = s.unit(P1, 1).expect("the friendly unit");
        assert_eq!(friendly.frozen, None, "a friendly unit is not");
        let marks = s
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Marked { .. }))
            .count();
        assert_eq!(marks, 2, "two Units are Frozen, and the enemy hero is not");
    });
}

// ---------------------------------------------------------------------------------------------
// R915004: a script that fails does nothing, and says so
// ---------------------------------------------------------------------------------------------

fn played_failure(id: &str, source: &str) -> (Vec<String>, Scenario) {
    let card = custom_unit(id, source);
    let mut s = scenario(json!({
        "custom": [card],
        "p1": { "hand": [id, FILLER], "library": ["core-011"] },
        "p2": { "hand": [FILLER] },
    }));
    s.play(id, json!({ "zone": 2 }));
    (failures(&s), s)
}

#[test]
fn r915004_a_loop_stops_at_its_budget_and_the_hook_does_nothing() {
    kit(|| {
        let (failed, s) = played_failure(
            "custom-loop",
            "return Card {\n  cry = function(c, n)\n    c:draw(1)\n    while true do\n    end\n  end,\n}\n",
        );
        assert_eq!(failed.len(), 1, "{failed:?}");
        assert!(failed[0].starts_with("cry failed"), "{failed:?}");
        assert!(
            failed[0].contains(&CUSTOM_HOOK_INTERRUPTS.to_string()),
            "{failed:?}"
        );
        assert_eq!(
            s.state().players[P1].hand.len(),
            1,
            "the draw before the loop did not resolve"
        );
        assert_eq!(
            s.unit(P1, 2).map(|unit| unit.def_id),
            Some("custom-loop".to_string()),
            "the card is played all the same"
        );
    });
}

#[test]
fn r915004_an_error_names_its_line_and_a_bad_word_says_what_the_words_are() {
    kit(|| {
        let (failed, _) = played_failure(
            "custom-error",
            "return Card {\n  cry = function(c, n)\n    error(\"boom\")\n  end,\n}\n",
        );
        assert_eq!(failed, vec!["cry failed on line 3: boom".to_string()]);
        let (failed, _) = played_failure(
            "custom-word",
            "return Card {\n  cry = function(c, n)\n    c:damage(\"the moon\", 1)\n  end,\n}\n",
        );
        assert_eq!(failed.len(), 1);
        assert!(
            failed[0].starts_with("cry failed on line 3: no target is called \"the moon\""),
            "{failed:?}"
        );
        let (failed, _) = played_failure(
            "custom-fraction",
            "return Card {\n  cry = function(c, n)\n    c:draw(n.dmg // 3 + 0.5)\n  end,\n}\n",
        );
        assert!(failed[0].contains("whole number"), "{failed:?}");
    });
}

#[test]
fn r915004_a_script_that_does_not_load_says_so_when_its_cry_would_run() {
    kit(|| {
        let (failed, _) = played_failure("custom-broken", "return Card {\n  cry = function(c, n)\n");
        assert_eq!(failed.len(), 1);
        assert!(failed[0].starts_with("its script did not load"), "{failed:?}");
    });
}

// ---------------------------------------------------------------------------------------------
// The creator's check (#913): every problem on its line, before a card is saved
// ---------------------------------------------------------------------------------------------

#[test]
fn r915002_the_check_names_the_line_of_each_kind_of_problem() {
    kit(|| {
        let check = |source: &str| check_custom_source(&custom_unit("custom-check-me", source));
        assert_eq!(
            check("return Card {\n  cry = function(c, n)\n    c:draw(1)\n  end,\n}\n"),
            Vec::new()
        );

        let syntax = check("return Card {\n  cry = function(c, n)\n    c:draw(1\n  end,\n}\n");
        assert_eq!(syntax.len(), 1);
        assert_eq!(syntax[0].line, Some(4), "{syntax:?}");

        let lint = check("return Card {\n  cry = function(c, n)\n    for k in pairs({}) do end\n  end,\n}\n");
        assert!(lint.iter().any(|problem| problem.line == Some(3)), "{lint:?}");

        let key = check("return Card {\n  cri = function(c, n) end,\n}\n");
        assert_eq!(key.len(), 1);
        assert!(key[0].message.contains("Card knows"), "{key:?}");

        let required = check("local J = require(\"@jackioh\")\nreturn Card {}\n");
        assert_eq!(required.len(), 1);
        assert!(required[0].message.contains("requires nothing"), "{required:?}");

        let nothing = check("return 1\n");
        assert_eq!(nothing.len(), 1);
        assert!(nothing[0].message.contains("Card"), "{nothing:?}");
    });
}

#[test]
fn the_kit_is_held_to_the_lint_but_for_its_key_count() {
    let findings = kit_findings();
    assert!(
        findings
            .iter()
            .all(|finding| finding.rule == jackioh_luau::Rule::GenericFor),
        "{findings:?}"
    );
    assert_eq!(findings.len(), 1, "{findings:?}");
}

// ---------------------------------------------------------------------------------------------
// R915006: a game with custom cards folds the same
// ---------------------------------------------------------------------------------------------

#[test]
fn r915006_a_game_with_a_kit_card_folds_to_its_own_hash() {
    kit(|| {
        let card = custom_unit(
            "custom-fold",
            "return Card {\n  cry = function(c, n)\n    c:damage(c:target(\"an enemy unit\"), n.dmg)\n  end,\n}\n",
        );
        let mut deck: Vec<String> = jackioh_engine::registered_catalog()
            .values()
            .filter(|def| jackioh_engine::deckable(def) && def.type_ == jackioh_engine::CardType::Unit)
            .take(19)
            .map(|def| def.id.clone())
            .collect();
        deck.insert(0, card.id.clone());
        let decks = (deck.clone(), deck);
        let input = jackioh_engine::FoldArgs {
            seed: "r915006".to_string(),
            decks,
            custom_cards: Some(vec![card.clone()]),
            log: Vec::new(),
            ..jackioh_engine::FoldArgs::default()
        };
        let start = fold(&input);
        assert!(start.errors.is_empty());
        assert_eq!(start.state.custom_cards.len(), 1);
        assert_eq!(
            start.state.custom_defs.get("custom-fold"),
            Some(&custom_def(&card))
        );
        assert_eq!(hash_state(&fold(&input).state), hash_state(&start.state));
    });
}
