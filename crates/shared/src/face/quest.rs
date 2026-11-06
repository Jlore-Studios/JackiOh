//! A Quest card's rules box (the Quest tag, R1837000; R2450000): it prints only the stage the card stands
//! at, as Hearthstone's Questlines print the part they are on.
//!
//! A staged Quest (Stinkyscape #7 The Greatest Heist, #112 Job Application, R1854000) has a catalog line per
//! stage, "Quest: Stage 2, Comradery. <goal>. Reward: <reward>." (or a Chinese face's, R1301), and prints one
//! in their place: on the field, the stage the view's line stands at (`QuestView.stages`) with the goal it
//! drew (R1854002) or, while it waits to hear back, the turns left (R1854004); off the field, its first. In
//! Too Deep's tree (Classic #90, R404, R543) has no stage lines: on the field it prints its text, its open
//! quests with their rewards and its auras; off the field, its text alone.
//!
//! All of it is the catalog face's words and the view's public quest line, the same on both seats (R1854000,
//! CLAUDE.md rule 7). A Radiant face's marks (R277) and its tuned numbers in play (R386) move with their
//! words (`assemble`). The whole text stays in the hover preview, the touch sheet and the detail view; the
//! inspect overlay lists every stage.

use jackioh_engine::wire::{QuestStageState, QuestView, Tag};

use super::chinese::list_separator;
use super::diff::TextRange;
use super::state::{current_stage, quest_words, stage_title, stage_words};
use super::text::Text;
use super::tuning::TunedRange;
use super::words::Words;

/// What a Quest card's rules box prints, and whether it is less than the whole text (a staged Quest's one stage).
pub struct QuestFace {
    pub text: Text,
    pub less: bool,
}

const QUEST_TAG: Tag = Tag::Quest;

/// A printed stage line's head, and where its reward starts: English, and a Chinese face's (R1301).
struct StageForm {
    /// The line's opening words, which the printed stage keeps ("Quest: ").
    lead: &'static str,
    /// What stands between them and the stage's number.
    before: &'static str,
    /// What stands between the number and the stage's name.
    middle: &'static str,
    /// What ends the stage's name.
    stop: &'static str,
    reward: &'static str,
}

const STAGE_FORMS: [StageForm; 2] = [
    StageForm {
        lead: "Quest: ",
        before: "Stage ",
        middle: ", ",
        stop: ". ",
        reward: "Reward: ",
    },
    StageForm {
        lead: "任务：",
        before: "第",
        middle: "阶段，",
        stop: "。",
        reward: "奖励：",
    },
];

/// A stage line of the face's text, its offsets in the whole text.
struct StageLine {
    number: usize,
    name: String,
    start: usize,
    /// Where its "Quest: " ends.
    quest: usize,
    /// Where its goal starts, after the stage's name.
    goal: usize,
    /// Where its reward starts, or its end when it prints none.
    reward: usize,
    end: usize,
    /// What ends the stage's name, ". " or "。", and so each part the face prints of it.
    stop: String,
}

struct Line {
    start: usize,
    end: usize,
    stage: Option<StageLine>,
}

/// A stretch of the face's text, or words the face prints in its place.
enum Piece {
    Span { from: usize, to: usize },
    Words(String),
}

fn starts_with_at(line: &[char], at: usize, literal: &str) -> Option<usize> {
    let mut i = at;
    for c in literal.chars() {
        if line.get(i) != Some(&c) {
            return None;
        }
        i += 1;
    }
    Some(i)
}

