//! Port of `packages/cards/test/re-entry.test.ts` (v0.3.0 part 27.5).
//!
//! Leaving the field, coming back, and what travels with a card between zones (SPEC §3.2, §4.5, R11,
//! R13, R47, R57, R64, R76, R78, R83, R174, R175). Found by the polish-4 edge-case hunt
//! (docs/polish/4-edge-cases.md, lenses L1, L2 and L8); every case here failed before its fix.
//!
//!  - R174: #50 K-Pop Fanatic's delayed steal fizzles on a target that has left the field since it was
//!    chosen, even when the same card is back — bounced and replayed, or returned by Reborn — and on
//!    a target dormant under a Stack pile when it fires (R13, R76).
//!  - R175: Reborn brings back a unit token, and a Reborn unit that died on top of a Stack pile
//!    returns on top of it.
//!  - §3.2 and R13: an aura does not reach a card dormant under a Stack pile.
//!  - §7, R41, R57: a copy keeps a Radiant Bread Token's Armor X beside its X/X.
//!  - §8 #52, R4, R78: a card radiant Silly Silas bounces into a full hand is burned without its
//!    "costing 0".
//!  - Round 4, lens L2. R77, R175: a Fuse onto a token summoned X/X sums that X/X, not the printed
//!    0/0, and the Bread Token's Armor X stands for its own Armor only. R35, §3.2: a Transform
//!    replaces a card in a Locked zone, since the Lock refuses summons and a Replace is none.
//!  - Round 5, lens L2. R174: a later part of one effect list is aimed at the stay the play chose, so
//!    a fused card's part fizzles on a card an earlier part took off the field — #68's damage on the
//!    meal's Reborn body, #61's copy of a card in a graveyard, #50's steal of a card bounced and
//!    replayed. #85 fuses the opponent's played card onto a unit, which is bounced and played again.
//!  - Round 6, lens L2. R174, R113: that holds across a prompt too — a crafted Cube + Scarab +
//!    Sorcerer's Discover splits the list across actions, and the Sorcerer's part still fizzles on the
//!    Reborn body of the unit the Cube's part ate.
//!  - Round 7, lens L2. R174: it holds for "this" card as well — a crafted Silas + Gary that its own
//!    Silas part bounced to hand is not buffed there by its Gary part.
//!  - Round 8, lens L2. R174: a Transform takes the card off the field like any departure, so a
//!    second Sheepish is not offered the play the first turned into a Sheep. R102, R212: a card a
//!    Fuse kept is on the same stay, so a trigger it queued before the Fuse (Fed Fauci's Plague Counter
//!    for the Cry that hit it) still resolves under the fused definition's namespaced id.
//!  - Round 9, lens "re-entry and stays". R174: #22 reads its meal on the stay the play chose, so a
//!    crafted Cube + Cube naming one Reborn unit twice remembers it once; and a card the play's own
//!    Stack card buried is not on the field for its Cry (§3.2, R13). §4.5 step 4: the Reborn bodies of
//!    one check return together, each at 1 health once all stand. R102, R77: what a card a Fuse kept
//!    remembered moves with its texts, so the texts fused onto it read none of it.

use jackioh_engine::subsystems::FuseArgs;
use jackioh_engine::testkit::*;

const RIGHT_HOUSE: &str = "core-003";
const STOCKPILE: &str = "core-005";
const MAGIC_JAMMED: &str = "core-036";
const TRANSMOGULATE: &str = "core-083";
const MANA_WELL: &str = "core-006";
const TIMMY: &str = "core-011";
const POSTDOC: &str = "core-061";
const SORCERER: &str = "core-068";
const PALANTIR: &str = "classic-004";
const HEROIC_POWER: &str = "core-098";
const VANILLA: &str = "core-008";
const HIT_JOB: &str = "core-016";
const FLOOD: &str = "core-017";
/// The Coin (§2.1): one more mana this turn, where a test's turn now needs it.
const COIN: &str = "core-t-coin";
const BREAD_AND_BUTTER: &str = "core-018";
const CUBE: &str = "core-022";
const SEVEN_SEVEN: &str = "core-025";
const AURA: &str = "core-046";
const MIND_CONTROL: &str = "core-049";
const KPOP: &str = "core-050";
const SILAS: &str = "core-052";
const JILLIAX: &str = "core-056";
const SURGERY: &str = "core-063";
const REMINISCE: &str = "core-072";
const SAINTESS: &str = "core-081";
const EXPERIMENTATION: &str = "core-085";
const HINDER: &str = "core-021";
const SHEEPISH: &str = "core-041";
const SHEEP: &str = "core-t-sheep";
const FAUCI: &str = "core-091";
const FIENDER: &str = "core-092";
const RUSH_TOKEN: &str = "core-t-rush";
const FELINOR_TOKEN: &str = "core-t-felinor";
const BREAD: &str = "core-t-bread";
const LIBRARY: [&str; 6] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

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

/// Hand the turn over until `player` is the active one (R82 can end a turn with nothing left in it).
fn until_active(g: &mut Scenario, player: PlayerId) {
    if g.state().active != player {
        g.end_turn();
    }
    if g.state().active != player {
        g.end_turn();
    }
    assert_eq!(g.state().active, player);
}

/// Every card in a unit zone, top first (§3.2).
fn pile_of(g: &Scenario, player: PlayerId, lane: usize) -> Vec<String> {
    g.state().players[player].units[lane - 1]
        .iter()
        .flatten()
        .map(|card| card.id.clone())
        .collect()
}

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("missing: {what}"),
    }
}

/// Whether the unit's keywords as the layers read them include `kind`.
fn has_keyword(g: &Scenario, id: &str, kind: KeywordKind) -> bool {
    g.stats(id).keywords.iter().any(|keyword| keyword.kind() == kind)
}

/// TS `g.card(ref).grantedKeywords = …`: a write to the live instance.
fn grant(g: &mut Scenario, id: &str, keywords: Vec<Keyword>) {
    must(find_instance_mut(g.state_mut(), id), id).granted_keywords = keywords;
}

mod r174_50_k_pop_fanatics_delayed_steal_and_a_target_that_left_the_field {
    use super::*;

