//! A play, and a cast, from §10.5 step 1 to step 8 (SPEC §2.3, §2.4, §4.5, §6.3 Sacrifice and
//! Tribute, §10.5, R17, R34, R57, R59, R65, R70, R210). Found by the polish-4 edge-case hunt, round 2
//! (docs/polish/4-edge-cases.md, lenses L2 and L7); every case here failed before its fix.
//!
//!  - R70: a cast is a play, so it runs §10.5's steps — Gifted Program's hook, Quickstriker's and
//!    /fullsend's granted Combos, its Echo repeats — and a cast-on-draw cast is whole, and the state
//!    check has run (§4.5, R59), before the draw repeats (§2.4).
//!  - R17: Unstable Clone Machine fires after the card resolves, and copies the face that resolved
//!    even when the card itself has ceased to exist (R34, R57).
//!  - R65: X is a play-time choice, so an X-cost Spell back in hand costs 0 again.
//!  - R210: the zone a play names is held while its Tribute is paid, and a tributed Reborn unit
//!    comes back (§6.1, §6.3 "counts as a death").
//!  - Round 5, lens L8. R217: a draw a cast-on-draw cast makes continues that cast's chain, so
//!    R58's cap bounds a CN-Virus chain under /fullsend's Combo draw, which recursed without end.
//!  - Round 6, lens "engine invariants". R58, §10.3: the resolution loop's cap holds every trigger a
//!    legal play can set off, so a Call to Chaos drawing a library of CN-Viruses beside #33 resolves.
//!
//! Port of `packages/cards/test/plays-and-casts.test.ts`.

use jackioh_cards::register_all;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

const RIGHT_HOUSE: &str = "core-003";
const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const TIMMY: &str = "core-011";
const HINDER: &str = "core-021";
const DIVIDEND: &str = "core-024";
const BLOOD_RIDDEN: &str = "core-027";
const CLONE_MACHINE: &str = "core-033";
const QUICKSTRIKER: &str = "core-038";
const SHEEPISH: &str = "core-041";
const RENO: &str = "core-053";
const GIFTED: &str = "core-064";
const ROCK: &str = "core-066";
const FULLSEND: &str = "core-078";
const TWINSPELL: &str = "core-079";
const INFINITE_RESERVES: &str = "core-075"; // an empty library gives a Rush Token card, not fatigue
const GOING_LONG: &str = "core-084"; // hero Armor 2, so CN-Virus's 1 damage is absorbed
const CN_VIRUS: &str = "core-090-1"; // Cast on draw: take 1 damage; shuffle 2 copies of this into your library
const LIBRARY: &[&str] = &[
    VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA,
];

/// TS `findIndex`: the first index matching, or -1.
fn find_index(events: &[GameEvent], matches: impl Fn(&GameEvent) -> bool) -> i64 {
    events.iter().position(matches).map_or(-1, |at| at as i64)
}

mod r70_a_cast_on_draw_card_is_cast_through_10_5_s_steps_and_is_whole_before_the_draw_repeats {
    use super::*;

