//! M #59 Permanent Upperclassman (SPEC §8.8 row 59): (3) Unit, Human, Rare, 3/3 → 6/6.
//!
//! Base:    "Cry: Summon a random (4) Cost Unit. It gains "Death: Fuse a random Book and a random AI
//!          generated card into your hand.""
//! Radiant: "Cry: Summon a random Radiant (4) Cost Unit. It gains "Death: Fuse a random Radiant Book
//!          and a random Radiant AI generated card into your hand."" (the granted Death quoted in its
//!          sentence, R1445)
//! Engine: ME-GRANT (docs/meditative-set.md M5, MD-D13) plus multi-pool fusion (MD-D14) — the
//! summoned Unit is granted `meditative-059#fusedBookDeath` as plain data; the hook fuses one pick
//! from each pool, the Book first so the fused type falls back to it, into the controller's hand at
//! (min(sum,4)) (R469). The Radiant face's fused card is Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-059";

/// The granted Death, as both faces quote it and the grant map keys it.
pub const FUSED_BOOK_DEATH: &str = "meditative-059#fusedBookDeath";

/// Summon a random (4) Unit, then grant every Unit this Cry summoned the fused Book Death (one
/// summon, read off `summoned_so_far` as C+ #19 does).
fn upperclassman(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![
                summon_random(json_as(json!({
                    "query": { "type": "Unit", "cost": 4 },
                    "radiant": radiant,
                }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &mut EffectContext<'_>| summoned_so_far(ctx)),
                    each: Arc::new(move |instance_id: &str| {
                        grant_ability(json_as(json!({
                            "instanceId": instance_id,
                            "grant": FUSED_BOOK_DEATH,
                            "radiant": radiant,
                        })))
                    }),
                }),
            ]
        })),
        grants: IndexMap::from([(
            "fusedBookDeath",
            hook(move |_ctx| {
                vec![fuse_generated(json_as(json!({
                    "count": 2,
                    "pools": [{ "tags": ["Book"] }, { "tags": ["AI"] }],
                    "radiant": radiant,
                    "handPrice": "fused",
                })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: upperclassman(false),
        radiant: upperclassman(true),
    }
}

// M #59 Permanent Upperclassman — SPEC §8.8 row 59, BUILD M10 row M 59: "Cry: summon a random (4)
// Unit (Radiant: a random Radiant one) and grant it the face's Death (MD-D13, R1420); the grant
// shows on the Unit and is lost when the Unit leaves the field; at the death one Book and one AI
// card fuse into the controller's hand (MD-D14), a non-token Spell at (min(sum,4)), the Book first
// so the fused type falls back to it; the Radiant face's fused card is Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const UPPERCLASSMAN: &str = "meditative-059";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1 holds the Upperclassman (base unless `radiant_face`); both sides keep cards in hand so no
    /// turn auto-ends.
    fn casting(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": UPPERCLASSMAN, "radiant": radiant_face }, FILLER],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// Destroy the unit through the real death pass (marked, then `state_check`), returning the
    /// pass's events.
    fn kill(s: &mut Scenario, unit: &CardInstance) -> Vec<GameEvent> {
        s.card_mut(&unit.id).marked_destroyed = Some(true);
        let mut events = Vec::new();
        let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
        let mut rng = Rng::new(&seed, cursor);
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            state_check(&mut sink);
        }
        s.state_mut().rng_cursor = rng.cursor();
        events
    }

    /// Apply `effects` as `controller`'s, outside any card (a steal).
    fn run_as(s: &mut Scenario, effects: &[Effect], controller: PlayerId) {
        let mut events = Vec::new();
        let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
        let mut rng = Rng::new(&seed, cursor);
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(controller),
                    ..HookOptions::default()
                },
            );
            apply_effects(effects, &mut ctx);
        }
        s.state_mut().rng_cursor = rng.cursor();
    }

    /// The fused cards in `seat`'s hand.
    fn fused_of(s: &Scenario, seat: PlayerId) -> Vec<CardInstance> {
        s.hand(seat)
            .into_iter()
            .filter(|card| card.def_id.starts_with("t-"))
            .collect()
    }

    /// p1 plays the Upperclassman (`radiant_face`) into lane 1; the summoned Unit stands in lane 2.
    fn summoned(seed: &str, radiant_face: bool) -> (Scenario, CardInstance) {
        let mut s = casting(seed, radiant_face);
        s.play(UPPERCLASSMAN, json!({ "zone": 1 }));
        let unit = s.unit("p1", 2).expect("the summoned Unit");
        (s, unit)
    }

    /// Like `summoned`, from the first seed after `seed` whose pick the death pass can destroy (not
    /// Indestructible, no Divine Shield, no Reborn), so its granted Death fires.
    fn mortal(seed: &str, radiant_face: bool) -> (Scenario, CardInstance) {
        (0..200)
            .map(|n| summoned(&format!("{seed}-{n}"), radiant_face))
            .find(|(s, unit)| {
                !keywords_of(s.state(), unit).iter().any(|keyword| {
                    matches!(
                        keyword.kind(),
                        KeywordKind::Indestructible | KeywordKind::DivineShield | KeywordKind::Reborn
                    )
                })
            })
            .expect("a seed summons a mortal (4) Unit")
    }

    /// The fused card's definition and its two ingredients' definitions.
    fn fused_parts(s: &Scenario, card: &CardInstance) -> (CardDef, Vec<(CardDef, bool)>) {
        let def = def_of(Some(s.state()), &card.def_id).clone();
        let parts = def
            .ingredients
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|ingredient| {
                (
                    crate::card_def(&ingredient.def_id).clone(),
                    ingredient.radiant == Some(true),
                )
            })
            .collect();
        (def, parts)
    }

    /// The fused cards in p1's hand (transient `t-` definitions).
    fn fused(s: &Scenario) -> Vec<CardInstance> {
        s.hand("p1")
            .into_iter()
            .filter(|card| card.def_id.starts_with("t-"))
            .collect()
    }

    /// The granted lines on p1's Unit in `lane`, as the client reads them.
    fn granted_lines(s: &Scenario, lane: usize) -> Vec<String> {
        let view: Value =
            serde_json::to_value(view_for(s.state(), PlayerId::P1)).expect("the view serialises");
        view["you"]["units"][lane - 1]["grants"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|line| line.as_str().map(str::to_string))
            .collect()
    }

    mod m59_permanent_upperclassman {
        use super::*;

        #[test]
        fn summons_a_4_cost_unit_and_fuses_a_book_and_an_ai_card_at_its_death() {
            crate::register_all();
            let mut s = casting("upperclassman", false);
            s.play(UPPERCLASSMAN, json!({ "zone": 1 }));
            // The Upperclassman stands in lane 1, so the summon takes the leftmost open zone (R64).
            assert_eq!(
                s.unit("p1", 1).map(|unit| unit.def_id),
                Some(UPPERCLASSMAN.to_string())
            );
            assert!(
                granted_lines(&s, 1).is_empty(),
                "the Upperclassman itself gains nothing"
            );
            let summoned = s.unit("p1", 2).expect("the summoned Unit");
            assert_eq!(crate::card_def(&summoned.def_id).cost, CardCost::Fixed(4));
            assert!(!summoned.radiant);
            assert_eq!(granted_lines(&s, 2).len(), 1);
            kill(&mut s, &summoned);
            let made = fused(&s);
            assert_eq!(made.len(), 1);
            // A fused definition is transient, held in the state, not the catalog (R77).
            let def = def_of(Some(s.state()), &made[0].def_id).clone();
            assert!(!def.token, "the fused card is a non-token (R102)");
            assert_eq!(
                def.type_,
                CardType::Spell,
                "the Book comes first, so the fused type falls back to it"
            );
            let ingredients: Vec<String> = def
                .ingredients
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(|ingredient| ingredient.def_id)
                .collect();
            assert_eq!(ingredients.len(), 2);
            assert!(crate::card_def(&ingredients[0]).tags.contains(&Tag::Book));
            assert!(crate::card_def(&ingredients[1]).tags.contains(&Tag::Ai));
            // (min(sum,4)): the fused card costs no more than 4.
            assert!(
                cost_now(s.state(), &made[0]) <= 4,
                "{}",
                cost_now(s.state(), &made[0])
            );
        }

        #[test]
        fn the_summoned_unit_runs_no_cry() {
            // Find a seed whose (4) pick prints a Cry: the summon runs none of it (§6.3 Summon).
            let (s, unit) = (0..400)
                .map(|n| summoned(&format!("upperclassman-cry-{n}"), false))
                .find(|(_, unit)| crate::card_def(&unit.def_id).base.text.contains("Cry:"))
                .expect("some seed summons a (4) Unit with a Cry");
            assert!(s.state().pending.is_none(), "no Cry prompt opened");
            let p1_units = (1..=UNIT_ZONES)
                .filter(|lane| s.unit("p1", *lane).is_some())
                .count();
            assert_eq!(p1_units, 2, "only the Upperclassman and {} stand", unit.def_id);
            let summons = s
                .last_events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::Summoned)
                .count();
            // One for the played card's arrival, one for the summon: the Cry summoned nothing more.
            assert_eq!(summons, 2, "the summoned Unit's Cry summoned nothing more");
        }

        #[test]
        fn the_fused_card_is_a_non_token_spell_at_the_lesser_of_the_sum_and_4() {
            for n in 0..12 {
                let (mut s, unit) = mortal(&format!("upperclassman-cost-{n}"), false);
                kill(&mut s, &unit);
                let made = fused(&s);
                assert_eq!(made.len(), 1);
                let (def, parts) = fused_parts(&s, &made[0]);
                assert!(!def.token, "not a Token");
                // Books are all Spells, so the shared type (or the Book's fallback) is a Spell.
                assert_eq!(def.type_, CardType::Spell);
                let sum: i32 = parts
                    .iter()
                    .map(|(part, _)| match part.cost {
                        CardCost::Fixed(cost) => cost,
                        _ => 0,
                    })
                    .sum();
                assert_eq!(cost_now(s.state(), &made[0]), sum.min(4), "seed {n}: sum {sum}");
            }
        }

        #[test]
        fn the_fused_text_is_both_texts() {
            let (mut s, unit) = mortal("upperclassman-text", false);
            kill(&mut s, &unit);
            let made = fused(&s);
            let (def, parts) = fused_parts(&s, &made[0]);
            let joined: Vec<&str> = parts.iter().map(|(part, _)| part.base.text.as_str()).collect();
            assert_eq!(def.base.text, joined.join("\n"));
        }

        #[test]
        fn r1107_stolen_the_fused_card_goes_to_the_dying_units_controller() {
            let (mut s, unit) = mortal("upperclassman-steal", false);
            run_as(
                &mut s,
                &[steal(json_as(json!({ "instanceId": unit.id })))],
                PlayerId::P2,
            );
            assert_eq!(s.card(&unit.id).controller, PlayerId::P2);
            let stolen = s.card(&unit.id).clone();
            kill(&mut s, &stolen);
            assert_eq!(fused_of(&s, PlayerId::P2).len(), 1, "the fused card goes to p2");
            assert!(fused_of(&s, PlayerId::P1).is_empty());
        }

        #[test]
        fn a_full_hand_burns_the_fused_card() {
            let (mut s, unit) = mortal("upperclassman-burn", false);
            // Fill p1's hand to the cap.
            while s.hand("p1").len() < HAND_CAP as usize {
                let card = new_instance(
                    s.state_mut(),
                    FILLER,
                    PlayerId::P1,
                    Zone::Hand { player: PlayerId::P1 },
                );
                s.state_mut().players.p1.hand.push(card);
            }
            let events = kill(&mut s, &unit);
            assert!(fused(&s).is_empty(), "nothing fused reaches the full hand");
            assert_eq!(s.hand("p1").len(), HAND_CAP as usize);
            let burned: Vec<&String> = events
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Burned { def_id, .. } => Some(def_id),
                    _ => None,
                })
                .collect();
            assert_eq!(burned.len(), 1, "{events:?}");
            assert!(burned[0].starts_with("t-"), "the fused card burned: {burned:?}");
        }

        #[test]
        fn r468_the_fused_definition_survives_a_json_round_trip() {
            let (mut s, unit) = mortal("upperclassman-json", false);
            kill(&mut s, &unit);
            let made = fused(&s);
            let before = def_of(Some(s.state()), &made[0].def_id).clone();
            let round: GameState =
                serde_json::from_value(serde_json::to_value(s.state()).expect("serialises"))
                    .expect("deserialises");
            let after = def_of(Some(&round), &made[0].def_id).clone();
            assert_eq!(after, before);
            assert_eq!(cost_now(&round, &made[0]), cost_now(s.state(), &made[0]));
        }

        #[test]
        fn the_radiant_face_is_6_6() {
            let mut s = casting("upperclassman-stats", true);
            s.play(UPPERCLASSMAN, json!({ "zone": 1 }));
            s.expect_stats(UPPERCLASSMAN, json!({ "attack": 6, "health": 6 }));
        }

        #[test]
        fn r469_the_radiant_face_fuses_two_radiant_ingredients_into_a_radiant_card() {
            let (mut s, unit) = mortal("upperclassman-radiant-parts", true);
            kill(&mut s, &unit);
            let made = fused(&s);
            assert_eq!(made.len(), 1);
            let (def, parts) = fused_parts(&s, &made[0]);
            assert_eq!(parts.len(), 2);
            assert!(
                parts.iter().all(|(_, radiant)| *radiant),
                "both ingredients Radiant"
            );
            // Each went in on its Radiant face, so the fused text is both Radiant texts.
            let joined: Vec<&str> = parts.iter().map(|(part, _)| part.radiant.text.as_str()).collect();
            assert_eq!(def.radiant.text, joined.join("\n"));
            assert!(made[0].radiant, "the fused card is Radiant");
        }

        #[test]
        fn the_radiant_face_summons_radiant_and_fuses_radiant() {
            crate::register_all();
            let mut s = casting("upperclassman-radiant", true);
            s.play(UPPERCLASSMAN, json!({ "zone": 1 }));
            let summoned = s.unit("p1", 2).expect("the summoned Unit");
            assert_eq!(crate::card_def(&summoned.def_id).cost, CardCost::Fixed(4));
            assert!(summoned.radiant);
            kill(&mut s, &summoned);
            let made = fused(&s);
            assert_eq!(made.len(), 1);
            assert!(made[0].radiant);
        }
    }
}
