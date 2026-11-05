//! M #5 Death by 1000 cuts (SPEC §8.8 row 5, BUILD M10 row M 5). (1) Spell, Rare.
//!   Base:    "Echo X, Pierce\nDeal {damage} damage to a Unit. X is {echo|time|times} your mana left after
//!            paying for this." (1; 2)
//!   Radiant: "Echo X, Lifesteal, Pierce\n…" (1; 4)
//!   Through v0.5.11p: "X is {echo} times your max mana", and every repeat asked a fresh target
//!   (`script_v0_5_11o`, kept, R767000).
//!
//! A computed Echo X (R802, R3048001): `Script.echo_x` reads `param(echo) × unspent_mana_of(controller)`
//! as the card is played (§10.5 step 4), after its own (1) is paid, so X stays fixed while the card
//! resolves — mana gained mid-resolution changes nothing — and cast with exactly 1 mana it cuts once
//! (X = 0). A cast pays nothing, so a random cast reads its caster's mana as it is. The larger of the
//! hook and `staticFlags.echo` holds (here there is no fixed one); the card's "Echo" tuning step and a
//! Twinspell grant then add on (`echo::printed_echo`, R30, R386). The target is any Unit (R81), picked
//! once: `echo_keeps_targets` makes every repeat hit the first resolution's pick with no new prompt
//! (R3048002), and once it has died or left the field the repeats left fizzle (R76, R90). Lifesteal, as
//! on Core #93.1 Combo-Fodder, heals once per hit (§4.4 step 8). Pierce is printed on the Spell, so
//! every hit skips Armor (§4.4 step 2, R346).

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-005";

/// One script serves both faces: the Radiant face's Lifesteal is its catalog keyword, and its larger
/// `echo` is its declared number, both read through `param`. `v0_5_11o` is the kept script's X and
/// target prompts (R767000).
fn cuts(v0_5_11o: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        echo_x: Some(if v0_5_11o {
            // Through v0.5.11p: `echo` times the caster's max mana (R802).
            read_hook(|args| param(&args, "echo") * max_mana_of(args.state, args.self_.controller))
        } else {
            // R3048001: `echo` times the mana left once the card is paid for.
            read_hook(|args| param(&args, "echo") * unspent_mana_of(args.state, args.self_.controller))
        }),
        // R3048002: every Echo repeat hits the first resolution's pick, where through v0.5.11p each
        // repeat asked a fresh target prompt (§6.2 Echo, R81).
        echo_keeps_targets: !v0_5_11o,
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "chosen" },
                "amount": param(&*ctx, "damage"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let both = cuts(false);
    CardScripts {
        base: both.clone(),
        radiant: both,
    }
}

/// R767000: the scripts v0.5.11p and the versions before it ran, kept for their replays.
pub fn script_v0_5_11o() -> CardScripts {
    let both = cuts(true);
    CardScripts {
        base: both.clone(),
        radiant: both,
    }
}

