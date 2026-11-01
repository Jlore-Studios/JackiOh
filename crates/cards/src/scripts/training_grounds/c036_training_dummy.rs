//! TG #36 Training Dummy (SPEC §8.11 row 36): (1) Unit, Epic, 0/3 → 0/6, a card of the held Training
//! Grounds set (R1420, R2194000).
//!   Base:    "Can't be in Defense Position. / Death: Add {cards|random Radiant Training Grounds card|random
//!            Radiant Training Grounds cards} to your hand." — cards 1
//!   Radiant: "Death: Add {cards|…} to your hand." — cards 2
//! Engine: C+ #48 Jlockeed's Lobbyist line for line. The restriction is Can't Defend (R2657000; C+ #65.1's `neverDefense` flag through v0.5.9f),
//!   which the switch action, `legalActions` and a switch made as an effect all honor (R20); the Radiant
//!   face drops it. The Death's pool names the set (`{ "set": "Training Grounds" }`), so it works while
//!   the set is held (R2194001): every non-token card of the set but this one (R387), each Radiant, one
//!   draw of the match rng per card (R60); a full hand burns the rest (§2.4). Tunes: cards 1 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-036";

/// "Death: Add {cards} random Radiant Training Grounds cards to your hand."
fn add_radiant_cards(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![add_random_from_catalog(json_as(json!({
        "query": { "set": "Training Grounds" },
        "count": param(&*ctx, "cards"),
        "radiant": true,
    })))]
}

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(add_radiant_cards)),
        ..Script::default()
    };
    let radiant = Script {
        death: Some(hook(add_radiant_cards)),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

/// R767000: through v0.5.9f "Can't be in Defense Position" was the static flag `never_defense` on its
/// base face, before R2657000 made it the keyword Can't Defend; `KEPT_SCRIPTS` folds those replays under
/// it.
pub fn script_v0_5_9f() -> CardScripts {
    crate::never_defense_kept(script(), true, false)
}

// TG #36 Training Dummy — SPEC §8.11 row 36, BUILD M13 row TG 36: "cannot switch to Defense (action
// refused, an effect's switch leaves it in Attack); Death: one random Radiant TG card to your hand; never
// itself; hidden from the opponent (R97). radiant two; may be in Defense Position".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const VANILLA: &str = "core-008"; // (1) Mr. Vanilla, 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const SWITCH_ALL: &str = "core-048"; // (0) Spell: switch the position of every Unit.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.

    fn board(radiant: bool, seed: &str, hand: Value) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }, VANILLA],
                "hand": hand,
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "field": [VANILLA], "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    /// Hit Job the Dummy; the cards its Death added to p1's hand.
    fn kill_dummy(s: &mut Scenario) -> Vec<CardInstance> {
        let me = s.unit_at(P1, 1).id;
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": me }] }));
        s.expect_in_zone(me.as_str(), "graveyard");
        let added: Vec<String> = s
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player, instance_id, .. } if *player == P1 => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        added.iter().map(|id| s.card(id.as_str()).clone()).collect()
    }

    /// R2194001: a non-token card of the Training Grounds set, never this one (R387), Radiant.
    fn from_the_pool(card: &CardInstance) {
        let def = catalog::def_of(None, &card.def_id);
        assert_eq!(def.set, SetName::TrainingGrounds, "{}", card.def_id);
        assert!(!def.token, "{}", card.def_id);
        assert_ne!(card.def_id, ID);
        assert!(card.radiant);
        assert_eq!(card.zone, Zone::Hand { player: P1 });
    }

    mod base {
        use super::*;

        #[test]
        fn r20_a_switch_to_defense_position_is_refused() {
            let mut s = board(false, "training-dummy", json!([FILLER]));
            let me = s.unit_at(P1, 1);
            s.expect_refused_with(|s| s.switch_position(&me), "Defense Position");
            assert_eq!(s.stats(me.id.as_str()).position, Position::Atk);
        }

        #[test]
        fn r20_a_switch_all_effect_leaves_it_in_attack() {
            let mut s = board(false, "training-dummy", json!([SWITCH_ALL, FILLER]));
            s.play(SWITCH_ALL, json!({}));
            assert_eq!(s.stats(s.unit_at(P1, 1).id.as_str()).position, Position::Atk);
            assert_eq!(s.stats(s.unit_at(P1, 2).id.as_str()).position, Position::Def);
        }

        #[test]
        fn r2194001_its_death_adds_one_random_radiant_training_grounds_card() {
            let mut s = board(false, "training-dummy", json!([HIT_JOB, FILLER]));
            let added = kill_dummy(&mut s);
            assert_eq!(added.len(), 1);
            from_the_pool(&added[0]);
        }

        #[test]
        fn r387_it_never_adds_itself() {
            for n in 0..24 {
                let mut s = board(true, &format!("training-dummy-{n}"), json!([HIT_JOB, FILLER]));
                for card in kill_dummy(&mut s) {
                    from_the_pool(&card);
                }
            }
        }

        #[test]
        fn r97_the_opponent_sees_only_that_a_card_reached_your_hand() {
            let mut s = board(false, "training-dummy", json!([HIT_JOB, FILLER]));
            let added = kill_dummy(&mut s);
            let theirs = s.view(P2);
            assert!(!serde_json::to_string(&theirs).expect("serializes").contains(&format!("\"{}\"", added[0].id)));
            assert!(theirs.events.iter().any(|event| matches!(
                event,
                GameEvent::AddedToHand { player, instance_id, def_id, .. }
                    if *player == P1 && instance_id == "hidden" && def_id == "hidden"
            )));
        }

        #[test]
        fn cards_reads_through_param() {
            let mut s = board(false, "training-dummy", json!([HIT_JOB, FILLER]));
            let me = s.unit_at(P1, 1).id;
            assert_eq!(crate::upgrade_number(&mut s, &me, "cards"), 2);
            assert_eq!(kill_dummy(&mut s).len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2194001_its_death_adds_two_random_radiant_training_grounds_cards() {
            let mut s = board(true, "training-dummy", json!([HIT_JOB, FILLER]));
            let added = kill_dummy(&mut s);
            assert_eq!(added.len(), 2);
            added.iter().for_each(from_the_pool);
        }

        #[test]
        fn it_is_0_6_and_may_switch_to_defense_position() {
            let mut s = board(true, "training-dummy", json!([FILLER]));
            let me = s.unit_at(P1, 1);
            s.expect_stats(&me, json!({ "attack": 0, "health": 6 }));
            s.switch_position(&me);
            assert_eq!(s.stats(me.id.as_str()).position, Position::Def);
        }
    }
}
