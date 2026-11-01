//! The Zephyrs scorer (SPEC §10.7's scorer bullet, R29, §8 #97): rank every non-token definition of
//! every set that ships except Zephyrs itself (R1446) for the current state, so the Discover can offer
//! the top `DISCOVER_OPTIONS` (3).
//!
//! §10.7's priorities, in order: lethal available → max; can clear the enemy board → high; hero
//! below 10 and the card heals → high; otherwise stats per mana plus draw value. Each is read from the
//! printed data (§5) and from §10.7's dry run on a copy of the state, the only way to see what a card's
//! *text* does (#44's damage, #17's bounce, #53's heal).
//!
//! Pure in (state, viewer), as R29's Discover and §9.3's replay need: the dry run draws from a seed of
//! its own, and plays on a copy where every card §9.1 hides from the viewer is a stand-in that does
//! nothing and no record the viewer may not read is kept (R222, R1714000), so the offer never reveals
//! them. The weights live in `crate::config` (CLAUDE.md rule 9, SURFACE §6.4).

use std::borrow::Borrow;
use std::cmp::Ordering;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::catalog::index_rank;
use crate::config::{DISCOVER_OPTIONS, SCORER_DRY_RUN_PLAYS, SCORER_LOW_HEALTH, SCORER_WEIGHTS};
use crate::damage::DamageTarget;
use crate::layers::AuraTable;
use crate::play_choices::{PlayAction, graveyard_play_actions_for};
use crate::prompts::ANSWER_KEY;
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, find_instance};
use crate::wire::{
    CardCost, CardDef, CardFace, CardType, Counters, KeywordKind, Phase, PlayerId, Rarity, Row, Selection,
    SetName, Winner, Zone, ZoneChoice, has_keyword, opponent_of,
};

/// #97's §5 index in the Core set, read only to find #97's id, which the perfect pool leaves out
/// (R29, R387, R1446).
pub const ZEPHYRS_INDEX: &str = "97";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScorerOptions {
    /// #97 radiant: the picks are radiant, so the radiant face is the one scored (§5.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// Which §10.7 priority decided this score; the highest one that applied.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ScorePriority {
    Lethal,
    Clear,
    Heal,
    Value,
}

/// `Scored.parts`: each priority's contribution, for tuning and for the client's tooltip.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScoredParts {
    pub lethal: f64,
    pub clear: f64,
    pub kills: f64,
    pub heal: f64,
    pub stats: f64,
    pub draw: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Scored {
    pub def: CardDef,
    pub score: f64,
    pub priority: ScorePriority,
    pub parts: ScoredParts,
}

/// R1446: the perfect pool #97 and C+ #27 draw on, naming no set: every non-token card of every set
/// that ships (R380, R1420, §5.1, `SHIPPED_SETS`), less the asking card's own definition ids, a fused
/// card's ingredients included (R387, `catalog::self_def_ids`).
pub fn perfect_pool(state: Option<&GameState>, self_def_id: Option<&str>) -> Vec<&'static CardDef> {
    let args =
        crate::catalog::excluding_def_id(state, &crate::catalog::CatalogQueryArgs::default(), self_def_id);
    crate::catalog::query(&args)
}

/// R29, R1446: the candidate pool, the perfect pool less #97, left out by its id (R387).
pub fn candidate_defs() -> Vec<&'static CardDef> {
    let zephyrs = crate::catalog::def_by_index(SetName::Core, ZEPHYRS_INDEX).map(|def| def.id.clone());
    perfect_pool(None, zephyrs.as_deref())
}

fn face_for(def: &CardDef, radiant: bool) -> &CardFace {
    if radiant { &def.radiant } else { &def.base }
}

/// §4.4 steps 2 and 3: what one hit of `amount` takes off that hero (R44), by the pipeline's own Armor
/// (stored plus every backrow grant, #84, summed per R124), so the score and the hit never disagree.
fn hero_hit(state: &GameState, player: PlayerId, amount: i32, pierce: bool) -> i32 {
    // R346: Pierce skips step 2; B5 E6: divisors included.
    crate::damage::hero_hit_amount(state, player, amount, pierce)
}

/// Damage the viewer's board can already send at the enemy hero this turn, after Armor and the cap
/// (R44), from every unit `can_attack_in` lets attack it now: a board that cannot reach it projects 0.
pub fn projected_board_damage(state: &GameState, viewer: PlayerId) -> i32 {
    projected_board_damage_in(&AuraTable::new(state), viewer)
}

