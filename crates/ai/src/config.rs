//! Every number the AI states (CLAUDE.md rule 9, in the package's own config; SURFACE §9). The weights
//! are tuning defaults: tests pin rankings and puzzle outcomes, never these values. Each constant
//! object is a `const` of a struct named after it (SURFACE §4.2).

use jackioh_engine::config::{CN_TECH_MIN_GAMES, MARKETPLACE_CREDITS_PER_MANA};
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

/// R1563005 (Meditative+ #20 KY's Competition): the chance, in [0, 1], that the AI answers a question
/// of a sequence right, on the base face's three-digit sums and on the Radiant face's one- and two-digit
/// ones; a miss is one of the wrong answers. Drawn from the AI's own stream, never the match's.
pub const AI_KY_COMPETITION_ACCURACY: f64 = 0.85;
pub const AI_KY_COMPETITION_RADIANT_ACCURACY: f64 = 0.93;

/// R747000: the skill knob's range (`skill::clamp_skill`): 0.0 plays the skill miss
/// (`skill::skill_miss_action`, R2214000) at every decision, 1.0 follows the AI at every decision.
pub const AI_SKILL_MIN: f64 = 0.0;
pub const AI_SKILL_MAX: f64 = 1.0;

/// R2214000: the softmax temperature, in `evaluate`'s points (GREEDY_EVAL), over a skill miss's one-ply
/// scores (`skill::skill_miss_action`): a move 4 points below the best is drawn e⁻¹ times as often. Of
/// 3, 4, 6, 8 and 12 (#2214's plan, 840 turn starts), 4 is the highest that ends a turn with a card it
/// can play under half as often as the random policy (0.038 to 0.096).
pub const AI_SKILL_MISS_TEMPERATURE: f64 = 4.0;

/// The practice AI's budget (SPEC §9.9): the same at every difficulty (R180). R2575000 (#2575): 1200
/// and 300 lethal nodes; a fat state still scales from at most `AI_SEARCH.fat_budget_nodes`, and the
/// server's middle-strength ladder bots and Duos seats keep 600 and 150 (`BOT_MID_STRENGTH_BUDGET`,
/// `DUOS_AI_BUDGET` in `crates/server/src/config.rs`).
pub const AI_BUDGET: SearchBudget = SearchBudget {
    nodes: 1200,
    lethal_nodes: 300,
    determinizations: 3,
    beam_width: 4,
    root_branching: 20,
    branching: 6,
    max_depth: 8,
    finalists: 3,
};

/// The quality gates' budget, and the shadow-ban sweep's: the browser's own, so that the gates
/// (docs/polish/3-ai.md B28–B31) and the sweep (R186) measure the AI that ships.
pub const AI_GATE_BUDGET: SearchBudget = AI_BUDGET;

/// R2102000: the stream the practice AI's lethal hint (`lethal::lethal_hint`, SPEC §9.9) draws
/// its worlds from, its own: never the match's, nor the AI's, so asking it moves no other stream.
pub const LETHAL_HINT_SEED: &str = "ai:lethal-hint";

/// R2102000: the hint's worlds and its solver's nodes, the AI's own, so that it proves a lethal as the
/// AI's solver does.
pub const LETHAL_HINT_DETERMINIZATIONS: usize = AI_BUDGET.determinizations;
pub const LETHAL_HINT_NODES: usize = AI_BUDGET.lethal_nodes;

/// R2660000: the stream the ladder bots' concession check (`unstoppable::unstoppable_lethal`) draws
/// its worlds from, its own: never the match's, nor the AI's, so asking it moves no other stream.
pub const UNSTOPPABLE_LETHAL_SEED: &str = "ai:unstoppable-lethal";

/// R2660000: the worlds the check proves the lethal on, the AI's own; it holds only on all of them.
pub const UNSTOPPABLE_LETHAL_DETERMINIZATIONS: usize = AI_BUDGET.determinizations;
const _: () = assert!(
    UNSTOPPABLE_LETHAL_DETERMINIZATIONS > 0,
    "R2660000: no world proves nothing"
);

/// R2660000: the nodes one whole check may spend, the seat's every line and the opponent's every
/// reply on every world: twice a decision's, since it tries every line where the search tries a beam.
pub const UNSTOPPABLE_LETHAL_NODES: usize = 2 * AI_BUDGET.nodes;

/// R2660000: the nodes one proof of the opponent's lethal may spend, the AI's solver's own.
pub const UNSTOPPABLE_LETHAL_REPLY_NODES: usize = AI_BUDGET.lethal_nodes;

