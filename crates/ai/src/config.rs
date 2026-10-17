//! Every number the AI states (CLAUDE.md rule 9, in the package's own config). The weights are tuning
//! defaults: tests pin rankings and puzzle outcomes, never these values.
//!
//! Port of `packages/ai/src/config.ts` (SURFACE §9), minus the R645 emote personas (`EMOTE_TRIGGERS`,
//! `EMOTE_REPLY_KEYS`, `AI_EMOTE`, `AI_PERSONAS` and their types), which are presentation and live in
//! the web client (`apps/web/src/practice/personas.ts`, part 21). Each TS constant object is a `const`
//! of a struct named after it (SURFACE §4.2); `EvalWeights` keeps TS's own name for `AI_EVAL`'s shape.

use jackioh_engine::config::CN_TECH_MIN_GAMES;
use jackioh_engine::{KeywordKind, SetName};
use serde::{Deserialize, Serialize};

use crate::types::{AiStyle, SearchBudget};

/// The style that always plays the best line it found and never passes over lethal (R1452):
/// `decide`'s own, which takes no draw for it. The ladder bots set their own (the server's
/// `BOT_STRENGTHS`).
pub const AI_STYLE_EXACT: AiStyle = AiStyle {
    temperature: 0.0,
    missed_lethal_chance: 0.0,
};

/// R747000: the skill knob's range (`skill::clamp_skill`): 0.0 plays a random legal action at every
/// decision, 1.0 follows the AI at every decision as it did before the knob.
pub const AI_SKILL_MIN: f64 = 0.0;
pub const AI_SKILL_MAX: f64 = 1.0;

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

/// #936 B32: the games the Duos gate plays by default (`cargo jackioh gate --duos`): the Easy AI team
/// (`duos_decide` at AI_GATE_BUDGET) against a team on §10.7's random policy, `duos.rs`'s `duos_gate`.
pub const DUOS_GATE_GAMES_SMOKE: usize = 20;

/// #936 B32: the share of its games the Easy AI team must win against the random-policy team. Only
/// wins count; a draw is reported beside them.
pub const DUOS_GATE_WIN_FLOOR: f64 = 0.8;

/// `AI_SEARCH`'s shape.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiSearch {
    /// Plays identical but for `zone` keep the lowest lane of each placement (candidates.rs
    /// `placement_key`: what stands beside the lane or the board's edge, and in the lane's four
    /// zones), at most this many (#1014). 3 let a Tribute play's sets outgrow the lethal allowance.
    pub zone_variants: i32,
    /// Opponent prompts auto-answered inside one simulated step before it counts as an error.
    pub max_auto_answers: i32,
    /// Answers tried for each auto-answered prompt, in `legal_actions` order, before the answerer's
    /// best is committed (`simulate`'s and the opponent's reply's auto-answers, #891): each one node.
    pub max_auto_answer_options: i32,
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
    /// R90, R1282: the most sets a shipped card lists, #55's 3 of ten units.
    pub tribute_sets: i32,
    /// A decision's width at or under which the searched part of its budget is spent in full. Above
    /// it the budget shrinks in inverse proportion, so the slow states a long hand and a full board
    /// make (R186's sweep flags a decision over 2 s) cost about what an ordinary one does. A card
    /// held or in play on either side counts toward the width beside the root's candidates, since a
    /// prompt's few answers hide the fat state they answer into (decide.rs `fat_width`).
    pub fat_reference: i32,
    /// The least the scaled budget ever holds (the lethal walk still gets its depth-first pass and
    /// the beam a few expansions at the widest states).
    pub fat_min_nodes: i32,
    /// The least of the scaled budget the lethal solver's allowance shrinks to.
    pub fat_min_lethal: i32,
    /// The cards a simulated Play of which `simulate` charges `expensive_def_nodes` for (its
    /// `surcharge_nodes`): resolving one ranks every shipped card for the state (the §10.7 scorer,
    /// R29's Zephyrs faces), about a hundred ordinary reduces, so a hand that holds one is budgeted
    /// for what the step really costs.
    pub expensive_def_ids: &'static [&'static str],
    /// Nodes a Play of an `expensive_def_ids` card pays beyond the one every `simulate` takes.
    pub expensive_def_nodes: i32,
    /// The cards an armed backrow copy of which makes a simulated Attack cost `ai_turn_trap_nodes`
    /// (§8.5 core-096's "an AI plays the rest of their turn": a whole random turn inside the one
    /// reduce). `surcharge_nodes` asks it only of an attack that could be the lethal one the trap
    /// answers, so an ordinary poke at an armed board is not priced like losing the turn.
    pub ai_turn_trap_ids: &'static [&'static str],
    /// Nodes an Attack into an armed `ai_turn_trap_ids` card pays beyond the one every `simulate`
    /// takes — the playout's rough share of the nodes it burns.
    pub ai_turn_trap_nodes: i32,
}

