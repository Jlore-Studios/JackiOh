//! C #19 Lizard's Breath (SPEC §8.6 row 19). (1) Spell, Rare.
//!   Base:    "Deal {damage} damage. Your largest pile adds its effect: Deck, draw {draw}; Graveyard,
//!            gain {mana} mana; Exile, deal {extraDamage} more damage. Ties go to the pile listed
//!            first." — 2, 1, 2, 4
//!   Radiant: "Deal {damage} damage. Your two largest piles add their effects: …" — 4, 1, 2, 4
//!   Engine:  "Your own deck, graveyard and exile, counted as it resolves. The damage is one hit on the
//!            play's target: 2, or 6 with exile (Radiant 4, or 8); then the draw and the temporary mana.
//!            A `preview` (R280) names the pile or piles that would count now. Tunes: damage 2 ↑; draw
//!            1 ↑; mana 2 ↑; extra damage 4 ↑."
//!
//! "Deal N damage" with no target named is targeted (§8's Conventions): any unit or hero, either side,
//! chosen with the play (R81).
//!
//! The piles are counted as the Spell resolves — it is in the resolving zone then, so in none of them
//! (§10.5) — and ranked by size, a tie going to the pile the text lists first (Deck, then Graveyard,
//! then Exile); the base face takes the largest, the Radiant face the two largest. `countingPiles` is
//! that ranking, and both the resolution and the preview read it. So three empty piles pick the Deck,
//! which draws (from an empty deck, a fatigue, §2.4).
//!
//! Exile's effect is part of the one hit: the damage is {damage}, plus {extraDamage} when Exile counts,
//! dealt once. Then the Deck's draw and the Graveyard's mana — temporary mana, §2.3 — in that order.
//!
//! R280: the preview names the pile or piles that count now, one value per pile in rank order, each
//! carrying the pile's name as `display` beside its size, after the face's own words for the ranking
//! ("Your largest pile", "Your two largest piles"). It reads only the controller's pile sizes, which
//! are public (§10.8), never a pile's contents.
//!
//! The numbers are the declared `damage`, `draw`, `mana` and `extraDamage` (R386), read through `param`.

use jackioh_engine::effects::{damage, draw, gain_mana};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-019";

/// TS `PileName`: the three piles' names in the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PileName {
    Deck,
    Graveyard,
    Exile,
}

impl PileName {
    fn as_str(self) -> &'static str {
        match self {
            PileName::Deck => "Deck",
            PileName::Graveyard => "Graveyard",
            PileName::Exile => "Exile",
        }
    }
}

#[derive(Clone, Copy)]
struct Pile {
    zone: OffFieldZone,
    name: PileName,
}

/// The three piles in the text's order, which is the tie order.
const PILES: [Pile; 3] = [
    Pile {
        zone: OffFieldZone::Library,
        name: PileName::Deck,
    },
    Pile {
        zone: OffFieldZone::Graveyard,
        name: PileName::Graveyard,
    },
    Pile {
        zone: OffFieldZone::Exile,
        name: PileName::Exile,
    },
];

/// TS `RankedPile = { name: PileName; size: number }`.
#[derive(Clone, Copy, Debug)]
struct RankedPile {
    name: PileName,
    size: i32,
}

/// §8 Conventions: any unit or hero, either side.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// The `count` largest of `player`'s piles, largest first, a tie going to the pile listed first.
fn counting_piles(state: &GameState, player: PlayerId, count: usize) -> Vec<RankedPile> {
    let mut sized: Vec<(PileName, i32, usize)> = PILES
        .iter()
        .enumerate()
        .map(|(order, pile)| (pile.name, zone_count(state, player, pile.zone), order))
        .collect();
    // Stable, as `Array.prototype.sort` (SURFACE §4.4.1): `b.size - a.size || a.order - b.order`.
    sized.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));
    sized
        .into_iter()
        .take(count)
        .map(|(name, size, _)| RankedPile { name, size })
        .collect()
}

