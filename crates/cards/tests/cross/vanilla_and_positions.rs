//! Port of `packages/cards/test/vanilla-and-positions.test.ts`.
//!
//! A Vanilla copy has no text at all, and R46's knock-down reports only a real switch (SPEC §6.3
//! Vanilla, §6.2, R46, R49, R91, R115). Found by the polish-4 edge-case hunt
//! (docs/polish/4-edge-cases.md, lens L3); every case here failed before its fix.
//!
//! #61 Prejudiced Postdoc summons a Vanilla copy (R57, R23). §6.3's Vanilla "clears printed keywords
//! and scripts", and R115 says so of every hook, so the copy keeps none of the card's text: not Deft
//! Duelist's second exertion, not Spikey Pillow's "cannot be in Defense Position", not Fed Fauci's
//! on-damage trigger and not radiant Right-house defender's Death.
//!
//! Round 9, lens "keywords and layers": §6.1's keywords are a set, so a keyword two sources give — a
//! printed one an aura grants again, a Taunt unit's own Taunt in Defense Position — is listed once in
//! the view (§10.4, §10.8), Armor apart, which sums across its sources.
//!
//! Round 10 (lenses "engine invariants" and "keywords and layers"): the Taunt R46's knock-down takes
//! from a unit already in Attack Position went with no event (§10.3), and a unit's view did not say
//! its text was gone, so a client rendering it showed a Vanilla copy's scripted text (R243).

use jackioh_engine::testkit::*;

const RIGHT_HOUSE: &str = "core-003";
const VANILLA: &str = "core-008";
const HIT_JOB: &str = "core-016";
const POINTMASTER: &str = "core-020";
const HINDER: &str = "core-021";
const TRUE_STRIKE: &str = "core-044";
const DUELIST: &str = "core-045";
const POSTDOC: &str = "core-061";
const PILLOW: &str = "core-065-1";
const ROCK: &str = "core-066";
const FAUCI: &str = "core-091";
const LIBRARY: [&str; 4] = [VANILLA, VANILLA, VANILLA, VANILLA];

use super::scenario;

fn at(card: &CardInstance) -> Value {
    json!([{ "pick": "instance", "instanceId": card.id }])
}

fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match s.unit(player, lane) {
        Some(card) => card,
        None => panic!("setup: {player} should hold a unit in lane {lane}"),
    }
}

fn offers_switch(s: &Scenario, card: &CardInstance) -> bool {
    let live = s.card(card);
    legal_actions(s.state(), live.controller)
        .iter()
        .any(|action| matches!(action, ActionBody::SwitchPosition { instance_id } if *instance_id == live.id))
}

/// `keywords.map((k) => k.kind)`, by the kind's printed name.
fn kinds(keywords: &[Keyword]) -> Vec<String> {
    keywords.iter().map(|keyword| keyword.kind().as_str().to_string()).collect()
}

fn last_of_type(s: &Scenario, kind: GameEventType) -> Vec<GameEvent> {
    s.last_events().iter().filter(|event| event.event_type() == kind).cloned().collect()
}

mod r115_a_vanilla_copy_keeps_none_of_the_cards_text {
    use super::*;

