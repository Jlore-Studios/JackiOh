//! Every number of the dungeon run (CLAUDE.md rule 9), each named by the section of
//! `docs/dungeon-run.md` that sets it. The mode has no spec section yet: a number here is the plan's,
//! and one the plan marks a strawman is to be tuned.

use jackioh_engine::config::{DECK_SIZE, HERO_HEALTH};
use jackioh_engine::wire::PrintedRarity;

use crate::{RunByFight, RunNodeKind, RunTier};

/// DR2: the letters of a seed.
pub const RUN_SEED_LENGTH: usize = 6;

/// DR4: the fights before each act's boss. With the 2 mini-bosses and the final boss, 16 fights.
pub const RUN_ACT_FIGHTS: [usize; 3] = [5, 4, 4];
/// DR4: the fewest nodes a row of the map offers.
pub const RUN_ROW_WIDTH_MIN: i32 = 2;
/// DR4: the most nodes a row of the map offers.
pub const RUN_ROW_WIDTH_MAX: i32 = 3;
/// DR4: the chance, in percent, that a fight node of the map is an elite.
pub const RUN_ELITE_PERCENT: i32 = 30;
/// DR5: the chance, in percent, that a fight node that is no elite is a ghost.
pub const RUN_GHOST_PERCENT: i32 = 10;
/// DR4, DR7: the weights of the tiles a row between two fight rows deals. A campfire also always stands
/// on the row before each boss.
pub const RUN_TILE_WEIGHTS: [(RunNodeKind, i32); 5] = [
    (RunNodeKind::Shop, 2),
    (RunNodeKind::Campfire, 2),
    (RunNodeKind::Event, 3),
    (RunNodeKind::Chest, 2),
    (RunNodeKind::Shrine, 1),
];

/// DR3: the cards of a starting deck, at every level.
pub const RUN_STARTER_SIZE: usize = 10;
/// DR3, a strawman: the starting deck's slots at each level, from level 1, as counts of Common, Rare,
/// Epic, Legendary, Mythic and Anything slots. Each row holds `RUN_STARTER_SIZE`.
pub const RUN_SLOT_LADDER: [[usize; 6]; 6] = [
    [5, 4, 0, 0, 0, 1],
    [4, 4, 1, 0, 0, 1],
    [3, 4, 2, 0, 0, 1],
    [2, 3, 3, 1, 0, 1],
    [1, 2, 3, 2, 1, 1],
    [0, 2, 3, 2, 1, 2],
];
/// DR3: the cards any starting deck may hold before anything is extracted. Placeholders until the
/// designer's list: the 20 Training Grounds Commons and Rares with the fewest script lines (the
/// catalog's `loc`), in catalog order, 14 Commons and 6 Rares. The first `RUN_STARTER_SIZE` fit level
/// 1's slots.
pub const RUN_STARTER_CARDS: [&str; 20] = [
    "traininggrounds-001",
    "traininggrounds-002",
    "traininggrounds-004",
    "traininggrounds-006",
    "traininggrounds-007",
    "traininggrounds-008",
    "traininggrounds-011",
    "traininggrounds-017",
    "traininggrounds-020",
    "traininggrounds-023",
    "traininggrounds-042",
    "traininggrounds-043",
    "traininggrounds-044",
    "traininggrounds-047",
    "traininggrounds-048",
    "traininggrounds-050",
    "traininggrounds-051",
    "traininggrounds-054",
    "traininggrounds-058",
    "traininggrounds-059",
];
/// DR3, DR9, a strawman: the Embers each level of the starting deck costs from the one before it, so
/// `RUN_LEVEL_EMBERS[0]` buys level 2. One per step of `RUN_SLOT_LADDER`.
pub const RUN_LEVEL_EMBERS: [i64; RUN_SLOT_LADDER.len() - 1] = [10, 25, 45, 70, 100];
/// DR3: the most artifacts a run starts with.
pub const RUN_STARTER_ARTIFACTS_MAX: usize = 3;
/// DR3: the most points of artifacts a run starts with.
pub const RUN_STARTER_ARTIFACT_POINTS: i32 = 8;
/// DR3: an artifact's points by its tier.
pub const RUN_TIER_POINTS: [(RunTier, i32); 5] = [
    (RunTier::Bronze, 1),
    (RunTier::Silver, 2),
    (RunTier::Gold, 3),
    (RunTier::Platinum, 4),
    (RunTier::Diamond, 5),
];

/// DR5: the health a run starts with and the most it heals to, before passives raise it.
pub const RUN_MAX_HP: i32 = HERO_HEALTH;
/// DR5, a strawman: each enemy's hero health by its kind of fight.
pub const RUN_ENEMY_HEALTH: RunByFight = RunByFight {
    fight: 20,
    elite: 30,
    ghost: HERO_HEALTH,
    mini_boss: 40,
    boss: 50,
};
/// DR5: the enemy artifact that gives its carrier the Medium AI's resources (`AI_DIFFICULTY.medium`,
/// R180) in a fight.
pub const RUN_LOOMING_PRESENCE: &str = "looming-presence";
/// DR5: the enemy artifact that gives its carrier the Hard AI's resources (`AI_DIFFICULTY.hard`, R180)
/// in a fight. It outranks Looming Presence when an enemy carries both.
pub const RUN_OVERWHELMING_PRESENCE: &str = "overwhelming-presence";
/// DR5: an enemy's deck, its own cards first and the rest dealt from the pool.
pub const RUN_ENEMY_DECK_SIZE: usize = DECK_SIZE as usize;
/// DR5: the lowest tier of a mini-boss's drop when its content names none.
pub const RUN_MINI_BOSS_DROP_TIER: RunTier = RunTier::Gold;

