//! A ladder bot's emote persona (SPEC §9.9, R645, R1344; #638): the server's port of
//! `apps/web/src/practice/personas.ts`, which stays in the web client for the practice AI. The
//! tables are `crate::config`'s (`AI_PERSONAS`, `AI_EMOTE`); this module is the rolls, the trigger
//! detection, the caps and the rate limit, draw for draw as TS makes them, so the same scripted rng
//! gives the same emotes.
//!
//! R645: cosmetic only. A persona reads nothing but the bot seat's redacted `PlayerView`, the events
//! that view carries and its own injected rng (never `state.rng`, a determinization or the move
//! search's), and it returns emote intents a caller may send or drop. So it can never touch game
//! state, the action log, a replay or a search. R1344: every roll picks only from the bot seat's
//! dealt hand, or sends nothing.
//!
//! One documented difference from TS: R643's rate limit (`emote_gate`) takes whole milliseconds, so
//! the send instant `now + delay_ms` is rounded to the nearest millisecond before the gate judges it
//! and before it is recorded; TS judges and records the unrounded float.
//!
//! R639000: a ladder bot is dealt its persona by `BOT_PERSONA_WEIGHTS` (`pick_bot_persona`) and waits
//! out its emotes inside `BOT_EMOTE_DELAY_FIT` and `BOT_EMOTE_REPLY_DELAY_FIT`
//! (`EmotePersona::with_delays`), the rows `jackioh-server timing-fit` refits from people's emotes.
//! Everything else, and the TS port's own functions, keep the practice AI's weights and its 0.8–2.5 s
//! window, so the port still draws as `personas.ts` does.

use jackioh_engine::wire::{
    EMOTE_IDS, EmoteId, GameEvent, Phase, PlayerId, PlayerView, SideView, emote_gate, hand_holds, opponent_of,
};

use crate::config::{
    AI_EMOTE, AI_PERSONAS, BOT_PERSONA_WEIGHTS, EmoteDelayFit, EmoteReplyKey, EmoteTrigger, PERSONA_DEAL_STEPS,
    PERSONA_NAMES, PersonaName, persona_spec,
};

/// R645: the practice AI's delay window, `AI_EMOTE`'s, as an `EmoteDelayFit`.
pub const PRACTICE_EMOTE_DELAY: EmoteDelayFit = EmoteDelayFit {
    samples: 0,
    min_ms: AI_EMOTE.delay_min_ms,
    max_ms: AI_EMOTE.delay_max_ms,
};

/// R645: an emote the persona wants on screen, `delay_ms` after its trigger (TS `AiEmote`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AiEmote {
    pub emote: EmoteId,
    pub delay_ms: f64,
}

fn side_of(view: &PlayerView, seat: PlayerId) -> &SideView {
    if view.you.player == seat {
        &view.you
    } else {
        &view.opponent
    }
}

fn unit_count(side: &SideView) -> usize {
    side.units.iter().filter(|unit| unit.is_some()).count()
}

/// The highest attack on `side`'s units, never below 0 (TS's `reduce` from 0).
fn max_attack_of(side: &SideView) -> i32 {
    side.units
        .iter()
        .flatten()
        .fold(0, |top, unit| top.max(unit.attack))
}

/// The id a hit on `seat`'s hero names (`hero-p1`).
fn hero_id(seat: PlayerId) -> String {
    format!("hero-{seat}")
}

// ---------------------------------------------------------------------------------------------
// The pure rolls (R645, R1344).
// ---------------------------------------------------------------------------------------------

/// R645: the weighted deal (TS `pickPersona`), one draw. A tutorial bot is always Silent instead.
pub fn pick_persona(rng: &mut dyn FnMut() -> f64) -> PersonaName {
    deal_by(rng, &AI_PERSONAS.map(|(_, spec)| spec.weight))
}

/// R639000: a ladder bot's deal, the same draw over `BOT_PERSONA_WEIGHTS`.
pub fn pick_bot_persona(rng: &mut dyn FnMut() -> f64) -> PersonaName {
    deal_by(rng, &BOT_PERSONA_WEIGHTS.weights)
}