pub const AI_SEARCH: AiSearch = AiSearch {
    zone_variants: 2,
    max_auto_answers: 8,
    max_auto_answer_options: 4,
    lethal_max_depth: 10,
    lethal_quick_nodes: 40,
    lethal_width: 60,
    lines_per_action: 2,
    probe_seed: "ai:probe",
    tribute_sets: 120,
    fat_reference: 30,
    fat_min_nodes: 150,
    fat_min_lethal: 60,
    expensive_def_ids: &["core-097", "classicplus-027"],
    expensive_def_nodes: 100,
    ai_turn_trap_ids: &["core-096"],
    ai_turn_trap_nodes: 40,
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
    /// R783000: its attacks take no strike back from a defender that is not Ranged.
    #[serde(rename = "Ranged")]
    pub ranged: f64,
    /// R783002, R783003: in Attack Position only Flying and Ranged attackers may attack it.
    #[serde(rename = "Flying")]
    pub flying: f64,
    /// R783004: a hand card's face-down play; a unit on the field has no use for it.
    #[serde(rename = "Cloaked")]
    pub cloaked: f64,
    /// R783010: the Lock it makes as it arrives; a unit already on the field has made it.
    #[serde(rename = "Crater")]
    pub crater: f64,
    /// R783012: one more play from the graveyard after the unit dies.
    #[serde(rename = "Flashback")]
    pub flashback: f64,
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
            KeywordKind::Ranged => Some(self.ranged),
            KeywordKind::Flying => Some(self.flying),
            KeywordKind::Cloaked => Some(self.cloaked),
            KeywordKind::Crater => Some(self.crater),
            KeywordKind::Flashback => Some(self.flashback),
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
    /// heroValue(h) = h <= 0 ? −win : heroHealth × sqrt(h × start): h the hero's `effective_life`, start
    /// its `starting_life` (R964000).
    pub hero_health: f64,
    /// Per point of the enemy hero's health, on top of its concave value: damage to the enemy hero
    /// shortens the race by the same amount whatever its health, and a draw at the turn cap is no win.
    pub enemy_health: f64,
    /// Share of a printed Taunt and a point of Armor that Defense Position's grants are worth (§4.1).
    pub position_grants: f64,
    /// Whether Defense Position's discount (`position_grants`) leaves a unit's own Taunt alone and
    /// takes its Armor off only when Armor is on the body. Off, it takes a Taunt unit's Taunt and a
    /// point of Armor off whatever the Taunt's source, as the frozen baseline did.
    pub defense_own_taunt_kept: bool,
    /// Per point of heroArmorOf.
    pub hero_armor: f64,
    /// Per point of unitView.attack.
    pub attack: f64,
    /// The share of a Defense-Position unit's attack that counts (§4.1: it cannot attack until a
    /// switch spends its exertion, but it still strikes back in full). A Deft unit's switch spends no
    /// attack (R49), so with `deft_defense_attacks` it keeps its whole attack.
    pub defense_attack_share: f64,
    /// Whether a Deft unit in Defense Position (R49: it can switch to Attack and attack in the same
    /// turn) keeps its whole attack in `unit_worth` and counts as an attacker in `face_threat`. Off, it
    /// is read as any Defense-Position unit is, as the frozen baseline did.
    pub deft_defense_attacks: bool,
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
    /// Per Plague Counter on a unit (§10.1, R454: Fed Fauci's stored mana engine, a Corpse
    /// Plantation's unspent price), on top of the body's worth and outside its Brittle share.
    pub plague_counter: f64,
    /// Per grade step a readable backrow card's counter has climbed past E (§10.1, R27/R62:
    /// Combo-Index's cascade), plus one for a rise this turn's plays already arm.
    pub grade_counter: f64,
    /// R1246: per id a Journey Complete in the library still names in an exile pile — a card its
    /// cast-on-draw returns to hand Radiant, so the journey's exiles are a deferred gain rather than
    /// the loss the exiles alone read as.
    pub journey_card: f64,
    /// B3.4, R65: taken off an own hand card per crystal its `costMod` (a Degrade's +1, an Upgrade's −1)
    /// moves its cost; a cheaper card gains it.
    pub hand_cost_delta: f64,
    /// R637: the share of its worth an own Temporary hand card loses: cleanup discards it unless played.
    pub temporary_discount: f64,
    /// R961, R962: per step a Jade Counter has climbed toward the next threshold it has not crossed. A
    /// crossing's worth is the Jade Beauty it summons or ascends, which `unit_worth` prices.
    pub jade_step: f64,
    /// R437: the share of an enemy unit's `unit_worth` a side banks while its own delayed effect marks
    /// it. Every shipped mark is a pending steal or destroy its controller still has a turn to answer;
    /// a destroy's mark on an Indestructible unit, which it spares (R46), banks nothing.
    pub marked_share: f64,
    /// R846: per extra turn owed.
    pub extra_turn: f64,
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
    /// R1224: per mana owed on the coming refreshes (Jlarna's credit line) — a debt is coming
    /// turns' mana spent early. A lost refresh (R844) counts the refresh's whole `max_mana_for`
    /// as such a debt.
    pub owed_mana: f64,
    /// Per point of faceThreat(enemy) against the seat.
    pub threat_per_damage: f64,
    /// When faceThreat(enemy) >= the seat's life (`effective_life`, R964000).
    pub lethal_threat: f64,
    /// The share of a side's next fatigue hit that counts toward the damage it is about to take
    /// (evaluate's `fatigue_hit`, R3/R125): a draw on an empty library deals it, so the threat and
    /// pressure terms — which only ever saw attackers — read the long game's usual end. 1.0 counts
    /// the whole projected hit like a face hit's.
    pub fatigue: f64,
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

pub const AI_EVAL: EvalWeights = EvalWeights {
    win: 1_000_000.0,
    drawn: 0.0,
    hero_health: 1.0,
    enemy_health: 1.0,
    position_grants: 0.0,
    defense_own_taunt_kept: true,
    hero_armor: 0.6,
    attack: 1.2,
    defense_attack_share: 0.5,
    deft_defense_attacks: true,
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
        // Patch v0.3.7 (#841): untuned, conservative starts for the training lanes to move. R783000:
        // Ranged spares its attacks the strike back, as First Strike spares a kill's, but guards
        // nothing on defense, so half First Strike's worth.
        ranged: 0.5,
        // R783002: the reach Flying denies ground attackers is counted by `face_threat` and the reply
        // already, so on its own it is worth what Trample is.
        flying: 0.5,
        // R783004, R783010: a hand card's play and an arrival's Lock, nothing left on the field.
        cloaked: 0.0,
        crater: 0.0,
        // R783012: no card prints it yet (§6.1), so nothing until one does and the lanes measure it.
        flashback: 0.0,
    },
    spell_damage: 0.5,
    brittle_discount: 0.5,
    animated_share: 0.5,
    hand_card: 1.0,
    hand_per_cost: 0.3,
    hand_cost_cap: 6.0,
    radiant_in_hand: 0.5,
    plague_counter: 0.5,
    grade_counter: 1.5,
    journey_card: 2.6,
    hand_cost_delta: 0.5,
    // #1086: untuned starts for the training lanes. A Jade step as a grade step, a mark as
    // `animated_share`, a Temporary card at Brittle 1's half, an extra turn about a turn's mana at
    // MAX_MANA (4 × unspent_mana) plus a draw.
    temporary_discount: 0.5,
    jade_step: 1.5,
    marked_share: 0.5,
    extra_turn: 5.0,
    opponent_hand_cost: 2.0,
    backrow_base: 1.5,
    backrow_per_cost: 0.8,
    enemy_face_down: 2.0,
    library_card: 0.1,
    library_comfort: 10.0,
    unspent_mana: 0.8,
    // R1224: a debt counts the way unspent mana does — coming turns' mana, spent early.
    owed_mana: 0.8,
    threat_per_damage: 0.4,
    lethal_threat: 150.0,
    fatigue: 1.0,
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
    // The baseline is frozen: it discounted a Taunt unit's own Taunt in Defense Position, as it did.
    defense_own_taunt_kept: false,
    hero_armor: 0.6,
    attack: 1.2,
    defense_attack_share: 0.5,
    // The baseline is frozen: it read a Deft unit in Defense Position as one that cannot attack.
    deft_defense_attacks: false,
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
        // Patch v0.3.7's keywords came after the gates were fixed too: nothing, as it gave them.
        ranged: 0.0,
        flying: 0.0,
        cloaked: 0.0,
        crater: 0.0,
        flashback: 0.0,
    },
    spell_damage: 0.0,
    brittle_discount: 0.0,
    animated_share: 0.0,
    hand_card: 1.0,
    hand_per_cost: 0.3,
    hand_cost_cap: 6.0,
    radiant_in_hand: 0.5,
    // The counter terms came after the gates were fixed, so the frozen baseline gives each nothing,
    // as it did.
    plague_counter: 0.0,
    grade_counter: 0.0,
    journey_card: 0.0,
    hand_cost_delta: 0.0,
    // #1086's banked terms came after the gates were fixed too: nothing, as it gave them.
    temporary_discount: 0.0,
    jade_step: 0.0,
    marked_share: 0.0,
    extra_turn: 0.0,
    opponent_hand_cost: 2.0,
    backrow_base: 1.5,
    backrow_per_cost: 0.8,
    enemy_face_down: 2.0,
    library_card: 0.1,
    library_comfort: 10.0,
    unspent_mana: 0.8,
    // The baseline is frozen: a debt counted nothing when the gates were fixed, as everything after
    // them did.
    owed_mana: 0.0,
    threat_per_damage: 0.4,
    lethal_threat: 150.0,
    // The baseline is frozen: fatigue counted nothing when the gates were fixed, as everything after
    // them did.
    fatigue: 0.0,
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
    /// Activations of cards the seat can see on the field that one reply step tries, those aimed at the
    /// seat's hero first, then at a card it controls (reply.rs `aim_of`).
    pub visible_activations: i32,
    /// The opponent's value per point of damage its attack would deal the seat's hero.
    pub face_per_damage: f64,
    /// Per point an attack deals a unit it does not kill.
    pub chip_per_damage: f64,
    /// The share of a cleaved neighbour's payoff the reply counts (R63): a killed neighbour at
    /// this share of its `unit_worth`, a popped neighbour shield at this share of the Divine
    /// Shield weight. Board-dependent kills only — the Cleave body itself stays priced by
    /// `AI_EVAL.keyword.cleave`.
    pub cleave_share: f64,
}

