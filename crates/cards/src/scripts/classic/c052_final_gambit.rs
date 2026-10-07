//! C #52 Final Gambit (SPEC §8.6 row 52; §4.4 step 4a, §6.3 Redirect; R18, R33, R44, R58, R125, R216,
//! R317, R386). Trap, cost 2, Epic.
//!   Base:    "Activates when a hit would bring your hero to 0 or less: Redirect the hit to the enemy
//!            hero. Then heal your hero {heal} and draw {draw}." (10, 3)
//!   Radiant: "… Then heal your hero {heal} and draw your deck." (20)
//!   Engine:  a replacement at "would take lethal damage" (after Armor, multipliers and caps; this hit
//!            alone, R44): the trap fires and the hit goes on to the enemy hero as a new instance from
//!            the same source, through their Armor, multipliers and caps; then the heal and the draws
//!            ("draw your deck" counts the deck as the step starts, R58). Fatigue counts; losing health
//!            does not (R18). Either player's turn.
//!
//! The replacement is data (`Script.replacements`); its `then` step is owed on `state.work` and runs
//! after the hit has landed — never, when that hit ended the game (R216). A play's or a combat's state
//! check usually ends the game first; where none runs before the step (owed work drained at the start
//! of a turn), the step itself finds the enemy hero at 0 or less and does nothing. That is what stops a
//! Final Gambit fused into a Field Trap that stays (C+ #74) from re-aiming its own fatigue for ever.

use jackioh_engine::effects::{draw, heal};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-052";

