//! The state model of SPEC §10.1. Everything here is JSON: no functions, no class instances, no
//! closures, so a state survives a JSON round trip and a replay is exact (§9.3).
//!
//! Port of `packages/engine/src/state.ts` (part 1's type freeze, SURFACE §6.4), every type and
//! function. Also here, because a state field needs them and the state must compile on its own:
//! `CostRule` (TS `costRules.ts`, inside `PlayerModifier`), `EventStay` (TS `stays.ts`, carried by
//! `EffectContext` and paused work) and `EngineError` (SURFACE §6.2). Their TS modules' ports use
//! these and do not redefine them.
//!
//! Field order follows TS's type declarations; JSON key order never matters to the engine, whose
//! hash sorts keys (SURFACE §5.2). Presence matters: every `x?:` is an `Option` that serialises only
//! when set, and is `None` exactly where TS deletes the key (SURFACE §4.4.7).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{
    BACKROW_ZONES, DECK_SIZE, HERO_HEALTH, HUMAN_HANDICAP, Handicap, LIBRARY_CAP, SETUP_TURN, UNIT_ZONES,
};
use crate::rng::Rng;
use crate::wire::{
    AttackHealth, CardDef, CardDefs, CardType, Counters, Enchantment, GameEvent, Keyword, PLAYER_IDS,
    PerPlayer, PerPlayerOpt, PlayerId, PromptKind, RevealAt, Row, RowFlags, Selection, Tag, Tuning, Zone,
    ZoneRef, string_union,
};

pub use crate::wire::{GameResult, Phase, Position, Winner};

/// SURFACE §6.2: the one error type. A TS function that returned `{ error }` or a refusal string
/// returns `Result<T, EngineError>` with the TS message text verbatim (SURFACE §4.4.9).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EngineError {
    pub message: String,
}

impl EngineError {
    pub fn new(message: impl Into<String>) -> EngineError {
        EngineError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for EngineError {}

impl From<String> for EngineError {
    fn from(message: String) -> EngineError {
        EngineError { message }
    }
}

impl From<&str> for EngineError {
    fn from(message: &str) -> EngineError {
        EngineError {
            message: message.to_string(),
        }
    }
}

/// §4.1: `attacked` and `switched` spend the turn's exertion. R636: `attacks` counts the attacks
/// declared this turn once there is a second one (absent, it is 1 when `attacked` and 0 otherwise)
/// against `combat.attacksPerTurn`, two with Windfury.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Exertion {
    pub attacked: bool,
    pub switched: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attacks: Option<i32>,
}

/// R311: what a card's owner was shown of it as it went into their library (`CardInstance.knownAs`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct KnownAs {
    pub def_id: String,
    pub radiant: bool,
}

/// B3.3, R385, R638: a card's Brittle count and the turn it started (`CardInstance.brittle`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BrittleCounter {
    pub count: i32,
    pub since: i32,
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub printed: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardInstance {
    pub id: String,
    pub def_id: String,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub radiant: bool,
    pub zone: Zone,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub position: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub summoned_turn: Option<i32>,
    pub damage: i32,
    pub buffs: AttackHealth,
    pub granted_keywords: Vec<Keyword>,
    pub vanilla: bool,
    pub cost_mod: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub embiggened: Option<bool>,
    pub counters: Counters,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub memory: IndexMap<String, Value>,
    /// §4.1: `attacked` and `switched` spend the turn's exertion. R636: `attacks` counts the attacks
    /// declared this turn once there is a second one (absent, it is 1 when `attacked` and 0 otherwise)
    /// against `combat.attacksPerTurn`, two with Windfury.
    pub exertion: Exertion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stats_override: Option<AttackHealth>,
    /// §7: the Bread Token's radiant face prints "Armor X", where X is the same unspent-mana X its
    /// stats use — so it cannot be a printed number any more than its X/X can. Set beside
    /// `statsOverride` by whoever summons it, and substituted into the printed `Armor` keyword by
    /// `faceOf`. Inert until the token is radiant, because only the radiant face prints Armor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub armor_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub return_to_hand_at_end_of_turn: Option<bool>,
    /// Indestructible would-destroy: no Taunt for this turn (R46).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub taunt_suppressed_turn: Option<i32>,
    /// A backrow card whose identity is public, e.g. a Field Trap that has fired (R33).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub face_up: Option<bool>,
    /// R638: a backrow Trap or Field Trap both players may read while it stays armed — revealed, not
    /// face-up, so it still fires. Cleared by R78's reset with the card leaving the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revealed: Option<bool>,
    /// Instance id of the source whose damage instance was lethal — the hit that took this unit from
    /// above 0 health to 0 or less, or a Poisonous hit — for "destroys a unit" (R42, R89). Unset while
    /// no hit has killed it (`damage.creditKiller`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_damaged_by: Option<String>,
    /// Divine Shield has absorbed a hit and is gone until granted again (§6.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub divine_shield_spent: Option<bool>,
    /// Destroyed by an effect; the next state check collects it (§4.5, §6.3 Destroy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub marked_destroyed: Option<bool>,
    /// MD-D31, R1124: damaged by a card that exiles on damage; the next state check exiles it ahead
    /// of deaths (§4.4 step 7, §4.5 step 1). No Death, no Reborn, no `destroyed`. Only ever
    /// `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub marked_exiled: Option<bool>,
    /// Came back through Reborn, so it no longer has it (§4.5 step 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reborn_spent: Option<bool>,
    /// R311: what this card's owner was shown of it as it went into their library — its definition and
    /// its face — written by `own_library::show_to_owner` where a card goes in openly and read by
    /// `view_for` for the owner's library list (R310) and by nothing else. Absent on a library card its
    /// owner was never shown (R312). A change made to the card inside the library, where nobody sees
    /// it, leaves this record as it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub known_as: Option<KnownAs>,
    // ---- Patch v0.2.0: what rides a card through every zone (R78's reset leaves these alone) ----
    /// B3.4, R386: what Degrade, Upgrade and KY's Constant have changed on this card (`tuning.rs`). Kept
    /// in every zone and through leaving the field; a copy keeps it, a Transform makes a card without
    /// it, a Fuse sums it (R57, R102).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tuning: Option<Tuning>,
    /// B3.3, R385, R638: the card's Brittle count and the turn it started, which the first tick waits two
    /// player-turns behind (`brittle.rs`). Kept in every zone but ticking on the field only: a card in a
    /// hand or a deck holds its count, and `since` is set again as the card enters the field from one. A
    /// copy never inherits it (R57), and a count that has crumbled its card (0) is spent and goes with
    /// R78's reset (R441). `printed` marks a count its printed Brittle started as the card entered the
    /// field, which a Vanilla switches off while a given one stays (B3.3 rule 5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub brittle: Option<BrittleCounter>,
    /// B5 E39: lasting instructions riding the card through every zone (`enchantments.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub enchantments: Option<Vec<Enchantment>>,
    /// B5 E35: this unit has gone Berserk (Classic+ #19.2 sends Classic+ #19.5 there) — a status an
    /// effect sets (`effects::statuses::go_berserk`), never text, so a Vanilla keeps it; R78's reset
    /// takes it off with the card leaving the field. Its own card makes the forced attacks it owes.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub berserk: Option<bool>,
    /// R429: how many times this card has been played, the play under way included — counted at §10.5
    /// step 4 (casts too, R70; a countered play never reaches it) for a card whose script asks
    /// (`StaticFlags.countsPlays`, #31 KY's Math Equation) and absent on every other card. Kept in every
    /// zone and through leaving the field, like `costMod` (R78's reset leaves it alone); a copy or a
    /// Transform is a new card with a count of its own (R57).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub times_played: Option<i32>,
    /// ME-CN, R1300: the card is shown in Chinese (`effects::translate`). Presentation only: no rule
    /// reads it. Kept in every zone and through R78's and R766's resets; a copy keeps it, a
    /// Transform's new card is without it. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub chinese: Option<bool>,
    /// ME-ALTPLAY, R1040, R1044: the card was played face-down into the backrow as a Trap (a Unit
    /// under Knowledge Breaker's Aura, a Spell under Paranoia's) and has not finished revealing.
    /// Its controller's alone to read while it is face-down (R33, R1046); R78's reset takes it off
    /// with the card leaving the field (R1045). Absent on every other card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set_as: Option<SetAs>,
    /// MD-B15, R923: tags an effect gave the card (`effects::grant_tag`, Meditative #35). Part of
    /// the card for every instance-level tag read (`query::tags_of`); catalog pools never see it.
    /// Kept in every zone and through R78's and R766's resets; an instance copy keeps it, a Fuse
    /// unites it, a Transform drops it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub granted_tags: Option<Vec<Tag>>,
}

