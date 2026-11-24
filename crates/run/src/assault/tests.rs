//! The Assault's rules on synthetic content: the deal, the escalation and the attempt
//! ([[R4579000]], [[R4579001]], [[R4579002]]).

use super::*;
use crate::assault::content::content_file;
use crate::assault::day::MONTH_DAYS;
use crate::config::{
    ASSAULT_COMMANDER_HEALTH, ASSAULT_COMMANDER_LEVELS, ASSAULT_DECK_SIZE, ASSAULT_HERO_HEALTH,
    ASSAULT_HISTORY_DAYS, ASSAULT_LEVELS, ASSAULT_UPGRADES,
};
use jackioh_engine::catalog::{deckable, registered_catalog};
use jackioh_engine::config::{HUMAN_HANDICAP, Handicap};
use jackioh_engine::wire::{PrintedRarity, Rarity};

/// The file's doctrines and IA11's counts per group (neutral `[10, 8, 4, 2, 0]`, each doctrine
/// `[9, 7, 5, 2, 1]`) as cards named `ia-<group>-<rarity>-<n>`, with twelve Commanders of
/// IA12.10's ranks and doctrines, two `commander` cards each.
fn set_content() -> AssaultContent {
    let file = content_file();
    let slots = [
        Rarity::Common,
        Rarity::Rare,
        Rarity::Epic,
        Rarity::Legendary,
        Rarity::Mythic,
    ];
    let neutral = [10, 8, 4, 2, 0];
    let doctrinal = [9, 7, 5, 2, 1];
    let mut cards = Vec::new();
    for (at, rarity) in slots.iter().enumerate() {
        for n in 0..neutral[at] {
            cards.push(AssaultCard {
                id: format!("ia-neutral-{}-{n}", rarity.as_str()),
                rarity: *rarity,
                doctrine: None,
                commander: false,
            });
        }
        for doctrine in &file.doctrines {
            for n in 0..doctrinal[at] {
                cards.push(AssaultCard {
                    id: format!("ia-{}-{}-{n}", doctrine.id, rarity.as_str()),
                    rarity: *rarity,
                    doctrine: Some(doctrine.id.clone()),
                    commander: false,
                });
            }
        }
    }
    let ranks = [1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4];
    let doctrines: [Option<&str>; 12] = [
        Some("Hallucination"),
        Some("Overclock"),
        Some("Batch"),
        Some("Recursion"),
        Some("Jailbreak"),
        Some("Firewall"),
        Some("Scraper"),
        Some("Singularity"),
        Some("Recursion"),
        None,
        Some("Overclock"),
        Some("Jailbreak"),
    ];
    let mut commanders = Vec::new();
    for (n, (&rank, doctrine)) in ranks.iter().zip(doctrines.iter()).enumerate() {
        let id = format!("ia-cmd-{n}");
        let unit = format!("{id}-0");
        let signature = format!("{id}-1");
        for card in [&unit, &signature] {
            cards.push(AssaultCard {
                id: card.clone(),
                rarity: Rarity::Legendary,
                doctrine: doctrine.map(str::to_string),
                commander: true,
            });
        }
        commanders.push(AssaultCommander {
            id: id.clone(),
            name: format!("Commander {n}"),
            rank,
            doctrine: doctrine.map(str::to_string),
            cards: vec![unit, signature],
        });
    }
    AssaultContent {
        cards,
        doctrines: file.doctrines,
        commanders,
    }
}

/// Every date of 2027, from `MONTH_DAYS` (2027 is no leap year, so 365 days).
fn year() -> Vec<String> {
    let mut days = Vec::new();
    for (month, &length) in MONTH_DAYS.iter().enumerate() {
        for day in 1..=length {
            days.push(format!("2027-{:02}-{day:02}", month + 1));
        }
    }
    assert_eq!(days.len(), 365, "2027 holds 365 days");
    days
}