    #[test]
    fn r174_the_steal_fizzles_on_a_target_now_dormant_under_a_stack_pile_and_the_pile_stays_whole_r13_r76() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [KPOP, VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [FIENDER, VANILLA], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
        }));
        let prey = unit_at(&g, PlayerId::P2, 2).id;
        g.play(KPOP, json!({ "targets": at(&prey) }));
        g.end_turn();

        // p2 stacks Felinor Fiender onto the prey's zone: the prey is dormant (§3.2, R13).
        g.play(FIENDER, json!({ "zone": 2 }));
        let fiender = unit_at(&g, PlayerId::P2, 2);
        assert_eq!(fiender.def_id, FIENDER);
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);

        // Only the top of a pile is on the field, so there is nothing to take (R13, R76).
        assert_eq!(g.card(prey.as_str()).controller, PlayerId::P2);
        assert_eq!(pile_of(&g, PlayerId::P2, 2), vec![fiender.id.clone(), prey.clone()]);
        assert!(!g.state().players.p1.units.iter().flatten().flatten().any(|card| card.id == prey));
    }

    #[test]
    fn r174_the_steal_fizzles_on_a_target_that_was_bounced_and_replayed_before_it_fired_r76_r78() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [KPOP, VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
            // The Coin pays for the replay: Flood costs (4) since patch v0.2.0 (issue #40).
            "p2": { "hand": [FLOOD, SEVEN_SEVEN, COIN], "field": [{ "def": VANILLA, "lane": 2 }], "library": LIBRARY },
        }));
        let prey = unit_at(&g, PlayerId::P2, 2).id;
        g.play(KPOP, json!({ "targets": at(&prey) }));
        g.end_turn();

        // p2 bounces every unit, the prey included, and plays the prey again.
        g.play(COIN, json!({}));
        g.play(FLOOD, json!({}));
        g.expect_in_zone(prey.as_str(), "hand");
        g.play(prey.as_str(), json!({ "zone": 2 }));
        assert_eq!(g.unit(PlayerId::P2, 2).map(|card| card.id.clone()), Some(prey.clone()));
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);

        // The card on the field now is a new arrival (R78), which R76's steal never chose.
        assert_eq!(g.card(prey.as_str()).controller, PlayerId::P2);
        assert_eq!(g.unit(PlayerId::P2, 2).map(|card| card.id.clone()), Some(prey.clone()));
        assert!(!g.last_events().iter().any(|event| matches!(event, GameEvent::ControlChanged { .. })));
    }

    #[test]
    fn r174_the_steal_fizzles_on_a_target_that_died_and_came_back_through_reborn_r76_r83() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [KPOP, HIT_JOB, VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
            "p2": {
                "hand": [VANILLA],
                "field": [{ "def": RIGHT_HOUSE, "lane": 2 }, { "def": VANILLA, "lane": 4 }],
                "library": LIBRARY,
            },
        }));
        let prey = unit_at(&g, PlayerId::P2, 2).id;
        g.play(KPOP, json!({ "targets": at(&prey) }));
        g.play(HIT_JOB, json!({ "targets": at(&prey) }));
        // It died and its Reborn body is back in its reserved zone, the same instance id (§4.5 step 4).
        assert_eq!(g.unit(PlayerId::P2, 2).map(|card| card.id.clone()), Some(prey.clone()));
        assert_eq!(g.card(prey.as_str()).reborn_spent, Some(true));
        assert!(
            g.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == prey))
        );

        g.end_turn();
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        assert_eq!(g.card(prey.as_str()).controller, PlayerId::P2);
    }

    #[test]
    fn r76_a_target_that_stayed_on_the_field_is_still_stolen_so_the_fizzles_above_are_the_left_the_field_cases_alone() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [KPOP, VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [VANILLA], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "library": LIBRARY },
        }));
        let prey = unit_at(&g, PlayerId::P2, 2).id;
        g.play(KPOP, json!({ "targets": at(&prey) }));
        g.end_turn();
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        assert_eq!(g.card(prey.as_str()).controller, PlayerId::P1);
    }
}

mod r175_reborns_return_for_a_unit_token_and_onto_a_stack_pile {
    use super::*;

    #[test]
    fn r175_a_rush_token_given_reborn_comes_back_through_reborn_reset_and_sick_6_1s_pool_r21_r83() {
        jackioh_cards::register_all();
        // On this seed Plastic Surgery's random keyword is Reborn: §6.1's pool keeps Reborn for tokens.
        let mut g = scenario(json!({
            "seed": "re-entry-reborn-token-32", // R346's Pierce moved the roll off "-4", R636's Windfury off "-10" and "-19", R49's Deft off "-31"
            "p1": { "hand": [SURGERY, HIT_JOB], "field": [{ "def": RUSH_TOKEN, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [HIT_JOB], "library": LIBRARY },
        }));
        let token = g.card(RUSH_TOKEN).id.clone();
        g.play(SURGERY, json!({ "targets": at(&token) }));
        assert!(has_keyword(&g, &token, KeywordKind::Reborn));

        g.play(HIT_JOB, json!({ "targets": at(&token) }));

        // §4.5 step 4: back in its zone at 1 health, without Reborn and without Surgery's buff (R78).
        g.expect_in_zone(token.as_str(), "field");
        assert_eq!(g.unit(PlayerId::P1, 1).map(|card| card.id.clone()), Some(token.clone()));
        g.expect_stats(token.as_str(), json!({ "health": 1 }));
        assert!(!has_keyword(&g, &token, KeywordKind::Reborn));
        assert_eq!(g.card(token.as_str()).summoned_turn, Some(g.state().turn));
        // It never reached a graveyard (R11).
        assert!(!g.pile(PlayerId::P1, "graveyard").iter().any(|card| card.id == token));
    }

    #[test]
    fn r175_a_reborn_unit_that_died_on_top_of_a_stack_pile_returns_on_top_of_it_r47_r64() {
        jackioh_cards::register_all();
        // p1 holds Felinor Fiender (Stack) on a Felinor Token and an armed #85. p2 plays Right-house
        // defender, #85 fuses it onto the Fiender (R77: the Fiender instance survives with Reborn and
        // Stack), and p2 destroys the fused card with Hit Job.
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 10,
            "p1": {
                "field": [{ "def": FELINOR_TOKEN, "lane": 1 }, { "def": FIENDER, "stack": true }],
                "backrow": [{ "def": EXPERIMENTATION, "lane": 1 }],
                "hand": [VANILLA],
                "library": LIBRARY,
            },
            "p2": { "hand": [RIGHT_HOUSE, HIT_JOB], "library": LIBRARY },
        }));
        let fiender = g.card(FIENDER).id.clone();
        let token = g.card(FELINOR_TOKEN).id.clone();

        g.play(RIGHT_HOUSE, json!({}));
        let fused = g.card(fiender.as_str()).clone();
        assert_ne!(fused.def_id, FIENDER);
        assert!(has_keyword(&g, &fused.id, KeywordKind::Reborn));
        assert!(has_keyword(&g, &fused.id, KeywordKind::Stack));
        assert_eq!(pile_of(&g, PlayerId::P1, 1), vec![fiender.clone(), token.clone()]);

        g.play(HIT_JOB, json!({ "targets": at(&fused.id) }));

        // The zone was reserved and never Locked, so the body returns — on top, the token dormant again.
        g.expect_in_zone(fiender.as_str(), "field");
        assert_eq!(pile_of(&g, PlayerId::P1, 1), vec![fiender.clone(), token.clone()]);
        assert_eq!(g.card(fiender.as_str()).reborn_spent, Some(true));
    }
}

mod c3_2_and_r13_a_card_dormant_under_a_stack_pile {
    use super::*;

    #[test]
    fn r13_an_aura_does_not_reach_a_dormant_card_so_suppressive_aura_cannot_kill_it_under_the_pile() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [AURA, HIT_JOB],
                "field": [{ "def": VANILLA, "lane": 1 }, { "def": FIENDER, "stack": true }],
                "library": LIBRARY,
            },
            "p2": { "hand": [HIT_JOB], "library": LIBRARY },
        }));
        let dormant = g.card(VANILLA).id.clone();
        let fiender = g.card(FIENDER).id.clone();
        assert_eq!(pile_of(&g, PlayerId::P1, 1), vec![fiender.clone(), dormant.clone()]);

        // Paid 4: "all Units −2/−2". Mr. Vanilla is 4/4 and dormant; Felinor Fiender 5/7 is on top.
        g.play(AURA, json!({ "embiggen": true }));

        g.expect_in_zone(dormant.as_str(), "field");
        assert_eq!(pile_of(&g, PlayerId::P1, 1), vec![fiender.clone(), dormant.clone()]);
        // The dormant card keeps its own stats; the top of the pile takes the aura.
        g.expect_stats(dormant.as_str(), json!({ "attack": 4, "health": 4 }));
    }
}