/// ME-ALTPLAY, R1040, R1044: what a card played face-down as a Trap carries until it has revealed —
/// when it reveals, the turn it was set in, the Echo a Radiant Paranoia gave it (R1045), and, once
/// a set Spell has turned face-up, that its own text now resolves (`revealing`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SetAs {
    pub reveal: RevealAt,
    pub set_turn: i32,
    /// R1045: the Echo the permission added as the card was set (Radiant Paranoia's +1), once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub echo: Option<i32>,
    /// R1044: a set Spell has fired and its Spell text is resolving now. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub revealing: Option<bool>,
}

/// B5 E12, R452: one cast being driven that makes its caster's choices at random (`random`), narrows
/// its target picks to enemies when one is legal (`targetEnemies`), or both. `casts` counts the casts
/// a random cast's resolution has made in all, itself included, against RANDOM_CAST_CHAIN_CAP.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CastMode {
    pub instance_id: String,
    pub player: PlayerId,
    pub random: bool,
    pub target_enemies: bool,
    pub casts: i32,
}

/// A unit zone holds a Stack pile, top card first (§3.2).
pub type Pile = Vec<CardInstance>;

/// B3.1 rule 6: an animated "Animated on your turn" card's backrow zone, held for its return (SPEC §10.1).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct HomeZone {
    pub instance_id: String,
    pub zone: ZoneRef,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "until", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ModifierExpiry {
    ThisTurn {
        turn: i32,
    },
    /// Lasts through that player's next turn; `fromTurn` is the turn it was created on (R48).
    NextTurnOf {
        player: PlayerId,
        from_turn: i32,
    },
    Used,
    Never,
}

/// One rung of a price (E15). `types` names the card types it reaches, all of them when absent — a
/// card's type is its running face's (B2.7, `faces::card_type_of`), and a text that says "Trap" names
/// "Field Trap" too, which the card's own list spells out. `minCost` makes it a threshold rule, "Cost
/// (N)+ cards", tested against the price the flat rungs left (R363). `amount` is added to the price
/// (below 0 a discount, above 0 a surcharge); `setTo` is "costs (N)", which wins over every add.
/// (TS `costRules.ts`; here because `PlayerModifier` holds one.)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CostRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub types: Option<Vec<CardType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min_cost: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub amount: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set_to: Option<i32>,
}

/// What a player modifier does, discriminated on `kind` (TS: the union half of `PlayerModifier`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ModifierKind {
    /// `minCurrentCost`: R48, R363 — only a card whose cost is then this or more (#77, "Cost (4)+").
    /// `onlyType` is only ever "Spell".
    CostDiscount {
        amount: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional, type = "\"Spell\""))]
        only_type: Option<CardType>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        min_current_cost: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        once_per_turn: Option<bool>,
    },
    EchoNextSpell {
        amount: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        source_id: Option<String>,
    },
    RadiantFirstCheapCard {
        max_cost: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        used_turn: Option<i32>,
    },
    ComboDraw {
        amount: i32,
    },
    QuickstrikerDamage,
    // ---- v0.2.0 modifier kinds, by workstream: play pipeline (E15 costs, E39 stamping, Devil's Pact) ----
    // play pipeline B (E15, E39; R455). A price rule on this player's cards (`cost_rules.rs`): Classic #2's
    // "your next Trap or Field Spell costs (2) less" or "costs (0)" (until used, spent by the play it
    // priced), AI Alignment Tax's "(1) more during their next turn" (R48's `nextTurnOf`).
    CostRule {
        rule: CostRule,
    },
    /// Classic+ #14 Forever&: the next Spell its player plays gains this enchantment (E39), until used.
    EnchantNextSpell {
        enchantment: Enchantment,
    },
    /// Classic #23 Devil's Pact, R449: each card this player plays (a cast included, R70) is replaced at
    /// §10.5 step 3 by a new instance of `defId`, Radiant when `radiant` says so, which resolves as that
    /// play (`play_steps::replace_played_card`).
    ReplacePlays {
        def_id: String,
        radiant: bool,
    },
    // ---- v0.2.0 modifier kinds, by workstream: activate and turn (E28 rest of the game) ----
    /// B5 E10, R456: this player's turn ends once `actionsLeft` more main-phase actions of theirs have
    /// resolved. 0 is "your turn ends" — as soon as what is resolving now has resolved (Classic+ #26's
    /// "End your turn", the AI card Rate Limit) — and 1 is "you may take one more action, then your turn
    /// ends" (Classic+ #26 Radiant). `byInstanceId` is the card whose effect it is. Expiry `thisTurn`:
    /// ending the turn any other way ends it too (`reduce.rs` counts the actions and ends the turn).
    TurnEnds {
        actions_left: i32,
        by_instance_id: Option<String>,
    },
    /// B5 E28, R458: "For the rest of the game: at the start of your turn, …" (Classic+ #52). `resume`
    /// re-enters the card's step at each start of this player's turn, in R62's delayed-effect stage
    /// among the delayed effects in creation order (`seq`); `ranTurn` is the turn it last ran, so a
    /// prompt that pauses the stage never runs it twice. `label` is its badge (R169), the card's own
    /// words. Expiry `never`; several stack, each its own modifier.
    StartOfTurnEffect {
        seq: u32,
        resume: Resume,
        label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        ran_turn: Option<i32>,
    },
    // ---- v0.2.0 modifier kinds, by workstream: damage and combat (E8 heal into damage) ----
    /// B5 E8: a heal of X on one of this player's enemies — the enemy hero or an enemy unit — deals X
    /// Pierce damage to it instead, from the converting card (Classic+ #22 Blood Moon's base face, "for
    /// the rest of this turn"). `converterId` rather than `sourceId`: the converter is a Trap already in
    /// its graveyard, and `endOrphanedModifiers` ends a `sourceId` modifier whose card left the field.
    HealToDamage {
        converter_id: String,
    },
    /// R757: #98's Armor Up — Armor on this player's hero, added to `damage::hero_armor_of`'s total from
    /// the moment it is gained until the cleanup that ends the opponent's next turn takes it off (expiry
    /// `nextTurnOf` the opponent), so it is gone when this player's next turn begins.
    HeroArmor {
        amount: i32,
    },
    /// R848 (Meditative #20): a chosen alternative win condition, held for the rest of the game
    /// (expiry `never`, R458). Several stack, each its own modifier, and any one met wins.
    AltWin {
        condition: AltWinCondition,
        threshold: i32,
    },
}

/// `{ id; expiry } & (kind union)`: one player-level modifier (§10.1 `PlayerState.mods`). The kind's
/// fields are flattened beside `id` and `expiry`, as TS writes them.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PlayerModifier {
    pub id: String,
    pub expiry: ModifierExpiry,
    #[serde(flatten)]
    pub kind: ModifierKind,
}

/// `DelayedEffect.at`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DelayedAt {
    /// Only `Phase::Start` or `Phase::End` (TS `"start" | "end"`).
    #[cfg_attr(feature = "ts", ts(type = "\"start\" | \"end\""))]
    pub phase: Phase,
    pub player: PlayerId,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DelayedEffect {
    pub id: String,
    /// Whose script scheduled it, for R68's creation order.
    pub seq: u32,
    pub owner: PlayerId,
    pub at: DelayedAt,
    /// B5 E27, R458: the first turn number whose boundary may run it — "at the end of your *next* turn"
    /// (Classic #37 Radiant) is made with the current turn plus one, so the end of the turn it was made
    /// on passes it by. Absent: the next such boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub not_before: Option<i32>,
    /// A serializable continuation: script id, hook name, captured data (§10.6).
    pub resume: Resume,
    /// R174: the instance this effect is aimed at, when it is aimed at one on the field (#50 Kpop
    /// Fanatic's chosen permanent). The entry is dropped the moment that card leaves the field
    /// (`zones::move_to_zone`), so a card that comes back — bounced and replayed, or a Reborn body — is a
    /// new arrival the effect never chose, and R76's "fizzles if the target has left the field" holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub watch: Option<String>,
    /// R1140 (ME-HANDMARK, Meditative #76 Do or Die): the hand cards this effect is aimed at, in the
    /// order it picked them. Each is dropped the moment its stay in that hand ends
    /// (`zones::forget_hand_watch`), and the entry with it once none is left; as it runs, its step
    /// reads the ones still watched (`effects::delay::HAND_WATCH_KEY`). Absent on every other entry,
    /// so a game that never watches a hand hashes as it did before this field existed (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand_watch: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Resume {
    pub def_id: String,
    pub hook: String,
    /// A named step, so a continuation reads as the script wrote it (§10.6).
    pub step: String,
    pub radiant: bool,
    /// The instance the script belongs to, when it still exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub instance_id: Option<String>,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub data: IndexMap<String, Value>,
}

