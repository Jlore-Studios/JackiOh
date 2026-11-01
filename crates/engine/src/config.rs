//! Every rules constant lives here (BUILD §2). The engine never hard-codes these numbers.
//! Rows SPEC §11 marks "decide" (R1, R2, R4, R5, R14, R26, R39) are flipped here in one line. A
//! constant the engine plays at one value only (R1, R9, R14, R36, R39) is asserted at compile time
//! where its rule is played, so flipping it stops the build at the code to change, and
//! `crates/tools/tests/acceptance.rs` fails on a constant here that no code reads (#1640).
//!
//! Port of `packages/engine/src/config.ts` (part 1's type freeze, SURFACE §6.4), values verbatim,
//! plus (CLAUDE.md rule 9) the numbers TS stated outside `config.ts` (research A §4), moved here
//! under their TS names, and the training lanes' gate (SURFACE §14.2). A TS constant object becomes
//! a `pub const` of a struct named after it (SURFACE §4.2).
//!
//! Integer types: a game quantity (a stat, a cost, a count of cards, a turn, a lane) is `i32`, as
//! everywhere in the engine (SURFACE §4.3); a bound on how many times a Rust loop runs or how long a
//! Rust collection or string may grow is `usize`, so it compares with `len()` without a cast.

use std::ops::Index;

use serde::{Deserialize, Serialize};

use crate::wire::{
    CardElement, CardType, CraftHatKind, CraftHatPrice, CraftKeywordPrice, CraftVerb, CraftVerbPrice,
    DuosTeamRules, GlitchOutcome, KeywordKind, PrintedRarity, Rarity, ResourcePrice, SetName,
    StarscapeReward, Tag, string_union,
};

/// §2.6, R1705000
pub const DECK_SIZE: i32 = 25;
/// R1705003: §2.6's deck size before #1705 raised it to DECK_SIZE. A game made then, folded again
/// (`create_recorded_game`), holds a seat with no handicap to this.
pub const EARLIER_DECK_SIZE: i32 = 20;
/// R1705000: the cards a Medium AI seat's deck holds past DECK_SIZE (§9.9).
pub const MEDIUM_EXTRA_DECK_CARDS: i32 = 5;
/// R1705000: the cards a Hard AI seat's deck holds past DECK_SIZE (§9.9).
pub const HARD_EXTRA_DECK_CARDS: i32 = 10;
/// §2.6, §9.4
pub const MAX_COPIES: i32 = 1;
/// §9.4 L1, R252: the decks a Conquest loadout (a trio) holds, exactly. The validator's L1 reads it,
/// and so does the web's trio editor, through `engineConfig.ts`.
pub const LOADOUT_DECKS: usize = 3;
/// §2.3
pub const MAX_MANA: i32 = 4;
/// §2
pub const HERO_HEALTH: i32 = 30;

/// BUILD M1-T1 (`createGame` yields `turn = 0`): the turn number §2.1's setup runs at. Setup is no
/// player's turn — p1 takes the first turn after it (§2.1 step 5) — so nothing played during it
/// belongs to a turn of its controller's (R155, R241).
pub const SETUP_TURN: i32 = 0;
/// §2.5, R2, R389 (patch v0.2.0, docs/classic-sets.md B4.3): the cap counts player-turns, so 60 means
/// 30 each. Two 25-card decks that do nothing fatigue out at player-turn 58 (§2.4, R3, R2368000), so the
/// cap is a backstop for games that heal, gain Armor or refill a deck, and fatigue is a long game's usual
/// end.
pub const TURN_CAP_PLAYER_TURNS: i32 = 60;
/// §2.4, R4 (decide): a card drawn or added to a full hand is burned. A player's own hand size, once an
/// effect sets one (Meditative #79, R1143), is read in its place (`query::hand_cap_of`). Whether a hand
/// is full is `query::room_in`'s test, the one every rule reads.
pub const HAND_CAP: i32 = 10;
/// R1143: the most a set hand size may be (Meditative #79's `handSize` param's max), the most cards the
/// client lays out in one hand.
pub const HAND_CAP_MAX: i32 = 14;

/// §2.1, R2368000: indexed by seat, each seat's opening draw: three for the first, four for the second
/// (issue #2368, Hearthstone's split, as before issue #1156's four for both, R1156000).
pub const OPENING_DRAW: &[i32] = &[3, 4];

/// §2.1, R244: indexed by seat like `OPENING_DRAW`, the copies of The Coin each seat is dealt once both
/// mulligans are answered. Hearthstone's rule: the player going second gets one, the first none.
pub const OPENING_COINS: &[i32] = &[0, 1];
/// §7, R245: the catalog id of The Coin, the one token a rule deals rather than a card.
pub const COIN_DEF_ID: &str = "core-t-coin";

/// MD-D1 (Meditative #50 CN Tech): the games a card needs in the compiled win-rate table before it
/// can lead it. A thinner row never leads, so a new patch's first games cannot crown a card. The
/// trailer Meditative+ #2 NA Tech reads has the same floor (R1183000).
pub const CN_TECH_MIN_GAMES: i32 = 20;

/// R1183004 (Meditative+ #25 KY's Intuition): the most lines a card's code block shows, its script's
/// first lines of code. `crates/cards/build.rs` reads this line to cut the blocks it compiles in.
pub const CODE_BLOCK_MAX_LINES: usize = 12;

/// R1846000: a card is BUSTED when its script has more code lines than this, the designer's number
/// (Stinkyscape #109 xxFreshJack's pool). `crates/cards/build.rs` reads this line to compile the list in.
pub const BUSTED_CODE_LINES: usize = 100;

/// R1846000: a card is complicated when its script has more code lines than this (Stinkyscape #111
/// Shockwave447's pool). `crates/cards/build.rs` reads this line to compile the list in.
pub const COMPLICATED_CODE_LINES: usize = 60;

/// R1183003 (Meditative+ #18 Cognitive Offloading): the most actions the autopilot takes in one turn,
/// a prompt's answer counted as one. Past it, the rest of the turn is a timeout's (R79): its open
/// prompts answered by the random policy, then the turn ended. A turn rarely takes a third of it; the
/// bound keeps a card that refills the hand, and every search that reaches the autopilot, finite.
pub const AUTOPILOT_MAX_ACTIONS: i32 = 30;

/// R1183003: the answers to one prompt the autopilot tries on its copy of the board, in the order
/// `legal_actions` lists them; the rest are not considered.
pub const AUTOPILOT_ANSWERS_TRIED: usize = 8;

/// R382: a pool named by one of these tags also holds that tag's tokens. The five Grapes are Fruit
/// Tokens "that can be generated by any Fruit card", and appear in no other pool but one that takes
/// every token (`withTokens`). R1421: a Prime pool ("a random Prime card") holds the Prime tokens,
/// since every Prime card is a token a Catalyst's Death shuffles in, and an AI pool ("a random AI
/// generated card") holds the ten AI generated cards, every one a token. R1837000: no shipped card
/// draws a pool by #1837's five tags, so none of them is here yet; Other-Origins is carried by tokens
/// alone (Classic+ #32.1 to #32.3), and the card that first names its pool decides.
pub const POOL_TOKEN_TAGS: &[Tag] = &[Tag::Fruit, Tag::Prime, Tag::Ai];

/// R1424: the tribal tags, the tags that name a people or a faction. "A card with a Tribal tag" has
/// one of them, a card with all of them is "All Tribes" (Meditative #87 Tatches the Totem, and the
/// card frame says so), and "tribal tag based hate" is an effect that picks its cards by having or
/// lacking one. Book, Fruit, Pancake, AI, Plague, Catalyst, Prime, Acclaimed, Call to Chaos,
/// Quickdraw, Wincon, Potion (Meditative+, R1178000), Terminus, Quest, Otherworldly, Other-Origins,
/// Jelly Bean (#1837, R1837000), Drone Core, Weapon, War Doctrine, Swarm (the Stinkyscape set,
/// R1857000), Resource and Faction (#2521: the six Resources, and the five Faction cards themselves,
/// not their members) and Token name families and mechanics, not tribes.
pub const TRIBAL_TAGS: &[Tag] = &[Tag::Human, Tag::Felinor, Tag::Ky, Tag::Cn, Tag::Jlockeed];

/// R1176000: the tag every card in Chinese has. A card instance whose `chinese` flag is set (R1300)
/// carries it wherever an instance's tags are read (`query::tags_of`), as a granted tag is (R923).
pub const CHINESE_TAG: Tag = Tag::Cn;

/// §3
pub const UNIT_ZONES: i32 = 5;
/// §3
pub const BACKROW_ZONES: i32 = 5;
/// Classic #18 Glitch in the System's number choice (SPEC §8.6 row 18, BUILD §2): the numbers 0 to 10,
/// the same eleven options every time, so the options reveal nothing about any hand or deck.
pub const GLITCH_NUMBERS: &[i32] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

// The Glitch Easter egg (SPEC §7, issue #170; R673–R679).
/// R673: the "… in the System" cards — Classic #18 Glitch in the System and Classic #25 Lag in the System.
pub const SYSTEM_CARD_DEF_IDS: &[&str] = &["classic-018", "classic-025"];
/// R673, R674: the catalog id of Glitch, the hidden token a generated card may become.
pub const GLITCH_DEF_ID: &str = "classic-t-glitch";
/// R673: each System play adds this many chances in GLITCH_ODDS_DENOMINATOR that a generated card is Glitch.
pub const GLITCH_ODDS_PER_SYSTEM_PLAY: i32 = 1;
/// R673: "a 1/10000 chance".
pub const GLITCH_ODDS_DENOMINATOR: i32 = 10000;
/// R676: Glitch's four outcomes, in the order the issue lists them; one is drawn uniformly by the match rng.
pub const GLITCH_OUTCOMES: &[GlitchOutcome] = &[
    GlitchOutcome::Reset,
    GlitchOutcome::Swap,
    GlitchOutcome::Boards,
    GlitchOutcome::Void,
];

/// R5 (decide): any unit may attack any enemy unit or hero, subject to Taunt.
pub const LANE_RESTRICTED_ATTACKS: bool = false;
/// R1 (decide): Cry fires only when played from hand or cast by an effect.
pub const CRY_ON_PLAY_ONLY: bool = true;
/// R14 (decide): units and backrow form two independent rings.
pub const ROTATION_RING: &str = "two-rings";
/// R26 (decide): Genn's Greed exiles odd current-cost cards.
pub const GENN_GREED_EXILES: &str = "odd";
/// R39 (decide): Felinor Fiender = printed stats plus the sum of your Felinors.
pub const FIENDER_STATS_MODE: &str = "printed-plus-sum";

/// R3: the Nth draw from an empty library deals N damage.
pub const FATIGUE_DAMAGE: fn(i32) -> i32 = |n| n;

/// §2.5, R36: offers allowed per player per turn.
pub const DRAW_OFFERS_PER_TURN: i32 = 1;
/// §2.5, R36: a declined offer blocks that player for this many of their turns.
pub const DRAW_OFFER_BLOCK_TURNS: i32 = 3;

/// §6.1 Tradable, R1838000: the Trades a player may make each turn, counted over all their Tradable
/// cards. A player's own figure, once an effect sets one, is read in its place (`trade::trades_per_turn_of`).
pub const TRADES_PER_TURN: i32 = 1;
/// §6.1 Tradable, R1838000: the mana a Trade costs — none, as the designer wrote the keyword.
pub const TRADE_COST: i32 = 0;
/// §2.5, R79: the most prompts one `timeout` answers for the active player. A chain of prompts is a
/// handful (KY's Private Tutor asks three times), after which the turn ends. This only bounds a card
/// that would keep asking, so one `timeout` always returns: if a prompt is still open after this many
/// answers, the timeout stops there with the turn still running, and the next expiry carries on. No
/// Core card can reach it.
pub const TIMEOUT_ANSWER_CAP: usize = 500;

/// R28
pub const CALL_TO_CHAOS_CHAIN_CAP: i32 = 20;
/// R58: cast-on-draw cards one draw may cast before the next one goes to hand uncast.
pub const CAST_ON_DRAW_CHAIN_CAP: i32 = 20;
/// R2067000: cast-on-draw cards one action may cast over all its chains, before the next one goes to
/// hand uncast as R58's 21st does. A trigger's draw begins a chain of its own (R217), so a trigger
/// that draws for each cast's discard (Classic #64 Malzahar's Recycler under #21 Hinder) beside a
/// library that refills as it is drawn (#33 Unstable Clone Machine) began chains without end. A whole
/// library's worth, so an action that does not refill its library as it draws never reaches it.
pub const CAST_ON_DRAW_ACTION_CAP: i32 = LIBRARY_CAP;

/// `{ base, radiant }`: a number by the face that prints it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ByFace {
    pub base: i32,
    pub radiant: i32,
}

impl ByFace {
    /// `value[radiant ? "radiant" : "base"]`.
    pub const fn on(&self, radiant: bool) -> i32 {
        if radiant { self.radiant } else { self.base }
    }
}

/// §8 #73: largest single damage instance a hero with Anti-oneshot Armor takes.
pub const ANTI_ONESHOT_CAP: ByFace = ByFace { base: 5, radiant: 3 };

/// `{ normal, embiggened }`: #84 Going Long's Armor by the version played, its normal version at the
/// cheaper price or its Embiggened version at its Embiggened cost (R1175000).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HeroArmorPrice {
    pub normal: i32,
    pub embiggened: i32,
}

impl HeroArmorPrice {
    /// `value[embiggened ? "embiggened" : "normal"]`.
    pub const fn on(&self, embiggened: bool) -> i32 {
        if embiggened { self.embiggened } else { self.normal }
    }
}

/// `HERO_ARMOR`'s shape.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HeroArmor {
    pub base: HeroArmorPrice,
    pub radiant: HeroArmorPrice,
}