mod c7_r41_r57_what_a_copy_keeps {
    use super::*;

    #[test]
    fn r57_carnivorous_cubes_copies_of_a_radiant_bread_token_keep_its_armor_x_beside_its_x_x_7_r41() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "field": [{ "def": SAINTESS, "lane": 2 }],
                "backrow": [{ "def": BREAD_AND_BUTTER, "lane": 1 }],
                "hand": [CUBE, VANILLA],
                "library": LIBRARY,
                "mana": 3,
            },
            // The Coin pays for Jilliax beside Hit Job, (3) since patch v0.2.0 (issue #40).
            "p2": { "hand": [HIT_JOB, JILLIAX, HIT_JOB, COIN], "library": LIBRARY },
        }));
        let saintess = g.card(SAINTESS).id.clone();

        // p1 ends its turn with 3 unspent: Bread and Butter summons a 3/3 Bread Token, Armor X of 3.
        g.end_turn();
        let bread = unit_at(&g, PlayerId::P1, 1).id;
        assert_eq!(g.card(bread.as_str()).def_id, BREAD);
        g.expect_stats(bread.as_str(), json!({ "attack": 3, "health": 3 }));

        // p2 kills the Saintess: her Death makes every other p1 unit Radiant, so the Bread is Armor 3.
        g.play(COIN, json!({}));
        g.play(HIT_JOB, json!({ "targets": at(&saintess) }));
        g.play(JILLIAX, json!({}));
        assert!(g.card(bread.as_str()).radiant);
        assert_eq!(g.stats(bread.as_str()).armor, 3);
        g.end_turn();

        // p1 eats it with Carnivorous Cube, and p2 kills the Cube on its next turn.
        assert_eq!(g.state().active, PlayerId::P1);
        g.play(CUBE, json!({ "targets": at(&bread) }));
        let cube = g.card(CUBE).id.clone();
        g.end_turn();
        g.play(HIT_JOB, json!({ "targets": at(&cube) }));

        let copies: Vec<String> = g
            .state()
            .players
            .p1
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| {
                card.def_id == BREAD && card.radiant && card.stats_override.map(|stats| stats.attack) == Some(3)
            })
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(copies.len(), 2);
        for copy in &copies {
            g.expect_stats(copy.as_str(), json!({ "attack": 3, "health": 3 }));
            assert_eq!(g.stats(copy.as_str()).armor, 3);
        }
    }
}

mod c8_52_r4_r78_a_rider_on_a_card_that_never_reached_the_hand {
    use super::*;

    #[test]
    fn r4_a_stolen_card_radiant_silly_silas_bounces_into_a_full_hand_is_burned_without_its_costing_0_r78_r747() {
        jackioh_cards::register_all();
        let fillers = [VANILLA; 9];
        // Two cards to play, then Reminisce and nine fillers: the hand is full when Silas bounces.
        let mut hand = vec![json!(MIND_CONTROL), json!({ "def": SILAS, "radiant": true }), json!(REMINISCE)];
        hand.extend(fillers.iter().map(|filler| json!(filler)));
        let mut g = scenario(json!({
            "p1": { "hand": hand, "mana": 10, "library": LIBRARY },
            "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 1 }], "library": LIBRARY },
        }));
        let seven = g.card(SEVEN_SEVEN).id.clone();

        // p1 steals the 7/7 into its own lane 1 (R15). Rotating left, p1's lane 1 would move to p2's
        // lane 1 — to the opponent — so radiant Silas bounces it "costing 0" to its CONTROLLER's hand
        // (R747), but that hand is full, so it is burned instead (§2.4, R4) into the graveyard of the
        // hand's player, p1, whose card it became.
        g.play(MIND_CONTROL, json!({ "targets": at(&seven) }));
        assert_eq!(g.unit(PlayerId::P1, 1).map(|card| card.id.clone()), Some(seven.clone()));
        assert_eq!(g.state().players.p1.hand.len(), 11);
        g.play(SILAS, json!({ "zone": 3, "modes": ["left"] }));
        g.expect_in_zone(seven.as_str(), "graveyard");
        assert!(g.state().players.p1.graveyard.iter().any(|card| card.id == seven));
        assert_eq!(g.card(seven.as_str()).owner, PlayerId::P1);
        assert_eq!(g.card(seven.as_str()).cost_override, None);

        // p1 Reminisces it back: "it costs 1 less", so the printed 4 becomes 3.
        g.play(REMINISCE, json!({}));
        g.answer(json!(seven));
        g.expect_in_zone(seven.as_str(), "hand");
        assert_eq!(effective_cost(g.state(), g.card(seven.as_str()), CostOptions::default()), 3);
        assert_eq!(g.card(seven.as_str()).cost_override, None);
    }

    #[test]
    fn r747_a_stolen_card_radiant_silly_silas_bounces_is_its_controllers_card_in_their_hand_at_cost_0_and_they_can_play_it() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [MIND_CONTROL, { "def": SILAS, "radiant": true }], "mana": 10, "library": LIBRARY },
            "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 1 }], "library": LIBRARY },
        }));
        let seven = g.card(SEVEN_SEVEN).id.clone();

        g.play(MIND_CONTROL, json!({ "targets": at(&seven) }));
        g.play(SILAS, json!({ "zone": 3, "modes": ["left"] }));

        g.expect_in_zone(seven.as_str(), "hand");
        assert!(g.state().players.p1.hand.iter().any(|card| card.id == seven));
        assert_eq!(g.state().players.p2.hand.len(), 0);
        assert_eq!(g.card(seven.as_str()).owner, PlayerId::P1);
        assert_eq!(g.card(seven.as_str()).controller, PlayerId::P1);
        assert_eq!(g.card(seven.as_str()).cost_override, Some(0));
        // It is p1's own card now, so it is priced and played as one (a held card is read by its holder).
        assert_eq!(effective_cost(g.state(), g.card(seven.as_str()), CostOptions::default()), 0);
        g.play(SEVEN_SEVEN, json!({ "zone": 4 }));
        assert_eq!(g.unit(PlayerId::P1, 4).map(|card| card.id.clone()), Some(seven.clone()));
    }
}

mod r77_r175_a_fuse_onto_a_token_summoned_x_x {
    use super::*;

