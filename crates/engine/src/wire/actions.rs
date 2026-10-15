//! The action union (SPEC §10.2). Every action carries playerId and a client nonce, deduped by the reducer.
//!
//! Port of `packages/shared/src/actions.ts` (part 1's type freeze, SURFACE §6.4).

use serde::{Deserialize, Serialize};

use crate::wire::catalog_types::{PlayerId, Row};
use crate::wire::craft::CraftRecipe;
use crate::wire::emotes::EmoteId;
use crate::wire::string_union;

/// Where a permanent is being played (§3.2: the player picks the zone).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ZoneChoice {
    pub row: Row,
    pub lane: i32,
}

/// One selection inside a play: an instance, a hero, or a zone (R81).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "pick", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Selection {
    Instance {
        instance_id: String,
    },
    Hero {
        player: PlayerId,
    },
    Zone {
        player: PlayerId,
        row: Row,
        lane: i32,
    },
    Mode {
        option: String,
    },
    /// ME-CRAFT (Meditative #17, R880): the recipe a `craft` prompt's answer carries.
    Craft {
        recipe: CraftRecipe,
    },
    None,
}

/// B5 E11, E19: Plague Counters paying part of the price of a play from the graveyard (Classic #74
/// Corpse Plantation): `from` is the card they come off, `tokens` how many — each pays (1).
/// (TS: the `play` action's inline `plague` object; `graveyardPlay.PlagueSpend`.)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PlagueSpend {
    pub from: String,
    pub tokens: i32,
}

/// R783004, R1040, R1044: how a card played face-down as a Trap reveals. A Cloaked card is set
/// `cloaked`: it has no timing, and reveals only when its controller activates its Reveal (R783006).
/// The three timings — the end of the turn it was set in, the start of its controller's next turn,
/// the end of their next turn — were ME-ALTPLAY's, the face-down play Knowledge Breaker and Paranoia
/// granted until patch v0.3.7; nothing offers one now, and they stay so old records still read.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum RevealAt {
    EndOfThisTurn,
    StartOfNextTurn,
    EndOfNextTurn,
    Cloaked,
}

/// R923001: where the practice sandbox puts a card it spawns (`SandboxOp::Spawn`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "zone", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SpawnZone {
    /// Into the hand, as a card added to it (§2.4: a full hand burns it).
    Hand,
    /// Onto the field in that lane: the unit row for a Unit, the backrow for a Trap, a Field Spell or
    /// a Field Trap (a Trap face-down), as a summon puts it there. The zone must be open.
    Field { row: Row, lane: i32 },
    /// Into the graveyard.
    Graveyard,
    /// On top of the library, the next card drawn.
    LibraryTop,
}

/// R923000: one practice-sandbox tool (issue #923). Legal only in a game created as a sandbox
/// (`CreateGameOptions.sandbox`), never offered by `legalActions`, so no policy takes one, and refused
/// by the server's match actor.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SandboxOp {
    /// R923001: a fresh card of `def_id`, base or Radiant, for `player`, placed where `to` says. Placed,
    /// not played: no Cry, no play counted, no cost paid.
    Spawn {
        player: PlayerId,
        def_id: String,
        /// The Radiant form. Only ever `Some(true)`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
        radiant: Option<bool>,
        to: SpawnZone,
    },
    /// R923002: a hero's health (1 to `SANDBOX_HERO_HEALTH_MAX`) and Armor (0 to `SANDBOX_ARMOR_MAX`);
    /// an absent one stays as it is.
    SetHero {
        player: PlayerId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        health: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        armor: Option<i32>,
    },
    /// R923002: a player's current and maximum mana (0 to `SANDBOX_MANA_MAX`); an absent one stays.
    SetMana {
        player: PlayerId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        current: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        max: Option<i32>,
    },
    /// R923002: `count` draws for `player` (1 to `SANDBOX_DRAW_MAX`), each one a draw by §2.4.
    Draw { player: PlayerId, count: i32 },
    /// R923003: destroy a card on the field, as a destroy effect does (its Death runs).
    Destroy { instance_id: String },
    /// R923003: take a card out of play: it is exiled from wherever it is, its Death not run.
    Remove { instance_id: String },
}

