//! What a player is allowed to see (SPEC §10.8). The client renders this and nothing else.
//!
//! Port of `packages/shared/src/view.ts` (part 1's type freeze, SURFACE §6.4). Two TS unions are
//! discriminated on a boolean, which serde cannot tag on: `BackrowView` (`faceDown`) and
//! `PendingView` (`forYou`). Each is an enum of two structs that carry the boolean themselves;
//! deserialization reads the boolean first, so a malformed member is an error, never the other member.
//! TS's `BackrowView` includes `null`; here an empty backrow zone is `Option::<BackrowView>::None`.

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::state::{DuosLife, Slumber, StarshipMemory};
use crate::wire::actions::RevealAt;
use crate::wire::catalog_types::{
    CardDef, CardElement, CardType, Keyword, KeywordKind, PlayerId, PromptKind, Row, Tag,
};
use crate::wire::craft::CraftRecipe;
use crate::wire::events::{GameEvent, GameResult, Position, SecretChoice, StarscapeReward};
use crate::wire::string_union;

string_union! {
    /// The phase of a game (§2, `GameState.phase`, `PlayerView.phase`).
    pub enum Phase {
        Setup = "setup",
        Mulligan = "mulligan",
        Start = "start",
        Main = "main",
        End = "end",
        Over = "over",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardView {
    pub instance_id: String,
    pub def_id: String,
    pub radiant: bool,
    /// ME-CN, R1301: the card is shown in Chinese (`CardInstance.chinese`). Only on a card the viewer
    /// may read, so never on the sentinel, a face-down card someone else controls or an opponent's hand.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub chinese: Option<bool>,
    /// MD-B6, R943: the card was minted after the decks were built (`CardInstance.created`). Only on
    /// a card the viewer may read, as `chinese` is. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub created: Option<bool>,
    /// R1956001: on the viewer's own Created card, the catalog id of the card that generated it
    /// (`GameState.generators`), whose foil, finish and border the client dresses it in. Never on
    /// another player's card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_by: Option<String>,
    /// Cost as it stands now (§6.3 Cost, R65); "X" cards show 0 until X is chosen.
    pub cost: i32,
    /// #492, R81, §10.8: an "A embiggen B" card in the viewer's own hand, what a play of it at its
    /// embiggen price costs now, every cost change applied: the price §10.5 step 1 reads for a play
    /// with `embiggen: true` (`play_choices::embiggen_play_cost`), where `cost` is its normal price.
    /// Absent on every other card, and never on the opponent's cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub embiggen_cost: Option<i32>,
    /// #2268, R2268000, §10.8: a card in the viewer's own hand whose play asks a Tribute (§6.3
    /// "Tribute X"), how many Tributes a play of it asks now (`play_choices::tribute_cost_of`): the
    /// printed X as a Nerf or a Buff left it (R386), less what its owner's Floats took (R1841003),
    /// never what the units that could pay it are worth. Absent at 0, on every other card, and never
    /// on the opponent's cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tribute: Option<i32>,
    /// R243: a Unit card's stats in its owner's hand, as they stand: its printed face (the radiant one
    /// when it is Radiant, a fused card's summed one) plus the permanent buffs it has gained there
    /// (§10.4 layers 1, 3 and 4 — #89 Corpse Eater feeds in hand). Set on the viewer's own hand cards
    /// only; a unit on the field reads its layers off `UnitView`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub health: Option<i32>,
    /// R243, R43, R151: the power a #98 Heroic Power in its owner's hand rolled as it arrived, by name.
    /// Its X is the card's cost, which does not name it: four of the eight powers cost the same.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub power: Option<String>,
    /// R2247000: the word a card in its owner's hand shows in place of its text, which it holds in its
    /// memory (`effects::CARD_LABEL_KEY`): Anomaly Found!'s board code, "FO". Set on the viewer's own
    /// hand cards only. Absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// R195, §10.8: Hearthstone's yellow glow. Present, and `true`, only on the viewer's own card
    /// whose printed condition holds now. Absent otherwise: never `false`, never on the opponent's
    /// cards. `UnitView` and the public `BackrowView` inherit it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub condition_active: Option<bool>,
    /// R667, §10.8: Classic #87 Plague Chalice's warning. Present, and `true`, only on the viewer's own
    /// hand card that a card on the field the viewer may read would counter at every price it could be
    /// played at now. Absent otherwise: never `false`, never on the opponent's cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub countered_on_play: Option<bool>,
    /// R1844001, §10.8: the Pareto hint (Stinkyscape #147 IN33DH3). Present, and `true`, only on the
    /// viewer's own hand card that the judge would call optimal to play now while the viewer is shown
    /// the hint (`pareto::pareto_hint_ids`). Absent otherwise: never `false`, never on the opponent's
    /// cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub pareto_optimal: Option<bool>,
    /// R979000, §10.8: why the viewer's own hand card cannot be played now
    /// (`play_choices::why_hand_card_unplayable`), as `UnitView.attack_reason` (R849000) is for an
    /// attack; R1571000: and why a card in the viewer's own graveyard that a permission admits
    /// (`play_choices::why_graveyard_card_unplayable`) cannot be played now. Set only on those
    /// cards while they may act (their turn, the main phase, no prompt open) and
    /// `legal_actions` lists no play of the card. Absent otherwise, never
    /// `false` or empty, so a view with no unplayable card is byte for byte what it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub play_reason: Option<String>,
    /// R1838000, §10.8: why the viewer's own Tradable hand card cannot be Traded now
    /// (`trade::why_trade_refused`), as `play_reason` is for a play. Set only on those cards while the
    /// viewer may act (their turn, the main phase, no prompt open) and `legal_actions` lists no `trade`
    /// of the card. Absent otherwise, never empty, so a view with no Tradable card is what it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trade_reason: Option<String>,
    /// R2574000, §10.8: whether this hand card of the viewer's is Tradable (`trade::is_tradable`),
    /// whether by its printed face, a granted keyword or an aura over its controller's hand.
    /// Present, and `true`, only on the viewer's own hand card in a live game; absent otherwise,
    /// never `false`, never on the opponent's cards or post-game hands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub tradable: Option<bool>,
    /// #992, R90, §10.8: `legalActions` lists plays of this card in the viewer's own hand and R90's bound
    /// dropped some of the choice combinations they are built from, so the list is a slice whose first
    /// plays offer every pick. Present, and `true`, only then; absent otherwise, never `false`, never on
    /// the opponent's cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub choices_truncated: Option<bool>,
    /// R280, §10.8: what the card's formula comes to now, one entry per labeled number its script's
    /// `preview` hook returns — the label the formula as the running face prints it, the value what it
    /// would come to if the card resolved now. Only on a card view the viewer may read: its own hand, a
    /// unit on top of its pile, a face-up backrow card, a face-down one for its controller. Absent when
    /// the hook returns nothing or the card has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub preview: Option<Vec<PreviewValue>>,
    // ---- Patch v0.2.0 (docs/classic-sets.md B2.7, B3, B5) ----
    /// B2.7: the type the card has now, set only where it differs from its definition's — a face with
    /// its own type (Classic+ #22 Blood Moon's Radiant face is a Field Trap).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub type_: Option<CardType>,
    /// MD-B15, R923: the card's tags, set only where they differ from its definition's — a granted
    /// tag is part of the card, as CN is of a card in Chinese (R1176000), and the view says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tags: Option<Vec<Tag>>,
    /// B3.3, R385: the card's Brittle count, where the viewer may read the card and it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub brittle: Option<i32>,
    /// R980: the card's element (Meditative #40 Feng Shui), where the viewer may read the card and a
    /// Feng Shui acts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub element: Option<CardElement>,
    /// B3.4, R386: the card's declared numbers as they stand now (its face's `params`, moved by
    /// Degrade, Upgrade and KY's Constant), by key, which the client fills into the face's `{key}`s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub params: Option<IndexMap<String, i32>>,
    /// B3.4, R386: what Degrade and Upgrade have changed on the card, where the viewer may read it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tuning: Option<Tuning>,
    /// B5 E39: the enchantments riding the card (Classic+ #14's return, #40's cast on draw).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub enchantments: Option<Vec<Enchantment>>,
    /// B5 E38, R243: a card in the viewer's own hand, its keywords as it will carry them onto the field
    /// — printed as Degrade and Upgrade left them, and those it was granted in the hand or the deck —
    /// set only where they differ from its face's printed keywords. A unit on the field reads its
    /// keywords off `UnitView`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub keywords: Option<Vec<Keyword>>,
    /// R437: the marks on the card — an effect aimed at it and waiting (K-Pop Fanatic's steal). Both views.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub marks: Option<Vec<CardMark>>,
    /// B3.2, R384: the card's Activate abilities, on its controller's own view of it on the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub activations: Option<Vec<ActivationView>>,
    /// B5 E33, R404: Classic #90 In Too Deep's open quests with their progress and the rewards on offer,
    /// and the auras its quest line holds — on every view of the card on the field (a face-up Field Spell).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quest: Option<QuestView>,
    /// B5 E14, R399, R243: the Spell whose text a copier (Classic #57 Echo) has now, on the face it was
    /// played on, with that definition's declared numbers as they read on this card (`params`, R386) —
    /// on its owner's view of it in hand, and on both views while it resolves (its play is public). The
    /// card keeps its own name, cost and type; absent when it copies nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub copies: Option<CopiedTextView>,
    /// R1848003: the X an X-cost card is set at (`CardInstance.fixed_x`), which is its price (`cost`)
    /// and the X it is played for, so the face prints the number where it would print X. Only on a
    /// card the viewer may read; absent on every other card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fixed_x: Option<i32>,
}

/// One reward on offer, or one aura held, as `QuestView` lists it: `{ id, text }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct QuestItemView {
    pub id: String,
    pub text: String,
}

/// One open quest as `QuestView.open` lists it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct QuestOpenView {
    pub id: String,
    pub text: String,
    pub progress: i32,
    pub goal: i32,
    pub rewards: Vec<QuestItemView>,
    /// R1854002: the cost a play must have paid to meet a "play a card that costs …" goal, the number
    /// the quest card remembers when it holds one. Absent for every other goal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost: Option<i32>,
}