/// `projected_board_damage` read through a sweep's `AuraTable` (#892).
pub fn projected_board_damage_in(auras: &AuraTable<'_>, viewer: PlayerId) -> i32 {
    let state = auras.state();
    let enemy = opponent_of(viewer);
    let at_hero = DamageTarget::Hero { player: enemy };
    let mut sum = 0;
    for unit in crate::zones::active_units_of(state, viewer).iter() {
        if crate::combat::can_attack_in(auras, unit, &at_hero) {
            sum += hero_hit(
                state,
                enemy,
                auras.unit_view(unit).attack,
                crate::damage::pierces(state, Some(unit), None),
            );
        }
    }
    sum
}

/// §10.7 priority 1: a Unit with Charge, the only printed sign a card can hit the hero the turn it
/// arrives (§6.1; Rush may not), affordable, with a zone to land in, a clear path and damage of its
/// own. A damage spell, a Taunt-remover or a buff that makes the swing lethal is text, out of reach.
fn lethal_contribution(auras: &AuraTable<'_>, viewer: PlayerId, def: &CardDef, face: &CardFace) -> i32 {
    let state = auras.state();
    if def.type_ != CardType::Unit {
        return 0;
    }
    if !has_keyword(&face.keywords, KeywordKind::Charge) {
        return 0;
    }
    // R1223: lethal may be bought on borrowed mana.
    if crate::catalog::query_cost(def) > crate::credit::spendable_mana(state, viewer) {
        return 0;
    }
    // It must reach the field this turn (§3.2; Stack onto an occupied zone, #92).
    if !has_room_to_play(state, viewer, face) {
        return 0;
    }
    // And it must reach the hero: §4.2 step 3 makes any enemy Taunt unit the only legal target.
    let enemy = opponent_of(viewer);
    let taunted = crate::zones::active_units_of(state, enemy)
        .iter()
        .any(|unit| auras.unit_has(unit, KeywordKind::Taunt));
    if taunted {
        return 0;
    }
    hero_hit(
        state,
        enemy,
        face.attack.unwrap_or(0),
        has_keyword(&face.keywords, KeywordKind::Pierce),
    )
}

/// §3.2: whether a Unit with this face could be played into the viewer's unit row now.
fn has_room_to_play(state: &GameState, viewer: PlayerId, face: &CardFace) -> bool {
    if crate::zones::first_free_zone(state, viewer, Row::Units).is_some() {
        return true;
    }
    if !has_keyword(&face.keywords, KeywordKind::Stack) {
        return false;
    }
    crate::zones::slots_of(viewer, Row::Units)
        .iter()
        .any(|slot| !crate::zones::is_locked(state, slot) && !crate::zones::is_reserved(state, slot))
}

/// §10.7 priority 2: an attack that destroys an enemy unit through its Armor, or Poisonous (§6.1);
/// Divine Shield and Indestructible put a unit out of reach of both, and Cleave answers up to three
/// (§3.1). Board wipes and destroys are text.
fn killable_units<C: Borrow<CardInstance>>(auras: &AuraTable<'_>, enemy_units: &[C], face: &CardFace) -> i32 {
    let attack = face.attack.unwrap_or(0);
    let poisonous = has_keyword(&face.keywords, KeywordKind::Poisonous);
    let answered = enemy_units
        .iter()
        .filter(|unit| {
            let unit: &CardInstance = <C as Borrow<CardInstance>>::borrow(*unit);
            let view = auras.unit_view(unit);
            if has_keyword(&view.keywords, KeywordKind::Indestructible) {
                return false;
            }
            if has_keyword(&view.keywords, KeywordKind::DivineShield)
                && unit.divine_shield_spent != Some(true)
            {
                return false;
            }
            if poisonous {
                return attack > view.armor;
            }
            attack - view.armor >= view.health
        })
        .count() as i32;

    let reach = if has_keyword(&face.keywords, KeywordKind::Cleave) {
        3
    } else {
        1
    };
    answered.min(reach)
}

/// §10.7 priority 3. The one heal printed on a card face is Lifesteal, which heals its controller's
/// hero for the damage it deals (§6.1, §4.4 step 8). "Restore N Health" and healing Cries are text.
fn heals_from_printed_data(face: &CardFace) -> bool {
    has_keyword(&face.keywords, KeywordKind::Lifesteal)
}

