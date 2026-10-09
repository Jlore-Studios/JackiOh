//! M #53 Prestige (SPEC §8.8 row 53): (4) Spell, Rare.
//!
//! Base:    "De-Radiant a Radiant Unit or face-up backrow card on either side, or a Radiant card
//! in your hand. Add {cards|AI generated card|AI generated cards} to your hand."
//! Radiant: "De-Radiant every enemy card: every Radiant card of the opponent's field, hand and
//! library. Add {cards|AI generated card|AI generated cards} to your hand."
//! Engine: MD-D6 — De-Radiant is Make Radiant reversed (`clearRadiant`), §5.2's exception: the face
//! swaps back while damage, buffs and tuning stay. The base face clears the chosen card, if any
//! (R90 already allows the play with nothing to choose, MD-D7); the Radiant face clears every
//! Radiant card of the opponent's field, hand and library, never the graveyard (MD-D8). Both faces
//! then add {cards} AI generated cards (the R1421 AI pool).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-053";

/// The base face's pick: a Radiant Unit or face-up backrow card on either side, or a Radiant card
/// in your hand.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(
        1,
        1,
        json!({ "side": "any", "of": ["unit", "backrow", "hand"], "check": "radiant" }),
    )]
}

fn target_checks() -> IndexMap<&'static str, TargetCheck> {
    IndexMap::from([(
        "radiant",
        // A Radiant card that is not a face-down backrow card: hidden information never shows
        // (R440), so a face-down card is not a "face-up backrow card".
        target_check(|a| {
            let Some(candidate) = a.candidate else {
                return false;
            };
            if !candidate.radiant {
                return false;
            }
            if matches!(candidate.zone, Zone::Field { row: Row::Backrow, .. })
                && matches!(
                    card_type_of(a.state, candidate),
                    CardType::Trap | CardType::FieldTrap
                )
                && candidate.face_up != Some(true)
                && candidate.revealed != Some(true)
            {
                return false;
            }
            true
        }),
    )])
}

/// Add {cards} AI generated cards to your hand.
fn add_ai(ctx: &EffectContext<'_>, radiant: bool) -> Effect {
    add_random_from_catalog(json_as(json!({
        "query": { "tags": ["AI"] },
        "count": param(ctx, "cards"),
        "radiant": radiant,
    })))
}

fn prestige(radiant: bool) -> Script {
    Script {
        targets: targets(),
        target_checks: target_checks(),
        cry: Some(hook(move |ctx| {
            vec![
                clear_radiant(RadiantTarget::default()),
                add_ai(ctx, radiant),
            ]
        })),
        ..Script::default()
    }
}

fn prestige_radiant(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // MD-D8: every Radiant card of the opponent's field, hand and library — never the
            // graveyard, whose cards De-Radiant cannot reach.
            let enemy = opponent_of(ctx.controller);
            let mut effects: Vec<Effect> = permanents_on_field(ctx.state, enemy)
                .iter()
                .filter(|card| card.controller == enemy && card.radiant)
                .map(|card| {
                    clear_radiant(json_as(json!({ "instanceId": card.id })))
                })
                .collect();
            for card in ctx.state.players[enemy]
                .hand
                .iter()
                .chain(ctx.state.players[enemy].library.iter())
                .filter(|card| card.radiant)
            {
                effects.push(clear_radiant(json_as(json!({ "instanceId": card.id }))));
            }
            effects.push(add_ai(ctx, radiant));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: prestige(false),
        radiant: prestige_radiant(true),
    }
}

// M #53 Prestige — SPEC §8.8 row 53, BUILD M10 row M 53: "De-Radiant a Radiant Unit or face-up
// backrow card on either side, or a Radiant card in your hand (MD-D6, R90); the Radiant face
// De-Radiants every enemy card of the opponent's field, hand and library, never the graveyard
// (MD-D8); both faces add {cards} AI generated card(s) (R1421), a Radiant one off the Radiant face".
#[cfg(test)]
mod tests {
    use super::*;
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

    /// The AI generated cards in p1's hand.
    fn ai_cards(s: &Scenario) -> Vec<CardInstance> {
        s.hand("p1")
            .into_iter()
            .filter(|card| crate::card_def(&card.def_id).tags.contains(&Tag::Ai))
            .collect()
    }

    mod m53_prestige {
        use super::*;

        #[test]
        fn deradiants_the_chosen_unit_keeping_damage_and_buffs_then_adds_an_ai_card() {
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
            let added = ai_cards(&s);
            assert_eq!(added.len(), 1);
            assert!(!added[0].radiant);
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
            let held = s.hand("p1").into_iter().find(|card| card.def_id == VANILLA).expect("the Radiant hand card");
            assert!(held.radiant);
            s.play(
                PRESTIGE,
                json!({ "targets": [{ "pick": "instance", "instanceId": held.id }] }),
            );
            let cleared = s.hand("p1").into_iter().find(|card| card.id == held.id).expect("still in hand");
            assert!(!cleared.radiant);
            assert_eq!(ai_cards(&s).len(), 1);
        }

        #[test]
        fn r1103_with_no_radiant_card_the_play_is_legal_and_only_adds() {
            let mut s = casting("prestige-empty", false);
            // Nothing is Radiant anywhere: no target, but the play stays legal (R90).
            for unit in [must_unit(&s, "p1", 1), must_unit(&s, "p2", 1)] {
                s.card_mut(&unit.id).radiant = false;
            }
            s.play(PRESTIGE, json!({}));
            assert_eq!(ai_cards(&s).len(), 1);
        }

        #[test]
        fn r1104_radiant_clears_field_hand_deck_not_graveyard() {
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
            // Every enemy card is De-Radianted: the opponent's field, hand and library.
            assert!(!must_unit(&s, "p2", 1).radiant);
            assert!(s.hand("p2").iter().all(|card| !card.radiant));
            assert!(s.pile("p2", "library").iter().all(|card| !card.radiant));
            // Never the graveyard (MD-D8).
            assert!(s.pile("p2", "graveyard").iter().all(|card| card.radiant));
            // Your own Radiant unit stands Radiant still.
            assert!(must_unit(&s, "p1", 1).radiant);
            let added = ai_cards(&s);
            assert_eq!(added.len(), 1);
            assert!(added[0].radiant);
        }

        #[test]
        fn adds_a_radiant_ai_card_off_the_radiant_face() {
            let mut s = casting("prestige-ai", false);
            let unit = must_unit(&s, "p2", 1);
            s.play(
                PRESTIGE,
                json!({ "targets": [{ "pick": "instance", "instanceId": unit.id }] }),
            );
            let added = ai_cards(&s);
            assert_eq!(added.len(), 1);
            // The base face adds it un-Radianted.
            assert!(!added[0].radiant);
        }
    }
}
