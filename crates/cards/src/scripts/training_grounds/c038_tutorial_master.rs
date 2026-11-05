//! TG #38 Tutorial Master (SPEC §8.11 row 38): (4) Unit, Human, Legendary, 4/16 → 8/32, a card of the held
//! Training Grounds set (R1420, R2194000).
//!   Base:    "Cry & Death: Add {cards|random Training Grounds card|random Training Grounds cards} to your
//!            hand." — cards 3
//!   Radiant: the same with Radiant cards — cards 3
//! Engine: S #45 Drone Commander's add, as a Cry and as a Death. The pool names the set (R2194001): its
//!   non-token cards but this one (R387), one match-rng draw per card, repeats allowed (R60), each at its
//!   own price; a full hand burns the rest (§2.4). `cards` is read by both hooks. Radiant: added Radiant.
//!   The two hooks stay, the one label printing both: the words are one line since issue #3139, with no
//!   "They cost (1) less" (the `discount` param went with it). Through v0.5.11p (`script_v0_5_11o`, kept,
//!   R767000) each card cost `discount` less, a `costMod` flooring at (0) (R65).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-038";

/// `discounted` is v0.5.11p's: each card added cost the declared `discount` less (R65).
fn master(radiant: bool, discounted: bool) -> Script {
    let add = hook(move |ctx| {
        let mut args = json!({
            "query": { "set": "Training Grounds" },
            "count": param(&*ctx, "cards"),
            "radiant": radiant,
        });
        if discounted {
            args["costMod"] = json!(-param(&*ctx, "discount"));
        }
        vec![add_random_from_catalog(json_as(args))]
    });
    Script {
        cry: Some(add.clone()),
        death: Some(add),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: master(false, false),
        radiant: master(true, false),
    }
}

/// R767000: through v0.5.11p the cards it added cost `discount` (1) less, a number its definition declared;
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: master(false, true),
        radiant: master(true, true),
    }
}

// TG #38 Tutorial Master — SPEC §8.11 row 38, BUILD M13 row TG 38.
#[cfg(test)]
mod tests {
    use super::{ID, script_v0_5_11o};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.

