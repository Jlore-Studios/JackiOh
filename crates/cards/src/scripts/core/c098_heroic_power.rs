//! #98 Heroic Power (SPEC §8.5, §6.2 "Start of Game", Quickdraw, Activate; R43, R103, R352, R752–R761).
//! Field Spell, tags Quickdraw, cost (0), Mythic.
//!   Base:    "Indestructible. Start of game: Gain one of 13 random powers, each 'Activate: Spend
//!             (X)': (3) Expedition Map; (1) Life Tap; (1) Steady Shot; (2) Ranching; (1) Cat Cafe;
//!             (1) Ping; (2) Witness Value; (2) Stitching; (1) Armor Up; (2) Die Insect; (2) KY
//!             Brainstorm; (2) Pluck; (3) Terminus Tricks" — each power's words are its catalog line.
//!   Radiant: every power's Radiant words, Armor Up named Tank Up (R757).
//!
//! THE THIRTEEN POWERS LIVE IN `subsystems/hero_power.rs`, NOT HERE. R43 makes this card a subsystem:
//! `HERO_POWERS` is the table with each power's X, its names, its base words and its Radiant words;
//! `roll_power` is the roll and `hero_power` the continuation a Discover comes back to. Since the Heroic
//! Power patch (R752) each power is an Activate ability (R384) — `power_abilities` declares all
//! thirteen, each paying its X in mana, declaring Ping's target and present only while the card rolled
//! it — so this file is the wiring of the card's `Script` to them. Everything is on the instance
//! (`memory.power`, Activate's `memory.activations`), never in a module variable (R43, §10.1), which
//! keeps two Heroic Powers in one game independent.
//!
//! COST (R752). The card costs (0), printed in the catalog, and playing it uses nothing: it has no Cry.
//! The power's X is the ability's mana price, paid as it is activated (R384's costs), so the play
//! validator, `legal_actions` and the client read the printed (0) and the ability's price with no
//! special case for this card.
//!
//! THE PROMPTED POWERS (§10.6). Witness Value, Stitching and Terminus Tricks Discover, and `hero_power`
//! is the step they resume at, so the card exposes it under the key the subsystem names
//! (`POWER_RESUME`). Ping's target is declared with the activation (R81), so it never prompts.
//!
//! WHAT THIS FILE DELIBERATELY DOES NOT SAY:
//!   * Indestructible is a printed keyword on both catalog faces, so it is a §10.4 layer, not a
//!     script. R46: "an Indestructible Field Spell (Heroic Power) simply stays".
//!   * Quickdraw is `staticFlags.quickdraw`, which `setup.rs` step 2 reads to put the card in the
//!     opening hand instead of a draw (§6.2). The catalog's Quickdraw *tag* is what a filter sees;
//!     the flag is what setup sees.
//!   * The roll on arrival (R151) is the engine's: `draw.rs`'s `run_arrival_hooks` fires this card's
//!     `startOfGame` as it reaches a hand or library, and a summon onto the field does the same, so
//!     the roll this file declares is the one that runs everywhere.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-098";

