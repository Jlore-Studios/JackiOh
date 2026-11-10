//! The per-card states a face draws beside its printed text (SPEC §10.8): Brittle (B3.3, R385), Temporary
//! (#2437), enchantments (B5 E39), a card in a unit zone as a Unit (B3.1, R383), a card Degrade or Upgrade
//! changed (B3.4, R386, `tuning.rs`), open quests (Classic #90, B5 E33, R404) and a staged quest's stage
//! (R1854000). Each is a badge and an inspect-overlay line in the words below (R373, R432: "(N) Cost" the
//! noun, "costs (N)" the verb), as in a Quest card's rules box (`quest.rs`, R2450000). The rail draws what
//! ends the card (Brittle, Temporary) first, then what it is doing (the quests), then the rest, all read off
//! the face (CLAUDE.md rule 7).

use indexmap::IndexMap;
use jackioh_engine::wire::{
    CardType, Enchantment, QuestOpenView, QuestStageState, QuestStageView, QuestView,
};

use super::chinese::{created_word, type_word};
use crate::config::TEMPORARY_LAST_COUNT;

use super::tuning::verdict_word;
use super::types::{
    AnimatedFace, BadgeIcon, DormantFace, FaceTuning, StarshipFace, StateBadge, StateBadgeKind, TuneVerdict,
};
use super::words::Words;

/// #2437: the Temporary badge's glyph, an hourglass in text presentation (U+FE0E), like [`DORMANT_GLYPH`].
const TEMPORARY_GLYPH: &str = "\u{231B}\u{FE0E}";
/// R1174000: the frost badge's glyph, drawn in text presentation (U+FE0E) so it takes the badge's color.
const FROZEN_GLYPH: &str = "\u{2744}\u{FE0E}";
/// R1839000: the Dormant badge's glyph, a crescent moon in text presentation (U+FE0E).
const DORMANT_GLYPH: &str = "\u{263E}\u{FE0E}";
/// R1852000: the Starship badge's glyph, a comet in text presentation (U+FE0E).
const STARSHIP_GLYPH: &str = "\u{2604}\u{FE0E}";
/// R3536005: the Evolved badge's glyph, a star in text presentation (U+FE0E).
pub const EVOLVED_GLYPH: &str = "\u{2726}\u{FE0E}";
/// R3536005, §6.3: what an Evolved card's badge says.
pub const EVOLVED_WORDS: &str =
    "Evolved: its Evolve has happened, or happens as it enters the field; it evolves only once";

/// B5 E35, §6.1: what a Berserk unit's badge says.
const BERSERK_WORDS: &str = "Berserk: at the start and end of its controller's turn it attacks its own hero";
/// R1174000, R2246000, §6.1: what a Frozen unit's badge says.
const FROZEN_WORDS: &str = "Frozen: can't attack, switch position or use its Activate; it thaws at the end of the turn it could have acted in";
/// R1848000, R2246000, §6.1: what the badge of a unit Frozen forever says.
const FROZEN_FOREVER_WORDS: &str =
    "Frozen forever: can't attack, switch position or use its Activate, and never thaws";
/// R1839000, §6.1: what a Dormant card's badge says when no countdown will wake it.
const DORMANT_WORDS: &str =
    "Dormant: it can't attack or be attacked and nothing reaches it, until an effect awakens it";
/// R1839001: what a Dormant card's badge says on the last turn of its countdown.
const DORMANT_NEXT_TURN_WORDS: &str =
    "Dormant: it awakens at the start of its controller's next turn; until then nothing reaches it";

pub fn brittle_words(words: Words, count: i32) -> String {
    words.say_with("Brittle {n}: crumbles at 0", &[("n", count.to_string())])
}

/// #2437, R637, R2179001: a Temporary card's badge words, with the count it has left in its owner's hand.
pub fn temporary_words(words: Words, count: i32) -> String {
    if count > TEMPORARY_LAST_COUNT {
        return words.say_with(
            "Temporary {n}: discarded after {n} of its owner's ends of turn",
            &[("n", count.to_string())],
        );
    }
    words.say("Temporary 1: discarded at the end of its owner's turn")
}