/// The stacks of one kind a level holds, 0 when it holds none.
fn held(level: &AssaultLevel, kind: AssaultUpgradeKind) -> i32 {
    level
        .upgrades
        .iter()
        .find(|upgrade| upgrade.kind == kind)
        .map_or(0, |upgrade| upgrade.stacks)
}

/// The per-stack amount of one kind.
fn per(kind: AssaultUpgradeKind) -> i32 {
    ASSAULT_UPGRADES
        .iter()
        .find(|(row, _, _, _, _)| *row == kind)
        .map(|(_, amount, _, _, _)| *amount)
        .expect("every upgrade kind is in ASSAULT_UPGRADES")
}

#[test]
fn day_seed_is_the_prefixed_date_and_refuses_anything_else() {
    for date in ["2026-11-23", "2024-02-29", "2000-02-29"] {
        assert_eq!(
            assault_day_seed(date),
            Ok(format!("assault-{date}")),
            "{date} deals"
        );
    }
    for date in [
        "2026-02-29",
        "1900-02-29",
        "2026-13-01",
        "2026-11-31",
        "2026-11-00",
        "26-11-23",
        "2026/11/23",
        "2026-11-23T00",
        "",
    ] {
        assert!(assault_day_seed(date).is_err(), "{date} is refused");
        assert!(assault_day(date, &set_content()).is_err(), "{date} deals nothing");
    }
}

#[test]
fn the_same_date_deals_the_same_day_byte_for_byte_and_two_dates_differ() {
    let content = set_content();
    let first = assault_day("2026-11-23", &content).unwrap();
    let second = assault_day("2026-11-23", &content).unwrap();
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap(),
        "the same date deals the same day"
    );
    let json = serde_json::to_string(&first).unwrap();
    assert_eq!(
        serde_json::from_str::<AssaultDay>(&json).unwrap(),
        first,
        "the day round-trips"
    );
    let other = assault_day("2026-11-24", &content).unwrap();
    assert_ne!(first, other, "two dates differ");
}