string_union! {
    /// R1854000: where a staged quest's line stands with one of its stages.
    pub enum QuestStageState {
        Done = "done",
        Current = "current",
        Upcoming = "upcoming",
    }
}

/// R1854000, §10.8: one stage of a staged quest as the board shows it, to both players: its name, its
/// state, its goals (the one drawn once it has opened, every one it may draw before), its reward's text,
/// and, while the line waits to hear back with it, the turns left (R1854004).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct QuestStageView {
    pub name: String,
    pub state: QuestStageState,
    pub goals: Vec<String>,
    pub reward: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hear_back: Option<i32>,
}

/// B5 E33, R404, §10.8: a quest line as the board shows it. Each open quest carries its text, its
/// progress against its goal ("1/2") and the rewards it offers; `auras` are the rewards held while the
/// card stays on the field (In Too Deep's L and M). R1854000: a staged quest's line adds its stages,
/// in order; absent on a tree's line.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct QuestView {
    pub open: Vec<QuestOpenView>,
    pub auras: Vec<QuestItemView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stages: Option<Vec<QuestStageView>>,
}

/// B5 E14, R399: what a copier's text is now (`CardView.copies`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CopiedTextView {
    pub def_id: String,
    pub radiant: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub params: Option<IndexMap<String, i32>>,
}