/// The opponent's reply that the best lines are scored after (reply.rs, SPEC §9.9).
pub const AI_REPLY: AiReply = AiReply {
    // A reply that needs the whole dozen steps is a whole extra turn or a prompt chain — the fat
    // states the Meditative cards deal — and its tail nodes starve the lines and worlds behind
    // it. Nine still covers a normal turn twice over.
    max_steps: 9,
    reserve_steps: 5,
    known_plays: 8,
    visible_activations: 4,
    face_per_damage: 1.0,
    chip_per_damage: 0.3,
    cleave_share: 1.0,
};

/// `AI_MULLIGAN`'s shape, and `GREEDY_MULLIGAN`'s (the same one field).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiMulligan {
    pub keep_max_cost: i32,
}

pub const AI_MULLIGAN: AiMulligan = AiMulligan { keep_max_cost: 3 };

/// The greedy baseline's mulligan, frozen with GREEDY_EVAL for the same reason.
pub const GREEDY_MULLIGAN: AiMulligan = AiMulligan { keep_max_cost: 3 };

/// `AI_MULLIGAN_CURVE`'s shape (#993).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiMulliganCurve {
    /// The switch: false makes the AI's mulligan `mulligan_keep_shaped` exactly, as before #993.
    pub on: bool,
    /// A card costing at most this is a first-turn play. The mana bonus (R181) does not raise it: the
    /// cap is on the cheapest bodies, so a Hard seat's two-drops are not capped.
    pub first_turn_max_cost: i32,
    /// The most first-turn plays the mulligan keeps; the rest go back for a later turn's play.
    pub max_first_turn: usize,
    /// An early play costs at most this turn's crystals: the turn's number, plus the seat's mana
    /// bonus (R181) and its copies of The Coin (`OPENING_COINS`, R244).
    pub early_turn: i32,
    /// A hand without an early play keeps at most this many of its cheap cards, the cheapest, so the
    /// redraw digs for one.
    pub keep_without_early: usize,
}