#[test]
fn a_year_of_days_deals_twenty_levels_of_the_right_shape() {
    let content = set_content();
    for date in year() {
        let day = assault_day(&date, &content).unwrap();
        assert_eq!(day.levels.len(), ASSAULT_LEVELS as usize, "{date}");
        for (index, level) in day.levels.iter().enumerate() {
            assert_eq!(level.level, index as u8 + 1, "{date}");
        }
        for level in &day.levels {
            // Commanders sit exactly on 5, 10, 15 and 20, of ranks 1 to 4, and hold their own
            // cards; a normal level names no Commander.
            match level.level {
                5 | 10 | 15 | 20 => {
                    let rank = level.level / 5;
                    let id = level.commander.clone().expect("a Commander level names one");
                    let commander = content
                        .commanders
                        .iter()
                        .find(|commander| commander.id == id)
                        .expect("a Commander of the content");
                    assert_eq!(commander.rank, rank, "{date} level {}", level.level);
                    assert_eq!(level.doctrine, commander.doctrine, "{date}");
                    assert_eq!(level.enemy, commander.name, "{date}");
                    for card in &commander.cards {
                        assert!(level.deck.contains(card), "{date} level {}", level.level);
                    }
                }
                _ => assert!(level.commander.is_none(), "{date} level {}", level.level),
            }
            // No card twice, only content cards, and no other Commander's card.
            let mut seen = Vec::new();
            for id in &level.deck {
                assert!(!seen.contains(&id), "{date} level {}: {id} twice", level.level);
                seen.push(id);
                let card = content
                    .cards
                    .iter()
                    .find(|card| &card.id == id)
                    .unwrap_or_else(|| panic!("{date}: {id} is a card of the content"));
                if card.commander {
                    let own = level.commander.clone().expect("a Commander's own card");
                    let commander = content
                        .commanders
                        .iter()
                        .find(|commander| commander.id == own)
                        .expect("a Commander of the content");
                    assert!(
                        commander.cards.contains(id),
                        "{date} level {} holds no other Commander's card",
                        level.level
                    );
                }
            }
            // The deck's length is its handicap's size and the sum of its slots.
            assert_eq!(
                level.deck.len() as i32,
                level.handicap.deck_size,
                "{date} level {}",
                level.level
            );
            assert_eq!(
                level.rarities.iter().sum::<i32>(),
                level.deck.len() as i32,
                "{date} level {}",
                level.level
            );
            // No card outside the Commander's is rarer than the ceiling.
            let ceiling = ceiling_at(level.level);
            for id in &level.deck {
                let card = content.cards.iter().find(|card| &card.id == id).unwrap();
                if card.commander {
                    continue;
                }
                let slot = slot_of(card.rarity).expect("{id} is no token");
                assert!(slot <= ceiling, "{date} level {}: {id}", level.level);
            }
            // The marks number stacks times per stack.
            let radiant = level.marks.iter().filter(|mark| mark.radiant).count() as i32;
            assert_eq!(
                radiant,
                (held(level, AssaultUpgradeKind::Radiant) * per(AssaultUpgradeKind::Radiant))
                    .min(level.deck.len() as i32),
                "{date} level {}",
                level.level
            );
            let buffed = level.marks.iter().filter(|mark| mark.upgrades > 0).count() as i32;
            assert_eq!(
                buffed,
                (held(level, AssaultUpgradeKind::Buffed) * per(AssaultUpgradeKind::Buffed))
                    .min(level.deck.len() as i32),
                "{date} level {}",
                level.level
            );
        }
        // No two consecutive Some doctrines are equal, and no two levels' sorted decks are.
        for pair in day.levels.windows(2) {
            if let (Some(before), Some(after)) = (&pair[0].doctrine, &pair[1].doctrine) {
                assert_ne!(
                    before, after,
                    "{date}: levels {} and {}",
                    pair[0].level, pair[1].level
                );
            }
        }
        let mut decks: Vec<Vec<&str>> = day
            .levels
            .iter()
            .map(|level| {
                let mut deck: Vec<&str> = level.deck.iter().map(String::as_str).collect();
                deck.sort_unstable();
                deck
            })
            .collect();
        decks.sort();
        decks.dedup();
        assert_eq!(decks.len(), day.levels.len(), "{date}: every deck is distinct");
    }
}

#[test]
fn each_normal_level_adds_one_escalation_and_a_commander_a_card_step_and_an_upgrade() {
    let content = set_content();
    for date in year() {
        let day = assault_day(&date, &content).unwrap();
        for (index, level) in day.levels.iter().enumerate() {
            if level.level == 1 {
                assert!(level.escalations.is_empty(), "{date}");
            } else if commander_rank(level.level).is_some() {
                assert_eq!(
                    level.escalations,
                    vec![
                        AssaultEscalation::Cards,
                        AssaultEscalation::Upgrade {
                            upgrade: level
                                .escalations
                                .iter()
                                .find_map(|escalation| match escalation {
                                    AssaultEscalation::Upgrade { upgrade } => Some(*upgrade),
                                    AssaultEscalation::Cards => None,
                                })
                                .expect("a Commander level takes an upgrade"),
                        }
                    ],
                    "{date} level {}",
                    level.level
                );
            } else {
                assert_eq!(level.escalations.len(), 1, "{date} level {}", level.level);
            }
            // Each level's upgrades are the previous level's plus this level's.
            if index > 0 {
                let before = &day.levels[index - 1];
                for (kind, _, _, _, _) in ASSAULT_UPGRADES {
                    let added = level.escalations.iter().any(|escalation| {
                        matches!(escalation, AssaultEscalation::Upgrade { upgrade } if *upgrade == kind)
                    });
                    assert_eq!(
                        held(level, kind),
                        held(before, kind) + i32::from(added),
                        "{date} level {} {kind:?}",
                        level.level
                    );
                }
            }
        }
    }
}

