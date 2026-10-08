//! M #5 Death by 1000 cuts (SPEC §8.8 row 5, BUILD M10 row M 5). (1) Spell, Rare.
//!   Base:    "Echo X\nDeal {damage} damage to a Unit. X is {echo|time|times} your max mana." (1; 2)
//!   Radiant: "Echo X, Lifesteal\n…" (1; 4)
//!
//! A computed Echo X (R802): `Script.echo_x` reads `param(echo) × max_mana_of(controller)` (§2.3:
//! min(turns started, 4) plus persistent modifiers) as the card is played (§10.5 step 4), so X stays
//! fixed while the card resolves — mana gained mid-resolution changes nothing, and a random cast
//! uses its caster's max mana. The larger of the hook and `staticFlags.echo` holds (here there is no
//! fixed one); the card's "Echo" tuning step and a Twinspell grant then add on (`echo::printed_echo`,
//! R30, R386). At 4 max mana that is 9 hits (17 on the Radiant face).
//!
//! The declared target is any Unit on either side (R81); each repeat asks a fresh target prompt (§6.2
//! Echo, §10.6), and a repeat with no Unit to hit fizzles. Lifesteal is printed on the Radiant face,
//! as on Core #93.1 Combo-Fodder, and heals once per hit (§4.4 step 8). Both numbers are declared and
//! read through `param` (R386).

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-005";

/// One script serves both faces: the Radiant face's Lifesteal is its catalog keyword, and its larger
/// `echo` is its declared number, both read through `param`.
fn cuts() -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        // R802: X is `echo` times the caster's max mana, fixed as the card is played.
        echo_x: Some(read_hook(|args| param(&args, "echo") * max_mana_of(args.state, args.self_.controller))),
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
    let both = cuts();
    CardScripts {
        base: both.clone(),
        radiant: both,
    }
}

// M #5 Death by 1000 cuts — SPEC §8.8 row 5, BUILD M10 row M 5: "Echo X with X fixed at play from the
// caster's max mana (R802): 3 hits at 1 max mana, 9 at 4 (17 Lifesteal hits on the Radiant face); a
// random cast uses its caster's max mana; Twinspell adds 1; a repeat with no Unit fizzles; its tuned
// numbers (echo, damage) read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CUTS: &str = "meditative-005";
    const MENACE: &str = "core-019"; // 9/9 Taunt.
    const FODDER: &str = "core-015"; // 1/1 Unit.
    const FILLER: &str = "core-005"; // (1) Spell.
    const TWINSPELL: &str = "core-079"; // (2) Field Spell: the next Spell echoes once more.

    /// A target answer naming the given instance.
    fn at_id(id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": id }])
    }

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
            s.answer(at_id(id));
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

    mod meditative_005 {
        use super::*;

        #[test]
        fn is_a_1_cost_rare_spell_with_a_computed_echo_on_both_faces() {
            crate::register_all();
            assert_eq!(ID, CUTS);
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert_eq!(def.rarity, Rarity::Rare);
            assert_eq!(def.type_, CardType::Spell);
            let scripts = script();
            assert!(scripts.base.echo_x.is_some());
            assert!(scripts.radiant.echo_x.is_some());
            assert_eq!(scripts.base.targets.len(), 1);
        }

        #[test]
        fn r802_at_1_max_mana_it_hits_3_times() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-005-turn-1",
                "turn": 1,
                "p1": { "hand": [CUTS, FILLER], "mana": 9, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            let ids = p2_units(&s);
            let first = at_id(&ids[0]);
            s.play(CUTS, json!({ "targets": first }));

            // X = 2 × 1: the play plus 2 repeats.
            assert_eq!(answer_all(&mut s, &ids), 2);
            assert_eq!(damage_amounts(&s), vec![1, 1, 1]);
        }

        #[test]
        fn r802_at_4_max_mana_it_hits_9_times() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-005-turn-4",
                "p1": { "hand": [CUTS, FILLER], "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            assert_eq!(s.state().players.p1.mana.max, 4);
            let ids = p2_units(&s);
            let first = at_id(&ids[0]);
            s.play(CUTS, json!({ "targets": first }));

            // X = 2 × 4: the play plus 8 repeats, spread over two 9-health units.
            assert_eq!(answer_all(&mut s, &ids), 8);
            assert_eq!(damage_amounts(&s), vec![1; 9]);
        }

        #[test]
        fn r802_a_random_cast_uses_its_casters_max_mana() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-005-random-cast",
                "p1": { "hand": [FILLER], "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            assert_eq!(s.state().players.p1.mana.max, 4);
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
            let damage = events.iter().filter(|event| matches!(event, GameEvent::Damage { .. })).count();
            // p1's max mana, 4: nine random hits, no prompt ever opens.
            assert_eq!(damage, 9);
        }

        #[test]
        fn r802_a_twinspell_adds_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-005-twinspell",
                "turn": 1,
                "p1": { "hand": [TWINSPELL, CUTS, FILLER], "mana": 9, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));

            s.play(TWINSPELL, json!({}));
            let ids = p2_units(&s);
            let first = at_id(&ids[0]);
            s.play(CUTS, json!({ "targets": first }));

            // X = 2 × 1, plus the grant's 1: the play plus 3 repeats.
            assert_eq!(answer_all(&mut s, &ids), 3);
            assert_eq!(damage_amounts(&s), vec![1; 4]);
        }

        #[test]
        fn r802_a_repeat_with_no_unit_fizzles() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-005-fizzle",
                "turn": 1,
                "p1": { "hand": [CUTS, FILLER], "mana": 9 },
                "p2": { "hand": [FILLER], "field": [FODDER] },
            }));
            let fodder = s.card(FODDER).id.clone();
            s.play(CUTS, json!({ "targets": at_id(&fodder) }));

            // The first hit kills the 1/1; both repeats find no Unit and fizzle, asking nothing.
            assert!(s.state().pending.is_none());
            assert_eq!(damage_amounts(&s), vec![1]);
            s.expect_in_zone(FODDER, "graveyard");
        }

        #[test]
        fn r386_echo_and_damage_read_through_param() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-005-tuned",
                "turn": 1,
                "p1": { "hand": [CUTS, FILLER], "mana": 9, "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            step_param(s.card_mut(CUTS), "echo", 1);
            step_param(s.card_mut(CUTS), "damage", 1);
            let ids = p2_units(&s);
            let first = at_id(&ids[0]);
            s.play(CUTS, json!({ "targets": first }));

            // Echo 3 × 1 max mana: the play plus 3 repeats, each dealing 2.
            assert_eq!(answer_all(&mut s, &ids), 3);
            assert_eq!(damage_amounts(&s), vec![2; 4]);
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_hits_17_times_at_4_and_each_hit_heals_the_caster() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "meditative-005-radiant",
                    "p1": { "health": 10, "hand": [{ "def": CUTS, "radiant": true }, FILLER], "field": [MENACE] },
                    "p2": { "hand": [FILLER], "field": [MENACE, MENACE, MENACE] },
                }));
                assert_eq!(s.state().players.p1.mana.max, 4);
                let ids = p2_units(&s);
                assert_eq!(ids.len(), 3);
                let first = at_id(&ids[0]);
                s.play(CUTS, json!({ "targets": first }));

                // X = 4 × 4: the play plus 16 repeats, spread so no 9-health unit dies.
                assert_eq!(answer_all(&mut s, &ids), 16);
                assert_eq!(damage_amounts(&s), vec![1; 17]);
                // Lifesteal (§4.4 step 8): one hit heals 1, seventeen heal 17.
                s.expect_health(P1, 27);
            }
        }
    }
}