/// The weighted deal over `weights`, in `PERSONA_NAMES` order.
fn deal_by(rng: &mut dyn FnMut() -> f64, weights: &[f64; 4]) -> PersonaName {
    // Whole-number thresholds keep 0.85 in Silent despite floating-point summation.
    let draw = (rng() * PERSONA_DEAL_STEPS).floor();
    let mut cumulative = 0.0;
    for (name, weight) in PERSONA_NAMES.iter().zip(weights) {
        cumulative += (weight * PERSONA_DEAL_STEPS).round();
        if draw < cumulative {
            return *name;
        }
    }
    // A hostile rng returning 1 cannot pick nothing: the last interval is closed from the left.
    PersonaName::Silent
}

/// The uniform pick from a pool (TS `pickFrom`). It draws even from an empty pool, and then picks
/// nothing; a negative or NaN draw picks nothing too, as TS's `pool[at] ?? null` does.
fn pick_from(pool: &[EmoteId], rng: &mut dyn FnMut() -> f64) -> Option<EmoteId> {
    let drawn = (rng() * pool.len() as f64).floor();
    let last = pool.len().checked_sub(1)?;
    if drawn.is_nan() || drawn < 0.0 {
        return None;
    }
    pool.get((drawn as usize).min(last)).copied()
}

/// R645's "a random 0.8–2.5 s delay", drawn inside `window` (TS `rollDelay`; R639000: a ladder bot's
/// window is its refit row).
pub fn roll_delay(rng: &mut dyn FnMut() -> f64, window: &EmoteDelayFit) -> f64 {
    window.min_ms + rng() * (window.max_ms - window.min_ms)
}

/// R1344: the part of a row's pool the seat's hand holds, in the pool's order.
pub fn pool_in_hand(pool: &[EmoteId], hand: &[EmoteId]) -> Vec<EmoteId> {
    pool.iter()
        .copied()
        .filter(|emote| hand_holds(hand, *emote))
        .collect()
}

/// R645 trigger roll, R1344 filtering the pool by `hand` (`EMOTE_IDS` for the whole pool). The draws
/// are the chance, then the pick, then the delay; a dash draws nothing. Caps and the rate limit are
/// `EmotePersona`'s.
pub fn roll_for_trigger(
    persona: PersonaName,
    trigger: EmoteTrigger,
    rng: &mut dyn FnMut() -> f64,
    hand: &[EmoteId],
) -> Option<AiEmote> {
    roll_for_trigger_within(persona, trigger, rng, hand, &PRACTICE_EMOTE_DELAY)
}

/// `roll_for_trigger`, its delay drawn inside `window` (R639000).
pub fn roll_for_trigger_within(
    persona: PersonaName,
    trigger: EmoteTrigger,
    rng: &mut dyn FnMut() -> f64,
    hand: &[EmoteId],
    window: &EmoteDelayFit,
) -> Option<AiEmote> {
    let row = persona_spec(persona).trigger(trigger)?;
    if rng() >= row.chance {
        return None;
    }
    let emote = pick_from(&pool_in_hand(row.pool, hand), rng)?;
    Some(AiEmote {
        emote,
        delay_ms: roll_delay(rng, window),
    })
}

/// R645, R1344: the reply row a player's `emote` is answered as, or `None` (a yawn, a shrug, a
/// thinking face and a gasp earn none).
pub fn reply_key_of(emote: EmoteId) -> Option<EmoteReplyKey> {
    match emote {
        EmoteId::Greetings | EmoteId::Wave => Some(EmoteReplyKey::Greetings),
        EmoteId::WellPlayed
        | EmoteId::Thanks
        | EmoteId::Clap
        | EmoteId::ThumbsUp
        | EmoteId::Heart
        | EmoteId::Salute => Some(EmoteReplyKey::Compliment),
        EmoteId::Threaten
        | EmoteId::Laugh
        | EmoteId::WahWah
        | EmoteId::Angry
        | EmoteId::Fire
        | EmoteId::Skull
        | EmoteId::Cool
        | EmoteId::Party => Some(EmoteReplyKey::Taunt),
        EmoteId::Oops | EmoteId::Sob | EmoteId::Facepalm | EmoteId::Sweat => Some(EmoteReplyKey::Apology),
        EmoteId::Yawn | EmoteId::Shrug | EmoteId::Thinking | EmoteId::Gasp => None,
    }
}

