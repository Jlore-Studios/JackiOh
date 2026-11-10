//! Turning one event into its line (SPEC §10.10, BUILD M5-T1): a plain list of readable sentences, in the
//! reader's words. A line is built from the event payload plus public board data, and card identities that
//! are only in the view by accident are not printed: `drawn`, `addedToHand` and `shuffledIn` carry a
//! `defId` for the animation layer, so the log says "drew a card", and a redaction slip upstream cannot turn
//! into a leak on screen. A line never prints an id, and a `trapFired` sentinel (R763, R2734000) reads as "a
//! face-down trap". `cardResolved` gets no line: it is bookkeeping.
//!
//! Every sentence is an English key (`say`, `s`, `tw`) that `data/zh-CN.json` translates, the way the web's
//! `t()` was used: the English stays inline, and `tests/log.rs` checks each has its entry.

use jackioh_engine::game_over::{SeatOutcome, seat_outcome};
use jackioh_engine::wire::{
    CounterKind, FengShuiOutcome, GameEvent, GameOverReason, GlitchOutcome, LibraryOverflowOutcome, PlayerId,
    PlayerView, PredictOutcome, PromptKind, RedirectWhat, RotationDirection, Row, SecretChoice, Winner,
    ZoneName,
};

use crate::face::chinese::{list_separator, reader_keyword};
use crate::face::glitch::glitch_words;
use crate::face::glossary;
use crate::face::live::HIDDEN_CARD;
use crate::face::types::Span;

use super::chaos::chaos_effect_name;
use super::curve::curve_equation;
use super::names::{Names, find_card};
use super::say::{Part, capitalised, capitalised_spans, plain_words, sentence};

fn p<T: Into<Part>>(name: &'static str, value: T) -> (&'static str, Part) {
    (name, value.into())
}

/// "lane 3" or "backrow lane 3".
fn zone_label(names: &Names<'_>, row: Row, lane: i32) -> String {
    let english = match row {
        Row::Units => "lane {lane}",
        Row::Backrow => "backrow lane {lane}",
    };
    names.words.say_with(english, &[("lane", lane.to_string())])
}

/// What a prompt asks for, as the line "{seat} must choose {what}" ends.
fn prompt_what(names: &Names<'_>, kind: PromptKind) -> String {
    match kind {
        PromptKind::Discover => names.say("a card to Discover"),
        PromptKind::Target => names.say("a target"),
        PromptKind::Mode => names.say("a mode"),
        PromptKind::Mulligan => names.say("cards to keep"),
        PromptKind::Hand => names.say("a card from hand"),
        PromptKind::Zone => names.say("a zone"),
        PromptKind::Tribute => names.say("a Tribute"),
        PromptKind::Direction => names.say("a direction"),
        PromptKind::X => names.say("X"),
        PromptKind::Embiggen => names.say("the normal or the Embiggened version"),
        PromptKind::Number => names.say("a number"),
        PromptKind::Answer => names.say("an answer"),
        PromptKind::Cell => names.say("a cell"),
        PromptKind::Reward => names.say("a reward"),
        PromptKind::Pick => names.say("cards to take"),
        PromptKind::Craft => names.say("a card to craft"),
        PromptKind::Market => names.say("a deal at the night market"),
    }
}

fn secret_word(names: &Names<'_>, choice: SecretChoice) -> String {
    match choice {
        SecretChoice::Greed => names.say("Greed"),
        SecretChoice::Attack => names.say("Attack"),
        SecretChoice::Defend => names.say("Defend"),
    }
}

fn predicted_word(names: &Names<'_>, outcome: PredictOutcome) -> String {
    match outcome {
        PredictOutcome::Won => names.say("the reward is canceled"),
        PredictOutcome::Same => names.say("the same, nothing happens"),
        PredictOutcome::Lost => names.say("the penalty lands"),
    }
}

/// R676–R679: each Glitch outcome, as its log line ends.
fn glitch_outcome(names: &Names<'_>, outcome: GlitchOutcome) -> String {
    match outcome {
        GlitchOutcome::Reset => names.say("the match resets"),
        GlitchOutcome::Swap => names.say("the players swap seats"),
        GlitchOutcome::Boards => names.say("other games' boards appear"),
        GlitchOutcome::Void => names.say("the match never happened"),
    }
}

/// R983: each Feng Shui outcome, as its log line ends.
fn feng_shui_outcome(names: &Names<'_>, outcome: FengShuiOutcome) -> String {
    match outcome {
        FengShuiOutcome::Positive => names.say("auspicious (吉)"),
        FengShuiOutcome::Negative => names.say("inauspicious (凶)"),
    }
}

fn redirected_what(names: &Names<'_>, what: RedirectWhat) -> String {
    match what {
        RedirectWhat::Damage => names.say("a hit"),
        RedirectWhat::Attack => names.say("an attack"),
        RedirectWhat::Target => names.say("a target"),
    }
}

/// R437: the name a mark's badge says, never the engine's key (the web's `cards/marks.ts` holds the badge's
/// own words); an unknown mark gets generic words.
fn mark_name(names: &Names<'_>, mark: &str) -> String {
    match mark {
        "steal" => names.say("Steal"),
        "berserk" => names.say("Berserk"),
        "frozen" => names.say("Frozen"),
        "destroy" => names.say("Destroy"),
        "allure" => names.say("Allure"),
        "duo" => names.say("Duo"),
        "endOfTurn" => names.say("End of turn"),
        _ => names.say("Mark"),
    }
}

