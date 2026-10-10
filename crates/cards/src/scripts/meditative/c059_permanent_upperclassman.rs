//! M #59 Permanent Upperclassman (SPEC §8.8 row 59): (3) Unit, Human, Rare, 3/3 → 6/6.
//!
//! Base:    "Cry: Summon a random (4) Unit. It gains 'Death: Fuse a random Book and AI card.'"
//! Radiant: "Cry: Summon a random Radiant (4) Unit. It gains 'Death: Fuse a random Radiant Book and
//! AI card.'"
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

    /// The fused cards in p1's hand (transient `t-` definitions).
    fn fused(s: &Scenario) -> Vec<CardInstance> {
        s.hand("p1").into_iter().filter(|card| card.def_id.starts_with("t-")).collect()
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

    mod m59_permanent_upperclassman {
        use super::*;

        #[test]
        fn summons_a_4_cost_unit_and_fuses_a_book_and_an_ai_card_at_its_death() {
            crate::register_all();
            let mut s = casting("upperclassman", false);
            s.play(UPPERCLASSMAN, json!({ "zone": 1 }));
            let summoned = s.unit("p1", 1).expect("the summoned Unit");
            assert_eq!(crate::card_def(&summoned.def_id).cost, CardCost::Fixed(4));
            assert!(!summoned.radiant);
            assert_eq!(granted_lines(&s).len(), 1);
            kill(&mut s, &summoned);
            let made = fused(&s);
            assert_eq!(made.len(), 1);
            let def = crate::card_def(&made[0].def_id);
            assert!(!def.token, "the fused card is a non-token (R102)");
            assert_eq!(def.type_, CardType::Spell, "the Book comes first, so the fused type falls back to it");
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
            assert!(cost_now(s.state(), &made[0]) <= 4, "{}", cost_now(s.state(), &made[0]));
        }

        #[test]
        fn the_radiant_face_summons_radiant_and_fuses_radiant() {
            crate::register_all();
            let mut s = casting("upperclassman-radiant", true);
            s.play(UPPERCLASSMAN, json!({ "zone": 1 }));
            let summoned = s.unit("p1", 1).expect("the summoned Unit");
            assert!(summoned.radiant);
            kill(&mut s, &summoned);
            let made = fused(&s);
            assert_eq!(made.len(), 1);
            assert!(made[0].radiant);
        }
    }
}
