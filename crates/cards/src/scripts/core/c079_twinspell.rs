//! #79 Twinspell (SPEC §8.3 row 79; §2.2, §6.2 Echo, §10.5 step 6, R384, R3044000). (2) Field Spell, Rare.
//!   Base:    "Activate: The next Spell you play this turn gains Echo +{echoGain}. Then this goes to the
//!            graveyard." — 1
//!   Radiant: "Activate: The next Spell you play this turn gains Echo +{echoGain}." — 1; it stays on the
//!            field, so it serves once each turn.
//!   Through v0.5.11p: a lasting grant with no Activate, "Your next Spell gains Echo +{echoGain}. Then this
//!   goes to the graveyard." (1, Radiant 2): the engine's `staticFlags.echoGrant` rider, which waited past
//!   cleanup and buried its Twinspell when a Spell took it (R30, R209; `script_v0_5_11o`, kept, R767000).
//!   Engine:  "An Activate, once a turn (R384). Its effect puts a this-turn `echoNextSpell` rider with no
//!            source on its controller, which grants its own amount to the next Spell they play, a cast
//!            included (R70), and which this turn's cleanup ends unused (§2.2). The base face then goes to
//!            the graveyard by a plain move, no death (R3044000). Tunes: Echo gained 1 ↑."

use jackioh_engine::prelude::*;

pub const ID: &str = "core-079";

/// R3044000: "The next Spell you play this turn gains Echo +{echoGain}": a rider with no `sourceId`,
/// which grants its own amount (`echo::echo_grant_of`), is spent by the next Spell its player plays
/// (§10.5 step 4, `echo::granted_echo`) and ends at this turn's cleanup if none does (§2.2).
fn next_spell_echoes(ctx: &EffectContext<'_>) -> Effect {
    add_player_modifier(json_as(json!({
        "player": "self",
        "mod": {
            "kind": "echoNextSpell",
            "amount": param(ctx, "echoGain"),
            "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
        },
    })))
}

/// The one Activate both faces print (R384); the base face then sends itself to the graveyard, a plain
/// move and not a death (R3044000), while the Radiant one stays to be used again next turn.
fn twinspell(stays: bool) -> Script {
    let echo = ActivationDecl {
        id: "echo".to_string(),
        label: "The next Spell you play this turn gains Echo".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            let mut effects = vec![next_spell_echoes(ctx)];
            if !stays {
                effects.push(send_to_graveyard(json_as(json!({ "target": { "of": "self" } }))));
            }
            effects
        }),
    };
    Script {
        activations: vec![echo],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: twinspell(false),
        radiant: twinspell(true),
    }
}

/// R767000, R30, R209: through v0.5.11p Twinspell was a lasting grant. The engine reads the amount off
/// the face the card wears when a Spell takes it, as its declared `echoGain` (R386), and buries the
/// Twinspell that granted it; `KEPT_SCRIPTS` folds those replays under it.
fn twinspell_v0_5_11o(amount: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            echo_grant: Some(amount),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

/// R767000: the script v0.5.11p and the versions before it ran, kept for their replays.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: twinspell_v0_5_11o(1),
        radiant: twinspell_v0_5_11o(2),
    }
}