/// §10.7's fallback, first half: printed stats per mana. A spell prints no stats, so it scores 0.
fn stats_per_mana(def: &CardDef, face: &CardFace) -> f64 {
    let stats = face.attack.unwrap_or(0) + face.health.unwrap_or(0);
    f64::from(stats) / f64::from(crate::catalog::query_cost(def).max(1))
}

/// §10.7's fallback, second half: "draw value", whose printed stand-in is what the card leaves beyond
/// its turn: Reborn's second body, Divine Shield's second life, a backrow permanent that keeps working
/// where a spell is spent on resolution (§3.2).
fn draw_value(def: &CardDef, face: &CardFace) -> f64 {
    let mut value = 0.0;
    if has_keyword(&face.keywords, KeywordKind::Reborn) {
        value += 1.0;
    }
    if has_keyword(&face.keywords, KeywordKind::DivineShield) {
        value += 0.5;
    }
    if matches!(
        def.type_,
        CardType::FieldSpell | CardType::Trap | CardType::FieldTrap
    ) {
        value += 0.5;
    }
    value
}

// The dry run (§10.7: "for each candidate, simulate a dry-run score")

/// What playing a candidate now did, read off a copy of the state it was played in.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DryRun {
    pub lethal: bool,
    pub clears: bool,
    pub heals: bool,
}

const NOTHING: DryRun = DryRun {
    lethal: false,
    clears: false,
    heals: false,
};

/// The seed a dry run draws from: its own, so the match's rng is never touched (§9.3). The
/// autopilot's trials draw from it too (R1183003).
pub(crate) const DRY_RUN_SEED: &str = "zephyrs-dry-run";

/// Whether `state` is a dry run's copy (only those carry `conceal_from`'s stand-ins), so no dry run
/// runs inside one. The sink says so too (`EngineSink::dry_running`, SURFACE §3, §6.5).
fn dry_running(state: &GameState) -> bool {
    state.transient_defs.contains_key(HIDDEN_CARD_DEF_ID)
}

/// What the dry run reads off one `play` action.
struct PlayFields<'a> {
    x: i32,
    embiggen: bool,
    equip: bool,
    tributes: &'a [String],
    zone: Option<&'a ZoneChoice>,
    first_target: Option<&'a Selection>,
}

fn play_fields(action: &PlayAction) -> PlayFields<'_> {
    PlayFields {
        x: action.x.unwrap_or(0),
        embiggen: action.embiggen == Some(true),
        equip: action.equip == Some(true),
        tributes: action.tributes.as_deref().unwrap_or(&[]),
        zone: action.zone.as_ref(),
        first_target: action.targets.as_ref().and_then(|targets| targets.first()),
    }
}

/// The plays a dry run tries: `play_actions_for`'s at its strongest price (largest X, embiggened) and
/// one zone per paying set. Plays aimed at a hero or an enemy card, or paying with the enemy's units
/// (#55, R101), come first, so the viewer's own permanents cannot crowd an enemy unit out of the tries.
pub(crate) fn dry_run_plays(state: &GameState, viewer: PlayerId, card: &CardInstance) -> Vec<PlayAction> {
    // MD-D28, R1125: a card in the graveyard a permission lets its player play is played from there
    // (`play_choices::graveyard_play_actions_for`), as the judge's candidates may be.
    let in_graveyard = state.players[viewer]
        .graveyard
        .iter()
        .any(|held| held.id == card.id);
    let all = if in_graveyard {
        graveyard_play_actions_for(state, viewer, card)
    } else {
        crate::play_choices::play_actions_for(state, viewer, card)
    };
    if all.is_empty() {
        return vec![];
    }
    let x = all.iter().map(|action| play_fields(action).x).max().unwrap_or(0);
    let embiggen = all.iter().any(|action| play_fields(action).embiggen);
    // One zone per Tribute set: R391 pairs each set with the zones it leaves open, so a set that pays
    // with the enemy's units (#55) may not be offered the zone a set of the viewer's own empties.
    let mut zone_for: IndexMap<String, String> = IndexMap::new();
    let priced: Vec<PlayAction> = all
        .into_iter()
        .filter(|action| {
            let fields = play_fields(action);
            if fields.x != x || fields.embiggen != embiggen {
                return false;
            }
            // Keyed by serde JSON (SURFACE §4.4.3); R1086: an Equip play keys apart.
            let paying = format!(
                "{}|{}",
                serde_json::to_string(fields.tributes).unwrap_or_default(),
                fields.equip
            );
            let zone = serde_json::to_string(&fields.zone).unwrap_or_default();
            let kept = zone_for.get(&paying).cloned();
            if kept.is_none() {
                zone_for.insert(paying, zone.clone());
            }
            kept.unwrap_or_else(|| zone.clone()) == zone
        })
        .collect();
    // Ties keep `play_actions_for`'s order.
    let mut ranked: Vec<(PlayAction, usize, i32, i32)> = priced
        .into_iter()
        .enumerate()
        .map(|(at, action)| {
            let rank = target_rank(state, viewer, &action);
            let tribute = enemy_tributes(state, viewer, &action);
            (action, at, rank, tribute)
        })
        .collect();
    ranked.sort_by(|a, b| a.2.cmp(&b.2).then(b.3.cmp(&a.3)).then(a.1.cmp(&b.1)));
    ranked
        .into_iter()
        .take(SCORER_DRY_RUN_PLAYS)
        .map(|(action, ..)| action)
        .collect()
}

