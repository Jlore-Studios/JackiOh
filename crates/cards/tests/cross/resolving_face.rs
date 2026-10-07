//! Port of `packages/cards/test/resolving-face.test.ts` (v0.3.0 part 27.5).
//!
//! #64 Gifted Program's first cheap card, and the face a play's choices answer (SPEC §8 #64, §10.5
//! steps 1 and 3, R56, R81, R90, R213, R214). Found by the polish-4 edge-case hunt, round 3
//! (docs/polish/4-edge-cases.md, lenses L1, L2 and L9); every case here failed before its fix, except
//! the third R213 case, which pins the reading the fix adopts.
//!
//!  - R213: "the first card costing 1 or less you play each turn" counts the player's plays that turn,
//!    so a Gifted Program that changes hands, or leaves and comes back, neither uses up another
//!    player's first cheap card nor gives its own player a second.
//!  - R214: step 3 can make the played card Radiant after step 1 has read its choices, so step 1 and
//!    `legalActions` read the choices of the face step 5 will resolve.

use jackioh_engine::testkit::*;

const BIGOT: &str = "core-002";
const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const MOTHS: &str = "core-009";
const TIMMY: &str = "core-011";
const HINDER: &str = "core-021";
const PANTHER: &str = "core-032";
const PEK_CONTROLLER: &str = "core-048";
const MIND_CONTROL: &str = "core-049";
const SILAS: &str = "core-052";
const RENO: &str = "core-053";
const FRIEND: &str = "core-062";
const GIFTED: &str = "core-064";
const TWISTED_SORCERER: &str = "core-068";
const POCKET_CHAOS: &str = "core-087";
const CRAFT_A_CARD: &str = "core-099";
const LIBRARY: [&str; 8] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

/// TS `at(card)`: the one-instance selection list naming `id`.
fn at(id: &str) -> Value {
    json!([{ "pick": "instance", "instanceId": id }])
}

fn unit_at(g: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match g.unit(player, lane) {
        Some(card) => card.clone(),
        None => panic!("setup: {player} should hold a unit in lane {lane}"),
    }
}

fn backrow_at(g: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match g.backrow(player, lane) {
        Some(card) => card.clone(),
        None => panic!("setup: {player} should hold a backrow card in lane {lane}"),
    }
}

/// The `play` actions `legalActions` offers p1 for the card `id`.
fn offered(g: &Scenario, id: &str) -> Vec<ActionBody> {
    legal_actions(g.state(), PlayerId::P1)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if instance_id == id))
        .collect()
}

/// The `modes` of each offered play, as TS's `.map((action) => action.modes)`.
fn offered_modes(g: &Scenario, id: &str) -> Vec<Option<Vec<String>>> {
    offered(g, id)
        .into_iter()
        .map(|action| match action {
            ActionBody::Play { modes, .. } => modes,
            _ => None,
        })
        .collect()
}

fn modes(list: &[&str]) -> Option<Vec<String>> {
    Some(list.iter().map(|mode| (*mode).to_string()).collect())
}

/// The face the card's `cardResolved` says resolved, or `None` when there is none or it says nothing.
fn resolved_face(g: &Scenario, id: &str) -> Option<bool> {
    g.events().iter().find_map(|event| match event {
        GameEvent::CardResolved { instance_id, radiant, .. } if instance_id == id => Some(*radiant),
        _ => None,
    })?
}

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected {what}"),
    }
}

mod r213_gifted_programs_first_cheap_card_is_its_controllers_first_of_the_turn {
    use super::*;

    #[test]
    fn r213_a_gifted_program_stolen_after_it_fired_for_its_owner_this_turn_still_makes_the_thiefs_first_cheap_card_radiant_8_conventions_r171()
     {
        jackioh_cards::register_all();
        // p1's turn begins. p1 holds #9 Moths to the Flame, worn down to 4 health; p2 holds Gifted
        // Program, a Prem Panther, and a Hinder on top of the library.
        let mut g = scenario(json!({
            // Snom Bunny Mind Control (4 since patch v0.2.0, issue #40) and Stockpile (1).
            "p1": {
                "hand": [MIND_CONTROL, STOCKPILE],
                "field": [{ "def": MOTHS, "lane": 1, "damage": 10 }],
                "library": LIBRARY,
            },
            "p2": {
                "hand": [VANILLA],
                "field": [{ "def": PANTHER, "lane": 1 }],
                "backrow": [{ "def": GIFTED, "lane": 1, "faceUp": true }],
                "library": [HINDER, VANILLA, VANILLA, VANILLA],
            },
        }));
        let gifted = backrow_at(&g, PlayerId::P2, 1).id;

        // p1's start of turn: Moths makes p2's Panther attack it; the Panther destroys it and survives,
        // so p2 draws 2 — on p1's turn (R426: a forced attack is an attack). The first draw is Hinder,
        // cast as it is drawn: p2's play costing 0 (R70), made Radiant by p2's Gifted Program.
        g.start_turn();
        let hinder = g
            .events()
            .iter()
            .find(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == HINDER))
            .cloned();
        match hinder {
            Some(GameEvent::CardPlayed { player, cost_paid, .. }) => {
                assert_eq!(player, PlayerId::P2);
                assert_eq!(cost_paid, 0);
            }
            other => panic!("expected p2's Hinder to be played, found {other:?}"),
        }
        assert_eq!(g.state().players.p1.mana.next_turn_mod, -2); // the radiant face ran: "2 lower"