impl HeroArmor {
    /// `HERO_ARMOR[radiant ? "radiant" : "base"]`.
    pub const fn on(&self, radiant: bool) -> HeroArmorPrice {
        if radiant { self.radiant } else { self.base }
    }
}

/// §8 #84 Going Long: the Armor it gives its controller's hero (§4.4 step 2), by face and by which
/// version was played — "Armor 2. Embiggened (4): Armor 4", radiant "Armor 4. Embiggened (4): Armor 8"
/// (patch v0.1.1, R1175000). `normal` is the normal version's, at the printed cost 2, `embiggened` the
/// Embiggened version's, at its Embiggened cost 4 (R81 records which on the instance).
pub const HERO_ARMOR: HeroArmor = HeroArmor {
    base: HeroArmorPrice {
        normal: 2,
        embiggened: 4,
    },
    radiant: HeroArmorPrice {
        normal: 4,
        embiggened: 8,
    },
};
/// §8 #38 Quickstriker, R281: the multiple of X each granted Combo deals to the enemy hero, by the
/// granting card's face — "Combo X: deal X damage", radiant "deal 2X damage", one hit either way.
pub const QUICKSTRIKER_COMBO_MULTIPLE: ByFace = ByFace { base: 1, radiant: 2 };

/// R636: the attacks a Unit with Windfury may declare in one turn, where every other Unit has one.
pub const WINDFURY_ATTACKS: i32 = 2;

/// R1200 (Meditative #86 Mayor Medinamogger, ME-RANDOMTARGETS): the target an `attack` action
/// names while a Mayor acts — the engine draws the attack's target from the attacker's legal
/// ones, so the action lists one attack per attacker under this sentinel.
pub const RANDOM_ATTACK_TARGET: &str = "random";

/// R21: the combat keyword pool, which Plastic Surgery, Zao Gao, the Conjure Rush Tokens, a Buff's
/// keyword row and Keymaster Keenus's Aura draw from; a unit never gets a keyword it has. R346: patch
/// v0.1.1 added Pierce, R636 Windfury and R49 (patch v0.2.4) Deft. Patch v0.3.7 (R781005) made it the
/// designer's combat keyword pool, in the designer's order, adding Flying, Ranged, Indestructible and
/// Spell Damage +1; the order picks which keyword a seeded draw returns. Patch v0.3.Y (R1025000)
/// took Armor 1 and Spell Damage +1 out again, and issue #1615 (R1615000) Charge and Indestructible,
/// as too swingy a random grant, leaving fourteen entries in the same relative order.
/// Temporary (R637) stays out, since a card on the field is never in a hand to be discarded.
/// (Keyword keys, as `keyword_key` writes them.)
pub const RANDOM_KEYWORD_POOL: &[&str] = &[
    "Taunt",
    "Rush",
    "First Strike",
    "Flying",
    "Ranged",
    "Poisonous",
    "Reborn",
    "Divine Shield",
    "Windfury",
    "Cleave",
    "Deft",
    "Lifesteal",
    "Pierce",
    "Trample",
];

/// R349: a Unit that prints no Radiant form (the Ghoul Token) is, made Radiant, its base face with
/// its attack and health multiplied by this — a summon's X/X included (§7).
pub const RADIANT_FALLBACK_FACTOR: i32 = 2;

/// R348: the least X a player may choose for an X-cost card whose X is theirs to choose (#24, #74).
/// Heroic Power's X is its power's and never chosen (R43), so this does not reach it.
pub const MIN_CHOSEN_X: i32 = 1;

/// R25: KY's Math Equation; the index clamps at 11.
pub const FIB: &[i32] = &[0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89];

pub fn fib(index: i32) -> i32 {
    let clamped = index.clamp(0, FIB.len() as i32 - 1);
    FIB.get(clamped as usize).copied().unwrap_or(0)
}

/// R80: a library holds at most this many cards. Whether a library is full is `query::room_in`'s test,
/// the one every rule reads.
pub const LIBRARY_CAP: i32 = 60;

/// §6.3
pub const FUSE_COST_CAP: i32 = 4;
/// R9: replacements are drawn before the returned cards are shuffled back.
pub const MULLIGAN_ORDER: &str = "draw-then-shuffle";
/// §10.7: the random policy ends its turn with this probability when other actions exist.
pub const AI_END_TURN_PROBABILITY: f64 = 0.1;
/// R90: the most choice combinations `legalActions` enumerates for one play.
pub const MAX_CHOICE_COMBINATIONS: usize = 64;
/// §9.3: how many answered nonces the state remembers for dedupe.
pub const NONCE_HISTORY: usize = 64;

/// §2.4: the draws each start of turn makes before any handicap (R183).
pub const DRAWS_PER_TURN: i32 = 1;

/// §9.9, R180: the resources one seat plays with. Every field is a non-negative integer. The type
/// lives here because `config.ts` imports nothing; `state.rs` stores it on a `PlayerState`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Handicap {
    /// R184: exactly how many cards `createGame` requires in this seat's deck.
    pub deck_size: i32,
    /// R181: crystals added to the turns-started count before the cap.
    pub mana_bonus: i32,
    /// R181: the most max mana a refresh gives, before persistent modifiers.
    pub mana_cap: i32,
    /// R182: cards added to §2.1's opening draw.
    pub extra_opening_cards: i32,
    /// R183: separate draws after DRAWS_PER_TURN at each start of turn.
    pub extra_draws_per_turn: i32,
    /// R290: the health this seat's hero starts the game with, in place of HERO_HEALTH. Absent means
    /// HERO_HEALTH, which is why the three practice tiers never set it and hash exactly as before the
    /// field existed. Only the tutorial's handicap (`AI_TUTORIAL`) sets it, and only lower.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hero_health: Option<i32>,
    /// R2201000: the least health this seat's hero can fall to, so the seat cannot lose the game.
    /// Absent means none, which is why no tier sets it and every other game hashes as before the
    /// field existed. Only a lesson's human (`TUTORIAL_HUMAN`) sets it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hero_floor: Option<i32>,
    /// R2201001: while either seat's handicap sets it, every random pool the game draws from leaves
    /// out Legendary and Mythic cards. Absent means no hold, so every other game hashes as before.
    /// Only a lesson before the last sets it (§9.10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pool_hold: Option<bool>,
    /// R2533000 (replacing R2433000's `coreOnly`): this seat plays only the tutorial's cards, those of
    /// TUTORIAL_CARD_SET whose rarity is none of TUTORIAL_BARRED_RARITIES. It never plays, sets or
    /// casts another card, from its hand or its graveyard, however the card got there. Absent means
    /// no bar, so every other game hashes as before the field existed. Only the tutorial opponent
    /// sets it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tutorial_cards_only: Option<bool>,
    /// R2433001: the most copies of one card id this seat's deck may hold, in place of MAX_COPIES.
    /// Absent means MAX_COPIES, so every other game hashes as before the field existed. Only a
    /// lesson's two seats set it (`TUTORIAL_HUMAN`, and `AI_TUTORIAL` since R2533001).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub deck_copies: Option<i32>,
}

/// R180: this spec's own resources. A seat with no handicap plays with these.
pub const HUMAN_HANDICAP: Handicap = Handicap {
    deck_size: DECK_SIZE,
    mana_bonus: 0,
    mana_cap: MAX_MANA,
    extra_opening_cards: 0,
    extra_draws_per_turn: 0,
    hero_health: None,
    hero_floor: None,
    pool_hold: None,
    tutorial_cards_only: None,
    deck_copies: None,
};

string_union! {
    /// §9.9: the three practice tiers.
    pub enum Difficulty {
        Easy = "easy",
        Medium = "medium",
        Hard = "hard",
    }
}

pub const DIFFICULTIES: &[Difficulty] = &[Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

/// `Readonly<Record<Difficulty, Handicap>>`: `AI_DIFFICULTY`'s shape, indexed by `Difficulty`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct AiDifficulty {
    pub easy: Handicap,
    pub medium: Handicap,
    pub hard: Handicap,
}

impl Index<Difficulty> for AiDifficulty {
    type Output = Handicap;

    fn index(&self, difficulty: Difficulty) -> &Handicap {
        match difficulty {
            Difficulty::Easy => &self.easy,
            Difficulty::Medium => &self.medium,
            Difficulty::Hard => &self.hard,
        }
    }
}

/// §9.9's table (R180): the AI seat's handicap per tier. The AI itself is identical at every tier;
/// only these resources differ. Easy is a human's resources exactly, so an Easy game stores no
/// handicap and hashes like any other game.
pub const AI_DIFFICULTY: AiDifficulty = AiDifficulty {
    easy: HUMAN_HANDICAP,
    medium: Handicap {
        deck_size: DECK_SIZE + MEDIUM_EXTRA_DECK_CARDS,
        mana_bonus: 1,
        mana_cap: 5,
        extra_opening_cards: 1,
        extra_draws_per_turn: 0,
        hero_health: None,
        hero_floor: None,
        pool_hold: None,
        tutorial_cards_only: None,
        deck_copies: None,
    },
    hard: Handicap {
        deck_size: DECK_SIZE + HARD_EXTRA_DECK_CARDS,
        mana_bonus: 1,
        mana_cap: 7,
        extra_opening_cards: 1,
        extra_draws_per_turn: 1,
        hero_health: None,
        hero_floor: None,
        pool_hold: None,
        tutorial_cards_only: None,
        deck_copies: None,
    },
};

/// §9.10, R290: the tutorial opponent's handicap, the one below Easy. It is a handicap like the
/// three tiers (R180) and lowers the AI seat's resources instead of raising them: a 12-card deck
/// (each lesson's own list, R291, so it runs out and fatigues sooner), at most 3 mana crystals, and
/// a hero that starts at 20 health; it plays only Training Grounds cards that are not Legendary or
/// Mythic (R2533000), and its deck may hold TUTORIAL_DECK_COPIES copies of a card (R2533001). The AI's search,
/// evaluation and budget are the ones every tier plays with, but with the learned evaluation on this
/// seat keeps the hand eval (R1012003). It is not in `DIFFICULTIES`: nobody picks it, the tutorial
/// deals it.
pub const AI_TUTORIAL: Handicap = Handicap {
    deck_size: 12,
    mana_bonus: 0,
    mana_cap: 3,
    extra_opening_cards: 0,
    extra_draws_per_turn: 0,
    hero_health: Some(20),
    hero_floor: None,
    pool_hold: None,
    // R2533000: the tutorial opponent plays only Training Grounds cards that are not Legendary or
    // Mythic, whatever reaches its hand.
    tutorial_cards_only: Some(true),
    // R2533001: lesson 1's opponent holds copies of the set's vanilla units, as its human does.
    deck_copies: Some(TUTORIAL_DECK_COPIES),
};

/// §9.10, R1705005: the human seat's handicap in a lesson, whose fixed deal (R291) holds
/// EARLIER_DECK_SIZE cards. It is a human's resources but for the deck's size, so a lesson's deal
/// stays the one each lesson was written for while every other seat with no handicap holds
/// DECK_SIZE. A seat stores it (it is not a human's), so a lesson folds as it was played. R2201000:
/// its hero never falls below TUTORIAL_HERO_FLOOR, so a lesson cannot be lost. R2433001: its deck may
/// hold TUTORIAL_DECK_COPIES copies of a card.
pub const TUTORIAL_HUMAN: Handicap = Handicap {
    deck_size: EARLIER_DECK_SIZE,
    hero_floor: Some(TUTORIAL_HERO_FLOOR),
    deck_copies: Some(TUTORIAL_DECK_COPIES),
    ..HUMAN_HANDICAP
};

/// §9.10, R2433001, R2533001: the most copies of one card a lesson's deck holds, the human's and the
/// opponent's. Lesson 1 is mostly vanilla Units, and Training Grounds has only three with no text.
pub const TUTORIAL_DECK_COPIES: i32 = 4;

/// R2533000: the set a seat whose handicap sets `tutorialCardsOnly` plays from: the tutorial's own.
pub const TUTORIAL_CARD_SET: SetName = SetName::TrainingGrounds;

/// R2533000: the rarities of TUTORIAL_CARD_SET that such a seat never plays. A token is judged by
/// its printed rarity (TG #39.1 Training-Grounds Golem prints Legendary).
pub const TUTORIAL_BARRED_RARITIES: &[Rarity] = &[Rarity::Legendary, Rarity::Mythic];

/// §9.10, R2201000: the least health a lesson's human hero can fall to.
pub const TUTORIAL_HERO_FLOOR: i32 = 1;

// ---------------------------------------------------------------------------------------------
// Patch v0.2.0 (docs/classic-sets.md B3–B5). Every number the new rules state, by workstream.
// ---------------------------------------------------------------------------------------------

// ---- v0.2.0 constants: instance data (B3.3 Brittle, B3.4 Degrade and Upgrade) ----