/// B3.4, R386: the lasting changes Degrade, Upgrade and KY's Constant made to one card, kept in every
/// zone and through leaving the field (R78 does not reset it). The cost change is the card's
/// `costMod`, not a field here. `x` holds a step per numbered keyword or X ("Armor", "Echo",
/// "Activate", "Brittle", "Spell Damage", "Tribute", "Lucky", "X"), `numbers` a step per declared
/// number (`CardDef.params` key), `set` a declared number or numbered keyword set outright (KY's
/// Constant's "to 3"), which wins over the steps.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Tuning {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub health: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub add_keywords: Option<Vec<Keyword>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub remove_keywords: Option<Vec<KeywordKind>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub x: Option<IndexMap<String, i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub numbers: Option<IndexMap<String, i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub set: Option<IndexMap<String, i32>>,
}

/// B5 E39: a lasting instruction that rides a card through every zone. `returnAfterResolve` is Classic+
/// #14 Forever&'s "After this resolves, return it to hand. This can't cost less than (floor)";
/// `castOnDraw` and `targetEnemies` are Classic+ #40 Appropriations' "They have Cast on draw and aim
/// at enemies when they harm and at your side when they help". `swapsBook` is Classic #55's swap.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Enchantment {
    ReturnAfterResolve {
        floor: i32,
    },
    CastOnDraw,
    TargetEnemies,
    /// Classic #55 Book of Wildfire's "Becomes a different Book at the end of your turn", carried by the
    /// Book it became so the swap goes on (R671). `from` is the card that started it, which no swap picks.
    SwapsBook {
        from: String,
    },
}

/// R437: a mark on a card, and the color key the client draws it with ("purple").
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardMark {
    pub mark: String,
    pub color: String,
}

/// B3.2, R384: one Activate ability as its controller's client needs it. `usesLeft` is how many more
/// times it may be used this turn, null for Activate ♾️ (bounded only by `ACTIVATE_UNLIMITED_CAP`).
/// `usable` is `legalActions`' answer now; `reason` says why not when it is false.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ActivationView {
    pub ability: String,
    pub label: String,
    pub uses_left: Option<i32>,
    pub usable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<String>,
}

/// R280: one number a card's formula comes to now, and the formula it is ("+1 per card in your exile").
/// `display` is how the value prints when the text names it by a word rather than a numeral: #93
/// Combo-Index's grade 3 prints as its letter, "C" (R372). A client prints `display` when present and
/// the number otherwise, and never works one out from the other.
///
/// `ids` is the set of cards the value counts, by instance id, when the formula is a set of cards
/// rather than a number alone: Classic+ #44 Simplicity Audit's and #45 Complexity Audit's Radiant
/// "highlight targets", the permanents the card would exile now (docs/classic-sets.md C+ #44). A
/// client marks those cards on the board; the hook never names a card its controller may not read.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PreviewValue {
    pub label: String,
    pub value: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub display: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ids: Option<Vec<String>>,
}

/// R2321000, §10.8: a count a card on the field keeps on this Unit (`Script.tallies`), `value` of
/// `goal`, and the card keeping it by its catalog id (TG #31 Level Up!: the Unit's attacks toward its
/// Radiant). Only from a card the viewer may read where it lies, so a face-down card's count never
/// reaches the other seat.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TallyView {
    pub def_id: String,
    pub value: i32,
    pub goal: i32,
}

/// `{ plague?: number; grade?: number }`: a unit's counters (`UnitView.counters`,
/// `CardInstance.counters`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Counters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plague: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grade: Option<i32>,
}

/// `UnitView.animated`: B3.1, R383.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AnimatedView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub home: Option<i32>,
}