    #[test]
    fn r70_a_cast_on_draw_spell_s_echo_repeat_resolves_before_the_draw_repeats_2_4_10_5_step_6() {
        register_all();
        // p1 has Twinspell's grant up. At p1's start of turn the draw takes #27 Blood Ridden Glowy Jelly
        // Bean, which casts itself (R70: a cast Spell takes Twinspell's Echo). Its first resolution finds
        // no card in p1's hand to make Radiant; its Echo repeat, which is part of the same cast
        // (§10.5 step 6), finds none either. Only then does the draw repeat (§2.4) and put Mr. Vanilla in
        // the hand — so Mr. Vanilla must still be non-Radiant.
        let mut g = scenario(json!({
            "p1": {
                "hand": [TWINSPELL],
                "field": [{ "def": VANILLA, "lane": 1 }],
                "library": [BLOOD_RIDDEN, VANILLA, VANILLA],
            },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }], "library": [VANILLA, VANILLA] },
        }));

        g.play(TWINSPELL, json!({}));
        g.end_turn(); // p1 → p2
        g.end_turn(); // p2 → p1: the start-of-turn draw casts Blood Ridden

        assert_eq!(g.state().active, P1);
        // Both resolutions ran: 5 health each (R18), and the grant was spent (R30).
        g.expect_health(P1, 20);
        g.expect_in_zone(BLOOD_RIDDEN, "graveyard");
        let drawn_after: Vec<CardInstance> = g.hand(P1).iter().filter(|card| card.def_id == VANILLA).cloned().collect();
        assert_eq!(drawn_after.len(), 1);
        assert_eq!(drawn_after.first().map(|card| card.radiant), Some(false));

        // The cast is whole before the draw repeats: its cardResolved precedes the next drawn card.
        let resolved_at = find_index(
            g.events(),
            |event| matches!(event, GameEvent::CardResolved { def_id, .. } if def_id == BLOOD_RIDDEN),
        );
        let next_draw_at = find_index(
            g.events(),
            |event| matches!(event, GameEvent::Drawn { player, def_id, .. } if *player == P1 && def_id == VANILLA),
        );
        assert!(resolved_at >= 0);
        assert!(next_draw_at > resolved_at);
    }

    #[test]
    fn r59_the_state_check_runs_after_each_cast_on_draw_cast_so_a_hero_at_0_ends_the_game_before_the_draw_repeats_4_5_2_4()
     {
        register_all();
        // §4.5: the check runs after "one cast-on-draw cast". p1 is at 5 and the start-of-turn draw casts
        // Blood Ridden Glowy Jelly Bean ("you lose 5 health"): p1's hero is at 0 when that cast resolves,
        // so the game ends there, and the draw never repeats into the Mr. Vanilla beneath it.
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 8,
            "p1": { "health": 5, "field": [{ "def": VANILLA, "lane": 1 }], "library": [BLOOD_RIDDEN, VANILLA, VANILLA] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }], "library": [VANILLA, VANILLA] },
        }));

        g.end_turn(); // p2 → p1: the draw casts Blood Ridden

        assert_eq!(g.state().result.as_ref().map(|result| result.winner), Some(Winner::P2));
        assert_eq!(g.hand(P1).iter().filter(|card| card.def_id == VANILLA).count(), 0);
        assert_eq!(g.pile(P1, "library").len(), 2);
    }

    #[test]
    fn r70_quickstriker_s_x_is_the_count_before_each_cast_of_a_cast_on_draw_chain_not_after_the_chain_8_38() {
        register_all();
        // p1 has Quickstriker. At p1's start of turn the draw casts Hinder, the draw repeats and casts a
        // second Hinder, and the draw repeats again into Mr. Vanilla (§2.4, R58). Each cast is a play
        // (R70), and §8 #38's X is "cards you played earlier this turn" — `turnLog.cardsPlayed` before
        // THIS card: 0 for the first Hinder, 1 for the second. So p2's hero takes 1 in all, not 1 + 1.
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 8,
            "p1": { "backrow": [QUICKSTRIKER], "field": [{ "def": VANILLA, "lane": 1 }], "library": [HINDER, HINDER, VANILLA] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }], "library": [RENO, RENO] },
        }));

        g.end_turn(); // p2 → p1: the draw chain casts both Hinders

        assert_eq!(g.state().active, P1);
        assert_eq!(g.state().players.p1.turn_log.cards_played, 2);
        g.expect_health(P2, 29);
    }

    #[test]
    fn r70_gifted_program_makes_a_cast_on_draw_card_radiant_as_it_is_cast_10_5_step_3() {
        register_all();
        // R70 names Gifted Program among the rules a cast counts for, with cost paid 0: "The first card
        // costing 1 or less you play each turn becomes Radiant as it is played" (§8 #64). p1's start-of-
        // turn draw casts Hinder, the first card p1 plays this turn, for 0 — so it resolves Radiant
        // ("2 lower") and stays Radiant in the graveyard (R78).
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 8,
            "p1": { "backrow": [GIFTED], "field": [{ "def": VANILLA, "lane": 1 }], "library": [HINDER, VANILLA] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }], "library": [RENO, RENO] },
        }));

        g.end_turn(); // p2 → p1: the draw casts Hinder

        assert_eq!(g.state().active, P1);
        let hinder = g.card(HINDER).clone();
        g.expect_in_zone(&hinder.id, "graveyard");
        assert!(hinder.radiant);
    }

    #[test]
    fn r70_fullsend_s_granted_combo_draw_1_also_fires_for_a_cast_on_draw_card_10_5_step_5() {
        register_all();
        // p1 plays /fullsend (this turn "your cards gain 'Combo: draw 1'"), then Mr. Vanilla: 1 card
        // played earlier, so Mr. Vanilla's granted Combo draws — Hinder, which casts itself. The cast is
        // a play for every rule that counts or reacts to plays, Combo named first (R70), with 2 cards
        // played earlier this turn, so Hinder's granted Combo draws 1 (a Reno). The Cry's random
        // discard (R682) resolves after that draw, so it eats the Reno; then the cast-on-draw draw
        // repeats (§2.4) and brings a second Reno, the one card left standing.
        let mut g = scenario(json!({
            // The Radiant /fullsend: the face that grants "Combo: Draw 1" since patch v0.1.1.
            "p1": { "hand": [{ "def": FULLSEND, "radiant": true }, VANILLA], "library": [HINDER, RENO, RENO, RENO, RENO] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }], "library": [RENO, RENO] },
        }));

        g.play(FULLSEND, json!({}));
        g.play(VANILLA, json!({}));

        assert_eq!(g.state().players.p1.turn_log.cards_played, 3);
        g.expect_in_zone(HINDER, "graveyard");
        assert_eq!(g.hand(P1).iter().filter(|card| card.def_id == RENO).count(), 1);
        assert_eq!(g.pile(P1, "library").len(), 2);
    }
}