/// #936 B32: the games the Duos gate plays by default (`cargo jackioh gate --duos`): the Easy AI team
/// (`duos_decide` at AI_GATE_BUDGET) against each opponent team, §10.7's random policy and the greedy
/// baseline, `duos.rs`'s `duos_gate`.
pub const DUOS_GATE_GAMES_SMOKE: usize = 20;

/// #936 B32: the share of its games the Easy AI team must win against the random-policy team. Only
/// wins count; a draw is reported beside them.
pub const DUOS_GATE_WIN_FLOOR: f64 = 0.8;

/// #1760: the share of its games the Easy AI team must win against a team on the 1v1 gates' one-ply
/// greedy baseline (`duos_greedy_action`), B32's second matchup; only wins count. Set as §9.9 sets the
/// 1v1 gates' counts, the most wins of DUOS_GATE_GAMES_SMOKE a team as strong as measured (168 of 200
/// on fresh deals) reaches in 95% of runs: 14 of 20.
pub const DUOS_GATE_GREEDY_WIN_FLOOR: f64 = 0.7;

/// R1127000: the seed of the throwaway determinization `duos_passes` scores a teammate's
/// follow-ups on. Fixed (never the caller's stream), so the same state reproduces the pass order.
pub const DUOS_PASS_PROBE_SEED: &str = "ai:duos-pass";

/// R1127000: the most of the teammate's legal `Play` follow-ups one passed card is scored on, the
/// card's own plays first, then the rest in `legal_actions` order: bounds scoring at one hand of
/// cards times this many single `reduce` calls per `duos_passes` call.
pub const DUOS_PASS_MAX_FOLLOWUPS: usize = 8;

/// R1723000: what each crystal of a team's `pass_cost` charges a pass, in `evaluate`'s points: the
/// price of a crystal everywhere else in the evaluation (`AI_EVAL.unspent_mana`).
pub const DUOS_PASS_MARGIN_PER_COST: f64 = 0.8;

/// R1723000: the least a pass must gain past the card's worth in the seat's hand and its price, so a
/// card that would only move from one hand to the other (a tie, up to rounding) stays with the seat.
pub const DUOS_PASS_MIN_GAIN: f64 = 0.5;

/// `AI_SEARCH`'s shape.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiSearch {
    /// Plays identical but for `zone` keep the lowest lane of each placement, or its midlane lane when
    /// it has one (R2203001) (candidates.rs `placement_key`), at most this many (#1014; 3 let a Tribute
    /// play's sets outgrow the lethal allowance), everywhere but the search's root, which keeps every
    /// placement (#1635). In the beam, a line that differs from an earlier one only in where its units
    /// stand is a lane variant, at most this many of a position; it is closed beside the first (#1797).
    pub zone_variants: i32,
    /// #1759, R1759000, R2021000: the stand-downs (a switch to Defense Position of a unit an enemy unit
    /// that could attack it next turn would kill and survive) `expand` keeps past its `width`, beside
    /// endTurn: switches are the move order's last tier, so below the root a full board cut every one. A
    /// switch back to Attack is kept only where the width holds it.
    pub position_switches: i32,
    /// Opponent prompts auto-answered inside one simulated step before it counts as an error.
    pub max_auto_answers: i32,
    /// Answers tried for each auto-answered prompt before the answerer's best is committed
    /// (`simulate`'s, the opponent's reply's and `own_prompt_score`'s auto-answers, #891): each one
    /// node. A prompt with more answers tries this many drawn from all of them (#1382,
    /// `auto_answer_seed`).
    pub max_auto_answer_options: i32,
    /// #1382: the stream the answers an auto-answered prompt tries are drawn on, beside the world's
    /// own seed and the prompt's id, when it has more than `max_auto_answer_options`.
    pub auto_answer_seed: &'static str,
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
    /// A decision's width at or under which the searched part of its budget is spent in full; above it
    /// the budget shrinks in inverse proportion, so the slow states of a long hand and a full board
    /// (R186's sweep flags a decision over 2 s) cost about an ordinary one. Cards held or in play count
    /// toward the width, since a prompt's few answers hide the fat state they answer into (decide.rs
    /// `fat_width`).
    pub fat_reference: i32,
    /// The least the scaled budget ever holds (the lethal walk still gets its depth-first pass and
    /// the beam a few expansions at the widest states).
    pub fat_min_nodes: i32,
    /// The least of the scaled budget the lethal solver's allowance shrinks to.
    pub fat_min_lethal: i32,
    /// R2575000: the most nodes a fat state's allowance is scaled from: the practice budget before
    /// issue #2575 doubled it, so a state whose every step costs the most spends what it did.
    pub fat_budget_nodes: i32,
    /// The cards a simulated Play of which `simulate` charges `expensive_def_nodes` for (its
    /// `surcharge_nodes`): resolving one ranks every shipped card for the state (the §10.7 scorer,
    /// R29's Zephyrs faces), about a hundred ordinary reduces. A fused card holding one is charged the
    /// same (`stands_for_any`, #1240), and one a step casts without playing it once the step has run
    /// (`announced_surcharge_nodes`, #1347).
    pub expensive_def_ids: &'static [&'static str],
    /// Nodes a Play of an `expensive_def_ids` card pays beyond the one every `simulate` takes.
    pub expensive_def_nodes: i32,
    /// The cards an armed backrow copy of which makes a simulated Attack cost `ai_turn_trap_nodes`
    /// (§8.5 core-096: a whole random AI turn inside one reduce), asked only of an attack that could be
    /// the lethal one the trap answers, so an ordinary poke is not priced like losing the turn. A fused
    /// card holding one is charged the same (`stands_for_any`, #1240).
    pub ai_turn_trap_ids: &'static [&'static str],
    /// Nodes an Attack into an armed `ai_turn_trap_ids` card pays beyond the one every `simulate`
    /// takes — the playout's rough share of the nodes it burns.
    pub ai_turn_trap_nodes: i32,
    /// #1240: nodes a Play pays for each card in its player's hand while a card of the other side
    /// judges that player's plays (Pareto Optimality, `subsystems::watching`): the judge runs §10.7's
    /// scorer once per playable card, about 2.3 ordinary steps per card, rounded up.
    pub judged_card_nodes: i32,
}

