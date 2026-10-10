//! C+ #38.1 Solarius Prime (SPEC §8.7 row 38.1): (4) Unit, Token (printed Epic), 9/5 → 18/10.
//!   Base:    "Spell Damage +3. Cry: Cast {casts} random Spells. Each aims at enemies when it harms
//!            and at your side when it helps."
//!   Radiant: "Spell Damage +7. Cry: Cast {casts} random Radiant Spells. Each aims at enemies when it
//!            harms and at your side when it helps."
//! E12's random casts (R452, R656): non-token Spells of every set (R380), repeats allowed (R60), every
//! choice random, each target pick aimed by its declaration — enemies when it harms, friends when it
//! helps; X is the current mana, at least 1 (R348). Its own Spell Damage (the catalog keyword) raises
//! their hits, since it is on the field during its Cry.

use jackioh_engine::effects::{CastRandomArgs, CastRandomCount, CastRandomQuery, cast_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-038-1";

fn cast_random_spells(count: i32, radiant: bool) -> Effect {
    cast_random(CastRandomArgs {
        query: CastRandomQuery::Fixed(json_as(json!({ "type": "Spell" }))),
        count: Some(CastRandomCount::Fixed(count)),
        radiant: Some(radiant),
        target_enemies: Some(true),
        afterward: None,
    })
}

fn solarius_prime(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| vec![cast_random_spells(param(&*ctx, "casts"), radiant)])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: solarius_prime(false),
        radiant: solarius_prime(true),
    }
}