pub fn quest_progress(quest: &QuestOpenView) -> String {
    format!("{}/{}", quest.progress, quest.goal)
}

/// R404: an open quest in words, "Quest: Draw 2 cards (1/2)"; R1854002: with the cost its play must have.
pub fn quest_words(words: Words, quest: &QuestOpenView) -> String {
    // `quest.text` is the engine's English (the view's), looked up as it is displayed.
    let text = words.say(&quest.text);
    let progress = quest_progress(quest);
    match quest.cost {
        None => words.say_with(
            "Quest: {text} ({progress})",
            &[("text", text), ("progress", progress)],
        ),
        Some(cost) => words.say_with(
            "Quest: {text}, a card that costs ({cost}) ({progress})",
            &[("text", text), ("cost", cost.to_string()), ("progress", progress)],
        ),
    }
}

/// R1854000: the stage a staged line stands at — the one open, else the one it hears back with, else the
/// last done — and its number.
pub fn current_stage(stages: &[QuestStageView]) -> Option<(&QuestStageView, usize)> {
    let at = stages
        .iter()
        .position(|stage| stage.state == QuestStageState::Current)
        .or_else(|| stages.iter().position(|stage| stage.hear_back.is_some()))
        .or_else(|| {
            stages
                .iter()
                .rposition(|stage| stage.state == QuestStageState::Done)
        })?;
    Some((&stages[at], at + 1))
}

/// R1854000, R1854004, R2450000: a stage named, "Stage 2 of 3, Comradery", with the turns left while the line
/// waits to hear back with it.
pub fn stage_title(words: Words, number: usize, of: usize, name: &str, wait: Option<i32>) -> String {
    let base = [
        ("n", number.to_string()),
        ("of", of.to_string()),
        ("name", name.to_string()),
    ];
    match wait {
        None => words.say_with("Stage {n} of {of}, {name}", &base),
        Some(wait) if wait <= 1 => words.say_with("Stage {n} of {of}, {name}, hearing back next turn", &base),
        Some(wait) => {
            let mut all = base.to_vec();
            all.push(("turns", wait.to_string()));
            words.say_with("Stage {n} of {of}, {name}, hearing back in {turns} turns", &all)
        }
    }
}

/// R1854000, R1854004: a staged line in words, off the view's stages alone.
pub fn stage_words(words: Words, stages: &[QuestStageView]) -> String {
    if !stages.is_empty() && stages.iter().all(|stage| stage.state == QuestStageState::Done) {
        return words.say("Quest complete");
    }
    match current_stage(stages) {
        None => String::new(),
        Some((stage, number)) => stage_title(
            words,
            number,
            stages.len(),
            &words.say(&stage.name),
            stage.hear_back,
        ),
    }
}

/// E39, R432: an enchantment in a player's words. `radiant` is the face it rides, which Classic #55's swap
/// reads: a Radiant card becomes a Radiant Book (R671).
pub fn enchantment_words(words: Words, enchantment: &Enchantment, radiant: bool) -> String {
    match enchantment {
        Enchantment::ReturnAfterResolve { floor } => words.say_with(
            "Returns to hand · can't cost less than ({floor})",
            &[("floor", floor.to_string())],
        ),
        Enchantment::CastOnDraw => words.say("Cast on draw"),
        Enchantment::TargetEnemies => words.say("Targets enemies"),
        Enchantment::SwapsBook { .. } => {
            if radiant {
                words.say("End of turn: Become a random other Radiant Book")
            } else {
                words.say("End of turn: Become a random other Book")
            }
        }
    }
}

/// R1852000, R1852003, R1852005: a Starship's badge words — the pieces it holds, and whether it has launched.
pub fn starship_words(words: Words, pieces: i32, launched: bool) -> String {
    if launched {
        return words.say_with(
            "Starship: launched with {n} Starship Pieces fused into it",
            &[("n", pieces.to_string())],
        );
    }
    words.say_with(
        "Dormant Starship: {n} Starship Pieces fused into it; its Activate launches it",
        &[("n", pieces.to_string())],
    )
}