/// B3.3, R385 (the designer's B9 #41: t + 2): a Brittle count started on player-turn t first ticks at
/// the first start of its controller's turn numbered t + this or later, so the card lives through the
/// rest of the turn it got the count on and a whole turn of the other player's first.
pub const BRITTLE_FIRST_TICK_TURNS: i32 = 2;
/// B3.3 rule 2: how much one start-of-turn tick takes off a Brittle count.
pub const BRITTLE_TICK: i32 = 1;
/// B3.3 rule 5, B3.4: how far one Degrade or Upgrade moves a numbered keyword, a Brittle count or an
/// X-cost card's X — one step of "one X on the card gets 1 worse / better".
pub const TUNE_X_STEP: i32 = 1;
/// B3.4 rule 3: how far one Degrade or Upgrade moves a card's cost (`costMod`).
pub const TUNE_COST_STEP: i32 = 1;
/// B3.4 rule 3, the designer's "Increase cost by (1) (up to 4)": a Degrade never lifts a cost above this.
pub const TUNE_COST_CAP: i32 = 4;
/// B3.4 rule 3: an Upgrade never takes a cost below this.
pub const TUNE_COST_FLOOR: i32 = 0;
/// B3.4 rule 3, the designer's "reduces the stats … by a total of 4": the split one stats change
/// rolls, k in 0 to this to attack and the rest to health.
pub const TUNE_STAT_TOTAL: i32 = 4;
/// B3.4 rule 3: "attack floors at 0".
pub const TUNE_ATTACK_FLOOR: i32 = 0;
/// B3.4 rule 3, the designer's "not below 1 health": a Degrade never takes current health below this.
pub const TUNE_HEALTH_FLOOR: i32 = 1;
/// B3.4 rule 3: the keywords a Degrade never removes, because losing them would help the card —
/// "Cannot be in Defense Position" is a static flag (§8 #65.1), not a keyword, so it is never offered.
/// R1220: Untributable is harmful the same way — losing it would let a Tribute take the card.
pub const TUNE_HARMFUL_KEYWORDS: &[KeywordKind] = &[
    KeywordKind::CantAttack,
    KeywordKind::Brittle,
    KeywordKind::Untributable,
];
/// R1748001: the keywords a Buff's keyword row may add to a Field Spell, in this order; R2436000 added
/// Immutable after Indestructible.
pub const TUNE_FIELD_SPELL_KEYWORDS: &[KeywordKind] = &[KeywordKind::Indestructible, KeywordKind::Immutable];
/// R1748001: the keywords a Buff's keyword row may add to a Spell or a Field Spell that deals damage
/// (`CardDef.deals_damage`).
pub const TUNE_DAMAGE_KEYWORDS: &[KeywordKind] = &[KeywordKind::Pierce, KeywordKind::Lifesteal];
/// R1748002: the Plague Counters one Buff places on, or one Nerf removes from, a card that uses its
/// own (`CardDef.uses_plague`).
pub const TUNE_PLAGUE_STEP: i32 = 1;

/// `{ upTo, step }`: one band of `PARAM_DEFAULT_STEP`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ParamStepBand {
    pub up_to: i32,
    pub step: i32,
}

/// `PARAM_DEFAULT_STEP`'s shape.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ParamDefaultStep {
    pub small: ParamStepBand,
    pub medium: ParamStepBand,
    pub large_divisor: i32,
}

/// B3.4 rule 5: a declared number's default step — 1 for a number up to `small.upTo`, 2 up to
/// `medium.upTo`, and above that a quarter of it, rounded (`largeDivisor`), never less than 1.
pub const PARAM_DEFAULT_STEP: ParamDefaultStep = ParamDefaultStep {
    small: ParamStepBand { up_to: 5, step: 1 },
    medium: ParamStepBand { up_to: 12, step: 2 },
    large_divisor: 4,
};
/// B3.4 rule 5, R386: "an amount never drops below 1" — a tuned number's floor: a declared number that
/// names no `min`, an X, a numbered keyword (Activate X, Tribute X, Armor …) and a Brittle count a
/// Degrade lowers.
pub const TUNE_MIN_AMOUNT: i32 = 1;
/// R1160 (Meditative #84 Volatility, ME-TUNEMULT): no tune multiplier — one Buff or Nerf
/// application moves each row once.
pub const TUNE_MULTIPLIER_NONE: i32 = 1;
/// R1160: Volatility's base face makes Buffs and Nerfs twice as effective on it.
pub const VOLATILITY_TUNE_MULTIPLIER: i32 = 2;
/// R1160: Volatility's Radiant face makes Buffs three times as effective on it (Nerfs plain).
pub const VOLATILITY_RADIANT_BUFF_MULTIPLIER: i32 = 3;

// ---- v0.2.0 constants: field (B3.1 Animated, E20, E21, E22) ----

// ---- v0.2.0 constants: play pipeline (E1, E11, E12, E15) ----

// play pipeline B (E11, E12, E15; R452–R455). A cast's X is its caster's current mana, at least
// MIN_CHOSEN_X (R348, R453); the other numbers of these rules are cards' own (`params`).

/// B5 E11, R454: the fewest Plague Counters a play from the graveyard that pays with tokens spends
/// (Classic #74 Corpse Plantation: "each such play spends at least 1 counter").
pub const MIN_PLAGUE_PAYMENT: i32 = 1;
/// B5 E11, R454: what one Plague Counter spent as mana pays of a price ("each counter pays (1)").
pub const PLAGUE_TOKEN_MANA: i32 = 1;
/// B5 E12, R452: how many casts one random cast and every cast made inside its resolution may make in
/// all (Classic+ #47 Jogg's Box, #38.1 Solarius Prime), as R28 caps a Call to Chaos chain and R58 a
/// cast-on-draw chain: a random cast whose casts cast at random cannot multiply without end.
pub const RANDOM_CAST_CHAIN_CAP: i32 = 20;

/// B5 E4, R451: the tags whose cards a player's "last face-up card played" record passes over — the
/// AI generated cards (B8), so two Autocompletes can't copy each other for ever.
pub const LAST_FACE_UP_SKIPPED_TAGS: &[Tag] = &[Tag::Ai];

// ---- v0.2.0 constants: activate and turn (B3.2, B4.3, E3, E10) ----

/// B3.2 rule 7, R384: "Activate ♾️" is any number of uses of each ability per turn, bounded here so a game
/// still ends (a fuzz game's random policy may keep activating). Every ♾️ card in the sets is bounded
/// by a resource too (units to Tribute, Plague Counters to consume).
/// R884000: an "Activate N" a Buff or KY's Constant took past it counts this many uses too.
pub const ACTIVATE_UNLIMITED_CAP: i32 = 100;

// ---- v0.2.0 constants: damage and combat (E5, E6, E35, E37) ----

/// B5 E5, E9, R460: how many times one hit may be redirected before it lands where it stands. Each
/// Trap that redirects is spent as it fires, so two players' backrows bound a chain of Traps by
/// themselves; the cap is for a Field Trap, which stays and could send a hit back and forth for ever.
/// (`u32`, the type of the per-call `EngineSink::converting` counter it bounds.)
pub const DAMAGE_REDIRECT_CAP: u32 = 2 * BACKROW_ZONES as u32;

/// `BERSERK_MARK`'s shape: a mark and its color key (R437).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct MarkSpec {
    pub mark: &'static str,
    pub color: &'static str,
}

/// B5 E35: the mark a Berserk unit carries in both views (R437's reusable mark), and its color key.
pub const BERSERK_MARK: MarkSpec = MarkSpec {
    mark: "berserk",
    color: "red",
};

/// R1174000: the mark a Unit is announced Frozen with (`added: true`) and thawed with (`added:
/// false`), public in both views (R437's reusable mark), and its color key: the client's frost.
pub const FROZEN_MARK: MarkSpec = MarkSpec {
    mark: "frozen",
    color: "cyan",
};

// ---- Stinkyscape's spell verbs (issue #1848, R1848000–R1848006) ----

/// R1848000 (Stinkyscape #2 Absolute Zero, "Freeze a Unit forever"): the thaw a forever freeze is
/// stamped with (`CardInstance.frozen`), a turn no game reaches (R2's cap is `TURN_CAP_PLAYER_TURNS`),
/// so no cleanup ever thaws it; only leaving the field and a Vanilla take it off.
pub const FROZEN_FOREVER: i32 = i32::MAX;
/// R1848005 (Stinkyscape #120 Wild System, "max 20 attacks"): the most attacks one melee makes.
pub const MELEE_MAX_ATTACKS: i32 = 20;
/// R1848006 (Stinkyscape #153 Stimmycheck): the printed chance, in percent, that Inflation hits a card.
pub const INFLATION_CHANCE: i32 = 15;
/// R1848006: how much a Stimmycheck's chance grows on the copy it makes ("+10% chance at Inflation").
pub const INFLATION_CHANCE_STEP: i32 = 10;
/// R1848006: the sides of an Inflation roll: a chance is in percent, so a roll is one of 100.
pub const INFLATION_ROLL_SIDES: i32 = 100;
/// R1848006: what Inflation adds to a card's price when it hits ("cost (1) more").
pub const INFLATION_COST: i32 = 1;

// ---- Stinkyscape's staged Quests (issue #1854, R1854000–R1854004) ----

/// R1854000: the number of a staged quest's first stage, the one that opens as the card enters (its
/// stages count from it: `completed_stage_of`, `open_stage`, `QuestMemory.stage`).
pub const FIRST_QUEST_STAGE: u32 = 1;

// ---- v0.2.0 constants: prompts and generation (E18, E19, E23–E25) ----

// generation (E19, E23–E25), below A6a's prompts constants:
/// R468: the longest ingredient list a fused id spells out (`t-<n>:<a>+<b>`, R179), in characters of
/// the part after `t-<n>:`. A longer one — a card fused onto again and again, Classic+ #74 — is named
/// by a digest of that list instead (`t-<n>:#<hex>`), and its definition keeps the whole list.
pub const FUSED_ID_CAP: usize = 120;
/// R471: the placement multiplier of a card that doubles nothing (the Plague Counters placed as written).
pub const PLAGUE_MULTIPLIER_NONE: i32 = 1;
/// R820: the extra runs of a player under no trigger multiplier (Meditative #9 Joint Filing, #10 Double
/// Counting): each of their hooks runs once, as written.
pub const TRIGGER_EXTRA_NONE: i32 = 0;

// ---- v0.2.0 constants: Core patches (R423, R426–R431) ----
/// R423: how many different effects a Radiant Call to Chaos rolls from its list — "Three different
/// random effects, resolved in the order listed" (Core #95's Radiant face since patch v0.2.0, and
/// Classic+ #73's). The base face rolls one.
pub const CALL_TO_CHAOS_RADIANT_EFFECTS: i32 = 3;

// ---- v0.2.0 constants: Classic+ cards #1–#39 (cards-plus-c) ----
/// R59, §8.7 C+ #32.3 Blade Storm: the printed round cap, "up to 30 times". The card reads its declared
/// `rounds` (R386), which the catalog prints as this and a Degrade or Upgrade moves on that card only.
pub const BLADE_STORM_ROUNDS: i32 = 30;
/// C+ #35 Rollback (R419): the most player-turns it goes back ("Choose 1, 2 or 3").
pub const ROLLBACK_MAX_TURNS: i32 = 3;
/// R419: the board snapshots the history keeps — this turn's and the ROLLBACK_MAX_TURNS before it.
pub const BOARD_HISTORY_DEPTH: i32 = ROLLBACK_MAX_TURNS + 1;

// ---- v0.2.0 constants: Classic+ cards #40–#78 and the AI cards (the cards-plus-d workstream) ----

// Classic+ #42 KY's Test (SPEC §8.7, R420, BUILD §2).
string_union! {
    /// R420: a KY's Test difficulty.
    pub enum KyTestDifficulty {
        Easy = "Easy",
        Medium = "Medium",
        Hard = "Hard",
    }
}

/// R420: the three difficulties, in the order the first prompt offers them.
pub const KY_TEST_DIFFICULTIES: &[KyTestDifficulty] = &[
    KyTestDifficulty::Easy,
    KyTestDifficulty::Medium,
    KyTestDifficulty::Hard,
];
/// R420: the options every problem shows, exactly one of them its answer.
pub const KY_TEST_OPTIONS: i32 = 4;
/// R420: the floor of bank problems per difficulty (the bank test's floor, in `jackioh-cards`).
pub const KY_TEST_MIN_PROBLEMS: i32 = 30;

/// `{ min, max }`: an inclusive range.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IntRange {
    pub min: i32,
    pub max: i32,
}

/// R420: an Easy problem is a + b with each addend in this range, inclusive.
pub const KY_TEST_EASY_ADDENDS: IntRange = IntRange { min: 10, max: 99 };
/// R420: an Easy problem's three wrong sums are a + b moved by three different ones of these.
pub const KY_TEST_EASY_MISSES: &[i32] = &[1, -1, 2, -2, 10, -10];
/// R2179003: since issue #2179 an Easy problem is a + b or a − b with each operand four digits, in
/// this range, inclusive (`KY_TEST_EASY_ADDENDS` and `KY_TEST_EASY_MISSES` stay v0.5.2's).
pub const KY_TEST_FOUR_DIGIT_OPERANDS: IntRange = IntRange { min: 1000, max: 9999 };
/// R2179003: its three wrong answers are the answer moved by three different ones of these, a slip in
/// one column of a four-digit sum or difference.
pub const KY_TEST_FOUR_DIGIT_MISSES: &[i32] = &[1, -1, 10, -10, 100, -100];

// Meditative+ #20 KY's Competition's question sequence (SPEC §8.9 row 20, R1184000).

/// R1184000: the most right answers one sequence counts; the question that brings the streak to it
/// is its last, and the Kenny is summoned at once.
pub const KY_COMPETITION_MAX: i32 = 30;
/// R1563006: the seconds a sequence has from its first question, in every hosted game (the server's
/// match, the practice host); then its host answers Stop for its player (R1184001's answer).
pub const KY_COMPETITION_SECONDS: i32 = 60;
/// R1184000: a question's two operands on the base face: three-digit numbers.
pub const KY_COMPETITION_OPERANDS: IntRange = IntRange { min: 100, max: 999 };
/// R1184000: a question's two operands on the Radiant face: one- and two-digit numbers.
pub const KY_COMPETITION_EASY_OPERANDS: IntRange = IntRange { min: 1, max: 99 };
/// R1184000: the base face's three wrong options are the answer moved by three different ones of
/// these, a slip in one column of a three-digit sum, never below 0.
pub const KY_COMPETITION_MISSES: &[i32] = &[1, -1, 10, -10, 100, -100];
/// R1184000: the Radiant face's wrong options, C+ #42's Easy misses (R580), never below 0.
pub const KY_COMPETITION_EASY_MISSES: &[i32] = KY_TEST_EASY_MISSES;

/// `KyTestReward.pool`: a pool of non-token cards of every set (R380).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KyTestRewardPool {
    pub tags: Option<&'static [Tag]>,
    /// Only ever `Rarity::Legendary`.
    pub rarity: Option<Rarity>,
    pub cost: Option<i32>,
}