/// Which of a play's targets the dry run tries first: a hero, then an enemy card, then the rest.
fn target_rank(state: &GameState, viewer: PlayerId, action: &PlayAction) -> i32 {
    let Some(aim) = play_fields(action).first_target else {
        return 0;
    };
    match aim {
        Selection::Hero { player } => {
            if *player == viewer {
                1
            } else {
                0
            }
        }
        Selection::Instance { instance_id } => match crate::state::find_instance(state, instance_id) {
            Some(card) if card.controller != viewer => 2,
            _ => 3,
        },
        _ => 3,
    }
}

/// How many of the enemy's units a play tributes (#55, R101).
fn enemy_tributes(state: &GameState, viewer: PlayerId, action: &PlayAction) -> i32 {
    play_fields(action)
        .tributes
        .iter()
        .filter(|id| crate::state::find_instance(state, id).is_some_and(|unit| unit.controller != viewer))
        .count() as i32
}

/// Whether playing a card can do anything this turn that §10.7's three questions ask about (a
/// Spell's script is its Cry, §10.9). A Trap only answers later (§5.1), and a Death, a turn hook or a
/// flag does nothing now its printed data does not say.
fn may_act_now(state: &GameState, def: &CardDef, radiant: bool) -> bool {
    if matches!(def.type_, CardType::Trap | CardType::FieldTrap) {
        return false;
    }
    // A Charge body attacks the hero the turn it lands (§6.1), for what the viewer's auras make of its
    // attack (§10.4 layer 5), which only a play on the board can read.
    if def.type_ == CardType::Unit && has_keyword(&face_for(def, radiant).keywords, KeywordKind::Charge) {
        return true;
    }
    let scripts = crate::scripts::scripts_ref(state, &def.id);
    let script = if radiant { &scripts.radiant } else { &scripts.base };
    // A Tribute that may take the enemy's units (#55, R101) changes their board as it is paid.
    script
        .static_flags
        .as_ref()
        .and_then(|flags| flags.tribute_enemies)
        == Some(true)
        || script.cry.is_some()
        || script.aura.is_some()
        || script.attack_mods.is_some()
        || script.set_stat.is_some()
        || script.on_play_hook.is_some()
        || !script.triggers.is_empty()
}

