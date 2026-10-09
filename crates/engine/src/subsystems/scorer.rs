//! The Zephyrs scorer (SPEC §10.7's scorer bullet, R29, §8 #97): rank every non-token Core
//! definition except Zephyrs itself for the current state, so the Discover can offer the top 3.
//!
//! §10.7's priorities, in order: lethal available → max; can clear the enemy board → high; hero
//! below 10 and the card heals → high; otherwise stats per mana plus draw value.
//!
//! Each priority is read two ways. The printed data — type, cost, stats and keywords (§5) — stands in
//! for it where a card's face says so (a Charge body, a Poisonous one, Lifesteal), named at its own
//! function. And §10.7's dry run plays the candidate on a copy of the state (`dryRun`), which is the
//! only way to see what a card's *text* does: #44's 4 damage, #17's bounce of every unit, #53's heal
//! to 30. A card whose claim to a priority lives in neither scores on stats per mana like any other.
//!
//! The scorer is a pure function of (state, viewer): the dry run draws from a seed of its own and
//! never the match's, and no clock is read, so the same state always produces the same order, which
//! is what R29's Discover and §9.3's replay need. And it is a function of what the viewer may read
//! (R222): the dry run plays on a copy in which every card §9.1 hides from the viewer — the
//! opponent's hand, both libraries, a face-down trap of the other side's — stands in as a card that
//! does nothing, so the three cards it offers never tell the viewer what those cards are.
//!
//! Port of `packages/engine/src/subsystems/scorer.ts`. `SCORER_WEIGHTS`, `SCORER_LOW_HEALTH` and
//! `SCORER_DRY_RUN_PLAYS` live in `crate::config` (CLAUDE.md rule 9, SURFACE §6.4). TS's module
//! `let dryRunning` is gone (SURFACE §3, §6.5): the sink a dry run plays on carries
//! `EngineSink::dry_running`, and a scorer asked again from inside that play — whose state is the
//! dry run's copy — knows it by the stand-ins every copy carries (`dry_running`), so no flag
//! outlives a call and no two games share one. The dry runs' shared copy (`base`) is handed down as
//! `Option<&mut GameState>`: TS's default argument (`dryRunBase(state, viewer)`) is the caller's
//! to compute.

use std::borrow::Borrow;
use std::cmp::Ordering;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::{SCORER_DRY_RUN_PLAYS, SCORER_LOW_HEALTH, SCORER_WEIGHTS};
use crate::damage::DamageTarget;
use crate::layers::unit_view;
use crate::play_choices::{PlayAction, graveyard_play_actions_for};
use crate::prelude::json_as;
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, find_instance};
use crate::wire::{
    CardCost, CardDef, CardFace, CardType, KeywordKind, Phase, PlayerId, Rarity, Row, Selection, SetName,
    Winner, Zone, ZoneChoice, has_keyword, opponent_of,
};

/// R29: the scorer ranks every non-token Core card except #97 itself.
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
    /// Each priority's contribution, for tuning and for the client's tooltip.
    pub parts: ScoredParts,
}

/// R29, §5.1: the candidate pool — Core only, because #97 says "only from the core set" (B2.6), and
/// never #97 itself, named by its id (R387). `query` already drops tokens.
pub fn candidate_defs() -> Vec<&'static CardDef> {
    let zephyrs = crate::catalog::def_by_index(SetName::Core, ZEPHYRS_INDEX).map(|def| def.id.clone());
    let query = match zephyrs {
        None => json!({ "set": "Core" }),
        Some(id) => json!({ "set": "Core", "excludeDefId": id }),
    };
    crate::catalog::query(&json_as(query))
}

fn face_for(def: &CardDef, radiant: bool) -> &CardFace {
    if radiant { &def.radiant } else { &def.base }
}

/// §4.4 steps 2 and 3: what one hit of `amount` actually takes off that hero (R44). Step 2's Armor
/// is `heroArmorOf`, the pipeline's own reader — the stored Armor plus every backrow grant (#84),
/// summed per R124 — so the score and the hit never disagree.
fn hero_hit(state: &GameState, player: PlayerId, amount: i32, pierce: bool) -> i32 {
    // R346: a Pierce unit's hit skips step 2; B5 E6: the pipeline's own reading, divisors included.
    crate::damage::hero_hit_amount(state, player, amount, pierce)
}