/// `KyTestReward.count`: how many, or `"fill"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KyTestRewardCount {
    N(i32),
    /// As many as the hand has room for under the player's hand cap (`hand_cap_of`).
    Fill,
}

/// R420: one reward a difficulty may roll (BUILD §2 `KY_TEST_REWARDS`). `label` is the reward as the
/// card's text prints it, which the first prompt shows beside its difficulty. A reward is cards: the
/// one card the text names (`defId`: The Coin, KY's Gift), or random cards from a pool of non-token
/// cards of every set (`pool`, R380; never KY's Test itself, R387; repeats allowed, R60). `count` is
/// how many, or `"fill"`: as many as the hand has room for under the player's hand cap (`hand_cap_of`).
/// `costOverride` is the cost the text sets each card to ("which costs (0)", "which cost (1)").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KyTestReward {
    pub id: &'static str,
    pub label: &'static str,
    pub def_id: Option<&'static str>,
    pub pool: Option<KyTestRewardPool>,
    pub count: KyTestRewardCount,
    pub cost_override: Option<i32>,
}

/// `KY_TEST_REWARDS`'s shape: each difficulty's list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KyTestRewards {
    pub easy: &'static [KyTestReward],
    pub medium: &'static [KyTestReward],
    pub hard: &'static [KyTestReward],
}

impl KyTestRewards {
    /// `KY_TEST_REWARDS[difficulty]`.
    pub const fn of(&self, difficulty: KyTestDifficulty) -> &'static [KyTestReward] {
        match difficulty {
            KyTestDifficulty::Easy => self.easy,
            KyTestDifficulty::Medium => self.medium,
            KyTestDifficulty::Hard => self.hard,
        }
    }
}

const fn pool_of(
    tags: Option<&'static [Tag]>,
    rarity: Option<Rarity>,
    cost: Option<i32>,
) -> Option<KyTestRewardPool> {
    Some(KyTestRewardPool { tags, rarity, cost })
}

/// R420: each difficulty's list of rewards, in the order the text prints them; one is rolled per difficulty.
pub const KY_TEST_REWARDS: KyTestRewards = KyTestRewards {
    easy: &[
        KyTestReward {
            id: "coins",
            label: "3 The Coins",
            def_id: Some("core-t-coin"),
            pool: None,
            count: KyTestRewardCount::N(3),
            cost_override: None,
        },
        KyTestReward {
            id: "ky",
            label: "a random (2) Cost KY card",
            def_id: None,
            pool: pool_of(Some(&[Tag::Ky]), None, Some(2)),
            count: KyTestRewardCount::N(1),
            cost_override: None,
        },
        KyTestReward {
            id: "legendary",
            label: "a random Legendary card, which costs (0)",
            def_id: None,
            pool: pool_of(None, Some(Rarity::Legendary), None),
            count: KyTestRewardCount::N(1),
            cost_override: Some(0),
        },
        KyTestReward {
            id: "books",
            label: "2 random Books",
            def_id: None,
            pool: pool_of(Some(&[Tag::Book]), None, None),
            count: KyTestRewardCount::N(2),
            cost_override: None,
        },
    ],
    medium: &[
        KyTestReward {
            id: "fours",
            label: "2 random (4) Cost cards, which cost (1)",
            def_id: None,
            pool: pool_of(None, None, Some(4)),
            count: KyTestRewardCount::N(2),
            cost_override: Some(1),
        },
        KyTestReward {
            id: "books",
            label: "5 random Books",
            def_id: None,
            pool: pool_of(Some(&[Tag::Book]), None, None),
            count: KyTestRewardCount::N(5),
            cost_override: None,
        },
        KyTestReward {
            id: "ky",
            label: "5 random KY cards",
            def_id: None,
            pool: pool_of(Some(&[Tag::Ky]), None, None),
            count: KyTestRewardCount::N(5),
            cost_override: None,
        },
        KyTestReward {
            id: "fill",
            label: "fill your hand with random Books",
            def_id: None,
            pool: pool_of(Some(&[Tag::Book]), None, None),
            count: KyTestRewardCount::Fill,
            cost_override: None,
        },
    ],
    hard: &[KyTestReward {
        id: "gift",
        label: "KY's Gift",
        def_id: Some("classicplus-042-1"),
        pool: None,
        count: KyTestRewardCount::N(1),
        cost_override: None,
    }],
};

// Classic+ #62 KY's Papaya (SPEC §8.7, R422, BUILD §2).
/// R422: the rows of the curve's grid, y 0 (your backrow) to 3 (their backrow).
pub const PAPAYA_ROWS: i32 = 4;
/// R422: the most cells a curve is drawn through — four fix a cubic, the highest degree it may have.
pub const PAPAYA_MAX_CELLS: i32 = 4;
/// R831003: how many random cells each Discover of the curve offers (patch v0.3.10): the Radiant
/// face's, and both faces' through v0.4.0f.
pub const PAPAYA_OFFERS: i32 = 3;
/// R1704000: how many random cells each Discover of the curve offers on the base face.
pub const PAPAYA_BASE_OFFERS: i32 = 2;

// Classic+ #65 Two Grapes and #66 Vine of Grapes (SPEC §8.7, R382, BUILD §2).
/// One Grape and its chance in percent (`GRAPE_ODDS`).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct GrapeOdds {
    pub def_id: &'static str,
    pub percent: i32,
}

/// R960: one entry of any weighted roll table: the def id it names and its chance in percent. The
/// table runs from worst to best, so a roll with Lucky keeps the later of its draws.
pub type RollOdds = GrapeOdds;

/// R960: Auspicious Rock's table (Meditative #39.1, MD-C1): Dud 20, Jade 75, Red Jade 5 (patch
/// v0.3.10, #831; Jade 70 and Red Jade 10 before), ordered
/// worst to best (Dud < Jade < Red Jade). A distribution, so config and not params: the odds must
/// add to 100.
pub const AUSPICIOUS_ROCK_ODDS: &[RollOdds] = &[
    RollOdds {
        def_id: "meditative-039-3",
        percent: 20,
    },
    RollOdds {
        def_id: "meditative-039-2",
        percent: 75,
    },
    RollOdds {
        def_id: "meditative-039-4",
        percent: 5,
    },
];

/// R1106 (MD-D10): Meditative #54 Money Machine's hidden roll for the card it shuffles into its
/// controller's deck: The Coin 89, Jade 10, Red Jade 1, ordered worst to best. Unprinted, as R673's
/// Glitch odds are; a distribution, so config and not params: the odds must add to 100.
pub const MONEY_MACHINE_ODDS: &[RollOdds] = &[
    RollOdds {
        def_id: "core-t-coin",
        percent: 89,
    },
    RollOdds {
        def_id: "meditative-039-2",
        percent: 10,
    },
    RollOdds {
        def_id: "meditative-039-4",
        percent: 1,
    },
];

/// R1180003 (Meditative+ #7 Life Mage's Radiant face, issue #1180): "healing is twice as effective" —
/// what each such text acting on a player's side multiplies a heal of their hero or Units by. Several
/// multiply: two Radiant Life Mages make a heal four times as large.
pub const HEAL_MULTIPLIER: i32 = 2;

/// R1180006 (Meditative+ #17 Dogpile, issue #1180): answers one chain of R1180005 may name — the Spell
/// that started it answered, then each answer's own cast — before the next targeted resolution calls
/// nothing. A deck that refills itself with answering cards would otherwise answer forever.
pub const SPELL_ANSWER_CHAIN_CAP: i32 = 20;

/// R1870002 (Stinkyscape #4.2 KY's Last Exam, issue #1870): the most Echo repeats one play of a card may
/// give itself (`effects::gain_echo`). A run of right guesses is one in four each, but a Spell that gains
/// a repeat with every resolution would otherwise be owed one more for as long as the guesses held.
pub const SELF_ECHO_GAIN_CAP: i32 = 20;

/// R962: the Jade Counter a summon of Jade Beauty (Meditative #39.5) is crossed at (ME-JADE).
pub const JADE_BEAUTY_AT: i32 = 5;
/// R962: the Jade Counter every Jade Beauty its player controls is made Radiant at (ME-JADE).
pub const JADE_ASCEND_AT: i32 = 10;
/// R962: the definition ME-JADE summons and ascends (Meditative #39.5 Jade Beauty).
pub const JADE_BEAUTY_DEF_ID: &str = "meditative-039-5";

/// R382: each Grape's chance in percent, in Lucky's order from worst to best (Rotten < Normal < Large <
/// Golden < Mythic): a roll with Lucky keeps the later of its two.
pub const GRAPE_ODDS: &[GrapeOdds] = &[
    GrapeOdds {
        def_id: "classicplus-065-1",
        percent: 12,
    },
    GrapeOdds {
        def_id: "classicplus-065-2",
        percent: 60,
    },
    GrapeOdds {
        def_id: "classicplus-065-3",
        percent: 20,
    },
    GrapeOdds {
        def_id: "classicplus-065-4",
        percent: 7,
    },
    GrapeOdds {
        def_id: "classicplus-065-5",
        percent: 1,
    },
];

// Classic+ #73 Call to Chaos (Classic+ Edition) (SPEC §8.7 row 73, R423): its table's numbers.
/// §8.7 row 73 entry 1: "Add 5 random Fruits to your hand".
pub const CHAOS_PLUS_FRUITS: i32 = 5;
/// §8.7 row 73 entry 2: "Add 3 random Books to your hand".
pub const CHAOS_PLUS_BOOKS: i32 = 3;
/// §8.7 row 73 entry 4: "Add 3 random Classic cards to your hand".
pub const CHAOS_PLUS_CLASSIC_CARDS: i32 = 3;
/// §8.7 row 73 entries 1, 2, 4 and 9: the added and the replacing cards "cost (0)" (`costOverride`).
pub const CHAOS_PLUS_COST: i32 = 0;
/// §8.7 row 73 entry 5: "Buff every card in your hand and deck twice" (Buff is the engine's Upgrade, R1320).
pub const CHAOS_PLUS_UPGRADES: i32 = 2;
/// §8.7 row 73 entry 7: "Nerf every card on your opponent's field and in their hand three times" (Nerf is the
/// engine's Degrade, R1320).
pub const CHAOS_PLUS_DEGRADES: i32 = 3;

// Meditative #95 Call to Chaos (Meditative Edition) (SPEC §8.8 row 95, R423): its table's numbers.
/// §8.8 row 95 entry 1: "add 2 copies of it to your hand" (R1242).
pub const CHAOS_MED_FUSE_COPIES: i32 = 2;
/// §8.8 row 95 entry 2: "Add 2 random CN cards to your hand" (3 through patch v0.3.7b, #1149).
pub const CHAOS_MED_CN_CARDS: i32 = 2;
/// §8.8 row 95 entry 3: "Add 2 random Prime cards to your hand" (R1421).
pub const CHAOS_MED_PRIME_CARDS: i32 = 2;
/// §8.8 row 95 entry 4: "Your hero gains 6 Armor" (8 through patch v0.3.7b, #1149).
pub const CHAOS_MED_HERO_ARMOR: i32 = 6;
/// §8.8 row 95 entry 4: "and you heal it 6" (8 through patch v0.3.7b, #1149).
pub const CHAOS_MED_HEAL: i32 = 6;
/// §8.8 row 95 entry 6: "Summon 3 random Acclaimed cards" (R1243).
pub const CHAOS_MED_ACCLAIMED: i32 = 3;
/// §8.8 row 95 entry 7: "then Nerf each card bounced", once each (R1244).
pub const CHAOS_MED_NERFS: i32 = 1;
/// §8.8 row 95 entries 1, 2 and 3: the cards "cost (0)" (`costOverride`).
pub const CHAOS_MED_COST: i32 = 0;
/// §8.8 row 95 entry 9, R1241: the most rest-of-game "cast a random Call to Chaos" effects one player
/// holds; a roll of the entry for a player at the cap resolves into nothing.
pub const CALL_TO_CHAOS_ETERNAL_CAP: i32 = 3;

// Stinkyscape #150 Call to Chaos (Stinky Edition) and #151 (Scape Edition) (SPEC §8.10 rows 150 and
// 151, issue #1882): their tables' numbers, as printed.
/// §8.10 row 150 entry 2 and row 151 entries 5 and 8: the cards "cost (0)" (`costOverride`).
pub const CHAOS_STINKYSCAPE_COST: i32 = 0;
/// §8.10 row 150 entry 3: "summon 3 random Quest or Wincon Field Spells" (R1882001).
pub const CHAOS_STINKY_FIELD_SPELLS: i32 = 3;
/// §8.10 row 150 entry 8, R1882004: the most rest-of-game Acclaimed death rules one player holds, bounded
/// as M #95's entry 9 is (`CALL_TO_CHAOS_ETERNAL_CAP`, R1241); a roll at the cap resolves into nothing.
pub const CHAOS_STINKY_DEATH_WATCH_CAP: i32 = 3;
/// §8.10 row 150 entry 8, R2176001: the most Calls to Chaos the death watch casts for one player in one
/// turn, its terminator (REVIEW A3.6): each cast is a chain with R28's cap of its own, and a cast that
/// kills an Acclaimed Unit starts another, so an Acclaimed death past this many that turn casts nothing.
pub const CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN: i32 = 5;
/// §8.10 row 151 entry 2: "summon 3 Drone Corvettes".
pub const CHAOS_SCAPE_CORVETTES: i32 = 3;
/// §8.10 row 151 entry 4: "cast 2 different cards of" the four Systems (R1882006).
pub const CHAOS_SCAPE_SYSTEMS: i32 = 2;
/// §8.10 row 151 entry 5: "add 4 random Starship Pieces to your hand".
pub const CHAOS_SCAPE_HAND_PIECES: i32 = 4;
/// §8.10 row 151 entry 7: "fuse 4 random Starship Pieces into your Dormant Starship" (R1882008).
pub const CHAOS_SCAPE_FUSED_PIECES: i32 = 4;
/// §8.10 row 151 entry 8: "add 2 random Radiant Weapons to your hand".
pub const CHAOS_SCAPE_WEAPONS: i32 = 2;
/// §8.10 row 151 entry 9, R1882009: the Marketplace's Call to Chaos "for 20 Credits", a price that never
/// moves (R1853005).
pub const CHAOS_SCAPE_LOT_PRICE: i32 = 20;