        // p1 takes the Gifted Program, and has played nothing costing 1 or less this turn: Snom Bunny
        // Mind Control cost 4. Stockpile is p1's first cheap card, so it resolves its radiant text.
        g.state_mut().players.p1.mana.current = 5;
        g.play(MIND_CONTROL, json!({ "targets": at(&gifted) }));
        assert_eq!(g.card(gifted.as_str()).controller, PlayerId::P1);
        let stockpile = g.card(STOCKPILE).id.clone();
        let hand_before = g.hand(PlayerId::P1).len();
        g.play(STOCKPILE, json!({}));
        assert!(g.card(stockpile.as_str()).radiant);
        assert_eq!(g.hand(PlayerId::P1).len(), hand_before - 1 + 5);
    }

    #[test]
    fn r213_a_gifted_program_bounced_and_replayed_does_not_make_a_second_cheap_card_radiant_in_the_same_turn_r174() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [VANILLA, { "def": SILAS, "radiant": true }, STOCKPILE],
                "backrow": [{ "def": GIFTED, "lane": 5 }],
                "mana": 10,
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));
        let gifted = g.card(GIFTED).id.clone();
        let first = must(
            g.hand(PlayerId::P1).iter().find(|c| c.def_id == VANILLA).map(|c| c.id.clone()),
            "setup: hand",
        );
        let second = g.card(STOCKPILE).id.clone();

        g.play(first.as_str(), json!({ "zone": 1 }));
        assert!(g.card(first.as_str()).radiant);

        // Radiant Silas rotates right: Gifted Program in p1's backrow lane 5 would cross, so it is
        // bounced to p1's hand costing 0 (§8 #52), and p1 plays it again.
        g.play(SILAS, json!({ "zone": 2, "modes": ["right"] }));
        g.expect_in_zone(gifted.as_str(), "hand");
        g.play(gifted.as_str(), json!({ "zone": 4 }));

        // Stockpile is the second card costing 1 or less p1 has played this turn, not the first.
        g.play(second.as_str(), json!({}));
        assert!(!g.card(second.as_str()).radiant);
    }

    #[test]
    fn r213_a_card_costing_1_or_less_played_before_gifted_program_arrived_was_already_the_turns_first() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [STOCKPILE, GIFTED, FRIEND, RENO], "field": [TIMMY], "mana": 10, "library": LIBRARY },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }));
        let friend = g.card(FRIEND).id.clone();

        g.play(STOCKPILE, json!({}));
        g.play(GIFTED, json!({}));
        g.play(friend.as_str(), json!({}));

        // Pint-Sized Summoner reads the same way: the first minion played this turn, whenever it came.
        assert!(!g.card(friend.as_str()).radiant);
        let timmy = g.card(TIMMY).id.clone();
        g.expect_stats(timmy.as_str(), json!({ "attack": 3, "maxHealth": 3 }));
    }
}

mod r214_a_plays_choices_are_the_choices_of_the_face_it_resolves_with {
    use super::*;