mod r17_unstable_clone_machine_copies_the_card_after_it_resolves {
    use super::*;

    #[test]
    fn r17_unstable_clone_machine_shuffles_its_copies_in_after_the_played_spell_resolves_not_before_10_5_step_7() {
        register_all();
        // p1 has Unstable Clone Machine and an empty library, and plays Stockpile ("Draw 2; heal your
        // hero 2"). R17 and §10.5 step 7: the Clone Machine fires after the card resolves. So Stockpile's
        // two draws find the library empty (fatigue 1 then 2, §2.4), it heals 2, and only then do the 3
        // copies go into the library — none of them can be drawn by the Stockpile that made them.
        let mut g = scenario(json!({
            "p1": { "hand": [STOCKPILE], "backrow": [CLONE_MACHINE], "library": [] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }], "library": [RENO] },
        }));

        g.play(STOCKPILE, json!({}));

        assert_eq!(g.hand(P1).len(), 0);
        let library: Vec<String> = g.pile(P1, "library").iter().map(|card| card.def_id.clone()).collect();
        assert_eq!(library, vec![STOCKPILE, STOCKPILE, STOCKPILE]);
        assert_eq!(g.state().players.p1.fatigue_count, 2);
        g.expect_health(P1, 30 - 1 - 2 + 2);
    }

    #[test]
    fn r57_unstable_clone_machine_copies_a_radiant_tempo_timmy_that_sheepish_turned_into_a_sheep_as_radiant_r34() {
        register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [{ "def": TIMMY, "radiant": true }, VANILLA],
                "backrow": [{ "def": CLONE_MACHINE, "lane": 1 }],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "backrow": [{ "def": SHEEPISH, "lane": 1 }], "library": LIBRARY },
        }));
        let timmy = g.card(TIMMY).clone();
        g.play(TIMMY, json!({ "zone": 1 }));
        // Sheepish answered the play first: the Timmy is gone and a Sheep Token stands in its zone.
        assert!(g
            .events()
            .iter()
            .any(|event| matches!(event, GameEvent::Transformed { instance_id, .. } if *instance_id == timmy.id)));

        let copies: Vec<CardInstance> = g.pile(P1, "library").iter().filter(|card| card.def_id == TIMMY).cloned().collect();
        assert_eq!(copies.len(), 3);
        // "Copies of it": the card played was Radiant, so its copies are (R34, R57).
        assert_eq!(copies.iter().map(|card| card.radiant).collect::<Vec<_>>(), vec![true, true, true]);
    }
}

mod r65_x_is_chosen_for_one_play {
    use super::*;

