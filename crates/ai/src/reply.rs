//! The opponent's reply (SPEC §9.9, docs/polish/3-ai.md "The opponent's reply"). A line that passes
//! the turn is worth what remains after the opponent answers, scored one turn deeper.
//!
//! Rule order:
//! 1. Play handed cards (e.g. bounced units); one drawn guess at a secret of the seat's, and every play
//!    judged after the turn's end while a handed card would be discarded there (R3696001).
//! 2. Face-up activations (up to AI_REPLY.visible_activations).
//! 3. Static trades: best value attack (lethal, kill, trade up, face; Deft trials R49).
//! 4. Defense switches up to AI_REPLY.defense_switches (#1696).
//!
//! Step evaluation uses real `reduce` calls (catching unwinds, SURFACE §4.4.9). Countdowns
//! waiting where reply ends are blended via `AI_REPLY.countdown_share` (#1995, R1181000).

use std::cmp::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::IndexSet;
use jackioh_engine::{
    Action, ActionBody, AttackTarget, CardInstance, DelayedAt, GameState, KeywordKind, Phase, PlayerId,
    Position, Rng, SECRET_KEY, Selection, card_keywords, effective_life, find_instance, has_countdown,
    has_keyword, legal_actions, pierces, reduce_unpiloted, secret_of, subsystems, takes_strike_back,
    temporary_number, unit_view,
};

use crate::candidates::{Side, blow, candidate_actions_in, cleaved_neighbours, exchange, side_of};
use crate::config::{AI_EVAL, AI_GUESS_SEED, AI_REPLY};
use crate::evaluate::{
    NextSwing, Prices, coming_mana, evaluate_priced, float_credit, heal_converted, heal_worth, kill_worth,
    unit_worth,
};
use crate::learned::finalist_score;
use crate::lethal::{deft_ready_switch, ready_gap};
use crate::simulate::{
    AnswersTried, LineStatus, best_auto_answer, charge_announced, line_status, note_sim_panic, simulate,
    static_score, surcharge_nodes,
};
use crate::types::NodeCounter;

/// `trade_value`'s answer: what an attack is worth to the static trader, whether its strike kills
/// the target and whether its Cleave splash kills a neighbour.
pub(crate) struct Trade {
    pub(crate) value: f64,
    pub(crate) kills: bool,
    pub(crate) splash_kills: bool,
}

/// §4.4 step 8: what a Lifesteal hit's heal of `amount` is worth to `healer`: `heal_worth` on the hero
/// curve, or, when a heal converter turns it into damage (E8, R413), minus that much Pierce damage on
/// `healer`'s own hero (#3858).
fn lifesteal_value(state: &GameState, healer: PlayerId, amount: i32) -> f64 {
    if heal_converted(state, healer) {
        // E8, R413: the heal becomes that much Pierce damage on the healing hero instead,
        // on the stated amount (`damage.rs`: a replaced heal is never multiplied), ignoring
        // Armor, so `reply_face_value` without `projected_hero_damage`'s Armor pass.
        -reply_face_value(state, healer, amount)
    } else {
        heal_worth(state, healer, amount, &AI_EVAL)
    }
}

/// The opponent's static value of `attacker` hitting `target` (§4.3): what killing the target removes
/// (`kill_worth`, #1796), less what its own death removes when it dies too, with First Strike deciding
/// who strikes first and no strike back on a Ranged attacker unless the target is Ranged too (R783000).
/// A hit that kills nothing is worth a little per point dealt, so a second attacker can finish what the
/// first started. A Cleave splash is priced at `AI_REPLY.cleave_share` of what it kills or unshields
/// (R63), a Trample excess as a face hit and a Lifesteal heal by `heal_worth` (#1820). The move order
/// reads `kills` and `splash_kills` too (`candidates.rs`'s even trades, #1779, #1821).
pub(crate) fn trade_value(
    state: &GameState,
    attacker: &CardInstance,
    target_id: &str,
    prices: &Prices,
) -> Trade {
    let Some(target) = find_instance(state, target_id) else {
        return Trade {
            value: f64::NEG_INFINITY,
            kills: false,
            splash_kills: false,
        };
    };
    let a = side_of(state, attacker);
    let t = side_of(state, target);
    // R1120, R1283: each strike as the combat reads it, the bonus and the lane multiplier included.
    let combat = exchange(state, attacker, &a.view, target, &t.view);
    let out = blow(&a, combat.hit, combat.poisonous, &t);
    let mut kills = out.kills;
    let mut dies =
        takes_strike_back(state, attacker, target) && blow(&t, combat.strike_back, t.poisonous, &a).kills;
    if a.first_strike && !t.first_strike && kills {
        dies = false;
    }
    if t.first_strike && !a.first_strike && dies {
        kills = false;
    }
    // §4.3 step 1: an attacker a First Strike defender destroys first strikes nothing at all.
    let lands = !(t.first_strike && !a.first_strike && dies);
    // §4.4 step 5: a Trample attacker's hit counts on a unit only up to its health, the rest being step
    // 9's hit on the hero.
    let trample = has_keyword(&a.view.keywords, KeywordKind::Trample);
    let on_unit = |dealt: i32, to: &Side| {
        if trample {
            dealt.min(to.view.health.max(0))
        } else {
            dealt
        }
    };

    // R63: the splash belongs to the attack, so it lands whenever the strike does — every case but
    // the defender-first one above, where the attacker is destroyed before it strikes at all.
    let mut cleave_value = 0.0;
    let mut splashed = 0;
    let mut splash_kills = false;
    if has_keyword(&a.view.keywords, KeywordKind::Cleave) && lands {
        for neighbour in cleaved_neighbours(state, attacker, target) {
            let n = side_of(state, &neighbour);
            let splash = blow(&a, combat.splash, combat.poisonous, &n);
            splashed += on_unit(splash.dealt, &n);
            if splash.kills {
                splash_kills = true;
                cleave_value += AI_REPLY.cleave_share * unit_worth(state, &neighbour, &AI_EVAL, prices);
            } else if splash.pops_shield {
                cleave_value += AI_REPLY.cleave_share * AI_EVAL.keyword.divine_shield;
            }
        }
    }

    let mut value = if kills {
        kill_worth(state, target, &AI_EVAL, prices)
    } else {
        AI_REPLY.chip_per_damage * f64::from(out.dealt.min(t.view.health))
    };
    if out.pops_shield {
        value += AI_EVAL.keyword.divine_shield;
    }
    if dies {
        value -= kill_worth(state, attacker, &AI_EVAL, prices);
    }
    // §4.4 step 9: what the attack's hits deal beyond each unit's health goes on to the target's hero
    // through its Armor and caps (`projected_damage`: a Cleave's hits too, R176, and nothing when a
    // First Strike defender destroys the attacker first).
    let trampled = if trample {
        subsystems::projected_damage(
            state,
            attacker,
            &AttackTarget::Unit {
                instance: target.clone(),
            },
        )
    } else {
        0
    };
    if trampled > 0 {
        value += reply_face_value(state, target.controller, trampled);
    }
    // §4.4 step 8: Lifesteal heals the attacker's hero by what each of its hits deals, which step 5 does
    // not cap at the unit's health but for a Trample hit, whose rest heals as it lands on the hero.
    if lands && has_keyword(&a.view.keywords, KeywordKind::Lifesteal) {
        let amount = on_unit(out.dealt, &t) + splashed + trampled;
        value += lifesteal_value(state, attacker.controller, amount);
    }
    Trade {
        value: value + cleave_value,
        kills,
        splash_kills,
    }
}

