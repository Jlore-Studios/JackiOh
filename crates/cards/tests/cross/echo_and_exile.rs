//! Echo's repeats, the state check between them, and a Spell that exiles itself (SPEC §4.5, §6.2
//! Echo, §10.5 steps 4 to 7, R30, R59, R178). Found by the polish-4 edge-case hunt
//! (docs/polish/4-edge-cases.md, lens L7); every case here failed before its fix.
//!
//!  - §4.5: the state check runs after a Spell's first resolution, before its Echo repeat asks
//!    anything, so a unit the first resolution killed is not offered again and a hero it killed ends
//!    the game there.
//!  - R178: "exile this" is where §10.5 step 7 sends a Spell, so a self-exiling Spell still takes
//!    Twinspell's Echo and still resolves its repeat; and the Echo is gained as the Spell is played,
//!    so a Spell that moves Twinspell away (#87's board swap) has already taken it.
//!  - Round 9, lens "card by card". R119: an Echo repeat is the same play resolving again, so its
//!    granted Combo parts count neither a Quickstriker the first resolution summoned (#95) nor a
//!    Combo modifier the play itself installed (#78's own "Combo: draw 1").
//!
//! Port of `packages/cards/test/echo-and-exile.test.ts` (SURFACE §4.1, §8).

use jackioh_engine::testkit::*;
use jackioh_engine::PlayerId::{P1, P2};

const TWINSPELL: &str = "core-079";
const HIT_JOB: &str = "core-016";
const LUNAR_ECLIPSE: &str = "core-035";
const TRUE_STRIKE: &str = "core-044";
const MROW: &str = "core-086";
const CHAOS: &str = "core-087";
const POINTMASTER: &str = "core-020";
const VANILLA: &str = "core-008";
const SEVEN_SEVEN: &str = "core-025";
const RENO: &str = "core-053";
const LIBRARY: [&str; 6] = [RENO; 6];

use super::scenario;

/// TS `at(card)`: the one-instance target list a play sends.
fn at(card: &CardInstance) -> Value {
    json!([{ "pick": "instance", "instanceId": card.id }])
}

fn unit_at(g: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match g.unit(player, lane) {
        Some(card) => card.clone(),
        None => panic!("setup: {player} should hold a unit in lane {lane}"),
    }
}