/// An engine key ("drawLimit") in words ("draw limit").
fn key_words(key: &str) -> String {
    let mut out = String::with_capacity(key.len() + 2);
    let mut before: Option<char> = None;
    for c in key.chars() {
        if before.is_some_and(|b| b.is_ascii_lowercase()) && c.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(c);
        before = Some(c);
    }
    out.to_lowercase()
}

/// Why the game ended, as the viewer would say it. `winner` tells a draw by two fleers from the other
/// player's flight (R1843002).
fn result_reason(outcome: SeatOutcome, reason: GameOverReason, winner: Winner) -> &'static str {
    let win = outcome == SeatOutcome::Win;
    match reason {
        GameOverReason::HeroDeath => {
            if win {
                "The opposing hero has fallen."
            } else {
                "Your hero has fallen."
            }
        }
        GameOverReason::BothHeroesDead => "Both heroes fell together.",
        GameOverReason::Concede => {
            if win {
                "Your opponent conceded."
            } else {
                "You conceded."
            }
        }
        GameOverReason::DrawAccepted => "Game drawn by agreement.",
        GameOverReason::TurnCap => "The turn limit was reached.",
        GameOverReason::Disconnect => {
            if win {
                "Your opponent left the match."
            } else {
                "You left the match."
            }
        }
        GameOverReason::MatchCeiling => "The match reached its time limit.",
        // R679: a Glitch voided the match; it counts for nobody.
        GameOverReason::Voided => "This match never happened.",
        // R850: a held alternative win won.
        GameOverReason::AltWin => match outcome {
            SeatOutcome::Draw => "Both players met a winning condition.",
            SeatOutcome::Win => "You met your winning condition.",
            SeatOutcome::Loss => "Your opponent met their winning condition.",
        },
        // R850: an effect won the game outright.
        GameOverReason::WonByEffect => {
            if win {
                "Your card won the game."
            } else {
                "Your opponent's card won the game."
            }
        }
        // R1843000: a player's tally filled, and the cards that voted removed them.
        GameOverReason::VotedOut => {
            if win {
                "Your opponent was voted out of the game."
            } else {
                "You were voted out of the game."
            }
        }
        // R1843001: both tallies filled at once.
        GameOverReason::BothVotedOut => "Both players were voted out together.",
        // R1843002, R1843004: a win for the fleer and a draw for the other player; two fleers draw.
        GameOverReason::Fled => {
            if win {
                "You escaped the universe. Your opponent draws."
            } else if winner == Winner::Draw {
                "Both players escaped the universe."
            } else {
                "Your opponent escaped the universe. You draw."
            }
        }
        // R1843002: the fleer chose a loss for the other player.
        GameOverReason::FledDecisive => {
            if win {
                "You escaped the universe and left your opponent behind."
            } else {
                "Your opponent escaped the universe and left you behind."
            }
        }
    }
}

/// R2652001: a coin's or a die's face as words: heads or tails, a number.
fn face_word(names: &Names<'_>, coin: bool, face: i32) -> String {
    if coin {
        if face == jackioh_engine::config::ROLL_HEADS {
            names.say("heads")
        } else {
            names.say("tails")
        }
    } else {
        face.to_string()
    }
}