pub const AI_SEARCH: AiSearch = AiSearch {
    zone_variants: 2,
    position_switches: 2,
    max_auto_answers: 8,
    max_auto_answer_options: 4,
    auto_answer_seed: "ai:answers",
    lethal_max_depth: 10,
    lethal_quick_nodes: 40,
    lethal_width: 60,
    lines_per_action: 2,
    probe_seed: "ai:probe",
    tribute_sets: 120,
    fat_reference: 30,
    fat_min_nodes: 150,
    fat_min_lethal: 60,
    fat_budget_nodes: 600,
    expensive_def_ids: &["core-097", "classicplus-027"],
    expensive_def_nodes: 100,
    ai_turn_trap_ids: &["core-096"],
    ai_turn_trap_nodes: 40,
    judged_card_nodes: 3,
};

/// `AI_EVAL.keyword`: the worth of each keyword in `unitView.keywords` (spent Divine Shield and Reborn
/// are already gone). A keyword kind with no entry here (Armor, Deft, …) is worth nothing of its
/// own; `of` answers `None` for it.
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
    /// R636: a second attack each turn (`attacks_per_turn`). `face_threat` already counts the second
    /// swing at the next turn's face; this is what the body is worth for it beyond that, in trades.
    #[serde(rename = "Windfury")]
    pub windfury: f64,
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
            KeywordKind::Windfury => Some(self.windfury),
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
    /// point of Armor off whatever the Taunt's source.
    pub defense_own_taunt_kept: bool,
    /// Per point of heroArmorOf.
    pub hero_armor: f64,
    /// Per point of unitView.attack.
    pub attack: f64,
    /// The share of a Defense-Position unit's attack that counts (§4.1: it cannot attack until a
    /// switch spends its exertion, but it still strikes back in full).
    pub defense_attack_share: f64,
    /// Whether a Deft unit in Defense Position (R49: it can switch to Attack and attack in the same
    /// turn) keeps its whole attack in `unit_worth` and counts as an attacker in `face_threat`. Off, it
    /// is read as any Defense-Position unit is.
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
    /// B3.1: a readable backrow card with Animated and stats (R2280000) is a Unit in waiting, worth this
    /// share of its unit face's `unitWorth` on top of its backrow value. Animated, it stands in a unit
    /// zone and is a unit.
    pub animated_share: f64,
    /// Own hand card: handCard + handPerCost × min(queryCost(def), handCostCap).
    pub hand_card: f64,
    pub hand_per_cost: f64,
    pub hand_cost_cap: f64,
    /// Added per own Radiant hand card.
    pub radiant_in_hand: f64,
    /// R423: per effect a Radiant Call to Chaos rolls past its base face's one, the share of the card's
    /// per-cost hand price (`hand_per_cost` × min(cost, `hand_cost_cap`)) it adds. Its Radiant text is
    /// then priced at what it does, two more casts of the base face, in place of `radiant_in_hand`'s flat
    /// price when that is more (#1886).
    pub radiant_chaos_effect: f64,
    /// #1143: when both faces of an own hand card are Units, the share that counts of what the face
    /// it will enter with (its Radiant face when it is Radiant, its buffs and tuning) is worth over its
    /// printed base face, by the stat and keyword weights above and none of the field's layers. On top
    /// of `radiant_in_hand`, which prices the Radiant text.
    pub radiant_face_share: f64,
    /// Per Plague Counter on a unit (§10.1, R454: Fed Fauci's stored mana engine, a Corpse
    /// Plantation's unspent price), on top of the body's worth and outside its Brittle share.
    pub plague_counter: f64,
    /// Per grade step a readable backrow card's counter has climbed past E (§10.1, R27/R62:
    /// Combo-Index's cascade), plus one for a rise this turn's plays already arm.
    pub grade_counter: f64,
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
    /// R846: the share of `extra_turn` an own hand card of `AI_EXTRA_TURN_CARDS` counts on top of its
    /// hand value: a turn it owes once cast, which the beam sees only on the turn it can pay for it
    /// (#1886).
    pub extra_turn_in_hand: f64,
    /// R1855003: per miss a side's Scanning Probes have banked (`PlayerState.anomaly_misses`), each
    /// adding its steps to every later scan's chances. Own side only: a miss is silent (#1886).
    pub anomaly_miss: f64,
    /// R2250001, R1853002: per Credit the side holds (`PlayerState.market_credits`), priced at what it
    /// buys: the Marketplace asks `MARKETPLACE_CREDITS_PER_MANA` a mana, which the hand prices at
    /// `hand_per_cost`. Own side only (#2389).
    pub market_credit: f64,
    /// R458: per rest-of-game start-of-turn effect a side holds (`ModifierKind::StartOfTurnEffect`:
    /// Classic+ #52's card each turn, Call to Chaos's eternal cast, R1241), an annuity paid at each of
    /// its holder's turn starts. Every shipped one pays its holder.
    pub start_of_turn_effect: f64,
    /// R848, R849: what a side's closest held alternative win (`ModifierKind::AltWin`) is worth at its
    /// threshold. It counts the square of the progress toward it (`alt_win_progress`), so the last
    /// steps weigh most: for its holder a plan, for the holder's enemy a danger.
    pub alt_win: f64,
    /// B5 E27 (Classic #37 Last Hurrah): the share of its worth a hand card loses while a delayed
    /// discard of its owner's hand waits (`DELAYED_DISCARD_HAND_HOOK`), unless it is played first. A
    /// Temporary card (R637) keeps the smaller of the two shares, not their product.
    pub doomed_hand_discount: f64,
    /// §3.2, R13: the share of its `unit_worth` a card dormant under one of the seat's own unit piles
    /// counts. It resumes when the cards above it leave.
    pub dormant_share: f64,
    /// §3.2, R447: per card dormant under a pile the seat counts but does not read card by card: the
    /// enemy's unit piles, and every backrow pile, its own and the enemy's.
    pub dormant_card: f64,
    /// R2380000: the share of its awake `unit_worth` a Dormant card on the field (§6.1, R1839000)
    /// keeps per turn left on its countdown, so one that wakes in n of its controller's turns counts
    /// this to the power n of what it will be.
    pub slumber_turn_share: f64,
    /// R2380000: the share of its awake `unit_worth` a Dormant card with no countdown counts, one
    /// that waits for an effect to wake it (the Dormant Starship's launch).
    pub slumber_waiting_share: f64,
    /// R2424000: the share of what a unit's Death leaves its controller (`evaluate::death_price`) that
    /// `unit_worth` adds; `kill_worth` takes the whole price off a kill, as Reborn's body. Below 1, so
    /// killing a unit whose Death pays its owner is worth less than its body.
    pub death: f64,
    /// R2477000, R2565000: the share of what one firing of a card's start-of-turn and end-of-turn hooks
    /// gives its controller (`evaluate::turn_hook_price`) that `material` adds for each backrow card the
    /// seat can read, either side's, and `unit_worth` adds for each unit on the field.
    pub turn_hook: f64,
    /// R2564000: the share of what one use of a unit's Activate gives its controller
    /// (`evaluate::activation_price`) that `unit_worth` adds, inside the Brittle share, the seat's units
    /// and the enemy's alike. R2603000: `material` adds it, inside the card's Brittle share, for each
    /// backrow card the seat can read, on either side.
    pub activation: f64,
    /// R2688000: the share of what a unit's next attack gives its controller through its attack hooks
    /// (`evaluate::attack_hook_price`) that `unit_worth` adds, inside the Brittle share, while the unit
    /// may attack, the seat's units and the enemy's alike.
    pub attack_hook: f64,
    /// An unseen hand card is valued as if it cost this.
    pub opponent_hand_cost: f64,
    /// A readable backrow card: backrowBase + backrowPerCost × queryCost(def).
    pub backrow_base: f64,
    pub backrow_per_cost: f64,
    /// R2490000: a Unit standing in the backrow is priced once, as the Unit the unit walk priced
    /// — the backrow walk skips it. Off, it keeps the old backrow price on top.
    pub standing_unit_priced_once: bool,
    /// A backrow card the seat cannot read (on the enemy's side of the ledger).
    pub enemy_face_down: f64,
    /// R351: per crystal of the cost such a card shows both players (`shown_cost`), on top of
    /// `enemy_face_down`: a (6) back and a (0) back are not the same threat.
    pub enemy_face_down_per_cost: f64,
    /// Per library card up to libraryComfort.
    pub library_card: f64,
    pub library_comfort: f64,
    /// §2.4: taken off a side per fatigue draw it has taken (`fatigue_count`, public): the Nth draw
    /// from an empty library deals N, so each one taken adds a point to every fatigue hit to come.
    /// It reads the count alone, never the library, which `library_card` prices.
    pub fatigue_draw: f64,
    /// Per crystal left at the end of the seat's own turn (terminal only).
    pub unspent_mana: f64,
    /// R1224: per mana owed on the coming refreshes (Jlarna's credit line) — a debt is coming
    /// turns' mana spent early.
    pub owed_mana: f64,
    /// R844: per crystal a side will not get from the refreshes it still has to lose past this turn's,
    /// each at its last refresh's crystals (`mana.max`), the first with the rider it spends.
    pub lost_refresh: f64,
    /// §2.3: per crystal a side's next-turn rider moves its coming refresh by, signed, floored as the
    /// refresh floors it, and nothing while that refresh is lost (R844).
    pub next_turn_mana: f64,
    /// Per point of faceThreat(enemy) against the seat.
    pub threat_per_damage: f64,
    /// When faceThreat(enemy) >= the seat's life (`effective_life`, R964000).
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