/// One paused step of an engine sequence (§9.3: "mid-action choices are state, not callbacks").
/// A work item never holds effects — those are closures — it names the continuation to re-enter,
/// so a state with paused work survives JSON and replays exactly (`work.rs`).
///
/// §10.3: an emitted event waiting for the trigger loop to dispatch it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DispatchItem {
    pub id: String,
    pub seq: u32,
    pub event: GameEvent,
}

/// R30: a spell's pending Echo repeats. A repeat waits here while a prompt from the first
/// resolution is still open, so the sequence survives the pause (§10.6, `work.rs`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct EchoItem {
    pub id: String,
    pub seq: u32,
    pub instance_id: String,
    pub controller: PlayerId,
    /// Repeats still owed to this instance.
    pub remaining: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct WorkItem {
    pub id: String,
    /// Creation order, so the queue is deterministic (R68).
    pub seq: u32,
    pub owner: PlayerId,
    pub resume: Resume,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PromptOption {
    pub key: String,
    pub label: String,
    pub selection: Selection,
    /// B5 E18: what this option counts against the prompt's `budget`: a `pick` option's R65 cost, a
    /// `market` lot's price, a barter's yuan written negative (R1000, R1001).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost: Option<i32>,
    /// A face the option shows: the card it offers is Radiant, or a definition is offered Radiant.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub radiant: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PendingChoice {
    pub id: String,
    pub player_id: PlayerId,
    pub kind: PromptKind,
    pub prompt: String,
    pub options: Vec<PromptOption>,
    pub min: i32,
    pub max: i32,
    /// B5 E18: a `pick` prompt's budget — the most its picked options' `cost`s may add up to (Classic
    /// #44's "a total cost of (5) or less"), or a `market` prompt's yuan left (R1000). Absent on every
    /// other prompt, so a state without one hashes as it did before the field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub budget: Option<i32>,
    pub resume: Resume,
}

/// §2.1 step 3, R265: one seat's mulligan while the mulligans are open. `prompt` is the seat's own
/// prompt, whose options are its opening hand; `keep` is its sealed answer — the ids it keeps — and
/// null until it answers. Nothing reads an answer before both seats have given one (R266).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct MulliganSeat {
    pub prompt: PendingChoice,
    pub keep: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct QueuedTrigger {
    pub id: String,
    pub seq: u32,
    pub instance_id: String,
    pub hook: String,
    pub resume: Resume,
}

/// §4.2 step 4 and R44: the attack whose trap window is open — the moment between a declaration,
/// which has already spent the attacker's exertion, and the damage of step 5. `combat.rs` opens
/// it, a trap that fires inside it closes it with `effects::combat::cancel_attack` (§6.3 "Cancel an
/// attack"), and step 5 reads it back to find out whether there is still a combat to resolve.
///
/// Ids and flags only, like the rest of §10.1: no instances and no closures, so the field survives
/// a clone of the state. That is not decoration here — My Pawn's window hands the rest of the turn to
/// the AI policy, which drives `reduce`, which clones, so by the time step 5 runs every
/// `CardInstance` the declaration was built from is a different object and only the ids still name
/// the same cards.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DeclaredAttack {
    /// This declaration, told apart from one opened inside its own window (R44's AI turn takes
    /// actions of its own). Deterministic, from `nextSeq`, so a replay mints the same ids.
    pub id: String,
    /// The attacker's instance id.
    pub attacker_id: String,
    /// §4.2 step 2's two possibilities: an enemy unit's instance id, or `hero-<player>`.
    pub target_id: String,
    /// Set by `cancel_attack` inside the window, so step 5 resolves no combat (§6.3, R44).
    pub cancelled: bool,
    /// R220: the player who declared it, whose unit the attacker must still be at step 5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub by: Option<PlayerId>,
    /// R220, R174: the field's departures when it was declared (`stays::exit_mark`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub exits_from: Option<u32>,
    /// ME-ATTACKSUMMON (R1202): bounce the substitute after its combat when set — Windfast's base
    /// face bounces the summoned Unit if it is still on the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bounce_after: Option<bool>,
    /// MD-D19, R1122: a trap in the window re-aimed this attack at an ally of its attacker, which
    /// §4.3 then resolves as a combat between allies. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub redirected: Option<bool>,
    /// MD-D20, R1123: once that combat's state check has run, this player gets a fresh copy of each
    /// Unit it destroyed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub copies_for: Option<PlayerId>,
}

/// MD-D28, R1125: the verdict on one opponent's play — whether it was their best-scored playable
/// card, judged on the pre-play state from their own view. Never in a view (§10.8).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PlayJudgement {
    pub instance_id: String,
    pub optimal: bool,
    pub turn: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TurnLog {
    pub played_ids: Vec<String>,
    pub cards_played: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unspent_at_end: Option<i32>,
    /// The cost each play this turn actually paid (R56), in play order beside `playedIds`, a cast's 0
    /// included (R70). #64 Gifted Program's "the first card costing 1 or less you play each turn" is
    /// the player's count, not the card's (R213). Optional so a log written without it reads as no
    /// plays; `startTurn` rebuilds the log, which clears it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub costs_paid: Option<Vec<i32>>,
    /// B5 E4: this turn's plays by the type each was played as (B2.7), casts included (R70), countered
    /// plays never — Classic+ #37 Wardrum counts Spells, Field Spells and Traps. `startTurn` rebuilds
    /// the log for both players, which clears it as it clears the rest of "this turn".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub played_by_type: Option<IndexMap<CardType, i32>>,
    /// R1223: how much mana this player borrowed this turn, one debt however many spends made it;
    /// above 0 is "used the credit line" (R1225). `startTurn` rebuilds the log, which clears it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mana_borrowed: Option<i32>,
    /// R1223: how many instalments this turn's debt is split into. Cleared with the log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub borrowed_parts: Option<i32>,
    /// R1224: the instalment actually taken off this turn's refresh, after any forgiveness — the
    /// locked crystals the view shows. Set at the refresh, which follows the log reset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mana_locked: Option<i32>,
}

/// `PlayerState.hero`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct HeroState {
    pub health: i32,
    pub armor: i32,
}

/// `PlayerState.mana`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ManaState {
    pub current: i32,
    pub max: i32,
    pub next_turn_mod: i32,
    pub perm_mod: i32,
}

/// `PlayerState.drawOffer`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DrawOfferState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub offered_turn: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub blocked_until: Option<i32>,
}

