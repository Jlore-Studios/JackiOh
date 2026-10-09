//! A Tribute's deaths and the targets they take with them (SPEC §6.3 Tribute, §10.5 steps 1, 2 and 5,
//! R68, R90, R101, R174). Also cited: §4.1.
//!
//!  - R68, R101: the tributed set dies together, its Death hooks in R68's order, so the order a play
//!    lists it in means nothing: `legalActions` offers each set once, `reduce` accepts any listing.
//!  - R174: a target the play's own Tribute sacrificed has left the field, so the effect aimed at it
//!    fizzles (§8 Conventions) instead of landing on a card in a graveyard.
//!  - §6.3 Vanilla, §7: a Sheep Token's "worth 2 Tributes" is its text, so a Vanilla copy of one is
//!    worth 1 like any other unit.
//!  - R119, R210: a play's arrivals are counted from the moment it begins, so the copies a tributed
//!    Cube's Death puts on the field at step 2 answer neither its step 4 (#41), its step 5 (#38) nor
//!    its step 7 (#33).

use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;

const STOCKPILE: &str = "core-005"; // keeps a hand non-empty, so no turn auto-ends (§2.5)
const GARY: &str = "core-004";
const RENO: &str = "core-053";
const LAVA_GOLEM: &str = "core-055";
const TWISTED_SORCERER: &str = "core-068";
const RADIANT_SAINTESS: &str = "core-081";
const RIGHT_HOUSE: &str = "core-003";
const CRAFT_A_CARD: &str = "core-099";
const POSTDOC: &str = "core-061"; // radiant: "choose any unit on the field; summon a Vanilla copy"
const SHEEP: &str = "core-t-sheep"; // "Worth 2 Tributes while on the field."

use super::scenario;

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("missing: {what}"),
    }
}

/// A tributed set as one string, whatever order it is listed in.
fn sorted_join(ids: &[String]) -> String {
    let mut sorted = ids.to_vec();
    sorted.sort();
    sorted.join(",")
}

/// Whether `legalActions` offers a play of `card` whose `tributes` are the set `pair` names.
fn offers_tribute_set(state: &GameState, card: &CardInstance, pair: &str) -> bool {
    legal_actions(state, PlayerId::P1)
        .iter()
        .any(|action| match action {
            ActionBody::Play {
                instance_id,
                tributes,
                ..
            } => *instance_id == card.id && sorted_join(tributes.as_deref().unwrap_or(&[])) == pair,
            _ => false,
        })
}

/// The ids of the three units `tribute_game` lists for its Tribute.
struct TributeIds {
    saintess: String,
    defender: String,
    gary: String,
}

mod r68_a_tribute_s_deaths_resolve_in_lane_order_whatever_order_the_play_lists_them_in {
    use super::*;

    /// Radiant Saintess in lane 1 and a Radiant Right-house defender in lane 2 pay Lava Golem's Tribute
    /// with a Gary. The defender's Death summons a base Right-house defender; the Saintess's Death makes
    /// her controller's other Units Radiant — so whether that summoned defender is Radiant says which
    /// Death ran first.
    fn tribute_game(order: impl Fn(&TributeIds) -> Vec<String>) -> Scenario {
        let mut g = scenario(json!({
            "p1": {
                "hand": [LAVA_GOLEM, STOCKPILE],
                "field": [
                    { "def": RADIANT_SAINTESS, "lane": 1 }, // Death: Make your other Units Radiant
                    { "def": RIGHT_HOUSE, "radiant": true, "lane": 2 }, // Reborn; Death: summon a base Right-house defender
                    { "def": GARY, "lane": 3 },
                    { "def": GARY, "lane": 4 },
                ],
            },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": RENO, "lane": 2 }] },
        }));
        let ids = TributeIds {
            saintess: must(g.unit(PlayerId::P1, 1), "Saintess").id.clone(),
            defender: must(g.unit(PlayerId::P1, 2), "Right-house defender").id.clone(),
            gary: must(g.unit(PlayerId::P1, 3), "Gary").id.clone(),
        };
        g.play(LAVA_GOLEM, json!({ "zone": 5, "tributes": order(&ids) }));
        g
    }

