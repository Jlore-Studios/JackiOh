//! C+ #41 KY's Constant (SPEC §8.7 row 41, B3.4, R60, R81, R129, R386). (1) Spell, KY, Rare.
//!   Choose a card in your hand. Change a random number on it to 3.
//!   Radiant: Choose a card in your hand. Discover a number on it and change that number to 3.
//!
//! "A number on a card" is Degrade and Upgrade's (R386, `numbersOn`): its own cost (never an X), its
//! attack and health, a numbered keyword's value, a declared number; an Immutable card has none. The
//! card is a declared hand pick (R81) among your other hand cards with such a number not already 3;
//! with none the Spell fizzles and still counts as played. Base: one of them at random (R60; one
//! choice draws nothing, R129). Radiant: a Discover of up to 3 of them, shown to you only. The change
//! is the engine's `setNumber`: `tuning` (the cost through `costMod`), kept in every zone and by a copy,
//! reported by a `numberChanged` hidden as the card is.

use jackioh_engine::effects::{chosen_tuning_number, discover_number, set_number};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-041";

/// "change … to 3".
const CONSTANT: i32 = 3;
/// §6.3 Discover: up to 3 options.
const OFFERED: i32 = 3;

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::hand(1, 1, json!({ "of": ["hand"], "excludeSelf": true, "check": "number" }))]
}

fn target_checks() -> IndexMap<&'static str, TargetCheck> {
    IndexMap::from([(
        "number",
        target_check(|a| {
            a.candidate
                .is_some_and(|candidate| numbers_on(a.state, candidate).iter().any(|entry| entry.value != CONSTANT))
        }),
    )])
}

