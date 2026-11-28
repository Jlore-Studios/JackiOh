//! FvH #100 Call to Chaos (Felinors vs. Humans Edition) (SPEC §8.14 row 100, FV2 #100; R28, R87, R423,
//! R436, R5262000), a card of the held Felinors vs Humans set (R5235000). (4) Spell, Call to Chaos, Legendary.
//!   Base:    "One random effect: summon {felinors} random Felinors with Death or Reborn; summon {humans}
//!            random Humans with an Aura; give enemy Units on the field, in their hand and in their deck
//!            Fragile; cast {plagiarized} random Plagiarized cards; fill your board with random Radiant
//!            Felinors and your opponent's with random Humans; fill your board with random Radiant Humans
//!            and your opponent's with random Felinors; make enemy Units and Field Spells in their hand, deck
//!            and graveyard GIGA; summon a Radiant Call to Chaos Vending Machine; summon a Felinor Golem and a
//!            Human Golem; cast a random Call to Chaos."
//!   Radiant: "Three different random effects, resolved in the order listed: …" (the same ten).
//!
//! Core #95's subsystem (`subsystems::call_to_chaos`) with this edition's table (`CHAOS_FVH_EFFECTS`,
//! `subsystems/call_to_chaos_fvh.rs`): the roll, the announcement both players read (R436), the chain cap
//! counting every edition's casts (R28) and the Radiant's order (R423) are shared by every edition. The
//! face is passed explicitly, as S+ #142's file does. R5262000 reads the ten entries: the pools are the
//! non-token cards of every set, held ones included; entry 3 reaches enemy Units only, entry 7 enemy Units
//! and Field Spells off the field; entry 10 never draws this card (as R3570005). It carries the Call to
//! Chaos tag, as every edition does, so the `call-to-chaos` pool draws it (R3869001). Its Golems are
//! FvH #100.1 and #100.2.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-100";

