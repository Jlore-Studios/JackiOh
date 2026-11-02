//! M #53 Prestige (SPEC §8.8 row 53): (1) Spell, Common.
//!
//! Base:    "Tradable / De-Radiant a Radiant card on the field or in your hand. The next card you play
//! this turn becomes Radiant."
//! Radiant: "Tradable / De-Radiant every enemy card on the field. The next card you play this turn
//! becomes Radiant."
//! Engine: MD-D6 — De-Radiant is Make Radiant reversed (`clearRadiant`), §5.2's exception: the face
//! swaps back while damage, buffs and tuning stay. Both faces then add a this-turn `radiantNextPlay`
//! modifier (R831005): the next card its player plays, a cast included (R70), becomes Radiant as it
//! is played (§10.5 step 3) and spends it. Both faces are Tradable (R1838000), catalog data the
//! engine reads in the hand (`trade.rs`), so nothing here runs for it.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-053";

/// The base face's pick: a Radiant Unit or face-up backrow card on either side, or a Radiant card
/// in your hand. R1851003: the play needs it (R703), unless `needed` is off (through v0.4.11).
fn targets(needed: bool) -> Vec<TargetDecl> {
    let mut pick = TargetDecl::target(
        1,
        1,
        json!({ "side": "any", "of": ["unit", "backrow", "hand"], "check": "radiant" }),
    );
    if needed {
        pick.required = Some(true);
    }
    vec![pick]
}

/// A face-down backrow card: a Trap or Field Trap (a card set face-down reads as one) neither
/// face-up nor revealed. Hidden information never shows (R440), so whether it is Radiant is not
/// for its opponent to read.
fn face_down(state: &GameState, card: &CardInstance) -> bool {
    matches!(
        card.zone,
        Zone::Field {
            row: Row::Backrow,
            ..
        }
    ) && matches!(
        card_type_of(state, card),
        CardType::Trap | CardType::FieldTrap
    ) && card.face_up != Some(true)
        && card.revealed != Some(true)
}

fn target_checks() -> IndexMap<&'static str, TargetCheck> {
    IndexMap::from([(
        "radiant",
        // A Radiant card that is not a face-down backrow card: a face-down card is not a "face-up
        // backrow card".
        target_check(|a| {
            a.candidate
                .is_some_and(|candidate| candidate.radiant && !face_down(a.state, candidate))
        }),
    )])
}

/// R831005 (#831): "The next card you play this turn becomes Radiant", a this-turn modifier its
/// next play spends (it added AI generated cards before, R1421).
fn next_play(ctx: &EffectContext<'_>) -> Effect {
    add_player_modifier(json_as(json!({
        "mod": {
            "kind": "radiantNextPlay",
            "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
        },
    })))
}

/// MD-D8: every Radiant card of `player`'s opponent's field, face-down ones included.
fn enemy_radiant_cards(state: &GameState, player: PlayerId) -> Vec<&CardInstance> {
    let enemy = opponent_of(player);
    permanents_on_field(state, enemy)
        .into_iter()
        .filter(|card| card.controller == enemy && card.radiant)
        .collect()
}

fn prestige(needed: bool) -> Script {
    Script {
        targets: targets(needed),
        target_checks: target_checks(),
        cry: Some(hook(|ctx| vec![clear_radiant(RadiantTarget::default()), next_play(ctx)])),
        ..Script::default()
    }
}