impl RevealAt {
    /// Every timing, in the order a turn meets them (none offered since patch v0.3.7). `Cloaked` is no
    /// timing and is not one of them.
    pub const ALL: [RevealAt; 3] = [
        RevealAt::EndOfThisTurn,
        RevealAt::StartOfNextTurn,
        RevealAt::EndOfNextTurn,
    ];
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ActionBody {
    Mulligan {
        keep: Vec<String>,
    },
    Play {
        instance_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        zone: Option<ZoneChoice>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        x: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        embiggen: Option<bool>,
        /// R1086: play this Equip card onto one of its controller's Units (ME-EQUIP). A log
        /// recorded before the rename spells it `magnetic`, which reads as it (R781000).
        #[serde(default, alias = "magnetic", skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        equip: Option<bool>,
        /// Units sacrificed to pay a Tribute cost (§6.3).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        tributes: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        targets: Option<Vec<Selection>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        modes: Option<Vec<String>>,
        /// B5 E11, E19: Plague Counters paying part of the price of a play from the graveyard (Classic #74
        /// Corpse Plantation): `from` is the card they come off, `tokens` how many — each pays (1).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        plague: Option<PlagueSpend>,
        /// R783004, R1040, R1044: play the card face-down into the backrow as a Trap by its Cloaked
        /// (`cloaked`, the one way offered since patch v0.3.7). Absent, the card is played as it is
        /// printed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        face_down: Option<RevealAt>,
    },
    Attack {
        attacker_id: String,
        target_id: String,
    },
    SwitchPosition {
        instance_id: String,
    },
    /// B3.2, R384: use a card's Activate ability. `ability` names it when the card has several; the
    /// targets and modes it declares travel here as a play's do (R81), and `tributes` pays a Tribute its
    /// cost names (Classic #21 Turtinator).
    Activate {
        instance_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        ability: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        targets: Option<Vec<Selection>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        modes: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        tributes: Option<Vec<String>>,
    },
    /// R43, R384: Heroic Power's activation, kept as an alias of `activate` so old logs replay.
    ActivatePower {
        instance_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        targets: Option<Vec<Selection>>,
    },
    Answer {
        choice_id: String,
        selection: Vec<Selection>,
    },
    OfferDraw,
    AnswerDraw {
        accept: bool,
    },
    /// MD-D29, R1127: an emote as a move — legal only while a card hears it (`query::emotes_heard`),
    /// so other games never list it and `reduce` refuses it there.
    Emote {
        emote: EmoteId,
    },
    Concede,
    EndTurn,
    /// R345: the sender's own preference for R82's automatic turn end. A setting, not a move: it is
    /// accepted from either seat at any moment of a live game, changes nothing on the board, and is
    /// never offered by `legalActions`, so no policy ever sends it.
    SetAutoEndTurn {
        enabled: bool,
    },
    /// R923000: a practice-sandbox tool, from either seat on either turn, in a sandbox game only.
    Sandbox {
        op: SandboxOp,
    },
    // Server-only (R79): never sent by a client.
    Timeout,
    DisconnectExpired {
        player: PlayerId,
    },
    CeilingReached,
}

string_union! {
    /// `ActionBody["type"]`.
    pub enum ActionType {
        Mulligan = "mulligan",
        Play = "play",
        Attack = "attack",
        SwitchPosition = "switchPosition",
        Activate = "activate",
        ActivatePower = "activatePower",
        Answer = "answer",
        OfferDraw = "offerDraw",
        AnswerDraw = "answerDraw",
        Emote = "emote",
        Concede = "concede",
        EndTurn = "endTurn",
        SetAutoEndTurn = "setAutoEndTurn",
        Sandbox = "sandbox",
        Timeout = "timeout",
        DisconnectExpired = "disconnectExpired",
        CeilingReached = "ceilingReached",
    }
}

impl ActionBody {
    /// `action.type`.
    pub fn action_type(&self) -> ActionType {
        match self {
            ActionBody::Mulligan { .. } => ActionType::Mulligan,
            ActionBody::Play { .. } => ActionType::Play,
            ActionBody::Attack { .. } => ActionType::Attack,
            ActionBody::SwitchPosition { .. } => ActionType::SwitchPosition,
            ActionBody::Activate { .. } => ActionType::Activate,
            ActionBody::ActivatePower { .. } => ActionType::ActivatePower,
            ActionBody::Answer { .. } => ActionType::Answer,
            ActionBody::OfferDraw => ActionType::OfferDraw,
            ActionBody::AnswerDraw { .. } => ActionType::AnswerDraw,
            ActionBody::Emote { .. } => ActionType::Emote,
            ActionBody::Concede => ActionType::Concede,
            ActionBody::EndTurn => ActionType::EndTurn,
            ActionBody::SetAutoEndTurn { .. } => ActionType::SetAutoEndTurn,
            ActionBody::Sandbox { .. } => ActionType::Sandbox,
            ActionBody::Timeout => ActionType::Timeout,
            ActionBody::DisconnectExpired { .. } => ActionType::DisconnectExpired,
            ActionBody::CeilingReached => ActionType::CeilingReached,
        }
    }
}

/// `ActionBody & { playerId: PlayerId; nonce: string }`: the body's fields flattened beside the
/// sender and the nonce, as TS writes it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    #[serde(flatten)]
    pub body: ActionBody,
    pub player_id: PlayerId,
    pub nonce: String,
}

