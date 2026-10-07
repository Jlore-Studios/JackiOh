//! C+ #74 Twice Forward One Step Backwards (SPEC §8.7 row 74, B7). (2) Field Trap, Mythic.
//!   Base:    "Every {plays|card|cards} your opponent plays, the last one is fused into this after it
//!            resolves, and this gains +{brittleGain} Brittle." (Brittle 2, balance patch 1)
//!   Radiant: "Every {plays|card|cards} your opponent plays, a Radiant copy of the last one is fused
//!            into this, and this gains +{brittleGain} Brittle." (Brittle 4)
//!   Engine:  "A Field Trap (R425): a Trap is consumed when it fires, which would leave nothing to gain
//!            Brittle. No Brittle while it is unrevealed: its printed Brittle never starts face-down
//!            (R687); it reveals the first time it activates, and the count starts then. It counts the
//!            opponent's plays since it was set (`memory.plays`; casts count, R70; a countered card was
//!            never played). On each even count, once that card has resolved (§10.5 step 7), the card, if
//!            it still exists (a Unit on the field, a Spell in the graveyard, a trap in the backrow), is
//!            fused into this (Fuse, §6.3, R77, R102): this is the kept instance and stays a Field Trap,
//!            and the opponent's card ceases to exist. The Radiant fuses in a Radiant copy of the card's
//!            definition instead and leaves the card where it is. Either way this then gains +1 Brittle,
//!            on every even count. The texts fused in work for you where they can … a fused Cry never
//!            runs, since this is already on the field. It turns face-up, public, the first time it
//!            activates (R33, R687); until then its play count is read by its controller only, and a card
//!            destroyed while unrevealed reads public in its graveyard (R97). Tunes: Brittle 2 ↑ (its
//!            X); every 2 ↓ (never below 2); Brittle gained 1 ↑."
//!
//! The whole card is its subsystem's trap trigger (`subsystems/twice_forward.rs`): the count on the
//! instance, the fuse after the play resolves, the reveal with the first activation and the Brittle
//! from then on. The printed Brittle is the catalog face's keyword (no start while face-down, R687;
//! B3.4's X change tunes it), and "every N" and the Brittle gained are its declared `params`.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-074";

/// SPEC §8.7 row 74: "until then its Brittle count and its play count are read by its controller only".
/// The play count rides the card's preview (R280), which a face-down card shows its controller alone
/// (R33); in play only, since the count starts as it is set.
fn preview() -> PreviewHook {
    condition_hook(|c: ConditionContext<'_>| {
        if c.zone == ConditionZone::Field {
            vec![PreviewValue {
                label: "your opponent plays".to_string(),
                value: subsystems::twice_forward_plays(c.self_),
                display: None,
                ids: None,
            }]
        } else {
            vec![]
        }
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            triggers: vec![subsystems::twice_forward_trigger(json_as(json!({ "radiantCopy": false })))],
            preview: Some(preview()),
            ..Script::default()
        },
        radiant: Script {
            triggers: vec![subsystems::twice_forward_trigger(json_as(json!({ "radiantCopy": true })))],
            preview: Some(preview()),
            ..Script::default()
        },
    }
}