    #[test]
    fn r77_unlicensed_experimentation_fusing_a_7_7_onto_a_3_3_bread_token_makes_a_10_10_not_a_3_3_r175_7() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 10,
            "p1": {
                "field": [{ "def": BREAD, "lane": 1, "statsOverride": { "attack": 3, "health": 3 } }],
                "backrow": [{ "def": EXPERIMENTATION, "lane": 1 }],
                "hand": [STOCKPILE],
                "library": LIBRARY,
            },
            "p2": { "hand": [SEVEN_SEVEN, STOCKPILE], "library": LIBRARY },
        }));
        let bread = unit_at(&g, PlayerId::P1, 1).id;
        g.expect_stats(bread.as_str(), json!({ "attack": 3, "maxHealth": 3 }));

        g.play(SEVEN_SEVEN, json!({ "zone": 1 }));

        // R77 keeps the Bread Token's instance, and sums the two faces: its X/X and the 7/7.
        let fused = g.card(bread.as_str()).clone();
        assert_ne!(fused.def_id, BREAD);
        assert!(g.unit(PlayerId::P2, 1).is_none());
        g.expect_stats(fused.id.as_str(), json!({ "attack": 10, "maxHealth": 10 }));
    }

    #[test]
    fn r77_a_7_7s_armor_7_fused_onto_bread_and_butters_bread_token_stays_armor_7_not_the_tokens_x_7() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [STOCKPILE],
                "backrow": [{ "def": BREAD_AND_BUTTER, "lane": 1 }, { "def": EXPERIMENTATION, "lane": 2 }],
                "library": LIBRARY,
            },
            "p2": { "hand": [SEVEN_SEVEN, STOCKPILE], "library": LIBRARY },
        }));

        // p1 ends the turn with 4 unspent: a 4/4 Bread Token, whose radiant "Armor X" is carried as 4.
        g.end_turn();
        let bread = unit_at(&g, PlayerId::P1, 1);
        assert_eq!(bread.def_id, BREAD);
        g.expect_stats(bread.id.as_str(), json!({ "attack": 4, "maxHealth": 4 }));
        assert_eq!(g.stats(bread.id.as_str()).armor, 0);

        g.play(SEVEN_SEVEN, json!({ "zone": 1 }));

        // R77 unions the keywords: the base Bread Token prints none, the 7/7 prints Armor 7. The
        // token's X belongs to the Bread Token's own radiant Armor, not to every Armor on the face.
        let fused = g.card(bread.id.as_str()).clone();
        assert_ne!(fused.def_id, BREAD);
        assert!(g.stats(fused.id.as_str()).keywords.contains(&Keyword::Armor { n: 7 }));
        assert_eq!(g.stats(fused.id.as_str()).armor, 7);
    }
}

mod r35_3_2_a_transform_replaces_the_occupant_of_a_locked_zone {
    use super::*;

    #[test]
    fn r35_transmogulate_replaces_a_heroic_power_that_magic_jammed_could_not_destroy_although_its_zone_is_locked_3_2_r46() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [MAGIC_JAMMED, TRANSMOGULATE, STOCKPILE],
                "backrow": [{ "def": HEROIC_POWER, "lane": 1 }],
                "library": LIBRARY,
            },
            "p2": { "hand": [STOCKPILE], "library": LIBRARY },
        }));
        let power = g.card(HEROIC_POWER).id.clone();

        // Indestructible: the Field Spell simply stays (R46), in a zone that is now Locked.
        g.play(MAGIC_JAMMED, json!({ "targets": at(&power) }));
        assert_eq!(g.backrow(PlayerId::P1, 1).map(|card| card.id.clone()), Some(power.clone()));
        assert!(g.state().players.p1.locks.backrow[0]);

        // R35: every board card but an Immutable one is replaced in place; the lock only stops summons
        // and "the current occupant is unaffected" (§3.2). Since patch v0.2.9 (issue #44) the Legendary
        // Field Spell pool holds #93, Classic #4 and #7, Classic #28 and Classic+ #78; this seed draws
        // Classic #4 Palantir.
        g.play(TRANSMOGULATE, json!({}));
        assert_eq!(g.backrow(PlayerId::P1, 1).map(|card| card.def_id.clone()), Some(PALANTIR.to_string()));
        g.expect_in_zone(power.as_str(), "gone");
    }
}

mod r174_a_later_part_of_one_cry_meets_the_stay_the_play_chose {
    use super::*;

    #[test]
    fn r174_a_fused_cube_sorcerers_damage_aimed_at_the_meal_does_not_land_on_the_meals_reborn_body_r83() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [CUBE, VANILLA], "library": LIBRARY },
            "p2": {
                "hand": [{ "def": SILAS, "radiant": true }, TIMMY, VANILLA],
                "field": [{ "def": SORCERER, "lane": 5 }],
                "backrow": [{ "def": EXPERIMENTATION, "lane": 3 }],
                "library": LIBRARY,
            },
        }));
        let sorcerer = unit_at(&g, PlayerId::P2, 5).id;
        // p1 plays the Cube with nothing to eat (R41); p2's #85 fuses it onto the Sorcerer (R77).
        g.play(CUBE, json!({ "zone": 2 }));
        let fused = g.card(sorcerer.as_str()).clone();
        assert_ne!(fused.def_id, SORCERER);
        g.end_turn();
        // p2's radiant Silas rotates right: the fused unit in lane 5 would cross, so it is bounced to
        // p2's hand costing 0 (§8 #52 radiant, R14).
        let silas = must(
            g.hand(PlayerId::P2).iter().find(|card| card.def_id == SILAS).map(|card| card.id.clone()),
            "p2's Silas",
        );
        g.play(silas.as_str(), json!({ "zone": 3, "modes": ["right"] }));
        g.expect_in_zone(fused.id.as_str(), "hand");
        g.play(TIMMY, json!({ "zone": 1 }));
        let saintess = unit_at(&g, PlayerId::P2, 1).id;
        // A Timmy that has been granted Reborn (Plastic Surgery's pool, R21), with no shield or Armor to
        // hide whether a hit lands on its body.
        grant(&mut g, &saintess, vec![Keyword::Reborn]);
        // Replayed: the fused Cry runs the Cube's part (eat the Timmy) and then the Sorcerer's
        // (4 damage), both aimed at the Timmy.
        let meal = json!({ "pick": "instance", "instanceId": saintess });
        g.play(fused.id.as_str(), json!({ "zone": 5, "targets": [meal, meal] }));
        // The meal died and came back through Reborn (§4.5 step 4), a new arrival (R83): the damage aimed
        // at the stay that died fizzles (R174), so the body stands at 1 health.
        g.expect_in_zone(saintess.as_str(), "field");
        assert_eq!(g.card(saintess.as_str()).reborn_spent, Some(true));
        g.expect_stats(saintess.as_str(), json!({ "health": 1 }));
    }

    #[test]
    fn r174_a_fused_cube_postdoc_summons_no_copy_of_the_meal_its_own_cube_part_sacrificed_r57() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [CUBE, VANILLA], "library": LIBRARY },
            "p2": {
                "hand": [{ "def": SILAS, "radiant": true }, TIMMY, VANILLA],
                "field": [{ "def": POSTDOC, "lane": 5 }],
                "backrow": [{ "def": EXPERIMENTATION, "lane": 3 }],
                "library": LIBRARY,
            },
        }));
        let postdoc = unit_at(&g, PlayerId::P2, 5).id;
        g.play(CUBE, json!({ "zone": 2 }));
        let fused = g.card(postdoc.as_str()).clone();
        assert_ne!(fused.def_id, POSTDOC);
        g.end_turn();
        let silas = must(
            g.hand(PlayerId::P2).iter().find(|card| card.def_id == SILAS).map(|card| card.id.clone()),
            "p2's Silas",
        );
        g.play(silas.as_str(), json!({ "zone": 3, "modes": ["right"] }));
        g.expect_in_zone(fused.id.as_str(), "hand");
        g.play(TIMMY, json!({ "zone": 1 }));
        let timmy = unit_at(&g, PlayerId::P2, 1).id;
        let before = g.events().len();
        // The Cube's part eats Timmy, and the Postdoc's part asks for a Vanilla copy of the same Timmy.
        let meal = json!({ "pick": "instance", "instanceId": timmy });
        g.play(fused.id.as_str(), json!({ "zone": 5, "targets": [meal, meal] }));
        g.expect_in_zone(timmy.as_str(), "graveyard");
        // Timmy's stay on the field ended with the sacrifice: the copy aimed at it fizzles (R174), rather
        // than a copy being made of a card in a graveyard.
        let copies: Vec<GameEvent> = g.events()[before..]
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == TIMMY))
            .cloned()
            .collect();
        assert_eq!(copies, Vec::<GameEvent>::new());
    }

    #[test]
    fn r174_a_fused_silas_kpops_delayed_steal_fizzles_on_a_target_the_silas_part_bounced_even_once_it_is_replayed_r76_r14() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [SILAS, VANILLA, VANILLA, VANILLA],
                "backrow": [{ "def": MANA_WELL, "lane": 1 }, { "def": MANA_WELL, "lane": 2 }],
                "library": LIBRARY,
            },
            "p2": {
                // The Coin pays for Flood beside Magic Jammed: Flood costs (4) since patch v0.2.0 (issue #40).
                "hand": [MAGIC_JAMMED, FLOOD, VANILLA, VANILLA, COIN],
                "field": [{ "def": KPOP, "lane": 3 }],
                "backrow": [{ "def": EXPERIMENTATION, "lane": 3 }],
                "library": LIBRARY,
            },
        }));
        let kpop = unit_at(&g, PlayerId::P2, 3).id;
        let jammed = must(g.backrow(PlayerId::P1, 1).map(|card| card.id.clone()), "p1's backrow lane 1");
        let prey = must(g.backrow(PlayerId::P1, 2).map(|card| card.id.clone()), "p1's backrow lane 2");
        // Turn 9, p1: Silas rotates right (from p1's seat: p1's backrow 1 -> 2, 2 -> 3; p2's Kpop 3 -> 2),
        // then p2's #85 fuses the played Silas onto the Kpop: the fused Cry runs Silas's part first.
        g.play(SILAS, json!({ "zone": 3, "modes": ["right"] }));
        let fused = g.card(kpop.as_str()).clone();
        assert_ne!(fused.def_id, KPOP);
        assert_eq!(g.backrow(PlayerId::P1, 3).map(|card| card.id.clone()), Some(prey.clone()));
        assert_eq!(g.backrow(PlayerId::P1, 2).map(|card| card.id.clone()), Some(jammed.clone()));
        g.end_turn();
        // Turn 10, p2: Magic Jammed locks p1's backrow lane 2, and Flood returns the fused unit to hand.
        g.play(COIN, json!({}));
        g.play(MAGIC_JAMMED, json!({ "targets": at(&jammed) }));
        g.play(FLOOD, json!({}));
        g.expect_in_zone(fused.id.as_str(), "hand");
        until_active(&mut g, PlayerId::P1);
        until_active(&mut g, PlayerId::P2);
        // Turn 12, p2: the fused card again. Silas's part rotates right (from p2's seat p1's backrow
        // 3 -> 2), and lane 2 is Locked, so the prey is bounced to p1's hand (R14); the Kpop part then
        // schedules the steal of the prey, which has already left the field.
        g.play(fused.id.as_str(), json!({ "zone": 1, "modes": ["right"], "targets": at(&prey) }));
        g.expect_in_zone(prey.as_str(), "hand");
        // Turn 13, p1 plays the prey again: a new arrival (R78, R83).
        until_active(&mut g, PlayerId::P1);
        g.play(prey.as_str(), json!({ "zone": 4 }));
        g.expect_in_zone(prey.as_str(), "field");
        // Turn 14, p2's start of turn: the steal was aimed at a stay that had already ended (R174, R76).
        until_active(&mut g, PlayerId::P2);
        assert_eq!(g.card(prey.as_str()).controller, PlayerId::P1);
    }
}