/// `PlayerState.draws`: B5 E3, E4, R457.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DrawCount {
    pub turn: i32,
    pub count: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub hero: HeroState,
    pub mana: ManaState,
    pub hand: Vec<CardInstance>,
    pub library: Vec<CardInstance>,
    pub graveyard: Vec<CardInstance>,
    pub exile: Vec<CardInstance>,
    /// Cards mid-resolution: a Spell sits here between its play and its graveyard (§10.5).
    pub resolving: Vec<CardInstance>,
    pub units: Vec<Option<Pile>>,
    pub backrow: Vec<Option<CardInstance>>,
    pub locks: RowFlags,
    pub mods: Vec<PlayerModifier>,
    pub turn_log: TurnLog,
    pub draw_offer: DrawOfferState,
    pub fatigue_count: i32,
    /// Turns this player has started, for the mana refresh (§2.3).
    pub turns_started: i32,
    /// R1223: the instalment due at each of this player's coming refreshes, the next one first,
    /// with trailing zeros trimmed; `None` when empty, so a game without credit hashes as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub owed_instalments: Option<Vec<i32>>,
    /// My Pawn: the AI policy plays out the rest of this turn (R44).
    pub ai_turn: bool,
    /// R180: this seat's handicap. Absent means HUMAN_HANDICAP, and createGame never stores one equal
    /// to it, so a game without handicaps hashes exactly as it did before this field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub handicap: Option<Handicap>,
    /// R345: `false` once this player has turned R82's automatic turn end off (`setAutoEndTurn`).
    /// Absent means on, and turning it back on deletes the field, so a game in which nobody touched
    /// the setting hashes exactly as it did before this field existed. Only ever `Some(false)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "false"))]
    pub auto_end_turn: Option<bool>,
    // ---- v0.2.0 player fields, by workstream: field (B3.1, E20, E21, E22) ----
    /// B5 E21: the dormant cards beneath each backrow zone's top card, top first, by lane (index lane −
    /// 1). The top stays in `backrow` and is the one card that acts there (§3.2, R13); these are face-down
    /// and not on the field for effects (`zones::is_buried`). Absent while no backrow zone holds a pile, so a
    /// game that never builds one hashes as it did before the field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub backrow_piles: Option<Vec<Vec<CardInstance>>>,
    /// B5 E21, R446: the Unit a carrier in each backrow zone holds, by lane — a Unit played on top of a
    /// backrow card whose static flag lets one (Classic+ #33 Ivory Tower). It stands in that backrow
    /// zone (its `zone.row` is "backrow"), is a Unit for every rule (`zones::active_units_of`), and can
    /// neither attack nor be attacked (`zones::is_carried`). Absent while nothing is carried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub carried: Option<Vec<Option<CardInstance>>>,
    // ---- v0.2.0 player fields, by workstream: play pipeline (E4 play counters, E11) ----
    /// B5 E4, R451: what this player's plays leave for the rest of the game, never reset
    /// (`play_counts.rs`): `playedByTag`, their plays by tag (Classic+ #64's Fruit, AI Scaling Law's AI),
    /// casts included (R70) and countered plays never; `lastFaceUpPlay`, the last face-up card they
    /// played (AI Autocomplete). Absent until their first play, so a game without one hashes as it did
    /// before this field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub game_log: Option<GameLog>,
    // ---- v0.2.0 player fields, by workstream: activate and turn (E3, E4 draw counts, E10) ----
    /// B5 E3, E4, R457: how many draws this player has made on turn `turn`, whoever's turn it is — a
    /// fatigue draw included, a draw a limit stopped not. A count kept for an earlier turn reads as 0,
    /// so it resets where the turn log does without anything clearing it (`draw::draws_this_turn`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub draws: Option<DrawCount>,
    // ---- Meditative player fields (docs/meditative-set.md M5, ME-HANDCAP) ----
    /// R1143: this player's hand size for the rest of the game, once an effect has set one (Meditative
    /// #79 Touched by KY), the latest setting winning; every rule that reads the hand cap reads it
    /// (`query::hand_cap_of`). Absent means `HAND_CAP`, so a game that never sets one hashes as it did
    /// before this field existed (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand_cap: Option<i32>,
    /// ME-JADE, R961: this player's Jade Counter, public, which only rises (`effects::jade`). Absent until
    /// it first rises, so a game without a Jade hashes as it did before this field existed (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub jade: Option<i32>,
    /// R844 (Meditative #18, #19): the `turns_started` index through which this player's refresh
    /// gives 0 mana. Absent while no loss covers a future refresh, so a game that never loses one
    /// hashes as it did before this field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lost_refresh_through: Option<i32>,
    /// R846 (Meditative #19.1): extra turns owed to this player, taken when their turn ends by
    /// starting their turn again. Absent (never 0 stored) while none is owed, so a game without one
    /// hashes as it did before this field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub extra_turns: Option<i32>,
    /// R847 (Meditative #19.1): once-a-game Temporal Rift flag — set when a Rift grants this player
    /// an extra turn, so a later Rift grants none. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub rift_extra_turn: Option<bool>,
    /// R850 (Meditative #8, #20): set when an effect wins the game for this player outright
    /// (`win_game`). Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub won_by_effect: Option<bool>,
}

string_union! {
    /// R848 (Meditative #20): which measure a chosen alternative win condition reads.
    pub enum AltWinCondition {
        Health = "health",
        Graveyard = "graveyard",
        Board = "board",
    }
}

/// Ceaseless Void's four game counters (R55): `GameState.counters`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GameCounters {
    pub drawn: i32,
    pub played: i32,
    pub destroyed: i32,
    pub exiled: i32,
}