/// Damage the viewer's board can already send at the enemy hero this turn: every unit that may
/// legally attack the hero right now, after Armor and the cap (R44). Taunt, exertion, position and
/// summoning sickness are all in `canAttack`, so a board that cannot reach the hero projects 0.
pub fn projected_board_damage(state: &GameState, viewer: PlayerId) -> i32 {
    let enemy = opponent_of(viewer);
    let at_hero = DamageTarget::Hero { player: enemy };
    let mut sum = 0;
    for unit in crate::zones::active_units_of(state, viewer).iter() {
        if crate::combat::can_attack(state, unit, &at_hero) {
            sum += hero_hit(
                state,
                enemy,
                unit_view(state, unit).attack,
                crate::damage::pierces(state, Some(unit), None),
            );
        }
    }
    sum
}

/// §10.7 priority 1. The printed signal for "this card enables lethal" is a Unit with Charge: it is
/// the only printed data that says a card can hit the hero on the turn it arrives (§6.1, and Rush
/// explicitly may not). A card must also be affordable, have a zone to be played into and a clear
/// path to the hero (no enemy Taunt), or the lethal is not available this turn, and it must
/// contribute damage of its own, so "enables" means the card is part of the kill.
///
/// Out of reach from printed data: a damage spell, a Taunt-remover or a buff that would also make
/// the swing lethal. All three live in card text.
fn lethal_contribution(state: &GameState, viewer: PlayerId, def: &CardDef, face: &CardFace) -> i32 {
    if def.type_ != CardType::Unit {
        return 0;
    }
    if !has_keyword(&face.keywords, KeywordKind::Charge) {
        return 0;
    }
    if crate::catalog::query_cost(def) > state.players[viewer].mana.current {
        return 0;
    }
    // It must reach the field this turn: §3.2 plays a Unit into an empty, unlocked zone, or a Stack
    // card onto an occupied one (#92), so a full row keeps anything else off the board.
    if !has_room_to_play(state, viewer, face) {
        return 0;
    }
    // And it must reach the hero: §4.2 step 3 makes any enemy Taunt unit the only legal target, which
    // is the same check `projectedBoardDamage` makes through `canAttack` for the units already there.
    let enemy = opponent_of(viewer);
    let taunted = crate::zones::active_units_of(state, enemy)
        .iter()
        .any(|unit| has_keyword(&unit_view(state, unit).keywords, KeywordKind::Taunt));
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

/// §10.7 priority 2. The printed signal for "answers an enemy unit" is an attack big enough to
/// destroy it through its Armor, or Poisonous, which destroys any unit it damages (§6.1). Divine
/// Shield and Indestructible put a unit out of reach of both. Cleave hits the target's two
/// neighbours, so one attack can answer up to three units (§3.1).
///
/// Out of reach from printed data: every board wipe and every targeted destroy, which are text.
fn killable_units<C: Borrow<CardInstance>>(state: &GameState, enemy_units: &[C], face: &CardFace) -> i32 {
    let attack = face.attack.unwrap_or(0);
    let poisonous = has_keyword(&face.keywords, KeywordKind::Poisonous);
    let answered = enemy_units
        .iter()
        .filter(|unit| {
            let unit: &CardInstance = <C as Borrow<CardInstance>>::borrow(*unit);
            let view = unit_view(state, unit);
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
/// hero for the damage it deals (§6.1, §4.4 step 8).
///
/// Out of reach from printed data: every "Restore N Health" and every healing Cry, which are text.
fn heals_from_printed_data(face: &CardFace) -> bool {
    has_keyword(&face.keywords, KeywordKind::Lifesteal)
}

/// §10.7's fallback, first half: printed stats per mana. A spell prints no stats, so it scores 0.
fn stats_per_mana(def: &CardDef, face: &CardFace) -> f64 {
    let stats = face.attack.unwrap_or(0) + face.health.unwrap_or(0);
    f64::from(stats) / f64::from(crate::catalog::query_cost(def).max(1))
}

/// §10.7's fallback, second half: "draw value". "Draw a card" is text, so this is the printed-data
/// stand-in — what the card leaves behind beyond the turn it is played. Reborn is a second body
/// from one card, Divine Shield buys a second life, and a backrow permanent keeps working after it
/// lands while a spell is spent on resolution (§3.2).
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

// ---------------------------------------------------------------------------
// The dry run (§10.7: "for each candidate, simulate a dry-run score")
// ---------------------------------------------------------------------------

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

/// The seed a dry run draws from: its own, so the match's rng is never touched (§9.3).
const DRY_RUN_SEED: &str = "zephyrs-dry-run";

/// TS's `let dryRunning`: set while a dry run is playing a candidate, so a candidate whose play would
/// ask the scorer again scores on printed data rather than running a dry run inside a dry run. A dry
/// run plays only on copies of `dryRunBase`'s copy, every one of which carries the stand-ins
/// `concealFrom` wrote, and nothing else ever does: so a state that carries them is a dry run's, and
/// the scorer asked about it is inside one. The sink the play runs on says so too
/// (`EngineSink::dry_running`, SURFACE §6.5), for the engine code that holds it.
fn dry_running(state: &GameState) -> bool {
    state.transient_defs.contains_key(HIDDEN_CARD_DEF_ID)
}

/// What the dry run reads off one `play` action (TS reads `x`, `embiggen`, `tributes`, `zone` and
/// `targets[0]` straight off it).
struct PlayFields<'a> {
    x: i32,
    embiggen: bool,
    tributes: &'a [String],
    zone: Option<&'a ZoneChoice>,
    first_target: Option<&'a Selection>,
}

fn play_fields(action: &PlayAction) -> PlayFields<'_> {
    PlayFields {
        x: action.x.unwrap_or(0),
        embiggen: action.embiggen == Some(true),
        tributes: action.tributes.as_deref().unwrap_or(&[]),
        zone: action.zone.as_ref(),
        first_target: action.targets.as_ref().and_then(|targets| targets.first()),
    }
}

/// The plays a dry run tries: `playChoices.playActionsFor`'s, narrowed to one price per card — the
/// largest X and the embiggened price when either is affordable, the strongest thing the card can do
/// now — and one zone per paying set, since where a card lands is not what §10.7's three questions ask. What it aims
/// at and what it tributes are: the enemy hero is lethal's target and the viewer's own a heal's, an
/// enemy card is what a clear takes off the board, and a Tribute of the enemy's units (#55, R101) is
/// one — so those plays come first, whatever order `playActionsFor` found them in, and a board full
/// of the viewer's own permanents cannot crowd the one enemy unit out of the plays tried.
fn dry_run_plays(state: &GameState, viewer: PlayerId, card: &CardInstance) -> Vec<PlayAction> {
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
            // TS keys by `JSON.stringify` (SURFACE §4.4.3): the same struct's serde JSON.
            let paying = serde_json::to_string(fields.tributes).unwrap_or_default();
            let zone = serde_json::to_string(&fields.zone).unwrap_or_default();
            let kept = zone_for.get(&paying).cloned();
            if kept.is_none() {
                zone_for.insert(paying, zone.clone());
            }
            kept.unwrap_or_else(|| zone.clone()) == zone
        })
        .collect();
    // A stable sort, so plays that aim alike keep the order `playActionsFor` gave them.
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

/// Whether playing a card can do anything this turn that §10.7's three questions ask about: a Cry
/// (a Spell's script is its Cry, §10.9), an aura or a stat hook it brings to the field, a trigger, an
/// on-play hook, a Charge body's swing, or a Tribute paid with the enemy's units. Anything else — a Trap, which only answers the opponent later (§5.1); a body
/// whose text is a Death, a turn hook or a flag later plays read — does nothing now that its printed
/// data does not already say, so it is not played at all.
fn may_act_now(state: &GameState, def: &CardDef, radiant: bool) -> bool {
    if matches!(def.type_, CardType::Trap | CardType::FieldTrap) {
        return false;
    }
    // A Charge body attacks the hero the turn it lands (§6.1), for what the viewer's auras make of its
    // attack (§10.4 layer 5), which only a play on the board can read.
    if def.type_ == CardType::Unit && has_keyword(&face_for(def, radiant).keywords, KeywordKind::Charge) {
        return true;
    }
    // TS `scriptsFor(def.id)`: a candidate is a catalog card, read through the one script lookup.
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

/// §10.7's dry run of one candidate's plays: each play `dryRunPlays` names is made on a copy of
/// `base` (`dryRunBase`) with the viewer's own mana, and the copy after the play says what it did.
/// Lethal is available when the enemy hero is dead, or when what the viewer's board can then send at
/// it this turn finishes it (a Charge unit, a buff, a Taunt removed); it clears the enemy board when
/// the enemy had units and has none; and it heals when the viewer's hero ends above where it began. A
/// card whose play asks something is read as it stands at the question.
///
/// The copy draws from a seed of its own, so the match's state and rng are untouched and the ranking
/// stays a pure function of the state (R29, §9.3).
fn trial_plays(base: &GameState, viewer: PlayerId, card: &CardInstance) -> DryRun {
    let enemy = opponent_of(viewer);
    let enemy_units_before = crate::zones::active_units_of(base, enemy).len();
    let health_before = base.players[viewer].hero.health;
    // A play aimed at the viewer's own hero can only answer the heal question, which asks nothing
    // of a hero at SCORER_LOW_HEALTH or more, so it is not played then.
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

        let enemy_health = trial.players[enemy].hero.health;
        let won = trial
            .result
            .as_ref()
            .is_some_and(|result| result.winner == Winner::from(viewer));
        if won || (trial.result.is_none() && enemy_health <= projected_board_damage(&trial, viewer)) {
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

/// §10.7's dry run: the card is put in the viewer's hand on a copy of the state and played now.
///
/// The play is made on a copy of `base` (`dryRunBase`), which the candidate joins for its turn and
/// leaves again.
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
    // TS's `finally`: the candidate leaves the shared copy's hand again.
    let hand = &mut base.players[viewer].hand;
    if let Some(at) = hand.iter().position(|held| held.id == card.id) {
        hand.remove(at);
    }
    outcome
}

/// MD-D28, R1125: §10.7's dry run of a card already on the copy — the viewer's own card, at its own
/// price, tuning and face, where it stands (hand or graveyard). No `new_instance`, and it is not
/// removed afterwards: the copy is the judge's to keep.
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

/// The copy of the state every dry run of one ranking plays on, or null when the viewer cannot play
/// now: only the active player in the main phase with nothing open can (a rank a test asks of the
/// other seat has no dry run, and the printed signals alone score it). The copy leaves the history
/// of actions behind (§9.3's nonce dedupe, §10.8's event window), which nothing a play reads is in.
pub fn dry_run_base(state: &GameState, viewer: PlayerId) -> Option<GameState> {
    if state.result.is_some()
        || state.pending.is_some()
        || state.active != viewer
        || state.phase != Phase::Main
    {
        return None;
    }
    let mut base = state.clone();
    base.applied = Vec::new();
    conceal_from(&mut base, viewer);
    Some(base)
}

/// R222: the stand-ins for the cards the viewer may not read — a hand or library card, and a
/// face-down backrow card. Each is a card with no text, so it answers nothing, casts nothing when
/// drawn and fires on nothing; it keeps its place, so every count the viewer can see is unchanged.
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
        token: false,
        cost: CardCost::Fixed(0),
        refs: None,
        params: None,
        loc: None,
        radiant_fallback: None,
        ingredients: None,
        base: face.clone(),
        radiant: face,
    }
}

/// TS `concealFrom`'s `hide(card, defId)`: the card keeps its place and nothing else.
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
}

