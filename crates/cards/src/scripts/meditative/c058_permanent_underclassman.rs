//! M #58 Permanent Underclassman (SPEC §8.8 row 58): (1) Unit, Human, Rare, 2/1 → 3/2.
//!
//! Base:    "Cry: Summon a random (1) Unit. It gains 'Death: Add {books|random Book card|random Book
//! cards} to your hand.'"
//! Radiant: "Cry: Summon a random Radiant (1) Unit. It gains 'Death: Add {books|random Book card|
//! random Book cards} to your hand.'"
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

    /// The granted lines on p1's lane-1 Unit, as the client reads them.
    fn granted_lines(s: &Scenario) -> Vec<String> {
        let view: Value =
            serde_json::to_value(view_for(s.state(), PlayerId::P1)).expect("the view serialises");
        view["you"]["units"][0]["grants"]
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
            let summoned = s.unit("p1", 1).expect("the summoned Unit");
            assert_eq!(crate::card_def(&summoned.def_id).cost, CardCost::Fixed(1));
            assert!(!summoned.radiant);
            let lines = granted_lines(&s);
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
            let summoned = s.unit("p1", 1).expect("the summoned Unit");
            assert!(summoned.radiant);
            let lines = granted_lines(&s);
            assert_eq!(lines.len(), 1);
            kill(&mut s, &summoned);
            let added = books(&s);
            assert_eq!(added.len(), 1);
            assert!(added[0].radiant);
        }

        #[test]
        fn a_full_board_summons_nothing_and_grants_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "underclassman-full",
                "p1": {
                    "hand": [{ "def": UNDERCLASSMAN }, FILLER],
                    "field": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                    "library": filler(4),
                },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            // Six units stand; the Underclassman takes the last lane and its summon fizzles.
            s.play(UNDERCLASSMAN, json!({ "zone": 7 }));
            assert_eq!(s.state().players.p1.units.iter().filter(|lane| lane.is_some()).count(), 7);
            let view: Value =
                serde_json::to_value(view_for(s.state(), PlayerId::P1)).expect("the view serialises");
            let units = view["you"]["units"].as_array().cloned().unwrap_or_default();
            let grants: Vec<&Value> =
                units.iter().filter_map(|unit| unit.get("grants")).collect();
            assert!(grants.is_empty(), "{grants:?}");
        }
    }
}
