//! Every number the AI states (CLAUDE.md rule 9, in the package's own config). The weights are tuning
//! defaults: tests pin rankings and puzzle outcomes, never these values.
//!
//! Port of `packages/ai/src/config.ts` (SURFACE §9), minus the R645 emote personas (`EMOTE_TRIGGERS`,
//! `EMOTE_REPLY_KEYS`, `AI_EMOTE`, `AI_PERSONAS` and their types), which are presentation and live in
//! the web client (`apps/web/src/practice/personas.ts`, part 21). Each TS constant object is a `const`
//! of a struct named after it (SURFACE §4.2); `EvalWeights` keeps TS's own name for `AI_EVAL`'s shape.

use jackioh_engine::KeywordKind;
use serde::{Deserialize, Serialize};

use crate::types::SearchBudget;

/// The practice AI's budget (SPEC §9.9): the same at every difficulty (R180).
pub const AI_BUDGET: SearchBudget = SearchBudget {
    nodes: 600,
    lethal_nodes: 150,
    determinizations: 3,
    beam_width: 4,
    root_branching: 20,
    branching: 6,
    max_depth: 8,
    finalists: 3,
};

/// The quality gates' budget, and the shadow-ban sweep's: the browser's own, so that the gates
/// (docs/polish/3-ai.md B28–B31) and the sweep (R186) measure the AI that ships. A decision seldom
/// spends it all (the beam's shape, not the node count, bounds most turns), so a gate game costs
/// seconds, not minutes. Kept as its own name so that the gates can be given less, with the same
/// algorithm, if CI ever needs them faster.
pub const AI_GATE_BUDGET: SearchBudget = AI_BUDGET;

/// `AI_SEARCH`'s shape.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiSearch {
    /// Plays identical but for `zone` keep the leftmost and rightmost lane only.
    pub zone_variants: i32,
    /// Opponent prompts auto-answered inside one simulated step before it counts as an error.
    pub max_auto_answers: i32,
    /// Deepest line the lethal solver explores.
    pub lethal_max_depth: i32,
    /// Nodes of the lethal allowance its depth-first walk in move order gets before the best-first walk
    /// takes the rest (lethal.rs). At or above SearchBudget.lethalNodes the walk is depth-first alone.
    pub lethal_quick_nodes: i32,
    /// Moves the best-first lethal walk tries from each position it expands, in move order.
    pub lethal_width: i32,
    /// Lines per first action scored after the opponent's reply on determinization 0.
    pub lines_per_action: i32,
    /// Seed of the throwaway determinization that lists candidates for the forced check.
    pub probe_seed: &'static str,
}

pub const AI_SEARCH: AiSearch = AiSearch {
    zone_variants: 2,
    max_auto_answers: 8,
    lethal_max_depth: 10,
    lethal_quick_nodes: 40,
    lethal_width: 60,
    lines_per_action: 2,
    probe_seed: "ai:probe",
};

/// `AI_EVAL.keyword`: the worth of each keyword in `unitView.keywords` (spent Divine Shield and Reborn
/// are already gone). A keyword kind with no entry here (Armor, Windfury, …) is worth nothing of its
/// own; `of` answers `None` for it, as TS's `weights[keyword.kind]` was `undefined`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct KeywordWeights {
    #[serde(rename = "Taunt")]
    pub taunt: f64,
    #[serde(rename = "Divine Shield")]
    pub divine_shield: f64,
    #[serde(rename = "Lifesteal")]
    pub lifesteal: f64,
    #[serde(rename = "Poisonous")]
    pub poisonous: f64,
    #[serde(rename = "Reborn")]
    pub reborn: f64,
    #[serde(rename = "Charge")]
    pub charge: f64,
    #[serde(rename = "Rush")]
    pub rush: f64,
    #[serde(rename = "First Strike")]
    pub first_strike: f64,
    #[serde(rename = "Trample")]
    pub trample: f64,
    #[serde(rename = "Cleave")]
    pub cleave: f64,
    /// R346: its hits ignore Armor, worth about what Trample is.
    #[serde(rename = "Pierce")]
    pub pierce: f64,
    #[serde(rename = "Indestructible")]
    pub indestructible: f64,
    #[serde(rename = "Immutable")]
    pub immutable: f64,
    #[serde(rename = "Stack")]
    pub stack: f64,
    #[serde(rename = "Lucky")]
    pub lucky: f64,
    /// E35: no Spell targets it or touches it — protection from removal, and from its owner's buffs.
    #[serde(rename = "Immune to Spells")]
    pub immune_to_spells: f64,
}