const SCARAB: &str = "core-007";
const RENO: &str = "core-053";

mod r174_a_later_part_of_one_effect_list_meets_the_stay_the_play_chose_across_a_prompt_too {
    use super::*;

    #[test]
    fn r174_a_crafted_cube_scarab_sorcerer_that_eats_a_reborn_unit_does_not_hit_its_reborn_body_after_the_discover_r113_r83_r102()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "seed": "r6craft-1057",
            "p1": {
                "hand": [STOCKPILE],
                "mana": 4,
                "field": [{ "def": TIMMY, "lane": 1 }],
            },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": RENO, "lane": 1 }] },
        }));
        // Granted Reborn (Plastic Surgery's pool, R21): no shield or Armor hides whether a hit lands.
        let timmy = must(g.unit(PlayerId::P1, 1).map(|card| card.id.clone()), "Tempo Timmy");
        grant(&mut g, &timmy, vec![Keyword::Reborn]);
        // Radiant Craft a Card's three-ingredient card, made as the card makes it (R77): its Discovers
        // draw from every set's Units since patch v0.2.0 (R380), so the test builds the card directly
        // rather than hunting a seed that offers these three.
        let card = craft(&mut g, PlayerId::P1, &[CUBE, SCARAB, SORCERER]);
        let saintess = must(g.unit(PlayerId::P1, 1).map(|card| card.id.clone()), "the Reborn Timmy");
        let aim = json!({ "pick": "instance", "instanceId": saintess });

        // The Cube's part eats the Timmy and it is straight back through Reborn, a new arrival
        // (R78, R83); the Scarab's part then asks, which ends the action with the Sorcerer's part owed.
        g.play(card.id.as_str(), json!({ "zone": 2, "targets": [aim, aim] }));
        assert_eq!(g.state().pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Discover));
        assert_eq!(g.unit(PlayerId::P1, 1).map(|card| card.id.clone()), Some(saintess.clone()));
        let before = g.events().len();

        // R174: the Sorcerer's 4 damage is aimed at the stay the Cube's part ended, so it fizzles — as it
        // does when nothing asks in between. The answer resuming the list (R113) changes nothing.
        let option = must(
            g.state().pending.as_ref().and_then(|pending| pending.options.first()).cloned(),
            "a Discover option",
        );
        g.answer(json!([option.selection]));
        let hits = g.events()[before..]
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { target_id, .. } if *target_id == saintess))
            .count();
        assert_eq!((hits, g.card(saintess.as_str()).zone.z()), (0, ZoneName::Field));
    }
}

// ---------------------------------------------------------------------------
// Round 7 (lens L2): "this" card is aimed at its stay too.
// ---------------------------------------------------------------------------

const GARY: &str = "core-004";
const GLOWY_JELLY_BEAN: &str = "core-026";

/// §8 #99 Craft a Card's result, made the way the card makes it (R77): the definitions go into
/// `subsystems.fuse` as ingredients that were never cards, and the result is a fresh, non-Radiant
/// hand card costing 0.
fn craft(g: &mut Scenario, player: PlayerId, def_ids: &[&str]) -> CardInstance {
    let mut events: Vec<GameEvent> = vec![];
    let mut rng = Rng::new(&g.state().seed, g.state().rng_cursor);
    let made = {
        let state = g.state_mut();
        let ingredients: Vec<CardInstance> = def_ids
            .iter()
            .map(|def_id| new_instance(&mut *state, def_id, player, Zone::Gone { player }))
            .collect();
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        subsystems::fuse(&mut sink, FuseArgs { ingredients, to_hand: Some(player), ..FuseArgs::default() })
    };
    g.state_mut().rng_cursor = rng.cursor();
    must(made, "the crafted card")
}