fn option_ids(g: &Scenario) -> Vec<String> {
    g.state()
        .pending
        .as_ref()
        .map(|pending| pending.options.as_slice())
        .unwrap_or(&[])
        .iter()
        .filter_map(|option| match &option.selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

/// How many of `player`'s modifiers are an `echoNextSpell`.
fn echo_mods(g: &Scenario, player: PlayerId) -> usize {
    g.state().players[player]
        .mods
        .iter()
        .filter(|modifier| matches!(modifier.kind, ModifierKind::EchoNextSpell { .. }))
        .count()
}

/// Answers the open prompt with its first option until none is open (TS's `while (pending !== null)`
/// loop, which stops on a prompt with no options).
fn answer_first_selections(g: &mut Scenario) {
    while let Some(pending) = g.state().pending.as_ref() {
        let Some(first) = pending.options.first() else {
            break;
        };
        let selection = first.selection.clone();
        g.answer(json!([selection]));
    }
}

mod section_4_5_the_state_check_between_an_echos_resolutions {
    use super::*;

    #[test]
    fn r59_4_5_an_echo_repeats_prompt_is_asked_after_the_first_resolutions_deaths_so_a_destroyed_unit_is_not_offered_again() {
        // Twinspell gives Hit Job Echo +1. The first resolution destroys the enemy Mrow; by the time the
        // repeat asks its fresh target prompt Mrow has died, and its Death has run.
        let mut g = scenario(json!({
            "p1": { "hand": [TWINSPELL, HIT_JOB, RENO], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "mana": 8, "library": LIBRARY },
            "p2": { "hand": [RENO], "field": [{ "def": MROW, "lane": 1 }, { "def": POINTMASTER, "lane": 4 }], "library": LIBRARY },
        }));
        let mrow = unit_at(&g, P2, 1);

        g.play(TWINSPELL, json!({}));
        g.play(HIT_JOB, json!({ "targets": at(&mrow) }));

        assert_eq!(g.state().pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Target));
        assert!(!option_ids(&g).contains(&mrow.id));
        g.expect_in_zone(&mrow.id, "graveyard");
    }

    #[test]
    fn section_4_5_a_hero_at_0_after_an_echos_first_resolution_ends_the_game_before_the_repeat_asks_anything_2_5() {
        let mut g = scenario(json!({
            "p1": { "hand": [TWINSPELL, LUNAR_ECLIPSE, RENO], "mana": 8, "library": LIBRARY },
            "p2": { "hand": [RENO], "health": 3, "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));

        g.play(TWINSPELL, json!({}));
        g.play(LUNAR_ECLIPSE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

        assert_eq!(
            g.state().result,
            Some(GameResult { winner: Winner::P1, reason: GameOverReason::HeroDeath })
        );
        assert!(g.state().pending.is_none());
    }
}

mod r178_a_spells_exile_this_is_its_landing_and_its_echo_is_gained_as_it_is_played {
    use super::*;

    #[test]
    fn r178_r30_a_self_exiling_spell_takes_twinspells_echo_resolves_twice_and_lands_in_exile_6_2() {
        // True Strike: "Deal 4 damage to a target, ignoring Armor; exile this". With Twinspell it
        // resolves twice — the repeat asks for its target again — and only then goes to exile.
        let mut g = scenario(json!({
            "p1": { "hand": [TWINSPELL, TRUE_STRIKE, RENO], "mana": 8, "library": LIBRARY },
            "p2": { "hand": [RENO], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }], "library": LIBRARY },
        }));
        let twin = g.card(TWINSPELL).clone();
        let strike = g.card(TRUE_STRIKE).clone();

        g.play(TWINSPELL, json!({}));
        g.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
        // Still resolving, and still itself, while its repeat asks (R98).
        assert_eq!(g.card(&strike.id).zone.z(), ZoneName::Resolving);
        g.expect_in_zone(&twin.id, "graveyard");
        g.answer(json!([{ "pick": "hero", "player": "p2" }]));

        g.expect_health(P2, 22);
        g.expect_in_zone(&strike.id, "exile");
        assert_eq!(g.state().counters.exiled, 1);
        assert_eq!(echo_mods(&g, P1), 0);
    }

    #[test]
    fn r178_r30_twinspell_is_consumed_by_the_next_spell_played_even_when_that_spell_swaps_it_away_and_exiles_itself() {
        // Pocket Chaos swaps the boards — Twinspell with them — and exiles itself. The Echo was already
        // gained as it was played, so Twinspell went to the graveyard first and the repeat still comes.
        let mut g = scenario(json!({
            "p1": { "hand": [TWINSPELL, CHAOS, RENO], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }], "mana": 8, "library": LIBRARY },
            "p2": { "hand": [RENO], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));
        let twin = g.card(TWINSPELL).clone();
        let chaos = g.card(CHAOS).clone();

        g.play(TWINSPELL, json!({}));
        g.play(CHAOS, json!({ "modes": ["board"] }));
        // The repeat's fresh mode prompt (§10.6: an Echo repeat asks again).
        assert_eq!(g.state().pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Mode));
        answer_first_selections(&mut g);

        g.expect_in_zone(&twin.id, "graveyard");
        g.expect_in_zone(&chaos.id, "exile");
        assert_eq!(echo_mods(&g, P1), 0);
        assert_eq!(echo_mods(&g, P2), 0);
    }
}

// ---------------------------------------------------------------------------
// Round 9: an Echo repeat is the same play (R119, §10.5 step 6)
// ---------------------------------------------------------------------------

const STOCKPILE: &str = "core-005";
const MR_VANILLA: &str = "core-008";
const TEMPO_TIMMY: &str = "core-011";
const MIDRANGE_MENACE: &str = "core-019";
const QUICKSTRIKER: &str = "core-038";
const FULLSEND: &str = "core-078";
const CALL_TO_CHAOS: &str = "core-095";

const R119_LIBRARY: [&str; 6] = [MR_VANILLA; 6];
const R119_SEED: &str = "r9-c2c-0";

/// A cursor at which #95's one roll is "summon 5 random Field Spells or Traps" and a Quickstriker is
/// among the five picks: the roll is one draw over the ten, each pick one draw over the backrow pool
/// (`summonRandom`, R60), in that order.
fn quickstriker_cursor() -> u32 {
    let pool = catalog::query(&json_as(json!({ "type": ["Field Spell", "Trap", "Field Trap"] })));
    let backrow = subsystems::CHAOS_EFFECTS.iter().position(|effect| effect.name == "backrow");
    for cursor in 0..100_000u32 {
        let mut rng = Rng::new(R119_SEED, cursor);
        if Some(rng.int(subsystems::CHAOS_EFFECTS.len() as i32) as usize) != backrow {
            continue;
        }
        let picks: Vec<Option<String>> = (0..5)
            .map(|_| pool.get(rng.int(pool.len() as i32) as usize).map(|def| def.id.clone()))
            .collect();
        if picks.contains(&Some(QUICKSTRIKER.to_string())) {
            return cursor;
        }
    }
    panic!("no cursor rolls a Quickstriker into #95's backrow");
}

/// TS `ofType(events, "drawn")`.
fn drawn_count(events: &[GameEvent]) -> usize {
    events.iter().filter(|event| matches!(event, GameEvent::Drawn { .. })).count()
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

mod r119_10_5_step_6_an_echo_repeats_granted_combo_parts_do_not_answer_what_its_own_play_put_in_place {
    use super::*;

    #[test]
    fn r119_fullsend_echoed_by_twinspell_draws_nothing_from_the_combo_rider_its_own_first_resolution_installed_10_5_step_6() {
        let mut s = scenario(json!({
            // A unit that can still switch keeps §2.5's auto-end from passing the turn once the hand is empty.
            // The Radiant /fullsend: the face that grants "Combo: Draw 1" since patch v0.1.1.
            "p1": { "hand": [TWINSPELL, { "def": FULLSEND, "radiant": true }], "field": [MR_VANILLA], "mana": 8, "library": R119_LIBRARY },
            "p2": { "hand": [STOCKPILE], "field": [MIDRANGE_MENACE], "library": R119_LIBRARY },
        }));
        let combo_draws = |s: &Scenario| {
            s.state()
                .players
                .p1
                .mods
                .iter()
                .filter(|modifier| matches!(modifier.kind, ModifierKind::ComboDraw { .. }))
                .count()
        };
        // Twinspell is a card played earlier this turn, so every later card meets "Combo" (§6.2).
        s.play(TWINSPELL, json!({}));
        assert_eq!(combo_draws(&s), 0);

        s.play(FULLSEND, json!({}));

        // Twinspell's grant was taken, so /fullsend resolved twice (§10.5 step 6): two "Refresh 3 mana"
        // (8 − 2 − 4 = 2, refreshed to max 4, and the second has nothing to give back, R364) and two
        // "Combo: Draw 1" riders.
        s.expect_mana(P1, 4);
        assert_eq!(combo_draws(&s), 2);
        // R119: the rider /fullsend installs does not answer /fullsend's own play. Its first resolution
        // is counted before the rider exists (step 5's granted parts precede its script), and the repeat
        // is the same play re-resolving (§6.2 Echo: "the same instance re-resolves"), so it draws
        // nothing either.
        assert_eq!(drawn_count(s.last_events()), 0);
        assert_eq!(s.hand(P1).len(), 0);
    }

    #[test]
    fn r119_a_quickstriker_call_to_chaos_summons_in_its_first_resolution_deals_nothing_on_the_echo_repeat_of_that_same_play_10_5_step_6() {
        // The roll is pinned (R423 left the base face's one roll as it was): the play's first rng draw is
        // #95's roll, and the five picks of "summon 5 random Field Spells or Traps" follow it, from the
        // pool of every set (R380). `quickstrikerCursor` finds a cursor at which the roll is that effect
        // and a Quickstriker is among the five, so the fixture holds whatever else the pools hold.
        let mut s = scenario(json!({
            "seed": R119_SEED,
            "p1": { "hand": [TEMPO_TIMMY, TWINSPELL, CALL_TO_CHAOS], "mana": 10, "library": R119_LIBRARY },
            "p2": { "hand": [STOCKPILE], "field": [MIDRANGE_MENACE], "library": R119_LIBRARY },
        }));
        s.play(TEMPO_TIMMY, json!({ "zone": 1 }));
        s.play(TWINSPELL, json!({}));
        assert!(quickstrikers_of(&s, P1).is_empty());
        let call = s.card(CALL_TO_CHAOS).clone();

        s.state_mut().rng_cursor = quickstriker_cursor();
        s.play(CALL_TO_CHAOS, json!({}));
        while let Some(pending) = s.state().pending.as_ref() {
            let Some(first) = pending.options.first() else {
                break;
            };
            let key = first.key.clone();
            s.answer(json!(key));
        }

        // The setup did what it says: the Echo was taken (Twinspell reached the graveyard, two
        // resolutions), and a Quickstriker arrived in p1's backrow during this play.
        let graveyard: Vec<String> = s.pile(P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
        assert!(graveyard.contains(&TWINSPELL.to_string()));
        let rolls = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::ChaosRolled { instance_id, .. } if *instance_id == call.id))
            .count();
        assert_eq!(rolls, 2);
        assert!(quickstrikers_of(&s, P1).len() >= 1);
        // R119: a permanent #95 summons while the play resolves "starts counting from the next play";
        // the Echo repeat is not a next play, so the Quickstriker grants it no Combo damage.
        let hits = s
            .events()
            .iter()
            .filter(|event| {
                matches!(event, GameEvent::Damage { source_id, target_id, .. }
                    if source_id.as_deref() == Some(call.id.as_str()) && target_id == "hero-p2")
            })
            .count();
        assert_eq!(hits, 0);
    }
}