/// Nonce dedupe: one already-applied action and the events it produced (§9.3, `GameState.applied`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AppliedAction {
    pub nonce: String,
    pub events: Vec<GameEvent>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub seed: String,
    pub rng_cursor: u32,
    /// Player-turn counter, 1-based, capped by TURN_CAP_PLAYER_TURNS (§2.5, R2).
    pub turn: i32,
    pub active: PlayerId,
    pub phase: Phase,
    pub players: PerPlayer<PlayerState>,
    pub pending: Option<PendingChoice>,
    pub trigger_queue: Vec<QueuedTrigger>,
    /// The attack whose trap window is open, between declaration and damage (§4.2 step 4, R44).
    pub declared_attack: Option<DeclaredAttack>,
    /// MD-D28, R1125: the verdict on the last judged play, stored as the play began. Never in a
    /// view (§10.8). Absent while no card judges plays, so a game without one hashes as it did
    /// before this field existed (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub play_judgement: Option<PlayJudgement>,
    /// Paused sequences waiting to continue, in order (§9.3, §10.6).
    pub work: Vec<WorkItem>,
    /// R113: how many items the *current* pause cascade has parked. A scope parks its remainder at
    /// this index and advances it, so one cascade lands innermost-first, and taking an item resets it
    /// to 0 so the next cascade is inserted ahead of everything still owed. Neither a plain queue nor
    /// a plain stack is correct: a Cry's parked tail must run before the play steps that follow it,
    /// while a prompt opened *inside* that tail must run before both.
    pub work_cursor: usize,
    /// Echo repeats owed but not yet resolved (R30).
    pub echo_queue: Vec<EchoItem>,
    /// Events emitted but not yet dispatched to triggers (§10.3).
    pub dispatch: Vec<DispatchItem>,
    pub delayed: Vec<DelayedEffect>,
    /// Ceaseless Void's four game counters (R55).
    pub counters: GameCounters,
    pub transient_defs: IndexMap<String, CardDef>,
    /// Zones a dying Reborn unit holds until it returns (R64).
    pub reserved: Vec<ZoneRef>,
    /// Players whose mulligan has resolved, in seat order (§2.1, R265).
    pub mulliganed: Vec<PlayerId>,
    /// §2.1 step 3, R265: both seats' mulligans, open at once. Present only while they are open — the
    /// opening deal done, `pending` null, phase `mulligan` — and gone the moment the second answer
    /// resolves them, so a state past its mulligan hashes as it did before this field existed.
    /// `pending` stays the one prompt §10.1 allows; the two mulligans are the one sealed-bid step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mulligan: Option<PerPlayer<MulliganSeat>>,
    pub result: Option<GameResult>,
    /// Next instance/choice/trigger id, so ids are deterministic under replay.
    pub next_id: u32,
    /// Monotonic sequence for R68's creation order.
    pub next_seq: u32,
    /// Nonce dedupe: the events each already-applied action produced (§9.3).
    pub applied: Vec<AppliedAction>,
    /// R217: how many cards the cast-on-draw chain that is running has cast, draws made by its casts
    /// included, so R58's cap bounds the whole chain. Present only while a chain runs (a pause inside
    /// one keeps it here for the answer), and gone once the draw that began it has finished.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cast_chain: Option<i32>,
    /// R174: the field's departures, counted, and each card's latest (`stays.rs`). An effect aimed at a
    /// card on the field is aimed at that stay, and a sequence a prompt splits resumes in a later
    /// action whose event list does not hold what happened before the pause, so "has this card left
    /// the field since" is read off this record, which survives the pause. Absent until a card first
    /// leaves the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field_exits: Option<FieldExits>,
    // ---- v0.2.0 game fields, by workstream: field (B3.1 home zones, E21) ----
    /// B3.1 rule 6, R383: the backrow zone each animated "Animated on your turn" card goes back to at
    /// its controller's cleanup, held for it meanwhile as R64 holds a dying Reborn unit's zone
    /// (`zones::is_reserved` reads both). Absent while no such card is animated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub homes: Option<Vec<HomeZone>>,
    // ---- v0.2.0 game fields, by workstream: play pipeline (E1 announce, E4 last plays, E12) ----
    /// play pipeline B (E12, R452): the casts being driven right now that change how choices are made —
    /// a random cast (every choice its caster makes answered from the rng) or a cast that targets
    /// enemies when it can — innermost last. Present only while such a cast's steps run
    /// (`random_cast::with_cast_mode`), so a state at rest, a paused one included, never carries it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub casts_resolving: Option<Vec<CastMode>>,
    /// B5 E1, R448: the plays whose announce window is open, innermost last (a cast a responder makes
    /// announces inside the window it answers). Present only while one is open (`announce.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub announcing: Option<Vec<AnnounceRecord>>,
    /// B5 E4: the last Spell anyone played (Classic #57 Echo), overwritten by the next, never cleared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_spell: Option<PlayRecord>,
    /// R58: the cards a cast-on-draw draw is casting, by the id each was drawn under, whose `drawn` is
    /// held from every dispatch until that cast has resolved — the draw's "complete" point
    /// (`draw_complete.rs`). Present only while one is held.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub held_draws: Option<Vec<String>>,
    // ---- v0.2.0 game fields, by workstream: Core patches and cosmetics (R433) ----
    /// R437: the marks cards carry for an effect aimed at them that is still to come — #50 K-Pop
    /// Fanatic's pending steal on its target — one entry per mark, tied to the delayed effect that
    /// made it (`marks.rs`). `view_for` puts them on the card in both views; the mark goes when its
    /// effect resolves or fizzles, or is dropped because its card left the field (R174), and a
    /// `marked` event says so each way. Absent when no card is marked, so a game without marks hashes
    /// as it did before the field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub marks: Option<Vec<MarkRecord>>,
    // ---- v0.2.0 game fields, by workstream: cards-plus-c (E30) ----
    /// R417: each seat's last board, a `create_game` input frozen into the match and never written
    /// again (`subsystems::last_boards`). Only a seat with one has a key, and a game with none has no
    /// field, so it hashes as it did before. `view_for` never sends it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_boards: Option<PerPlayerOpt<Vec<LastBoardEntry>>>,
    /// C+ #35 Rollback, R419: the field as each of the last BOARD_HISTORY_DEPTH turns began, oldest first
    /// (`subsystems::board_history`). Never in a view (§10.8). Absent until the first turn starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub board_history: Option<Vec<BoardSnapshot>>,
    // ---- the Glitch Easter egg (issue #170; R673–R679) ----
    /// R673: how many "… in the System" cards either player has played. Absent at 0, so a match without one hashes as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub system_plays: Option<i32>,
    /// R676: the decks and dealt seats `create_game` began with, which a reset deals again. Never in a view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opening: Option<OpeningRecord>,
    /// R676: how many times Glitch has reset the match; keys the reset's id numbering. Absent until the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub resets: Option<i32>,
    /// R676: a reset Glitch owes, done once the action that drew it has settled (`subsystems::glitch`).
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub reset_owed: Option<bool>,
    /// R677: how many times Glitch has swapped the seats; odd means each account now holds the other seat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub seat_swaps: Option<i32>,
    /// R678: the two boards of other players' games a Glitch may put on the field, a `create_game` input
    /// frozen like `lastBoards` (R417, R564). Never in a view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub glitch_boards: Option<PerPlayerOpt<Vec<LastBoardEntry>>>,
    // ---- derived, never sent, stored or hashed ----
    /// R179: the scripts of `transient_defs`' fused definitions, composed once (as a Fuse mints one, and
    /// on entry to `reduce`: `scripts::sync_fused_scripts`) and shared by every state cloned from this
    /// one, so a lookup does not compose them again. Serde skips it and every state compares equal on
    /// it; a state that came through JSON has none, and its lookups compose until its next `reduce`.
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    pub fused_scripts: crate::scripts::FusedScripts,
}

/// R676: what a Glitch reset deals again — `create_game`'s decks and dealt seats.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct OpeningRecord {
    pub decks: (Vec<String>, Vec<String>),
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dealt: Option<Vec<PlayerId>>,
}

/// R417: one card of a last board — the card and its face, never stats, buffs or damage.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LastBoardEntry {
    pub def_id: String,
    pub radiant: bool,
}

/// R417: each seat's last board in seat order, as `create_game` and `replay::fold` take them.
pub type LastBoardInput = (Vec<LastBoardEntry>, Vec<LastBoardEntry>);

/// R437: one mark on one card, while the delayed effect `delayedId` waits (`marks.rs`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct MarkRecord {
    pub instance_id: String,
    pub mark: String,
    pub color: String,
    pub delayed_id: String,
}

/// R419: one side of the field as a turn began — its zones' cards whole, its Locks, the homes held then.
/// (TS `Pick<PlayerState, "units" | "backrow" | "backrowPiles" | "carried" | "locks"> & { homes? }`.)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SideSnapshot {
    pub units: Vec<Option<Pile>>,
    pub backrow: Vec<Option<CardInstance>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub backrow_piles: Option<Vec<Vec<CardInstance>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub carried: Option<Vec<Option<CardInstance>>>,
    pub locks: RowFlags,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub homes: Option<Vec<HomeZone>>,
}

/// R419: the field at the start of player-turn `turn` (`subsystems::board_history`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BoardSnapshot {
    pub turn: i32,
    pub sides: PerPlayer<SideSnapshot>,
}

/// R419: every card one side of a snapshot holds.
pub fn side_snapshot_instances(side: &SideSnapshot) -> Vec<&CardInstance> {
    let mut out: Vec<&CardInstance> = Vec::new();
    out.extend(side.units.iter().flatten().flatten());
    out.extend(side.backrow.iter().flatten());
    out.extend(side.backrow_piles.iter().flatten().flatten());
    out.extend(side.carried.iter().flatten().flatten());
    out
}

/// `side_snapshot_instances`, mutably, in the same order.
pub fn side_snapshot_instances_mut(side: &mut SideSnapshot) -> Vec<&mut CardInstance> {
    let mut out: Vec<&mut CardInstance> = Vec::new();
    out.extend(side.units.iter_mut().flatten().flatten());
    out.extend(side.backrow.iter_mut().flatten());
    out.extend(side.backrow_piles.iter_mut().flatten().flatten());
    out.extend(side.carried.iter_mut().flatten().flatten());
    out
}

/// R227, R419: a card that took a fresh id is still the card the history recorded, so the history follows it.
pub fn rename_in_board_history(state: &mut GameState, from: &str, to: &str) {
    for snapshot in state.board_history.iter_mut().flatten() {
        for player in PLAYER_IDS {
            let side = &mut snapshot.sides[player];
            for card in side_snapshot_instances_mut(side) {
                if card.id == from {
                    card.id = to.to_string();
                }
            }
            for home in side.homes.iter_mut().flatten() {
                if home.instance_id == from {
                    home.instance_id = to.to_string();
                }
            }
        }
    }
}

/// R174, R212, R436: what a queued trigger answering an event carries — the cards the event names,
/// and the field's departures when it happened (`triggers::queue_trigger`, `EffectContext.event_stay`),
/// so a card the event names is aimed at the stay it had then, while every other card the trigger
/// reads off the board as it resolves is judged from when its run began. All JSON.
/// (TS `stays.ts`; here because `EffectContext` and paused work carry one.)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct EventStay {
    pub from: u32,
    pub ids: Vec<String>,
}

/// R174: `count` departures so far; `last` maps a card to the departure that was its latest.
/// `uncovered` (R212, §3.2) maps a card that has left the top of a Stack pile — died, bounced,
/// exiled, stolen, fused away — to the note of that removal (`stays::note_uncovered`), so the events
/// reporting its leaving, and every event before them, are not answered by the card it uncovered.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FieldExits {
    pub count: u32,
    pub last: IndexMap<String, u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uncovered: Option<IndexMap<String, UncoveredNote>>,
}

/// R212, §3.2: `resumed` is the dormant card that became its pile's top when the card left.
/// `reported`: the loop has dispatched a report of that removal (`stays::note_reported`). `movedOn`: the
/// card that left has moved zones again since (`stays::note_moved`). The note goes once both are true.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct UncoveredNote {
    pub resumed: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reported: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub moved_on: Option<bool>,
}