fn lizards_breath(piles_counted: usize, label: &'static str) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let counting: Vec<PileName> = counting_piles(&*ctx.state, ctx.controller, piles_counted)
                .into_iter()
                .map(|pile| pile.name)
                .collect();
            let extra = if counting.contains(&PileName::Exile) {
                param(&*ctx, "extraDamage")
            } else {
                0
            };
            let mut effects = vec![damage(json_as(
                json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") + extra }),
            ))];
            if counting.contains(&PileName::Deck) {
                effects.push(draw(json_as(json!({ "count": param(&*ctx, "draw") }))));
            }
            if counting.contains(&PileName::Graveyard) {
                effects.push(gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))));
            }
            effects
        })),
        // R280: the pile or piles that would count now, in rank order.
        preview: Some(condition_hook(move |ctx| -> Vec<PreviewValue> {
            counting_piles(ctx.state, ctx.controller, piles_counted)
                .into_iter()
                .map(|pile| PreviewValue {
                    label: label.to_string(),
                    value: pile.size,
                    display: Some(pile.name.as_str().to_string()),
                    ids: None,
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: lizards_breath(1, "Your largest pile"),
        radiant: lizards_breath(2, "Your two largest piles"),
    }
}

// C #19 Lizard's Breath — SPEC §8.6 row 19, BUILD M9 Classic row C 19: "One hit on the play's target
// (any Unit or hero): 2, or 6 when Exile counts; your own deck, graveyard and exile are counted as it
// resolves (this Spell, resolving, is in none); the largest pile adds its effect: Deck draws 1,
// Graveyard gives 2 mana this turn, Exile adds 4 damage; ties go to the pile listed first (Deck, then
// Graveyard, then Exile), so three empty piles pick Deck and draw (fatigue from an empty deck); the
// draw and the mana follow the hit; its preview names the pile that would count now (R280); radiant:
// 4, or 8 with Exile, and the two largest piles add their effects with the same ties; the preview
// names both; its tuned numbers (damage, draw, mana, extra damage) read through `param()` (R386)".
//
// The preview's proofs (R280) are in `../preview.test.ts`, with the other cards'.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BREATH: &str = "classic-019";
    const MENACE: &str = "core-019";
    const FILLER: &str = "core-005";
    const ANCHOR: &str = "core-010";
    const X: &str = "core-008";

    fn at_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Face {
        Base,
        Radiant,
    }

    /// TS `piles(deck, graveyard, exile)`: the side's library, graveyard and exile, as setup JSON.
    fn piles(deck: usize, graveyard: usize, exile: usize) -> (Value, Value, Value) {
        (json!(vec![X; deck]), json!(vec![FILLER; graveyard]), json!(vec![FILLER; exile]))
    }

    fn hits(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { source_id: Some(_), amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn drawn(s: &Scenario) -> usize {
        s.events().iter().filter(|event| event.event_type() == GameEventType::Drawn).count()
    }

    fn breath(face: Face, deck: usize, graveyard: usize, exile: usize, mana: Option<i32>) -> Scenario {
        let (library, grave, exiled) = piles(deck, graveyard, exile);
        let mut p1 = json!({
            "hand": [{ "def": BREATH, "radiant": face == Face::Radiant }, ANCHOR],
            "library": library,
            "graveyard": grave,
            "exile": exiled,
        });
        if let Some(mana) = mana {
            p1["mana"] = json!(mana);
        }
        scenario(json!({
            "p1": p1,
            "p2": { "hand": [ANCHOR], "field": [{ "def": MENACE, "radiant": true }] },
        }))
    }

    /// TS `stepParam(s.card(ref), key, steps)`: on the live instance, found again by id.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        step_param(find_instance_mut(s.state_mut(), &id).expect("the card is in the state"), key, steps);
    }

    mod c_n19_lizard_s_breath {
        use super::*;

        #[test]
        fn declares_one_target_any_unit_or_hero_on_either_side_and_a_preview_on_both_faces() {
            crate::register_all();
            let scripts = script();
            assert_eq!(crate::card_def(ID).id, BREATH);
            assert_eq!(
                serde_json::to_value(&scripts.base.targets).unwrap(),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }])
            );
            assert!(scripts.base.preview.is_some());
            assert!(scripts.radiant.preview.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn the_deck_largest_one_hit_of_2_then_draw_1() {
                crate::register_all();
                let mut s = breath(Face::Base, 3, 1, 1, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![2]);
                assert_eq!(drawn(&s), 1);
                s.expect_mana(P1, 3);
                s.expect_events(json!(["damage", "drawn"]));
            }

            #[test]
            fn a_unit_target_is_legal_too_the_hit_lands_on_it() {
                crate::register_all();
                let mut s = breath(Face::Base, 3, 1, 1, None);
                let menace = s.card(MENACE).clone();

                s.play(BREATH, json!({ "targets": [{ "pick": "instance", "instanceId": menace.id }] }));

                s.expect_stats(&menace, json!({ "health": 16 }));
            }

            #[test]
            fn the_graveyard_largest_2_damage_then_2_mana_this_turn() {
                crate::register_all();
                let mut s = breath(Face::Base, 1, 3, 1, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![2]);
                assert_eq!(drawn(&s), 0);
                s.expect_mana(P1, 5);
                s.expect_events(json!(["damage", "manaChanged"]));
            }

            #[test]
            fn s2_3_the_graveyard_s_mana_is_temporary_current_mana_rises_max_mana_stays_4() {
                crate::register_all();
                let mut s = breath(Face::Base, 1, 3, 1, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(serde_json::to_value(&s.view(P1).you.mana).unwrap(), json!({ "current": 5, "max": 4 }));
            }

            #[test]
            fn ties_go_to_the_pile_listed_first_deck_over_exile() {
                crate::register_all();
                let mut s = breath(Face::Base, 2, 0, 2, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![2]);
                assert_eq!(drawn(&s), 1);
            }

            #[test]
            fn the_exile_largest_one_hit_of_6() {
                crate::register_all();
                let mut s = breath(Face::Base, 1, 1, 3, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![6]);
                s.expect_health(P2, 24);
                assert_eq!(drawn(&s), 0);
                s.expect_mana(P1, 3);
            }

            #[test]
            fn counted_as_it_resolves_this_spell_is_in_no_pile_so_a_2_2_deck_and_graveyard_tie_goes_to_the_deck() {
                crate::register_all();
                let mut s = breath(Face::Base, 2, 2, 0, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(drawn(&s), 1);
                s.expect_mana(P1, 3);
                let graveyard: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.def_id).collect();
                assert!(graveyard.contains(&BREATH.to_string()));
            }

            #[test]
            fn ties_go_to_the_pile_listed_first_graveyard_over_exile() {
                crate::register_all();
                let mut s = breath(Face::Base, 0, 2, 2, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![2]);
                s.expect_mana(P1, 5);
            }

            #[test]
            fn s2_4_three_empty_piles_pick_the_deck_and_its_draw_is_a_fatigue() {
                crate::register_all();
                let mut s = breath(Face::Base, 0, 0, 0, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(
                    s.events().iter().filter(|event| event.event_type() == GameEventType::Fatigue).count(),
                    1
                );
                s.expect_health(P1, 29).expect_health(P2, 28);
            }

            #[test]
            fn r386_each_number_tunes_damage_3_draw_2_mana_3_extra_damage_5() {
                crate::register_all();
                let mut dmg = breath(Face::Base, 3, 1, 1, None);
                step(&mut dmg, BREATH, "damage", 1);
                dmg.play(BREATH, json!({ "targets": at_hero() }));
                assert_eq!(hits(&dmg), vec![3]);

                let mut more = breath(Face::Base, 3, 1, 1, None);
                step(&mut more, BREATH, "draw", 1);
                more.play(BREATH, json!({ "targets": at_hero() }));
                assert_eq!(drawn(&more), 2);

                let mut mana = breath(Face::Base, 1, 3, 1, None);
                step(&mut mana, BREATH, "mana", 1);
                mana.play(BREATH, json!({ "targets": at_hero() }));
                mana.expect_mana(P1, 6);

                let mut extra = breath(Face::Base, 1, 1, 3, None);
                step(&mut extra, BREATH, "extraDamage", 1);
                extra.play(BREATH, json!({ "targets": at_hero() }));
                assert_eq!(hits(&extra), vec![7]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_exile_and_the_graveyard_largest_one_hit_of_8_then_2_mana() {
                crate::register_all();
                let mut s = breath(Face::Radiant, 1, 3, 5, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![8]);
                assert_eq!(drawn(&s), 0);
                s.expect_mana(P1, 5);
            }

            #[test]
            fn the_deck_and_the_exile_largest_one_hit_of_8_then_draw_1() {
                crate::register_all();
                let mut s = breath(Face::Radiant, 4, 1, 3, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![8]);
                assert_eq!(drawn(&s), 1);
                s.expect_mana(P1, 3);
            }

            #[test]
            fn the_deck_and_the_graveyard_largest_one_hit_of_4_then_the_draw_then_the_mana() {
                crate::register_all();
                let mut s = breath(Face::Radiant, 4, 3, 1, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![4]);
                assert_eq!(drawn(&s), 1);
                s.expect_mana(P1, 5);
                s.expect_events(json!(["damage", "drawn", "manaChanged"]));
            }

            #[test]
            fn ties_for_second_place_go_to_the_pile_listed_first_exile_largest_a_deck_graveyard_tie_behind_it_picks_the_deck() {
                crate::register_all();
                let mut s = breath(Face::Radiant, 2, 2, 5, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![8]);
                assert_eq!(drawn(&s), 1);
                s.expect_mana(P1, 3);
            }

            #[test]
            fn ties_go_to_the_piles_listed_first_three_equal_piles_pick_the_deck_and_the_graveyard() {
                crate::register_all();
                let mut s = breath(Face::Radiant, 2, 2, 2, None);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![4]);
                assert_eq!(drawn(&s), 1);
                s.expect_mana(P1, 5);
            }

            #[test]
            fn r386_an_upgrade_of_damage_on_the_radiant_face_steps_4_to_5_and_with_exile_9() {
                crate::register_all();
                let mut s = breath(Face::Radiant, 4, 1, 3, None);
                step(&mut s, BREATH, "damage", 1);

                s.play(BREATH, json!({ "targets": at_hero() }));

                assert_eq!(hits(&s), vec![9]);
            }
        }
    }
}
