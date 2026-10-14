//! M #58 Permanent Underclassman (SPEC §8.8 row 58): (2) Unit, Human, Common, 3/3 → 6/6.
//!
//! Base:    "Cry: Summon a random (1) Cost Unit. It gains "Death: Add {books|random Book|random Books}
//! to your hand.""
//! Radiant: "Cry: Summon a random Radiant (1) Cost Unit. It gains "Death: Add {books|random Radiant
//! Book|random Radiant Books} to your hand."" (the granted Death quoted in its sentence, R1445)
//! Engine: ME-GRANT (docs/meditative-set.md M5, MD-D13) — the summoned Unit is granted
//! `meditative-058#bookDeath` with the face's numbers, as plain data naming the hook this file
//! registers beside its faces; `grants::death_hook_of` runs it after the Unit's own Death, for the
//! controller at the death. Both faces carry the quoted ability in `grants`, so the view prints it.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-058";

/// The granted Death, as both faces quote it and the grant map keys it.
pub const BOOK_DEATH: &str = "meditative-058#bookDeath";

/// Summon a random (1) Unit, then grant every Unit this Cry summoned the Book Death with the
/// face's numbers (one summon, read off `summoned_so_far` as C+ #19 does).
fn underclassman(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let books = param(&*ctx, "books");
            vec![
                summon_random(json_as(json!({
                    "query": { "type": "Unit", "cost": 1 },
                    "radiant": radiant,
                }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &mut EffectContext<'_>| summoned_so_far(ctx)),
                    each: Arc::new(move |instance_id: &str| {
                        grant_ability(json_as(json!({
                            "instanceId": instance_id,
                            "grant": BOOK_DEATH,
                            "radiant": radiant,
                            "params": { "books": books },
                        })))
                    }),
                }),
            ]
        })),
        grants: IndexMap::from([(
            "bookDeath",
            hook(move |ctx| {
                vec![add_random_from_catalog(json_as(json!({
                    "query": { "tags": ["Book"] },
                    "count": grant_param(ctx, "books"),
                    "radiant": radiant,
                })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: underclassman(false),
        radiant: underclassman(true),
    }
}

// M #58 Permanent Underclassman — SPEC §8.8 row 58, BUILD M10 row M 58: "Cry: summon a random (1)
// Unit (Radiant: a random Radiant one) and grant it the face's Death (MD-D13, R1420); the grant
// shows on the Unit, survives a Vanilla, a copy and a JSON round trip, and is lost when the Unit
// leaves the field; the controller gets the card (a stolen unit's controller too, proved at the
// engine level); a full board summons nothing and grants nothing".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const UNDERCLASSMAN: &str = "meditative-058";
    const FILLER: &str = "core-005";
    /// (1) 4/4 Unit with no text: what fills the board.
    const VANILLA: &str = "core-008";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1 holds the Underclassman (base unless `radiant_face`); both sides keep cards in hand so no
    /// turn auto-ends.
    fn casting(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": UNDERCLASSMAN, "radiant": radiant_face }, FILLER],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The Book cards in p1's hand.
    fn books(s: &Scenario) -> Vec<CardInstance> {
        s.hand("p1")
            .into_iter()
            .filter(|card| crate::card_def(&card.def_id).tags.contains(&Tag::Book))
            .collect()
    }

    /// Destroy the unit through the real death pass (marked, then `state_check`).
    fn kill(s: &mut Scenario, unit: &CardInstance) {
        s.card_mut(&unit.id).marked_destroyed = Some(true);
        let mut events = Vec::new();
        let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
        let mut rng = Rng::new(&seed, cursor);
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            state_check(&mut sink);
        }
        s.state_mut().rng_cursor = rng.cursor();
    }

    /// Apply `effects` as `controller`'s, outside any card (a steal, a transform, a copy, a cast).
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

    /// The Book cards in `seat`'s hand.
    fn books_of(s: &Scenario, seat: PlayerId) -> Vec<CardInstance> {
        s.hand(seat)
            .into_iter()
            .filter(|card| crate::card_def(&card.def_id).tags.contains(&Tag::Book))
            .collect()
    }

    /// p1 plays the base Underclassman into lane 1; the summoned Unit stands in lane 2.
    fn summoned(seed: &str) -> (Scenario, CardInstance) {
        let mut s = casting(seed, false);
        s.play(UNDERCLASSMAN, json!({ "zone": 1 }));
        let unit = s.unit("p1", 2).expect("the summoned Unit");
        (s, unit)
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

    mod m58_permanent_underclassman {
        use super::*;

        #[test]
        fn summons_a_1_cost_unit_and_grants_the_death_shown_on_the_unit() {
            crate::register_all();
            let mut s = casting("underclassman", false);
            s.play(UNDERCLASSMAN, json!({ "zone": 1 }));
            // The Underclassman stands in lane 1, so the summon takes the leftmost open zone (R64).
            assert_eq!(
                s.unit("p1", 1).map(|unit| unit.def_id),
                Some(UNDERCLASSMAN.to_string())
            );
            assert!(
                granted_lines(&s, 1).is_empty(),
                "the Underclassman itself gains nothing"
            );
            let summoned = s.unit("p1", 2).expect("the summoned Unit");
            assert_eq!(crate::card_def(&summoned.def_id).cost, CardCost::Fixed(1));
            assert!(!summoned.radiant);
            let lines = granted_lines(&s, 2);
            assert_eq!(lines.len(), 1);
            assert!(lines[0].starts_with("Death:"), "{lines:?}");
            // One Book reaches the controller's hand when it dies.
            kill(&mut s, &summoned);
            assert_eq!(books(&s).len(), 1);
        }

        #[test]
        fn the_radiant_face_summons_radiant_and_grants_radiant_books() {
            crate::register_all();
            let mut s = casting("underclassman-radiant", true);
            s.play(UNDERCLASSMAN, json!({ "zone": 1 }));
            let summoned = s.unit("p1", 2).expect("the summoned Unit");
            assert_eq!(crate::card_def(&summoned.def_id).cost, CardCost::Fixed(1));
            assert!(summoned.radiant);
            let lines = granted_lines(&s, 2);
            assert_eq!(lines.len(), 1);
            kill(&mut s, &summoned);
            let added = books(&s);
            assert_eq!(added.len(), 1);
            assert!(added[0].radiant);
        }

        #[test]
        fn the_summoned_unit_runs_no_cry() {
            // Find a seed whose pick prints a Cry: the summon runs none of it (§6.3 Summon).
            let (s, unit) = (0..400)
                .map(|n| summoned(&format!("underclassman-cry-{n}")))
                .find(|(_, unit)| crate::card_def(&unit.def_id).base.text.contains("Cry:"))
                .expect("some seed summons a (1) Unit with a Cry");
            assert!(s.state().pending.is_none(), "no Cry prompt opened");
            let p1_units = (1..=UNIT_ZONES)
                .filter(|lane| s.unit("p1", *lane).is_some())
                .count();
            assert_eq!(p1_units, 2, "only the Underclassman and {} stand", unit.def_id);
            let summons = s
                .last_events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::Summoned)
                .count();
            // One for the played card's arrival, one for the summon: the Cry summoned nothing more.
            assert_eq!(summons, 2, "the summoned Unit's Cry summoned nothing more");
        }

        #[test]
        fn a_cast_underclassman_runs_its_cry_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "underclassman-cast",
                "p1": { "hand": [FILLER], "library": filler(4) },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            run_as(
                &mut s,
                &[cast_new(CastNewArgs {
                    def: UNDERCLASSMAN.into(),
                    radiant: None,
                    how: CastHow::default(),
                })],
                PlayerId::P1,
            );
            assert_eq!(
                s.unit("p1", 1).map(|unit| unit.def_id),
                Some(UNDERCLASSMAN.to_string())
            );
            let unit = s.unit("p1", 2).expect("the cast Cry summoned a Unit");
            assert_eq!(crate::card_def(&unit.def_id).cost, CardCost::Fixed(1));
            assert_eq!(granted_lines(&s, 2).len(), 1);
            kill(&mut s, &unit);
            assert_eq!(books(&s).len(), 1);
        }

        #[test]
        fn the_grant_survives_a_vanilla() {
            let (mut s, unit) = summoned("underclassman-vanilla");
            run_as(
                &mut s,
                &[vanilla(json_as(json!({ "instanceId": unit.id })))],
                PlayerId::P1,
            );
            assert!(s.card(&unit.id).vanilla, "the Vanilla applied");
            assert_eq!(granted_lines(&s, 2).len(), 1);
            kill(&mut s, &unit);
            assert_eq!(books(&s).len(), 1);
        }

        #[test]
        fn the_grant_survives_a_copy() {
            let (mut s, unit) = summoned("underclassman-copy");
            run_as(
                &mut s,
                &[summon_copy(json_as(
                    json!({ "of": { "of": "instance", "instanceId": unit.id } }),
                ))],
                PlayerId::P1,
            );
            let copy = s.unit("p1", 3).expect("the copy");
            assert_eq!(copy.def_id, unit.def_id);
            assert_ne!(copy.id, unit.id);
            assert_eq!(copy.grants, s.card(&unit.id).grants);
            assert_eq!(granted_lines(&s, 3).len(), 1);
            kill(&mut s, &copy);
            assert_eq!(books(&s).len(), 1, "the copy's Death adds the Book");
        }

        #[test]
        fn r1107_the_grant_survives_a_json_round_trip() {
            let (mut s, unit) = summoned("underclassman-json");
            let round: GameState =
                serde_json::from_value(serde_json::to_value(s.state()).expect("serialises"))
                    .expect("deserialises");
            assert_eq!(
                find_instance(&round, &unit.id).and_then(|card| card.grants.clone()),
                unit.grants
            );
            *s.state_mut() = round;
            assert_eq!(granted_lines(&s, 2).len(), 1);
            kill(&mut s, &unit);
            assert_eq!(books(&s).len(), 1, "the round-tripped grant still runs");
        }

        #[test]
        fn the_grant_is_lost_when_the_unit_leaves_the_field() {
            let (mut s, unit) = summoned("underclassman-leave");
            kill(&mut s, &unit);
            assert_eq!(books(&s).len(), 1);
            // In the graveyard it carries no grant (R78).
            let gone = s
                .pile("p1", "graveyard")
                .into_iter()
                .find(|card| card.id == unit.id)
                .expect("the summoned Unit in the graveyard");
            assert_eq!(gone.grants, None);
        }

        #[test]
        fn the_grant_is_lost_when_the_unit_is_transformed() {
            let (mut s, unit) = summoned("underclassman-transform");
            run_as(
                &mut s,
                &[transform(json_as(
                    json!({ "instanceId": unit.id, "defId": VANILLA }),
                ))],
                PlayerId::P1,
            );
            let now = s.unit("p1", 2).expect("the transformed Unit");
            assert_eq!(now.def_id, VANILLA);
            assert_eq!(now.grants, None);
            assert!(granted_lines(&s, 2).is_empty());
            kill(&mut s, &now);
            assert!(books(&s).is_empty(), "a transformed Unit adds no Book");
        }

        #[test]
        fn r1107_stolen_it_adds_the_book_to_its_new_controller() {
            let (mut s, unit) = summoned("underclassman-steal");
            run_as(
                &mut s,
                &[steal(json_as(json!({ "instanceId": unit.id })))],
                PlayerId::P2,
            );
            assert_eq!(s.card(&unit.id).controller, PlayerId::P2);
            let stolen = s.card(&unit.id).clone();
            kill(&mut s, &stolen);
            assert_eq!(
                books_of(&s, PlayerId::P2).len(),
                1,
                "the Book goes to the new controller"
            );
            assert!(books_of(&s, PlayerId::P1).is_empty());
        }

        #[test]
        fn r386_books_reads_through_param() {
            crate::register_all();
            let mut s = casting("underclassman-param", false);
            step_param(s.card_mut(UNDERCLASSMAN), "books", 1);
            s.play(UNDERCLASSMAN, json!({ "zone": 1 }));
            let unit = s.unit("p1", 2).expect("the summoned Unit");
            kill(&mut s, &unit);
            assert_eq!(books(&s).len(), 2, "a Buffed {{books}} adds two");
        }

        #[test]
        fn the_radiant_face_is_6_6() {
            let mut s = casting("underclassman-stats", true);
            s.play(UNDERCLASSMAN, json!({ "zone": 1 }));
            s.expect_stats(UNDERCLASSMAN, json!({ "attack": 6, "health": 6 }));
        }

        #[test]
        fn a_full_board_summons_nothing_and_grants_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "underclassman-full",
                "p1": {
                    "hand": [{ "def": UNDERCLASSMAN }, FILLER],
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            // Four units stand; the Underclassman takes the last lane and its summon fizzles.
            s.play(UNDERCLASSMAN, json!({ "zone": UNIT_ZONES }));
            let standing: Vec<String> = (1..=UNIT_ZONES)
                .filter_map(|lane| s.unit("p1", lane).map(|unit| unit.def_id))
                .collect();
            assert_eq!(
                standing,
                [VANILLA, VANILLA, VANILLA, VANILLA, UNDERCLASSMAN].map(str::to_string)
            );
            let view: Value =
                serde_json::to_value(view_for(s.state(), PlayerId::P1)).expect("the view serialises");
            let units = view["you"]["units"].as_array().cloned().unwrap_or_default();
            let grants: Vec<&Value> = units.iter().filter_map(|unit| unit.get("grants")).collect();
            assert!(grants.is_empty(), "{grants:?}");
        }
    }
}