#[test]
fn r4579000_nothing_falls_from_one_level_to_the_next_over_a_year() {
    let content = set_content();
    let health_above = |level: &AssaultLevel| level.handicap.hero_health.expect("a level sets its health");
    for date in year() {
        let day = assault_day(&date, &content).unwrap();
        for pair in day.levels.windows(2) {
            let (before, after) = (&pair[0], &pair[1]);
            let tag = format!("{date}: levels {} and {}", before.level, after.level);
            // No handicap field other than health and fewer opening cards falls.
            assert!(
                after.handicap.extra_opening_cards >= before.handicap.extra_opening_cards,
                "{tag}"
            );
            assert!(after.handicap.deck_size >= before.handicap.deck_size, "{tag}");
            assert!(after.handicap.mana_bonus >= before.handicap.mana_bonus, "{tag}");
            assert!(after.handicap.mana_cap >= before.handicap.mana_cap, "{tag}");
            assert!(
                after.handicap.extra_draws_per_turn >= before.handicap.extra_draws_per_turn,
                "{tag}"
            );
            assert!(
                after.handicap.hero_armor.unwrap_or(0) >= before.handicap.hero_armor.unwrap_or(0),
                "{tag}"
            );
            // Fewer opening cards never rise.
            assert!(
                after.handicap.fewer_opening_cards.unwrap_or(0)
                    <= before.handicap.fewer_opening_cards.unwrap_or(0),
                "{tag}"
            );
            // Health minus its base never falls.
            assert!(
                health_above(after) - health_base(after) >= health_above(before) - health_base(before),
                "{tag}"
            );
            // No upgrade's stacks fall.
            for (kind, _, _, _, _) in ASSAULT_UPGRADES {
                assert!(held(after, kind) >= held(before, kind), "{tag} {kind:?}");
            }
            // The count of slots at or above each rarity never falls.
            for rarity in 0..5 {
                let at = |level: &AssaultLevel| level.rarities[rarity..].iter().sum::<i32>();
                assert!(at(after) >= at(before), "{tag} rarity {rarity}");
            }
        }
        // Health rises from each normal level to the next normal level, and from each Commander
        // level to the next.
        let normals: Vec<&AssaultLevel> = day
            .levels
            .iter()
            .filter(|level| commander_rank(level.level).is_none())
            .collect();
        for pair in normals.windows(2) {
            if pair[1].level == pair[0].level + 1 {
                assert!(
                    pair[1].handicap.hero_health > pair[0].handicap.hero_health,
                    "{date}: levels {} and {}",
                    pair[0].level,
                    pair[1].level
                );
            }
        }
        let bosses: Vec<&AssaultLevel> = day
            .levels
            .iter()
            .filter(|level| commander_rank(level.level).is_some())
            .collect();
        for pair in bosses.windows(2) {
            assert!(
                pair[1].handicap.hero_health > pair[0].handicap.hero_health,
                "{date}: levels {} and {}",
                pair[0].level,
                pair[1].level
            );
        }
    }
}

/// A level's health base: a Commander's is a peak, the next level goes back to the normal base
/// and keeps every Redundancy stack ([[R4579000]]).
fn health_base(level: &AssaultLevel) -> i32 {
    let (base, each) = if commander_rank(level.level).is_some() {
        ASSAULT_COMMANDER_HEALTH
    } else {
        ASSAULT_HERO_HEALTH
    };
    base + each * i32::from(level.level)
}