impl KeywordWeights {
    /// `weights[kind]`: the entry for a keyword kind, `None` for a kind the table does not name.
    pub fn of(&self, kind: KeywordKind) -> Option<f64> {
        match kind {
            KeywordKind::Taunt => Some(self.taunt),
            KeywordKind::DivineShield => Some(self.divine_shield),
            KeywordKind::Lifesteal => Some(self.lifesteal),
            KeywordKind::Poisonous => Some(self.poisonous),
            KeywordKind::Reborn => Some(self.reborn),
            KeywordKind::Charge => Some(self.charge),
            KeywordKind::Rush => Some(self.rush),
            KeywordKind::FirstStrike => Some(self.first_strike),
            KeywordKind::Trample => Some(self.trample),
            KeywordKind::Cleave => Some(self.cleave),
            KeywordKind::Pierce => Some(self.pierce),
            KeywordKind::Indestructible => Some(self.indestructible),
            KeywordKind::Immutable => Some(self.immutable),
            KeywordKind::Stack => Some(self.stack),
            KeywordKind::Lucky => Some(self.lucky),
            KeywordKind::ImmuneToSpells => Some(self.immune_to_spells),
            _ => None,
        }
    }
}

/// Every weight `evaluate` reads, as numbers (AI_EVAL's shape without its literal types).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EvalWeights {
    /// won: +win − state.turn; lost: −win + state.turn.
    pub win: f64,
    /// result "draw".
    pub drawn: f64,
    /// heroValue(h) = h <= 0 ? −win : heroHealth × sqrt(h × HERO_HEALTH).
    pub hero_health: f64,
    /// Per point of the enemy hero's health, on top of its concave value: damage to the enemy hero
    /// shortens the race by the same amount whatever its health, and a draw at the turn cap is no win.
    pub enemy_health: f64,
    /// Share of a printed Taunt and a point of Armor that Defense Position's grants are worth (§4.1).
    pub position_grants: f64,
    /// Per point of heroArmorOf.
    pub hero_armor: f64,
    /// Per point of unitView.attack.
    pub attack: f64,
    /// The share of a Defense-Position unit's attack that counts (§4.1: it cannot attack until a
    /// switch spends its exertion, but it still strikes back in full).
    pub defense_attack_share: f64,
    /// Per point of unitView.health (current).
    pub health: f64,
    /// Per point of unitView.armor.
    pub armor_point: f64,
    /// Per keyword in unitView.keywords (spent Divine Shield and Reborn are already gone).
    pub keyword: KeywordWeights,
    /// E6: per point of Spell Damage a unit has (a Spell its controller casts deals that much more).
    pub spell_damage: f64,
    /// B3.3: a card with a Brittle count n is worth (1 − brittleDiscount / n) of itself — half at
    /// Brittle 1, the next tick away from crumbling — on the board, in the backrow and in the own hand.
    pub brittle_discount: f64,
    /// B3.1: a readable backrow card with Animated is a Unit in waiting, worth this share of its unit
    /// face's `unitWorth` on top of its backrow value. Animated, it stands in a unit zone and is a unit.
    pub animated_share: f64,
    /// Own hand card: handCard + handPerCost × min(queryCost(def), handCostCap).
    pub hand_card: f64,
    pub hand_per_cost: f64,
    pub hand_cost_cap: f64,
    /// Added per own Radiant hand card.
    pub radiant_in_hand: f64,
    /// B3.4, R65: taken off an own hand card per crystal its `costMod` (a Degrade's +1, an Upgrade's −1)
    /// moves its cost; a cheaper card gains it.
    pub hand_cost_delta: f64,
    /// An unseen hand card is valued as if it cost this.
    pub opponent_hand_cost: f64,
    /// A readable backrow card: backrowBase + backrowPerCost × queryCost(def).
    pub backrow_base: f64,
    pub backrow_per_cost: f64,
    /// A backrow card the seat cannot read (on the enemy's side of the ledger).
    pub enemy_face_down: f64,
    /// Per library card up to libraryComfort.
    pub library_card: f64,
    pub library_comfort: f64,
    /// Per crystal left at the end of the seat's own turn (terminal only).
    pub unspent_mana: f64,
    /// Per point of faceThreat(enemy) against the seat.
    pub threat_per_damage: f64,
    /// When faceThreat(enemy) >= the seat's hero health.
    pub lethal_threat: f64,
    pub pressure_per_damage: f64,
    pub lethal_pressure: f64,
    /// The share of the threat terms that counts when `seat` swings first (evaluate's "seat" frame).
    pub answerable_threat: f64,
    /// Lethal pressure when `seat` swings first: the enemy has no turn left to answer it.
    pub lethal_on_board: f64,
    /// From this turn on, damage on the enemy hero gains value, rising linearly to the turn cap.
    pub closing_from: f64,
    /// The extra value per point of enemy hero damage once the turn cap is reached (a draw scores 0).
    pub closing_weight: f64,
}