mod r174_r78_a_later_part_of_a_cry_acting_on_the_card_itself_once_an_earlier_part_took_it_off_the_field {
    use super::*;

    #[test]
    fn r174_a_radiant_crafted_silly_silas_gary_that_its_own_silas_part_bounces_is_not_buffed_in_hand_by_its_gary_part_r78_8_52_radiant()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [GLOWY_JELLY_BEAN, STOCKPILE], "mana": 4 },
            "p2": { "hand": [STOCKPILE] },
        }));
        let card = craft(&mut g, PlayerId::P1, &[SILAS, GARY]).id;
        // #26 Glowy Jelly Bean makes the crafted card Radiant, so its Silas part is radiant #52's text.
        g.play(GLOWY_JELLY_BEAN, json!({ "targets": at(&card) }));
        assert!(g.card(card.as_str()).radiant);

        // Played into lane 5 and rotated right: the card itself would cross to p2's side, so radiant
        // Silas bounces it to p1's hand costing 0 (R14). It has left the field, and R78 has reset it.
        g.play(card.as_str(), json!({ "zone": 5, "modes": ["right"] }));
        g.expect_in_zone(card.as_str(), "hand");

        // The Gary part comes next in the same Cry. It is aimed at the card on the field, whose stay has
        // ended (R174: "a fused card's part aimed at a card an earlier part has taken off the field
        // fizzles"), so the coins buff nothing — least of all a card in a hand, which R78 has just made
        // the printed card again.
        assert_eq!(g.card(card.as_str()).buffs, AttackHealth { attack: 0, health: 0 });
    }
}

mod r174_a_transformed_card_has_left_the_field {
    use super::*;

    #[test]
    fn r174_a_second_sheepish_is_not_offered_the_play_the_first_one_already_turned_into_a_sheep_r17_r61() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "seed": "edge-r8-sheepish",
            "p1": { "hand": [TIMMY, HINDER], "mana": 10 },
            "p2": {
                "backrow": [
                    { "def": SHEEPISH, "lane": 1 },
                    { "def": SHEEPISH, "lane": 2 },
                ],
            },
        }));
        let (Some(first), Some(second)) = (
            g.backrow(PlayerId::P2, 1).map(|card| card.id.clone()),
            g.backrow(PlayerId::P2, 2).map(|card| card.id.clone()),
        ) else {
            panic!("setup: two Sheepish");
        };

        g.play(TIMMY, json!({ "zone": 1 }));

        // The first Sheepish turned the played unit into a Sheep, which is no longer the card played.
        assert_eq!(unit_at(&g, PlayerId::P1, 1).def_id, SHEEP);
        g.expect_in_zone(first.as_str(), "graveyard");
        // A trap answering the play behind one that took the card off the field is not offered it, so
        // the second Sheepish stays armed and face-down rather than firing for nothing.
        assert_eq!(
            g.last_events().iter().filter(|event| matches!(event, GameEvent::TrapFired { .. })).count(),
            1
        );
        assert_eq!(g.backrow(PlayerId::P2, 2).map(|card| card.id.clone()), Some(second));
    }
}

mod r102_r212_a_card_a_fuse_kept_is_the_same_card_on_the_same_stay {
    use super::*;

    #[test]
    fn r102_fed_fauci_hit_by_twisted_sorcerers_cry_and_then_fused_with_the_sorcerer_by_unlicensed_experimentation_still_gains_its_plague_counter_r77_r212()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "seed": "edge-r8-fauci-fuse",
            "p1": { "hand": [SORCERER, HINDER], "mana": 10 },
            "p2": { "field": [{ "def": FAUCI, "lane": 1 }], "backrow": [{ "def": EXPERIMENTATION, "lane": 1 }] },
        }));
        let fauci = unit_at(&g, PlayerId::P2, 1).id;

        g.play(SORCERER, json!({ "zone": 1, "targets": at(&fauci) }));

        // The Sorcerer's Cry hit Fauci for 4, and the trap fused the Sorcerer onto Fauci (R77: the
        // instance is kept, on the same stay, carrying both texts — Fauci's trigger included, R102).
        assert!(
            g.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Damage { target_id, .. } if *target_id == fauci))
        );
        assert!(g.events().iter().any(
            |event| matches!(event, GameEvent::Fused { result_instance_id, .. } if *result_instance_id == fauci)
        ));
        let kept = g.card(fauci.as_str()).clone();
        assert_eq!(kept.zone.z(), ZoneName::Field);
        assert_eq!(kept.damage, 4);
        // "Whenever this takes damage, +1 Plague Counter": the hit happened to this card on this stay
        // (R212), and the Fuse neither moved it nor dropped Fauci's text, so the token lands.
        assert_eq!(kept.counters.plague.unwrap_or(0), 1);
    }
}

// ---------------------------------------------------------------------------
// Round 9: meals, Reborn bodies and buried picks (R174, R102, §4.5 step 4, §3.2)
// ---------------------------------------------------------------------------

const MR_VANILLA: &str = "core-008";
const MIDRANGE_MENACE: &str = "core-019";
const MROW: &str = "core-086";
const NETHER: &str = "core-088";
const UNLICENSED: &str = "core-085";

/// Craft a Card's and #85's fusions are built directly, as `fused-hooks.test.ts` does (R77). TS
/// `subsystems.fuse(sinkFor(s), args)`: a fresh sink over the live state, whose rng cursor is not
/// written back (as TS's `sinkFor` did not).
fn fuse_on(s: &mut Scenario, args: FuseArgs) -> Option<CardInstance> {
    let mut events: Vec<GameEvent> = vec![];
    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
    subsystems::fuse(&mut sink, args)
}

/// TS `const hand = s.hand("p1")`: a copy of the hand as it stands.
fn hand_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
    s.hand(player)
}

mod r174_r41_a_cubes_meal_is_read_on_the_stay_the_play_chose {
    use super::*;

    #[test]
    fn r174_a_crafted_cube_cube_that_names_the_same_reborn_unit_twice_eats_it_once_and_remembers_it_once_so_its_death_copies_it_twice_not_four_times_r41_r83()
     {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "r9-cube-cube-reborn",
            "p1": {
                "mana": 10,
                "hand": [CUBE, CUBE, HIT_JOB, RENO],
                // A Right-house defender: Reborn and no Death of its own. (A Radiant Saintess's Death would
                // make the crafted Cube Radiant, and a Radiant Cube fills the board whatever it remembers.)
                "field": [{ "def": RIGHT_HOUSE, "lane": 1 }],
                "library": LIBRARY,
            },
            "p2": { "hand": [STOCKPILE], "field": [MIDRANGE_MENACE], "library": LIBRARY },
        }));
        let cubes: Vec<CardInstance> = hand_of(&s, PlayerId::P1).into_iter().filter(|card| card.def_id == CUBE).collect();
        let crafted = must(
            fuse_on(&mut s, FuseArgs { ingredients: cubes, to_hand: Some(PlayerId::P1), ..FuseArgs::default() }),
            "the crafted card",
        );
        let defender = must(s.unit(PlayerId::P1, 1).map(|card| card.id.clone()), "p1's Right-house defender");

        // The defender is p1's only other permanent, so each Cube's declaration takes it: this is the
        // one play legalActions offers for the crafted card (R81, R90).
        let picks = json!([
            { "pick": "instance", "instanceId": defender },
            { "pick": "instance", "instanceId": defender },
        ]);
        s.play(crafted.id.as_str(), json!({ "zone": 3, "targets": picks }));

        // The first Cube's part ate it and Reborn brought it straight back (§4.5 step 4). The second
        // part's sacrifice fizzles on the body (R174): it died once.
        assert_eq!(s.card(defender.as_str()).zone.z(), ZoneName::Field);
        assert_eq!(
            s.events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == defender))
                .count(),
            1
        );
        assert!(!s.card(crafted.id.as_str()).radiant);

        // R41: "nothing eaten → Death does nothing", so only the first Cube's Death summons copies.
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": crafted.id }] }));
        let defenders = (1..=5)
            .filter_map(|lane| s.unit(PlayerId::P1, lane).map(|card| card.def_id.clone()))
            .filter(|id| id == RIGHT_HOUSE)
            .count();
        // Its Reborn body plus the first Cube's 2 copies; the second Cube remembered nothing.
        assert_eq!(defenders, 3);
    }
}