/// R2652001: the batch in words, `who` naming the card that rolled: what landed, and how many extra rolls
/// Lucky made.
fn roll_words(
    names: &Names<'_>,
    coin: bool,
    sides: i32,
    rolls: &[jackioh_engine::wire::RollResult],
    who: String,
) -> String {
    let w = names.words;
    let count = rolls.len();
    let kept: Vec<i32> = rolls.iter().map(|roll| roll.kept_face()).collect();
    let sentence_of =
        |english: &str, parts: Vec<(&'static str, Part)>| plain_words(&sentence(w, english, &parts));
    let main = if coin {
        let heads = rolls.iter().filter(|roll| roll.heads()).count();
        if count == 1 {
            sentence_of(
                "{who} flipped a coin: {face}",
                vec![p("who", who), p("face", face_word(names, true, kept[0]))],
            )
        } else {
            sentence_of(
                "{who} flipped {count} coins: {heads} heads, {tails} tails",
                vec![
                    p("who", who),
                    p("count", count),
                    p("heads", heads),
                    p("tails", count - heads),
                ],
            )
        }
    } else if count == 1 {
        sentence_of(
            "{who} rolled a {sides}-sided die: {face}",
            vec![
                p("who", who),
                p("sides", sides),
                p("face", face_word(names, false, kept[0])),
            ],
        )
    } else {
        let faces: Vec<String> = kept.iter().map(|face| face_word(names, false, *face)).collect();
        sentence_of(
            "{who} rolled {count} {sides}-sided dice: {faces}",
            vec![
                p("who", who),
                p("count", count),
                p("sides", sides),
                p("faces", faces.join(list_separator(w))),
            ],
        )
    };
    let extra: usize = rolls.iter().map(|roll| roll.faces.len().saturating_sub(1)).sum();
    if extra > 0 {
        sentence_of(
            "{sentence} (Lucky rolled {count} more)",
            vec![p("sentence", main), p("count", extra)],
        )
    } else {
        main
    }
}

/// The keyword a line is about, as the glossary's term in the reader's words.
fn keyword_span(names: &Names<'_>, kind: &str) -> Vec<Span> {
    let word = reader_keyword(names.words, kind);
    match glossary::entry(kind) {
        Some(entry) => {
            let entry = if names.words.is_zh_cn() {
                glossary::chinese_entry(entry)
            } else {
                entry
            };
            vec![Span::Keyword {
                entry,
                children: vec![Span::Text { text: word }],
            }]
        }
        None => vec![Span::Text { text: word }],
    }
}

/// The quest's words while the card's view still lists the quest open.
fn quest_text(view: &PlayerView, instance_id: &str, quest: &str) -> Option<String> {
    find_card(view, instance_id, true)?
        .quest?
        .open
        .iter()
        .find(|open| open.id == quest)
        .map(|open| open.text.clone())
}

/// The line `event` reads as, or None for an event that has no line. `aims` are the targets of the play's
/// own announce.
pub fn describe(names: &Names<'_>, event: &GameEvent, aims: Option<&[String]>) -> Option<Vec<Span>> {
    let w = names.words;
    let view = names.view;
    let s = |english: &str, parts: Vec<(&'static str, Part)>| sentence(w, english, &parts);
    let cap = capitalised_spans;
    let hidden = |id: &str| id == HIDDEN_CARD;
    let seat = |player: PlayerId| names.seat(player);
    let whose = |player: PlayerId| names.whose(player);
    // "Your hero", "A unit": the target as the line opens.
    let target = |id: &str| capitalised(&names.instance(id, None));
    let line = match event {
        GameEvent::CardPlayed {
            player,
            def_id,
            cost_paid,
            ..
        } => {
            // B5 E1: the targets its announce declared, each where the viewer may name it (R97).
            let targets: Vec<String> = aims
                .unwrap_or(&[])
                .iter()
                .filter_map(|id| names.known(id))
                .collect();
            let mut parts = vec![
                p("seat", seat(*player)),
                p("card", names.def(def_id)),
                p("cost", *cost_paid),
            ];
            if targets.is_empty() {
                s("{seat} played {card} for {cost}", parts)
            } else {
                parts.push(p("targets", targets.join(list_separator(w))));
                s("{seat} played {card} for {cost}, aiming at {targets}", parts)
            }
        }
        GameEvent::Summoned {
            player,
            instance_id,
            def_id,
            row,
            lane,
            ..
        } => {
            // R370: a card set face down is a trap the viewer may not read; the line says so, with the
            // cost its back shows while it still stands there.
            let mut parts = vec![
                p("whose", whose(*player)),
                p("zone", zone_label(names, *row, *lane)),
            ];
            if hidden(def_id) && *row == Row::Backrow {
                return Some(cap(match names.face_down_cost(*player, *lane) {
                    None => s("{whose} {zone}: a face-down trap was set", parts),
                    Some(cost) => {
                        parts.push(p("cost", cost));
                        s("{whose} {zone}: a face-down trap was set, ({cost}) Cost", parts)
                    }
                }));
            }
            parts.push(p("card", names.card(instance_id, def_id)));
            cap(s("{card} entered {whose} {zone}", parts))
        }
        GameEvent::Damage {
            source_id,
            target_id,
            amount,
            combat,
            absorbed,
            ..
        } => {
            // A hit off combat names what dealt it where the viewer may name it; a hidden or null source
            // names none (R97).
            let source = if *combat {
                None
            } else {
                source_id.as_deref().and_then(|id| names.known(id))
            };
            if let Some(source) = source {
                let dealt = vec![
                    p("source", capitalised(&source)),
                    p("target", names.instance(target_id, None)),
                    p("amount", *amount),
                    p("absorbed", *absorbed),
                ];
                if *absorbed > 0 {
                    s(
                        "{source} dealt {amount} damage to {target}, {absorbed} absorbed by Armor",
                        dealt,
                    )
                } else {
                    s("{source} dealt {amount} damage to {target}", dealt)
                }
            } else {
                let hit = vec![
                    p("target", target(target_id)),
                    p("amount", *amount),
                    p("absorbed", *absorbed),
                ];
                // R1360: the Armor's part of the hit, where it took one.
                match (*absorbed > 0, *combat) {
                    (true, true) => s(
                        "{target} took {amount} damage in combat, {absorbed} absorbed by Armor",
                        hit,
                    ),
                    (true, false) => s("{target} took {amount} damage, {absorbed} absorbed by Armor", hit),
                    (false, true) => s("{target} took {amount} damage in combat", hit),
                    (false, false) => s("{target} took {amount} damage", hit),
                }
            }
        }
        GameEvent::DamageAbsorbed {
            source_id,
            target_id,
            absorbed,
            combat,
            ..
        } => {
            // R1361: the Armor took the whole hit, so nothing was dealt.
            let hit = vec![p("target", target(target_id)), p("absorbed", *absorbed)];
            let source = if *combat {
                None
            } else {
                source_id.as_deref().and_then(|id| names.known(id))
            };
            match source {
                Some(source) => {
                    let mut hit = hit;
                    hit.push(p("source", source));
                    s("{target}'s Armor absorbed {absorbed} damage from {source}", hit)
                }
                None if *combat => s("{target}'s Armor absorbed {absorbed} damage in combat", hit),
                None => s("{target}'s Armor absorbed {absorbed} damage", hit),
            }
        }
        // R860: the line names only that a secret is held, never the choice.
        GameEvent::SecretChosen { player, .. } => s("{seat} kept a secret", vec![p("seat", seat(*player))]),
        // R864: revealed, the choice is public history.
        GameEvent::SecretRevealed { player, choice, .. } => cap(s(
            "{whose} secret was {choice}",
            vec![
                p("whose", whose(*player)),
                p("choice", secret_word(names, *choice)),
            ],
        )),
        GameEvent::Predicted {
            player,
            guess,
            outcome,
            ..
        } => s(
            "{seat} guessed {guess}: {outcome}",
            vec![
                p("seat", seat(*player)),
                p("guess", secret_word(names, *guess)),
                p("outcome", predicted_word(names, *outcome)),
            ],
        ),
        GameEvent::HealthLost { player, amount } => s(
            "{seat} lost {amount} health",
            vec![p("seat", seat(*player)), p("amount", *amount)],
        ),
        GameEvent::Healed {
            target_id, amount, ..
        } => s(
            "{target} healed {amount}",
            vec![p("target", target(target_id)), p("amount", *amount)],
        ),
        GameEvent::DivineShieldLost { instance_id } => s(
            "{target} lost its Divine Shield",
            vec![p("target", target(instance_id))],
        ),
        GameEvent::Destroyed {
            instance_id, def_id, ..
        } => cap(s(
            "{card} was destroyed",
            vec![p("card", names.card(instance_id, def_id))],
        )),
        GameEvent::CardResolved { .. } => return None,
        GameEvent::EnteredGraveyard {
            instance_id,
            def_id,
            owner,
        } => cap(s(
            "{card} went to {whose} graveyard",
            vec![
                p("card", names.card(instance_id, def_id)),
                p("whose", whose(*owner)),
            ],
        )),
        GameEvent::Exiled {
            instance_id, def_id, ..
        } => cap(s(
            "{card} was exiled",
            vec![p("card", names.card(instance_id, def_id))],
        )),
        GameEvent::Bounced {
            instance_id,
            def_id,
            owner,
        } => cap(s(
            "{card} returned to {whose} hand",
            vec![
                p("card", names.card(instance_id, def_id)),
                p("whose", whose(*owner)),
            ],
        )),
        // R317: a full hand's card, named where the viewer reads it (the sentinel is "a card").
        GameEvent::Burned { def_id, owner, .. } => cap(s(
            "{whose} hand is full: {card} burned",
            vec![p("whose", whose(*owner)), p("card", names.named(def_id))],
        )),
        // R315: public, and it names no card. The hit is the `damage` line after it.
        GameEvent::Fatigue { player, count, .. } => cap(s(
            "{whose} deck is empty: fatigue {count}",
            vec![p("whose", whose(*player)), p("count", *count)],
        )),
        // R316: the card a full library turned away, named only where the viewer reads it.
        GameEvent::LibraryOverflow {
            player,
            def_id,
            outcome,
            ..
        } => {
            let english = match outcome {
                LibraryOverflowOutcome::NotCreated => "{whose} deck is full: {card} was not created",
                LibraryOverflowOutcome::Graveyard => "{whose} deck is full: {card} went to the graveyard",
                LibraryOverflowOutcome::Ceased => "{whose} deck is full: {card} ceased to exist",
            };
            cap(s(
                english,
                vec![p("whose", whose(*player)), p("card", names.named(def_id))],
            ))
        }
        GameEvent::Discarded { def_id, owner, .. } => s(
            "{seat} discarded {card}",
            vec![p("seat", seat(*owner)), p("card", names.def(def_id))],
        ),
        GameEvent::Drawn { player, .. } => s("{seat} drew a card", vec![p("seat", seat(*player))]),
        GameEvent::AddedToHand { player, .. } => {
            s("{seat} added a card to hand", vec![p("seat", seat(*player))])
        }
        GameEvent::ShuffledIn { player, .. } => s(
            "{seat} shuffled a card into the deck",
            vec![p("seat", seat(*player))],
        ),
        // R177: a buff on a card this seat may not read arrives as the sentinel with 0/0, which says
        // nothing of its size, so the line does not claim one.
        GameEvent::Buffed {
            instance_id,
            attack,
            health,
        } => {
            if hidden(instance_id) {
                s("A hidden card was buffed", vec![])
            } else {
                s(
                    "{target} gained +{attack}/+{health}",
                    vec![
                        p("target", target(instance_id)),
                        p("attack", *attack),
                        p("health", *health),
                    ],
                )
            }
        }
        // R46: the same event reports the Taunt an Indestructible unit's knock-down takes.
        GameEvent::KeywordGranted {
            instance_id,
            keyword,
            lost,
        } => {
            let change = vec![
                p("target", target(instance_id)),
                p("keyword", keyword_span(names, keyword.kind().as_str())),
            ];
            if *lost == Some(true) {
                s("{target} lost {keyword}", change)
            } else {
                s("{target} gained {keyword}", change)
            }
        }
        GameEvent::CounterChanged {
            instance_id,
            counter,
            value,
            ..
        } => {
            let english = match counter {
                CounterKind::Plague => "{target} plague counters: {value}",
                CounterKind::Grade => "{target} grade counters: {value}",
                CounterKind::Brittle => "{target} brittle counters: {value}",
            };
            s(
                english,
                vec![p("target", target(instance_id)), p("value", *value)],
            )
        }
        GameEvent::JadeChanged { player, value } => cap(s(
            "{whose} Jade Counter is {value}",
            vec![p("whose", whose(*player)), p("value", *value)],
        )),
        // R1847001: the node the travel reached; the map itself shows its reward.
        GameEvent::StarscapeTravelled { player, node, .. } => cap(s(
            "{whose} starscape map reaches node {node}",
            vec![p("whose", whose(*player)), p("node", *node)],
        )),
        // R177: a card this seat may not read arrives with its cost redacted to a negative sentinel.
        GameEvent::CostChanged {
            instance_id, cost, ..
        } => {
            let target = capitalised(&names.instance(instance_id, Some("a card")));
            if *cost < 0 {
                s("{target} changed cost", vec![p("target", target)])
            } else {
                s(
                    "{target} now costs ({cost})",
                    vec![p("target", target), p("cost", *cost)],
                )
            }
        }
        // The id is the engine's handle ("m123"); the label, while the view still lists it, is the words.
        GameEvent::ModifierChanged {
            player,
            modifier_id,
            added,
        } => {
            let label = view
                .you
                .modifiers
                .iter()
                .chain(&view.opponent.modifiers)
                .find(|modifier| modifier.id == *modifier_id)
                .map(|modifier| modifier.label.as_str());
            let what = match label {
                None => names.say("an effect"),
                Some(label) => format!("\"{}\"", names.say(label)),
            };
            let change = vec![p("seat", seat(*player)), p("what", what)];
            if *added {
                s("{seat} gained {what}", change)
            } else {
                s("{seat} lost {what}", change)
            }
        }
        GameEvent::RadiantSet {
            instance_id, def_id, ..
        } => cap(s(
            "{card} became Radiant",
            vec![p("card", names.card(instance_id, def_id))],
        )),
        // MD-D6: the mirror of `radiantSet`'s line.
        GameEvent::Deradianted {
            instance_id, def_id, ..
        } => cap(s(
            "{card} is no longer Radiant",
            vec![p("card", names.card(instance_id, def_id))],
        )),
        // MD-D26: the spend is noise beside the play or activation it paid for; `manaChanged` says the total.
        GameEvent::ManaSpent { .. } => return None,
        // R97: a card the viewer may not read is a hidden card on both sides; with nothing to name the line
        // says only that it changed (a hidden card in a hand becoming another, #552).
        GameEvent::Transformed {
            from_def_id,
            to_def_id,
            new_instance_id,
            ..
        } => {
            if hidden(from_def_id) && hidden(to_def_id) {
                s("A hidden card was transformed", vec![])
            } else {
                cap(s(
                    "{from} became {to}",
                    vec![
                        p("from", names.def(from_def_id)),
                        p("to", names.card(new_instance_id, to_def_id)),
                    ],
                ))
            }
        }
        GameEvent::Fused {
            instance_ids, def_id, ..
        } => s(
            "{count} cards fused into {card}",
            vec![p("count", instance_ids.len()), p("card", names.def(def_id))],
        ),
        GameEvent::PositionSwitched {
            instance_id,
            position,
        } => s(
            "{target} switched to {position}",
            vec![
                p("target", target(instance_id)),
                p("position", names.say(position.as_str())),
            ],
        ),
        GameEvent::ControlChanged {
            instance_id,
            controller,
            row,
            lane,
            ..
        } => s(
            "{target} moved to {whose} {zone}",
            vec![
                p("target", target(instance_id)),
                p("whose", whose(*controller)),
                p("zone", zone_label(names, *row, *lane)),
            ],
        ),
        GameEvent::Rotated { direction } => match direction {
            RotationDirection::Left => s("The board rotated left", vec![]),
            RotationDirection::Right => s("The board rotated right", vec![]),
        },
        // Built from the event so the line reads as the engine names it; the dictionary holds each whole sentence.
        GameEvent::Swapped { what } => vec![Span::Text {
            text: names.say(&format!("Both players swapped {}", what.as_str())),
        }],
        GameEvent::Locked { player, row, lane } => s(
            "{whose} {zone} was locked",
            vec![
                p("whose", capitalised(&whose(*player))),
                p("zone", zone_label(names, *row, *lane)),
            ],
        ),
        GameEvent::TrapFired {
            def_id,
            controller,
            again,
            ..
        } => {
            let whose = capitalised(&whose(*controller));
            // R2734000: named on both seats where public; the sentinel (R763) is not a name.
            if hidden(def_id) {
                s("{whose} face-down trap fired", vec![p("whose", whose)])
            } else if *again == Some(true) {
                // R2657001: a face-up Field Trap firing again reveals nothing, so its line says only that it triggered.
                s(
                    "{whose} {card} triggered",
                    vec![p("whose", whose), p("card", names.def(def_id))],
                )
            } else {
                s(
                    "{whose} trap {card} fired",
                    vec![p("whose", whose), p("card", names.def(def_id))],
                )
            }
        }
        GameEvent::AttackDeclared {
            attacker_id,
            target_id,
            forced,
            ..
        } => {
            let attack = vec![
                p("attacker", target(attacker_id)),
                p("target", names.instance(target_id, None)),
            ];
            if *forced {
                s("{attacker} attacked {target} (forced)", attack)
            } else {
                s("{attacker} attacked {target}", attack)
            }
        }
        GameEvent::AttackCancelled { attacker_id, .. } => s(
            "{attacker}'s attack was canceled",
            vec![p("attacker", target(attacker_id))],
        ),
        GameEvent::ManaChanged { player, current, max } => {
            let mana = vec![p("seat", seat(*player)), p("current", *current), p("max", *max)];
            if names.is_you(*player) {
                s("{seat} have {current}/{max} mana", mana)
            } else {
                s("{seat} has {current}/{max} mana", mana)
            }
        }
        // R845: an extra turn's line says so.
        GameEvent::TurnStarted { player, turn, extra } => {
            let turn = vec![p("turn", *turn), p("seat", seat(*player))];
            if *extra == Some(true) {
                s("Turn {turn}: {seat} (extra turn)", turn)
            } else {
                s("Turn {turn}: {seat}", turn)
            }
        }
        GameEvent::TurnEnded {
            player,
            turn,
            unspent_mana,
        } => s(
            "{seat} ended turn {turn} with {mana} mana unspent",
            vec![
                p("seat", seat(*player)),
                p("turn", *turn),
                p("mana", *unspent_mana),
            ],
        ),
        GameEvent::TurnAutoEnded { player, turn } => s(
            "{seat} had no moves left, so turn {turn} ended",
            vec![p("seat", seat(*player)), p("turn", *turn)],
        ),
        // R81, §10.6: the other seat's event names whose prompt it is, not what it asks for.
        GameEvent::PromptOpened { player, kind, .. } => match kind {
            None => s("{seat} must choose", vec![p("seat", seat(*player))]),
            Some(kind) => s(
                "{seat} must choose {what}",
                vec![p("seat", seat(*player)), p("what", prompt_what(names, *kind))],
            ),
        },
        GameEvent::PromptAnswered { player, .. } => s("{seat} chose", vec![p("seat", seat(*player))]),
        GameEvent::DrawOffered { player } => s("{seat} offered a draw", vec![p("seat", seat(*player))]),
        GameEvent::DrawAnswered { player, accept } => {
            if *accept {
                s("{seat} accepted the draw", vec![p("seat", seat(*player))])
            } else {
                s("{seat} declined the draw", vec![p("seat", seat(*player))])
            }
        }
        GameEvent::Emoted { player, .. } => s("{seat} emoted", vec![p("seat", seat(*player))]),
        GameEvent::GameOver { winner, reason } => {
            let outcome = seat_outcome(*winner, *reason, view.viewer);
            let why = names.say(result_reason(outcome, *reason, *winner));
            match winner.player() {
                None => s("Draw. {why}", vec![p("why", why)]),
                Some(winner) => s("{seat} won. {why}", vec![p("seat", seat(winner)), p("why", why)]),
            }
        }
        // The play's own line is its `cardPlayed`, which names what this declared (`announced_aims`); a
        // countered one has its `countered` line.
        GameEvent::CardAnnounced { .. } => return None,
        GameEvent::Countered { def_id, to, .. } => {
            let card = vec![p("card", names.named(def_id))];
            if *to == jackioh_engine::wire::CounteredTo::Hand {
                s("{card} was countered and stolen", card)
            } else {
                s("{card} was countered", card)
            }
        }
        GameEvent::Stolen {
            def_id,
            from,
            to,
            zone,
            ..
        } => {
            // Where a stolen card was taken from, as its line reads (R373: the rules' library is the Deck).
            let english = match zone {
                ZoneName::Hand => "{seat} stole {card} from {whose} hand",
                ZoneName::Library => "{seat} stole {card} from {whose} deck",
                ZoneName::Resolving => "{seat} stole {card} from {whose} play",
                ZoneName::Graveyard => "{seat} stole {card} from {whose} graveyard",
                ZoneName::Exile => "{seat} stole {card} from {whose} exile",
                // The wire names no stolen card from "gone": a card on the field is the line's last resort.
                ZoneName::Field | ZoneName::Gone => "{seat} stole {card} from {whose} side of the field",
            };
            cap(s(
                english,
                vec![
                    p("seat", seat(*to)),
                    p("card", names.named(def_id)),
                    p("whose", whose(*from)),
                ],
            ))
        }
        GameEvent::Unlocked { player, row, lane } => s(
            "{whose} {zone} was unlocked",
            vec![
                p("whose", capitalised(&whose(*player))),
                p("zone", zone_label(names, *row, *lane)),
            ],
        ),
        GameEvent::Activated { player, def_id, .. } => s(
            "{seat} activated {card}",
            vec![p("seat", seat(*player)), p("card", names.def(def_id))],
        ),
        GameEvent::Animated {
            player,
            def_id,
            unit_lane,
            ..
        } => s(
            "{card} came to life in {whose} {zone}",
            vec![
                p("card", names.def(def_id)),
                p("whose", whose(*player)),
                p("zone", zone_label(names, Row::Units, *unit_lane)),
            ],
        ),
        GameEvent::Deanimated {
            player,
            def_id,
            backrow_lane,
            ..
        } => s(
            "{card} returned to {whose} {zone}",
            vec![
                p("card", names.def(def_id)),
                p("whose", whose(*player)),
                p("zone", zone_label(names, Row::Backrow, *backrow_lane)),
            ],
        ),
        GameEvent::Crumbled {
            instance_id, def_id, ..
        } => cap(s(
            "{card} crumbled",
            vec![p("card", names.named_card(instance_id, def_id))],
        )),
        // R1320: players read the engine's Degrade as a Nerf and its Upgrade as a Buff.
        GameEvent::Degraded {
            instance_id, def_id, ..
        } => cap(s(
            "{card} was nerfed",
            vec![p("card", names.named_card(instance_id, def_id))],
        )),
        GameEvent::Upgraded {
            instance_id, def_id, ..
        } => cap(s(
            "{card} was buffed",
            vec![p("card", names.named_card(instance_id, def_id))],
        )),
        GameEvent::NumberChanged {
            instance_id,
            def_id,
            key,
            value,
            ..
        } => {
            let card = names.named_card(instance_id, def_id);
            if key == HIDDEN_CARD {
                cap(s("{card} changed", vec![p("card", card)]))
            } else if key == "cost" {
                // A cost is a price (R432); any other key is a word.
                cap(s(
                    "{card} now costs ({value})",
                    vec![p("card", card), p("value", *value)],
                ))
            } else {
                cap(s(
                    "{card}'s {key} became {value}",
                    vec![
                        p("card", card),
                        p("key", names.say(&key_words(key))),
                        p("value", *value),
                    ],
                ))
            }
        }
        GameEvent::Redirected { what, to_id, .. } => s(
            "{what} was redirected to {target}",
            vec![
                p("what", capitalised(&redirected_what(names, *what))),
                p("target", names.instance(to_id, None)),
            ],
        ),
        GameEvent::HealthSet { player, health, .. } => s(
            "{whose} hero's health was set to {health}",
            vec![p("whose", capitalised(&whose(*player))), p("health", *health)],
        ),
        // R404: the event names the quest by its id; its words are the card's view's while it is open.
        GameEvent::QuestProgressed {
            player,
            instance_id,
            quest,
            progress,
            goal,
        } => {
            let text = quest_text(view, instance_id, quest);
            let mut parts = vec![
                p("whose", whose(*player)),
                p("card", names.instance(instance_id, Some("card"))),
                p("progress", *progress),
                p("goal", *goal),
            ];
            cap(match text {
                None => s("{whose} {card} quest: {progress}/{goal}", parts),
                Some(text) => {
                    parts.push(p("text", names.say(&text)));
                    s("{whose} {card} quest: {text}, {progress}/{goal}", parts)
                }
            })
        }
        // A completed quest has left the view's open quests, so its words are gone with it.
        GameEvent::QuestCompleted {
            player, instance_id, ..
        } => cap(s(
            "{whose} {card} completed a quest",
            vec![
                p("whose", whose(*player)),
                p("card", names.instance(instance_id, Some("card"))),
            ],
        )),
        // R831003: the board draws the curve; the line names it as the equation its cells make, read from
        // the caster's side (curve.rs), and says only whose it was when the cells make none.
        GameEvent::CurveDrawn { player, cells } => match curve_equation(cells, *player) {
            None => s(
                "{seat} drew a curve across the board",
                vec![p("seat", seat(*player))],
            ),
            Some(equation) => s(
                "{seat} drew the curve {equation} across the board",
                vec![p("seat", seat(*player)), p("equation", equation)],
            ),
        },
        GameEvent::RolledBack { turns_ago, .. } => {
            if *turns_ago == 1 {
                s("The board went back {n} turn", vec![p("n", *turns_ago)])
            } else {
                s("The board went back {n} turns", vec![p("n", *turns_ago)])
            }
        }
        // R436: the effects in the words the reveal shows (chaos.rs), never the engine's keys. The roll is
        // public; a card the viewer may not read is still a Call to Chaos (R97).
        GameEvent::ChaosRolled { def_id, effects, .. } => {
            let who = if hidden(def_id) {
                names.say("Call to Chaos")
            } else {
                names.def(def_id)
            };
            let said: Vec<String> = effects
                .iter()
                .map(|effect| names.say(&chaos_effect_name(def_id, effect)))
                .collect();
            s(
                "{who} rolled: {names}",
                vec![p("who", who), p("names", said.join(list_separator(w)))],
            )
        }
        // R2652001: the batch in the words the panel lands on; a card the viewer may not read is a hidden
        // card (R97). A face-down trap's flip never reaches the other seat (R2652000).
        GameEvent::Rolled {
            instance_id,
            def_id,
            kind,
            sides,
            rolls,
            ..
        } => {
            let who = names.card(instance_id, def_id);
            let said = roll_words(
                names,
                *kind == jackioh_engine::wire::RollKind::Coin,
                *sides,
                rolls,
                who,
            );
            cap(vec![Span::Text { text: said }])
        }
        GameEvent::Flickered {
            instance_id, def_id, ..
        } => cap(s(
            "{card} flickered",
            vec![p("card", names.card(instance_id, def_id))],
        )),
        GameEvent::DrawLimited { player } => s(
            "{seat} could not draw more this turn",
            vec![p("seat", seat(*player))],
        ),
        GameEvent::DiscardPrevented { player, .. } => s(
            "{seat} could not be made to discard",
            vec![p("seat", seat(*player))],
        ),
        GameEvent::TurnCutShort { player, .. } => {
            cap(s("{whose} turn was cut short", vec![p("whose", whose(*player))]))
        }
        // R1183002: the engine plays the rest of that turn; its actions log as their own lines.
        GameEvent::AutopilotEngaged { player, .. } => s(
            "The AI plays the rest of {whose} turn",
            vec![p("whose", whose(*player))],
        ),
        // R1839002: a Dormant card awakened, its countdown run out or an effect's doing.
        GameEvent::Awakened { def_id, .. } => s("{card} awakened", vec![p("card", names.def(def_id))]),
        // R3536005: a card evolved, its Evolve text running next.
        GameEvent::Evolved { def_id, .. } => s("{card} evolved", vec![p("card", names.def(def_id))]),
        // R1838001: a Trade names its player alone; the card going in and the one drawn log as their own.
        GameEvent::Traded { player } => s("{seat} traded a card", vec![p("seat", seat(*player))]),
        // R1840005: Credits are public.
        GameEvent::MarketCreditsChanged { player, value } => cap(s(
            "{whose} Credits are {value}",
            vec![p("whose", whose(*player)), p("value", *value)],
        )),
        // R1853003: a purchase is hidden like a gain, so the other seat's line reads "a card"; the price is public.
        GameEvent::MarketBought {
            player,
            def_id,
            credits,
            ..
        } => s(
            "{seat} bought {card} for {credits} Credits",
            vec![
                p("seat", seat(*player)),
                p("card", names.named(def_id)),
                p("credits", *credits),
            ],
        ),
        // R1853003: a sale is public, and names its Resource.
        GameEvent::MarketSold {
            player,
            def_id,
            credits,
            ..
        } => s(
            "{seat} sold {card} for {credits} Credits",
            vec![
                p("seat", seat(*player)),
                p("card", names.named(def_id)),
                p("credits", *credits),
            ],
        ),
        // R1852000: a piece died, and a copy of it fused into its owner's Dormant Starship.
        GameEvent::StarshipFused {
            player,
            def_id,
            pieces,
            ..
        } => {
            let english = if *pieces == 1 {
                "{card} fused into {whose} Dormant Starship ({n} piece)"
            } else {
                "{card} fused into {whose} Dormant Starship ({n} pieces)"
            };
            s(
                english,
                vec![
                    p("card", names.def(def_id)),
                    p("whose", whose(*player)),
                    p("n", *pieces as i32),
                ],
            )
        }
        // R1852003: the Starship launched under the name it took.
        GameEvent::StarshipLaunched { player, name, .. } => cap(s(
            "{whose} Starship launched as {name}",
            vec![p("whose", whose(*player)), p("name", name.as_str())],
        )),
        GameEvent::VoteCast {
            player,
            instance_id,
            def_id,
            votes,
            needed,
        } => cap(s(
            "{card} voted to remove {who} ({votes} of {needed})",
            vec![
                p("card", names.card(instance_id, def_id)),
                p("who", names.removed(*player)),
                p("votes", *votes),
                p("needed", *needed),
            ],
        )),
        // R1843000: the cards that agreed, by name (a card the viewer may not read is "a hidden card").
        GameEvent::VotedOut { player, ballots } => {
            let cards: Vec<String> = ballots
                .iter()
                .map(|ballot| names.card(&ballot.instance_id, &ballot.def_id))
                .collect();
            cap(s(
                "{cards} agreed: {who} removed from the game",
                vec![
                    p("cards", cards.join(list_separator(w))),
                    p("who", names.removed(*player)),
                ],
            ))
        }
        // R1843002: the escape, and what it leaves the other player.
        GameEvent::Fled { player, opponent } => {
            let seat = vec![p("seat", seat(*player))];
            match opponent {
                jackioh_engine::wire::FleeOpponent::Loss => {
                    s("{seat} escaped the universe, leaving the other side a loss", seat)
                }
                jackioh_engine::wire::FleeOpponent::Draw => {
                    s("{seat} escaped the universe, leaving the other side a draw", seat)
                }
            }
        }
        // R1850003: the card and the one it is held beneath, each a card back where the view hides it.
        GameEvent::Held {
            instance_id,
            def_id,
            holder_id,
            ..
        } => cap(s(
            "{card} is held beneath {holder}",
            vec![
                p("card", names.card(instance_id, def_id)),
                p("holder", names.instance(holder_id, None)),
            ],
        )),
        // R1855000: whose board an Anomaly replaced; the cards' `exiled` and `summoned` lines follow.
        GameEvent::BoardReplaced { player, .. } => {
            cap(s("{whose} board was replaced", vec![p("whose", whose(*player))]))
        }
        // R676: what a Glitch did, under its corrupted name (face/glitch.rs).
        GameEvent::Glitched { outcome, .. } => s(
            "{glitch}: {outcome}",
            vec![
                p("glitch", glitch_words().name),
                p("outcome", glitch_outcome(names, *outcome)),
            ],
        ),
        GameEvent::FengShui { player, outcome, .. } => cap(s(
            "{whose} play was {outcome}",
            vec![
                p("whose", whose(*player)),
                p("outcome", feng_shui_outcome(names, *outcome)),
            ],
        )),
        // R437: the mark by the name its badge says, never the engine's key; a marked card the viewer may not
        // read (a face-down trap) is "a card".
        GameEvent::Marked {
            instance_id,
            mark,
            added,
            ..
        } => {
            if !*added {
                return None;
            }
            s(
                "{target} was marked ({mark})",
                vec![
                    p(
                        "target",
                        capitalised(&names.instance(instance_id, Some("a card"))),
                    ),
                    p("mark", mark_name(names, mark)),
                ],
            )
        }
        // R1301: only the language the card is shown in changed; its face says so.
        GameEvent::Translated { .. } => return None,
        // R2783000: a marker for the animations; the hook's own effects log as their own lines.
        GameEvent::HookResolving { .. } => return None,
    };
    Some(line)
}