// ---- v0.2.0 play pipeline A (E1 announce, E4 records) ----

/// B5 E1, R448: one play or cast between its announce and §10.5 step 4. `faceDown` is a card that
/// will be set face-down (a Trap or Field Trap): while it waits in the resolving zone only `player`
/// reads it (`view_for`). `countered` is set by the Counter that cancelled it (`effects::move_::counter_play`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AnnounceRecord {
    pub instance_id: String,
    pub player: PlayerId,
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub face_down: Option<bool>,
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub countered: Option<bool>,
}

/// B5 E4: a card as a play record names it — the definition and the face it was played with.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PlayRecord {
    pub def_id: String,
    pub radiant: bool,
}

/// B5 E4: a face-up play's record, with the type it was played as (B2.7). (TS `PlayRecord & { type }`.)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FaceUpRecord {
    pub def_id: String,
    pub radiant: bool,
    #[serde(rename = "type")]
    pub type_: CardType,
    /// R1300: the played card was Chinese (T-AI-5's copy is too). Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub chinese: Option<bool>,
}

/// B5 E4, R451: a player's per-game play record (`PlayerState.gameLog`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GameLog {
    pub played_by_tag: IndexMap<Tag, i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_face_up_play: Option<FaceUpRecord>,
}

fn empty_row<T: Clone>(size: i32) -> Vec<Option<T>> {
    vec![None; size.max(0) as usize]
}

fn empty_locks(size: i32) -> Vec<bool> {
    vec![false; size.max(0) as usize]
}

pub fn create_player_state() -> PlayerState {
    PlayerState {
        hero: HeroState {
            health: HERO_HEALTH,
            armor: 0,
        },
        mana: ManaState {
            current: 0,
            max: 0,
            next_turn_mod: 0,
            perm_mod: 0,
        },
        hand: Vec::new(),
        library: Vec::new(),
        graveyard: Vec::new(),
        exile: Vec::new(),
        resolving: Vec::new(),
        units: empty_row::<Pile>(UNIT_ZONES),
        backrow: empty_row::<CardInstance>(BACKROW_ZONES),
        locks: RowFlags {
            units: empty_locks(UNIT_ZONES),
            backrow: empty_locks(BACKROW_ZONES),
        },
        mods: Vec::new(),
        turn_log: TurnLog {
            played_ids: Vec::new(),
            cards_played: 0,
            ..TurnLog::default()
        },
        draw_offer: DrawOfferState::default(),
        fatigue_count: 0,
        turns_started: 0,
        owed_instalments: None,
        ai_turn: false,
        handicap: None,
        auto_end_turn: None,
        backrow_piles: None,
        carried: None,
        game_log: None,
        draws: None,
        hand_cap: None,
        jade: None,
        lost_refresh_through: None,
        extra_turns: None,
        rift_extra_turn: None,
        won_by_effect: None,
    }
}

/// `createGame`'s options.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameOptions {
    pub seed: String,
    pub decks: (Vec<String>, Vec<String>),
    /// The catalog to deal from. TS registered it for the process; Rust registers nothing here (the
    /// registry is a `OnceLock`, SURFACE §3, and tests use the testkit's override, §8): when given,
    /// `create_game` validates the decks against it instead of the registered catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub catalog: Option<CardDefs>,
    /// R180: per-seat handicaps. An omitted seat, or one equal to HUMAN_HANDICAP, stores nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub handicaps: Option<PerPlayerOpt<Handicap>>,
    /// R433: the seats whose deck their player was dealt rather than built — All Random's (R258),
    /// practice's fresh random deck. Their starting library is written no record of what its owner
    /// was shown (R311), so its cards list as unknown until they leave it (R312). Setup, not an
    /// action: a replay passes the same list (`replay::ReplayInput.dealt`). Omitted, every deck was built.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dealt: Option<Vec<PlayerId>>,
    /// R417: each seat's last board (C+ #29). Setup, not an action: a replay passes the same boards
    /// (`replay::ReplayInput.lastBoards`). Omitted, both are empty (hotseat, practice, a first game).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_boards: Option<LastBoardInput>,
    /// R678: the boards of two other players' games a Glitch may put on the field, one per seat. Setup,
    /// not an action, frozen as `lastBoards` is (R564): a replay passes the same boards
    /// (`replay::ReplayInput.glitchBoards`). Omitted, a Glitch's boards outcome leaves both fields empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub glitch_boards: Option<LastBoardInput>,
}

/// SURFACE §6.1's name for `CreateGameOptions`.
pub type CreateGameArgs = CreateGameOptions;

/// The five fields of a handicap, in §9.9's order, so every reader walks the same list.
const HANDICAP_FIELDS: [&str; 5] = [
    "deckSize",
    "manaBonus",
    "manaCap",
    "extraOpeningCards",
    "extraDrawsPerTurn",
];

/// `handicap[field]` for one of `HANDICAP_FIELDS`.
fn handicap_field(handicap: &Handicap, field: &str) -> i32 {
    match field {
        "deckSize" => handicap.deck_size,
        "manaBonus" => handicap.mana_bonus,
        "manaCap" => handicap.mana_cap,
        "extraOpeningCards" => handicap.extra_opening_cards,
        _ => handicap.extra_draws_per_turn,
    }
}

/// R180: the handicap a seat plays with. A seat that stores none has this spec's resources.
pub fn handicap_of(side: &PlayerState) -> Handicap {
    side.handicap.unwrap_or(HUMAN_HANDICAP)
}

/// R290: the health a seat's hero starts with — its handicap's `heroHealth`, else §2's HERO_HEALTH.
pub fn starting_hero_health(handicap: Option<&Handicap>) -> i32 {
    handicap.and_then(|h| h.hero_health).unwrap_or(HERO_HEALTH)
}

/// R180: whether a handicap is exactly a human's, which is what `create_game` declines to store.
fn is_human_handicap(handicap: &Handicap) -> bool {
    HANDICAP_FIELDS
        .iter()
        .all(|field| handicap_field(handicap, field) == handicap_field(&HUMAN_HANDICAP, field))
        && starting_hero_health(Some(handicap)) == HERO_HEALTH
}

/// R180, R184: a handicap is five non-negative integers, and its deck size is one a library can
/// hold (R80's LIBRARY_CAP). Refuses naming the seat and the field, as `validate_deck` does. (TS
/// throws; `create_game` panics with the message.)
pub fn validate_handicap(handicap: &Handicap, label: &str) -> Result<(), EngineError> {
    for field in HANDICAP_FIELDS {
        let value = handicap_field(handicap, field);
        if value < 0 {
            return Err(EngineError::new(format!(
                "{label}: handicap {field} must be a non-negative integer (R180), got {value}"
            )));
        }
    }
    if handicap.deck_size < 1 || handicap.deck_size > LIBRARY_CAP {
        return Err(EngineError::new(format!(
            "{label}: handicap deckSize must be between 1 and {LIBRARY_CAP} (R184), got {}",
            handicap.deck_size
        )));
    }
    // R290: optional, and when given a hero that starts alive.
    if let Some(hero_health) = handicap.hero_health
        && hero_health < 1
    {
        return Err(EngineError::new(format!(
            "{label}: handicap heroHealth must be a positive integer (R290), got {hero_health}"
        )));
    }
    Ok(())
}