/// R1839000, R1839001: a Dormant card's badge words, with its countdown when it has one.
pub fn dormant_words(words: Words, turns: Option<i32>) -> String {
    match turns {
        None => words.say(DORMANT_WORDS),
        Some(turns) if turns <= 1 => words.say(DORMANT_NEXT_TURN_WORDS),
        Some(turns) => words.say_with(
            "Dormant: it awakens {n} of its controller's turns from now; until then nothing reaches it",
            &[("n", turns.to_string())],
        ),
    }
}

fn enchantment_icon(enchantment: &Enchantment) -> BadgeIcon {
    match enchantment {
        Enchantment::ReturnAfterResolve { .. } => BadgeIcon::ReturnHand,
        Enchantment::CastOnDraw => BadgeIcon::CastOnDraw,
        Enchantment::TargetEnemies => BadgeIcon::Target,
        Enchantment::SwapsBook { .. } => BadgeIcon::Cog,
    }
}

fn enchantment_kind(enchantment: &Enchantment) -> (StateBadgeKind, &'static str) {
    match enchantment {
        Enchantment::ReturnAfterResolve { .. } => (StateBadgeKind::ReturnAfterResolve, "returnAfterResolve"),
        Enchantment::CastOnDraw => (StateBadgeKind::CastOnDraw, "castOnDraw"),
        Enchantment::TargetEnemies => (StateBadgeKind::TargetEnemies, "targetEnemies"),
        Enchantment::SwapsBook { .. } => (StateBadgeKind::SwapsBook, "swapsBook"),
    }
}

/// R383: a Field Spell, Trap or Field Trap standing in a unit zone as a Unit, and where an "Animated on your
/// turn" card goes back to (`UnitView.animated.home`, a backrow lane).
pub fn animated_words(words: Words, animated: &AnimatedFace, type_: CardType) -> String {
    match animated.home {
        None => {
            let kind = if words.is_zh_cn() {
                type_word(type_).to_string()
            } else {
                type_.as_str().to_string()
            };
            words.say_with(
                "Animated: this {type} stands in a unit zone as a Unit",
                &[("type", kind)],
            )
        }
        Some(lane) => words.say_with(
            "Animated on your turn: back to its backrow (lane {lane}) at the end of its controller's turn",
            &[("lane", lane.to_string())],
        ),
    }
}

/// An enchantment list with each distinct enchantment once, in order.
pub fn distinct_enchantments(list: &[Enchantment]) -> Vec<&Enchantment> {
    let mut out: Vec<&Enchantment> = Vec::new();
    for entry in list {
        if !out.contains(&entry) {
            out.push(entry);
        }
    }
    out
}

/// What a face in play says of its states, as `state_badges` reads it.
pub struct BadgeFacts<'a> {
    pub words: Words,
    pub radiant: bool,
    pub type_: CardType,
    /// ME-CN, R1301: the face is Chinese.
    pub chinese: bool,
    pub brittle: Option<i32>,
    pub temporary: Option<i32>,
    pub quest: Option<&'a QuestView>,
    pub tuning: Option<&'a FaceTuning>,
    pub enchantments: &'a [Enchantment],
    pub animated: Option<&'a AnimatedFace>,
    pub berserk: bool,
    pub frozen: bool,
    pub frozen_forever: bool,
    pub dormant: Option<&'a DormantFace>,
    pub starship: Option<&'a StarshipFace>,
    pub created: bool,
    pub evolved: bool,
}

fn data(pairs: &[(&str, String)]) -> IndexMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