/// SURFACE §4.2's name for `AI_EVAL`'s struct; TS already named the shape `EvalWeights`.
pub type AiEval = EvalWeights;

pub const AI_EVAL: EvalWeights = EvalWeights {
    win: 1_000_000.0,
    drawn: 0.0,
    hero_health: 1.0,
    enemy_health: 1.0,
    position_grants: 0.0,
    hero_armor: 0.6,
    attack: 1.2,
    defense_attack_share: 0.5,
    health: 1.0,
    armor_point: 0.8,
    keyword: KeywordWeights {
        taunt: 1.5,
        divine_shield: 2.0,
        lifesteal: 1.0,
        poisonous: 2.0,
        reborn: 2.0,
        charge: 0.5,
        rush: 0.3,
        first_strike: 1.0,
        trample: 0.5,
        cleave: 1.0,
        // R346: its hits ignore Armor, worth about what Trample is.
        pierce: 0.5,
        indestructible: 4.0,
        immutable: 0.3,
        stack: 0.0,
        lucky: 0.2,
        // E35: no Spell targets it or touches it — protection from removal, and from its owner's buffs.
        immune_to_spells: 1.0,
    },
    spell_damage: 0.5,
    brittle_discount: 0.5,
    animated_share: 0.5,
    hand_card: 1.0,
    hand_per_cost: 0.3,
    hand_cost_cap: 6.0,
    radiant_in_hand: 0.5,
    hand_cost_delta: 0.5,
    opponent_hand_cost: 2.0,
    backrow_base: 1.5,
    backrow_per_cost: 0.8,
    enemy_face_down: 2.0,
    library_card: 0.1,
    library_comfort: 10.0,
    unspent_mana: 0.8,
    threat_per_damage: 0.4,
    lethal_threat: 150.0,
    pressure_per_damage: 0.5,
    lethal_pressure: 20.0,
    answerable_threat: 0.3,
    lethal_on_board: 20.0,
    closing_from: 10.0,
    closing_weight: 3.0,
};