    #[test]
    fn r65_efficiency_dividend_returned_at_end_of_turn_does_not_keep_the_x_it_was_played_for_2_3() {
        register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [DIVIDEND, VANILLA], "library": LIBRARY },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }));
        let dividend = g.card(DIVIDEND).clone();
        g.play(DIVIDEND, json!({ "x": 3, "modes": ["damage"], "targets": [{ "pick": "hero", "player": "p2" }] }));
        g.end_turn();
        g.expect_in_zone(&dividend.id, "hand");
        // R65: "Outside play (library, hand, GY …) an X-cost card's [printed cost] is 0".
        assert_eq!(effective_cost(g.state(), g.card(&dividend.id), CostOptions::default()), 0);
        let hand = g.view(P1).you.hand;
        let in_view = match &hand {
            HandView::Cards(cards) => cards.iter().find(|card| card.instance_id == dividend.id).cloned(),
            HandView::Count { .. } => None,
        };
        assert_eq!(in_view.map(|card| card.cost), Some(0));
    }
}

mod r210_the_tribute_a_play_pays {
    use super::*;

    #[test]
    fn r210_a_right_house_defender_tributed_to_the_rock_returns_through_reborn_since_a_sacrifice_is_a_death_6_1_6_3() {
        register_all();
        let mut g = scenario(json!({
            "p1": { "hand": [ROCK, VANILLA], "field": [{ "def": RIGHT_HOUSE, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }));
        let guard = g.card(RIGHT_HOUSE).clone();
        g.play(ROCK, json!({ "zone": 2, "tributes": [guard.id] }));

        // §6.3 Sacrifice "counts as a death"; §6.1 Reborn: "first death: return at 1 health".
        g.expect_in_zone(&guard.id, "field");
        assert_eq!(g.unit(P1, 1).map(|card| card.id.clone()), Some(guard.id.clone()));
        assert_eq!(g.card(&guard.id).reborn_spent, Some(true));
        assert_eq!(g.card(&guard.id).summoned_turn, Some(g.state().turn));
    }

    #[test]
    fn r210_the_rock_tributing_a_radiant_right_house_defender_still_lands_in_the_zone_it_named() {
        register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [ROCK, VANILLA],
                "field": [{ "def": RIGHT_HOUSE, "radiant": true, "lane": 2 }],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "library": LIBRARY },
        }));
        let guard = g.card(RIGHT_HOUSE).clone();
        let rock = g.card(ROCK).clone();
        // Step 1 accepts lane 1; step 2's tribute fires the radiant Death, which summons a base
        // Right-house defender per R64 — and lane 1 is held for The Rock, so the summon takes lane 3.
        g.play(ROCK, json!({ "zone": 1, "tributes": [guard.id] }));

        // §3.2 / §10.5 step 4: the played card is on the field where the player put it, never in no
        // zone at all, and the Death's base Right-house defender is on the field beside it.
        g.expect_in_zone(&rock.id, "field");
        assert_eq!(g.unit(P1, 1).map(|card| card.id.clone()), Some(rock.id.clone()));
        let units: Vec<CardInstance> = g
            .state()
            .players
            .p1
            .units
            .iter()
            .flat_map(|pile| pile.iter().flatten())
            .cloned()
            .collect();
        assert!(units.iter().any(|card| card.def_id == RIGHT_HOUSE && card.id != guard.id));
    }
}

mod r217_a_draw_a_cast_makes_continues_its_chain {
    use super::*;

    #[test]
    fn r217_a_cn_virus_cast_under_fullsend_s_combo_draw_is_one_chain_bounded_by_r58_s_cap_2_4_r70_9_3() {
        register_all();
        // /fullsend gives every card p1 plays this turn "Combo: draw 1", and a cast is a play (R70), so
        // each CN-Virus cast draws once more before its own script shuffles two copies back in. With the
        // Armor absorbing the virus's damage and Infinite Reserves standing in for fatigue nothing else
        // ends it: counted as chains of their own, the nested draws recursed until the call stack ran out.
        let mut g = scenario(json!({
            "p1": {
                "hand": [{ "def": FULLSEND, "radiant": true }, VANILLA],
                "backrow": [GOING_LONG, INFINITE_RESERVES],
                "library": [CN_VIRUS],
                "mana": 4,
            },
            "p2": { "hand": [VANILLA], "field": [VANILLA], "library": [VANILLA, VANILLA] },
        }));
        g.play(FULLSEND, json!({}));

        // §9.3: `reduce` returns a state (or refuses the action); it never throws on a legal play. (TS
        // `expect(() => g.play(VANILLA)).not.toThrow()`: a refusal or a panic fails this test.)
        g.play(VANILLA, json!({}));
        assert!(g.state().pending.is_none());
        // The Vanilla's Combo draw began the chain, and every CN-Virus cast in it counts toward one cap.
        let casts = g
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == CN_VIRUS))
            .count();
        assert!(casts > 0);
        assert!(casts as i32 <= CAST_ON_DRAW_CHAIN_CAP);
        assert_eq!(g.state().cast_chain, None);
    }
}