/// Static trader's value of `damage` to `seat`'s hero: `AI_EVAL.win` if lethal against `effective_life`
/// (pool in Duos, R964000; never if cannot lose, R2201000), else `AI_REPLY.face_per_damage` per point.
pub fn reply_face_value(state: &GameState, seat: PlayerId, damage: i32) -> f64 {
    if jackioh_engine::can_lose(state, seat) && damage >= effective_life(state, seat) {
        AI_EVAL.win
    } else {
        AI_REPLY.face_per_damage * f64::from(damage)
    }
}

/// The static trader's value of `attacker` hitting `defender`'s hero: `AI_EVAL.win` when
/// `face_is_lethal`, else `reply_face_value` of what the hit deals through the hero's Armor and caps
/// (`projected_hero_damage`). A Lifesteal attacker's hit that is not lethal adds the heal of what it
/// deals (`lifesteal_value`, the term `trade_value` adds for a hit on a unit, #3858). The reply's ranking
/// and the seat's own move order (`candidates.rs`, #2825) read it alike.
pub(crate) fn face_value(
    state: &GameState,
    attacker: &CardInstance,
    defender: PlayerId,
    face_is_lethal: bool,
) -> f64 {
    if face_is_lethal {
        return AI_EVAL.win;
    }
    let view = unit_view(state, attacker);
    let pierce = pierces(state, Some(attacker), None);
    let damage = subsystems::projected_hero_damage(state, defender, view.attack, pierce);
    let hit = reply_face_value(state, defender, damage);
    // §4.4 step 8: Lifesteal heals the attacker's hero by what the hit deals, on a hero as on a unit
    // (#3858). A lethal hit is the win, whatever it heals.
    if hit >= AI_EVAL.win || !has_keyword(&view.keywords, KeywordKind::Lifesteal) {
        return hit;
    }
    hit + lifesteal_value(state, attacker.controller, damage)
}

/// Ranked attacks worth making from `legal`. When `face_is_lethal`, every hero attack is `AI_EVAL.win` (#1634).
fn ranked_attacks(
    state: &GameState,
    seat: PlayerId,
    legal: &[ActionBody],
    face_is_lethal: bool,
    prices: &Prices,
) -> Vec<ActionBody> {
    let hero = format!("hero-{}", seat.as_str());
    let mut ranked: Vec<(ActionBody, f64)> = Vec::new();
    for action in legal.iter().cloned() {
        let ActionBody::Attack {
            attacker_id,
            target_id,
        } = &action
        else {
            continue;
        };
        let Some(attacker) = find_instance(state, attacker_id) else {
            continue;
        };
        let value = if *target_id == hero {
            face_value(state, attacker, seat, face_is_lethal)
        } else {
            trade_value(state, attacker, target_id, prices).value
        };
        if value > 0.0 {
            ranked.push((action, value));
        }
    }
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    ranked.into_iter().map(|(action, _)| action).collect()
}

/// Reduces one opponent action; `None` on panic or refusal. Counts node and charges surcharges (#1429).
fn reduce_for(
    state: &GameState,
    actor: PlayerId,
    action: &ActionBody,
    counter: &dyn NodeCounter,
) -> Option<GameState> {
    // `None` here already reads "skip this step" to the reply walk, so a surcharge the slice
    // cannot pay returns early instead of draining it node by node first.
    let surcharge = surcharge_nodes(state, actor, action);
    if counter.used() + surcharge > counter.limit() {
        return None;
    }
    for _ in 0..surcharge {
        if !counter.take() {
            return None;
        }
    }
    let action = Action::new(action.clone(), actor, format!("reply:{}", counter.used()));
    match catch_unwind(AssertUnwindSafe(|| reduce_unpiloted(state, &action))) {
        Ok(next) if next.error.is_none() => {
            charge_announced(counter, state, &action.body, &next);
            Some(next.state)
        }
        Ok(_) => None,
        Err(payload) => {
            note_sim_panic(counter, payload.as_ref());
            None
        }
    }
}

/// `known_play`'s, `visible_activation`'s, `deft_switch`'s and `defense_switch`'s answer.
pub(crate) enum Known {
    /// The counter refused a node.
    Cut,
    /// No such play is worth making.
    Nothing,
    Played(Box<GameState>),
}

/// R3696001: whether `opp` holds a hand card the seat can see (not in `hidden`) that this turn's
/// cleanup discards: Temporary with a count of 1 left (R637, R2179001).
fn doomed_known_card(state: &GameState, opp: PlayerId, hidden: &IndexSet<String>) -> bool {
    state.players[opp].hand.iter().any(|card| {
        !hidden.contains(&card.id)
            && temporary_number(&card_keywords(state, card)).is_some_and(|left| left <= 1)
    })
}

/// R3696001: whether a play of `instance_id` guesses a secret of `seat`'s it may not read: the card's
/// link names a secret `seat` owns that is not revealed (R862, R860).
fn guesses_secret(state: &GameState, seat: PlayerId, instance_id: &str) -> bool {
    find_instance(state, instance_id)
        .and_then(|card| card.memory.get(SECRET_KEY))
        .and_then(|link| link.as_str())
        .and_then(|id| secret_of(state, id))
        .is_some_and(|secret| secret.owner == seat && secret.revealed != Some(true))
}

