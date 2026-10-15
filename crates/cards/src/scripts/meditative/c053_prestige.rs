//! M #53 Prestige (SPEC §8.8 row 53): (1) Spell, Common.
//!
//! Base:    "De-Radiant a Radiant card on the field or in your hand. The next card you play this turn
//! becomes Radiant."
//! Radiant: "De-Radiant every enemy card on the field. The next card you play this turn becomes
//! Radiant."
//! Engine: MD-D6 — De-Radiant is Make Radiant reversed (`clearRadiant`), §5.2's exception: the face
//! swaps back while damage, buffs and tuning stay. The base face clears the chosen card, if any
//! (R90 already allows the play with nothing to choose, MD-D7): a Radiant Unit or face-up backrow card
//! on either side, or a Radiant card in your hand; the Radiant face clears every Radiant card of the
//! opponent's field (MD-D8). Both faces then add a this-turn `radiantNextPlay` modifier (R831005): the
//! next card its player plays, a cast included (R70), becomes Radiant as it is played (§10.5 step 3)
//! and spends it. Patch v0.3.9 (#831): both faces added AI generated cards instead (R1421), and the
//! Radiant face cleared the opponent's hand and deck too.

pub const ID: &str = "meditative-053";

// M #53 Prestige — SPEC §8.8 row 53, BUILD M10 row M 53: "De-Radiant a Radiant Unit or face-up
// backrow card on either side, or a Radiant card in your hand (MD-D6, R90); the Radiant face
// De-Radiants every enemy card of the opponent's field (MD-D8); both faces make the next card you play
// this turn Radiant, a cast included, once (R831005)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const PRESTIGE: &str = "meditative-053";
    const FILLER: &str = "core-005";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no text.

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1 holds Prestige (base unless `radiant_face`) with a Radiant vanilla Unit on each side's
    /// field; both sides keep cards in hand so no turn auto-ends.
    fn casting(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": PRESTIGE, "radiant": radiant_face }, FILLER],
                "field": [{ "def": VANILLA, "radiant": true }],
                "library": filler(4),
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": VANILLA, "radiant": true }],
                "library": filler(4),
            },
        }))
    }

    fn must_unit(s: &Scenario, seat: &str, lane: i32) -> CardInstance {
        s.unit(seat, lane).unwrap_or_else(|| panic!("{seat} lane {lane}"))
    }

    /// Mark the unit with 1 damage and +1 Attack: De-Radiant keeps both.
    fn mark(s: &mut Scenario, unit: &CardInstance) {
        let live = s.card_mut(&unit.id);
        live.damage = 1;
        live.buffs = AttackHealth { attack: 1, health: 0 };
    }

    /// Whether p1 holds a live `radiantNextPlay` modifier (R831005).
    fn next_play_radiant(s: &Scenario) -> bool {
        s.state().players.p1.mods.iter().any(|modifier| matches!(modifier.kind, ModifierKind::RadiantNextPlay))
    }

    /// Plays the first `def_id` in p1's hand (a Unit into its leftmost open zone) and says whether it
    /// arrived Radiant.
    fn play_one(s: &mut Scenario, def_id: &str) -> bool {
        let card = s.hand("p1").into_iter().find(|card| card.def_id == def_id).expect("the card in hand");
        s.play(&card.id, json!({}));
        s.card(&card.id).radiant
    }

    /// Runs `effects` as p1's on a hand-built sink at the state's rng cursor, settling after.
    fn by_hand(s: &mut Scenario, effects: Vec<Effect>) -> Vec<GameEvent> {
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(PlayerId::P1),
                        ..HookOptions::default()
                    },
                );
                apply_effects(&effects, &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
        events
    }

    mod m53_prestige {
        use super::*;

        #[test]
        fn deradiants_the_chosen_unit_keeping_damage_and_buffs_then_makes_the_next_play_radiant() {
            let mut s = casting("prestige-base", false);
            let unit = must_unit(&s, "p1", 1);
            mark(&mut s, &unit);
            s.play(
                PRESTIGE,
                json!({ "targets": [{ "pick": "instance", "instanceId": unit.id }] }),
            );
            let cleared = must_unit(&s, "p1", 1);
            assert!(!cleared.radiant);
            assert_eq!(cleared.damage, 1);
            assert_eq!(cleared.buffs, AttackHealth { attack: 1, health: 0 });
            // The other side's Radiant unit stands Radiant still.
            assert!(must_unit(&s, "p2", 1).radiant);
            assert!(next_play_radiant(&s));
        }

        #[test]
        fn deradiants_a_radiant_card_in_your_hand() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "prestige-hand",
                "p1": {
                    "hand": [{ "def": PRESTIGE }, { "def": VANILLA, "radiant": true }],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            let held = s
                .hand("p1")
                .into_iter()
                .find(|card| card.def_id == VANILLA)
                .expect("the Radiant hand card");
            assert!(held.radiant);
            s.play(
                PRESTIGE,
                json!({ "targets": [{ "pick": "instance", "instanceId": held.id }] }),
            );
            let cleared = s
                .hand("p1")
                .into_iter()
                .find(|card| card.id == held.id)
                .expect("still in hand");
            assert!(!cleared.radiant);
            assert!(next_play_radiant(&s));
        }

        #[test]
        fn r1103_with_no_radiant_card_the_play_is_legal_and_only_makes_the_next_play_radiant() {
            let mut s = casting("prestige-empty", false);
            // Nothing is Radiant anywhere: no target, but the play stays legal (R90).
            for unit in [must_unit(&s, "p1", 1), must_unit(&s, "p2", 1)] {
                s.card_mut(&unit.id).radiant = false;
            }
            s.play(PRESTIGE, json!({}));
            assert!(next_play_radiant(&s));
        }

        #[test]
        fn r831005_the_next_card_you_play_this_turn_becomes_radiant_and_only_that_one() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "prestige-next",
                "p1": { "mana": 10, "hand": [PRESTIGE, VANILLA, VANILLA, FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            s.play(PRESTIGE, json!({}));
            assert!(play_one(&mut s, VANILLA), "the next play is made Radiant");
            assert!(!next_play_radiant(&s), "and it spent the modifier");
            assert!(!play_one(&mut s, VANILLA), "the one after is not");
        }

        #[test]
        fn r831005_r70_a_cast_is_the_next_play_too() {
            let mut s = casting("prestige-cast", false);
            let unit = must_unit(&s, "p1", 1);
            s.play(PRESTIGE, pick(&unit.id));
            let events = by_hand(&mut s, vec![jackioh_engine::effects::cast_new(json_as(json!({ "def": FILLER })))]);
            assert!(events.iter().any(|event| matches!(event, GameEvent::RadiantSet { def_id, .. } if def_id == FILLER)));
            assert!(!next_play_radiant(&s));
        }

        #[test]
        fn r831005_unused_it_ends_with_the_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "prestige-expiry",
                "p1": { "mana": 10, "hand": [PRESTIGE, VANILLA, FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            s.play(PRESTIGE, json!({}));
            s.end_turn().end_turn();
            assert!(!next_play_radiant(&s));
            assert!(!play_one(&mut s, VANILLA));
        }

        #[test]
        fn r831005_your_view_names_the_modifier_on_a_badge() {
            let mut s = casting("prestige-badge", false);
            let unit = must_unit(&s, "p1", 1);
            s.play(PRESTIGE, pick(&unit.id));
            let mine = serde_json::to_string(&s.view(PlayerId::P1)).expect("the view serialises");
            assert!(mine.contains("The next card you play becomes Radiant"));
        }

        #[test]
        fn r1104_radiant_clears_the_enemy_field_only_not_their_hand_deck_or_graveyard() {
            let mut s = casting("prestige-radiant", true);
            // A Radiant card in the opponent's hand and library, and one in their graveyard.
            fn radiant_off_field(s: &mut Scenario, zone: Zone) -> CardInstance {
                let mut card = new_instance(s.state_mut(), VANILLA, PlayerId::P2, zone);
                card.radiant = true;
                card
            }
            let hand = radiant_off_field(&mut s, Zone::Hand { player: PlayerId::P2 });
            s.state_mut().players.p2.hand.push(hand);
            let deck = radiant_off_field(&mut s, Zone::Library { player: PlayerId::P2 });
            s.state_mut().players.p2.library.push(deck);
            let grave = radiant_off_field(&mut s, Zone::Graveyard { player: PlayerId::P2 });
            s.state_mut().players.p2.graveyard.push(grave);
            s.play(PRESTIGE, json!({}));
            // Every enemy card on the field is De-Radianted (patch v0.3.9: the field alone).
            assert!(!must_unit(&s, "p2", 1).radiant);
            assert!(s.hand("p2").iter().any(|card| card.radiant));
            assert!(s.pile("p2", "library").iter().any(|card| card.radiant));
            // Never the graveyard (MD-D8).
            assert!(s.pile("p2", "graveyard").iter().all(|card| card.radiant));
            // Your own Radiant unit stands Radiant still.
            assert!(must_unit(&s, "p1", 1).radiant);
            assert!(next_play_radiant(&s));
        }

        /// Me and Mr Token (Core #15): (1) Unit, "Cry: Summon a Rush Token" (Radiant: three).
        const MR_TOKEN: &str = "core-015";
        /// K-Pop Fanatic (Core #50): (1) 1/1 Unit with a Cry; Radiant 2/2 with Divine Shield.
        const KPOP: &str = "core-050";
        /// Anti-oneshot Armor (Core #73): a face-up Field Spell.
        const FIELD_SPELL: &str = "core-073";
        /// Sheepish (Core #41): a Trap, face-down.
        const TRAP: &str = "core-041";

        fn pick(id: &str) -> Value {
            json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
        }

        /// p1 holds Prestige and plenty of mana; p2's backrow holds a Radiant Field Spell and a
        /// Radiant face-down Trap, and its field a plain Unit.
        fn backrow_game(seed: &str) -> Scenario {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": seed,
                "p1": { "mana": 10, "hand": [PRESTIGE, FILLER], "library": filler(4) },
                "p2": {
                    "hand": [FILLER],
                    "field": [VANILLA],
                    "backrow": [{ "def": FIELD_SPELL, "lane": 1 }, { "def": TRAP, "lane": 2 }],
                    "library": filler(4),
                },
            }));
            for lane in [1, 2] {
                let id = s.backrow("p2", lane).expect("a backrow card").id;
                s.card_mut(&id).radiant = true;
            }
            s
        }

        #[test]
        fn targets_a_face_up_backrow_card_on_the_enemy_side() {
            let mut s = backrow_game("prestige-backrow");
            let field_spell = s.backrow("p2", 1).expect("the Field Spell");
            s.play(PRESTIGE, pick(&field_spell.id));
            assert!(!s.card(&field_spell.id).radiant);
            assert!(next_play_radiant(&s));
        }

        #[test]
        fn a_face_down_trap_or_a_card_that_is_not_radiant_is_refused() {
            let mut s = backrow_game("prestige-refused");
            let trap = s.backrow("p2", 2).expect("the Trap");
            let plain = must_unit(&s, "p2", 1);
            assert!(!plain.radiant);
            s.expect_refused(|s| s.play(PRESTIGE, pick(&trap.id)));
            s.expect_refused(|s| s.play(PRESTIGE, pick(&plain.id)));
            assert!(s.card(&trap.id).radiant, "the face-down Trap stays Radiant");
        }

        #[test]
        fn radiant_only_keywords_are_lost_and_no_cry_runs() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "prestige-kpop",
                "p1": { "mana": 10, "hand": [PRESTIGE, FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "field": [{ "def": KPOP, "radiant": true }], "library": filler(4) },
            }));
            let kpop = must_unit(&s, "p2", 1);
            let kinds = |s: &Scenario| -> Vec<KeywordKind> {
                keywords_of(s.state(), s.card(&kpop.id))
                    .iter()
                    .map(|keyword| keyword.kind())
                    .collect()
            };
            assert!(kinds(&s).contains(&KeywordKind::DivineShield));
            s.play(PRESTIGE, pick(&kpop.id));
            // Back to the base 1/1 with no Divine Shield, and its Cry did not run: no prompt.
            assert!(!kinds(&s).contains(&KeywordKind::DivineShield));
            s.expect_stats(&kpop.id, json!({ "attack": 1, "health": 1 }));
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn the_card_follows_its_base_text_from_then_on() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "prestige-base-text",
                "p1": {
                    "mana": 10,
                    "hand": [PRESTIGE, { "def": MR_TOKEN, "radiant": true }, FILLER, FILLER],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            let held = s
                .hand("p1")
                .into_iter()
                .find(|card| card.def_id == MR_TOKEN)
                .expect("Mr Token");
            s.play(PRESTIGE, pick(&held.id));
            // A Stockpile spends the next play's Radiant (R831005), so Mr Token is played as it stands.
            play_one(&mut s, FILLER);
            // Played now, it runs its base Cry: one Rush Token, not the Radiant face's three.
            s.play(&held.id, json!({ "zone": 1 }));
            let p1_units = (1..=UNIT_ZONES)
                .filter(|lane| s.unit("p1", *lane).is_some())
                .count();
            assert_eq!(p1_units, 2, "Mr Token and one Rush Token");
            assert!(!must_unit(&s, "p1", 1).radiant);
        }

        #[test]
        fn r440_deradianted_is_hidden_in_a_hand_and_public_on_the_field() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "prestige-hidden",
                "p1": {
                    "mana": 10,
                    "hand": [PRESTIGE, { "def": PRESTIGE }, { "def": VANILLA, "radiant": true }, FILLER],
                    "field": [{ "def": VANILLA, "radiant": true }],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            let deradianted = |s: &Scenario| -> Vec<Value> {
                let events = serde_json::to_value(s.view(PlayerId::P2).events).expect("events");
                events
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|event| event["type"] == json!("deradianted"))
                    .collect()
            };
            let held = s
                .hand("p1")
                .into_iter()
                .find(|card| card.def_id == VANILLA)
                .expect("held");
            s.play(PRESTIGE, pick(&held.id));
            let seen = deradianted(&s);
            assert_eq!(seen.len(), 1);
            assert_eq!(
                seen[0]["instanceId"],
                json!("hidden"),
                "a hand card's stays hidden"
            );
            // A Stockpile spends the next play's Radiant (R831005), so the second Prestige is base too.
            play_one(&mut s, FILLER);
            let unit = must_unit(&s, "p1", 1);
            s.play(PRESTIGE, pick(&unit.id));
            // The view keeps the earlier actions' events too: the new one comes last.
            let seen = deradianted(&s);
            assert_eq!(seen.len(), 2);
            assert_eq!(seen[1]["instanceId"], json!(unit.id), "a field card's is public");
        }

        #[test]
        fn r1104_radiant_leaves_the_opponents_exile_untouched() {
            let mut s = casting("prestige-exile", true);
            let mut card = new_instance(
                s.state_mut(),
                VANILLA,
                PlayerId::P2,
                Zone::Exile { player: PlayerId::P2 },
            );
            card.radiant = true;
            s.state_mut().players.p2.exile.push(card);
            s.play(PRESTIGE, json!({}));
            assert!(!must_unit(&s, "p2", 1).radiant);
            assert!(
                s.pile("p2", "exile").iter().all(|card| card.radiant),
                "the exile stays Radiant"
            );
        }

        #[test]
        fn r831005_the_radiant_face_makes_the_next_play_radiant_too() {
            let mut s = casting("prestige-radiant-next", true);
            s.play(PRESTIGE, json!({}));
            assert!(next_play_radiant(&s));
            assert!(play_one(&mut s, FILLER));
        }
    }
}