    #[test]
    fn r68_r101_radiant_saintess_lane_1_dies_before_the_right_house_defender_lane_2_whether_the_play_lists_them_in_lane_order_or_not_so_the_defender_its_death_summons_is_not_made_radiant()
     {
        let in_lane_order =
            tribute_game(|ids| vec![ids.saintess.clone(), ids.defender.clone(), ids.gary.clone()]);
        let defender_first =
            tribute_game(|ids| vec![ids.defender.clone(), ids.saintess.clone(), ids.gary.clone()]);

        for g in [&in_lane_order, &defender_first] {
            // The summoned base defender takes the leftmost free zone, the Saintess's lane 1 (R64); the
            // Reborn body keeps lane 2.
            let summoned = g
                .unit(PlayerId::P1, 1)
                .map(|card| (card.def_id.clone(), card.radiant));
            assert_eq!(summoned, Some((RIGHT_HOUSE.to_string(), false)));
            assert_eq!(g.unit(PlayerId::P1, 2).map(|card| card.radiant), Some(true));
        }
        assert_eq!(defender_first.last_events(), in_lane_order.last_events());
    }
}

mod r174_a_target_the_play_s_own_tribute_sacrificed_is_no_longer_a_target {
    use super::*;

    #[test]
    fn r174_r78_a_crafted_lava_golem_twisted_sorcerer_that_tributes_its_own_target_leaves_that_card_in_the_graveyard_undamaged_8_conventions()
     {
        let mut g = scenario(json!({
            "seed": "r3craft-1990", // the first Discover offers Lava Golem, the second Twisted Sorcerer (every set's Units, R380)
            "p1": {
                "hand": [CRAFT_A_CARD, RENO],
                "mana": 4,
                "field": [
                    { "def": GARY, "lane": 1 },
                    { "def": GARY, "lane": 2 },
                ],
            },
            "p2": { "hand": [RENO], "field": [{ "def": RENO, "lane": 1 }] },
        }));
        g.play(CRAFT_A_CARD, json!({}));
        g.answer(json!(LAVA_GOLEM));
        g.answer(json!(TWISTED_SORCERER));
        let card: CardInstance = must(
            g.hand(PlayerId::P1)
                .iter()
                .find(|held| held.def_id.starts_with("t-"))
                .cloned(),
            "the crafted card",
        );
        let first = must(g.unit(PlayerId::P1, 1), "Gary").id.clone();
        let second = must(g.unit(PlayerId::P1, 2), "Gary").id.clone();
        let reno = must(g.unit(PlayerId::P2, 1), "p2's Reno");
        let target = json!([{ "pick": "instance", "instanceId": reno.id }]);
        let before = g.events().len();

        // Lava Golem may tribute enemy units (§8 #55), and the Sorcerer may target any unit. Step 1
        // checks the two declarations each on its own (R90), so the play is legal; step 2 sacrifices
        // the Reno before step 5 resolves the Sorcerer's damage, which then has no unit to hit.
        g.play(
            &card.id,
            json!({ "zone": 3, "tributes": [first, second, reno.id], "targets": target }),
        );

        let hits = g.events()[before..]
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { target_id, .. } if *target_id == reno.id))
            .count();
        let now = g.card(&reno.id);
        assert_eq!((now.zone.z(), now.damage, hits), (ZoneName::Graveyard, 0, 0));
    }
}

mod s6_3_vanilla_a_vanilla_sheep_token_has_no_text_so_it_is_worth_1_tribute {
    use super::*;

    #[test]
    fn r101_s6_3_a_vanilla_copy_of_a_sheep_token_and_one_other_unit_do_not_pay_lava_golem_s_tribute_3_7() {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": POSTDOC, "radiant": true }, LAVA_GOLEM, STOCKPILE],
                "mana": 9,
                "field": [
                    { "def": SHEEP, "lane": 1 },
                    { "def": GARY, "lane": 2 },
                ],
            },
            "p2": { "hand": [STOCKPILE] },
        }));
        let sheep = must(s.unit(PlayerId::P1, 1), "Sheep Token");
        let gary = must(s.unit(PlayerId::P1, 2), "Gary");
        assert_eq!(tribute_value_of(s.state(), &sheep), 2);

        // Radiant Prejudiced Postdoc: "any unit" → a Vanilla copy of the Sheep in lane 3.
        s.play(
            POSTDOC,
            json!({ "zone": 4, "targets": [{ "pick": "instance", "instanceId": sheep.id }] }),
        );
        let copy = must(s.unit(PlayerId::P1, 3), "the Vanilla copy");
        assert_eq!((copy.def_id.clone(), copy.vanilla), (SHEEP.to_string(), true));

        // §6.3 Vanilla "removes a unit's text", and "Worth 2 Tributes while on the field" is the Sheep's
        // text (§7), so the copy is worth 1 (§3.2 reads the worth off the face that is up).
        assert_eq!(tribute_value_of(s.state(), s.card(&copy.id)), 1);

        // The copy and Gary pay 2 of the 3: R101 refuses the play, and `legalActions` does not offer it.
        let golem = must(
            s.hand(PlayerId::P1)
                .iter()
                .find(|card| card.def_id == LAVA_GOLEM)
                .cloned(),
            "Lava Golem in hand",
        );
        let pair = sorted_join(&[copy.id.clone(), gary.id.clone()]);
        let offered = offers_tribute_set(s.state(), &golem, &pair);
        assert!(!offered);

        let result = reduce(
            s.state(),
            &json_as::<Action>(json!({
                "type": "play",
                "playerId": "p1",
                "nonce": "vanilla-sheep",
                "instanceId": golem.id,
                "zone": { "row": "units", "lane": 5 },
                "tributes": [copy.id, gary.id],
            })),
        );
        assert!(
            result.error.as_deref().unwrap_or_default().contains("Tribute 3"),
            "{:?} should match /Tribute 3/",
            result.error
        );
    }
}