/// R222, §9.1, §10.8: the dry run plays on what the viewer may read. The opponent's hand and both
/// libraries are hidden (the viewer's own library from the viewer too, §3), as is a face-down trap
/// the viewer does not control (R33) — so on the copy each of those becomes a stand-in with no text,
/// in the same place. Played on the real cards instead, a Sheepish would turn a Unit candidate into a
/// Sheep before its Cry, and a cast on draw at the top of the viewer's library would hurt the viewer,
/// and the three cards offered would say what the trap is or what is on top of the library.
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
        // Which cards are traps is read first (`cardTypeOf` reads the state), then they are hidden.
        let traps: Vec<String> = {
            let state: &GameState = base;
            let side = &state.players[player];
            side.backrow
                .iter()
                .flatten()
                .chain(side.backrow_piles.iter().flatten().flatten())
                .filter(|card| card.controller != viewer && card.face_up != Some(true))
                .filter(|card| {
                    matches!(
                        crate::faces::card_type_of(state, card),
                        CardType::Trap | CardType::FieldTrap
                    )
                })
                .map(|card| card.id.clone())
                .collect()
        };
        if traps.is_empty() {
            continue;
        }
        let side = &mut base.players[player];
        for card in side.backrow.iter_mut().flatten() {
            if traps.contains(&card.id) {
                hide(card, HIDDEN_TRAP_DEF_ID);
            }
        }
        for card in side.backrow_piles.iter_mut().flatten().flatten() {
            if traps.contains(&card.id) {
                hide(card, HIDDEN_TRAP_DEF_ID);
            }
        }
    }
}