/// §2.6 and §9.4 L2, L3, L6: the rules a deck must satisfy before a game exists. `size` is the
/// seat's handicap deck size (R184), DECK_SIZE when it has none, whose message is §2.6's own; any
/// other size is the handicap's, and the message says so. (TS throws; `create_game` panics with the
/// message.)
pub fn validate_deck(deck: &[String], catalog: &CardDefs, label: &str, size: i32) -> Result<(), EngineError> {
    if deck.len() as i32 != size {
        if size == DECK_SIZE {
            return Err(EngineError::new(format!(
                "{label}: deck must hold exactly {DECK_SIZE} cards (§2.6 L2), got {}",
                deck.len()
            )));
        }
        return Err(EngineError::new(format!(
            "{label}: deck must hold exactly {size} cards (its handicap, R184), got {}",
            deck.len()
        )));
    }
    let mut seen: Vec<&str> = Vec::with_capacity(deck.len());
    for def_id in deck {
        let Some(def) = catalog.get(def_id) else {
            return Err(EngineError::new(format!(
                "{label}: \"{def_id}\" is not in the catalog (§9.4 L6)"
            )));
        };
        if seen.contains(&def_id.as_str()) {
            return Err(EngineError::new(format!(
                "{label}: \"{def_id}\" appears twice; no duplicate card ids (§2.6 L3)"
            )));
        }
        seen.push(def_id);
        if def.token || def.tags.contains(&Tag::Token) {
            return Err(EngineError::new(format!(
                "{label}: \"{def_id}\" is a Token card and cannot be in a deck (§2.6 L3)"
            )));
        }
        // R1420: nor is a card of a set that has not shipped, unless the testkit previews it.
        if !crate::catalog::set_is_open(def.set) {
            return Err(EngineError::new(format!(
                "{label}: \"{def_id}\" is a card of {}, which has not shipped yet (§2.6 L3, R1420)",
                def.set.as_str()
            )));
        }
    }
    Ok(())
}

/// TS's `Pick<GameState, "nextId">`: whatever hands out instance ids. The state does; so does a
/// bare counter (fuse's scratch numbering, `{ nextId: 0 }` in TS).
pub trait NextId {
    fn next_id_mut(&mut self) -> &mut u32;
}

impl NextId for GameState {
    fn next_id_mut(&mut self) -> &mut u32 {
        &mut self.next_id
    }
}

impl NextId for u32 {
    fn next_id_mut(&mut self) -> &mut u32 {
        self
    }
}

pub fn new_instance(state: &mut impl NextId, def_id: &str, owner: PlayerId, zone: Zone) -> CardInstance {
    let next_id = state.next_id_mut();
    let instance = CardInstance {
        id: format!("c{next_id}"),
        def_id: def_id.to_string(),
        owner,
        controller: owner,
        radiant: false,
        zone,
        position: None,
        summoned_turn: None,
        damage: 0,
        buffs: AttackHealth { attack: 0, health: 0 },
        granted_keywords: Vec::new(),
        vanilla: false,
        cost_mod: 0,
        cost_override: None,
        x: None,
        embiggened: None,
        counters: Counters::default(),
        memory: IndexMap::new(),
        exertion: Exertion {
            attacked: false,
            switched: false,
            attacks: None,
        },
        stats_override: None,
        armor_override: None,
        return_to_hand_at_end_of_turn: None,
        taunt_suppressed_turn: None,
        face_up: None,
        revealed: None,
        last_damaged_by: None,
        divine_shield_spent: None,
        marked_destroyed: None,
        marked_exiled: None,
        reborn_spent: None,
        known_as: None,
        tuning: None,
        brittle: None,
        enchantments: None,
        berserk: None,
        times_played: None,
        chinese: None,
        set_as: None,
        granted_tags: None,
    };
    *next_id += 1;
    instance
}

/// A game in phase `setup`: libraries hold the decks in list order, and `setup.rs` (M1-T5)
/// shuffles them with the match rng and deals the opening hands.
///
/// R180: each seat's handicap is validated first, so a bad deck size is named as the handicap's
/// fault; then each deck is checked against its seat's deck size (R184). A handicap is stored on the
/// seat only when it differs from a human's, so a game with none — or with Easy's, which *is* a
/// human's — carries no `handicap` key and hashes and replays exactly as before the field existed.
///
/// Panics with TS's message on a bad handicap or deck, as TS threw (SURFACE §6.1 fixes the return
/// type; a caller that must not panic checks `validate_handicap` and `validate_deck` first).
pub fn create_game(options: &CreateGameOptions) -> GameState {
    build_game(options, 1, "")
}

/// R676: the new game a Glitch reset deals — `create_game`'s, with ids numbered on from `next_id` in
/// an order drawn from a stream of this reset's own (R223), so no id the old game showed names a card
/// of the new one. (TS `createGameForReset(options, { nextId, resets })`.)
pub fn create_game_for_reset(options: &CreateGameOptions, next_id: u32, resets: i32) -> GameState {
    build_game(options, next_id, &format!(":reset-{resets}"))
}

fn deck_of(options: &CreateGameOptions, seat: usize) -> &Vec<String> {
    if seat == 0 {
        &options.decks.0
    } else {
        &options.decks.1
    }
}

fn build_game(options: &CreateGameOptions, first_id: u32, stream: &str) -> GameState {
    let catalog: &CardDefs = match &options.catalog {
        Some(catalog) => catalog,
        None => crate::catalog::registered_catalog(),
    };

    let no_handicaps = PerPlayerOpt::default();
    let handicaps: &PerPlayerOpt<Handicap> = options.handicaps.as_ref().unwrap_or(&no_handicaps);
    for player in PLAYER_IDS {
        if let Some(handicap) = handicaps.get(player)
            && let Err(error) = validate_handicap(handicap, player.as_str())
        {
            panic!("{error}");
        }
    }

    for (seat, player) in PLAYER_IDS.into_iter().enumerate() {
        let size = handicaps.get(player).map_or(DECK_SIZE, |h| h.deck_size);
        if let Err(error) = validate_deck(deck_of(options, seat), catalog, player.as_str(), size) {
            panic!("{error}");
        }
    }

    let mut state = GameState {
        seed: options.seed.clone(),
        rng_cursor: 0,
        turn: SETUP_TURN,
        active: PlayerId::P1,
        phase: Phase::Setup,
        players: PerPlayer {
            p1: create_player_state(),
            p2: create_player_state(),
        },
        pending: None,
        trigger_queue: Vec::new(),
        declared_attack: None,
        play_judgement: None,
        work: Vec::new(),
        work_cursor: 0,
        echo_queue: Vec::new(),
        dispatch: Vec::new(),
        delayed: Vec::new(),
        counters: GameCounters {
            drawn: 0,
            played: 0,
            destroyed: 0,
            exiled: 0,
        },
        transient_defs: IndexMap::new(),
        reserved: Vec::new(),
        mulliganed: Vec::new(),
        mulligan: None,
        result: None,
        next_id: first_id,
        next_seq: 1,
        applied: Vec::new(),
        cast_chain: None,
        field_exits: None,
        homes: None,
        casts_resolving: None,
        announcing: None,
        last_spell: None,
        held_draws: None,
        marks: None,
        last_boards: None,
        board_history: None,
        system_plays: None,
        opening: None,
        resets: None,
        reset_owed: None,
        seat_swaps: None,
        glitch_boards: None,
        fused_scripts: crate::scripts::FusedScripts::default(),
    };

    // R223: the numbers each deck's cards take are drawn in an order of the seed's own, so a card's id
    // says nothing about where it stood in the list the deck was handed over in — which the server's
    // store sorts by card id, so an id numbered in list order told the opponent how many of a deck's
    // cards sort before it, hidden ones included (§9.1, R97). The library itself is still the list in
    // order, for §2.1's shuffle, and the stream is not the match's rng, whose draws are untouched.
    let mut numbering = Rng::new(&format!("{}{}{}", options.seed, INSTANCE_ID_STREAM, stream), 0);
    for (seat, player) in PLAYER_IDS.into_iter().enumerate() {
        let deck = deck_of(options, seat);
        let mut library: Vec<CardInstance> = deck
            .iter()
            .map(|def_id| new_instance(&mut state, def_id, player, Zone::Library { player }))
            .collect();
        let numbers: Vec<String> = library.iter().map(|card| card.id.clone()).collect();
        let ids = numbering.shuffle(&numbers);
        // R433: a dealt deck is not one its player built, so they know none of it yet.
        let built = !options
            .dealt
            .as_ref()
            .is_some_and(|dealt| dealt.contains(&player));
        for (at, card) in library.iter_mut().enumerate() {
            if let Some(id) = ids.get(at) {
                card.id = id.clone();
            }
            // R311: a player's own deck is the first thing they know of their library.
            if built {
                crate::own_library::show_to_owner(card);
            }
        }
        let side = &mut state.players[player];
        side.library = library;

        // R180: a copy of the five fields and nothing else, so no stray key reaches the state or its hash.
        // R290: `heroHealth` joins them only when it moves the hero off HERO_HEALTH, so a tier that does
        // not set it stores exactly what it stored before the field existed.
        if let Some(handicap) = handicaps.get(player)
            && !is_human_handicap(handicap)
        {
            let hero_health = starting_hero_health(Some(handicap));
            side.handicap = Some(Handicap {
                deck_size: handicap.deck_size,
                mana_bonus: handicap.mana_bonus,
                mana_cap: handicap.mana_cap,
                extra_opening_cards: handicap.extra_opening_cards,
                extra_draws_per_turn: handicap.extra_draws_per_turn,
                hero_health: if hero_health == HERO_HEALTH {
                    None
                } else {
                    Some(hero_health)
                },
            });
            side.hero.health = hero_health;
        }
    }

    // R417, R564: frozen as the match is created, minus every entry this match cannot rebuild.
    state.last_boards =
        crate::subsystems::last_boards::freeze_last_boards(options.last_boards.as_ref(), catalog);
    // R678: the same freeze for the boards a Glitch may lay down.
    state.glitch_boards =
        crate::subsystems::last_boards::freeze_last_boards(options.glitch_boards.as_ref(), catalog);
    // R676: what a Glitch reset deals again.
    state.opening = Some(OpeningRecord {
        decks: (options.decks.0.clone(), options.decks.1.clone()),
        dealt: match &options.dealt {
            Some(dealt) if !dealt.is_empty() => Some(dealt.clone()),
            _ => None,
        },
    });

    state
}