const VANILLA: &str = "core-008";
const TIMMY: &str = "core-011";
const EXPERIMENTATION: &str = "core-085";
const TRIBUTE_LIBRARY: [&str; 6] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

fn unit_at(g: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match g.unit(player, lane) {
        Some(card) => card.clone(),
        None => panic!("setup: {player} should hold a unit in lane {lane}"),
    }
}

mod r102_3_2_a_sheep_s_worth_is_its_text_and_a_fuse_keeps_it {
    use super::*;

    #[test]
    fn r102_r77_a_sheep_token_fused_by_unlicensed_experimentation_is_still_worth_2_tributes_3_2_7() {
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 10,
            "p1": {
                // Mr. Vanilla is Immutable, so #85 never picks it (R23): the Sheep is the Fuse target.
                "field": [
                    { "def": SHEEP, "lane": 1 },
                    { "def": VANILLA, "lane": 2 },
                ],
                "backrow": [{ "def": EXPERIMENTATION, "lane": 3 }],
                "hand": [LAVA_GOLEM, STOCKPILE],
                "library": TRIBUTE_LIBRARY,
            },
            "p2": { "hand": [TIMMY, STOCKPILE], "library": TRIBUTE_LIBRARY },
        }));
        let sheep = unit_at(&g, PlayerId::P1, 1);
        let vanilla = unit_at(&g, PlayerId::P1, 2);
        assert_eq!(tribute_value_of(g.state(), &sheep), 2);

        g.play(TIMMY, json!({ "zone": 1 }));
        let fused = g.card(&sheep.id).clone();
        assert_ne!(fused.def_id, SHEEP);
        assert!(g.unit(PlayerId::P2, 1).is_none());

        // R77/R102: the fused card's text is both texts joined, and "nothing about an ingredient is
        // silently dropped" — the Sheep's "worth 2 Tributes" (§7, §3.2) is part of that text.
        assert_eq!(tribute_value_of(g.state(), &fused), 2);

        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        let Some(golem) = g
            .hand(PlayerId::P1)
            .iter()
            .find(|card| card.def_id == LAVA_GOLEM)
            .cloned()
        else {
            panic!("setup: Lava Golem in hand");
        };
        let pair = sorted_join(&[fused.id.clone(), vanilla.id.clone()]);
        let offered = offers_tribute_set(g.state(), &golem, &pair);
        assert!(offered);
        g.play(
            &golem.id,
            json!({ "zone": 3, "tributes": [fused.id, vanilla.id] }),
        );
        assert_eq!(
            g.unit(PlayerId::P1, 3).map(|card| card.def_id.clone()),
            Some(LAVA_GOLEM.to_string())
        );
    }
}

// What a Tribute's Death puts on the field is an arrival (R119, R210)

const MR_VANILLA: &str = "core-008";
const TEMPO_TIMMY: &str = "core-011";
const MIDRANGE_MENACE: &str = "core-019";
const CARNIVOROUS_CUBE: &str = "core-022";
const UNSTABLE_CLONE_MACHINE: &str = "core-033";
const QUICKSTRIKER: &str = "core-038";
const SHEEPISH: &str = "core-041";
const THE_ROCK: &str = "core-066";
const SHEEP_TOKEN: &str = "core-t-sheep";

const R119_LIBRARY: [&str; 6] = [
    MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA,
];

fn summoned_defs(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
            _ => None,
        })
        .collect()
}