/// `AI_EVAL.hand_per_cost`: an own hand card's worth per mana of its cost, which `market_credit` reads
/// too (#2389).
pub const AI_HAND_PER_COST: f64 = 0.3;

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
        pierce: 0.5,
        indestructible: 4.0,
        immutable: 0.3,
        stack: 0.0,
        lucky: 0.2,
        immune_to_spells: 1.0,
        // #841: untuned, conservative starts for the training lanes to move. R783000:
        // Ranged spares its attacks the strike back, as First Strike spares a kill's, but guards
        // nothing on defense, so half First Strike's worth.
        ranged: 0.5,
        // R783002: the reach Flying denies ground attackers is counted by `face_threat` and the reply
        // already, so on its own it is worth what Trample is.
        flying: 0.5,
        cloaked: 0.0,
        crater: 0.0,
        // R783012: no card prints it yet (§6.1), so nothing until one does and the lanes measure it.
        flashback: 0.0,
        // #1602: off. At 1.0 its screen read 201-199, edge 0.005 (-0.005 to 0.015), which keeps nothing
        // (training/improve.md, A screen).
        windfury: 0.0,
    },
    spell_damage: 0.5,
    brittle_discount: 0.5,
    animated_share: 0.5,
    hand_card: 1.0,
    hand_per_cost: AI_HAND_PER_COST,
    hand_cost_cap: 6.0,
    radiant_in_hand: 0.5,
    radiant_chaos_effect: 1.0,
    // #1143: an untuned start for the training lanes, as `animated_share` and `marked_share` are.
    radiant_face_share: 0.5,
    plague_counter: 0.5,
    grade_counter: 1.5,
    hand_cost_delta: 0.5,
    // #1086: untuned starts for the training lanes. A Jade step as a grade step, a mark as
    // `animated_share`, a Temporary card at Brittle 1's half, an extra turn about a turn's mana at
    // MAX_MANA (4 × unspent_mana) plus a draw.
    temporary_discount: 0.5,
    jade_step: 1.5,
    marked_share: 0.5,
    extra_turn: 5.0,
    // #1886: untuned starts for the training lanes. An extra-turn card in hand at half the turn it
    // owes once cast; a banked miss at about the 5 points of find chance it adds to each later scan
    // (ANOMALY_RADIANT_STEP + ANOMALY_PLAIN_STEP) times a find's worth, a 4-cost card in hand (2.2),
    // over a couple of scans.
    extra_turn_in_hand: 0.5,
    anomaly_miss: 0.2,
    market_credit: AI_HAND_PER_COST / MARKETPLACE_CREDITS_PER_MANA as f64,
    // #1722: untuned starts for the training lanes, as #1086's are. A start-of-turn effect as about
    // the two cards it adds over its holder's next two turns (2 × (hand_card + hand_per_cost ×
    // opponent_hand_cost)), an alternative win at its threshold as three lethal pressures, a doomed
    // hand at Temporary's half.
    start_of_turn_effect: 3.0,
    alt_win: 60.0,
    doomed_hand_discount: 0.5,
    // #1506: off. At 0.5 and 1.0 its screen read 200-200, edge 0.0 (-0.024 to 0.024), which keeps
    // nothing (training/improve.md, A screen).
    dormant_share: 0.0,
    dormant_card: 0.0,
    // R2380000: untuned starts for the training lanes, as #1886's are. A turn's wait at half, so
    // "Dormant for 2 turns" counts a quarter of the unit it wakes as; an effect's wake at half too.
    slumber_turn_share: 0.5,
    slumber_waiting_share: 0.5,
    // R2424000: an untuned start for the training lanes, as R2380000's shares are.
    death: 0.5,
    // R2477000: an untuned start for the training lanes, as R2424000's is. At 0.5 the smoke gate read
    // 20, 16 and 17 of 20; at 1.0, 19, 16 and 16, Hard against Easy on its floor of 16.
    turn_hook: 0.5,
    // R2564000: an untuned start for the training lanes, as R2477000's is. At 0.5 the smoke gate read
    // 20, 16 and 20 of 20; at 0, 19, 16 and 20.
    activation: 0.5,
    // R2688000: an untuned start for the training lanes, as R2564000's is. At 0.5 the smoke gate read
    // 20, 20 and 20 of 20; at 0, 20, 20 and 20.
    attack_hook: 0.5,
    opponent_hand_cost: 2.0,
    backrow_base: 1.5,
    backrow_per_cost: 0.8,
    standing_unit_priced_once: true,
    enemy_face_down: 2.0,
    // #1602: off. At 0.8 its screen read 201-199, edge 0.005 (-0.017 to 0.027), which keeps nothing
    // (training/improve.md, A screen).
    enemy_face_down_per_cost: 0.0,
    library_card: 0.1,
    library_comfort: 10.0,
    // #1602: off. At 1.0 its screen read 199-201, edge -0.005 (-0.022 to 0.012), which keeps nothing
    // (training/improve.md, A screen).
    fatigue_draw: 0.0,
    unspent_mana: 0.8,
    // R1224: a debt counts as unspent mana does.
    owed_mana: 0.8,
    // #1224: untuned starts: a crystal a coming refresh will or will not give counts as one left
    // unspent does.
    lost_refresh: 0.8,
    next_turn_mana: 0.8,
    threat_per_damage: 0.4,
    lethal_threat: 150.0,
    pressure_per_damage: 0.5,
    lethal_pressure: 20.0,
    answerable_threat: 0.3,
    lethal_on_board: 20.0,
    closing_from: 10.0,
    closing_weight: 3.0,
};