// M #5 Death by 1000 cuts — SPEC §8.8 row 5, BUILD M10 row M 5. X is the mana left once the card is
// paid for (R3048001) and every repeat hits the first resolution's pick (R3048002); R802 fixes X as the
// card is played, which still holds.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CUTS: &str = "meditative-005";
    const ECHO: &str = "classic-057"; // (1) Spell: has the text of the last Spell either player played.
    const MENACE: &str = "core-019"; // 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FODDER: &str = "core-015"; // 1/1 Unit.
    const FILLER: &str = "core-005"; // (1) Spell.
    const TWINSPELL: &str = "core-079"; // (2) Field Spell, Activate: the next Spell gains Echo +1.

    /// The ids of p2's units, lane by lane.
    fn p2_units(s: &Scenario) -> Vec<String> {
        (1..=5).filter_map(|lane| s.unit(P2, lane)).map(|card| card.id.clone()).collect()
    }

    /// Answer every open prompt by cycling through `ids`; returns the answers given.
    fn answer_all(s: &mut Scenario, ids: &[String]) -> usize {
        let mut given = 0;
        for _ in 0..40 {
            if s.state().pending.is_none() {
                break;
            }
            let id = &ids[given % ids.len()];
            s.answer(pick::id(id));
            given += 1;
        }
        assert!(s.state().pending.is_none(), "no prompt stays open");
        given
    }

    fn damage_events(s: &Scenario) -> Vec<GameEvent> {
        s.events().iter().filter(|event| matches!(event, GameEvent::Damage { .. })).cloned().collect()
    }

    fn damage_amounts(s: &Scenario) -> Vec<i32> {
        damage_events(s)
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    }

    /// The target of every hit, in order.
    fn damage_targets(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, .. } => Some(target_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// p1 holding `mana` and a Death by 1000 cuts (on `face`) over two 9/9 Units of p2's, whose first it
    /// is played at; returns the scenario with the play made, and that first Unit.
    fn cut(seed: &str, radiant: bool, mana: i32) -> (Scenario, String) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "health": 10, "hand": [{ "def": CUTS, "radiant": radiant }, FILLER], "mana": mana, "field": [MENACE] },
            "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
        }));
        let ids = p2_units(&s);
        let first = ids[0].clone();
        s.play(CUTS, json!({ "targets": pick::id(&first) }));
        (s, first)
    }

    mod meditative_005 {
        use super::*;

        #[test]
        fn r3048001_cast_with_exactly_1_mana_it_cuts_once() {
            let (s, first) = cut("meditative-005-one-mana", false, 1);

            // Paying (1) leaves 0: X = 2 × 0, so the play alone and no repeat, asking nothing.
            assert!(s.state().pending.is_none());
            assert_eq!(damage_amounts(&s), vec![1]);
            assert_eq!(damage_targets(s.events()), vec![first]);
        }

        #[test]
        fn r3048001_x_is_echo_times_the_mana_left_after_paying_for_it_base_with_3_mana_4_repeats() {
            let (mut s, first) = cut("meditative-005-three-mana", false, 3);

            // Paying (1) leaves 2: X = 2 × 2 = 4, so the play and 4 repeats, five hits on the first pick
            // with no prompt for any repeat (R3048002).
            assert!(s.state().pending.is_none());
            assert_eq!(damage_amounts(&s), vec![1; 5]);
            assert_eq!(damage_targets(s.events()), vec![first; 5]);
            s.expect_mana(P1, 2);
        }

        #[test]
        fn r3048001_r802_its_max_mana_is_not_what_it_reads() {
            let mut s = scenario(json!({
                "seed": "meditative-005-max-mana",
                "p1": { "hand": [CUTS, FILLER], "mana": 2, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            assert_eq!(s.state().players.p1.mana.max, 4);
            let first = pick::id(&p2_units(&s)[0]);
            s.play(CUTS, json!({ "targets": first }));

            // 1 left of 4 max: X = 2 × 1, three hits (a max-mana X would be 2 × 4).
            assert!(s.state().pending.is_none());
            assert_eq!(damage_amounts(&s), vec![1; 3]);
        }

        #[test]
        fn r3048001_r802_a_random_cast_reads_the_mana_its_caster_holds_and_hits_one_target() {
            let mut s = scenario(json!({
                "seed": "meditative-005-random-cast",
                "p1": { "hand": [FILLER], "mana": 2, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let state = s.state_mut();
                let mut rng = Rng::new(&state.seed, state.rng_cursor);
                {
                    let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
                    {
                        let mut ctx = jackioh_engine::resolve::make_context(
                            &mut sink,
                            None,
                            jackioh_engine::resolve::HookOptions { controller: Some(P1), ..Default::default() },
                        );
                        jackioh_engine::resolve::apply_effects(
                            &[jackioh_engine::effects::cast_new(json_as(json!({ "def": CUTS, "random": true })))],
                            &mut ctx,
                        );
                    }
                    jackioh_engine::triggers::settle(&mut sink, jackioh_engine::triggers::SettleOptions::default());
                }
                state.rng_cursor = rng.cursor();
            }
            // A cast pays nothing, so p1's 2 mana is what is left: X = 2 × 2 = 4, five hits. The pick was
            // made once, at random, and every repeat hit it; no prompt ever opens.
            let targets = damage_targets(&events);
            assert_eq!(targets.len(), 5);
            assert!(targets.iter().all(|target| *target == targets[0]));
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r3048002_r30_a_twinspell_adds_1() {
            let mut s = scenario(json!({
                "seed": "meditative-005-twinspell",
                "p1": { "hand": [TWINSPELL, CUTS, FILLER], "mana": 9, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            s.play(TWINSPELL, json!({}));
            s.activate(TWINSPELL, json!({}));
            s.state_mut().players.p1.mana.current = 3;
            let first = p2_units(&s)[0].clone();
            s.play(CUTS, json!({ "targets": pick::id(&first) }));

            // 2 left: X = 2 × 2 = 4, plus the grant's 1: the play and 5 repeats, all on the first pick.
            assert!(s.state().pending.is_none());
            assert_eq!(damage_amounts(&s), vec![1; 6]);
            assert_eq!(damage_targets(s.events()), vec![first; 6]);
        }

        #[test]
        fn r3048002_r76_the_repeats_fizzle_once_the_target_has_died_and_hit_nothing_else() {
            let mut s = scenario(json!({
                "seed": "meditative-005-fizzle",
                "p1": { "hand": [CUTS, FILLER], "mana": 9 },
                "p2": { "hand": [FILLER], "field": [FODDER, MENACE] },
            }));
            let fodder = s.card(FODDER).id.clone();
            let menace = s.card(MENACE).id.clone();
            s.play(CUTS, json!({ "targets": pick::id(&fodder) }));

            // The first hit kills the 1/1; the 16 repeats find it gone, ask nothing, and the 9/9 beside
            // it, a legal Unit, takes none of them.
            assert!(s.state().pending.is_none());
            assert_eq!(damage_amounts(&s), vec![1]);
            s.expect_in_zone(FODDER, "graveyard");
            assert_eq!(s.stats(&menace).health, 9);
        }

        #[test]
        fn r3048002_a_target_that_dies_midway_takes_every_hit_up_to_its_death() {
            let mut s = scenario(json!({
                "seed": "meditative-005-midway",
                "p1": { "hand": [CUTS, FILLER], "mana": 3, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [VANILLA, MENACE] },
            }));
            step_param(s.card_mut(CUTS), "damage", 1);
            let vanilla = s.card(VANILLA).id.clone();
            let menace = s.card(MENACE).id.clone();
            s.play(CUTS, json!({ "targets": pick::id(&vanilla) }));

            // X = 2 × 2 = 4 repeats of 2 damage: the 4/4 dies on the first repeat, the three after it
            // fizzle without a prompt, and the 9/9 is never reached.
            assert!(s.state().pending.is_none());
            assert_eq!(damage_amounts(&s), vec![2, 2]);
            assert_eq!(damage_targets(s.events()), vec![vanilla.clone(); 2]);
            s.expect_in_zone(VANILLA, "graveyard");
            assert_eq!(s.stats(&menace).health, 9);
        }

        #[test]
        fn r3048001_r3048002_r399_an_echo_with_its_text_reads_the_mana_left_after_its_own_price_and_keeps_its_pick() {
            let mut s = scenario(json!({
                "seed": "meditative-005-echo",
                "p1": { "hand": [CUTS, ECHO, FILLER], "mana": 2, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            let ids = p2_units(&s);
            // 1 left: X = 2, so three hits on the first Unit, which makes Death by 1000 cuts the last Spell.
            s.play(CUTS, json!({ "targets": pick::id(&ids[0]) }));
            assert_eq!(damage_targets(s.events()), vec![ids[0].clone(); 3]);
            s.state_mut().players.p1.mana.current = 3;

            // Echo's own (1) is paid first, 2 left: X = 2 × 2 = 4, five hits on the one Unit it picks.
            s.play(ECHO, json!({ "targets": pick::id(&ids[1]) }));
            assert!(s.state().pending.is_none());
            assert_eq!(damage_targets(s.events())[3..].to_vec(), vec![ids[1].clone(); 5]);
        }

        #[test]
        fn r386_echo_and_damage_read_through_param() {
            let mut s = scenario(json!({
                "seed": "meditative-005-tuned",
                "p1": { "hand": [CUTS, FILLER], "mana": 2, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            step_param(s.card_mut(CUTS), "echo", 1);
            step_param(s.card_mut(CUTS), "damage", 1);
            let first = p2_units(&s)[0].clone();
            s.play(CUTS, json!({ "targets": pick::id(&first) }));

            // Echo 3 × 1 mana left: the play and 3 repeats, each dealing 2, all on the first pick.
            assert!(s.state().pending.is_none());
            assert_eq!(damage_amounts(&s), vec![2; 4]);
            assert_eq!(damage_targets(s.events()), vec![first; 4]);
        }

        mod radiant {
            use super::*;

            #[test]
            fn r3048001_cast_with_exactly_1_mana_it_cuts_once_and_heals_once() {
                let (mut s, first) = cut("meditative-005-radiant-one-mana", true, 1);

                assert!(s.state().pending.is_none());
                assert_eq!(damage_amounts(&s), vec![1]);
                assert_eq!(damage_targets(s.events()), vec![first]);
                // Lifesteal (§4.4 step 8): one hit heals 1.
                s.expect_health(P1, 11);
            }

            #[test]
            fn r3048001_with_3_mana_x_is_8_nine_hits_on_the_first_pick_each_healing_the_caster() {
                let (mut s, first) = cut("meditative-005-radiant-three-mana", true, 3);

                // 2 left: X = 4 × 2 = 8, the play and 8 repeats with no prompt. The ninth hit kills the
                // 9/9 it was all aimed at; Lifesteal healed once per hit (§4.4 step 8).
                assert!(s.state().pending.is_none());
                assert_eq!(damage_amounts(&s), vec![1; 9]);
                assert_eq!(damage_targets(s.events()), vec![first; 9]);
                s.expect_health(P1, 19);
            }
        }
    }

    /// R767000: the script kept for the replays of v0.5.11p and before.
    mod v0_5_11o {
        use super::*;

        #[test]
        fn r3048001_r3048002_r767000_the_kept_script_reads_max_mana_and_asks_a_fresh_target_each_repeat() {
            crate::register_all();
            let kept = script_v0_5_11o();
            for face in [&kept.base, &kept.radiant] {
                assert!(!face.echo_keeps_targets);
                assert!(face.echo_x.is_some());
            }
            let now = script();
            assert!(now.base.echo_keeps_targets && now.radiant.echo_keeps_targets);
            crate::under_kept(CUTS, kept.clone(), || {
                let mut s = scenario(json!({
                    "seed": "meditative-005-turn-1",
                    "turn": 1,
                    "p1": { "hand": [CUTS, FILLER], "mana": 9, "field": [MENACE] },
                    "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
                }));
                let ids = p2_units(&s);
                let first = pick::id(&ids[0]);
                s.play(CUTS, json!({ "targets": first }));

                // X = 2 × 1 max mana, whatever mana is left: the play plus 2 repeats, each asking a
                // fresh target of the caster.
                assert_eq!(answer_all(&mut s, &ids), 2);
                assert_eq!(damage_amounts(&s), vec![1, 1, 1]);
            });
        }
    }
}
