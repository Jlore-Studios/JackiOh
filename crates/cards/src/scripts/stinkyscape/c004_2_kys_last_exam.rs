//! S #4.2 KY's Last Exam (SPEC §8.10 row S 4.2, SPEC §7; §4.4, §6.2 Echo, R79, R185, R420, R465,
//! R1857002, R1870002): (4) Spell, Terminus, KY, Token (printed Mythic), one of Olivian Expanse's picks.
//!   Base:    "Solve an impossible problem. If you get it right, deal {damage} damage to the enemy hero,
//!            and this gains Echo +1." (30)
//!   Radiant: "Solve an impossible problem. You snuck in a cheat sheet: the right answer is marked. …"
//!
//! Engine: C+ #42's `answer` prompt (R420) for its caster: one problem drawn from the bank of
//! impossible ones (`ky_last_exam_bank.rs`), its right option drawn with it, the four shown under
//! letters in an rng order and the key kept in the engine (R465). No answerer can work it out, so
//! each guesses: the player, the AI, the random policy and a timeout (R79). Right: one hit of
//! `damage` on the enemy hero (§4.4, its caster's Spell Damage raising it), then `gain_echo`: the card
//! is owed one more resolution (§6.2 Echo), which asks a new problem, the repeats a play gives itself
//! capped at `SELF_ECHO_GAIN_CAP`.

use jackioh_engine::effects::{ChooseAnswerArgs, answered_correctly, choose_answer, damage, gain_echo};
use jackioh_engine::prelude::*;

use crate::ky_last_exam_bank::{KY_LAST_EXAM_BANK, KY_LAST_EXAM_OPTIONS};

pub const ID: &str = "stinkyscape-004-2";

/// The cheat sheet: what the Radiant face writes after the right option's label.
pub const CHEAT_SHEET_MARK: &str = " ✓";

/// "This gains Echo +1."
const ECHO_GAIN: i32 = 1;

/// The answer comes back here (§10.6).
const GRADED: &str = "graded";

fn ask(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let problem = &KY_LAST_EXAM_BANK[ctx.rng.int(KY_LAST_EXAM_BANK.len() as i32) as usize];
    let correct = ctx.rng.int(KY_LAST_EXAM_OPTIONS as i32) as usize;
    let options: Vec<String> = problem
        .options
        .iter()
        .enumerate()
        .map(|(at, option)| {
            if ctx.radiant && at == correct {
                format!("{option}{CHEAT_SHEET_MARK}")
            } else {
                (*option).to_string()
            }
        })
        .collect();
    vec![choose_answer(ChooseAnswerArgs {
        step: GRADED.to_string(),
        statement: problem.statement.to_string(),
        options,
        correct,
        ..ChooseAnswerArgs::default()
    })]
}

fn graded(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    if !answered_correctly(ctx) {
        return vec![];
    }
    vec![
        damage(json_as(
            json!({ "to": { "of": "enemyHero" }, "amount": param(&*ctx, "damage") }),
        )),
        gain_echo(json_as(json!({ "amount": ECHO_GAIN }))),
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(ask)),
        resume: IndexMap::from([(GRADED, hook(graded))]),
        ..Script::default()
    };
    // The face is read off the context, so both faces run the one script.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #4.2 KY's Last Exam — SPEC §8.10 row S 4.2, BUILD M12 row S 4.2 (R1870002).
#[cfg(test)]
mod tests {
    use super::{CHEAT_SHEET_MARK, ID};
    use crate::ky_last_exam_bank::{KY_LAST_EXAM_BANK, KY_LAST_EXAM_OPTIONS};
    use jackioh_engine::config::SELF_ECHO_GAIN_CAP;
    use jackioh_engine::params::step_param;
    use jackioh_engine::subsystems::ai_policy::choose_action;
    use jackioh_engine::testkit::*;

    use crate::js;
    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const SOLARIUS: &str = "classicplus-038"; // a Unit with Spell Damage +2.
    const SOLARIUS_SPELL_DAMAGE: i32 = 2;
    const DAMAGE: i32 = 30;