/// `form.head` read off the start of `line`, and `form.reward` found after it.
fn stage_of(line: &[char], start: usize) -> Option<StageLine> {
    for form in &STAGE_FORMS {
        // The form is "<lead><before><digits><middle><name><stop>".
        let Some(lead_end) = starts_with_at(line, 0, form.lead) else {
            continue;
        };
        let Some(number_at) = starts_with_at(line, lead_end, form.before) else {
            continue;
        };
        let digits = line[number_at..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count();
        if digits == 0 {
            continue;
        }
        let Some(name_at) = starts_with_at(line, number_at + digits, form.middle) else {
            continue;
        };
        // The name is as short as it can be and not empty, up to the first stop after it.
        let stop: Vec<char> = form.stop.chars().collect();
        let Some(stop_at) = (name_at + 1..line.len()).find(|at| line[*at..].starts_with(&stop)) else {
            continue;
        };
        let head = stop_at + stop.len();
        let number = line[number_at..number_at + digits]
            .iter()
            .collect::<String>()
            .parse()
            .ok()?;
        let reward_words: Vec<char> = form.reward.chars().collect();
        let reward = (head..line.len()).find(|at| line[*at..].starts_with(&reward_words));
        return Some(StageLine {
            number,
            name: line[name_at..stop_at].iter().collect(),
            start,
            quest: start + lead_end,
            goal: start + head,
            reward: start + reward.unwrap_or(line.len()),
            end: start + line.len(),
            stop: form.stop.to_string(),
        });
    }
    None
}

fn lines_of(full: &[char]) -> Vec<Line> {
    let mut start = 0;
    full.split(|c| *c == '\n')
        .map(|line| {
            let at = start;
            start += line.len() + 1;
            Line {
                start: at,
                end: at + line.len(),
                stage: stage_of(line, at),
            }
        })
        .collect()
}

/// The face's text made of `pieces`, its marks and tuned numbers kept where their words now stand.
fn assemble(text: &Text, pieces: &[Piece]) -> Text {
    let mut full: Vec<char> = Vec::new();
    let mut marks: Vec<TextRange> = Vec::new();
    let mut tuned: Vec<TunedRange> = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Words(words) => full.extend(words.chars()),
            Piece::Span { from, to } => {
                let by = full.len() as isize - *from as isize;
                for mark in &text.marks {
                    let (start, end) = (mark.start.max(*from), mark.end.min(*to));
                    if start < end {
                        marks.push(TextRange {
                            start: start.saturating_add_signed(by),
                            end: end.saturating_add_signed(by),
                        });
                    }
                }
                for range in &text.tuned {
                    if range.start >= *from && range.end <= *to {
                        tuned.push(TunedRange {
                            start: range.start.saturating_add_signed(by),
                            end: range.end.saturating_add_signed(by),
                            ..range.clone()
                        });
                    }
                }
                full.extend(&text.full[*from..*to]);
            }
        }
    }
    Text { full, marks, tuned }
}

/// A stage as its line prints it, under `title`: "Stage 1 of 3, Initiation. Quest: <goal>. Reward: <reward>."
fn printed_stage(line: &StageLine, title: String) -> Vec<Piece> {
    vec![
        Piece::Words(format!("{title}{}", line.stop)),
        Piece::Span {
            from: line.start,
            to: line.quest,
        },
        Piece::Span {
            from: line.goal,
            to: line.end,
        },
    ]
}

/// What a staged Quest prints in place of its stage lines, or `None` when the view names no stage it prints.
fn stage_pieces(words: Words, lines: &[&StageLine], quest: Option<&QuestView>) -> Option<Vec<Piece>> {
    let Some(stages) = quest.and_then(|quest| quest.stages.as_deref()) else {
        let first = lines.first()?;
        return Some(printed_stage(
            first,
            stage_title(words, first.number, lines.len(), &first.name, None),
        ));
    };
    let quest = quest?;
    if !stages.is_empty() && stages.iter().all(|stage| stage.state == QuestStageState::Done) {
        return Some(vec![Piece::Words(stage_words(words, stages))]);
    }
    let (stage, number) = current_stage(stages)?;
    let line = lines.iter().find(|one| one.number == number)?;
    let title = stage_title(words, number, stages.len(), &line.name, stage.hear_back);
    let open = if stage.state == QuestStageState::Current {
        quest.open.as_slice()
    } else {
        &[]
    };
    if open.is_empty() {
        return Some(printed_stage(line, title));
    }
    let goals: Vec<String> = open.iter().map(|one| quest_words(words, one)).collect();
    Some(vec![
        Piece::Words(format!(
            "{title}{}{}{}",
            line.stop,
            goals.join(&line.stop),
            line.stop
        )),
        Piece::Span {
            from: line.reward,
            to: line.end,
        },
    ])
}