/// `CardView & { … }`: a unit on the field. Every `CardView` field is repeated here, since the unit
/// makes `attack`, `health` and `keywords` required where the card view has them optional — all but
/// `embiggenCost` and `tribute`, which only a card in its owner's hand carries (#492, #2268).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct UnitView {
    // ---- CardView ----
    pub instance_id: String,
    pub def_id: String,
    pub radiant: bool,
    pub cost: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub power: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub condition_active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub countered_on_play: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub preview: Option<Vec<PreviewValue>>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub type_: Option<CardType>,
    /// MD-B15, R923: the card's tags, set only where they differ from its definition's — a granted
    /// tag is part of the card, as CN is of a card in Chinese (R1176000), and the view says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub brittle: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub element: Option<CardElement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub params: Option<IndexMap<String, i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tuning: Option<Tuning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub enchantments: Option<Vec<Enchantment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub marks: Option<Vec<CardMark>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub activations: Option<Vec<ActivationView>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quest: Option<QuestView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub copies: Option<CopiedTextView>,
    // ---- UnitView ----
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub attack: i32,
    pub max_health: i32,
    pub health: i32,
    pub keywords: Vec<Keyword>,
    pub armor: i32,
    pub position: Position,
    pub counters: Counters,
    /// Cards under this one in a Stack pile are face-down and dormant (§3.2).
    pub buried: i32,
    pub can_act: bool,
    /// R243, §6.3 Vanilla, R115: the unit's text is gone — its printed keywords and every script, the
    /// ones its definition still names included — so a client shows none of it. Absent otherwise.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub vanilla: Option<bool>,
    /// B3.1, R383: a Field Spell, Trap or Field Trap standing in a unit zone as a Unit. `home` is the
    /// backrow lane an "Animated on your turn" card returns to at its controller's cleanup, when it has
    /// one (that zone is reserved for it meanwhile, `SideView.reserved`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub animated: Option<AnimatedView>,
    /// B5 E35: the unit has gone Berserk (a status, lost when it leaves the field). Absent otherwise.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub berserk: Option<bool>,
    /// R1174000: the unit is Frozen (a status, public on the field on both seats): it can neither
    /// attack nor switch its own position, nor (R2246000) use an Activate, until it thaws. Absent
    /// otherwise. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub frozen: Option<bool>,
    /// R1839000, R1839001: the card is Dormant (a status, public on the field on both seats): it can
    /// neither attack, be attacked, be targeted nor be reached by an effect until it awakens, and
    /// `turns` is how many more of its controller's turns start before it does (absent: until an effect
    /// awakens it). What the board's Dormant badge and its countdown read. Absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub slumber: Option<Slumber>,
    /// R1848000: the unit is Frozen forever (Stinkyscape #2 Absolute Zero), so it never thaws; public
    /// as `frozen` is. Absent otherwise. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub frozen_forever: Option<bool>,
    /// R1852000, R1852005: the card is a Starship (public on the field on both seats): `pieces` the
    /// Starship Pieces fused into it, oldest first, and, once it has launched, `name`, the name it took.
    /// What the board's Starship badge and its list of pieces read. Absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub starship: Option<StarshipMemory>,
    /// ME-CN, R1301: the card is shown in Chinese (`CardInstance.chinese`). Only on a card the viewer
    /// may read, so never on the sentinel, a face-down card someone else controls or an opponent's hand.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub chinese: Option<bool>,
    /// MD-D5, R1121: the enemy Units an `attack_mods` entry would apply to if this Unit attacked them
    /// now — the drag layer's yellow, beside the green legal glow (R195's sibling). Only on the
    /// viewer's own attackers that may act now, and only when the list is non-empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub condition_targets: Option<Vec<String>>,
    /// MD-B6, R943: the card was minted after the decks were built (`CardInstance.created`). Only on
    /// a card the viewer may read, as `chinese` is. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub created: Option<bool>,
    /// R1956001: on the viewer's own Created card, the catalog id of the card that generated it
    /// (`GameState.generators`), whose foil, finish and border the client dresses it in. Never on
    /// another player's card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_by: Option<String>,
    /// ME-GRANT (MD-D13): the Death abilities other cards granted this Unit, as its lines read them
    /// (`grants::grant_texts`). Absent when it holds none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grants: Option<Vec<String>>,
    /// §4.1, R849000: summoning sick with neither Rush nor Charge (`combat::is_asleep`), on both seats.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub sick: Option<bool>,
    /// R849000: why the viewer's own unit cannot declare an attack now (`combat::why_cannot_declare`),
    /// as `ActivationView.reason` (R384). Only while `canAct`, or while the unit is Frozen in its
    /// controller's main phase (R1174000); never on the opponent's units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack_reason: Option<String>,
    /// R2321000: the counts cards on the field the viewer may read keep on this Unit, in R68's order
    /// of the cards keeping them. Absent when none does, never `[]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tallies: Option<Vec<TallyView>>,
}

/// A public backrow card's counters: `grade` is #93 Combo-Index's counter, 1..6; `gradeLetter` is the
/// letter that number is, E..S, which the engine names so a client prints it rather than working it
/// out (R372). `plague` is the card's Plague Counters (§6.3, B5 E19), absent at none.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BackrowCounters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grade: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grade_letter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plague: Option<i32>,
}

/// The public member of `BackrowView`: `CardView & { faceDown: false; type: CardType; … }`. Every
/// `CardView` field is repeated here, since this one makes `type` required — all but `embiggenCost`
/// and `tribute`, which only a card in its owner's hand carries (#492, #2268).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PublicBackrowView {
    // ---- CardView ----
    pub instance_id: String,
    pub def_id: String,
    pub radiant: bool,
    /// ME-CN, R1301: the card is shown in Chinese (`CardInstance.chinese`). Only on a card the viewer
    /// may read, so never on the sentinel, a face-down card someone else controls or an opponent's hand.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub chinese: Option<bool>,
    /// MD-B6, R943: the card was minted after the decks were built (`CardInstance.created`). Only on
    /// a card the viewer may read, as `chinese` is. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub created: Option<bool>,
    /// R1956001: on the viewer's own Created card, the catalog id of the card that generated it
    /// (`GameState.generators`), whose foil, finish and border the client dresses it in. Never on
    /// another player's card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_by: Option<String>,
    pub cost: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub health: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub power: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub condition_active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub countered_on_play: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub preview: Option<Vec<PreviewValue>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub brittle: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub element: Option<CardElement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub params: Option<IndexMap<String, i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tuning: Option<Tuning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub enchantments: Option<Vec<Enchantment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub keywords: Option<Vec<Keyword>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub marks: Option<Vec<CardMark>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub activations: Option<Vec<ActivationView>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub quest: Option<QuestView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub copies: Option<CopiedTextView>,
    // ---- the public backrow card ----
    /// Always `false` here (the discriminant).
    #[cfg_attr(feature = "ts", ts(type = "false"))]
    pub face_down: bool,
    #[serde(rename = "type")]
    pub type_: CardType,
    /// MD-B15, R923: the card's tags, set only where they differ from its definition's — a granted
    /// tag is part of the card, as CN is of a card in Chinese (R1176000), and the view says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tags: Option<Vec<Tag>>,
    pub counters: BackrowCounters,
    pub owner: PlayerId,
    pub controller: PlayerId,
    /// R351, R371: present, and `true`, on the controller's own view of a Trap or Field Trap that
    /// is still face-down: the controller reads the card (R33), and the other player sees only its
    /// back. Absent on every public card and on a Field Trap that has fired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub unrevealed: Option<bool>,
    /// ME-ALTPLAY (R1046): when a face-down play of yours reveals. Present on the controller's own
    /// view only, never on the opponent's, and skipped when the card is not a face-down play (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reveal_at: Option<RevealAt>,
    /// R243, §6.3 Vanilla: the backrow card's text is gone — a client stamps it as it stamps a
    /// vanilla unit. Absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub vanilla: Option<bool>,
    /// B5 E21: face-down, dormant cards beneath this one in a backrow pile (§3.2). Absent for none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub buried: Option<i32>,
    /// R1081000: the Unit standing in this backrow zone, when the public top card is one — a Flying
    /// Unit there is a Unit for every rule, so the client draws it as the minion it is. Units are
    /// public in any zone, so this carries no hidden information; absent for every other backrow card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<UnitView>,
}