// #79 Twinspell — SPEC §8.3 row 79, BUILD M4-T4 must-pass row 79 (R384, R3044000). The echo count is
// read through The Coin (§7), whose "Gain 1 mana this turn." makes each resolution a number: one extra
// resolution is +2 instead of +1, two extra is +3. The "fresh prompts per repeat" half of §10.5 step 6 is
// the engine's Echo rule (`crates/engine/tests/rules/echo.rs`).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{can_degrade_number, can_upgrade_number, js, matches_object, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;

    const TWINSPELL: &str = "core-079";
    const COIN: &str = "core-t-coin";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.

    const LIBRARY: [&str; 4] = ["core-008", "core-008", "core-008", "core-008"];

    /// Harness gap (reported): no `mods()` accessor, so the test reads `state` — B1.7 covers `src` only.
    fn echo_riders(s: &Scenario) -> Vec<Value> {
        s.state()
            .players
            .p1
            .mods
            .iter()
            .map(js)
            .filter(|modifier| modifier["kind"] == "echoNextSpell")
            .collect()
    }

    /// The first `echoNextSpell` rider.
    fn first_echo_rider(s: &Scenario) -> Value {
        echo_riders(s).into_iter().next().expect("an echoNextSpell modifier")
    }

    /// Mana 4, so Twinspell (2), The Coin (0) and the Vanilla (1) all fit on one turn.
    fn with_coin(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": TWINSPELL, "radiant": radiant }, COIN, VANILLA],
                "mana": 4,
                "library": LIBRARY
            },
            "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
        }))
    }

    /// Twinspell played and activated, 2 mana left.
    fn activated(radiant: bool) -> Scenario {
        let mut s = with_coin(radiant);
        s.play(TWINSPELL, json!({}));
        s.activate(TWINSPELL, json!({}));
        s
    }

    #[test]
    fn r3044000_r767000_the_kept_script_is_the_lasting_grant_echo_1_and_radiant_echo_2() {
        let kept = script_v0_5_11o();
        let grant = |face: &Script| face.static_flags.as_ref().and_then(|flags| flags.echo_grant);
        assert_eq!(grant(&kept.base), Some(1));
        assert_eq!(grant(&kept.radiant), Some(2));
        assert!(kept.base.activations.is_empty() && kept.radiant.activations.is_empty());
        let now = script();
        assert_eq!(grant(&now.base), None);
        assert_eq!(grant(&now.radiant), None);
    }

    #[test]
    fn r386_an_upgrade_grants_echo_2_and_neither_face_can_be_nerfed_below_echo_1() {
        for radiant in [false, true] {
            let mut s = with_coin(radiant);
            assert!(!can_degrade_number(&s, TWINSPELL, "echoGain"), "radiant {radiant}");
            assert!(can_upgrade_number(&s, TWINSPELL, "echoGain"), "radiant {radiant}");
            assert_eq!(upgrade_number(&mut s, TWINSPELL, "echoGain"), 2);
            s.play(TWINSPELL, json!({}));
            s.activate(TWINSPELL, json!({}));
            assert_eq!(first_echo_rider(&s)["amount"], 2);

            // The Coin's "gain 1 mana" once per resolution: 2 left after the Twinspell, plus three.
            s.play(COIN, json!({}));

            s.expect_mana(P1, 2 + 1 + 2);
        }
    }

    mod n79_twinspell_base {
        use super::*;

        #[test]
        fn r384_playing_it_grants_nothing_until_it_is_activated() {
            let mut s = with_coin(false);
            s.play(TWINSPELL, json!({}));

            assert_eq!(echo_riders(&s).len(), 0);
            s.play(COIN, json!({}));

            // One resolution: 2 + 1.
            s.expect_mana(P1, 3);
            s.expect_in_zone(TWINSPELL, "field");
        }

        #[test]
        fn r3044000_its_activate_puts_a_this_turn_echo_1_rider_with_no_source_on_its_controller() {
            let s = activated(false);

            assert_eq!(echo_riders(&s).len(), 1);
            let rider = first_echo_rider(&s);
            assert!(
                matches_object(
                    &rider,
                    &json!({
                        "kind": "echoNextSpell",
                        "amount": 1,
                        // §2.2: this turn's cleanup ends it if no Spell takes it.
                        "expiry": { "until": "thisTurn", "turn": s.state().turn },
                    })
                ),
                "{rider}"
            );
            // No source: it grants its own amount, wherever Twinspell has gone.
            assert_eq!(rider.get("sourceId"), None, "{rider}");
        }

        #[test]
        fn r3044000_then_it_goes_to_the_graveyard_by_a_plain_move_and_not_a_death() {
            let mut s = with_coin(false);
            let twin = s.card(TWINSPELL).clone();
            s.play(TWINSPELL, json!({}));

            s.activate(TWINSPELL, json!({}));

            s.expect_in_zone(&twin, "graveyard");
            assert!(
                s.last_events()
                    .iter()
                    .all(|event| event.event_type() != GameEventType::Destroyed)
            );
            assert_eq!(s.state().counters.destroyed, 0);
        }

        #[test]
        fn r3044000_indestructible_does_not_keep_it_on_the_field() {
            let mut s = with_coin(false);
            let twin = s.card(TWINSPELL).clone();
            s.play(TWINSPELL, json!({}));
            s.card_mut(TWINSPELL)
                .granted_keywords
                .push(Keyword::Indestructible);

            s.activate(TWINSPELL, json!({}));

            s.expect_in_zone(&twin, "graveyard");
            assert_eq!(echo_riders(&s).len(), 1);
        }

        #[test]
        fn s10_5_step_6_the_next_spell_this_turn_resolves_one_extra_time() {
            let mut s = activated(false); // 4 − 2 = 2

            s.play(COIN, json!({})); // 0, then 1 mana per resolution

            // Two resolutions of "gain 1 mana": 2 + 2 = 4 rather than 2 + 1 = 3.
            s.expect_mana(P1, 4);
            assert_eq!(echo_riders(&s).len(), 0);
        }

        #[test]
        fn r3044000_a_unit_played_first_leaves_the_grant_for_the_next_spell() {
            let mut s = activated(false); // 2
            s.play(VANILLA, json!({})); // 1
            assert_eq!(echo_riders(&s).len(), 1);

            s.play(COIN, json!({})); // +1 twice

            s.expect_mana(P1, 3);
            assert_eq!(echo_riders(&s).len(), 0);
        }

        #[test]
        fn r3044000_only_the_next_spell_echoes_a_second_spell_resolves_once() {
            let mut s = scenario(json!({
                "p1": { "hand": [TWINSPELL, COIN, COIN], "mana": 4, "library": LIBRARY },
                "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
            }));
            s.play(TWINSPELL, json!({})); // 4 − 2 = 2
            s.activate(TWINSPELL, json!({}));
            let coins: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == COIN).collect();
            let (Some(first), Some(second)) = (coins.first().cloned(), coins.get(1).cloned()) else {
                panic!("setup: two Coins in hand");
            };
            s.play(&first, json!({})); // +1 per resolution, twice: 4
            s.expect_mana(P1, 4);

            s.play(&second, json!({})); // +1 once: 5

            // The rider was spent on the first spell, so this one carries no Echo (§10.5 step 6).
            s.expect_mana(P1, 5);
            assert_eq!(echo_riders(&s).len(), 0);
        }

        #[test]
        fn s2_2_an_unused_grant_ends_at_this_turns_cleanup() {
            let mut s = activated(false);

            s.end_turn();

            assert_eq!(echo_riders(&s).len(), 0);
            // Back on p1's turn, The Coin resolves once: 1 mana more.
            s.end_turn();
            let before = s.state().players.p1.mana.current;
            s.play(COIN, json!({}));
            s.expect_mana(P1, before + 1);
        }
    }

    mod n79_twinspell_radiant {
        use super::*;

        #[test]
        fn r3044000_its_activate_grants_echo_1_and_it_stays_on_the_field() {
            let mut s = with_coin(true);
            let twin = s.card(TWINSPELL).clone();
            s.play(TWINSPELL, json!({}));

            s.activate(TWINSPELL, json!({}));

            s.expect_in_zone(&twin, "field");
            let rider = first_echo_rider(&s);
            assert!(
                matches_object(
                    &rider,
                    &json!({
                        "kind": "echoNextSpell",
                        "amount": 1,
                        "expiry": { "until": "thisTurn", "turn": s.state().turn },
                    })
                ),
                "{rider}"
            );
            assert_eq!(rider.get("sourceId"), None, "{rider}");
        }

        #[test]
        fn s10_5_step_6_radiant_makes_the_next_spell_resolve_one_extra_time() {
            let mut s = activated(true); // 2

            s.play(COIN, json!({})); // two resolutions of +1

            s.expect_mana(P1, 4);
            s.expect_in_zone(TWINSPELL, "field");
        }

        #[test]
        fn r384_once_a_turn_a_second_activation_is_refused() {
            let mut s = activated(true);

            s.expect_refused(|s| s.activate(TWINSPELL, json!({})));

            assert_eq!(echo_riders(&s).len(), 1);
        }

        #[test]
        fn r384_it_serves_again_on_its_controllers_next_turn() {
            let mut s = activated(true);
            s.end_turn();
            assert_eq!(echo_riders(&s).len(), 0);
            s.end_turn();

            s.activate(TWINSPELL, json!({}));
            let before = s.state().players.p1.mana.current;
            s.play(COIN, json!({}));

            // Two resolutions of "gain 1 mana".
            s.expect_mana(P1, before + 2);
            s.expect_in_zone(TWINSPELL, "field");
        }
    }
}
