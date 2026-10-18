//! Meditative+ #20 KY's Competition's question sequence (SPEC §8.9 row 20, R1184000, R1184001).
//!
//! C+ #42 KY's Test's `answer` prompt (R420, R465) asked again after each right answer. Each step is a
//! plain-data continuation — the streak rides the prompt's resume data under `KY_COMPETITION_STREAK`
//! — so a paused game survives JSON and replays (§9.3, `work.rs`):
//!
//!   cry     the first question, the streak 0.
//!   graded  a right answer raises the streak and asks the next question, until the streak reaches
//!           `KY_COMPETITION_MAX`; a wrong answer or Stop (a timeout's answer, R1184001) ends it. An
//!           ended sequence summons the card's Kenny with X the streak (`summon`'s `x`, R1184003),
//!           Radiant on the Radiant face; a streak of 0 summons nothing.
//!
//! A question is rolled from the match rng, never drawn from a bank: two operands from the face's
//! range (`KY_COMPETITION_OPERANDS`, the Radiant face's `KY_COMPETITION_EASY_OPERANDS`), added or
//! subtracted with equal chance, a subtraction's larger operand first, and three wrong options, the
//! answer moved by three different misses of the face's list, never below 0. `choose_answer` shows the
//! four in an rng-shuffled order under letters and keeps the right letter where no view reaches (R465).

use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::config::{
    IntRange, KY_COMPETITION_EASY_MISSES, KY_COMPETITION_EASY_OPERANDS, KY_COMPETITION_MAX,
    KY_COMPETITION_MISSES, KY_COMPETITION_OPERANDS, KY_TEST_OPTIONS,
};
use crate::effects::{ChooseAnswerArgs, SummonArgs, answered_correctly, choose_answer, summon};
use crate::rng::Rng;
use crate::script::{Effect, Hook, hook};

/// Resume-data key: the right answers so far, in a row.
pub const KY_COMPETITION_STREAK: &str = "kyCompetitionStreak";

/// The step a question's answer re-enters.
const GRADED: &str = "graded";

/// R1184000: a question's two operations, drawn with equal chance. The minus is Unicode's (§8.7's
/// plain text with Unicode maths), so a statement reads as it is written.
const PLUS: &str = "+";
const MINUS: &str = "−";
const OPERATIONS: [&str; 2] = [PLUS, MINUS];

/// R1184000: one question: its statement and its options, the answer first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KyCompetitionQuestion {
    pub statement: String,
    pub options: Vec<String>,
}

fn operand(rng: &mut Rng, range: IntRange) -> i32 {
    range.min + rng.int(range.max - range.min + 1)
}

/// R1184000: a question rolled from the match rng, three-digit on the base face and one- or two-digit
/// on the Radiant face (`easy`): `a + b` or `a − b` with a ≥ b, and three wrong options, each the
/// answer moved by a different miss, none below 0, so the four always differ.
pub fn competition_question(rng: &mut Rng, easy: bool) -> KyCompetitionQuestion {
    let (range, misses) = if easy {
        (KY_COMPETITION_EASY_OPERANDS, KY_COMPETITION_EASY_MISSES)
    } else {
        (KY_COMPETITION_OPERANDS, KY_COMPETITION_MISSES)
    };
    let first = operand(rng, range);
    let second = operand(rng, range);
    let operation = rng.pick(&OPERATIONS).copied().unwrap_or(PLUS);
    let (a, b, answer) = if operation == MINUS {
        let (a, b) = (first.max(second), first.min(second));
        (a, b, a - b)
    } else {
        (first, second, first + second)
    };
    let wrong = (KY_TEST_OPTIONS - 1).max(0) as usize;
    let mut options = vec![answer.to_string()];
    options.extend(
        rng.shuffle(misses)
            .into_iter()
            .map(|miss| answer + miss)
            .filter(|option| *option >= 0)
            .take(wrong)
            .map(|option| option.to_string()),
    );
    KyCompetitionQuestion {
        statement: format!("{a} {operation} {b} = ?"),
        options,
    }
}

/// R1184002: what a player works out from a question's statement as it is shown, `a + b = ?` or
/// `a − b = ?`: the AI's answer, which never reads the key (R465). `None` for any other statement.
pub fn worked_answer(statement: &str) -> Option<i32> {
    let sum = statement.strip_suffix(" = ?")?;
    OPERATIONS.iter().find_map(|operation| {
        let (a, b) = sum.split_once(&format!(" {operation} "))?;
        let (a, b) = (a.parse::<i32>().ok()?, b.parse::<i32>().ok()?);
        if *operation == MINUS {
            a.checked_sub(b)
        } else {
            a.checked_add(b)
        }
    })
}

/// The streak a continuation carries (0 when it carries none: the cry).
fn streak_of(data: &IndexMap<String, Value>) -> i32 {
    data.get(KY_COMPETITION_STREAK)
        .and_then(Value::as_i64)
        .and_then(|streak| i32::try_from(streak).ok())
        .unwrap_or(0)
}

/// The next question, rolled as it is asked, with `streak` right answers so far.
fn ask(streak: i32) -> Effect {
    Effect::new("kyCompetitionAsk", move |ctx| {
        let question = competition_question(&mut *ctx.sink.rng, ctx.radiant);
        let mut data = IndexMap::new();
        data.insert(KY_COMPETITION_STREAK.to_string(), json!(streak));
        let asked = choose_answer(ChooseAnswerArgs {
            step: GRADED.to_string(),
            statement: question.statement,
            options: question.options,
            correct: 0,
            streak: Some(streak),
            data: Some(data),
            ..ChooseAnswerArgs::default()
        });
        (asked.apply)(ctx);
    })
}

/// `ky_competition_script`'s answer: the card's Cry and the step its prompts re-enter.
#[derive(Clone)]
pub struct KyCompetitionScript {
    pub cry: Hook,
    pub resume: IndexMap<&'static str, Hook>,
}

/// The whole card: its Cry and the step each answer re-enters. `kenny` is the token an ended
/// sequence summons with X the streak (Meditative+ #20.1).
pub fn ky_competition_script(kenny: &'static str) -> KyCompetitionScript {
    let graded: Hook = hook(move |ctx| {
        // A wrong answer and Stop alike end the sequence with the right answers before them, every
        // one of them in a row, since the first miss ends it.
        let right = answered_correctly(ctx);
        let streak = streak_of(&ctx.data) + i32::from(right);
        if right && streak < KY_COMPETITION_MAX {
            return vec![ask(streak)];
        }
        if streak <= 0 {
            return vec![];
        }
        vec![summon(SummonArgs {
            def_id: Some(kenny.to_string()),
            x: Some(streak),
            radiant: ctx.radiant.then_some(true),
            ..SummonArgs::default()
        })]
    });
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(GRADED, graded);
    KyCompetitionScript {
        cry: hook(|_ctx| vec![ask(0)]),
        resume,
    }
}