pub fn script() -> CardScripts {
    // §8.14: "One random effect".
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(false),
                table: Some(subsystems::CHAOS_FVH_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    // §8.14, R423: three different random effects of the ten, resolved in the order the list writes them.
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(true),
                table: Some(subsystems::CHAOS_FVH_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// FvH #100 Call to Chaos (Felinors vs. Humans Edition) — SPEC §8.14 row 100, R5262000.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{card_def, filler, js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // Mr. Vanilla: a Human Unit
    const STOCKPILE: &str = "core-005"; // Stockpile: a Spell
    /// A sturdy Unit with no tag.
    const BODY: &str = "stand-in (1) 2/9";
    const FIELD_SPELL: &str = "stand-in (1) Field Spell";
    const VENDING_MACHINE: &str = "felinorsvshumans-115";
    const FELINOR_GOLEM: &str = "felinorsvshumans-100-1";
    const HUMAN_GOLEM: &str = "felinorsvshumans-100-2";

    const SEED: &str = "chaos-fvh";
    /// How many cursors the sweeps of the table walk.
    const SWEEP: u32 = 800;
    /// How many games the pool sweeps play.
    const GAMES: u32 = 8;
    /// The printed count of entries 1, 2 and 4.
    const PRINTED_COUNT: i32 = 5;

    fn rolled_names(seed: &str, cursor: u32, radiant: bool) -> Vec<&'static str> {
        let mut rng = Rng::new(seed, cursor);
        subsystems::roll_chaos_effects(&mut rng, radiant, Some(subsystems::CHAOS_FVH_EFFECTS))
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn cursor_in(seed: &str, name: &str) -> u32 {
        cursor_where(seed, |cursor| {
            rolled_names(seed, cursor, false).first() == Some(&name)
        })
    }

    fn label_of(name: &str) -> &'static str {
        subsystems::CHAOS_FVH_EFFECTS
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.label)
            .unwrap_or_else(|| panic!("no entry {name}"))
    }

    /// p1 holds this card and a spare Vanilla (which keeps §2.5's auto-end away), with 10 mana.
    fn side(radiant: bool) -> Value {
        json!({
            "hand": [{ "def": ID, "radiant": radiant }, VANILLA],
            "library": filler(6),
            "mana": 10,
        })
    }

    fn chaos_in(seed: &str, cursor: u32, p1: Value, p2: Value) -> Scenario {
        let mut p2 = p2;
        if p2.get("library").is_none() {
            p2["library"] = filler(6);
        }
        let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
        s.state_mut().rng_cursor = cursor;
        s.play(ID, json!({}));
        s
    }

    /// The base face, rolling `entry`.
    fn chaos(entry: &str, p1: Value, p2: Value) -> Scenario {
        chaos_in(SEED, cursor_in(SEED, entry), p1, p2)
    }

    /// The base face, rolling `entry`, in a game seeded `seed`.
    fn chaos_seeded(seed: &str, entry: &str) -> Scenario {
        chaos_in(
            seed,
            cursor_in(seed, entry),
            side(false),
            json!({ "hand": [VANILLA] }),
        )
    }

    fn has(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> bool {
        jackioh_engine::layers::unit_has(s.state(), card, kind)
    }

    /// Whether a card was given `kind` (`granted_keywords`, as S+ #100 Plesiosize's tests read it off the field).
    fn granted(card: &CardInstance, kind: KeywordKind) -> bool {
        card.granted_keywords.iter().any(|keyword| keyword.kind() == kind)
    }

    /// The definitions summoned since setup, in order.
    fn summoned(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// The definitions played since setup but this card, in order.
    fn played_after(s: &Scenario) -> Vec<String> {
        s.events_of("cardPlayed")
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed { def_id, .. } if def_id != ID => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn has_death(def_id: &str) -> bool {
        jackioh_engine::scripts::registered_entry(def_id).is_some_and(|scripts| scripts.base.death.is_some())
    }

    fn has_aura(def_id: &str) -> bool {
        jackioh_engine::scripts::registered_entry(def_id).is_some_and(|scripts| {
            scripts.base.aura.is_some()
                || scripts.base.hand_aura.is_some()
                || scripts.base.cost_aura.is_some()
        })
    }

    #[test]
    fn r436_r5262000_each_label_is_a_clause_of_both_printed_faces_in_order() {
        crate::register_all();
        let def = card_def(ID);
        for face in [FaceKind::Base, FaceKind::Radiant] {
            let text = fill_params(&def, face, None).to_lowercase();
            let mut from = 0;
            for entry in subsystems::CHAOS_FVH_EFFECTS {
                let label = entry.label.to_lowercase();
                let at = text[from..].find(&label).map(|at| at + from);
                assert!(at.is_some(), "{:?}: {}", face, entry.label);
                from = at.unwrap_or(from) + label.len();
            }
        }
        assert_eq!(subsystems::CHAOS_FVH_EFFECTS.len(), 10);
    }

    #[test]
    fn r3869001_it_carries_the_call_to_chaos_tag() {
        crate::register_all();
        assert!(card_def(ID).tags.contains(&Tag::CallToChaos));
    }

    #[test]
    fn each_of_the_ten_entries_is_rolled_over_the_seeds() {
        let mut seen: Vec<&str> = Vec::new();
        for cursor in 0..SWEEP {
            seen.extend(rolled_names(SEED, cursor, false));
        }
        for entry in subsystems::CHAOS_FVH_EFFECTS {
            assert!(seen.contains(&entry.name), "{} never rolled", entry.name);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r436_both_players_read_the_rolled_clause() {
            let mut s = chaos("golems", side(false), json!({ "hand": [VANILLA] }));
            for viewer in [P1, P2] {
                let rolled: Vec<Value> = s
                    .view(viewer)
                    .events
                    .iter()
                    .filter(|event| event.event_type().as_str() == "chaosRolled")
                    .map(js)
                    .collect();
                assert_eq!(rolled.len(), 1);
                assert_eq!(rolled[0]["effects"], json!([label_of("golems")]));
            }
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r5262000_r5239002_entry_felinors_summons_felinor_units_with_a_death_or_reborn() {
            crate::register_all();
            for n in 0..GAMES {
                let s = chaos_seeded(&format!("{SEED}-felinors-{n}"), "felinors");
                let made = nonempty(summoned(&s), "a summoned Felinor");
                // The first summon is the entry's first draw; a summoned card's own text may add more.
                let def = card_def(&made[0]);
                assert!(def.tags.contains(&Tag::Felinor), "{}", def.id);
                assert_eq!(def.type_, CardType::Unit, "{}", def.id);
                assert!(!def.token, "{}", def.id);
                assert!(
                    has_death(&def.id) || has_keyword(&def.base.keywords, KeywordKind::Reborn),
                    "{} has neither a Death nor Reborn",
                    def.id
                );
                assert!(
                    s.unit(P1, 2).is_some(),
                    "game {n}: each of the five is its own summon"
                );
            }
        }

        #[test]
        fn r5262000_r5239002_entry_humans_summons_human_units_with_an_aura() {
            crate::register_all();
            for n in 0..GAMES {
                let s = chaos_seeded(&format!("{SEED}-humans-{n}"), "humans");
                let made = nonempty(summoned(&s), "a summoned Human");
                let def = card_def(&made[0]);
                assert!(def.tags.contains(&Tag::Human), "{}", def.id);
                assert_eq!(def.type_, CardType::Unit, "{}", def.id);
                assert!(!def.token, "{}", def.id);
                assert!(has_aura(&def.id), "{} has no Aura", def.id);
            }
        }

        #[test]
        fn r5262000_a_lowered_count_summons_fewer() {
            let seed = format!("{SEED}-count");
            let cursor = cursor_in(&seed, "felinors");
            let mut s = scenario(json!({
                "seed": seed,
                "p1": side(false),
                "p2": { "hand": [VANILLA], "library": filler(6) },
            }));
            step_param(s.card_mut(ID), "felinors", -(PRINTED_COUNT - 1));
            s.state_mut().rng_cursor = cursor;
            s.play(ID, json!({}));
            assert!(s.unit(P1, 1).is_some(), "one Felinor");
            assert!(
                s.unit(P1, 2).is_none(),
                "and only one, its `felinors` lowered to 1"
            );
        }

        #[test]
        fn r5262000_r5236010_entry_fragile_reaches_enemy_units_on_the_field_in_hand_and_in_deck() {
            let mut p1 = side(false);
            p1["field"] = json!([BODY]);
            let s = chaos(
                "fragile",
                p1,
                json!({
                    "hand": [VANILLA, FIELD_SPELL],
                    "field": [BODY],
                    "library": [VANILLA, VANILLA, STOCKPILE, STOCKPILE],
                }),
            );
            let mut theirs: Vec<CardInstance> = vec![s.unit_at(P2, 1)];
            theirs.extend(s.hand(P2).into_iter().filter(|card| card.def_id == VANILLA));
            theirs.extend(
                s.pile(P2, "library")
                    .into_iter()
                    .filter(|card| card.def_id == VANILLA),
            );
            assert_eq!(theirs.len(), 4, "the field's, the hand's and the deck's two");
            for card in nonempty(theirs, "an enemy Unit card") {
                assert!(
                    has(&s, &card, KeywordKind::Fragile),
                    "{} {:?}",
                    card.def_id,
                    card.zone
                );
            }
            assert!(!has(&s, &s.unit_at(P1, 1), KeywordKind::Fragile), "your own Unit");
            let spell = s
                .hand(P2)
                .into_iter()
                .find(|card| card.def_id == FIELD_SPELL)
                .expect("their Field Spell");
            assert!(!granted(&spell, KeywordKind::Fragile), "a Field Spell is no Unit");
        }

        #[test]
        fn r5262000_r5235002_entry_plagiarized_casts_plagiarized_cards() {
            crate::register_all();
            for n in 0..GAMES {
                let s = chaos_seeded(&format!("{SEED}-plagiarized-{n}"), "plagiarized");
                let played = nonempty(played_after(&s), "a cast Plagiarized card");
                // The first play after this card is the entry's first cast.
                assert!(
                    card_def(&played[0]).tags.contains(&Tag::Plagiarized),
                    "game {n}: {}",
                    played[0]
                );
            }
        }

        /// The unit-row summons onto `player`'s side since setup: each one's lane, definition and instance.
        fn summons_onto(s: &Scenario, player: PlayerId) -> Vec<(i32, String, String)> {
            s.events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Summoned {
                        player: at,
                        row: Row::Units,
                        lane,
                        def_id,
                        instance_id,
                        ..
                    } if *at == player => Some((*lane, def_id.clone(), instance_id.clone())),
                    _ => None,
                })
                .collect()
        }

        fn prints_giga(def_id: &str) -> bool {
            has_keyword(&card_def(def_id).base.keywords, KeywordKind::Giga)
        }

        /// R1080, R5262000: every unit zone of each side is filled as the entry resolves, read off the
        /// summons, since a summoned card's own Aura may clear a side once it stands (an exiling Aura).
        /// R3546000: a GIGA covers the zone beside its own, and one drawn for the last zone fits nowhere
        /// there, so its summon fails and that zone alone may stay open.
        fn fills(entry: &str, own: Tag, theirs: Tag) {
            // Whether each side's last zone took a summon in some game: the GIGA case is rare, never the rule.
            let mut last_filled = [false; 2];
            for n in 0..GAMES {
                let s = chaos_seeded(&format!("{SEED}-{entry}-{n}"), entry);
                for (seat, (player, tag, radiant)) in
                    [(P1, own, true), (P2, theirs, false)].into_iter().enumerate()
                {
                    let summons = summons_onto(&s, player);
                    let reached = |lane: i32| {
                        summons
                            .iter()
                            .any(|(at, def_id, _)| *at == lane || (*at == lane - 1 && prints_giga(def_id)))
                    };
                    for lane in 1..UNIT_ZONES {
                        assert!(reached(lane), "game {n}: {player:?}'s lane {lane} is filled");
                    }
                    last_filled[seat] |= reached(UNIT_ZONES);
                    for (lane, def_id, instance_id) in nonempty(summons.clone(), "the fill's summons") {
                        let def = card_def(&def_id);
                        assert_eq!(def.type_, CardType::Unit, "{def_id}");
                        assert!(
                            def.tags.contains(&tag),
                            "game {n}: {def_id} in {player:?}'s lane {lane}"
                        );
                        assert_eq!(
                            s.card(instance_id.as_str()).radiant,
                            radiant,
                            "game {n}: {def_id}"
                        );
                    }
                }
            }
            assert_eq!(last_filled, [true, true], "the last zone is filled in most games");
        }

        #[test]
        fn r5262000_r1080_entry_radiant_felinors_fills_your_board_with_radiant_felinors_and_theirs_with_humans()
         {
            fills("radiantFelinors", Tag::Felinor, Tag::Human);
        }

        #[test]
        fn r5262000_r1080_entry_radiant_humans_fills_your_board_with_radiant_humans_and_theirs_with_felinors()
        {
            fills("radiantHumans", Tag::Human, Tag::Felinor);
        }

        #[test]
        fn r5262000_r3546002_entry_giga_makes_enemy_units_and_field_spells_off_the_field_giga() {
            let s = chaos(
                "giga",
                side(false),
                json!({
                    "hand": [VANILLA, FIELD_SPELL, STOCKPILE],
                    "field": [BODY],
                    "library": [VANILLA, STOCKPILE, STOCKPILE],
                    "graveyard": [VANILLA],
                }),
            );
            let mut reached: Vec<CardInstance> = s
                .hand(P2)
                .into_iter()
                .filter(|card| card.def_id == VANILLA || card.def_id == FIELD_SPELL)
                .collect();
            reached.extend(
                s.pile(P2, "library")
                    .into_iter()
                    .filter(|card| card.def_id == VANILLA),
            );
            reached.extend(
                s.pile(P2, "graveyard")
                    .into_iter()
                    .filter(|card| card.def_id == VANILLA),
            );
            assert_eq!(reached.len(), 4);
            for card in nonempty(reached, "an enemy Unit or Field Spell card") {
                assert!(
                    granted(&card, KeywordKind::Giga),
                    "{} {:?}",
                    card.def_id,
                    card.zone
                );
            }
            assert!(
                !granted(&s.unit_at(P2, 1), KeywordKind::Giga),
                "the field's are not"
            );
            let spell = s
                .hand(P2)
                .into_iter()
                .find(|card| card.def_id == STOCKPILE)
                .expect("their Spell");
            assert!(!granted(&spell, KeywordKind::Giga), "a Spell is neither");
        }

        #[test]
        fn r5262000_entry_vending_machine_summons_a_radiant_vending_machine_into_your_backrow() {
            let mut s = chaos("vendingMachine", side(false), json!({ "hand": [VANILLA] }));
            let machine = s
                .events()
                .iter()
                .find_map(|event| match event {
                    GameEvent::Summoned {
                        instance_id, def_id, ..
                    } if def_id == VENDING_MACHINE => Some(instance_id.clone()),
                    _ => None,
                })
                .expect("the Vending Machine was summoned");
            assert!(s.card(machine.as_str()).radiant);
            assert_eq!(s.card(machine.as_str()).controller, P1);
            s.expect_in_zone(machine.as_str(), "field");
        }

        #[test]
        fn r5262000_entry_golems_summons_both_golems() {
            let s = chaos("golems", side(false), json!({ "hand": [VANILLA] }));
            assert_eq!(s.unit_at(P1, 1).def_id, FELINOR_GOLEM);
            assert_eq!(s.unit_at(P1, 2).def_id, HUMAN_GOLEM);
            assert!(!s.unit_at(P1, 1).radiant && !s.unit_at(P1, 2).radiant);
        }

        /// R3570005's recursion: the tenth entry casts a Call to Chaos, never this edition.
        #[test]
        fn r5262000_r3570005_entry_recast_casts_another_call_to_chaos() {
            crate::register_all();
            let mut plays = 0;
            for cursor in (0..SWEEP).filter(|at| rolled_names(SEED, *at, false).first() == Some(&"recast")) {
                let s = chaos_in(SEED, cursor, side(false), json!({ "hand": [VANILLA] }));
                let played = nonempty(played_after(&s), "the recast's cast");
                assert!(
                    card_def(&played[0]).tags.contains(&Tag::CallToChaos),
                    "cursor {cursor}"
                );
                assert_ne!(played[0], ID, "cursor {cursor}");
                plays += 1;
                if plays >= GAMES {
                    break;
                }
            }
            assert_eq!(plays, GAMES);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r423_the_radiant_face_rolls_three_different_entries_in_list_order() {
            let cursor = cursor_where(SEED, |at| {
                rolled_names(SEED, at, true) == ["fragile", "vendingMachine", "golems"]
            });
            let s = chaos_in(
                SEED,
                cursor,
                side(true),
                json!({ "hand": [VANILLA, VANILLA], "library": filler(6) }),
            );
            assert_eq!(
                js(&s.events_of("chaosRolled")[0])["effects"],
                json!([
                    label_of("fragile"),
                    label_of("vendingMachine"),
                    label_of("golems")
                ])
            );
            for card in nonempty(s.hand(P2), "an enemy hand card") {
                assert!(has(&s, &card, KeywordKind::Fragile), "{}", card.def_id);
            }
            assert!(summoned(&s).iter().any(|id| id == VENDING_MACHINE));
            assert_eq!(s.unit_at(P1, 1).def_id, FELINOR_GOLEM);
            assert_eq!(s.unit_at(P1, 2).def_id, HUMAN_GOLEM);
            let order: Vec<&str> = subsystems::CHAOS_FVH_EFFECTS
                .iter()
                .map(|entry| entry.name)
                .collect();
            for at in 0..SWEEP {
                let names = rolled_names(SEED, at, true);
                assert_eq!(names.len(), 3);
                let places: Vec<usize> = names
                    .iter()
                    .map(|name| order.iter().position(|each| each == name).unwrap_or(usize::MAX))
                    .collect();
                assert!(places.windows(2).all(|pair| pair[0] < pair[1]), "{names:?}");
            }
        }

        /// The most actions one play's prompts may take before the test gives up on it.
        const ANSWER_BOUND: usize = 200;
        /// How many seeded Radiant plays the loop sweep makes.
        const LOOP_GAMES: u32 = 20;

        #[test]
        fn r28_many_seeds_end_every_play_within_the_caps() {
            for n in 0..LOOP_GAMES {
                let mut s = scenario(json!({
                    "seed": format!("{SEED}-loops-{n}"),
                    "p1": side(true),
                    "p2": { "hand": [VANILLA], "library": filler(6) },
                }));
                s.play(ID, json!({}));
                for _ in 0..ANSWER_BOUND {
                    let Some(pending) = s.state().pending.clone() else {
                        break;
                    };
                    if s.state().result.is_some() {
                        break;
                    }
                    let action = legal_actions(s.state(), pending.player_id)
                        .into_iter()
                        .next()
                        .expect("a prompt always has an answer");
                    s.act(action, pending.player_id, "answer the prompt");
                }
                assert!(
                    s.state().pending.is_none() || s.state().result.is_some(),
                    "seed {n}: the play completes"
                );
            }
        }
    }
}