// Training Grounds #39 Call to Chaos (Training-Grounds Edition) (SPEC §8.11 row 39, issue #2200): its
// table's numbers, as printed.
/// §8.11 row 39 entry 1: "Summon 3 Rush Tokens", base faces.
pub const CHAOS_TG_RUSH_TOKENS: i32 = 3;
/// §8.11 row 39 entry 2: "refresh 4 mana" (§6.3 Refresh, R364).
pub const CHAOS_TG_REFRESH: i32 = 4;
/// §8.11 row 39 entry 3: "add 4 random Training Grounds cards to your hand" (R2200000).
pub const CHAOS_TG_CARDS: i32 = 4;
/// §8.11 row 39 entry 4: "gain 8 mana", which may pass max (§2.3).
pub const CHAOS_TG_MANA: i32 = 8;
/// §8.11 row 39 entry 4: "and draw 2 cards".
pub const CHAOS_TG_DRAW: i32 = 2;
/// §8.11 row 39 entry 5: "summon 6 random Training Grounds permanents" (R2200000).
pub const CHAOS_TG_PERMANENTS: i32 = 6;
/// §8.11 row 39 entry 6: "add 2 random Radiant Training Grounds cards to your hand".
pub const CHAOS_TG_RADIANT_CARDS: i32 = 2;
/// §8.11 row 39 entry 7: "summon 5 Rush Tokens", base faces.
pub const CHAOS_TG_ARMY_TOKENS: i32 = 5;
/// §8.11 row 39 entries 3, 6 and 9: the cards "cost (0)" (`costOverride`).
pub const CHAOS_TG_COST: i32 = 0;
/// §8.11 row 39 entry 9, R2200002: "every second turn of yours": the entry adds on the caster's own
/// turns whose count since the entry resolved is a multiple of this.
pub const CHAOS_TG_EVERY: i32 = 2;
/// §8.11 row 39 entry 9, R2200002: the most rest-of-game every-second-turn adds one player holds, its
/// terminator (REVIEW A3.6), bounded as M #95's entry 9 is (`CALL_TO_CHAOS_ETERNAL_CAP`, R1241); a roll at
/// the cap resolves into nothing, the "now" add included.
pub const CHAOS_TG_ALTERNATE_CAP: i32 = 3;

// T-AI-4 Chain of Thought (SPEC §8.7, BUILD §2).
/// T-AI-4: the repeats after the first draw, a termination bound: five draws at most.
pub const CHAIN_OF_THOUGHT_REPEATS: i32 = 4;

// ---------------------------------------------------------------------------------------------
// Moved here in v0.3.0 (CLAUDE.md rule 9; research A §4): the numbers TS stated outside
// `config.ts`, under their TS names. Each names the module that reads it.
// ---------------------------------------------------------------------------------------------

/// §10.8, §10.10: how many of the most recent events the view carries for animation (`view_for`).
pub const VIEW_EVENT_LIMIT: usize = 32;

/// §7: a Radiant Sheep Token is "2/2, worth 3 Tributes" — the base one is worth 2 (`play_choices`).
pub const RADIANT_SHEEP_TRIBUTE_VALUE: i32 = 3;
pub const SHEEP_TRIBUTE_VALUE: i32 = 2;

/// §6.3 Discover: "choose 1 of 3" (`effects::choose`, `effects::last_board`, `subsystems::scorer`).
pub const DISCOVER_OPTIONS: i32 = 3;
/// C+ #29: "It costs (0)", "Each costs (0)" — a `costOverride` (R65) (`effects::last_board`).
pub const LAST_BOARD_CARD_COST: i32 = 0;

/// §4.5 loops until nothing changes; this bounds a pathological loop loudly (R69, R89) (`state_check`).
pub const STATE_CHECK_PASS_CAP: usize = 100;

/// A drain that keeps parking new work stops here rather than spinning (§2.5's cap spirit) (`work`).
pub const MAX_WORK_STEPS: usize = 500;

/// Enumerating answers is bounded like `legalActions`'s other combinations (R90) (`prompts`).
pub const MAX_PROMPT_ANSWERS: usize = 256;

/// The most keep-subsets `legalActions` lists for one mulligan (`reduce`).
pub const MAX_MULLIGAN_SUBSETS: usize = 256;

/// R77's smallest fusion, as `subsystems::fuse`'s `FUSE_MIN_INGREDIENTS` (`subsystems::last_boards`).
pub const FUSED_MIN_PARTS: usize = 2;

/// §8 #95: "summon 3 random 3-cost Units" (`subsystems::call_to_chaos`).
pub const CHAOS_UNIT_COUNT: i32 = 3;
pub const CHAOS_UNIT_COST: i32 = 3;
/// §8 #95: "heal your hero 30" — a hero heal has no cap (§6.3 Heal).
pub const CHAOS_HEAL: i32 = 30;
/// §8 #95: "draw your whole library and gain 4 mana".
pub const CHAOS_MANA: i32 = 4;
/// §8 #95: "add 3 random cards to hand costing 0".
pub const CHAOS_ADDED_CARDS: i32 = 3;
/// §8 #95 and §7: "summon five Radiant Rush Tokens".
///
/// They used to be a bespoke 5/5 through `statsOverride`, which made #95 the only card in the set
/// that invented a Rush Token size. It now summons the token's own RADIANT face (6/6, Rush), so
/// the stats live in one place — the catalog — and a later change to the radiant token moves this
/// card with it instead of leaving it behind.
pub const CHAOS_RUSH_TOKENS: i32 = 5;
/// §8 #95: "every card in your hand and library costs 2 less".
pub const CHAOS_COST_DISCOUNT: i32 = 2;
/// §8 #95: "summon 5 random Field Spells or Traps … into your backrow".
pub const CHAOS_BACKROW_CARDS: i32 = 5;

/// `SCORER_WEIGHTS`'s shape (`subsystems::scorer`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScorerWeights {
    /// "lethal available → max".
    pub lethal: f64,
    /// "can clear the enemy board → high".
    pub clears_board: f64,
    /// Partial credit: an answer to some of the enemy board is worth less than all of it.
    pub per_kill: f64,
    /// "hero below 10 and card heals → high".
    pub heal: f64,
    /// The fallback: printed stats per mana.
    pub stats_per_mana: f64,
    /// The fallback's second half: what the card leaves behind beyond this turn.
    pub draw_value: f64,
}

/// The weights, in one place so they can be tuned without touching the scoring functions (§10.7:
/// "the weights are engine constants, tested against fixed states"). The tests pin the ranking a
/// fixed state produces, never these numbers, so a tuning pass does not rewrite the test file.
///
/// The gaps are deliberate: the four priorities of §10.7 are a strict order, so no amount of stats
/// per mana can outrank a heal at low health, no heal can outrank a board clear, and nothing
/// outranks lethal.
pub const SCORER_WEIGHTS: ScorerWeights = ScorerWeights {
    lethal: 100_000.0,
    clears_board: 10_000.0,
    per_kill: 1_000.0,
    heal: 5_000.0,
    stats_per_mana: 10.0,
    draw_value: 25.0,
};

/// §10.7: the hero health below which a heal becomes a priority (`subsystems::scorer`).
pub const SCORER_LOW_HEALTH: i32 = 10;

/// How many of one candidate's plays the dry run tries. A play is tried at its strongest price (the
/// largest X, embiggened when that is affordable) in the first zone it could take, so what varies is
/// its targets and modes, taken heroes first (`targetRank`): the enemy hero is lethal's target, the
/// viewer's own a heal's, and the board's units come after them (`subsystems::scorer`).
pub const SCORER_DRY_RUN_PLAYS: usize = 8;

/// R753: Life Tap's "Draw 1" (on the Radiant face, from each player's deck) and its "Take 2 damage",
/// dealt to its own hero by the card (`subsystems::hero_power`).
pub const LIFE_TAP_DRAW: i32 = 1;
pub const LIFE_TAP_DAMAGE: i32 = 2;
/// R754: how far Steady Shot's Radiant face raises its declared number (`"shot"`) per use.
pub const STEADY_SHOT_RAISE: i32 = 2;
/// R756: Ping's damage.
pub const PING_DAMAGE: i32 = 1;
/// R757: Armor Up's Armor until your next turn, and the Armor Tank Up (its Radiant face) keeps.
pub const ARMOR_UP_ARMOR: i32 = 2;
pub const TANK_UP_ARMOR: i32 = 4;
/// R758: Die Insect's damage, and its Radiant face's Lucky.
pub const DIE_INSECT_DAMAGE: i32 = 8;
pub const DIE_INSECT_LUCKY: i32 = 1;
/// R759: how much KY Brainstorm takes off each Spell in your hand.
pub const BRAINSTORM_DISCOUNT: i32 = 1;
/// R760: what Pluck's Fruit costs.
pub const PLUCK_COST: i32 = 0;
/// R352: Stitching's "Discover two (2) Cost or less Units".
pub const STITCHING_INGREDIENTS: i32 = 2;
pub const STITCHING_MAX_COST: i32 = 2;

/// A playout covers one turn, so this ceiling is far above any reachable turn; it exists only so a
/// card that keeps refilling the hand cannot spin forever (BUILD M3-T7) (`subsystems::ai_policy`).
pub const AI_PLAYOUT_STEP_CAP: usize = 500;

/// E: where a Combo-Index starts (§8 #93) (`subsystems::combo_index`).
pub const FIRST_GRADE: i32 = 1;
/// Grade D: "2 different random hand cards cost 1 less" (§8 #93).
pub const GRADE_D_CARDS: i32 = 2;
pub const GRADE_D_DISCOUNT: i32 = 1;
/// Grade A: "8 damage to the enemy hero with Lifesteal" (§8 #93).
pub const GRADE_A_DAMAGE: i32 = 8;

/// R77: Craft a Card fuses "two or three cards", and #85 fuses two. Fewer is not a fusion
/// (`subsystems::fuse`).
pub const FUSE_MIN_INGREDIENTS: usize = 2;
/// R874000: the most cards one fusion holds, each fused ingredient counting the cards it was fused
/// from (`subsystems::fuse::fused_card_count`). Meditative #95 fuses a hand into one card and two
/// copies of it (R1242), so a hand of those three fused again holds three times as many, and nothing
/// else stops that growth; a fusion past this is not made, as R28 caps a Call to Chaos chain. A full
/// hand (`HAND_CAP`) fused once, then again with its two copies and seven more cards, holds 37.
pub const FUSED_CARDS_CAP: usize = 64;

/// §8 #99: "the result costs 0 and goes to your hand", as a `costOverride` per R65 (`subsystems::fuse`).
pub const CRAFTED_CARD_COST: i32 = 0;

// ---------------------------------------------------------------------------------------------
// Meditative #42's night market (ME-MARKET, `subsystems::night_market`).
// ---------------------------------------------------------------------------------------------

/// R1000: a lot's yuan per mana of its printed cost out of play (R65: X counts 0, an embiggen card
/// its base price).
pub const YUAN_PER_COST: i32 = 10;
/// R1000: a lot's yuan per step of its rarity's rank (`YUAN_RARITY_RANKS`).
pub const YUAN_PER_RARITY: i32 = 5;
/// R1000, R1001: a Radiant card's price, bought or bartered, is this many times its plain one.
pub const YUAN_RADIANT_MULTIPLIER: i32 = 2;
/// R1000: each rarity's rank, Common 1 to Mythic 5. A token is ranked by its printed rarity, and one
/// that prints none (the AI generated cards) as Common.
pub const YUAN_RARITY_RANKS: &[(Rarity, i32)] = &[
    (Rarity::Common, 1),
    (Rarity::Rare, 2),
    (Rarity::Epic, 3),
    (Rarity::Legendary, 4),
    (Rarity::Mythic, 5),
];
/// R1000: the stall's random CN cards, all different.
pub const NIGHT_MARKET_CN_LOTS: i32 = 3;
/// R1000: the stall's Auspicious Rocks (M #39.1).
pub const NIGHT_MARKET_ROCK_LOTS: i32 = 2;
/// R1000: the stall's random AI generated card.
pub const NIGHT_MARKET_AI_LOTS: i32 = 1;

// ---------------------------------------------------------------------------------------------
// Stinkyscape's Resources and Credits (#1840, `resources`).
// ---------------------------------------------------------------------------------------------

/// R1840002: "Gain a random Resource" rolls a rarity by these weights (Common 55, Rare 25, Epic 14,
/// Legendary 5, Mythic 1, out of 100), over the rarities the pool holds, then a Resource of that
/// rarity uniformly.
pub const RESOURCE_RARITY_ODDS: &[(PrintedRarity, i32)] = &[
    (PrintedRarity::Common, 55),
    (PrintedRarity::Rare, 25),
    (PrintedRarity::Epic, 14),
    (PrintedRarity::Legendary, 5),
    (PrintedRarity::Mythic, 1),
];

// ---------------------------------------------------------------------------------------------
// Stinkyscape's Marketplace (#1853, R1853000–R1853005; docs/stinkyscape-set.md ST3).
// ---------------------------------------------------------------------------------------------

/// R1853001: what one lot of `MarketLot` draws its cards from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketPool {
    /// The ids named, those the catalog holds.
    Ids(&'static [&'static str]),
    /// Every card of the set that is no token, in catalog order.
    Set(SetName),
    /// R1881006: every card of the set that is no token and prints the keyword on its base face, in
    /// catalog order (S21's two Starship Pieces, `query::starship_pieces()`'s pool).
    SetKeyword(SetName, KeywordKind),
}