#[test]
fn r4579001_a_slot_falls_back_to_the_neutral_cards_any_doctrine_then_a_rarity_lower() {
    // A content whose Epic and rarer cards are all neutral deals every level in full.
    let mut content = set_content();
    for card in content.cards.iter_mut() {
        if matches!(card.rarity, Rarity::Epic | Rarity::Legendary | Rarity::Mythic) && !card.commander {
            card.doctrine = None;
        }
    }
    for date in year() {
        let day = assault_day(&date, &content).unwrap();
        assert_eq!(day.levels.len(), ASSAULT_LEVELS as usize, "{date}");
    }
    // So does a content of Commons alone.
    let commanders = set_content().commanders;
    let mut commons = AssaultContent {
        cards: Vec::new(),
        doctrines: content.doctrines.clone(),
        commanders: commanders.clone(),
    };
    let groups: Vec<Option<String>> = std::iter::once(None)
        .chain(content.doctrines.iter().map(|row| Some(row.id.clone())))
        .collect();
    for (n, doctrine) in groups.into_iter().enumerate() {
        for m in 0..30 {
            commons.cards.push(AssaultCard {
                id: format!("common-{n}-{m}"),
                rarity: Rarity::Common,
                doctrine: doctrine.clone(),
                commander: false,
            });
        }
    }
    for commander in &commanders {
        for id in &commander.cards {
            commons.cards.push(AssaultCard {
                id: id.clone(),
                rarity: Rarity::Legendary,
                doctrine: commander.doctrine.clone(),
                commander: true,
            });
        }
    }
    for date in year() {
        let day = assault_day(&date, &commons).unwrap();
        assert_eq!(day.levels.len(), ASSAULT_LEVELS as usize, "{date}");
    }
    // A content with too few cards is refused, and so is one with no rank-2 Commander.
    let mut few = set_content();
    few.cards
        .retain(|card| card.commander || card.rarity == Rarity::Common);
    few.cards.truncate(10);
    for commander in &few.commanders {
        for id in &commander.cards {
            if !few.cards.iter().any(|card| &card.id == id) {
                few.cards.push(AssaultCard {
                    id: id.clone(),
                    rarity: Rarity::Legendary,
                    doctrine: commander.doctrine.clone(),
                    commander: true,
                });
            }
        }
    }
    assert!(
        assault_day("2027-03-04", &few).is_err(),
        "too few cards fill no deck"
    );
    let mut norank = set_content();
    norank.commanders.retain(|commander| commander.rank != 2);
    assert!(
        assault_day("2027-03-04", &norank).is_err(),
        "no rank-2 Commander deals no day"
    );
}

/// The first 25 `deckable` cards of the real catalog: the attempt tests' human deck.
fn human_deck() -> Vec<String> {
    jackioh_cards::register_all();
    registered_catalog()
        .values()
        .filter(|def| deckable(def))
        .take(25)
        .map(|def| def.id.clone())
        .collect()
}

fn record_for(date: &str, attempts: u32) -> AssaultRecord {
    AssaultRecord {
        date: date.into(),
        attempts,
        ..Default::default()
    }
}

fn setup_for(date: &str, attempt: u32, start_level: u8) -> AssaultSetup {
    AssaultSetup {
        date: date.into(),
        deck: human_deck(),
        source: AssaultDeckSource::Saved,
        start_level,
        attempt,
    }
}

fn day_for(content: &AssaultContent, date: &str) -> AssaultDay {
    assault_day(date, content).expect("the synthetic content deals")
}

