//! M #6 Me no Likey (SPEC §8.8 row 6, BUILD M10 row M 6). (0) Spell, Epic.
//!   Base:    "Give one of your Units to your opponent. If you do, gain mana equal to its cost."
//!   Radiant: "Give one of your Units to your opponent. If you do, gain mana equal to {multiple}
//!            times its cost." (2)
//!
//! R1423's `give_control`, already on `main`: the declared target — a Unit its controller controls,
//! the top of its pile (R13, R81) — moves to the opponent's side, placed per R15 (the same lane, else
//! their first free zone) as an entry (R171: summoning sick, fresh exertion), emitting
//! `controlChanged`. Its cost is read before the move: `cost_now` (R65, as it stands on the field; an
//! X Unit costs its X, R396; a token its printed cost). The mana is `gain_mana`, temporary mana that
//! may exceed 4.
//!
//! "If you do": the gift's mana follows only when the Unit actually changed sides — with no free zone
//! on the opponent's side it stays and nothing is gained (R803). The owner doesn't change (§3.2), so
//! a given Unit that dies goes to its owner's graveyard. The number is declared and read through
//! `param` (R386); the base face prints none (R749).

use jackioh_engine::effects::{ForEachCardArgs, for_each_card, gain_mana, give_control, instance_of};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-006";

/// "Give one of your Units to your opponent."
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let Some(unit) = instance_of(ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            // The gift's price, read before it moves (R803, R65, R396).
            let gain = cost_now(&*ctx.state, &unit) * param(&*ctx, "multiple");
            let id = unit.id.clone();
            let moved = id.clone();
            vec![
                give_control(json_as(json!({ "instanceId": id }))),
                for_each_card(ForEachCardArgs {
                    // "If you do": only a Unit that now stands on the opponent's side pays out.
                    cards: Arc::new(move |now: &mut EffectContext<'_>| {
                        let enemy = opponent_of(now.controller);
                        let changed_sides = active_units_of(&*now.state, enemy)
                            .iter()
                            .any(|card| card.id == moved);
                        if gain > 0 && changed_sides {
                            vec![moved.clone()]
                        } else {
                            vec![]
                        }
                    }),
                    each: Arc::new(move |_instance_id: &str| {
                        gain_mana(json_as(json!({ "amount": gain })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared `multiple` (2), read through
    // `param`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// M #6 Me no Likey — SPEC §8.8 row 6, BUILD M10 row M 6: "Give one of your Units to their lane (R15,
// R171, R1423) and gain its cost on the field (R65, R396, R803); with their row full it stays and gains
// nothing; an X Unit gives its X; a dead gift goes to your graveyard (§3.2); an enemy Unit is no
// legal target; radiant gains twice the cost; its tuned number (multiple) reads through `param()`
// (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LIKELY: &str = "meditative-006";
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const FODDER: &str = "core-015"; // (1) 1/1 Unit.
    const BILLY: &str = "classicplus-069"; // (X) Unit 3X/3X.
    const FILLER: &str = "core-005"; // (1) Spell.

    use crate::js;

    use crate::matches_object;

    /// A target answer naming p1's card of `def`.
    fn at(s: &Scenario, def: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": s.card(def).id }])
    }

    mod meditative_006 {
        use super::*;

        #[test]
        fn r803_gives_your_unit_to_their_lane_and_gains_its_cost() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [LIKELY, FILLER], "mana": 9, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [FODDER] },
            }));
            let menace = s.card(MENACE).id.clone();

            s.play(LIKELY, json!({ "targets": at(&s, MENACE) }));

            let moved = s.card(&menace).clone();
            assert_eq!(moved.controller, P2);
            assert_eq!(moved.owner, P1);
            assert!(matches_object(&js(&moved.zone), &json!({ "row": "units" })));
            // (0) played from 9, plus the Menace's 3.
            s.expect_mana(P1, 12);
        }

        #[test]
        fn r803_with_their_row_full_it_stays_and_gains_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [LIKELY, FILLER], "mana": 9, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [FODDER, FODDER, FODDER, FODDER, FODDER] },
            }));
            let menace = s.card(MENACE).id.clone();

            s.play(LIKELY, json!({ "targets": at(&s, MENACE) }));

            assert_eq!(s.card(&menace).controller, P1);
            s.expect_mana(P1, 9);
        }

        #[test]
        fn r803_an_x_unit_gives_its_x() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [BILLY, LIKELY, FILLER], "mana": 9 },
                "p2": { "hand": [FILLER] },
            }));
            s.play(BILLY, json!({ "x": 3 }));
            let billy = s.card(BILLY).id.clone();

            s.play(LIKELY, json!({ "targets": at(&s, BILLY) }));

            assert_eq!(s.card(&billy).controller, P2);
            // 9 − 3 for Billy, nothing for the (0), plus its X of 3.
            s.expect_mana(P1, 9);
        }

        #[test]
        fn r803_dying_it_goes_to_your_graveyard() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [LIKELY, FILLER], "mana": 9, "field": [MENACE, FODDER] },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));
            let gift = s.card(FODDER).id.clone();
            let targets = at(&s, FODDER);
            s.play(LIKELY, json!({ "targets": targets }));
            assert_eq!(s.card(&gift).controller, P2);

            // The 9-attack Menace trades into the gift: its owner takes the body home (§3.2).
            let menace = s.card(MENACE).id.clone();
            s.attack(&menace, &gift);

            let dead = s.card(&gift).clone();
            assert_eq!(dead.zone.z(), ZoneName::Graveyard);
            assert_eq!(dead.owner, P1);
            let home: Vec<String> = s.pile(P1, "graveyard").iter().map(|card| card.id.clone()).collect();
            assert!(home.contains(&gift));
        }

        #[test]
        fn an_enemy_unit_is_no_legal_target() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [LIKELY, FILLER], "mana": 9, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [FODDER] },
            }));
            let enemy = s.card(FODDER).id.clone();

            s.expect_refused(|s| {
                s.play(LIKELY, json!({ "targets": [{ "pick": "instance", "instanceId": enemy }] }))
            });
        }

        #[test]
        fn radiant_gains_twice_the_cost() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": LIKELY, "radiant": true }, FILLER], "mana": 9, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [FODDER] },
            }));
            let menace = s.card(MENACE).id.clone();

            s.play(LIKELY, json!({ "targets": at(&s, MENACE) }));

            assert_eq!(s.card(&menace).controller, P2);
            // Twice the Menace's 3.
            s.expect_mana(P1, 15);
        }
    }
}