/// The face-down member of `BackrowView`: a marker. §10.8 grants the non-controller the fact that
/// something is there and what it costs (R351), and nothing else.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FaceDownBackrowView {
    /// Always `true` here (the discriminant).
    #[cfg_attr(feature = "ts", ts(type = "true"))]
    pub face_down: bool,
    /// R351, R370: a face-down Trap shows its cost to both players, the number its controller's own
    /// view shows (§6.3 Cost, R65). Always set by `viewFor`; optional so a client draws a back with
    /// or without it (a view built before the patch, a test fixture).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost: Option<i32>,
    /// B5 E19, R471: the Plague Counters on the face-down card. Tokens are public wherever they sit, so
    /// both players see the count on the card's back; the card stays hidden. Absent at none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plague: Option<i32>,
    /// B5 E21: how many dormant cards lie beneath it in a backrow pile — a count, never an identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub buried: Option<i32>,
    /// R437: the marks the face-down card carries — an effect aimed at it that waits (#50's
    /// pending steal) — which the player who may not read it sees on its back (R33).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub marks: Option<Vec<CardMark>>,
}

/// A backrow zone: a public card or a face-down trap (§3, §10.8); an empty zone is `None` where it is
/// held. A public card names its `owner` and `controller` like a `UnitView` does, because R33 keys
/// readability on the controller: after a steal (#36 radiant, #49), a board swap (#87) or a rotation
/// (#52) the card sits in a backrow that is not its controller's, and the view says so rather than
/// leaving the client to track `controlChanged` out of band. A face-down zone stays a marker: §10.8
/// grants the non-controller the fact that something is there and what it costs (R351), and nothing
/// else.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(untagged)]
#[allow(
    clippy::large_enum_variant,
    reason = "a view is built, serialized and dropped; a Box buys nothing"
)]
pub enum BackrowView {
    Public(PublicBackrowView),
    FaceDown(FaceDownBackrowView),
}

impl<'de> Deserialize<'de> for BackrowView {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value.get("faceDown").and_then(serde_json::Value::as_bool) {
            Some(false) => serde_json::from_value(value)
                .map(BackrowView::Public)
                .map_err(D::Error::custom),
            Some(true) => serde_json::from_value(value)
                .map(BackrowView::FaceDown)
                .map_err(D::Error::custom),
            None => Err(D::Error::custom("a backrow view needs a boolean `faceDown`")),
        }
    }
}

/// A Heroic Power on the field (§8 #98, R43, R752), as the client needs it to act.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct HeroPowerView {
    /// #98's instance, so its `activate {instanceId}` is built from the view alone (§10.2).
    pub instance_id: String,
    pub def_id: String,
    /// The power's stored name (R103), the id of its Activate ability.
    pub name: String,
    /// The power's name on the card's face (R752): "Expedition Map", "Tank Up" on a Radiant Armor Up.
    pub title: String,
    /// "Activate: Spend (X)" (R752): the mana each use pays.
    pub x: i32,
    pub used_this_turn: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct HeroView {
    pub health: i32,
    /// The Armor §4.4 step 2 subtracts from each hit on this hero, as a computed total and not a
    /// stored field: whatever is written on the hero plus every backrow card granting it (#84 Going
    /// Long), which R124 adds up. So it drops back when a granting card leaves the backrow, exactly
    /// like a unit's `armor` above.
    pub armor: i32,
    /// Every Heroic Power this player controls, in board order. Usually none or one, but #36 radiant
    /// and #49 steal a backrow permanent and a Field Spell is one (§3), so a player holding their own
    /// #98 can come to control the opponent's as well — and each is separately once-per-turn (R43).
    pub powers: Vec<HeroPowerView>,
    /// `powers[0] ?? null`: the one a single-button hero panel shows.
    pub power: Option<HeroPowerView>,
    /// #936 S3, B61: in a Duos duel, this hero's team pool and whether its own HP is spent, public to
    /// both seats so the board draws both bars. Absent in a 1v1 game, whose view is as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub duos: Option<DuosLife>,
}

/// One player-level modifier (§10.1 `PlayerState.mods`) as the hero panel shows it.
///
/// `id` is the `PlayerModifier.id` that §10.3's `modifierChanged` event already names on both
/// seats, so the badge an animation plays on is the badge the view carries. `label` is a short
/// caption built from the modifier's own kind and numbers — and its timing while R48 keeps it
/// dormant — and it is the *whole* of what a modifier reveals: never `sourceId`, never the card
/// that installed it, so nothing that could name a face-down card rides out on a badge.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ModifierView {
    pub id: String,
    pub label: String,
}

/// R310: one kind of card left in the viewer's own library: a definition, the face it went in with
/// (R311) and how many such cards are there. No instance id and no position, so nothing in it can
/// say where a card lies.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntryView {
    pub def_id: String,
    pub radiant: bool,
    pub count: i32,
    /// MD-B6, R943: the entry's cards are Created (`CardInstance.created`, read off the live card
    /// the owner's `known_as` record names). Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub created: Option<bool>,
}

