//! M #99 Paranoia (SPEC §8.8 row 99): (2) Field Spell, Epic.
//!
//! Base:    "Aura: You may play your Spells face-down as Traps. Each reveals at the time you choose
//!           as you play it: the end of this turn, the start of your next turn, or the end of your
//!           next turn.
//!           Cry: Draw {draw|card|cards}."
//! Radiant: "Aura: You may play your Spells face-down as Traps. Each reveals at the time you choose
//!           as you play it: the end of this turn, the start of your next turn, or the end of your
//!           next turn. Each has Echo +{echo}.
//!           Cry: Draw {draw|card|cards}."
//! Engine: ME-ALTPLAY permission for Spells (Radiant adds Echo), Cry draws.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-099";

fn face_down_spells(radiant: bool) -> FaceDownPlayHook {
    Arc::new(move |args| {
        let mut permission = FaceDownPlayPermission {
            spells: Some(true),
            ..FaceDownPlayPermission::default()
        };
        if radiant {
            permission.echo = Some(param(&args, "echo"));
        }
        vec![permission]
    })
}

fn paranoia(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = param(&*ctx, "draw");
            vec![draw(json_as(json!({ "count": count })))]
        })),
        face_down_play: Some(face_down_spells(radiant)),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: paranoia(false),
        radiant: paranoia(true),
    }
}