/// R2424000: the most nodes a decision's Death probes (`evaluate::death_prices`) spend: two per field
/// unit with a Death, three or four per own hand Unit with one.
pub const AI_DEATH_PROBE_NODES: usize = 24;

/// R2477000, R2565000: the most nodes a decision's turn-hook probes (`evaluate::turn_hook_prices`)
/// spend: one per turn hook of each card on either board the seat can read with one, and a spawn
/// (three nodes on a full unit row) more per own hand Unit or Field Spell with one. 24, as the Death
/// probes', for a board with a few engines on both sides and the hand's.
pub const AI_TURN_HOOK_PROBE_NODES: usize = 24;

/// R2564000, R2603000: the most nodes a decision's activation probes (`evaluate::activation_prices`)
/// spend: two turn ends and `AI_REPLY.visible_activations` uses for each of about four cards, units and
/// face-up backrow cards together.
pub const AI_ACTIVATION_PROBE_NODES: usize = 20;

/// R2688000: the most nodes a decision's attack-hook probes (`evaluate::attack_hook_prices`) spend: two
/// turn ends, and two attacks (the unit's and its hookless twin's) for each of about five units with an
/// attack hook, on either field.
pub const AI_ATTACK_HOOK_PROBE_NODES: usize = 24;

/// Which evaluation scores the search's finalists (#1010; `learned.rs`, `docs/learned-eval.md` D6).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EvalKind {
    /// `evaluate` with `AI_EVAL`: the hand-written evaluation, everywhere.
    Hand,
    /// The finalists after the opponent's reply are scored by the learned network's correction of
    /// the hand eval (`learned::finalist_score`), 1v1 only, and as much more of the search as
    /// `LEARNED_REACH` says.
    Learned,
}