    #[test]
    fn r115_r49_a_vanilla_copy_of_deft_duelist_has_one_exertion_not_two_s6_3_vanilla() {
        let mut s = scenario(json!({
            "p1": { "hand": [POSTDOC, HINDER], "field": [{ "def": DUELIST, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [HINDER], "field": [{ "def": POINTMASTER, "lane": 5 }], "library": LIBRARY },
        }));
        let duelist = s.card(DUELIST).clone();
        s.play(POSTDOC, json!({ "zone": 2, "targets": at(&duelist) }));
        let copy = unit_at(&s, PlayerId::P1, 3);
        assert_eq!(copy.def_id, DUELIST);
        assert!(copy.vanilla);

        s.end_turn().end_turn(); // p2's turn, then back to p1: the copy is no longer sick.
        assert_eq!(s.state().active, PlayerId::P1);

        s.attack(&copy, "hero");
        // One exertion: having attacked, the plain copy cannot also switch (§4.1, R6).
        assert!(!offers_switch(&s, &copy));
        s.expect_refused_with(|s| s.switch_position(&copy), "already acted");
        // The original keeps its text, so it still has both.
        s.attack(&duelist, "hero");
        assert!(offers_switch(&s, &duelist));
    }

    #[test]
    fn r115_a_vanilla_copy_of_spikey_pillow_may_enter_defense_position_s6_3_vanilla_s7_s4_1() {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": POSTDOC, "radiant": true }, HINDER],
                "field": [{ "def": PILLOW, "lane": 1 }],
                "library": LIBRARY,
            },
            "p2": { "hand": [HINDER], "library": LIBRARY },
        }));
        let pillow = s.card(PILLOW).clone();
        s.play(POSTDOC, json!({ "zone": 2, "targets": at(&pillow) }));
        let copy = unit_at(&s, PlayerId::P1, 3);
        assert_eq!(copy.def_id, PILLOW);
        assert!(copy.vanilla);

        assert!(offers_switch(&s, &copy));
        s.switch_position(&copy);
        assert_eq!(s.card(&copy).position, Some(Position::Def));
        // The Pillow itself still cannot.
        assert!(!offers_switch(&s, &pillow));
    }

    #[test]
    fn r115_a_vanilla_copy_of_fed_fauci_gains_no_plague_counter_when_damaged_s6_3_vanilla_s8_c91() {
        let mut s = scenario(json!({
            "p1": {
                "hand": [POSTDOC, TRUE_STRIKE, HINDER],
                "field": [{ "def": FAUCI, "lane": 1 }],
                "mana": 10,
                "library": LIBRARY,
            },
            "p2": { "library": LIBRARY },
        }));
        let fauci = s.card(FAUCI).clone();
        s.play(POSTDOC, json!({ "zone": 2, "targets": at(&fauci) }));
        let copy = unit_at(&s, PlayerId::P1, 3);
        assert_eq!(copy.def_id, FAUCI);
        assert!(copy.vanilla);

        s.play(TRUE_STRIKE, json!({ "targets": at(&copy) }));
        assert_eq!(s.card(&copy).damage, 4);
        assert_eq!(s.card(&copy).counters.plague.unwrap_or(0), 0);
    }

    #[test]
    fn r115_r57_a_vanilla_copy_of_radiant_right_house_defender_fires_no_death_s6_3_vanilla_s6_2() {
        let mut s = scenario(json!({
            "p1": {
                "hand": [POSTDOC, TRUE_STRIKE, HINDER],
                "field": [{ "def": RIGHT_HOUSE, "radiant": true, "lane": 1 }],
                "mana": 10,
                "library": LIBRARY,
            },
            "p2": { "library": LIBRARY },
        }));
        let original = s.card(RIGHT_HOUSE).clone();
        s.play(POSTDOC, json!({ "zone": 2, "targets": at(&original) }));
        let copy = unit_at(&s, PlayerId::P1, 3);
        assert_eq!(copy.def_id, RIGHT_HOUSE);
        assert!(copy.vanilla);
        assert!(copy.radiant);

        s.play(TRUE_STRIKE, json!({ "targets": at(&copy) }));
        s.expect_in_zone(&copy, "graveyard");
        // Nothing came back into the copy's lane or anywhere else: only the original stands.
        let right_houses: Vec<String> = s
            .state()
            .players
            .p1
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == RIGHT_HOUSE)
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(right_houses, vec![s.card(RIGHT_HOUSE).id.clone()]);
    }
}

mod r46_r91_the_knock_down_reports_a_switch_only_when_there_is_one {
    use super::*;