/// R1853001, R2250002: one lot of the Marketplace's table: a pool and its `count`, the share of a
/// player's offers its cards take (its weight in each draw).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarketLot {
    pub pool: MarketPool,
    pub count: i32,
}

/// R1881006: the twelve v3–v5 Weapons (Stinkyscape #50 to #61), one lot of S21's table.
pub const MARKETPLACE_WEAPONS: &[&str] = &[
    "stinkyscape-050",
    "stinkyscape-051",
    "stinkyscape-052",
    "stinkyscape-053",
    "stinkyscape-054",
    "stinkyscape-055",
    "stinkyscape-056",
    "stinkyscape-057",
    "stinkyscape-058",
    "stinkyscape-059",
    "stinkyscape-060",
    "stinkyscape-061",
];
/// R1881006: both Warp Disruptors (Stinkyscape #23, #24).
pub const MARKETPLACE_WARP_DISRUPTORS: &[&str] = &["stinkyscape-023", "stinkyscape-024"];
/// R1881006: Scanning Probe Mk1 (Stinkyscape #166), a lot of its own.
pub const MARKETPLACE_PROBE_MK1: &[&str] = &["stinkyscape-166"];
/// R1881006: Scanning Probe Mk2 (Stinkyscape #167), a lot of its own.
pub const MARKETPLACE_PROBE_MK2: &[&str] = &["stinkyscape-167"];
/// R1881006: the ten Drone Cores (Stinkyscape #27 to #36).
pub const MARKETPLACE_DRONE_CORES: &[&str] = &[
    "stinkyscape-027",
    "stinkyscape-028",
    "stinkyscape-029",
    "stinkyscape-030",
    "stinkyscape-031",
    "stinkyscape-032",
    "stinkyscape-033",
    "stinkyscape-034",
    "stinkyscape-035",
    "stinkyscape-036",
];
/// R1881006: the four Launchers (Stinkyscape #140 to #143).
pub const MARKETPLACE_LAUNCHERS: &[&str] = &[
    "stinkyscape-140",
    "stinkyscape-141",
    "stinkyscape-142",
    "stinkyscape-143",
];
/// R1881006: the ten AI cards (Stinkyscape T-AI-1 to T-AI-10, ST15).
pub const MARKETPLACE_AI_CARDS: &[&str] = &[
    "stinkyscape-t-ai-01",
    "stinkyscape-t-ai-02",
    "stinkyscape-t-ai-03",
    "stinkyscape-t-ai-04",
    "stinkyscape-t-ai-05",
    "stinkyscape-t-ai-06",
    "stinkyscape-t-ai-07",
    "stinkyscape-t-ai-08",
    "stinkyscape-t-ai-09",
    "stinkyscape-t-ai-10",
];
/// R1881006: one of S21's lots of `count` cards of `pool`.
const fn card_lot(pool: MarketPool, count: i32) -> MarketLot {
    MarketLot { pool, count }
}

/// R1853001, R1881006, R2250002: the Marketplace's table of lots, the set's own (S21) in the order ST3
/// lists them, less its Resources' lot, which #2250 took out: a player's offers draw from it at the
/// start of their turn, each lot weighted by its count. A test registers a fixture table in its place
/// (`testkit::register_marketplace_table`).
pub const MARKETPLACE_TABLE: &[MarketLot] = &[
    card_lot(MarketPool::Ids(MARKETPLACE_WEAPONS), 3),
    card_lot(MarketPool::Ids(MARKETPLACE_WARP_DISRUPTORS), 2),
    card_lot(MarketPool::Ids(MARKETPLACE_PROBE_MK1), 1),
    card_lot(MarketPool::Ids(MARKETPLACE_PROBE_MK2), 1),
    card_lot(MarketPool::Ids(MARKETPLACE_DRONE_CORES), 2),
    card_lot(MarketPool::Ids(MARKETPLACE_LAUNCHERS), 1),
    card_lot(
        MarketPool::SetKeyword(SetName::Stinkyscape, KeywordKind::StarshipPiece),
        2,
    ),
    card_lot(MarketPool::Set(SetName::Stinkyscape), 2),
    card_lot(MarketPool::Ids(MARKETPLACE_AI_CARDS), 1),
];
/// R2250002: the offers the Marketplace rolls a player at the start of each of their turns.
pub const MARKETPLACE_OFFERS_PER_TURN: i32 = 3;
/// R2250002: the copies of each rolled offer, its owner's to buy.
pub const MARKETPLACE_OFFER_STOCK: i32 = 1;
/// R1853002: the Credits a lot asks for each mana of its cost up to `MARKETPLACE_FULL_PRICE_MANA`.
pub const MARKETPLACE_CREDITS_PER_MANA: i32 = 10;
/// R1853002: the cost up to which each mana is priced at `MARKETPLACE_CREDITS_PER_MANA`.
pub const MARKETPLACE_FULL_PRICE_MANA: i32 = 4;
/// R1853002: the Credits a lot asks for each mana of its cost past `MARKETPLACE_FULL_PRICE_MANA`.
pub const MARKETPLACE_CREDITS_PER_MANA_PAST: i32 = 5;
/// R1853002: the price of a (0) card.
pub const MARKETPLACE_ZERO_COST_PRICE: i32 = 5;
/// R1853001: the chance a lot is rolled Radiant.
pub const MARKETPLACE_RADIANT_CHANCE: f64 = 0.2;
/// R1853002: a Radiant lot's price, as a multiple of its base face's.
pub const MARKETPLACE_RADIANT_PRICE_MULTIPLIER: i32 = 2;
/// R1853002: a price moves each turn within a tenth of its base: the base divided by this, rounded down.
pub const MARKETPLACE_PRICE_SPREAD_DIVISOR: i32 = 10;
/// R1853005, R2250002: the copies a lot an effect added for the rest of the game holds, renewed at its
/// owner's turn start.
pub const MARKETPLACE_FIXED_LOT_STOCK: i32 = 1;
/// R1861000: the quote range a card that sells from the hand (`handSale`, Stinkyscape #89 Spice) draws
/// each turn, its base face's: "its price is determined like all other Resources", so a Rare Resource's,
/// Gellium's. `buy` is never read: such a card is no Resource and its lot, if any, is priced as a card.
pub const HAND_SALE_PRICE: ResourcePrice = ResourcePrice {
    sell_min: 3,
    sell_max: 10,
    buy: 0,
};
/// R1861000: `HAND_SALE_PRICE` on the Radiant face, a Radiant Rare Resource's.
pub const HAND_SALE_RADIANT_PRICE: ResourcePrice = ResourcePrice {
    sell_min: 6,
    sell_max: 20,
    buy: 0,
};

// ---------------------------------------------------------------------------------------------
// Stinkyscape #148 Alteran Reach's starscape map (#1847, R1847000–R1847002,
// `subsystems::starscape`).
// ---------------------------------------------------------------------------------------------

/// R1847000, R1847001: a starscape map's nodes, the edge the last: the travels from the start to the
/// edge of the universe.
pub const STARSCAPE_EDGE_TRAVELS: i32 = 20;
/// R1847002: a travel's mana, the Radiant face's first each turn free.
pub const STARSCAPE_TRAVEL_MANA: i32 = 1;
/// R1847000, R1847001: the rewards a node before the edge may hold, one entry each, drawn uniformly,
/// and each one's amount: the health healed, the cards drawn, the mana gained, the damage dealt.
pub const STARSCAPE_REWARDS: &[(StarscapeReward, i32)] = &[
    (StarscapeReward::Heal, 3),
    (StarscapeReward::Draw, 1),
    (StarscapeReward::Mana, 1),
    (StarscapeReward::Damage, 2),
];

// ---------------------------------------------------------------------------------------------
// The AI training lanes (SURFACE §14.2; docs/v0.3.0/README.md §8).
// ---------------------------------------------------------------------------------------------

/// R1679000: a training lane's promotion gate (SURFACE §14.2), a sequential probability ratio test
/// on paired hold-out deals. `cargo jackioh promote` plays deal after deal against the parent, the AI
/// on `main`, each twice with the seats swapped (R850000), and reads each deal's edge for the
/// candidate: its wins less the parent's over the two games, halved, from −1 to 1 (a draw, or a game
/// without a result, is nobody's win). The test weighs H0, a true mean edge of `edge_h0` (a successor
/// no stronger than its parent), against H1, `edge_h1` (the smallest gain worth banking), by the log
/// of their likelihood ratio, read as two normal laws with the deals' own variance: after `n` deals of
/// edges summing to `S`, `(edge_h1 − edge_h0) · (S − n · (edge_h0 + edge_h1) / 2) / σ²`, the
/// variance `σ²` the deals' own, never below `variance_floor`. From deal `min_deals` on it stops at
/// the first deal whose ratio reaches `ln((1 − beta) / alpha)`, a pass, or falls to
/// `ln(beta / (1 − alpha))`, a fail; one still between the two at `max_deals` fails. So an equal
/// successor passes about `alpha` of the time and one at H1 fails about `beta` of it, and a clear
/// loser stops early. The deals are played `batch_deals` at a time and read in deal order, so where it
/// stops never depends on the thread count, and CI's `promote --verify` plays the same deals again.
///
/// The lanes play no games against SPEC §10.7's random policy: they dropped them once generation 9,
/// the first AI dealt the Meditative set (issue #732), had beaten random in at least 90 of 100
/// (R732001). The daily quality gates (`cargo jackioh gate --full`) still hold every AI on `main` to
/// their floor against random.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrainingGate {
    /// The deals the test reads before it may stop: the variance it reads needs this many.
    pub min_deals: i32,
    /// The most deals it plays; a test undecided at the last one fails.
    pub max_deals: i32,
    /// The deals played at once (on rayon's pool) before the test reads them.
    pub batch_deals: i32,
    /// H0's mean edge: a successor no stronger than its parent.
    pub edge_h0: f64,
    /// H1's mean edge: the gain the gate is built to bank, +0.06 (a 53% win rate, about +21 Elo).
    pub edge_h1: f64,
    /// The chance it promotes a successor at H0.
    pub alpha: f64,
    /// The chance it fails one at H1.
    pub beta: f64,
    /// The least variance the ratio divides by: deals that all come out alike (an AI against
    /// itself splits every deal) have none.
    pub variance_floor: f64,
}

/// R1679000: every lane's gate, the `improve`, `unban` and `learn` lanes' alike (the unban lane adds
/// strictly fewer shadow bans than the parent's). On #1058's paired deals, whose edges had a variance
/// of about 0.27 (of 200 deals, 25 won twice, 146 split and 29 lost twice), it promotes an equal
/// successor about 4.5% of the time and one that truly wins 52%, 53% or 55% about 60%, 91% or all of
/// it, where 220 of 400 (R850000) let those through about 6%, 15% and 53% of the time; it averages
/// about 800 games at H0 and H1, 1,050 between them, and 300 for a clear loser, never more than 2,000.
pub const TRAINING_GATE: TrainingGate = TrainingGate {
    min_deals: 100,
    max_deals: 1000,
    batch_deals: 100,
    edge_h0: 0.0,
    edge_h1: 0.06,
    alpha: 0.05,
    beta: 0.05,
    variance_floor: 0.01,
};

// ---------------------------------------------------------------------------------------------
// Replays (R768, issue #508).
// ---------------------------------------------------------------------------------------------

/// R768: a replay keeps the state every this many accepted actions, so reaching any step costs at
/// most this many `reduce` calls, and so does a page (`replay::replay_page`).
pub const REPLAY_CHECKPOINT_EVERY: usize = 16;
/// R768: the most steps one page of a replay gives.
pub const REPLAY_PAGE_STEPS: usize = 16;

/// R1371000: the scheme a file's catalog versions are named by, its `versions` marker. A replay,
/// game-record or self-play file that carries no marker names them as they were before the version
/// retcon (#1371), and its reader maps each through `patches/aliases.json`. R2532000: 3 since the
/// v0.5 retcon (#2532); a file marked 2 names the v0.5 line as it was before it, and its reader maps
/// those names through the same file's `marked`.
pub const VERSIONS_SCHEME: u32 = 3;

// ---------------------------------------------------------------------------------------------
// The testkit's seed search (issue #848).
// ---------------------------------------------------------------------------------------------

/// Issue #848: how many seeds (`<prefix>-0` … `<prefix>-N-1`) or rng cursors (`0..N`) the testkit's
/// `seed_where` and `cursor_where` try before they panic. A test that needs a pool roll to land one way
/// searches for it, so a card or keyword joining the pool moves the position it finds and breaks nothing;
/// a roll that takes more than a few hundred tries is built directly instead of widening this bound.
pub const SEED_SEARCH_BOUND: usize = 4000;

// ---------------------------------------------------------------------------------------------
// ME-CRAFT (Meditative #17 True Craft a Card, docs/meditative-set.md M6 #17, R880–R883).
// ---------------------------------------------------------------------------------------------