    #[test]
    fn r214_a_pocket_chaos_that_radiant_gifted_program_will_make_radiant_is_offered_and_may_carry_the_choice_to_skip_the_gift_8_87_radiant_r81()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            // Patch v0.2.9 costs Pocket Chaos at (4): a −2 costMod puts the play at (2), inside
            // radiant Gifted Program's threshold, while its base cost stays (4).
            "p1": {
                "hand": [{ "def": POCKET_CHAOS, "costMod": -2 }, STOCKPILE],
                "library": [STOCKPILE],
                "backrow": [{ "def": GIFTED, "radiant": true }],
                "health": 20,
            },
            "p2": { "hand": [STOCKPILE] },
        }));
        let chaos = g.card(POCKET_CHAOS).id.clone();

        // It costs 2, inside radiant Gifted Program's threshold, so step 3 will make it Radiant: the play
        // answers the radiant face's two mode declarations, and `legalActions` offers them.
        assert!(offered_modes(&g, &chaos).contains(&modes(&["health", "skip"])));
        assert!(!offered_modes(&g, &chaos).contains(&modes(&["health"])));

        g.play(chaos.as_str(), json!({ "modes": ["health", "skip"] }));

        assert_eq!(resolved_face(&g, &chaos), Some(true));
        g.expect_health(PlayerId::P1, 30);
        assert_eq!(g.hand(PlayerId::P2).iter().filter(|card| card.def_id == POCKET_CHAOS).count(), 0);
        // The radiant face draws nothing since patch v0.1.1: the library's Stockpile stays there.
        assert_eq!(g.hand(PlayerId::P1).iter().filter(|card| card.def_id == STOCKPILE).count(), 1);
        let library: Vec<String> = g.pile(PlayerId::P1, "library").iter().map(|card| card.def_id.clone()).collect();
        assert_eq!(library, vec![STOCKPILE.to_string()]);
    }

    #[test]
    fn r214_a_5pek_controller_that_gifted_program_will_make_radiant_switches_the_enemy_units_only_when_the_play_says_so_8_48_radiant()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [PEK_CONTROLLER, STOCKPILE], "field": [{ "def": RENO, "lane": 1 }], "backrow": [GIFTED] },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": RENO, "lane": 1 }] },
        }));
        let pek = g.card(PEK_CONTROLLER).id.clone();
        assert_eq!(offered_modes(&g, &pek), vec![modes(&["enemy"]), modes(&["all"])]);

        g.play(pek.as_str(), json!({ "modes": ["enemy"] }));

        assert_eq!(resolved_face(&g, &pek), Some(true));
        assert_eq!(g.unit(PlayerId::P1, 1).and_then(|card| card.position), Some(Position::Atk));
        assert_eq!(g.unit(PlayerId::P2, 1).and_then(|card| card.position), Some(Position::Def));
    }

    #[test]
    fn r214_a_crafted_bigot_twisted_sorcerer_that_gifted_program_will_make_radiant_names_the_sorcerers_target_alone_r90_r102()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "seed": "craft-453", // the first Discover offers Bigot, the second Twisted Sorcerer (pools of every set, R380)
            "p1": { "hand": [CRAFT_A_CARD, RENO], "mana": 4, "backrow": [GIFTED] },
            "p2": { "hand": [RENO], "field": [PANTHER] },
        }));
        g.play(CRAFT_A_CARD, json!({}));
        g.answer(json!(BIGOT));
        g.answer(json!(TWISTED_SORCERER));
        let card = must(
            g.hand(PlayerId::P1).iter().find(|held| held.def_id.starts_with("t-")).map(|held| held.id.clone()),
            "setup: the crafted card",
        );
        let panther = unit_at(&g, PlayerId::P2, 1).id;
        let hero = json!({ "pick": "hero", "player": "p2" });

        // It costs 0, so it is the first card costing 1 or less this turn: step 3 makes it Radiant, and
        // radiant Bigot destroys all enemy non-Humans with no target. The flat list is the Sorcerer's
        // alone, so a list built for the base face (Bigot's target, then the Sorcerer's) is refused.
        let refused = json!({ "zone": 2, "targets": [{ "pick": "instance", "instanceId": panther }, hero.clone()] });
        g.expect_refused(|g| {
            g.play(card.as_str(), refused.clone());
        });
        g.play(card.as_str(), json!({ "zone": 2, "targets": [hero] }));

        // Radiant Twisted Sorcerer deals 8 to the target the player named for it: p2's hero, 30 → 22.
        assert_eq!(resolved_face(&g, &card), Some(true));
        g.expect_health(PlayerId::P2, 22);
    }
}

const GARY: &str = "core-004";
const JEWELOSCO_SCARAB: &str = "core-007";
const CARNIVOROUS_CUBE: &str = "core-022";
const SEVEN_SEVEN: &str = "core-025";
const BIG_FELINOR: &str = "core-043";
const LAVA_GOLEM: &str = "core-055";

mod r214_step_3_applies_the_face_step_1_checked_whatever_step_2_put_on_the_board {
    use super::*;