    #[test]
    fn r91_r46_on_an_indestructible_unit_already_in_attack_position_emits_no_position_switched() {
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, HINDER], "library": LIBRARY },
            "p2": { "field": [{ "def": ROCK, "lane": 1, "position": "ATK" }], "library": LIBRARY },
        }));
        let rock = s.card(ROCK).clone();
        s.play(HIT_JOB, json!({ "targets": at(&rock) }));

        // R46 still happened: on the field, in Attack Position, its Taunt suppressed for the turn.
        s.expect_in_zone(&rock, "field");
        assert_eq!(s.card(&rock).position, Some(Position::Atk));
        assert_eq!(s.card(&rock).taunt_suppressed_turn, Some(s.state().turn));
        let switched: Vec<GameEvent> = s
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::PositionSwitched { instance_id, .. } if *instance_id == rock.id))
            .cloned()
            .collect();
        assert_eq!(switched, Vec::<GameEvent>::new());
    }

    #[test]
    fn r46_on_an_indestructible_unit_in_defense_position_still_switches_it_and_says_so() {
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, HINDER], "library": LIBRARY },
            "p2": { "field": [{ "def": ROCK, "lane": 1, "position": "DEF" }], "library": LIBRARY },
        }));
        let rock = s.card(ROCK).clone();
        s.play(HIT_JOB, json!({ "targets": at(&rock) }));

        assert_eq!(s.card(&rock).position, Some(Position::Atk));
        assert!(s
            .last_events()
            .contains(&GameEvent::PositionSwitched { instance_id: rock.id.clone(), position: Position::Atk }));
    }

    // Found by the probe's silent-change check (a top unit's view changed in an action whose events
    // name neither it nor anything on the board around it): #55 radiant Lava Golem and #56 radiant
    // Jilliax, both printing Taunt beside Indestructible, lost Taunt to R46 with nothing in the
    // action's stream saying so, and R46 was given its `keywordGranted … lost` report. R347 (patch
    // v0.1.1) has since taken Taunt off every Indestructible unit, so such a unit never has a Taunt
    // for a knock-down to take: it shows none before the destroy and none after, and so reports none.
    #[test]
    fn r46_r347_r91_an_indestructible_unit_given_taunt_shows_none_so_shrugging_off_a_destroy_changes_nothing_a_view_shows_s10_3() {
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, HINDER], "library": LIBRARY },
            "p2": { "field": [{ "def": ROCK, "lane": 1, "position": "ATK" }], "library": LIBRARY },
        }));
        let rock = s.card(ROCK).clone();
        // A granted Taunt, as #63 Plastic Surgery's roll can give it: R347 holds it off.
        find_instance_mut(s.state_mut(), &rock.id).expect("The Rock stands").granted_keywords.push(Keyword::Taunt);
        assert!(!kinds(&s.stats(&rock).keywords).contains(&"Taunt".to_string()));
        let shown = s.view(PlayerId::P1).opponent.units[0].clone().expect("p2's unit zone 1 holds The Rock");
        assert!(!kinds(&shown.keywords).contains(&"Taunt".to_string()));

        // #16 Hit Job: "Destroy target unit". R46: an Indestructible unit that would be destroyed
        // switches to Attack Position (it already is in it).
        s.play(HIT_JOB, json!({ "targets": at(&rock) }));

        s.expect_in_zone(&rock, "field");
        assert!(!kinds(&s.stats(&rock).keywords).contains(&"Taunt".to_string()));
        assert_eq!(last_of_type(&s, GameEventType::PositionSwitched), Vec::<GameEvent>::new());
        assert_eq!(last_of_type(&s, GameEventType::KeywordGranted), Vec::<GameEvent>::new());
    }

    #[test]
    fn r46_r91_a_unit_with_no_taunt_to_lose_reports_none_the_rock_in_attack_position_is_knocked_down_in_silence() {
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, HINDER], "library": LIBRARY },
            "p2": { "field": [{ "def": ROCK, "lane": 1, "position": "ATK" }], "library": LIBRARY },
        }));
        let rock = s.card(ROCK).clone();
        assert!(!kinds(&s.stats(&rock).keywords).contains(&"Taunt".to_string()));
        s.play(HIT_JOB, json!({ "targets": at(&rock) }));
        assert_eq!(last_of_type(&s, GameEventType::KeywordGranted), Vec::<GameEvent>::new());
    }
}