/// §10.7's dry run of one candidate's plays, each made on a copy of `base` with the viewer's own mana.
/// Lethal: the enemy hero is dead, or the viewer's board can then finish it this turn; clear: the enemy
/// had units and has none; heal: the viewer's hero ends higher. A play that asks something is read at
/// the question.
fn trial_plays(base: &GameState, viewer: PlayerId, card: &CardInstance) -> DryRun {
    let enemy = opponent_of(viewer);
    let enemy_units_before = crate::zones::active_units_of(base, enemy).len();
    let health_before = base.players[viewer].hero.health;
    // A play aimed at the viewer's own hero only answers the heal question, moot at SCORER_LOW_HEALTH+.
    let heal_matters = health_before < SCORER_LOW_HEALTH;
    let mut outcome = NOTHING;
    for action in dry_run_plays(base, viewer, card) {
        if !heal_matters && target_rank(base, viewer, &action) == 1 {
            continue;
        }
        let mut trial = base.clone();
        let mut events = Vec::new();
        let mut rng = Rng::new(DRY_RUN_SEED, 0);
        {
            let mut sink = EngineSink::new(&mut trial, &mut events, &mut rng);
            sink.dry_running = true;
            if crate::play_steps::run_play_steps(&mut sink, viewer, &action).is_err() {
                continue;
            }
            crate::triggers::settle(&mut sink, crate::triggers::SettleOptions::default());
        }

        // R1168003: in a Duos duel lethal is judged against the enemy's own HP and its team's pool
        // both (§13.4's lethal judge).
        let enemy_health = crate::duos_life::effective_life(&trial, enemy);
        let won = trial
            .result
            .as_ref()
            .is_some_and(|result| result.winner == Winner::from(viewer));
        // R2201000: no lethal against a seat that cannot lose.
        let can_lose = crate::state::can_lose(&trial, enemy);
        if won
            || (trial.result.is_none() && can_lose && enemy_health <= projected_board_damage(&trial, viewer))
        {
            outcome.lethal = true;
        }
        if enemy_units_before > 0 && crate::zones::active_units_of(&trial, enemy).is_empty() {
            outcome.clears = true;
        }
        if trial.players[viewer].hero.health > health_before {
            outcome.heals = true;
        }
        if outcome.lethal && outcome.clears && outcome.heals {
            break;
        }
    }
    outcome
}

/// §10.7's dry run: the card joins the viewer's hand on `base` (`dry_run_base`'s copy), is played on a
/// copy of it now, and leaves `base` again.
pub fn dry_run(
    state: &GameState,
    viewer: PlayerId,
    def: &CardDef,
    radiant: bool,
    base: Option<&mut GameState>,
) -> DryRun {
    let Some(base) = base else {
        return NOTHING;
    };
    if dry_running(state) || !may_act_now(state, def, radiant) {
        return NOTHING;
    }
    let mut card = crate::state::new_instance(&mut *base, &def.id, viewer, Zone::Hand { player: viewer });
    card.radiant = radiant;
    base.players[viewer].hand.push(card.clone());

    let outcome = trial_plays(base, viewer, &card);
    let hand = &mut base.players[viewer].hand;
    if let Some(at) = hand.iter().position(|held| held.id == card.id) {
        hand.remove(at);
    }
    outcome
}

/// MD-D28, R1125: §10.7's dry run of the viewer's own card where it stands on the copy (hand or
/// graveyard), at its own price, tuning and face; nothing is removed after: the copy is the judge's.
fn dry_run_card(state: &GameState, viewer: PlayerId, card: &CardInstance, base: &mut GameState) -> DryRun {
    if dry_running(state) {
        return NOTHING;
    }
    let Some(standing) = find_instance(base, &card.id).cloned() else {
        return NOTHING;
    };
    let def = crate::catalog::def_of(Some(base), &standing.def_id).clone();
    if !may_act_now(base, &def, standing.radiant) {
        return NOTHING;
    }
    trial_plays(base, viewer, &standing)
}

/// The copy every dry run of one ranking plays on, or `None` unless the viewer is the active player in
/// the main phase with nothing open. It leaves the history of actions behind (§9.3's nonce dedupe,
/// §10.8's event window), which nothing a play reads is in.
pub fn dry_run_base(state: &GameState, viewer: PlayerId) -> Option<GameState> {
    if state.result.is_some()
        || state.pending.is_some()
        || state.active != viewer
        || state.phase != Phase::Main
    {
        return None;
    }
    Some(concealed_copy(state, viewer))
}

/// R222, R1714000: a copy of the state as `viewer` may read it, without the history of actions: the
/// dry run's copy, and the autopilot's at any moment of its turn, prompt open or not (R1183003).
pub fn concealed_copy(state: &GameState, viewer: PlayerId) -> GameState {
    let mut base = state.clone();
    base.applied = Vec::new();
    conceal_from(&mut base, viewer);
    forget_hidden_records(state, &mut base, viewer);
    base
}

/// R222: the stand-ins for a hand, library or face-down backrow card the viewer may not read: no text,
/// so each answers, casts and fires on nothing, in its place, so every count the viewer sees holds.
pub const HIDDEN_CARD_DEF_ID: &str = "zephyrs:hidden-card";
pub const HIDDEN_TRAP_DEF_ID: &str = "zephyrs:hidden-trap";