/// Both faces are the same wiring: `power_abilities(radiant)` builds each power on the face that is
/// running (§5.2), and `has` keeps only the one the instance rolled.
fn heroic_power(radiant: bool) -> Script {
    Script {
        // §6.2: starts in the opening hand instead of a draw (setup step 2).
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        // R43: every copy in either hand or library rolls its power after the mulligan (§6.2).
        start_of_game: Some(hook(|_ctx| vec![subsystems::roll_power()])),
        // R752: each power is an "Activate: Spend (X)" ability; the card has the one it rolled.
        activations: subsystems::power_abilities(radiant),
        // §10.6: where a Discover comes back to.
        resume: IndexMap::from([(subsystems::POWER_RESUME, hook(subsystems::hero_power::hero_power))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: heroic_power(false),
        radiant: heroic_power(true),
    }
}

// #98 Heroic Power — SPEC §8.5, §6.2 ("Start of Game", Quickdraw, Activate), §10.2, §10.6, §10.8;
// R43, R46, R81, R103, R352, R752–R761 (the Heroic Power patch, issue #37).
//
// BUILD M4-T4 row 98: "In opening hand; costs (0) and playing it uses nothing; power chosen at start
// of game from the seed; each power an Activate paying its X once per turn; Indestructible; each power
// on both faces".
//
// HOW A POWER IS PINNED. R103 makes the power names state, so a test that wants a named one writes
// that name into `memory.power` — which is exactly and only what `subsystems::ensure_power` writes, so
// the state is one the engine produces. Every assertion below is then about what the card DID.
//
// What the harness cannot reach: `scenario()` skips §2.1, so `startOfGame` never runs and the
// start-of-game roll has no card-level path; `crates/engine/tests/rules/hero_power.rs` covers the roll
// by calling `finish_setup` directly, and the arrival roll (R151) with a bounce.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HEROIC: &str = "core-098"; // Field Spell, cost (0), Mythic, tag Quickdraw, Indestructible
    const RUSH_TOKEN: &str = "core-t-rush";
    const FELINOR_TOKEN: &str = "core-t-felinor";
    const GHOUL_TOKEN: &str = "core-t-ghoul";

    /// #53 Reno, a 3-cost Unit: the spare card that keeps §2.5's auto-end away from the assertions.
    const SPARE: &str = "core-053";
    /// #36 Magic Jammed, a 1-cost Spell that destroys a chosen backrow card (R46's test).
    const JAMMED: &str = "core-036";
    /// #19 Midrange Menace, a 3-cost 9/9 Unit — the permanent Expedition Map finds in a library.
    const MENACE: &str = "core-019";
    /// #93.1 Combo-Fodder, a 0-cost Spell token: a card p2 can always play, so its turn never auto-ends.
    const FREE: &str = "core-093-1";

    use crate::js;

    use crate::matches_object;

    /// The keys TS's `Object.keys(script)` would list: every member the face sets.
    fn members(script: &Script) -> Vec<&'static str> {
        let mut keys = Vec::new();
        let mut note = |present: bool, key: &'static str| {
            if present {
                keys.push(key);
            }
        };
        note(script.cost.is_some(), "cost");
        note(script.cry.is_some(), "cry");
        note(script.death.is_some(), "death");
        note(script.start_of_game.is_some(), "startOfGame");
        note(!script.resume.is_empty(), "resume");
        note(script.delayed.is_some(), "delayed");
        note(script.set_stat.is_some(), "setStat");
        note(script.start_of_turn.is_some(), "startOfTurn");
        note(script.end_of_turn.is_some(), "endOfTurn");
        note(script.aura.is_some(), "aura");
        note(!script.triggers.is_empty(), "triggers");
        note(script.on_play_hook.is_some(), "onPlayHook");
        note(!script.hand_triggers.is_empty(), "handTriggers");
        note(script.static_flags.is_some(), "staticFlags");
        note(!script.targets.is_empty(), "targets");
        note(!script.modes.is_empty(), "modes");
        note(script.condition_met.is_some(), "conditionMet");
        note(script.preview.is_some(), "preview");
        note(!script.activations.is_empty(), "activations");
        note(!script.target_checks.is_empty(), "targetChecks");
        note(script.cost_aura.is_some(), "costAura");
        note(script.graveyard_play.is_some(), "graveyardPlay");
        note(script.targeting_discards.is_some(), "targetingDiscards");
        note(script.records_play_as.is_some(), "recordsPlayAs");
        note(script.draw_limit.is_some(), "drawLimit");
        note(!script.replacements.is_empty(), "replacements");
        note(script.hero_guard.is_some(), "heroGuard");
        note(script.conditional_keywords.is_some(), "conditionalKeywords");
        note(script.after_attack.is_some(), "afterAttack");
        note(script.plague_multiplier.is_some(), "plagueMultiplier");
        note(!script.deck_triggers.is_empty(), "deckTriggers");
        note(!script.graveyard_triggers.is_empty(), "graveyardTriggers");
        note(script.quests.is_some(), "quests");
        note(script.tribute_when.is_some(), "tributeWhen");
        note(script.would_counter.is_some(), "wouldCounter");
        note(script.start_of_opponent_turn.is_some(), "startOfOpponentTurn");
        keys
    }

    fn sorted(mut keys: Vec<&'static str>) -> Vec<&'static str> {
        keys.sort();
        keys
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        value.unwrap_or_else(|| panic!("expected {what}"))
    }

    fn events_of(s: &Scenario, kind: &str) -> Vec<Value> {
        s.last_events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == kind)
            .collect()
    }

    fn units_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        (1..=5).filter_map(|lane| s.unit(player, lane)).collect()
    }

    /// Writes the power's name into `memory.power` (R103), as `subsystems::ensure_power` would, and
    /// hands back the card as it now stands.
    fn set_power(s: &mut Scenario, card: &CardInstance, name: &str) -> CardInstance {
        let live = find_instance_mut(s.state_mut(), &card.id).expect("the Heroic Power is in the state");
        live.memory.insert(subsystems::POWER_KEY.to_string(), json!(name));
        live.clone()
    }

    #[derive(Default)]
    struct OnField {
        radiant_face: bool,
        p1: Option<Value>,
        p2: Option<Value>,
        seed: Option<String>,
    }

    /// Both sides' turns pass with cards to draw and, for p2, a card to play (§2.5's auto-end aside).
    fn turns() -> OnField {
        OnField {
            p1: Some(json!({ "hand": [SPARE], "mana": 8, "library": [JAMMED, JAMMED, JAMMED] })),
            p2: Some(json!({ "hand": [FREE], "library": [JAMMED, JAMMED, JAMMED] })),
            ..OnField::default()
        }
    }

    /// A #98 on p1's backrow with a named power, mana to spend and a spare card in hand.
    fn on_field(name: &str, opts: OnField) -> (Scenario, CardInstance) {
        let mut p1 = json!({ "hand": [SPARE], "mana": 8 });
        let extra = opts.p1.unwrap_or_else(|| json!({}));
        if let Some(fields) = extra.as_object() {
            for (key, value) in fields {
                p1[key.as_str()] = value.clone();
            }
        }
        let mut backrow = vec![json!({ "def": HEROIC, "radiant": opts.radiant_face })];
        backrow.extend(extra["backrow"].as_array().cloned().unwrap_or_default());
        p1["backrow"] = json!(backrow);
        let mut options = json!({
            "seed": opts.seed.unwrap_or_else(|| format!("hp-{name}")),
            "p1": p1,
        });
        if let Some(p2) = opts.p2 {
            options["p2"] = p2;
        }
        let mut s = scenario(options);
        let card = must(s.backrow(P1, 1), "the Heroic Power");
        let power = set_power(&mut s, &card, name);
        (s, power)
    }

    fn unit_pick(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    /// The def ids a Discover prompt offers (§6.3), in the order offered.
    fn offered_ids(s: &Scenario) -> Vec<String> {
        let pending = js(must(s.state().pending.as_ref(), "a discover prompt"));
        pending["options"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter(|option| option["selection"]["pick"] == "mode")
            .map(|option| option["selection"]["option"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    /// The power's activations `legal_actions` lists for p1 now.
    fn listed(s: &Scenario, card: &CardInstance) -> Vec<Value> {
        legal_actions(s.state(), P1)
            .iter()
            .map(js)
            .filter(|body| body["type"] == "activate" && body["instanceId"] == card.id.as_str())
            .collect()
    }

    fn effective(s: &Scenario, card: &CardInstance) -> i32 {
        effective_cost(s.state(), card, Default::default())
    }

    fn def_json(id: &str) -> Value {
        js(&crate::card_def(id))
    }

    fn tags_of(id: &str) -> Vec<Value> {
        def_json(id)["tags"].as_array().cloned().unwrap_or_default()
    }

    fn hand_ids(s: &Scenario) -> Vec<String> {
        s.hand(P1).into_iter().map(|card| card.def_id).collect()
    }

    fn view_json(s: &Scenario, seat: PlayerId) -> Value {
        js(&s.view(seat))
    }

    // -------------------------------------------------------------------------------------------
    // The card: its data, its wiring and its keyword.
    // -------------------------------------------------------------------------------------------

    mod n98_heroic_power_the_card_r752 {
        use super::*;

        #[test]
        fn r752_is_a_mythic_field_spell_costing_0_tagged_quickdraw_indestructible_on_both_faces() {
            crate::register_all();
            let def = def_json(HEROIC);
            assert_eq!(def["type"], "Field Spell");
            assert_eq!(def["cost"], 0);
            assert_eq!(def["rarity"], "Mythic");
            assert!(tags_of(HEROIC).contains(&json!("Quickdraw")));
            for face in [&def["base"], &def["radiant"]] {
                let kinds: Vec<Value> = face["keywords"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|keyword| keyword["kind"].clone())
                    .collect();
                assert!(kinds.contains(&json!("Indestructible")));
                let text = face["text"].as_str().unwrap_or_default();
                assert!(text.contains("Gain one of 13 random powers. Each is an Activate that spends its (X)."));
                assert!(!text.contains("Playing it") && !text.contains("Once per turn, spend"));
            }
            assert_eq!(
                def["params"][0],
                json!({ "key": "shot", "base": 2, "radiant": 4, "better": "up", "step": 2, "min": 1 }),
            );
            // #493: the other powers' numbers, each one a power's (`n98_heroic_power_the_numbers_r386`).
            let keys: Vec<Value> = def["params"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|param| param["key"].clone())
                .collect();
            assert_eq!(
                keys,
                ["shot", "tapDraw", "tapDamage", "ping", "stitchCost", "armor", "insect", "discount", "fruitCost"]
                    .map(|key| json!(key))
                    .to_vec(),
            );
            assert_eq!(def["refs"], json!([RUSH_TOKEN, FELINOR_TOKEN, GHOUL_TOKEN]));
        }

        #[test]
        fn r752_each_power_s_line_on_each_face_is_its_ability_s_words_after_its_x_and_its_name() {
            crate::register_all();
            let def = def_json(HEROIC);
            let base = def["base"]["text"].as_str().unwrap_or_default().to_string();
            let radiant = def["radiant"]["text"].as_str().unwrap_or_default().to_string();
            for entry in subsystems::HERO_POWERS.iter() {
                assert!(base.contains(&format!("({}) {}: {}", entry.x, entry.title, entry.label)));
                assert!(radiant.contains(&format!("({}) {}: {}", entry.x, entry.radiant_title, entry.radiant_label)));
            }
            // The printed words, the card's numbers filled in (B3.4 rule 5).
            let printed = fill_params(&crate::card_def(HEROIC), FaceKind::Radiant, None);
            assert!(printed.contains("(1) Tank Up: Your hero gains 4 Armor, then this power refreshes."));
        }

        #[test]
        fn s6_2_quickdraw_both_faces_carry_the_flag_setup_ts_reads_for_the_opening_hand() {
            crate::register_all();
            let scripts = script();
            assert_eq!(scripts.base.flags().quickdraw, Some(true));
            assert_eq!(scripts.radiant.flags().quickdraw, Some(true));
        }

        #[test]
        fn r752_both_faces_wire_the_roll_the_thirteen_abilities_and_the_discover_step_no_cost_hook_no_cry() {
            crate::register_all();
            let scripts = script();
            let wanted = sorted(vec!["staticFlags", "startOfGame", "activations", "resume"]);
            assert_eq!(sorted(members(&scripts.base)), wanted);
            assert_eq!(sorted(members(&scripts.radiant)), wanted);
            let ids: Vec<String> = scripts.base.activations.iter().map(|decl| decl.id.clone()).collect();
            assert_eq!(json!(ids), js(subsystems::HERO_POWER_NAMES));
            let prices: Vec<Option<i32>> = scripts
                .radiant
                .activations
                .iter()
                .map(|decl| decl.cost.and_then(|cost| cost.mana))
                .collect();
            let xs: Vec<Option<i32>> = subsystems::HERO_POWERS.iter().map(|entry| Some(entry.x)).collect();
            assert_eq!(prices, xs);
            assert_eq!(scripts.base.resume.keys().copied().collect::<Vec<_>>(), vec![subsystems::POWER_RESUME]);
        }

        #[test]
        fn r46_indestructible_a_field_spell_that_is_destroyed_simply_stays() {
            crate::register_all();
            let (mut s, card) = on_field(
                "burn",
                OnField { p1: Some(json!({ "hand": [JAMMED], "mana": 8 })), ..OnField::default() },
            );
            s.play(JAMMED, json!({ "targets": unit_pick(&card) }));
            s.expect_in_zone(&card, "field");
            assert_eq!(s.backrow(P1, 1).map(|found| found.id), Some(card.id.clone()));
        }
    }

    // -------------------------------------------------------------------------------------------
    // Playing it, and using a power (R752).
    // -------------------------------------------------------------------------------------------

    mod n98_heroic_power_costs_0_each_power_an_activate_r752 {
        use super::*;

        #[test]
        fn r752_playing_it_costs_0_and_uses_nothing_the_power_is_then_one_activation_a_turn_for_its_x() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "hp-play", "p1": { "hand": [HEROIC, SPARE], "mana": 8 } }));
            let in_hand = must(s.hand(P1).into_iter().next(), "the Heroic Power");
            let card = set_power(&mut s, &in_hand, "burn");
            assert_eq!(effective(&s, &card), 0);
            s.play(&card, json!({ "zone": 1 }));
            s.expect_mana(P1, 8).expect_health(P2, 30);
            assert!(events_of(&s, "activated").is_empty());

            s.activate(&card, json!({}));
            s.expect_mana(P1, 7).expect_health(P2, 28);
            assert_eq!(events_of(&s, "activated").len(), 1);
            assert!(listed(&s, &card).is_empty());
            s.expect_refused_with(|s| s.activate(&card, json!({})), "already been used this turn");
        }

        #[test]
        fn r752_a_power_its_controller_cannot_pay_for_is_neither_listed_nor_accepted() {
            crate::register_all();
            let (mut s, card) = on_field(
                "tricks",
                OnField { p1: Some(json!({ "hand": [SPARE], "mana": 2 })), ..OnField::default() },
            ); // X 3
            assert!(listed(&s, &card).is_empty());
            s.expect_refused_with(|s| s.activate(&card, json!({})), "costs 3, more than your mana");
        }

        #[test]
        fn r752_r81_ping_is_listed_once_per_unit_or_hero_it_may_hit_so_the_client_can_drag_it_onto_one() {
            crate::register_all();
            let (s, card) = on_field(
                "ping",
                OnField { p2: Some(json!({ "field": [FELINOR_TOKEN] })), ..OnField::default() },
            );
            let aims: Vec<Value> = listed(&s, &card)
                .iter()
                .map(|body| if body["type"] == "activate" { body["targets"].clone() } else { Value::Null })
                .collect();
            assert!(aims.contains(&json!([{ "pick": "hero", "player": "p2" }])));
            assert!(aims.contains(&json!([{ "pick": "hero", "player": "p1" }])));
            assert_eq!(aims.len(), 3);
        }

        #[test]
        fn r752_the_hero_panel_names_the_power_by_its_name_on_the_card_with_its_x_and_its_use() {
            crate::register_all();
            let (s, card) = on_field("armor", OnField { radiant_face: true, ..OnField::default() });
            assert!(matches_object(
                &view_json(&s, P1)["you"]["hero"]["power"],
                &json!({ "instanceId": card.id, "name": "armor", "title": "Tank Up", "x": 1, "usedThisTurn": false }),
            ));
            assert!(matches_object(
                &view_json(&s, P2)["opponent"]["hero"]["power"],
                &json!({ "name": "armor", "title": "Tank Up" }),
            ));
        }
    }

    // -------------------------------------------------------------------------------------------
    // The thirteen powers, base face.
    // -------------------------------------------------------------------------------------------

    mod n98_heroic_power_the_powers_r752_r761 {
        use super::*;

        #[test]
        fn r43_3_expedition_map_recruits_a_permanent() {
            crate::register_all();
            let (mut s, card) = on_field(
                "recruit",
                OnField { p1: Some(json!({ "library": [MENACE], "hand": [SPARE], "mana": 8 })), ..OnField::default() },
            );
            s.activate(&card, json!({}));
            assert_eq!(
                units_of(&s, P1).into_iter().map(|unit| unit.def_id).collect::<Vec<_>>(),
                vec![MENACE.to_string()],
            );
            assert_eq!(units_of(&s, P1).first().map(|unit| unit.radiant), Some(false));
            s.expect_mana(P1, 5);
        }

        #[test]
        fn r753_1_life_tap_draws_1_then_deals_2_damage_to_your_own_hero() {
            crate::register_all();
            let (mut s, card) = on_field(
                "draw",
                OnField {
                    p1: Some(json!({ "library": [MENACE, JAMMED], "hand": [SPARE], "mana": 8 })),
                    ..OnField::default()
                },
            );
            s.activate(&card, json!({}));
            assert_eq!(hand_ids(&s), vec![SPARE.to_string(), MENACE.to_string()]);
            s.expect_health(P1, 28).expect_mana(P1, 7);
            assert_eq!(
                events_of(&s, "damage").iter().map(|event| event["targetId"].clone()).collect::<Vec<_>>(),
                vec![json!("hero-p1")],
            );
        }

        #[test]
        fn r754_1_steady_shot_deals_shot_2_to_the_enemy_hero() {
            crate::register_all();
            let (mut s, card) = on_field("burn", OnField::default());
            s.activate(&card, json!({}));
            s.expect_health(P2, 28);
        }

        #[test]
        fn r752_2_ranching_summons_a_rush_token() {
            crate::register_all();
            let (mut s, card) = on_field("rush", OnField::default());
            s.activate(&card, json!({}));
            assert_eq!(
                units_of(&s, P1).into_iter().map(|unit| (unit.def_id, unit.radiant)).collect::<Vec<_>>(),
                vec![(RUSH_TOKEN.to_string(), false)],
            );
        }

        #[test]
        fn r755_1_cat_cafe_summons_a_felinor_token() {
            crate::register_all();
            let (mut s, card) = on_field("felinor", OnField::default());
            s.activate(&card, json!({}));
            assert_eq!(
                units_of(&s, P1).into_iter().map(|unit| unit.def_id).collect::<Vec<_>>(),
                vec![FELINOR_TOKEN.to_string()],
            );
        }

        #[test]
        fn r756_1_ping_deals_1_pierce_damage_to_the_unit_or_hero_declared_with_it() {
            crate::register_all();
            let (mut s, card) = on_field("ping", OnField::default());
            s.activate(&card, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_health(P2, 29);
            assert!(s.state().pending.is_none());
            // R81: the target travels with the activation; a Ping that names none is refused.
            let (mut bare, bare_power) = on_field("ping", OnField { seed: Some("hp-ping-bare".into()), ..OnField::default() });
            bare.expect_refused(|s| s.activate(&bare_power, json!({})));
        }

        #[test]
        fn r103_2_witness_value_discovers_a_unit_which_goes_to_hand() {
            crate::register_all();
            let (mut s, card) = on_field("discover", OnField::default());
            s.activate(&card, json!({}));
            let offered = offered_ids(&s);
            assert_eq!(offered.len(), 3);
            for id in &offered {
                assert_eq!(def_json(id)["type"], "Unit");
            }
            let first = must(offered.first().cloned(), "an offered Unit");
            s.answer(json!(first));
            assert_eq!(
                s.hand(P1).into_iter().filter(|entry| entry.def_id == first).map(|entry| entry.radiant).collect::<Vec<_>>(),
                vec![false],
            );
            s.expect_mana(P1, 6);
        }

        #[test]
        fn r352_2_stitching_discovers_two_units_that_cost_2_or_less_and_fuses_them_into_your_hand() {
            crate::register_all();
            let (mut s, card) = on_field("stitching", OnField::default());
            let before = s.hand(P1).len();
            s.activate(&card, json!({}));
            for step in [1, 2] {
                let offered = offered_ids(&s);
                assert_eq!(offered.len(), 3, "Discover {step}");
                for id in &offered {
                    let cost = def_json(id)["cost"].clone();
                    assert!(cost.as_i64().is_some_and(|price| price <= i64::from(subsystems::STITCHING_MAX_COST)));
                }
                s.answer(json!(must(offered.first().cloned(), "an offered Unit")));
            }
            assert_eq!(s.hand(P1).len(), before + 1);
            assert_eq!(events_of(&s, "fused").len(), 1);
            s.expect_refused_with(|s| s.activate(&card, json!({})), "already been used this turn");
        }

        #[test]
        fn r757_1_armor_up_gives_your_hero_2_armor_until_your_next_turn() {
            crate::register_all();
            let (mut s, card) = on_field("armor", turns());
            s.activate(&card, json!({}));
            assert_eq!(view_json(&s, P1)["you"]["hero"]["armor"], 2);
            s.end_turn();
            assert_eq!(s.state().active, P2);
            assert_eq!(view_json(&s, P1)["you"]["hero"]["armor"], 2);
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(view_json(&s, P1)["you"]["hero"]["armor"], 0);
        }

        #[test]
        fn r758_2_die_insect_deals_8_damage_to_a_random_enemy() {
            crate::register_all();
            let (mut s, card) = on_field("insect", OnField { p2: Some(json!({ "field": [MENACE] })), ..OnField::default() });
            s.activate(&card, json!({}));
            let hits = events_of(&s, "damage");
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0]["amount"], 8);
            let menace = must(s.unit(P2, 1), "the enemy Menace");
            assert!([json!("hero-p2"), json!(menace.id)].contains(&hits[0]["targetId"]));
        }

        #[test]
        fn r759_2_ky_brainstorm_adds_a_random_ky_card_then_every_spell_in_your_hand_costs_1_less() {
            crate::register_all();
            let (mut s, card) = on_field(
                "brainstorm",
                OnField { p1: Some(json!({ "hand": [SPARE, JAMMED], "mana": 8 })), ..OnField::default() },
            );
            s.activate(&card, json!({}));
            let hand = s.hand(P1);
            let added = must(hand.last().cloned(), "the KY card");
            assert!(tags_of(&added.def_id).contains(&json!("KY")));
            assert!(!crate::card_def(&added.def_id).token);
            assert!(!added.radiant);
            let printed = def_json(&added.def_id)["cost"].as_i64().unwrap_or_default() as i32;
            assert_eq!(effective(&s, &added), (printed - 1).max(0));
            assert_eq!(effective(&s, &must(hand.iter().find(|entry| entry.def_id == JAMMED).cloned(), "Magic Jammed")), 0);
            assert_eq!(effective(&s, &must(hand.iter().find(|entry| entry.def_id == SPARE).cloned(), "Reno")), 3);
        }

        #[test]
        fn r760_2_pluck_adds_a_random_fruit_to_your_hand_costing_0() {
            crate::register_all();
            let (mut s, card) = on_field("pluck", OnField::default());
            s.activate(&card, json!({}));
            let fruit = must(s.hand(P1).last().cloned(), "the Fruit");
            assert!(tags_of(&fruit.def_id).contains(&json!("Fruit")));
            assert_eq!(effective(&s, &fruit), 0);
            assert!(!fruit.radiant);
        }

        #[test]
        fn r761_3_terminus_tricks_discovers_a_trap_and_summons_it_face_down_into_your_backrow() {
            crate::register_all();
            let (mut s, card) = on_field("tricks", OnField::default());
            s.activate(&card, json!({}));
            let offered = offered_ids(&s);
            assert_eq!(offered.len(), 3);
            let traps: Vec<String> = crate::query::query(&json_as(json!({ "type": ["Trap", "Field Trap"] })))
                .iter()
                .map(|def| def.id.clone())
                .collect();
            for id in &offered {
                assert!(traps.contains(id));
            }
            let first = must(offered.first().cloned(), "an offered Trap");
            s.answer(json!(first));
            let set = must(s.backrow(P1, 2), "the summoned Trap");
            assert_eq!(set.def_id, first);
            assert!(!set.radiant);
            assert!(matches_object(&view_json(&s, P2)["opponent"]["backrow"][1], &json!({ "faceDown": true })));
            s.expect_mana(P1, 5);
        }
    }

    // -------------------------------------------------------------------------------------------
    // The thirteen powers, Radiant face.
    // -------------------------------------------------------------------------------------------

    mod n98_heroic_power_the_powers_radiant_r752_r761 {
        use super::*;

        #[test]
        fn r43_expedition_map_makes_the_permanent_radiant() {
            crate::register_all();
            let (mut s, card) = on_field(
                "recruit",
                OnField {
                    radiant_face: true,
                    p1: Some(json!({ "library": [MENACE], "hand": [SPARE], "mana": 8 })),
                    ..OnField::default()
                },
            );
            s.activate(&card, json!({}));
            assert!(must(units_of(&s, P1).into_iter().next(), "the recruited #19").radiant);
        }

        #[test]
        fn r753_life_tap_draws_the_top_card_of_each_player_s_deck_and_deals_no_damage() {
            crate::register_all();
            let (mut s, card) = on_field(
                "draw",
                OnField {
                    radiant_face: true,
                    p1: Some(json!({ "library": [MENACE], "hand": [SPARE], "mana": 8 })),
                    p2: Some(json!({ "library": [JAMMED] })),
                    ..OnField::default()
                },
            );
            s.activate(&card, json!({}));
            assert_eq!(hand_ids(&s), vec![SPARE.to_string(), MENACE.to_string(), JAMMED.to_string()]);
            s.expect_health(P1, 30);
        }

        #[test]
        fn r754_steady_shot_deals_shot_4_then_upgrades_itself_by_2_damage_for_good() {
            crate::register_all();
            let (mut s, card) = on_field("burn", OnField { radiant_face: true, ..turns() });
            s.activate(&card, json!({}));
            s.expect_health(P2, 26);
            assert!(matches_object(&json!(events_of(&s, "numberChanged")), &json!([{ "key": "shot", "value": 6 }])));
            s.end_turn().end_turn();
            s.activate(&card, json!({}));
            assert_eq!(
                events_of(&s, "damage")
                    .iter()
                    .map(|event| (event["targetId"].clone(), event["amount"].clone()))
                    .collect::<Vec<_>>(),
                vec![(json!("hero-p2"), json!(6))],
            );
            assert!(matches_object(&json!(events_of(&s, "numberChanged")), &json!([{ "key": "shot", "value": 8 }])));
        }

        #[test]
        fn r752_ranching_summons_a_radiant_rush_token() {
            crate::register_all();
            let (mut s, card) = on_field("rush", OnField { radiant_face: true, ..OnField::default() });
            s.activate(&card, json!({}));
            assert_eq!(
                units_of(&s, P1).into_iter().map(|unit| (unit.def_id, unit.radiant)).collect::<Vec<_>>(),
                vec![(RUSH_TOKEN.to_string(), true)],
            );
        }

        #[test]
        fn r755_cat_cafe_summons_a_random_non_token_felinor() {
            crate::register_all();
            let (mut s, card) = on_field("felinor", OnField { radiant_face: true, ..OnField::default() });
            s.activate(&card, json!({}));
            let summoned = must(units_of(&s, P1).into_iter().next(), "a Felinor");
            assert!(tags_of(&summoned.def_id).contains(&json!("Felinor")));
            assert!(!crate::card_def(&summoned.def_id).token);
        }

        #[test]
        fn r756_ping_killing_a_unit_summons_a_ghoul_token_with_its_stats() {
            crate::register_all();
            let (mut s, card) = on_field(
                "ping",
                OnField { radiant_face: true, p2: Some(json!({ "field": [FELINOR_TOKEN] })), ..OnField::default() },
            );
            let kitten = must(s.unit(P2, 1), "the enemy Felinor Token");
            s.activate(&card, json!({ "targets": unit_pick(&kitten) }));
            assert!(s.unit(P2, 1).is_none());
            let ghoul = must(units_of(&s, P1).into_iter().next(), "a Ghoul Token");
            assert_eq!(ghoul.def_id, GHOUL_TOKEN);
            s.expect_stats(&ghoul, json!({ "attack": 1, "health": 1 }));
        }

        #[test]
        fn r103_witness_value_discovers_a_radiant_unit() {
            crate::register_all();
            let (mut s, card) = on_field("discover", OnField { radiant_face: true, ..OnField::default() });
            s.activate(&card, json!({}));
            let picked = must(offered_ids(&s).into_iter().next(), "an offered Unit");
            s.answer(json!(picked));
            assert_eq!(
                s.hand(P1).into_iter().filter(|entry| entry.def_id == picked).map(|entry| entry.radiant).collect::<Vec<_>>(),
                vec![true],
            );
        }

        #[test]
        fn r352_stitching_fuses_two_radiant_units_into_a_radiant_card() {
            crate::register_all();
            let (mut s, card) = on_field("stitching", OnField { radiant_face: true, ..OnField::default() });
            s.activate(&card, json!({}));
            s.answer(json!(must(offered_ids(&s).into_iter().next(), "a first Unit")));
            s.answer(json!(must(offered_ids(&s).into_iter().next(), "a second Unit")));
            assert!(must(s.hand(P1).last().cloned(), "the fused card").radiant);
        }

        #[test]
        fn r757_tank_up_keeps_4_armor_then_refreshes_into_a_different_power_you_may_use_this_turn() {
            crate::register_all();
            let (mut s, card) = on_field("armor", OnField { radiant_face: true, ..turns() });
            s.activate(&card, json!({}));
            assert_eq!(s.state().players.p1.hero.armor, 4);
            let now = must(s.backrow(P1, 1), "the Heroic Power");
            let next = must(subsystems::power_of(&now), "the new power");
            assert_ne!(js(&next.name), json!("armor"));
            let panel = view_json(&s, P1)["you"]["hero"]["power"].clone();
            assert!(!panel.is_null(), "the hero panel's power");
            assert!(matches_object(&panel, &json!({ "name": js(&next.name), "usedThisTurn": false })));
            s.end_turn().end_turn();
            assert!(view_json(&s, P1)["you"]["hero"]["armor"].as_i64().unwrap_or_default() >= 4);
        }

        #[test]
        fn r758_die_insect_is_lucky_1_and_still_deals_8() {
            crate::register_all();
            let (mut s, card) = on_field(
                "insect",
                OnField { radiant_face: true, p2: Some(json!({ "field": [MENACE] })), ..OnField::default() },
            );
            s.activate(&card, json!({}));
            assert_eq!(
                events_of(&s, "damage").iter().map(|event| event["amount"].clone()).collect::<Vec<_>>(),
                vec![json!(8)],
            );
        }

        #[test]
        fn r759_ky_brainstorm_s_ky_card_is_radiant() {
            crate::register_all();
            let (mut s, card) = on_field("brainstorm", OnField { radiant_face: true, ..OnField::default() });
            s.activate(&card, json!({}));
            let added = must(s.hand(P1).last().cloned(), "the KY card");
            assert!(tags_of(&added.def_id).contains(&json!("KY")));
            assert!(added.radiant);
        }

        #[test]
        fn r760_pluck_s_fruit_is_radiant_and_costs_0() {
            crate::register_all();
            let (mut s, card) = on_field("pluck", OnField { radiant_face: true, ..OnField::default() });
            s.activate(&card, json!({}));
            let fruit = must(s.hand(P1).last().cloned(), "the Fruit");
            assert!(fruit.radiant);
            assert_eq!(effective(&s, &fruit), 0);
        }

        #[test]
        fn r761_terminus_tricks_summons_a_radiant_trap() {
            crate::register_all();
            let (mut s, card) = on_field("tricks", OnField { radiant_face: true, ..OnField::default() });
            s.activate(&card, json!({}));
            s.answer(json!(must(offered_ids(&s).into_iter().next(), "an offered Trap")));
            assert!(must(s.backrow(P1, 2), "the summoned Trap").radiant);
        }
    }

    // -------------------------------------------------------------------------------------------
    // #493: each power's number is one the card declares (B3.4 rule 5, R386), so a Degrade, an
    // Upgrade or KY's Constant moves it; the engine reads it as `crate::config` prints it, moved by
    // the card's tuning (`subsystems::hero_power::power_number`).
    // -------------------------------------------------------------------------------------------

    mod n98_heroic_power_the_numbers_r386 {
        use super::*;

        /// #57 Conjure KY, a (2) Spell: a discount of 2 takes it to (0) where 1 leaves it at (1).
        const CONJURE_KY: &str = "core-057";

        fn tuned(s: &mut Scenario, key: &str, upgrade: bool) -> i32 {
            if upgrade {
                crate::upgrade_number(s, HEROIC, key)
            } else {
                crate::degrade_number(s, HEROIC, key)
            }
        }

        #[test]
        fn r386_the_catalog_prints_each_power_s_number_as_the_engine_reads_it_on_each_face() {
            crate::register_all();
            let printed: Vec<(String, i64, i64)> = def_json(HEROIC)["params"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|param| {
                    (
                        param["key"].as_str().unwrap_or_default().to_string(),
                        param["base"].as_i64().unwrap_or_default(),
                        param["radiant"].as_i64().unwrap_or_default(),
                    )
                })
                .collect();
            let both = |base: i32, radiant: i32| (i64::from(base), i64::from(radiant));
            for (key, (base, radiant)) in [
                (subsystems::LIFE_TAP_DRAW_PARAM, both(subsystems::LIFE_TAP_DRAW, subsystems::LIFE_TAP_DRAW)),
                (subsystems::LIFE_TAP_DAMAGE_PARAM, both(subsystems::LIFE_TAP_DAMAGE, subsystems::LIFE_TAP_DAMAGE)),
                (subsystems::PING_PARAM, both(subsystems::PING_DAMAGE, subsystems::PING_DAMAGE)),
                (subsystems::STITCHING_PARAM, both(subsystems::STITCHING_MAX_COST, subsystems::STITCHING_MAX_COST)),
                (subsystems::ARMOR_PARAM, both(subsystems::ARMOR_UP_ARMOR, subsystems::TANK_UP_ARMOR)),
                (subsystems::DIE_INSECT_PARAM, both(subsystems::DIE_INSECT_DAMAGE, subsystems::DIE_INSECT_DAMAGE)),
                (subsystems::BRAINSTORM_PARAM, both(subsystems::BRAINSTORM_DISCOUNT, subsystems::BRAINSTORM_DISCOUNT)),
                (subsystems::PLUCK_PARAM, both(subsystems::PLUCK_COST, subsystems::PLUCK_COST)),
            ] {
                assert!(printed.contains(&(key.to_string(), base, radiant)), "{key}");
            }
        }

        #[test]
        fn r386_life_tap_an_upgrade_draws_2_and_a_degrade_finds_the_draw_at_its_floor_of_1() {
            crate::register_all();
            let (mut s, card) = on_field(
                "draw",
                OnField {
                    p1: Some(json!({ "library": [MENACE, JAMMED], "hand": [SPARE], "mana": 8 })),
                    ..OnField::default()
                },
            );
            assert!(!crate::can_degrade_number(&s, HEROIC, "tapDraw"));
            assert_eq!(tuned(&mut s, "tapDraw", true), 2);
            s.activate(&card, json!({}));
            assert_eq!(hand_ids(&s), vec![SPARE.to_string(), MENACE.to_string(), JAMMED.to_string()]);
            s.expect_health(P1, 28);
        }

        #[test]
        fn r386_a_radiant_life_tap_upgraded_draws_2_from_each_player_s_deck() {
            crate::register_all();
            let (mut s, card) = on_field(
                "draw",
                OnField {
                    radiant_face: true,
                    p1: Some(json!({ "library": [MENACE, SPARE], "hand": [SPARE], "mana": 8 })),
                    p2: Some(json!({ "library": [JAMMED, FREE] })),
                    ..OnField::default()
                },
            );
            assert_eq!(tuned(&mut s, "tapDraw", true), 2);
            s.activate(&card, json!({}));
            assert_eq!(
                hand_ids(&s),
                [SPARE, MENACE, SPARE, JAMMED, FREE].map(str::to_string).to_vec(),
            );
            s.expect_health(P1, 30);
        }

        #[test]
        fn r386_life_tap_an_upgrade_takes_1_damage_and_a_degrade_3() {
            crate::register_all();
            for (upgrade, damage) in [(true, 1), (false, 3)] {
                let (mut s, card) = on_field(
                    "draw",
                    OnField {
                        p1: Some(json!({ "library": [MENACE, JAMMED], "hand": [SPARE], "mana": 8 })),
                        ..OnField::default()
                    },
                );
                assert_eq!(tuned(&mut s, "tapDamage", upgrade), damage);
                s.activate(&card, json!({}));
                s.expect_health(P1, 30 - damage);
            }
        }

        #[test]
        fn r386_ping_an_upgrade_deals_2_and_a_degrade_finds_it_at_its_floor_of_1() {
            crate::register_all();
            let (mut s, card) = on_field("ping", OnField::default());
            assert!(!crate::can_degrade_number(&s, HEROIC, "ping"));
            assert_eq!(tuned(&mut s, "ping", true), 2);
            s.activate(&card, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_health(P2, 28);
        }

        #[test]
        fn r386_stitching_a_degrade_offers_units_that_cost_1_or_less_and_an_upgrade_3_or_less() {
            crate::register_all();
            for (upgrade, max_cost) in [(false, 1), (true, 3)] {
                let (mut s, card) = on_field("stitching", OnField::default());
                assert_eq!(tuned(&mut s, "stitchCost", upgrade), max_cost);
                s.activate(&card, json!({}));
                for step in [1, 2] {
                    let pending = js(must(s.state().pending.as_ref(), "a discover prompt"));
                    assert_eq!(pending["prompt"], json!(format!("Discover a Unit that costs ({max_cost}) or less")));
                    let offered = offered_ids(&s);
                    assert_eq!(offered.len(), 3, "Discover {step}");
                    for id in &offered {
                        let cost = def_json(id)["cost"].clone();
                        assert!(cost.as_i64().is_some_and(|price| price <= i64::from(max_cost)), "{id}");
                    }
                    s.answer(json!(must(offered.first().cloned(), "an offered Unit")));
                }
                assert_eq!(events_of(&s, "fused").len(), 1);
            }
        }

        #[test]
        fn r386_armor_up_an_upgrade_gives_3_armor_and_a_degraded_tank_up_keeps_3() {
            crate::register_all();
            for (radiant_face, upgrade) in [(false, true), (true, false)] {
                let (mut s, card) = on_field("armor", OnField { radiant_face, ..turns() });
                assert_eq!(tuned(&mut s, "armor", upgrade), 3);
                s.activate(&card, json!({}));
                assert_eq!(view_json(&s, P1)["you"]["hero"]["armor"], 3);
            }
        }

        #[test]
        fn r386_die_insect_an_upgrade_deals_10_and_a_degrade_6() {
            crate::register_all();
            for (upgrade, damage) in [(true, 10), (false, 6)] {
                let (mut s, card) =
                    on_field("insect", OnField { p2: Some(json!({ "field": [MENACE] })), ..OnField::default() });
                assert_eq!(tuned(&mut s, "insect", upgrade), damage);
                s.activate(&card, json!({}));
                assert_eq!(
                    events_of(&s, "damage").iter().map(|event| event["amount"].clone()).collect::<Vec<_>>(),
                    vec![json!(damage)],
                );
            }
        }

        #[test]
        fn r386_ky_brainstorm_an_upgrade_takes_2_off_each_spell_and_a_degrade_finds_it_at_its_floor_of_1() {
            crate::register_all();
            let (mut s, card) = on_field(
                "brainstorm",
                OnField { p1: Some(json!({ "hand": [SPARE, CONJURE_KY], "mana": 8 })), ..OnField::default() },
            );
            assert!(!crate::can_degrade_number(&s, HEROIC, "discount"));
            assert_eq!(tuned(&mut s, "discount", true), 2);
            s.activate(&card, json!({}));
            let hand = s.hand(P1);
            let conjure = must(hand.iter().find(|entry| entry.def_id == CONJURE_KY).cloned(), "Conjure KY");
            assert_eq!(effective(&s, &conjure), 0);
            assert_eq!(effective(&s, &must(hand.iter().find(|entry| entry.def_id == SPARE).cloned(), "Reno")), 3);
        }

        #[test]
        fn r386_pluck_a_degrade_makes_the_fruit_cost_1_and_an_upgrade_finds_it_at_its_floor_of_0() {
            crate::register_all();
            let (mut s, card) = on_field("pluck", OnField::default());
            assert!(!crate::can_upgrade_number(&s, HEROIC, "fruitCost"));
            assert_eq!(tuned(&mut s, "fruitCost", false), 1);
            s.activate(&card, json!({}));
            let fruit = must(s.hand(P1).last().cloned(), "the Fruit");
            assert!(tags_of(&fruit.def_id).contains(&json!("Fruit")));
            assert_eq!(effective(&s, &fruit), 1);
        }
    }
}