mod c4_5_step_4_r89_the_reborn_bodies_of_one_check_return_together {
    use super::*;

    /// p1 plays a Felinor Fiender crafted with a Right-house defender (6/8, Stack, Taunt, Divine Shield,
    /// Reborn, and the Fiender's layer 2) and a "Miss" Mrow crafted with one (2/2 Felinor, Reborn) into the two lanes
    /// given, then destroys both with Twisting Nether. Both die in one state check and both come back
    /// through Reborn. Returns the Fiender body's health afterwards.
    fn fiender_after_reborn(fiender_lane: i32, mrow_lane: i32) -> i32 {
        let mut s = scenario(json!({
            "seed": "r9-reborn-order",
            "p1": {
                "mana": 10,
                "hand": [RIGHT_HOUSE, FIENDER, RIGHT_HOUSE, MROW, NETHER, RENO],
                "library": LIBRARY,
            },
            "p2": { "hand": [STOCKPILE], "library": LIBRARY },
        }));
        let hand = hand_of(&s, PlayerId::P1);
        let saints: Vec<CardInstance> = hand.iter().filter(|card| card.def_id == RIGHT_HOUSE).cloned().collect();
        let fiender_in = must(hand.iter().find(|card| card.def_id == FIENDER).cloned(), "Felinor Fiender in hand");
        let mrow_in = must(hand.iter().find(|card| card.def_id == MROW).cloned(), "Mrow in hand");
        let reborn_fiender = must(
            fuse_on(
                &mut s,
                FuseArgs {
                    ingredients: vec![fiender_in, must(saints.first().cloned(), "a Right-house defender")],
                    to_hand: Some(PlayerId::P1),
                    ..FuseArgs::default()
                },
            ),
            "Fiender + Right-house defender",
        );
        let reborn_mrow = must(
            fuse_on(
                &mut s,
                FuseArgs {
                    ingredients: vec![mrow_in, must(saints.get(1).cloned(), "a Right-house defender")],
                    to_hand: Some(PlayerId::P1),
                    ..FuseArgs::default()
                },
            ),
            "Mrow + Right-house defender",
        );
        s.play(reborn_fiender.id.as_str(), json!({ "zone": fiender_lane }));
        s.play(reborn_mrow.id.as_str(), json!({ "zone": mrow_lane }));
        // Layer 2: printed 6/8 plus the crafted Mrow's 2/2 (R116).
        assert_eq!(s.stats(reborn_fiender.id.as_str()).max_health, 8 + 2);

        s.play(NETHER, json!({}));
        s.expect_in_zone(reborn_fiender.id.as_str(), "field");
        s.expect_in_zone(reborn_mrow.id.as_str(), "field");
        assert_eq!(s.stats(reborn_fiender.id.as_str()).max_health, 8 + 2);
        s.stats(reborn_fiender.id.as_str()).health
    }

    #[test]
    fn c4_5_a_felinor_fiender_and_the_felinor_feeding_it_that_come_back_through_reborn_in_one_check_leave_the_fiender_at_the_same_health_whichever_lane_is_first_r89_r116()
     {
        jackioh_cards::register_all();
        // Mrow first: the Fiender's body is read with Mrow back, 10 max health and 1 left. Fiender first:
        // it is read with Mrow still in the graveyard, 8 max and 1 left, and Mrow's return then lifts it
        // to 3. §4.5 step 4 returns every collected Reborn unit in one step, at 1 health.
        assert_eq!(fiender_after_reborn(1, 2), fiender_after_reborn(2, 1));
    }
}

mod c3_2_r13_r174_a_card_the_plays_own_stack_buried_is_not_on_the_field_for_its_cry {
    use super::*;

    /// p1 crafts a Felinor Fiender (Stack) with `partner`, whose Cry chooses one of p1's units, and
    /// plays it onto lane 1, where Mr. Vanilla stands, choosing Mr. Vanilla. Step 4 puts the crafted
    /// card on top of the pile, so by step 5 Mr. Vanilla is dormant under it.
    fn bury_own_pick(partner: &str, seed: &str) -> (Scenario, CardInstance, CardInstance) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "mana": 10, "hand": [FIENDER, partner, RENO], "field": [{ "def": MR_VANILLA, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [STOCKPILE], "library": LIBRARY },
        }));
        let hand = hand_of(&s, PlayerId::P1);
        let fiender = must(hand.iter().find(|card| card.def_id == FIENDER).cloned(), "Felinor Fiender in hand");
        let other = must(hand.iter().find(|card| card.def_id == partner).cloned(), "the partner in hand");
        let crafted = must(
            fuse_on(
                &mut s,
                FuseArgs { ingredients: vec![fiender, other], to_hand: Some(PlayerId::P1), ..FuseArgs::default() },
            ),
            "the crafted card",
        );
        let vanilla = must(s.unit(PlayerId::P1, 1), "Mr. Vanilla");
        s.play(crafted.id.as_str(), json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
        (s, crafted, vanilla)
    }

    #[test]
    fn r174_a_crafted_fiender_prejudiced_postdoc_played_onto_its_chosen_human_copies_nothing_once_that_human_is_buried_3_2_r13()
     {
        jackioh_cards::register_all();
        let (mut s, crafted, vanilla) = bury_own_pick(POSTDOC, "r9-buried-postdoc");
        // Step 4 buried Mr. Vanilla under the crafted card: it is dormant, not on the field (R13).
        assert_eq!(s.unit(PlayerId::P1, 1).map(|card| card.id.clone()), Some(crafted.id.clone()));
        s.expect_in_zone(vanilla.id.as_str(), "field");
        // "Choose a Human unit on the field; summon a Vanilla copy": the chosen unit is no longer on the
        // field, so the copy fizzles, as #68's damage does on a buried pick. The engine summons one.
        let rest: Vec<Option<String>> =
            (2..=5).map(|lane| s.unit(PlayerId::P1, lane).map(|card| card.def_id.clone())).collect();
        assert_eq!(rest, vec![None, None, None, None]);
    }

    #[test]
    fn r174_a_crafted_fiender_carnivorous_cube_played_onto_its_chosen_meal_does_not_eat_the_card_buried_beneath_it_3_2_r13_r41()
     {
        jackioh_cards::register_all();
        let (mut s, crafted, vanilla) = bury_own_pick(CUBE, "r9-buried-cube");
        assert_eq!(s.unit(PlayerId::P1, 1).map(|card| card.id.clone()), Some(crafted.id.clone()));
        // The meal is dormant under the crafted card when the Cry resolves, so there is nothing on the
        // field to tribute and the Cry fizzles (R41). The engine sacrifices it out of the pile.
        s.expect_in_zone(vanilla.id.as_str(), "field");
        assert!(
            !s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == vanilla.id))
        );
    }
}