/// How many cards the follow-up draws: the declared number, or the deck as the step begins (R58).
type DrawCount = Arc<dyn Fn(&EffectContext<'_>) -> i32 + Send + Sync>;

fn final_gambit(draw_count: DrawCount) -> Script {
    Script {
        replacements: vec![ReplacementDef {
            id: "final-gambit".to_string(),
            on: ReplacementMoment::LethalHit,
            where_: None,
            when: None,
            instead: ReplacementInstead {
                redirect: Some(InsteadRedirect::EnemyHero),
                ..ReplacementInstead::default()
            },
            then: Some("afterRedirect".to_string()),
            by: None,
        }],
        resume: IndexMap::from([(
            "afterRedirect",
            hook(move |ctx| {
                // R216: the re-aimed hit left the enemy hero at 0 or less, so the game is over.
                let to = replacement_of(ctx).and_then(|record| record.redirected_to);
                if let Some(to) = to {
                    if hero_of(&ctx.state, to).health <= 0 {
                        return vec![];
                    }
                }
                let amount = param(&*ctx, "heal");
                let count = draw_count(&*ctx);
                vec![
                    heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": amount }))),
                    draw(json_as(json!({ "count": count }))),
                ]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = final_gambit(Arc::new(|ctx: &EffectContext<'_>| -> i32 { param(ctx, "draw") }));

    let radiant = final_gambit(Arc::new(|ctx: &EffectContext<'_>| -> i32 {
        zone_count(&ctx.state, ctx.controller, OffFieldZone::Library)
    }));

    CardScripts { base, radiant }
}

// C #52 Final Gambit — SPEC §8.6 row 52, BUILD M9 Classic row C 52: "Face-down (R33); fires at §4.4
// step 4a when one hit would leave your hero at 0 or less, judged after Armor, multipliers and caps,
// this hit alone (R44's reading); fatigue counts, while losing health (R18) and set health (C #29)
// never open it; a hit that isn't lethal leaves it set; the hit is re-aimed at the enemy hero as a new
// instance from the same source (`redirected`), through their Armor, multipliers and caps; then heal
// your hero 10 and draw 3; a redirected hit that kills the opponent ends the game at the state check,
// a draw if both heroes are at 0 (§2.5); radiant: heal 20 and draw your whole deck (R58), most of it
// burning (R317); its tuned numbers (heal, draw) read through `param()` (R386)".
//
// The hits come from Core cards with their own tests: attacks by Mr. Vanilla (4/4), Pointmaster (7/1)
// and Midrange Menace (9/9), Lunar Eclipse's 3, and fatigue on an empty deck. Going Long (Armor 2),
// C #75 Argusland (halved) and Anti-oneshot Armor (a cap of 5) stand on either hero's side. Blood
// Ridden Glowy Jelly Bean loses health on draw (R18), C #29 Book of Vital Kill sets a hero's health to
// 13, and Hinder, drawn by the follow-up, discards at random with no prompt (R682).
//
// "A draw if both heroes are at 0" (§2.5) needs a second hit inside the same effect, after the trap is
// spent: Prem Panther's "draw 2" into an empty deck, whose first fatigue the trap re-aims at a 1-health
// opponent and whose second then lands on you.
#[cfg(test)]
mod tests {
    use super::{script, ID};
    use jackioh_engine::testkit::*;

    const GAMBIT: &str = "classic-052";
    const ARGUSLAND: &str = "classic-075";
    const VITAL_KILL: &str = "classic-029"; // C #29 Book of Vital Kill: set a hero's health to 13.
    const VANILLA: &str = "core-008"; // 4/4
    const POINTMASTER: &str = "core-020"; // 7/1 First Strike
    const MENACE: &str = "core-019"; // 9/9 Taunt
    const PANTHER: &str = "core-032"; // 5/4 Rush: after this attacks and survives, draw 2 for each Unit that attack destroyed.
    const GARY: &str = "core-004"; // 1/1
    const LUNAR: &str = "core-035"; // (1) Spell: deal 3 damage to a target.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const BLOOD_BEAN: &str = "core-027"; // Cast on draw: make a random hand card Radiant. Lose 5 health.
    const HINDER: &str = "core-021"; // Cast on draw: their next refresh −1. Discard 1 at random (R682).
    const GOING_LONG: &str = "core-084"; // Field Spell: your hero has Armor 2.
    const ANTI_ONESHOT: &str = "core-073"; // Field Spell: your hero can't take more than 5 damage at once.
    const FILLER: &str = "core-005";
    const DECK: [&str; 5] = [FILLER, FILLER, FILLER, FILLER, FILLER];
    const FORWARD: &str = "classicplus-074"; // (2) Field Trap: every 2 cards the opponent plays, the last is fused into this.
    const RAPID: &str = "core-010"; // (0) Spell: Combo 3: draw 3.
    const TRUE_STRIKE: &str = "core-044"; // (1) Spell: Pierce. Deal 4 damage. Exile this.

    /// An engine value as the JSON the TS test reads (SURFACE §5.1: the same keys and values).
    fn js<T: serde::Serialize + ?Sized>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    fn hero_hits(s: &Scenario, player: PlayerId) -> Vec<i64> {
        let target = format!("hero-{}", js(&player).as_str().unwrap_or_default());
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "damage" && event["targetId"] == target)
            .filter_map(|event| event["amount"].as_i64())
            .collect()
    }

    fn fired(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(js).filter(|event| event["type"] == "trapFired").collect()
    }

    /// `opts[key]`, or `fallback` where the TS default (`??`) applies.
    fn or(opts: &Value, key: &str, fallback: Value) -> Value {
        if opts[key].is_null() { fallback } else { opts[key].clone() }
    }

    /// `base` with every key of `extra` laid over it, as a TS object spread (`{ ...base, ...extra }`).
    fn spread(base: Value, extra: &Value) -> Value {
        let mut out = base;
        if let (Some(fields), Some(over)) = (out.as_object_mut(), extra.as_object()) {
            for (key, value) in over {
                fields.insert(key.clone(), value.clone());
            }
        }
        out
    }

    /// p2 attacks p1's hero with `attacker`. p1 has `health`, a Final Gambit set face-down beside the rest
    /// of its backrow, a card in hand and a deck to draw from; p2 has whatever `p2` adds.
    /// `opts`: `attacker`, `health`, `gambit`, `p1Backrow`, `p1`, `p2`, as the TS helper's (a `Gambit` is a
    /// bare id or a `{ def, radiant? }` object).
    fn lethal_attack(opts: Value) -> Scenario {
        let attacker = opts["attacker"].as_str().expect("an attacker").to_string();
        let mut backrow = vec![spread(as_entry(&or(&opts, "gambit", json!(GAMBIT))), &json!({ "faceUp": false }))];
        backrow.extend(opts["p1Backrow"].as_array().cloned().unwrap_or_default());
        let p1 = spread(
            json!({ "hand": [FILLER], "backrow": backrow, "library": DECK, "health": opts["health"] }),
            &opts["p1"],
        );
        let p2 = spread(json!({ "hand": [FILLER], "field": [attacker], "library": DECK }), &opts["p2"]);
        let mut s = scenario(json!({ "p1": p1, "p2": p2, "active": "p2" }));
        s.attack(&attacker, "hero");
        s
    }

    fn as_entry(entry: &Value) -> Value {
        if entry.is_string() { json!({ "def": entry }) } else { entry.clone() }
    }

    mod c52_final_gambit {
        use super::*;

        #[test]
        fn declares_one_replacement_at_the_lethal_hit_window_re_aiming_the_hit_at_the_enemy_hero_then_its_follow_up() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["id"], GAMBIT);
            assert_eq!(def["type"], "Trap");
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                // TS `toEqual([{ id, on, instead, then }])`: a `ReplacementDef` holds a hook, so it is
                // compared field by field, the absent ones absent.
                assert_eq!(face.replacements.len(), 1);
                let only = &face.replacements[0];
                assert_eq!(only.id, "final-gambit");
                assert_eq!(only.on, ReplacementMoment::LethalHit);
                assert_eq!(js(&only.instead), json!({ "redirect": "enemyHero" }));
                assert_eq!(only.then.as_deref(), Some("afterRedirect"));
                assert!(only.where_.is_none() && only.when.is_none() && only.by.is_none());
                assert!(face.resume.contains_key("afterRedirect"));
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r33_it_is_set_face_down_the_opponents_view_shows_its_back_and_never_names_it() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [GAMBIT, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.play(GAMBIT, json!({}));
                let card = s.card(GAMBIT);
                assert_ne!(card.face_up, Some(true));
                let theirs = js(&s.view(PlayerId::P2));
                assert_eq!(theirs["opponent"]["backrow"][0]["faceDown"], true);
                assert!(!theirs.to_string().contains(GAMBIT));
                assert!(js(&s.view(PlayerId::P1)).to_string().contains(GAMBIT));
            }

            #[test]
            fn a_hit_that_is_not_lethal_leaves_it_set_face_down() {
                crate::register_all();
                let mut s = lethal_attack(json!({ "attacker": VANILLA, "health": 5 }));
                assert!(fired(&s).is_empty());
                s.expect_health(PlayerId::P1, 1);
                s.expect_in_zone(GAMBIT, "field");
                assert_ne!(s.card(GAMBIT).face_up, Some(true));
            }

            #[test]
            fn a_lethal_hit_fires_it_on_the_opponents_turn_re_aimed_at_the_enemy_hero_as_a_hit_from_the_same_source_then_heal_10_and_draw_3() {
                crate::register_all();
                let mut s = lethal_attack(json!({ "attacker": VANILLA, "health": 4 }));
                let gambit = s.card(GAMBIT).id.clone();
                let vanilla = s.card(VANILLA).id.clone();

                s.expect_events(json!(["attackDeclared", "trapFired", "redirected", "damage", "healed"]));
                assert_eq!(
                    s.events().iter().map(js).find(|event| event["type"] == "redirected"),
                    Some(json!({
                        "type": "redirected",
                        "what": "damage",
                        "fromId": "hero-p1",
                        "toId": "hero-p2",
                        "byInstanceId": gambit,
                    })),
                );
                let hit = s.events().iter().map(js).find(|event| event["type"] == "damage").unwrap_or_default();
                assert_eq!(hit["sourceId"], vanilla);
                assert_eq!(hit["targetId"], "hero-p2");
                assert_eq!(hit["amount"], 4);
                assert!(hero_hits(&s, PlayerId::P1).is_empty());
                s.expect_health(PlayerId::P2, 26);
                s.expect_health(PlayerId::P1, 14);
                assert_eq!(s.hand(PlayerId::P1).len(), 4);
                assert_eq!(s.pile(PlayerId::P1, "library").len(), 2);
                s.expect_in_zone(&gambit, "graveyard");
            }

            #[test]
            fn a_hit_that_leaves_the_hero_at_exactly_0_is_lethal() {
                crate::register_all();
                let mut s = lethal_attack(json!({ "attacker": POINTMASTER, "health": 7 }));
                assert_eq!(fired(&s).len(), 1);
                s.expect_health(PlayerId::P2, 23);
                s.expect_health(PlayerId::P1, 17);
            }

            #[test]
            fn r44_lethal_is_this_hit_alone_a_4_at_5_health_leaves_it_set_and_the_3_that_follows_at_1_fires_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [{ "def": GAMBIT, "faceUp": false }], "library": DECK, "health": 5 },
                    "p2": { "hand": [LUNAR, FILLER], "field": [VANILLA], "library": DECK },
                    "active": "p2",
                }));
                s.attack(VANILLA, "hero");
                assert!(fired(&s).is_empty());
                s.expect_health(PlayerId::P1, 1);

                s.play(LUNAR, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                assert_eq!(fired(&s).len(), 1);
                s.expect_health(PlayerId::P2, 27);
                s.expect_health(PlayerId::P1, 11);
            }

            #[test]
            fn s4_4_it_is_judged_after_armor_going_longs_2_off_a_7_is_a_5_lethal_at_5_not_at_6() {
                crate::register_all();
                let at5 = lethal_attack(json!({ "attacker": POINTMASTER, "health": 5, "p1Backrow": [GOING_LONG] }));
                assert_eq!(fired(&at5).len(), 1);
                let mut at6 = lethal_attack(json!({ "attacker": POINTMASTER, "health": 6, "p1Backrow": [GOING_LONG] }));
                assert!(fired(&at6).is_empty());
                at6.expect_health(PlayerId::P1, 1);
            }

            #[test]
            fn s4_4_it_is_judged_after_the_hero_damage_multipliers_argusland_halves_a_7_to_4_lethal_at_4_not_at_5() {
                crate::register_all();
                let at4 = lethal_attack(json!({ "attacker": POINTMASTER, "health": 4, "p1Backrow": [ARGUSLAND] }));
                assert_eq!(fired(&at4).len(), 1);
                let mut at5 = lethal_attack(json!({ "attacker": POINTMASTER, "health": 5, "p1Backrow": [ARGUSLAND] }));
                assert!(fired(&at5).is_empty());
                at5.expect_health(PlayerId::P1, 1);
            }

            #[test]
            fn s4_4_it_is_judged_after_the_hit_caps_anti_oneshot_armor_caps_a_9_at_5_lethal_at_5_not_at_6() {
                crate::register_all();
                let at5 = lethal_attack(json!({ "attacker": MENACE, "health": 5, "p1Backrow": [ANTI_ONESHOT] }));
                assert_eq!(fired(&at5).len(), 1);
                let mut at6 = lethal_attack(json!({ "attacker": MENACE, "health": 6, "p1Backrow": [ANTI_ONESHOT] }));
                assert!(fired(&at6).is_empty());
                at6.expect_health(PlayerId::P1, 1);
            }

            #[test]
            fn s6_3_redirect_the_re_aimed_hit_meets_the_enemy_heros_own_armor_and_multipliers_7_2_5_halved_to_3() {
                crate::register_all();
                let mut s = lethal_attack(json!({
                    "attacker": POINTMASTER,
                    "health": 4,
                    "p2": { "backrow": [GOING_LONG, ARGUSLAND] },
                }));
                assert_eq!(hero_hits(&s, PlayerId::P2), vec![3]);
                s.expect_health(PlayerId::P2, 27);
            }

            #[test]
            fn s6_3_redirect_and_the_enemy_heros_caps_anti_oneshot_armor_caps_the_re_aimed_9_at_5() {
                crate::register_all();
                let mut s = lethal_attack(json!({ "attacker": MENACE, "health": 4, "p2": { "backrow": [ANTI_ONESHOT] } }));
                assert_eq!(hero_hits(&s, PlayerId::P2), vec![5]);
                s.expect_health(PlayerId::P2, 25);
            }

            #[test]
            fn r125_fatigue_is_damage_and_counts_and_it_fires_on_its_controllers_own_turn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [{ "def": GAMBIT, "faceUp": false }], "health": 1 },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                s.start_turn();
                assert_eq!(fired(&s).len(), 1);
                // The 1st fatigue (1) went to p2's hero; then heal 10 and draw 3 into the empty deck: fatigue 2, 3, 4.
                assert_eq!(hero_hits(&s, PlayerId::P2), vec![1]);
                assert_eq!(hero_hits(&s, PlayerId::P1), vec![2, 3, 4]);
                s.expect_health(PlayerId::P2, 29);
                s.expect_health(PlayerId::P1, 2);
            }

            #[test]
            fn r18_losing_health_never_opens_it_the_jelly_beans_5_at_5_health_ends_the_game_with_the_trap_still_set() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [STOCKPILE, FILLER],
                        "backrow": [{ "def": GAMBIT, "faceUp": false }],
                        "library": [BLOOD_BEAN, FILLER, FILLER],
                        "health": 5,
                    },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(STOCKPILE, json!({}));
                assert!(fired(&s).is_empty());
                assert_eq!(js(&s.state().result)["winner"], "p2");
                s.expect_in_zone(GAMBIT, "field");
            }

            #[test]
            fn set_health_never_opens_it_c_29_sets_your_hero_to_13_and_the_trap_stays_set() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [{ "def": GAMBIT, "faceUp": false }], "library": DECK },
                    "p2": { "hand": [VITAL_KILL, FILLER] },
                    "active": "p2",
                }));
                s.play(VITAL_KILL, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                assert!(fired(&s).is_empty());
                s.expect_health(PlayerId::P1, 13);
                s.expect_in_zone(GAMBIT, "field");
            }

            #[test]
            fn a_hit_on_a_unit_never_opens_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FILLER],
                        "field": [VANILLA],
                        "backrow": [{ "def": GAMBIT, "faceUp": false }],
                        "library": DECK,
                        "health": 1,
                    },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                    "active": "p2",
                }));
                let vanilla = s.unit(PlayerId::P1, 1).expect("Mr. Vanilla should be on the board").id.clone();
                s.attack(MENACE, &vanilla);
                assert!(fired(&s).is_empty());
                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_in_zone(GAMBIT, "field");
            }

            #[test]
            fn s2_5_a_re_aimed_hit_that_kills_the_opponent_ends_the_game_at_the_state_check() {
                crate::register_all();
                let mut s = lethal_attack(json!({ "attacker": VANILLA, "health": 4, "p2": { "health": 4 } }));
                assert_eq!(fired(&s).len(), 1);
                assert_eq!(js(&s.state().result)["winner"], "p1");
                s.expect_health(PlayerId::P2, 0);
            }

            #[test]
            fn s2_5_a_draw_if_both_heroes_are_at_0_the_re_aimed_fatigue_kills_the_opponent_and_the_next_one_in_the_same_draw_kills_you() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [PANTHER], "backrow": [{ "def": GAMBIT, "faceUp": false }], "health": 1 },
                    "p2": { "hand": [FILLER], "field": [GARY], "health": 1 },
                }));
                let gary = s.card(GARY).id.clone();
                s.attack(PANTHER, &gary);
                assert_eq!(fired(&s).len(), 1);
                assert_eq!(hero_hits(&s, PlayerId::P2), vec![1]);
                assert_eq!(js(&s.state().result)["winner"], "draw");
                s.expect_health(PlayerId::P2, 0);
                assert!(s.state().players.p1.hero.health <= 0);
            }

            #[test]
            fn r216_the_game_ends_at_that_state_check_so_nothing_after_it_happens_the_follow_up_neither_heals_nor_draws() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [{ "def": GAMBIT, "faceUp": false }], "library": DECK, "health": 1 },
                    "p2": { "hand": [LUNAR, FILLER], "health": 3 },
                    "active": "p2",
                }));
                s.play(LUNAR, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                assert_eq!(js(&s.state().result)["winner"], "p1");
                assert_eq!(s.events().last().map(|event| js(event)["type"].clone()), Some(json!("gameOver")));
                s.expect_health(PlayerId::P1, 1);
                assert_eq!(s.hand(PlayerId::P1).len(), 1);
            }

            #[test]
            fn r216_fused_into_a_field_trap_that_stays_c_74_it_re_aims_its_own_fatigue_only_until_the_enemy_hero_falls_and_the_follow_ups_owed_after_that_do_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FORWARD, FILLER], "health": 4 },
                    "p2": { "hand": [RAPID, GAMBIT, TRUE_STRIKE], "library": DECK, "health": 40 },
                }));
                s.play(FORWARD, json!({ "zone": 2 }));
                s.end_turn();
                s.state_mut().players.p2.mana.current = 10;
                // p2's second play is fused into p1's Field Trap, which keeps Final Gambit's text and stays.
                s.play(RAPID, json!({}));
                s.play(GAMBIT, json!({}));
                assert!(s.backrow(PlayerId::P1, 2).is_some_and(|card| card.def_id.contains(GAMBIT)));
                s.state_mut().players.p1.fatigue_count = 20;
                // 4 at 4 health is lethal: re-aimed (4), then heal 10 and draw 3 from an empty deck. Each
                // fatigue, 21, 22 and 23, is lethal at 14 and re-aimed too; p2 falls at the second, so the
                // three follow-ups owed by then neither heal nor draw, and the drain ends at the state check.
                s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                assert_eq!(s.events().iter().map(js).filter(|event| event["type"] == "redirected").count(), 4);
                assert_eq!(hero_hits(&s, PlayerId::P2), vec![4, 21, 22, 23]);
                assert_eq!(js(&s.state().result)["winner"], "p1");
                s.expect_health(PlayerId::P1, 14);
            }

            #[test]
            fn a_second_final_gambit_finds_no_lethal_hit_once_the_first_has_re_aimed_it_and_stays_set() {
                crate::register_all();
                let mut s = lethal_attack(json!({ "attacker": VANILLA, "health": 4, "p1Backrow": [{ "def": GAMBIT }] }));
                assert_eq!(fired(&s).len(), 1);
                let (first, second) = (s.backrow(PlayerId::P1, 1), s.backrow(PlayerId::P1, 2));
                assert!(first.is_none());
                assert_eq!(second.as_ref().map(|card| card.def_id.as_str()), Some(GAMBIT));
                assert_ne!(second.as_ref().and_then(|card| card.face_up), Some(true));
                s.expect_health(PlayerId::P1, 14);
            }

            #[test]
            fn the_hand_cap_applies_to_its_draws_the_overflow_burns_r317() {
                crate::register_all();
                let s = lethal_attack(json!({
                    "attacker": VANILLA,
                    "health": 4,
                    "p1": { "hand": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] },
                }));
                assert_eq!(s.hand(PlayerId::P1).len(), 10);
                assert_eq!(s.events().iter().map(js).filter(|event| event["type"] == "burned").count(), 2);
            }

            #[test]
            fn its_draws_resolve_with_no_prompt_r682_and_the_follow_up_runs_to_the_same_game() {
                crate::register_all();
                let mut s = lethal_attack(json!({
                    "attacker": VANILLA,
                    "health": 4,
                    "p1": { "library": [HINDER, FILLER, FILLER, FILLER] },
                }));
                // Hinder's discard is random (R682): no prompt opens and the follow-up runs straight through.
                assert!(s.state().pending.is_none());
                s.expect_health(PlayerId::P1, 14);

                let thawed: GameState =
                    serde_json::from_str(&serde_json::to_string(s.state()).expect("a state serialises")).expect("a state parses");
                assert_eq!(&thawed, s.state());
                // The whole follow-up ran: Hinder's draw-again and the two draws after it.
                assert_eq!(s.state().players.p1.library.len(), 0);
            }

            #[test]
            fn r386_an_upgrade_heals_12_and_draws_4() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [{ "def": GAMBIT, "faceUp": false }], "library": DECK, "health": 4 },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                    "active": "p2",
                }));
                let gambit = s.card(GAMBIT).id.clone();
                step_param(s.card_mut(&gambit), "heal", 1);
                step_param(s.card_mut(&gambit), "draw", 1);
                s.attack(VANILLA, "hero");
                s.expect_health(PlayerId::P1, 16);
                assert_eq!(s.hand(PlayerId::P1).len(), 5);
            }

            #[test]
            fn r386_a_degrade_heals_8_and_draws_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "backrow": [{ "def": GAMBIT, "faceUp": false }], "library": DECK, "health": 4 },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                    "active": "p2",
                }));
                let gambit = s.card(GAMBIT).id.clone();
                step_param(s.card_mut(&gambit), "heal", -1);
                step_param(s.card_mut(&gambit), "draw", -1);
                s.attack(VANILLA, "hero");
                s.expect_health(PlayerId::P1, 12);
                assert_eq!(s.hand(PlayerId::P1).len(), 3);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r33_it_is_set_face_down_like_the_base_face() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": GAMBIT, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(GAMBIT, json!({}));
                assert!(!js(&s.view(PlayerId::P2)).to_string().contains(GAMBIT));
            }

            #[test]
            fn re_aims_a_lethal_hit_at_the_enemy_hero_then_heals_20() {
                crate::register_all();
                let mut s = lethal_attack(json!({ "attacker": VANILLA, "health": 4, "gambit": { "def": GAMBIT, "radiant": true } }));
                assert_eq!(fired(&s).len(), 1);
                s.expect_health(PlayerId::P2, 26);
                s.expect_health(PlayerId::P1, 24);
            }

            #[test]
            fn r58_draws_your_deck_its_size_as_the_step_begins_and_r317_most_of_it_burns_at_the_hand_cap() {
                crate::register_all();
                let library: Vec<&str> = (0..15).map(|_| FILLER).collect();
                let mut s = lethal_attack(json!({
                    "attacker": VANILLA,
                    "health": 4,
                    "gambit": { "def": GAMBIT, "radiant": true },
                    "p1": { "hand": [FILLER, FILLER], "library": library },
                }));
                assert_eq!(s.pile(PlayerId::P1, "library").len(), 0);
                assert_eq!(s.hand(PlayerId::P1).len(), 10);
                assert_eq!(s.events().iter().map(js).filter(|event| event["type"] == "burned").count(), 7);
                // Exactly the deck: no draw past its end, so no fatigue.
                assert!(hero_hits(&s, PlayerId::P1).is_empty());
                s.expect_health(PlayerId::P1, 24);
            }

            #[test]
            fn r58_an_empty_deck_draws_nothing_so_no_fatigue() {
                crate::register_all();
                let mut s = lethal_attack(json!({
                    "attacker": VANILLA,
                    "health": 4,
                    "gambit": { "def": GAMBIT, "radiant": true },
                    "p1": { "library": [] },
                }));
                assert!(hero_hits(&s, PlayerId::P1).is_empty());
                s.expect_health(PlayerId::P1, 24);
            }

            #[test]
            fn s4_4_judged_after_armor_multipliers_and_caps_like_the_base_face() {
                crate::register_all();
                let mut s = lethal_attack(json!({
                    "attacker": MENACE,
                    "health": 6,
                    "gambit": { "def": GAMBIT, "radiant": true },
                    "p1Backrow": [ANTI_ONESHOT],
                }));
                assert!(fired(&s).is_empty());
                s.expect_health(PlayerId::P1, 1);
            }

            #[test]
            fn r18_losing_health_never_opens_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [STOCKPILE, FILLER],
                        "backrow": [{ "def": GAMBIT, "radiant": true, "faceUp": false }],
                        "library": [BLOOD_BEAN, FILLER, FILLER],
                        "health": 5,
                    },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(STOCKPILE, json!({}));
                assert!(fired(&s).is_empty());
                assert_eq!(js(&s.state().result)["winner"], "p2");
            }

            #[test]
            fn r386_a_degrade_heals_18() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": GAMBIT, "radiant": true, "faceUp": false }],
                        "library": DECK,
                        "health": 4,
                    },
                    "p2": { "hand": [FILLER], "field": [VANILLA] },
                    "active": "p2",
                }));
                let gambit = s.card(GAMBIT).id.clone();
                step_param(s.card_mut(&gambit), "heal", -1);
                s.attack(VANILLA, "hero");
                s.expect_health(PlayerId::P1, 22);
            }
        }
    }
}