#[test]
fn r4579002_a_start_at_a_checkpoint_the_record_lacks_or_a_wrong_attempt_number_is_refused() {
    let content = set_content();
    let date = "2027-05-06";
    let day = day_for(&content, date);
    // A fresh record starts level 1's first attempt.
    let state = assault_create(&setup_for(date, 1, 1), &record_for(date, 0), &day).unwrap();
    assert_eq!(state.level, 1);
    assert_eq!(state.phase, AssaultPhase::Briefing);
    // A wrong attempt number is refused.
    assert!(
        assault_create(&setup_for(date, 2, 1), &record_for(date, 0), &day).is_err(),
        "attempt 2 follows no attempt"
    );
    // A checkpoint the record lacks is refused.
    assert!(
        assault_create(&setup_for(date, 1, 6), &record_for(date, 0), &day).is_err(),
        "level 6 is no checkpoint reached today"
    );
    // A checkpoint the record holds starts.
    let mut record = record_for(date, 0);
    record.checkpoints = vec![6];
    let state = assault_create(&setup_for(date, 1, 6), &record, &day).unwrap();
    assert_eq!(state.level, 6);
    // A level that is no checkpoint is refused even beside one held.
    assert!(
        assault_create(&setup_for(date, 1, 7), &record, &day).is_err(),
        "level 7 is no checkpoint"
    );
    // Another day's setup and record are refused.
    assert!(
        assault_create(&setup_for("2027-05-07", 1, 1), &record_for(date, 0), &day).is_err(),
        "another day's setup"
    );
    assert!(
        assault_create(&setup_for(date, 1, 1), &record_for("2027-05-07", 0), &day).is_err(),
        "another day's record"
    );
}

#[test]
fn a_step_in_the_wrong_phase_is_refused_with_the_state_unchanged() {
    let content = set_content();
    let date = "2027-05-06";
    let day = day_for(&content, date);
    let state = assault_create(&setup_for(date, 1, 1), &record_for(date, 0), &day).unwrap();
    // A result while briefing is refused.
    let before = state.clone();
    assert!(assault_reduce(&state, &day, &AssaultAction::FightOver { won: true, turns: 3 }).is_err());
    assert_eq!(state, before, "a refusal changes nothing");
    // A second fight while fighting is refused.
    let fighting = assault_reduce(&state, &day, &AssaultAction::Fight).unwrap();
    let before = fighting.clone();
    assert!(assault_reduce(&fighting, &day, &AssaultAction::Fight).is_err());
    assert_eq!(fighting, before, "a refusal changes nothing");
    // Abandoning ends the attempt, and every step after is refused.
    let left = assault_reduce(&fighting, &day, &AssaultAction::Abandon).unwrap();
    assert_eq!(left.phase, AssaultPhase::Abandoned);
    for action in [
        AssaultAction::Fight,
        AssaultAction::FightOver { won: true, turns: 1 },
        AssaultAction::Abandon,
    ] {
        assert_eq!(
            assault_reduce(&left, &day, &action),
            Err("the attempt is over".to_string()),
            "{action:?}"
        );
    }
    // Another day's state is refused.
    let other = day_for(&content, "2027-05-07");
    assert!(assault_reduce(&state, &other, &AssaultAction::Fight).is_err());
}

#[test]
fn every_step_of_an_ended_attempt_is_refused() {
    let content = set_content();
    let date = "2027-05-06";
    let day = day_for(&content, date);
    let start = assault_create(&setup_for(date, 1, 1), &record_for(date, 0), &day).unwrap();
    let fighting = assault_reduce(&start, &day, &AssaultAction::Fight).unwrap();
    // A loss ends the attempt lost.
    let lost = assault_reduce(
        &fighting,
        &day,
        &AssaultAction::FightOver { won: false, turns: 4 },
    )
    .unwrap();
    assert_eq!(lost.phase, AssaultPhase::Lost);
    for action in [
        AssaultAction::Fight,
        AssaultAction::FightOver { won: true, turns: 1 },
        AssaultAction::Abandon,
    ] {
        assert_eq!(
            assault_reduce(&lost, &day, &action),
            Err("the attempt is over".to_string()),
            "{action:?}"
        );
    }
}