/// R645 reply roll, with R1344's hand filtering: the answer and the row it answered from. An emote
/// with no row draws nothing.
pub fn roll_for_reply(
    persona: PersonaName,
    emote: EmoteId,
    rng: &mut dyn FnMut() -> f64,
    hand: &[EmoteId],
) -> Option<(AiEmote, EmoteReplyKey)> {
    roll_for_reply_within(persona, emote, rng, hand, &PRACTICE_EMOTE_DELAY)
}

/// `roll_for_reply`, its delay drawn inside `window` (R639000).
pub fn roll_for_reply_within(
    persona: PersonaName,
    emote: EmoteId,
    rng: &mut dyn FnMut() -> f64,
    hand: &[EmoteId],
    window: &EmoteDelayFit,
) -> Option<(AiEmote, EmoteReplyKey)> {
    let key = reply_key_of(emote)?;
    let row = persona_spec(persona).reply(key)?;
    if rng() >= row.chance {
        return None;
    }
    let picked = pick_from(&pool_in_hand(row.pool, hand), rng)?;
    Some((
        AiEmote {
            emote: picked,
            delay_ms: roll_delay(rng, window),
        },
        key,
    ))
}

// ---------------------------------------------------------------------------------------------
// One match's persona: the session triggers, the caps and the rate limit (R645, R643).
// ---------------------------------------------------------------------------------------------

/// Whose turn it is and in which phase: what a roll's caps are counted against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Moment {
    active: PlayerId,
    phase: Phase,
}

/// R643: the instant an emote rolled at `now` goes out, in whole milliseconds (`emote_gate` takes
/// them; TS keeps the float).
fn send_instant(now: i64, delay_ms: f64) -> i64 {
    (now as f64 + delay_ms).round() as i64
}

/// R645: one match's persona for the bot in `seat` (TS `createEmotePersona`). R1344: every roll is
/// constrained to its dealt hand.
pub struct EmotePersona {
    persona: PersonaName,
    seat: PlayerId,
    rng: Box<dyn FnMut() -> f64 + Send>,
    hand: Vec<EmoteId>,
    /// R639000: the windows a triggered emote's and a reply's delays are drawn inside.
    trigger_delay: EmoteDelayFit,
    reply_delay: EmoteDelayFit,
    /// R643: the send instants of the emotes it let through, oldest first.
    sent_at: Vec<i64>,
    sent_match: usize,
    sent_own_turn: usize,
    sent_other_turn: usize,
    /// R645: the reply rows already answered; each answers at most once a match.
    replies_used: Vec<EmoteReplyKey>,
    kills_this_turn: usize,
    big_play_rolled_turn: Option<i32>,
    long_turn_rolled_turn: Option<i32>,
    /// The latest moment seen, which a reply's caps are counted against.
    last_moment: Option<Moment>,
}

impl EmotePersona {
    /// A persona for `seat`, drawing on `rng` and choosing within `hand` (`None`: the whole pool,
    /// `EMOTE_IDS`, as a test makes it).
    pub fn new(
        persona: PersonaName,
        seat: PlayerId,
        rng: Box<dyn FnMut() -> f64 + Send>,
        hand: Option<Vec<EmoteId>>,
    ) -> Self {
        EmotePersona {
            persona,
            seat,
            rng,
            hand: hand.unwrap_or_else(|| EMOTE_IDS.to_vec()),
            trigger_delay: PRACTICE_EMOTE_DELAY,
            reply_delay: PRACTICE_EMOTE_DELAY,
            sent_at: Vec::new(),
            sent_match: 0,
            sent_own_turn: 0,
            sent_other_turn: 0,
            replies_used: Vec::new(),
            kills_this_turn: 0,
            big_play_rolled_turn: None,
            long_turn_rolled_turn: None,
            last_moment: None,
        }
    }

    /// R639000: the persona, waiting out its triggered emotes inside `trigger` and its replies inside
    /// `reply` instead of the practice AI's window: a ladder bot's refit rows.
    pub fn with_delays(self, trigger: EmoteDelayFit, reply: EmoteDelayFit) -> Self {
        EmotePersona {
            trigger_delay: trigger,
            reply_delay: reply,
            ..self
        }
    }