/// #993: the curve the AI's mulligan keeps its cheap cards to (`mulligan_keep_curved`). The greedy
/// baseline's mulligan never reads it.
pub const AI_MULLIGAN_CURVE: AiMulliganCurve = AiMulliganCurve {
    on: true,
    first_turn_max_cost: 1,
    max_first_turn: 2,
    early_turn: 2,
    keep_without_early: 1,
};

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

/// R1390: the soft gate. The sets the AI's randomized decks are never dealt from, whatever ships or a
/// thread previews (R1420): `build_ai_deck`'s default `gated_sets`, so the AI's own decks (practice,
/// the quality gates, the sweep's AI seat, the arena, development runs) hold none of their cards,
/// while a player's random deck (`gated_sets: Some(vec![])`, beside `banned: Some(vec![])`) is dealt
/// from every set. A lean on a gated set leans on the newest set that ships and is not gated instead.
/// The list is empty since issue #732 measured the AI on the Meditative set (MN08 of #496), which it
/// held until then: the AI's decks are dealt from every set that ships. Gating a set that ships
/// before the AI is trained on it is listing it here.
pub const AI_DECK_GATED_SETS: &[SetName] = &[];

/// `AI_WIN_RATE`'s shape: how a deal's `strength` leans the draw on the compiled win-rate table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AiWinRate {
    /// The in-deck games a card's row needs before its rate moves its weight (R377's figures).
    pub min_games: i32,
    /// The most a card's weight is multiplied by at strength 1 and a 100% rate, above the 1.0 a 50%
    /// rate keeps.
    pub max_boost: f64,
    /// The least a card's weight is multiplied by, however low its rate.
    pub floor: f64,
}