/// The highest cost a crafted card may have (0 to 4).
pub const CRAFT_MAX_COST: i32 = 4;
/// How many seeded presets a `craft` prompt offers: a Unit, a Spell, a Field Spell and a Trap; in
/// the potion frame, this many Potions (R1182000).
pub const CRAFT_PRESETS: usize = 4;
/// The point budget is this base plus this per point of the chosen cost (2, 7, 12, 17, 22).
pub const CRAFT_POINTS_BASE: i32 = 2;
pub const CRAFT_POINTS_PER_MANA: i32 = 5;
/// The most lines of code a recipe may use, frame included (the catalog's 75th percentile).
pub const CRAFT_LOC_BUDGET: i32 = 24;
/// The `pub const ID` / `pub fn script()` / `CardScripts` frame every card file carries
/// (Core #8's `loc` 3).
pub const CRAFT_LOC_SKELETON: i32 = 3;
/// The most effect blocks one recipe may hold.
pub const CRAFT_MAX_EFFECTS: usize = 8;
/// R1563007: a `craft` prompt's offer (Meditative #17, Meditative+ #16 and #16.1; not Scratch Pad's):
/// this many keywords, hats and effect verbs, drawn from the match rng among those its frame may take,
/// and the only blocks its answer may use.
pub const CRAFT_OFFER_KEYWORDS: usize = 4;
pub const CRAFT_OFFER_HATS: usize = 2;
pub const CRAFT_OFFER_VERBS: usize = 3;
/// R1563009: the verbs no recipe may use any more — "give your hero Armor" since #1563. Each keeps its
/// price row, so a recipe an older game crafted still prices and compiles.
pub const CRAFT_RETIRED_VERBS: &[CraftVerb] = &[CraftVerb::GainArmor];
/// R881: the points a Unit preset must hold before it spends a 2-point keyword, so at least as many
/// are left for its stats.
pub const CRAFT_PRESET_KEYWORD_MIN_POINTS: i32 = 4;
/// R881: the health every preset recipe starts from before its points are spent.
pub const CRAFT_PRESET_BASE_HEALTH: i32 = 1;
/// The range of every block number: 1 to 10.
pub const CRAFT_MAX_N: i32 = 10;
/// The Radiant face doubles every number and stat (R275 by construction).
pub const CRAFT_RADIANT_MULTIPLIER: i32 = 2;
/// What one Echo costs, per N.
pub const CRAFT_ECHO_POINTS_PER_N: i32 = 4;
/// What a crafted definition's id starts with, before the recipe's digest (R882).
pub const CRAFT_ID_PREFIX: &str = "craft:";
/// The first words of a crafted card's name; the designer may replace them (R883).
pub const CRAFT_ADJECTIVES: &[&str] = &[
    "Pure",
    "Seeded",
    "Async",
    "Recursive",
    "Generic",
    "Static",
    "Lazy",
    "Mutable",
];
/// The last words of a crafted card's name; the designer may replace them (R883).
pub const CRAFT_NOUNS: &[&str] = &[
    "Closure", "Reducer", "Iterator", "Trait", "Crate", "Struct", "Macro", "Enum",
];

/// The point budget for the chosen cost: `CRAFT_POINTS_BASE + CRAFT_POINTS_PER_MANA × cost`.
pub fn craft_points_budget(cost: i32) -> i32 {
    CRAFT_POINTS_BASE + CRAFT_POINTS_PER_MANA * cost
}

/// One row of the verb price table (`docs/meditative-set.md` M6 #17): `points + per_n × N`,
/// halved (rounded up) when `halve` is set. `lines` counts the `TargetDecl` line a targeted
/// verb's declaration adds. In table order.
pub const CRAFT_VERB_PRICES: &[CraftVerbPrice] = &[
    CraftVerbPrice {
        verb: CraftVerb::DamageTarget,
        points: 0,
        per_n: 1,
        halve: false,
        lines: 2,
        targeted: true,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::DamageEnemyHero,
        points: 0,
        per_n: 1,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::DamageRandomEnemy,
        points: 0,
        per_n: 1,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::DamageEachEnemy,
        points: 0,
        per_n: 3,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::HealTarget,
        points: 0,
        per_n: 1,
        halve: true,
        lines: 2,
        targeted: true,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::HealYourHero,
        points: 0,
        per_n: 1,
        halve: true,
        lines: 1,
        targeted: false,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::HealYourSide,
        points: 0,
        per_n: 1,
        halve: false,
        lines: 2,
        targeted: false,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::Draw,
        points: 0,
        per_n: 3,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::GainMana,
        points: 0,
        per_n: 3,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::GainArmor,
        points: 0,
        per_n: 1,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::SummonRush,
        points: 5,
        per_n: 0,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: false,
    },
    CraftVerbPrice {
        verb: CraftVerb::SummonFelinor,
        points: 2,
        per_n: 0,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: false,
    },
    CraftVerbPrice {
        verb: CraftVerb::SummonSheep,
        points: 2,
        per_n: 0,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: false,
    },
    CraftVerbPrice {
        verb: CraftVerb::BuffTarget,
        points: 0,
        per_n: 2,
        halve: false,
        lines: 2,
        targeted: true,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::BuffYourUnits,
        points: 0,
        per_n: 4,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::GrantKeyword,
        points: 0,
        per_n: 0,
        halve: false,
        lines: 2,
        targeted: true,
        takes_n: false,
    },
    CraftVerbPrice {
        verb: CraftVerb::Destroy,
        points: 9,
        per_n: 0,
        halve: false,
        lines: 2,
        targeted: true,
        takes_n: false,
    },
    CraftVerbPrice {
        verb: CraftVerb::Bounce,
        points: 4,
        per_n: 0,
        halve: false,
        lines: 2,
        targeted: true,
        takes_n: false,
    },
    CraftVerbPrice {
        verb: CraftVerb::AddRandom,
        points: 4,
        per_n: 0,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: false,
    },
    CraftVerbPrice {
        verb: CraftVerb::Discover,
        points: 5,
        per_n: 0,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: false,
    },
    CraftVerbPrice {
        verb: CraftVerb::OpponentDiscards,
        points: 0,
        per_n: 3,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: true,
    },
    CraftVerbPrice {
        verb: CraftVerb::BuffRandomCard,
        points: 3,
        per_n: 0,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: false,
    },
    CraftVerbPrice {
        verb: CraftVerb::NerfRandomEnemyCard,
        points: 3,
        per_n: 0,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: false,
    },
    CraftVerbPrice {
        verb: CraftVerb::LockRandomZone,
        points: 2,
        per_n: 0,
        halve: false,
        lines: 1,
        targeted: false,
        takes_n: false,
    },
];

/// One row of the keyword price table, in palette order. Armor costs `per_n × N`; `spell` marks
/// the keywords a Spell may take (R883).
pub const CRAFT_KEYWORD_PRICES: &[CraftKeywordPrice] = &[
    CraftKeywordPrice {
        kind: KeywordKind::Taunt,
        points: 2,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Rush,
        points: 2,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::FirstStrike,
        points: 2,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Cleave,
        points: 2,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Trample,
        points: 2,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Pierce,
        points: 2,
        per_n: 0,
        spell: true,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Lifesteal,
        points: 2,
        per_n: 0,
        spell: true,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Deft,
        points: 1,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::DivineShield,
        points: 3,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Reborn,
        points: 3,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Charge,
        points: 4,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Poisonous,
        points: 4,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Windfury,
        points: 4,
        per_n: 0,
        spell: false,
    },
    CraftKeywordPrice {
        kind: KeywordKind::Armor,
        points: 0,
        per_n: 2,
        spell: false,
    },
];

/// One row of the hat price table: `lines` for the hook, `multiplier` on the points of the
/// effects under it, and the card types that may take it (R883).
pub const CRAFT_HAT_PRICES: &[CraftHatPrice] = &[
    CraftHatPrice {
        hat: CraftHatKind::Cry,
        lines: 2,
        multiplier: 1,
        types: &[CardType::Unit, CardType::FieldSpell],
    },
    CraftHatPrice {
        hat: CraftHatKind::Death,
        lines: 2,
        multiplier: 1,
        types: &[CardType::Unit],
    },
    CraftHatPrice {
        hat: CraftHatKind::StartOfTurn,
        lines: 2,
        multiplier: 2,
        types: &[CardType::Unit, CardType::FieldSpell],
    },
    CraftHatPrice {
        hat: CraftHatKind::EndOfTurn,
        lines: 2,
        multiplier: 2,
        types: &[CardType::Unit, CardType::FieldSpell],
    },
    CraftHatPrice {
        hat: CraftHatKind::WhenCast,
        lines: 2,
        multiplier: 1,
        types: &[CardType::Spell],
    },
    CraftHatPrice {
        hat: CraftHatKind::OpponentPlaysUnit,
        lines: 4,
        multiplier: 1,
        types: &[CardType::Trap],
    },
    CraftHatPrice {
        hat: CraftHatKind::OpponentPlaysSpell,
        lines: 4,
        multiplier: 1,
        types: &[CardType::Trap],
    },
    CraftHatPrice {
        hat: CraftHatKind::OpponentAttacks,
        lines: 4,
        multiplier: 1,
        types: &[CardType::Trap],
    },
    // R1182000: a Potion's one hat until #1563, "Activate: Tribute this.", an `ActivationDecl` with
    // its Tribute cost (as many lines as a reveal hat's trigger). No type takes it: a Potion is a
    // Spell cast When cast since #1563 (R1563008), and the row stays so an old recipe still prices.
    CraftHatPrice {
        hat: CraftHatKind::Activate,
        lines: 4,
        multiplier: 1,
        types: &[],
    },
];

/// R1182000: a Potion's fixed cost (Meditative+ #16 Jazakus: "It costs (1).").
pub const CRAFT_POTION_COST: i32 = 1;
/// R1563008: a Potion's points. A Potion is a Spell with no stats since #1563, so it has fewer than
/// the crafter's budget at its cost (`craft_points_budget(CRAFT_POTION_COST)`, 7), which a Unit
/// Potion spent mostly on stats.
pub const CRAFT_POTION_POINTS: i32 = 4;
/// R1182001: a HUGE Potion's fixed cost (Meditative+ #16.1 Jazakus Prime: "It costs (3).").
pub const CRAFT_HUGE_POTION_COST: i32 = 3;
/// R1182001: a HUGE Potion has this many times a Potion's points ("about 5 times better than the
/// normal custom potion"); its lines, effects and numbers are the frame's, as a Potion's.
pub const CRAFT_HUGE_POTION_BUDGET_MULTIPLE: i32 = 5;

// ---------------------------------------------------------------------------------------------
// Meditative #40 Feng Shui (R980, R981, R985). Hidden numbers: printed nowhere, never tuned, and not
// exported to the client.
// ---------------------------------------------------------------------------------------------

/// R985: the hit a punished play's player takes from the Feng Shui that judged it, once the play has
/// resolved: 10 from the base face, 20 from the Radiant face ("punished heavily", D10).
pub const FENG_SHUI_DAMAGE: ByFace = ByFace {
    base: 10,
    radiant: 20,
};
/// R985: the Brittle count a punished play is given at §10.5 step 3 (a given count, R638).
pub const FENG_SHUI_BRITTLE: i32 = 2;
/// R980: an index's last digit, 0–9, to its Hetu (河图) element: 1 and 6 水, 2 and 7 火, 3 and 8 木,
/// 4 and 9 金, 5 and 0 土.
pub const HETU_ELEMENTS: [CardElement; 10] = [
    CardElement::Earth,
    CardElement::Water,
    CardElement::Fire,
    CardElement::Wood,
    CardElement::Metal,
    CardElement::Earth,
    CardElement::Water,
    CardElement::Fire,
    CardElement::Wood,
    CardElement::Metal,
];
/// R980: the element of an index with no digit (T-rush, T-coin and the other named tokens): 土, the
/// center.
pub const ELEMENT_WITHOUT_DIGIT: CardElement = CardElement::Earth;
/// R981: the generating cycle, each element generating the next (木→火→土→金→水→木).
pub const GENERATING_CYCLE: [CardElement; 5] = [
    CardElement::Wood,
    CardElement::Fire,
    CardElement::Earth,
    CardElement::Metal,
    CardElement::Water,
];
/// R981: the overcoming cycle, each element overcoming the next (木→土→水→火→金→木).
pub const OVERCOMING_CYCLE: [CardElement; 5] = [
    CardElement::Wood,
    CardElement::Earth,
    CardElement::Water,
    CardElement::Fire,
    CardElement::Metal,
];

/// R923002: the most health the practice sandbox's tools may set a hero to (`sandbox.rs`). A hero's
/// health is set at least to 1: the tools arrange a board, and a hero at 0 would end the game.
pub const SANDBOX_HERO_HEALTH_MAX: i32 = 99;
/// R923002: the most Armor the sandbox's tools may give a hero.
pub const SANDBOX_ARMOR_MAX: i32 = 99;
/// R923002: the most mana, current or maximum, the sandbox's tools may set. More than `MAX_MANA`, so a
/// board that needs a big turn can be tried; the next refresh reads the maximum as it reads any other.
pub const SANDBOX_MANA_MAX: i32 = 10;
/// R923002: the most cards one of the sandbox's draws takes.
pub const SANDBOX_DRAW_MAX: i32 = 10;

// ---------------------------------------------------------------------------------------------
// Custom cards (#915, SPEC §12, R915000–R915009)
// ---------------------------------------------------------------------------------------------

/// R915000: what every custom card's id begins with. No catalog id does.
pub const CUSTOM_ID_PREFIX: &str = "custom-";
/// R915000: the longest custom id, the prefix included: room for the server's 32 hex digits and more.
pub const CUSTOM_ID_MAX_LENGTH: usize = 64;
/// R915001: the longest custom card name, in characters, as a deck's (`DECK_NAME_MAX_LENGTH`).
pub const CUSTOM_NAME_MAX_LENGTH: usize = 40;
/// R915001: the longest custom card text, in characters: two lines of a card face.
pub const CUSTOM_TEXT_MAX_LENGTH: usize = 240;
/// R915001: the most a custom card costs, as the catalog's dearest cards (mana past `MAX_MANA` is gained).
pub const CUSTOM_COST_MAX: i32 = 10;
/// R915001: the highest attack a custom Unit prints.
pub const CUSTOM_ATTACK_MAX: i32 = 12;
/// R915001: the highest health a custom Unit prints.
pub const CUSTOM_HEALTH_MAX: i32 = 12;
/// R915001: the highest attack or health a custom Unit's Radiant face prints, twice the base caps.
pub const CUSTOM_RADIANT_STAT_MAX: i32 = 24;
/// R915001: a custom Unit's Radiant stat the frame leaves unprinted is twice the base one (`custom::radiant_stat`).
pub const CUSTOM_RADIANT_STAT_FACTOR: i32 = 2;
/// R915001: the most keywords a custom Unit prints.
pub const CUSTOM_KEYWORDS_MAX: usize = 4;
/// R915001: the keywords a custom Unit may print, and the card kit may give: the game's own simple
/// combat keywords, none of which takes a number.
pub const CUSTOM_KEYWORDS: &[KeywordKind] = &[
    KeywordKind::Taunt,
    KeywordKind::Rush,
    KeywordKind::Charge,
    KeywordKind::FirstStrike,
    KeywordKind::Poisonous,
    KeywordKind::Lifesteal,
    KeywordKind::Reborn,
    KeywordKind::DivineShield,
    KeywordKind::Trample,
    KeywordKind::Windfury,
    KeywordKind::Ranged,
    KeywordKind::Flying,
];
/// R915001: the most numbers (`{key}`s) a custom card prints.
pub const CUSTOM_PARAMS_MAX: usize = 6;
/// R915001: the longest name of a custom card's number.
pub const CUSTOM_PARAM_KEY_MAX_LENGTH: usize = 16;
/// R915001: the highest value a custom card's number prints, on either face.
pub const CUSTOM_PARAM_MAX: i32 = 20;
/// R915001: the longest card-kit script, in bytes of UTF-8.
pub const CUSTOM_SOURCE_MAX_BYTES: usize = 8_192;
/// R915001: the longest builder's model a custom card keeps, in bytes of JSON.
pub const CUSTOM_BUILDER_MAX_BYTES: usize = 16_384;