/// The switch: a promotion that ships a network sets it with the weights in one change (#1012).
pub const AI_EVAL_KIND: EvalKind = EvalKind::Learned;

/// R1012001: the share of the network's correction a finalist's score takes, `hand + share·(points
/// − hand)` (`learned::finalist_score_with`). A decision compares lines a move or two apart, where
/// much of the network's correction is noise: on 1,600 paired games each against the hand eval (#466)
/// the whole correction read +0.010 edge and half +0.064; on TD targets (#1190), half +0.076.
pub const LEARNED_CORRECTION_SHARE: f64 = 0.5;
const _: () = assert!(LEARNED_CORRECTION_SHARE > 0.0 && LEARNED_CORRECTION_SHARE <= 1.0);

/// Which order the beam expands each line's moves in (#1171; `policy.rs`, R1171001).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MoveOrder {
    /// `candidate_actions`' own order, the heuristic tiers.
    Heuristic,
    /// That order re-sorted by the policy prior `POLICY_PRIOR` (`policy::prior_order`), 1v1 only.
    Learned,
}

/// The switch: a promotion that ships a table sets it with the table in one change.
pub const AI_MOVE_ORDER: MoveOrder = MoveOrder::Heuristic;

/// R1171000: move-order ranks at or past this share one key (`rank:24`); every rank the root expands
/// has its own.
pub const POLICY_RANK_CAP: usize = 24;
const _: () = assert!(POLICY_RANK_CAP >= AI_BUDGET.root_branching);