#[test]
fn a_win_moves_on_a_loss_ends_and_level_twenty_repels() {
    let content = set_content();
    let date = "2027-05-06";
    let day = day_for(&content, date);
    let start = assault_create(&setup_for(date, 1, 1), &record_for(date, 0), &day).unwrap();
    // A win moves to the next level's briefing and counts its turns.
    let fighting = assault_reduce(&start, &day, &AssaultAction::Fight).unwrap();
    assert_eq!(fighting.phase, AssaultPhase::Fight);
    let won = assault_reduce(&fighting, &day, &AssaultAction::FightOver { won: true, turns: 5 }).unwrap();
    assert_eq!(won.level, 2);
    assert_eq!(won.phase, AssaultPhase::Briefing);
    assert_eq!(won.beaten, vec![1]);
    assert_eq!(won.turns, 5);
    // A negative turn count is refused.
    assert!(
        assault_reduce(
            &assault_reduce(&won, &day, &AssaultAction::Fight).unwrap(),
            &day,
            &AssaultAction::FightOver { won: true, turns: -1 }
        )
        .is_err()
    );
    // Beating level 20 repels the Assault.
    let mut record = record_for(date, 0);
    record.checkpoints = vec![16];
    let mut state = assault_create(&setup_for(date, 1, 16), &record, &day).unwrap();
    for level in 16..=ASSAULT_LEVELS {
        assert_eq!(state.level, level);
        state = assault_reduce(&state, &day, &AssaultAction::Fight).unwrap();
        state = assault_reduce(&state, &day, &AssaultAction::FightOver { won: true, turns: 2 }).unwrap();
    }
    assert_eq!(state.phase, AssaultPhase::Repelled);
    assert_eq!(state.beaten, vec![16, 17, 18, 19, 20]);
    assert_eq!(state.turns, 10);
}

#[test]
fn the_record_takes_attempts_best_checkpoints_and_the_first_repel_idempotently() {
    let content = set_content();
    let date = "2027-05-06";
    let day = day_for(&content, date);
    let record = record_for(date, 0);
    // A beaten Commander is its next level's checkpoint.
    let mut state = assault_create(&setup_for(date, 1, 1), &record, &day).unwrap();
    for _ in 1..=5 {
        state = assault_reduce(&state, &day, &AssaultAction::Fight).unwrap();
        state = assault_reduce(&state, &day, &AssaultAction::FightOver { won: true, turns: 1 }).unwrap();
    }
    assert_eq!(state.beaten, vec![1, 2, 3, 4, 5]);
    let folded = assault_record(&record, &state);
    assert_eq!(folded.attempts, 1);
    assert_eq!(folded.best, 5);
    assert_eq!(folded.checkpoints, vec![6]);
    assert_eq!(folded.repelled_on, None);
    // Folding the same state twice reads the same record.
    assert_eq!(assault_record(&folded, &state), folded);
    // A later attempt keeps the larger number and the best level, and the first repel.
    let mut running = assault_create(&setup_for(date, 2, 1), &folded, &day).unwrap();
    for _ in 1..=3 {
        running = assault_reduce(&running, &day, &AssaultAction::Fight).unwrap();
        running = assault_reduce(&running, &day, &AssaultAction::FightOver { won: true, turns: 1 }).unwrap();
    }
    let next = assault_record(&folded, &running);
    assert_eq!(next.attempts, 2);
    assert_eq!(next.best, 5);
    assert_eq!(next.checkpoints, vec![6]);
    // The smaller repelling attempt wins. Level 16 needs its checkpoint.
    let mut held = next.clone();
    held.checkpoints = vec![6, 16];
    let mut repelled = assault_create(&setup_for(date, 3, 16), &held, &day).unwrap();
    for _ in 16..=ASSAULT_LEVELS {
        repelled = assault_reduce(&repelled, &day, &AssaultAction::Fight).unwrap();
        repelled =
            assault_reduce(&repelled, &day, &AssaultAction::FightOver { won: true, turns: 1 }).unwrap();
    }
    assert_eq!(repelled.phase, AssaultPhase::Repelled);
    let first = assault_record(&held, &repelled);
    assert_eq!(first.repelled_on, Some(3));
    assert_eq!(first.checkpoints, vec![6, 16]);
    let again = assault_record(&first, &repelled);
    assert_eq!(again, first, "the record is idempotent");
    let earlier = AssaultRecord {
        repelled_on: Some(2),
        ..first.clone()
    };
    assert_eq!(
        assault_record(&earlier, &repelled).repelled_on,
        Some(2),
        "the first repel stands"
    );
    // Another day's state leaves the record alone.
    let other = day_for(&content, "2027-05-07");
    let foreign = assault_create(
        &setup_for("2027-05-07", 1, 1),
        &record_for("2027-05-07", 0),
        &other,
    )
    .unwrap();
    assert_eq!(assault_record(&first, &foreign), first);
}