fn tree_words(words: Words, quest: &QuestView) -> String {
    let quests: Vec<String> = quest
        .open
        .iter()
        .map(|one| {
            if one.rewards.is_empty() {
                quest_words(words, one)
            } else {
                let rewards: Vec<String> = one.rewards.iter().map(|reward| words.say(&reward.text)).collect();
                words.say_with(
                    "{quest}. Rewards: {rewards}.",
                    &[
                        ("quest", quest_words(words, one)),
                        ("rewards", rewards.join(list_separator(words))),
                    ],
                )
            }
        })
        .collect();
    let mut lines = if quests.is_empty() {
        vec![words.say("Quest complete")]
    } else {
        quests
    };
    lines.extend(quest.auras.iter().map(|aura| words.say(&aura.text)));
    lines.iter().map(|line| format!("\n{line}")).collect()
}

/// R2450000: what a Quest card's rules box prints, or `None` for a face that prints its text as it is
/// (every other face, In Too Deep off the field, a staged face the view names no printed stage of).
pub fn quest_face_text(
    words: Words,
    tags: &[Tag],
    text: &Text,
    quest: Option<&QuestView>,
) -> Option<QuestFace> {
    if !tags.contains(&QUEST_TAG) {
        return None;
    }
    let lines = lines_of(&text.full);
    let stages: Vec<&StageLine> = lines.iter().filter_map(|line| line.stage.as_ref()).collect();
    if !stages.is_empty() {
        let printed = stage_pieces(words, &stages, quest)?;
        let mut printed = Some(printed);
        let mut pieces: Vec<Piece> = Vec::new();
        let mut placed = false;
        for line in &lines {
            if line.stage.is_some() && placed {
                continue;
            }
            if !pieces.is_empty() {
                pieces.push(Piece::Words("\n".to_string()));
            }
            match &line.stage {
                None => pieces.push(Piece::Span {
                    from: line.start,
                    to: line.end,
                }),
                Some(_) => {
                    pieces.extend(printed.take().unwrap_or_default());
                    placed = true;
                }
            }
        }
        return Some(QuestFace {
            text: assemble(text, &pieces),
            less: true,
        });
    }
    let quest = quest?;
    let pieces = [
        Piece::Span {
            from: 0,
            to: text.len(),
        },
        Piece::Words(tree_words(words, quest)),
    ];
    Some(QuestFace {
        text: assemble(text, &pieces),
        less: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::types::Locale;

    fn staged(text: &str) -> Text {
        Text::plain(text)
    }

    #[test]
    fn a_staged_quest_off_the_field_prints_its_first_stage() {
        let words = Words::new(Locale::En);
        let text = staged(
            "Quest: Stage 1, Initiation. Play 2 cards. Reward: Draw a card.\nQuest: Stage 2, Comradery. Play 3 cards. Reward: Gain 2 Armor.",
        );
        let got = quest_face_text(words, &[Tag::Quest], &text, None).expect("a quest box");
        assert!(got.less);
        assert_eq!(
            got.text.string(),
            "Stage 1 of 2, Initiation. Quest: Play 2 cards. Reward: Draw a card."
        );
    }

    #[test]
    fn a_tree_off_the_field_prints_its_text_alone() {
        let words = Words::new(Locale::En);
        assert!(quest_face_text(words, &[Tag::Quest], &staged("Quests are drawn."), None).is_none());
        assert!(quest_face_text(words, &[], &staged("Quest: Stage 1, A. B."), None).is_none());
    }
}