/// The events of one type.
fn of_type(events: &[GameEvent], type_: GameEventType) -> Vec<GameEvent> {
    events
        .iter()
        .filter(|event| event.event_type() == type_)
        .cloned()
        .collect()
}

fn backrow_defs(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
    s.state().players[player]
        .backrow
        .iter()
        .map(|card| card.as_ref().map(|card| card.def_id.clone()))
        .collect()
}

fn quickstrikers_of(s: &Scenario, player: PlayerId) -> Vec<String> {
    s.state().players[player]
        .backrow
        .iter()
        .flatten()
        .filter(|card| card.def_id == QUICKSTRIKER)
        .map(|card| card.id.clone())
        .collect()
}

/// R428: Carnivorous Cube eats Units only, so a backrow card reaches its Death's copies as the text of
/// a Unit it was fused onto (R77: the target keeps its instance and its type, and carries the text).
fn unit_carrying(s: &mut Scenario, player: PlayerId, unit_lane: i32, backrow_lane: i32) -> CardInstance {
    let unit = must(
        s.unit(player, unit_lane),
        &format!("{player}'s lane-{unit_lane} unit"),
    );
    let carried = must(
        s.backrow(player, backrow_lane),
        &format!("{player}'s backrow card in lane {backrow_lane}"),
    );
    // The live state, a fresh event list and an rng whose cursor is never written back.
    let state = s.state_mut();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    must(
        fuse(
            &mut sink,
            FuseArgs {
                ingredients: vec![unit.clone(), carried],
                target: Some(unit),
                ..Default::default()
            },
        ),
        "the Unit carrying the text",
    )
}

mod r119_r210_what_a_tribute_s_death_puts_on_the_field_does_not_answer_the_play_that_paid_it_10_5_step_2 {
    use super::*;

    #[test]
    fn r119_r210_a_sheepish_text_a_tributed_carnivorous_cube_s_death_copies_at_step_2_does_not_turn_the_lava_golem_being_played_into_a_sheep()
     {
        // p2's Cube eats p2's own Midrange Menace carrying Sheepish's text (R428: a Unit), so its Death
        // summons two copies of it into p2's unit row, each answering an opponent's resolved Unit play.
        let mut s = scenario(json!({
            "active": "p2",
            "p1": {
                // Radiant, so the Golem stays on p1's side though its Tribute takes p2's Cube (R360).
                "hand": [{ "def": LAVA_GOLEM, "radiant": true }, STOCKPILE],
                "field": [
                    { "def": MR_VANILLA, "lane": 1 },
                    { "def": MR_VANILLA, "lane": 2 },
                ],
                "mana": 10,
                "library": R119_LIBRARY,
            },
            "p2": {
                "hand": [CARNIVOROUS_CUBE, STOCKPILE],
                "backrow": [SHEEPISH],
                "field": [{ "def": MIDRANGE_MENACE, "lane": 5 }],
                "mana": 10,
                "library": R119_LIBRARY,
            },
        }));
        let carrier = unit_carrying(&mut s, PlayerId::P2, 5, 1);
        s.play(
            CARNIVOROUS_CUBE,
            json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": carrier.id }] }),
        );
        let cube = must(s.unit(PlayerId::P2, 1), "p2's Carnivorous Cube");
        s.end_turn();
        assert_eq!(s.state().active, PlayerId::P1);
        assert_eq!(
            s.state()
                .players
                .p2
                .units
                .iter()
                .filter(|pile| pile.is_some())
                .count(),
            1
        );

        // §8 #55: Lava Golem "can use opposing Units as Tributes", so the Cube is one of its three.
        let golem = must(
            s.hand(PlayerId::P1)
                .iter()
                .find(|card| card.def_id == LAVA_GOLEM)
                .cloned(),
            "p1's Lava Golem",
        );
        let first = must(s.unit(PlayerId::P1, 1), "p1's lane-1 Mr. Vanilla");
        let second = must(s.unit(PlayerId::P1, 2), "p1's lane-2 Mr. Vanilla");
        s.play(
            &golem.id,
            json!({ "zone": 4, "tributes": [cube.id, first.id, second.id] }),
        );

        // Step 2 paid the Tribute and the Cube's Death summoned its two copies (R41, R210).
        assert_eq!(
            summoned_defs(s.last_events())
                .iter()
                .filter(|def_id| **def_id == carrier.def_id)
                .count(),
            2
        );
        // R119: those copies arrived while this play resolved, so they start counting from the next play:
        // nothing answers the Golem's resolution, and the Golem lands as itself.
        assert_eq!(
            of_type(s.last_events(), GameEventType::Transformed),
            Vec::<GameEvent>::new()
        );
        assert_eq!(
            s.unit(PlayerId::P1, 4).map(|card| card.def_id.clone()),
            Some(LAVA_GOLEM.to_string())
        );
        assert_ne!(
            s.unit(PlayerId::P1, 4).map(|card| card.def_id.clone()),
            Some(SHEEP_TOKEN.to_string())
        );
    }