    /// The persona this bot was dealt.
    pub fn persona(&self) -> PersonaName {
        self.persona
    }

    /// R677: a Glitch's swap moved the bot to the other engine seat, which its triggers read from now.
    /// The server's driver alone calls it; practice has no swap to follow.
    pub fn set_seat(&mut self, seat: PlayerId) {
        self.seat = seat;
    }

    /// R645's caps and R643's shared limiter (TS `admit`); a blocked emote is dropped, never queued.
    fn admit(
        &mut self,
        roll: Option<AiEmote>,
        moment: Moment,
        now: i64,
        match_exempt: bool,
    ) -> Option<AiEmote> {
        let roll = roll?;
        let at = send_instant(now, roll.delay_ms);
        if moment.phase == Phase::Over {
            // R645 exempts a finished game from the caps, not from R643's rate limit.
            if !emote_gate(&self.sent_at, at).ok {
                return None;
            }
            self.sent_at.push(at);
            return Some(roll);
        }
        let caps = persona_spec(self.persona).caps;
        let own_turn = moment.active == self.seat;
        let (sent, cap) = if own_turn {
            (self.sent_own_turn, caps.own_turn)
        } else {
            (self.sent_other_turn, caps.other_turn)
        };
        if sent >= cap {
            return None;
        }
        if !match_exempt && self.sent_match >= caps.per_match {
            return None;
        }
        if !emote_gate(&self.sent_at, at).ok {
            return None;
        }
        if own_turn {
            self.sent_own_turn += 1;
        } else {
            self.sent_other_turn += 1;
        }
        if !match_exempt {
            self.sent_match += 1;
        }
        self.sent_at.push(at);
        Some(roll)
    }

    /// Rolls `trigger` and pushes what `admit` lets through (TS `emit`).
    fn fire(
        &mut self,
        trigger: EmoteTrigger,
        moment: Moment,
        now: i64,
        match_exempt: bool,
        out: &mut Vec<AiEmote>,
    ) {
        let roll = roll_for_trigger_within(
            self.persona,
            trigger,
            &mut *self.rng,
            &self.hand,
            &self.trigger_delay,
        );
        if let Some(admitted) = self.admit(roll, moment, now, match_exempt) {
            out.push(admitted);
        }
    }