#[test]
fn an_attempt_file_folds_back_to_its_state_and_names_a_refused_step() {
    let content = set_content();
    let date = "2027-05-06";
    let day = day_for(&content, date);
    let setup = setup_for(date, 1, 1);
    let record = record_for(date, 0);
    let file = AssaultFile {
        version: "test".into(),
        setup: setup.clone(),
        record: record.clone(),
        actions: vec![
            AssaultAction::Fight,
            AssaultAction::FightOver { won: true, turns: 3 },
            AssaultAction::Fight,
        ],
    };
    let mut state = assault_create(&setup, &record, &day).unwrap();
    for action in &file.actions {
        state = assault_reduce(&state, &day, action).unwrap();
    }
    assert_eq!(assault_fold(&file, &day), Ok(state));
    // A file whose second step is refused names it.
    let refused = AssaultFile {
        version: "test".into(),
        setup,
        record,
        actions: vec![AssaultAction::Fight, AssaultAction::Fight],
    };
    let error = assault_fold(&refused, &day).expect_err("the second fight is refused");
    assert!(error.starts_with("step 1: "), "{error} names its step");
}

#[test]
fn the_fight_setup_answers_only_in_a_fight_with_its_seed_decks_and_handicaps() {
    let content = set_content();
    let date = "2027-05-06";
    let day = day_for(&content, date);
    let setup = setup_for(date, 1, 1);
    let record = record_for(date, 0);
    let briefing = assault_create(&setup, &record, &day).unwrap();
    assert!(
        assault_fight_setup(&briefing, &day).is_none(),
        "no setup while briefing"
    );
    let fighting = assault_reduce(&briefing, &day, &AssaultAction::Fight).unwrap();
    let answered = assault_fight_setup(&fighting, &day).expect("a setup while fighting");
    assert_eq!(answered.seed, format!("{}-L1-A1", day.seed));
    assert_eq!(answered.human_deck, setup.deck);
    assert_eq!(
        answered.human_handicap,
        Handicap {
            deck_size: ASSAULT_DECK_SIZE,
            ..HUMAN_HANDICAP
        }
    );
    assert_eq!(answered.enemy_deck, day.levels[0].deck);
    assert_eq!(answered.enemy_handicap, day.levels[0].handicap);
    assert_eq!(answered.enemy_marks, day.levels[0].marks);
    assert_eq!(answered.level, day.levels[0]);
    // Another day answers nothing, nor does a level it does not hold.
    let other = day_for(&content, "2027-05-07");
    assert!(assault_fight_setup(&fighting, &other).is_none());
    let nowhere = AssaultState {
        level: 99,
        ..fighting.clone()
    };
    assert!(assault_fight_setup(&nowhere, &day).is_none());
}

#[test]
fn the_constants_are_the_configs() {
    assert_eq!(
        assault_constants(),
        AssaultConstants {
            levels: ASSAULT_LEVELS,
            commander_levels: ASSAULT_COMMANDER_LEVELS.to_vec(),
            checkpoints: checkpoints(),
            history_days: ASSAULT_HISTORY_DAYS,
        }
    );
    assert_eq!(checkpoints(), vec![6, 11, 16]);
    assert_eq!(ASSAULT_COMMANDER_LEVELS.len(), 4);
    assert!(PrintedRarity::ALL.len() == 5, "five slots");
}