/// R310–R312: the viewer's own library as a list without order. `cards` holds what the viewer was
/// shown of each card as it went in, one entry per definition and face, sorted by printed cost, then
/// name, then id, base face first (R310): an order that depends on the cards alone, never on where
/// they lie. `unknown` counts the cards the viewer was never shown (R312: a library Pocket Chaos
/// swapped in, Transmogulate's picks), which a client draws as backs. The two add up to
/// `libraryCount`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LibraryView {
    pub cards: Vec<LibraryEntryView>,
    pub unknown: i32,
}

/// `{ current: number; max: number }`: `SideView.mana`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ManaView {
    pub current: i32,
    pub max: i32,
}

/// `CardView[] | { count: number }`: full cards for the viewer, a count only for the opponent (§10.8).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(untagged)]
pub enum HandView {
    Cards(Vec<CardView>),
    Count { count: i32 },
}

/// `{ units: boolean[]; backrow: boolean[] }`, one flag per lane of each row: `SideView.locks` and
/// `.reserved`, and `PlayerState.locks`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RowFlags {
    pub units: Vec<bool>,
    pub backrow: Vec<bool>,
}

impl RowFlags {
    /// `flags[row]`.
    pub fn row(&self, row: Row) -> &Vec<bool> {
        match row {
            Row::Units => &self.units,
            Row::Backrow => &self.backrow,
        }
    }

    pub fn row_mut(&mut self, row: Row) -> &mut Vec<bool> {
        match row {
            Row::Units => &mut self.units,
            Row::Backrow => &mut self.backrow,
        }
    }
}

/// R1223–R1225: what the client shows of a credit line — how much more its controller can borrow
/// (`available`, while a line acts), the instalment each coming refresh owes (`owed`, next first),
/// the instalment the last refresh took (`locked`), and, while a lapsing face acts, whether the line
/// was used this turn (`used`). Public: lenders are face-up field cards and the schedule is public.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CreditView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub available: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owed: Vec<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locked: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub used: Option<bool>,
}

/// R1847000: one node of a starscape map, its reward and the reward's amount (config
/// `STARSCAPE_REWARDS`); the edge has none.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct StarscapeNodeView {
    pub reward: StarscapeReward,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub amount: Option<i32>,
}

/// R1847000: a seat's starscape map, public on both seats: its nodes in order, the edge last, and the
/// node the player stands on (`at`, 0 the start).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct StarscapeView {
    pub nodes: Vec<StarscapeNodeView>,
    pub at: i32,
}

/// ME-SECRET, R860: one secret of a side, as the viewer reads it — the choice only for its owner,
/// or for anyone once revealed (R864).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SecretView {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub choice: Option<SecretChoice>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SideView {
    pub player: PlayerId,
    pub hero: HeroView,
    /// The player modifiers on this seat, in the order they were installed. Public on BOTH seats:
    /// every modifier in the Core set is installed by the Cry of a card played face-up (§10.5 step 4,
    /// #35, #77, #78, #79), and `modifierChanged` is already an unredacted event for both players, so
    /// the label states only what the public play already said. Nothing derived from a hidden card
    /// travels with it (see `ModifierView`).
    pub modifiers: Vec<ModifierView>,
    pub mana: ManaView,
    /// Full cards for the viewer; a count only for the opponent (§10.8).
    pub hand: HandView,
    pub library_count: i32,
    /// R310: the viewer's own library, as a list without order. Present on the viewer's own side only;
    /// the opponent's library is `libraryCount` and nothing else (§9.1, §10.8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub own_library: Option<LibraryView>,
    pub graveyard: Vec<CardView>,
    pub exile: Vec<CardView>,
    /// Cards mid-resolution: a Spell between its play and its graveyard (§10.5 step 4). Public for
    /// both sides — playing a card is public — and R98 makes one still itself while it sits here, so
    /// a Spell that opened a prompt can be shown on the board instead of vanishing until it lands.
    pub resolving: Vec<CardView>,
    pub units: Vec<Option<UnitView>>,
    pub backrow: Vec<Option<BackrowView>>,
    /// B5 E21, R446: the Unit standing on each backrow zone's carrier (Classic+ #33 Ivory Tower), by lane —
    /// a Unit on the field, public like any, that can neither attack nor be attacked. Absent when no
    /// carrier on this side holds one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub carried: Option<Vec<Option<UnitView>>>,
    /// R1143: this seat's hand size, once an effect has set one for the rest of the game (Meditative #79);
    /// public on both seats. Absent means `HAND_CAP`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand_cap: Option<i32>,
    /// R1141: on the opponent's seat, how many cards of their hand carry a mark (Meditative #76's
    /// pending steal) — never which: the viewer's own hand shows each mark on its card. Absent when
    /// none does, on the viewer's own seat and once the game is over (both hands are revealed, R434).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand_marked: Option<i32>,
    /// R1223: this seat's credit line as the client shows it. Absent with no line, no debt and no
    /// locked instalment, so a game without credit views as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub credit: Option<CreditView>,
    pub locks: RowFlags,
    /// R64: a zone held for a dying Reborn unit until it comes back. It takes no summon, exactly as a
    /// Locked zone takes none, so a client that reads only `locks` would draw it open. B3.1 rule 6: the
    /// backrow zone an animated "Animated on your turn" card will return to is held the same way.
    pub reserved: RowFlags,
    pub fatigue_count: i32,
    /// ME-JADE, R961: this seat's Jade Counter, shown to both seats beside its hero. Absent until it
    /// first rises, so a game without a Jade sends the view it always did (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub jade: Option<i32>,
    /// R1840005: this seat's Credits, shown to both seats. Absent until first gained (D14); named
    /// apart from the credit line's `credit` (R1223).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub market_credits: Option<i32>,
    /// R846 (Meditative #19.1): extra turns owed to this player. Absent while none is owed, so a
    /// game without one serializes as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub extra_turns: Option<i32>,
    /// R847 (Meditative #19.1): this player's once-a-game Temporal Rift flag. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub rift_extra_turn: Option<bool>,
    /// ME-SECRET, R860: the secrets this side holds, the choice only where the viewer may read it.
    /// Absent while none are held (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub secrets: Option<Vec<SecretView>>,
    /// R987, Meditative #40 Feng Shui: the side's Luck for best-of rolls, read through
    /// `query::luck_of` as the hero panel reads its armor through `hero_armor_of`. Absent at 0, so a
    /// game with no Feng Shui looks as it did (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub luck: Option<i32>,
    /// R1847000: this seat's starscape map, shown to both seats. Absent until the player first
    /// travels, so a game without one sends the view it always did (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub starscape: Option<StarscapeView>,
}