/// How far into the search the learned evaluation reaches with the switch on (`docs/learned-eval.md`
/// D6). Each step takes in the one before it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum LearnedReach {
    /// D6's option B: the finalists after the opponent's reply.
    Finalists,
    /// Every complete line too, where the beam closes it (`static_score`): the network also picks
    /// which lines become the finalists.
    Lines,
    /// Every open line too, where the beam ranks it at each depth (D6's option A in the beam).
    Beam,
}

/// R1012002: the learned evaluation's reach (`learned::line_score`, `learned::open_score`), measured
/// on paired games against the hand eval (#466). Ranking open lines prunes the beam on a scale its
/// widths were never tuned for. At the finalists alone the beam keeps the hand eval, so the network's
/// errors on positions unlike its games (the puzzles' bare boards) reach only the choice among lines
/// the hand eval found: at `Lines` a retrained network missed two of the AI's puzzles.
pub const LEARNED_REACH: LearnedReach = LearnedReach::Finalists;

/// The greedy baseline's own evaluation (baselines.rs): AI_EVAL exactly as it stood when the quality
/// gates were fixed on the seed series `gate:v2`, frozen here, so tuning AI_EVAL changes the AI and never
/// its yardstick. Every term and keyword AI_EVAL gained after that (R346, #1602, #1886, …) is 0 here, and
/// each flag that opts into a later reading (`defense_own_taunt_kept`, `deft_defense_attacks`,
/// `standing_unit_priced_once`) is off.
pub const GREEDY_EVAL: EvalWeights = EvalWeights {
    win: 1_000_000.0,
    drawn: 0.0,
    hero_health: 1.0,
    enemy_health: 1.0,
    position_grants: 0.0,
    defense_own_taunt_kept: false,
    hero_armor: 0.6,
    attack: 1.2,
    defense_attack_share: 0.5,
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
        pierce: 0.0,
        indestructible: 4.0,
        immutable: 0.3,
        stack: 0.0,
        lucky: 0.2,
        immune_to_spells: 0.0,
        ranged: 0.0,
        flying: 0.0,
        cloaked: 0.0,
        crater: 0.0,
        flashback: 0.0,
        windfury: 0.0,
    },
    spell_damage: 0.0,
    brittle_discount: 0.0,
    animated_share: 0.0,
    hand_card: 1.0,
    hand_per_cost: 0.3,
    hand_cost_cap: 6.0,
    radiant_in_hand: 0.5,
    radiant_chaos_effect: 0.0,
    radiant_face_share: 0.0,
    plague_counter: 0.0,
    grade_counter: 0.0,
    hand_cost_delta: 0.0,
    temporary_discount: 0.0,
    jade_step: 0.0,
    marked_share: 0.0,
    extra_turn: 0.0,
    extra_turn_in_hand: 0.0,
    anomaly_miss: 0.0,
    market_credit: 0.0,
    start_of_turn_effect: 0.0,
    alt_win: 0.0,
    doomed_hand_discount: 0.0,
    dormant_share: 0.0,
    dormant_card: 0.0,
    slumber_turn_share: 0.0,
    slumber_waiting_share: 0.0,
    death: 0.0,
    turn_hook: 0.0,
    activation: 0.0,
    attack_hook: 0.0,
    opponent_hand_cost: 2.0,
    backrow_base: 1.5,
    backrow_per_cost: 0.8,
    standing_unit_priced_once: false,
    enemy_face_down: 2.0,
    enemy_face_down_per_cost: 0.0,
    library_card: 0.1,
    library_comfort: 10.0,
    fatigue_draw: 0.0,
    unspent_mana: 0.8,
    owed_mana: 0.0,
    lost_refresh: 0.0,
    next_turn_mana: 0.0,
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
    /// #1634: the static trader's lethal counts every attack it has left. When `lethal::ready_gap`
    /// says the opponent's attacks still to come reach the seat's hero past its Taunts, every attack
    /// on that hero is worth `AI_EVAL.win`, so the face goes first; false scores one hit at a time
    /// (`reply_face_value`).
    pub open_lethal: bool,
    /// #1696: switches to Defense Position that one reply step tries once no attack is worth making,
    /// of the opponent's units in Attack Position whose switch `legal_actions` lists, in that order.
    pub defense_switches: i32,
    /// #1995: per turn a countdown (R1181000) still waits past its owner's next turn start, the share
    /// of a line's second score, the one with every countdown fired at the seat's next turn start, that
    /// `reply_score` blends in: `first + share^wait · (second − first)`.
    pub countdown_share: f64,
}