/// R223: the seed suffix of the stream `create_game` numbers the decks' cards from.
pub const INSTANCE_ID_STREAM: &str = ":instance-ids";

/// R223 mid-game: the order a batch of new cards takes its numbers in, when the batch fills a zone
/// nobody may read — #83 Transmogulate replaces a whole library, top down, and numbered in that walk
/// the new ids were one run in library order, so the first id of the next public card told its owner
/// where every library card they are later shown lies (§9.1, §10.8). The order is drawn from the
/// seed's own stream, keyed by the next id to be handed out so each batch draws its own, and the
/// match's rng is untouched.
pub fn numbering_order<T: Clone>(state: &GameState, items: &[T]) -> Vec<T> {
    if items.len() < 2 {
        return items.to_vec();
    }
    Rng::new(
        &format!("{}{}:{}", state.seed, INSTANCE_ID_STREAM, state.next_id),
        0,
    )
    .shuffle(items)
}

/// A deep copy of a state. §10.1 keeps the state JSON-only; in Rust the clone is the derived one
/// (SURFACE §4.4.8: TS's JSON round trip had no other effect on a JSON-only state).
pub fn clone_state(state: &GameState) -> GameState {
    state.clone()
}

pub fn active_units(side: &PlayerState) -> Vec<&CardInstance> {
    side.units
        .iter()
        .filter_map(|pile| pile.as_ref().and_then(|pile| pile.first()))
        .collect()
}

pub fn all_zones_empty(side: &PlayerState) -> bool {
    side.units.iter().all(Option::is_none)
        && side.backrow.iter().all(Option::is_none)
        && side.carried.iter().flatten().all(Option::is_none)
}

/// §2.1, §2.2: whether it is `player`'s turn. Setup (`SETUP_TURN`) is no player's turn: `active`
/// names p1 there only as a placeholder until p1 takes the first turn (§2.1 step 5), so a clause a
/// card makes during setup — a cast on the opening deal or a mulligan's replacement draw (R70) — is
/// made on no turn of its controller's (R155, R241).
pub fn is_turn_of(state: &GameState, player: PlayerId) -> bool {
    state.turn != SETUP_TURN && state.active == player
}

/// Every place a card can be, in `find_instance`'s order, for one side.
fn side_cards(side: &PlayerState) -> impl Iterator<Item = &CardInstance> {
    side.hand
        .iter()
        .chain(side.library.iter())
        .chain(side.graveyard.iter())
        .chain(side.exile.iter())
        .chain(side.units.iter().flatten().flatten())
        .chain(side.backrow.iter().flatten())
        // B5 E21: a backrow pile's dormant cards and a carrier's Unit are on the board too (R446).
        .chain(side.backrow_piles.iter().flatten().flatten())
        .chain(side.carried.iter().flatten().flatten())
        // R98: a card that asks a question mid-resolution is still itself, and §10.5 parks it here
        // between its play and its destination, so a resumed step finds `ctx.self` rather than null.
        .chain(side.resolving.iter())
}

fn side_cards_mut(side: &mut PlayerState) -> impl Iterator<Item = &mut CardInstance> {
    side.hand
        .iter_mut()
        .chain(side.library.iter_mut())
        .chain(side.graveyard.iter_mut())
        .chain(side.exile.iter_mut())
        .chain(side.units.iter_mut().flatten().flatten())
        .chain(side.backrow.iter_mut().flatten())
        .chain(side.backrow_piles.iter_mut().flatten().flatten())
        .chain(side.carried.iter_mut().flatten().flatten())
        .chain(side.resolving.iter_mut())
}

pub fn find_instance<'a>(state: &'a GameState, instance_id: &str) -> Option<&'a CardInstance> {
    PLAYER_IDS
        .into_iter()
        .find_map(|player| side_cards(&state.players[player]).find(|card| card.id == instance_id))
}

/// `find_instance`, mutably: TS handed back the live object, which its callers wrote through.
pub fn find_instance_mut<'a>(state: &'a mut GameState, instance_id: &str) -> Option<&'a mut CardInstance> {
    let in_p1 = side_cards(&state.players.p1).any(|card| card.id == instance_id);
    let side = if in_p1 {
        &mut state.players.p1
    } else {
        &mut state.players.p2
    };
    side_cards_mut(side).find(|card| card.id == instance_id)
}

/// `rowOf`'s answer: a row's zones, which hold piles (units) or single cards (backrow).
#[derive(Clone, Copy, Debug)]
pub enum RowSlots<'a> {
    Units(&'a Vec<Option<Pile>>),
    Backrow(&'a Vec<Option<CardInstance>>),
}

pub fn row_of(side: &PlayerState, row: Row) -> RowSlots<'_> {
    match row {
        Row::Units => RowSlots::Units(&side.units),
        Row::Backrow => RowSlots::Backrow(&side.backrow),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_instances_number_from_next_id() {
        let mut counter: u32 = 7;
        let card = new_instance(
            &mut counter,
            "core-001",
            PlayerId::P2,
            Zone::Hand { player: PlayerId::P2 },
        );
        assert_eq!(card.id, "c7");
        assert_eq!(counter, 8);
        let json = serde_json::to_value(&card).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "id": "c7", "defId": "core-001", "owner": "p2", "controller": "p2", "radiant": false,
                "zone": { "z": "hand", "player": "p2" }, "damage": 0, "buffs": { "attack": 0, "health": 0 },
                "grantedKeywords": [], "vanilla": false, "costMod": 0, "counters": {}, "memory": {},
                "exertion": { "attacked": false, "switched": false }
            })
        );
        assert_eq!(serde_json::from_value::<CardInstance>(json).unwrap(), card);
    }

    #[test]
    fn player_modifiers_flatten_their_kind() {
        let modifier = PlayerModifier {
            id: "m1".into(),
            expiry: ModifierExpiry::NextTurnOf {
                player: PlayerId::P1,
                from_turn: 3,
            },
            kind: ModifierKind::CostDiscount {
                amount: 2,
                only_type: Some(CardType::Spell),
                min_current_cost: None,
                once_per_turn: None,
            },
        };
        let json = serde_json::to_value(&modifier).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "id": "m1", "expiry": { "until": "nextTurnOf", "player": "p1", "fromTurn": 3 },
                "kind": "costDiscount", "amount": 2, "onlyType": "Spell"
            })
        );
        assert_eq!(serde_json::from_value::<PlayerModifier>(json).unwrap(), modifier);
    }

    #[test]
    fn handicaps_and_decks_refuse_with_ts_messages() {
        let bad = Handicap {
            deck_size: 0,
            ..HUMAN_HANDICAP
        };
        assert_eq!(
            validate_handicap(&bad, "p2").unwrap_err().message,
            "p2: handicap deckSize must be between 1 and 60 (R184), got 0"
        );
        let catalog = CardDefs::new();
        assert_eq!(
            validate_deck(&["core-001".to_string()], &catalog, "p1", DECK_SIZE)
                .unwrap_err()
                .message,
            "p1: deck must hold exactly 20 cards (§2.6 L2), got 1"
        );
        assert!(is_human_handicap(&HUMAN_HANDICAP));
    }
}