/// DR6: the chance, in percent, that a won fight offers an artifact. A boss's drop is preseeded.
pub const RUN_ARTIFACT_PERCENT: RunByFight = RunByFight {
    fight: 10,
    elite: 25,
    ghost: 25,
    mini_boss: 100,
    boss: 100,
};
/// DR6, a strawman: the Coins a won fight pays.
pub const RUN_COINS: RunByFight = RunByFight {
    fight: 15,
    elite: 30,
    ghost: 30,
    mini_boss: 60,
    boss: 100,
};
/// DR6, a strawman: the Tickets a won fight pays.
pub const RUN_TICKETS: RunByFight = RunByFight {
    fight: 1,
    elite: 2,
    ghost: 2,
    mini_boss: 5,
    boss: 10,
};
/// DR6: the cards an opened pack shows, before passives add more.
pub const RUN_PACK_SIZE: usize = 8;
/// DR6: the cards of an opened pack the player keeps, before passives add more.
pub const RUN_KEEP: usize = 3;
/// DR6: the most cards of a pack anyone keeps.
pub const RUN_KEEP_MAX: usize = 5;

/// DR7: a campfire's heal, in percent of the run's max HP.
pub const RUN_CAMPFIRE_HEAL_PERCENT: i32 = 30;
/// DR7: the cards a campfire removes.
pub const RUN_CAMPFIRE_REMOVE: usize = 2;
/// DR7: the cards a shop sells.
pub const RUN_SHOP_CARDS: usize = 5;
/// DR7: the artifacts a shop sells.
pub const RUN_SHOP_ARTIFACTS: usize = 2;
/// DR7, a strawman: a shop card's Coin price by its rarity.
pub const RUN_CARD_PRICE: [(PrintedRarity, i32); 5] = [
    (PrintedRarity::Common, 20),
    (PrintedRarity::Rare, 35),
    (PrintedRarity::Epic, 60),
    (PrintedRarity::Legendary, 90),
    (PrintedRarity::Mythic, 130),
];
/// DR7, a strawman: a shop artifact's Coin price by its tier.
pub const RUN_ARTIFACT_PRICE: [(RunTier, i32); 5] = [
    (RunTier::Bronze, 50),
    (RunTier::Silver, 80),
    (RunTier::Gold, 120),
    (RunTier::Platinum, 160),
    (RunTier::Diamond, 220),
];
/// DR7: the shop's Coin price for removing a card.
pub const RUN_REMOVE_PRICE: i32 = 50;
/// DR7: the shop's Ticket price for making a card Radiant (a campfire does it free).
pub const RUN_RADIANT_TICKETS: i32 = 3;
/// DR7: the shop's Ticket price for buffing a card (a campfire does it free).
pub const RUN_BUFF_TICKETS: i32 = 2;
/// DR7: the Tickets an artifact's next level costs, times the level it is at.
pub const RUN_LEVEL_TICKETS: i32 = 5;
/// DR7: an artifact's highest level.
pub const RUN_ARTIFACT_LEVEL_MAX: i32 = 3;
/// DR7: the Coins a chest holds.
pub const RUN_CHEST_COINS: i32 = 40;
/// DR7: the Tickets a chest holds.
pub const RUN_CHEST_TICKETS: i32 = 2;
/// DR7: the chance, in percent, that a chest also holds an artifact.
pub const RUN_CHEST_ARTIFACT_PERCENT: i32 = 50;
/// DR7: the blessings a shrine offers to choose from.
pub const RUN_SHRINE_OFFERS: usize = 3;

/// DR9, a strawman: the score for each row of the map reached.
pub const RUN_SCORE_ROW: i64 = 10;
/// DR9, a strawman: the score for each fight won, by its kind.
pub const RUN_SCORE_FIGHT: RunByFight = RunByFight {
    fight: 20,
    elite: 50,
    ghost: 50,
    mini_boss: 100,
    boss: 200,
};
/// DR9, a strawman: the score for each point of damage the fights' reports dealt.
pub const RUN_SCORE_DAMAGE: i64 = 1;
/// DR9, a strawman: the score for each point of healing the fights' reports did.
pub const RUN_SCORE_HEALING: i64 = 1;
/// DR9, a strawman: the score for each card the fights' reports drew.
pub const RUN_SCORE_CARD_DRAWN: i64 = 2;
/// DR9: each mini-boss beaten multiplies the score by this percent.
pub const RUN_SCORE_MINI_BOSS_PERCENT: i64 = 150;
/// DR9: beating the final boss multiplies the score by this percent.
pub const RUN_SCORE_FINAL_BOSS_PERCENT: i64 = 200;
/// DR8, DR9, a strawman: each curse point adds this percent to the score.
pub const RUN_SCORE_CURSE_POINT_PERCENT: i64 = 10;
/// DR9, a strawman: the score one Ember is worth.
pub const RUN_SCORE_PER_EMBER: i64 = 100;