fn stand_in(id: &str, type_: CardType) -> CardDef {
    let face = CardFace {
        type_: None,
        attack: None,
        health: None,
        x_stats: None,
        keywords: vec![],
        text: String::new(),
        grants: None,
    };
    CardDef {
        id: id.to_string(),
        index: id.to_string(),
        name: "Hidden card".to_string(),
        set: SetName::Core,
        type_,
        tags: vec![],
        rarity: Rarity::Common,
        printed_rarity: None,
        flavor: None,
        token: false,
        cost: CardCost::Fixed(0),
        refs: None,
        params: None,
        deals_damage: None,
        uses_plague: None,
        resource: None,
        loc: None,
        radiant_fallback: None,
        ingredients: None,
        craft: None,
        snapshot_of: None,
        base: face.clone(),
        radiant: face,
    }
}

/// R33: a face-down Trap or Field Trap the viewer does not control, which the viewer may not read.
fn hidden_trap(state: &GameState, card: &CardInstance, viewer: PlayerId) -> bool {
    card.controller != viewer
        && card.face_up != Some(true)
        && matches!(
            crate::faces::card_type_of(state, card),
            CardType::Trap | CardType::FieldTrap
        )
}

/// `hide` for a face-down trap a backrow zone shows: its Plague Counters are public wherever they
/// sit (R471), so its stand-in keeps them.
fn hide_shown_trap(card: &mut CardInstance) {
    let plague = card.counters.plague;
    hide(card, HIDDEN_TRAP_DEF_ID);
    card.counters.plague = plague;
}

/// The card keeps its place and nothing else.
fn hide(card: &mut CardInstance, def_id: &str) {
    card.def_id = def_id.to_string();
    card.radiant = false;
    card.vanilla = false;
    card.cost_mod = 0;
    card.cost_override = None;
    card.memory = IndexMap::new();
    card.granted_keywords = vec![];
    card.buffs.attack = 0;
    card.buffs.health = 0;
    // R311: the record of what a library card's owner was shown names it as well.
    card.known_as = None;
    // R1714000: and nothing else the card carries, as the AI's placeholder keeps nothing
    // (`jackioh_ai::observe::to_placeholder`).
    card.counters = Counters::default();
    card.granted_tags = None;
    card.chinese = None;
    card.set_as = None;
    card.created = None;
    card.damage = 0;
    card.embiggen_cost_override = None;
    card.x = None;
    card.embiggened = None;
    card.return_to_hand_at_end_of_turn = None;
    card.tuning = None;
    card.brittle = None;
    card.enchantments = None;
}

/// R222, §9.1, §10.8: the opponent's hand, both libraries (§3) and a face-down trap the viewer does
/// not control (R33) become text-less stand-ins in place, or the offer would reveal them.
fn conceal_from(base: &mut GameState, viewer: PlayerId) {
    base.transient_defs.insert(
        HIDDEN_CARD_DEF_ID.to_string(),
        stand_in(HIDDEN_CARD_DEF_ID, CardType::Spell),
    );
    base.transient_defs.insert(
        HIDDEN_TRAP_DEF_ID.to_string(),
        stand_in(HIDDEN_TRAP_DEF_ID, CardType::Trap),
    );
    let opponent = opponent_of(viewer);
    for card in base.players[opponent].hand.iter_mut() {
        hide(card, HIDDEN_CARD_DEF_ID);
    }
    for player in [viewer, opponent] {
        for card in base.players[player].library.iter_mut() {
            hide(card, HIDDEN_CARD_DEF_ID);
        }
        // B5 E21: a face-down card dormant under a backrow pile is as hidden as one on top (R447).
        // The traps are found first, as `card_type_of` reads the state.
        let traps: Vec<String> = {
            let state: &GameState = base;
            let side = &state.players[player];
            side.backrow
                .iter()
                .flatten()
                .chain(side.backrow_piles.iter().flatten().flatten())
                .filter(|card| hidden_trap(state, card, viewer))
                .map(|card| card.id.clone())
                .collect()
        };
        if traps.is_empty() {
            continue;
        }
        let side = &mut base.players[player];
        for card in side.backrow.iter_mut().flatten() {
            if traps.contains(&card.id) {
                hide_shown_trap(card);
            }
        }
        for card in side.backrow_piles.iter_mut().flatten().flatten() {
            if traps.contains(&card.id) {
                hide(card, HIDDEN_TRAP_DEF_ID);
            }
        }
    }
    // R419, R1714000: Rollback's snapshots hold the field as each turn began, face-down traps
    // included; a try that restores one sets it down as the stand-in the field shows.
    let mut history = base.board_history.take();
    for snapshot in history.iter_mut().flatten() {
        for player in [viewer, opponent] {
            let side = &mut snapshot.sides[player];
            for card in side.backrow.iter_mut().flatten() {
                if hidden_trap(base, card, viewer) {
                    hide_shown_trap(card);
                }
            }
            for card in side.backrow_piles.iter_mut().flatten().flatten() {
                if hidden_trap(base, card, viewer) {
                    hide(card, HIDDEN_TRAP_DEF_ID);
                }
            }
        }
    }
    base.board_history = history;
    // R1845000: no seat reads a hidden Nerf, the viewer's own hand's included, so the dry run lands none.
    crate::state::forget_hidden_nerfs(base);
}