// C+ #38.1 Solarius Prime (SPEC §8.7): Cry casts 5 random non-token Spells of any set (R380),
// aimed by declaration (R656), each a play (R70) raised by Spell Damage, additions hidden (R97).
// Tests read casts off the event stream at depth 1 (until `cardResolved`), handling nested casts.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PRIME: &str = "classicplus-038-1";
    const MENACE: &str = "core-019";
    const FILLER: &str = "core-005";

    fn seeds() -> Vec<String> {
        (1..=12).map(|i| format!("prime-{i}")).collect()
    }

    use crate::scenario;

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| serde_json::to_value(event).expect("an event is JSON")).collect()
    }

    fn view(s: &Scenario, seat: PlayerId) -> Value {
        serde_json::to_value(s.view(seat)).expect("a view is JSON")
    }

    fn text(value: &Value, key: &str) -> String {
        value[key].as_str().unwrap_or_default().to_string()
    }

    /// One `cardAnnounced` made during the Prime's play, and how deep in nested casts it was made.
    struct Cast {
        event: Value,
        depth: usize,
    }

    /// Everything announced during the Prime's play, each with how deep in nested casts it was made.
    fn casts_during(s: &Scenario, prime_id: &str) -> Vec<Cast> {
        let mut open: Vec<String> = Vec::new();
        let mut out: Vec<Cast> = Vec::new();
        let mut inside = false;
        for event in events_json(s) {
            let kind = text(&event, "type");
            let instance_id = text(&event, "instanceId");
            if kind == "cardAnnounced" {
                if instance_id == prime_id {
                    inside = true;
                } else if inside {
                    out.push(Cast { event: event.clone(), depth: open.len() });
                }
                if inside {
                    open.push(instance_id);
                }
            } else if (kind == "cardResolved" || kind == "countered") && inside {
                if let Some(at) = open.iter().rposition(|id| *id == instance_id) {
                    open.remove(at);
                }
                if instance_id == prime_id {
                    inside = false;
                }
            }
        }
        out
    }

    /// What each of the Prime's casts dealt the enemy hero in all, by cast: a Spell's damage is raised by
    /// the Spell Damage on its side, a split's total once (each of its hits is 1) and a single hit whole.
    fn hero_damage_by_cast(s: &Scenario, prime: &str) -> Vec<i64> {
        let casts: IndexSet<String> = casts_during(s, prime).iter().map(|cast| text(&cast.event, "instanceId")).collect();
        let mut totals: IndexMap<String, i64> = IndexMap::new();
        // Only while the Prime stands as it was played: the first event that names it after its own play
        // (a destroy, a bounce, a Transform, a Vanilla, a keyword change …) ends what its Spell Damage covers.
        let own = ["cardAnnounced", "cardPlayed", "summoned", "cardResolved"];
        let events = events_json(s);
        let changed = events.iter().position(|event| {
            event.get("instanceId").and_then(Value::as_str) == Some(prime) && !own.contains(&text(event, "type").as_str())
        });
        let window = match changed {
            Some(at) => &events[..at],
            None => &events[..],
        };
        for event in window {
            if event["type"] != "damage" {
                continue;
            }
            let Some(source) = event["sourceId"].as_str() else {
                continue;
            };
            if !casts.contains(source) {
                continue;
            }
            if event["targetId"] != "hero-p2" {
                continue;
            }
            *totals.entry(source.to_string()).or_insert(0) += event["amount"].as_i64().unwrap_or(0);
        }
        totals.into_values().collect()
    }

    #[derive(Default)]
    struct PrimeOptions {
        radiant: bool,
        p2_health: Option<i32>,
        casts: Option<i32>,
    }

    /// Play a Prime; returns the game and the Prime's id (a token: a Flood it casts can end it, R11).
    fn play_prime(seed: &str, opts: PrimeOptions) -> (Scenario, String) {
        let mut p2 = json!({ "hand": [FILLER], "field": [MENACE], "library": [FILLER, FILLER, FILLER] });
        if let Some(health) = opts.p2_health {
            p2["health"] = json!(health);
        }
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": PRIME, "radiant": opts.radiant }, FILLER],
                "field": [MENACE],
                "library": [FILLER, FILLER, FILLER, FILLER, FILLER],
            },
            "p2": p2,
        }));
        let prime = s.card(PRIME).id.clone();
        if let Some(casts) = opts.casts {
            let live = find_instance_mut(s.state_mut(), &prime).expect("the Prime in hand");
            step_param(live, "casts", casts);
        }
        s.play(PRIME, json!({}));
        // A cast Spell may hand the opponent a choice of their own (C #8 Pickle's modes): theirs to make,
        // never answered at random (R452 drives only the caster's). Answer it so the casts can go on.
        let mut guard = 0;
        while guard < 20 {
            let Some(pending) = s.state().pending.clone() else {
                break;
            };
            if pending.player_id != P2 {
                break;
            }
            let take = std::cmp::max(1, pending.min) as usize;
            let keys: Vec<String> = pending.options.iter().take(take).map(|option| option.key.clone()).collect();
            s.answer(json!(keys));
            guard += 1;
        }
        (s, prime)
    }

    #[test]
    fn is_a_prime_token_named_solarius_prime_spaced_as_c_46_1_felinor_flagbearer_prime_is_patch_v0_2_y() {
        let def = crate::card_def(PRIME);
        assert_eq!(def.name, "Solarius Prime");
        assert!(def.token);
        assert_eq!(def.tags, vec![Tag::Prime, Tag::Token]);
    }

    mod base {
        use super::*;

        #[test]
        fn prints_spell_damage_3() {
            let s = scenario(json!({ "p1": { "field": [PRIME] } }));
            assert!(s.stats(PRIME).keywords.contains(&Keyword::SpellDamage { n: 3 }));
        }

        #[test]
        fn r70_r452_its_cry_casts_exactly_5_spells_one_after_another_free_with_no_prompt_of_its_casters() {
            for seed in seeds() {
                let (s, prime) = play_prime(&seed, PrimeOptions::default());
                let own: Vec<Cast> = casts_during(&s, &prime).into_iter().filter(|cast| cast.depth == 1).collect();
                assert_eq!(own.len(), 5, "{seed}");
                for Cast { event, .. } in &own {
                    assert_eq!(event["player"], "p1");
                    assert_eq!(event["costPaid"], 0);
                    assert_eq!(event["cardType"], "Spell");
                    assert!(!def_of(Some(s.state()), &text(event, "defId")).token);
                }
                assert!(s.state().pending.is_none(), "{seed}");
            }
        }

        #[test]
        fn r380_r387_the_spells_come_from_every_set_never_a_token_and_never_itself() {
            let mut sets: BTreeSet<String> = BTreeSet::new();
            for seed in seeds() {
                let (s, prime) = play_prime(&seed, PrimeOptions::default());
                for Cast { event, .. } in casts_during(&s, &prime) {
                    assert_ne!(text(&event, "defId"), PRIME);
                    sets.insert(def_of(Some(s.state()), &text(&event, "defId")).set.to_string());
                }
            }
            assert_eq!(sets.into_iter().collect::<Vec<_>>(), vec!["Classic", "Classic+", "Core"]);
        }

        #[test]
        fn r656_each_pick_aims_by_its_declaration_harm_never_at_your_hero_help_never_at_an_enemy() {
            // The helpful Spells a random cast can pick: their target declarations aim "help".
            let help: IndexSet<&str> =
                ["classic-003", "classicplus-010", "classicplus-057", "classicplus-071", "core-047", "core-063"].into_iter().collect();
            let mut aimed = 0;
            for seed in seeds() {
                let (s, prime) = play_prime(&seed, PrimeOptions::default());
                let mut mine: IndexSet<String> = IndexSet::new();
                mine.insert("hero-p1".to_string());
                mine.insert(prime.clone());
                mine.extend(s.pile(P1, "graveyard").into_iter().map(|card| card.id));
                let mut foe: IndexSet<String> = IndexSet::new();
                foe.insert("hero-p2".to_string());
                foe.extend(s.state().players.p2.units.iter().flatten().flatten().map(|card| card.id.clone()));
                foe.extend(s.pile(P2, "graveyard").into_iter().map(|card| card.id));
                for Cast { event, .. } in casts_during(&s, &prime) {
                    let def_id = text(&event, "defId");
                    for target in event["targets"].as_array().cloned().unwrap_or_default() {
                        let target = target.as_str().unwrap_or_default().to_string();
                        if help.contains(def_id.as_str()) {
                            assert!(!foe.contains(&target), "{seed} {def_id}");
                        } else {
                            assert!(target != "hero-p1", "{seed} {def_id}");
                        }
                        if !mine.contains(&target) {
                            aimed += 1;
                        }
                    }
                }
            }
            assert!(aimed > 0);
        }

        #[test]
        fn r70_each_cast_is_a_play_the_turns_log_counts_all_of_them() {
            let (s, prime) = play_prime(&seeds()[0], PrimeOptions::default());
            let casts = casts_during(&s, &prime);
            let played = &s.state().players.p1.turn_log.played_ids;
            for Cast { event, .. } in &casts {
                assert!(played.contains(&text(event, "instanceId")));
            }
        }

        #[test]
        fn its_own_spell_damage_raises_the_casts_hits_each_cast_deals_the_enemy_hero_at_least_4_in_all() {
            let mut checked = 0;
            for seed in seeds() {
                let (s, prime) = play_prime(&seed, PrimeOptions::default());
                for total in hero_damage_by_cast(&s, &prime) {
                    assert!(total >= 4, "{seed}");
                    checked += 1;
                }
            }
            assert!(checked > 0);
        }

        #[test]
        fn a_game_that_ends_midway_stops_the_rest_nothing_is_announced_after_the_game_is_over() {
            let mut ended = 0;
            for seed in seeds() {
                let (s, _) = play_prime(&seed, PrimeOptions { p2_health: Some(1), ..PrimeOptions::default() });
                let events = events_json(&s);
                let Some(over) = events.iter().position(|event| event["type"] == "gameOver") else {
                    continue;
                };
                ended += 1;
                assert!(!events[over..].iter().any(|event| event["type"] == "cardAnnounced"), "{seed}");
                assert_eq!(serde_json::to_value(s.state().result).expect("a result is JSON")["winner"], "p1");
            }
            assert!(ended > 0);
        }

        #[test]
        fn r97_the_casts_are_public_a_card_a_cast_adds_to_your_hand_is_hidden_from_the_opponent() {
            for seed in seeds() {
                let (s, prime) = play_prime(&seed, PrimeOptions::default());
                let theirs = view(&s, P2);
                // A cast is public: its announce names it, unless the card now lies where p2 may not read it.
                let hand: Vec<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
                let mut unread: IndexSet<String> = hand.iter().cloned().collect();
                unread.extend(s.pile(P1, "library").into_iter().map(|card| card.id));
                let casts: IndexSet<String> = casts_during(&s, &prime).iter().map(|cast| text(&cast.event, "instanceId")).collect();
                let events = theirs["events"].as_array().cloned().unwrap_or_default();
                for event in &events {
                    let instance_id = text(event, "instanceId");
                    if event["type"] == "cardAnnounced" && casts.contains(&instance_id) && !unread.contains(&instance_id) {
                        assert_ne!(event["defId"], "hidden", "{seed}");
                    }
                }
                for event in &events {
                    if event["type"] == "addedToHand" || event["type"] == "drawn" {
                        let instance_id = text(event, "instanceId");
                        if hand.contains(&instance_id) {
                            assert_ne!(text(event, "defId"), s.card(&instance_id).def_id, "{seed}");
                        }
                    }
                }
                assert!(!theirs["opponent"]["hand"].is_array());
            }
        }

        #[test]
        fn r386_the_casts_read_through_param_a_degrade_makes_4() {
            let (s, prime) = play_prime(&seeds()[1], PrimeOptions { casts: Some(-1), ..PrimeOptions::default() });
            assert_eq!(casts_during(&s, &prime).into_iter().filter(|cast| cast.depth == 1).count(), 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn prints_spell_damage_7() {
            let s = scenario(json!({ "p1": { "field": [{ "def": PRIME, "radiant": true }] } }));
            assert!(s.stats(PRIME).keywords.contains(&Keyword::SpellDamage { n: 7 }));
        }

        #[test]
        fn casts_5_radiant_spells() {
            for seed in seeds().into_iter().take(6) {
                let (s, prime) = play_prime(&seed, PrimeOptions { radiant: true, ..PrimeOptions::default() });
                let own: Vec<Cast> = casts_during(&s, &prime).into_iter().filter(|cast| cast.depth == 1).collect();
                assert_eq!(own.len(), 5, "{seed}");
                let events = events_json(&s);
                for Cast { event, .. } in &own {
                    // Read off its resolution, since a cast may since have ceased to exist (a Transform).
                    let resolved = events
                        .iter()
                        .find(|e| e["type"] == "cardResolved" && e["instanceId"] == event["instanceId"]);
                    assert!(resolved.is_some_and(|e| e["radiant"] == true), "{seed} {}", text(event, "defId"));
                }
            }
        }

        #[test]
        fn its_spell_damage_7_raises_the_casts_each_deals_the_enemy_hero_at_least_8_in_all() {
            let mut checked = 0;
            for seed in seeds() {
                let (s, prime) = play_prime(&seed, PrimeOptions { radiant: true, ..PrimeOptions::default() });
                for total in hero_damage_by_cast(&s, &prime) {
                    assert!(total >= 8, "{seed}");
                    checked += 1;
                }
            }
            assert!(checked > 0);
        }
    }
}