// ---------------------------------------------------------------------------------------------
// Duos co-op (#936, fullsend SPEC S1): two 1v1 duels side by side whose teams share a life pool and
// pass cards to each other (`crate::duos`, `crate::duos_life`).
// ---------------------------------------------------------------------------------------------

/// B12: each Duos player's own HP, its duel hero's health.
pub const DUOS_OWN_HEALTH: i32 = 20;
/// B12: a team's shared pool, which its seats draw on once their own HP is spent (B13).
pub const DUOS_POOL_HEALTH: i32 = 20;
/// B30: the Hard AI team's pool.
pub const DUOS_POOL_HEALTH_HARD: i32 = 30;
/// B19, B20: the mana a pass costs its passer.
pub const DUOS_PASS_COST: i32 = 1;
/// B30: what a pass costs the Medium and Hard AI teams: nothing.
pub const DUOS_FREE_PASS_COST: i32 = 0;
/// B19: the passes a seat may make in one round.
pub const DUOS_PASS_CAP: i32 = 10;
/// The rules a human team plays by, and the Easy AI team (B30).
pub const DUOS_HUMAN_TEAM: DuosTeamRules = DuosTeamRules {
    own_health: DUOS_OWN_HEALTH,
    pool_health: DUOS_POOL_HEALTH,
    pass_cost: DUOS_PASS_COST,
    pass_cap: DUOS_PASS_CAP,
};

/// The AI team's rules by difficulty (B30): Easy = DUOS_HUMAN_TEAM; Medium = pass_cost 0;
/// Hard = pass_cost 0 and pool_health DUOS_POOL_HEALTH_HARD. Each AI seat keeps §9.9's
/// `AI_DIFFICULTY` handicap besides.
pub fn duos_ai_team(difficulty: Difficulty) -> DuosTeamRules {
    match difficulty {
        Difficulty::Easy => DUOS_HUMAN_TEAM,
        Difficulty::Medium => DuosTeamRules {
            pass_cost: DUOS_FREE_PASS_COST,
            ..DUOS_HUMAN_TEAM
        },
        Difficulty::Hard => DuosTeamRules {
            pass_cost: DUOS_FREE_PASS_COST,
            pool_health: DUOS_POOL_HEALTH_HARD,
            ..DUOS_HUMAN_TEAM
        },
    }
}

/// The most passes `duos::reduce`'s settle makes after one action: each pass opens a round or ends a
/// seat's turn, and a round takes each duel at least one player-turn on, so the duels' own turn cap
/// (R2) ends the game well inside it. A backstop, as `end_due_turns`' guard is.
pub const DUOS_SETTLE_CAP: usize = 4 * TURN_CAP_PLAYER_TURNS as usize;

// ---------------------------------------------------------------------------------------------
// Patch numbers and card statistics by window (#1360, R1365000, R1367000; SPEC §9.11, R654)
// ---------------------------------------------------------------------------------------------

/// R1365000, raised from 10 by R2455000: the card families that must play differently for a change to
/// be a patch; fewer is a micro. A patch restarts every card's statistics (R1367000), so it waits for
/// a change broad enough to move them. `cargo jackioh classify` reads it, and so does a card's stats
/// window for a shipped version that recorded no tier.
pub const PATCH_MIN_CARDS: usize = 50;

/// SPEC §11 R654, amended by R1367000: the live games (ranked and unranked, tutorial excluded) a
/// card's stats window must log before its figures are live-only. Fewer, and the card shows its
/// previous window; the previous window short too, and AI development games pad the current one.
pub const PUBLIC_STATS_MIN_LIVE_GAMES: i64 = 1000;

// ---------------------------------------------------------------------------------------------
// Whole-game figures (#1994, R1994000; SPEC §9.11)
// ---------------------------------------------------------------------------------------------

/// R1994000: the normal quantile of a two-sided 95% interval, which the first seat's Wilson
/// interval (`wire::stats::wilson_interval`) and the arena's tallies take.
pub const STATS_INTERVAL_Z: f64 = 1.96;

/// R1994000: a share of the game figures is rounded to this many parts of one (three decimals).
pub const STATS_SHARE_PRECISION: f64 = 1000.0;

// ---------------------------------------------------------------------------------------------
// The weapon kit (#1841, R1841000–R1841003; SPEC §4.4, §6.2, §6.3)
// ---------------------------------------------------------------------------------------------

/// R1841000: how many of its user's own turns an ability that "has an extra turn cooldown" sits out
/// after a use (`subsystems::activate`).
pub const ACTIVATE_COOLDOWN_TURNS: i32 = 1;

/// R1841001: "deals half damage to heroes": a hit on a hero is divided by this, rounded down, before
/// Armor (§4.4 step 2).
pub const HALF_DAMAGE_TO_HEROES_DIVISOR: i32 = 2;

/// R1841003: "Float reduces Tribute requirement by 1": what each of its owner's Floats takes off the
/// card's Tribute X, whatever the Float's size (`float::float_lowers_tribute`).
pub const FLOAT_TRIBUTE_REDUCTION: i32 = 1;

// ---------------------------------------------------------------------------------------------
// Stinkyscape's combat and replacement verbs (#1849, R1849000–R1849009; SPEC §4.2, §4.4, §6)
// ---------------------------------------------------------------------------------------------

/// R1849005 (Stinkyscape #157 The Gauntlet): "the middle 3 unit lanes", the 1-based lanes of the
/// `UNIT_ZONES` whose Units the Gauntlet traps, on both sides.
pub const GAUNTLET_LANES: [i32; 3] = [2, 3, 4];

/// R1849009: what a chance in percent is out of ("30% chance to Reveal" is 30 of this).
pub const PERCENT: i32 = 100;

// ---------------------------------------------------------------------------------------------
// The Starship (Stinkyscape, issue #1852; SPEC §6.1, §6.2, §6.3; R1852000–R1852007)
// ---------------------------------------------------------------------------------------------

/// R1852000: the card the engine summons for a player whose Starship Piece dies while they have no
/// Dormant Starship — Stinkyscape's Dormant Starship, a 0/0 Unit that prints Dormant and the hull
/// (`StaticFlags.starshipHull`). Its catalog entry ships with the set's cards (S02); until then no
/// catalog holds it, and a piece's death summons nothing.
pub const DORMANT_STARSHIP_DEF_ID: &str = "stinkyscape-t-dormant-starship";
/// R1852000: the unit zone a Dormant Starship is stacked onto when both its player's rows are full
/// ("stack it on top of your Lane 3 front row, regardless if it's Locked or not").
pub const STARSHIP_FULL_ROW_LANE: i32 = 3;
/// R2249000: the lanes a Dormant Starship is summoned into, the middle first: its owner's backrow zones
/// in this order, standing there (R1081000), then their unit zones in the same order.
pub const STARSHIP_LANE_ORDER: [i32; 5] = [3, 2, 4, 1, 5];
/// R1852003: what a Dormant Starship's Activate costs to launch it ("Activate: pay (3) to awaken").
pub const STARSHIP_LAUNCH_COST: i32 = 3;
/// R1852003: how many times a turn the launch may be activated (once; a launched Starship has no
/// launch left to activate).
pub const STARSHIP_LAUNCH_USES: i32 = 1;
/// R1852004: the names a launched Starship takes, one drawn at random, each with the tag a Starship
/// must hold for the name to be in the draw (`None` suits any ship). The names are names only.
pub const STARSHIP_NAMES: &[(&str, Option<Tag>)] = &[
    ("The Ninth Life", Some(Tag::Felinor)),
    ("Frontier's Promise", Some(Tag::Human)),
    ("The Celestial Archive", Some(Tag::Ky)),
    ("The Lantern Ferry", Some(Tag::Cn)),
    ("The Jlockeed Skylark", Some(Tag::Jlockeed)),
    ("Ember of Departure", None),
    ("The Silent Choir", None),
];

// ---------------------------------------------------------------------------------------------
// Two new endings, voted out and fled (#1843, SPEC §2.5, R1843000–R1843005)
// ---------------------------------------------------------------------------------------------

/// R1843000: the votes of different cards in one turn that remove a player from the game, a loss for
/// them (Stinkyscape #18's three Brains, MELCHIOR, BALTHASAR and CASPER).
pub const VOTE_TALLY_SIZE: i32 = 3;

/// R1843004: the endings that are a win for the winner and a draw, not a loss, for the other player
/// (Stinkyscape #148.1 Edge of the Universe's `fled`). The ladder rates the two sides so, and a game
/// of one with no winner (two fleers at once, or a fleer who has since deleted their account) is a
/// draw for the player left. Every other ending is the same for both sides: the winner's win and the
/// other's loss, or a draw for both. The client reads it (`engineConfig.ts`).
pub const LOSER_DRAWS_REASONS: &[crate::wire::GameOverReason] = &[crate::wire::GameOverReason::Fled];

// ---------------------------------------------------------------------------------------------
// Anomalies and the Scanning Probes (Stinkyscape, issue #1855; SPEC §6.3; R1855003)
// ---------------------------------------------------------------------------------------------

/// R1855003: a scan's chance, out of `ANOMALY_ROLL_SIDES`, to find a Radiant Anomaly before any miss
/// (Stinkyscape #166 and #167, the Scanning Probes: "5% chance to add a Radiant Anomaly Found!").
pub const ANOMALY_RADIANT_CHANCE: i32 = 5;
/// R1855003: a scan's chance, out of `ANOMALY_ROLL_SIDES`, to find a plain Anomaly before any miss
/// ("20% chance to add a Anomaly Found!").
pub const ANOMALY_PLAIN_CHANCE: i32 = 20;
/// R1855003: what each earlier miss of the player's adds to the Radiant chance ("+1%").
pub const ANOMALY_RADIANT_STEP: i32 = 1;
/// R1855003: what each earlier miss of the player's adds to the plain chance ("+4%").
pub const ANOMALY_PLAIN_STEP: i32 = 4;
/// R1855003: the sides of a scan's one roll, the chances' denominator (percent).
pub const ANOMALY_ROLL_SIDES: i32 = 100;

// ---------------------------------------------------------------------------------------------
// Stinkyscape M11 (issue #1845, R1845000–R1845002): hidden Nerfs and trade offers
// ---------------------------------------------------------------------------------------------

/// R1845002: the offer comes due at the start of the opponent's next turn, counted in their own turns
/// (R1181000), so its badge counts down on the offerer's side while it waits.
pub const TRADE_OFFER_TURNS: i32 = 1;

// ---------------------------------------------------------------------------------------------
// Emotes (SPEC §9.5, §10.10; R643, R1341): the dealt hand and the rate limit, which the server and
// the web enforce alike (`wire::emotes`, `apps/web/src/wire/emotes.ts`)
// ---------------------------------------------------------------------------------------------

/// How many emotes a seat is dealt each game (R1341): the menu shows these and nothing else.
pub const EMOTE_HAND_SIZE: usize = 8;
/// How many of a hand are voice lines (R1341); the other `EMOTE_HAND_SIZE - EMOTE_HAND_VOICE` are
/// emoji. A fixed split, so every hand speaks in its portrait's voice and the menu keeps one shape.
pub const EMOTE_HAND_VOICE: usize = 3;
/// R643: the pause between two emotes from one player.
pub const EMOTE_COOLDOWN_MS: i64 = 1500;
/// R643: the rolling window the cap is counted over.
pub const EMOTE_WINDOW_MS: i64 = 20_000;
/// R643: at most this many emotes in a window.
pub const EMOTE_WINDOW_MAX: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ai_teams_rules_follow_their_tier() {
        assert_eq!(duos_ai_team(Difficulty::Easy), DUOS_HUMAN_TEAM);
        assert_eq!(duos_ai_team(Difficulty::Medium).pass_cost, DUOS_FREE_PASS_COST);
        assert_eq!(duos_ai_team(Difficulty::Medium).pool_health, DUOS_POOL_HEALTH);
        assert_eq!(duos_ai_team(Difficulty::Hard).pool_health, DUOS_POOL_HEALTH_HARD);
        assert_eq!(duos_ai_team(Difficulty::Hard).pass_cap, DUOS_PASS_CAP);
    }

    #[test]
    fn fib_clamps_and_fatigue_is_n() {
        assert_eq!(fib(-3), 0);
        assert_eq!(fib(6), 8);
        assert_eq!(fib(40), 89);
        assert_eq!(FATIGUE_DAMAGE(4), 4);
        assert_eq!(AI_DIFFICULTY[Difficulty::Easy], HUMAN_HANDICAP);
    }
}