/// The Radiant face's continuation: the number picked off the Discover becomes 3.
fn picked(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    match chosen_tuning_number(ctx) {
        None => vec![],
        Some(pick) => vec![set_number(json_as(json!({
            "instanceId": pick.instance_id,
            "which": pick.which,
            "value": CONSTANT,
        })))],
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        target_checks: target_checks(),
        cry: Some(hook(|_ctx| {
            vec![set_number(json_as(json!({ "target": { "of": "chosen" }, "which": "random", "value": CONSTANT })))]
        })),
        ..Script::default()
    };
    let radiant = Script {
        targets: targets(),
        target_checks: target_checks(),
        cry: Some(hook(|_ctx| {
            vec![discover_number(json_as(json!({
                "target": { "of": "chosen" },
                "value": CONSTANT,
                "count": OFFERED,
                "step": "picked",
            })))]
        })),
        resume: IndexMap::from([("picked", hook(picked))]),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C+ #41 KY's Constant — SPEC §8.7 row 41, B3.4, R60, R81, R97, R129, R386, BUILD M9 row C+ 41.
// The setting itself is the engine's `setNumber` / `discoverNumber` (packages/engine/test/effects-tune.test.ts).
#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CONSTANT: &str = "classicplus-041";
    const TIMMY: &str = "core-011"; // (1) 3/3: its cost is its one number that is not 3
    const VANILLA: &str = "core-008"; // (1) 4/4
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt
    const ARMORED: &str = "core-025"; // (4) 7/7 Armor 7
    const DUPLICATING: &str = "core-012"; // (2) 3/4, Cry: summon a copy of this
    const HIT_JOB: &str = "core-016"; // (3) Spell: no number other than 3
    const DIVIDEND: &str = "core-024"; // (X) Spell: an X is never a number
    const GIFT: &str = "classicplus-042-1"; // (4) Field Spell with mana 1, discards 1, heal 5
    const FILLER: &str = "core-005";

    use crate::scenario;

    fn constant(hand: &[&str], radiant_face: bool, seed: Option<&str>) -> Scenario {
        let mut cards = vec![json!({ "def": CONSTANT, "radiant": radiant_face })];
        cards.extend(hand.iter().map(|id| json!(id)));
        let mut opts = json!({ "p1": { "hand": cards }, "p2": { "hand": [FILLER] } });
        if let Some(seed) = seed {
            opts["seed"] = json!(seed);
        }
        scenario(opts)
    }

    fn numbers(s: &Scenario, card: &str) -> BTreeMap<String, i32> {
        numbers_on(s.state(), s.card(card)).into_iter().map(|entry| (entry.id, entry.value)).collect()
    }

    fn expected(entries: &[(&str, i32)]) -> BTreeMap<String, i32> {
        entries.iter().map(|(key, value)| (key.to_string(), *value)).collect()
    }

    fn pick(s: &Scenario, card: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": s.card(card).id }] })
    }

    /// The hand cards `legalActions` offers as the Constant's pick.
    fn offered(s: &Scenario) -> Vec<String> {
        let constant_id = s.card(CONSTANT).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == constant_id => targets
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|target| match target {
                        Selection::Instance { instance_id } => Some(s.card(&instance_id).def_id.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                _ => vec![],
            })
            .collect()
    }

    fn sorted(mut list: Vec<String>) -> Vec<String> {
        list.sort();
        list
    }

    fn view(s: &Scenario, seat: PlayerId) -> Value {
        serde_json::to_value(s.view(seat)).expect("a view is JSON")
    }

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| serde_json::to_value(event).expect("an event is JSON")).collect()
    }

    #[test]
    fn r81_both_faces_declare_one_hand_pick_never_itself() {
        assert_eq!(crate::card_def(CONSTANT).id, CONSTANT);
        let scripts = super::script();
        assert_eq!(scripts.base.targets, scripts.radiant.targets);
        assert_eq!(
            serde_json::to_value(&scripts.base.targets).expect("declarations are JSON"),
            json!([{ "kind": "hand", "min": 1, "max": 1, "filter": { "of": ["hand"], "excludeSelf": true, "check": "number" } }])
        );
    }

    mod the_pick_r81_r386 {
        use super::*;

        #[test]
        fn r386_offers_only_hand_cards_with_a_number_other_than_3_never_one_whose_numbers_are_all_3_nor_an_x() {
            let s = constant(&[TIMMY, HIT_JOB, DIVIDEND, VANILLA], false, None);
            assert_eq!(sorted(offered(&s)), sorted(vec![TIMMY.to_string(), VANILLA.to_string()]));
        }

        #[test]
        fn r386_an_immutable_card_has_no_number_to_change_so_it_is_never_offered() {
            let mut s = constant(&[VANILLA, TIMMY], false, None);
            // As E38's grantKeywordCards leaves a hand card it makes Immutable.
            let vanilla = s.card(VANILLA).id.clone();
            find_instance_mut(s.state_mut(), &vanilla).expect("Mr. Vanilla in hand").granted_keywords.push(Keyword::Immutable);
            assert!(numbers_on(s.state(), s.card(VANILLA)).is_empty());
            assert_eq!(offered(&s), vec![TIMMY]);
        }

        #[test]
        fn with_no_such_hand_card_the_spell_fizzles_and_still_counts_as_played() {
            let mut s = constant(&[HIT_JOB], false, None);
            s.play(CONSTANT, json!({}));
            s.expect_in_zone(CONSTANT, "graveyard").expect_events(json!(["cardPlayed", "cardResolved"]));
            assert!(!events_json(&s).iter().any(|event| event["type"] == "numberChanged"));
            assert_eq!(numbers(&s, HIT_JOB), expected(&[("cost", 3)]));
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r129_one_number_not_3_tempo_timmys_cost_becomes_3_raised_through_costmod_with_no_random_draw() {
            let mut s = constant(&[TIMMY], false, None);
            let cursor = s.state().rng_cursor;
            let targets = pick(&s, TIMMY);
            s.play(CONSTANT, targets);
            assert_eq!(numbers(&s, TIMMY), expected(&[("cost", 3), ("attack", 3), ("health", 3)]));
            assert_eq!(s.card(TIMMY).cost_mod, 2);
            assert_eq!(effective_cost(s.state(), s.card(TIMMY), Default::default()), 3);
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn r60_one_random_number_of_several_becomes_3_any_of_them_over_seeds_the_others_stay() {
            let mut changed: BTreeSet<String> = BTreeSet::new();
            for n in 0..20 {
                let seed = format!("constant-{n}");
                let mut s = constant(&[VANILLA], false, Some(&seed));
                let targets = pick(&s, VANILLA);
                s.play(CONSTANT, targets);
                let now = numbers(&s, VANILLA);
                let moved: Vec<&str> = [("cost", 1), ("attack", 4), ("health", 4)]
                    .into_iter()
                    .filter(|(key, was)| now.get(*key) != Some(was))
                    .map(|(key, _)| key)
                    .collect();
                assert_eq!(moved.len(), 1);
                assert_eq!(now.get(moved[0]), Some(&3));
                changed.insert(moved[0].to_string());
            }
            assert_eq!(changed.into_iter().collect::<Vec<_>>(), vec!["attack", "cost", "health"]);
        }

        #[test]
        fn r386_it_lowers_as_well_midrange_menaces_attack_or_health_becomes_3() {
            let mut s = constant(&[MENACE], false, None);
            let targets = pick(&s, MENACE);
            s.play(CONSTANT, targets);
            let now = numbers(&s, MENACE);
            let mut pair = vec![now.get("attack").copied(), now.get("health").copied()];
            pair.sort();
            assert_eq!(pair, vec![Some(3), Some(9)]);
        }

        #[test]
        fn r386_a_numbered_keyword_and_a_declared_number_are_numbers_too() {
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for n in 0..24 {
                let seed = format!("constant-kinds-{n}");
                let mut s = constant(&[ARMORED, GIFT], false, Some(&seed));
                let target = if n % 2 == 0 { ARMORED } else { GIFT };
                let before = numbers(&s, target);
                let targets = pick(&s, target);
                s.play(CONSTANT, targets);
                let now = numbers(&s, target);
                for (key, was) in &before {
                    if now.get(key) != Some(was) {
                        seen.insert(key.clone());
                    }
                }
            }
            assert!(seen.contains("keyword:Armor"));
            assert!(seen.iter().any(|key| key.starts_with("param:")));
        }

        #[test]
        fn r386_the_change_is_tuning_it_stays_as_the_card_is_played_and_a_copy_keeps_it() {
            for n in 0..20 {
                let seed = format!("constant-copy-{n}");
                let mut s = constant(&[DUPLICATING, FILLER], false, Some(&seed));
                let targets = pick(&s, DUPLICATING);
                s.play(CONSTANT, targets);
                if numbers(&s, DUPLICATING).get("health") != Some(&3) {
                    continue;
                }
                s.play(DUPLICATING, json!({ "zone": 1 }));
                // Cry: summon a copy of this — both the card and its copy are 3/3.
                let first = s.unit(P1, 1).map(|card| card.id).unwrap_or_else(|| DUPLICATING.to_string());
                s.expect_stats(&first, json!({ "attack": 3, "health": 3 }));
                let second = s.unit(P1, 2).map(|card| card.id).unwrap_or_else(|| DUPLICATING.to_string());
                s.expect_stats(&second, json!({ "attack": 3, "health": 3 }));
                return;
            }
            panic!("no seed changed the health");
        }

        #[test]
        fn r97_the_opponents_view_names_neither_the_card_nor_the_number() {
            let mut s = constant(&[TIMMY], false, None);
            let targets = pick(&s, TIMMY);
            s.play(CONSTANT, targets);
            let changed = |seat: PlayerId| -> Option<Value> {
                view(&s, seat)["events"]
                    .as_array()
                    .and_then(|events| events.iter().find(|event| event["type"] == "numberChanged").cloned())
            };
            let mine = changed(P1).map_or(json!([]), |event| json!([event["defId"], event["key"], event["value"]]));
            let theirs = changed(P2)
                .map_or(json!([]), |event| json!([event["instanceId"], event["defId"], event["key"], event["value"]]));
            assert_eq!(mine, json!([TIMMY, "cost", 3]));
            assert_eq!(theirs, json!(["hidden", "hidden", "hidden", 0]));
            assert!(!serde_json::to_string(&s.view(P2)).expect("a view is JSON").contains(TIMMY));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn discover_up_to_3_different_numbers_on_the_card_shown_to_you_only_the_chosen_one_becomes_3() {
            let mut s = constant(&[ARMORED], true, None);
            let targets = pick(&s, ARMORED);
            s.play(CONSTANT, targets);
            let pending = s.state().pending.clone().expect("the Discover");
            assert_eq!(pending.kind, PromptKind::Discover);
            assert_eq!(pending.player_id, P1);
            assert_eq!(pending.options.len(), 3);
            assert_eq!(view(&s, P2)["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
            let option = pending
                .options
                .iter()
                .find(|entry| entry.key == "mode:health")
                .or_else(|| pending.options.first())
                .cloned()
                .expect("an option");
            let key = match &option.selection {
                Selection::Mode { option } => option.clone(),
                _ => String::new(),
            };
            s.answer(json!(option.key));
            assert_eq!(numbers(&s, ARMORED).get(&key), Some(&3));
        }

        #[test]
        fn r129_with_no_more_than_3_numbers_not_3_all_are_offered_and_nothing_is_drawn() {
            let mut s = constant(&[VANILLA], true, None);
            let cursor = s.state().rng_cursor;
            let targets = pick(&s, VANILLA);
            s.play(CONSTANT, targets);
            let keys: Vec<String> = s
                .state()
                .pending
                .as_ref()
                .map(|prompt| prompt.options.iter().map(|option| option.key.clone()).collect())
                .unwrap_or_default();
            assert_eq!(sorted(keys), vec!["mode:attack", "mode:cost", "mode:health"]);
            assert_eq!(s.state().rng_cursor, cursor);
            s.answer(json!("mode:cost"));
            assert_eq!(numbers(&s, VANILLA), expected(&[("cost", 3), ("attack", 4), ("health", 4)]));
        }

        #[test]
        fn s9_3_paused_on_the_discover_the_state_survives_json_and_answers_to_the_same_hash() {
            let mut s = constant(&[ARMORED], true, None);
            let targets = pick(&s, ARMORED);
            s.play(CONSTANT, targets);
            let thawed: GameState =
                serde_json::from_str(&serde_json::to_string(s.state()).expect("the state is JSON")).expect("and back");
            let pending = s.state().pending.clone();
            let action = Action::new(
                ActionBody::Answer {
                    choice_id: pending.as_ref().map(|prompt| prompt.id.clone()).unwrap_or_default(),
                    selection: pending
                        .as_ref()
                        .and_then(|prompt| prompt.options.first())
                        .map(|option| vec![option.selection.clone()])
                        .unwrap_or_default(),
                },
                P1,
                "constant-json",
            );
            let live = reduce(s.state(), &action);
            assert_eq!(live.error, None);
            assert!(live.state.pending.is_none());
            let armored = live
                .state
                .players
                .p1
                .hand
                .iter()
                .find(|card| card.def_id == ARMORED)
                .cloned()
                .unwrap_or_else(|| s.card(ARMORED).clone());
            assert_eq!(numbers_on(&live.state, &armored).iter().filter(|entry| entry.value == 3).count(), 1);
            assert_eq!(hash_state(&reduce(&thawed, &action).state), hash_state(&live.state));
        }

        #[test]
        fn with_no_such_hand_card_it_fizzles_asking_nothing() {
            let mut s = constant(&[HIT_JOB], true, None);
            s.play(CONSTANT, json!({}));
            assert!(s.state().pending.is_none());
            s.expect_in_zone(CONSTANT, "graveyard");
        }
    }
}