    fn exam(seed: &str, radiant: bool, health: i32) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER], "health": health },
        }));
        s.play(ID, json!({}));
        s
    }

    fn key(s: &Scenario) -> String {
        let pending = s.state().pending.as_ref().expect("a problem is open");
        answer_key_of(&pending.resume.data).expect("its key")
    }

    fn answer(s: &mut Scenario, right: bool) {
        let correct = key(s);
        let letter = if right {
            correct
        } else {
            ["A", "B", "C", "D"]
                .into_iter()
                .find(|letter| *letter != correct)
                .unwrap_or_default()
                .to_string()
        };
        s.answer(json!(format!("mode:{letter}")));
    }

    fn labels(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .map(|pending| pending.options.iter().map(|option| option.label.clone()).collect())
            .unwrap_or_default()
    }

    fn exam_in_graveyard(s: &Scenario) -> bool {
        s.pile(P1, "graveyard").iter().any(|card| card.def_id == ID)
    }

    mod r1870002_kys_last_exam {
        use super::*;

        #[test]
        fn r1870002_opens_a_four_option_answer_for_its_caster() {
            let s = exam("kys-last-exam-open", false, HERO_HEALTH);
            let pending = s.state().pending.clone().expect("a problem is open");
            assert_eq!((pending.kind, pending.player_id), (PromptKind::Answer, P1));
            assert_eq!(pending.options.len(), KY_LAST_EXAM_OPTIONS);
            assert!(
                KY_LAST_EXAM_BANK.iter().any(|problem| problem.statement == pending.prompt),
                "a problem of the bank: {}",
                pending.prompt
            );
            let ids: Vec<String> = pending.options.iter().map(|option| option.key.clone()).collect();
            assert_eq!(ids.len(), 4);
            assert_eq!(
                legal_actions(s.state(), P1)
                    .iter()
                    .filter(|action| matches!(action, ActionBody::Answer { .. }))
                    .count(),
                4
            );
            assert!(
                legal_actions(s.state(), P2)
                    .iter()
                    .all(|action| !matches!(action, ActionBody::Answer { .. }))
            );
        }

        #[test]
        fn r1870002_key_in_neither_view() {
            let s = exam("kys-last-exam-key", false, HERO_HEALTH);
            assert_eq!(js(&s.view(P2))["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
            for viewer in [P1, P2] {
                let text = serde_json::to_string(&s.view(viewer)).expect("a view is JSON");
                assert!(!text.contains(ANSWER_KEY), "{viewer:?}");
            }
        }

        #[test]
        fn r1870002_right_answer_deals_thirty_and_asks_again() {
            let mut s = exam("kys-last-exam-right", false, 2 * DAMAGE + 1);
            answer(&mut s, true);
            s.expect_health(P2, DAMAGE + 1);
            assert!(s.state().pending.is_some(), "it resolves again with a new problem");
            assert!(!exam_in_graveyard(&s), "it is still resolving");
            answer(&mut s, false);
            assert!(s.state().pending.is_none());
            s.expect_health(P2, DAMAGE + 1);
            assert!(exam_in_graveyard(&s), "a wrong answer ends it");
        }

        #[test]
        fn r1870002_spell_damage_raises_it() {
            let mut s = scenario(json!({
                "seed": "kys-last-exam-spell-damage",
                "p1": { "hand": [ID, FILLER], "field": [SOLARIUS], "mana": 10 },
                "p2": { "hand": [FILLER] },
            }));
            s.play(ID, json!({}));
            answer(&mut s, true);
            s.expect_health(P2, HERO_HEALTH - DAMAGE - SOLARIUS_SPELL_DAMAGE);
        }

        #[test]
        fn r1870002_wrong_answer_ends_it() {
            let mut s = exam("kys-last-exam-wrong", false, HERO_HEALTH);
            answer(&mut s, false);
            assert!(s.state().pending.is_none());
            s.expect_health(P2, HERO_HEALTH);
            assert!(exam_in_graveyard(&s), "the Spell was still played");
        }

        #[test]
        fn r1870002_random_policy_and_timeout_answer_it() {
            let s = exam("kys-last-exam-policy", false, HERO_HEALTH);
            let mut rng = Rng::new("kys-last-exam-policy", 0);
            let action = choose_action(s.state(), P1, &mut rng).expect("the policy answers");
            assert!(matches!(action, ActionBody::Answer { .. }), "{action:?}");
            let answered = reduce(s.state(), &Action::new(action, P1, "kys-last-exam-policy"));
            assert_eq!(answered.error, None);

            let timed = reduce(s.state(), &Action::new(ActionBody::Timeout, P1, "kys-last-exam-timeout"));
            assert_eq!(timed.error, None);
            let still = timed.state.pending.as_ref().map(|pending| pending.id.clone());
            let asked = s.state().pending.as_ref().map(|pending| pending.id.clone());
            assert_ne!(still, asked, "the timeout answered the problem");
        }

        #[test]
        fn r1870002_repeats_stop_at_the_cap() {
            let rounds = 1 + SELF_ECHO_GAIN_CAP;
            let mut s = exam("kys-last-exam-cap", false, DAMAGE * rounds + 1);
            for round in 0..rounds {
                assert!(s.state().pending.is_some(), "round {round} asks");
                answer(&mut s, true);
            }
            assert!(s.state().pending.is_none(), "no repeat past the cap");
            s.expect_health(P2, 1);
            assert!(exam_in_graveyard(&s));
        }

        #[test]
        fn r1870002_damage_reads_param() {
            let mut s = scenario(json!({
                "seed": "kys-last-exam-param",
                "p1": { "hand": [ID, FILLER], "mana": 10 },
                "p2": { "hand": [FILLER] },
            }));
            let card = s.hand(P1).into_iter().find(|card| card.def_id == ID).expect("in hand");
            step_param(s.card_mut(card.id.as_str()), "damage", -1);
            s.play(ID, json!({}));
            answer(&mut s, true);
            s.expect_health(P2, HERO_HEALTH - (DAMAGE - 5));
        }

        #[test]
        fn r1870002_bank_has_four_different_options_each() {
            let mut ids: Vec<&str> = KY_LAST_EXAM_BANK.iter().map(|problem| problem.id).collect();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), KY_LAST_EXAM_BANK.len(), "every id is unique");
            for problem in KY_LAST_EXAM_BANK {
                let mut options = problem.options.to_vec();
                options.sort_unstable();
                options.dedup();
                assert_eq!(options.len(), KY_LAST_EXAM_OPTIONS, "{}", problem.id);
                assert!(!problem.options.iter().any(|option| option.contains(CHEAT_SHEET_MARK.trim())));
            }
        }

        #[test]
        fn r1870002_the_right_option_varies_over_seeds() {
            let mut labels_right: Vec<String> = Vec::new();
            for i in 0..20 {
                let s = exam(&format!("kys-last-exam-vary-{i}"), false, HERO_HEALTH);
                labels_right.push(key(&s));
            }
            labels_right.sort();
            labels_right.dedup();
            assert!(labels_right.len() > 1, "the key is not always one letter");
        }

        #[test]
        fn r1870002_radiant_marks_the_right_option_for_the_chooser_only() {
            for i in 0..10 {
                let s = exam(&format!("kys-last-exam-sheet-{i}"), true, HERO_HEALTH);
                let pending = s.state().pending.clone().expect("a problem is open");
                let key = key(&s);
                for option in &pending.options {
                    let marked = option.label.ends_with(CHEAT_SHEET_MARK);
                    assert_eq!(marked, option.key == format!("mode:{key}"), "{:?}", labels(&s));
                }
                let mine = js(&s.view(P1))["pending"]["options"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                assert_eq!(
                    mine.iter()
                        .filter(|option| option["label"].as_str().unwrap_or_default().ends_with(CHEAT_SHEET_MARK))
                        .count(),
                    1
                );
                let theirs = serde_json::to_string(&s.view(P2)).expect("a view is JSON");
                assert!(!theirs.contains(CHEAT_SHEET_MARK.trim()), "p2 never sees it");
                assert!(!theirs.contains(ANSWER_KEY));
            }
            let s = exam("kys-last-exam-sheet-base", false, HERO_HEALTH);
            assert!(labels(&s).iter().all(|label| !label.ends_with(CHEAT_SHEET_MARK)));
        }
    }
}