/// R3696001: `plays` with each `guesses_secret` card's plays cut to one, in the first one's place,
/// drawn on `AI_GUESS_SEED`'s stream before any evaluation, so it never reads the secret (R185, R860).
fn one_guess_per_card(state: &GameState, seat: PlayerId, plays: Vec<ActionBody>) -> Vec<ActionBody> {
    let guessed = |play: &ActionBody| match play {
        ActionBody::Play { instance_id, .. } if guesses_secret(state, seat, instance_id) => {
            Some(instance_id.clone())
        }
        _ => None,
    };
    let mut settled: IndexSet<String> = IndexSet::new();
    let mut out: Vec<ActionBody> = Vec::with_capacity(plays.len());
    for play in &plays {
        let Some(card) = guessed(play) else {
            out.push(play.clone());
            continue;
        };
        if !settled.insert(card.clone()) {
            continue;
        }
        let guesses: Vec<&ActionBody> = plays
            .iter()
            .filter(|other| guessed(other).as_deref() == Some(card.as_str()))
            .collect();
        let stream = format!("{AI_GUESS_SEED}:{}:{card}", state.seed);
        let picked = Rng::new(&stream, 0).pick(&guesses).copied().unwrap_or(play);
        out.push(picked.clone());
    }
    out
}

/// Step 1: best play of a known hand card not in `hidden` if beating standing still. R3696001: a card
/// guessing an unrevealed secret of the seat's is tried with one guess (`one_guess_per_card`), and while
/// a known card would be discarded at the end of `opp`'s turn (`doomed_known_card`), every play and
/// standing still are compared after that end.
fn known_play(
    state: &GameState,
    seat: PlayerId,
    legal: &[ActionBody],
    hidden: &IndexSet<String>,
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Known {
    let opp = seat.opponent();
    let plays: Vec<ActionBody> = candidate_actions_in(state, opp, legal.to_vec(), prices)
        .into_iter()
        .filter(
            |action| matches!(action, ActionBody::Play { instance_id, .. } if !hidden.contains(instance_id)),
        )
        .collect();
    let plays: Vec<ActionBody> = one_guess_per_card(state, seat, plays)
        .into_iter()
        .take(AI_REPLY.known_plays.max(0) as usize)
        .collect();
    best_try(
        state,
        opp,
        &plays,
        doomed_known_card(state, opp, hidden),
        counter,
        prices,
    )
}

/// `opp`'s own evaluation of `state`, or with `at_close` of the position once it ends its turn there
/// (R3696001), `state`'s own when the end of turn is refused. `None` when the counter refused a node.
fn judged(
    state: &GameState,
    opp: PlayerId,
    at_close: bool,
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Option<f64> {
    let closed = if at_close {
        simulate(state, opp, &ActionBody::EndTurn, counter, prices)?.ok()
    } else {
        None
    };
    Some(evaluate_priced(
        closed.as_ref().unwrap_or(state),
        opp,
        NextSwing::Enemy,
        &AI_EVAL,
        prices,
    ))
}

/// The best of `tries` for `opp` by its own evaluation: one that wins the game at once, else the one
/// whose result beats standing still by the most (ties keep the list's order). With `at_close`, each
/// try and standing still are judged after `opp` ends its turn there (`judged`, R3696001).
fn best_try(
    state: &GameState,
    opp: PlayerId,
    tries: &[ActionBody],
    at_close: bool,
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Known {
    if tries.is_empty() {
        return Known::Nothing;
    }
    let mut best: Option<GameState> = None;
    let Some(mut best_value) = judged(state, opp, at_close, counter, prices) else {
        return Known::Cut;
    };
    for action in tries {
        let next = match simulate(state, opp, action, counter, prices) {
            None => return Known::Cut,
            Some(Err(_)) => continue,
            Some(Ok(next)) => next,
        };
        if next
            .result
            .is_some_and(|result| result.winner.player() == Some(opp))
        {
            return Known::Played(Box::new(next));
        }
        let Some(value) = judged(&next, opp, at_close, counter, prices) else {
            return Known::Cut;
        };
        // Strictly greater, so ties keep move order.
        if value > best_value {
            best_value = value;
            best = Some(next);
        }
    }
    match best {
        Some(state) => Known::Played(Box::new(state)),
        None => Known::Nothing,
    }
}

/// Where a visible activation is aimed, in the order a reply step tries them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Aim {
    /// At the seat's hero, where a lethal one is.
    SeatHero,
    /// At a card the seat controls: the opponent's activated removal.
    SeatCard,
    /// Anything else, an activation with no target among them.
    Other,
}

/// Where `action` aims, read off the targets it carries.
fn aim_of(state: &GameState, seat: PlayerId, action: &ActionBody) -> Aim {
    let targets: &[Selection] = match action {
        ActionBody::Activate { targets, .. } | ActionBody::ActivatePower { targets, .. } => {
            targets.as_deref().unwrap_or(&[])
        }
        _ => &[],
    };
    if targets
        .iter()
        .any(|target| matches!(target, Selection::Hero { player } if *player == seat))
    {
        return Aim::SeatHero;
    }
    let at_seat_card = targets.iter().any(|target| {
        matches!(target, Selection::Instance { instance_id }
            if find_instance(state, instance_id).is_some_and(|card| card.controller == seat))
    });
    if at_seat_card { Aim::SeatCard } else { Aim::Other }
}

/// Whether the seat can see the source of `action` (R752): face-up on the field,
/// not a face-down Reveal (R783006) or determinized unseen card.
fn visible_source(state: &GameState, hidden: &IndexSet<String>, action: &ActionBody) -> bool {
    let (ActionBody::Activate { instance_id, .. } | ActionBody::ActivatePower { instance_id, .. }) = action
    else {
        return false;
    };
    !hidden.contains(instance_id)
        && find_instance(state, instance_id).is_some_and(|card| subsystems::is_acting_on_field(state, card))
}

/// Step 2: the best of the opponent's activations whose source the seat can see, by the opponent's own
/// evaluation, when it beats standing still. At most AI_REPLY.visible_activations are tried, ordered by
/// `aim_of`: `legal_actions` lists the activating side's own targets first, so the seat's hero would
/// otherwise come last. R2564000: the root's activation probes try each unit's uses through it too.
pub(crate) fn visible_activation(
    state: &GameState,
    seat: PlayerId,
    legal: &[ActionBody],
    hidden: &IndexSet<String>,
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Known {
    let mut tries: Vec<ActionBody> = legal
        .iter()
        .filter(|action| visible_source(state, hidden, action))
        .cloned()
        .collect();
    // Stable, so each aim keeps legal_actions order.
    tries.sort_by_key(|action| aim_of(state, seat, action));
    tries.truncate(AI_REPLY.visible_activations.max(0) as usize);
    best_try(state, seat.opponent(), &tries, false, counter, prices)
}

/// Step 3, R49: the opponent's Deft units in Defense Position may switch to Attack and still attack.
/// Each one whose switch `legal` lists and whose attack is unspent (`deft_ready_switch`), in
/// legal_actions order, is switched on trial, one node; the first whose best-ranked attack after the
/// switch is one of its own keeps the switch. Otherwise every such unit stays in Defense.
fn deft_switch(
    state: &GameState,
    seat: PlayerId,
    legal: &[ActionBody],
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Known {
    let opp = seat.opponent();
    for action in legal {
        let ActionBody::SwitchPosition { instance_id } = action else {
            continue;
        };
        if !deft_ready_switch(state, instance_id) {
            continue;
        }
        if !counter.take() {
            return Known::Cut;
        }
        let Some(switched) = reduce_for(state, opp, action, counter) else {
            continue;
        };
        let after = legal_actions(&switched, opp);
        // The trial ranks as before #1634: one hit at a time.
        let best = ranked_attacks(&switched, seat, &after, false, prices)
            .into_iter()
            .next();
        if matches!(&best, Some(ActionBody::Attack { attacker_id, .. }) if attacker_id == instance_id) {
            return Known::Played(Box::new(switched));
        }
    }
    Known::Nothing
}

/// Step 4 (#1696): trials switching up to `AI_REPLY.defense_switches` units from Attack to
/// Defense when no attack is worth making (§4.1, R49).
fn defense_switch(
    state: &GameState,
    seat: PlayerId,
    legal: &[ActionBody],
    counter: &dyn NodeCounter,
    prices: &Prices,
) -> Known {
    let tries: Vec<ActionBody> = legal
        .iter()
        .filter(|action| {
            matches!(action, ActionBody::SwitchPosition { instance_id }
                if find_instance(state, instance_id)
                    .is_some_and(|unit| unit_view(state, unit).position == Position::Atk))
        })
        .take(AI_REPLY.defense_switches.max(0) as usize)
        .cloned()
        .collect();
    best_try(state, seat.opponent(), &tries, false, counter, prices)
}

/// The ids of the cards the opponent holds unseen: its hand and its library, as sampled.
pub fn hidden_card_ids(state: &GameState, seat: PlayerId) -> IndexSet<String> {
    let side = &state.players[seat.opponent()];
    side.hand
        .iter()
        .chain(side.library.iter())
        .map(|card| card.id.clone())
        .collect()
}

/// Simulates the opponent's reply turn according to the reply rules until the seat's next main phase
/// or prompt, or game end. Prompts get `best_auto_answer`. Returns `None` when the node counter exhausts.
pub fn simulate_reply(
    state: &GameState,
    seat: PlayerId,
    counter: &dyn NodeCounter,
    hidden: &IndexSet<String>,
    prices: &Prices,
) -> Option<GameState> {
    simulate_reply_with(state, seat, counter, hidden, AI_REPLY.open_lethal, prices)
}

/// `simulate_reply` with the open-lethal switch given (`AI_REPLY.open_lethal`, #1634), for tests and
/// screens.
pub fn simulate_reply_with(
    state: &GameState,
    seat: PlayerId,
    counter: &dyn NodeCounter,
    hidden: &IndexSet<String>,
    open_lethal: bool,
    prices: &Prices,
) -> Option<GameState> {
    let opp = seat.opponent();
    let mut current = state.clone();
    for _step in 0..AI_REPLY.max_steps {
        if current.result.is_some() {
            return Some(current);
        }
        if let Some(pending) = &current.pending {
            // The seat's own prompt at its next turn start is where its next decision begins.
            if pending.player_id == seat && current.active == seat {
                return Some(current);
            }
            let answerer = pending.player_id;
            let Ok(next) =
                best_auto_answer(&current, answerer, AnswersTried::Drawn, counter, "reply", prices)?
            else {
                return Some(current);
            };
            current = next;
            continue;
        }
        if current.active != opp {
            return Some(current);
        }

        // One legal_actions serves the known-play try (its ordered candidates), the visible
        // activations, the Deft switches, the static trader's attack list and the switches to Defense.
        let legal = legal_actions(&current, opp);
        match known_play(&current, seat, &legal, hidden, counter, prices) {
            Known::Cut => return None,
            Known::Played(played) => {
                current = *played;
                continue;
            }
            Known::Nothing => {}
        }
        match visible_activation(&current, seat, &legal, hidden, counter, prices) {
            Known::Cut => return None,
            Known::Played(played) => {
                current = *played;
                continue;
            }
            Known::Nothing => {}
        }
        match deft_switch(&current, seat, &legal, counter, prices) {
            Known::Cut => return None,
            Known::Played(switched) => {
                current = *switched;
                continue;
            }
            Known::Nothing => {}
        }

        // #1634: one board read per step: do the attacks left reach the face together?
        let face_is_lethal = open_lethal && ready_gap(&current, opp) <= 0.0;
        let attack = ranked_attacks(&current, seat, &legal, face_is_lethal, prices)
            .into_iter()
            .next();
        // #1696: no attack is worth making, so before it ends its turn the opponent switches a unit
        // its evaluation would rather have in Defense Position.
        if attack.is_none() {
            match defense_switch(&current, seat, &legal, counter, prices) {
                Known::Cut => return None,
                Known::Played(switched) => {
                    current = *switched;
                    continue;
                }
                Known::Nothing => {}
            }
        }
        if !counter.take() {
            return None;
        }
        let step = attack.clone().unwrap_or(ActionBody::EndTurn);
        if let Some(next) = reduce_for(&current, opp, &step, counter) {
            current = next;
            continue;
        }
        // A refused attack ends the turn instead; a refused endTurn ends the reply where it stands.
        if attack.is_none() || !counter.take() {
            return if attack.is_none() { Some(current) } else { None };
        }
        let Some(ended) = reduce_for(&current, opp, &ActionBody::EndTurn, counter) else {
            return Some(current);
        };
        current = ended;
    }
    Some(current)
}

/// Scores a closed line after opponent's reply, evaluated at next decision (`coming_mana`, #1995).
/// `None` when the counter ran out.
pub fn reply_score(
    end: &GameState,
    seat: PlayerId,
    root_turn: i32,
    counter: &dyn NodeCounter,
    hidden: &IndexSet<String>,
    prices: &Prices,
) -> Option<f64> {
    let status = line_status(end, seat, root_turn);
    if status == LineStatus::Over {
        return Some(static_score(end, seat, root_turn, prices));
    }
    // A line that stopped on the seat's own prompt keeps its static score, from the evaluation that
    // scores the finalists (#1010).
    if status == LineStatus::Open {
        return Some(finalist_score(end, seat, NextSwing::Enemy, prices));
    }
    let unspent = if status == LineStatus::Passed {
        end.players[seat].turn_log.unspent_at_end.unwrap_or(0)
    } else {
        0
    };
    let after = simulate_reply(end, seat, counter, hidden, prices)?;
    let first = replied_score(end, &after, seat, unspent, prices);
    // #1995: a countdown still waiting at the seat's next decision is scored a second time, fired.
    let Some((fired, wait)) = countdowns_due(end, &after, seat) else {
        return Some(first);
    };
    let refired = simulate_reply(&fired, seat, counter, hidden, prices)?;
    let second = replied_score(end, &refired, seat, unspent, prices);
    let share = (0..wait).fold(1.0, |share, _| share * AI_REPLY.countdown_share);
    Some(first + share * (second - first))
}

/// A passed line's score at `after`, where the reply from `end` stopped: the finalists' evaluation
/// with whoever swings next, less the crystals the line left `unspent`, plus what that Float brought
/// its Float-discount cards at `end` (R3695000).
fn replied_score(end: &GameState, after: &GameState, seat: PlayerId, unspent: i32, prices: &Prices) -> f64 {
    let next = if after.active == seat {
        NextSwing::Seat
    } else {
        NextSwing::Enemy
    };
    // R844, §2.3: the coming refreshes count as the line's end priced them (`coming_mana`): what the
    // reply's refreshes spent (lost, or moved by a rider) still counts, and what the reply set does not.
    let spent = if after.result.is_none() {
        coming_mana(end, seat, &AI_EVAL) - coming_mana(after, seat, &AI_EVAL)
    } else {
        0.0
    };
    finalist_score(after, seat, next, prices) + spent - AI_EVAL.unspent_mana * f64::from(unspent)
        + float_credit(end, seat, unspent, &AI_EVAL)
}

/// #1995: advances countdowns waiting at `after` (public badges, R1181000, R169, R185) to seat's
/// next turn start; calculates wait turns for `AI_REPLY.countdown_share`. `None` when none waits.
fn countdowns_due(end: &GameState, after: &GameState, seat: PlayerId) -> Option<(GameState, i32)> {
    if !after.delayed.iter().any(has_countdown) {
        return None;
    }
    let mut fired = end.clone();
    let mut wait: Option<i32> = None;
    for entry in fired.delayed.iter_mut().filter(|entry| has_countdown(entry)) {
        let Some(due) = after
            .delayed
            .iter()
            .find(|waiting| waiting.id == entry.id)
            .and_then(|waiting| waiting.due_own_turn)
        else {
            continue;
        };
        let past_next = (due - after.players[entry.at.player].turns_started - 1).max(0);
        wait = Some(wait.map_or(past_next, |farthest| farthest.max(past_next)));
        entry.at = DelayedAt {
            phase: Phase::Start,
            player: seat,
        };
        entry.due_own_turn = Some(end.players[seat].turns_started + 1);
        entry.not_before = None;
    }
    wait.map(|wait| (fired, wait))
}

#[cfg(test)]
mod tests {
    use jackioh_engine::config::HERO_HEALTH;
    use jackioh_engine::testkit::{Keyword, Scenario, Value, json, scenario};
    use jackioh_engine::{ModifierExpiry, ModifierKind, PlayerModifier};

    use super::*;
    use crate::simulate::create_node_counter;

    /// C+ #13 Mommy Barker, whose Radiant face is a 4/4 Reborn.
    const BARKER: &str = "classicplus-013";
    /// #77 Professor Curvature, whose Radiant face is a vanilla 6/6.
    const CURVATURE: &str = "core-077";
    /// #8 Mr. Vanilla, a vanilla 4/4.
    const VANILLA: &str = "core-008";
    /// #67 Zoomerbin Oomen, a 1/2.
    const OOMEN: &str = "core-067";
    /// #20 Pointmaster, a 7/1 First Strike.
    const POINTMASTER: &str = "core-020";
    /// M+ #7 Life Mage, whose Radiant face makes healing its controller's hero twice as effective
    /// (R1180003).
    const LIFE_MAGE: &str = "meditativeplus-007";
    /// C+ #22 Blood Moon, whose Radiant face is a face-up Field Trap converting enemy heals.
    const MOON: &str = "classicplus-022";
    /// M #22 Mind Games: a choice kept secret, its reward at the caster's next turn start, a Fortify
    /// Mind handed to the other side.
    const MIND_GAMES: &str = "meditative-022";
    /// M #22.1 Fortify Mind: Temporary, a guess at the Mind Games; discarded, all three penalties.
    const FORTIFY: &str = "meditative-022-1";
    /// #5 Stockpile, left in p1's hand so that its turn does not end by itself after the cast.
    const STOCKPILE: &str = "core-005";
    /// Mind Games' three choices, Fortify Mind's three guesses.
    const GUESSES: [&str; 3] = ["greed", "attack", "defend"];

    /// p1's `attacker` facing p2's Radiant Mommy Barker.
    fn facing_barker(attacker: Value) -> Scenario {
        jackioh_cards::register_all();
        scenario(json!({
            "p1": { "field": [attacker] },
            "p2": { "field": [{ "def": BARKER, "radiant": true }] },
        }))
    }

    /// The worth of Mommy Barker's Reborn body (§4.5 step 4): its Radiant 4/4 at 1 health, no Reborn.
    fn barker_return() -> f64 {
        4.0 * AI_EVAL.attack + AI_EVAL.health
    }

    /// `setup`'s scenario with `keywords` granted to its Mr. Vanilla.
    fn vanilla_with(setup: Value, keywords: &[Keyword]) -> Scenario {
        jackioh_cards::register_all();
        let mut s = scenario(setup);
        s.card_mut(VANILLA).granted_keywords.extend_from_slice(keywords);
        s
    }

    /// What healing a 1v1 hero at `life` by `amount` is worth on `hero_value`'s curve.
    fn healed(life: i32, amount: i32) -> f64 {
        let start = f64::from(HERO_HEALTH);
        AI_EVAL.hero_health * ((f64::from(life + amount) * start).sqrt() - (f64::from(life) * start).sqrt())
    }

    #[test]
    fn a_kill_of_a_reborn_unit_is_worth_its_worth_less_its_one_health_return() {
        let s = facing_barker(json!({ "def": CURVATURE, "radiant": true }));
        let barker = s.card(BARKER);
        let trade = trade_value(s.state(), s.card(CURVATURE), &barker.id, Prices::NONE);
        assert!(trade.kills);
        let worth = unit_worth(s.state(), barker, &AI_EVAL, Prices::NONE);
        assert!(
            (trade.value - (worth - barker_return())).abs() < 1e-9,
            "{}",
            trade.value
        );
    }

    #[test]
    fn a_kill_of_a_unit_whose_reborn_is_spent_is_worth_its_whole_worth() {
        let mut s = facing_barker(json!({ "def": CURVATURE, "radiant": true }));
        s.card_mut(BARKER).reborn_spent = Some(true);
        let barker = s.card(BARKER);
        let trade = trade_value(s.state(), s.card(CURVATURE), &barker.id, Prices::NONE);
        assert!(trade.kills);
        let worth = unit_worth(s.state(), barker, &AI_EVAL, Prices::NONE);
        assert!((trade.value - worth).abs() < 1e-9, "{}", trade.value);
        assert!(
            (worth - (4.0 * AI_EVAL.attack + 4.0 * AI_EVAL.health)).abs() < 1e-9,
            "{worth}"
        );
    }

    #[test]
    fn a_dying_reborn_attacker_is_charged_its_worth_less_its_return() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "p1": { "field": [{ "def": BARKER, "radiant": true }] },
            "p2": { "field": [VANILLA] },
        }));
        let barker = s.card(BARKER);
        let vanilla = s.card(VANILLA);
        let trade = trade_value(s.state(), barker, &vanilla.id, Prices::NONE);
        assert!(trade.kills);
        let expected = unit_worth(s.state(), vanilla, &AI_EVAL, Prices::NONE)
            - (unit_worth(s.state(), barker, &AI_EVAL, Prices::NONE) - barker_return());
        assert!((trade.value - expected).abs() < 1e-9, "{}", trade.value);
    }

    #[test]
    fn a_trample_kill_adds_the_excess_it_lands_on_the_hero() {
        let s = vanilla_with(
            json!({ "p1": { "field": [VANILLA] }, "p2": { "field": [OOMEN] } }),
            &[Keyword::Trample],
        );
        let trade = trade_value(s.state(), s.card(VANILLA), &s.card(OOMEN).id, Prices::NONE);
        assert!(trade.kills);
        let kill = kill_worth(s.state(), s.card(OOMEN), &AI_EVAL, Prices::NONE);
        // §4.4 step 9: 4 on a 2-health Oomen sends 2 on to p2's hero.
        let expected = kill + AI_REPLY.face_per_damage * 2.0;
        assert!((trade.value - expected).abs() < 1e-9, "{}", trade.value);
        assert!(trade.value > kill, "{}", trade.value);
    }

    #[test]
    fn an_indestructible_target_lets_no_trample_through() {
        let mut s = vanilla_with(
            json!({ "p1": { "field": [VANILLA] }, "p2": { "field": [OOMEN] } }),
            &[Keyword::Trample],
        );
        s.card_mut(OOMEN).granted_keywords.push(Keyword::Indestructible);
        let trade = trade_value(s.state(), s.card(VANILLA), &s.card(OOMEN).id, Prices::NONE);
        // §4.4 step 4: the Oomen takes no damage, so there is no step 5 and no excess for step 9.
        assert!(!trade.kills);
        assert!(trade.value.abs() < 1e-9, "{}", trade.value);
    }

    #[test]
    fn a_lifesteal_hit_is_worth_the_heal_of_all_it_deals() {
        let s = vanilla_with(
            json!({ "p1": { "health": 10, "field": [VANILLA] }, "p2": { "field": [OOMEN] } }),
            &[Keyword::Lifesteal],
        );
        let trade = trade_value(s.state(), s.card(VANILLA), &s.card(OOMEN).id, Prices::NONE);
        assert!(trade.kills);
        // §4.4 step 5: all 4 dealt heal, not only the Oomen's 2 health.
        let expected = kill_worth(s.state(), s.card(OOMEN), &AI_EVAL, Prices::NONE) + healed(10, 4);
        assert!((trade.value - expected).abs() < 1e-9, "{}", trade.value);
    }

    #[test]
    fn a_trample_lifesteal_hit_heals_what_lands_on_the_unit_and_on_the_hero() {
        let s = vanilla_with(
            json!({
                "p1": { "health": 10, "field": [VANILLA] },
                "p2": { "armor": 1, "field": [OOMEN] },
            }),
            &[Keyword::Trample, Keyword::Lifesteal],
        );
        let trade = trade_value(s.state(), s.card(VANILLA), &s.card(OOMEN).id, Prices::NONE);
        assert!(trade.kills);
        // 2 land on the Oomen and 2 trample on, of which p2's Armor takes 1: 3 heal.
        let expected = kill_worth(s.state(), s.card(OOMEN), &AI_EVAL, Prices::NONE)
            + AI_REPLY.face_per_damage * 1.0
            + healed(10, 3);
        assert!((trade.value - expected).abs() < 1e-9, "{}", trade.value);
    }

    #[test]
    fn a_healing_multiplier_doubles_the_lifesteal_heal_it_prices() {
        let s = vanilla_with(
            json!({
                "p1": { "health": 10, "field": [VANILLA, { "def": LIFE_MAGE, "radiant": true }] },
                "p2": { "field": [OOMEN] },
            }),
            &[Keyword::Lifesteal],
        );
        let trade = trade_value(s.state(), s.card(VANILLA), &s.card(OOMEN).id, Prices::NONE);
        assert!(trade.kills);
        // R1180003: the Radiant Life Mage doubles the 4 healed.
        let expected = kill_worth(s.state(), s.card(OOMEN), &AI_EVAL, Prices::NONE) + healed(10, 8);
        assert!((trade.value - expected).abs() < 1e-9, "{}", trade.value);
    }

    #[test]
    fn a_lifesteal_hit_under_a_face_up_blood_moon_is_worth_self_damage() {
        let s = vanilla_with(
            json!({
                "p1": { "health": 10, "field": [VANILLA] },
                "p2": { "field": [OOMEN], "backrow": [{ "def": MOON, "radiant": true, "faceUp": true }] },
            }),
            &[Keyword::Lifesteal],
        );
        let trade = trade_value(s.state(), s.card(VANILLA), &s.card(OOMEN).id, Prices::NONE);
        assert!(trade.kills);
        // E8, R413: the 4 healed become 4 Pierce damage on p1's own hero instead.
        let expected = kill_worth(s.state(), s.card(OOMEN), &AI_EVAL, Prices::NONE)
            - reply_face_value(s.state(), PlayerId::P1, 4);
        assert!((trade.value - expected).abs() < 1e-9, "{}", trade.value);
        assert!(trade.value < 0.0, "{}", trade.value);
    }

    #[test]
    fn a_lifesteal_hit_under_a_live_heal_to_damage_modifier_is_worth_self_damage() {
        let mut s = vanilla_with(
            json!({ "p1": { "health": 10, "field": [VANILLA] }, "p2": { "field": [OOMEN] } }),
            &[Keyword::Lifesteal],
        );
        // A fired base-face Moon leaves a rest-of-turn converter on the healed hero's opponent.
        let turn = s.state().turn;
        s.state_mut().players[PlayerId::P2].mods.push(PlayerModifier {
            id: "moon".into(),
            expiry: ModifierExpiry::ThisTurn { turn },
            kind: ModifierKind::HealToDamage {
                converter_id: "moon".into(),
            },
        });
        let trade = trade_value(s.state(), s.card(VANILLA), &s.card(OOMEN).id, Prices::NONE);
        assert!(trade.kills);
        let expected = kill_worth(s.state(), s.card(OOMEN), &AI_EVAL, Prices::NONE)
            - reply_face_value(s.state(), PlayerId::P1, 4);
        assert!((trade.value - expected).abs() < 1e-9, "{}", trade.value);
        assert!(trade.value < 0.0, "{}", trade.value);
    }

    #[test]
    fn a_hidden_blood_moon_leaves_the_lifesteal_price_unchanged() {
        let s = vanilla_with(
            json!({
                "p1": { "health": 10, "field": [VANILLA] },
                "p2": { "field": [OOMEN], "backrow": [{ "def": MOON, "radiant": true, "faceUp": false }] },
            }),
            &[Keyword::Lifesteal],
        );
        let trade = trade_value(s.state(), s.card(VANILLA), &s.card(OOMEN).id, Prices::NONE);
        assert!(trade.kills);
        // R33, R1487000: a face-down card's text sleeps, so the trap stays unmodeled.
        let expected = kill_worth(s.state(), s.card(OOMEN), &AI_EVAL, Prices::NONE) + healed(10, 4);
        assert!((trade.value - expected).abs() < 1e-9, "{}", trade.value);
    }

    #[test]
    fn an_attacker_a_first_strike_defender_destroys_first_heals_nothing() {
        let s = vanilla_with(
            json!({ "p1": { "health": 10, "field": [VANILLA] }, "p2": { "field": [POINTMASTER] } }),
            &[Keyword::Lifesteal],
        );
        let trade = trade_value(s.state(), s.card(VANILLA), &s.card(POINTMASTER).id, Prices::NONE);
        // §4.3 step 1: the Pointmaster's 7 destroys the Vanilla before it strikes.
        assert!(!trade.kills);
        let expected =
            AI_REPLY.chip_per_damage * 1.0 - kill_worth(s.state(), s.card(VANILLA), &AI_EVAL, Prices::NONE);
        assert!((trade.value - expected).abs() < 1e-9, "{}", trade.value);
    }

    /// p1's Mr. Vanilla's `face_value` on p2's hero in `setup`, with Lifesteal and without.
    fn face_twins(setup: Value, face_is_lethal: bool) -> (Scenario, f64, f64) {
        let steal = vanilla_with(setup.clone(), &[Keyword::Lifesteal]);
        let plain = vanilla_with(setup, &[]);
        let steal_value = face_value(steal.state(), steal.card(VANILLA), PlayerId::P2, face_is_lethal);
        let plain_value = face_value(plain.state(), plain.card(VANILLA), PlayerId::P2, face_is_lethal);
        (steal, steal_value, plain_value)
    }

    #[test]
    fn a_lifesteal_hit_on_the_hero_is_worth_the_heal_of_all_it_deals() {
        let (s, steal, plain) = face_twins(json!({ "p1": { "health": 10, "field": [VANILLA] } }), false);
        assert!((plain - AI_REPLY.face_per_damage * 4.0).abs() < 1e-9, "{plain}");
        let heal = heal_worth(s.state(), PlayerId::P1, 4, &AI_EVAL);
        assert!(heal > 0.0 && (heal - healed(10, 4)).abs() < 1e-9, "{heal}");
        assert!((steal - plain - heal).abs() < 1e-9, "{steal} {plain}");
    }

    #[test]
    fn a_lifesteal_hit_on_the_hero_heals_what_the_heros_armor_leaves() {
        let (_, steal, plain) = face_twins(
            json!({ "p1": { "health": 10, "field": [VANILLA] }, "p2": { "armor": 1 } }),
            false,
        );
        assert!((plain - AI_REPLY.face_per_damage * 3.0).abs() < 1e-9, "{plain}");
        assert!((steal - plain - healed(10, 3)).abs() < 1e-9, "{steal} {plain}");
    }

    #[test]
    fn a_lifesteal_hit_on_the_hero_under_a_face_up_blood_moon_is_worth_self_damage() {
        let (s, steal, plain) = face_twins(
            json!({
                "p1": { "health": 10, "field": [VANILLA] },
                "p2": { "backrow": [{ "def": MOON, "radiant": true, "faceUp": true }] },
            }),
            false,
        );
        assert!(steal < plain, "{steal} {plain}");
        let burn = reply_face_value(s.state(), PlayerId::P1, 4);
        assert!((plain - steal - burn).abs() < 1e-9, "{steal} {plain}");
    }

    #[test]
    fn a_lethal_lifesteal_hit_on_the_hero_scores_the_win_and_no_more() {
        let (_, steal, _) = face_twins(
            json!({ "p1": { "health": 10, "field": [VANILLA] }, "p2": { "health": 4 } }),
            false,
        );
        assert_eq!(steal.to_bits(), AI_EVAL.win.to_bits(), "{steal}");
        let (_, steal, _) = face_twins(json!({ "p1": { "health": 10, "field": [VANILLA] } }), true);
        assert_eq!(steal.to_bits(), AI_EVAL.win.to_bits(), "{steal}");
    }

    /// p2's reply attacks at `health`, best first: its Lifesteal Mr. Vanilla and its Radiant Professor
    /// Curvature, a 6/6, facing p1's Zoomerbin Oomen.
    fn reply_attack_order(health: i32) -> (Scenario, Vec<ActionBody>) {
        let s = vanilla_with(
            json!({
                "active": "p2",
                "p1": { "field": [OOMEN] },
                "p2": { "health": health, "field": [VANILLA, { "def": CURVATURE, "radiant": true }] },
            }),
            &[Keyword::Lifesteal],
        );
        let ranked = ranked_attacks(
            s.state(),
            PlayerId::P1,
            &legal_actions(s.state(), PlayerId::P2),
            false,
            Prices::NONE,
        );
        (s, ranked)
    }

    /// `s`'s attack of `attacker` on `target_id`.
    fn attack_of(s: &Scenario, attacker: &str, target_id: &str) -> ActionBody {
        ActionBody::Attack {
            attacker_id: s.card(attacker).id.clone(),
            target_id: target_id.to_owned(),
        }
    }

    #[test]
    fn at_low_life_both_lifesteal_hits_outrank_a_plain_face_hit() {
        // At 3 life the heal of 4 is worth about 5.00 wherever the Vanilla hits: on the hero 4 + 5.00,
        // on the Oomen its kill's 3.20 + 5.00, both above the Curvature's 6 on the face (#1820, #3858).
        let (s, ranked) = reply_attack_order(3);
        let oomen = s.card(OOMEN).id.clone();
        assert_eq!(
            ranked,
            vec![
                attack_of(&s, VANILLA, "hero-p1"),
                attack_of(&s, VANILLA, &oomen),
                attack_of(&s, CURVATURE, "hero-p1"),
                attack_of(&s, CURVATURE, &oomen),
            ]
        );
        // At full life the same heal is worth about 1.94, so the Curvature's 6 on the face comes first.
        let (s, ranked) = reply_attack_order(HERO_HEALTH);
        let oomen = s.card(OOMEN).id.clone();
        assert_eq!(
            ranked,
            vec![
                attack_of(&s, CURVATURE, "hero-p1"),
                attack_of(&s, VANILLA, "hero-p1"),
                attack_of(&s, VANILLA, &oomen),
                attack_of(&s, CURVATURE, &oomen),
            ]
        );
    }

    /// p2's turn after p1 cast Mind Games keeping `kept` secret and ended its turn, the Fortify Mind it
    /// handed p2 in p2's hand; with the ids p2 held unseen before the cast (`hidden`, so the Fortify
    /// Mind, which the AI watched arrive, is known).
    fn mind_games_cast(kept: &str) -> (Scenario, IndexSet<String>) {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "reply-fortify",
            "p1": { "hand": [MIND_GAMES, STOCKPILE], "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
            "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
        }));
        let hidden = hidden_card_ids(s.state(), PlayerId::P1);
        s.play(MIND_GAMES, json!({ "modes": [kept] })).end_turn();
        assert_eq!(s.state().active, PlayerId::P2, "{kept}: p2's turn");
        (s, hidden)
    }

    /// p2's candidate plays of its Fortify Mind, one per guess.
    fn fortify_plays(s: &Scenario) -> Vec<ActionBody> {
        let fortify = &s.card(FORTIFY).id;
        candidate_actions_in(
            s.state(),
            PlayerId::P2,
            legal_actions(s.state(), PlayerId::P2),
            Prices::NONE,
        )
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if instance_id == fortify))
        .collect()
    }

    /// Whether p1's Mind Games reward still waits.
    fn holds_reward(state: &GameState) -> bool {
        state
            .delayed
            .iter()
            .any(|entry| entry.owner == PlayerId::P1 && entry.resume.def_id == MIND_GAMES)
    }

    /// Before R3696001 the reply compared each guess with the position now, where no guess pays, and
    /// kept the card into cleanup's three penalties; judged after its turn ends, it plays it.
    #[test]
    fn r3696001_the_opponent_plays_the_fortify_mind_it_would_otherwise_discard() {
        for kept in GUESSES {
            let (s, hidden) = mind_games_cast(kept);
            let state = s.state();
            let guess = one_guess_per_card(state, PlayerId::P1, fortify_plays(&s));
            let now = best_try(
                state,
                PlayerId::P2,
                &guess,
                false,
                &create_node_counter(200, None),
                Prices::NONE,
            );
            assert!(
                matches!(now, Known::Nothing),
                "{kept}: judged now, the card is kept"
            );
            let legal = legal_actions(state, PlayerId::P2);
            let Known::Played(next) = known_play(
                state,
                PlayerId::P1,
                &legal,
                &hidden,
                &create_node_counter(200, None),
                Prices::NONE,
            ) else {
                panic!("{kept}: the Fortify Mind is played");
            };
            assert_eq!(next.active, PlayerId::P2, "{kept}");
            let fortify = &s.card(FORTIFY).id;
            assert!(
                next.players[PlayerId::P2]
                    .hand
                    .iter()
                    .all(|card| card.id != *fortify),
                "{kept}: the Fortify Mind left p2's hand"
            );
        }
    }

    /// The one guess tried is drawn before anything is weighed, so it is the same whichever choice
    /// the AI kept: it wins against exactly one of them (R864), and only there is the reward gone.
    #[test]
    fn r3696001_the_guess_is_the_same_whichever_choice_the_ai_kept_secret() {
        let mut drawn: Vec<ActionBody> = Vec::new();
        let mut cancelled = 0;
        for kept in GUESSES {
            let (s, hidden) = mind_games_cast(kept);
            let state = s.state();
            let plays = fortify_plays(&s);
            assert_eq!(plays.len(), 3, "{kept}: one play per guess");
            assert!(holds_reward(state), "{kept}: the reward waits");
            let one = one_guess_per_card(state, PlayerId::P1, plays);
            assert_eq!(one.len(), 1, "{kept}: one guess is tried");
            drawn.push(one[0].clone());
            let legal = legal_actions(state, PlayerId::P2);
            let Known::Played(next) = known_play(
                state,
                PlayerId::P1,
                &legal,
                &hidden,
                &create_node_counter(200, None),
                Prices::NONE,
            ) else {
                panic!("{kept}: the Fortify Mind is played");
            };
            if !holds_reward(&next) {
                cancelled += 1;
            }
        }
        assert!(drawn.iter().all(|play| *play == drawn[0]), "{drawn:?}");
        assert_eq!(cancelled, 1, "the one guess wins against one choice");
    }

    /// A revealed secret is no guess, so every play of the card is tried; and the guess is the
    /// world's: other seeds draw other guesses.
    #[test]
    fn r3696001_a_revealed_secret_keeps_every_guess_and_other_worlds_draw_other_guesses() {
        let (mut s, _) = mind_games_cast("greed");
        let plays = fortify_plays(&s);
        let mut revealed = s.state().clone();
        for secret in revealed.secrets.iter_mut().flatten() {
            secret.revealed = Some(true);
        }
        assert_eq!(one_guess_per_card(&revealed, PlayerId::P1, plays.clone()), plays);
        let mut guesses: IndexSet<String> = IndexSet::new();
        for world in 0..30 {
            s.state_mut().seed = format!("w{world}");
            let one = one_guess_per_card(s.state(), PlayerId::P1, plays.clone());
            let [
                ActionBody::Play {
                    modes: Some(modes), ..
                },
            ] = one.as_slice()
            else {
                panic!("one guess with its mode: {one:?}");
            };
            guesses.insert(modes.join(","));
        }
        assert!(guesses.len() > 1, "{guesses:?}");
    }
}