/// R1714000: the records the viewer may not read go too, as the AI's `redact` drops them (R185): the
/// opening decks (R676), Glitch boards (R678), the other seat's last board (R417), unrevealed secret
/// choices (R865), and in every continuation a problem's key (R465) and the other seat's in-flight
/// `secret_modes`. Each continuation keeps its shape, so a try still resumes it.
fn forget_hidden_records(state: &GameState, base: &mut GameState, viewer: PlayerId) {
    let opponent = opponent_of(viewer);
    base.opening = None;
    base.glitch_boards = None;
    if let Some(last_boards) = base.last_boards.as_mut() {
        *last_boards.slot(opponent) = None;
        if last_boards.is_empty() {
            base.last_boards = None;
        }
    }
    for secret in base.secrets.iter_mut().flatten() {
        if secret.owner == opponent && secret.revealed != Some(true) {
            secret.choice = None;
        }
    }
    let resumes = base
        .trigger_queue
        .iter_mut()
        .map(|entry| &mut entry.resume)
        .chain(base.work.iter_mut().map(|item| &mut item.resume))
        .chain(base.delayed.iter_mut().map(|entry| &mut entry.resume))
        .chain(base.pending.iter_mut().map(|pending| &mut pending.resume));
    for resume in resumes {
        resume.data.shift_remove(ANSWER_KEY);
        crate::secrets::scrub_secret_modes(state, resume, viewer);
    }
}

/// One candidate's score; `base` is `dry_run_base`'s shared copy, `None` when the viewer cannot play.
pub fn score_def(
    state: &GameState,
    viewer: PlayerId,
    def: &CardDef,
    options: &ScorerOptions,
    base: Option<&mut GameState>,
) -> Scored {
    score_def_in(&AuraTable::new(state), viewer, def, options, base)
}

/// `score_def` read through a sweep's `AuraTable` (#892).
pub fn score_def_in(
    auras: &AuraTable<'_>,
    viewer: PlayerId,
    def: &CardDef,
    options: &ScorerOptions,
    base: Option<&mut GameState>,
) -> Scored {
    let radiant = options.radiant == Some(true);
    let face = face_for(def, radiant);
    let has_base = base.is_some();
    let played = dry_run(auras.state(), viewer, def, radiant, base);
    score_parts(auras, viewer, def, face, played, has_base)
}

/// MD-D28, R1125: the judge's score of one of the viewer's own cards, dry-run where it stands on the
/// shared copy `base` (`dry_run_base`); `None` for a card that is not the viewer's to play.
pub fn score_instance(
    state: &GameState,
    viewer: PlayerId,
    instance_id: &str,
    base: Option<&mut GameState>,
) -> Option<Scored> {
    let card = find_instance(state, instance_id)?.clone();
    if card.controller != viewer {
        return None;
    }
    let def = crate::catalog::def_of(Some(state), &card.def_id).clone();
    let face = face_for(&def, card.radiant).clone();
    let has_base = base.is_some();
    let played = match base {
        None => NOTHING,
        Some(base) => dry_run_card(state, viewer, &card, base),
    };
    let auras = AuraTable::new(state);
    Some(score_parts(&auras, viewer, &def, &face, played, has_base))
}