fn prestige_radiant(gated: bool) -> Script {
    Script {
        // R1851003: a Radiant enemy card the Cry would clear, standing where its player can see it.
        play_allowed: gated.then(|| {
            play_allowed_hook(|state, player| {
                enemy_radiant_cards(state, player)
                    .into_iter()
                    .any(|card| !face_down(state, card))
            })
        }),
        cry: Some(hook(|ctx| {
            let mut effects: Vec<Effect> = enemy_radiant_cards(ctx.state, ctx.controller)
                .into_iter()
                .map(|card| clear_radiant(json_as(json!({ "instanceId": card.id }))))
                .collect();
            effects.push(next_play(ctx));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: prestige(true),
        radiant: prestige_radiant(true),
    }
}

/// R767000: through v0.4.11 either face could be played with nothing to De-Radiant (R90, R1103);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_4_11() -> CardScripts {
    CardScripts {
        base: prestige(false),
        radiant: prestige_radiant(false),
    }
}

// M #53 Prestige — SPEC §8.8 row 53, BUILD M10 row M 53.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filler;
    use crate::js;
    use jackioh_engine::testkit::*;

    const PRESTIGE: &str = "meditative-053";
    const FILLER: &str = "core-005";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no text.

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

    /// Anti-oneshot Armor (Core #73): a face-up Field Spell.
    const FIELD_SPELL: &str = "core-073";
    /// Sheepish (Core #41): a Trap, face-down.
    const TRAP: &str = "core-041";

    fn pick(id: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
    }

    /// p1 holds Prestige (base unless `radiant_face`) and plenty of mana; p2's backrow holds a
    /// Radiant Field Spell and a Radiant face-down Trap, and its field a plain Unit.
    fn backrow_game(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 10,
                "hand": [{ "def": PRESTIGE, "radiant": radiant_face }, FILLER],
                "library": filler(4),
            },
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
        fn r831005_the_next_card_you_play_this_turn_becomes_radiant_and_only_that_one() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "prestige-next",
                "p1": { "mana": 10, "hand": [PRESTIGE, VANILLA, VANILLA, FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "radiant": true }], "library": filler(4) },
            }));
            let enemy = must_unit(&s, "p2", 1);
            s.play(PRESTIGE, pick(&enemy.id));
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
                "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "radiant": true }], "library": filler(4) },
            }));
            let enemy = must_unit(&s, "p2", 1);
            s.play(PRESTIGE, pick(&enemy.id));
            s.end_turn().end_turn();
            assert!(!next_play_radiant(&s));
            assert!(!play_one(&mut s, VANILLA));
        }

        #[test]
        fn r831005_your_view_names_the_modifier_on_a_badge() {
            let mut s = casting("prestige-badge", false);
            let unit = must_unit(&s, "p1", 1);
            s.play(PRESTIGE, pick(&unit.id));
            let mine = serde_json::to_string(&s.view(PlayerId::P1)).expect("the view serializes");
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
            assert!(!must_unit(&s, "p2", 1).radiant);
            assert!(s.hand("p2").iter().any(|card| card.radiant));
            assert!(s.pile("p2", "library").iter().any(|card| card.radiant));
            // Never the graveyard (MD-D8).
            assert!(s.pile("p2", "graveyard").iter().all(|card| card.radiant));
            assert!(must_unit(&s, "p1", 1).radiant);
            assert!(next_play_radiant(&s));
        }

        /// Me and Mr Token (Core #15): (1) Unit, "Cry: Summon a Rush Token" (Radiant: three).
        const MR_TOKEN: &str = "core-015";
        /// K-Pop Fanatic (Core #50): (1) 1/1 Unit with a Cry; Radiant 2/2 with Divine Shield.
        const KPOP: &str = "core-050";
        #[test]
        fn targets_a_face_up_backrow_card_on_the_enemy_side() {
            let mut s = backrow_game("prestige-backrow", false);
            let field_spell = s.backrow("p2", 1).expect("the Field Spell");
            s.play(PRESTIGE, pick(&field_spell.id));
            assert!(!s.card(&field_spell.id).radiant);
            assert!(next_play_radiant(&s));
        }

        #[test]
        fn a_face_down_trap_or_a_card_that_is_not_radiant_is_refused() {
            let mut s = backrow_game("prestige-refused", false);
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
                let events = js(&s.view(PlayerId::P2).events);
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

    /// R1838000: Prestige is Tradable on both faces — Traded from the hand into the deck for a draw,
    /// at no mana, one Trade a turn — and a Trade is no play of it (R1838002).
    mod r1838000_tradable {
        use super::*;

        #[test]
        fn r1838000_both_faces_trade_into_the_deck_for_a_draw_at_no_mana() {
            for radiant in [false, true] {
                let mut s = casting("prestige-trade", radiant);
                let prestige = s.card(PRESTIGE).clone();
                assert!(is_tradable(s.state(), &prestige), "radiant {radiant}: the face prints Tradable");
                let (hand, deck, mana) = (
                    s.hand("p1").len(),
                    s.pile("p1", "library").len(),
                    s.state().players.p1.mana.current,
                );
                s.trade(&prestige.id);
                s.expect_events(json!(["traded", "shuffledIn", "drawn"]));
                assert_eq!(s.hand("p1").len(), hand, "one card went in and one came out");
                assert_eq!(s.pile("p1", "library").len(), deck);
                assert_eq!(s.state().players.p1.mana.current, mana, "at no mana");
                assert_eq!(s.card(&prestige.id).radiant, radiant, "the card keeps its face");
                assert_eq!(trades_left(s.state(), PlayerId::P1), 0, "the turn's one Trade is made");
            }
        }

        #[test]
        fn r1838002_a_trade_is_no_play_nothing_is_cleared_and_no_next_play_is_made_radiant() {
            for radiant in [false, true] {
                let mut s = casting("prestige-trade-play", radiant);
                let prestige = s.card(PRESTIGE).clone();
                s.trade(&prestige.id);
                assert!(!next_play_radiant(&s), "radiant {radiant}: no Cry ran");
                assert!(must_unit(&s, "p1", 1).radiant && must_unit(&s, "p2", 1).radiant);
                assert_eq!(s.state().players.p1.turn_log.cards_played, 0);
            }
        }
    }

    /// R1851003 (#1851): either face is offered, and its play accepted, only with a card to
    /// De-Radiant: the base face's pick is needed (R703), and the Radiant face's play gate asks for a
    /// Radiant enemy card on the field its player can see (R1851000). A cast is never refused (R70,
    /// R1851001), and a Trade is no play (R1838002).
    mod r1851003_only_with_a_card_to_deradiant {
        use super::*;

        /// R703's refusal, the base face's.
        const NEEDS_A_TARGET: &str = "Prestige cannot be played without 1 legal target";
        /// R1851000's refusal, the Radiant face's.
        const NOTHING_TO_ACT_ON: &str = "Prestige cannot be played without something to act on";

        /// `casting` with nothing Radiant anywhere: both Units plain.
        fn bare(seed: &str, radiant_face: bool) -> Scenario {
            let mut s = casting(seed, radiant_face);
            for unit in [must_unit(&s, "p1", 1), must_unit(&s, "p2", 1)] {
                s.card_mut(&unit.id).radiant = false;
            }
            s
        }

        /// Whether `legal_actions` offers p1 a play of a Prestige in their hand.
        fn offered(s: &Scenario) -> bool {
            let held: Vec<String> = s
                .hand("p1")
                .into_iter()
                .filter(|card| card.def_id == PRESTIGE)
                .map(|card| card.id)
                .collect();
            legal_actions(s.state(), PlayerId::P1)
                .iter()
                .any(|action| matches!(action, ActionBody::Play { instance_id, .. } if held.contains(instance_id)))
        }

        /// Whether `legal_actions` offers p1 a Trade of the first Prestige in their hand.
        fn trade_offered(s: &Scenario) -> bool {
            let prestige = s.card(PRESTIGE).id.clone();
            legal_actions(s.state(), PlayerId::P1)
                .iter()
                .any(|action| matches!(action, ActionBody::Trade { instance_id } if *instance_id == prestige))
        }

        /// Why p1's own view says the first Prestige in their hand cannot be played (`playReason`).
        fn play_reason(s: &Scenario) -> Option<String> {
            let prestige = s.card(PRESTIGE).id.clone();
            let HandView::Cards(cards) = s.view(PlayerId::P1).you.hand else {
                panic!("the viewer's own hand is a list");
            };
            cards
                .into_iter()
                .find(|card| card.instance_id == prestige)
                .and_then(|card| card.play_reason)
        }

        #[test]
        fn r1103_r1851003_with_no_radiant_card_anywhere_the_base_face_is_not_offered_and_its_play_is_refused() {
            let mut s = bare("prestige-empty", false);
            assert!(!offered(&s));
            assert_eq!(play_reason(&s).as_deref(), Some(NEEDS_A_TARGET));
            s.expect_refused_with(|s| s.play(PRESTIGE, json!({})), NEEDS_A_TARGET);
            assert!(!next_play_radiant(&s), "nothing resolved");
            assert!(s.hand("p1").iter().any(|card| card.def_id == PRESTIGE), "it stays in hand");
        }

        #[test]
        fn r1851003_the_base_face_is_offered_once_a_radiant_card_is_on_either_field_or_in_your_hand() {
            for seat in ["p1", "p2"] {
                let mut s = bare("prestige-offered-field", false);
                let unit = must_unit(&s, seat, 1);
                s.card_mut(&unit.id).radiant = true;
                assert!(offered(&s), "a Radiant Unit on {seat}'s field");
                assert_eq!(play_reason(&s), None);
                s.play(PRESTIGE, pick(&unit.id));
                assert!(!s.card(&unit.id).radiant);
            }
            crate::register_all();
            let s = scenario(json!({
                "seed": "prestige-offered-hand",
                "p1": {
                    "hand": [PRESTIGE, { "def": VANILLA, "radiant": true }, FILLER],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            assert!(offered(&s), "a Radiant card in your own hand");
        }

        #[test]
        fn r1851003_a_face_down_trap_or_a_card_in_the_opponents_hand_is_no_candidate_for_the_base_face() {
            // Only p2's face-down Trap is Radiant: its face is hidden (R440), so no pick exists.
            let mut s = backrow_game("prestige-face-down-base", false);
            let field_spell = s.backrow("p2", 1).expect("the Field Spell").id;
            s.card_mut(&field_spell).radiant = false;
            assert!(!offered(&s));
            assert_eq!(play_reason(&s).as_deref(), Some(NEEDS_A_TARGET));
            s.expect_refused_with(|s| s.play(PRESTIGE, json!({})), NEEDS_A_TARGET);
            // A Radiant card in the opponent's hand is no hand pick of yours (§9.1).
            crate::register_all();
            let s = scenario(json!({
                "seed": "prestige-their-hand",
                "p1": { "hand": [PRESTIGE, FILLER], "library": filler(4) },
                "p2": { "hand": [{ "def": VANILLA, "radiant": true }], "library": filler(4) },
            }));
            assert!(!offered(&s));
        }

        #[test]
        fn r1851003_the_radiant_face_with_no_radiant_enemy_card_on_the_field_is_not_offered_your_own_counting_for_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "prestige-radiant-empty",
                "p1": {
                    "hand": [{ "def": PRESTIGE, "radiant": true }, { "def": VANILLA, "radiant": true }, FILLER],
                    "field": [{ "def": VANILLA, "radiant": true }],
                    "library": filler(4),
                },
                "p2": { "hand": [{ "def": VANILLA, "radiant": true }], "field": [VANILLA], "library": filler(4) },
            }));
            assert!(!offered(&s));
            assert_eq!(play_reason(&s).as_deref(), Some(NOTHING_TO_ACT_ON));
            s.expect_refused_with(|s| s.play(PRESTIGE, json!({})), NOTHING_TO_ACT_ON);
            assert!(must_unit(&s, "p1", 1).radiant, "your own Radiant Unit stays Radiant");
            assert!(!next_play_radiant(&s), "nothing resolved");
        }

        #[test]
        fn r1851003_the_radiant_face_is_offered_once_a_radiant_enemy_card_stands_on_the_field() {
            let mut s = bare("prestige-radiant-unit", true);
            let enemy = must_unit(&s, "p2", 1);
            s.card_mut(&enemy.id).radiant = true;
            assert!(offered(&s), "a Radiant enemy Unit");
            assert_eq!(play_reason(&s), None);
            s.play(PRESTIGE, json!({}));
            assert!(!s.card(&enemy.id).radiant);
            assert!(next_play_radiant(&s));
            // A Radiant enemy Field Spell, face-up in the backrow.
            let mut s = backrow_game("prestige-radiant-backrow", true);
            let trap = s.backrow("p2", 2).expect("the Trap").id;
            s.card_mut(&trap).radiant = false;
            assert!(offered(&s), "a Radiant enemy Field Spell");
        }

        #[test]
        fn r1851003_a_face_down_radiant_enemy_card_does_not_open_the_radiant_face_though_its_cry_clears_it() {
            let mut s = backrow_game("prestige-radiant-face-down", true);
            let field_spell = s.backrow("p2", 1).expect("the Field Spell").id;
            let trap = s.backrow("p2", 2).expect("the Trap").id;
            // Only the face-down Trap is Radiant, and p1 cannot see its face (§9.1).
            s.card_mut(&field_spell).radiant = false;
            assert!(!offered(&s));
            assert_eq!(play_reason(&s).as_deref(), Some(NOTHING_TO_ACT_ON));
            s.expect_refused_with(|s| s.play(PRESTIGE, json!({})), NOTHING_TO_ACT_ON);
            // A Radiant enemy card p1 can see opens it, and the Cry clears the face-down one too (MD-D8).
            s.card_mut(&field_spell).radiant = true;
            assert!(offered(&s));
            s.play(PRESTIGE, json!({}));
            assert!(!s.card(&field_spell).radiant);
            assert!(!s.card(&trap).radiant);
        }

        #[test]
        fn r1851001_r70_a_cast_of_either_face_with_nothing_to_deradiant_still_resolves() {
            for radiant in [false, true] {
                let mut s = bare("prestige-cast-empty", radiant);
                let events = by_hand(
                    &mut s,
                    vec![jackioh_engine::effects::cast_new(json_as(
                        json!({ "def": PRESTIGE, "radiant": radiant }),
                    ))],
                );
                assert!(
                    events
                        .iter()
                        .any(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == PRESTIGE)),
                    "radiant {radiant}: the cast is played"
                );
                assert!(s.state().pending.is_none(), "radiant {radiant}: no pick is asked");
                assert!(next_play_radiant(&s), "radiant {radiant}: its Cry ran");
            }
        }

        #[test]
        fn r1851003_r1838002_either_face_is_traded_with_nothing_to_deradiant() {
            for radiant in [false, true] {
                let mut s = bare("prestige-trade-empty", radiant);
                assert!(!offered(&s), "radiant {radiant}: no play is offered");
                assert!(trade_offered(&s), "radiant {radiant}: the Trade is");
                let prestige = s.card(PRESTIGE).id.clone();
                s.trade(&prestige);
                s.expect_events(json!(["traded", "shuffledIn", "drawn"]));
                assert_eq!(trades_left(s.state(), PlayerId::P1), 0);
                assert_eq!(s.state().players.p1.turn_log.cards_played, 0, "a Trade is no play");
            }
        }

        #[test]
        fn r1851000_r214_a_base_prestige_the_next_play_makes_radiant_answers_the_radiant_faces_gate() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "prestige-r214",
                "p1": {
                    "mana": 10,
                    "hand": [PRESTIGE, PRESTIGE, { "def": VANILLA, "radiant": true }, FILLER],
                    "field": [{ "def": VANILLA, "radiant": true }],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(4) },
            }));
            let own = must_unit(&s, "p1", 1);
            s.play(PRESTIGE, pick(&own.id));
            assert!(next_play_radiant(&s));
            // The second Prestige would resolve on its Radiant face (R831005, R214), so that face's gate
            // decides: the Radiant card in p1's hand, a pick for the base face, does not open it.
            assert!(!offered(&s));
            assert_eq!(play_reason(&s).as_deref(), Some(NOTHING_TO_ACT_ON));
            let enemy = must_unit(&s, "p2", 1);
            s.card_mut(&enemy.id).radiant = true;
            assert!(offered(&s));
            s.play(PRESTIGE, json!({}));
            assert!(!s.card(&enemy.id).radiant);
        }
    }
}