    /// R645: detects the trigger table's triggers across one view step (`prev` to `view`, with the
    /// `fresh` events between them), in the table's order, and rolls each that fires. The whole batch
    /// is counted against `view`'s moment.
    pub fn on_events(
        &mut self,
        fresh: &[GameEvent],
        prev: Option<&PlayerView>,
        view: &PlayerView,
        now: i64,
    ) -> Vec<AiEmote> {
        let mut out = Vec::new();
        let seat = self.seat;
        let foe = opponent_of(seat);
        let you = side_of(view, seat);
        let enemy = side_of(view, foe);
        let own_hero = hero_id(seat);
        let foe_hero = hero_id(foe);
        let moment = Moment {
            active: view.active,
            phase: view.phase,
        };
        self.last_moment = Some(moment);

        // The mulligan's end is the step away from a view that showed it open.
        if prev.is_some_and(|prev| prev.mulligan.is_some()) && view.mulligan.is_none() {
            self.fire(EmoteTrigger::MulliganEnd, moment, now, true, &mut out);
        }

        for event in fresh {
            match event {
                GameEvent::TurnStarted { player, .. } => {
                    // Reset before the same batch's rolls use the new turn's caps and kill count.
                    if *player == seat {
                        self.sent_own_turn = 0;
                    } else {
                        self.sent_other_turn = 0;
                    }
                    self.kills_this_turn = 0;
                    if *player != seat {
                        continue;
                    }
                    let ahead = you.hero.health - enemy.hero.health >= AI_EMOTE.lead_health
                        || unit_count(you) >= unit_count(enemy) + AI_EMOTE.lead_units;
                    if ahead {
                        self.fire(EmoteTrigger::TurnStartAhead, moment, now, false, &mut out);
                    }
                }
                GameEvent::Damage {
                    target_id, amount, ..
                } => {
                    if *target_id == foe_hero && *amount >= AI_EMOTE.big_hit {
                        self.fire(EmoteTrigger::DealtBigHit, moment, now, false, &mut out);
                    }
                    if *target_id == own_hero && *amount >= AI_EMOTE.big_hit {
                        // The same hit rolls tookBigHit and playerBigPlay, independently.
                        self.fire(EmoteTrigger::TookBigHit, moment, now, false, &mut out);
                        self.fire(EmoteTrigger::PlayerBigPlay, moment, now, false, &mut out);
                    }
                }
                GameEvent::HealthLost { player, amount } | GameEvent::Fatigue { player, amount, .. } => {
                    if *player == seat && *amount >= AI_EMOTE.big_hit {
                        self.fire(EmoteTrigger::TookBigHit, moment, now, false, &mut out);
                    }
                }
                GameEvent::Destroyed {
                    controller,
                    attack,
                    killer_id,
                    ..
                } => {
                    if killer_id.is_none() {
                        continue;
                    }
                    if *controller == seat {
                        let top = prev.map_or(*attack, |prev| max_attack_of(side_of(prev, seat)));
                        if *attack >= top {
                            self.fire(EmoteTrigger::LostTopUnit, moment, now, false, &mut out);
                        }
                        // playerBigPlay rolls once, as a player's turn reaches its kill threshold.
                        if view.active != seat {
                            self.kills_this_turn += 1;
                            if self.kills_this_turn >= AI_EMOTE.kills_for_big_play
                                && self.big_play_rolled_turn != Some(view.turn)
                            {
                                self.big_play_rolled_turn = Some(view.turn);
                                self.fire(EmoteTrigger::PlayerBigPlay, moment, now, false, &mut out);
                            }
                        }
                    } else {
                        let top = prev.map_or(*attack, |prev| max_attack_of(side_of(prev, foe)));
                        if *attack >= top {
                            self.fire(EmoteTrigger::KilledTopUnit, moment, now, false, &mut out);
                        }
                    }
                }
                GameEvent::GameOver { winner, .. } => match winner.player() {
                    // R645 exempts the end-of-match emotes from the match cap.
                    Some(player) if player == seat => {
                        self.fire(EmoteTrigger::MatchWon, moment, now, true, &mut out);
                    }
                    Some(_) => self.fire(EmoteTrigger::MatchLost, moment, now, true, &mut out),
                    None => {}
                },
                _ => {}
            }
        }
        out
    }

    /// R645: a player's emote may earn a reply, each reply row at most once a match. Only a player's
    /// emote is passed here, so the bot never replies to a reply. A row is spent only when its reply
    /// is sent; the roll comes before that check, so a spent row still draws.
    pub fn on_player_emote(&mut self, emote: EmoteId, now: i64) -> Vec<AiEmote> {
        let Some((roll, key)) =
            roll_for_reply_within(self.persona, emote, &mut *self.rng, &self.hand, &self.reply_delay)
        else {
            return Vec::new();
        };
        if self.replies_used.contains(&key) {
            return Vec::new();
        }
        let moment = self.last_moment.unwrap_or(Moment {
            active: opponent_of(self.seat),
            phase: Phase::Mulligan,
        });
        match self.admit(Some(roll), moment, now, false) {
            Some(admitted) => {
                self.replies_used.push(key);
                vec![admitted]
            }
            None => Vec::new(),
        }
    }

    /// R645's long-turn trigger: the driver calls it when the player's turn has run past
    /// `AI_EMOTE.long_turn_ms`. It rolls once per turn number, and never on the bot's own turn.
    pub fn on_player_turn_long(&mut self, view: &PlayerView, now: i64) -> Vec<AiEmote> {
        let moment = Moment {
            active: view.active,
            phase: view.phase,
        };
        self.last_moment = Some(moment);
        if view.active == self.seat || self.long_turn_rolled_turn == Some(view.turn) {
            return Vec::new();
        }
        self.long_turn_rolled_turn = Some(view.turn);
        let roll = roll_for_trigger_within(
            self.persona,
            EmoteTrigger::PlayerTurnLong,
            &mut *self.rng,
            &self.hand,
            &self.trigger_delay,
        );
        self.admit(roll, moment, now, false).into_iter().collect()
    }
}