const CHAOS: &str = "core-095"; // #95 Call to Chaos (Core Edition), Spell, 4
// TS `LONG_PLAY_TIMEOUT_MS = 60_000`: the one play below sets off about 1,100 casts, each a play #33
// answers — seconds of work, and several times that under the coverage run's instrumentation, past
// vitest's default 5 s. A Rust `#[test]` has no per-test timeout, so the constant is not ported.
const MENACE: &str = "core-019"; // #19 Midrange Menace, a spare 3-cost Unit, so the turn never auto-ends

/// #95's roll is the first rng draw of its play (see `095-call-to-chaos.test.ts`), so the cursor
/// picks the effect. The seed and the search are that file's.
const CHAOS_SEED: &str = "chaos-card";
fn chaos_cursor_for(effect: &str) -> u32 {
    for cursor in 0..500 {
        let rolled = subsystems::roll_chaos_effects(&mut Rng::new(CHAOS_SEED, cursor), false, subsystems::CHAOS_EFFECTS)
            .first()
            .map(|rolled| rolled.name.to_string());
        if rolled.as_deref() == Some(effect) {
            return cursor;
        }
    }
    panic!("no cursor below 500 rolls \"{effect}\" from \"{CHAOS_SEED}\"");
}

mod r58_10_3_the_resolution_loop_runs_until_the_rules_say_it_is_done {
    // Found by the lens's probe, which tried every action `legalActions` offered in seeded random
    // games: on turn 24 of one, p2's library had filled with #90's CN-Viruses, p1 had a #33 in play,
    // and the #95 in p2's hand threw out of `reduce` when played.
    //
    // "Draw your whole library" is one draw per card the library held when the effect started (R58),
    // and each of those draws casts at most 20 CN-Viruses (R58, R217). Each virus used to shuffle its
    // two copies in at once, so the chain fed itself and the play ran to about 1,100 casts, every one
    // a play #33 answers (R70): past a flat 1,000-pass `settle`, which is why SETTLE_PASS_CAP is now
    // derived from the rules (`triggers.ts`). R350 (patch v0.1.1) holds a virus's copies to the end
    // of the turn, and #33's copies wait for its queued trigger, so the same play now casts only what
    // the library held and fatigues for the rest: it still resolves whole, and the fatigue kills.
    use super::*;

    #[test]
    fn r58_r350_a_call_to_chaos_that_draws_a_library_of_55_cn_viruses_beside_unstable_clone_machine_resolves_instead_of_throwing_10_3_r217_r70()
     {
        register_all();
        let mut s = scenario(json!({
            "seed": CHAOS_SEED,
            "p1": {
                "hand": [CHAOS, MENACE],
                "mana": 8,
                "backrow": [GOING_LONG, CLONE_MACHINE],
                "library": vec![CN_VIRUS; 55],
            },
        }));
        s.state_mut().rng_cursor = chaos_cursor_for("draw");

        // TS `expect(() => s.play(CHAOS)).not.toThrow()`: a refusal or a panic fails this test.
        s.play(CHAOS, json!({}));
        assert!(s.state().pending.is_none());
        // R58: 55 draws; the chains cast only viruses the library held, so at most 55, and never a copy.
        let casts = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == CN_VIRUS))
            .count();
        assert!(casts > 0);
        assert!(casts <= 55);
        assert!(!s.events().iter().any(|event| matches!(event, GameEvent::ShuffledIn { .. })));
        // The draws past the empty library are §2.4's fatigue, which Going Long's Armor 2 cannot hold.
        assert!(s.events().iter().filter(|event| matches!(event, GameEvent::Fatigue { .. })).count() > 0);
        assert_eq!(
            s.state().result,
            Some(GameResult { winner: Winner::P2, reason: GameOverReason::HeroDeath })
        );
    }
}