// ---------------------------------------------------------------------------
// Round 9: a unit's keywords are a set (§6.1, §10.4, §10.8)
// ---------------------------------------------------------------------------

const TIMMY: &str = "core-011"; // Unit, 1: Rush, First Strike
const WEAPONS: &str = "core-014"; // Field Spell: your units have +4 attack, Rush, First Strike

mod a_units_keywords_are_a_set_s6_1_s10_4_s10_8 {
    use super::*;

    #[test]
    fn s6_1_the_view_lists_each_keyword_a_unit_has_once_however_many_sources_give_it_tempo_timmy_under_jlockeeds_weapons_a_taunt_unit_in_defense_position_s10_4_s10_8() {
        let s = scenario(json!({
            "p1": {
                "hand": [HINDER],
                "field": [
                    { "def": TIMMY, "lane": 1 },
                    { "def": RIGHT_HOUSE, "lane": 2, "position": "DEF" },
                ],
                "backrow": [WEAPONS],
                "library": LIBRARY,
            },
            "p2": { "hand": [HINDER], "library": LIBRARY },
        }));

        let units = s.view(PlayerId::P1).you.units;
        let kinds_in = |lane: usize| -> Vec<String> {
            let Some(Some(unit)) = units.get(lane - 1) else {
                panic!("no unit in lane {lane}");
            };
            let mut found: Vec<String> = kinds(&unit.keywords).into_iter().filter(|kind| kind != "Armor").collect();
            found.sort();
            found
        };

        // Timmy prints Rush and First Strike, and the Weapons aura grants both again: two keywords, not four.
        assert_eq!(kinds_in(1), ["First Strike", "Rush"].map(str::to_string).to_vec());
        // Right-house defender prints Taunt and Defense Position grants it: one Taunt among its keywords.
        assert_eq!(
            kinds_in(2),
            ["Divine Shield", "First Strike", "Reborn", "Rush", "Taunt"].map(str::to_string).to_vec()
        );
    }
}

// ---------------------------------------------------------------------------
// Round 10: a Vanilla unit's view says its text is gone (R243)
// ---------------------------------------------------------------------------

mod r243_r115_a_vanilla_units_view_says_its_text_is_gone_s6_3_vanilla_s10_8 {
    use super::*;

    #[test]
    fn r243_a_vanilla_copy_of_fed_fauci_is_marked_vanilla_in_both_seats_views_and_the_original_is_not() {
        let mut s = scenario(json!({
            "p1": { "hand": [POSTDOC, HINDER], "field": [{ "def": FAUCI, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [HINDER], "library": LIBRARY },
        }));
        let fauci = s.card(FAUCI).clone();
        s.play(POSTDOC, json!({ "zone": 2, "targets": at(&fauci) }));
        let copy = unit_at(&s, PlayerId::P1, 3);
        assert_eq!(copy.def_id, FAUCI);
        assert!(copy.vanilla);

        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = s.view(viewer);
            let side = if viewer == PlayerId::P1 { &view.you } else { &view.opponent };
            let original = side.units.first().cloned().flatten();
            let vanilla_copy = side.units.get(2).cloned().flatten();
            // Same definition, same printed keywords (Rush) on the original and none on the copy — but
            // Fauci's scripted text (the Plague Counter trigger, the start-of-turn mana) is text too, and
            // only the view can tell a client the copy has none of it.
            assert_eq!(
                vanilla_copy.as_ref().map(|unit| unit.def_id.clone()),
                original.as_ref().map(|unit| unit.def_id.clone())
            );
            assert_eq!(
                vanilla_copy.as_ref().and_then(|unit| unit.vanilla),
                Some(true),
                "{viewer}'s view marks the copy Vanilla"
            );
            assert_eq!(original.as_ref().and_then(|unit| unit.vanilla), None);
        }
    }
}