/// One candidate's score from its dry run: the shared half of `score_def` and `score_instance`.
fn score_parts(
    auras: &AuraTable<'_>,
    viewer: PlayerId,
    def: &CardDef,
    face: &CardFace,
    played: DryRun,
    has_base: bool,
) -> Scored {
    let state = auras.state();
    let enemy = opponent_of(viewer);
    let enemy_units = crate::zones::active_units_of(state, enemy);

    // With a dry run, a Charge body's swing is the one it plays (`may_act_now`); the printed attack
    // stands in only when the viewer cannot play now.
    let contribution = if has_base {
        0
    } else {
        lethal_contribution(auras, viewer, def, face)
    };
    let lethal = played.lethal
        || (contribution > 0
            // R2201000: no lethal against a seat that cannot lose.
            && crate::state::can_lose(state, enemy)
            && projected_board_damage_in(auras, viewer) + contribution
                >= crate::duos_life::effective_life(state, enemy));

    let kills = killable_units(auras, &enemy_units, face);
    let clears = played.clears || (!enemy_units.is_empty() && kills >= enemy_units.len() as i32);

    let heals = state.players[viewer].hero.health < SCORER_LOW_HEALTH
        && (played.heals || heals_from_printed_data(face));

    let parts = ScoredParts {
        lethal: if lethal { SCORER_WEIGHTS.lethal } else { 0.0 },
        clear: if clears { SCORER_WEIGHTS.clears_board } else { 0.0 },
        kills: f64::from(kills) * SCORER_WEIGHTS.per_kill,
        heal: if heals { SCORER_WEIGHTS.heal } else { 0.0 },
        stats: stats_per_mana(def, face) * SCORER_WEIGHTS.stats_per_mana,
        draw: draw_value(def, face) * SCORER_WEIGHTS.draw_value,
    };

    let priority = if lethal {
        ScorePriority::Lethal
    } else if clears {
        ScorePriority::Clear
    } else if heals {
        ScorePriority::Heal
    } else {
        ScorePriority::Value
    };
    let score = parts.lethal + parts.clear + parts.kills + parts.heal + parts.stats + parts.draw;
    Scored {
        def: def.clone(),
        score,
        priority,
        parts,
    }
}

/// Higher score first, then the §5 index, then the catalog id: ids are unique, so the order is strict
/// and total whatever the weights and the catalog's order.
pub fn compare_scored(a: &Scored, b: &Scored) -> Ordering {
    if a.score != b.score {
        return b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal);
    }
    let by_index = index_rank(&a.def.index) - index_rank(&b.def.index);
    // NaN (two infinite ranks) compares equal: the pair stays as it was.
    if by_index.is_nan() {
        return Ordering::Equal;
    }
    if by_index != 0.0 {
        return if by_index > 0.0 {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }
    if a.def.index != b.def.index {
        return if a.def.index < b.def.index {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }
    if a.def.id < b.def.id {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

/// R29, R1446: the perfect pool less #97 (`candidate_defs`), best first, as a total order.
pub fn rank(state: &GameState, viewer: PlayerId, options: &ScorerOptions) -> Vec<Scored> {
    // One copy for every candidate's dry run, each put back as it was.
    let mut base = dry_run_base(state, viewer);
    // #892: one aura table for every candidate.
    let auras = AuraTable::new(state);
    let mut scored: Vec<Scored> = candidate_defs()
        .iter()
        .map(|def| score_def_in(&auras, viewer, def, options, base.as_mut()))
        .collect();
    scored.sort_by(compare_scored);
    scored
}

/// R29, §8 #97: the Discover offers the top `DISCOVER_OPTIONS`. R2204001, R2511001: a card whose pair
/// key (`catalog::pair_key`) a better-ranked card holds is passed over for the next. Only a carried
/// catalog of v0.5.5 to v0.5.7 pairs two cards, so a replay of those games offers what it did; the
/// build's catalog pairs none, so its offer is the top three.
pub fn top_three(state: &GameState, viewer: PlayerId, options: &ScorerOptions) -> Vec<Scored> {
    let mut pairs: Vec<String> = Vec::new();
    rank(state, viewer, options)
        .into_iter()
        .filter(|scored| {
            let key = crate::catalog::pair_key(&scored.def).to_string();
            let fresh = !pairs.contains(&key);
            if fresh {
                pairs.push(key);
            }
            fresh
        })
        .take(DISCOVER_OPTIONS as usize)
        .collect()
}