/// The greedy baseline's own evaluation (baselines.rs): AI_EVAL exactly as it stood when the quality
/// gates were fixed on the seed series `gate:v2`, frozen here. A baseline the AI is measured against
/// must not move when the AI is tuned, so tuning AI_EVAL changes the AI and never its yardstick.
pub const GREEDY_EVAL: EvalWeights = EvalWeights {
    win: 1_000_000.0,
    drawn: 0.0,
    hero_health: 1.0,
    enemy_health: 1.0,
    position_grants: 0.0,
    hero_armor: 0.6,
    attack: 1.2,
    defense_attack_share: 0.5,
    health: 1.0,
    armor_point: 0.8,
    keyword: KeywordWeights {
        taunt: 1.5,
        divine_shield: 2.0,
        lifesteal: 1.0,
        poisonous: 2.0,
        reborn: 2.0,
        charge: 0.5,
        rush: 0.3,
        first_strike: 1.0,
        trample: 0.5,
        cleave: 1.0,
        // R346 came after the gates were fixed, so the frozen baseline gives Pierce nothing, as it did.
        pierce: 0.0,
        indestructible: 4.0,
        immutable: 0.3,
        stack: 0.0,
        lucky: 0.2,
        // E35, E6, B3.1, B3.3 and B3.4 (patch v0.2.0) came after the gates were fixed, so the frozen
        // baseline gives each of these terms nothing, as it did.
        immune_to_spells: 0.0,
    },
    spell_damage: 0.0,
    brittle_discount: 0.0,
    animated_share: 0.0,
    hand_card: 1.0,
    hand_per_cost: 0.3,
    hand_cost_cap: 6.0,
    radiant_in_hand: 0.5,
    hand_cost_delta: 0.0,
    opponent_hand_cost: 2.0,
    backrow_base: 1.5,
    backrow_per_cost: 0.8,
    enemy_face_down: 2.0,
    library_card: 0.1,
    library_comfort: 10.0,
    unspent_mana: 0.8,
    threat_per_damage: 0.4,
    lethal_threat: 150.0,
    pressure_per_damage: 0.5,
    lethal_pressure: 20.0,
    answerable_threat: 0.3,
    lethal_on_board: 20.0,
    closing_from: 10.0,
    closing_weight: 3.0,
};

/// The opponent's reply that the best lines are scored after (reply.rs, SPEC §9.9). `AI_REPLY`'s shape.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiReply {
    /// Engine steps one reply may take: plays, attacks, prompt answers and the closing endTurn.
    pub max_steps: i32,
    /// Nodes `decide` reserves per reply when it splits the budget (a typical reply, not the most).
    pub reserve_steps: i32,
    /// Plays of cards the line put in the opponent's hand that one reply step tries, in move order.
    pub known_plays: i32,
    /// The opponent's value per point of damage its attack would deal the seat's hero.
    pub face_per_damage: f64,
    /// Per point an attack deals a unit it does not kill.
    pub chip_per_damage: f64,
}

/// The opponent's reply that the best lines are scored after (reply.rs, SPEC §9.9).
pub const AI_REPLY: AiReply = AiReply {
    max_steps: 12,
    reserve_steps: 5,
    known_plays: 8,
    face_per_damage: 1.0,
    chip_per_damage: 0.3,
};

/// `AI_MULLIGAN`'s shape, and `GREEDY_MULLIGAN`'s (the same one field).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiMulligan {
    pub keep_max_cost: i32,
}

/// SURFACE §4.2's name for `GREEDY_MULLIGAN`'s struct: the same shape as `AI_MULLIGAN`'s.
pub type GreedyMulligan = AiMulligan;

pub const AI_MULLIGAN: AiMulligan = AiMulligan { keep_max_cost: 3 };

/// The greedy baseline's mulligan, frozen with GREEDY_EVAL for the same reason.
pub const GREEDY_MULLIGAN: AiMulligan = AiMulligan { keep_max_cost: 3 };

/// `AI_DETERMINIZE`'s shape.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiDeterminize {
    /// Catalog ids never sampled into a hidden slot: #98 keeps its rolled power in memory (R43). Ids,
    /// not indexes, since an index repeats across sets (B2.2, R387).
    pub exclude_def_ids: &'static [&'static str],
}

pub const AI_DETERMINIZE: AiDeterminize = AiDeterminize {
    exclude_def_ids: &["core-098"],
};

// ---------------------------------------------------------------------------
// R645: the AI's emote personas (`EMOTE_TRIGGERS`, `EMOTE_REPLY_KEYS`, `AI_EMOTE`, `AI_PERSONAS`,
// `PersonaName`, `PersonaSpec`) are cosmetic only: none of them reaches the engine, the action log or
// a game record, and `decide`/`search.rs` never read them. They moved with the personas to the web
// client, `apps/web/src/practice/personas.ts` (part 21).
// ---------------------------------------------------------------------------