    #[test]
    fn r214_a_gifted_program_a_tributes_death_puts_on_the_field_at_step_2_does_not_change_the_face_the_plays_choices_were_checked_against_r213_10_5_step_3()
     {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [CARNIVOROUS_CUBE, LAVA_GOLEM, BIGOT],
                "field": [GARY, JEWELOSCO_SCARAB, VANILLA],
                "backrow": [GIFTED],
                "library": [RENO, RENO],
                "mana": 4,
            },
            "p2": { "field": [BIG_FELINOR, SEVEN_SEVEN], "hand": [RENO], "library": [RENO] },
        }));
        // A Unit that carries Gifted Program's text: the Gifted Program fused onto p1's Mr. Vanilla, which
        // keeps its instance and its type (R77). #22 eats Units only (R428), and this one is a Unit.
        let vanilla = must(s.unit(PlayerId::P1, 3).map(|card| card.clone()), "p1's Mr. Vanilla");
        let gifted = must(s.backrow(PlayerId::P1, 1).map(|card| card.clone()), "p1's Gifted Program");
        // #99's result, built the way `099-craft-a-card.test.ts` builds one: Lava Golem + Bigot, a
        // Unit with Tribute 3 costing 0, whose base face names an enemy non-Human unit to destroy and
        // whose radiant face destroys every enemy non-Human unit and names nothing (R102, R214).
        let lava_golem = s.card(LAVA_GOLEM).clone();
        let bigot = s.card(BIGOT).clone();
        let (gifted_unit, crafted) = {
            let mut events: Vec<GameEvent> = vec![];
            let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            let gifted_unit = must(
                subsystems::fuse(
                    &mut sink,
                    FuseArgs { ingredients: vec![vanilla.clone(), gifted], target: Some(vanilla.clone()), ..FuseArgs::default() },
                ),
                "the Mr. Vanilla carrying Gifted Program",
            );
            let crafted = must(
                subsystems::fuse(
                    &mut sink,
                    FuseArgs { ingredients: vec![lava_golem, bigot], to_hand: Some(PlayerId::P1), ..FuseArgs::default() },
                ),
                "the crafted Lava Golem + Bigot",
            );
            (gifted_unit, crafted)
        };
        // #22 eats the Unit carrying the Gifted Program's text (R41, R428), so none stands on p1's side
        // any more; its Death will summon two copies of it.
        s.play(CARNIVOROUS_CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": gifted_unit.id }] }));
        s.expect_in_zone(gifted_unit.id.as_str(), "graveyard");
        let cube = must(s.unit(PlayerId::P1, 4).map(|card| card.id.clone()), "p1's Carnivorous Cube");
        let felinor = must(s.unit(PlayerId::P2, 1).map(|card| card.id.clone()), "p2's Big Felinor");
        let seven_seven = must(s.unit(PlayerId::P2, 2).map(|card| card.id.clone()), "p2's 4-mana 7/7");

        // Step 1 sees no Gifted Program, so the face that answers the play's choices is the base one
        // (R214), and legalActions offers its single-target Cry with the Tribute paid by the Cube.
        let tributes = vec![
            cube.clone(),
            must(s.unit(PlayerId::P1, 1).map(|card| card.id.clone()), "Gary"),
            must(s.unit(PlayerId::P1, 2).map(|card| card.id.clone()), "Scarab"),
        ];
        let mut wanted = tributes.clone();
        wanted.sort();
        let offered = legal_actions(s.state(), PlayerId::P1).into_iter().any(|action| match action {
            ActionBody::Play { instance_id, targets, tributes: paid, .. } => {
                let mut paid = paid.unwrap_or_default();
                paid.sort();
                instance_id == crafted.id
                    && targets.as_ref().map(Vec::len) == Some(1)
                    && matches!(
                        targets.as_ref().and_then(|list| list.first()),
                        Some(Selection::Instance { instance_id }) if *instance_id == felinor
                    )
                    && paid.join(",") == wanted.join(",")
            }
            _ => false,
        });
        assert!(offered);

        // Step 2 pays the Tribute: the Cube's Death summons two copies of the Unit it ate, each carrying
        // Gifted Program's text.
        s.play(
            crafted.id.as_str(),
            json!({ "tributes": tributes, "targets": [{ "pick": "instance", "instanceId": felinor }], "zone": 5 }),
        );
        let copies = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if *def_id == gifted_unit.def_id))
            .count();
        assert_eq!(copies, 2);

        // R214: "step 1 already knows" the face, and step 1 checked the play's choices against the base
        // face, so that is the face step 5 resolves: the chosen Big Felinor is destroyed and nothing
        // else. A Gifted Program that arrived after step 1 cannot turn it into a face whose choices no
        // step checked, destroying the 7/7 the play never named.
        assert!(!s.card(crafted.id.as_str()).radiant);
        s.expect_in_zone(felinor.as_str(), "graveyard");
        s.expect_in_zone(seven_seven.as_str(), "field");
    }
}
