//! IA1 and IA2: the day's seed and its deal ([[R4579000]], [[R4579001]]). One `Rng` deals the
//! whole day in level order, so the same date and content deal the same day byte for byte.

use crate::assault::{
    AssaultCommander, AssaultContent, AssaultDay, AssaultEscalation, AssaultLevel, AssaultUpgrade,
    AssaultUpgradeKind, ceiling_at, commander_rank, slot_of,
};
use crate::config::{
    ASSAULT_BASE_HANDICAP, ASSAULT_CARD_STEP, ASSAULT_CARD_STEP_PERCENT, ASSAULT_COMMANDER_HEALTH,
    ASSAULT_COMMANDER_LEVELS, ASSAULT_DECK_SIZE, ASSAULT_DOCTRINE_SHARE, ASSAULT_FEWER_OPENING,
    ASSAULT_FEWER_OPENING_LEVELS, ASSAULT_HERO_HEALTH, ASSAULT_LEVELS, ASSAULT_SEED_PREFIX,
    ASSAULT_START_SLOTS, ASSAULT_UPGRADES,
};
use jackioh_engine::config::PERCENT;
use jackioh_engine::rng::Rng;

/// IA1: the shape a day is written in.
const DATE_SHAPE: &[u8] = b"YYYY-MM-DD";
/// IA1: the days of each month, February in a common year (`tests` counts the year from it).
pub(super) const MONTH_DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
/// IA1: February's month.
const FEBRUARY: u32 = 2;
/// IA1: leap years are divisible by 4, except centuries not divisible by 400.
const LEAP: (u32, u32, u32) = (4, 100, 400);

/// IA1: the day's seed: `assault-YYYY-MM-DD`. Any other date is refused, and only a real Gregorian
/// date deals ([[R4579002]]).
pub fn assault_day_seed(date: &str) -> Result<String, String> {
    let refused = format!("{date:?} is not a day: write it YYYY-MM-DD");
    if date.len() != DATE_SHAPE.len() {
        return Err(refused);
    }
    for (at, byte) in date.bytes().enumerate() {
        let shaped = DATE_SHAPE[at] == b'-';
        if shaped != (byte == b'-') || (!shaped && !byte.is_ascii_digit()) {
            return Err(refused);
        }
    }
    let number = |from: usize, to: usize| date[from..to].parse::<u32>().map_err(|_| refused.clone());
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let leap = year % LEAP.0 == 0 && (year % LEAP.1 != 0 || year % LEAP.2 == 0);
    let days = month
        .checked_sub(1)
        .and_then(|at| MONTH_DAYS.get(at as usize).copied())
        .unwrap_or(0)
        + u32::from(month == FEBRUARY && leap);
    if month == 0 || day == 0 || day > days {
        return Err(refused);
    }
    Ok(format!("{ASSAULT_SEED_PREFIX}{date}"))
}

/// The `ASSAULT_UPGRADES` row of one kind: `(per stack, most stacks, from level, weight)`.
fn upgrade_row(kind: AssaultUpgradeKind) -> (i32, i32, u8, i32) {
    let (_, per, most, from, weight) = ASSAULT_UPGRADES
        .iter()
        .find(|(row, _, _, _, _)| *row == kind)
        .expect("every upgrade kind is in ASSAULT_UPGRADES");
    (*per, *most, *from, *weight)
}

/// The stacks one kind holds.
fn stacks_of(stacks: &[i32; 9], kind: AssaultUpgradeKind) -> i32 {
    let at = ASSAULT_UPGRADES
        .iter()
        .position(|(row, _, _, _, _)| *row == kind)
        .expect("every upgrade kind is in ASSAULT_UPGRADES");
    stacks[at]
}

/// IA2: a card step: up to `ASSAULT_CARD_STEP` slots promoted one rarity, lowest first, never past
/// the level's ceiling. True when any slot moved.
fn card_step(slots: &mut [i32; 5], level: u8) -> bool {
    let mut moved = false;
    for _ in 0..ASSAULT_CARD_STEP {
        let Some(below) = (0..ceiling_at(level)).find(|&rarity| slots[rarity] > 0) else {
            break;
        };
        slots[below] -= 1;
        slots[below + 1] += 1;
        moved = true;
    }
    moved
}