    fn game(radiant: bool, seed: &str) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB, FILLER],
                "library": crate::filler(4),
                "mana": 30,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    /// The cards the last step added to p1's hand.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player, instance_id, .. } if *player == P1 => Some(instance_id.clone()),
                _ => None,
            })
            .map(|id| s.card(id.as_str()).clone())
            .collect()
    }

    fn play_master(s: &mut Scenario) -> Vec<CardInstance> {
        s.play(ID, json!({ "zone": 1 }));
        added(s)
    }

    fn kill_master(s: &mut Scenario) -> Vec<CardInstance> {
        let me = s.unit_at(P1, 1).id;
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": me }] }));
        s.expect_in_zone(me.as_str(), "graveyard");
        added(s)
    }

    /// R2194001, R387: a non-token Training Grounds card, never this one, of the face asked, at its own price.
    fn from_the_pool(card: &CardInstance, radiant: bool) {
        let def = catalog::def_of(None, &card.def_id);
        assert_eq!(def.set, SetName::TrainingGrounds, "{}", card.def_id);
        assert!(!def.token, "{}", card.def_id);
        assert_ne!(card.def_id, ID);
        assert_eq!(card.radiant, radiant);
        assert_eq!(card.cost_mod, 0);
    }

    /// p1's hand as p1 sees it: (instance, cost).
    fn hand_costs(s: &Scenario) -> Vec<(String, i32)> {
        let HandView::Cards(hand) = s.view(P1).you.hand else {
            panic!("your own hand is shown card by card");
        };
        hand.iter().map(|view| (view.instance_id.clone(), view.cost)).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r2194001_its_cry_adds_3_training_grounds_cards_never_itself() {
            let mut s = game(false, "tutorial-master");
            let cards = play_master(&mut s);
            assert_eq!(cards.len(), 3);
            cards.iter().for_each(|card| from_the_pool(card, false));
        }

        #[test]
        fn r2194001_its_death_adds_3_more() {
            let mut s = game(false, "tutorial-master");
            play_master(&mut s);
            let cards = kill_master(&mut s);
            assert_eq!(cards.len(), 3);
            cards.iter().for_each(|card| from_the_pool(card, false));
        }

        #[test]
        fn r60_r387_repeats_are_allowed_and_it_never_adds_itself() {
            for n in 0..64 {
                let mut s = game(false, &format!("tutorial-master-{n}"));
                let mut cards = play_master(&mut s);
                cards.extend(kill_master(&mut s));
                cards.iter().for_each(|card| from_the_pool(card, false));
                let mut defs: Vec<&str> = cards.iter().map(|card| card.def_id.as_str()).collect();
                defs.sort_unstable();
                if defs.windows(2).any(|pair| pair[0] == pair[1]) {
                    return;
                }
            }
            panic!("no seed drew a repeat");
        }

        /// Issue #3139: "They cost (1) less" is gone, so a Cry's cards and a Death's both cost what they print.
        #[test]
        fn r65_no_card_it_adds_costs_less_on_the_cry_or_on_the_death() {
            let mut s = game(false, "tutorial-master-price");
            let mut cards = play_master(&mut s);
            cards.extend(kill_master(&mut s));
            assert_eq!(cards.len(), 6);
            let costs = hand_costs(&s);
            for card in &cards {
                let printed = printed_cost(s.state(), card);
                assert_eq!(card.cost_mod, 0, "{}", card.def_id);
                assert!(costs.contains(&(card.id.clone(), printed)), "{} {costs:?}", card.def_id);
            }
        }

        #[test]
        fn cards_reads_through_param_and_the_discount_is_no_longer_declared() {
            crate::register_all();
            let declared: Vec<String> = crate::card_def(ID)
                .params
                .iter()
                .flatten()
                .map(|param| param.key.clone())
                .collect();
            assert_eq!(declared, vec!["cards".to_string()]);
            let mut s = game(false, "tutorial-master");
            let me = s.card(ID).id.clone();
            assert_eq!(crate::upgrade_number(&mut s, &me, "cards"), 4);
            let cards = play_master(&mut s);
            assert_eq!(cards.len(), 4);
            assert!(cards.iter().all(|card| card.cost_mod == 0));
        }

        /// Issue #3139: the Cry and the Death are one printed line, "Cry & Death: Add …", on both faces.
        #[test]
        fn the_cry_and_the_death_are_one_printed_line_on_both_faces() {
            crate::register_all();
            let def = crate::card_def(ID);
            for (face, radiant) in [(&def.base, false), (&def.radiant, true)] {
                let which = if radiant { "random Radiant Training Grounds" } else { "random Training Grounds" };
                let expected = format!(
                    "Cry & Death: Add {{cards|{which} card|{which} cards}} to your hand."
                );
                assert_eq!(face.text, expected, "radiant {radiant}");
            }
        }

        /// R767000: through v0.5.11p each card it added cost `discount` (1) less, on the Cry and on the Death.
        #[test]
        fn r767000_the_script_kept_for_v0_5_11o_adds_each_card_1_less_on_the_cry_and_the_death() {
            let old = |def: &mut CardDef| {
                let declared: Vec<Param> = serde_json::from_value(json!([{
                    "key": "discount", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1
                }]))
                .expect("v0.5.11p's discount");
                def.params.get_or_insert_with(Vec::new).extend(declared);
            };
            crate::under_kept_as(ID, script_v0_5_11o(), old, || {
                let mut s = game(false, "tutorial-master");
                let mut cards = play_master(&mut s);
                cards.extend(kill_master(&mut s));
                assert_eq!(cards.len(), 6);
                assert!(cards.iter().all(|card| card.cost_mod == -1));
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2194001_it_is_8_32_and_its_cry_and_death_add_radiant_cards() {
            let mut s = game(true, "tutorial-master");
            let cards = play_master(&mut s);
            let me = s.unit_at(P1, 1);
            s.expect_stats(&me, json!({ "attack": 8, "maxHealth": 32 }));
            assert_eq!(cards.len(), 3);
            cards.iter().for_each(|card| from_the_pool(card, true));
            let cards = kill_master(&mut s);
            assert_eq!(cards.len(), 3);
            cards.iter().for_each(|card| from_the_pool(card, true));
        }
    }
}