impl Action {
    pub fn new(body: ActionBody, player_id: PlayerId, nonce: impl Into<String>) -> Action {
        Action {
            body,
            player_id,
            nonce: nonce.into(),
        }
    }

    /// `action.type`.
    pub fn action_type(&self) -> ActionType {
        self.body.action_type()
    }
}

/// An action without its nonce, which the caller or the server adds (`ActionBody & { playerId }`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ActionInput {
    #[serde(flatten)]
    pub body: ActionBody,
    pub player_id: PlayerId,
}

impl ActionInput {
    /// The action with its nonce.
    pub fn with_nonce(self, nonce: impl Into<String>) -> Action {
        Action {
            body: self.body,
            player_id: self.player_id,
            nonce: nonce.into(),
        }
    }
}

/// Actions the non-active player may take (BUILD M1-T3).
pub const NON_ACTIVE_ACTION_TYPES: &[ActionType] = &[
    ActionType::Answer,
    ActionType::Concede,
    ActionType::AnswerDraw,
    // MD-D29, R1127: either seat may emote while a card hears it.
    ActionType::Emote,
    ActionType::SetAutoEndTurn,
    // R923000: the sandbox's tools arrange the board on either turn (`sandbox.rs` refuses them outside a
    // sandbox game).
    ActionType::Sandbox,
    ActionType::DisconnectExpired,
    ActionType::Timeout,
    ActionType::CeilingReached,
];

/// The actions that may be sent while a prompt is open (BUILD M1-T3).
pub const PROMPT_OPEN_ACTION_TYPES: &[ActionType] = &[
    ActionType::Answer,
    ActionType::Mulligan,
    ActionType::Concede,
    ActionType::SetAutoEndTurn,
    ActionType::Timeout,
    ActionType::DisconnectExpired,
    ActionType::CeilingReached,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_serialise_as_ts_writes_them() {
        let action = Action::new(
            ActionBody::Play {
                instance_id: "c12".into(),
                zone: Some(ZoneChoice {
                    row: Row::Units,
                    lane: 2,
                }),
                x: None,
                embiggen: None,
                equip: None,
                tributes: None,
                targets: Some(vec![Selection::Hero { player: PlayerId::P2 }, Selection::None]),
                modes: None,
                plague: None,
                face_down: None,
            },
            PlayerId::P1,
            "n1",
        );
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "play", "instanceId": "c12", "zone": { "row": "units", "lane": 2 },
                "targets": [{ "pick": "hero", "player": "p2" }, { "pick": "none" }],
                "playerId": "p1", "nonce": "n1"
            })
        );
        assert_eq!(serde_json::from_value::<Action>(json).unwrap(), action);
        let end: Action = serde_json::from_str(r#"{"type":"endTurn","playerId":"p2","nonce":"x"}"#).unwrap();
        assert_eq!(end.action_type(), ActionType::EndTurn);
        // MD-D29, R1127: the emote action rides the wire as `{"type":"emote","emote":…}`.
        let emote: Action =
            serde_json::from_str(r#"{"type":"emote","emote":"greetings","playerId":"p1","nonce":"e"}"#)
                .unwrap();
        assert_eq!(emote.action_type(), ActionType::Emote);
        assert_eq!(
            serde_json::to_value(&emote).unwrap(),
            serde_json::json!({"type":"emote","emote":"greetings","playerId":"p1","nonce":"e"})
        );
    }
}