/// IA3: one stack of one upgrade, drawn by weight among the rows allowed at this level that hold
/// fewer than their most stacks. Bigger Dataset also grows the slots of the rarity holding the
/// most, the lower on a tie. None when no upgrade is left.
fn upgrade_draw(
    slots: &mut [i32; 5],
    stacks: &mut [i32; 9],
    level: u8,
    rng: &mut Rng,
) -> Option<AssaultUpgradeKind> {
    let rows: Vec<(usize, i32)> = ASSAULT_UPGRADES
        .iter()
        .enumerate()
        .filter(|(at, (_, _, most, from, _))| level >= *from && stacks[*at] < *most)
        .map(|(at, (_, _, _, _, weight))| (at, *weight))
        .collect();
    if rows.is_empty() {
        return None;
    }
    let at = crate::weighted(&rows, rng);
    stacks[at] += 1;
    let (kind, per, _, _, _) = ASSAULT_UPGRADES[at];
    if kind == AssaultUpgradeKind::DeckSize {
        let mut richest = 0;
        for rarity in 1..slots.len() {
            if slots[rarity] > slots[richest] {
                richest = rarity;
            }
        }
        slots[richest] += per;
    }
    Some(kind)
}

/// IA2: one escalation, trying the card step first when `card_first` and falling back to the
/// other kind. Nothing when neither can happen.
fn escalate(
    slots: &mut [i32; 5],
    stacks: &mut [i32; 9],
    level: u8,
    card_first: bool,
    rng: &mut Rng,
) -> Option<AssaultEscalation> {
    if card_first {
        if card_step(slots, level) {
            return Some(AssaultEscalation::Cards);
        }
        return upgrade_draw(slots, stacks, level, rng).map(|upgrade| AssaultEscalation::Upgrade { upgrade });
    }
    if let Some(upgrade) = upgrade_draw(slots, stacks, level, rng) {
        return Some(AssaultEscalation::Upgrade { upgrade });
    }
    if card_step(slots, level) {
        return Some(AssaultEscalation::Cards);
    }
    None
}

/// IA4: the index of one doctrine's cards in the buckets.
fn doctrine_at(content: &AssaultContent, doctrine: &str) -> Option<usize> {
    content.doctrines.iter().position(|row| row.id == doctrine)
}

/// One card of `buckets[rarity]` not taken yet, drawn by the seed.
fn draw_from(buckets: &[Vec<usize>], rarity: usize, taken: &[bool], rng: &mut Rng) -> Option<usize> {
    let open: Vec<usize> = buckets[rarity].iter().copied().filter(|at| !taken[*at]).collect();
    rng.pick(&open).copied()
}