/// The opponent's reply that the best lines are scored after (reply.rs, SPEC §9.9).
pub const AI_REPLY: AiReply = AiReply {
    max_steps: 12,
    reserve_steps: 5,
    known_plays: 8,
    visible_activations: 4,
    face_per_damage: 1.0,
    chip_per_damage: 0.3,
    cleave_share: 1.0,
    // #1634: off. On, its screen read 203-197, edge 0.015 (-0.002 to 0.032), which keeps nothing
    // (training/improve.md, A screen).
    open_lethal: false,
    // #1696: on. Its screen (400 games of improve-dev against the parent) read 199-199 with deal 136
    // aborted at the action cap in both seatings (the parent aborts it against itself too), edge 0.0
    // (-0.057 to 0.057), every other game clean (training/improve.md, A screen); 0 turns it off.
    defense_switches: 4,
    // #1995: untuned; the training lanes may move it.
    countdown_share: 0.5,
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
    /// The switch: false makes the AI's mulligan `mulligan_keep_shaped` exactly.
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
/// thread previews (R1420): `build_ai_deck`'s default `gated_sets`, while a player's random deck
/// (`gated_sets: Some(vec![])`) is dealt from every set. A lean on a gated set leans on the newest
/// ungated set instead. Meditative was gated until #732 (MN08 of #496), Stinkyscape from #1884 until
/// #1885's sweep (R1885000).
pub const AI_DECK_GATED_SETS: &[SetName] = &[];

/// R846, #1886: the cards that owe their caster an extra turn when cast (`take_extra_turn`), priced in
/// hand by `AI_EVAL.extra_turn_in_hand`: M #19.1 Temporal Rift, S #4.3 Event Horizon (twice on its
/// Radiant face, counted once) and S #4.4 Last Word. A test casts each and checks the turn is owed.
pub const AI_EXTRA_TURN_CARDS: &[&str] = &["meditative-019-1", "stinkyscape-004-3", "stinkyscape-004-4"];

/// The most times the AI plays one card (one instance) in a turn; past it the card is no candidate
/// until the next turn. Only a card that comes back after each play reaches it: a (0) Spell castable
/// again from the graveyard (Classic #90 In Too Deep's reward) scored a little better each cast than
/// ending the turn, so the AI cast it until the match's action cap (#1885).
pub const AI_SAME_CARD_PLAYS_PER_TURN: usize = 3;

/// R1885000: the most Stinkyscape cards the shadow ban (R186) may hold, 20% of the set's 175 non-token
/// cards (`docs/stinkyscape-set.md` ST6, fullsend 2). `cargo jackioh sweep --write` refuses reports of
/// record that ban more; the Meditative and Meditative+ sets keep R732000's half.
pub const STINKYSCAPE_BAN_CEILING: usize = 35;

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

/// #750 (R1454): a ladder bot's random decks lean on the cards that win, by its `strength` (0 to 1;
/// `deck.rs`'s `win_rate_weight` holds the formula), so a deal with no strength is the one the seed
/// always dealt. 20 games is CN Tech's floor and the public stats' (`CARD_STATS_MIN_SAMPLE`); 3.0 sits
/// under `AI_DECK.themeBoost` (4.0), so a win rate never outweighs the theme or the curve.
pub const AI_WIN_RATE: AiWinRate = AiWinRate {
    min_games: CN_TECH_MIN_GAMES,
    max_boost: 3.0,
    floor: 0.25,
};

// R645: the AI's emote personas live in the web client (`apps/web/src/practice/personas.ts`): they
// are cosmetic only, and none of them reaches the engine, the action log or a game record.