/// What p2's Cube eats in the cases below: one of p2's own Units (R428), no Timmy and no Mr. Vanilla.
const P2_MEAL: &str = "core-019";

mod r102_r77_r41_a_card_85_keeps_reads_its_meals_with_its_own_text {
    use super::*;

    /// p1's unit row as def ids, lane 1 to 5.
    fn unit_row(s: &Scenario) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(PlayerId::P1, lane).map(|card| card.def_id.clone())).collect()
    }

    fn count(row: &[Option<String>], def_id: &str) -> usize {
        row.iter().filter(|id| id.as_deref() == Some(def_id)).count()
    }

    #[test]
    fn r102_a_cube_that_ate_once_and_had_a_played_cube_fused_onto_it_copies_its_one_meal_twice_not_four_times_r77_r41() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "r9-cube-kept-meal",
            "p1": {
                "mana": 10,
                "hand": [CUBE, RENO, HIT_JOB],
                "field": [{ "def": TIMMY, "lane": 1 }],
                "backrow": [UNLICENSED],
                "library": LIBRARY,
            },
            "p2": { "hand": [CUBE, STOCKPILE], "field": [{ "def": P2_MEAL, "lane": 1 }], "library": LIBRARY },
        }));
        let timmy = must(s.unit(PlayerId::P1, 1).map(|card| card.id.clone()), "Tempo Timmy");
        s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": timmy }] }));
        let kept = must(s.unit(PlayerId::P1, 2).map(|card| card.id.clone()), "p1's Cube");

        // p2's Cube eats p2's Midrange Menace (a Unit: R428); after its Cry, #85 fuses it onto p1's Cube,
        // the only unit p1 has (R61). The kept instance is p1's Cube, and its memory is the Timmy it ate (R77).
        s.end_turn();
        let menace = must(s.unit(PlayerId::P2, 1).map(|card| card.id.clone()), "p2's Midrange Menace");
        s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": menace }] }));
        assert!(s.card(kept.as_str()).def_id.contains("core-022+core-022"));

        s.end_turn();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": kept }] }));

        // The kept Cube's text copies its meal twice ("each Death copies its own", R102). The played
        // Cube's text ate a Midrange Menace on another instance, never a Timmy. The engine hands the kept
        // card's one meal to both texts and summons four Timmies.
        assert_eq!(count(&unit_row(&s), TIMMY), 2);
    }

    #[test]
    fn r77_a_fuse_moves_what_the_kept_cards_own_text_remembered_to_the_place_that_text_now_runs_at_and_leaves_the_rest_of_its_memory_as_it_was_r102()
     {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "r11-r77-kept-memory",
            "p1": {
                "mana": 10,
                "hand": [CUBE, RENO],
                "field": [{ "def": TIMMY, "lane": 1 }],
                "backrow": [UNLICENSED],
                "library": LIBRARY,
            },
            "p2": { "hand": [CUBE, STOCKPILE], "field": [{ "def": P2_MEAL, "lane": 1 }], "library": LIBRARY },
        }));
        // p1's Cube eats Tempo Timmy: its text remembers the meal through `remember`, under "eaten".
        let timmy = must(s.unit(PlayerId::P1, 1).map(|card| card.id.clone()), "Tempo Timmy");
        s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": timmy }] }));
        let kept = must(s.unit(PlayerId::P1, 2).map(|card| card.id.clone()), "p1's Cube");
        let before = s.card(kept.as_str()).memory.clone();
        assert!(before.contains_key("eaten"));

        // p2's Cube, after its Cry, is fused onto p1's Cube by #85 (R61): the kept instance's texts are
        // one ingredient of the new fusion now, and the meal goes with them to that ingredient's path.
        s.end_turn();
        let menace = must(s.unit(PlayerId::P2, 1).map(|card| card.id.clone()), "p2's Midrange Menace");
        s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": menace }] }));
        let after = s.card(kept.as_str()).memory.clone();
        assert!(s.card(kept.as_str()).def_id.contains("core-022+core-022"));
        assert!(!after.contains_key("eaten"));
        let moved: Vec<String> = after.keys().filter(|key| key.starts_with("eaten@")).cloned().collect();
        assert_eq!(moved.len(), 1);
        let moved_key = moved.first().cloned().unwrap_or_default();
        assert_eq!(after.get(&moved_key), before.get("eaten"));
        // Every other entry is as it was; the only one the Fuse may add is the ingredients' prices.
        let mut rest_before = before.clone();
        rest_before.shift_remove("eaten");
        let mut rest_after = after.clone();
        rest_after.shift_remove(&moved_key);
        rest_after.shift_remove(INGREDIENTS_KEY);
        assert_eq!(rest_after, rest_before);
    }

    #[test]
    fn r102_a_crafted_cube_cube_that_85_fuses_a_played_cube_onto_still_copies_both_of_its_own_meals_r77() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "r9-crafted-cube-kept",
            "p1": {
                "mana": 10,
                "hand": [CUBE, CUBE, RENO, HIT_JOB],
                "field": [{ "def": TIMMY, "lane": 1 }, { "def": MR_VANILLA, "lane": 2 }],
                "backrow": [UNLICENSED],
                "library": LIBRARY,
            },
            "p2": { "hand": [CUBE, STOCKPILE], "field": [{ "def": P2_MEAL, "lane": 1 }], "library": LIBRARY },
        }));
        let cubes: Vec<CardInstance> = hand_of(&s, PlayerId::P1).into_iter().filter(|card| card.def_id == CUBE).collect();
        let crafted = must(
            fuse_on(&mut s, FuseArgs { ingredients: cubes, to_hand: Some(PlayerId::P1), ..FuseArgs::default() }),
            "Cube + Cube",
        );
        let timmy = must(s.unit(PlayerId::P1, 1).map(|card| card.id.clone()), "Tempo Timmy");
        let vanilla = must(s.unit(PlayerId::P1, 2).map(|card| card.id.clone()), "Mr. Vanilla");
        s.play(
            crafted.id.as_str(),
            json!({
                "zone": 3,
                "targets": [
                    { "pick": "instance", "instanceId": timmy },
                    { "pick": "instance", "instanceId": vanilla },
                ],
            }),
        );
        // R102: each Cube remembered its own meal.
        assert_eq!(unit_row(&s), vec![None, None, Some(crafted.def_id.clone()), None, None]);

        s.end_turn();
        let menace = must(s.unit(PlayerId::P2, 1).map(|card| card.id.clone()), "p2's Midrange Menace");
        s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": menace }] }));
        s.end_turn();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": crafted.id }] }));

        // The kept card "still reads what it remembered before" (R102): its two Cubes copy a Timmy twice
        // and a Mr. Vanilla twice. The engine reads its first meal off the played Cube's text and
        // nothing off its own, so only two Timmies arrive.
        let row = unit_row(&s);
        assert_eq!(count(&row, TIMMY), 2);
        assert_eq!(count(&row, MR_VANILLA), 2);
    }
}