/// Every state badge a face draws, in rail order; empty for a face with none.
pub fn state_badges(facts: &BadgeFacts<'_>) -> Vec<StateBadge> {
    let words = facts.words;
    let mut badges = Vec::new();
    if let Some(brittle) = facts.brittle {
        badges.push(StateBadge {
            kind: StateBadgeKind::Brittle,
            text: Some(brittle.to_string()),
            icon: Some(BadgeIcon::Brittle),
            words: brittle_words(words, brittle),
            data: data(&[("data-brittle", brittle.to_string())]),
        });
    }
    // #2437: a hand card's Temporary count, beside the moon's pattern (R1839000).
    if let Some(temporary) = facts.temporary {
        badges.push(StateBadge {
            kind: StateBadgeKind::Temporary,
            text: Some(format!("{TEMPORARY_GLYPH}{temporary}")),
            icon: None,
            words: temporary_words(words, temporary),
            data: data(&[("data-temporary", temporary.to_string())]),
        });
    }
    if let Some(quest) = facts.quest {
        for open in &quest.open {
            badges.push(StateBadge {
                kind: StateBadgeKind::Quest,
                text: Some(quest_progress(open)),
                icon: None,
                words: quest_words(words, open),
                data: data(&[
                    ("data-quest", open.id.clone()),
                    ("data-progress", open.progress.to_string()),
                    ("data-goal", open.goal.to_string()),
                ]),
            });
        }
    }
    let stages: &[QuestStageView] = facts
        .quest
        .and_then(|quest| quest.stages.as_deref())
        .unwrap_or(&[]);
    if let Some((stage, number)) = current_stage(stages) {
        let mut pairs = vec![
            ("data-stage", number.to_string()),
            ("data-stages", stages.len().to_string()),
            ("data-stage-state", stage.state.as_str().to_string()),
        ];
        if let Some(waiting) = stage.hear_back {
            pairs.push(("data-hear-back", waiting.to_string()));
        }
        badges.push(StateBadge {
            kind: StateBadgeKind::QuestStage,
            text: Some(format!("{number}/{}", stages.len())),
            icon: None,
            words: stage_words(words, stages),
            data: data(&pairs),
        });
    }
    if let Some(tuning) = facts.tuning {
        let key = match tuning.verdict {
            TuneVerdict::Upgraded => "upgraded",
            TuneVerdict::Degraded => "degraded",
            TuneVerdict::Tuned => "tuned",
        };
        badges.push(StateBadge {
            kind: StateBadgeKind::Tuned,
            text: Some(tuning.glyph.clone()),
            icon: None,
            words: tuning.summary.clone(),
            data: data(&[
                ("data-tuned", key.to_string()),
                ("data-tuned-word", verdict_word(tuning.verdict).to_string()),
            ]),
        });
    }
    for enchantment in distinct_enchantments(facts.enchantments) {
        let (kind, name) = enchantment_kind(enchantment);
        let mut pairs = vec![("data-enchantment", name.to_string())];
        if let Enchantment::ReturnAfterResolve { floor } = enchantment {
            pairs.push(("data-floor", floor.to_string()));
        }
        badges.push(StateBadge {
            kind,
            text: None,
            icon: Some(enchantment_icon(enchantment)),
            words: enchantment_words(words, enchantment, facts.radiant),
            data: data(&pairs),
        });
    }
    if let Some(animated) = facts.animated {
        badges.push(StateBadge {
            kind: StateBadgeKind::Animated,
            text: None,
            icon: Some(BadgeIcon::Cog),
            words: animated_words(words, animated, facts.type_),
            data: data(&[(
                "data-animated",
                animated.home.map_or("true".to_string(), |home| home.to_string()),
            )]),
        });
    }
    if facts.berserk {
        badges.push(StateBadge {
            kind: StateBadgeKind::Berserk,
            text: None,
            icon: Some(BadgeIcon::Sword),
            words: words.say(BERSERK_WORDS),
            data: data(&[("data-berserk", "true".to_string())]),
        });
    }
    if facts.frozen {
        let line = if facts.frozen_forever {
            FROZEN_FOREVER_WORDS
        } else {
            FROZEN_WORDS
        };
        badges.push(StateBadge {
            kind: StateBadgeKind::Frozen,
            text: Some(FROZEN_GLYPH.to_string()),
            icon: None,
            words: words.say(line),
            data: data(&[(
                "data-frozen",
                if facts.frozen_forever { "forever" } else { "true" }.to_string(),
            )]),
        });
    }
    // R3536005: an Evolved card, its Evolve behind or ahead of it.
    if facts.evolved {
        badges.push(StateBadge {
            kind: StateBadgeKind::Evolved,
            text: Some(EVOLVED_GLYPH.to_string()),
            icon: None,
            words: words.say(EVOLVED_WORDS),
            data: data(&[("data-evolved", "true".to_string())]),
        });
    }
    if let Some(dormant) = facts.dormant {
        badges.push(StateBadge {
            kind: StateBadgeKind::Dormant,
            text: Some(dormant.turns.map_or(DORMANT_GLYPH.to_string(), |turns| {
                format!("{DORMANT_GLYPH}{turns}")
            })),
            icon: None,
            words: dormant_words(words, dormant.turns),
            data: data(&[(
                "data-dormant",
                dormant
                    .turns
                    .map_or("true".to_string(), |turns| turns.to_string()),
            )]),
        });
    }
    if let Some(starship) = facts.starship {
        badges.push(StateBadge {
            kind: StateBadgeKind::Starship,
            text: Some(format!("{STARSHIP_GLYPH}{}", starship.pieces)),
            icon: None,
            words: starship_words(words, starship.pieces, starship.launched),
            data: data(&[
                (
                    "data-starship",
                    if starship.launched { "launched" } else { "dormant" }.to_string(),
                ),
                ("data-starship-pieces", starship.pieces.to_string()),
            ]),
        });
    }
    // MD-B6, R943: a Created card, made during the game. R2647000: only the inspect state notes list it
    // (CardStates.tsx draws it on no face).
    if facts.created {
        badges.push(StateBadge {
            kind: StateBadgeKind::Created,
            text: Some(if facts.chinese {
                created_word().to_string()
            } else {
                "Created".to_string()
            }),
            icon: None,
            words: words.say("Created: made during the game"),
            data: data(&[("data-created", "true".to_string())]),
        });
    }
    badges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::types::Locale;

    fn stage(name: &str, state: QuestStageState, hear_back: Option<i32>) -> QuestStageView {
        QuestStageView {
            name: name.to_string(),
            state,
            goals: Vec::new(),
            reward: String::new(),
            hear_back,
        }
    }

    #[test]
    fn the_stage_a_line_stands_at_is_the_open_one_then_the_one_it_hears_back_with_then_the_last_done() {
        let stages = [
            stage("A", QuestStageState::Done, None),
            stage("B", QuestStageState::Current, None),
            stage("C", QuestStageState::Upcoming, None),
        ];
        assert_eq!(current_stage(&stages).map(|(_, number)| number), Some(2));
        let waiting = [
            stage("A", QuestStageState::Done, None),
            stage("B", QuestStageState::Upcoming, Some(2)),
        ];
        assert_eq!(current_stage(&waiting).map(|(_, number)| number), Some(2));
        let done = [
            stage("A", QuestStageState::Done, None),
            stage("B", QuestStageState::Done, None),
        ];
        assert_eq!(current_stage(&done).map(|(_, number)| number), Some(2));
        assert!(current_stage(&[]).is_none());
    }

    #[test]
    fn a_stage_is_named_with_the_turns_left_before_it_opens() {
        let en = Words::new(Locale::En);
        assert_eq!(
            stage_title(en, 2, 3, "Comradery", None),
            "Stage 2 of 3, Comradery"
        );
        assert_eq!(
            stage_title(en, 2, 3, "Comradery", Some(1)),
            "Stage 2 of 3, Comradery, hearing back next turn"
        );
        assert_eq!(
            stage_title(en, 2, 3, "Comradery", Some(4)),
            "Stage 2 of 3, Comradery, hearing back in 4 turns"
        );
    }

    #[test]
    fn a_temporary_card_says_how_many_ends_of_turn_it_has() {
        let en = Words::new(Locale::En);
        assert_eq!(
            temporary_words(en, 1),
            "Temporary 1: discarded at the end of its owner's turn"
        );
        assert_eq!(
            temporary_words(en, 3),
            "Temporary 3: discarded after 3 of its owner's ends of turn"
        );
    }
}
