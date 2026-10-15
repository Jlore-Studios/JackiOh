//! M #45 Knowledge Breaker (SPEC §8.8 row 45): (1) Unit, CN, KY, Catalyst, Legendary, 1/3 → 2/6.
//!
//! Base:    "Cry: Nerf every other CN or KY card on the field, in each hand and in each deck.
//!           Aura: Your Units have Cloaked.
//!           Death: Shuffle a Knowledge Breaker Prime into your deck."
//! Radiant: "Divine Shield
//!           Cry: Nerf every other CN or KY card on the field, in each hand and in each deck
//!           {times|time|times}.
//!           Aura: Your Units have Cloaked.
//!           Death: Shuffle a Radiant Knowledge Breaker Prime into your deck."
//! Engine:
//! - Cry: `degrade` over every other CN or KY card on fields, hands and decks, `times`.
//! - Death: `shuffle_into` the Prime, Radiant on the Radiant face (C+ #38's shape).
//! - Aura: Cloaked for the Units in its controller's hand while it acts (`cloak_hand_aura`, R1040);
//!   until patch v0.3.7 it was ME-ALTPLAY's face-down play permission.

use jackioh_engine::alt_play::cloak_hand_aura;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-045";

const PRIME: &str = "meditative-045-1";

fn breaker(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let times = param(&*ctx, "times");
            vec![degrade(json_as(json!({
                "scope": {
                    "side": "any",
                    "zones": ["field", "hand", "library"],
                    "tags": ["CN", "KY"],
                    "excludeSelf": true,
                },
                "times": times,
            })))]
        })),
        death: Some(hook(move |_ctx| {
            vec![shuffle_into(json_as(json!({
                "defId": PRIME,
                "count": 1,
                "radiant": radiant,
            })))]
        })),
        aura: Some(cloak_hand_aura(CardType::Unit)),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: breaker(false),
        radiant: breaker(true),
    }
}

// M #45 Knowledge Breaker — SPEC §8.8 row 45, BUILD M10 row M 45.
#[cfg(test)]
mod tests {
    use jackioh_engine::alt_play::CLOAKED_REVEAL;
    use jackioh_engine::prelude::keywords_of;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BREAKER: &str = "meditative-045";
    const PRIME: &str = "meditative-045-1";
    const CNKY: &str = "meditative-038";
    const VANILLA: &str = "core-008";
    const HIT_JOB: &str = "core-016"; // Destroy a Unit.