/// The `forYou: true` member of `PendingView`: the prompt this viewer must answer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PendingPromptView {
    /// Always `true` here (the discriminant).
    #[cfg_attr(feature = "ts", ts(type = "true"))]
    pub for_you: bool,
    pub choice_id: String,
    pub kind: PromptKind,
    pub options: Vec<PendingOption>,
    pub min: i32,
    pub max: i32,
    pub prompt: String,
    /// B5 E18: a `pick` prompt's budget — the most the picked options' `cost`s may add up to
    /// (Classic #44's "total cost of (5) or less") — or a `market` prompt's yuan left (R1000). Absent
    /// on every other prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub budget: Option<i32>,
    /// A `craft` prompt's mode (R1182000 to R1182002): `scratch` opens the block editor on an empty
    /// canvas with no presets, tooltips or hints; `potion` and `hugePotion` open it in the potion
    /// frame. Absent on every other prompt and on Meditative #17's free crafter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub craft_mode: Option<crate::wire::CraftMode>,
    /// R1563007: a `craft` prompt's offer, the blocks the editor shows and the answer may use. Absent
    /// on every other prompt and on Scratch Pad's, which codes from the whole palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub craft_offer: Option<crate::wire::CraftOffer>,
    /// R1184000: a sequence question's right answers so far (Meditative+ #20 KY's Competition), its
    /// chooser's to see; the prompt then offers Stop after its answers. Absent on every other prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub streak: Option<i32>,
}

/// The `forYou: false` member of `PendingView`: someone else is answering.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PendingElsewhereView {
    /// Always `false` here (the discriminant).
    #[cfg_attr(feature = "ts", ts(type = "false"))]
    pub for_you: bool,
    pub pending_for: PlayerId,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(untagged)]
pub enum PendingView {
    ForYou(PendingPromptView),
    Elsewhere(PendingElsewhereView),
}

impl<'de> Deserialize<'de> for PendingView {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value.get("forYou").and_then(serde_json::Value::as_bool) {
            Some(true) => serde_json::from_value(value)
                .map(PendingView::ForYou)
                .map_err(D::Error::custom),
            Some(false) => serde_json::from_value(value)
                .map(PendingView::Elsewhere)
                .map_err(D::Error::custom),
            None => Err(D::Error::custom("a pending view needs a boolean `forYou`")),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PendingOption {
    /// The selection to send back in an `answer` action.
    pub key: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub def_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub player: Option<PlayerId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub row: Option<Row>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lane: Option<i32>,
    /// B5 E18: what this option counts against the prompt's `budget`: a `pick` option's cost, a
    /// `market` lot's price, a barter's yuan written negative (R1000, R1001).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub cost: Option<i32>,
    /// A face the option shows, when the card it names is Radiant (a Discover of Radiant cards).
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub radiant: Option<bool>,
    /// ME-CRAFT (Meditative #17, R880): the recipe a `craft` prompt's option offers. Only the
    /// prompt's holder is ever sent it (§10.8). Optional and skipped when absent (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub recipe: Option<CraftRecipe>,
    /// ME-CN, R1301: the card is shown in Chinese (`CardInstance.chinese`). Only on a card the viewer
    /// may read, so never on the sentinel, a face-down card someone else controls or an opponent's hand.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub chinese: Option<bool>,
    /// MD-B6, R943: the card was minted after the decks were built (`CardInstance.created`). Only on
    /// a card the viewer may read, as `chinese` is. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub created: Option<bool>,
    /// R1183005 (Meditative+ #25 KY's Intuition): the option is a card's code block, its `label`,
    /// and names no card (`defId` is absent), so a client draws the code as code. Only ever
    /// `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub code: Option<bool>,
}

/// R265, R266: the concurrent mulligan as one seat may see it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct MulliganView {
    pub you_ready: bool,
    pub opponent_ready: bool,
    /// The ids the viewer kept, once it has answered (R266).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kept: Option<Vec<String>>,
}

/// `PlayerView.drawOffer`: the standing draw offer and who made it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DrawOfferView {
    pub by: PlayerId,
}

/// R1838000, §10.8: the viewer's own Trades — what one costs and how many they have left this turn.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TradesView {
    /// `TRADE_COST`, in mana.
    pub cost: i32,
    /// The Trades a turn less those made this turn (`trade::trades_left`).
    pub left: i32,
}

/// R1853000–R1853003, R2250002, §10.8: the open Marketplace as the viewer reads it
/// (`marketplace::marketplace_view`): the lots offered to them and the sale prices of their own hand
/// sales (R1861000). Which buys and sales are offered stays `legal_actions`' answer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceView {
    /// The viewer's own offers (R2250002): the three rolled at the start of their turn, then the lots
    /// an effect added for them (R1853005).
    pub offers: Vec<MarketOfferView>,
    pub sales: Vec<MarketSaleView>,
}