/// IA1, IA2, IA4: the day's deal: a pure function of the date and the content ([[R4579000]],
/// [[R4579001]]). One `Rng` draws in level order: each rank's Commander first, then each level's
/// escalations, doctrine, enemy, handicap, deck and marks.
pub fn assault_day(date: &str, content: &AssaultContent) -> Result<AssaultDay, String> {
    let seed = assault_day_seed(date)?;
    let mut rng = Rng::new(&seed, 0);

    // Each rank's Commander, drawn first so a normal level keeps clear of its neighbours'.
    let mut commanders = Vec::new();
    for rank in 1..=ASSAULT_COMMANDER_LEVELS.len() as u8 {
        let drawn: Vec<&AssaultCommander> = content
            .commanders
            .iter()
            .filter(|commander| commander.rank == rank)
            .collect();
        let Some(commander) = rng.pick(&drawn).copied() else {
            return Err(format!("the content holds no rank {rank} Commander"));
        };
        commanders.push(commander.clone());
    }

    // Every card that is no Commander's and no token, once per day: each doctrine's, the
    // neutral cards', and every doctrine's, each split by slot.
    let mut by_doctrine: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); 5]; content.doctrines.len()];
    let mut neutral: Vec<Vec<usize>> = vec![Vec::new(); 5];
    let mut any: Vec<Vec<usize>> = vec![Vec::new(); 5];
    for (at, card) in content.cards.iter().enumerate() {
        if card.commander {
            continue;
        }
        let Some(slot) = slot_of(card.rarity) else {
            continue;
        };
        any[slot].push(at);
        match &card.doctrine {
            Some(doctrine) => match doctrine_at(content, doctrine) {
                Some(index) => by_doctrine[index][slot].push(at),
                None => neutral[slot].push(at),
            },
            None => neutral[slot].push(at),
        }
    }

    let mut slots = ASSAULT_START_SLOTS;
    let mut stacks = [0; 9];
    let mut levels = Vec::new();
    for level in 1..=ASSAULT_LEVELS {
        // a. Escalations.
        let mut escalations = Vec::new();
        if level == 1 {
            // The base row alone.
        } else if commander_rank(level).is_some() {
            // A Commander level takes a card step and an upgrade.
            if let Some(escalation) = escalate(&mut slots, &mut stacks, level, true, &mut rng) {
                escalations.push(escalation);
            }
            if let Some(escalation) = escalate(&mut slots, &mut stacks, level, false, &mut rng) {
                escalations.push(escalation);
            }
        } else if let Some(escalation) = escalate(
            &mut slots,
            &mut stacks,
            level,
            rng.int(PERCENT) < ASSAULT_CARD_STEP_PERCENT,
            &mut rng,
        ) {
            escalations.push(escalation);
        }

        // b. Doctrine: a Commander level takes its Commander's, anywhere else the seed draws one
        // that is neither the previous level's nor the next level's Commander's.
        let rank = commander_rank(level);
        let doctrine = match rank {
            Some(index) => commanders[index].doctrine.clone(),
            None => {
                let previous = levels
                    .last()
                    .and_then(|before: &AssaultLevel| before.doctrine.clone());
                let next = commander_rank(level + 1).and_then(|index| commanders[index].doctrine.clone());
                let open: Vec<&str> = content
                    .doctrines
                    .iter()
                    .map(|row| row.id.as_str())
                    .filter(|id| previous.as_deref() != Some(*id) && next.as_deref() != Some(*id))
                    .collect();
                let Some(drawn) = rng.pick(&open).copied() else {
                    return Err(format!("the content holds no doctrine for level {level}"));
                };
                Some(drawn.to_string())
            }
        };

        // c. Enemy: the Commander's name, or else one of its doctrine's names.
        let enemy = match rank {
            Some(index) => commanders[index].name.clone(),
            None => {
                let id = doctrine.clone().expect("a normal level draws a doctrine");
                let row = content
                    .doctrines
                    .iter()
                    .find(|row| row.id == id)
                    .expect("a drawn doctrine is one of the file's");
                let Some(drawn) = rng.pick(&row.enemies).cloned() else {
                    return Err(format!("doctrine {id} names no enemy"));
                };
                drawn
            }
        };

        // d. Handicap: Easy's resources, then every held stack, then the level's health.
        let mut handicap = ASSAULT_BASE_HANDICAP;
        handicap.extra_opening_cards += stacks_of(&stacks, AssaultUpgradeKind::OpeningCards)
            * upgrade_row(AssaultUpgradeKind::OpeningCards).0;
        handicap.deck_size = ASSAULT_DECK_SIZE
            + stacks_of(&stacks, AssaultUpgradeKind::DeckSize) * upgrade_row(AssaultUpgradeKind::DeckSize).0;
        handicap.mana_bonus +=
            stacks_of(&stacks, AssaultUpgradeKind::ManaBonus) * upgrade_row(AssaultUpgradeKind::ManaBonus).0;
        handicap.mana_cap +=
            stacks_of(&stacks, AssaultUpgradeKind::ManaCap) * upgrade_row(AssaultUpgradeKind::ManaCap).0;
        handicap.extra_draws_per_turn +=
            stacks_of(&stacks, AssaultUpgradeKind::ExtraDraw) * upgrade_row(AssaultUpgradeKind::ExtraDraw).0;
        let armor =
            stacks_of(&stacks, AssaultUpgradeKind::HeroArmor) * upgrade_row(AssaultUpgradeKind::HeroArmor).0;
        if armor > 0 {
            handicap.hero_armor = Some(armor);
        }
        let (base, per) = if rank.is_some() {
            ASSAULT_COMMANDER_HEALTH
        } else {
            ASSAULT_HERO_HEALTH
        };
        handicap.hero_health = Some(
            base + per * i32::from(level)
                + stacks_of(&stacks, AssaultUpgradeKind::HeroHealth)
                    * upgrade_row(AssaultUpgradeKind::HeroHealth).0,
        );
        if level <= ASSAULT_FEWER_OPENING_LEVELS {
            handicap.fewer_opening_cards = Some(ASSAULT_FEWER_OPENING);
        }

        // e. Deck: the Commander's cards first, each off the lowest slot holding one, then the
        // remaining slots, rarest first, `ASSAULT_DOCTRINE_SHARE` percent from the doctrine.
        let rarities = slots;
        let mut remaining = slots;
        let mut deck = Vec::new();
        let mut taken = vec![false; content.cards.len()];
        let own: &[String] = match rank {
            Some(index) => &commanders[index].cards,
            None => &[],
        };
        for id in own {
            let Some(at) = content.cards.iter().position(|card| &card.id == id) else {
                return Err(format!("commander {id} is no card of the content"));
            };
            let Some(below) = (0..remaining.len()).find(|&rarity| remaining[rarity] > 0) else {
                return Err(format!("the content cannot fill level {level}'s deck"));
            };
            remaining[below] -= 1;
            deck.push(id.clone());
            taken[at] = true;
        }
        let mut order = Vec::new();
        for rarity in (0..remaining.len()).rev() {
            for _ in 0..remaining[rarity] {
                order.push(rarity);
            }
        }
        let share = (order.len() as i32 * ASSAULT_DOCTRINE_SHARE / PERCENT).max(0) as usize;
        let drawn = rng.shuffle(&(0..order.len()).collect::<Vec<_>>());
        let doctrine_positions: Vec<bool> = (0..order.len())
            .map(|position| drawn[..share.min(drawn.len())].contains(&position))
            .collect();
        for (position, &rarity) in order.iter().enumerate() {
            let mut filled = None;
            for below in (0..=rarity).rev() {
                // A doctrine slot reads its doctrine's bucket (any doctrine's when the
                // doctrine is none), then the neutral cards, then any doctrine; any other
                // slot reads the neutral cards, then any doctrine.
                let first: &[Vec<usize>] = if doctrine_positions[position] {
                    match &doctrine {
                        Some(id) => match doctrine_at(content, id) {
                            Some(index) => &by_doctrine[index],
                            None => &any,
                        },
                        None => &any,
                    }
                } else {
                    &neutral
                };
                let buckets: Vec<&[Vec<usize>]> = if doctrine_positions[position] {
                    vec![first, &neutral[..], &any[..]]
                } else {
                    vec![&neutral[..], &any[..]]
                };
                let mut at = None;
                for buckets in buckets {
                    if let Some(drawn) = draw_from(buckets, below, &taken, &mut rng) {
                        at = Some(drawn);
                        break;
                    }
                }
                if let Some(at) = at {
                    filled = Some(at);
                    break;
                }
            }
            let Some(at) = filled else {
                return Err(format!("the content cannot fill level {level}'s deck"));
            };
            taken[at] = true;
            deck.push(content.cards[at].id.clone());
        }

        // f. Marks: Radiant's stacks, then Buffed's with its own shuffle, so a card may be both.
        let length = deck.len();
        let radiant = (stacks_of(&stacks, AssaultUpgradeKind::Radiant)
            * upgrade_row(AssaultUpgradeKind::Radiant).0)
            .min(length as i32)
            .max(0) as usize;
        let buffed = (stacks_of(&stacks, AssaultUpgradeKind::Buffed)
            * upgrade_row(AssaultUpgradeKind::Buffed).0)
            .min(length as i32)
            .max(0) as usize;
        let positions: Vec<usize> = (0..length).collect();
        let radiant_at = if radiant == 0 {
            Vec::new()
        } else {
            rng.shuffle(&positions)[..radiant].to_vec()
        };
        let buffed_at = if buffed == 0 {
            Vec::new()
        } else {
            rng.shuffle(&positions)[..buffed].to_vec()
        };
        let mut marks = Vec::new();
        for (index, _) in deck.iter().enumerate() {
            let radiant = radiant_at.contains(&index);
            let buffed = buffed_at.contains(&index);
            if radiant || buffed {
                marks.push(jackioh_engine::wire::DeckMark {
                    index,
                    radiant,
                    upgrades: i32::from(buffed),
                });
            }
        }

        // g. The level, with everything held at it.
        let mut upgrades = Vec::new();
        for (at, (kind, _, _, _, _)) in ASSAULT_UPGRADES.iter().enumerate() {
            if stacks[at] > 0 {
                upgrades.push(AssaultUpgrade {
                    kind: *kind,
                    stacks: stacks[at],
                });
            }
        }
        levels.push(AssaultLevel {
            level,
            enemy,
            doctrine,
            commander: rank.map(|index| commanders[index].id.clone()),
            escalations,
            upgrades,
            rarities,
            deck,
            marks,
            handicap,
        });
    }

    Ok(AssaultDay {
        date: date.into(),
        seed,
        levels,
    })
}