/// #750 (R1454): a ladder bot's random decks lean on the cards that win. At `strength` s (0 to 1,
/// `AiDeckOptions::strength`) a card whose row has at least `min_games` in-deck games has its weight
/// multiplied by `1 + s × max_boost × (rate − 0.5) × 2`, held between `floor` and `1 + max_boost`;
/// a card with a thinner row or none, and every card at strength 0, keeps its weight, so a deal with
/// no strength is the one the seed always dealt. 20 games is CN Tech's floor (`CN_TECH_MIN_GAMES`)
/// and the public stats' (`CARD_STATS_MIN_SAMPLE`); 3.0 sits under `AI_DECK.themeBoost` (4.0), so a
/// win rate never outweighs the theme or the curve.
pub const AI_WIN_RATE: AiWinRate = AiWinRate {
    min_games: CN_TECH_MIN_GAMES,
    max_boost: 3.0,
    floor: 0.25,
};

// ---------------------------------------------------------------------------
// R645: the AI's emote personas (`EMOTE_TRIGGERS`, `EMOTE_REPLY_KEYS`, `AI_EMOTE`, `AI_PERSONAS`,
// `PersonaName`, `PersonaSpec`) are cosmetic only: none of them reaches the engine, the action log or
// a game record, and `decide`/`search.rs` never read them. They moved with the personas to the web
// client, `apps/web/src/practice/personas.ts` (part 21).
// ---------------------------------------------------------------------------