// M #99 Paranoia — SPEC §8.8 row 99, BUILD M10 row M 99.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PARANOIA: &str = "meditative-099";
    const STOCKPILE: &str = "core-005"; // Draw, heal; no declared targets.
    const ECLIPSE: &str = "core-035"; // Deal 3 to a target.
    const VANILLA: &str = "core-008";
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2.
    const TOE_CRACKER: &str = "classic-006"; // Aura: Your Traps cost (0).
    const COUNTERSPELL: &str = "classic-017"; // Trap: counters the opponent's next Spell.
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.

    /// Answer the open Target prompt at the enemy hero.
    fn answer_hero(s: &mut Scenario) {
        let pending = s.state().pending.clone().expect("an open prompt");
        let picked = pending
            .options
            .iter()
            .find(|option| option.selection == Selection::Hero { player: P2 })
            .expect("the enemy hero")
            .selection
            .clone();
        s.answer(json!([picked]));
    }

    fn set_plays(s: &Scenario, id: &str) -> Vec<ActionBody> {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter(|action| {
                matches!(
                    action,
                    ActionBody::Play { instance_id, face_down: Some(_), .. }
                    if instance_id == id
                )
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn cry_draws_one() {
            let mut s = scenario(json!({
                "seed": "paranoia-draw",
                "p1": { "mana": 8, "hand": [PARANOIA], "library": [VANILLA, VANILLA] },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let before = s.hand(P1).len();
            let paranoia = s.hand(P1).iter().find(|card| card.def_id == PARANOIA).cloned().unwrap();
            s.play(&paranoia.id, json!({}));
            // Played one, drew one.
            assert_eq!(s.hand(P1).len(), before);
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r1044_a_spell_offers_three_timings_a_unit_none() {
            let s = scenario(json!({
                "seed": "paranoia-timings",
                "p1": {
                    "mana": 8,
                    "hand": [PARANOIA, STOCKPILE, VANILLA],
                    "field": [{ "def": PARANOIA, "lane": 1 }],
                    "library": [VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let spell = s.hand(P1).iter().find(|card| card.def_id == STOCKPILE).cloned().unwrap();
            let mut timings: Vec<String> = set_plays(&s, &spell.id)
                .iter()
                .map(|action| match action {
                    ActionBody::Play { face_down: Some(timing), .. } => format!("{timing:?}"),
                    _ => unreachable!(),
                })
                .collect();
            timings.sort();
            timings.dedup();
            assert_eq!(timings, vec!["EndOfNextTurn", "EndOfThisTurn", "StartOfNextTurn"]);
            // A Unit gets no Spell permission.
            let unit = s.hand(P1).iter().find(|card| card.def_id == VANILLA).cloned().unwrap();
            assert_eq!(set_plays(&s, &unit.id), vec![]);
        }

        #[test]
        fn r1044_no_spell_damage() {
            let mut s = scenario(json!({
                "seed": "paranoia-nodamage",
                "p1": {
                    "mana": 8,
                    "hand": [ECLIPSE],
                    "field": [{ "def": PARANOIA, "lane": 1 }, { "def": SOLARIUS, "lane": 1 }],
                    "library": [VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let eclipse = s.hand(P1).iter().find(|card| card.def_id == ECLIPSE).cloned().unwrap();
            let health = s.state().players.p2.hero.health;
            // Setting the Spell deals nothing: its text waits for the reveal.
            s.play(&eclipse.id, json!({ "zone": 2, "row": "backrow", "faceDown": "endOfThisTurn" }));
            assert_eq!(s.state().players.p2.hero.health, health);
            // Revealed, it deals its 3 to the chosen hero: its damage is a Trap's, so Solarius's
            // Spell Damage +2 adds nothing (MD-E17).
            s.end_turn();
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 3);
        }
        /// Paranoia acting in p1's backrow, `hand` in p1's hand, `p2_field` opposite.
        fn paranoid(seed: &str, hand: Value, p1_field: Value, p2: Value) -> Scenario {
            let mut field = vec![json!({ "def": PARANOIA, "lane": 1 })];
            field.extend(p1_field.as_array().cloned().unwrap_or_default());
            let mut p2 = p2;
            if p2.get("hand").is_none() {
                p2["hand"] = json!([VANILLA]);
            }
            p2["library"] = json!([VANILLA, VANILLA]);
            scenario(json!({
                "seed": seed,
                "p1": { "mana": 8, "hand": hand, "field": field, "library": [VANILLA, VANILLA] },
                "p2": p2,
            }))
        }

        fn set(s: &mut Scenario, def: &str, timing: &str) -> String {
            let card = s.hand(P1).iter().find(|card| card.def_id == def).cloned().unwrap();
            s.play(&card.id, json!({ "zone": 2, "row": "backrow", "faceDown": timing }));
            // A set card takes a fresh id (R1040).
            s.backrow(P1, 2).expect("the set card").id
        }

        /// The card of `def` in p1's graveyard, if one is there.
        fn in_grave(s: &Scenario, def: &str) -> Option<CardInstance> {
            s.state().players.p1.graveyard.iter().find(|card| card.def_id == def).cloned()
        }

        #[test]
        fn r1044_set_at_its_price_shown_as_a_trap_play_with_its_cost() {
            let mut s = paranoid("paranoia-price", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let mana = s.state().players.p1.mana.current;
            set(&mut s, ECLIPSE, "endOfThisTurn");
            assert_eq!(s.state().players.p1.mana.current, mana - 1, "Lunar Eclipse's (1)");
            let by_type = s.state().players.p1.turn_log.played_by_type.clone().unwrap_or_default();
            assert_eq!(by_type.get(&CardType::Trap), Some(&1), "a Trap play: {by_type:?}");
            assert_eq!(by_type.get(&CardType::Spell), None);
            // The opponent sees a face-down card and its cost, nothing more.
            let foe = serde_json::to_value(s.view(P2).opponent.backrow[1].clone()).unwrap();
            assert_eq!(foe["faceDown"], json!(true));
            assert_eq!(foe["cost"], json!(1));
            assert!(foe.get("defId").is_none());
        }

        #[test]
        fn r1044_it_counts_as_a_trap_play_for_trap_discounts() {
            // Cloaked Toe Cracker: "Aura: Your Traps cost (0)." The set Spell is free.
            let mut s =
                paranoid("paranoia-discount", json!([ECLIPSE, VANILLA]), json!([TOE_CRACKER]), json!({}));
            let mana = s.state().players.p1.mana.current;
            set(&mut s, ECLIPSE, "endOfThisTurn");
            assert_eq!(s.state().players.p1.mana.current, mana, "priced as a Trap: (0)");
        }

        #[test]
        fn r1044_a_counter_for_spells_never_sees_it() {
            // Counterspell: "Reveals when your opponent plays a Spell: Counter it."
            let mut s = paranoid(
                "paranoia-counter",
                json!([ECLIPSE, VANILLA]),
                json!([]),
                json!({ "backrow": [{ "def": COUNTERSPELL, "faceUp": false }] }),
            );
            let counter = s.backrow(P2, 1).unwrap().id;
            let eclipse = set(&mut s, ECLIPSE, "endOfThisTurn");
            s.expect_in_zone(&eclipse, "field");
            s.expect_in_zone(&counter, "field");
            assert!(
                !s.events().iter().any(|event| matches!(event, GameEvent::Countered { .. })),
                "nothing was countered"
            );
        }

        #[test]
        fn r1044_reveals_at_the_start_of_your_next_turn_asks_then_and_resolves() {
            let mut s = paranoid("paranoia-start", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE, "startOfNextTurn");
            s.end_turn();
            // Silent through the opponent's turn.
            assert!(s.state().pending.is_none());
            s.expect_in_zone(&eclipse, "field");
            s.end_turn();
            assert_eq!(s.state().pending.as_ref().map(|p| p.kind), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 3);
            assert!(in_grave(&s, ECLIPSE).is_some());
        }

        #[test]
        fn r1044_reveals_at_the_end_of_your_next_turn() {
            let mut s = paranoid(
                "paranoia-end-next",
                json!([ECLIPSE, VANILLA, VANILLA]),
                json!([]),
                json!({}),
            );
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE, "endOfNextTurn");
            s.end_turn();
            s.end_turn();
            assert!(s.state().pending.is_none(), "not at the start of the next turn");
            s.expect_in_zone(&eclipse, "field");
            s.end_turn();
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 3);
        }

        #[test]
        fn r1044_with_no_legal_target_it_fizzles() {
            // Hit Job ("Destroy a Unit"), set with no Unit anywhere: its reveal finds none.
            let mut s = paranoid("paranoia-fizzle", json!([HIT_JOB, VANILLA]), json!([]), json!({}));
            set(&mut s, HIT_JOB, "endOfThisTurn");
            s.end_turn();
            assert!(s.state().pending.is_none(), "no prompt: it fizzled");
            assert!(in_grave(&s, HIT_JOB).is_some(), "the Spell lands in the graveyard");
        }

        #[test]
        fn r1044_it_goes_to_your_graveyard_as_the_spell() {
            let mut s = paranoid("paranoia-grave", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            set(&mut s, ECLIPSE, "endOfThisTurn");
            s.end_turn();
            answer_hero(&mut s);
            let card = in_grave(&s, ECLIPSE).expect("the Spell in the graveyard");
            assert_eq!(card.set_as, None);
            assert_eq!(card_type_of(s.state(), &card), CardType::Spell);
            assert_eq!(card.owner, P1);
        }

        #[test]
        fn r1045_set_before_paranoia_left_it_still_reveals_on_time() {
            let mut s = paranoid("paranoia-left", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE, "endOfThisTurn");
            // Paranoia leaves the field.
            let state = s.state_mut();
            let mut aura = state.players.p1.backrow[0].take().expect("Paranoia");
            aura.zone = Zone::Graveyard { player: P1 };
            state.players.p1.graveyard.push(aura);
            s.expect_in_zone(&eclipse, "field");
            s.end_turn();
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 3);
        }

        #[test]
        fn r1045_bounced_it_is_the_spell_again() {
            let mut s = paranoid("paranoia-bounce", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let eclipse = set(&mut s, ECLIPSE, "endOfThisTurn");
            let card = s.card(eclipse.as_str()).clone();
            assert!(card.set_as.is_some());
            {
                let state = s.state_mut();
                let mut events = Vec::new();
                let mut rng = Rng::new(&state.seed, state.rng_cursor);
                let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
                jackioh_engine::effects::move_::bounce_card(&mut sink, &card);
            }
            let home = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == ECLIPSE)
                .expect("the bounced Spell in hand");
            assert_eq!(home.set_as, None);
            assert_eq!(card_type_of(s.state(), &home), CardType::Spell);
        }

        #[test]
        fn draw_reads_through_param() {
            let mut s = scenario(json!({
                "seed": "paranoia-tuned",
                "p1": { "mana": 8, "hand": [PARANOIA], "library": [VANILLA, VANILLA, VANILLA] },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            assert_eq!(crate::upgrade_number(&mut s, PARANOIA, "draw"), 2);
            s.play(PARANOIA, json!({}));
            // Played one, drew two.
            assert_eq!(s.hand(P1).len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1045_radiant_gives_echo_plus_one() {
            let mut s = scenario(json!({
                "seed": "paranoia-echo",
                "p1": {
                    "mana": 8,
                    // A Vanilla still to play, so the set does not end the turn by itself (R82)
                    // and `end_turn` below is what reveals it.
                    "hand": [ECLIPSE, VANILLA],
                    "field": [{ "def": PARANOIA, "lane": 1, "radiant": true }],
                    "library": [VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let eclipse = s.hand(P1).iter().find(|card| card.def_id == ECLIPSE).cloned().unwrap();
            let health = s.state().players.p2.hero.health;
            s.play(&eclipse.id, json!({ "zone": 2, "row": "backrow", "faceDown": "endOfThisTurn" }));
            s.end_turn();
            // First resolution asks with a prompt; so does the Echo repeat.
            assert_eq!(s.state().pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert_eq!(s.state().pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 6);
        }
    }
}