    /// The cards a Nerf changed. A card of a hidden pile the scope's filters left out is cued with
    /// no change (R440), so a cue is not a Nerf.
    fn degraded_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Degraded {
                    instance_id, change, ..
                } if *change != TuningChange::None => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// The cards of a hidden pile the Nerf cued without changing them (R440).
    fn cued_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Degraded {
                    instance_id,
                    change: TuningChange::None,
                    hidden_from: Some(_),
                    ..
                } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Breaker in p1's hand; CN/KY cards spread over both fields, hands and decks; a vanilla
    /// control on each side.
    fn spread(seed: &str, radiant_face: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 8,
                "hand": [
                    { "def": BREAKER, "radiant": radiant_face },
                    { "def": CNKY },
                    { "def": VANILLA },
                ],
                "field": [{ "def": CNKY, "lane": 1 }],
                "library": [{ "def": CNKY }, { "def": VANILLA }],
            },
            "p2": {
                "hand": [{ "def": CNKY }],
                "field": [{ "def": CNKY, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                "library": [{ "def": CNKY }],
            },
        }))
    }

    /// Every H1B Printer of a side: a Field Spell, so the one on the field stands in the backrow.
    fn cnky_ids(s: &Scenario, player: PlayerId) -> Vec<String> {
        let state = s.state();
        let side = &state.players[player];
        side.units
            .iter()
            .flatten()
            .flatten()
            .chain(side.backrow.iter().flatten())
            .chain(side.hand.iter())
            .chain(side.library.iter())
            .filter(|card| card.def_id == CNKY)
            .map(|card| card.id.clone())
            .collect()
    }

    const BIGOT: &str = "core-002";
    const ENEMY: &str = "core-025";

    fn in_hand(s: &Scenario, def_id: &str) -> CardInstance {
        s.hand(P1)
            .iter()
            .find(|card| card.def_id == def_id)
            .cloned()
            .expect("the card in p1's hand")
    }

    /// R1040, R783004: set the card face-down by its Cloaked into this backrow lane.
    fn set_face_down(s: &mut Scenario, id: &str, lane: i32) {
        s.play(id, json!({ "zone": lane, "row": "backrow", "faceDown": "cloaked" }));
    }

    fn reveal(s: &mut Scenario, id: &str) {
        s.activate(id, json!({ "ability": CLOAKED_REVEAL }));
    }

    fn reveal_offered(s: &Scenario, id: &str) -> bool {
        legal_actions(s.state(), P1).into_iter().any(|action| {
            matches!(&action, ActionBody::Activate { instance_id, ability, .. }
                if instance_id == id && ability.as_deref() == Some(CLOAKED_REVEAL))
        })
    }

    fn cloaked(s: &Scenario, id: &str) -> bool {
        keywords_of(s.state(), s.card(id))
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Cloaked)
    }

    fn face_down_plays(s: &Scenario, player: PlayerId, id: &str) -> Vec<ActionBody> {
        legal_actions(s.state(), player)
            .into_iter()
            .filter(|action| {
                matches!(action, ActionBody::Play { instance_id, face_down: Some(_), .. } if instance_id == id)
            })
            .collect()
    }

    fn attacks_of(s: &Scenario, id: &str) -> Vec<ActionBody> {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if attacker_id == id))
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1042_cry_nerfs_every_other_cn_or_ky_card_on_fields_hands_decks() {
            let mut s = spread("breaker-cry", false);
            let breaker = s
                .hand(P1)
                .iter()
                .find(|card| card.def_id == BREAKER)
                .cloned()
                .unwrap();
            let before_p1 = cnky_ids(&s, P1);
            let before_p2 = cnky_ids(&s, P2);
            assert_eq!(before_p1.len(), 3);
            assert_eq!(before_p2.len(), 3);
            s.play(&breaker.id, json!({ "zone": 2 }));
            let degraded = degraded_ids(&s);
            // Every other CN or KY card exactly once, on fields, hands and decks alike.
            for id in before_p1.iter().chain(before_p2.iter()) {
                assert_eq!(degraded.iter().filter(|hit| *hit == id).count(), 1, "{id}");
            }
            // Itself is excluded, and the vanilla controls are untouched: the two in p1's hidden
            // hand and deck are only cued (R440), the one on p2's field gets no event at all.
            assert!(!degraded.contains(&breaker.id));
            let state = s.state();
            let hidden_vanillas: Vec<String> = state.players[P1]
                .hand
                .iter()
                .chain(state.players[P1].library.iter())
                .filter(|card| card.def_id == VANILLA)
                .map(|card| card.id.clone())
                .collect();
            assert_eq!(hidden_vanillas.len(), 2);
            let cued = cued_ids(&s);
            for id in &hidden_vanillas {
                assert!(!degraded.contains(id), "{id} was Nerfed");
                assert!(cued.contains(id), "{id} was not cued");
            }
            let field_vanilla = s.unit(P2, 2).expect("p2's vanilla").id;
            assert!(!s.events().iter().any(|event| {
                matches!(event, GameEvent::Degraded { instance_id, .. } if *instance_id == field_vanilla)
            }));
        }

        #[test]
        fn r1040_r1041_a_set_unit_waits_for_its_reveal_then_attacks() {
            let mut s = scenario(json!({
                "seed": "breaker-set",
                "p1": {
                    "mana": 8,
                    "hand": [BREAKER, VANILLA],
                    "field": [{ "def": BREAKER, "lane": 1 }],
                    "library": [VANILLA, VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
            }));
            let vanilla = in_hand(&s, VANILLA);
            set_face_down(&mut s, &vanilla.id, 2);
            let set = s.backrow(P1, 2).expect("set in the backrow").id;
            // R783006: it has no timing, so the turns pass and it stays set.
            s.end_turn().end_turn();
            assert!(s.state().pending.is_none());
            assert!(s.unit(P1, 2).is_none(), "it never reveals by itself");
            assert!(s.backrow(P1, 2).is_some());
            reveal(&mut s, &set);
            // R1041, R383: it turned face-up and animated into the unit zone of its own lane.
            assert!(s.backrow(P1, 2).is_none());
            let unit = s.unit(P1, 2).expect("the revealed unit");
            assert_eq!(unit.def_id, VANILLA);
            assert_eq!(unit.face_up, Some(true));
            assert_eq!(unit.set_as, None);
            // It kept its set turn, so it may attack at once.
            assert!(!attacks_of(&s, &unit.id).is_empty());
        }

        #[test]
        fn death_shuffles_a_prime_radiant_on_radiant() {
            for radiant_face in [false, true] {
                let mut s = scenario(json!({
                    "seed": if radiant_face { "breaker-death-r" } else { "breaker-death" },
                    "p1": {
                        "mana": 8,
                        "hand": [HIT_JOB],
                        "field": [{ "def": BREAKER, "radiant": radiant_face, "lane": 2 }],
                        "library": [VANILLA],
                    },
                    "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 1 }] },
                }));
                let breaker = s.unit(P1, 2).expect("the Breaker");
                // Hit Job destroys it (the Radiant face's Divine Shield stops damage, not a
                // destroy): its Death shuffles exactly one Prime.
                s.play(
                    HIT_JOB,
                    json!({ "targets": [{ "pick": "instance", "instanceId": breaker.id }] }),
                );
                s.expect_in_zone(&breaker, "graveyard");
                let library = s.pile(P1, "library");
                let primes: Vec<_> = library.iter().filter(|card| card.def_id == PRIME).collect();
                assert_eq!(primes.len(), 1);
                assert_eq!(primes[0].radiant, radiant_face);
            }
        }

        #[test]
        fn m25_is_skipped() {
            // M 25 carries a granted CN tag: the Nerf's tag-filtered scope cannot reach it while it
            // acts on the field (ME-TRIBAL).
            const FELINOR: &str = "meditative-025";
            let mut s = scenario(json!({
                "seed": "breaker-felinor",
                "p1": { "mana": 8, "hand": [BREAKER, VANILLA], "library": [VANILLA] },
                "p2": { "hand": [VANILLA], "field": [{ "def": FELINOR, "lane": 1 }, { "def": CNKY, "lane": 2 }], "library": [VANILLA] },
            }));
            let felinor = s.unit(P2, 1).expect("M 25").id;
            s.card_mut(&felinor).granted_tags = Some(vec![Tag::Cn]);
            s.play(BREAKER, json!({ "zone": 2 }));
            let degraded = degraded_ids(&s);
            assert!(!degraded.contains(&felinor), "M 25 is skipped");
            assert!(!degraded.is_empty(), "the Printer beside it is Nerfed");
        }

        #[test]
        fn r1040_r1046_a_set_unit_pays_its_own_price_takes_no_targets_and_shows_as_a_trap() {
            // Bigot (2): its Cry targets an enemy non-Human, but set face-down it names none and its
            // text is dormant.
            let mut s = scenario(json!({
                "seed": "breaker-set-price",
                "p1": { "mana": 5, "hand": [BIGOT, VANILLA], "field": [{ "def": BREAKER, "lane": 1 }], "library": [VANILLA] },
                "p2": { "hand": [VANILLA], "field": [{ "def": ENEMY, "lane": 1 }], "library": [VANILLA] },
            }));
            let bigot = in_hand(&s, BIGOT);
            set_face_down(&mut s, &bigot.id, 2);
            assert!(s.state().pending.is_none(), "no target is asked");
            s.expect_mana(P1, 3);
            assert!(s.unit(P2, 1).is_some(), "the Cry is dormant");
            let set = s.backrow(P1, 2).expect("set in the backrow");
            assert_eq!(set.def_id, BIGOT);
            // The opponent reads a face-down backrow card and a Trap play, never the Unit.
            let theirs = serde_json::to_string(&s.view(P2)).expect("the view serialises");
            assert!(!theirs.contains(BIGOT), "p2 never reads the set Unit");
        }

        #[test]
        fn r1041_it_reveals_into_the_leftmost_open_zone_when_its_lane_is_taken() {
            let mut s = scenario(json!({
                "seed": "breaker-leftmost",
                "p1": {
                    "mana": 8,
                    "hand": [VANILLA, BREAKER],
                    "field": [{ "def": BREAKER, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                    "library": [VANILLA, VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
            }));
            let card = in_hand(&s, VANILLA);
            set_face_down(&mut s, &card.id, 2);
            let set = s.backrow(P1, 2).expect("set").id;
            s.end_turn().end_turn();
            reveal(&mut s, &set);
            assert!(s.backrow(P1, 2).is_none());
            assert_eq!(s.unit(P1, 3).expect("the leftmost open zone").def_id, VANILLA);
        }

        #[test]
        fn r1041_it_runs_its_cry_with_prompts_as_it_reveals() {
            let mut s = scenario(json!({
                "seed": "breaker-reveal-cry",
                "p1": { "mana": 8, "hand": [BIGOT, VANILLA], "field": [{ "def": BREAKER, "lane": 1 }], "library": [VANILLA, VANILLA] },
                "p2": { "hand": [VANILLA], "field": [{ "def": ENEMY, "lane": 1 }], "library": [VANILLA, VANILLA] },
            }));
            let enemy = s.unit(P2, 1).expect("the enemy").id;
            let bigot = in_hand(&s, BIGOT);
            set_face_down(&mut s, &bigot.id, 2);
            let set = s.backrow(P1, 2).expect("set").id;
            s.end_turn().end_turn();
            reveal(&mut s, &set);
            // The Cry's pick is asked as a prompt at the reveal.
            let open = s.state().pending.clone().expect("the Cry asks for its target");
            assert_eq!(open.player_id, P1);
            s.answer(json!([format!("instance:{enemy}")]));
            assert_eq!(s.unit(P1, 2).expect("revealed").def_id, BIGOT);
            s.expect_in_zone(&enemy, "graveyard");
        }

        #[test]
        fn r1041_with_no_open_unit_zone_its_reveal_waits_for_one() {
            let mut s = scenario(json!({
                "seed": "breaker-no-room",
                "p1": {
                    "mana": 8,
                    "hand": [VANILLA, HIT_JOB, VANILLA],
                    "field": [
                        { "def": BREAKER, "lane": 1 }, { "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 },
                        { "def": VANILLA, "lane": 4 }, { "def": VANILLA, "lane": 5 },
                    ],
                    "library": [VANILLA, VANILLA, VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
            }));
            let card = in_hand(&s, VANILLA);
            set_face_down(&mut s, &card.id, 2);
            let set = s.backrow(P1, 2).expect("set").id;
            s.end_turn().end_turn();
            // R783006: every unit zone is full, so its Reveal is not offered and is refused.
            assert!(!reveal_offered(&s, &set));
            s.expect_refused(|s| s.activate(set.as_str(), json!({ "ability": CLOAKED_REVEAL })));
            assert_eq!(s.backrow(P1, 2).expect("still set").id, set);
            // A zone opens, and it may reveal into it.
            let blocker = s.unit(P1, 5).expect("lane 5").id;
            s.play(
                HIT_JOB,
                json!({ "targets": [{ "pick": "instance", "instanceId": blocker }] }),
            );
            assert!(reveal_offered(&s, &set));
            reveal(&mut s, &set);
            assert!(s.backrow(P1, 2).is_none());
            assert_eq!(s.unit(P1, 5).expect("it animated").def_id, VANILLA);
        }

        #[test]
        fn r1040_its_hand_units_have_cloaked_only_while_it_acts_and_a_set_one_stays_set() {
            let mut s = scenario(json!({
                "seed": "breaker-left",
                "p1": { "mana": 8, "hand": [VANILLA, HIT_JOB, VANILLA], "field": [{ "def": BREAKER, "lane": 1 }], "library": [VANILLA, VANILLA] },
                "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
            }));
            let first = in_hand(&s, VANILLA);
            assert!(cloaked(&s, &first.id), "the Aura lends it Cloaked");
            assert!(!cloaked(&s, &in_hand(&s, HIT_JOB).id), "a Spell is not a Unit");
            set_face_down(&mut s, &first.id, 2);
            let set = s.backrow(P1, 2).expect("set").id;
            let other = in_hand(&s, VANILLA);
            let breaker = s.unit(P1, 1).expect("the Breaker").id;
            s.play(
                HIT_JOB,
                json!({ "targets": [{ "pick": "instance", "instanceId": breaker }] }),
            );
            s.expect_in_zone(&breaker, "graveyard");
            // The Aura ended: the Unit left in hand is no longer Cloaked, and has no face-down play.
            assert!(!cloaked(&s, &other.id));
            assert!(face_down_plays(&s, P1, &other.id).is_empty());
            // The one already set stays set, and reveals.
            s.end_turn().end_turn();
            reveal(&mut s, &set);
            assert_eq!(s.unit(P1, 2).expect("it still reveals").def_id, VANILLA);
        }

        #[test]
        fn r80_death_shuffles_no_prime_into_a_full_deck() {
            let library = vec![json!(VANILLA); LIBRARY_CAP as usize];
            let mut s = scenario(json!({
                "seed": "breaker-cap",
                "p1": { "mana": 8, "hand": [HIT_JOB], "field": [{ "def": BREAKER, "lane": 2 }], "library": library },
                "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 1 }] },
            }));
            let breaker = s.unit(P1, 2).expect("the Breaker").id;
            s.play(
                HIT_JOB,
                json!({ "targets": [{ "pick": "instance", "instanceId": breaker }] }),
            );
            s.expect_in_zone(&breaker, "graveyard");
            let library = s.pile(P1, "library");
            assert_eq!(library.len(), LIBRARY_CAP as usize, "the cap holds");
            assert!(!library.iter().any(|card| card.def_id == PRIME));
        }

        #[test]
        fn r1040_the_opponents_units_are_not_cloaked() {
            // p2's own turn, so its plays are listed at all; the Breaker acts on p1's side.
            let s = scenario(json!({
                "seed": "breaker-foe",
                "active": "p2",
                "p1": { "mana": 8, "hand": [VANILLA], "field": [{ "def": BREAKER, "lane": 1 }], "library": [VANILLA] },
                "p2": { "mana": 8, "hand": [VANILLA], "library": [VANILLA] },
            }));
            let enemy = s
                .hand(P2)
                .iter()
                .find(|card| card.def_id == VANILLA)
                .cloned()
                .unwrap();
            assert!(!cloaked(&s, &enemy.id));
            assert!(legal_actions(s.state(), P2).into_iter().any(|action| {
                matches!(&action, ActionBody::Play { instance_id, .. } if instance_id == &enemy.id)
            }));
            assert!(face_down_plays(&s, P2, &enemy.id).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1042_radiant_nerfs_twice_with_divine_shield() {
            // The Radiant face carries Divine Shield.
            let def = crate::card_def(BREAKER);
            assert!(def.radiant.keywords.contains(&Keyword::DivineShield));
            let mut s = spread("breaker-cry-radiant", true);
            let breaker = s
                .hand(P1)
                .iter()
                .find(|card| card.def_id == BREAKER)
                .cloned()
                .unwrap();
            let before_p1 = cnky_ids(&s, P1);
            let before_p2 = cnky_ids(&s, P2);
            // Each Printer at (2), as before patch v0.3.10 made it (3): a Nerf that lifts the cost has
            // two steps left under the cap of (4), so both Nerfs change something.
            for id in before_p1.iter().chain(before_p2.iter()) {
                s.card_mut(id.as_str()).cost_mod = -1;
            }
            s.play(&breaker.id, json!({ "zone": 2 }));
            let degraded = degraded_ids(&s);
            for id in before_p1.iter().chain(before_p2.iter()) {
                assert_eq!(degraded.iter().filter(|hit| *hit == id).count(), 2, "{id}");
            }
            assert!(!degraded.contains(&breaker.id));
        }
    }
}