/// R1853001–R1853003, R1853005, R2250002: one lot offered to the viewer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct MarketOfferView {
    /// Its index in the offering, which `marketBuy` names.
    pub offer: i32,
    pub def_id: String,
    /// A Radiant lot. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub radiant: Option<bool>,
    /// This turn's price, in Credits.
    pub price: i32,
    /// The copies the viewer may still buy (`marketplace::market_left`): 1 or 0 (R2250002).
    pub left: i32,
    /// A lot an effect added for the rest of the game, the viewer's alone (R1853005). Only ever
    /// `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub fixed: Option<bool>,
    /// Why the viewer may not buy it now (`marketplace::why_market_buy_refused`), only while they may
    /// act (their turn, the main phase, no prompt open) and only on a lot `legal_actions` lists no
    /// `marketBuy` of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<String>,
}

/// R1853002, R1853003, R1861000: a card in the viewer's hand that sells from there, and what it sells
/// for this turn.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct MarketSaleView {
    pub instance_id: String,
    pub def_id: String,
    /// A Radiant card, sold at its Radiant face's quote. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub radiant: Option<bool>,
    pub credits: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub viewer: PlayerId,
    pub turn: i32,
    pub active: PlayerId,
    pub phase: Phase,
    pub you: SideView,
    pub opponent: SideView,
    pub pending: Option<PendingView>,
    /// The last N events, for animation (§10.10). Redacted, not truncated: an event that names a card
    /// this viewer may not read keeps its type and its animation fields and carries the sentinel
    /// `"hidden"` in place of that card's `instanceId` and `defId` (R97).
    pub events: Vec<GameEvent>,
    pub result: Option<GameResult>,
    /// Milliseconds left on the turn clock, when the server is running one (R79).
    pub clock_ms: Option<i32>,
    /// §2.1 step 3, R265, R266: while both mulligans are open, whether each seat has answered, and the
    /// ids the viewer itself kept once it has. Never the opponent's choice or cards (§9.1): that the
    /// opponent is ready is all it shows. Absent outside that window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mulligan: Option<MulliganView>,
    /// §2.5, R36, R269: the draw offer standing right now — made by the active player this turn and
    /// not yet answered — on both seats, since the offer was public (`drawOffered`). Absent when none;
    /// it disappears when the offer is answered or lapses at the end of the offerer's turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub draw_offer: Option<DrawOfferView>,
    /// §2.5, R36: while a declined offer still keeps the viewer from offering a draw, how many of
    /// their own turns it covers — the one under way when it is theirs, then the coming ones; the
    /// turn of the decline itself is not counted. Only ever the viewer's own block, never the
    /// opponent's (CLAUDE.md rule 7). Absent when nothing blocks them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub draw_offer_blocked_turns: Option<i32>,
    /// R345: `false` when the viewer has turned R82's automatic turn end off for themselves. Absent
    /// means on, the rule's default. Only ever the viewer's own preference, never the opponent's.
    /// Only ever `Some(false)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "false"))]
    pub auto_end_turn: Option<bool>,
    /// R243: the definitions of the match-made cards this view names — a Fuse's (R77), a crafted
    /// card's (R102, R179) — by id. They exist only in the match, so no catalog a client holds has
    /// them, and a card the view shows could not otherwise be read. Only a card the viewer may read
    /// brings its definition: a hidden one's id is already the sentinel (R97). Absent when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub defs: Option<IndexMap<String, CardDef>>,
    /// R1200 (ME-RANDOMTARGETS): a Mayor acts on the field, so both players' declared targets,
    /// `target` prompts and attack targets are drawn at random and the client asks for none.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub random_targets: Option<bool>,
    /// MD-D29, R1127: an acting card of the opponent hears emotes, so the client's emote layer sends
    /// them as `Emote` actions. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub emotes_heard: Option<bool>,
    /// R1838000: the viewer's own Trades, while their hand holds a Tradable card, so the Trade control
    /// can say its price and how many are left; which Trades are offered stays `legal_actions`' answer.
    /// Only ever the viewer's own (CLAUDE.md rule 7). Absent while their hand holds no Tradable card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub trades: Option<TradesView>,
    /// R1853000, R1853003: the Marketplace as the viewer reads it, once it is open: the offering and
    /// its prices, which are public, and the viewer's own stock and sales (CLAUDE.md rule 7). Absent
    /// while it is closed, so a game without one sends the view it did before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub marketplace: Option<MarketplaceView>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_discriminated_unions_read_their_boolean() {
        let down: BackrowView = serde_json::from_str(r#"{"faceDown":true,"cost":2}"#).unwrap();
        assert!(matches!(
            down,
            BackrowView::FaceDown(FaceDownBackrowView { cost: Some(2), .. })
        ));
        assert_eq!(
            serde_json::to_string(&down).unwrap(),
            r#"{"faceDown":true,"cost":2}"#
        );
        let elsewhere: PendingView = serde_json::from_str(r#"{"forYou":false,"pendingFor":"p2"}"#).unwrap();
        assert!(matches!(
            elsewhere,
            PendingView::Elsewhere(PendingElsewhereView {
                pending_for: PlayerId::P2,
                ..
            })
        ));
        assert!(serde_json::from_str::<PendingView>(r#"{"forYou":true,"pendingFor":"p2"}"#).is_err());
        let hand: HandView = serde_json::from_str(r#"{"count":4}"#).unwrap();
        assert_eq!(hand, HandView::Count { count: 4 });
    }
}