// C+ #74 Twice Forward One Step Backwards — SPEC §8.7 row 74, BUILD M9 Classic+ row C+ 74: "A Field Trap
// (R425) set face-down with printed Brittle 2 and no count while unrevealed (R687); it counts the
// opponent's plays from then on (`memory.plays`; a cast counts, R70; a countered play doesn't) and on
// every second one, after that card resolves, the card, if it still exists (a Unit on the field, a Spell
// in the graveyard, a trap in the backrow), is fused into this (R77, R102): this is the kept instance and
// stays a Field Trap, the opponent's card ceases to exist; the first activation reveals it and starts
// its Brittle, then this gains +1 Brittle, even when there was no card left to fuse (revealing first);
// fused texts act for you where they can (an end-of-turn line or an aura on your side) and a fused Cry
// never runs; it turns face-up at its first activation (R33, R687), and until then the opponent's view
// shows neither the card nor its play count, while a card destroyed unrevealed reads public in its
// graveyard (R97); with no fuse it holds no Brittle and never crumbles; its memory and fused definition
// survive JSON and replay; Brittle, the every-2 step (never below 2) and Brittle gained read through
// `param()`; radiant Brittle 4, and a Radiant copy of the card is fused in, the original staying where
// it is".
//
// p1 sets the trap on its own turn (turn 9), then p2's turn begins and p2 plays. The machinery is proved
// through fixtures in `packages/engine/test/twiceForward.test.ts`.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const FORWARD: &str = "classicplus-074";
    const REFUSAL: &str = "classicplus-t-ai-09";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt; End of turn: heal this to full
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const RAPID: &str = "core-010"; // (0) Spell
    const TRUE_STRIKE: &str = "core-044"; // (1) Spell: Pierce. Deal 4 damage. Exile this.
    const BEAR: &str = "core-060"; // (1) Trap
    const SHREDDER: &str = "core-013"; // (3) Unit: End of turn: deal 2 damage to each enemy Unit and the enemy hero.
    const FELINORS: &str = "core-012"; // (2) Unit: Cry: Summon a copy of this.
    const TIGER_DOJO: &str = "core-014"; // (4) Field Spell: Aura: your Units have +4 attack, Rush and First Strike.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const HINDER: &str = "core-021"; // (0) Spell, cast on draw
    const TIMMY: &str = "core-011"; // (1) Unit
    const GUY_ATT: &str = "classicplus-005"; // (2) Unit: Cry: Destroy every backrow card you control.

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    fn deck(n: usize, card: &str) -> Vec<Value> {
        (0..n).map(|_| json!(card)).collect()
    }

    /// TS's `{ ...base, ...extra }` on a side setup.
    fn spread(base: Value, extra: &Value) -> Value {
        let mut out = base;
        if let (Some(into), Some(from)) = (out.as_object_mut(), extra.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    /// p1 sets the trap (lane 2) on turn 9 and ends its turn; p2's turn 10 begins, p2 with mana to spare.
    fn set_then_their_turn(radiant_face: bool, p1: Value, p2: Value, tune: impl FnOnce(&mut Scenario)) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": spread(json!({ "hand": [{ "def": FORWARD, "radiant": radiant_face }, VANILLA], "library": deck(10, TIMMY) }), &p1),
            "p2": spread(json!({ "hand": [STOCKPILE, RAPID, VANILLA, VANILLA], "library": deck(10, TIMMY) }), &p2),
        }));
        tune(&mut s);
        s.play(FORWARD, json!({ "zone": 2 })).end_turn();
        s.state_mut().players.p2.mana.current = 10;
        s
    }

    fn their_turn() -> Scenario {
        set_then_their_turn(false, json!({}), json!({}), |_| {})
    }

    fn trap_of(s: &Scenario) -> CardInstance {
        match s.backrow(P1, 2) {
            Some(card) => card,
            None => panic!("no trap in p1's backrow lane 2"),
        }
    }

    fn plays_of(s: &Scenario) -> i32 {
        subsystems::twice_forward_plays(&trap_of(s))
    }

    fn fused_events(s: &Scenario) -> usize {
        s.events().iter().filter(|event| event.event_type() == GameEventType::Fused).count()
    }

    fn ingredient_ids(s: &Scenario, def_id: &str) -> Option<Vec<String>> {
        def_of(Some(s.state()), def_id)
            .ingredients
            .as_ref()
            .map(|parts| parts.iter().map(|part| part.def_id.clone()).collect())
    }

    fn view_text(s: &Scenario, player: PlayerId) -> String {
        serde_json::to_string(&s.view(player)).expect("serialises")
    }

    fn id_or_empty(card: Option<CardInstance>) -> String {
        card.map(|card| card.id).unwrap_or_default()
    }

    mod c_n74_twice_forward_one_step_backwards {
        use super::*;

        #[test]
        fn is_a_2_mythic_field_trap_printing_brittle_2_radiant_4_declaring_every_2_min_2_and_brittle_gained_1() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.type_, CardType::FieldTrap);
            assert_eq!(js(&def.cost), json!(2));
            assert_eq!(def.rarity, Rarity::Mythic);
            assert_eq!(js(&def.base.keywords), json!([{ "kind": "Brittle", "n": 2 }]));
            assert_eq!(js(&def.radiant.keywords), json!([{ "kind": "Brittle", "n": 4 }]));
            assert_eq!(
                js(&def.params),
                json!([
                    { "key": "plays", "base": 2, "radiant": 2, "better": "down", "step": 1, "min": 2 },
                    { "key": "brittleGain", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 },
                ])
            );
            let scripts = super::super::script();
            assert_eq!(
                scripts.base.triggers.first().map(|trigger| trigger.on.clone()),
                Some(vec![GameEventType::CardPlayed, GameEventType::CardResolved])
            );
            assert!(scripts.radiant.triggers.first().is_some_and(|trigger| trigger.when.is_some()));
        }

        mod base {
            use super::*;

            #[test]
            fn r687_r425_set_face_down_it_holds_no_brittle_only_its_controller_reads_the_back() {
                let s = their_turn();
                let trap = trap_of(&s);
                assert!(trap.face_up != Some(true));
                assert_eq!(active_brittle_count(&trap), None);
                assert!(trap.brittle.is_none());
                assert!(js(&s.view(P1))["you"]["backrow"][1]["brittle"].is_null());
                let theirs = view_text(&s, P2);
                assert!(!theirs.contains(FORWARD));
                assert!(!theirs.contains(&trap.id));
            }

            #[test]
            fn r687_r97_destroyed_while_unrevealed_it_is_shown_to_the_opponent_their_view_names_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FORWARD, GUY_ATT], "backrow": [], "library": deck(10, TIMMY), "mana": 10 },
                    "p2": { "hand": [VANILLA], "library": deck(10, TIMMY) },
                }));
                s.play(FORWARD, json!({ "zone": 2 }));
                let trap = trap_of(&s);
                assert!(trap.face_up != Some(true));
                s.play(GUY_ATT, json!({ "zone": 1 }));
                s.expect_in_zone(&trap, "graveyard");
                let seen = view_text(&s, P2);
                assert!(seen.contains(&trap.id));
                assert!(seen.contains(FORWARD));
            }

            #[test]
            fn r425_r99_their_1st_play_is_counted_and_leaves_it_face_down_and_unfired() {
                let mut s = their_turn();
                s.play(RAPID, json!({}));
                assert_eq!(plays_of(&s), 1);
                assert!(trap_of(&s).face_up != Some(true));
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
            }

            #[test]
            fn r33_r687_until_it_activates_the_opponent_reads_neither_the_card_nor_its_play_count() {
                let mut s = their_turn();
                s.play(RAPID, json!({}));
                let trap = trap_of(&s);
                let seen = js(&s.view(P2))["opponent"]["backrow"][1].clone();
                assert!(!seen.is_null());
                let seen_text = seen.to_string();
                assert!(!seen_text.contains("brittle") && !seen_text.contains("plays") && !seen_text.contains("memory"));
                let theirs = view_text(&s, P2);
                assert!(!theirs.contains(&trap.id));
                assert!(!theirs.contains(FORWARD));
                assert!(js(&s.view(P1))["you"]["backrow"][1]["brittle"].is_null());
            }

            #[test]
            fn r425_r77_r687_the_2nd_a_unit_is_fused_into_this_after_it_resolves_still_a_field_trap_the_unit_gone_revealed_with_brittle_2_plus_1_face_up_r33()
             {
                let mut s = their_turn();
                s.play(RAPID, json!({}));
                let unit = s.card(VANILLA).clone();
                s.play(VANILLA, json!({ "zone": 1 }));
                let trap = trap_of(&s);
                assert!(s.unit(P2, 1).is_none());
                s.expect_in_zone(&unit, "gone");
                assert_eq!(def_of(Some(s.state()), &trap.def_id).type_, CardType::FieldTrap);
                assert_eq!(js(&trap.zone), json!({ "z": "field", "row": "backrow", "lane": 2, "player": "p1" }));
                assert_eq!(active_brittle_count(&trap), Some(3));
                assert_eq!(trap.face_up, Some(true));
                s.expect_events(json!(["cardResolved", "trapFired", "fused"]));
                assert!(view_text(&s, P2).contains(&trap.def_id));
            }

            #[test]
            fn r425_a_spell_is_fused_from_its_owners_graveyard() {
                let mut s = their_turn();
                s.play(RAPID, json!({})).play(STOCKPILE, json!({}));
                assert!(!s.pile(P2, "graveyard").iter().any(|card| card.def_id == STOCKPILE));
                assert_eq!(fused_events(&s), 1);
                assert_eq!(active_brittle_count(&trap_of(&s)), Some(3));
            }

            #[test]
            fn r425_a_trap_is_fused_from_its_owners_backrow() {
                let mut s = set_then_their_turn(false, json!({}), json!({ "hand": [RAPID, BEAR, VANILLA] }), |_| {});
                s.play(RAPID, json!({})).play(BEAR, json!({ "zone": 1 }));
                assert!(s.backrow(P2, 1).is_none());
                assert_eq!(fused_events(&s), 1);
            }

            #[test]
            fn r687_r589_r425_with_nothing_left_to_fuse_a_spell_that_exiles_itself_it_reveals_and_still_gains_plus_1_brittle_unfired()
             {
                let mut s =
                    set_then_their_turn(false, json!({}), json!({ "hand": [RAPID, TRUE_STRIKE, VANILLA] }), |_| {});
                s.play(RAPID, json!({}));
                s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                s.expect_in_zone(TRUE_STRIKE, "exile");
                assert_eq!(fused_events(&s), 0);
                // No Brittle sits on an unrevealed card: the gain reveals it first, then lands on the started 2.
                assert_eq!(active_brittle_count(&trap_of(&s)), Some(3));
                assert_eq!(trap_of(&s).face_up, Some(true));
                assert!(view_text(&s, P2).contains(&trap_of(&s).id));
            }

            #[test]
            fn r70_a_cast_is_a_play_the_cast_on_draw_card_their_turns_draw_casts_is_their_1st_so_the_next_play_is_fused() {
                let mut library = vec![json!({ "def": HINDER, "radiant": true })];
                library.extend(deck(6, TIMMY));
                let mut s = set_then_their_turn(
                    false,
                    json!({}),
                    json!({ "hand": [STOCKPILE, VANILLA], "library": library }),
                    |_| {},
                );
                assert_eq!(plays_of(&s), 1);
                let graveyard: Vec<String> = s.pile(P2, "graveyard").iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(graveyard, vec![HINDER]);
                s.play(STOCKPILE, json!({}));
                assert_eq!(plays_of(&s), 2);
                let fused: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Fused { def_id, .. } => Some(def_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(fused, vec![trap_of(&s).def_id]);
                assert_eq!(
                    ingredient_ids(&s, &trap_of(&s).def_id),
                    Some(vec![STOCKPILE.to_string(), FORWARD.to_string()])
                );
            }

            #[test]
            fn r70_r425_a_card_their_play_casts_is_the_later_play_the_hinder_stockpile_draws_is_their_2nd_fused_once_it_resolves()
             {
                let mut library = vec![json!(TIMMY), json!({ "def": HINDER, "radiant": true })];
                library.extend(deck(6, TIMMY));
                let mut s = set_then_their_turn(
                    false,
                    json!({}),
                    json!({ "hand": [STOCKPILE, VANILLA], "library": library }),
                    |_| {},
                );
                assert_eq!(plays_of(&s), 0);
                s.play(STOCKPILE, json!({}));
                let played: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::CardPlayed { player: PlayerId::P2, def_id, .. } => Some(def_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(played, vec![STOCKPILE, HINDER]);
                assert_eq!(plays_of(&s), 2);
                assert_eq!(
                    ingredient_ids(&s, &trap_of(&s).def_id),
                    Some(vec![HINDER.to_string(), FORWARD.to_string()])
                );
                let graveyard: Vec<String> = s.pile(P2, "graveyard").iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(graveyard, vec![STOCKPILE]);
            }

            #[test]
            fn r448_a_countered_play_is_never_played_and_doesnt_count() {
                let mut s = set_then_their_turn(
                    false,
                    json!({ "field": [MENACE], "backrow": [{ "def": REFUSAL, "faceUp": false, "lane": 4 }] }),
                    json!({ "hand": [HIT_JOB, RAPID, VANILLA] }),
                    |_| {},
                );
                let target = id_or_empty(s.unit(P1, 1));
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": target }] }));
                assert!(s.events().iter().any(|event| event.event_type() == GameEventType::Countered));
                assert_eq!(plays_of(&s), 0);
                s.play(RAPID, json!({}));
                assert_eq!(plays_of(&s), 1);
            }

            #[test]
            fn r425_its_own_controllers_plays_never_count() {
                let mut s = their_turn();
                s.end_turn();
                assert_eq!(s.state().active, P1);
                s.play(VANILLA, json!({ "zone": 1 }));
                assert_eq!(plays_of(&s), 0);
            }

            #[test]
            fn r102_a_fused_end_of_turn_line_runs_on_your_side_a_fused_shredder_hits_their_hero_and_units_at_your_end_of_turn() {
                let mut s =
                    set_then_their_turn(false, json!({}), json!({ "hand": [RAPID, SHREDDER, VANILLA] }), |_| {});
                s.play(VANILLA, json!({ "zone": 1 })).play(SHREDDER, json!({ "zone": 2 }));
                assert!(s.unit(P2, 2).is_none());
                s.end_turn();
                let health = s.state().players.p2.hero.health;
                s.end_turn();
                assert_eq!(s.state().players.p2.hero.health, health - 2);
                let unit = id_or_empty(s.unit(P2, 1));
                s.expect_stats(unit.as_str(), json!({ "health": 2 }));
            }

            #[test]
            fn r102_a_fused_aura_covers_your_side_a_fused_tiger_dojo_gives_your_units_plus_4_attack() {
                let mut s = set_then_their_turn(
                    false,
                    json!({ "field": [TIMMY] }),
                    json!({ "hand": [RAPID, TIGER_DOJO, VANILLA] }),
                    |_| {},
                );
                let before = s.stats(id_or_empty(s.unit(P1, 1)).as_str()).attack;
                s.play(RAPID, json!({})).play(TIGER_DOJO, json!({ "zone": 1 }));
                assert!(s.backrow(P2, 1).is_none());
                assert_eq!(s.stats(id_or_empty(s.unit(P1, 1)).as_str()).attack, before + 4);
            }

            #[test]
            fn r1_r102_a_fused_cry_never_runs_a_fused_duplicating_felinors_summons_nothing_on_your_side() {
                let mut s =
                    set_then_their_turn(false, json!({}), json!({ "hand": [RAPID, FELINORS, VANILLA] }), |_| {});
                s.play(RAPID, json!({})).play(FELINORS, json!({ "zone": 1 }));
                assert_eq!(fused_events(&s), 1);
                assert!(s.unit(P1, 1).is_none());
                // Its own Cry ran as it was played: the copy stays p2's.
                assert_eq!(s.unit(P2, 2).map(|unit| unit.def_id), Some(FELINORS.to_string()));
            }

            #[test]
            fn r425_every_second_play_fuses_again_plus_1_brittle_each_time() {
                let mut s = set_then_their_turn(
                    false,
                    json!({}),
                    json!({ "hand": [RAPID, RAPID, RAPID, RAPID, VANILLA] }),
                    |_| {},
                );
                for _ in 0..4 {
                    s.play(RAPID, json!({}));
                }
                assert_eq!(plays_of(&s), 4);
                assert_eq!(fused_events(&s), 2);
                assert_eq!(active_brittle_count(&trap_of(&s)), Some(4));
            }

            #[test]
            fn r687_with_no_fuse_it_holds_no_brittle_and_never_crumbles_turns_pass_with_no_count_and_no_crumble() {
                let mut s = set_then_their_turn(
                    false,
                    json!({ "library": deck(12, TIMMY) }),
                    json!({ "library": deck(12, TIMMY) }),
                    |_| {},
                );
                let trap = trap_of(&s);
                s.end_turn();
                assert_eq!(s.state().turn, 11);
                assert_eq!(active_brittle_count(&trap_of(&s)), None);
                s.end_turn().end_turn().end_turn().end_turn();
                assert_eq!(s.state().turn, 15);
                assert_eq!(active_brittle_count(&trap_of(&s)), None);
                s.end_turn().end_turn();
                assert_eq!(s.state().turn, 17);
                assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(trap.id.clone()));
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Crumbled));
            }

            #[test]
            fn r179_its_count_and_fused_definition_survive_json_and_the_round_trip_plays_on_exactly_as_the_live_game() {
                let mut s = set_then_their_turn(
                    false,
                    json!({}),
                    json!({ "hand": [RAPID, VANILLA, RAPID, VANILLA] }),
                    |_| {},
                );
                s.play(RAPID, json!({})).play(VANILLA, json!({ "zone": 1 }));
                let revived: GameState =
                    serde_json::from_str(&serde_json::to_string(s.state()).expect("serialises")).expect("parses");
                assert_eq!(&revived, s.state());
                assert!(revived.transient_defs.contains_key(&trap_of(&s).def_id));
                let next = s.hand(P2).into_iter().find(|card| card.def_id == RAPID);
                let next_id = next.as_ref().map(|card| card.id.clone()).unwrap_or_default();
                let resumed = reduce(
                    &revived,
                    &json_as::<Action>(json!({
                        "type": "play",
                        "playerId": "p2",
                        "instanceId": next_id,
                        "nonce": "forward-json",
                    })),
                );
                assert!(resumed.error.is_none());
                let next_ref = next.map(|card| card.id).unwrap_or_else(|| RAPID.to_string());
                s.play(next_ref.as_str(), json!({}));
                assert_eq!(hash_state(&resumed.state), hash_state(s.state()));
                assert_eq!(plays_of(&s), 3);
            }

            #[test]
            fn r386_every_n_reads_through_param_never_below_2_a_degrade_makes_it_3_and_brittle_gained_moves_with_it() {
                let mut floor = set_then_their_turn(false, json!({}), json!({}), |s| {
                    step_param(s.card_mut(FORWARD), "plays", -1);
                });
                floor.play(RAPID, json!({})).play(STOCKPILE, json!({}));
                assert_eq!(fused_events(&floor), 1);

                let mut slow =
                    set_then_their_turn(false, json!({}), json!({ "hand": [RAPID, RAPID, RAPID, VANILLA] }), |s| {
                        step_param(s.card_mut(FORWARD), "plays", 1);
                        step_param(s.card_mut(FORWARD), "brittleGain", 1);
                    });
                slow.play(RAPID, json!({})).play(RAPID, json!({}));
                assert_eq!(fused_events(&slow), 0);
                slow.play(RAPID, json!({}));
                assert_eq!(fused_events(&slow), 1);
                assert_eq!(active_brittle_count(&trap_of(&slow)), Some(4));
            }

            #[test]
            fn r687_r386_its_brittle_is_its_numbered_keyword_an_upgrades_x_change_starts_it_at_3_on_its_first_fuse_plus_1_gained()
             {
                let mut s = set_then_their_turn(false, json!({}), json!({}), |s0| {
                    let card = s0.card_mut(FORWARD);
                    let mut tuning = card.tuning.clone().unwrap_or_default();
                    let mut x = tuning.x.clone().unwrap_or_default();
                    x.insert("Brittle".to_string(), 1);
                    tuning.x = Some(x);
                    card.tuning = Some(tuning);
                });
                // Face-down it holds nothing, tuned or not; the first fuse reveals it and starts the tuned 3.
                assert_eq!(active_brittle_count(&trap_of(&s)), None);
                s.play(RAPID, json!({})).play(VANILLA, json!({ "zone": 1 }));
                assert_eq!(active_brittle_count(&trap_of(&s)), Some(4));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r687_brittle_4_set_face_down_with_no_count() {
                let s = set_then_their_turn(true, json!({}), json!({}), |_| {});
                assert_eq!(active_brittle_count(&trap_of(&s)), None);
                assert!(trap_of(&s).face_up != Some(true));
            }

            #[test]
            fn r469_a_radiant_copy_of_the_2nd_play_is_fused_in_the_original_staying_where_it_is() {
                let mut s = set_then_their_turn(true, json!({}), json!({}), |_| {});
                s.play(RAPID, json!({})).play(VANILLA, json!({ "zone": 1 }));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(VANILLA.to_string()));
                let trap = trap_of(&s);
                assert_ne!(trap.def_id, FORWARD);
                let fused = def_of(Some(s.state()), &trap.def_id);
                assert_eq!(fused.type_, CardType::FieldTrap);
                let part = fused
                    .ingredients
                    .as_ref()
                    .and_then(|parts| parts.iter().find(|part| part.def_id == VANILLA))
                    .map(js);
                assert_eq!(part, Some(json!({ "defId": VANILLA, "radiant": true })));
                assert_eq!(active_brittle_count(&trap), Some(5));
                assert_eq!(trap.face_up, Some(true));
            }

            #[test]
            fn r425_a_card_that_has_left_still_has_its_copy_fused_the_radiant_face_fuses_on_every_second_play() {
                let mut s =
                    set_then_their_turn(true, json!({}), json!({ "hand": [RAPID, TRUE_STRIKE, VANILLA] }), |_| {});
                s.play(RAPID, json!({}));
                s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                s.expect_in_zone(TRUE_STRIKE, "exile");
                assert_eq!(fused_events(&s), 1);
                assert_eq!(trap_of(&s).face_up, Some(true));
            }
        }
    }
}