/// One candidate's score for this state. Pure: the same arguments always give the same number.
///
/// `base` is TS's `base = dryRunBase(state, viewer)`: the copy the dry run plays on, shared by every
/// candidate of one ranking, or `None` when the viewer cannot play now.
pub fn score_def(
    state: &GameState,
    viewer: PlayerId,
    def: &CardDef,
    options: &ScorerOptions,
    base: Option<&mut GameState>,
) -> Scored {
    let radiant = options.radiant == Some(true);
    let face = face_for(def, radiant);
    let has_base = base.is_some();
    // What the card's text does, which its printed data cannot say (§10.7's dry run).
    let played = dry_run(state, viewer, def, radiant, base);
    score_parts(state, viewer, def, face, played, has_base)
}

/// MD-D28, R1125: one of the viewer's own cards' score for this state — the judge's reading of a
/// play. The card is dry-run where it stands, at its own price, tuning and face, on the shared copy
/// `base` the caller computed (`dry_run_base`); `None` for a card that is not the viewer's to play.
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
    Some(score_parts(state, viewer, &def, &face, played, has_base))
}

/// One candidate's score from its dry run: the shared half of `score_def` and `score_instance`.
fn score_parts(
    state: &GameState,
    viewer: PlayerId,
    def: &CardDef,
    face: &CardFace,
    played: DryRun,
    has_base: bool,
) -> Scored {
    let enemy = opponent_of(viewer);
    let enemy_units = crate::zones::active_units_of(state, enemy);

    // With a dry run to read, a Charge body's swing is the one it plays (`mayActNow`), through the
    // viewer's auras; the printed attack stands in for it only when the viewer cannot play now.
    let contribution = if has_base {
        0
    } else {
        lethal_contribution(state, viewer, def, face)
    };
    let lethal = played.lethal
        || (contribution > 0
            && projected_board_damage(state, viewer) + contribution >= state.players[enemy].hero.health);

    let kills = killable_units(state, &enemy_units, face);
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

/// §5's index as a number, so "2" sorts before "10" and a token suffix still has a place.
fn index_rank(index: &str) -> f64 {
    // SURFACE §4.4.4: `Number.parseFloat` on catalog indexes, non-finite as +Infinity.
    match index.parse::<f64>() {
        Ok(parsed) if parsed.is_finite() => parsed,
        _ => f64::INFINITY,
    }
}

/// The order two candidates sit in: higher score first, then the §5 index, then the catalog id.
/// Ids are unique, so this is a strict total order whatever the weights are — `rank` never leaves
/// two candidates tied and never depends on the order the catalog handed them over.
pub fn compare_scored(a: &Scored, b: &Scored) -> Ordering {
    if a.score != b.score {
        // TS `b.score - a.score`.
        return b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal);
    }
    let by_index = index_rank(&a.def.index) - index_rank(&b.def.index);
    // TS returns `byIndex` whenever it is not 0, NaN included (two infinite ranks), and `sort` reads a
    // NaN comparison as 0: the pair stays as it was.
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

/// R29: every non-token Core definition except #97, best first, as a total order.
pub fn rank(state: &GameState, viewer: PlayerId, options: &ScorerOptions) -> Vec<Scored> {
    // One copy for every candidate's dry run: each plays on a copy of it and puts it back as it was.
    let mut base = dry_run_base(state, viewer);
    let mut scored: Vec<Scored> = candidate_defs()
        .iter()
        .map(|def| score_def(state, viewer, def, options, base.as_mut()))
        .collect();
    scored.sort_by(compare_scored);
    scored
}

/// R29, §8 #97: the Discover offers the top 3.
pub fn top_three(state: &GameState, viewer: PlayerId, options: &ScorerOptions) -> Vec<Scored> {
    rank(state, viewer, options).into_iter().take(3).collect()
}