    #[test]
    fn r119_r210_an_unstable_clone_machine_text_a_tributed_cube_s_death_copies_at_step_2_does_not_shuffle_copies_of_the_card_that_paid_the_tribute()
     {
        let mut s = scenario(json!({
            "p1": {
                "hand": [CARNIVOROUS_CUBE, THE_ROCK],
                "field": [{ "def": MR_VANILLA, "lane": 3 }],
                "backrow": [UNSTABLE_CLONE_MACHINE],
                "mana": 20,
                "library": R119_LIBRARY,
            },
            "p2": { "hand": [STOCKPILE], "field": [MIDRANGE_MENACE], "library": R119_LIBRARY },
        }));
        // The Cube eats the Mr. Vanilla carrying the Clone Machine's text (R428), so none is on the field
        // when The Rock is played.
        let carrier = unit_carrying(&mut s, PlayerId::P1, 3, 1);
        s.play(
            CARNIVOROUS_CUBE,
            json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": carrier.id }] }),
        );
        let cube = must(s.unit(PlayerId::P1, 1), "p1's Carnivorous Cube");
        assert_eq!(
            backrow_defs(&s, PlayerId::P1),
            vec![None::<String>, None, None, None, None]
        );
        let library = s.pile(PlayerId::P1, "library").len();

        // §8 #66: The Rock's Tribute 1 is the Cube, whose Death summons two copies of its meal.
        s.play(THE_ROCK, json!({ "zone": 4, "tributes": [cube.id] }));

        assert_eq!(
            summoned_defs(s.last_events())
                .iter()
                .filter(|def_id| **def_id == carrier.def_id)
                .count(),
            2
        );
        // R119: "After you play a card" — both copies arrived during this play, so neither answers it.
        assert_eq!(
            of_type(s.last_events(), GameEventType::ShuffledIn),
            Vec::<GameEvent>::new()
        );
        assert_eq!(s.pile(PlayerId::P1, "library").len(), library);
    }

    #[test]
    fn r119_r210_a_quickstriker_text_a_tributed_cube_s_death_copies_at_step_2_grants_the_card_that_paid_the_tribute_no_combo_damage()
     {
        let mut s = scenario(json!({
            "p1": {
                "hand": [CARNIVOROUS_CUBE, TEMPO_TIMMY, THE_ROCK],
                "field": [{ "def": MR_VANILLA, "lane": 3 }],
                "backrow": [QUICKSTRIKER],
                "mana": 20,
                "library": R119_LIBRARY,
            },
            "p2": { "hand": [STOCKPILE], "field": [MIDRANGE_MENACE], "library": R119_LIBRARY },
        }));
        let carrier = unit_carrying(&mut s, PlayerId::P1, 3, 1);
        s.play(
            CARNIVOROUS_CUBE,
            json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": carrier.id }] }),
        );
        let cube = must(s.unit(PlayerId::P1, 1), "p1's Carnivorous Cube");
        s.play(TEMPO_TIMMY, json!({ "zone": 2 }));
        assert_eq!(quickstrikers_of(&s, PlayerId::P1), Vec::<String>::new());
        let rock = s.card(THE_ROCK).clone();

        s.play(THE_ROCK, json!({ "zone": 4, "tributes": [cube.id] }));

        // Two Units carrying Quickstriker's text arrived at step 2; The Rock is the third card played this
        // turn (X = 2).
        assert_eq!(
            summoned_defs(s.last_events())
                .iter()
                .filter(|def_id| **def_id == carrier.def_id)
                .count(),
            2
        );
        // R119: they start counting from the next play, so The Rock's step 5 deals no Combo damage.
        let hits: Vec<GameEvent> = s
            .last_events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    GameEvent::Damage { source_id: Some(source), target_id, .. }
                        if *source == rock.id && target_id == "hero-p2"
                )
            })
            .cloned()
            .collect();
        assert_eq!(hits, Vec::<GameEvent>::new());
        s.expect_health(PlayerId::P2, 30);
    }
}
